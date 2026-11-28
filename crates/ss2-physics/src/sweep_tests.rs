//! Tests of the capsule and box sweeps and overlaps against triangles and meshes, and of
//! `TriMesh::raycast_all` (T07-004b). The random checks compare against references that share no
//! code with what they check: the distance from a segment to a triangle by a ternary search along
//! the segment (the distance to a convex set is convex along a line), and whether a box and a
//! triangle meet by clipping the triangle to the box.

use super::*;
use crate::geometry::{Aabb, Ray};
use ss2_core::math::Vec3;

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
    fn triangle(&mut self, extent: f32) -> Triangle {
        loop {
            let t = Triangle::new(self.vec(extent), self.vec(extent), self.vec(extent));
            if (t.b - t.a).cross(t.c - t.a).length() > 0.5 {
                return t;
            }
        }
    }
}

/// The reference distance from the segment `a..b` to the triangle.
fn segment_distance(tri: &Triangle, a: Vec3, b: Vec3) -> f32 {
    let at = |s: f32| {
        let p = a + (b - a) * s;
        (p - tri.closest_point(p)).length()
    };
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..60 {
        let third = (hi - lo) / 3.0;
        let (m1, m2) = (lo + third, hi - third);
        if at(m1) < at(m2) {
            hi = m2;
        } else {
            lo = m1;
        }
    }
    at((lo + hi) * 0.5).min(at(0.0)).min(at(1.0))
}

/// The reference box test: clip the triangle to the box; something left means they meet.
fn box_meets(tri: &Triangle, center: Vec3, half: Vec3) -> bool {
    let (lo, hi) = (center - half, center + half);
    let mut poly = vec![tri.a, tri.b, tri.c];
    for axis in 0..3 {
        for (bound, keep_above) in [(lo[axis], true), (hi[axis], false)] {
            let inside = |p: &Vec3| {
                if keep_above {
                    p[axis] >= bound
                } else {
                    p[axis] <= bound
                }
            };
            let mut next = Vec::new();
            for i in 0..poly.len() {
                let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
                if inside(&p) {
                    next.push(p);
                }
                if inside(&p) != inside(&q) {
                    let s = (bound - p[axis]) / (q[axis] - p[axis]);
                    let mut x = p + (q - p) * s;
                    x[axis] = bound;
                    next.push(x);
                }
            }
            poly = next;
            if poly.is_empty() {
                return false;
            }
        }
    }
    true
}

fn floor(size: f32) -> TriMesh {
    let v = [
        Vec3::new(-size, 0.0, -size),
        Vec3::new(size, 0.0, -size),
        Vec3::new(size, 0.0, size),
        Vec3::new(-size, 0.0, size),
    ];
    TriMesh::new(&v, &[0, 2, 1, 0, 3, 2]).unwrap()
}

fn upright(center: Vec3, half_height: f32) -> (Vec3, Vec3) {
    (
        center - Vec3::Y * half_height,
        center + Vec3::Y * half_height,
    )
}

// ---- capsules: hand-made cases ------------------------------------------------------------------

#[test]
fn an_upright_capsule_lands_on_a_floor_with_its_lower_end() {
    let mesh = floor(5.0);
    let (a, b) = upright(Vec3::new(1.0, 3.0, 1.0), 0.5);
    let (hit, triangle) = mesh
        .sweep_capsule(a, b, 0.3, Vec3::new(0.0, -10.0, 0.0), Layers::ALL)
        .unwrap();
    assert!((hit.t * 10.0 - 2.2).abs() < 1e-5, "{hit:?}");
    assert!((hit.position - Vec3::new(1.0, 0.8, 1.0)).length() < 1e-5);
    assert!((hit.normal - Vec3::Y).length() < 1e-5);
    assert!(!hit.overlapping_at_start);
    assert!(triangle < 2);
}

