//! The mixer core (roadmap T06-006, T06-013, T06-017): **voices, buses, culling and the sum of it all as plain sample arithmetic**, with
//! no audio device and no decoder. A device callback (T06-001) or an offline renderer ([`render_offline`]) calls
//! [`Mixer::render`] for each block; decoded samples come from [`SampleBuffer`]s registered under a [`SoundId`].
//!
//! * **Voices** play a buffer with a volume, a pitch (resampled by linear interpolation), a bus, a priority, optional looping and an optional
//!   position. A spatial voice is attenuated by distance (linearly from `min_distance` to `max_distance`, silent beyond) and panned with an
//!   equal-power law from where it is relative to the listener; a 2D voice is centred.
//! * **Buses** (`Master`, `Music`, `Sfx`, `Voice`, `Ui`) have user volumes (0 to 1); `Master` scales the rest. **Ducking** lowers `Music` while any
//!   `Voice` voice is audible, with an attack and a release.
//! * **Voice management.** At most `max_voices` voices are mixed (a mobile tier sets fewer); the rest are **virtualized**: they keep advancing as if
//!   playing, so they are in the right place if they become audible again, but cost no mixing. Which voices stay audible: higher priority first,
//!   then the louder, then the older. **`Priority::CRITICAL` voices are never virtualized by pressure**, and do not count against the limit.
//!   A voice beyond its `max_distance` is silent and so virtualized too.
//! * **Suspend** (`set_suspended`) fades the output to silence and back without dropping voices.
//! * **Smooth**: gains change by a ramp across each block (and stops fade out), so nothing clicks.
//!
//! **Deterministic**: voices are visited in id order, all arithmetic is `f32` (the playback cursor is `f64`), nothing reads a clock: the same calls give the
//! same samples on one machine and build (across architectures the samples may differ in the last bits: golden tests compare with a tolerance, see
//! [`compare`]). Stale voice ids are harmless: operations on a finished voice do nothing.
//!
//! Not here: decoding, streaming from archives, the output device and its thread (T06-001), music layers (T06-005), reverb (T06-007), the Doppler effect,
//! occlusion, the original game's attenuation curves (T06-003: the default is linear) and `impl Audio` for the mixer (it needs sound resolution by path).

use std::collections::BTreeMap;
use std::sync::Arc;

use ss2_audio_api::{Bus, Listener, PlayParams, Priority, SoundId, VoiceId, VoiceUpdate};
use ss2_core::{Error, Result};

/// Decoded audio: interleaved `f32` frames of 1 (mono) or 2 (stereo) channels.
#[derive(Clone, Debug, PartialEq)]
pub struct SampleBuffer {
    /// Frames per second of the samples.
    pub rate: u32,
    /// 1 or 2.
    pub channels: u8,
    /// Interleaved samples; its length is a multiple of `channels`.
    pub samples: Arc<[f32]>,
}

impl SampleBuffer {
    /// A buffer from samples. `Err` for a rate of 0, a channel count other than 1 or 2, or a length that is not a whole number of frames.
    pub fn new(rate: u32, channels: u8, samples: Vec<f32>) -> Result<Self> {
        if rate == 0
            || !(1..=2).contains(&channels)
            || !samples.len().is_multiple_of(usize::from(channels))
        {
            return Err(Error::invalid(
                "sample buffer",
                format!(
                    "rate {rate}, {channels} channels, {} samples",
                    samples.len()
                ),
            ));
        }
        Ok(Self {
            rate,
            channels,
            samples: samples.into(),
        })
    }

    /// How many frames.
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }
}

/// How the music bus is lowered while a voice-bus voice is audible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ducking {
    /// The gain the music bus is lowered to (0 to 1; 1 turns ducking off).
    pub music_gain: f32,
    /// How long it takes to go down, in milliseconds.
    pub attack_ms: f32,
    /// How long it takes to come back up, in milliseconds.
    pub release_ms: f32,
}

impl Default for Ducking {
    /// Music down to 30 % in 50 ms, back in 400 ms.
    fn default() -> Self {
        Self {
            music_gain: 0.3,
            attack_ms: 50.0,
            release_ms: 400.0,
        }
    }
}

