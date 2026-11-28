//! The player's view: the main camera's field of view and sway, and the first-person weapon's
//! walk bob, strafe lean and landing-dip damping (roadmap T09-008).
//!
//! Sources: `docs/re/weapon-view.md` section 3 (the weapon placement's bob/lag/damping, with the
//! exact triangle-wave and spring formulas) and section 2 (the charge/recoil displacement of the
//! three charging weapons) and section 4.2/6 (the main view's field of view); `docs/re/camera-view.md`
//! (the player's view-spring and banking constants, read as "parameters of the player" per that
//! page's reader invariant 4).
//!
//! **What this computes, every tick, as a pure function of the player's own movement this tick**
//! ([`PlayerView::update`], `(state, input, dt)`, no clock, no allocation):
//!
//! - [`PlayerView::weapon_offset`]: the weapon's walk-bob displacement in its own local frame
//!   (`docs/re/weapon-view.md` section 3 point 2: a triangle-wave phase at 7/s, scaled by the
//!   weapon's own bobbing multiplier and the sprint factor, moved by `u` along local X and `30u²`
//!   along local Y) plus the view-damping spring's own contribution to the weapon's height
//!   (section 3 point 3: the vertical spring, times 0.15).
//! - [`PlayerView::camera_roll`]: a camera bank driven by the strafe-lean value `q`
//!   (`docs/re/weapon-view.md` section 3 point 2's last sentence: "the strafe lean `q` ... feeds
//!   the view roll"), scaled by the player's own `ppp_fMaxSideStepBanking` (`docs/re/camera-view.md`,
//!   1 degree in every player file). **Simplification:** the original's own roll formula
//!   (`AnimateViewOrigin`, section 3b) adds a second, forward-movement banking term from
//!   `ppp_fMaxMoveBanking` and a signed, squared magnitude term whose coefficients were not
//!   resolved from the decompile (section 3b, "medium"); this module reproduces only the
//!   side-step term, linearly in `q`, since that is the one piece with both a concrete driver
//!   (`q`) and a concrete constant.
//! - [`PlayerView::camera_damping_offset`]: the view-damping spring itself (section 3 point 3:
//!   stiffness 160, damping 10, clamped to -0.4..0.1), applied as a vertical offset of the camera
//!   ("offsets of the view", `docs/re/camera-view.md`'s own words for the same spring) — this
//!   module's chosen reading of "camera bob": a landing dip, not a walking bob (the original's own
//!   walking-bob formula, read in full, moves only the weapon; the camera's own unconditional
//!   "added height" term in `AnimateViewOrigin` rests on two unresolved constants, section 3b, and
//!   is not reproduced here).
//! - [`main_view_fov_y`]: the main view's field of view (`docs/re/weapon-view.md` section 4.2/6: 80
//!   degrees horizontal at 4:3, widened for other aspect ratios, limited to 45 to 135 degrees).
//!
//! **Split off** (needs a firing/charging weapon state that does not exist yet, T09-005): the charge
//! and recoil displacement of the three charging weapons is a pure, fully specified formula
//! (`docs/re/weapon-view.md` section 2) and is reproduced here as [`charge_recoil_offset`], tested
//! against the spec's own test vectors — but nothing calls it yet, since there is no charging state
//! to drive `t` from (`docs/re/weapon-view.md`'s own `DoCharging`/shot-fired triggers). Also split
//! off: the sniper zoom's field-of-view multiplier (needs the sniper's own zoom state) and the swim
//! wobble (needs a "swimming" pose, section 3b point 1) — both left at their rest values (1.0,
//! none) until those systems exist.

use ss2_core::math::{Vec2, Vec3};

