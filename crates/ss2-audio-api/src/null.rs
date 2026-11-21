//! [`NullAudio`]: an [`Audio`] that plays nothing but behaves like the real thing (voice ids,
//! culling under pressure, buses, music, listener) so game code can be tested without a device.

use std::collections::BTreeMap;
use std::time::Duration;

use ss2_core::Result;

use crate::api::{Audio, AudioResources};
use crate::types::{Bus, Listener, PlayParams, SoundId, VoiceId, VoiceUpdate};

/// Hands out sequential [`SoundId`]s from 1 and remembers each normalized path.
#[derive(Clone, Debug, Default)]
pub struct MockAudioResources {
    ids: BTreeMap<String, u32>,
}

impl MockAudioResources {
    /// The normalized path a sound id was created from.
    pub fn path_of(&self, id: SoundId) -> Option<&str> {
        self.ids
            .iter()
            .find(|(_, raw)| **raw == id.raw())
            .map(|(path, _)| path.as_str())
    }
}

/// `\` → `/`, lower case, no empty or `.` segments (stand-in for `VfsPath` normalization).
fn normalize(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != ".")
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("/")
}

impl AudioResources for MockAudioResources {
    fn sound(&mut self, path: &str) -> SoundId {
        let key = normalize(path);
        let next = u32::try_from(self.ids.len() + 1).unwrap_or(u32::MAX);
        let raw = *self.ids.entry(key).or_insert(next);
        SoundId::from_raw(raw).expect("ids start at 1")
    }
}

/// Something [`NullAudio`] was asked to do, in order; for assertions in tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCall {
    /// A voice was started (it may have been culled immediately).
    Play {
        /// The new voice.
        voice: VoiceId,
        /// What it plays.
        sound: SoundId,
    },
    /// A voice was stopped by the caller.
    Stop(VoiceId),
    /// A one-shot voice finished by itself ([`NullAudio::end_voice`]).
    Ended(VoiceId),
    /// A voice was culled to make room for a higher-priority one.
    Culled(VoiceId),
    /// Music changed to this track.
    MusicStarted(SoundId),
    /// Music was stopped.
    MusicStopped,
}

#[derive(Clone, Debug)]
struct Voice {
    sound: SoundId,
    params: PlayParams,
}

/// A headless [`Audio`]. Keeps full state (voices and their parameters, listener, bus volumes,
/// music, suspend flag) plus a [`call log`](Self::calls).
///
/// ```
/// use std::time::Duration;
/// use ss2_audio_api::{Audio, AudioResources, NullAudio, PlayParams};
/// use ss2_core::math::Vec3;
///
/// let mut audio = NullAudio::new();
/// let shot = audio.resources().sound("Sounds/Weapons/Shotgun.wav");
/// let voice = audio.play(shot, &PlayParams::sfx().at(Vec3::new(3.0, 0.0, -1.0))).unwrap();
/// assert!(audio.is_playing(voice));
/// audio.stop(voice, Duration::ZERO);
/// assert!(!audio.is_playing(voice));
/// ```
#[derive(Debug)]
pub struct NullAudio {
    resources: MockAudioResources,
    next_voice: u64,
    voice_limit: usize,
    voices: BTreeMap<VoiceId, Voice>,
    listener: Listener,
    bus_volumes: BTreeMap<Bus, f32>,
    music: Option<SoundId>,
    suspended: bool,
    calls: Vec<AudioCall>,
}

impl Default for NullAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl NullAudio {
    /// An unlimited-voice device at default state (all buses at 1.0).
    pub fn new() -> Self {
        Self::with_voice_limit(usize::MAX)
    }

    /// A device that mixes at most `limit` voices at once, culling the lowest priority beyond it
    /// (like the mobile voice budget, T06-006).
    pub fn with_voice_limit(limit: usize) -> Self {
        Self {
            resources: MockAudioResources::default(),
            next_voice: 1,
            voice_limit: limit,
            voices: BTreeMap::new(),
            listener: Listener::default(),
            bus_volumes: Bus::ALL.iter().map(|b| (*b, 1.0)).collect(),
            music: None,
            suspended: false,
            calls: Vec::new(),
        }
    }

