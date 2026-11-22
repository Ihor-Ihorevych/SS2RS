//! Frustum culling primitives and the sub-pixel jitter of temporal anti-aliasing (roadmap T04-004).
//!
//! A [`Frustum`] is built from a camera's world-to-clip matrix and answers, conservatively, whether
//! a point, a [`Sphere`] or an [`Aabb`] can be seen. "Conservatively" is the contract: a test may
//! say [`Visibility::Intersecting`] for something that is in fact just outside a corner (the
//! standard plane test does), but it **never** says [`Visibility::Outside`] for something that a
//! pixel could show, which would make objects vanish. Property tests check that against the
//! clip-space definition of "visible".
//!
//! Depth conventions are those of [`crate::Camera`]: right-handed, wgpu clip space (`z` in `0..=w`),
//! **reverse-Z** (near is depth 1) and an infinite far plane for perspective, which therefore has
//! no far plane here.

use crate::Camera;
use ss2_core::math::{Mat4, Vec2, Vec3, Vec4};

/// A plane with a unit normal. A point `p` is on the inside when `normal . p + d >= 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    /// The unit normal, pointing to the inside.
    pub normal: Vec3,
    /// The offset: the signed distance of the origin.
    pub d: f32,
}

impl Plane {
    /// A plane from the coefficients `(a, b, c, d)` of `a x + b y + c z + d >= 0`, normalized;
    /// `None` when the normal is (nearly) zero, which is how an infinite far plane shows up.
    fn from_coefficients(v: Vec4) -> Option<Self> {
        let length = v.truncate().length();
        (length > 1e-6 && length.is_finite()).then(|| Self {
            normal: v.truncate() / length,
            d: v.w / length,
        })
    }

    /// The signed distance of `p` from the plane (positive on the inside).
    pub fn distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }
}

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// The corner with the smallest coordinates.
    pub min: Vec3,
    /// The corner with the largest coordinates.
    pub max: Vec3,
}

impl Aabb {
    /// A box from two opposite corners.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// The smallest box that contains all `points`; `None` for no points.
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        let mut iter = points.into_iter();
        let first = iter.next()?;
        Some(iter.fold(Self::new(first, first), |b, p| {
            Self::new(b.min.min(p), b.max.max(p))
        }))
    }

    /// The centre of the box.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Half the size of the box along each axis.
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// The eight corners.
    pub fn corners(&self) -> [Vec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vec3::new(a.x, a.y, a.z),
            Vec3::new(b.x, a.y, a.z),
            Vec3::new(a.x, b.y, a.z),
            Vec3::new(b.x, b.y, a.z),
            Vec3::new(a.x, a.y, b.z),
            Vec3::new(b.x, a.y, b.z),
            Vec3::new(a.x, b.y, b.z),
            Vec3::new(b.x, b.y, b.z),
        ]
    }

    /// The box that contains this box after it is transformed by the affine matrix `m`: a
    /// rotated box grows (it is the bounding box of the rotated corners, computed without them).
    #[must_use]
    pub fn transformed(&self, m: Mat4) -> Self {
        let half = self.half_extents();
        let new_half = m.x_axis.truncate().abs() * half.x
            + m.y_axis.truncate().abs() * half.y
            + m.z_axis.truncate().abs() * half.z;
        let center = m.transform_point3(self.center());
        Self::new(center - new_half, center + new_half)
    }
}

/// A sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// The centre.
    pub center: Vec3,
    /// The radius.
    pub radius: f32,
}

impl Sphere {
    /// A sphere.
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// The sphere around a box (it contains every corner).
    pub fn from_aabb(b: Aabb) -> Self {
        Self::new(b.center(), b.half_extents().length())
    }
}

/// How a shape stands against a [`Frustum`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// Entirely outside: it can be skipped.
    Outside,
    /// Crosses at least one plane, or is close enough that the test cannot rule it out: draw it.
    Intersecting,
    /// Entirely inside.
    Inside,
}