/// How fast the walk-bob phase moves towards an end while moving, per second
/// (`docs/re/weapon-view.md` section 3 point 2).
pub const WALK_BOB_FREQUENCY: f32 = 7.0;
/// How fast the walk-bob phase returns to 0 with no movement, per second (same source).
pub const WALK_BOB_RETURN_RATE: f32 = 2.0;
/// How fast the strafe-lean value moves towards its target (either direction), per second.
pub const STRAFE_LEAN_RATE: f32 = 8.0;
/// Seconds for the sprint factor to ramp fully up or down (same source: "over 0.5 s" both ways).
pub const SPRINT_RAMP_TIME: f32 = 0.5;
/// The sprint factor while sprinting (1.0 otherwise).
pub const SPRINT_MULTIPLIER: f32 = 1.5;
/// The view-damping spring's stiffness (`ppp_fViewDampingK`, `docs/re/camera-view.md`).
pub const VIEW_DAMPING_K: f32 = 160.0;
/// The view-damping spring's damping (`ppp_fViewDampingD`).
pub const VIEW_DAMPING_D: f32 = 10.0;
/// The view-damping spring's clamp range, metres (walking; swimming uses a narrower range this
/// module does not model, no swim state existing yet).
pub const VIEW_DAMPING_RANGE: (f32, f32) = (-0.4, 0.1);
/// How much of the view-damping spring reaches the weapon's height (`docs/re/weapon-view.md`
/// section 3 point 3: "times 0.15").
pub const WEAPON_DAMPING_SCALE: f32 = 0.15;
/// The walk-bob's own coefficient, metres (`u = b × p × f × (-0.02)`).
pub const BOB_X_COEFFICIENT: f32 = -0.02;
/// The walk-bob's Y term's coefficient (`30 × u²`).
pub const BOB_Y_COEFFICIENT: f32 = 30.0;
/// `ppp_fMaxSideStepBanking`, degrees (`docs/re/camera-view.md`): the camera roll's limit this
/// module reproduces (see module docs for what is simplified).
pub const MAX_SIDESTEP_BANK_DEG: f32 = 1.0;
/// The main view's horizontal field of view at a 4:3 aspect ratio, degrees
/// (`CGlobalPlayerParams.gpp_fFOV`, `docs/re/camera-view.md`).
pub const MAIN_FOV_X_AT_4_3_DEG: f32 = 80.0;
/// The main view's widened horizontal field of view is clamped to this range, degrees
/// (`docs/re/weapon-view.md` section summary point 5).
pub const MAIN_FOV_X_RANGE_DEG: (f32, f32) = (45.0, 135.0);

/// What the player's movement this tick feeds into [`PlayerView::update`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerViewInput {
    /// The relative-move vector, `x` right, `y` forward (same convention as
    /// `ss2_physics::controller::WalkInput::wish`); whether either is nonzero decides the walk-bob
    /// phase, and the sign of `x` decides the strafe lean's target.
    pub wish: Vec2,
    /// Sprinting this tick (`docs/re/weapon-view.md` section 3 point 2's sprint factor). No real
    /// sprint key exists yet (T09-007); callers without one pass `false`.
    pub sprinting: bool,
    /// The player's current vertical velocity, metres per second (the walker's `velocity.y`); the
    /// view-damping spring is driven by this value's **change** from tick to tick, not its level.
    pub vertical_velocity: f32,
}

/// The player's view state: the walk-bob phase, the strafe lean, the sprint factor and the
/// view-damping spring, each a pure function of the movement fed to [`PlayerView::update`] tick by
/// tick. `Default` is at rest (no movement, no sprint, the spring settled at 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerView {
    /// Walk-bob phase `p`, in `[-1, 1]`.
    phase: f32,
    /// Which way `phase` is currently moving (`+1` or `-1`); flips when `phase` reaches an end.
    phase_dir: f32,
    /// Strafe lean `q`, in `[-1, 1]`.
    lean: f32,
    /// Sprint factor `f`, in `[1, 1.5]`.
    sprint: f32,
    /// The view-damping spring's value (the camera's vertical offset, metres).
    damping: f32,
    /// The view-damping spring's own velocity (not the player's).
    damping_vel: f32,
    /// The player's vertical velocity on the previous tick, to find this tick's change.
    prev_vertical_velocity: f32,
    /// Time not yet simulated (less than one substep; `ss2_physics::controller::Walker`'s own
    /// pattern, so this state is frame-rate independent exactly as the walker it reads from is).
    carry: f32,
}

