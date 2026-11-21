//! The value types of the audio contract: ids, buses, voice parameters, the listener.

use std::num::NonZeroU32;

use ss2_core::math::Vec3;
use ss2_core::{Error, Result};

/// A sound the audio system can play: a `.wav`/`.ogg`, or a sound collection (`.scs`) that picks
/// a variant each time. Obtained from [`AudioResources::sound`](crate::AudioResources::sound);
/// meaningless outside the audio system that issued it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoundId(NonZeroU32);

impl SoundId {
    /// Wrap a raw id; `None` for `0`.
    pub fn from_raw(raw: u32) -> Option<Self> {
        NonZeroU32::new(raw).map(Self)
    }

    /// The raw id.
    pub fn raw(self) -> u32 {
        self.0.get()
    }
}

/// One playing (or finished, or culled) instance of a sound. Ids are never reused within a run,
/// so a stale `VoiceId` is harmless: operations on a finished voice do nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoiceId(pub u64);

/// A mixer bus. Every voice plays on exactly one; each bus has a user-facing volume
/// (T06-017), and `Master` scales all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Bus {
    /// Everything.
    Master,
    /// Music.
    Music,
    /// Sound effects (weapons, footsteps, ambience).
    Sfx,
    /// Voice-overs, taunts, voice chat.
    Voice,
    /// Menu and HUD sounds.
    Ui,
}

impl Bus {
    /// All buses, `Master` first.
    pub const ALL: [Bus; 5] = [Bus::Master, Bus::Music, Bus::Sfx, Bus::Voice, Bus::Ui];
}

/// How important a voice is when there are more sounds than the device (or mobile voice budget)
/// can mix: lower priority voices are culled first. Culled voices still get a [`VoiceId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Priority(pub u8);

impl Priority {
    /// Ambience and distant background detail.
    pub const LOW: Self = Self(64);
    /// Ordinary effects.
    pub const NORMAL: Self = Self(128);
    /// Own weapon, own footsteps, important enemies.
    pub const HIGH: Self = Self(192);
    /// Must never be culled (UI, voice-overs, music).
    pub const CRITICAL: Self = Self(255);
}

/// Where a voice is in the world. A voice without one is **2D** (UI, music, own-weapon sounds
/// that should not pan).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spatial {
    /// World position (engine space, see `ss2_core::math`).
    pub position: Vec3,
    /// World velocity in units/second, for Doppler.
    pub velocity: Vec3,
    /// Inside this distance the sound is at full volume.
    pub min_distance: f32,
    /// Beyond this distance the sound is silent (and may be culled). The curve between the two
    /// is the audio system's business (from the original game's behavior, T01-034).
    pub max_distance: f32,
}

impl Spatial {
    /// A stationary emitter at `position` with the default range (full volume within 2 units,
    /// silent beyond 60).
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            min_distance: 2.0,
            max_distance: 60.0,
        }
    }
}

/// How to start a voice.
///
/// ```
/// use ss2_audio_api::{Bus, PlayParams, Priority};
/// use ss2_core::math::Vec3;
///
/// let shot = PlayParams::sfx().at(Vec3::new(4.0, 1.0, -2.0)).with_priority(Priority::HIGH);
/// assert_eq!(shot.bus, Bus::Sfx);
/// assert!(shot.spatial.is_some());
/// assert!(PlayParams::ui().spatial.is_none());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayParams {
    /// Linear gain, `>= 0`, `1.0` = as authored.
    pub volume: f32,
    /// Playback-rate multiplier, `0.1..=4.0`, `1.0` = unchanged (also changes pitch).
    pub pitch: f32,
    /// The bus to play on.
    pub bus: Bus,
    /// Cull order under voice pressure.
    pub priority: Priority,
    /// Repeat until [`stop`](crate::Audio::stop)ped.
    pub looped: bool,
    /// World placement; `None` = 2D.
    pub spatial: Option<Spatial>,
}

