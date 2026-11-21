//! Math types shared by every crate: a thin re-export of [`glam`] so the whole engine agrees on
//! one vector/matrix library and one version (`ss2_core::math::Vec3`, never `glam::Vec3`).
//!
//! Engine-internal convention (see `docs/rfcs/0002-render-frontend-api.md`): **right-handed,
//! Y up, -Z forward**, angles in radians, clip depth in `0..=1`. Converting from the original
//! game's own conventions is the job of the format loaders, not of consumers of these types.

pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

/// Projection matrix constructors for engine space: view space is right-handed Y-up, clip depth
/// is `0..=1` and clip Y is up (the wgpu/Direct3D convention).
pub mod projection {
    pub use glam::camera::rh::proj::directx::{orthographic, perspective_infinite_reverse};
}
