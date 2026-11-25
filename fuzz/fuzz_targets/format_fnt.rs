//! Fuzzes: ss2_format_fnt::Font::from_bytes and the text layout
//!
//! The property is `ss2_fuzzkit::format_fnt` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/format_fnt/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_fnt(data));
