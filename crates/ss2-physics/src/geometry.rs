//! Geometry primitives and the exact queries between them (roadmap T07-001).
//!
//! Shapes: [`Aabb`], [`Sphere`], [`Capsule`], [`Obb`] (an oriented box) and [`Plane`]; a [`Ray`];
//! and the results [`RayHit`] and [`SweepHit`]. Queries: ray casts against every shape, overlap
//! tests, closest points, signed distances and **sweeps** (moving a sphere or a box along a line and
//! finding when it first touches something). Everything is on `glam` vectors in the engine's
//! space (right-handed, Y up, -Z forward; `docs/GEOMETRY.md`).
//!
//! **Platform-stable by construction:** only `+ - * /`, `sqrt`, `min`, `max`, `abs` and comparisons
//! on `f32`, no trigonometry (whose results differ between math libraries) and no fused
//! multiply-add, so the same inputs give the same bits on x86_64 and aarch64 (the cross-architecture
//! check is T07-026). Solid shapes are **solid**: a ray that starts inside one hits it at `t = 0`.

use ss2_core::math::{Quat, Vec3};

const EPS: f32 = 1e-12;

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// A half-line: an origin and a **unit** direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    /// Where it starts.
    pub origin: Vec3,
    /// The direction, length 1.
    pub dir: Vec3,
}

impl Ray {
    /// A ray from `origin` along `dir` (normalised); `None` for a zero or non-finite direction or origin.
    pub fn new(origin: Vec3, dir: Vec3) -> Option<Self> {
        if !origin.is_finite() || !dir.is_finite() || dir.length_squared() <= EPS {
            return None;
        }
        Some(Self {
            origin,
            dir: dir.normalize(),
        })
    }

    /// The point `t` along the ray.
    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}

/// Where and how a ray met a shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    /// The distance along the ray.
    pub t: f32,
    /// The point.
    pub point: Vec3,
    /// The surface normal there (unit, facing the ray); for a ray that started inside, the
    /// reverse of its direction.
    pub normal: Vec3,
    /// The ray started inside the (solid) shape: `t` is 0.
    pub inside: bool,
}

/// When a moving shape first touches something.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepHit {
    /// How far along the motion, from 0 (at the start) to 1 (at the end).
    pub t: f32,
    /// The moving shape's centre at that moment.
    pub position: Vec3,
    /// The contact normal: unit, pointing from the thing that was hit toward the moving shape. When
    /// the shapes already overlap it is the direction to push the moving one out.
    pub normal: Vec3,
    /// The shapes already interpenetrated at the start (`t` is 0).
    pub overlapping_at_start: bool,
}

enum Slab {
    Miss,
    Inside,
    Hit { t: f32, normal: Vec3 },
}

/// A ray against the box `lo..hi` by the slab method. `Inside` means strictly inside; a ray that
/// starts on the surface and goes inward hits at 0, one that goes outward or along it misses.
fn slab(origin: Vec3, dir: Vec3, lo: Vec3, hi: Vec3, max_t: f32) -> Slab {
    if origin.cmpgt(lo).all() && origin.cmplt(hi).all() {
        return Slab::Inside;
    }
    let (mut t_min, mut t_max) = (0.0_f32, max_t);
    let mut normal = Vec3::ZERO;
    for axis in 0..3 {
        let (o, d, low, high) = (origin[axis], dir[axis], lo[axis], hi[axis]);
        if d == 0.0 {
            if o < low || o > high {
                return Slab::Miss;
            }
            continue;
        }
        let inv = 1.0 / d;
        let (mut t1, mut t2) = ((low - o) * inv, (high - o) * inv);
        let mut sign = -1.0;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            sign = 1.0;
        }
        if t1 >= t_min {
            t_min = t1;
            normal = Vec3::ZERO;
            normal[axis] = sign;
        }
        t_max = t_max.min(t2);
        if t_min >= t_max {
            return Slab::Miss;
        }
    }
    if normal == Vec3::ZERO {
        return Slab::Miss;
    }
    Slab::Hit { t: t_min, normal }
}

fn point_hit(ray: &Ray, t: f32, normal: Vec3) -> RayHit {
    RayHit {
        t,
        point: ray.point_at(t),
        normal,
        inside: false,
    }
}

fn inside_hit(ray: &Ray) -> RayHit {
    RayHit {
        t: 0.0,
        point: ray.origin,
        normal: -ray.dir,
        inside: true,
    }
}

/// An axis-aligned box.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_physics::geometry::{Aabb, Ray, Sphere};
///
/// let wall = Aabb::new(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 3.0, 1.0));
/// let ray = Ray::new(Vec3::new(0.0, 1.0, 10.0), Vec3::NEG_Z).unwrap();
/// let hit = wall.raycast(&ray, 100.0).unwrap();
/// assert_eq!((hit.t, hit.normal), (9.0, Vec3::Z));
///
/// // A sphere of radius 0.5 pushed 20 units toward the wall stops half a unit short of it.
/// let ball = Sphere::new(Vec3::new(0.0, 1.0, 10.0), 0.5);
/// let stop = ball.sweep_aabb(Vec3::new(0.0, 0.0, -20.0), &wall).unwrap();
/// assert!((stop.position.z - 1.5).abs() < 1e-5);
/// ```
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

    /// A box from its centre and half sizes.
    pub fn from_center_half(center: Vec3, half: Vec3) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// The centre.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Half the size along each axis.
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// The box grown by `amount` on every side.
    #[must_use]
    pub fn expanded(&self, amount: f32) -> Self {
        Self {
            min: self.min - Vec3::splat(amount),
            max: self.max + Vec3::splat(amount),
        }
    }

    /// The smallest box containing both.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Whether `p` is inside or on the surface.
    pub fn contains_point(&self, p: Vec3) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    /// Whether the boxes share any point (touching counts).
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    fn overlaps_strictly(&self, other: &Self) -> bool {
        self.min.cmplt(other.max).all() && self.max.cmpgt(other.min).all()
    }

    /// The point of the box nearest to `p` (`p` itself when inside).
    pub fn closest_point(&self, p: Vec3) -> Vec3 {
        p.clamp(self.min, self.max)
    }

    /// The distance from `p` to the surface: negative inside.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        let q = (p - self.center()).abs() - self.half_extents();
        q.max(Vec3::ZERO).length() + q.x.max(q.y.max(q.z)).min(0.0)
    }

    /// Cast a ray, at most `max_t` long.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        match slab(ray.origin, ray.dir, self.min, self.max, max_t) {
            Slab::Miss => None,
            Slab::Inside => Some(inside_hit(ray)),
            Slab::Hit { t, normal } => Some(point_hit(ray, t, normal)),
        }
    }

    /// Move this box by `delta` and find when it first touches `other`. Exact: the moving box's
    /// centre is a ray against `other` grown by this box's half sizes.
    pub fn sweep_aabb(&self, delta: Vec3, other: &Aabb) -> Option<SweepHit> {
        if !delta.is_finite() {
            return None;
        }
        let center = self.center();
        if self.overlaps_strictly(other) {
            return Some(SweepHit {
                t: 0.0,
                position: center,
                normal: separation_normal_boxes(self, other),
                overlapping_at_start: true,
            });
        }
        let len = delta.length();
        if len <= EPS {
            return None;
        }
        let half = self.half_extents();
        match slab(center, delta / len, other.min - half, other.max + half, len) {
            Slab::Hit { t, normal } => {
                let t = t / len;
                Some(SweepHit {
                    t,
                    position: center + delta * t,
                    normal,
                    overlapping_at_start: false,
                })
            }
            Slab::Miss | Slab::Inside => None,
        }
    }
}

