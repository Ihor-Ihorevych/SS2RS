//! ss2-ui — UI framework, HUD, menus and localization (track T10)

pub mod accessibility;
pub mod devconsole;
pub mod draw;
pub mod focus;
pub mod geometry;

pub use draw::*;
pub use geometry::*;
pub mod menu;
pub use menu::*;
#[cfg(test)]
mod about_tests;
#[cfg(test)]
mod draw_tests;
pub mod layout;
mod pointer;
pub mod scale;
mod skin;
pub mod theme;
