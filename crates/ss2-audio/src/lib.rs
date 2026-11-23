//! ss2-audio — audio mixer, 3D sound and music (track T06)
//!
//! Today this crate holds the **mixer core** (`mixer`: voices, buses, ducking, voice management, an offline renderer), WAV writing for it
//! (`wav`), a tolerance for golden-audio tests (`compare`) and the **decoders** (`decode`: WAV and Ogg Vorbis, T06-002, `docs/AUDIO_DECODERS.md`). See
//! `docs/MIXER.md`. The output device (T06-001) and the rest of the track are not built.

pub mod compare;
pub mod decode;
pub mod mixer;
pub mod wav;

#[cfg(test)]
mod decode_tests;

pub use compare::{AudioDiff, AudioTolerance, compare};
pub use mixer::{
    Ducking, Mixer, MixerConfig, MixerStats, SampleBuffer, VoiceState, render_offline,
};
