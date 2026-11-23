//! Drawing the physical world for a developer (roadmap T07-016): colliders, the BVH, ray and shape
//! casts and triangle meshes, as lines in a [`DebugDraw`] under [`DebugCategory::Physics`].
//!
//! Nothing here is drawn unless the `physics` overlay is on (`r_debug physics`), so the calls can
//! stay in the code. The renderer's line pass (T04-018b) draws what is recorded.

use ss2_core::math::{Mat4, Vec3};
use ss2_physics::geometry::{Aabb, Capsule, Obb, Ray};
use ss2_physics::query::{DynamicCollider, Hit, Shape};
use ss2_physics::spatial::{Bvh, TriMesh};
use ss2_render_api::{
    Aabb as DebugAabb, DebugCategory, DebugColor, DebugDraw, Sphere as DebugSphere,
};

const CATEGORY: DebugCategory = DebugCategory::Physics;

fn aabb(debug: &mut DebugDraw, b: &Aabb, color: DebugColor) {
    debug.aabb(CATEGORY, DebugAabb::new(b.min, b.max), color);
}

fn sphere(debug: &mut DebugDraw, center: Vec3, radius: f32, color: DebugColor) {
    debug.sphere(CATEGORY, DebugSphere::new(center, radius), color);
}

/// A capsule: a sphere at each end and four lines along the sides.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_game::physics_debug::draw_capsule;
/// use ss2_physics::geometry::Capsule;
/// use ss2_render_api::{DebugCategory, DebugColor, DebugDraw};
///
/// let mut debug = DebugDraw::new(10_000, 16);
/// let capsule = Capsule::new(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5);
/// draw_capsule(&mut debug, &capsule, DebugColor::GREEN);
/// assert_eq!(debug.line_count(), 0, "the physics overlay is off");
/// debug.overlays_mut().set(DebugCategory::Physics, true); // `r_debug physics`
/// draw_capsule(&mut debug, &capsule, DebugColor::GREEN);
/// assert!(debug.line_count() > 0);
/// ```
pub fn draw_capsule(debug: &mut DebugDraw, capsule: &Capsule, color: DebugColor) {
    sphere(debug, capsule.a, capsule.radius, color);
    sphere(debug, capsule.b, capsule.radius, color);
    let axis = capsule.b - capsule.a;
    if let Some(u) = axis.try_normalize() {
        let (p, q) = u.any_orthonormal_pair();
        for side in [p, -p, q, -q] {
            let offset = side * capsule.radius;
            debug.line(CATEGORY, capsule.a + offset, capsule.b + offset, color);
        }
    }
}

/// An oriented box: its twelve edges.
pub fn draw_obb(debug: &mut DebugDraw, obb: &Obb, color: DebugColor) {
    let local = DebugAabb::new(-obb.half_extents, obb.half_extents);
    debug.obb(
        CATEGORY,
        local,
        Mat4::from_rotation_translation(obb.rotation, obb.center),
        color,
    );
}

/// The boxes of a BVH down to `max_depth` (the root is depth 0): inner nodes blue, leaves yellow.
pub fn draw_bvh<T>(debug: &mut DebugDraw, tree: &Bvh<T>, max_depth: usize) {
    tree.visit_nodes(&mut |depth, bounds, is_leaf| {
        if depth <= max_depth {
            aabb(
                debug,
                bounds,
                if is_leaf {
                    DebugColor::YELLOW
                } else {
                    DebugColor::BLUE
                },
            );
        }
    });
}

/// A ray cast: green to the hit with an arrow for the surface normal, or red to its full length
/// when nothing was hit.
pub fn draw_ray_cast(debug: &mut DebugDraw, ray: &Ray, max_distance: f32, hit: Option<&Hit>) {
    match hit {
        Some(hit) => {
            debug.line(CATEGORY, ray.origin, hit.point, DebugColor::GREEN);
            debug.arrow(
                CATEGORY,
                hit.point,
                hit.point + hit.normal * 0.5,
                DebugColor::YELLOW,
            );
        }
        None => debug.line(
            CATEGORY,
            ray.origin,
            ray.point_at(max_distance),
            DebugColor::RED,
        ),
    }
}

