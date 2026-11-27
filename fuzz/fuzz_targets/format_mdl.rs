//! Fuzzes: ss2_format_mdl::Model::from_bytes
//!
//! The property is `ss2_fuzzkit::format_mdl` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/format_mdl/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_mdl(data));
