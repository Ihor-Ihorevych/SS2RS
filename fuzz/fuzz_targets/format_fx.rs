//! Fuzzes: ss2_format_fx::{Effect (pfx), FlareConfiguration (lfx), TextEffectConfiguration (tfx), VibrationEffect (vfx)}::from_bytes
//!
//! The property is `ss2_fuzzkit::format_fx` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/format_fx/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_fx(data));
