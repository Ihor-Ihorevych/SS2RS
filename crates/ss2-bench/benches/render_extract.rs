//! The per-tick render extract: filling, validating and interpolating a `RenderWorld`.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ss2_bench::{fill_world, sample_world};
use ss2_core::math::{Quat, Vec3};
use ss2_render_api::{Camera, NullRenderer, Renderer, Transform};

fn extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_extract");
    for n in [100_u64, 1_000, 10_000] {
        group.throughput(Throughput::Elements(n));

        // What the sim does every tick: reset a reused world and refill it.
        group.bench_with_input(BenchmarkId::new("reset_and_fill", n), &n, |b, &n| {
            let mut world = sample_world(0, n);
            let mut tick = 0;
            b.iter(|| {
                tick += 1;
                world.reset(tick);
                fill_world(&mut world, n);
                black_box(world.renderables.len())
            });
        });

        // What every renderer does on submit.
        group.bench_with_input(BenchmarkId::new("validate", n), &n, |b, &n| {
            let world = sample_world(1, n);
            b.iter(|| black_box(&world).validate().unwrap());
        });

        // Submit through the null renderer (validate + double-buffer swap, moves the world).
        group.bench_with_input(BenchmarkId::new("submit_null", n), &n, |b, &n| {
            let mut renderer = NullRenderer::new();
            let mut tick = 0;
            b.iter_batched_ref(
                || {
                    tick += 1;
                    sample_world(tick, n)
                },
                |world| renderer.submit(std::mem::take(world)).unwrap(),
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

fn math(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_math");
    let a = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let b_t = Transform {
        translation: Vec3::new(4.0, 5.0, 6.0),
        rotation: Quat::from_rotation_y(1.0),
        scale: Vec3::splat(2.0),
    };
    group.bench_function("transform_interpolate", |b| {
        b.iter(|| black_box(&a).interpolate(black_box(&b_t), black_box(0.37)));
    });
    group.bench_function("transform_matrix", |b| b.iter(|| black_box(&b_t).matrix()));
    let cam = Camera::default();
    group.bench_function("camera_view_projection", |b| {
        b.iter(|| black_box(&cam).view_projection(black_box(16.0 / 9.0)));
    });
    group.finish();
}

criterion_group!(benches, extract, math);
criterion_main!(benches);
