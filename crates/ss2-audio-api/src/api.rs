//! The two traits the rest of the engine uses: resolve sounds, then play them.

use std::time::Duration;

use ss2_core::Result;

use crate::types::{Bus, Listener, PlayParams, SoundId, VoiceId, VoiceUpdate};

/// Turns sound paths into [`SoundId`]s. Resolve **once** (at spawn / level load), keep the id.
///
/// Cheap and idempotent: the same normalized path yields the same id. The id is valid right away
/// even while the file is still loading or decoding, or if it failed to load: playing such a
/// sound is silent, never an error for the caller. `path` is a game-data path
/// (`"sounds/weapons/shotgun.wav"`, or a `.scs` collection) normalized like `ss2_vfs::VfsPath`.
pub trait AudioResources {
    /// The sound (or sound collection) at `path`.
    fn sound(&mut self, path: &str) -> SoundId;
}

/// What the game talks to. The real mixer (`ss2-audio`, T06-001) implements it on top of `cpal`;
/// [`NullAudio`](crate::NullAudio) is the headless / test implementation.
///
/// All methods take effect from the next mixer block (a few milliseconds). Methods never block on
/// the audio device. Operations naming a voice that has finished, been stopped or been culled are
/// silent no-ops.
pub trait Audio {
    /// Resolve sound paths.
    fn resources(&mut self) -> &mut dyn AudioResources;

    /// Start `sound`. Returns the voice to control it with.
    ///
    /// Under voice pressure the implementation may cull the quietest low-priority voice or this
    /// one; a culled voice still gets a [`VoiceId`] (it is simply not [`playing`](Self::is_playing)),
    /// so callers never need to handle "no voice available".
    ///
    /// # Errors
    /// [`Error::Invalid`](ss2_core::Error::Invalid) if `params` fail
    /// [`PlayParams::validate`]; nothing is played.
    fn play(&mut self, sound: SoundId, params: &PlayParams) -> Result<VoiceId>;

    /// Stop a voice, fading out over `fade` (zero = cut immediately). Stopping a looped voice is
    /// the only way it ends.
    fn stop(&mut self, voice: VoiceId, fade: Duration);

    /// `true` while the voice is audibly in the mix or still queued to start; `false` once it has
    /// finished, been stopped or culled.
    fn is_playing(&self, voice: VoiceId) -> bool;

    /// Move / re-level / re-pitch a voice (call per frame for emitters attached to entities).
    ///
    /// # Errors
    /// [`Error::Invalid`](ss2_core::Error::Invalid) if `update` fails [`VoiceUpdate::validate`].
    fn update_voice(&mut self, voice: VoiceId, update: &VoiceUpdate) -> Result<()>;

    /// Set the ears for 3D sound; call once per frame (usually from the camera).
    ///
    /// # Errors
    /// [`Error::Invalid`](ss2_core::Error::Invalid) if the listener fails [`Listener::is_valid`];
    /// the previous listener is kept.
    fn set_listener(&mut self, listener: &Listener) -> Result<()>;

    /// Start looping `track` as the music, crossfading from the current music (if any) over
    /// `crossfade`. Playing the track that is already current does nothing.
    fn play_music(&mut self, track: SoundId, crossfade: Duration);

    /// Fade the music out over `fade` and stop it.
    fn stop_music(&mut self, fade: Duration);

    /// The sound currently set as music, if any.
    fn current_music(&self) -> Option<SoundId>;

    /// Set a bus's user volume (`0.0..=1.0`; values are clamped, NaN is ignored).
    fn set_bus_volume(&mut self, bus: Bus, volume: f32);

    /// A bus's user volume, `1.0` unless changed.
    fn bus_volume(&self, bus: Bus) -> f32;

    /// Mute everything while the app is suspended or unfocused (T06-017, mobile pause) and unmute
    /// it again; voices keep their positions so playback resumes seamlessly.
    fn set_suspended(&mut self, suspended: bool);
}
