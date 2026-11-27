//! The play session of the playable MVP (T07-031): one loaded level, its collision world, a walker and a free-fly
//! camera, driven by one input frame per rendered frame. The app turns devices into [`PlayInput`] and renders from
//! [`PlaySession::camera`]; nothing here knows about windows or the GPU.
//!
//! - **Walk** (the default): the look turns the camera, the walker ([`ss2_physics::controller`], provisional
//!   numbers) moves along the camera's heading, and the camera sits at the walker's eyes.
//! - **Fly**: the [`FreeFlyCamera`] moves freely through walls. Leaving fly mode drops the walker where the camera
//!   is.
//!
//! A walker that falls far below the level is put back at the start.

use std::sync::Arc;

use ss2_core::math::{Vec2, Vec3};
use ss2_physics::controller::{WalkConfig, WalkInput, Walker};
use ss2_physics::query::QueryWorld;
use ss2_render_api::Camera;
use ss2_world::scene::LevelScene;

use crate::freefly::{FreeFlyCamera, FreeFlyConfig, FreeFlyInput};

/// How far below the level's lowest point a walker may fall before it is put back at the start, metres.
pub const FALL_LIMIT: f32 = 100.0;

/// What the player does this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayInput {
    /// `x` right, `y` up (fly mode only), `z` forward; each from -1 to 1.
    pub movement: Vec3,
    /// Look change in input units (mouse pixels): `x` positive turns right, `y` positive looks down.
    pub look: Vec2,
    /// Jump (walk mode); held is fine, a jump starts only on the ground.
    pub jump: bool,
    /// Faster flying.
    pub boost: bool,
    /// Switch between walking and flying (pressed this frame).
    pub toggle_fly: bool,
}

/// Walking or flying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// On foot, with collision.
    Walk,
    /// Free camera, no collision.
    Fly,
}

/// One level being played.
pub struct PlaySession {
    scene: Arc<LevelScene>,
    physics: QueryWorld,
    walk: WalkConfig,
    walker: Walker,
    camera: FreeFlyCamera,
    mode: Mode,
    floor_limit: f32,
    respawns: u32,
}

impl PlaySession {
    /// Start `scene` walking at its start point; `physics` is its collision world
    /// (`ss2_world::collision::build_static`).
    pub fn new(scene: Arc<LevelScene>, physics: QueryWorld) -> Self {
        let walk = WalkConfig::PROVISIONAL;
        let floor_limit = scene.bounds().map_or(-FALL_LIMIT, |b| b.min.y - FALL_LIMIT);
        let start = scene.start().position;
        let mut session = Self {
            scene,
            physics,
            walk,
            walker: Walker::new(start),
            camera: FreeFlyCamera::new(start, FreeFlyConfig::default()),
            mode: Mode::Walk,
            floor_limit,
            respawns: 0,
        };
        session.respawn();
        session
    }

    /// Advance by `dt` seconds.
    pub fn update(&mut self, input: &PlayInput, dt: f32) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }
        if input.toggle_fly {
            self.mode = match self.mode {
                Mode::Walk => Mode::Fly,
                Mode::Fly => {
                    // Drop the walker where the eyes were.
                    self.walker =
                        Walker::new(self.camera.position() - Vec3::Y * self.walk.eye_height);
                    Mode::Walk
                }
            };
        }
        match self.mode {
            Mode::Fly => {
                let fly = FreeFlyInput {
                    movement: input.movement,
                    look: input.look,
                    boost: input.boost,
                    slow: false,
                };
                self.camera.update(dt, &fly);
            }
            Mode::Walk => {
                // The look only (a step of 0 moves nothing), then walk along the new heading.
                let look = FreeFlyInput {
                    look: input.look,
                    ..FreeFlyInput::default()
                };
                self.camera.update(0.0, &look);
                let walk = WalkInput {
                    wish: Vec2::new(input.movement.x, input.movement.z),
                    yaw: self.camera.yaw(),
                    jump: input.jump,
                };
                let gravity = self.scene.gravity;
                self.walker
                    .step(&self.physics, &self.walk, gravity, &walk, dt);
                if self.walker.position.y < self.floor_limit || !self.walker.position.is_finite() {
                    self.respawns += 1;
                    self.respawn();
                }
                self.camera.set_position(self.walker.eye(&self.walk));
            }
        }
    }

    /// Put the walker back at the level's start, walking, facing the start's heading.
    pub fn respawn(&mut self) {
        let start = self.scene.start();
        self.mode = Mode::Walk;
        self.walker = Walker::new(start.position);
        let eye = self.walker.eye(&self.walk);
        self.camera.set_position(eye);
        let (sin, cos) = start.yaw.sin_cos();
        self.camera.look_at(eye + Vec3::new(-sin, 0.0, -cos));
    }

    /// Move the walker's feet (or, flying, the camera's eyes) to `feet` at once.
    pub fn teleport(&mut self, feet: Vec3) {
        if !feet.is_finite() {
            return;
        }
        self.walker = Walker::new(feet);
        self.camera
            .set_position(feet + Vec3::Y * self.walk.eye_height);
    }

    /// The camera to render from.
    pub fn camera(&self) -> Camera {
        self.camera.camera()
    }

    /// Walking or flying.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The walker's feet.
    pub fn feet(&self) -> Vec3 {
        self.walker.position
    }

    /// Whether the walker stands on ground.
    pub fn grounded(&self) -> bool {
        self.walker.grounded
    }

    /// How many times the walker was put back at the start after falling out.
    pub fn respawns(&self) -> u32 {
        self.respawns
    }

    /// The level being played.
    pub fn scene(&self) -> &Arc<LevelScene> {
        &self.scene
    }
}

#[cfg(test)]
#[path = "play_tests.rs"]
mod tests;