impl Default for PlayerView {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerView {
    /// At rest: no bob, no lean, sprint factor 1, the spring settled at 0.
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            phase_dir: 1.0,
            lean: 0.0,
            sprint: 1.0,
            damping: 0.0,
            damping_vel: 0.0,
            prev_vertical_velocity: 0.0,
            carry: 0.0,
        }
    }

    /// Advance by `dt` seconds with this tick's movement. A non-finite or non-positive `dt` does
    /// nothing (matching `Walker::step`'s own rule). Runs fixed substeps of
    /// `ss2_physics::controller::SUBSTEP` and carries the remainder to the next call, so (as for
    /// `Walker`) frame rate does not change the result.
    pub fn update(&mut self, input: &PlayerViewInput, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.carry += dt.min(ss2_physics::controller::MAX_FRAME);
        while self.carry >= ss2_physics::controller::SUBSTEP {
            self.carry -= ss2_physics::controller::SUBSTEP;
            self.substep(input, ss2_physics::controller::SUBSTEP);
        }
    }

    fn substep(&mut self, input: &PlayerViewInput, h: f32) {
        // Walk-bob phase: a triangle wave while moving, decaying to 0 otherwise
        // (`docs/re/weapon-view.md` section 3 point 2).
        let moving = input.wish.x != 0.0 || input.wish.y != 0.0;
        if moving {
            self.phase += self.phase_dir * WALK_BOB_FREQUENCY * h;
            if self.phase >= 1.0 {
                self.phase = 1.0;
                self.phase_dir = -1.0;
            } else if self.phase <= -1.0 {
                self.phase = -1.0;
                self.phase_dir = 1.0;
            }
        } else {
            self.phase = move_towards(self.phase, 0.0, WALK_BOB_RETURN_RATE * h);
        }

        // Strafe lean: moves towards +1/-1/0 depending on the strafe direction, at the same rate
        // both ways.
        let lean_target = if input.wish.x > 0.0 {
            1.0
        } else if input.wish.x < 0.0 {
            -1.0
        } else {
            0.0
        };
        self.lean = move_towards(self.lean, lean_target, STRAFE_LEAN_RATE * h);

        // Sprint factor: a linear ramp over `SPRINT_RAMP_TIME` seconds, either way.
        let sprint_target = if input.sprinting {
            SPRINT_MULTIPLIER
        } else {
            1.0
        };
        let sprint_rate = (SPRINT_MULTIPLIER - 1.0) / SPRINT_RAMP_TIME;
        self.sprint = move_towards(self.sprint, sprint_target, sprint_rate * h);

        // View-damping spring: a damped harmonic oscillator kicked by the player's vertical
        // velocity's change this tick (a landing or a jump start perturbs it; it then relaxes).
        let vertical_velocity = if input.vertical_velocity.is_finite() {
            input.vertical_velocity
        } else {
            self.prev_vertical_velocity
        };
        let kick = vertical_velocity - self.prev_vertical_velocity;
        self.prev_vertical_velocity = vertical_velocity;
        self.damping_vel -= kick;
        let accel = -VIEW_DAMPING_K * self.damping - VIEW_DAMPING_D * self.damping_vel;
        self.damping_vel += accel * h;
        self.damping += self.damping_vel * h;
        self.damping = self
            .damping
            .clamp(VIEW_DAMPING_RANGE.0, VIEW_DAMPING_RANGE.1);
    }

    /// The weapon's walk-bob-and-damping offset in its own local frame (`x` right, `y` up, `z`
    /// forward), for `bobbing_mul` (`bwrp_fBobbingMul`, the weapon's own rendering file).
    /// `docs/re/weapon-view.md` section 3 points 2 and 3.
    pub fn weapon_offset(&self, bobbing_mul: f32) -> Vec3 {
        let u = bobbing_mul * self.phase * self.sprint * BOB_X_COEFFICIENT;
        Vec3::new(
            u,
            BOB_Y_COEFFICIENT * u * u + self.damping * WEAPON_DAMPING_SCALE,
            0.0,
        )
    }

    /// The camera's bank from strafing, radians (see module docs for what this simplifies).
    pub fn camera_roll(&self) -> f32 {
        (self.lean * MAX_SIDESTEP_BANK_DEG).to_radians()
    }

    /// The view-damping spring's own value, metres: the camera's vertical offset (see module docs).
    pub fn camera_damping_offset(&self) -> f32 {
        self.damping
    }
}