#[test]
fn a_lying_capsule_meets_an_edge_with_the_middle_of_its_axis() {
    // A wall in the plane x = 0 whose top edge runs along Z at y = 0. A capsule lying along X is
    // dropped on it: only the inside of its axis meets the inside of the edge (the ends of the
    // capsule are 2 away, the wall's corners 1 away along Z), so only the edge test can find it.
    let wall = Triangle::new(
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, -1.0, 0.0),
    );
    let hit = wall
        .sweep_capsule(
            Vec3::new(-2.0, 2.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            0.3,
            Vec3::new(0.0, -5.0, 0.0),
        )
        .unwrap();
    assert!((hit.t * 5.0 - 1.7).abs() < 1e-5, "{hit:?}");
    assert!((hit.normal - Vec3::Y).length() < 1e-5);
    // Turned a little about Y, the axis still crosses the edge's middle.
    let hit = wall
        .sweep_capsule(
            Vec3::new(-2.0, 2.0, -0.3),
            Vec3::new(2.0, 2.0, 0.3),
            0.3,
            Vec3::new(0.0, -5.0, 0.0),
        )
        .unwrap();
    assert!((hit.t * 5.0 - 1.7).abs() < 1e-4, "{hit:?}");
}

#[test]
fn an_upright_capsule_meets_a_corner_with_its_side() {
    // A flat triangle pointing a corner at the capsule, level with its middle: the capsule's
    // round ends pass a whole unit above and below, so only its side can touch the corner.
    let tri = Triangle::new(
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, -1.0),
        Vec3::new(3.0, 0.0, 1.0),
    );
    let (a, b) = upright(Vec3::new(-3.0, 0.0, 0.0), 1.0);
    let hit = tri
        .sweep_capsule(a, b, 0.5, Vec3::new(10.0, 0.0, 0.0))
        .unwrap();
    assert!((hit.t * 10.0 - 3.5).abs() < 1e-5, "{hit:?}");
    assert!((hit.normal - Vec3::NEG_X).length() < 1e-5);
    assert!((hit.position - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-5);
}

#[test]
fn a_capsule_that_starts_in_contact_reports_it_with_the_way_out() {
    let floor_tri = Triangle::new(
        Vec3::new(-5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, 5.0),
        Vec3::new(5.0, 0.0, -5.0),
    );
    // Sunk 0.1 into the floor: out is up, whichever way it moves.
    let (a, b) = upright(Vec3::new(1.0, 0.7, 0.0), 0.5);
    for delta in [Vec3::NEG_Y, Vec3::Y, Vec3::X, Vec3::ZERO] {
        let hit = floor_tri.sweep_capsule(a, b, 0.3, delta).unwrap();
        assert!(hit.overlapping_at_start && hit.t == 0.0, "{delta}: {hit:?}");
        assert!((hit.normal - Vec3::Y).length() < 1e-5);
    }
    // The axis goes right through the floor: there is no nearest direction, so the way out is
    // against the motion.
    let (a, b) = upright(Vec3::new(1.0, 0.0, 0.0), 0.5);
    let hit = floor_tri
        .sweep_capsule(a, b, 0.3, Vec3::new(0.0, 1.0, 0.0))
        .unwrap();
    assert!(hit.overlapping_at_start);
    assert_eq!(hit.normal, Vec3::NEG_Y);
}

#[test]
fn a_capsule_resting_on_a_floor_slides_freely_and_is_stopped_going_down() {
    let mesh = floor(5.0);
    // The lower end exactly touches the floor.
    let (a, b) = upright(Vec3::new(0.0, 0.8, 0.0), 0.5);
    assert!(
        mesh.sweep_capsule(a, b, 0.3, Vec3::new(3.0, 0.0, 1.0), Layers::ALL)
            .is_none(),
        "grazing a floor it slides along is not a hit"
    );
    assert!(
        mesh.sweep_capsule(a, b, 0.3, Vec3::new(0.0, 2.0, 0.0), Layers::ALL)
            .is_none(),
        "nor is leaving it"
    );
    let (hit, _) = mesh
        .sweep_capsule(a, b, 0.3, Vec3::new(0.0, -1.0, 0.0), Layers::ALL)
        .unwrap();
    assert_eq!(hit.t, 0.0);
    assert_eq!(hit.normal, Vec3::Y);
}