/// The planes that bound what a camera can see (five for an infinite far plane, six otherwise).
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_render_api::{Aabb, Camera, Frustum, Visibility};
///
/// let frustum = Frustum::from_camera(&Camera::default(), 16.0 / 9.0);
/// let ahead = Aabb::new(Vec3::new(-1.0, 0.0, -11.0), Vec3::new(1.0, 2.0, -9.0));
/// let behind = Aabb::new(Vec3::new(-1.0, 0.0, 9.0), Vec3::new(1.0, 2.0, 11.0));
/// assert_ne!(frustum.test_aabb(ahead), Visibility::Outside); // draw it
/// assert_eq!(frustum.test_aabb(behind), Visibility::Outside); // skip it
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Frustum {
    planes: Vec<Plane>,
    /// A frustum made from an invalid camera or matrix sees nothing.
    sees_nothing: bool,
}

impl Frustum {
    /// The frustum of a world-to-clip matrix in wgpu clip space (`z` from 0 to `w`), by the
    /// Gribb and Hartmann method. A matrix with non-finite entries gives a frustum that sees
    /// nothing.
    pub fn from_view_projection(m: Mat4) -> Self {
        if !m.is_finite() {
            return Self {
                planes: Vec::new(),
                sees_nothing: true,
            };
        }
        let (r0, r1, r2, r3) = (m.row(0), m.row(1), m.row(2), m.row(3));
        let planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2]
            .into_iter()
            .filter_map(Plane::from_coefficients)
            .collect();
        Self {
            planes,
            sees_nothing: false,
        }
    }

    /// The frustum of `camera` for a viewport of `aspect` = width / height; an invalid camera
    /// (see [`Camera::is_valid`]) sees nothing.
    pub fn from_camera(camera: &Camera, aspect: f32) -> Self {
        if !camera.is_valid() || !(aspect.is_finite() && aspect > 0.0) {
            return Self {
                planes: Vec::new(),
                sees_nothing: true,
            };
        }
        Self::from_view_projection(camera.view_projection(aspect))
    }

    /// The planes, with unit normals pointing inwards.
    pub fn planes(&self) -> &[Plane] {
        &self.planes
    }

    /// Whether the point can be seen.
    pub fn contains_point(&self, p: Vec3) -> bool {
        !self.sees_nothing && self.planes.iter().all(|plane| plane.distance(p) >= 0.0)
    }

    /// Classify a sphere (conservative: never `Outside` for a sphere that can be seen).
    pub fn test_sphere(&self, s: Sphere) -> Visibility {
        if self.sees_nothing {
            return Visibility::Outside;
        }
        let mut result = Visibility::Inside;
        for plane in &self.planes {
            let distance = plane.distance(s.center);
            if distance < -s.radius {
                return Visibility::Outside;
            }
            if distance < s.radius {
                result = Visibility::Intersecting;
            }
        }
        result
    }

    /// Classify a box (conservative: never `Outside` for a box that can be seen).
    pub fn test_aabb(&self, b: Aabb) -> Visibility {
        if self.sees_nothing {
            return Visibility::Outside;
        }
        let (center, half) = (b.center(), b.half_extents());
        let mut result = Visibility::Inside;
        for plane in &self.planes {
            // How far the box reaches along the plane normal.
            let reach = half.dot(plane.normal.abs());
            let distance = plane.distance(center);
            if distance + reach < 0.0 {
                return Visibility::Outside;
            }
            if distance - reach < 0.0 {
                result = Visibility::Intersecting;
            }
        }
        result
    }
}

/// The length of the jitter sequence of [`jitter_for_frame`].
const JITTER_PERIOD: u32 = 8;

fn halton(mut index: u32, base: u32) -> f32 {
    let (mut f, mut r) = (1.0_f32, 0.0_f32);
    while index > 0 {
        f /= base as f32;
        r += f * (index % base) as f32;
        index /= base;
    }
    r
}

