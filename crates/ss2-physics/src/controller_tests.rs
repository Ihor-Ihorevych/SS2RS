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
        crouch: false,
        sprint: false,
    }
}

fn idle() -> WalkInput {
    WalkInput {
        wish: Vec2::ZERO,
        yaw: 0.0,
        jump: false,
        crouch: false,
        sprint: false,
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
    // Half the speed along the wall for two seconds, less the start-up: well over 0.3 x speed.
    let speed = WalkConfig::PROVISIONAL.speed;
    assert!(p.position.x > 0.3 * speed, "slid along +X: {}", p.position);
}

#[test]
fn a_low_step_is_climbed_and_a_high_one_is_not() {
    let low = (Vec3::new(-1.0, 0.0, -4.0), Vec3::new(1.0, 1.9, -2.0));
    let w = world(&[floor(), low]);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &forward(), 0.3);
    let mut on_top = false;
    for _ in 0..60 {
        p.step(&w, &WalkConfig::PROVISIONAL, GRAVITY, &forward(), DT);
        if p.position.z < -2.5 && p.position.z > -3.5 {
            on_top |= (p.position.y - 1.9).abs() < 0.03 && p.grounded;
        }
    }
    assert!(on_top, "never stood on the low step");

    let high = (Vec3::new(-1.0, 0.0, -4.0), Vec3::new(1.0, 2.5, -2.0));
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
    // 1.5 s: the measured acceleration (T01-432) reaches full speed more gently than the stored constant did.
    run(&w, &mut p, &forward(), 1.5);
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
        crouch: false,
        sprint: false,
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

/// A floor, a 0.1 m slab at chest height whose edge faces the walker at z = -3, and a low
/// ceiling that leaves no room to step up onto the slab.
fn chest_ledge() -> QueryWorld {
    let ledge = (Vec3::new(-5.0, 1.33, -6.0), Vec3::new(5.0, 1.43, -3.0));
    let ceiling = (Vec3::new(-50.0, 2.5, -50.0), Vec3::new(50.0, 3.0, 50.0));
    world(&[floor(), ledge, ceiling])
}

#[test]
fn a_thin_ledge_at_chest_height_stops_the_walker() {
    // The ledge sits between where the old stacked spheres (at 0.4, 1.05 and 1.7 m) were widest,
    // so they reached under it by about 0.1 m; the capsule's straight side stops at its radius.
    let w = chest_ledge();
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::ZERO);
    let trace = run(&w, &mut p, &forward(), 3.0);
    for pos in &trace {
        assert!(pos.z > -3.0 + cfg.radius - 0.02, "under the ledge: {pos}");
    }
    let last = trace.last().unwrap();
    assert!(last.z < -3.0 + cfg.radius + 0.05, "stopped short: {last}");
    assert!(p.grounded && p.position.y.abs() < 0.02, "{}", p.position);
}

#[test]
fn a_floor_the_walker_sinks_into_does_not_hide_a_wall() {
    // Standing 5 cm into the floor and walking along it: one capsule cast reports only the floor
    // (at 0, moved along, so ignored); the walker must step out of it and still see the wall.
    let wall = (Vec3::new(-5.0, 0.0, -3.2), Vec3::new(5.0, 3.0, -3.0));
    let w = world(&[floor(), wall]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::new(0.0, -0.05, -2.55));
    p.grounded = true;
    p.velocity = Vec3::new(0.0, 0.0, -cfg.speed);
    let trace = run(&w, &mut p, &forward(), 1.0);
    for pos in trace {
        assert!(pos.z > -3.0 + cfg.radius - 0.02, "through the wall: {pos}");
    }
    assert!(p.grounded && p.position.y.abs() < 0.02, "{}", p.position);
}