#[test]
fn a_zero_length_capsule_cast_only_checks_for_overlap() {
    let mesh = floor(5.0);
    let (a, b) = upright(Vec3::new(0.0, 0.7, 0.0), 0.5);
    let (hit, _) = mesh
        .sweep_capsule(a, b, 0.3, Vec3::ZERO, Layers::ALL)
        .unwrap();
    assert!(hit.overlapping_at_start && hit.t == 0.0);
    let (a, b) = upright(Vec3::new(0.0, 2.0, 0.0), 0.5);
    assert!(
        mesh.sweep_capsule(a, b, 0.3, Vec3::ZERO, Layers::ALL)
            .is_none()
    );
}

#[test]
fn a_capsule_with_no_straight_part_is_a_sphere() {
    let mut rng = Lcg(21);
    for _ in 0..500 {
        let tri = rng.triangle(3.0);
        let c = rng.vec(5.0);
        let delta = rng.vec(8.0);
        let r = rng.range(0.1, 1.0);
        let s = tri.sweep_sphere(c, r, delta);
        let k = tri.sweep_capsule(c, c, r, delta);
        assert_eq!(s.is_some(), k.is_some(), "{s:?} {k:?}");
        if let (Some(s), Some(k)) = (s, k) {
            assert!((s.t - k.t).abs() < 1e-5, "{s:?} {k:?}");
            assert_eq!(s.overlapping_at_start, k.overlapping_at_start);
        }
    }
}

#[test]
fn degenerate_triangles_are_never_hit_and_are_left_out_of_a_mesh() {
    let line = Triangle::new(Vec3::ZERO, Vec3::X, Vec3::X * 2.0);
    let (a, b) = upright(Vec3::new(1.0, 2.0, 0.0), 0.5);
    assert!(
        line.sweep_capsule(a, b, 0.3, Vec3::new(0.0, -5.0, 0.0))
            .is_none()
    );
    assert!(
        line.sweep_aabb(
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::splat(0.5),
            Vec3::NEG_Y * 5.0
        )
        .is_none()
    );
    let v = [
        Vec3::new(-5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, 5.0),
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(1.0, 3.0, 0.0),
        Vec3::new(2.0, 3.0, 0.0),
    ];
    let mesh = TriMesh::new(&v, &[3, 4, 5, 0, 2, 1]).unwrap();
    assert_eq!(mesh.dropped_degenerate(), 1);
    let (hit, triangle) = mesh
        .sweep_capsule(
            Vec3::new(2.0, 5.0, -2.0),
            Vec3::new(2.0, 6.0, -2.0),
            0.25,
            Vec3::NEG_Y * 10.0,
            Layers::ALL,
        )
        .unwrap();
    assert_eq!(triangle, 0, "the floor, the only triangle kept");
    assert!(
        (hit.t * 10.0 - 4.75).abs() < 1e-4,
        "it passed the line at y = 3"
    );
}

#[test]
fn a_long_capsule_cast_is_still_accurate() {
    let mesh = floor(5000.0);
    let (a, b) = upright(Vec3::new(1234.5, 9000.0, -321.0), 0.9);
    let (hit, _) = mesh
        .sweep_capsule(a, b, 0.4, Vec3::new(0.0, -20_000.0, 0.0), Layers::ALL)
        .unwrap();
    assert!(
        (hit.t * 20_000.0 - (9000.0 - 1.3)).abs() < 1e-2,
        "{}",
        hit.t * 20_000.0
    );
    // Across most of the floor, sideways and down at once.
    let (a, b) = upright(Vec3::new(-4000.0, 300.0, 0.0), 0.9);
    let (hit, _) = mesh
        .sweep_capsule(a, b, 0.4, Vec3::new(8000.0, -600.0, 0.0), Layers::ALL)
        .unwrap();
    let travelled = hit.t * Vec3::new(8000.0, -600.0, 0.0).length();
    let expected = (300.0 - 1.3) / 600.0 * Vec3::new(8000.0, -600.0, 0.0).length();
    assert!((travelled - expected).abs() < 0.1, "{travelled} {expected}");
}

