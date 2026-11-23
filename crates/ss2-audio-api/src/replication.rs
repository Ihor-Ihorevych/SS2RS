//! **Replicated audio events** for co-op (roadmap T06-015, `docs/AUDIO_REPLICATION.md`): who hears what, one-shots against loops, and what a late joiner gets.
//!
//! Device-free and transport-free. The server decides which sounds reach which client ([`AudioReplicator`]), the events travel as the small messages of
//! [`AudioEvent`] (with a compact, validated byte form), and each client turns them into [`Audio`] calls ([`AudioEventPlayer`]). Nothing here sends a packet.
//!
//! * A **one-shot** (a gunshot, a pickup) is fire-and-forget: it goes to the clients **in earshot** at that moment and is **never replayed** to anyone who joins later.
//! * A **loop** (an engine, a torch, a hum) is **state**: the server remembers it, tells a client to start it when the client comes into earshot, to stop it when the
//!   client leaves (with hysteresis, so walking along the edge does not flicker) or the loop ends, and tells a **late joiner** about every loop in earshot at once.
//! * A [`Scope`] restricts a sound to everyone, to its owner only (a 2D sound of the player's own, a damage cue), or to everyone **except** its owner (the owner played it
//!   at once, by prediction, and must not hear it twice).
//!
//! Sounds are named on the wire by a [`SoundRef`], a number both sides agree on (an index into a table built from the level's sound list), not by [`SoundId`], which
//! is only meaningful to the audio system that issued it.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ss2_core::math::Vec3;
use ss2_core::{Error, Result};

use crate::api::Audio;
use crate::types::{Bus, PlayParams, Priority, SoundId, Spatial, VoiceId};

/// A client of the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(pub u32);

/// Names a looping emitter for as long as it exists (an entity id and a slot, say).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmitterId(pub u64);

/// A sound as named on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoundRef(pub u32);

/// Who may hear a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Everyone in earshot.
    All,
    /// Only this client.
    Owner(ClientId),
    /// Everyone in earshot except this client (who played it locally already).
    AllExceptOwner(ClientId),
}

impl Scope {
    fn includes(self, client: ClientId) -> bool {
        match self {
            Scope::All => true,
            Scope::Owner(o) => o == client,
            Scope::AllExceptOwner(o) => o != client,
        }
    }
}

/// A message from the server to a client about a sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioEvent {
    /// Play once and forget.
    OneShot {
        /// The sound.
        sound: SoundRef,
        /// How to play it (its `looped` flag is false).
        params: PlayParams,
    },
    /// Start a loop that lasts until a [`LoopStop`](Self::LoopStop) for the same emitter.
    LoopStart {
        /// The emitter.
        emitter: EmitterId,
        /// The sound.
        sound: SoundRef,
        /// How to play it (its `looped` flag is true).
        params: PlayParams,
    },
    /// End a loop.
    LoopStop {
        /// The emitter.
        emitter: EmitterId,
        /// Fade out over this many milliseconds.
        fade_ms: u16,
    },
}

const TAG_ONE_SHOT: u8 = 1;
const TAG_LOOP_START: u8 = 2;
const TAG_LOOP_STOP: u8 = 3;
/// The largest volume a wire byte can express (the byte is a share of this).
const MAX_WIRE_VOLUME: f32 = 4.0;

fn quantize(v: f32, lo: f32, hi: f32) -> u8 {
    (((v.clamp(lo, hi) - lo) / (hi - lo)) * 255.0).round() as u8
}

fn dequantize(b: u8, lo: f32, hi: f32) -> f32 {
    lo + f32::from(b) / 255.0 * (hi - lo)
}

fn bus_index(b: Bus) -> u8 {
    Bus::ALL.iter().position(|x| *x == b).map_or(0, |i| i as u8)
}

fn push_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn encode_params(out: &mut Vec<u8>, p: &PlayParams) {
    out.push(bus_index(p.bus));
    out.push(p.priority.0);
    out.push(quantize(p.volume, 0.0, MAX_WIRE_VOLUME));
    out.push(quantize(p.pitch, 0.1, 4.0));
    match &p.spatial {
        None => out.push(0),
        Some(s) => {
            out.push(1);
            for v in [
                s.position.x,
                s.position.y,
                s.position.z,
                s.min_distance,
                s.max_distance,
            ] {
                push_f32(out, v);
            }
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.bytes.len() < n {
            return Err(Error::invalid(
                "audio event",
                "the message ends early".to_owned(),
            ));
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("two bytes"),
        ))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn params(&mut self, looped: bool) -> Result<PlayParams> {
        let bus = *Bus::ALL
            .get(usize::from(self.u8()?))
            .ok_or_else(|| Error::invalid("audio event", "unknown bus".to_owned()))?;
        let priority = Priority(self.u8()?);
        let volume = dequantize(self.u8()?, 0.0, MAX_WIRE_VOLUME);
        let pitch = dequantize(self.u8()?, 0.1, 4.0);
        let spatial = match self.u8()? {
            0 => None,
            1 => {
                let position = Vec3::new(self.f32()?, self.f32()?, self.f32()?);
                let (min_distance, max_distance) = (self.f32()?, self.f32()?);
                Some(Spatial {
                    position,
                    velocity: Vec3::ZERO,
                    min_distance,
                    max_distance,
                })
            }
            _ => {
                return Err(Error::invalid(
                    "audio event",
                    "unknown placement flag".to_owned(),
                ));
            }
        };
        let params = PlayParams {
            volume,
            pitch,
            bus,
            priority,
            looped,
            spatial,
        };
        params.validate()?;
        Ok(params)
    }

    fn finish(self) -> Result<()> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(Error::invalid(
                "audio event",
                format!("{} bytes left over", self.bytes.len()),
            ))
        }
    }
}