/// The sub-pixel offset, in normalized device coordinates, for frame number `frame` of a
/// viewport of `width` x `height` pixels: the Halton sequence of bases 2 and 3, repeating every 8
/// frames, within half a pixel of the centre. Feed it to [`Camera::projection_matrix_jittered`]
/// for temporal anti-aliasing. An empty viewport gives no jitter.
pub fn jitter_for_frame(frame: u32, width: u32, height: u32) -> Vec2 {
    if width == 0 || height == 0 {
        return Vec2::ZERO;
    }
    let index = frame % JITTER_PERIOD + 1;
    let pixel = Vec2::new(halton(index, 2) - 0.5, halton(index, 3) - 0.5);
    // One pixel is 2 / size in normalized device coordinates.
    Vec2::new(pixel.x * 2.0 / width as f32, pixel.y * 2.0 / height as f32)
}

impl Camera {
    /// [`Camera::projection_matrix`] with the image shifted by `jitter` in normalized device
    /// coordinates (see [`jitter_for_frame`]). Depth is untouched; a jitter of zero returns the
    /// plain matrix. Culling should use the **unjittered** frustum.
    pub fn projection_matrix_jittered(&self, aspect: f32, jitter: Vec2) -> Mat4 {
        Mat4::from_translation(Vec3::new(jitter.x, jitter.y, 0.0)) * self.projection_matrix(aspect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Camera, Projection};
    use ss2_core::math::{Mat4, Quat, Vec2, Vec3, Vec4};

    /// A camera at the origin looking down -Z with a 90 degree vertical field of view and
    /// aspect 1, near plane at 1: the visible region is |x| <= -z and |y| <= -z for z <= -1.
    fn camera() -> Camera {
        Camera {
            projection: Projection::Perspective {
                fov_y: std::f32::consts::FRAC_PI_2,
                near: 1.0,
            },
            ..Camera::default()
        }
    }

    fn frustum(camera: &Camera) -> Frustum {
        Frustum::from_camera(camera, 1.0)
    }

    /// The definition of visible: the point lands inside the clip volume.
    fn visible_in_clip(m: Mat4, p: Vec3) -> bool {
        let c = m * Vec4::new(p.x, p.y, p.z, 1.0);
        c.w > 0.0 && c.x.abs() <= c.w && c.y.abs() <= c.w && c.z >= 0.0 && c.z <= c.w
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) as f32) / (1_u64 << 31) as f32
        }
        fn range(&mut self, lo: f32, hi: f32) -> f32 {
            lo + (hi - lo) * self.next()
        }
        fn point(&mut self, extent: f32) -> Vec3 {
            Vec3::new(
                self.range(-extent, extent),
                self.range(-extent, extent),
                self.range(-extent, extent),
            )
        }
    }

    #[test]
    fn points_in_front_inside_the_angles_are_visible() {
        let f = frustum(&camera());
        assert!(f.contains_point(Vec3::new(0.0, 0.0, -5.0)));
        assert!(f.contains_point(Vec3::new(4.9, -4.9, -5.0)));
        assert!(
            f.contains_point(Vec3::new(0.0, 0.0, -1.0e6)),
            "no far plane: far things are visible"
        );
    }

    #[test]
    fn points_beside_behind_or_nearer_than_the_near_plane_are_not() {
        let f = frustum(&camera());
        assert!(
            !f.contains_point(Vec3::new(5.1, 0.0, -5.0)),
            "right of the view"
        );
        assert!(!f.contains_point(Vec3::new(-5.1, 0.0, -5.0)), "left");
        assert!(!f.contains_point(Vec3::new(0.0, 5.1, -5.0)), "above");
        assert!(!f.contains_point(Vec3::new(0.0, -5.1, -5.0)), "below");
        assert!(
            !f.contains_point(Vec3::new(0.0, 0.0, 5.0)),
            "behind the camera"
        );
        assert!(
            !f.contains_point(Vec3::new(0.0, 0.0, -0.5)),
            "nearer than the near plane"
        );
    }

    #[test]
    fn the_frustum_follows_the_camera() {
        let turned = Camera {
            position: Vec3::new(10.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), // looks along -X
            ..camera()
        };
        let f = frustum(&turned);
        assert!(
            f.contains_point(Vec3::new(10.0 - 5.0, 2.0, 3.0)),
            "five metres ahead"
        );
        assert!(
            !f.contains_point(Vec3::new(10.0 + 5.0, 2.0, 3.0)),
            "five metres behind"
        );
        assert!(
            !f.contains_point(Vec3::new(10.0, 2.0, 3.0 - 20.0)),
            "to the side"
        );
    }

    #[test]
    fn an_orthographic_camera_has_a_far_plane() {
        let ortho = Camera {
            projection: Projection::Orthographic {
                height: 10.0,
                near: 1.0,
                far: 50.0,
            },
            ..Camera::default()
        };
        let f = Frustum::from_camera(&ortho, 1.0);
        assert!(f.contains_point(Vec3::new(4.0, 4.0, -49.0)));
        assert!(
            !f.contains_point(Vec3::new(0.0, 0.0, -51.0)),
            "beyond the far plane"
        );
        assert!(
            !f.contains_point(Vec3::new(6.0, 0.0, -10.0)),
            "outside the 10 high, 10 wide box"
        );
    }

    #[test]
    fn spheres_are_classified_inside_intersecting_or_outside() {
        let f = frustum(&camera());
        assert_eq!(
            f.test_sphere(Sphere::new(Vec3::new(0.0, 0.0, -10.0), 1.0)),
            Visibility::Inside
        );
        assert_eq!(
            f.test_sphere(Sphere::new(Vec3::new(10.0, 0.0, -10.0), 1.0)),
            Visibility::Intersecting,
            "straddles the right plane"
        );
        assert_eq!(
            f.test_sphere(Sphere::new(Vec3::new(30.0, 0.0, -10.0), 1.0)),
            Visibility::Outside
        );
        assert_eq!(
            f.test_sphere(Sphere::new(Vec3::new(0.0, 0.0, 10.0), 2.0)),
            Visibility::Outside,
            "behind"
        );
        assert_eq!(
            f.test_sphere(Sphere::new(Vec3::new(0.0, 0.0, 0.0), 3.0)),
            Visibility::Intersecting,
            "the camera is inside it"
        );
    }

    #[test]
    fn boxes_are_classified_inside_intersecting_or_outside() {
        let f = frustum(&camera());
        let near_centre = Aabb::new(Vec3::new(-1.0, -1.0, -11.0), Vec3::new(1.0, 1.0, -9.0));
        assert_eq!(f.test_aabb(near_centre), Visibility::Inside);
        let straddling = Aabb::new(Vec3::new(9.0, -1.0, -11.0), Vec3::new(12.0, 1.0, -9.0));
        assert_eq!(f.test_aabb(straddling), Visibility::Intersecting);
        let off_to_the_side = Aabb::new(Vec3::new(40.0, -1.0, -11.0), Vec3::new(42.0, 1.0, -9.0));
        assert_eq!(f.test_aabb(off_to_the_side), Visibility::Outside);
        let behind = Aabb::new(Vec3::new(-1.0, -1.0, 5.0), Vec3::new(1.0, 1.0, 9.0));
        assert_eq!(f.test_aabb(behind), Visibility::Outside);
    }

    #[test]
    fn an_aabb_is_built_from_points_and_transformed_conservatively() {
        let b = Aabb::from_points([
            Vec3::new(1.0, 5.0, -2.0),
            Vec3::new(-3.0, 0.0, 4.0),
            Vec3::new(0.0, 2.0, 0.0),
        ])
        .unwrap();
        assert_eq!(
            (b.min, b.max),
            (Vec3::new(-3.0, 0.0, -2.0), Vec3::new(1.0, 5.0, 4.0))
        );
        assert!(Aabb::from_points(std::iter::empty()).is_none());
        // Rotating a box by 45 degrees about Y: the new box must contain every rotated corner.
        let m = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let rotated = b.transformed(m);
        for corner in b.corners() {
            let p = m.transform_point3(corner);
            assert!(
                p.cmpge(rotated.min - Vec3::splat(1e-4)).all()
                    && p.cmple(rotated.max + Vec3::splat(1e-4)).all(),
                "{p:?} escaped {rotated:?}"
            );
        }
        assert_eq!(b.center(), Vec3::new(-1.0, 2.5, 1.0));
        assert_eq!(b.half_extents(), Vec3::new(2.0, 2.5, 3.0));
    }

    #[test]
    fn a_sphere_that_contains_a_box_contains_all_its_corners() {
        let b = Aabb::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(4.0, 5.0, 6.0));
        let s = Sphere::from_aabb(b);
        for c in b.corners() {
            assert!(c.distance(s.center) <= s.radius + 1e-4);
        }
    }

    #[test]
    fn culling_a_point_never_disagrees_with_the_clip_space_definition() {
        let mut rng = Lcg(7);
        // The sample cube is sized to the view volume, so that enough points are visible.
        for (projection, extent) in [
            (
                Projection::Perspective {
                    fov_y: 1.1,
                    near: 0.3,
                },
                60.0,
            ),
            (
                Projection::Orthographic {
                    height: 8.0,
                    near: 0.5,
                    far: 40.0,
                },
                12.0,
            ),
        ] {
            let camera = Camera {
                position: Vec3::new(1.0, 2.0, 3.0),
                rotation: Quat::from_rotation_y(0.7)
                    * Quat::from_rotation_x(-0.3)
                    * Quat::from_rotation_z(0.1),
                projection,
            };
            let aspect = 16.0 / 9.0;
            let f = Frustum::from_camera(&camera, aspect);
            let m = camera.view_projection(aspect);
            let mut visible = 0;
            for _ in 0..20_000 {
                let p = camera.position + rng.point(extent);
                let truth = visible_in_clip(m, p);
                visible += usize::from(truth);
                // Allow the planes to disagree only within float rounding of the boundary.
                if truth != f.contains_point(p) {
                    let clip = m * Vec4::new(p.x, p.y, p.z, 1.0);
                    let margin = [
                        clip.w - clip.x.abs(),
                        clip.w - clip.y.abs(),
                        clip.z,
                        clip.w - clip.z,
                    ]
                    .into_iter()
                    .fold(f32::MAX, f32::min)
                    .abs();
                    assert!(
                        margin < 1e-3 * clip.w.abs().max(1.0),
                        "{p:?} truth {truth} but the frustum says {}",
                        !truth
                    );
                }
            }
            assert!(visible > 150, "the test must see visible points: {visible}");
        }
    }

    #[test]
    fn a_sphere_or_box_with_a_visible_point_is_never_called_outside() {
        let mut rng = Lcg(99);
        let camera = Camera {
            position: Vec3::new(-4.0, 1.0, 2.0),
            rotation: Quat::from_rotation_y(-1.2) * Quat::from_rotation_x(0.2),
            projection: Projection::Perspective {
                fov_y: 0.9,
                near: 0.2,
            },
        };
        let aspect = 1.6;
        let f = Frustum::from_camera(&camera, aspect);
        let m = camera.view_projection(aspect);
        let mut checked = 0;
        for _ in 0..20_000 {
            let centre = camera.position + rng.point(40.0);
            let half = Vec3::new(
                rng.range(0.1, 6.0),
                rng.range(0.1, 6.0),
                rng.range(0.1, 6.0),
            );
            let aabb = Aabb::new(centre - half, centre + half);
            let sphere = Sphere::new(centre, rng.range(0.1, 6.0));
            // Sample points inside each shape; if any is visible, the test must not say Outside.
            let mut box_sees = false;
            let mut sphere_sees = false;
            for _ in 0..12 {
                let t = Vec3::new(
                    rng.range(-1.0, 1.0),
                    rng.range(-1.0, 1.0),
                    rng.range(-1.0, 1.0),
                );
                box_sees |= visible_in_clip(m, centre + t * half);
                let dir = t.normalize_or_zero();
                sphere_sees |= visible_in_clip(m, sphere.center + dir * sphere.radius * rng.next());
            }
            if box_sees {
                checked += 1;
                assert_ne!(f.test_aabb(aabb), Visibility::Outside, "{aabb:?}");
            }
            if sphere_sees {
                assert_ne!(f.test_sphere(sphere), Visibility::Outside, "{sphere:?}");
            }
        }
        assert!(checked > 300, "the test must see visible boxes: {checked}");
    }

    #[test]
    fn an_invalid_matrix_gives_a_frustum_that_sees_nothing_not_everything() {
        let f = Frustum::from_view_projection(Mat4::from_cols_array(&[f32::NAN; 16]));
        assert!(!f.contains_point(Vec3::ZERO));
        assert_eq!(
            f.test_sphere(Sphere::new(Vec3::ZERO, 1.0e6)),
            Visibility::Outside
        );
    }

    #[test]
    fn jitter_of_zero_changes_nothing_and_shifts_the_image_by_the_given_ndc_amount() {
        let camera = camera();
        let plain = camera.projection_matrix(1.0);
        assert_eq!(camera.projection_matrix_jittered(1.0, Vec2::ZERO), plain);
        let jitter = Vec2::new(0.01, -0.02);
        let shifted = camera.projection_matrix_jittered(1.0, jitter);
        // The point straight ahead lands at the jitter in normalized device coordinates.
        let clip = shifted * Vec4::new(0.0, 0.0, -10.0, 1.0);
        assert!(
            (clip.x / clip.w - jitter.x).abs() < 1e-6 && (clip.y / clip.w - jitter.y).abs() < 1e-6,
            "{clip:?}"
        );
        // Depth is not touched.
        let unshifted = plain * Vec4::new(0.0, 0.0, -10.0, 1.0);
        assert!((clip.z / clip.w - unshifted.z / unshifted.w).abs() < 1e-6);
    }

    #[test]
    fn the_jitter_sequence_is_sub_pixel_deterministic_and_covers_the_pixel() {
        let (w, h) = (1920, 1080);
        let mut seen = std::collections::BTreeSet::new();
        for frame in 0..8 {
            let j = jitter_for_frame(frame, w, h);
            assert!(
                j.x.abs() <= 1.0 / w as f32 + 1e-9 && j.y.abs() <= 1.0 / h as f32 + 1e-9,
                "{j:?} is more than half a pixel in NDC"
            );
            seen.insert((j.x.to_bits(), j.y.to_bits()));
            assert_eq!(j, jitter_for_frame(frame, w, h), "deterministic");
        }
        assert_eq!(seen.len(), 8, "eight distinct sub-pixel positions");
        assert_eq!(
            jitter_for_frame(8, w, h),
            jitter_for_frame(0, w, h),
            "the sequence repeats every 8 frames"
        );
        assert_eq!(
            jitter_for_frame(0, 0, 0),
            Vec2::ZERO,
            "an empty viewport is no jitter, not a division by zero"
        );
    }

    #[test]
    fn the_planes_have_unit_normals() {
        let f = frustum(&camera());
        for plane in f.planes() {
            assert!((plane.normal.length() - 1.0).abs() < 1e-5, "{plane:?}");
        }
        assert_eq!(f.planes().len(), 5, "no far plane with an infinite far");
        assert_eq!(
            Frustum::from_camera(
                &Camera {
                    projection: Projection::Orthographic {
                        height: 4.0,
                        near: 0.1,
                        far: 9.0
                    },
                    ..Camera::default()
                },
                1.0
            )
            .planes()
            .len(),
            6
        );
    }
}
