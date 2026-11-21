//! The per-frame extract: everything the renderer needs to draw one simulation tick.

use std::collections::BTreeSet;
use std::ops::BitOr;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_ui::DrawList;

use crate::camera::Camera;
use crate::ids::{ClipId, MaterialId, ModelId, RenderId};

/// Position, orientation and scale of something in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// World-space position.
    pub translation: Vec3,
    /// World-space orientation (must be normalized).
    pub rotation: Quat,
    /// Per-axis scale.
    pub scale: Vec3,
}

impl Transform {
    /// No translation, no rotation, scale 1.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Identity rotation and scale at `translation`.
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    /// Local → world matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Blend between `self` (`t = 0`) and `other` (`t = 1`): linear for position and scale,
    /// spherical for rotation. This is the interpolation the renderer applies between two
    /// consecutive ticks (see [`Renderer::render`](crate::Renderer::render)).
    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }

    fn is_valid(&self) -> bool {
        self.translation.is_finite()
            && self.scale.is_finite()
            && self.rotation.is_finite()
            && (self.rotation.length_squared() - 1.0).abs() < 1e-3
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Per-renderable switches. Combine with `|`; test with [`contains`](Self::contains).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderFlags(pub u32);

impl RenderFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// The object is rendered into shadow maps.
    pub const CAST_SHADOWS: Self = Self(1 << 0);
    /// First-person view model (the weapon in the player's hands): drawn in its own depth range
    /// so it never clips into walls.
    pub const VIEWMODEL: Self = Self(1 << 1);

    /// `true` if every bit of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for RenderFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Which animation a skinned model plays, and how far into it. The renderer samples the pose,
/// the simulation only decides *what* plays *when* (keeps the sim free of skeleton data).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimState {
    /// The clip, from [`RenderResources::clip`](crate::RenderResources::clip).
    pub clip: ClipId,
    /// Seconds since the clip started, `>= 0` and finite. The renderer wraps or clamps it
    /// according to the clip.
    pub time: f32,
}

/// One thing to draw: a model at a transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Renderable {
    /// Stable identity across frames; unique within a [`RenderWorld`].
    pub id: RenderId,
    /// What to draw.
    pub model: ModelId,
    /// Where to draw it.
    pub transform: Transform,
    /// Replace the model's own materials (damage flash, spawn-in effects), if set.
    pub material_override: Option<MaterialId>,
    /// Multiplied into the final color; **linear** RGBA, `[1.0; 4]` = unchanged.
    pub tint: [f32; 4],
    /// Animation to play for skinned models.
    pub animation: Option<AnimState>,
    /// Switches.
    pub flags: RenderFlags,
}

impl Renderable {
    /// A renderable with defaults: untinted, no override, no animation, casts shadows.
    pub fn new(id: RenderId, model: ModelId, transform: Transform) -> Self {
        Self {
            id,
            model,
            transform,
            material_override: None,
            tint: [1.0; 4],
            animation: None,
            flags: RenderFlags::CAST_SHADOWS,
        }
    }
}

/// Shape of a [`Light`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightKind {
    /// Radiates in all directions.
    Point,
    /// A cone.
    Spot {
        /// Unit vector the cone points along.
        direction: Vec3,
        /// Half-angle (radians) inside which the light is at full strength.
        inner_angle: f32,
        /// Half-angle (radians) at which it fades to zero; `>= inner_angle`.
        outer_angle: f32,
    },
}

/// A dynamic light (muzzle flashes, explosions, lamps). Baked lighting is not extracted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// Shape.
    pub kind: LightKind,
    /// World-space position.
    pub position: Vec3,
    /// **Linear** RGB.
    pub color: [f32; 3],
    /// Scalar multiplier on `color`, `>= 0`.
    pub intensity: f32,
    /// Distance at which the light has faded to nothing, `> 0`.
    pub radius: f32,
    /// Whether the light casts dynamic shadows (subject to the quality tier's budget).
    pub casts_shadow: bool,
}

