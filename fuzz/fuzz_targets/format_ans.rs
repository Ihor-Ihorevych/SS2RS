//! Fuzzes: ss2_format_ans::AnimSet::from_bytes and the pose sampling
//!
//! The property is `ss2_fuzzkit::format_ans` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/format_ans/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_ans(data));
