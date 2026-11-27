//! Fuzzes: ss2_ui::lobby::{parse_address, normalize_invite}
//!
//! The property is `ss2_fuzzkit::lobby_text` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/lobby_text/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::lobby_text(data));