/// Move `from` towards `to` by at most `max` (`ss2_physics::controller`'s own helper, duplicated
/// here rather than exposed from a sim-only crate this module has no other reason to depend on).
fn move_towards(from: f32, to: f32, max: f32) -> f32 {
    let d = to - from;
    if d.abs() <= max || d.abs() < 1e-9 {
        to
    } else {
        from + max.copysign(d)
    }
}

/// A charging weapon's own six sway members (`docs/re/weapon-view.md` section 2:
/// `bwp_fOffsetX`/`Y`/`Z`, `bwp_fFrequencyX`/`Y`, `bwp_fOffsetZExponent`) plus its charging and
/// recoil durations, for [`charge_recoil_offset`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeRecoilParams {
    /// `bwp_fChargingTime`.
    pub charging_time: f32,
    /// The weapon's recoil duration.
    pub recoil_time: f32,
    /// `bwp_fOffsetX`, `bwp_fOffsetY`.
    pub offset: Vec2,
    /// `bwp_fOffsetZ`.
    pub offset_z: f32,
    /// `bwp_fFrequencyX`, `bwp_fFrequencyY`.
    pub frequency: Vec2,
    /// `bwp_fOffsetZExponent`.
    pub offset_z_exponent: f32,
}

/// The charge-and-recoil displacement of a charging weapon (grenade launcher, cannon, zap gun;
/// `docs/re/weapon-view.md` section 2, `GetWeaponChargeDisplaceMatrix`), in the weapon's own local
/// frame. `t` is seconds since the charge started (not reset by a shot); `charging` is `true` while
/// the charge builds, `false` while recoiling (any other state has no displacement — return `None`
/// in that case instead of calling this).
///
/// **Not called anywhere yet** (module docs: split off, needs a firing/charging state, T09-005).
pub fn charge_recoil_offset(t: f32, charging: bool, params: &ChargeRecoilParams) -> Vec3 {
    let (k, s) = if charging {
        (1.0, (t / params.charging_time).clamp(0.0, 1.0))
    } else {
        (1.0 - (t / params.recoil_time).clamp(0.0, 1.0), 0.0)
    };
    let tau = std::f32::consts::TAU;
    let x = (tau * params.frequency.x * t).sin() * params.offset.x * s * k;
    let y = (tau * params.frequency.y * t).sin() * params.offset.y * s * k;
    let z = (1.0 - (-t * params.offset_z_exponent).exp()) * params.offset_z * k;
    Vec3::new(x, y, z)
}

