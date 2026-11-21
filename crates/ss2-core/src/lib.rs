//! ss2-core — ids, errors, logging and time helpers shared by all crates (track T00)

pub mod error;
pub mod input;
pub mod logging;
pub mod settings;

pub use error::{Error, Result};
