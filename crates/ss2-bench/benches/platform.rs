//! Platform-contract hot paths: per-frame input action queries and lifecycle tracking.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use ss2_platform_api::{
    ActionMap, ActionQuery, Binding, GamepadButton, KeyCode, LifecycleEvent, LifecycleTracker,
    MockInput, MouseButton, SurfaceInfo,
};

const ACTIONS: [&str; 12] = [
    "move_forward",
    "move_back",
    "move_left",
    "move_right",
    "jump",
    "crouch",
    "fire",
    "alt_fire",
    "reload",
    "use",
    "next_weapon",
    "console",
];

fn action_map() -> ActionMap {
    let mut map = ActionMap::default();
    for (i, name) in ACTIONS.iter().enumerate() {
        map.bind(*name, Binding::Key(KeyCode::Space));
        map.bind(*name, Binding::Mouse(MouseButton::Left));
        map.bind(*name, Binding::GamepadButton(GamepadButton::South));
        if i % 2 == 0 {
            map.bind(*name, Binding::Key(KeyCode::W));
        }
    }
    map
}

fn input(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform");

    let map = action_map();
    group.bench_function("action_map_lookup", |b| {
        b.iter(|| black_box(&map).bindings(black_box("alt_fire")).len());
    });

    // A frame of gameplay code asking about every action once.
    let mut input = MockInput::new();
    input.set_pressed("fire", true);
    input.next_frame();
    group.bench_function("action_query_frame", |b| {
        b.iter(|| {
            let mut held = 0;
            for name in ACTIONS {
                held += usize::from(black_box(&input).pressed(name));
                held += usize::from(input.just_pressed(name));
            }
            held
        });
    });

    group.bench_function("lifecycle_cycle", |b| {
        // Inputs go through `black_box` so the optimizer cannot fold the whole cycle away.
        let surface = SurfaceInfo {
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        };
        b.iter(|| {
            let mut life = LifecycleTracker::new();
            life.apply(black_box(&LifecycleEvent::Resumed));
            life.apply(black_box(&LifecycleEvent::SurfaceReady(surface)));
            life.apply(black_box(&LifecycleEvent::Resized {
                width: 1280,
                height: 720,
            }));
            life.apply(black_box(&LifecycleEvent::SurfaceLost));
            life.apply(black_box(&LifecycleEvent::Suspended));
            black_box(life.can_render())
        });
    });
    group.finish();
}

criterion_group!(benches, input);
criterion_main!(benches);