#[test]
fn of_two_triangles_hit_at_once_the_lower_index_wins() {
    // The same floor twice (indices 0, 1 and 2, 3): every hit is a tie between two copies.
    let v = [
        Vec3::new(-5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, 5.0),
        Vec3::new(-5.0, 0.0, 5.0),
    ];
    let mesh = TriMesh::new(&v, &[0, 2, 1, 0, 3, 2, 0, 2, 1, 0, 3, 2]).unwrap();
    let mut rng = Lcg(31);
    for _ in 0..200 {
        let at = Vec3::new(rng.range(-4.0, 4.0), 3.0, rng.range(-4.0, 4.0));
        let (a, b) = upright(at, 0.5);
        let (_, triangle) = mesh
            .sweep_capsule(a, b, 0.3, Vec3::NEG_Y * 5.0, Layers::ALL)
            .unwrap();
        assert!(triangle < 2, "a copy with a higher index won: {triangle}");
        let (_, triangle) = mesh
            .sweep_aabb(at, Vec3::splat(0.4), Vec3::NEG_Y * 5.0, Layers::ALL)
            .unwrap();
        assert!(triangle < 2, "a copy with a higher index won: {triangle}");
        let (_, triangle) = mesh
            .sweep_sphere(at, 0.4, Vec3::NEG_Y * 5.0, Layers::ALL)
            .unwrap();
        assert!(triangle < 2, "a copy with a higher index won: {triangle}");
    }
}

// ---- capsules: against the references ---------------------------------------------------------

#[test]
fn capsule_sweeps_match_the_sampled_distance_to_the_triangle() {
    let mut rng = Lcg(41);
    let (mut hits, mut misses, mut inside) = (0, 0, 0);
    for _ in 0..3000 {
        let tri = rng.triangle(3.0);
        let center = rng.vec(5.0);
        // Any direction: the triangle sweep takes any segment, not only an upright one.
        let half = rng.vec(1.0);
        let (a, b) = (center - half, center + half);
        let r = rng.range(0.05, 1.0);
        // Aim at the triangle most of the time, so hits are common.
        let target = (tri.a + tri.b + tri.c) / 3.0 + rng.vec(2.0);
        let delta = (target - center) * rng.range(0.3, 2.0);
        let len = delta.length();
        match tri.sweep_capsule(a, b, r, delta) {
            Some(h) if h.overlapping_at_start => {
                inside += 1;
                assert!(segment_distance(&tri, a, b) < r + 1e-3);
            }
            Some(h) => {
                hits += 1;
                let shift = delta * h.t;
                let at = segment_distance(&tri, a + shift, b + shift);
                assert!(
                    (at - r).abs() < 2e-3,
                    "not touching at the hit: {at} vs {r}"
                );
                for i in 0..64 {
                    let s = delta * (h.t * i as f32 / 64.0);
                    let d = segment_distance(&tri, a + s, b + s);
                    assert!(d > r - 2e-3, "overlapping before the hit: {d} vs {r}");
                }
                assert!((h.normal.length() - 1.0).abs() < 1e-3);
                assert!(
                    h.normal.dot(delta) <= 1e-3 * len,
                    "the normal faces the motion"
                );
            }
            None => {
                misses += 1;
                for i in 0..=256 {
                    let s = delta * (i as f32 / 256.0);
                    let d = segment_distance(&tri, a + s, b + s);
                    assert!(d > r - 2e-3, "a miss, yet overlapping: {d} vs {r}");
                }
            }
        }
    }
    assert!(
        hits > 500 && misses > 300 && inside > 50,
        "hits {hits}, misses {misses}, inside {inside}"
    );
}

#[test]
fn a_capsule_sweep_is_never_later_than_a_sphere_sweep_from_a_point_of_its_axis() {
    let mut rng = Lcg(43);
    for _ in 0..3000 {
        let tri = rng.triangle(3.0);
        let center = rng.vec(5.0);
        let half = rng.vec(1.0);
        let (a, b) = (center - half, center + half);
        let r = rng.range(0.05, 1.0);
        let target = (tri.a + tri.b + tri.c) / 3.0 + rng.vec(2.0);
        let delta = (target - center) * rng.range(0.3, 2.0);
        let capsule = tri.sweep_capsule(a, b, r, delta);
        for s in [0.0, 0.25, 0.5, 0.8, 1.0] {
            let p = a + (b - a) * s;
            if let Some(sphere) = tri.sweep_sphere(p, r, delta) {
                let capsule = capsule.expect("the capsule missed what a sphere on its axis hit");
                assert!(
                    capsule.t <= sphere.t + 1e-4,
                    "the capsule hit later: {} > {}",
                    capsule.t,
                    sphere.t
                );
            }
        }
    }
}

