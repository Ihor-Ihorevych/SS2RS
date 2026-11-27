//! Headless tests of the play session on the procedural test room.

use std::sync::Arc;

use super::*;
use ss2_world::collision::build_static;
use ss2_world::scene::LevelScene;

const DT: f32 = 1.0 / 60.0;

fn session() -> PlaySession {
    let room = Arc::new(LevelScene::test_room());
    let physics = build_static(&room).unwrap();
    PlaySession::new(room, physics)
}

fn run(s: &mut PlaySession, input: &PlayInput, seconds: f32) {
    for _ in 0..(seconds / DT).round() as usize {
        s.update(input, DT);
    }
}

fn forward() -> PlayInput {
    PlayInput {
        movement: Vec3::new(0.0, 0.0, 1.0),
        ..PlayInput::default()
    }
}

#[test]
fn a_session_starts_walking_at_the_spawn_with_the_eye_above_it() {
    let s = session();
    assert_eq!(s.mode(), Mode::Walk);
    let spawn = LevelScene::test_room().start();
    let cam = s.camera();
    assert!(
        (cam.position - (spawn.position + Vec3::Y * WalkConfig::PROVISIONAL.eye_height)).length()
            < 0.05
    );
    // Yaw 0 looks down -Z.
    assert!(
        (cam.forward() - Vec3::NEG_Z).length() < 1e-3,
        "{}",
        cam.forward()
    );
    assert!(cam.is_valid());
}

#[test]
fn walking_forward_moves_towards_the_steps_and_stays_on_the_floor() {
    let mut s = session();
    run(&mut s, &forward(), 0.4);
    let feet = s.feet();
    // From z = 6: at least 2 m in 0.4 s.
    assert!(feet.z < 4.0, "{feet}");
    assert!(feet.y.abs() < 0.05, "{feet}");
    assert!(s.grounded());
}

#[test]
fn looking_turns_the_walk_direction() {
    let mut s = session();
    // Turn right a quarter turn (look x positive turns right), then walk forward: towards +X.
    let quarter = std::f32::consts::FRAC_PI_2 / FreeFlyConfig::default().sensitivity;
    s.update(
        &PlayInput {
            look: Vec2::new(quarter, 0.0),
            ..PlayInput::default()
        },
        DT,
    );
    run(&mut s, &forward(), 0.3);
    let feet = s.feet();
    assert!(feet.x > 1.0 && (feet.z - 6.0).abs() < 0.3, "{feet}");
}

#[test]
fn the_walls_hold_the_walker_in() {
    let mut s = session();
    run(&mut s, &forward(), 4.0);
    assert!(s.feet().z > -10.0 + 0.1 + 0.3, "{}", s.feet());
}

#[test]
fn toggling_fly_lets_the_camera_leave_the_ground_and_back_lands_the_walker() {
    let mut s = session();
    s.update(
        &PlayInput {
            toggle_fly: true,
            ..PlayInput::default()
        },
        DT,
    );
    assert_eq!(s.mode(), Mode::Fly);
    let up = PlayInput {
        movement: Vec3::new(0.0, 1.0, 0.0),
        ..PlayInput::default()
    };
    run(&mut s, &up, 0.5);
    let high = s.camera().position.y;
    assert!(high > 3.0, "{high}");
    // Back to walking: the walker starts where the camera was and falls to the floor.
    s.update(
        &PlayInput {
            toggle_fly: true,
            ..PlayInput::default()
        },
        DT,
    );
    assert_eq!(s.mode(), Mode::Walk);
    run(&mut s, &PlayInput::default(), 1.5);
    assert!(s.grounded() && s.feet().y.abs() < 0.05, "{}", s.feet());
}

#[test]
fn jumping_leaves_the_ground_and_lands() {
    let mut s = session();
    run(&mut s, &PlayInput::default(), 0.2);
    s.update(
        &PlayInput {
            jump: true,
            ..PlayInput::default()
        },
        DT,
    );
    run(&mut s, &PlayInput::default(), 0.1);
    assert!(!s.grounded() && s.feet().y > 0.3);
    run(&mut s, &PlayInput::default(), 1.5);
    assert!(s.grounded());
}

#[test]
fn falling_out_of_the_level_respawns_at_the_start() {
    let mut s = session();
    // Fly outside the room and below the floor, then walk: there is nothing under the walker.
    s.update(
        &PlayInput {
            toggle_fly: true,
            ..PlayInput::default()
        },
        DT,
    );
    s.teleport(Vec3::new(50.0, -5.0, 50.0));
    s.update(
        &PlayInput {
            toggle_fly: true,
            ..PlayInput::default()
        },
        DT,
    );
    run(&mut s, &PlayInput::default(), 5.0);
    let spawn = LevelScene::test_room().start().position;
    assert!((s.feet() - spawn).length() < 0.1, "{}", s.feet());
    assert!(s.respawns() >= 1);
}

#[test]
fn respawn_puts_the_walker_back_at_the_start() {
    let mut s = session();
    run(&mut s, &forward(), 0.5);
    s.respawn();
    let spawn = LevelScene::test_room().start().position;
    assert_eq!(s.feet(), spawn);
    assert_eq!(s.mode(), Mode::Walk);
}

#[test]
fn bad_time_steps_change_nothing() {
    let mut s = session();
    let before = s.camera();
    for dt in [f32::NAN, -1.0, f32::INFINITY] {
        s.update(&forward(), dt);
    }
    assert_eq!(s.camera(), before);
}
