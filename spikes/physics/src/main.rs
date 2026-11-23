//! Throwaway benchmark behind `docs/rfcs/0008-physics-engine.md` (T07-006): `rapier3d` against the
//! query layer we already have (`ss2-physics`), on the things the game needs.
//!
//! Every result is one line `key value`, so the numbers can be copied into the RFC. Run it with
//! `cargo run --release`; run it several times to see whether the determinism hashes change.

use std::time::Instant;

use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::prelude::*;
use ss2_core::math::Vec3;
use ss2_physics::geometry::Ray as OurRay;
use ss2_physics::query::{
    ColliderId, DynamicCollider, PhysMaterialId, QueryFilter as OurFilter, QueryWorld,
    Shape as OurShape, SpatialQuery,
};
use ss2_physics::spatial::{Layers, TriMesh};

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32 * (hi - lo)
    }
}

fn fnv(hash: &mut u64, bits: u32) {
    for byte in bits.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
}

fn median_us(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn height(x: f32, z: f32) -> f32 {
    2.0 * (x * 0.05).sin() * (z * 0.07).cos() + 0.5 * (x * 0.3 + z * 0.2).sin()
}

/// A terrain of `cells` by `cells` squares of 1 unit, two triangles each.
fn terrain(cells: usize) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let n = cells + 1;
    let mut vertices = Vec::with_capacity(n * n);
    for iz in 0..n {
        for ix in 0..n {
            let (x, z) = (ix as f32, iz as f32);
            vertices.push([x, height(x, z), z]);
        }
    }
    let mut triangles = Vec::with_capacity(cells * cells * 2);
    for iz in 0..cells {
        for ix in 0..cells {
            let a = (iz * n + ix) as u32;
            let (b, c, d) = (a + 1, a + n as u32, a + n as u32 + 1);
            triangles.push([a, c, b]);
            triangles.push([b, c, d]);
        }
    }
    (vertices, triangles)
}

fn section(name: &str) {
    println!("\n== {name}");
}

fn report(key: &str, value: impl std::fmt::Display) {
    println!("{key} {value}");
}

// ---- A: static triangle mesh ----------------------------------------------------------------

fn static_mesh_queries() {
    section("A. static triangle mesh: build and raycast");
    for cells in [100usize, 300] {
        let (vertices, triangles) = terrain(cells);
        let triangle_count = triangles.len();
        report("mesh_triangles", triangle_count);

        let ours_vertices: Vec<Vec3> = vertices
            .iter()
            .map(|v| Vec3::new(v[0], v[1], v[2]))
            .collect();
        let ours_indices: Vec<u32> = triangles.iter().flatten().copied().collect();
        let t = Instant::now();
        let mesh = TriMesh::new(&ours_vertices, &ours_indices).unwrap();
        let mut ours = QueryWorld::new();
        ours.set_static(mesh, vec![PhysMaterialId(0); triangle_count])
            .unwrap();
        report(
            "ours_build_ms",
            format!("{:.2}", t.elapsed().as_secs_f64() * 1e3),
        );

        let t = Instant::now();
        let mut world = PhysicsWorld::default();
        let rv: Vec<Vector> = vertices
            .iter()
            .map(|v| Vector::new(v[0], v[1], v[2]))
            .collect();
        world.insert_collider(
            ColliderBuilder::trimesh(rv, triangles.clone())
                .unwrap()
                .build(),
            None,
        );
        world.step(); // brings the broad phase up to date
        report(
            "rapier_build_ms",
            format!("{:.2}", t.elapsed().as_secs_f64() * 1e3),
        );

        // Rays from above, slightly slanted, like shots and view traces.
        let mut rng = Rng(7);
        let rays: Vec<([f32; 3], [f32; 3])> = (0..100_000)
            .map(|_| {
                let o = [
                    rng.range(1.0, cells as f32 - 1.0),
                    rng.range(6.0, 12.0),
                    rng.range(1.0, cells as f32 - 1.0),
                ];
                let d = [rng.range(-0.4, 0.4), -1.0, rng.range(-0.4, 0.4)];
                (o, d)
            })
            .collect();

        let t = Instant::now();
        let mut ours_hits = Vec::with_capacity(rays.len());
        for (o, d) in &rays {
            let ray =
                OurRay::new(Vec3::new(o[0], o[1], o[2]), Vec3::new(d[0], d[1], d[2])).unwrap();
            ours_hits.push(
                ours.raycast(&ray, 1000.0, &OurFilter::default())
                    .map(|h| h.distance),
            );
        }
        let ours_s = t.elapsed().as_secs_f64();

        let t = Instant::now();
        let mut rapier_hits = Vec::with_capacity(rays.len());
        for (o, d) in &rays {
            let dir = Vector::new(d[0], d[1], d[2]);
            let norm = dir.length();
            let ray = Ray::new(Vector::new(o[0], o[1], o[2]), dir / norm);
            rapier_hits.push(
                world
                    .cast_ray(
                        &ray,
                        1000.0,
                        true,
                        rapier3d::pipeline::QueryFilter::default(),
                    )
                    .map(|(_, toi)| toi),
            );
        }
        let rapier_s = t.elapsed().as_secs_f64();
        report(
            "ours_ns_per_ray",
            format!("{:.0}", ours_s * 1e9 / rays.len() as f64),
        );
        report(
            "rapier_ns_per_ray",
            format!("{:.0}", rapier_s * 1e9 / rays.len() as f64),
        );

        let (mut both, mut only_ours, mut only_rapier, mut max_diff) = (0, 0, 0, 0.0f32);
        for (a, b) in ours_hits.iter().zip(&rapier_hits) {
            match (a, b) {
                (Some(x), Some(y)) => {
                    both += 1;
                    max_diff = max_diff.max((x - y).abs());
                }
                (Some(_), None) => only_ours += 1,
                (None, Some(_)) => only_rapier += 1,
                (None, None) => {}
            }
        }
        report("agree_hits", both);
        report("only_ours_hit", only_ours);
        report("only_rapier_hit", only_rapier);
        report("max_distance_difference", format!("{max_diff:.6}"));

        // Capsule casts (what a character controller asks for): ours has none yet (T07-004b).
        let capsule = Capsule::new_y(0.5, 0.4);
        let t = Instant::now();
        let mut capsule_hits = 0;
        for (o, d) in rays.iter().take(50_000) {
            let velocity = Vector::new(d[0], d[1], d[2]) * 14.0;
            let options = rapier3d::parry::query::ShapeCastOptions::with_max_time_of_impact(1.0);
            let pose = Pose::from_translation(Vector::new(o[0], o[1], o[2]));
            if world
                .cast_shape(
                    &pose,
                    velocity,
                    &capsule,
                    options,
                    rapier3d::pipeline::QueryFilter::default(),
                )
                .is_some()
            {
                capsule_hits += 1;
            }
        }
        report(
            "rapier_capsule_cast_ns",
            format!("{:.0}", t.elapsed().as_secs_f64() * 1e9 / 50_000.0),
        );
        report("rapier_capsule_cast_hits_of_50000", capsule_hits);
    }
}

