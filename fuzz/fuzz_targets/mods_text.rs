//! Fuzzes: ss2_vfs::{ModManifest::parse, Nfo::parse, parse_current_mod}
//!
//! The property is `ss2_fuzzkit::mods_text` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/mods_text/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::mods_text(data));