impl AudioEvent {
    /// Append the wire form: a tag, then little-endian fields. Volume is stored in a byte (0 to 4, step 0.016), pitch in a byte (0.1 to 4, step 0.015), and a placement's
    /// velocity is **not sent** (the receiver uses zero).
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            AudioEvent::OneShot { sound, params } => {
                out.push(TAG_ONE_SHOT);
                out.extend_from_slice(&sound.0.to_le_bytes());
                encode_params(out, params);
            }
            AudioEvent::LoopStart {
                emitter,
                sound,
                params,
            } => {
                out.push(TAG_LOOP_START);
                out.extend_from_slice(&emitter.0.to_le_bytes());
                out.extend_from_slice(&sound.0.to_le_bytes());
                encode_params(out, params);
            }
            AudioEvent::LoopStop { emitter, fade_ms } => {
                out.push(TAG_LOOP_STOP);
                out.extend_from_slice(&emitter.0.to_le_bytes());
                out.extend_from_slice(&fade_ms.to_le_bytes());
            }
        }
    }

    /// Read one event, which must fill `bytes` exactly.
    ///
    /// # Errors
    /// [`Error::Invalid`] for an unknown tag, a short or over-long message, an unknown bus or flag, or parameters that fail [`PlayParams::validate`] (NaN, a bad range).
    /// Never panics on any input.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader { bytes };
        let event = match r.u8()? {
            TAG_ONE_SHOT => {
                let sound = SoundRef(r.u32()?);
                AudioEvent::OneShot {
                    sound,
                    params: r.params(false)?,
                }
            }
            TAG_LOOP_START => {
                let emitter = EmitterId(r.u64()?);
                let sound = SoundRef(r.u32()?);
                AudioEvent::LoopStart {
                    emitter,
                    sound,
                    params: r.params(true)?,
                }
            }
            TAG_LOOP_STOP => AudioEvent::LoopStop {
                emitter: EmitterId(r.u64()?),
                fade_ms: r.u16()?,
            },
            tag => return Err(Error::invalid("audio event", format!("unknown tag {tag}"))),
        };
        r.finish()?;
        Ok(event)
    }
}

/// How much further than its maximum distance a loop must be before a client is told to stop it: it starts at `max_distance` and stops at 1.25 times that, so a listener
/// walking along the edge does not make it start and stop every step.
pub const LEAVE_FACTOR: f32 = 1.25;

#[derive(Clone, Debug)]
struct LoopState {
    sound: SoundRef,
    params: PlayParams,
    scope: Scope,
}

/// The server's side: remembers the loops and decides who hears what.
///
/// ```
/// use ss2_audio_api::replication::{AudioReplicator, ClientId, EmitterId, Scope, SoundRef};
/// use ss2_audio_api::PlayParams;
/// use ss2_core::math::Vec3;
///
/// let mut server = AudioReplicator::new();
/// // A hum at the origin with the default range (60 units).
/// server.start_loop(EmitterId(1), SoundRef(7), PlayParams::sfx().at(Vec3::ZERO).looped(), Scope::All);
/// let near = ClientId(1);
/// let far = ClientId(2);
/// let events = server.update(&[(near, Vec3::new(5.0, 0.0, 0.0)), (far, Vec3::new(500.0, 0.0, 0.0))]);
/// assert_eq!(events.len(), 1);          // only the near client is told
/// assert_eq!(events[0].0, near);
/// ```
#[derive(Clone, Debug, Default)]
pub struct AudioReplicator {
    loops: BTreeMap<EmitterId, LoopState>,
    /// For each client, the loops it has been told to start and not yet to stop.
    known: BTreeMap<ClientId, BTreeMap<EmitterId, ()>>,
    /// Loops ended since the last update, with their fade, for the clients that know them.
    ended: Vec<(EmitterId, u16)>,
}

fn distance(a: Vec3, b: Vec3) -> f32 {
    (a - b).length()
}

impl AudioReplicator {
    /// A replicator with no loops.
    pub fn new() -> Self {
        Self::default()
    }