#[test]
fn capsule_overlap_matches_the_sampled_distance() {
    let mut rng = Lcg(47);
    let (mut yes, mut no) = (0, 0);
    for _ in 0..4000 {
        let tri = rng.triangle(3.0);
        let center = rng.vec(4.0);
        let half = rng.vec(1.5);
        let r = rng.range(0.05, 1.5);
        let d = segment_distance(&tri, center - half, center + half);
        if (d - r).abs() < 1e-3 {
            continue;
        }
        let got = tri.overlaps_capsule(center - half, center + half, r);
        assert_eq!(got, d < r, "distance {d}, radius {r}");
        if got {
            yes += 1;
        } else {
            no += 1;
        }
    }
    assert!(yes > 300 && no > 500, "{yes} {no}");
}

#[test]
fn the_closest_points_to_a_segment_are_as_close_as_the_reference() {
    let mut rng = Lcg(53);
    for _ in 0..3000 {
        let tri = rng.triangle(3.0);
        let (p, q) = (rng.vec(4.0), rng.vec(4.0));
        let (on_segment, on_triangle) = tri.closest_points_segment(p, q);
        let reference = segment_distance(&tri, p, q);
        assert!(((on_segment - on_triangle).length() - reference).abs() < 1e-3);
        assert!((tri.closest_point(on_triangle) - on_triangle).length() < 1e-3);
        let along = closest_points_segment_segment(p, q, on_segment, on_segment).0;
        assert!((along - on_segment).length() < 1e-3, "on the segment");
    }
}

// ---- boxes --------------------------------------------------------------------------------------

#[test]
fn a_box_lands_on_a_floor_and_slides_along_it_freely() {
    let mesh = floor(5.0);
    let (hit, _) = mesh
        .sweep_aabb(
            Vec3::new(1.0, 3.0, 1.0),
            Vec3::splat(0.5),
            Vec3::NEG_Y * 10.0,
            Layers::ALL,
        )
        .unwrap();
    assert!((hit.t * 10.0 - 2.5).abs() < 1e-5, "{hit:?}");
    assert_eq!(hit.normal, Vec3::Y);
    assert!((hit.position - Vec3::new(1.0, 0.5, 1.0)).length() < 1e-5);
    assert!(
        mesh.sweep_aabb(
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::splat(0.5),
            Vec3::new(2.0, 0.0, -1.0),
            Layers::ALL
        )
        .is_none(),
        "resting on the floor and sliding is no hit"
    );
    assert!(
        mesh.sweep_aabb(
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::splat(0.5),
            Vec3::Y,
            Layers::ALL
        )
        .is_none(),
        "nor is lifting off"
    );
    let (sunk, _) = mesh
        .sweep_aabb(
            Vec3::new(1.0, 0.4, 1.0),
            Vec3::splat(0.5),
            Vec3::X,
            Layers::ALL,
        )
        .unwrap();
    assert!(sunk.overlapping_at_start && sunk.t == 0.0);
    assert_eq!(sunk.normal, Vec3::Y, "the shallowest way out");
    let (still, _) = mesh
        .sweep_aabb(
            Vec3::new(1.0, 0.4, 1.0),
            Vec3::splat(0.5),
            Vec3::ZERO,
            Layers::ALL,
        )
        .unwrap();
    assert!(
        still.overlapping_at_start,
        "a zero-length cast checks for overlap"
    );
    assert!(
        mesh.sweep_aabb(
            Vec3::new(1.0, 2.0, 1.0),
            Vec3::splat(0.5),
            Vec3::ZERO,
            Layers::ALL
        )
        .is_none()
    );
}

