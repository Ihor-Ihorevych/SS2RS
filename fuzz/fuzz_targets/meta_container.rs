//! Fuzzes: ss2_meta::Container::parse, decode_objects, write
//!
//! The property is `ss2_fuzzkit::meta_container` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/meta_container/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::meta_container(data));
