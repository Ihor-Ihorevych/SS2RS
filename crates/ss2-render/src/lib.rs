//! ss2-render — wgpu renderer (track T04): 2D quads and text for the menu slice.

mod batch;
mod context;
pub mod golden;
pub mod gpu;
mod image_diff;
mod text;

pub use batch::*;
pub use context::*;
pub use image_diff::*;