    /// Everything asked of this device so far, oldest first.
    pub fn calls(&self) -> &[AudioCall] {
        &self.calls
    }

    /// Number of voices currently playing.
    pub fn playing_count(&self) -> usize {
        self.voices.len()
    }

    /// The sound and parameters of a playing voice.
    pub fn voice(&self, voice: VoiceId) -> Option<(SoundId, &PlayParams)> {
        self.voices.get(&voice).map(|v| (v.sound, &v.params))
    }

    /// The listener last accepted.
    pub fn listener(&self) -> &Listener {
        &self.listener
    }

    /// Whether [`set_suspended`](Audio::set_suspended)`(true)` is in effect.
    pub fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// The path registry, for resolving ids back to names in assertions.
    pub fn mock_resources(&self) -> &MockAudioResources {
        &self.resources
    }

    /// Simulate a one-shot voice reaching its end. Returns `false` (and does nothing) for looped
    /// or already-finished voices: those only end through [`stop`](Audio::stop).
    pub fn end_voice(&mut self, voice: VoiceId) -> bool {
        match self.voices.get(&voice) {
            Some(v) if !v.params.looped => {
                self.voices.remove(&voice);
                self.calls.push(AudioCall::Ended(voice));
                true
            }
            _ => false,
        }
    }
}

impl Audio for NullAudio {
    fn resources(&mut self) -> &mut dyn AudioResources {
        &mut self.resources
    }

    fn play(&mut self, sound: SoundId, params: &PlayParams) -> Result<VoiceId> {
        params.validate()?;
        let id = VoiceId(self.next_voice);
        self.next_voice += 1;
        self.calls.push(AudioCall::Play { voice: id, sound });

        if self.voices.len() >= self.voice_limit {
            // Cull the lowest-priority voice (oldest first among equals) if the newcomer outranks
            // it; otherwise the newcomer itself is culled and never becomes audible.
            let victim = self
                .voices
                .iter()
                .min_by_key(|(vid, v)| (v.params.priority, **vid))
                .map(|(vid, v)| (*vid, v.params.priority));
            match victim {
                Some((vid, priority)) if params.priority > priority => {
                    self.voices.remove(&vid);
                    self.calls.push(AudioCall::Culled(vid));
                }
                _ => {
                    self.calls.push(AudioCall::Culled(id));
                    return Ok(id);
                }
            }
        }
        self.voices.insert(
            id,
            Voice {
                sound,
                params: *params,
            },
        );
        Ok(id)
    }

    fn stop(&mut self, voice: VoiceId, _fade: Duration) {
        if self.voices.remove(&voice).is_some() {
            self.calls.push(AudioCall::Stop(voice));
        }
    }

    fn is_playing(&self, voice: VoiceId) -> bool {
        self.voices.contains_key(&voice)
    }

    fn update_voice(&mut self, voice: VoiceId, update: &VoiceUpdate) -> Result<()> {
        update.validate()?;
        if let Some(v) = self.voices.get_mut(&voice) {
            if let Some(spatial) = &mut v.params.spatial {
                if let Some(p) = update.position {
                    spatial.position = p;
                }
                if let Some(vel) = update.velocity {
                    spatial.velocity = vel;
                }
            }
            if let Some(volume) = update.volume {
                v.params.volume = volume;
            }
            if let Some(pitch) = update.pitch {
                v.params.pitch = pitch;
            }
        }
        Ok(())
    }

    fn set_listener(&mut self, listener: &Listener) -> Result<()> {
        if !listener.is_valid() {
            return Err(ss2_core::Error::invalid(
                "audio listener",
                format!("{listener:?} is not a usable listener"),
            ));
        }
        self.listener = *listener;
        Ok(())
    }

    fn play_music(&mut self, track: SoundId, _crossfade: Duration) {
        if self.music != Some(track) {
            self.music = Some(track);
            self.calls.push(AudioCall::MusicStarted(track));
        }
    }

    fn stop_music(&mut self, _fade: Duration) {
        if self.music.take().is_some() {
            self.calls.push(AudioCall::MusicStopped);
        }
    }

    fn current_music(&self) -> Option<SoundId> {
        self.music
    }