// ---- B: many moving colliders ------------------------------------------------------------------

fn moving_colliders() {
    section("B. 2,000 moving spheres: per-tick update plus 1,000 rays");
    let count = 2_000usize;
    let mut rng = Rng(11);
    let start: Vec<[f32; 3]> = (0..count)
        .map(|_| {
            [
                rng.range(0.0, 200.0),
                rng.range(0.0, 20.0),
                rng.range(0.0, 200.0),
            ]
        })
        .collect();
    let rays: Vec<([f32; 3], [f32; 3])> = (0..1_000)
        .map(|_| {
            (
                [rng.range(0.0, 200.0), 30.0, rng.range(0.0, 200.0)],
                [rng.range(-0.3, 0.3), -1.0, rng.range(-0.3, 0.3)],
            )
        })
        .collect();

    // Ours: the moving colliders are replaced each tick (and sorted and the tree rebuilt).
    let mut ours = QueryWorld::new();
    let mut tick_us = Vec::new();
    let mut hits_ours = 0;
    for tick in 0..60 {
        let dx = tick as f32 * 0.1;
        let t = Instant::now();
        ours.set_dynamic(
            start
                .iter()
                .enumerate()
                .map(|(i, p)| DynamicCollider {
                    id: ColliderId(i as u64),
                    layers: Layers::ALL,
                    shape: OurShape::Sphere { radius: 0.5 },
                    position: Vec3::new(p[0] + dx, p[1], p[2]),
                    material: PhysMaterialId(0),
                })
                .collect(),
        );
        hits_ours = 0;
        for (o, d) in &rays {
            let ray =
                OurRay::new(Vec3::new(o[0], o[1], o[2]), Vec3::new(d[0], d[1], d[2])).unwrap();
            if ours.raycast(&ray, 1000.0, &OurFilter::default()).is_some() {
                hits_ours += 1;
            }
        }
        tick_us.push(t.elapsed().as_secs_f64() * 1e6);
    }
    report(
        "ours_tick_us_update_and_1000_rays",
        format!("{:.0}", median_us(tick_us)),
    );

    // rapier: kinematic bodies moved every tick, one step, the same rays.
    let mut world = PhysicsWorld::default();
    world.gravity = Vector::ZERO;
    let handles: Vec<RigidBodyHandle> = start
        .iter()
        .map(|p| {
            world
                .insert(
                    RigidBodyBuilder::kinematic_position_based()
                        .translation(Vector::new(p[0], p[1], p[2])),
                    ColliderBuilder::ball(0.5),
                )
                .0
        })
        .collect();
    let mut tick_us = Vec::new();
    let mut hits_rapier = 0;
    for tick in 0..60 {
        let dx = tick as f32 * 0.1;
        let t = Instant::now();
        for (h, p) in handles.iter().zip(&start) {
            world.bodies[*h].set_next_kinematic_translation(Vector::new(p[0] + dx, p[1], p[2]));
        }
        world.step();
        hits_rapier = 0;
        for (o, d) in &rays {
            let dir = Vector::new(d[0], d[1], d[2]);
            let ray = Ray::new(Vector::new(o[0], o[1], o[2]), dir / dir.length());
            if world
                .cast_ray(
                    &ray,
                    1000.0,
                    true,
                    rapier3d::pipeline::QueryFilter::default(),
                )
                .is_some()
            {
                hits_rapier += 1;
            }
        }
        tick_us.push(t.elapsed().as_secs_f64() * 1e6);
    }
    report(
        "rapier_tick_us_update_and_1000_rays",
        format!("{:.0}", median_us(tick_us)),
    );
    report(
        "ray_hits_ours_vs_rapier_last_tick",
        format!("{hits_ours} {hits_rapier}"),
    );
}