impl PlayParams {
    /// A 2D sound effect: volume 1, pitch 1, [`Bus::Sfx`], [`Priority::NORMAL`], one-shot.
    pub fn sfx() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            bus: Bus::Sfx,
            priority: Priority::NORMAL,
            looped: false,
            spatial: None,
        }
    }

    /// A menu/HUD sound: like [`sfx`](Self::sfx) but on [`Bus::Ui`] and never culled.
    pub fn ui() -> Self {
        Self {
            bus: Bus::Ui,
            priority: Priority::CRITICAL,
            ..Self::sfx()
        }
    }

    /// Place the sound in the world at `position` (default range).
    #[must_use]
    pub fn at(mut self, position: Vec3) -> Self {
        self.spatial = Some(Spatial::at(position));
        self
    }

    /// Repeat until stopped.
    #[must_use]
    pub fn looped(mut self) -> Self {
        self.looped = true;
        self
    }

    /// Set the volume.
    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// Set the playback rate.
    #[must_use]
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch;
        self
    }

    /// Set the bus.
    #[must_use]
    pub fn on_bus(mut self, bus: Bus) -> Self {
        self.bus = bus;
        self
    }

    /// Set the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Check the invariants every implementation relies on.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming the first bad field.
    pub fn validate(&self) -> Result<()> {
        let bad = |what: &str| Err(Error::invalid("play params", what.to_owned()));
        if !(self.volume.is_finite() && self.volume >= 0.0) {
            return bad(&format!("volume {} must be finite and >= 0", self.volume));
        }
        if !(self.pitch.is_finite() && (0.1..=4.0).contains(&self.pitch)) {
            return bad(&format!("pitch {} must be within 0.1..=4.0", self.pitch));
        }
        if let Some(s) = &self.spatial {
            if !(s.position.is_finite() && s.velocity.is_finite()) {
                return bad("spatial position/velocity must be finite");
            }
            let ranges_ok = s.min_distance.is_finite()
                && s.max_distance.is_finite()
                && s.min_distance > 0.0
                && s.max_distance > s.min_distance;
            if !ranges_ok {
                return bad(&format!(
                    "spatial range must satisfy 0 < min ({}) < max ({})",
                    s.min_distance, s.max_distance
                ));
            }
        }
        Ok(())
    }
}

/// Changes to apply to a playing voice (e.g. every frame for a moving emitter). `None` leaves a
/// value as it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VoiceUpdate {
    /// New world position (ignored for 2D voices).
    pub position: Option<Vec3>,
    /// New world velocity (ignored for 2D voices).
    pub velocity: Option<Vec3>,
    /// New linear gain.
    pub volume: Option<f32>,
    /// New playback rate.
    pub pitch: Option<f32>,
}

impl VoiceUpdate {
    /// Check the same invariants as [`PlayParams::validate`] for the fields that are set.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming the bad field.
    pub fn validate(&self) -> Result<()> {
        let bad = |what: &str| Err(Error::invalid("voice update", what.to_owned()));
        if self.position.is_some_and(|v| !v.is_finite())
            || self.velocity.is_some_and(|v| !v.is_finite())
        {
            return bad("position/velocity must be finite");
        }
        if self.volume.is_some_and(|v| !(v.is_finite() && v >= 0.0)) {
            return bad("volume must be finite and >= 0");
        }
        if self
            .pitch
            .is_some_and(|p| !(p.is_finite() && (0.1..=4.0).contains(&p)))
        {
            return bad("pitch must be within 0.1..=4.0");
        }
        Ok(())
    }
}

/// The ears: where sounds are heard from. Usually the camera, set once per frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    /// World position.
    pub position: Vec3,
    /// World velocity for Doppler.
    pub velocity: Vec3,
    /// Direction the listener faces (need not be normalized).
    pub forward: Vec3,
    /// The listener's up direction (need not be normalized, must not be parallel to `forward`).
    pub up: Vec3,
}

impl Default for Listener {
    /// At the origin, facing -Z with +Y up (engine convention).
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
        }
    }
}