    fn set_bus_volume(&mut self, bus: Bus, volume: f32) {
        if volume.is_finite() {
            self.bus_volumes.insert(bus, volume.clamp(0.0, 1.0));
        }
    }

    fn bus_volume(&self, bus: Bus) -> f32 {
        self.bus_volumes.get(&bus).copied().unwrap_or(1.0)
    }

    fn set_suspended(&mut self, suspended: bool) {
        self.suspended = suspended;
    }
}

#[cfg(test)]
mod tests {
    use ss2_core::Error;
    use ss2_core::math::Vec3;

    use super::*;
    use crate::types::Priority;

    fn sound(audio: &mut NullAudio, name: &str) -> SoundId {
        audio.resources().sound(name)
    }

    #[test]
    fn same_path_same_sound_regardless_of_spelling() {
        let mut a = NullAudio::new();
        let s = sound(&mut a, "Sounds/A.wav");
        assert_eq!(sound(&mut a, r"sounds\a.WAV"), s);
        assert_ne!(sound(&mut a, "sounds/b.wav"), s);
        assert_eq!(a.mock_resources().path_of(s), Some("sounds/a.wav"));
    }

    #[test]
    fn play_stop_and_stale_voice_ids() {
        let mut a = NullAudio::new();
        let s = sound(&mut a, "x.wav");
        let v1 = a.play(s, &PlayParams::sfx()).unwrap();
        let v2 = a.play(s, &PlayParams::sfx()).unwrap();
        assert_ne!(v1, v2);
        assert_eq!(a.playing_count(), 2);
        a.stop(v1, Duration::ZERO);
        a.stop(v1, Duration::ZERO); // second stop: no-op, not logged twice
        assert!(!a.is_playing(v1) && a.is_playing(v2));
        let v3 = a.play(s, &PlayParams::sfx()).unwrap();
        assert!(v3 > v2, "ids are never reused");
        assert_eq!(
            a.calls()
                .iter()
                .filter(|c| matches!(c, AudioCall::Stop(_)))
                .count(),
            1
        );
    }

    #[test]
    fn invalid_params_play_nothing() {
        let mut a = NullAudio::new();
        let s = sound(&mut a, "x.wav");
        let err = a
            .play(s, &PlayParams::sfx().with_volume(f32::NAN))
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
        assert_eq!(a.playing_count(), 0);
        assert!(a.calls().is_empty());
    }

    #[test]
    fn one_shots_end_but_loops_only_stop() {
        let mut a = NullAudio::new();
        let s = sound(&mut a, "x.wav");
        let shot = a.play(s, &PlayParams::sfx()).unwrap();
        let lp = a.play(s, &PlayParams::sfx().looped()).unwrap();
        assert!(a.end_voice(shot));
        assert!(!a.end_voice(shot), "already finished");
        assert!(!a.end_voice(lp), "loops never end by themselves");
        assert!(a.is_playing(lp));
        a.stop(lp, Duration::from_millis(100));
        assert!(!a.is_playing(lp));
    }

