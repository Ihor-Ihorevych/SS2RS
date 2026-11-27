//! The static collision world of a level (RFC 0013).
//!
//! Every collidable instance's [`CollisionData`](crate::scene::CollisionData), moved into world
//! space by its transform, goes into one triangle mesh with one physics material. Mirrored
//! placements need no special care: the queries treat triangles as two-sided.
//!
//! ```
//! use ss2_core::math::Vec3;
//! use ss2_physics::geometry::Ray;
//! use ss2_physics::query::{QueryFilter, SpatialQuery};
//! use ss2_world::{collision::build_static, scene::LevelScene};
//!
//! let world = build_static(&LevelScene::test_room()).unwrap();
//! let down = Ray::new(Vec3::new(0.0, 5.0, 6.0), Vec3::NEG_Y).unwrap();
//! let hit = world.raycast(&down, 100.0, &QueryFilter::default()).unwrap();
//! assert!(hit.point.y.abs() < 1e-4); // the floor's top
//! ```

use ss2_core::math::Vec3;
use ss2_core::{Error, Result};
use ss2_physics::query::{PhysMaterialId, QueryWorld};
use ss2_physics::spatial::TriMesh;

use crate::scene::LevelScene;

/// Build the static collision world of `scene`.
///
/// # Errors
/// [`Error::Invalid`] if the collision data is broken (run [`LevelScene::validate`] first) or
/// has more vertices than a `u32` index can reach.
pub fn build_static(scene: &LevelScene) -> Result<QueryWorld> {
    let bad = |detail: String| Error::invalid("collision", detail);
    let mut positions: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (i, inst) in scene.instances.iter().enumerate() {
        if !inst.collidable {
            continue;
        }
        let model = scene
            .models
            .get(inst.model as usize)
            .ok_or_else(|| bad(format!("instance {i} names a missing model {}", inst.model)))?;
        let Some(c) = &model.collision else { continue };
        let base = u32::try_from(positions.len())
            .map_err(|_| bad("more vertices than a u32 index reaches".to_string()))?;
        if let Some(x) = c.indices.iter().find(|x| **x as usize >= c.positions.len()) {
            return Err(bad(format!(
                "model {} ({}) index {x} of {} positions",
                inst.model,
                model.name,
                c.positions.len()
            )));
        }
        let m = inst.transform.matrix();
        positions.extend(c.positions.iter().map(|p| m.transform_point3(*p)));
        indices.extend(c.indices.iter().map(|x| base + x));
    }
    let mesh = TriMesh::new(&positions, &indices).map_err(|e| bad(e.to_string()))?;
    let materials = vec![PhysMaterialId(0); mesh.triangle_count()];
    let mut world = QueryWorld::new();
    world
        .set_static(mesh, materials)
        .map_err(|e| bad(e.to_string()))?;
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_physics::geometry::Ray;
    use ss2_physics::query::{QueryFilter, Shape, SpatialQuery};

    fn down_from(x: f32, z: f32, world: &QueryWorld) -> Option<f32> {
        let ray = Ray::new(Vec3::new(x, 10.0, z), Vec3::NEG_Y).unwrap();
        world
            .raycast(&ray, 100.0, &QueryFilter::default())
            .map(|h| h.point.y)
    }

    #[test]
    fn the_test_room_collides_where_it_is_drawn() {
        let world = build_static(&LevelScene::test_room()).unwrap();
        let close = |a: Option<f32>, b: f32| (a.unwrap() - b).abs() < 1e-4;
        assert!(close(down_from(0.0, 6.0, &world), 0.0), "floor");
        assert!(close(down_from(3.0, -3.0, &world), 0.4), "low step");
        assert!(close(down_from(-3.0, -3.0, &world), 1.0), "high step");
        let on_ramp = down_from(7.0, 4.0, &world).unwrap();
        assert!(on_ramp > 0.4 && on_ramp < 2.2, "ramp at {on_ramp}");
        // The mirrored pillar stands at (-6, _, 5), 2 m tall.
        assert!(close(down_from(-6.0, 5.0, &world), 2.0), "mirrored pillar");
        // A sphere rolled at a wall stops in front of it.
        let hit = world
            .shapecast(
                &Shape::Sphere { radius: 0.5 },
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, -20.0),
                &QueryFilter::default(),
            )
            .unwrap()
            .unwrap();
        assert!(
            (hit.distance - (10.0 - 0.1 - 0.5)).abs() < 1e-3,
            "{}",
            hit.distance
        );
        assert_eq!(hit.material, PhysMaterialId(0));
    }

    #[test]
    fn instances_that_do_not_collide_are_left_out() {
        let mut room = LevelScene::test_room();
        room.instances[5].collidable = false; // the low step
        let world = build_static(&room).unwrap();
        assert!(
            (down_from(3.0, -3.0, &world).unwrap()).abs() < 1e-4,
            "falls to the floor"
        );
        // A model without collision data adds nothing either.
        let mut room = LevelScene::test_room();
        room.models[0].collision = None;
        let world = build_static(&room).unwrap();
        assert_eq!(down_from(0.0, 6.0, &world), None);
    }

    #[test]
    fn an_empty_level_builds_an_empty_world() {
        let mut room = LevelScene::test_room();
        room.instances.clear();
        let world = build_static(&room).unwrap();
        assert_eq!(down_from(0.0, 0.0, &world), None);
    }

    #[test]
    fn broken_collision_is_an_error() {
        let mut room = LevelScene::test_room();
        room.models[0]
            .collision
            .as_mut()
            .unwrap()
            .indices
            .push(9999);
        assert!(build_static(&room).is_err());
        let mut room = LevelScene::test_room();
        room.instances[0].model = 42;
        assert!(build_static(&room).is_err());
    }
}
