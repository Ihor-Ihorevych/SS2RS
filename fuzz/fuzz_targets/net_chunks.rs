//! Fuzzes: ss2_net::join::{ChunkSender, ChunkReceiver}
//!
//! The property is `ss2_fuzzkit::net_chunks` (crates/ss2-fuzzkit/src/targets.rs); the seeds are in `fuzz/seeds/net_chunks/`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ss2_fuzzkit::net_chunks(data));