// ---- C and D: rigid bodies and determinism ------------------------------------------------------

fn dynamics_world(boxes_per_side: usize) -> (PhysicsWorld, Vec<RigidBodyHandle>) {
    let mut world = PhysicsWorld::default();
    let (vertices, triangles) = terrain(60);
    let rv: Vec<Vector> = vertices
        .iter()
        .map(|v| Vector::new(v[0], v[1], v[2]))
        .collect();
    world.insert_collider(
        ColliderBuilder::trimesh(rv, triangles).unwrap().build(),
        None,
    );
    let mut handles = Vec::new();
    for ix in 0..boxes_per_side {
        for iy in 0..boxes_per_side {
            for iz in 0..boxes_per_side {
                let p = Vector::new(
                    20.0 + ix as f32 * 1.3,
                    8.0 + iy as f32 * 1.3,
                    20.0 + iz as f32 * 1.3,
                );
                let (h, _) = world.insert(
                    RigidBodyBuilder::dynamic().translation(p),
                    ColliderBuilder::cuboid(0.5, 0.5, 0.5),
                );
                handles.push(h);
            }
        }
    }
    (world, handles)
}

fn state_hash(world: &PhysicsWorld, handles: &[RigidBodyHandle]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for handle in handles {
        let body = &world.bodies[*handle];
        let t = body.translation();
        for v in [t.x, t.y, t.z] {
            fnv(&mut h, v.to_bits());
        }
    }
    h
}

