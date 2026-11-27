//! ss2-ui — UI framework, HUD, menus and localization (track T10)

pub mod accessibility;
pub mod coop_overlays;
#[cfg(test)]
mod coop_overlays_tests;
pub mod data_wizard;
#[cfg(test)]
mod data_wizard_tests;
pub mod devconsole;
pub mod draw;
pub mod focus;
pub mod geometry;
pub mod lobby;

pub use draw::*;
pub use geometry::*;
pub mod menu;
pub use menu::*;
#[cfg(test)]
mod about_tests;
#[cfg(test)]
mod draw_tests;
pub mod layout;
pub mod level_select;
pub mod loading;
mod pointer;
pub mod scale;
pub mod session_screens;
#[cfg(test)]
mod session_screens_tests;
pub mod settings_model;
mod skin;
pub mod subtitles;
#[cfg(test)]
mod subtitles_tests;
pub mod theme;