impl Light {
    fn is_valid(&self) -> bool {
        let shape_ok = match self.kind {
            LightKind::Point => true,
            LightKind::Spot {
                direction,
                inner_angle,
                outer_angle,
            } => {
                direction.is_finite()
                    && (direction.length_squared() - 1.0).abs() < 1e-3
                    && inner_angle.is_finite()
                    && outer_angle.is_finite()
                    && 0.0 <= inner_angle
                    && inner_angle <= outer_angle
                    && outer_angle <= std::f32::consts::PI
            }
        };
        shape_ok
            && self.position.is_finite()
            && self.color.iter().all(|c| c.is_finite() && *c >= 0.0)
            && self.intensity.is_finite()
            && self.intensity >= 0.0
            && self.radius.is_finite()
            && self.radius > 0.0
    }
}

/// A snapshot of everything visible at one simulation tick: the **extract** the simulation hands
/// to the renderer. It is plain owned data (`Send + 'static`, no GPU types, no borrows into the
/// simulation), so it can move to a render thread or be dropped on a headless server.
///
/// Build one per tick, ideally reusing the previous allocation via [`RenderWorld::reset`].
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_render_api::{ModelId, RenderId, RenderWorld, Renderable, Transform};
///
/// let model = ModelId::from_raw(1).unwrap();
/// let mut world = RenderWorld::new(42);
/// world.renderables.push(Renderable::new(
///     RenderId(7),
///     model,
///     Transform::from_translation(Vec3::new(1.0, 0.0, -3.0)),
/// ));
/// assert!(world.validate().is_ok());
/// ```
#[derive(Clone, Debug, Default)]
pub struct RenderWorld {
    /// The simulation tick this snapshot was taken at; strictly increases between submissions.
    pub tick: u64,
    /// The view to render from.
    pub camera: Camera,
    /// Opaque and transparent 3D objects (the renderer sorts and batches).
    pub renderables: Vec<Renderable>,
    /// Dynamic lights.
    pub lights: Vec<Light>,
    /// 2D/UI/HUD, in canvas coordinates, drawn on top of the 3D scene. This is the same
    /// [`DrawList`] `ss2-ui` already produces.
    pub ui: DrawList,
}

