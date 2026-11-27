//! Fuzzes: ss2_audio::decode::{sniff, probe, decode, decode_wav, decode_vorbis, VorbisStream, last_granule}, ss2_audio::wav::decode
//!
//! The property is `ss2_fuzzkit::audio_decode` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/audio_decode/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::audio_decode(data));
