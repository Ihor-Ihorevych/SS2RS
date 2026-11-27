//! Tests of the provisional walking controller on small hand-built worlds.

use super::*;
use crate::query::QueryWorld;
use crate::spatial::TriMesh;

const GRAVITY: Vec3 = Vec3::new(0.0, -30.0, 0.0);
const DT: f32 = 1.0 / 60.0;

/// Axis-aligned boxes as one static world.
fn world(boxes: &[(Vec3, Vec3)]) -> QueryWorld {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (min, max) in boxes {
        let base = u32::try_from(vertices.len()).unwrap();
        for i in 0..8u32 {
            vertices.push(Vec3::new(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            ));
        }
        let faces: [[u32; 4]; 6] = [
            [0, 1, 3, 2],
            [4, 6, 7, 5],
            [0, 4, 5, 1],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 5, 7, 3],
        ];
        for [a, b, c, d] in faces {
            indices.extend([a, b, c, a, c, d].map(|i| base + i));
        }
    }
    let mesh = TriMesh::new(&vertices, &indices).unwrap();
    let mut w = QueryWorld::new();
    let n = mesh.triangle_count();
    w.set_static(mesh, vec![crate::query::PhysMaterialId(0); n])
        .unwrap();
    w
}

/// A 100 m floor with its top at y = 0.
fn floor() -> (Vec3, Vec3) {
    (Vec3::new(-50.0, -1.0, -50.0), Vec3::new(50.0, 0.0, 50.0))
}

fn run(w: &QueryWorld, walker: &mut Walker, input: &WalkInput, seconds: f32) -> Vec<Vec3> {
    let cfg = WalkConfig::PROVISIONAL;
    let mut trace = Vec::new();
    for _ in 0..(seconds / DT).round() as usize {
        walker.step(w, &cfg, GRAVITY, input, DT);
        trace.push(walker.position);
    }
    trace
}

fn forward() -> WalkInput {
    WalkInput {
        wish: Vec2::new(0.0, 1.0),
        yaw: 0.0,
        jump: false,
    }
}

fn idle() -> WalkInput {
    WalkInput {
        wish: Vec2::ZERO,
        yaw: 0.0,
        jump: false,
    }
}

#[test]
fn a_walker_on_the_floor_stands_still() {
    let w = world(&[floor()]);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &idle(), 2.0);
    assert!(p.grounded);
    assert!(p.position.y.abs() < 0.02, "{}", p.position);
    assert!(p.position.x.abs() < 1e-4 && p.position.z.abs() < 1e-4);
}

#[test]
fn a_walker_falls_and_lands() {
    let w = world(&[floor()]);
    let mut p = Walker::new(Vec3::new(0.0, 5.0, 0.0));
    run(&w, &mut p, &idle(), 0.2);
    assert!(!p.grounded && p.position.y < 5.0 && p.position.y > 0.5);
    run(&w, &mut p, &idle(), 1.5);
    assert!(p.grounded);
    assert!(p.position.y.abs() < 0.02, "{}", p.position);
    assert_eq!(p.velocity.y, 0.0);
}

#[test]
fn walking_forward_goes_down_minus_z_at_about_the_set_speed_and_yaw_turns_it() {
    let w = world(&[floor()]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &forward(), 1.0);
    let moved = -p.position.z;
    assert!(
        moved > cfg.speed * 0.85 && moved < cfg.speed * 1.01,
        "{moved}"
    );
    assert!(p.position.x.abs() < 1e-3);
    // Yaw +90 degrees (counter-clockwise seen from above) turns "forward" to -X.
    let mut p = Walker::new(Vec3::ZERO);
    let turned = WalkInput {
        yaw: std::f32::consts::FRAC_PI_2,
        ..forward()
    };
    run(&w, &mut p, &turned, 1.0);
    assert!(p.position.x < -cfg.speed * 0.85, "{}", p.position);
    // Letting go stops the walker within a short distance.
    let before = p.position;
    run(&w, &mut p, &idle(), 1.0);
    assert!(p.velocity.length() < 1e-3);
    assert!((p.position - before).length() < 2.0);
}