#[test]
fn a_box_meets_a_slanted_edge_with_the_edge_axis() {
    // A roof ridge along the diagonal of X and Z, at y = 0: a box dropped on it touches the ridge
    // with its bottom face first, at the box's centre height 0.5.
    let tri = Triangle::new(
        Vec3::new(-3.0, 0.0, -3.0),
        Vec3::new(3.0, 0.0, 3.0),
        Vec3::new(0.0, -2.0, 3.0),
    );
    let hit = tri
        .sweep_aabb(
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::splat(0.5),
            Vec3::NEG_Y * 5.0,
        )
        .unwrap();
    assert!((hit.t * 5.0 - 2.5).abs() < 1e-4, "{hit:?}");
    assert!(hit.normal.y > 0.0);
    // Sideways into the same ridge, the contact is an edge of the box on the ridge.
    let hit = tri
        .sweep_aabb(
            Vec3::new(-3.0, 0.0, 3.0),
            Vec3::splat(0.5),
            Vec3::new(6.0, 0.0, -6.0),
        )
        .unwrap();
    let at = Vec3::new(-3.0, 0.0, 3.0) + Vec3::new(6.0, 0.0, -6.0) * hit.t;
    assert!(box_meets(&tri, at, Vec3::splat(0.501)));
    assert!(!box_meets(&tri, at, Vec3::splat(0.499)));
}

#[test]
fn box_sweeps_match_clipping_the_triangle_to_the_box() {
    let mut rng = Lcg(59);
    let (mut hits, mut misses, mut inside) = (0, 0, 0);
    for _ in 0..3000 {
        let tri = rng.triangle(3.0);
        let center = rng.vec(5.0);
        let half = Vec3::new(
            rng.range(0.2, 1.2),
            rng.range(0.2, 1.2),
            rng.range(0.2, 1.2),
        );
        let target = (tri.a + tri.b + tri.c) / 3.0 + rng.vec(2.0);
        let delta = (target - center) * rng.range(0.3, 2.0);
        let (grown, shrunk) = (half + Vec3::splat(2e-3), half - Vec3::splat(2e-3));
        match tri.sweep_aabb(center, half, delta) {
            Some(h) if h.overlapping_at_start => {
                inside += 1;
                assert!(box_meets(&tri, center, grown));
                assert!((h.normal.length() - 1.0).abs() < 1e-3);
            }
            Some(h) => {
                hits += 1;
                assert!(
                    box_meets(&tri, center + delta * h.t, grown),
                    "apart at the hit"
                );
                for i in 0..64 {
                    let at = center + delta * (h.t * i as f32 / 64.0);
                    assert!(!box_meets(&tri, at, shrunk), "overlapping before the hit");
                }
                assert!((h.normal.length() - 1.0).abs() < 1e-3);
                assert!(h.normal.dot(delta) <= 1e-3 * delta.length());
                assert_eq!(h.position, center + delta * h.t);
            }
            None => {
                misses += 1;
                for i in 0..=256 {
                    let at = center + delta * (i as f32 / 256.0);
                    assert!(!box_meets(&tri, at, shrunk), "a miss, yet overlapping");
                }
            }
        }
    }
    assert!(
        hits > 500 && misses > 300 && inside > 50,
        "hits {hits}, misses {misses}, inside {inside}"
    );
}

#[test]
fn box_overlap_matches_clipping() {
    let mut rng = Lcg(61);
    let (mut yes, mut no) = (0, 0);
    for _ in 0..4000 {
        let tri = rng.triangle(3.0);
        let center = rng.vec(4.0);
        let half = Vec3::new(
            rng.range(0.1, 2.0),
            rng.range(0.1, 2.0),
            rng.range(0.1, 2.0),
        );
        let big = box_meets(&tri, center, half + Vec3::splat(1e-3));
        let small = box_meets(&tri, center, half - Vec3::splat(1e-3));
        if big != small {
            continue;
        }
        let got = tri.overlaps_aabb(&Aabb::from_center_half(center, half));
        assert_eq!(got, big, "{tri:?} {center} {half}");
        if got {
            yes += 1;
        } else {
            no += 1;
        }
    }
    assert!(yes > 500 && no > 500, "{yes} {no}");
}

// ---- meshes -------------------------------------------------------------------------------------

fn random_mesh(rng: &mut Lcg, count: usize) -> TriMesh {
    let mut v = Vec::new();
    let mut idx = Vec::new();
    for i in 0..count {
        let base = rng.vec(10.0);
        let t = rng.triangle(2.0);
        v.extend([base + t.a, base + t.b, base + t.c]);
        idx.extend([3 * i as u32, 3 * i as u32 + 1, 3 * i as u32 + 2]);
    }
    TriMesh::new(&v, &idx).unwrap()
}