fn draw_shape(debug: &mut DebugDraw, shape: &Shape, at: Vec3, color: DebugColor) {
    match *shape {
        Shape::Sphere { radius } => sphere(debug, at, radius, color),
        Shape::Box { half_extents } => {
            aabb(debug, &Aabb::from_center_half(at, half_extents), color)
        }
        Shape::Capsule {
            half_height,
            radius,
        } => {
            let up = Vec3::new(0.0, half_height, 0.0);
            draw_capsule(debug, &Capsule::new(at - up, at + up, radius), color);
        }
    }
}

/// A shape cast: the shape at its start (white), the path, and either where it stopped with the
/// contact normal (red) or the shape at the end of the path (green).
pub fn draw_shape_cast(
    debug: &mut DebugDraw,
    shape: &Shape,
    from: Vec3,
    delta: Vec3,
    hit: Option<&Hit>,
) {
    draw_shape(debug, shape, from, DebugColor::WHITE);
    match hit {
        Some(hit) => {
            let stop = from
                + delta
                    .try_normalize()
                    .map_or(Vec3::ZERO, |d| d * hit.distance);
            draw_shape(debug, shape, stop, DebugColor::RED);
            debug.line(CATEGORY, from, stop, DebugColor::WHITE);
            debug.line(CATEGORY, stop, from + delta, DebugColor::GREEN);
            debug.line(
                CATEGORY,
                hit.point,
                hit.point + hit.normal * 0.5,
                DebugColor::YELLOW,
            );
        }
        None => {
            draw_shape(debug, shape, from + delta, DebugColor::GREEN);
            debug.line(CATEGORY, from, from + delta, DebugColor::WHITE);
        }
    }
}

/// The moving colliders, each by its shape, in yellow.
pub fn draw_colliders(debug: &mut DebugDraw, colliders: &[DynamicCollider]) {
    for c in colliders {
        draw_shape(debug, &c.shape, c.position, DebugColor::YELLOW);
    }
}