    /// Who should get a one-shot: the clients in `scope` whose listener is within the sound's maximum distance (or all of them in scope for a 2D sound), in client order.
    /// The caller sends each of them an [`AudioEvent::OneShot`].
    pub fn route_one_shot(
        &self,
        params: &PlayParams,
        scope: Scope,
        listeners: &[(ClientId, Vec3)],
    ) -> Vec<ClientId> {
        let mut out: Vec<ClientId> = listeners
            .iter()
            .filter(|(c, pos)| {
                scope.includes(*c)
                    && params
                        .spatial
                        .as_ref()
                        .is_none_or(|s| distance(s.position, *pos) <= s.max_distance)
            })
            .map(|(c, _)| *c)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// A loop begins (or is replaced, if the emitter already exists). `params.looped` is forced on. Clients are told at the next [`update`](Self::update).
    pub fn start_loop(
        &mut self,
        emitter: EmitterId,
        sound: SoundRef,
        params: PlayParams,
        scope: Scope,
    ) {
        let params = PlayParams {
            looped: true,
            ..params
        };
        // A replaced loop is stopped everywhere first, so a client hears the new one start, not a changed old one.
        if self.loops.contains_key(&emitter) {
            self.ended.push((emitter, 0));
        }
        self.loops.insert(
            emitter,
            LoopState {
                sound,
                params,
                scope,
            },
        );
    }

    /// Move a loop's emitter (an engine on a moving thing). Clients keep the position they were told at; it is sent to **new** listeners as it is now.
    pub fn move_loop(&mut self, emitter: EmitterId, position: Vec3) {
        if !position.is_finite() {
            return;
        }
        if let Some(s) = self
            .loops
            .get_mut(&emitter)
            .and_then(|l| l.params.spatial.as_mut())
        {
            s.position = position;
        }
    }

    /// A loop ends. Every client that knows it is told to stop at the next [`update`](Self::update), fading out over `fade_ms`. Ending an unknown emitter does nothing.
    pub fn stop_loop(&mut self, emitter: EmitterId, fade_ms: u16) {
        if self.loops.remove(&emitter).is_some() {
            self.ended.push((emitter, fade_ms));
        }
    }

    /// How many loops are active.
    pub fn loop_count(&self) -> usize {
        self.loops.len()
    }

    /// The loops a client has been told about and not told to stop.
    pub fn known_by(&self, client: ClientId) -> Vec<EmitterId> {
        self.known
            .get(&client)
            .map(|m| m.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Bring every client up to date with `listeners` (each connected client and where it listens) and return the events to send, ordered by client and then emitter:
    ///
    /// * **ended loops** are stopped for the clients that know them;
    /// * a loop **in range** (within `max_distance`, or any distance for a 2D loop) for a client in its scope that does not know it yet is **started**: this is how a **late
    ///   joiner**, or someone walking up to it, learns of it;
    /// * a loop a client knows that is now **out of range** (beyond [`LEAVE_FACTOR`] times its `max_distance`) is stopped for that client without fade.
    ///
    /// A client missing from `listeners` is forgotten (it disconnected); if it comes back it is a new client.
    pub fn update(&mut self, listeners: &[(ClientId, Vec3)]) -> Vec<(ClientId, AudioEvent)> {
        let mut events: Vec<(ClientId, AudioEvent)> = Vec::new();
        let connected: BTreeSet<ClientId> = listeners.iter().map(|(c, _)| *c).collect();
        self.known.retain(|c, _| connected.contains(c));
        let positions: BTreeMap<ClientId, Vec3> = listeners.iter().copied().collect();

        // Loops that ended since the last update.
        for (emitter, fade_ms) in std::mem::take(&mut self.ended) {
            for (client, known) in &mut self.known {
                if known.remove(&emitter).is_some() {
                    events.push((*client, AudioEvent::LoopStop { emitter, fade_ms }));
                }
            }
        }
        for (client, pos) in &positions {
            let known = self.known.entry(*client).or_default();
            // Out of range or out of scope: stop.
            let leaving: Vec<EmitterId> = known
                .keys()
                .copied()
                .filter(|e| match self.loops.get(e) {
                    None => true,
                    Some(l) => {
                        !l.scope.includes(*client)
                            || l.params.spatial.as_ref().is_some_and(|s| {
                                distance(s.position, *pos) > s.max_distance * LEAVE_FACTOR
                            })
                    }
                })
                .collect();
            for e in leaving {
                known.remove(&e);
                events.push((
                    *client,
                    AudioEvent::LoopStop {
                        emitter: e,
                        fade_ms: 0,
                    },
                ));
            }
            // In range and in scope: start.
            for (emitter, l) in &self.loops {
                if known.contains_key(emitter) || !l.scope.includes(*client) {
                    continue;
                }
                let in_range = l
                    .params
                    .spatial
                    .as_ref()
                    .is_none_or(|s| distance(s.position, *pos) <= s.max_distance);
                if in_range {
                    known.insert(*emitter, ());
                    events.push((
                        *client,
                        AudioEvent::LoopStart {
                            emitter: *emitter,
                            sound: l.sound,
                            params: l.params,
                        },
                    ));
                }
            }
        }
        events.sort_by_key(|(c, e)| {
            let (kind, emitter) = match e {
                AudioEvent::LoopStop { emitter, .. } => (0u8, *emitter),
                AudioEvent::LoopStart { emitter, .. } => (1, *emitter),
                AudioEvent::OneShot { .. } => (2, EmitterId(0)),
            };
            (*c, kind, emitter)
        });
        events
    }
}

/// The client's side: turns events into [`Audio`] calls and remembers which voice plays each loop.
///
/// ```
/// use ss2_audio_api::replication::{AudioEvent, AudioEventPlayer, EmitterId, SoundRef};
/// use ss2_audio_api::{Audio, AudioResources, NullAudio, PlayParams};
///
/// let mut audio = NullAudio::new();
/// let hum = audio.resources().sound("sounds/hum.wav");
/// let mut player = AudioEventPlayer::new(vec![hum]);          // SoundRef(0) is the hum
/// player.apply(&mut audio, &AudioEvent::LoopStart { emitter: EmitterId(1), sound: SoundRef(0), params: PlayParams::sfx().looped() });
/// assert_eq!(audio.playing_count(), 1);
/// player.apply(&mut audio, &AudioEvent::LoopStop { emitter: EmitterId(1), fade_ms: 0 });
/// assert_eq!(audio.playing_count(), 0);
/// ```
#[derive(Clone, Debug)]
pub struct AudioEventPlayer {
    sounds: Vec<SoundId>,
    loops: BTreeMap<EmitterId, VoiceId>,
    /// Events naming a sound that is not in the table, or whose parameters the audio refused.
    pub dropped: u64,
}

impl AudioEventPlayer {
    /// A player whose `SoundRef(n)` is `sounds[n]`.
    pub fn new(sounds: Vec<SoundId>) -> Self {
        Self {
            sounds,
            loops: BTreeMap::new(),
            dropped: 0,
        }
    }

    /// How many loops are playing.
    pub fn loop_count(&self) -> usize {
        self.loops.len()
    }

    /// Do what the event says. A duplicate `LoopStart` for an emitter already playing is **ignored** (it may have been sent twice), a `LoopStop` for an unknown emitter is
    /// ignored, and a sound outside the table or parameters the audio refuses are counted in [`dropped`](Self::dropped) and skipped: a bad message never panics and
    /// never stops the others.
    pub fn apply(&mut self, audio: &mut dyn Audio, event: &AudioEvent) {
        match event {
            AudioEvent::OneShot { sound, params } => {
                let Some(id) = self.sounds.get(sound.0 as usize).copied() else {
                    self.dropped += 1;
                    return;
                };
                if audio
                    .play(
                        id,
                        &PlayParams {
                            looped: false,
                            ..*params
                        },
                    )
                    .is_err()
                {
                    self.dropped += 1;
                }
            }
            AudioEvent::LoopStart {
                emitter,
                sound,
                params,
            } => {
                if self.loops.contains_key(emitter) {
                    return;
                }
                let Some(id) = self.sounds.get(sound.0 as usize).copied() else {
                    self.dropped += 1;
                    return;
                };
                match audio.play(
                    id,
                    &PlayParams {
                        looped: true,
                        ..*params
                    },
                ) {
                    Ok(voice) => {
                        self.loops.insert(*emitter, voice);
                    }
                    Err(_) => self.dropped += 1,
                }
            }
            AudioEvent::LoopStop { emitter, fade_ms } => {
                if let Some(voice) = self.loops.remove(emitter) {
                    audio.stop(voice, Duration::from_millis(u64::from(*fade_ms)));
                }
            }
        }
    }

    /// Stop every loop at once (a level change, a disconnect).
    pub fn clear(&mut self, audio: &mut dyn Audio) {
        for (_, voice) in std::mem::take(&mut self.loops) {
            audio.stop(voice, Duration::ZERO);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::null::NullAudio;
    use crate::types::VoiceId;
    use ss2_core::Result;

    fn at(x: f32) -> PlayParams {
        PlayParams::sfx().at(Vec3::new(x, 0.0, 0.0))
    }

    fn client(n: u32) -> ClientId {
        ClientId(n)
    }

    // ---- who hears a one-shot -------------------------------------------------------------------------

    #[test]
    fn a_one_shot_reaches_the_clients_in_earshot_and_in_scope() {
        let server = AudioReplicator::new();
        let listeners = [
            (client(3), Vec3::new(100.0, 0.0, 0.0)),
            (client(1), Vec3::new(10.0, 0.0, 0.0)),
            (client(2), Vec3::new(59.0, 0.0, 0.0)),
        ];
        // The default range is 60: clients 1 and 2 are in earshot of a sound at the origin; 3 is not. The result is in client order.
        assert_eq!(
            server.route_one_shot(&at(0.0), Scope::All, &listeners),
            [client(1), client(2)]
        );
        assert_eq!(
            server.route_one_shot(&at(0.0), Scope::Owner(client(2)), &listeners),
            [client(2)]
        );
        assert_eq!(
            server.route_one_shot(&at(0.0), Scope::AllExceptOwner(client(1)), &listeners),
            [client(2)]
        );
        // An owner out of earshot does not hear their own spatial sound either.
        assert!(
            server
                .route_one_shot(&at(0.0), Scope::Owner(client(3)), &listeners)
                .is_empty()
        );
    }

    #[test]
    fn a_2d_sound_has_no_range_and_the_range_edge_is_inclusive() {
        let server = AudioReplicator::new();
        let far = [(client(1), Vec3::new(1.0e6, 0.0, 0.0))];
        assert_eq!(
            server.route_one_shot(&PlayParams::ui(), Scope::All, &far),
            [client(1)]
        );
        let edge = [
            (client(1), Vec3::new(60.0, 0.0, 0.0)),
            (client(2), Vec3::new(60.01, 0.0, 0.0)),
        ];
        assert_eq!(
            server.route_one_shot(&at(0.0), Scope::All, &edge),
            [client(1)],
            "exactly at max_distance hears it"
        );
        let dup = [(client(1), Vec3::ZERO), (client(1), Vec3::ZERO)];
        assert_eq!(
            server.route_one_shot(&at(0.0), Scope::All, &dup),
            [client(1)],
            "a client listed twice gets one copy"
        );
        assert!(server.route_one_shot(&at(0.0), Scope::All, &[]).is_empty());
    }

    // ---- loops ----------------------------------------------------------------------------------------------

    #[test]
    fn a_loop_is_started_for_clients_in_range_and_not_repeated() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(9), at(0.0), Scope::All);
        let l = [
            (client(1), Vec3::new(5.0, 0.0, 0.0)),
            (client(2), Vec3::new(500.0, 0.0, 0.0)),
        ];
        let first = s.update(&l);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, client(1));
        assert!(
            matches!(first[0].1, AudioEvent::LoopStart { emitter: EmitterId(1), sound: SoundRef(9), params } if params.looped)
        );
        assert!(s.update(&l).is_empty(), "nothing to say twice");
        assert_eq!(
            (s.known_by(client(1)), s.known_by(client(2)), s.loop_count()),
            (vec![EmitterId(1)], vec![], 1)
        );
    }

    #[test]
    fn walking_into_and_out_of_earshot_starts_and_stops_the_loop_with_hysteresis() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::All);
        let x = |d: f32| [(client(1), Vec3::new(d, 0.0, 0.0))];
        assert!(s.update(&x(100.0)).is_empty(), "out of range");
        assert!(
            matches!(s.update(&x(59.0))[0].1, AudioEvent::LoopStart { .. }),
            "in range: start"
        );
        assert!(
            s.update(&x(70.0)).is_empty(),
            "between 60 and 75: it keeps playing (hysteresis)"
        );
        assert!(s.update(&x(75.0)).is_empty(), "exactly 1.25 x 60: still on");
        let leave = s.update(&x(75.5));
        assert!(
            matches!(
                leave[0].1,
                AudioEvent::LoopStop {
                    emitter: EmitterId(1),
                    fade_ms: 0
                }
            ),
            "{leave:?}"
        );
        assert!(
            s.update(&x(70.0)).is_empty(),
            "70 is out of 60: not restarted until it is back inside the range"
        );
        assert!(matches!(
            s.update(&x(60.0))[0].1,
            AudioEvent::LoopStart { .. }
        ));
    }

