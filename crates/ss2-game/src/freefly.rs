//! The free-fly debug camera (roadmap T07-020): fly anywhere through a level, through walls, before
//! the character controller exists, and later as the camera of the `noclip` cheat and the spectator.
//!
//! It is pure logic: the app turns keys, mouse and gamepad into a [`FreeFlyInput`] each frame and
//! asks for the [`Camera`] to render from. Nothing here touches a window or a clock (time is the
//! `dt` argument), so it is tested without a GPU.
//!
//! Conventions (`ss2_core::math`): right-handed, Y up, -Z forward. A **yaw** of 0 looks down -Z and
//! a positive yaw turns left; a positive **pitch** looks up. There is no roll.

use ss2_core::math::{Quat, Vec2, Vec3};
use ss2_render_api::{Camera, Projection};

/// How the camera feels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreeFlyConfig {
    /// Cruising speed in units per second.
    pub speed: f32,
    /// What the boost key multiplies the speed by.
    pub boost: f32,
    /// What the slow key multiplies the speed by.
    pub slow: f32,
    /// Radians of turn per unit of look input (a pixel of mouse movement).
    pub sensitivity: f32,
    /// The steepest look up or down, in radians; below a quarter turn so the view never flips.
    pub max_pitch: f32,
    /// Seconds to reach about two thirds of the target speed; 0 follows the input at once.
    pub accel_time: f32,
}

impl FreeFlyConfig {
    /// The slowest cruising speed the scroll wheel can set.
    pub const MIN_SPEED: f32 = 0.1;
    /// The fastest cruising speed the scroll wheel can set.
    pub const MAX_SPEED: f32 = 1000.0;
    /// The longest step simulated at once; a breakpoint or a stall does not fling the camera.
    pub const MAX_STEP: f32 = 0.25;
}

impl Default for FreeFlyConfig {
    fn default() -> Self {
        Self {
            speed: 8.0,
            boost: 4.0,
            slow: 0.25,
            sensitivity: 0.0025,
            max_pitch: 89.0_f32.to_radians(),
            accel_time: 0.08,
        }
    }
}

/// What the player asked for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FreeFlyInput {
    /// `x` right, `y` up (world up), `z` forward along the view; each from -1 to 1 (a stick can give less).
    pub movement: Vec3,
    /// Look change this frame in input units (mouse pixels): `x` positive turns right, `y` positive looks down.
    pub look: Vec2,
    /// The boost key is down.
    pub boost: bool,
    /// The slow key is down.
    pub slow: bool,
}

/// The camera's state; see the module documentation.
///
/// ```
/// use ss2_core::math::{Vec2, Vec3};
/// use ss2_game::freefly::{FreeFlyCamera, FreeFlyConfig, FreeFlyInput};
///
/// let mut cam = FreeFlyCamera::new(Vec3::ZERO, FreeFlyConfig { accel_time: 0.0, ..FreeFlyConfig::default() });
/// let forward = FreeFlyInput { movement: Vec3::Z, ..FreeFlyInput::default() };
/// cam.update(0.1, &forward); // 0.1 s at the default 8 units per second, straight ahead (-Z)
/// assert!((cam.position().z + 0.8).abs() < 1e-5);
/// assert!(cam.camera().is_valid());
/// ```
#[derive(Clone, Debug)]
pub struct FreeFlyCamera {
    position: Vec3,
    velocity: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    config: FreeFlyConfig,
    projection: Projection,
}

impl FreeFlyCamera {
    /// A camera at `position` looking down -Z.
    pub fn new(position: Vec3, config: FreeFlyConfig) -> Self {
        Self {
            position: if position.is_finite() {
                position
            } else {
                Vec3::ZERO
            },
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            speed: config
                .speed
                .clamp(FreeFlyConfig::MIN_SPEED, FreeFlyConfig::MAX_SPEED),
            config,
            projection: Camera::default().projection,
        }
    }

    /// Where the camera is.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// How fast the camera is moving.
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// The turn about the up axis in radians, in `[-π, π]`.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// The look up (positive) or down in radians.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// The current cruising speed (the scroll wheel changes it).
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Use another projection (field of view, near plane).
    pub fn set_projection(&mut self, projection: Projection) {
        self.projection = projection;
    }

    /// Put the camera somewhere and stop it; a non-finite position is refused.
    pub fn set_position(&mut self, position: Vec3) {
        if position.is_finite() {
            self.position = position;
            self.velocity = Vec3::ZERO;
        }
    }