/// What a mixer needs to know.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerConfig {
    /// Output frames per second.
    pub sample_rate: u32,
    /// The most voices mixed at once; the others are virtualized (`Priority::CRITICAL` voices do not count).
    pub max_voices: usize,
    /// Music ducking under voice-bus voices.
    pub ducking: Ducking,
    /// How long a stop with no fade, and a suspend or resume, take to fade, in milliseconds (so nothing clicks).
    pub declick_ms: f32,
}

impl Default for MixerConfig {
    /// 48 kHz, 32 voices, default ducking, a 10 ms declick.
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            max_voices: 32,
            ducking: Ducking::default(),
            declick_ms: 10.0,
        }
    }
}

/// The most voices that may exist at once (playing, virtualized or fading out).
pub const MAX_VOICES_ALIVE: usize = 1024;

/// Counters of what the mixer did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MixerStats {
    /// Voices mixed in the last block.
    pub audible: usize,
    /// Voices alive but not mixed in the last block (over the limit, or out of range).
    pub virtualized: usize,
    /// Output samples that had to be clamped to the range -1 to 1, in total.
    pub clipped_samples: u64,
    /// Output frames rendered in total.
    pub frames: u64,
}

/// What a voice is doing, for tests and tools.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceState {
    /// In the buffer, in source frames.
    pub position: f64,
    /// Not mixed in the last block.
    pub virtualized: bool,
    /// Fading out to a stop.
    pub stopping: bool,
}

struct Voice {
    buffer: SampleBuffer,
    params: PlayParams,
    cursor: f64,
    virtualized: bool,
    /// Frames left of a fade-out, and its length; `None` while playing normally.
    stop: Option<(u64, u64)>,
    gain: [f32; 2],
    fresh: bool,
}

/// The mixer. See the module documentation.
///
/// ```
/// use ss2_audio::{Mixer, MixerConfig, SampleBuffer};
/// use ss2_audio_api::{PlayParams, SoundId};
///
/// let mut mixer = Mixer::new(MixerConfig::default()).unwrap();
/// let beep = SoundId::from_raw(1).unwrap();
/// mixer.add_sound(beep, SampleBuffer::new(48_000, 1, vec![0.5; 4_800]).unwrap());
/// let voice = mixer.play(beep, &PlayParams::sfx()).unwrap();
/// let mut block = vec![0.0f32; 2 * 480];
/// mixer.render(&mut block);
/// assert!(block.iter().any(|s| *s != 0.0) && mixer.is_playing(voice));
/// ```
pub struct Mixer {
    cfg: MixerConfig,
    sounds: BTreeMap<SoundId, SampleBuffer>,
    voices: BTreeMap<VoiceId, Voice>,
    next_voice: u64,
    bus: [f32; 5],
    listener: Listener,
    duck: f32,
    suspend: f32,
    suspended: bool,
    stats: MixerStats,
}

fn bus_index(bus: Bus) -> usize {
    match bus {
        Bus::Master => 0,
        Bus::Music => 1,
        Bus::Sfx => 2,
        Bus::Voice => 3,
        Bus::Ui => 4,
        // A bus added later that this mixer does not know: it plays at the master volume only.
        _ => 0,
    }
}

impl Mixer {
    /// A mixer with every bus at full volume. `Err` for a sample rate of 0 or a configuration out of range.
    pub fn new(cfg: MixerConfig) -> Result<Self> {
        let d = cfg.ducking;
        let ok = cfg.sample_rate > 0
            && cfg.max_voices > 0
            && (0.0..=1.0).contains(&d.music_gain)
            && d.attack_ms.is_finite()
            && d.attack_ms >= 0.0
            && d.release_ms.is_finite()
            && d.release_ms >= 0.0
            && cfg.declick_ms.is_finite()
            && cfg.declick_ms >= 0.0;
        if !ok {
            return Err(Error::invalid("mixer config", format!("{cfg:?}")));
        }
        Ok(Self {
            cfg,
            sounds: BTreeMap::new(),
            voices: BTreeMap::new(),
            next_voice: 1,
            bus: [1.0; 5],
            listener: Listener::default(),
            duck: 1.0,
            suspend: 1.0,
            suspended: false,
            stats: MixerStats::default(),
        })
    }

