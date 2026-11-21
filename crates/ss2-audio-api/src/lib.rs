//! ss2-audio-api: the device-free audio contract (tracks T00/T06).
//!
//! This is the **v0 contract** (roadmap T00-018, `docs/rfcs/0004-audio-api.md`): the only audio
//! types gameplay, UI and the app may use. It mentions no `cpal`, decoder or device, so the
//! headless server compiles it and every consumer can be tested silently. The real mixer
//! (`ss2-audio`, T06-001) implements [`Audio`]; [`NullAudio`] behaves like it (voice ids, culling
//! by priority, buses, music, listener) until then.
//!
//! - [`AudioResources::sound`]: path to [`SoundId`], once, at spawn / level load.
//! - [`Audio::play`]: start a sound with [`PlayParams`] (volume, pitch, [`Bus`], [`Priority`],
//!   loop, optional [`Spatial`] placement); get a [`VoiceId`] to move or stop it.
//! - [`Audio::set_listener`]: the ears, once per frame.
//! - [`Audio::play_music`] / [`stop_music`](Audio::stop_music): one looping, crossfaded track.
//! - [`Audio::set_bus_volume`]: the user's master/music/sfx/voice/ui sliders.
//!
//! ```
//! use std::time::Duration;
//! use ss2_audio_api::{Audio, AudioResources, Bus, Listener, NullAudio, PlayParams, VoiceUpdate};
//! use ss2_core::math::Vec3;
//!
//! let mut audio = NullAudio::new();                    // or the real mixer later
//! let growl = audio.resources().sound("Sounds/Enemies/Gnaar/Growl.wav");
//! let theme = audio.resources().sound("Music/Level1.ogg");
//!
//! // Level start
//! audio.play_music(theme, Duration::from_secs(2));
//!
//! // Every frame
//! audio.set_listener(&Listener::default()).unwrap();
//!
//! // An enemy growls at a position and then walks.
//! let voice = audio.play(growl, &PlayParams::sfx().at(Vec3::new(10.0, 0.0, -4.0))).unwrap();
//! audio
//!     .update_voice(voice, &VoiceUpdate { position: Some(Vec3::new(9.0, 0.0, -4.0)), ..Default::default() })
//!     .unwrap();
//!
//! // Settings screen
//! audio.set_bus_volume(Bus::Music, 0.5);
//! assert_eq!(audio.bus_volume(Bus::Music), 0.5);
//! ```

mod api;
mod null;
mod types;

pub use api::{Audio, AudioResources};
pub use null::{AudioCall, MockAudioResources, NullAudio};
pub use types::{Bus, Listener, PlayParams, Priority, SoundId, Spatial, VoiceId, VoiceUpdate};
