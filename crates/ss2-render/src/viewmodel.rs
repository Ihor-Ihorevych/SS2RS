//! The weapon viewmodel pass (roadmap T04-030, `docs/re/weapon-view.md`): the first-person weapon is drawn
//! **after** the world, with its **own** projection, and lands on top of everything drawn so far.
//!
//! The original (`docs/re/weapon-view.md` section 4) builds the weapon's projection from its own horizontal
//! field of view and near plane (`bwrp_fFOV`, `bwrp_fFrontClipDist`, 0.1 in every retail file) and squeezes the
//! weapon's depth into the first 10 percent of the *shared* depth buffer's range, so that even a weapon model
//! that reaches into a wall never sinks into it. This renderer keeps no depth buffer shared between the two
//! passes at all: the viewmodel pass has its **own** depth attachment, cleared fresh every frame, so a
//! viewmodel draw can never be occluded by (or occlude) world geometry — a stronger guarantee than the
//! original's depth-range trick, reached by a simpler mechanism that fits this renderer's frame structure
//! (one colour target, drawn into by one pass after another, `ss2-render::context`). What a viewmodel draw
//! occludes is only *other* viewmodel draws (so a muzzle flash in front of the weapon model still looks
//! right), which is also why the pass keeps its own depth test rather than drawing with none at all.
//!
//! The pass reuses the world's existing skinned-draw machinery end to end: a [`ViewModel`] draw is a
//! [`crate::SkinnedDraw`], the very struct [`crate::Renderer::set_skinned`] takes, built from a mesh uploaded
//! with [`crate::Renderer::create_skinned_mesh`] and posed with [`crate::Renderer::set_bone_palette`]; a
//! weapon attached to a hand bone is placed the same way T04-046 places any other attachment
//! (`attachment_transform`). The only difference from a world skinned draw is *where* `SkinnedDraw::transform`
//! puts the mesh: here it is camera space directly (right: +X, up: +Y, forward: -Z, the engine's usual
//! convention, origin at the eye) rather than world space, because the viewmodel pass has no view matrix of
//! its own to compose with — the caller's transform already carries every offset the original's placement
//! chain applies (the weapon-in-view placement, walk bob, view damping, charge/recoil displacement; section 3).
//!
//! The field of view is read the same way the original reads it (section 4.2): a **horizontal** field of
//! view in radians *for a 4:3 screen* (`bwrp_fFOV`, 0.531 to 1.228 rad across the 16 retail files), widened
//! for other aspect ratios by [`widen_fov_x`] — the original's own (quirky) formula, reproduced exactly rather
//! than the mathematically "correct" vertical-fov-preserving widening, since the two measurably disagree away
//! from 4:3 (test vector 4: 0.5706 rad at 16:9 widens to 0.7608, not the 0.746 a true Hor+ scaling would give).

use ss2_core::math::Vec3;
use ss2_render_api::{Camera, Projection};

use crate::world::SkinnedDraw;

/// What the viewmodel pass draws this frame ([`crate::Renderer::set_viewmodel`]); `None` draws nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewModel {
    /// The weapon's draws, in the pass's own camera space (an eye at the origin looking down -Z, +Y up; see
    /// the module docs). Posed and attached exactly like a world skinned draw.
    pub draws: Vec<SkinnedDraw>,
    /// Horizontal field of view in radians, **at a 4:3 aspect ratio** (`bwrp_fFOV`): 0.531 to 1.228 rad (30.4
    /// to 70.4 degrees) across the 16 retail weapons; widened for other aspect ratios by the module's own
    /// `widen_fov_x`. Must be finite and in `(0, π)`.
    pub fov_x_at_4_3: f32,
    /// Near clip distance (`bwrp_fFrontClipDist`, 0.1 in every retail file). Must be finite and positive.
    pub near: f32,
}

/// The original's widening of a horizontal field of view given at 4:3, for another aspect ratio
/// (`docs/re/weapon-view.md` section 4 point 2, `CBaseWeaponEntity::Render`): `(360/π) atan(aspect x 0.75 x
/// tan(π x fov / 360))`. The main view uses the same shape of formula (`docs/re/weapon-view.md` section summary
/// point 5). Written out, this *should* be the standard vertical-fov-preserving ("Hor+") widening if `fov` were
/// first converted from radians to degrees and the whole expression worked in degrees throughout — but the
/// original does not convert it, so a value that is a few tenths of a *radian* is run through this formula as
/// if it were that many *degrees*; because the resulting angles are then tiny, `tan` and `atan` are both close
/// to the identity there, and the formula collapses to very nearly a **plain multiplication by `aspect x
/// 0.75`** (to a precision of 1e-4 at a weapon's own FOV range) rather than genuinely preserving the implied
/// vertical FOV. `None` for a non-finite input or a non-finite result.
pub(crate) fn widen_fov_x(fov_x_at_4_3: f32, aspect: f32) -> Option<f32> {
    if !(fov_x_at_4_3.is_finite() && aspect.is_finite() && aspect > 0.0) {
        return None;
    }
    let widened = (360.0 / std::f32::consts::PI)
        * (aspect * 0.75 * (std::f32::consts::PI * fov_x_at_4_3 / 360.0).tan()).atan();
    widened.is_finite().then_some(widened)
}

