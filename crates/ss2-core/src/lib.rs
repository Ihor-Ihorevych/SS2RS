//! ss2-core — ids, errors, logging and time helpers shared by all crates (track T00)

pub mod console;
pub mod crash;
pub mod error;
pub mod input;
pub mod logfile;
pub mod logging;
pub mod math;
pub mod settings;
pub mod time;

pub use error::{Error, Result};

#[cfg(test)]
mod settings_persistence_tests;