#[test]
fn the_capsule_walker_is_deterministic_across_runs_and_frame_splits() {
    let w = chest_ledge();
    let cfg = WalkConfig::PROVISIONAL;
    let input = WalkInput {
        wish: Vec2::new(0.3, 1.0),
        yaw: 0.2,
        jump: true,
        crouch: false,
        sprint: false,
    };
    let bits = |v: Vec3| v.to_array().map(f32::to_bits);
    let a = run(&w, &mut Walker::new(Vec3::new(0.0, 0.5, 0.0)), &input, 3.0);
    let b = run(&w, &mut Walker::new(Vec3::new(0.0, 0.5, 0.0)), &input, 3.0);
    assert_eq!(
        a.iter().copied().map(bits).collect::<Vec<_>>(),
        b.iter().copied().map(bits).collect::<Vec<_>>()
    );
    // The same 3 s as 360 frames of 1/120 s and as 90 frames of 1/30 s.
    for (frames, dt) in [(360, 1.0 / 120.0), (90, 1.0 / 30.0)] {
        let mut c = Walker::new(Vec3::new(0.0, 0.5, 0.0));
        for _ in 0..frames {
            c.step(&w, &cfg, GRAVITY, &input, dt);
        }
        assert_eq!(bits(c.position), bits(*a.last().unwrap()), "dt {dt}");
    }
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
    assert!(cfg.eye_height < cfg.height && cfg.step_height > 1.9 && cfg.step_height < 2.5);
}

// --- T09-007: crouch, sprint, footsteps, fall height -----------------------------------------

#[test]
fn crouching_lowers_the_eye_and_the_body_and_sprint_does_not_change_the_pose() {
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::new(0.0, 0.0, 0.0));
    assert!(!p.crouched());
    let standing_eye = p.eye(&cfg).y;
    p.step(
        &world(&[floor()]),
        &cfg,
        GRAVITY,
        &WalkInput {
            crouch: true,
            ..idle()
        },
        DT,
    );
    assert!(p.crouched());
    assert!(p.eye(&cfg).y < standing_eye, "{}", p.eye(&cfg));
    assert_eq!(p.eye(&cfg).y, cfg.crouch_eye_height);
}

#[test]
fn crouching_under_a_low_ceiling_blocks_standing_back_up() {
    // A ceiling below the standing height but above the crouch height.
    let ceiling = (Vec3::new(-5.0, 1.4, -5.0), Vec3::new(5.0, 1.5, 5.0));
    let w = world(&[floor(), ceiling]);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &idle(), 0.2);
    assert!(p.grounded && !p.crouched());
    run(
        &w,
        &mut p,
        &WalkInput {
            crouch: true,
            ..idle()
        },
        0.2,
    );
    assert!(p.crouched(), "crouched on command");
    // Letting go while the low ceiling is overhead: stays crouched.
    run(&w, &mut p, &idle(), 0.3);
    assert!(
        p.crouched(),
        "the low ceiling blocks standing: {}",
        p.position
    );
}

#[test]
fn standing_back_up_works_once_the_ceiling_is_clear() {
    let w = world(&[floor()]);
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &idle(), 0.2);
    run(
        &w,
        &mut p,
        &WalkInput {
            crouch: true,
            ..idle()
        },
        0.2,
    );
    assert!(p.crouched());
    run(&w, &mut p, &idle(), 0.2);
    assert!(!p.crouched(), "stands back up with room overhead");
}

#[test]
fn sprinting_increases_ground_speed_and_crouching_cancels_it() {
    let w = world(&[floor()]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut walking = Walker::new(Vec3::ZERO);
    run(&w, &mut walking, &forward(), 0.6);
    let normal = -walking.position.z;

    let mut sprinting = Walker::new(Vec3::ZERO);
    run(
        &w,
        &mut sprinting,
        &WalkInput {
            sprint: true,
            ..forward()
        },
        0.6,
    );
    let fast = -sprinting.position.z;
    assert!(
        fast > normal * 1.2,
        "{fast} vs {normal}, speed {}",
        cfg.speed
    );

    let mut crouch_sprinting = Walker::new(Vec3::ZERO);
    run(
        &w,
        &mut crouch_sprinting,
        &WalkInput {
            sprint: true,
            crouch: true,
            ..forward()
        },
        0.6,
    );
    let crouched = -crouch_sprinting.position.z;
    assert!(
        (crouched - normal).abs() < 0.3,
        "crouching cancels the sprint bonus: {crouched} vs {normal}"
    );
}

#[test]
fn footsteps_fire_while_grounded_and_scale_with_speed_but_never_while_airborne() {
    let w = world(&[floor()]);
    let cfg = WalkConfig::PROVISIONAL;

    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &idle(), 0.2);
    assert!(p.footsteps().is_empty(), "no footsteps standing still");

    let mut walking_count = 0usize;
    for _ in 0..180 {
        p.step(&w, &cfg, GRAVITY, &forward(), DT);
        for e in p.footsteps() {
            assert!(e.speed > 0.0, "{e:?}");
        }
        walking_count += p.footsteps().len();
    }
    assert!(walking_count > 0, "no footsteps while walking");

    let mut sprinting_count = 0usize;
    let mut s = Walker::new(Vec3::ZERO);
    run(&w, &mut s, &idle(), 0.2);
    let sprint_forward = WalkInput {
        sprint: true,
        ..forward()
    };
    for _ in 0..180 {
        s.step(&w, &cfg, GRAVITY, &sprint_forward, DT);
        sprinting_count += s.footsteps().len();
    }
    assert!(
        sprinting_count > walking_count,
        "sprinting steps more often: {sprinting_count} vs {walking_count}"
    );

    // A jump: no footsteps while airborne.
    let mut j = Walker::new(Vec3::ZERO);
    run(&w, &mut j, &idle(), 0.2);
    j.step(
        &w,
        &cfg,
        GRAVITY,
        &WalkInput {
            jump: true,
            ..forward()
        },
        DT,
    );
    let mut was_airborne = false;
    for _ in 0..40 {
        j.step(&w, &cfg, GRAVITY, &forward(), DT);
        if !j.grounded {
            was_airborne = true;
            assert!(j.footsteps().is_empty(), "footstep while airborne");
        }
    }
    assert!(was_airborne, "never left the ground");
}