/// The direction in which to push `a` out of `b`: along the axis with the least overlap.
fn separation_normal_boxes(a: &Aabb, b: &Aabb) -> Vec3 {
    let overlap_pos = Vec3::new(b.max.x - a.min.x, b.max.y - a.min.y, b.max.z - a.min.z); // pushing a toward +
    let overlap_neg = Vec3::new(a.max.x - b.min.x, a.max.y - b.min.y, a.max.z - b.min.z); // pushing a toward -
    let mut best = (f32::MAX, Vec3::Y);
    for axis in 0..3 {
        let mut unit = Vec3::ZERO;
        unit[axis] = 1.0;
        if overlap_pos[axis] < best.0 {
            best = (overlap_pos[axis], unit);
        }
        if overlap_neg[axis] < best.0 {
            best = (overlap_neg[axis], -unit);
        }
    }
    best.1
}

/// A sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// The centre.
    pub center: Vec3,
    /// The radius, at least 0.
    pub radius: f32,
}

impl Sphere {
    /// A sphere.
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// The distance from `p` to the surface: negative inside.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        (p - self.center).length() - self.radius
    }

    /// Whether the spheres share any point.
    pub fn overlaps_sphere(&self, other: &Self) -> bool {
        let r = self.radius + other.radius;
        (self.center - other.center).length_squared() <= r * r
    }

    /// Whether the sphere and the box share any point.
    pub fn overlaps_aabb(&self, b: &Aabb) -> bool {
        (self.center - b.closest_point(self.center)).length_squared() <= self.radius * self.radius
    }

    /// Cast a ray, at most `max_t` long.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        let m = ray.origin - self.center;
        let b = m.dot(ray.dir);
        let c = m.length_squared() - self.radius * self.radius;
        if c < 0.0 {
            return Some(inside_hit(ray));
        }
        if c >= 0.0 && b >= 0.0 {
            return None; // outside or on the surface, and not approaching
        }
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let t = (-b - disc.sqrt()).max(0.0);
        if t > max_t {
            return None;
        }
        let point = ray.point_at(t);
        let normal = if self.radius > EPS {
            (point - self.center) / self.radius
        } else {
            -ray.dir
        };
        Some(RayHit {
            t,
            point,
            normal,
            inside: false,
        })
    }

    /// Move this sphere by `delta` and find when it first touches `target`.
    pub fn sweep_sphere(&self, delta: Vec3, target: &Sphere) -> Option<SweepHit> {
        if !delta.is_finite() {
            return None;
        }
        let sum = self.radius + target.radius;
        let apart = self.center - target.center;
        if apart.length_squared() < sum * sum {
            let normal = apart.try_normalize().unwrap_or(Vec3::Y);
            return Some(SweepHit {
                t: 0.0,
                position: self.center,
                normal,
                overlapping_at_start: true,
            });
        }
        let len = delta.length();
        let ray = Ray::new(self.center, delta).filter(|_| len > EPS)?;
        let hit = Sphere::new(target.center, sum).raycast(&ray, len)?;
        let t = hit.t / len;
        Some(SweepHit {
            t,
            position: hit.point,
            normal: hit.normal,
            overlapping_at_start: false,
        })
    }

    /// Move this sphere by `delta` and find when it first touches the front of `plane`
    /// (the half-space behind the plane is solid).
    pub fn sweep_plane(&self, delta: Vec3, plane: &Plane) -> Option<SweepHit> {
        if !delta.is_finite() {
            return None;
        }
        let d0 = plane.signed_distance(self.center);
        if d0 < self.radius {
            return Some(SweepHit {
                t: 0.0,
                position: self.center,
                normal: plane.normal,
                overlapping_at_start: true,
            });
        }
        let approach = plane.normal.dot(delta);
        if approach >= 0.0 {
            return None;
        }
        let t = (self.radius - d0) / approach;
        (t <= 1.0).then(|| SweepHit {
            t,
            position: self.center + delta * t,
            normal: plane.normal,
            overlapping_at_start: false,
        })
    }

    /// Move this sphere by `delta` and find when it first touches `b`: the sphere's centre is a
    /// ray against the box grown by the radius, whose edges and corners are rounded: three slabs
    /// (the box grown along one axis) and twelve capsules along the edges (their round ends are the
    /// corners) make up exactly that shape.
    pub fn sweep_aabb(&self, delta: Vec3, b: &Aabb) -> Option<SweepHit> {
        if !delta.is_finite() {
            return None;
        }
        let r = self.radius;
        let closest = b.closest_point(self.center);
        if (self.center - closest).length_squared() < r * r {
            let normal = if b.contains_point(self.center) {
                // Centre inside the box: out through the nearest face.
                let probe = Aabb::new(self.center, self.center);
                separation_normal_boxes(&probe, b)
            } else {
                (self.center - closest).try_normalize().unwrap_or(Vec3::Y)
            };
            return Some(SweepHit {
                t: 0.0,
                position: self.center,
                normal,
                overlapping_at_start: true,
            });
        }
        let len = delta.length();
        let ray = Ray::new(self.center, delta).filter(|_| len > EPS)?;
        let mut best: Option<(f32, Vec3)> = None;
        let mut consider = |hit: Option<RayHit>| {
            if let Some(h) = hit
                && best.is_none_or(|(t, _)| h.t < t)
            {
                best = Some((h.t, h.normal));
            }
        };
        for axis in 0..3 {
            let mut grow = Vec3::ZERO;
            grow[axis] = r;
            consider(Aabb::new(b.min - grow, b.max + grow).raycast(&ray, len));
        }
        let corner = |bits: usize| {
            Vec3::new(
                if bits & 1 == 0 { b.min.x } else { b.max.x },
                if bits & 2 == 0 { b.min.y } else { b.max.y },
                if bits & 4 == 0 { b.min.z } else { b.max.z },
            )
        };
        for bits in 0..8_usize {
            for axis_bit in [1_usize, 2, 4] {
                if bits & axis_bit == 0 {
                    consider(
                        Capsule::new(corner(bits), corner(bits | axis_bit), r).raycast(&ray, len),
                    );
                }
            }
        }
        let (distance, face_normal) = best?;
        let t = distance / len;
        let position = self.center + delta * t;
        let normal = (position - b.closest_point(position))
            .try_normalize()
            .filter(|_| r > EPS)
            .unwrap_or(face_normal);
        Some(SweepHit {
            t,
            position,
            normal,
            overlapping_at_start: false,
        })
    }
}

