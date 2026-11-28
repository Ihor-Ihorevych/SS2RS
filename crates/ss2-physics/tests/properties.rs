//! Property tests for the physics queries (roadmap T07-025): random meshes, rays and sweeps, checked
//! against slow brute-force answers and against invariants, instead of hand-picked examples.
//!
//! What they check:
//! * the BVH ray cast finds **exactly what a loop over every triangle finds**, and does not depend on the order of the
//!   triangles;
//! * a sphere sweep **never tunnels**: if the sphere, sampled along its path, ever overlaps a triangle, the sweep reports a hit at or
//!   before that sample; and the position it stops at is not inside anything;
//! * a sphere sweep is **never later than a ray** along the same line (a sphere cannot pass through what a ray hits);
//! * a capsule sweep and a box sweep **never tunnel** either, a capsule stops outside what it hits, and a capsule sweep is
//!   **never later than a sphere sweep** of the same radius from any point of its axis (T07-004b);
//! * `raycast_all` lists **exactly what a loop over every triangle finds**, nearest first, and starts with what `raycast` finds;
//! * the moving colliders of a `QueryWorld` give the same answer **whatever order they were handed over in** (rays, capsule and
//!   box casts);
//! * the same queries asked from **several threads** give the same bits as from one.
//!
//! What they do **not** check: "no stuck in corners" and "no tunneling at maximum speed" of the character controller,
//! which does not exist yet (T07-005); the property of tunneling here is about the sweep itself.
//! A failing case is shrunk and printed with its minimal input; **it is not saved to a file** (proptest cannot find a
//! `lib.rs` next to an integration test, so its regression files do not work here): copy the printed input into an ordinary
//! test next to the bug's fix. `PROPTEST_CASES=20000` runs more cases than the default 256.

use proptest::prelude::*;
use ss2_core::math::Vec3;
use ss2_physics::geometry::{Aabb, Ray};
use ss2_physics::query::{
    ColliderId, DynamicCollider, PhysMaterialId, QueryFilter, QueryWorld, Shape, SpatialQuery,
};
use ss2_physics::spatial::{Layers, TriMesh, Triangle};

fn coord() -> impl Strategy<Value = f32> {
    -20.0f32..20.0
}

fn point() -> impl Strategy<Value = Vec3> {
    (coord(), coord(), coord()).prop_map(|(x, y, z)| Vec3::new(x, y, z))
}

/// A triangle with area (corners at least a little apart, not on a line).
fn triangle() -> impl Strategy<Value = [Vec3; 3]> {
    (point(), point(), point())
        .prop_filter("a triangle with area", |(a, b, c)| {
            Triangle::new(*a, *b, *c)
                .aabb()
                .max
                .distance(Triangle::new(*a, *b, *c).aabb().min)
                > 0.5
                && !Triangle::new(*a, *b, *c).is_degenerate()
                && (*b - *a).cross(*c - *a).length() > 0.5
        })
        .prop_map(|(a, b, c)| [a, b, c])
}

fn soup(max: usize) -> impl Strategy<Value = Vec<[Vec3; 3]>> {
    prop::collection::vec(triangle(), 1..max)
}

/// What a ray cast found, as bits: the distance and the point.
type HitBits = Option<(u32, [u32; 3])>;

fn mesh_of(triangles: &[[Vec3; 3]]) -> TriMesh {
    let vertices: Vec<Vec3> = triangles.iter().flatten().copied().collect();
    let indices: Vec<u32> = (0..vertices.len() as u32).collect();
    TriMesh::new(&vertices, &indices).expect("a soup of valid triangles builds")
}

/// The least distance from 33 points along an upright axis (centre `centre`, half length `half_height`) to the
/// triangles: never less than the true distance from the axis, so a value under the radius means a sure overlap.
fn axis_distance(tris: &[Triangle], centre: Vec3, half_height: f32) -> f32 {
    (0..=32)
        .map(|i| centre + Vec3::Y * (half_height * (i as f32 / 16.0 - 1.0)))
        .flat_map(|p| tris.iter().map(move |t| (p - t.closest_point(p)).length()))
        .fold(f32::MAX, f32::min)
}

