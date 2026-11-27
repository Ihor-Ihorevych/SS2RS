//! Fuzzes: ss2_game::{campaign::CarryOver, campaign::UnlockSet, campaign::CampaignNote, difficulty::CampaignDifficulty, level_progress::LevelProgress}::from_bytes
//!
//! The property is `ss2_fuzzkit::game_state_bytes` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/game_state_bytes/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::game_state_bytes(data));