    #[test]
    fn emitters_follow_updates_and_2d_voices_ignore_position() {
        let mut a = NullAudio::new();
        let s = sound(&mut a, "x.wav");
        let moving = a.play(s, &PlayParams::sfx().at(Vec3::ZERO)).unwrap();
        let flat = a.play(s, &PlayParams::ui()).unwrap();
        let up = VoiceUpdate {
            position: Some(Vec3::new(5.0, 0.0, 0.0)),
            volume: Some(0.25),
            ..Default::default()
        };
        a.update_voice(moving, &up).unwrap();
        a.update_voice(flat, &up).unwrap();
        let (_, p) = a.voice(moving).unwrap();
        assert_eq!(p.spatial.unwrap().position.x, 5.0);
        assert_eq!(p.volume, 0.25);
        let (_, p) = a.voice(flat).unwrap();
        assert!(p.spatial.is_none());
        assert_eq!(p.volume, 0.25);
        // Updating a dead voice is fine; invalid values are not.
        a.stop(moving, Duration::ZERO);
        assert!(a.update_voice(moving, &up).is_ok());
        assert!(
            a.update_voice(
                flat,
                &VoiceUpdate {
                    pitch: Some(0.0),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn voice_limit_culls_lowest_priority_and_still_returns_ids() {
        let mut a = NullAudio::with_voice_limit(2);
        let s = sound(&mut a, "x.wav");
        let low = a
            .play(s, &PlayParams::sfx().with_priority(Priority::LOW))
            .unwrap();
        let normal = a.play(s, &PlayParams::sfx()).unwrap();
        // Full. A HIGH newcomer evicts the LOW voice.
        let high = a
            .play(s, &PlayParams::sfx().with_priority(Priority::HIGH))
            .unwrap();
        assert!(!a.is_playing(low) && a.is_playing(normal) && a.is_playing(high));
        // A LOW newcomer is itself culled but still gets an id.
        let weak = a
            .play(s, &PlayParams::sfx().with_priority(Priority::LOW))
            .unwrap();
        assert!(!a.is_playing(weak));
        assert_eq!(a.playing_count(), 2);
        assert!(a.calls().contains(&AudioCall::Culled(low)));
        assert!(a.calls().contains(&AudioCall::Culled(weak)));
        // Equal priority does not evict (no churn).
        let same = a.play(s, &PlayParams::sfx()).unwrap();
        assert!(!a.is_playing(same) && a.is_playing(normal));
    }

    #[test]
    fn listener_validation_keeps_the_old_one() {
        let mut a = NullAudio::new();
        let good = Listener {
            position: Vec3::new(1.0, 2.0, 3.0),
            ..Listener::default()
        };
        a.set_listener(&good).unwrap();
        let bad = Listener {
            forward: Vec3::ZERO,
            ..good
        };
        assert!(a.set_listener(&bad).is_err());
        assert_eq!(*a.listener(), good);
    }

    #[test]
    fn music_crossfade_is_idempotent_per_track() {
        let mut a = NullAudio::new();
        let t1 = sound(&mut a, "music/a.ogg");
        let t2 = sound(&mut a, "music/b.ogg");
        assert_eq!(a.current_music(), None);
        a.play_music(t1, Duration::from_secs(2));
        a.play_music(t1, Duration::from_secs(2)); // already current: nothing
        a.play_music(t2, Duration::from_secs(2));
        a.stop_music(Duration::from_secs(1));
        a.stop_music(Duration::from_secs(1)); // nothing to stop
        assert_eq!(a.current_music(), None);
        let music_calls: Vec<_> = a
            .calls()
            .iter()
            .filter(|c| matches!(c, AudioCall::MusicStarted(_) | AudioCall::MusicStopped))
            .copied()
            .collect();
        assert_eq!(
            music_calls,
            [
                AudioCall::MusicStarted(t1),
                AudioCall::MusicStarted(t2),
                AudioCall::MusicStopped
            ]
        );
    }

    #[test]
    fn bus_volumes_clamp_and_ignore_nan() {
        let mut a = NullAudio::new();
        assert!(Bus::ALL.iter().all(|b| a.bus_volume(*b) == 1.0));
        a.set_bus_volume(Bus::Music, 0.3);
        a.set_bus_volume(Bus::Sfx, 5.0);
        a.set_bus_volume(Bus::Voice, -2.0);
        a.set_bus_volume(Bus::Ui, f32::NAN);
        assert_eq!(a.bus_volume(Bus::Music), 0.3);
        assert_eq!(a.bus_volume(Bus::Sfx), 1.0);
        assert_eq!(a.bus_volume(Bus::Voice), 0.0);
        assert_eq!(a.bus_volume(Bus::Ui), 1.0);
    }

    #[test]
    fn suspend_keeps_voices() {
        let mut a = NullAudio::new();
        let s = sound(&mut a, "x.wav");
        let v = a.play(s, &PlayParams::sfx().looped()).unwrap();
        a.set_suspended(true);
        assert!(a.is_suspended() && a.is_playing(v));
        a.set_suspended(false);
        assert!(!a.is_suspended() && a.is_playing(v));
    }

    #[test]
    fn usable_as_trait_object() {
        let mut boxed: Box<dyn Audio> = Box::new(NullAudio::new());
        let s = boxed.resources().sound("a.wav");
        assert!(boxed.play(s, &PlayParams::sfx()).is_ok());
    }
}
