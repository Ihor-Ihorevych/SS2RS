//! The camera: where the player looks from and how the view is projected.

use ss2_core::math::{Mat4, Quat, Vec3, projection};

/// How the scene is projected.
///
/// Depth is **reverse-Z** (near plane maps to depth `1`, far to `0`) for precision, and
/// perspective projections have an **infinite far plane** (roadmap T04-004).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// Perspective with an infinite far plane.
    Perspective {
        /// Vertical field of view in radians, in `(0, π)`.
        fov_y: f32,
        /// Near plane distance, `> 0`.
        near: f32,
    },
    /// Orthographic (editor views, some UI-in-world uses).
    Orthographic {
        /// Height of the view volume in world units, `> 0`; width follows the aspect ratio.
        height: f32,
        /// Near plane distance.
        near: f32,
        /// Far plane distance, `> near`.
        far: f32,
    },
}

/// A camera in the engine's right-handed, Y-up, -Z-forward space (see `ss2_core::math`).
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_render_api::Camera;
///
/// let cam = Camera { position: Vec3::new(0.0, 1.7, 5.0), ..Camera::default() };
/// assert_eq!(cam.forward(), Vec3::NEG_Z);
/// assert!(cam.is_valid());
/// let _clip_from_world = cam.view_projection(16.0 / 9.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// World-space position.
    pub position: Vec3,
    /// World-space orientation (identity looks down -Z with +Y up).
    pub rotation: Quat,
    /// Projection parameters.
    pub projection: Projection,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            projection: Projection::Perspective {
                fov_y: 60f32.to_radians(),
                near: 0.1,
            },
        }
    }
}

impl Camera {
    /// Direction the camera looks along.
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// The camera's up direction.
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// The camera's right direction.
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// World → view matrix.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position).inverse()
    }

    /// View → clip matrix for a viewport of `aspect` = width / height.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        match self.projection {
            Projection::Perspective { fov_y, near } => {
                projection::perspective_infinite_reverse(fov_y, aspect, near)
            }
            Projection::Orthographic { height, near, far } => {
                let half_h = height * 0.5;
                let half_w = half_h * aspect;
                // Swapping near and far reverses the depth range.
                projection::orthographic(-half_w, half_w, -half_h, half_h, far, near)
            }
        }
    }

    /// World → clip matrix.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// `true` if every value is finite, the rotation is (nearly) normalized and the projection
    /// parameters are in range. Extraction code may assert this in debug builds; renderers must
    /// reject (not draw with) an invalid camera instead of producing NaN matrices.
    pub fn is_valid(&self) -> bool {
        let projection_ok = match self.projection {
            Projection::Perspective { fov_y, near } => {
                fov_y.is_finite()
                    && fov_y > 0.0
                    && fov_y < std::f32::consts::PI
                    && near.is_finite()
                    && near > 0.0
            }
            Projection::Orthographic { height, near, far } => {
                height.is_finite()
                    && height > 0.0
                    && near.is_finite()
                    && far.is_finite()
                    && far > near
            }
        };
        projection_ok
            && self.position.is_finite()
            && self.rotation.is_finite()
            && (self.rotation.length_squared() - 1.0).abs() < 1e-3
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use ss2_core::math::Vec4;

    use super::*;

    fn depth_of(m: Mat4, view_space_z: f32) -> f32 {
        let clip = m * Vec4::new(0.0, 0.0, view_space_z, 1.0);
        clip.z / clip.w
    }

    #[test]
    fn identity_camera_looks_down_negative_z() {
        let cam = Camera::default();
        assert_eq!(cam.forward(), Vec3::NEG_Z);
        assert_eq!(cam.up(), Vec3::Y);
        assert_eq!(cam.right(), Vec3::X);
        assert!(cam.view_matrix().abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    #[test]
    fn view_matrix_moves_world_into_camera_space() {
        let cam = Camera {
            position: Vec3::new(10.0, 0.0, 0.0),
            ..Camera::default()
        };
        let p = cam
            .view_matrix()
            .transform_point3(Vec3::new(10.0, 0.0, -5.0));
        assert!(p.abs_diff_eq(Vec3::new(0.0, 0.0, -5.0), 1e-5), "{p}");
    }

    #[test]
    fn turning_left_points_forward_at_negative_x() {
        let cam = Camera {
            rotation: Quat::from_rotation_y(FRAC_PI_2),
            ..Camera::default()
        };
        assert!(
            cam.forward().abs_diff_eq(Vec3::NEG_X, 1e-6),
            "{}",
            cam.forward()
        );
    }

    #[test]
    fn perspective_is_reverse_z_with_infinite_far() {
        let cam = Camera::default();
        let proj = cam.projection_matrix(1.0);
        assert!((depth_of(proj, -0.1) - 1.0).abs() < 1e-5, "near -> 1");
        let far = depth_of(proj, -1.0e6);
        assert!(far > 0.0 && far < 1e-5, "very far -> ~0, got {far}");
        assert!(
            depth_of(proj, -1.0) > depth_of(proj, -10.0),
            "closer is larger"
        );
    }

    #[test]
    fn orthographic_is_reverse_z_between_near_and_far() {
        let cam = Camera {
            projection: Projection::Orthographic {
                height: 10.0,
                near: 1.0,
                far: 100.0,
            },
            ..Camera::default()
        };
        let proj = cam.projection_matrix(2.0);
        assert!((depth_of(proj, -1.0) - 1.0).abs() < 1e-5);
        assert!(depth_of(proj, -100.0).abs() < 1e-5);
        // Aspect 2: x in [-10, 10] maps to [-1, 1]
        let clip = proj * Vec4::new(10.0, 0.0, -5.0, 1.0);
        assert!((clip.x - 1.0).abs() < 1e-5);
    }

    #[test]
    fn validity_rejects_garbage() {
        assert!(Camera::default().is_valid());
        let bad = |f: &dyn Fn(&mut Camera)| {
            let mut c = Camera::default();
            f(&mut c);
            assert!(!c.is_valid(), "{c:?}");
        };
        bad(&|c| c.position.x = f32::NAN);
        bad(&|c| c.position.y = f32::INFINITY);
        bad(&|c| c.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 2.0));
        bad(&|c| {
            c.projection = Projection::Perspective {
                fov_y: 0.0,
                near: 0.1,
            };
        });
        bad(&|c| {
            c.projection = Projection::Perspective {
                fov_y: 1.0,
                near: 0.0,
            };
        });
        bad(&|c| {
            c.projection = Projection::Perspective {
                fov_y: 4.0,
                near: 0.1,
            };
        });
        bad(&|c| {
            c.projection = Projection::Orthographic {
                height: 1.0,
                near: 5.0,
                far: 5.0,
            };
        });
    }
}
