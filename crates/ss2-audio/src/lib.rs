//! ss2-audio — audio mixer, 3D sound and music (track T06)
//!
//! Today this crate holds the **mixer core** (`mixer`: voices, buses, ducking, voice management, an offline renderer), WAV writing for it
//! (`wav`) and a tolerance for golden-audio tests (`compare`): sample arithmetic with no device and no decoder. See `docs/MIXER.md`. The output device
//! (T06-001), the decoders (T06-002) and the rest of the track are not built.

pub mod compare;
pub mod mixer;
pub mod wav;

pub use compare::{AudioDiff, AudioTolerance, compare};
pub use mixer::{
    Ducking, Mixer, MixerConfig, MixerStats, SampleBuffer, VoiceState, render_offline,
};
