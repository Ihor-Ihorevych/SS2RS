//! Physics queries (roadmap T07-017): the BVH, mesh ray casts and sphere sweeps, and the named
//! workload "300 AI + 500 debris + projectiles" as one tick of queries. Synthetic scenes only (no
//! game data); the numbers and what they mean are in `docs/PERF_BUDGETS.md` section 5.6.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ss2_core::math::Vec3;
use ss2_physics::geometry::{Aabb, Ray};
use ss2_physics::query::{
    ColliderId, DynamicCollider, PhysMaterialId, QueryFilter, QueryWorld, Shape, SpatialQuery,
};
use ss2_physics::spatial::{Bvh, Layers, TriMesh};

/// A small deterministic generator, so every run measures the same scene.
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
}

/// `n` boxes scattered over a 200 x 20 x 200 arena.
fn boxes(n: usize) -> Vec<(Aabb, Layers, u32)> {
    let mut rng = Lcg(1);
    (0..n)
        .map(|i| {
            let c = Vec3::new(
                rng.range(-100.0, 100.0),
                rng.range(0.0, 20.0),
                rng.range(-100.0, 100.0),
            );
            (
                Aabb::from_center_half(c, Vec3::splat(rng.range(0.2, 1.0))),
                Layers::ALL,
                i as u32,
            )
        })
        .collect()
}

/// A hilly terrain of `side` by `side` cells, two triangles each, 1 unit apart.
fn terrain(side: u32) -> TriMesh {
    let height = |x: u32, z: u32| ((x as f32 * 0.21).sin() + (z as f32 * 0.17).cos()) * 1.5;
    let mut vertices = Vec::new();
    for z in 0..=side {
        for x in 0..=side {
            vertices.push(Vec3::new(
                x as f32 - side as f32 / 2.0,
                height(x, z),
                z as f32 - side as f32 / 2.0,
            ));
        }
    }
    let mut indices = Vec::new();
    let stride = side + 1;
    for z in 0..side {
        for x in 0..side {
            let i = z * stride + x;
            indices.extend([i, i + stride, i + 1, i + 1, i + stride, i + stride + 1]);
        }
    }
    TriMesh::new(&vertices, &indices).expect("a valid terrain")
}

fn rays(n: usize, extent: f32) -> Vec<Ray> {
    let mut rng = Lcg(2);
    (0..n)
        .filter_map(|_| {
            let origin = Vec3::new(
                rng.range(-extent, extent),
                rng.range(2.0, 15.0),
                rng.range(-extent, extent),
            );
            let target = Vec3::new(
                rng.range(-extent, extent),
                rng.range(-2.0, 4.0),
                rng.range(-extent, extent),
            );
            Ray::new(origin, target - origin)
        })
        .collect()
}

fn bvh(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_bvh");
    for n in [1_000_usize, 10_000, 100_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("build", n), &n, |b, &n| {
            b.iter_batched(
                || boxes(n),
                |items| black_box(Bvh::build(items)),
                BatchSize::LargeInput,
            );
        });
        let items = boxes(n);
        let bounds: Vec<Aabb> = items.iter().map(|i| i.0).collect();
        let tree = Bvh::build(items);
        let rays = rays(256, 100.0);
        group.throughput(Throughput::Elements(rays.len() as u64));
        group.bench_with_input(BenchmarkId::new("nearest_hit_256_rays", n), &n, |b, _| {
            b.iter(|| {
                let mut hits = 0_u32;
                for ray in &rays {
                    // The exact test of a box is the ray against its bounds.
                    let hit = tree.query_ray(ray, 200.0, Layers::ALL, 0.0, &mut |i, best| {
                        bounds[*i as usize].raycast(ray, best).map(|h| h.t)
                    });
                    hits += u32::from(hit.is_some());
                }
                black_box(hits)
            });
        });
    }
    group.finish();
}