impl Listener {
    /// `true` if all values are finite and `forward`/`up` define a usable orientation.
    pub fn is_valid(&self) -> bool {
        self.position.is_finite()
            && self.velocity.is_finite()
            && self.forward.is_finite()
            && self.up.is_finite()
            && self.forward.length_squared() > 1e-12
            && self.up.length_squared() > 1e-12
            && self.forward.cross(self.up).length_squared() > 1e-12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_reject_zero_and_round_trip() {
        assert!(SoundId::from_raw(0).is_none());
        assert_eq!(SoundId::from_raw(9).unwrap().raw(), 9);
        assert_eq!(size_of::<Option<SoundId>>(), size_of::<u32>());
    }

    #[test]
    fn priorities_order_and_buses_listed() {
        assert!(Priority::LOW < Priority::NORMAL);
        assert!(Priority::NORMAL < Priority::HIGH);
        assert!(Priority::HIGH < Priority::CRITICAL);
        assert_eq!(Bus::ALL[0], Bus::Master);
        assert_eq!(Bus::ALL.len(), 5);
    }

    #[test]
    fn presets_and_builders() {
        let p = PlayParams::sfx();
        assert_eq!(
            (p.volume, p.pitch, p.bus, p.looped),
            (1.0, 1.0, Bus::Sfx, false)
        );
        assert!(p.spatial.is_none());
        let u = PlayParams::ui();
        assert_eq!((u.bus, u.priority), (Bus::Ui, Priority::CRITICAL));
        let loud = PlayParams::sfx()
            .with_volume(0.5)
            .with_pitch(1.5)
            .looped()
            .on_bus(Bus::Voice);
        assert_eq!(
            (loud.volume, loud.pitch, loud.looped, loud.bus),
            (0.5, 1.5, true, Bus::Voice)
        );
        let at = PlayParams::sfx().at(Vec3::X);
        assert_eq!(at.spatial.unwrap().position, Vec3::X);
        assert!(at.validate().is_ok());
    }

    #[test]
    fn play_params_validation_rejects_garbage() {
        let bad = |p: PlayParams| assert!(p.validate().is_err(), "{p:?}");
        bad(PlayParams::sfx().with_volume(-0.1));
        bad(PlayParams::sfx().with_volume(f32::NAN));
        bad(PlayParams::sfx().with_pitch(0.0));
        bad(PlayParams::sfx().with_pitch(9.0));
        bad(PlayParams::sfx().with_pitch(f32::INFINITY));
        bad(PlayParams::sfx().at(Vec3::new(f32::NAN, 0.0, 0.0)));
        let mut p = PlayParams::sfx().at(Vec3::ZERO);
        p.spatial.as_mut().unwrap().min_distance = 10.0;
        p.spatial.as_mut().unwrap().max_distance = 5.0;
        bad(p);
        p.spatial.as_mut().unwrap().min_distance = 0.0;
        p.spatial.as_mut().unwrap().max_distance = 5.0;
        bad(p);
        assert!(
            PlayParams::sfx().with_volume(0.0).validate().is_ok(),
            "silence is valid"
        );
        assert!(
            PlayParams::sfx().with_volume(3.0).validate().is_ok(),
            "boost is valid"
        );
    }

    #[test]
    fn voice_update_validates_only_set_fields() {
        assert!(VoiceUpdate::default().validate().is_ok());
        let ok = VoiceUpdate {
            position: Some(Vec3::ONE),
            volume: Some(0.2),
            ..Default::default()
        };
        assert!(ok.validate().is_ok());
        assert!(
            VoiceUpdate {
                volume: Some(-1.0),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            VoiceUpdate {
                pitch: Some(100.0),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            VoiceUpdate {
                velocity: Some(Vec3::splat(f32::NAN)),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn listener_validity() {
        assert!(Listener::default().is_valid());
        let bad = |f: &dyn Fn(&mut Listener)| {
            let mut l = Listener::default();
            f(&mut l);
            assert!(!l.is_valid(), "{l:?}");
        };
        bad(&|l| l.position.x = f32::NAN);
        bad(&|l| l.forward = Vec3::ZERO);
        bad(&|l| l.up = l.forward);
        bad(&|l| l.up = -l.forward * 3.0);
    }
}