#[test]
fn a_wall_stops_the_walker() {
    let wall = (Vec3::new(-5.0, 0.0, -3.2), Vec3::new(5.0, 3.0, -3.0));
    let w = world(&[floor(), wall]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::ZERO);
    let trace = run(&w, &mut p, &forward(), 3.0);
    for pos in trace {
        assert!(pos.z > -3.0 + cfg.radius - 0.02, "{pos}");
    }
    assert!(p.grounded);
}

#[test]
fn a_walker_slides_along_a_wall_it_meets_at_an_angle() {
    let wall = (Vec3::new(-5.0, 0.0, -3.2), Vec3::new(5.0, 3.0, -3.0));
    let w = world(&[floor(), wall]);
    let mut p = Walker::new(Vec3::ZERO);
    let diagonal = WalkInput {
        wish: Vec2::new(1.0, 1.0).normalize(),
        ..forward()
    };
    run(&w, &mut p, &diagonal, 2.0);
    assert!(p.position.x > 3.0, "slid along +X: {}", p.position);
}

#[test]
fn a_low_step_is_climbed_and_a_high_one_is_not() {
    let low = (Vec3::new(-1.0, 0.0, -4.0), Vec3::new(1.0, 0.4, -2.0));
    let w = world(&[floor(), low]);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &forward(), 0.3);
    let mut on_top = false;
    for _ in 0..60 {
        p.step(&w, &WalkConfig::PROVISIONAL, GRAVITY, &forward(), DT);
        if p.position.z < -2.5 && p.position.z > -3.5 {
            on_top |= (p.position.y - 0.4).abs() < 0.03 && p.grounded;
        }
    }
    assert!(on_top, "never stood on the low step");

    let high = (Vec3::new(-1.0, 0.0, -4.0), Vec3::new(1.0, 1.0, -2.0));
    let w = world(&[floor(), high]);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &forward(), 2.0);
    assert!(p.position.z > -2.0 + 0.3, "{}", p.position);
    assert!(p.position.y < 0.05);
}

#[test]
fn a_jump_reaches_the_height_the_numbers_give() {
    let w = world(&[floor()]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &idle(), 0.2);
    let jump = WalkInput {
        jump: true,
        ..idle()
    };
    p.step(&w, &cfg, GRAVITY, &jump, DT);
    let trace = run(&w, &mut p, &idle(), 1.5);
    let apex = trace.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    let expected = cfg.jump_speed * cfg.jump_speed / (2.0 * 30.0);
    assert!(
        apex > expected * 0.9 && apex < expected * 1.05,
        "{apex} vs {expected}"
    );
    assert!(p.grounded && p.position.y.abs() < 0.02);
}

#[test]
fn holding_jump_in_the_air_does_not_jump_again() {
    let w = world(&[floor()]);
    let mut p = Walker::new(Vec3::new(0.0, 3.0, 0.0));
    let jump = WalkInput {
        jump: true,
        ..idle()
    };
    p.step(&w, &WalkConfig::PROVISIONAL, GRAVITY, &jump, DT);
    assert!(p.velocity.y <= 0.0, "jumped in the air");
}