/// The edges of the first `max_triangles` triangles of a mesh, in green (a big level would fill the
/// buffer; the cap is a choice of the caller, usually with a bounds test of its own).
pub fn draw_mesh(debug: &mut DebugDraw, mesh: &TriMesh, max_triangles: usize) {
    for i in 0..mesh.triangle_count().min(max_triangles) {
        let t = mesh.triangle(i as u32);
        for (a, b) in [(t.a, t.b), (t.b, t.c), (t.c, t.a)] {
            debug.line(CATEGORY, a, b, DebugColor::GREEN);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::math::{Quat, Vec3};
    use ss2_physics::geometry::{Aabb, Capsule, Obb, Ray};
    use ss2_physics::query::{ColliderId, DynamicCollider, Hit, PhysMaterialId, Shape};
    use ss2_physics::spatial::{Bvh, Layers, TriMesh};
    use ss2_render_api::{DebugCategory, DebugColor, DebugDraw};

    fn on() -> DebugDraw {
        let mut d = DebugDraw::new(100_000, 100);
        d.overlays_mut().set(DebugCategory::Physics, true);
        d
    }

    fn off() -> DebugDraw {
        DebugDraw::new(100_000, 100)
    }

    #[test]
    fn nothing_is_drawn_unless_the_physics_overlay_is_on() {
        let mut d = off();
        let tree = Bvh::build(vec![(Aabb::new(Vec3::ZERO, Vec3::ONE), Layers::ALL, 0_u32)]);
        draw_bvh(&mut d, &tree, 8);
        draw_capsule(
            &mut d,
            &Capsule::new(Vec3::ZERO, Vec3::Y, 0.5),
            DebugColor::GREEN,
        );
        draw_obb(
            &mut d,
            &Obb::new(Vec3::ZERO, Vec3::ONE, Quat::IDENTITY),
            DebugColor::GREEN,
        );
        draw_ray_cast(&mut d, &Ray::new(Vec3::ZERO, Vec3::X).unwrap(), 10.0, None);
        assert_eq!(d.line_count(), 0);
        // Another overlay does not switch it on.
        d.overlays_mut().set(DebugCategory::Net, true);
        draw_capsule(
            &mut d,
            &Capsule::new(Vec3::ZERO, Vec3::Y, 0.5),
            DebugColor::GREEN,
        );
        assert_eq!(d.line_count(), 0);
    }

    #[test]
    fn a_capsule_is_two_end_spheres_and_four_sides() {
        let mut d = on();
        draw_capsule(
            &mut d,
            &Capsule::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 3.0, 0.0), 0.5),
            DebugColor::GREEN,
        );
        assert_eq!(
            d.line_count(),
            2 * 3 * DebugDraw::CIRCLE_SEGMENTS as usize + 4
        );
        // The sides run the length of the axis, on the surface.
        let sides: Vec<_> = d.lines().iter().rev().take(4).collect();
        for side in sides {
            assert!(
                (side.from.y - 0.0).abs() < 1e-5 && (side.to.y - 3.0).abs() < 1e-5,
                "{side:?}"
            );
            assert!((Vec3::new(side.from.x, 0.0, side.from.z).length() - 0.5).abs() < 1e-4);
        }
    }

    #[test]
    fn a_capsule_that_is_not_vertical_has_sides_along_its_own_axis() {
        let mut d = on();
        draw_capsule(
            &mut d,
            &Capsule::new(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), 1.0),
            DebugColor::GREEN,
        );
        for side in d.lines().iter().rev().take(4) {
            assert!(
                (side.to - side.from - Vec3::new(4.0, 0.0, 0.0)).length() < 1e-4,
                "{side:?}"
            );
        }
    }

    #[test]
    fn an_oriented_box_is_twelve_edges() {
        let mut d = on();
        draw_obb(
            &mut d,
            &Obb::new(
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::ONE,
                Quat::from_rotation_y(0.7),
            ),
            DebugColor::BLUE,
        );
        assert_eq!(d.line_count(), 12);
        // Turned about Y by 0.7 radians, a unit cube reaches further along x than it did (1.0): cos 0.7 + sin 0.7 = 1.41.
        let reach = d
            .lines()
            .iter()
            .flat_map(|l| [l.from.x, l.to.x])
            .fold(f32::MIN, f32::max);
        assert!(
            (reach - (5.0 + 0.7_f32.cos() + 0.7_f32.sin())).abs() < 1e-3,
            "{reach}"
        );
    }

    #[test]
    fn the_bvh_overlay_draws_one_box_per_node_down_to_a_depth() {
        let mut items = Vec::new();
        for i in 0..40_u32 {
            let c = Vec3::new((i % 8) as f32 * 3.0, (i / 8) as f32 * 3.0, 0.0);
            items.push((Aabb::from_center_half(c, Vec3::splat(0.5)), Layers::ALL, i));
        }
        let tree = Bvh::build(items);
        let mut nodes_by_depth = vec![0_usize; tree.depth() + 1];
        tree.visit_nodes(&mut |depth, _, _| nodes_by_depth[depth] += 1);
        assert_eq!(nodes_by_depth[0], 1, "one root");
        let (mut shallow, mut all) = (on(), on());
        draw_bvh(&mut shallow, &tree, 1);
        draw_bvh(&mut all, &tree, 100);
        assert_eq!(
            shallow.line_count(),
            12 * (nodes_by_depth[0] + nodes_by_depth[1]),
            "the first two levels"
        );
        assert_eq!(
            all.line_count(),
            12 * nodes_by_depth.iter().sum::<usize>(),
            "every node"
        );
        // Leaves and inner nodes are told apart by colour.
        let colours: std::collections::BTreeSet<[u8; 4]> =
            all.lines().iter().map(|l| l.color.0).collect();
        assert!(colours.len() >= 2, "{colours:?}");
        assert_eq!(
            all.lines()[0].color,
            DebugColor::BLUE,
            "the root is an inner node"
        );
        assert_eq!(
            all.lines().last().unwrap().color,
            DebugColor::YELLOW,
            "the last node visited is a leaf"
        );
    }

    #[test]
    fn a_ray_that_hit_is_drawn_to_the_hit_with_the_normal_and_a_miss_to_its_end() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y).unwrap();
        let hit = Hit {
            distance: 5.0,
            point: Vec3::ZERO,
            normal: Vec3::Y,
            material: PhysMaterialId(1),
            collider: None,
            started_overlapping: false,
        };
        let mut d = on();
        draw_ray_cast(&mut d, &ray, 20.0, Some(&hit));
        let first = d.lines()[0];
        assert_eq!((first.from, first.to), (ray.origin, Vec3::ZERO));
        assert_eq!(first.color, DebugColor::GREEN);
        assert!(d.line_count() > 1, "the normal is an arrow at the hit");
        assert!(
            d.lines().iter().skip(1).any(|l| l.from == Vec3::ZERO),
            "which starts on the surface"
        );
        let mut miss = on();
        draw_ray_cast(&mut miss, &ray, 20.0, None);
        assert_eq!(miss.line_count(), 1);
        assert_eq!(miss.lines()[0].to, Vec3::new(0.0, -15.0, 0.0));
        assert_eq!(miss.lines()[0].color, DebugColor::RED);
    }

    #[test]
    fn a_shape_cast_shows_the_start_the_path_and_where_it_stopped() {
        let hit = Hit {
            distance: 6.0,
            point: Vec3::new(6.0, 0.0, 0.0),
            normal: Vec3::NEG_X,
            material: PhysMaterialId(0),
            collider: Some(ColliderId(3)),
            started_overlapping: false,
        };
        let mut d = on();
        draw_shape_cast(
            &mut d,
            &Shape::Sphere { radius: 0.5 },
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Some(&hit),
        );
        let path = d
            .lines()
            .iter()
            .find(|l| l.from == Vec3::ZERO && l.color == DebugColor::WHITE);
        assert_eq!(
            path.map(|l| l.to),
            Some(Vec3::new(6.0, 0.0, 0.0)),
            "the shape stopped 6 along the path"
        );
        // Two spheres (start and stop) and the path.
        assert_eq!(
            d.line_count(),
            2 * 3 * DebugDraw::CIRCLE_SEGMENTS as usize + 1 + 1 + 1,
            "two spheres, the path to the hit, the rest of the path, the normal"
        );
        let mut free = on();
        draw_shape_cast(
            &mut free,
            &Shape::Box {
                half_extents: Vec3::ONE,
            },
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            None,
        );
        assert_eq!(free.line_count(), 12 * 2 + 1, "two boxes and the path");
    }

    #[test]
    fn colliders_are_drawn_by_shape_and_meshes_by_their_edges() {
        let mut d = on();
        let colliders = [
            DynamicCollider {
                id: ColliderId(1),
                layers: Layers::ALL,
                shape: Shape::Box {
                    half_extents: Vec3::ONE,
                },
                position: Vec3::ZERO,
                material: PhysMaterialId(0),
            },
            DynamicCollider {
                id: ColliderId(2),
                layers: Layers::ALL,
                shape: Shape::Sphere { radius: 1.0 },
                position: Vec3::X * 5.0,
                material: PhysMaterialId(0),
            },
            DynamicCollider {
                id: ColliderId(3),
                layers: Layers::ALL,
                shape: Shape::Capsule {
                    half_height: 1.0,
                    radius: 0.5,
                },
                position: Vec3::X * 9.0,
                material: PhysMaterialId(0),
            },
        ];
        draw_colliders(&mut d, &colliders);
        assert_eq!(
            d.line_count(),
            12 + 3 * DebugDraw::CIRCLE_SEGMENTS as usize
                + (2 * 3 * DebugDraw::CIRCLE_SEGMENTS as usize + 4)
        );
        let v = [
            Vec3::new(-5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, 5.0),
            Vec3::new(-5.0, 0.0, 5.0),
        ];
        let mesh = TriMesh::new(&v, &[0, 2, 1, 0, 3, 2]).unwrap();
        let mut m = on();
        draw_mesh(&mut m, &mesh, 100);
        assert_eq!(m.line_count(), 6, "three edges per triangle");
        let mut limited = on();
        draw_mesh(&mut limited, &mesh, 1);
        assert_eq!(
            limited.line_count(),
            3,
            "a limit on the triangles drawn keeps a big level from filling the buffer"
        );
    }
}