fn ray_from(origin: Vec3, toward: Vec3) -> Option<Ray> {
    Ray::new(origin, toward - origin)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]

    /// The tree finds what a loop over every triangle finds.
    #[test]
    fn the_bvh_ray_cast_matches_a_loop_over_every_triangle(triangles in soup(40), origin in point(), toward in point()) {
        let Some(ray) = ray_from(origin, toward) else { return Ok(()) };
        let mesh = mesh_of(&triangles);
        let brute = triangles
            .iter()
            .filter_map(|t| Triangle::new(t[0], t[1], t[2]).raycast(&ray, 1000.0))
            .map(|h| h.t)
            .fold(None, |best: Option<f32>, t| Some(best.map_or(t, |b| b.min(t))));
        let tree = mesh.raycast(&ray, 1000.0, Layers::ALL).map(|h| h.t);
        match (brute, tree) {
            (None, None) => {}
            (Some(a), Some(b)) => prop_assert!((a - b).abs() < 1e-4, "brute {a} tree {b}"),
            other => prop_assert!(false, "one finds a hit and the other does not: {other:?}"),
        }
    }

    /// A ray finds a hit that is within its length and nothing beyond it.
    #[test]
    fn the_ray_cast_respects_its_maximum_distance(triangles in soup(30), origin in point(), toward in point()) {
        let Some(ray) = ray_from(origin, toward) else { return Ok(()) };
        let mesh = mesh_of(&triangles);
        let Some(nearest) = mesh.raycast(&ray, 1000.0, Layers::ALL).map(|h| h.t) else { return Ok(()) };
        if nearest < 0.05 {
            return Ok(());
        }
        let long_enough = mesh.raycast(&ray, nearest * 1.01 + 1e-3, Layers::ALL).map(|h| h.t);
        prop_assert!(long_enough.is_some_and(|t| (t - nearest).abs() < 1e-4), "{long_enough:?} for a hit at {nearest}");
        prop_assert!(mesh.raycast(&ray, nearest * 0.99 - 1e-3, Layers::ALL).is_none(), "a ray that stops short of the nearest hit at {nearest} still hit something");
    }

    /// Building the mesh from the triangles in another order changes nothing.
    #[test]
    fn the_ray_cast_does_not_depend_on_the_order_of_the_triangles(triangles in soup(30), origin in point(), toward in point(), rotate in 0usize..30) {
        let Some(ray) = ray_from(origin, toward) else { return Ok(()) };
        let mut shuffled = triangles.clone();
        shuffled.reverse();
        let r = rotate % shuffled.len();
        shuffled.rotate_left(r);
        let a = mesh_of(&triangles).raycast(&ray, 1000.0, Layers::ALL).map(|h| h.t);
        let b = mesh_of(&shuffled).raycast(&ray, 1000.0, Layers::ALL).map(|h| h.t);
        match (a, b) {
            (None, None) => {}
            (Some(x), Some(y)) => prop_assert!((x - y).abs() < 1e-4, "{x} vs {y}"),
            other => prop_assert!(false, "{other:?}"),
        }
    }

    /// A sphere that overlaps a triangle somewhere along its path is stopped at or before that place.
    #[test]
    fn a_sphere_sweep_never_tunnels(triangles in soup(25), start in point(), end in point(), radius in 0.2f32..2.0) {
        const STEPS: usize = 64;
        let mesh = mesh_of(&triangles);
        let delta = end - start;
        if delta.length() < 0.5 {
            return Ok(());
        }
        let tris: Vec<Triangle> = triangles.iter().map(|t| Triangle::new(t[0], t[1], t[2])).collect();
        // The first sample at which the sphere is clearly inside a triangle (not merely touching it).
        let first_inside = (0..=STEPS).find(|i| {
            let centre = start + delta * (*i as f32 / STEPS as f32);
            tris.iter().any(|t| (centre - t.closest_point(centre)).length() < radius - 1e-3)
        });
        let hit = mesh.sweep_sphere(start, radius, delta, Layers::ALL);
        if let Some(i) = first_inside {
            let (hit, _) = hit.ok_or_else(|| TestCaseError::fail(format!("the sphere is inside a triangle at sample {i} but the sweep found nothing")))?;
            prop_assert!(hit.t <= i as f32 / STEPS as f32 + 1e-3, "hit at {} but the sphere is already inside at {}", hit.t, i as f32 / STEPS as f32);
        }
    }

    /// Where a sweep stops, the sphere is not inside any triangle.
    #[test]
    fn a_sphere_stops_outside_what_it_hits(triangles in soup(25), start in point(), end in point(), radius in 0.2f32..2.0) {
        let mesh = mesh_of(&triangles);
        let delta = end - start;
        if delta.length() < 0.5 {
            return Ok(());
        }
        let tris: Vec<Triangle> = triangles.iter().map(|t| Triangle::new(t[0], t[1], t[2])).collect();
        if let Some((hit, _)) = mesh.sweep_sphere(start, radius, delta, Layers::ALL)
            && !hit.overlapping_at_start
        {
            let nearest = tris.iter().map(|t| (hit.position - t.closest_point(hit.position)).length()).fold(f32::MAX, f32::min);
            prop_assert!(nearest >= radius - 2e-3, "stopped {nearest} from a triangle with radius {radius}");
            prop_assert!((hit.normal.length() - 1.0).abs() < 1e-3, "the normal is a unit vector");
        }
    }

    /// A sphere cannot pass through what a ray along the same line hits: it touches it no later (a sphere of
    /// radius 0 included). The other way round is **not** a property: a sphere of radius 0 can report a touch where
    /// the ray misses by a hair (about 2e-3 on a large triangle), because the zero-radius edge test is a tangent test.
    #[test]
    fn a_sphere_sweep_is_never_later_than_a_ray_along_the_same_line(triangles in soup(25), origin in point(), toward in point(), radius in prop_oneof![Just(0.0f32), 0.01f32..1.0]) {
        let delta = toward - origin;
        let len = delta.length();
        if len < 0.5 {
            return Ok(());
        }
        let Some(ray) = Ray::new(origin, delta) else { return Ok(()) };
        let mesh = mesh_of(&triangles);
        if let Some(hit) = mesh.raycast(&ray, len, Layers::ALL) {
            let swept = mesh.sweep_sphere(origin, radius, delta, Layers::ALL);
            let (sweep, _) = swept.ok_or_else(|| TestCaseError::fail(format!("the ray hits at {} but the sphere of radius {radius} finds nothing", hit.t)))?;
            prop_assert!(sweep.t * len <= hit.t + 1e-3, "sphere touches at {} but the ray hits at {}", sweep.t * len, hit.t);
        }
    }

    /// A capsule that clearly overlaps a triangle somewhere along its path is stopped at or before that place.
    #[test]
    fn a_capsule_sweep_never_tunnels(triangles in soup(25), start in point(), end in point(), half_height in 0.0f32..2.0, radius in 0.2f32..1.5) {
        const STEPS: usize = 64;
        let mesh = mesh_of(&triangles);
        let delta = end - start;
        if delta.length() < 0.5 {
            return Ok(());
        }
        let tris: Vec<Triangle> = triangles.iter().map(|t| Triangle::new(t[0], t[1], t[2])).collect();
        let first_inside = (0..=STEPS).find(|i| {
            let centre = start + delta * (*i as f32 / STEPS as f32);
            axis_distance(&tris, centre, half_height) < radius - 1e-3
        });
        let hit = mesh.sweep_capsule(start - Vec3::Y * half_height, start + Vec3::Y * half_height, radius, delta, Layers::ALL);
        if let Some(i) = first_inside {
            let (hit, _) = hit.ok_or_else(|| TestCaseError::fail(format!("the capsule is inside a triangle at sample {i} but the sweep found nothing")))?;
            prop_assert!(hit.t <= i as f32 / STEPS as f32 + 1e-3, "hit at {} but the capsule is already inside at {}", hit.t, i as f32 / STEPS as f32);
        }
    }

    /// Where a capsule sweep stops, no point of its axis is nearer than the radius to a triangle.
    #[test]
    fn a_capsule_stops_outside_what_it_hits(triangles in soup(25), start in point(), end in point(), half_height in 0.0f32..2.0, radius in 0.2f32..1.5) {
        let mesh = mesh_of(&triangles);
        let delta = end - start;
        if delta.length() < 0.5 {
            return Ok(());
        }
        let tris: Vec<Triangle> = triangles.iter().map(|t| Triangle::new(t[0], t[1], t[2])).collect();
        let (a, b) = (start - Vec3::Y * half_height, start + Vec3::Y * half_height);
        if let Some((hit, _)) = mesh.sweep_capsule(a, b, radius, delta, Layers::ALL)
            && !hit.overlapping_at_start
        {
            let nearest = axis_distance(&tris, hit.position, half_height);
            prop_assert!(nearest >= radius - 2e-3, "stopped {nearest} from a triangle with radius {radius}");
            prop_assert!((hit.normal.length() - 1.0).abs() < 1e-3, "the normal is a unit vector");
            prop_assert!(hit.normal.dot(delta) <= 1e-3 * delta.length(), "the normal faces the motion");
        }
    }

    /// A capsule cannot pass through what a sphere of its radius, moved along with any point of its axis, touches.
    #[test]
    fn a_capsule_sweep_is_never_later_than_a_sphere_sweep_from_its_axis(triangles in soup(25), start in point(), end in point(), half_height in 0.0f32..2.0, radius in 0.05f32..1.5, along in 0.0f32..=1.0) {
        let mesh = mesh_of(&triangles);
        let delta = end - start;
        let (a, b) = (start - Vec3::Y * half_height, start + Vec3::Y * half_height);
        let capsule = mesh.sweep_capsule(a, b, radius, delta, Layers::ALL);
        for p in [a, b, a + (b - a) * along] {
            if let Some((sphere, _)) = mesh.sweep_sphere(p, radius, delta, Layers::ALL) {
                let (capsule, _) = capsule.ok_or_else(|| TestCaseError::fail(format!("a sphere at {p} hits at {} but the capsule finds nothing", sphere.t)))?;
                prop_assert!(capsule.t <= sphere.t + 1e-4, "the capsule touches at {} but the sphere at {}", capsule.t, sphere.t);
            }
        }
    }

    /// A box that clearly overlaps a triangle somewhere along its path is stopped at or before that place, and
    /// where it stops it is touching and not inside.
    #[test]
    fn a_box_sweep_never_tunnels(triangles in soup(25), start in point(), end in point(), hx in 0.2f32..2.0, hy in 0.2f32..2.0, hz in 0.2f32..2.0) {
        const STEPS: usize = 64;
        let mesh = mesh_of(&triangles);
        let delta = end - start;
        if delta.length() < 0.5 {
            return Ok(());
        }
        let half = Vec3::new(hx, hy, hz);
        let tris: Vec<Triangle> = triangles.iter().map(|t| Triangle::new(t[0], t[1], t[2])).collect();
        let inside = |centre: Vec3, half: Vec3| {
            tris.iter().any(|t| t.overlaps_aabb(&Aabb::from_center_half(centre, half)))
        };
        let shrunk = half - Vec3::splat(2e-3);
        let first_inside = (0..=STEPS).find(|i| inside(start + delta * (*i as f32 / STEPS as f32), shrunk));
        let hit = mesh.sweep_aabb(start, half, delta, Layers::ALL);
        if let Some(i) = first_inside {
            let (hit, _) = hit.ok_or_else(|| TestCaseError::fail(format!("the box is inside a triangle at sample {i} but the sweep found nothing")))?;
            prop_assert!(hit.t <= i as f32 / STEPS as f32 + 1e-3, "hit at {} but the box is already inside at {}", hit.t, i as f32 / STEPS as f32);
        }
        if let Some((hit, _)) = hit
            && !hit.overlapping_at_start
        {
            prop_assert!(!inside(hit.position, shrunk), "stopped inside a triangle");
            prop_assert!(inside(hit.position, half + Vec3::splat(2e-3)), "stopped short of the triangle it hit");
        }
    }

    /// `raycast_all` finds what a loop over every triangle finds, nearest first, and begins with the nearest hit.
    #[test]
    fn raycast_all_matches_a_loop_over_every_triangle(triangles in soup(40), origin in point(), toward in point()) {
        let Some(ray) = ray_from(origin, toward) else { return Ok(()) };
        let mesh = mesh_of(&triangles);
        let mut want: Vec<(f32, u32)> = (0..mesh.triangle_count() as u32)
            .filter_map(|i| mesh.triangle(i).raycast(&ray, 1000.0).map(|h| (h.t, i)))
            .collect();
        want.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let mut all = Vec::new();
        mesh.raycast_all(&ray, 1000.0, Layers::ALL, &mut all);
        prop_assert_eq!(all.iter().map(|h| (h.t, h.triangle)).collect::<Vec<_>>(), want);
        prop_assert_eq!(all.first().map(|h| h.t), mesh.raycast(&ray, 1000.0, Layers::ALL).map(|h| h.t));
    }

    /// Capsule and box casts against the moving colliders do not depend on the order they were handed over in.
    #[test]
    fn shape_casts_do_not_depend_on_the_order_of_the_moving_colliders(
        things in prop::collection::vec((point(), 0.3f32..2.0, 0u8..3), 1..30),
        from in point(),
        toward in point(),
        rotate in 0usize..30,
    ) {
        let colliders: Vec<DynamicCollider> = things
            .iter()
            .enumerate()
            .map(|(i, (p, r, kind))| DynamicCollider {
                id: ColliderId(i as u64),
                layers: Layers::ALL,
                shape: match kind {
                    0 => Shape::Sphere { radius: *r },
                    1 => Shape::Capsule { half_height: *r, radius: *r * 0.5 },
                    _ => Shape::Box { half_extents: Vec3::new(*r, *r * 0.5, *r) },
                },
                position: *p,
                material: PhysMaterialId(0),
            })
            .collect();
        let mut shuffled = colliders.clone();
        shuffled.reverse();
        let r = rotate % shuffled.len();
        shuffled.rotate_left(r);
        let (mut a, mut b) = (QueryWorld::new(), QueryWorld::new());
        a.set_dynamic(colliders);
        b.set_dynamic(shuffled);
        let f = QueryFilter::default();
        for shape in [Shape::Capsule { half_height: 0.7, radius: 0.4 }, Shape::Box { half_extents: Vec3::new(0.5, 0.8, 0.3) }] {
            prop_assert_eq!(a.shapecast(&shape, from, toward - from, &f), b.shapecast(&shape, from, toward - from, &f));
        }
    }

    /// The moving colliders may be handed over in any order.
    #[test]
    fn the_answer_does_not_depend_on_the_order_of_the_moving_colliders(
        spheres in prop::collection::vec((point(), 0.3f32..2.0), 1..30),
        origin in point(),
        toward in point(),
        rotate in 0usize..30,
    ) {
        let Some(ray) = ray_from(origin, toward) else { return Ok(()) };
        let colliders: Vec<DynamicCollider> = spheres
            .iter()
            .enumerate()
            .map(|(i, (p, r))| DynamicCollider {
                id: ColliderId(i as u64),
                layers: Layers::ALL,
                shape: Shape::Sphere { radius: *r },
                position: *p,
                material: PhysMaterialId(0),
            })
            .collect();
        let mut shuffled = colliders.clone();
        shuffled.reverse();
        let r = rotate % shuffled.len();
        shuffled.rotate_left(r);
        let (mut a, mut b) = (QueryWorld::new(), QueryWorld::new());
        a.set_dynamic(colliders);
        b.set_dynamic(shuffled);
        let f = QueryFilter::default();
        prop_assert_eq!(a.raycast(&ray, 1000.0, &f), b.raycast(&ray, 1000.0, &f));
    }

    /// Asking from several threads gives the same bits as asking from one.
    #[test]
    fn queries_give_the_same_bits_from_several_threads(triangles in soup(30), rays in prop::collection::vec((point(), point()), 1..40)) {
        let mesh = mesh_of(&triangles);
        let rays: Vec<Ray> = rays.iter().filter_map(|(o, t)| ray_from(*o, *t)).collect();
        let ask = |mesh: &TriMesh| -> Vec<HitBits> {
            rays.iter()
                .map(|r| {
                    mesh.raycast(r, 1000.0, Layers::ALL)
                        .map(|h| (h.t.to_bits(), [h.point.x.to_bits(), h.point.y.to_bits(), h.point.z.to_bits()]))
                })
                .collect()
        };
        let single = ask(&mesh);
        let threaded: Vec<Vec<HitBits>> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..4).map(|_| s.spawn(|| ask(&mesh))).collect();
            handles.into_iter().map(|h| h.join().expect("a query thread did not panic")).collect()
        });
        for t in &threaded {
            prop_assert_eq!(t, &single);
        }
    }
}