impl RenderWorld {
    /// An empty world for `tick`.
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Self::default()
        }
    }

    /// Empty the world for reuse at `tick`, **keeping the allocated capacity**.
    pub fn reset(&mut self, tick: u64) {
        self.tick = tick;
        self.camera = Camera::default();
        self.renderables.clear();
        self.lights.clear();
        self.ui.quads.clear();
        self.ui.texts.clear();
    }

    /// Check the invariants renderers rely on: a valid camera, unique [`RenderId`]s, finite and
    /// normalized transforms, sane tints, animation times and lights.
    ///
    /// Renderers call this in [`Renderer::submit`](crate::Renderer::submit) and **reject** an
    /// invalid world instead of drawing garbage or panicking; extraction code can assert it in
    /// debug builds.
    ///
    /// # Errors
    /// [`Error::Invalid`] describing the first problem found.
    pub fn validate(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("render world", detail));
        if !self.camera.is_valid() {
            return bad(format!("invalid camera {:?}", self.camera));
        }
        let mut seen = BTreeSet::new();
        for r in &self.renderables {
            if !seen.insert(r.id) {
                return bad(format!("duplicate {:?}", r.id));
            }
            if !r.transform.is_valid() {
                return bad(format!("invalid transform on {:?}", r.id));
            }
            if r.tint.iter().any(|c| !c.is_finite()) {
                return bad(format!("non-finite tint on {:?}", r.id));
            }
            if let Some(a) = r.animation
                && !(a.time.is_finite() && a.time >= 0.0)
            {
                return bad(format!("bad animation time {} on {:?}", a.time, r.id));
            }
        }
        for (i, l) in self.lights.iter().enumerate() {
            if !l.is_valid() {
                return bad(format!("invalid light #{i}: {l:?}"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> ModelId {
        ModelId::from_raw(1).unwrap()
    }

    fn renderable(id: u64) -> Renderable {
        Renderable::new(RenderId(id), model(), Transform::IDENTITY)
    }

    #[test]
    fn transform_matrix_applies_scale_then_rotation_then_translation() {
        let t = Transform {
            translation: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::splat(2.0),
        };
        let p = t.matrix().transform_point3(Vec3::new(0.0, 0.0, -1.0));
        // (0,0,-1) scaled -> (0,0,-2); +90° about Y sends -Z to -X -> (-2,0,0); +10 in x.
        assert!(p.abs_diff_eq(Vec3::new(8.0, 0.0, 0.0), 1e-5), "{p}");
    }

    #[test]
    fn interpolate_hits_endpoints_and_midpoint() {
        let a = Transform::IDENTITY;
        let b = Transform {
            translation: Vec3::new(2.0, 4.0, 0.0),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            scale: Vec3::splat(3.0),
        };
        assert_eq!(a.interpolate(&b, 0.0), a);
        let end = a.interpolate(&b, 1.0);
        assert!(end.translation.abs_diff_eq(b.translation, 1e-6));
        assert!(end.rotation.abs_diff_eq(b.rotation, 1e-6));
        let mid = a.interpolate(&b, 0.5);
        assert!(mid.translation.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-6));
        assert!(mid.scale.abs_diff_eq(Vec3::splat(2.0), 1e-6));
        let quarter_turn = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        assert!(mid.rotation.abs_diff_eq(quarter_turn, 1e-5));
    }

    #[test]
    fn flags_combine_and_test() {
        let f = RenderFlags::CAST_SHADOWS | RenderFlags::VIEWMODEL;
        assert!(f.contains(RenderFlags::CAST_SHADOWS));
        assert!(f.contains(RenderFlags::VIEWMODEL));
        assert!(!RenderFlags::CAST_SHADOWS.contains(RenderFlags::VIEWMODEL));
        assert!(f.contains(RenderFlags::NONE));
    }

    #[test]
    fn new_world_with_defaults_validates() {
        let mut w = RenderWorld::new(1);
        w.renderables.push(renderable(1));
        w.renderables.push(renderable(2));
        w.lights.push(Light {
            kind: LightKind::Spot {
                direction: Vec3::NEG_Y,
                inner_angle: 0.2,
                outer_angle: 0.5,
            },
            position: Vec3::Y,
            color: [1.0, 0.9, 0.8],
            intensity: 2.0,
            radius: 10.0,
            casts_shadow: true,
        });
        assert!(w.validate().is_ok());
    }

    #[test]
    fn validate_rejects_each_kind_of_garbage() {
        let case = |name: &str, edit: &dyn Fn(&mut RenderWorld)| {
            let mut w = RenderWorld::new(1);
            w.renderables.push(renderable(1));
            edit(&mut w);
            let err = w.validate().expect_err(name);
            assert!(matches!(err, Error::Invalid { .. }), "{name}: {err}");
        };
        case("duplicate id", &|w| w.renderables.push(renderable(1)));
        case("nan position", &|w| {
            w.renderables[0].transform.translation.x = f32::NAN;
        });
        case("unnormalized rotation", &|w| {
            w.renderables[0].transform.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 3.0);
        });
        case("inf scale", &|w| {
            w.renderables[0].transform.scale.y = f32::INFINITY;
        });
        case("nan tint", &|w| w.renderables[0].tint[2] = f32::NAN);
        case("negative anim time", &|w| {
            w.renderables[0].animation = Some(AnimState {
                clip: ClipId::from_raw(1).unwrap(),
                time: -1.0,
            });
        });
        case("bad camera", &|w| w.camera.position.z = f32::NAN);
        case("zero radius light", &|w| {
            w.lights.push(Light {
                kind: LightKind::Point,
                position: Vec3::ZERO,
                color: [1.0; 3],
                intensity: 1.0,
                radius: 0.0,
                casts_shadow: false,
            });
        });
        case("spot angles reversed", &|w| {
            w.lights.push(Light {
                kind: LightKind::Spot {
                    direction: Vec3::Y,
                    inner_angle: 0.8,
                    outer_angle: 0.2,
                },
                position: Vec3::ZERO,
                color: [1.0; 3],
                intensity: 1.0,
                radius: 5.0,
                casts_shadow: false,
            });
        });
    }

    #[test]
    fn reset_keeps_capacity_and_clears_content() {
        let mut w = RenderWorld::new(1);
        for i in 0..100 {
            w.renderables.push(renderable(i));
        }
        let cap = w.renderables.capacity();
        w.reset(2);
        assert_eq!(w.tick, 2);
        assert!(w.renderables.is_empty() && w.lights.is_empty());
        assert!(w.ui.quads.is_empty() && w.ui.texts.is_empty());
        assert_eq!(w.renderables.capacity(), cap);
    }

    #[test]
    fn world_is_send_and_owned() {
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<RenderWorld>();
    }
}