    #[test]
    fn ending_a_loop_tells_only_the_clients_that_know_it_with_the_fade() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::All);
        let l = [
            (client(1), Vec3::ZERO),
            (client(2), Vec3::new(900.0, 0.0, 0.0)),
        ];
        s.update(&l);
        s.stop_loop(EmitterId(1), 250);
        s.stop_loop(EmitterId(99), 10); // unknown: nothing
        let events = s.update(&l);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            (
                client(1),
                AudioEvent::LoopStop {
                    emitter: EmitterId(1),
                    fade_ms: 250
                }
            )
        );
        assert_eq!((s.loop_count(), s.known_by(client(1)).len()), (0, 0));
        assert!(s.update(&l).is_empty());
    }

    #[test]
    fn a_late_joiner_gets_every_loop_in_earshot_at_once_and_no_one_shots() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::All);
        s.start_loop(EmitterId(2), SoundRef(2), at(10.0), Scope::All);
        s.start_loop(EmitterId(3), SoundRef(3), at(1000.0), Scope::All);
        s.start_loop(
            EmitterId(4),
            SoundRef(4),
            PlayParams::sfx().looped(),
            Scope::All,
        ); // 2D: heard anywhere
        s.update(&[(client(1), Vec3::ZERO)]);
        // A second client joins at the origin: it learns of 1, 2 and the 2D loop 4 (not the far 3), in emitter order.
        let events = s.update(&[(client(1), Vec3::ZERO), (client(2), Vec3::ZERO)]);
        let started: Vec<(ClientId, u64)> = events
            .iter()
            .map(|(c, e)| {
                (
                    *c,
                    if let AudioEvent::LoopStart { emitter, .. } = e {
                        emitter.0
                    } else {
                        0
                    },
                )
            })
            .collect();
        assert_eq!(started, [(client(2), 1), (client(2), 2), (client(2), 4)]);
        assert!(
            events
                .iter()
                .all(|(_, e)| !matches!(e, AudioEvent::OneShot { .. })),
            "one-shots are never replayed"
        );
    }

    #[test]
    fn scope_applies_to_loops_too_and_a_disconnected_client_is_forgotten() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::Owner(client(1)));
        s.start_loop(
            EmitterId(2),
            SoundRef(2),
            at(0.0),
            Scope::AllExceptOwner(client(1)),
        );
        let l = [(client(1), Vec3::ZERO), (client(2), Vec3::ZERO)];
        let events = s.update(&l);
        let pairs: Vec<(u32, u64)> = events
            .iter()
            .map(|(c, e)| {
                (
                    c.0,
                    if let AudioEvent::LoopStart { emitter, .. } = e {
                        emitter.0
                    } else {
                        0
                    },
                )
            })
            .collect();
        assert_eq!(pairs, [(1, 1), (2, 2)]);
        // Client 2 drops out and comes back: it is new and hears the loop again.
        assert!(s.update(&[(client(1), Vec3::ZERO)]).is_empty());
        assert_eq!(s.known_by(client(2)), []);
        let back = s.update(&l);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].0, client(2));
    }

    #[test]
    fn replacing_a_loop_stops_the_old_one_then_starts_the_new() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::All);
        let l = [(client(1), Vec3::ZERO)];
        s.update(&l);
        s.start_loop(EmitterId(1), SoundRef(2), at(0.0), Scope::All);
        let events = s.update(&l);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0].1,
            AudioEvent::LoopStop {
                emitter: EmitterId(1),
                fade_ms: 0
            }
        ));
        assert!(matches!(
            events[1].1,
            AudioEvent::LoopStart {
                sound: SoundRef(2),
                ..
            }
        ));
    }

    #[test]
    fn a_moving_emitter_is_sent_to_new_listeners_where_it_is_now() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::All);
        s.move_loop(EmitterId(1), Vec3::new(40.0, 0.0, 0.0));
        s.move_loop(EmitterId(1), Vec3::new(f32::NAN, 0.0, 0.0)); // ignored
        s.move_loop(EmitterId(77), Vec3::ZERO); // unknown: ignored
        let events = s.update(&[(client(1), Vec3::ZERO)]);
        let AudioEvent::LoopStart { params, .. } = events[0].1 else {
            panic!("{events:?}")
        };
        assert_eq!(params.spatial.unwrap().position, Vec3::new(40.0, 0.0, 0.0));
    }

    #[test]
    fn a_loop_forced_to_loop_even_if_the_params_said_otherwise() {
        let mut s = AudioReplicator::new();
        s.start_loop(EmitterId(1), SoundRef(1), at(0.0), Scope::All);
        let events = s.update(&[(client(1), Vec3::ZERO)]);
        assert!(matches!(events[0].1, AudioEvent::LoopStart { params, .. } if params.looped));
    }

    // ---- the wire form ------------------------------------------------------------------------------------------

    fn round(e: AudioEvent) -> AudioEvent {
        let mut bytes = Vec::new();
        e.encode(&mut bytes);
        AudioEvent::decode(&bytes).unwrap_or_else(|err| panic!("{e:?}: {err}"))
    }

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn events_survive_the_wire_within_the_quantization() {
        let params = PlayParams {
            volume: 1.5,
            pitch: 1.25,
            bus: Bus::Voice,
            priority: Priority::HIGH,
            looped: false,
            spatial: Some(Spatial {
                position: Vec3::new(1.5, -2.25, 30.0),
                velocity: Vec3::ZERO,
                min_distance: 3.0,
                max_distance: 80.0,
            }),
        };
        let AudioEvent::OneShot { sound, params: got } = round(AudioEvent::OneShot {
            sound: SoundRef(123_456),
            params,
        }) else {
            panic!()
        };
        assert_eq!(sound, SoundRef(123_456));
        assert!(
            close(got.volume, 1.5, 0.008) && close(got.pitch, 1.25, 0.008),
            "{got:?}"
        );
        assert_eq!(
            (got.bus, got.priority, got.looped),
            (Bus::Voice, Priority::HIGH, false)
        );
        assert_eq!(got.spatial, params.spatial);
        let looped = PlayParams {
            looped: true,
            spatial: None,
            ..params
        };
        let AudioEvent::LoopStart {
            emitter,
            sound,
            params: got,
        } = round(AudioEvent::LoopStart {
            emitter: EmitterId(u64::MAX),
            sound: SoundRef(u32::MAX),
            params: looped,
        })
        else {
            panic!()
        };
        assert_eq!(
            (emitter, sound, got.looped, got.spatial),
            (EmitterId(u64::MAX), SoundRef(u32::MAX), true, None)
        );
        assert_eq!(
            round(AudioEvent::LoopStop {
                emitter: EmitterId(5),
                fade_ms: 65_535
            }),
            AudioEvent::LoopStop {
                emitter: EmitterId(5),
                fade_ms: 65_535
            }
        );
    }

    #[test]
    fn every_bus_and_the_edges_of_volume_and_pitch_round_trip() {
        for bus in Bus::ALL {
            let AudioEvent::OneShot { params, .. } = round(AudioEvent::OneShot {
                sound: SoundRef(0),
                params: PlayParams::sfx().on_bus(bus),
            }) else {
                panic!()
            };
            assert_eq!(params.bus, bus);
        }
        let loud = PlayParams {
            volume: 4.0,
            pitch: 4.0,
            ..PlayParams::sfx()
        };
        let AudioEvent::OneShot { params, .. } = round(AudioEvent::OneShot {
            sound: SoundRef(0),
            params: loud,
        }) else {
            panic!()
        };
        assert!(close(params.volume, 4.0, 1e-5) && close(params.pitch, 4.0, 1e-5));
        let quiet = PlayParams {
            volume: 0.0,
            pitch: 0.1,
            ..PlayParams::sfx()
        };
        let AudioEvent::OneShot { params, .. } = round(AudioEvent::OneShot {
            sound: SoundRef(0),
            params: quiet,
        }) else {
            panic!()
        };
        assert!(close(params.volume, 0.0, 1e-6) && close(params.pitch, 0.1, 1e-6));
        // Values beyond the wire's range are clamped, not wrapped.
        let over = PlayParams {
            volume: 100.0,
            ..PlayParams::sfx()
        };
        let AudioEvent::OneShot { params, .. } = round(AudioEvent::OneShot {
            sound: SoundRef(0),
            params: over,
        }) else {
            panic!()
        };
        assert!(close(params.volume, 4.0, 1e-5));
    }

    #[test]
    fn bad_messages_are_refused_without_panicking() {
        let mut ok = Vec::new();
        AudioEvent::OneShot {
            sound: SoundRef(1),
            params: at(5.0),
        }
        .encode(&mut ok);
        assert!(AudioEvent::decode(&ok).is_ok());
        assert!(AudioEvent::decode(&[]).is_err());
        assert!(AudioEvent::decode(&[0]).is_err(), "tag 0");
        assert!(AudioEvent::decode(&[99, 1, 2, 3]).is_err(), "unknown tag");
        for cut in 0..ok.len() {
            assert!(
                AudioEvent::decode(&ok[..cut]).is_err(),
                "truncated at {cut}"
            );
        }
        let mut extra = ok.clone();
        extra.push(0);
        assert!(AudioEvent::decode(&extra).is_err(), "trailing byte");
        let mut bad_bus = ok.clone();
        bad_bus[5] = 200;
        assert!(AudioEvent::decode(&bad_bus).is_err(), "bus 200");
        let mut bad_flag = ok.clone();
        bad_flag[9] = 7;
        assert!(AudioEvent::decode(&bad_flag).is_err(), "placement flag 7");
        let mut nan = ok.clone();
        nan[10..14].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(AudioEvent::decode(&nan).is_err(), "NaN position");
        let mut inverted = ok.clone();
        let len = inverted.len();
        inverted[len - 8..len - 4].copy_from_slice(&100.0f32.to_le_bytes()); // min 100
        inverted[len - 4..].copy_from_slice(&1.0f32.to_le_bytes()); // max 1
        assert!(AudioEvent::decode(&inverted).is_err(), "min above max");
    }

    #[test]
    fn decoding_arbitrary_bytes_never_panics() {
        let mut seed = 0x0DDB_1A5E_5BAD_5EEDu64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..20_000 {
            let len = (rnd() % 40) as usize;
            let bytes: Vec<u8> = (0..len).map(|_| rnd() as u8).collect();
            if let Ok(event) = AudioEvent::decode(&bytes) {
                // Whatever decodes is valid parameters and re-encodes to something that decodes again.
                round(event);
            }
        }
    }

    // ---- the client ---------------------------------------------------------------------------------------------------

    fn player() -> (NullAudio, AudioEventPlayer, [SoundId; 2]) {
        let mut audio = NullAudio::new();
        let a = audio.resources().sound("sounds/a.wav");
        let b = audio.resources().sound("sounds/b.wav");
        (audio, AudioEventPlayer::new(vec![a, b]), [a, b])
    }

    #[test]
    fn one_shots_play_and_loops_play_until_stopped() {
        let (mut audio, mut p, sounds) = player();
        p.apply(
            &mut audio,
            &AudioEvent::OneShot {
                sound: SoundRef(1),
                params: at(1.0),
            },
        );
        assert_eq!(audio.playing_count(), 1);
        assert!(
            matches!(audio.calls()[0], crate::null::AudioCall::Play { sound, .. } if sound == sounds[1])
        );
        assert_eq!(p.loop_count(), 0, "a one-shot is not tracked");
        p.apply(
            &mut audio,
            &AudioEvent::LoopStart {
                emitter: EmitterId(7),
                sound: SoundRef(0),
                params: PlayParams::sfx(),
            },
        );
        assert_eq!((p.loop_count(), audio.playing_count()), (1, 2));
        p.apply(
            &mut audio,
            &AudioEvent::LoopStop {
                emitter: EmitterId(7),
                fade_ms: 100,
            },
        );
        assert_eq!((p.loop_count(), audio.playing_count()), (0, 1));
    }

    #[test]
    fn a_repeated_loop_start_and_a_stop_of_nothing_are_harmless() {
        let (mut audio, mut p, _) = player();
        let start = AudioEvent::LoopStart {
            emitter: EmitterId(1),
            sound: SoundRef(0),
            params: PlayParams::sfx(),
        };
        p.apply(&mut audio, &start);
        p.apply(&mut audio, &start);
        assert_eq!(
            (p.loop_count(), audio.playing_count()),
            (1, 1),
            "the duplicate did not start a second voice"
        );
        p.apply(
            &mut audio,
            &AudioEvent::LoopStop {
                emitter: EmitterId(2),
                fade_ms: 0,
            },
        );
        assert_eq!(audio.playing_count(), 1);
        p.apply(
            &mut audio,
            &AudioEvent::LoopStop {
                emitter: EmitterId(1),
                fade_ms: 0,
            },
        );
        p.apply(
            &mut audio,
            &AudioEvent::LoopStop {
                emitter: EmitterId(1),
                fade_ms: 0,
            },
        );
        assert_eq!(audio.playing_count(), 0);
        // After a stop the same emitter can start again.
        p.apply(&mut audio, &start);
        assert_eq!(audio.playing_count(), 1);
    }

    #[test]
    fn unknown_sounds_and_refused_parameters_are_counted_and_skipped() {
        let (mut audio, mut p, _) = player();
        p.apply(
            &mut audio,
            &AudioEvent::OneShot {
                sound: SoundRef(99),
                params: at(0.0),
            },
        );
        p.apply(
            &mut audio,
            &AudioEvent::LoopStart {
                emitter: EmitterId(1),
                sound: SoundRef(99),
                params: at(0.0),
            },
        );
        let bad = PlayParams {
            volume: f32::NAN,
            ..PlayParams::sfx()
        };
        p.apply(
            &mut audio,
            &AudioEvent::OneShot {
                sound: SoundRef(0),
                params: bad,
            },
        );
        p.apply(
            &mut audio,
            &AudioEvent::LoopStart {
                emitter: EmitterId(2),
                sound: SoundRef(0),
                params: bad,
            },
        );
        assert_eq!(
            (p.dropped, p.loop_count(), audio.playing_count()),
            (4, 0, 0)
        );
        p.apply(
            &mut audio,
            &AudioEvent::OneShot {
                sound: SoundRef(0),
                params: at(0.0),
            },
        );
        assert_eq!(audio.playing_count(), 1, "later events still work");
    }

    #[test]
    fn clear_stops_every_loop_and_the_flags_follow_the_event_kind() {
        let (mut audio, mut p, _) = player();
        for i in 0..3 {
            p.apply(
                &mut audio,
                &AudioEvent::LoopStart {
                    emitter: EmitterId(i),
                    sound: SoundRef(0),
                    params: PlayParams::sfx(),
                },
            );
        }
        assert_eq!(audio.playing_count(), 3);
        p.clear(&mut audio);
        assert_eq!((p.loop_count(), audio.playing_count()), (0, 0));
        // A one-shot carrying looped = true is played once (the event kind decides), and a loop start carrying false still loops.
        p.apply(
            &mut audio,
            &AudioEvent::OneShot {
                sound: SoundRef(0),
                params: PlayParams::sfx().looped(),
            },
        );
        let voice = audio
            .calls()
            .iter()
            .rev()
            .find_map(|c| {
                if let crate::null::AudioCall::Play { voice, .. } = c {
                    Some(*voice)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(!audio.voice(voice).unwrap().1.looped);
        p.apply(
            &mut audio,
            &AudioEvent::LoopStart {
                emitter: EmitterId(9),
                sound: SoundRef(0),
                params: PlayParams::sfx(),
            },
        );
        let voice = audio
            .calls()
            .iter()
            .rev()
            .find_map(|c| {
                if let crate::null::AudioCall::Play { voice, .. } = c {
                    Some(*voice)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(audio.voice(voice).unwrap().1.looped);
    }

    /// An [`Audio`] that remembers the fade of every stop, delegating the rest to a [`NullAudio`].
    struct Recording {
        inner: NullAudio,
        fades: Vec<Duration>,
    }

    impl Audio for Recording {
        fn resources(&mut self) -> &mut dyn crate::api::AudioResources {
            self.inner.resources()
        }
        fn play(&mut self, sound: SoundId, params: &PlayParams) -> Result<VoiceId> {
            self.inner.play(sound, params)
        }
        fn stop(&mut self, voice: VoiceId, fade: Duration) {
            self.fades.push(fade);
            self.inner.stop(voice, fade);
        }
        fn is_playing(&self, voice: VoiceId) -> bool {
            self.inner.is_playing(voice)
        }
        fn update_voice(
            &mut self,
            voice: VoiceId,
            update: &crate::types::VoiceUpdate,
        ) -> Result<()> {
            self.inner.update_voice(voice, update)
        }
        fn set_listener(&mut self, listener: &crate::types::Listener) -> Result<()> {
            self.inner.set_listener(listener)
        }
        fn play_music(&mut self, track: SoundId, crossfade: Duration) {
            self.inner.play_music(track, crossfade);
        }
        fn stop_music(&mut self, fade: Duration) {
            self.inner.stop_music(fade);
        }
        fn current_music(&self) -> Option<SoundId> {
            self.inner.current_music()
        }
        fn set_bus_volume(&mut self, bus: Bus, volume: f32) {
            self.inner.set_bus_volume(bus, volume);
        }
        fn bus_volume(&self, bus: Bus) -> f32 {
            self.inner.bus_volume(bus)
        }
        fn set_suspended(&mut self, suspended: bool) {
            self.inner.set_suspended(suspended);
        }
    }

    #[test]
    fn a_loop_stop_fades_for_the_time_the_server_said_and_clear_cuts() {
        let mut audio = Recording {
            inner: NullAudio::new(),
            fades: Vec::new(),
        };
        let a = audio.resources().sound("sounds/a.wav");
        let mut p = AudioEventPlayer::new(vec![a]);
        for i in 1..=2 {
            p.apply(
                &mut audio,
                &AudioEvent::LoopStart {
                    emitter: EmitterId(i),
                    sound: SoundRef(0),
                    params: PlayParams::sfx(),
                },
            );
        }
        p.apply(
            &mut audio,
            &AudioEvent::LoopStop {
                emitter: EmitterId(1),
                fade_ms: 250,
            },
        );
        assert_eq!(audio.fades, [Duration::from_millis(250)]);
        p.clear(&mut audio);
        assert_eq!(
            audio.fades,
            [Duration::from_millis(250), Duration::ZERO],
            "clear cuts at once"
        );
    }

    #[test]
    fn a_loop_whose_params_said_not_to_loop_is_still_sent_as_a_loop() {
        let mut s = AudioReplicator::new();
        let not_looped = PlayParams {
            looped: false,
            ..at(0.0)
        };
        s.start_loop(EmitterId(1), SoundRef(1), not_looped, Scope::All);
        let events = s.update(&[(client(1), Vec3::ZERO)]);
        assert!(
            matches!(events[0].1, AudioEvent::LoopStart { params, .. } if params.looped),
            "{events:?}"
        );
    }

    // ---- server and client together ------------------------------------------------------------------------------------

    #[test]
    fn a_late_joiner_hears_the_loops_through_the_whole_pipeline() {
        let mut server = AudioReplicator::new();
        server.start_loop(EmitterId(1), SoundRef(0), at(0.0), Scope::All);
        server.start_loop(EmitterId(2), SoundRef(1), at(5.0), Scope::All);
        server.update(&[(client(1), Vec3::ZERO)]);
        // Client 2 joins: its events travel as bytes and are played.
        let (mut audio, mut p, _) = player();
        for (c, e) in server.update(&[(client(1), Vec3::ZERO), (client(2), Vec3::ZERO)]) {
            assert_eq!(c, client(2));
            let mut bytes = Vec::new();
            e.encode(&mut bytes);
            p.apply(&mut audio, &AudioEvent::decode(&bytes).unwrap());
        }
        assert_eq!((p.loop_count(), audio.playing_count()), (2, 2));
        server.stop_loop(EmitterId(1), 0);
        for (c, e) in server.update(&[(client(1), Vec3::ZERO), (client(2), Vec3::ZERO)]) {
            if c == client(2) {
                p.apply(&mut audio, &e);
            }
        }
        assert_eq!((p.loop_count(), audio.playing_count()), (1, 1));
    }
}