    /// Register decoded audio under `sound` (replacing an earlier one; voices already playing keep the old buffer).
    pub fn add_sound(&mut self, sound: SoundId, buffer: SampleBuffer) {
        self.sounds.insert(sound, buffer);
    }

    /// Start a voice. `Err` for invalid parameters, a sound that was never added, or too many voices alive ([`MAX_VOICES_ALIVE`]).
    pub fn play(&mut self, sound: SoundId, params: &PlayParams) -> Result<VoiceId> {
        params.validate()?;
        let buffer = self.sounds.get(&sound).cloned().ok_or_else(|| {
            Error::invalid("play", format!("sound {} was never added", sound.raw()))
        })?;
        if self.voices.len() >= MAX_VOICES_ALIVE {
            return Err(Error::invalid("play", "too many voices".to_owned()));
        }
        let id = VoiceId(self.next_voice);
        self.next_voice += 1;
        self.voices.insert(
            id,
            Voice {
                buffer,
                params: *params,
                cursor: 0.0,
                virtualized: false,
                stop: None,
                gain: [0.0; 2],
                fresh: true,
            },
        );
        Ok(id)
    }

    /// Stop a voice, fading out over `fade_ms` milliseconds (at least the declick time). Does nothing for a finished voice.
    pub fn stop(&mut self, voice: VoiceId, fade_ms: f32) {
        let fade = if fade_ms.is_finite() {
            fade_ms.max(self.cfg.declick_ms)
        } else {
            self.cfg.declick_ms
        };
        let frames = self.ms_to_frames(fade).max(1);
        if let Some(v) = self.voices.get_mut(&voice).filter(|v| v.stop.is_none()) {
            v.stop = Some((frames, frames));
        }
    }

    /// Whether the voice is playing (including virtualized or fading out).
    pub fn is_playing(&self, voice: VoiceId) -> bool {
        self.voices.contains_key(&voice)
    }

    /// Change a playing voice. `Err` for invalid values (nothing changes); a finished voice is ignored.
    pub fn update_voice(&mut self, voice: VoiceId, update: &VoiceUpdate) -> Result<()> {
        update.validate()?;
        if let Some(v) = self.voices.get_mut(&voice) {
            if let Some(s) = &mut v.params.spatial {
                if let Some(p) = update.position {
                    s.position = p;
                }
                if let Some(vel) = update.velocity {
                    s.velocity = vel;
                }
            }
            if let Some(vol) = update.volume {
                v.params.volume = vol;
            }
            if let Some(p) = update.pitch {
                v.params.pitch = p;
            }
        }
        Ok(())
    }

    /// Where the voice is and what it is doing.
    pub fn voice_state(&self, voice: VoiceId) -> Option<VoiceState> {
        self.voices.get(&voice).map(|v| VoiceState {
            position: v.cursor,
            virtualized: v.virtualized,
            stopping: v.stop.is_some(),
        })
    }

    /// Set the listener; an unusable one (not finite, `forward` parallel to `up`) is refused and the previous one kept.
    pub fn set_listener(&mut self, listener: &Listener) -> Result<()> {
        if !listener.is_valid() {
            return Err(Error::invalid(
                "listener",
                "not finite, or forward and up are parallel".to_owned(),
            ));
        }
        self.listener = *listener;
        Ok(())
    }

    /// Set a bus volume, clamped to 0 to 1; NaN is ignored.
    pub fn set_bus_volume(&mut self, bus: Bus, volume: f32) {
        if !volume.is_nan() {
            self.bus[bus_index(bus)] = volume.clamp(0.0, 1.0);
        }
    }

    /// A bus volume.
    pub fn bus_volume(&self, bus: Bus) -> f32 {
        self.bus[bus_index(bus)]
    }

