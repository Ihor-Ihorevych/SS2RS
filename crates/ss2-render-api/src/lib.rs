//! ss2-render-api — the GPU-free renderer front-end contract (tracks T00/T04).
//!
//! This is the **v0 contract** (roadmap T00-015, `docs/rfcs/0002-render-frontend-api.md`): the
//! only rendering types that simulation, world, gameplay and UI code may use. Nothing here
//! mentions `wgpu`, a window or a thread, so a headless dedicated server compiles it without any
//! graphics stack. The wgpu implementation (`ss2-render`, T04-026) implements [`Renderer`] later;
//! until then [`NullRenderer`] and [`MockResources`] let every consumer be written and tested.
//!
//! - [`RenderWorld`]: the per-tick **extract**: a [`Camera`], [`Renderable`]s, [`Light`]s and the
//!   2D/UI [`DrawList`](ss2_ui::DrawList).
//! - [`RenderResources`]: turns asset paths into cheap ids ([`ModelId`], [`MaterialId`],
//!   [`TextureId`], [`ClipId`]) once, at spawn time.
//! - [`Renderer`]: `submit` a world per tick, `render` a frame with an interpolation `alpha`.
//!
//! ```
//! use ss2_core::math::{Quat, Vec3};
//! use ss2_render_api::{
//!     Camera, NullRenderer, RenderId, RenderWorld, Renderable, Renderer, Transform,
//! };
//!
//! let mut renderer = NullRenderer::new();                       // or the wgpu renderer later
//! let model = renderer.resources().model("Models/Enemies/Gnaar.mdl");
//!
//! // Once per simulation tick:
//! let mut world = RenderWorld::new(1);
//! world.camera = Camera { position: Vec3::new(0.0, 1.7, 0.0), ..Camera::default() };
//! world.renderables.push(Renderable::new(
//!     RenderId(100),
//!     model,
//!     Transform { translation: Vec3::new(0.0, 0.0, -8.0), rotation: Quat::IDENTITY, scale: Vec3::ONE },
//! ));
//! renderer.submit(world).unwrap();
//!
//! // Once per display frame:
//! let stats = renderer.render(0.5).unwrap();
//! assert_eq!(stats.renderables, 1);
//! ```

mod camera;
mod culling;
mod debug_draw;
mod ids;
pub mod lod;
pub mod quality;
mod renderer;
mod resources;
mod scene;

pub use camera::{Camera, Projection};
pub use culling::{Aabb, Frustum, Plane, Sphere, Visibility, jitter_for_frame};
pub use debug_draw::{
    DebugCategory, DebugColor, DebugDraw, DebugLabel, DebugLine, DebugOverlays, ScreenLabel,
};
pub use ids::{ClipId, MaterialId, ModelId, RenderId, TextureId};
pub use renderer::{FrameStats, NullRenderer, Renderer};
pub use resources::{MockResources, RenderResources};
pub use scene::{AnimState, Light, LightKind, RenderFlags, RenderWorld, Renderable, Transform};
