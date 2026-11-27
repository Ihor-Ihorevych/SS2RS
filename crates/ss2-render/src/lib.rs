//! ss2-render — wgpu renderer (track T04): the 2D/UI renderer (quads, text, images, 9-slice, scissor,
//! render-to-texture; `docs/UI_RENDERER.md`).

mod batch;
mod context;
pub mod golden;
pub mod gpu;
mod image_diff;
mod images;
mod nine_slice;
mod plan;
mod scissor;
mod text;
mod ui_frame;
#[cfg(test)]
mod ui_gpu_tests;
mod world;
#[cfg(test)]
mod world_gpu_tests;

pub use batch::*;
pub use context::*;
pub use image_diff::*;
pub use nine_slice::*;
pub use scissor::*;
pub use ui_frame::{Filter, ImageDraw, ImageHandle, NineSliceDraw, UiCommand, UiFrame};
pub use world::{MeshHandle, TextureHandle, WorldBatch};