    /// Fade the output to silence (the app is suspended or unfocused) or back, without dropping voices.
    pub fn set_suspended(&mut self, suspended: bool) {
        self.suspended = suspended;
    }

    /// What the mixer has done so far.
    pub fn stats(&self) -> MixerStats {
        self.stats
    }

    /// How many voices are alive.
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    fn ms_to_frames(&self, ms: f32) -> u64 {
        (f64::from(ms) * f64::from(self.cfg.sample_rate) / 1000.0).round() as u64
    }

    /// The bus gain a voice sees: its bus times master (the master bus itself once), times the ducking of music.
    fn bus_gain(&self, bus: Bus) -> f32 {
        let i = bus_index(bus);
        let base = if i == 0 {
            self.bus[0]
        } else {
            self.bus[0] * self.bus[i]
        };
        if i == 1 { base * self.duck } else { base }
    }

    /// Distance attenuation and equal-power pan of a voice: (gain, pan -1 to 1).
    fn spatialize(&self, params: &PlayParams) -> (f32, f32) {
        let Some(s) = params.spatial else {
            return (1.0, 0.0);
        };
        let to = s.position - self.listener.position;
        let distance = to.length();
        let gain = if distance <= s.min_distance {
            1.0
        } else if distance >= s.max_distance {
            0.0
        } else {
            1.0 - (distance - s.min_distance) / (s.max_distance - s.min_distance)
        };
        let right = self
            .listener
            .forward
            .cross(self.listener.up)
            .normalize_or_zero();
        let pan = if distance > 1e-6 {
            (to / distance).dot(right).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        (gain, pan)
    }

    /// Render the next block of interleaved stereo frames into `out` (its length should be even; an odd last sample is left silent).
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let frames = out.len() / 2;
        if frames == 0 {
            return;
        }
        let dt_ms = frames as f32 * 1000.0 / self.cfg.sample_rate as f32;
        let declick_frames = self.ms_to_frames(self.cfg.declick_ms).max(1) as f32;
        // Suspend ramp: towards 0 or 1 at the declick rate.
        let suspend_target = if self.suspended { 0.0 } else { 1.0 };
        let suspend_from = self.suspend;
        let step = frames as f32 / declick_frames;
        self.suspend = if suspend_target > suspend_from {
            (suspend_from + step).min(1.0)
        } else if suspend_target < suspend_from {
            (suspend_from - step).max(0.0)
        } else {
            suspend_from
        };
        let suspend_to = self.suspend;

        // Pass 1: what each voice would sound like (before the limit).
        struct Candidate {
            id: VoiceId,
            priority: Priority,
            audibility: f32,
            gains: [f32; 2],
        }
        let mut candidates: Vec<Candidate> = Vec::with_capacity(self.voices.len());
        let mut any_voice_bus_audible = false;
        for (id, v) in &self.voices {
            let (distance_gain, pan) = self.spatialize(&v.params);
            let level = v.params.volume * distance_gain * self.bus_gain(v.params.bus);
            let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            candidates.push(Candidate {
                id: *id,
                priority: v.params.priority,
                audibility: level,
                gains: [level * angle.cos(), level * angle.sin()],
            });
        }
        // Pass 2: who is mixed. Critical voices always; the others by priority, loudness, age, up to the limit; the silent ones never.
        let mut audible: BTreeMap<VoiceId, [f32; 2]> = BTreeMap::new();
        let mut contenders: Vec<&Candidate> = Vec::new();
        for c in &candidates {
            if c.priority == Priority::CRITICAL {
                audible.insert(c.id, c.gains);
            } else if c.audibility > 0.0 {
                contenders.push(c);
            }
        }
        contenders.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(b.audibility.total_cmp(&a.audibility))
                .then(a.id.cmp(&b.id))
        });
        for c in contenders.into_iter().take(self.cfg.max_voices) {
            audible.insert(c.id, c.gains);
        }
        for (id, gains) in &audible {
            if self
                .voices
                .get(id)
                .is_some_and(|v| v.params.bus == Bus::Voice)
                && gains[0].max(gains[1]) > 0.0
            {
                any_voice_bus_audible = true;
            }
        }

        // Ducking: move the music duck gain towards its target by the time constant.
        let d = self.cfg.ducking;
        let (target, ms) = if any_voice_bus_audible {
            (d.music_gain, d.attack_ms)
        } else {
            (1.0, d.release_ms)
        };
        if ms <= 0.0 {
            self.duck = target;
        } else {
            let alpha = (dt_ms / ms).min(1.0);
            self.duck += (target - self.duck) * alpha;
        }

        // Pass 3: mix the audible voices, advance the others.
        let mut finished: Vec<VoiceId> = Vec::new();
        let (mut audible_count, mut virtual_count) = (0usize, 0usize);
        for (id, v) in &mut self.voices {
            let src_frames = v.buffer.frames();
            let step = f64::from(v.params.pitch) * f64::from(v.buffer.rate)
                / f64::from(self.cfg.sample_rate);
            let target = audible.get(id).copied();
            v.virtualized = target.is_none();
            let mixing = target.is_some();
            if mixing {
                audible_count += 1;
            } else {
                virtual_count += 1;
            }
            let target = target.unwrap_or([0.0; 2]);
            if v.fresh {
                // A new voice starts at its gain (with the declick fade-in below) rather than ramping from zero over the block.
                v.gain = [0.0; 2];
                v.fresh = false;
            }
            let from = v.gain;
            let channels = usize::from(v.buffer.channels);
            let mut done = false;
            for f in 0..frames {
                if v.cursor >= src_frames as f64 {
                    if v.params.looped && src_frames > 0 {
                        v.cursor -= src_frames as f64;
                    } else {
                        done = true;
                        break;
                    }
                }
                if mixing {
                    let t = (f + 1) as f32 / frames as f32;
                    let mut fade = 1.0;
                    if let Some((left, total)) = v.stop.as_mut() {
                        fade = *left as f32 / *total as f32;
                        *left = left.saturating_sub(1);
                    }
                    let suspend = suspend_from + (suspend_to - suspend_from) * t;
                    let gl = (from[0] + (target[0] - from[0]) * t) * fade * suspend;
                    let gr = (from[1] + (target[1] - from[1]) * t) * fade * suspend;
                    let i = v.cursor as usize;
                    let frac = (v.cursor - i as f64) as f32;
                    let next = if i + 1 < src_frames {
                        i + 1
                    } else if v.params.looped {
                        0
                    } else {
                        i
                    };
                    let sample = |frame: usize, ch: usize| {
                        v.buffer.samples[frame * channels + ch.min(channels - 1)]
                    };
                    let l = sample(i, 0) * (1.0 - frac) + sample(next, 0) * frac;
                    let r = sample(i, 1) * (1.0 - frac) + sample(next, 1) * frac;
                    out[2 * f] += l * gl;
                    out[2 * f + 1] += r * gr;
                } else if let Some((left, _)) = v.stop.as_mut() {
                    *left = left.saturating_sub(1);
                }
                v.cursor += step;
            }
            v.gain = target;
            if done || v.stop.is_some_and(|(left, _)| left == 0) {
                finished.push(*id);
            }
        }
        for id in finished {
            self.voices.remove(&id);
        }
        for s in out.iter_mut() {
            if s.abs() > 1.0 {
                *s = s.clamp(-1.0, 1.0);
                self.stats.clipped_samples += 1;
            }
        }
        self.stats.audible = audible_count;
        self.stats.virtualized = virtual_count;
        self.stats.frames += frames as u64;
    }
}

/// Render `frames` frames of stereo audio from `mixer` in blocks of `block_frames`, as interleaved samples (the offline renderer: golden-audio
/// tests and capture). The same mixer state, frame count and block size give the same samples.
pub fn render_offline(mixer: &mut Mixer, frames: usize, block_frames: usize) -> Vec<f32> {
    let block = block_frames.max(1);
    let mut out = vec![0.0f32; frames * 2];
    let mut done = 0;
    while done < frames {
        let n = block.min(frames - done);
        mixer.render(&mut out[done * 2..(done + n) * 2]);
        done += n;
    }
    out
}
