//! Fuzzes: ss2_format_mtr::MaterialFile::from_bytes
//!
//! The property is `ss2_fuzzkit::format_mtr` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/format_mtr/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_mtr(data));