fn rigid_bodies() {
    section("C. rigid bodies: boxes dropped on the terrain, 240 steps at 60 Hz");
    for side in [4usize, 6, 10] {
        let (mut world, handles) = dynamics_world(side);
        let mut step_us = Vec::new();
        for _ in 0..240 {
            let t = Instant::now();
            world.step();
            step_us.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let mean: f64 = step_us.iter().sum::<f64>() / step_us.len() as f64;
        let mut sorted = step_us.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let finite = handles
            .iter()
            .all(|h| world.bodies[*h].translation().is_finite());
        let asleep = handles
            .iter()
            .filter(|h| world.bodies[**h].is_sleeping())
            .count();
        report(
            &format!("bodies_{}_step_us_mean_p99_max", handles.len()),
            format!(
                "{mean:.0} {:.0} {:.0}",
                sorted[sorted.len() * 99 / 100],
                sorted[sorted.len() - 1]
            ),
        );
        report(
            &format!("bodies_{}_all_finite_asleep_after_240_steps", handles.len()),
            format!("{finite} {asleep}"),
        );
    }
}

fn determinism() {
    section(
        "D. determinism: the same scene twice in this process (compare across processes by running again)",
    );
    let run = || {
        let (mut world, handles) = dynamics_world(6);
        for _ in 0..240 {
            world.step();
        }
        state_hash(&world, &handles)
    };
    let (a, b) = (run(), run());
    report("hash_run_1", format!("{a:016x}"));
    report("hash_run_2", format!("{b:016x}"));
    report("same_process_identical", a == b);
}

// ---- E: ragdoll ------------------------------------------------------------------------------------

fn ragdoll() {
    section("E. ragdoll: a chain of 12 capsules joined by spherical joints, dropped on a floor");
    for (name, multibody) in [("impulse_joints", false), ("multibody_joints", true)] {
        let mut world = PhysicsWorld::default();
        world.insert_collider(
            ColliderBuilder::cuboid(50.0, 0.5, 50.0)
                .translation(Vector::new(0.0, -0.5, 0.0))
                .build(),
            None,
        );
        let mut previous: Option<RigidBodyHandle> = None;
        let mut handles = Vec::new();
        for i in 0..12 {
            let (h, _) = world.insert(
                RigidBodyBuilder::dynamic().translation(Vector::new(
                    0.03 * i as f32,
                    6.0 - i as f32 * 0.5,
                    0.0,
                )), // a lean, so that the chain does not balance
                ColliderBuilder::capsule_y(0.15, 0.1),
            );
            if let Some(p) = previous {
                let joint = SphericalJointBuilder::new()
                    .local_anchor1(Vector::new(0.0, -0.25, 0.0))
                    .local_anchor2(Vector::new(0.0, 0.25, 0.0));
                if multibody {
                    world.insert_multibody_joint(p, h, joint);
                } else {
                    world.insert_impulse_joint(p, h, joint);
                }
            }
            previous = Some(h);
            handles.push(h);
        }
        let t = Instant::now();
        for _ in 0..360 {
            world.step();
        }
        let per_step = t.elapsed().as_secs_f64() * 1e6 / 360.0;
        let finite = handles
            .iter()
            .all(|h| world.bodies[*h].translation().is_finite());
        let max_speed = handles
            .iter()
            .map(|h| world.bodies[*h].linvel().length())
            .fold(0.0f32, f32::max);
        let lowest = handles
            .iter()
            .map(|h| world.bodies[*h].translation().y)
            .fold(f32::MAX, f32::min);
        let highest = handles
            .iter()
            .map(|h| world.bodies[*h].translation().y)
            .fold(f32::MIN, f32::max);
        report(
            &format!("{name}_step_us_all_finite_max_speed_lowest_highest"),
            format!("{per_step:.1} {finite} {max_speed:.3} {lowest:.2} {highest:.2}"),
        );
    }
}

// ---- F: character controller ------------------------------------------------------------------------

fn character() {
    section("F. kinematic character controller: capsule 1.8 high, radius 0.4");
    let mut world = PhysicsWorld::default();
    // A floor, a step of 0.3, a wall further on and a 60 degree ramp.
    world.insert_collider(
        ColliderBuilder::cuboid(100.0, 0.5, 100.0)
            .translation(Vector::new(0.0, -0.5, 0.0))
            .build(),
        None,
    );
    world.insert_collider(
        ColliderBuilder::cuboid(5.0, 0.15, 5.0)
            .translation(Vector::new(10.0, 0.15, 0.0))
            .build(),
        None,
    );
    world.insert_collider(
        ColliderBuilder::cuboid(0.5, 3.0, 5.0)
            .translation(Vector::new(30.0, 3.0, 0.0))
            .build(),
        None,
    );
    world.step();
    let shape = Capsule::new_y(0.5, 0.4);
    let mut controller = KinematicCharacterController::default();
    controller.autostep = Some(CharacterAutostep {
        max_height: CharacterLength::Absolute(0.4),
        min_width: CharacterLength::Absolute(0.2),
        include_dynamic_bodies: false,
    });
    controller.snap_to_ground = Some(CharacterLength::Absolute(0.2));
    let mut pos = Vector::new(0.0, 0.91, 0.0);
    let (mut steps_total, mut grounded_ticks) = (0, 0);
    let t = Instant::now();
    for _ in 0..600 {
        // Walk along +x at 8 units per second, with gravity pulling down.
        let desired = Vector::new(8.0 / 60.0, -9.81 / 60.0 / 60.0, 0.0);
        let query = world.query_pipeline();
        let moved = controller.move_shape(
            1.0 / 60.0,
            &query,
            &shape,
            &Pose::from_translation(pos),
            desired,
            |_| {},
        );
        pos += moved.translation;
        steps_total += 1;
        grounded_ticks += moved.grounded as i32;
    }
    let per_step = t.elapsed().as_secs_f64() * 1e6 / steps_total as f64;
    report(
        "walk_600_ticks_final_x_y_grounded_ticks",
        format!("{:.2} {:.2} {grounded_ticks}", pos.x, pos.y),
    );
    report("controller_move_us_per_tick", format!("{per_step:.1}"));
    report("climbed_the_0.3_step", pos.x > 15.0);
    report("stopped_by_the_wall", pos.x < 29.6);
}

fn main() {
    static_mesh_queries();
    moving_colliders();
    rigid_bodies();
    determinism();
    ragdoll();
    character();
}
