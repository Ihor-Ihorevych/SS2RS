//! Fuzzes: ss2_audio_api::replication::AudioEvent::decode
//!
//! The property is `ss2_fuzzkit::audio_event` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/audio_event/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::audio_event(data));