/// A plane with a unit normal. The half-space behind it (where the signed distance is negative)
/// counts as solid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    /// The unit normal, pointing to the open side.
    pub normal: Vec3,
    /// The signed distance of the origin.
    pub d: f32,
}

impl Plane {
    /// The plane through `point` with the given `normal` (normalised); `None` for a zero or
    /// non-finite normal.
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Option<Self> {
        if !point.is_finite() || !normal.is_finite() || normal.length_squared() <= EPS {
            return None;
        }
        let normal = normal.normalize();
        Some(Self {
            normal,
            d: -normal.dot(point),
        })
    }

    /// The signed distance from the plane: positive on the open side.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }

    /// Cast a ray, at most `max_t` long; only the open side can be hit.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        let side = self.signed_distance(ray.origin);
        if side < 0.0 {
            return Some(inside_hit(ray));
        }
        let approach = self.normal.dot(ray.dir);
        if approach >= 0.0 {
            return None;
        }
        let t = -side / approach;
        (t <= max_t).then(|| point_hit(ray, t, self.normal))
    }
}

fn closest_on_segment(a: Vec3, b: Vec3, p: Vec3) -> Vec3 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= EPS {
        return a;
    }
    a + ab * clamp01((p - a).dot(ab) / len2)
}

/// The squared distance between segments `p1..q1` and `p2..q2`.
fn segment_segment_distance_squared(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t);
    if a <= EPS && e <= EPS {
        return r.length_squared();
    }
    if a <= EPS {
        s = 0.0;
        t = clamp01(f / e);
    } else {
        let c = d1.dot(r);
        if e <= EPS {
            t = 0.0;
            s = clamp01(-c / a);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s_try = if denom > EPS {
                clamp01((b * f - c * e) / denom)
            } else {
                0.0
            };
            let mut t_try = (b * s_try + f) / e;
            if t_try < 0.0 {
                t_try = 0.0;
                s_try = clamp01(-c / a);
            } else if t_try > 1.0 {
                t_try = 1.0;
                s_try = clamp01((b - c) / a);
            }
            s = s_try;
            t = t_try;
        }
    }
    ((p1 + d1 * s) - (p2 + d2 * t)).length_squared()
}

/// A capsule: every point within `radius` of the segment `a..b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    /// One end of the axis.
    pub a: Vec3,
    /// The other end of the axis.
    pub b: Vec3,
    /// The radius.
    pub radius: f32,
}

impl Capsule {
    /// A capsule around the segment `a..b`.
    pub fn new(a: Vec3, b: Vec3, radius: f32) -> Self {
        Self { a, b, radius }
    }

    /// The distance from `p` to the surface: negative inside.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        (p - closest_on_segment(self.a, self.b, p)).length() - self.radius
    }

    /// Whether the capsule and the sphere share any point.
    pub fn overlaps_sphere(&self, s: &Sphere) -> bool {
        let r = self.radius + s.radius;
        (s.center - closest_on_segment(self.a, self.b, s.center)).length_squared() <= r * r
    }

    /// Whether the capsules share any point.
    pub fn overlaps_capsule(&self, other: &Self) -> bool {
        let r = self.radius + other.radius;
        segment_segment_distance_squared(self.a, self.b, other.a, other.b) <= r * r
    }

    /// Whether the capsule and the box share any point. The distance from a point of the axis to
    /// the box is convex along the axis, so a ternary search finds its minimum.
    pub fn overlaps_aabb(&self, b: &Aabb) -> bool {
        let distance_at = |t: f32| {
            let p = self.a + (self.b - self.a) * t;
            (p - b.closest_point(p)).length()
        };
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..40 {
            let third = (hi - lo) / 3.0;
            let (m1, m2) = (lo + third, hi - third);
            if distance_at(m1) < distance_at(m2) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
        distance_at((lo + hi) * 0.5)
            .min(distance_at(0.0))
            .min(distance_at(1.0))
            <= self.radius
    }

    /// Cast a ray, at most `max_t` long.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        if self.signed_distance(ray.origin) < 0.0 {
            return Some(inside_hit(ray));
        }
        let mut best = f32::MAX;
        let mut take = |t: f32| {
            // A hit a rounding error behind the origin is a hit at the origin.
            if t >= -1e-5 && t <= max_t {
                best = best.min(t.max(0.0));
            }
        };
        for end in [self.a, self.b] {
            if let Some(h) = Sphere::new(end, self.radius).raycast(ray, max_t) {
                take(h.t);
            }
        }
        let axis = self.b - self.a;
        let length = axis.length();
        if length > 1e-6 {
            let u = axis / length;
            let m = ray.origin - self.a;
            let (m_perp, n_perp) = (m - u * m.dot(u), ray.dir - u * ray.dir.dot(u));
            let qa = n_perp.dot(n_perp);
            if qa > EPS {
                let qb = m_perp.dot(n_perp);
                let qc = m_perp.dot(m_perp) - self.radius * self.radius;
                let disc = qb * qb - qa * qc;
                if disc >= 0.0 {
                    let t = (-qb - disc.sqrt()) / qa;
                    let along = (m + ray.dir * t).dot(u);
                    if (0.0..=length).contains(&along) {
                        take(t);
                    }
                }
            }
        }
        if best == f32::MAX {
            return None;
        }
        let point = ray.point_at(best);
        let normal = (point - closest_on_segment(self.a, self.b, point))
            .try_normalize()
            .unwrap_or(-ray.dir);
        Some(RayHit {
            t: best,
            point,
            normal,
            inside: false,
        })
    }
}