/// The main view's field of view (vertical, radians, for [`ss2_render_api::Projection::Perspective`])
/// at `aspect` (width / height): [`MAIN_FOV_X_AT_4_3_DEG`] horizontal at 4:3, widened to preserve
/// the implied vertical field of view at other aspect ratios, with the widened **horizontal** value
/// clamped to [`MAIN_FOV_X_RANGE_DEG`] before converting back (`docs/re/weapon-view.md` section
/// summary point 5: "widened ..., limited to 45 to 135 degrees"). A non-finite or non-positive
/// `aspect` is treated as 4:3.
pub fn main_view_fov_y(aspect: f32) -> f32 {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        4.0 / 3.0
    };
    let fov_x_4_3 = MAIN_FOV_X_AT_4_3_DEG.to_radians();
    // The vertical FOV this horizontal-at-4:3 value implies, held constant across aspect ratios
    // (`docs/re/weapon-view.md`'s own widening formula, `2 atan(aspect × 0.75 × tan(fov / 2))`,
    // read backwards from aspect = 4:3 to find the vertical FOV it is built to preserve).
    let fov_y = 2.0 * ((fov_x_4_3 * 0.5).tan() * 0.75).atan();
    // The horizontal FOV this vertical FOV gives at the real aspect ratio.
    let fov_x = 2.0 * ((fov_y * 0.5).tan() * aspect).atan();
    let fov_x_clamped = fov_x
        .to_degrees()
        .clamp(MAIN_FOV_X_RANGE_DEG.0, MAIN_FOV_X_RANGE_DEG.1)
        .to_radians();
    // Convert the (possibly clamped) horizontal FOV back to the vertical FOV `Camera` stores.
    2.0 * ((fov_x_clamped * 0.5).tan() / aspect).atan()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Advance `view` by `seconds`, in whole substeps (`ss2_physics::controller::SUBSTEP`), so
    /// these tests land on exact multiples of the internal step and are not a floating-point
    /// rounding coin flip at a boundary.
    fn step(view: &mut PlayerView, input: &PlayerViewInput, seconds: f32) {
        let substep = ss2_physics::controller::SUBSTEP;
        let steps = (seconds / substep).round() as u32;
        for _ in 0..steps {
            view.update(input, substep);
        }
    }

    /// Advance `view` one substep at a time until `pred` holds (checked before each substep, so
    /// it fires on the first substep that reaches it), or panic after `max_steps` — used where the
    /// triangle wave's own exact clamp (not the wall-clock time a real walk would take, which does
    /// not divide evenly into `SUBSTEP`) is the thing a test wants to land on precisely.
    fn run_until(
        view: &mut PlayerView,
        input: &PlayerViewInput,
        max_steps: u32,
        pred: impl Fn(&PlayerView) -> bool,
    ) {
        let substep = ss2_physics::controller::SUBSTEP;
        for _ in 0..max_steps {
            if pred(view) {
                return;
            }
            view.update(input, substep);
        }
        panic!("condition not reached within {max_steps} substeps");
    }

    #[test]
    fn standing_still_gives_no_bob_or_lean() {
        let mut view = PlayerView::new();
        let input = PlayerViewInput {
            wish: Vec2::ZERO,
            sprinting: false,
            vertical_velocity: 0.0,
        };
        step(&mut view, &input, 2.0);
        assert_eq!(view.weapon_offset(0.9), Vec3::ZERO);
        assert_eq!(view.camera_roll(), 0.0);
        assert_eq!(view.camera_damping_offset(), 0.0);
    }

    #[test]
    fn walking_gives_a_periodic_bob_of_the_spec_s_amplitude() {
        // docs/re/weapon-view.md test vector 3: bobbing multiplier 0.9, sprint factor 1.0, phase 1:
        // u = -0.018, up 0.0097 m. The phase moves towards +1 at 7/s and clamps there, exactly,
        // once it arrives (it does not wrap or overshoot).
        let mut view = PlayerView::new();
        let input = PlayerViewInput {
            wish: Vec2::new(0.0, 1.0),
            sprinting: false,
            vertical_velocity: 0.0,
        };
        run_until(&mut view, &input, 60, |v| v.phase >= 1.0);
        assert_eq!(view.phase, 1.0);
        let offset = view.weapon_offset(0.9);
        assert!((offset.x - -0.018).abs() < 1e-4, "{offset:?}");
        assert!((offset.y - 0.0097).abs() < 1e-4, "{offset:?}");

        // It is periodic: still walking, the phase swings all the way to the other end too
        // (the triangle wave flips direction at each clamped end, `docs/re/weapon-view.md` section
        // 3 point 2: "it moves ... towards +1 or -1 and flips at the ends").
        run_until(&mut view, &input, 120, |v| v.phase <= -1.0);
        assert_eq!(view.phase, -1.0);
        let offset = view.weapon_offset(0.9);
        assert!((offset.x - 0.018).abs() < 1e-4, "the other end: {offset:?}");
    }

    #[test]
    fn sprinting_scales_the_bob_amplitude_by_the_spec_s_sprint_factor() {
        // Test vector 3: sprinting for 0.5 s or more, f = 1.5, phase 1: u = -0.027, up 0.0219 m.
        let p = PlayerView {
            phase: 1.0,
            phase_dir: -1.0,
            lean: 0.0,
            sprint: 1.5,
            damping: 0.0,
            damping_vel: 0.0,
            prev_vertical_velocity: 0.0,
            carry: 0.0,
        };
        let offset = p.weapon_offset(0.9);
        assert!((offset.x - -0.027).abs() < 1e-3, "{offset:?}");
        assert!((offset.y - 0.0219).abs() < 1e-3, "{offset:?}");

        // The real update path reaches the full sprint factor after the spec's own 0.5 s ramp.
        let mut view = PlayerView::new();
        let input = PlayerViewInput {
            wish: Vec2::new(0.0, 1.0),
            sprinting: true,
            vertical_velocity: 0.0,
        };
        step(&mut view, &input, SPRINT_RAMP_TIME);
        assert!(
            (view.sprint - SPRINT_MULTIPLIER).abs() < 1e-3,
            "{}",
            view.sprint
        );
    }

    #[test]
    fn it_is_deterministic() {
        let input = PlayerViewInput {
            wish: Vec2::new(0.3, 1.0),
            sprinting: false,
            vertical_velocity: -5.0,
        };
        let mut a = PlayerView::new();
        let mut b = PlayerView::new();
        for i in 0..600 {
            let v = if i % 7 == 0 {
                -8.0
            } else {
                input.vertical_velocity
            };
            let input = PlayerViewInput {
                vertical_velocity: v,
                ..input
            };
            a.update(&input, 1.0 / 60.0);
            b.update(&input, 1.0 / 60.0);
        }
        assert_eq!(a, b, "the same inputs, run twice, give the same result");

        // The same total time in different-sized steps gives the same result (fixed internal
        // substeps, `ss2_physics::controller::Walker`'s own pattern, reused here for the same
        // reason: frame rate must not change the path). Both sides are built from exact multiples
        // of `SUBSTEP` so the comparison is not a floating-point-rounding coin flip at the
        // boundary (a frame length like 0.01 s, not a multiple of `SUBSTEP` = 1/120 s, can land a
        // whole substep either side of the boundary by a bit of `f32` rounding).
        let substep = ss2_physics::controller::SUBSTEP;
        let mut one_big_step = PlayerView::new();
        let mut many_small_steps = PlayerView::new();
        one_big_step.update(&input, substep * 12.0);
        for _ in 0..4 {
            many_small_steps.update(&input, substep * 3.0);
        }
        assert!(
            (one_big_step.weapon_offset(1.0) - many_small_steps.weapon_offset(1.0)).length() < 1e-4,
            "{:?} vs {:?}",
            one_big_step.weapon_offset(1.0),
            many_small_steps.weapon_offset(1.0)
        );
    }

    #[test]
    fn a_landing_jolt_decays_back_to_rest() {
        let mut view = PlayerView::new();
        let falling = PlayerViewInput {
            wish: Vec2::ZERO,
            sprinting: false,
            vertical_velocity: -8.0,
        };
        step(&mut view, &falling, 0.5);
        // Landing: the vertical velocity snaps to 0 in one tick, kicking the spring.
        view.update(
            &PlayerViewInput {
                vertical_velocity: 0.0,
                ..falling
            },
            1.0 / 60.0,
        );
        assert_ne!(
            view.camera_damping_offset(),
            0.0,
            "the landing kicked the spring"
        );
        for _ in 0..600 {
            view.update(
                &PlayerViewInput {
                    vertical_velocity: 0.0,
                    ..falling
                },
                1.0 / 60.0,
            );
        }
        assert!(
            view.camera_damping_offset().abs() < 1e-3,
            "settles back to rest: {}",
            view.camera_damping_offset()
        );
    }

    #[test]
    fn the_damping_spring_stays_within_its_clamp() {
        let mut view = PlayerView::new();
        let mut v = 0.0f32;
        for i in 0..600 {
            v = if i % 10 == 0 { v - 20.0 } else { v };
            view.update(
                &PlayerViewInput {
                    wish: Vec2::ZERO,
                    sprinting: false,
                    vertical_velocity: v,
                },
                1.0 / 60.0,
            );
            assert!(
                view.camera_damping_offset() >= VIEW_DAMPING_RANGE.0
                    && view.camera_damping_offset() <= VIEW_DAMPING_RANGE.1
            );
        }
    }

    #[test]
    fn strafing_leans_the_camera_and_releasing_returns_it() {
        let mut view = PlayerView::new();
        let right = PlayerViewInput {
            wish: Vec2::new(1.0, 0.0),
            sprinting: false,
            vertical_velocity: 0.0,
        };
        step(&mut view, &right, 1.0);
        assert!(view.camera_roll() < 0.0 || view.camera_roll() > 0.0);
        assert!((view.lean - 1.0).abs() < 1e-3);
        let still = PlayerViewInput {
            wish: Vec2::ZERO,
            ..right
        };
        step(&mut view, &still, 1.0);
        assert!(view.camera_roll().abs() < 1e-3, "releases back to level");
    }

    #[test]
    fn charge_recoil_offset_matches_the_spec_s_test_vectors() {
        // docs/re/weapon-view.md test vector 1: grenade launcher at t = 0.25 s of a 0.5 s charge.
        let v = charge_recoil_offset(
            0.25,
            true,
            &ChargeRecoilParams {
                charging_time: 0.5,
                recoil_time: 0.3,
                offset: Vec2::new(0.005, 0.005),
                offset_z: 0.4,
                frequency: Vec2::new(5.0, 6.0),
                offset_z_exponent: 3.0,
            },
        );
        assert!((v.x - 0.0025).abs() < 1e-4, "{v:?}");
        assert!(v.y.abs() < 1e-4, "{v:?}");
        assert!((v.z - 0.211).abs() < 1e-3, "{v:?}");

        // Test vector 2: cannon, 1.0 s into a 1.25 s charge.
        let v = charge_recoil_offset(
            1.0,
            true,
            &ChargeRecoilParams {
                charging_time: 1.25,
                recoil_time: 0.5,
                offset: Vec2::new(0.01, 0.01),
                offset_z: 0.4,
                frequency: Vec2::new(10.0, 8.0),
                offset_z_exponent: 2.0,
            },
        );
        assert!(v.x.abs() < 1e-4, "{v:?}");
        assert!((v.z - 0.346).abs() < 1e-3, "{v:?}");
    }

    #[test]
    fn main_view_fov_is_unchanged_at_4_3_and_widens_at_wider_aspects() {
        let base = main_view_fov_y(4.0 / 3.0);
        // At 4:3 the implied horizontal FOV must reproduce 80 degrees.
        let fov_x = 2.0 * ((base * 0.5).tan() * (4.0 / 3.0)).atan();
        assert!(
            (fov_x.to_degrees() - 80.0).abs() < 1e-2,
            "{}",
            fov_x.to_degrees()
        );

        // Wider aspect ratios keep the vertical FOV constant (Hor+) and so widen the horizontal
        // FOV, up to the clamp.
        let widescreen = main_view_fov_y(16.0 / 9.0);
        let fov_x_wide = 2.0 * ((widescreen * 0.5).tan() * (16.0 / 9.0)).atan();
        assert!(fov_x_wide.to_degrees() > 80.0);
        assert!(fov_x_wide.to_degrees() < MAIN_FOV_X_RANGE_DEG.1);

        // An extreme aspect ratio clamps the horizontal FOV rather than exceeding the range.
        let extreme = main_view_fov_y(8.0);
        let fov_x_extreme = 2.0 * ((extreme * 0.5).tan() * 8.0).atan();
        assert!((fov_x_extreme.to_degrees() - MAIN_FOV_X_RANGE_DEG.1).abs() < 1e-1);

        let narrow = main_view_fov_y(0.3);
        let fov_x_narrow = 2.0 * ((narrow * 0.5).tan() * 0.3).atan();
        assert!((fov_x_narrow.to_degrees() - MAIN_FOV_X_RANGE_DEG.0).abs() < 1e-1);
    }

    #[test]
    fn a_bad_aspect_falls_back_to_4_3() {
        assert_eq!(main_view_fov_y(f32::NAN), main_view_fov_y(4.0 / 3.0));
        assert_eq!(main_view_fov_y(0.0), main_view_fov_y(4.0 / 3.0));
        assert_eq!(main_view_fov_y(-1.0), main_view_fov_y(4.0 / 3.0));
    }
}