impl ViewModel {
    /// A camera for this pass at `aspect` (width / height): at the origin, looking down -Z, with the vertical
    /// field of view that reproduces `widen_fov_x(fov_x_at_4_3, aspect)` as [`Camera`]'s own horizontal field
    /// of view at that same aspect ratio. `None` if [`ViewModel::fov_x_at_4_3`], [`ViewModel::near`] or
    /// `aspect` are not finite and in range ([`Camera::is_valid`]); refused by
    /// [`crate::Renderer::set_viewmodel`] (checked at the 4:3 aspect, where widening is the identity) rather
    /// than drawn.
    pub(crate) fn camera(&self, aspect: f32) -> Option<Camera> {
        Self::camera_for(self.fov_x_at_4_3, self.near, aspect)
    }

    /// As [`ViewModel::camera`], without needing a whole [`ViewModel`] (the viewmodel pass keeps only the
    /// field of view and near plane from frame to frame, not a [`Camera`], since the widening depends on the
    /// aspect ratio of whichever frame is drawn).
    pub(crate) fn camera_for(fov_x_at_4_3: f32, near: f32, aspect: f32) -> Option<Camera> {
        let fov_x = widen_fov_x(fov_x_at_4_3, aspect)?;
        // `Camera`'s own horizontal fov at `aspect` is `2 atan(aspect tan(fov_y/2))`; solved for the `fov_y`
        // that reproduces `fov_x` there.
        let fov_y = 2.0 * ((fov_x * 0.5).tan() / aspect).atan();
        let camera = Camera {
            position: Vec3::ZERO,
            rotation: Default::default(),
            projection: Projection::Perspective { fov_y, near },
        };
        camera.is_valid().then_some(camera)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_4_3_the_widening_is_the_identity() {
        // docs/re/weapon-view.md section 4 point 2: "At 4:3: unchanged."
        for fov_x in [0.531f32, 0.8262, 1.228] {
            let widened = widen_fov_x(fov_x, 4.0 / 3.0).unwrap();
            assert!((widened - fov_x).abs() < 1e-5, "fov_x {fov_x}: {widened}");
        }
    }

    #[test]
    fn widening_at_another_aspect_matches_the_spec_formula() {
        // docs/re/weapon-view.md section 4 point 2 / test vector 4: sniper FOV 0.5706 at 16:9 -> 0.7608.
        let widened = widen_fov_x(0.5706, 16.0 / 9.0).unwrap();
        assert!((widened - 0.7608).abs() < 1e-3, "widened fov {widened}");
    }

    #[test]
    fn the_camera_reproduces_the_widened_horizontal_fov_at_its_own_aspect() {
        for (fov_x_at_4_3, aspect) in [
            (0.531f32, 4.0 / 3.0),
            (0.8262, 16.0 / 9.0),
            (1.228, 1.0),
            (0.5706, 21.0 / 9.0),
        ] {
            let camera = ViewModel::camera_for(fov_x_at_4_3, 0.1, aspect).unwrap();
            let proj = camera.projection_matrix(aspect);
            // The clip-space half-width at z = -1 is `tan(fov_x(aspect) / 2)` (`m00 = cot(fov_y/2) / aspect`).
            let half_width = 1.0 / proj.x_axis.x;
            let fov_x = 2.0 * half_width.atan();
            let expected = widen_fov_x(fov_x_at_4_3, aspect).unwrap();
            assert!(
                (fov_x - expected).abs() < 1e-4,
                "fov_x_at_4_3 {fov_x_at_4_3}, aspect {aspect}: got {fov_x}, expected {expected}"
            );
        }
    }

    #[test]
    fn an_invalid_fov_near_or_aspect_has_no_camera() {
        let bad = |fov_x_at_4_3: f32, near: f32, aspect: f32| {
            ViewModel::camera_for(fov_x_at_4_3, near, aspect).is_none()
        };
        assert!(bad(f32::NAN, 0.1, 4.0 / 3.0));
        assert!(bad(0.8, f32::NAN, 4.0 / 3.0));
        assert!(bad(0.8, 0.0, 4.0 / 3.0), "near must be positive");
        assert!(bad(0.8, -0.1, 4.0 / 3.0));
        assert!(bad(0.0, 0.1, 4.0 / 3.0), "fov of zero");
        assert!(bad(-1.0, 0.1, 4.0 / 3.0), "negative fov");
        assert!(bad(0.8, 0.1, 0.0), "aspect must be positive");
        assert!(bad(0.8, 0.1, f32::NAN));
    }
}