#[test]
fn mesh_sweeps_and_overlaps_match_a_loop_over_every_triangle() {
    let mut rng = Lcg(67);
    let mesh = random_mesh(&mut rng, 120);
    let n = mesh.triangle_count() as u32;
    for _ in 0..600 {
        let center = rng.vec(12.0);
        let delta = rng.vec(10.0);
        let half_h = rng.range(0.0, 1.0);
        let r = rng.range(0.1, 0.8);
        let (a, b) = upright(center, half_h);
        // Capsules: the nearest, ties to the lower index.
        let brute = (0..n)
            .filter_map(|i| {
                mesh.triangle(i)
                    .sweep_capsule(a, b, r, delta)
                    .map(|h| (h.t, i))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let got = mesh
            .sweep_capsule(a, b, r, delta, Layers::ALL)
            .map(|(h, i)| (h.t, i));
        assert_eq!(got, brute);
        // Boxes.
        let half = Vec3::new(
            rng.range(0.1, 1.0),
            rng.range(0.1, 1.0),
            rng.range(0.1, 1.0),
        );
        let brute = (0..n)
            .filter_map(|i| {
                mesh.triangle(i)
                    .sweep_aabb(center, half, delta)
                    .map(|h| (h.t, i))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let got = mesh
            .sweep_aabb(center, half, delta, Layers::ALL)
            .map(|(h, i)| (h.t, i));
        assert_eq!(got, brute);
        // Overlaps.
        let mut found = Vec::new();
        mesh.query_capsule(a, b, r * 3.0, Layers::ALL, &mut |i| found.push(i));
        found.sort_unstable();
        let want: Vec<u32> = (0..n)
            .filter(|i| mesh.triangle(*i).overlaps_capsule(a, b, r * 3.0))
            .collect();
        assert_eq!(found, want);
        let region = Aabb::from_center_half(center, half * 3.0);
        let mut found = Vec::new();
        mesh.query_box(&region, Layers::ALL, &mut |i| found.push(i));
        found.sort_unstable();
        let want: Vec<u32> = (0..n)
            .filter(|i| mesh.triangle(*i).overlaps_aabb(&region))
            .collect();
        assert_eq!(found, want);
    }
}

#[test]
fn raycast_all_lists_every_triangle_crossed_nearest_first() {
    let mut rng = Lcg(71);
    let mesh = random_mesh(&mut rng, 150);
    let n = mesh.triangle_count() as u32;
    let mut crossed_many = 0;
    for _ in 0..400 {
        let origin = rng.vec(14.0);
        let ray = Ray::new(origin, rng.vec(4.0) - origin).unwrap();
        let max_t = rng.range(1.0, 40.0);
        let mut all = vec![MeshHit {
            t: -1.0,
            point: Vec3::ZERO,
            normal: Vec3::Y,
            triangle: 9999,
        }];
        mesh.raycast_all(&ray, max_t, Layers::ALL, &mut all);
        assert_eq!(all[0].triangle, 9999, "appended, not replaced");
        let all = &all[1..];
        let mut want: Vec<(f32, u32)> = (0..n)
            .filter_map(|i| mesh.triangle(i).raycast(&ray, max_t).map(|h| (h.t, i)))
            .collect();
        want.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let got: Vec<(f32, u32)> = all.iter().map(|h| (h.t, h.triangle)).collect();
        assert_eq!(got, want);
        if let Some(first) = all.first() {
            let nearest = mesh.raycast(&ray, max_t, Layers::ALL).unwrap();
            assert_eq!(first.t, nearest.t, "the first is the nearest");
        }
        if all.len() >= 2 {
            crossed_many += 1;
        }
    }
    assert!(crossed_many > 10, "{crossed_many}");
    // A ray down the seam of a floor's two triangles meets both at once: lower index first.
    let mut seam = Vec::new();
    floor(5.0).raycast_all(
        &Ray::new(Vec3::new(1.0, 3.0, 1.0), Vec3::NEG_Y).unwrap(),
        10.0,
        Layers::ALL,
        &mut seam,
    );
    assert_eq!(seam.iter().map(|h| h.triangle).collect::<Vec<_>>(), [0, 1]);
}