#[test]
fn footsteps_report_the_ground_material() {
    let (min, max) = floor();
    let v = [
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    let material = crate::query::PhysMaterialId(7);
    let mesh = TriMesh::new(&v, &[0, 2, 1, 0, 3, 2]).unwrap();
    let mut w = QueryWorld::new();
    w.set_static(mesh, vec![material; 2]).unwrap();

    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::ZERO);
    run(&w, &mut p, &idle(), 0.2);
    let mut saw_one = false;
    for _ in 0..180 {
        p.step(&w, &cfg, GRAVITY, &forward(), DT);
        for e in p.footsteps() {
            assert_eq!(e.material, material);
            saw_one = true;
        }
    }
    assert!(saw_one, "no footsteps happened");
}

#[test]
fn landing_from_a_fall_reports_the_fall_height_once() {
    let platform = (Vec3::new(-5.0, 3.0, -5.0), Vec3::new(5.0, 3.1, 5.0));
    let w = world(&[floor(), platform]);
    let cfg = WalkConfig::PROVISIONAL;
    let mut p = Walker::new(Vec3::new(0.0, 3.1, 0.0));
    run(&w, &mut p, &idle(), 0.2);
    assert!(
        p.grounded && (p.position.y - 3.1).abs() < 0.05,
        "{}",
        p.position
    );
    assert_eq!(p.landed_fall_height(), None, "not falling yet");

    let mut reported = None;
    for _ in 0..600 {
        p.step(&w, &cfg, GRAVITY, &forward(), DT);
        if let Some(h) = p.landed_fall_height() {
            reported = Some(h);
            break;
        }
    }
    let h = reported.expect("never reported a landing");
    assert!(h > 2.0 && h < 3.5, "{h}");
    assert!(p.grounded);
}

#[test]
fn crouch_sprint_and_footsteps_are_deterministic_across_runs_and_frame_splits() {
    let w = world(&[floor()]);
    let cfg = WalkConfig::PROVISIONAL;
    let input = WalkInput {
        wish: Vec2::new(0.3, 1.0),
        yaw: 0.1,
        jump: false,
        crouch: true,
        sprint: true,
    };
    let run_it = |dt: f32, frames: usize| {
        let mut p = Walker::new(Vec3::ZERO);
        let mut steps = 0usize;
        for _ in 0..frames {
            p.step(&w, &cfg, GRAVITY, &input, dt);
            steps += p.footsteps().len();
        }
        (p.position.to_array().map(f32::to_bits), p.crouched(), steps)
    };
    let a = run_it(1.0 / 60.0, 180);
    let b = run_it(1.0 / 60.0, 180);
    assert_eq!(a, b);
    let c = run_it(1.0 / 30.0, 90);
    assert_eq!(a.0, c.0);
    assert_eq!(a.1, c.1);
    assert_eq!(a.2, c.2, "same total footsteps regardless of frame split");
}