#[test]
fn a_gentle_ramp_is_walked_up_and_a_steep_one_is_slid_down() {
    let ramp = |angle: f32| -> QueryWorld {
        // A plane rising towards -Z from z = -1, as two triangles.
        let (s, c) = (angle.to_radians().sin(), angle.to_radians().cos());
        let len = 10.0;
        let v = [
            Vec3::new(-3.0, 0.0, -1.0),
            Vec3::new(3.0, 0.0, -1.0),
            Vec3::new(3.0, s * len, -1.0 - c * len),
            Vec3::new(-3.0, s * len, -1.0 - c * len),
        ];
        let (fmin, fmax) = floor();
        let mut verts = v.to_vec();
        verts.extend([
            Vec3::new(fmin.x, 0.0, fmin.z),
            Vec3::new(fmax.x, 0.0, fmin.z),
            Vec3::new(fmax.x, 0.0, fmax.z),
            Vec3::new(fmin.x, 0.0, fmax.z),
        ]);
        let idx = [0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6];
        let mesh = TriMesh::new(&verts, &idx).unwrap();
        let mut w = QueryWorld::new();
        let n = mesh.triangle_count();
        w.set_static(mesh, vec![crate::query::PhysMaterialId(0); n])
            .unwrap();
        w
    };
    let w = ramp(20.0);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &forward(), 1.0);
    assert!(p.position.y > 1.0, "climbed: {}", p.position);
    assert!(p.grounded);

    let w = ramp(60.0);
    let mut p = Walker::new(Vec3::new(0.0, 5.2, -4.0));
    run(&w, &mut p, &idle(), 1.0);
    assert!(p.position.y < 4.0, "slid down: {}", p.position);
}

#[test]
fn a_fast_walker_does_not_pass_through_a_thin_wall() {
    let thin = (Vec3::new(-5.0, -2.0, -3.05), Vec3::new(5.0, 5.0, -2.95));
    let w = world(&[thin]);
    let mut p = Walker::new(Vec3::new(0.0, 0.0, 0.0));
    p.velocity = Vec3::new(0.0, 0.0, -60.0);
    let trace = run(&w, &mut p, &idle(), 0.5);
    for pos in trace {
        assert!(pos.z > -3.05, "tunnelled: {pos}");
    }
}

#[test]
fn the_same_inputs_give_the_same_trace_and_frame_splits_do_not_matter() {
    let wall = (Vec3::new(-5.0, 0.0, -3.2), Vec3::new(5.0, 3.0, -3.0));
    let w = world(&[floor(), wall]);
    let cfg = WalkConfig::PROVISIONAL;
    let diagonal = WalkInput {
        wish: Vec2::new(0.6, 0.8),
        yaw: 0.3,
        jump: true,
    };
    let mut a = Walker::new(Vec3::new(0.0, 1.0, 0.0));
    let mut b = Walker::new(Vec3::new(0.0, 1.0, 0.0));
    let ta = run(&w, &mut a, &diagonal, 2.0);
    let tb = run(&w, &mut b, &diagonal, 2.0);
    assert_eq!(
        ta.iter()
            .map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        tb.iter()
            .map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    // 120 frames of 1/60 s and 60 frames of 1/30 s run the same fixed substeps.
    let mut c = Walker::new(Vec3::new(0.0, 1.0, 0.0));
    for _ in 0..60 {
        c.step(&w, &cfg, GRAVITY, &diagonal, 1.0 / 30.0);
    }
    assert_eq!(
        c.position.to_array().map(f32::to_bits),
        a.position.to_array().map(f32::to_bits)
    );
}

#[test]
fn bad_time_steps_do_nothing_and_a_long_frame_is_clamped() {
    let w = world(&[floor()]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::new(0.0, 5.0, 0.0));
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        p.step(&w, &cfg, GRAVITY, &idle(), dt);
        assert_eq!(p.position, Vec3::new(0.0, 5.0, 0.0), "dt {dt}");
    }
    p.step(&w, &cfg, GRAVITY, &idle(), 10.0);
    assert!(
        p.position.y > 3.0,
        "a 10 s frame counts as {MAX_FRAME} s: {}",
        p.position
    );
}

#[test]
fn the_eye_is_above_the_feet() {
    let cfg = WalkConfig::PROVISIONAL;
    let p = Walker::new(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(p.eye(&cfg), Vec3::new(1.0, 2.0 + cfg.eye_height, 3.0));
    assert!(cfg.eye_height < cfg.height && cfg.step_height > 0.4 && cfg.step_height < 1.0);
}
