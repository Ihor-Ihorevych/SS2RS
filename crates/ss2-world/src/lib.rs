//! ss2-world — level instance, navigation, triggers and mechanisms (track T07).
//!
//! - [`collision`]: the static collision world of a level.
//! - [`load`]: loading a level's files into a [`scene::LevelScene`] ([`load::load_level`]).
//! - [`scene`]: one loaded level as plain data ([`scene::LevelScene`], RFC 0013).

pub mod collision;
pub mod load;
pub mod scene;
