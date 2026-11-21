//! ss2-core — ids, errors, logging and time helpers shared by all crates (track T00)

pub mod error;
pub mod input;
pub mod logging;
pub mod math;
pub mod settings;

pub use error::{Error, Result};

#[cfg(test)]
mod settings_persistence_tests;
