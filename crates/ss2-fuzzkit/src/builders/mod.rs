//! Builders of synthetic files, copied from the unit tests of the format crates.
//!
//! The format crates keep their file writers inside `#[cfg(test)]` modules, where nothing outside the crate can reach
//! them. Each module here is the head of that test module, unchanged apart from visibility and the imports (the
//! file names say where it came from). They write the same bytes the crate's own tests read, built from the clean-room
//! specs in `docs/re/` with the testkit's `MetaBuilder`: no game data, no bytes copied from an install. If a format changes
//! and its crate's builder is updated, the matching one here must follow; `tests/smoke.rs` fails when a seed stops reading.
#![allow(dead_code, clippy::all, clippy::pedantic)]

pub mod ans;
pub mod bmf;
pub mod ep;
pub mod fnt;
pub mod fx_lfx;
pub mod fx_pfx;
pub mod fx_tfx;
pub mod fx_vfx;
pub mod mat;
pub mod mdl;
pub mod mtr;
pub mod rsc;
pub mod wld;