/// An oriented box: a box turned by `rotation` about its centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obb {
    /// The centre.
    pub center: Vec3,
    /// Half the size along each of the box's own axes.
    pub half_extents: Vec3,
    /// The orientation (normalised on construction).
    pub rotation: Quat,
}

impl Obb {
    /// An oriented box.
    pub fn new(center: Vec3, half_extents: Vec3, rotation: Quat) -> Self {
        Self {
            center,
            half_extents,
            rotation: rotation.normalize(),
        }
    }

    fn to_local(&self, p: Vec3) -> Vec3 {
        self.rotation.inverse() * (p - self.center)
    }

    fn local_box(&self) -> Aabb {
        Aabb::new(-self.half_extents, self.half_extents)
    }

    /// The distance from `p` to the surface: negative inside.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        self.local_box().signed_distance(self.to_local(p))
    }

    /// Whether `p` is inside or on the surface.
    pub fn contains_point(&self, p: Vec3) -> bool {
        self.local_box().contains_point(self.to_local(p))
    }

    /// The point of the box nearest to `p`.
    pub fn closest_point(&self, p: Vec3) -> Vec3 {
        self.center + self.rotation * self.local_box().closest_point(self.to_local(p))
    }

    /// The smallest axis-aligned box that contains this one.
    pub fn bounding_aabb(&self) -> Aabb {
        let h = self.half_extents;
        let reach = (self.rotation * Vec3::X).abs() * h.x
            + (self.rotation * Vec3::Y).abs() * h.y
            + (self.rotation * Vec3::Z).abs() * h.z;
        Aabb::from_center_half(self.center, reach)
    }

    /// Cast a ray, at most `max_t` long.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        let inverse = self.rotation.inverse();
        let (origin, dir) = (inverse * (ray.origin - self.center), inverse * ray.dir);
        match slab(origin, dir, -self.half_extents, self.half_extents, max_t) {
            Slab::Miss => None,
            Slab::Inside => Some(inside_hit(ray)),
            Slab::Hit { t, normal } => Some(point_hit(ray, t, self.rotation * normal)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::math::{Quat, Vec3};

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
        fn vec(&mut self, extent: f32) -> Vec3 {
            Vec3::new(
                self.range(-extent, extent),
                self.range(-extent, extent),
                self.range(-extent, extent),
            )
        }
        fn unit(&mut self) -> Vec3 {
            loop {
                let v = self.vec(1.0);
                if v.length_squared() > 0.01 && v.length_squared() <= 1.0 {
                    return v.normalize();
                }
            }
        }
    }

    fn ray(origin: Vec3, dir: Vec3) -> Ray {
        Ray::new(origin, dir).expect("a usable ray")
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    // ---- rays -------------------------------------------------------------------------------

    #[test]
    fn a_ray_needs_a_finite_nonzero_direction_and_is_normalised() {
        assert!(Ray::new(Vec3::ZERO, Vec3::ZERO).is_none());
        assert!(Ray::new(Vec3::ZERO, Vec3::new(f32::NAN, 0.0, 0.0)).is_none());
        assert!(Ray::new(Vec3::new(f32::INFINITY, 0.0, 0.0), Vec3::X).is_none());
        let r = Ray::new(Vec3::ONE, Vec3::new(0.0, 0.0, -5.0)).unwrap();
        assert_eq!(r.dir, Vec3::NEG_Z);
        assert_eq!(r.point_at(2.0), Vec3::new(1.0, 1.0, -1.0));
    }

    #[test]
    fn rays_hit_the_faces_of_a_box_with_the_right_normal_and_distance() {
        let b = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let h = b
            .raycast(&ray(Vec3::new(5.0, 0.0, 0.0), Vec3::NEG_X), 100.0)
            .unwrap();
        assert!(close(h.t, 4.0) && h.normal == Vec3::X && !h.inside, "{h:?}");
        assert_eq!(h.point, Vec3::new(1.0, 0.0, 0.0));
        let h = b
            .raycast(&ray(Vec3::new(0.0, -7.0, 0.3), Vec3::Y), 100.0)
            .unwrap();
        assert!(close(h.t, 6.0) && h.normal == Vec3::NEG_Y);
        assert!(
            b.raycast(&ray(Vec3::new(5.0, 0.0, 0.0), Vec3::NEG_X), 3.9)
                .is_none(),
            "too short to arrive"
        );
        assert!(
            b.raycast(&ray(Vec3::new(5.0, 3.0, 0.0), Vec3::NEG_X), 100.0)
                .is_none(),
            "passes above"
        );
        assert!(
            b.raycast(&ray(Vec3::new(5.0, 0.0, 0.0), Vec3::X), 100.0)
                .is_none(),
            "points away"
        );
    }

    #[test]
    fn a_ray_along_a_face_or_starting_on_a_slab_boundary_does_not_give_nan() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        // Parallel to the x slab and exactly on its boundary.
        let h = b.raycast(&ray(Vec3::new(-5.0, 1.0, 0.0), Vec3::X), 100.0);
        if let Some(h) = h {
            assert!(h.t.is_finite() && h.normal.is_finite());
        }
        // Axis-parallel ray at the centre: no zero-times-infinity.
        let h = b
            .raycast(&ray(Vec3::new(0.0, 0.0, 9.0), Vec3::NEG_Z), 100.0)
            .unwrap();
        assert!(close(h.t, 8.0));
    }

    #[test]
    fn a_ray_starting_inside_a_solid_hits_at_zero_facing_back_along_the_ray() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let h = b.raycast(&ray(Vec3::ZERO, Vec3::X), 10.0).unwrap();
        assert!(h.inside && h.t == 0.0 && h.normal == Vec3::NEG_X, "{h:?}");
        let s = Sphere::new(Vec3::ZERO, 2.0);
        let h = s
            .raycast(&ray(Vec3::new(0.5, 0.0, 0.0), Vec3::Y), 10.0)
            .unwrap();
        assert!(h.inside && h.t == 0.0);
        let c = Capsule::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5);
        assert!(c.raycast(&ray(Vec3::ZERO, Vec3::Z), 10.0).unwrap().inside);
        let o = Obb::new(Vec3::ZERO, Vec3::ONE, Quat::from_rotation_y(0.5));
        assert!(o.raycast(&ray(Vec3::ZERO, Vec3::Z), 10.0).unwrap().inside);
    }

    #[test]
    fn spheres_capsules_planes_and_oriented_boxes_are_hit_where_they_should() {
        let s = Sphere::new(Vec3::new(0.0, 0.0, -10.0), 2.0);
        let h = s.raycast(&ray(Vec3::ZERO, Vec3::NEG_Z), 100.0).unwrap();
        assert!(close(h.t, 8.0) && h.normal == Vec3::Z, "{h:?}");
        assert!(
            s.raycast(&ray(Vec3::new(2.5, 0.0, 0.0), Vec3::NEG_Z), 100.0)
                .is_none()
        );

        let c = Capsule::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 4.0, 0.0), 1.0);
        let h = c
            .raycast(&ray(Vec3::new(5.0, 2.0, 0.0), Vec3::NEG_X), 100.0)
            .unwrap();
        assert!(close(h.t, 4.0) && close(h.normal.x, 1.0), "the side: {h:?}");
        let h = c
            .raycast(&ray(Vec3::new(0.0, 9.0, 0.0), Vec3::NEG_Y), 100.0)
            .unwrap();
        assert!(
            close(h.t, 4.0) && close(h.normal.y, 1.0),
            "the top cap: {h:?}"
        );

        let p = Plane::from_point_normal(Vec3::new(0.0, 1.0, 0.0), Vec3::Y).unwrap();
        let h = p
            .raycast(&ray(Vec3::new(3.0, 6.0, 2.0), Vec3::NEG_Y), 100.0)
            .unwrap();
        assert!(close(h.t, 5.0) && h.normal == Vec3::Y);
        assert!(
            p.raycast(&ray(Vec3::new(3.0, 6.0, 2.0), Vec3::Y), 100.0)
                .is_none(),
            "away from the plane"
        );
        assert!(
            p.raycast(&ray(Vec3::new(0.0, 6.0, 0.0), Vec3::X), 100.0)
                .is_none(),
            "parallel to it"
        );
        assert!(Plane::from_point_normal(Vec3::ZERO, Vec3::ZERO).is_none());

        let o = Obb::new(
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(1.0, 1.0, 1.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
        );
        let h = o.raycast(&ray(Vec3::ZERO, Vec3::NEG_Z), 100.0).unwrap();
        let corner = 2.0_f32.sqrt();
        assert!(
            (h.t - (10.0 - corner)).abs() < 1e-3,
            "a diamond seen from the front: its corner is {corner} nearer: {h:?}"
        );
    }

    /// The reference: the signed distance of a point to each shape, negative inside.
    fn sdf_aabb(b: &Aabb, p: Vec3) -> f32 {
        b.signed_distance(p)
    }

    #[test]
    fn signed_distances_agree_with_the_obvious_cases() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        assert!(close(sdf_aabb(&b, Vec3::new(3.0, 0.0, 0.0)), 2.0));
        assert!(close(
            sdf_aabb(&b, Vec3::new(2.0, 2.0, 0.0)),
            2.0_f32.sqrt()
        ));
        assert!(close(sdf_aabb(&b, Vec3::ZERO), -1.0));
        assert!(close(
            Sphere::new(Vec3::ZERO, 2.0).signed_distance(Vec3::new(5.0, 0.0, 0.0)),
            3.0
        ));
        let c = Capsule::new(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 1.0);
        assert!(close(c.signed_distance(Vec3::new(3.0, 1.0, 0.0)), 2.0));
        assert!(close(c.signed_distance(Vec3::new(0.0, 5.0, 0.0)), 2.0));
        let p = Plane::from_point_normal(Vec3::ZERO, Vec3::Y).unwrap();
        assert!(close(p.signed_distance(Vec3::new(9.0, -4.0, 9.0)), -4.0));
        let o = Obb::new(
            Vec3::ZERO,
            Vec3::ONE,
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
        );
        assert!(close(
            o.signed_distance(Vec3::new(2.0_f32.sqrt() + 1.0, 0.0, 0.0)),
            1.0
        ));
    }

    /// For random rays against random shapes: a hit lies on the surface and nothing before it is
    /// inside; a miss means no sampled point of the ray is inside.
    fn ray_property(
        name: &str,
        focus: Vec3,
        sdf: impl Fn(Vec3) -> f32,
        cast: impl Fn(&Ray, f32) -> Option<RayHit>,
        seed: u64,
    ) {
        let mut rng = Lcg(seed);
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..4000 {
            let origin = rng.vec(6.0);
            // Aim near the shape, so that hits and misses are both common.
            let Some(r) = Ray::new(origin, focus + rng.vec(2.5) - origin) else {
                continue;
            };
            let max_t = rng.range(1.0, 20.0);
            match cast(&r, max_t) {
                Some(h) => {
                    hits += 1;
                    assert!(h.t >= 0.0 && h.t <= max_t + 1e-3, "{name}: t {}", h.t);
                    assert!(
                        close(h.normal.length(), 1.0),
                        "{name}: normal {:?}",
                        h.normal
                    );
                    assert!(
                        (h.point - r.point_at(h.t)).length() < 1e-3,
                        "{name}: the point is on the ray"
                    );
                    if !h.inside {
                        assert!(
                            sdf(h.point).abs() < 2e-3,
                            "{name}: hit point is {} from the surface",
                            sdf(h.point)
                        );
                        assert!(
                            h.normal.dot(r.dir) <= 1e-3,
                            "{name}: the normal faces the ray"
                        );
                        for i in 0..64 {
                            let t = h.t * (i as f32) / 64.0;
                            assert!(
                                sdf(r.point_at(t)) > -2e-3,
                                "{name}: the ray was inside the shape before the hit (t = {t})"
                            );
                        }
                    } else {
                        assert!(
                            sdf(r.origin) <= 1e-3,
                            "{name}: called inside but the origin is outside"
                        );
                    }
                }
                None => {
                    misses += 1;
                    for i in 0..=200 {
                        let t = max_t * (i as f32) / 200.0;
                        assert!(
                            sdf(r.point_at(t)) > -2e-3,
                            "{name}: a miss, yet the ray is inside at t = {t}"
                        );
                    }
                }
            }
        }
        assert!(
            hits > 200 && misses > 200,
            "{name}: hits {hits}, misses {misses}"
        );
    }

    #[test]
    fn ray_casts_match_the_signed_distance_of_every_shape() {
        let b = Aabb::new(Vec3::new(-1.5, -0.5, -1.0), Vec3::new(1.0, 2.0, 2.5));
        ray_property(
            "aabb",
            b.center(),
            |p| b.signed_distance(p),
            |r, m| b.raycast(r, m),
            1,
        );
        let s = Sphere::new(Vec3::new(0.5, -0.5, 1.0), 1.7);
        ray_property(
            "sphere",
            s.center,
            |p| s.signed_distance(p),
            |r, m| s.raycast(r, m),
            2,
        );
        let c = Capsule::new(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.5, 2.0, 1.0), 0.8);
        ray_property(
            "capsule",
            (c.a + c.b) * 0.5,
            |p| c.signed_distance(p),
            |r, m| c.raycast(r, m),
            3,
        );
        let o = Obb::new(
            Vec3::new(0.3, 0.2, -0.4),
            Vec3::new(1.0, 0.5, 1.5),
            Quat::from_rotation_x(0.7) * Quat::from_rotation_y(-0.4),
        );
        ray_property(
            "obb",
            o.center,
            |p| o.signed_distance(p),
            |r, m| o.raycast(r, m),
            4,
        );
        let pl =
            Plane::from_point_normal(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.2, 1.0, -0.1)).unwrap();
        ray_property(
            "plane",
            Vec3::new(0.0, 0.5, 0.0),
            |p| pl.signed_distance(p),
            |r, m| pl.raycast(r, m),
            5,
        );
    }

    // ---- overlaps and closest points ----------------------------------------------------------

    #[test]
    fn overlap_tests_say_yes_only_for_touching_or_crossing_shapes() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        assert!(b.overlaps(&Aabb::new(Vec3::splat(0.5), Vec3::splat(3.0))));
        assert!(
            b.overlaps(&Aabb::new(
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 1.0, 1.0)
            )),
            "touching counts"
        );
        assert!(!b.overlaps(&Aabb::new(
            Vec3::new(1.01, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 1.0)
        )));
        let s = Sphere::new(Vec3::new(2.0, 0.0, 0.0), 1.0);
        assert!(s.overlaps_aabb(&b), "touching the face");
        assert!(!Sphere::new(Vec3::new(2.1, 0.0, 0.0), 1.0).overlaps_aabb(&b));
        assert!(
            !Sphere::new(Vec3::new(2.0, 2.0, 2.0), 1.0).overlaps_aabb(&b),
            "near the corner it is farther than it looks"
        );
        assert!(Sphere::new(Vec3::new(1.5, 1.5, 1.5), 1.0).overlaps_aabb(&b));
        assert!(
            Sphere::new(Vec3::ZERO, 1.0)
                .overlaps_sphere(&Sphere::new(Vec3::new(1.5, 0.0, 0.0), 0.5))
        );
        assert!(
            !Sphere::new(Vec3::ZERO, 1.0)
                .overlaps_sphere(&Sphere::new(Vec3::new(1.6, 0.0, 0.0), 0.5))
        );

        let c = Capsule::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 5.0, 0.0), 1.0);
        assert!(c.overlaps_sphere(&Sphere::new(Vec3::new(1.4, 2.5, 0.0), 0.5)));
        assert!(!c.overlaps_sphere(&Sphere::new(Vec3::new(1.6, 2.5, 0.0), 0.5)));
        assert!(c.overlaps_capsule(&Capsule::new(
            Vec3::new(1.5, 4.0, 0.0),
            Vec3::new(1.5, 9.0, 0.0),
            0.6
        )));
        assert!(!c.overlaps_capsule(&Capsule::new(
            Vec3::new(1.7, 4.0, 0.0),
            Vec3::new(1.7, 9.0, 0.0),
            0.6
        )));
        assert!(c.overlaps_aabb(&Aabb::new(
            Vec3::new(0.5, 1.0, -1.0),
            Vec3::new(3.0, 2.0, 1.0)
        )));
        assert!(!c.overlaps_aabb(&Aabb::new(
            Vec3::new(1.5, 1.0, -1.0),
            Vec3::new(3.0, 2.0, 1.0)
        )));
        assert!(
            c.overlaps_aabb(&Aabb::new(
                Vec3::new(-1.0, 5.5, -1.0),
                Vec3::new(1.0, 6.0, 1.0)
            )),
            "the top cap reaches 6"
        );
    }

    #[test]
    fn overlap_tests_agree_with_signed_distance_sampling() {
        let mut rng = Lcg(11);
        for _ in 0..3000 {
            let b = Aabb::from_center_half(
                rng.vec(2.0),
                Vec3::new(
                    rng.range(0.2, 1.5),
                    rng.range(0.2, 1.5),
                    rng.range(0.2, 1.5),
                ),
            );
            let c = Capsule::new(rng.vec(3.0), rng.vec(3.0), rng.range(0.1, 1.2));
            // Ground truth: the smallest distance from the capsule's axis to the box, by fine sampling.
            let best = (0..=400)
                .map(|i| b.signed_distance(c.a + (c.b - c.a) * (i as f32 / 400.0)))
                .fold(f32::MAX, f32::min);
            let says = c.overlaps_aabb(&b);
            if (best - c.radius).abs() > 0.02 {
                assert_eq!(
                    says,
                    best <= c.radius,
                    "capsule {c:?} box {b:?}: sampled distance {best}"
                );
            }
        }
    }

    #[test]
    fn closest_points_lie_on_the_shape() {
        let mut rng = Lcg(21);
        for _ in 0..2000 {
            let b = Aabb::from_center_half(
                rng.vec(2.0),
                Vec3::new(
                    rng.range(0.2, 1.5),
                    rng.range(0.2, 1.5),
                    rng.range(0.2, 1.5),
                ),
            );
            let p = rng.vec(6.0);
            let q = b.closest_point(p);
            assert!(b.signed_distance(q).abs() < 1e-4 || b.contains_point(q));
            assert!(
                close((p - q).length(), b.signed_distance(p).max(0.0)),
                "distance to the closest point is the unsigned distance"
            );
        }
    }

    #[test]
    fn boxes_are_built_unioned_and_transformed() {
        let b = Aabb::from_center_half(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            (b.min, b.max),
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 6.0))
        );
        assert_eq!(
            (b.center(), b.half_extents()),
            (Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 2.0, 3.0))
        );
        let u = b.union(&Aabb::new(Vec3::splat(-1.0), Vec3::ZERO));
        assert_eq!(
            (u.min, u.max),
            (Vec3::splat(-1.0), Vec3::new(2.0, 4.0, 6.0))
        );
        assert_eq!(b.expanded(1.0).min, Vec3::splat(-1.0));
        let o = Obb::new(
            Vec3::ZERO,
            Vec3::ONE,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
        );
        let bound = o.bounding_aabb();
        assert!(
            close(bound.max.x, 2.0_f32.sqrt()) && close(bound.max.y, 1.0),
            "{bound:?}"
        );
    }

    // ---- sweeps -------------------------------------------------------------------------------

    /// A moving sphere from `start` along `delta` against `sdf`: the first fraction at which the
    /// sphere touches (sampled), to compare with the exact answer.
    fn sweep_property(
        name: &str,
        sdf: impl Fn(Vec3) -> f32,
        sweep: impl Fn(&Sphere, Vec3) -> Option<SweepHit>,
        seed: u64,
    ) {
        let mut rng = Lcg(seed);
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..3000 {
            let radius = rng.range(0.1, 1.0);
            let moving = Sphere::new(rng.vec(6.0), radius);
            let delta = rng.vec(8.0);
            match sweep(&moving, delta) {
                Some(h) => {
                    hits += 1;
                    assert!((0.0..=1.0).contains(&h.t), "{name}: t {}", h.t);
                    assert!(
                        close(h.normal.length(), 1.0),
                        "{name}: normal {:?}",
                        h.normal
                    );
                    if !h.overlapping_at_start {
                        let at = moving.center + delta * h.t;
                        assert!(
                            (h.position - at).length() < 1e-3,
                            "{name}: position is the centre at the hit"
                        );
                        assert!(
                            (sdf(at) - radius).abs() < 3e-3,
                            "{name}: at the hit the sphere is {} from touching",
                            sdf(at) - radius
                        );
                        for i in 0..64 {
                            let t = h.t * (i as f32) / 64.0;
                            assert!(
                                sdf(moving.center + delta * t) > radius - 3e-3,
                                "{name}: the sphere was already overlapping at t = {t}"
                            );
                        }
                        assert!(
                            h.normal.dot(delta) <= 1e-3,
                            "{name}: the normal opposes the motion"
                        );
                    } else {
                        assert!(
                            h.t == 0.0 && sdf(moving.center) <= radius + 1e-3,
                            "{name}: overlapping at the start"
                        );
                    }
                }
                None => {
                    misses += 1;
                    for i in 0..=300 {
                        let t = i as f32 / 300.0;
                        assert!(
                            sdf(moving.center + delta * t) > radius - 3e-3,
                            "{name}: a miss, yet overlapping at t = {t}"
                        );
                    }
                }
            }
        }
        assert!(
            hits > 150 && misses > 150,
            "{name}: hits {hits}, misses {misses}"
        );
    }

    #[test]
    fn sphere_sweeps_match_sampled_signed_distance() {
        let b = Aabb::new(Vec3::new(-1.5, -0.5, -1.0), Vec3::new(1.0, 2.0, 2.5));
        sweep_property(
            "sphere vs box",
            |p| b.signed_distance(p),
            |s, d| s.sweep_aabb(d, &b),
            31,
        );
        let target = Sphere::new(Vec3::new(0.5, -0.5, 1.0), 1.2);
        sweep_property(
            "sphere vs sphere",
            |p| target.signed_distance(p),
            |s, d| s.sweep_sphere(d, &target),
            32,
        );
        let pl =
            Plane::from_point_normal(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.2, 1.0, -0.1)).unwrap();
        sweep_property(
            "sphere vs plane",
            |p| pl.signed_distance(p),
            |s, d| s.sweep_plane(d, &pl),
            33,
        );
    }

    #[test]
    fn a_sphere_slides_off_a_box_edge_and_corner_correctly() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        // Straight at a face.
        let h = Sphere::new(Vec3::new(5.0, 0.0, 0.0), 0.5)
            .sweep_aabb(Vec3::new(-10.0, 0.0, 0.0), &b)
            .unwrap();
        assert!(close(h.t, 0.35) && h.normal == Vec3::X, "{h:?}");
        // At the corner along the diagonal: the normal points away from the corner.
        let start = Vec3::new(4.0, 4.0, 4.0);
        let h = Sphere::new(start, 0.5)
            .sweep_aabb(Vec3::new(-6.0, -6.0, -6.0), &b)
            .unwrap();
        let d = 1.0 / 3.0_f32.sqrt();
        assert!(
            (h.normal - Vec3::splat(d)).length() < 1e-3,
            "{:?}",
            h.normal
        );
        let travelled = h.t * 6.0 * 3.0_f32.sqrt();
        assert!(
            (travelled - (3.0 * 3.0_f32.sqrt() - 0.5)).abs() < 1e-3,
            "{travelled}"
        );
        // Past the corner without touching.
        assert!(
            Sphere::new(Vec3::new(1.6, 1.6, 5.0), 0.5)
                .sweep_aabb(Vec3::new(0.0, 0.0, -10.0), &b)
                .is_none()
        );
    }

    #[test]
    fn sweeping_from_overlap_reports_it_and_moving_away_is_not_a_hit() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let h = Sphere::new(Vec3::new(1.2, 0.0, 0.0), 0.5)
            .sweep_aabb(Vec3::new(-1.0, 0.0, 0.0), &b)
            .unwrap();
        assert!(h.overlapping_at_start && h.t == 0.0);
        // Even moving away: the caller must learn that the shapes interpenetrate, to push them apart.
        let away = Sphere::new(Vec3::new(1.2, 0.0, 0.0), 0.5)
            .sweep_aabb(Vec3::new(3.0, 0.0, 0.0), &b)
            .unwrap();
        assert!(
            away.overlapping_at_start && away.normal == Vec3::X,
            "{away:?}"
        );
        // Resting exactly on a face and moving away or along it is not a hit; moving into it is, at 0.
        let resting = Sphere::new(Vec3::new(1.5, 0.0, 0.0), 0.5);
        assert!(
            resting.sweep_aabb(Vec3::new(3.0, 0.0, 0.0), &b).is_none(),
            "moving away from a touching face"
        );
        assert!(
            resting.sweep_aabb(Vec3::new(0.0, 0.9, 0.0), &b).is_none(),
            "sliding along it, short of the edge (an exactly tangent graze is not tested: it is a coin flip of rounding)"
        );
        let into = resting.sweep_aabb(Vec3::new(-1.0, 0.0, 0.0), &b).unwrap();
        assert!(
            into.t == 0.0 && !into.overlapping_at_start && into.normal == Vec3::X,
            "{into:?}"
        );
        assert!(
            Sphere::new(Vec3::new(9.0, 0.0, 0.0), 0.5)
                .sweep_aabb(Vec3::ZERO, &b)
                .is_none(),
            "not moving, not touching"
        );
        assert!(
            Sphere::new(Vec3::new(9.0, 0.0, 0.0), 0.5)
                .sweep_aabb(Vec3::new(f32::NAN, 0.0, 0.0), &b)
                .is_none(),
            "bad input is no hit"
        );
    }

    #[test]
    fn box_sweeps_stop_at_the_first_touching_face() {
        let target = Aabb::new(Vec3::new(5.0, -1.0, -1.0), Vec3::new(7.0, 1.0, 1.0));
        let moving = Aabb::new(Vec3::new(-1.0, -0.5, -0.5), Vec3::new(1.0, 0.5, 0.5));
        let h = moving
            .sweep_aabb(Vec3::new(10.0, 0.0, 0.0), &target)
            .unwrap();
        assert!(close(h.t, 0.4) && h.normal == Vec3::NEG_X, "{h:?}");
        assert!(close(h.position.x, 4.0), "{h:?}");
        assert!(
            moving
                .sweep_aabb(Vec3::new(10.0, 5.0, 0.0), &target)
                .is_none(),
            "goes over the top"
        );
        let mut rng = Lcg(41);
        for _ in 0..2000 {
            let m = Aabb::from_center_half(
                rng.vec(5.0),
                Vec3::new(
                    rng.range(0.1, 1.0),
                    rng.range(0.1, 1.0),
                    rng.range(0.1, 1.0),
                ),
            );
            let t = Aabb::from_center_half(
                rng.vec(3.0),
                Vec3::new(
                    rng.range(0.2, 1.5),
                    rng.range(0.2, 1.5),
                    rng.range(0.2, 1.5),
                ),
            );
            let delta = rng.vec(8.0);
            let at = |f: f32| Aabb::new(m.min + delta * f, m.max + delta * f);
            match m.sweep_aabb(delta, &t) {
                Some(h) if !h.overlapping_at_start => {
                    let touching = at(h.t).expanded(1e-3);
                    assert!(touching.overlaps(&t), "the box touches at the hit");
                    assert!(
                        !at((h.t - 0.02).max(0.0)).overlaps(&t) || h.t < 0.02,
                        "and not clearly before it"
                    );
                }
                Some(_) => assert!(m.overlaps(&t)),
                None => {
                    for i in 0..=100 {
                        let shrunk = Aabb::new(
                            at(i as f32 / 100.0).min + Vec3::splat(2e-3),
                            at(i as f32 / 100.0).max - Vec3::splat(2e-3),
                        );
                        assert!(
                            !shrunk.overlaps(&t),
                            "a miss, yet the boxes overlap along the path"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn results_are_bit_identical_when_repeated() {
        let b = Aabb::new(Vec3::new(-1.5, -0.5, -1.0), Vec3::new(1.0, 2.0, 2.5));
        let s = Sphere::new(Vec3::new(-6.0, 0.3, 0.2), 0.4);
        let a = s.sweep_aabb(Vec3::new(9.0, 0.1, 0.1), &b);
        let c = s.sweep_aabb(Vec3::new(9.0, 0.1, 0.1), &b);
        assert_eq!(a, c);
        assert_eq!(a.map(|h| h.t.to_bits()), c.map(|h| h.t.to_bits()));
    }
}
