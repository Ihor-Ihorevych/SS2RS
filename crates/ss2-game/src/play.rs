//! The play session of the playable MVP (T07-031): one loaded level, its collision world, a walker and a free-fly
//! camera, driven by one input frame per rendered frame. The app turns devices into [`PlayInput`] and renders from
//! [`PlaySession::camera`]; nothing here knows about windows or the GPU.
//!
//! - **Walk** (the default): the look turns the camera, the walker ([`ss2_physics::controller`], provisional
//!   numbers) moves along the camera's heading, and the camera sits at the walker's eyes.
//!   [`PlayInput::crouch`] and [`PlayInput::sprint`] (T09-007) pass straight through to the walker's own
//!   [`WalkInput`]; the walker's crouch/footstep/fall-height state is read back through
//!   [`PlaySession::footsteps`] and [`PlaySession::landed_fall_height`]/[`PlaySession::last_fall_damage`].
//! - **Fly**: the [`FreeFlyCamera`] moves freely through walls. Leaving fly mode drops the walker where the camera
//!   is. Crouch and sprint have no effect while flying (`boost` already covers "faster flying").
//!
//! A walker that falls far below the level is put back at the start.

use std::sync::Arc;

use ss2_core::math::{Quat, Vec2, Vec3};
use ss2_physics::controller::{WalkConfig, WalkInput, Walker};
use ss2_physics::query::QueryWorld;
use ss2_render_api::{Camera, Projection};
use ss2_world::scene::LevelScene;

use crate::freefly::{FreeFlyCamera, FreeFlyConfig, FreeFlyInput};
use crate::player_view::{PlayerView, PlayerViewInput, main_view_fov_y};

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
    /// Crouch (walk mode; T09-007). Held, the walker shrinks; released, it stands back up only
    /// once there is room overhead ([`ss2_physics::controller::WalkInput::crouch`]). No effect
    /// while flying.
    pub crouch: bool,
    /// Sprint (walk mode; T09-007, ×1.5 ground speed, cancelled while crouched). No effect while
    /// flying; `boost` is the fly-mode equivalent.
    pub sprint: bool,
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
    /// The camera's sway/damping and the weapon's walk-bob state (T09-008), updated from the
    /// walker's own movement each tick.
    view: PlayerView,
    /// This frame's fall-damage hook result (T09-007; see [`PlaySession::last_fall_damage`]).
    last_fall_damage: Option<u32>,
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
            view: PlayerView::new(),
            last_fall_damage: None,
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
                self.last_fall_damage = None;
                // No walking, so no walk bob or lean (T09-008); the view-damping spring just
                // relaxes towards rest (no vertical-velocity kicks while flying).
                self.view.update(
                    &PlayerViewInput {
                        wish: Vec2::ZERO,
                        sprinting: false,
                        vertical_velocity: 0.0,
                    },
                    dt,
                );
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
                    crouch: input.crouch,
                    sprint: input.sprint,
                };
                let gravity = self.scene.gravity;
                self.walker
                    .step(&self.physics, &self.walk, gravity, &walk, dt);
                // The fall-damage hook (T09-007): `ss2_game::fall_damage` always returns `None`
                // today (no formula was found, `docs/re/puppet-damage.md`, T01-436 open), but the
                // measurement-to-hook wiring is real.
                self.last_fall_damage = self
                    .walker
                    .landed_fall_height()
                    .and_then(crate::fall_damage::fall_damage);
                if self.walker.position.y < self.floor_limit || !self.walker.position.is_finite() {
                    self.respawns += 1;
                    self.respawn();
                }
                self.camera.set_position(self.walker.eye(&self.walk));
                // The weapon's walk bob/lean and the view's landing-dip damping (T09-008), driven
                // by this tick's wish vector and the walker's own vertical velocity. Sprinting
                // (T09-007) feeds the same sprint factor the walker itself used, cancelled while
                // crouched, so the cosmetic bob matches the real ground-speed bonus.
                self.view.update(
                    &PlayerViewInput {
                        wish: walk.wish,
                        sprinting: walk.sprint && !self.walker.crouched(),
                        vertical_velocity: self.walker.velocity.y,
                    },
                    dt,
                );
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
        self.view = PlayerView::new();
        self.last_fall_damage = None;
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

    /// The camera to render from, with no field-of-view override and none of this frame's
    /// sway/bob/damping (T09-008's [`PlaySession::rendered_camera`] applies those).
    pub fn camera(&self) -> Camera {
        self.camera.camera()
    }

    /// The camera to actually render from: [`PlaySession::camera`], with the main view's field of
    /// view at `aspect` (width / height, `docs/re/weapon-view.md` section summary point 5,
    /// [`main_view_fov_y`]) and this frame's view-damping offset and strafe-lean roll
    /// ([`PlayerView::camera_damping_offset`], [`PlayerView::camera_roll`]).
    pub fn rendered_camera(&self, aspect: f32) -> Camera {
        let mut camera = self.camera();
        if let Projection::Perspective { near, .. } = camera.projection {
            camera.projection = Projection::Perspective {
                fov_y: main_view_fov_y(aspect),
                near,
            };
        }
        camera.position += camera.up() * self.view.camera_damping_offset();
        camera.rotation *= Quat::from_rotation_z(-self.view.camera_roll());
        camera
    }

    /// This frame's camera sway/damping and weapon walk-bob state (T09-008), for the app to apply
    /// to the viewmodel's own placement.
    pub fn view(&self) -> PlayerView {
        self.view
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

    /// This frame's footstep events (T09-007), for audio later — no sound is played here; empty
    /// while flying (`ss2_physics::controller::Walker::footsteps`, only produced while walking).
    pub fn footsteps(&self) -> &[ss2_physics::controller::FootstepEvent] {
        match self.mode {
            Mode::Walk => self.walker.footsteps(),
            Mode::Fly => &[],
        }
    }

    /// How far the walker fell before landing this frame, if it landed this frame (T09-007);
    /// `None` on every other frame, and while flying.
    pub fn landed_fall_height(&self) -> Option<f32> {
        match self.mode {
            Mode::Walk => self.walker.landed_fall_height(),
            Mode::Fly => None,
        }
    }

    /// This frame's fall-damage hook result (T09-007, [`crate::fall_damage::fall_damage`]).
    /// **Always `None` today**: see that module — no fall-damage formula was found in the
    /// decompile (`docs/re/puppet-damage.md`, T01-436 open), so this is a named hook, not a
    /// working formula.
    pub fn last_fall_damage(&self) -> Option<u32> {
        self.last_fall_damage
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