fn mesh(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_mesh");
    for side in [64_u32, 256] {
        let triangles = u64::from(side * side * 2);
        let mesh = terrain(side);
        let rays = rays(256, side as f32 / 2.0 - 2.0);
        group.throughput(Throughput::Elements(rays.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("raycast_256", triangles),
            &triangles,
            |b, _| {
                b.iter(|| {
                    let mut hits = 0_u32;
                    for ray in &rays {
                        hits += u32::from(mesh.raycast(ray, 100.0, Layers::ALL).is_some());
                    }
                    black_box(hits)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("sphere_sweep_256", triangles),
            &triangles,
            |b, _| {
                let mut rng = Lcg(3);
                let starts: Vec<Vec3> = (0..256)
                    .map(|_| {
                        Vec3::new(
                            rng.range(-(side as f32) / 2.0 + 2.0, side as f32 / 2.0 - 2.0),
                            6.0,
                            rng.range(-(side as f32) / 2.0 + 2.0, side as f32 / 2.0 - 2.0),
                        )
                    })
                    .collect();
                b.iter(|| {
                    let mut hits = 0_u32;
                    for from in &starts {
                        hits += u32::from(
                            mesh.sweep_sphere(*from, 0.4, Vec3::new(0.0, -10.0, 0.0), Layers::ALL)
                                .is_some(),
                        );
                    }
                    black_box(hits)
                });
            },
        );
    }
    group.finish();
}

/// "300 AI + 500 debris + projectiles" (T07-017) on a 128 x 128 terrain: the moving colliders are
/// rebuilt, then 300 enemies each look at the player (a ray) and probe the ground (a sphere cast
/// down), 100 projectiles move (sphere casts), and 20 explosions ask what is in range (overlaps).
fn horde_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_horde_tick");
    let mesh = terrain(128);
    let triangles = mesh.triangle_count();
    let mut world = QueryWorld::new();
    world
        .set_static(mesh, vec![PhysMaterialId(1); triangles])
        .expect("one material per triangle");

    let mut rng = Lcg(4);
    let mut at = |y_lo: f32, y_hi: f32| {
        Vec3::new(
            rng.range(-60.0, 60.0),
            rng.range(y_lo, y_hi),
            rng.range(-60.0, 60.0),
        )
    };
    let mut colliders = Vec::new();
    for i in 0..300_u64 {
        colliders.push(DynamicCollider {
            id: ColliderId(i),
            layers: Layers::bit(1),
            shape: Shape::Capsule {
                half_height: 0.6,
                radius: 0.4,
            },
            position: at(1.0, 3.0),
            material: PhysMaterialId(2),
        });
    }
    for i in 300..800_u64 {
        colliders.push(DynamicCollider {
            id: ColliderId(i),
            layers: Layers::bit(2),
            shape: Shape::Sphere { radius: 0.15 },
            position: at(0.5, 4.0),
            material: PhysMaterialId(3),
        });
    }
    let player = Vec3::new(0.0, 3.0, 0.0);
    let sight: Vec<Ray> = colliders[..300]
        .iter()
        .filter_map(|c| Ray::new(c.position, player - c.position))
        .collect();
    let ground: Vec<Vec3> = colliders[..300].iter().map(|c| c.position).collect();
    let projectiles: Vec<(Vec3, Vec3)> = (0..100)
        .map(|_| (at(1.0, 3.0), Vec3::new(rng_dir(), 0.0, rng_dir()) * 2.0))
        .collect();
    let blasts: Vec<Vec3> = (0..20).map(|_| at(0.5, 2.0)).collect();
    let filter = QueryFilter::default();

    group.bench_function("rebuild_800_colliders", |b| {
        b.iter_batched(
            || colliders.clone(),
            |set| world.set_dynamic(black_box(set)),
            BatchSize::LargeInput,
        );
    });
    world.set_dynamic(colliders.clone());
    group.bench_function(
        "queries_300_sight_300_ground_100_projectiles_20_blasts",
        |b| {
            b.iter(|| {
                let mut seen = 0_u32;
                for ray in &sight {
                    seen += u32::from(world.raycast(ray, 80.0, &filter).is_none());
                }
                for p in &ground {
                    seen += u32::from(
                        world
                            .shapecast(
                                &Shape::Sphere { radius: 0.4 },
                                *p,
                                Vec3::new(0.0, -4.0, 0.0),
                                &filter,
                            )
                            .unwrap()
                            .is_some(),
                    );
                }
                for (from, delta) in &projectiles {
                    seen += u32::from(
                        world
                            .shapecast(&Shape::Sphere { radius: 0.2 }, *from, *delta, &filter)
                            .unwrap()
                            .is_some(),
                    );
                }
                let mut out = Vec::new();
                for blast in &blasts {
                    out.clear();
                    world
                        .overlap(&Shape::Sphere { radius: 6.0 }, *blast, &filter, &mut out)
                        .unwrap();
                    seen += out.len() as u32;
                }
                black_box(seen)
            });
        },
    );
    group.finish();
}

/// A fixed pseudo-random direction component in -1..1 for the projectile set.
fn rng_dir() -> f32 {
    use std::cell::Cell;
    thread_local!(static STATE: Cell<u64> = const { Cell::new(99) });
    STATE.with(|s| {
        let next = s
            .get()
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        s.set(next);
        (((next >> 33) as f32) / (1_u64 << 31) as f32) * 2.0 - 1.0
    })
}

criterion_group!(benches, bvh, mesh, horde_tick);
criterion_main!(benches);