    /// Turn to face `target`, within the pitch limit; a target at the camera or a non-finite one changes nothing.
    pub fn look_at(&mut self, target: Vec3) {
        let d = target - self.position;
        let length = d.length();
        if !d.is_finite() || length < 1e-6 {
            return;
        }
        if d.x * d.x + d.z * d.z > 1e-12 {
            self.yaw = (-d.x).atan2(-d.z);
        }
        self.pitch = (d.y / length)
            .clamp(-1.0, 1.0)
            .asin()
            .clamp(-self.config.max_pitch, self.config.max_pitch);
    }

    /// Change the cruising speed by a factor of 1.25 per wheel notch (negative slows down), within limits.
    pub fn scroll_speed(&mut self, notches: f32) {
        if notches.is_finite() {
            self.speed = (self.speed * 1.25_f32.powf(notches))
                .clamp(FreeFlyConfig::MIN_SPEED, FreeFlyConfig::MAX_SPEED);
        }
    }

    /// The camera to render from.
    pub fn camera(&self) -> Camera {
        Camera {
            position: self.position,
            rotation: Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch),
            projection: self.projection,
        }
    }

    /// Advance by `dt` seconds with this frame's input. A `dt` that is negative or not a number
    /// changes nothing; a `dt` of 0 applies only the look; one over [`FreeFlyConfig::MAX_STEP`] counts as that long.
    pub fn update(&mut self, dt: f32, input: &FreeFlyInput) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }
        let dt = dt.min(FreeFlyConfig::MAX_STEP);

        if input.look.is_finite() {
            let yaw = self.yaw - input.look.x * self.config.sensitivity;
            self.yaw = yaw
                - std::f32::consts::TAU
                    * ((yaw + std::f32::consts::PI) / std::f32::consts::TAU).floor();
            self.pitch = (self.pitch - input.look.y * self.config.sensitivity)
                .clamp(-self.config.max_pitch, self.config.max_pitch);
        }

        let mut push = if input.movement.is_finite() {
            input.movement
        } else {
            Vec3::ZERO
        };
        if push.length_squared() > 1.0 {
            push = push.normalize();
        }
        let camera = self.camera();
        let mut speed = self.speed;
        if input.boost {
            speed *= self.config.boost;
        }
        if input.slow {
            speed *= self.config.slow;
        }
        let target =
            (camera.right() * push.x + Vec3::Y * push.y + camera.forward() * push.z) * speed;
        if self.config.accel_time > 0.0 {
            let follow = 1.0 - (-dt / self.config.accel_time).exp();
            self.velocity += (target - self.velocity) * follow;
        } else {
            self.velocity = target;
        }
        self.position += self.velocity * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::math::{Vec2, Vec3};

    fn cam() -> FreeFlyCamera {
        FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        )
    }

    fn input(movement: Vec3, look: Vec2) -> FreeFlyInput {
        FreeFlyInput {
            movement,
            look,
            boost: false,
            slow: false,
        }
    }

    /// Fly for `seconds` in steps within the one-step limit.
    fn fly(c: &mut FreeFlyCamera, seconds: f32, input: &FreeFlyInput) {
        let steps = (seconds / 0.05).round() as u32;
        for _ in 0..steps {
            c.update(seconds / steps as f32, input);
        }
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn it_starts_looking_down_minus_z() {
        let c = cam();
        assert!(close(c.camera().forward(), Vec3::NEG_Z));
        assert!(close(c.camera().up(), Vec3::Y));
        assert!(c.camera().is_valid());
    }

    #[test]
    fn mouse_right_turns_right_and_mouse_up_looks_up() {
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                sensitivity: 0.01,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        c.update(0.016, &input(Vec3::ZERO, Vec2::new(100.0, 0.0)));
        assert!((c.yaw() + 1.0).abs() < 1e-5, "{}", c.yaw());
        assert!(
            c.camera().forward().x > 0.5,
            "turned to the right: {:?}",
            c.camera().forward()
        );
        let mut c = cam();
        c.update(0.016, &input(Vec3::ZERO, Vec2::new(0.0, -100.0)));
        assert!(
            c.pitch() > 0.0 && c.camera().forward().y > 0.0,
            "mouse up looks up"
        );
    }

    #[test]
    fn pitch_stops_short_of_straight_up_and_down() {
        let mut c = cam();
        c.update(0.016, &input(Vec3::ZERO, Vec2::new(0.0, -1.0e6)));
        assert!(
            c.pitch() < std::f32::consts::FRAC_PI_2 && c.pitch() > 1.5,
            "{}",
            c.pitch()
        );
        c.update(0.016, &input(Vec3::ZERO, Vec2::new(0.0, 1.0e6)));
        assert!(c.pitch() > -std::f32::consts::FRAC_PI_2 && c.pitch() < -1.5);
        assert!(c.camera().is_valid());
    }

    #[test]
    fn yaw_stays_within_one_turn_so_precision_does_not_drift() {
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                sensitivity: 1.0,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        for _ in 0..10_000 {
            c.update(0.016, &input(Vec3::ZERO, Vec2::new(3.0, 0.0)));
        }
        assert!(c.yaw().abs() <= std::f32::consts::PI + 1e-3, "{}", c.yaw());
    }

    #[test]
    fn movement_follows_the_view_strafes_sideways_and_rises_along_world_up() {
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                speed: 10.0,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        fly(&mut c, 1.0, &input(Vec3::new(0.0, 0.0, 1.0), Vec2::ZERO));
        assert!(
            close(c.position(), Vec3::new(0.0, 0.0, -10.0)),
            "forward: {:?}",
            c.position()
        );
        fly(&mut c, 1.0, &input(Vec3::new(1.0, 0.0, 0.0), Vec2::ZERO));
        assert!(
            close(c.position(), Vec3::new(10.0, 0.0, -10.0)),
            "right: {:?}",
            c.position()
        );
        fly(&mut c, 1.0, &input(Vec3::new(0.0, 1.0, 0.0), Vec2::ZERO));
        assert!(
            close(c.position(), Vec3::new(10.0, 10.0, -10.0)),
            "up: {:?}",
            c.position()
        );
        // Looking up, forward goes up and sideways stays level.
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                speed: 10.0,
                sensitivity: 1.0,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        c.update(
            0.0,
            &input(Vec3::ZERO, Vec2::new(0.0, -std::f32::consts::FRAC_PI_4)),
        );
        fly(&mut c, 1.0, &input(Vec3::new(0.0, 0.0, 1.0), Vec2::ZERO));
        assert!(
            c.position().y > 6.0 && c.position().z < -6.0,
            "{:?}",
            c.position()
        );
        let before = c.position().y;
        fly(&mut c, 1.0, &input(Vec3::new(1.0, 0.0, 0.0), Vec2::ZERO));
        assert!((c.position().y - before).abs() < 1e-3, "strafing is level");
        // Rising is straight up in the world even when looking up (not along the view's up).
        let (x, z, y0) = (c.position().x, c.position().z, c.position().y);
        fly(&mut c, 1.0, &input(Vec3::Y, Vec2::ZERO));
        assert!(
            (c.position().x - x).abs() < 1e-3 && (c.position().z - z).abs() < 1e-3,
            "{:?}",
            c.position()
        );
        assert!((c.position().y - y0 - 10.0).abs() < 1e-2);
    }

    #[test]
    fn a_diagonal_is_no_faster_than_straight_ahead() {
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                speed: 10.0,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        fly(&mut c, 1.0, &input(Vec3::new(1.0, 1.0, 1.0), Vec2::ZERO));
        assert!(
            (c.position().length() - 10.0).abs() < 1e-3,
            "{}",
            c.position().length()
        );
        // A smaller push is slower, not rescaled up to full speed (a gamepad stick).
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                speed: 10.0,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        fly(&mut c, 1.0, &input(Vec3::new(0.0, 0.0, 0.5), Vec2::ZERO));
        assert!((c.position().length() - 5.0).abs() < 1e-3);
    }

    #[test]
    fn boost_and_slow_scale_the_speed() {
        let run = |boost: bool, slow: bool| {
            let mut c = FreeFlyCamera::new(
                Vec3::ZERO,
                FreeFlyConfig {
                    speed: 10.0,
                    boost: 4.0,
                    slow: 0.25,
                    accel_time: 0.0,
                    ..FreeFlyConfig::default()
                },
            );
            fly(
                &mut c,
                1.0,
                &FreeFlyInput {
                    movement: Vec3::Z,
                    look: Vec2::ZERO,
                    boost,
                    slow,
                },
            );
            c.position().length()
        };
        assert!((run(false, false) - 10.0).abs() < 1e-3);
        assert!((run(true, false) - 40.0).abs() < 1e-3);
        assert!((run(false, true) - 2.5).abs() < 1e-3);
        assert!((run(true, true) - 10.0).abs() < 1e-3, "both cancel");
    }

    #[test]
    fn the_same_time_in_many_small_steps_goes_the_same_distance() {
        let cfg = FreeFlyConfig {
            speed: 10.0,
            accel_time: 0.0,
            ..FreeFlyConfig::default()
        };
        let mut one = FreeFlyCamera::new(Vec3::ZERO, cfg);
        fly(&mut one, 1.0, &input(Vec3::Z, Vec2::ZERO));
        let mut many = FreeFlyCamera::new(Vec3::ZERO, cfg);
        for _ in 0..100 {
            many.update(0.01, &input(Vec3::Z, Vec2::ZERO));
        }
        assert!(
            close(one.position(), many.position()),
            "{:?} vs {:?}",
            one.position(),
            many.position()
        );
    }

    #[test]
    fn smoothing_speeds_up_and_slows_down_gradually() {
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                speed: 10.0,
                accel_time: 0.2,
                ..FreeFlyConfig::default()
            },
        );
        c.update(0.05, &input(Vec3::Z, Vec2::ZERO));
        let early = c.velocity().length();
        assert!(early > 0.0 && early < 5.0, "still speeding up: {early}");
        for _ in 0..200 {
            c.update(0.05, &input(Vec3::Z, Vec2::ZERO));
        }
        assert!(
            (c.velocity().length() - 10.0).abs() < 0.01,
            "reaches the speed: {}",
            c.velocity().length()
        );
        c.update(0.05, &input(Vec3::ZERO, Vec2::ZERO));
        assert!(c.velocity().length() > 1.0, "does not stop at once");
        for _ in 0..200 {
            c.update(0.05, &input(Vec3::ZERO, Vec2::ZERO));
        }
        assert!(c.velocity().length() < 0.01, "and comes to rest");
    }

    #[test]
    fn bad_input_and_bad_time_change_nothing_and_never_make_nan() {
        let mut c = cam();
        c.update(
            0.016,
            &input(Vec3::new(f32::NAN, 1.0, 0.0), Vec2::new(f32::INFINITY, 0.0)),
        );
        assert!(c.position().is_finite() && c.yaw().is_finite() && c.pitch().is_finite());
        assert_eq!(c.position(), Vec3::ZERO);
        let before = (c.position(), c.yaw());
        c.update(f32::NAN, &input(Vec3::Z, Vec2::new(5.0, 5.0)));
        c.update(-1.0, &input(Vec3::Z, Vec2::new(5.0, 5.0)));
        assert_eq!(
            (c.position(), c.yaw()),
            before,
            "a bad or negative step is ignored"
        );
        // A huge step (a breakpoint) is limited rather than flinging the camera away.
        let mut c = FreeFlyCamera::new(
            Vec3::ZERO,
            FreeFlyConfig {
                speed: 10.0,
                accel_time: 0.0,
                ..FreeFlyConfig::default()
            },
        );
        c.update(1.0e6, &input(Vec3::Z, Vec2::ZERO));
        assert!(
            c.position().length() <= 10.0 * 0.25 + 1e-3,
            "{}",
            c.position().length()
        );
    }

    #[test]
    fn the_scroll_wheel_changes_the_speed_by_a_factor_within_limits() {
        let mut c = cam();
        let base = c.speed();
        c.scroll_speed(1.0);
        assert!((c.speed() / base - 1.25).abs() < 1e-4);
        c.scroll_speed(-1.0);
        assert!((c.speed() - base).abs() < 1e-3);
        for _ in 0..200 {
            c.scroll_speed(1.0);
        }
        assert_eq!(c.speed(), FreeFlyConfig::MAX_SPEED);
        for _ in 0..400 {
            c.scroll_speed(-1.0);
        }
        assert_eq!(c.speed(), FreeFlyConfig::MIN_SPEED);
        c.scroll_speed(f32::NAN);
        assert_eq!(c.speed(), FreeFlyConfig::MIN_SPEED);
    }

    #[test]
    fn it_can_be_placed_and_aimed() {
        let mut c = cam();
        c.set_position(Vec3::new(5.0, 2.0, 5.0));
        c.look_at(Vec3::new(5.0, 2.0, -5.0));
        assert!(
            close(c.camera().forward(), Vec3::NEG_Z),
            "{:?}",
            c.camera().forward()
        );
        c.look_at(Vec3::new(15.0, 2.0, 5.0));
        assert!(
            close(c.camera().forward(), Vec3::X),
            "{:?}",
            c.camera().forward()
        );
        c.look_at(Vec3::new(5.0, 12.0, 5.0));
        assert!(
            c.camera().forward().y > 0.99,
            "straight up is limited, not flipped: {:?}",
            c.camera().forward()
        );
        assert!(
            c.pitch() < std::f32::consts::FRAC_PI_2 - 1e-3,
            "the pitch limit applies to aiming too: {}",
            c.pitch()
        );
        c.look_at(c.position());
        assert!(
            c.camera().is_valid(),
            "aiming at itself changes nothing and does not break it"
        );
        c.set_position(Vec3::new(f32::NAN, 0.0, 0.0));
        assert!(c.position().is_finite(), "a bad position is refused");
    }
}
