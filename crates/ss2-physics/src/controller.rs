//! A **provisional** walking controller (slice of T07-005 for the playable MVP; RFC 0013).
//!
//! The numbers in [`WalkConfig::PROVISIONAL`] are the original's stored single-player movement
//! constants where `docs/re/player-movement-constants.md` (T01-410) has them, and guesses where it does not
//! (each field says which). They are not measurements of the running game: those come from T01-024, and this
//! controller is then replaced or retuned. The original also has a short air-control window after leaving the
//! ground and a separate movement formula; neither is modelled here.
//!
//! The body is one capsule standing along +Y from `radius` to `height - radius` above the feet (T07-047;
//! it was three stacked spheres, which left gaps a thin ledge could slip into), swept against the static
//! world of a [`SpatialQuery`]. [`Walker::step`] is a pure function of
//! (state, input, world, dt): it runs fixed substeps of [`SUBSTEP`] seconds and carries the
//! remainder to the next call, so frame rate does not change the path. No clock, no hash maps.
//!
//! **T09-007** adds four things on top of T07-030's walk/jump: **crouch** (the capsule shrinks to
//! [`WalkConfig::crouch_height`] while [`WalkInput::crouch`] is held and the eyes drop to
//! [`WalkConfig::crouch_eye_height`], `docs/re/player-movement-constants.md`'s stored 0.7, data,
//! high; standing back up is refused — stays crouched — while a short upward probe finds no room,
//! since no walkable-slope-style constant for "how far up must be clear" exists in the data either);
//! **sprint** ([`WalkInput::sprint`], ×[`WalkConfig::sprint_multiplier`] on the ground speed,
//! mirroring `ss2_game::player_view`'s own sprint factor, gated off while crouched exactly as
//! `docs/re/player-movement-constants.md`'s `GetSpeedMultiplier` reading describes — real
//! single-player defaults this off at the game-rule level, `sgp_bEnableSprinting` = 0,
//! `docs/re/console-config.md`, which this crate has no game-rules layer to gate on yet); a
//! **footstep event stream** ([`Walker::footsteps`], distance-accumulated while grounded and
//! moving, so cadence scales with speed and nothing fires while airborne — the original drives
//! footsteps from animation hot-spots this crate has no animation system to read yet,
//! `docs/re/puppet-legged-classes.md`'s `OnAnimEventStep`, so [`WalkConfig::footstep_stride`] is a
//! guessed distance, not a data value); and a **fall-height measurement**
//! ([`Walker::landed_fall_height`], the drop from the airborne peak to the landing) for
//! `ss2_game::fall_damage`'s hook — `docs/re/puppet-damage.md` found no fall-damage formula to
//! reproduce, so that hook always returns `None`.
//!
//! ```
//! use ss2_core::math::{Vec2, Vec3};
//! use ss2_physics::controller::{WalkConfig, WalkInput, Walker};
//! use ss2_physics::query::{PhysMaterialId, QueryWorld};
//! use ss2_physics::spatial::TriMesh;
//!
//! // A floor: two triangles at y = 0.
//! let v = [Vec3::new(-9.0, 0.0, -9.0), Vec3::new(9.0, 0.0, -9.0),
//!          Vec3::new(9.0, 0.0, 9.0), Vec3::new(-9.0, 0.0, 9.0)];
//! let mut world = QueryWorld::new();
//! world.set_static(TriMesh::new(&v, &[0, 2, 1, 0, 3, 2]).unwrap(), vec![PhysMaterialId(0); 2]).unwrap();
//!
//! let cfg = WalkConfig::PROVISIONAL;
//! let mut walker = Walker::new(Vec3::new(0.0, 1.0, 0.0));
//! let input = WalkInput { wish: Vec2::new(0.0, 1.0), ..WalkInput::default() };
//! for _ in 0..60 {
//!     walker.step(&world, &cfg, Vec3::new(0.0, -30.0, 0.0), &input, 1.0 / 60.0);
//! }
//! assert!(walker.grounded && walker.position.z < -3.0);
//! ```

use ss2_core::math::{Vec2, Vec3};

use crate::geometry::Ray;
use crate::query::{Hit, PhysMaterialId, QueryFilter, Shape, SpatialQuery};

/// Length of one internal step, seconds.
pub const SUBSTEP: f32 = 1.0 / 120.0;
/// The longest frame [`Walker::step`] simulates; longer frames are cut to this.
pub const MAX_FRAME: f32 = 0.1;
/// How far the body keeps from surfaces, metres.
const SKIN: f32 = 0.01;
/// An overlap shallower than this (metres) is rounding at a resting contact, not penetration.
const TOUCH: f32 = 1e-4;
/// Slide iterations per move.
const MAX_SLIDES: usize = 4;
/// Horizontal speed below which no footstep distance accumulates (standing still, not a real
/// step), metres per second.
const MIN_FOOTSTEP_SPEED: f32 = 0.05;
/// How many footstep events [`Walker::step`] keeps per call; far more than one frame (even the
/// longest, [`MAX_FRAME`]) can produce at a sane stride and speed, so this is a safety cap, not a
/// tuned limit.
const MAX_FOOTSTEPS_PER_STEP: usize = 16;

/// The walker's size and feel. Units: metres, seconds, radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WalkConfig {
    /// Radius of the body's capsule.
    pub radius: f32,
    /// Height from feet to the top of the head.
    pub height: f32,
    /// Height of the eyes above the feet.
    pub eye_height: f32,
    /// The highest ledge walked onto without jumping.
    pub step_height: f32,
    /// Cosine of the steepest slope that counts as ground.
    pub max_slope_cos: f32,
    /// Running speed on the ground.
    pub speed: f32,
    /// How fast the ground speed approaches the wished velocity.
    pub accel: f32,
    /// How fast the ground speed drops with no input.
    pub friction: f32,
    /// How fast the speed approaches the wished velocity in the air.
    pub air_accel: f32,
    /// Upward speed at the start of a jump.
    pub jump_speed: f32,
    /// Height from feet to the top of the head while crouched (T09-007).
    pub crouch_height: f32,
    /// Height of the eyes above the feet while crouched (T09-007).
    pub crouch_eye_height: f32,
    /// Ground speed multiplier while sprinting (T09-007; gated off while crouched).
    pub sprint_multiplier: f32,
    /// Horizontal distance between footstep events while grounded and moving (T09-007); the
    /// period between events is this divided by the current horizontal speed, so cadence scales
    /// with speed on its own.
    pub footstep_stride: f32,
}

impl WalkConfig {
    /// The single-player player's stored constants (T01-410, `docs/re/player-movement-constants.md`; confidence as
    /// that page gives it) with guesses where it has none; **not** measurements of the running game (T01-024).
    pub const PROVISIONAL: Self = Self {
        // Guess: the collision body is not resolved (T01-057).
        radius: 0.4,
        // Guess, chosen to stay above the eyes.
        height: 2.1,
        // Stored standing view height (high); whether the model stretch of 0.9 applies on top is open.
        eye_height: 1.95,
        // Stored step-up height (high for the value; it is shared with most enemies, so its exact use is open).
        step_height: 2.0,
        // Guess: no general walkable-slope limit was found (the stored 60 degrees is only for starting a jump).
        max_slope_cos: 0.707,
        // Stored nominal ground speed, one for all directions (high).
        speed: 7.5,
        // Measured on the running original (T01-432, medium): a linear ramp of about 47 units/s^2, full speed in
        // about 0.16 s. The stored `pep_fAcceleration` (75) drives a joint force, not dv/dt directly.
        accel: 47.0,
        // Measured (T01-432, medium): about 55 to 60 units/s^2, a stop in about 0.14 s (stored
        // `pep_fDecceleration` 45 is again a joint force).
        friction: 57.0,
        // The original keeps the ground acceleration in the air (high); its short air-control window is not modelled.
        air_accel: 47.0,
        // Stored jump speed (high).
        jump_speed: 10.0,
        // Guess: no crouch body height is in the data (collision shape is open, T01-410's own
        // "Collision shape" section); tall enough to clear the stored crouch eye height below.
        crouch_height: 1.2,
        // Stored crouch view height (high; `pbp_fViewHeightCrouch`, `docs/re/player-movement-constants.md`).
        crouch_eye_height: 0.7,
        // `GetSpeedMultiplier`'s sprint factor (code, medium; `docs/re/player-movement-constants.md`),
        // the same value `ss2_game::player_view::SPRINT_MULTIPLIER` uses for the cosmetic bob.
        sprint_multiplier: 1.5,
        // Guess: no footstep stride or period is in the data (the original drives footsteps from
        // animation hot-spots, `docs/re/puppet-legged-classes.md`'s `OnAnimEventStep`, which this
        // crate has no animation system to read yet); tuned to feel like a walk cycle at the
        // nominal ground speed.
        footstep_stride: 0.9,
    };
}

/// What the player asks for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WalkInput {
    /// Wished direction relative to the heading: `x` right, `y` forward; longer than 1 is cut.
    pub wish: Vec2,
    /// Heading in radians around +Y; 0 looks down -Z.
    pub yaw: f32,
    /// Jump if standing on ground.
    pub jump: bool,
    /// Crouch (T09-007): held, the body shrinks to [`WalkConfig::crouch_height`]; released, it
    /// stands back up only once there is room overhead.
    pub crouch: bool,
    /// Sprint (T09-007): `×`[`WalkConfig::sprint_multiplier`] on the ground speed, ignored while
    /// crouched (`docs/re/player-movement-constants.md`'s `GetSpeedMultiplier` reading).
    pub sprint: bool,
}

/// A footstep that happened during one [`Walker::step`] call (T09-007), for audio later — no
/// sound is played here. Fired by distance travelled while grounded, not by animation (the
/// original's own mechanism, `OnAnimEventStep`, needs an animation system this crate does not
/// have yet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootstepEvent {
    /// The walker's feet at the moment of the step.
    pub position: Vec3,
    /// The material of the ground walked on (`docs/re/puppet-legged-classes.md`'s
    /// `OnAnimEventStep` spawns its effect "for the floor material under it"; this is the same
    /// idea, read from the ground probe instead of an animation attachment).
    pub material: PhysMaterialId,
    /// The walker's horizontal speed at the moment of the step, metres per second.
    pub speed: f32,
}

/// A walking body.
#[derive(Clone, Debug, PartialEq)]
pub struct Walker {
    /// Feet position.
    pub position: Vec3,
    /// Velocity, metres per second.
    pub velocity: Vec3,
    /// Standing on walkable ground.
    pub grounded: bool,
    /// Time not yet simulated (less than one substep).
    carry: f32,
    /// Crouched (T09-007): the body is [`WalkConfig::crouch_height`] tall and the eyes are at
    /// [`WalkConfig::crouch_eye_height`] instead of the standing values.
    crouched: bool,
    /// The material of the ground last stood on (T09-007; for [`FootstepEvent::material`]).
    ground_material: PhysMaterialId,
    /// Distance accumulated towards the next footstep while grounded and moving, metres
    /// (T09-007).
    footstep_phase: f32,
    /// This call's footstep events (T09-007); cleared at the start of every [`Walker::step`].
    footsteps: Vec<FootstepEvent>,
    /// The highest point reached since last leaving the ground, if currently airborne
    /// (T09-007; for [`Walker::landed_fall_height`]).
    air_peak_y: Option<f32>,
    /// How far the walker fell before landing this call, if it landed this call (T09-007);
    /// cleared at the start of every [`Walker::step`].
    landed_fall_height: Option<f32>,
}

impl Walker {
    /// A walker standing still at `position` (feet), not yet on ground.
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            grounded: false,
            carry: 0.0,
            crouched: false,
            ground_material: PhysMaterialId::default(),
            footstep_phase: 0.0,
            footsteps: Vec::new(),
            air_peak_y: None,
            landed_fall_height: None,
        }
    }

    /// Where the eyes are: [`WalkConfig::eye_height`], or [`WalkConfig::crouch_eye_height`]
    /// while [`Walker::crouched`].
    pub fn eye(&self, cfg: &WalkConfig) -> Vec3 {
        let h = if self.crouched {
            cfg.crouch_eye_height
        } else {
            cfg.eye_height
        };
        self.position + Vec3::Y * h
    }

    /// Crouched (T09-007): the body is shorter and the eyes are lower; set by
    /// [`WalkInput::crouch`], but standing back up is refused while there is no room overhead.
    pub fn crouched(&self) -> bool {
        self.crouched
    }

    /// This call's footstep events (T09-007), for audio later; empty on a call that produced
    /// none (not moving, not grounded, or no stride crossed yet).
    pub fn footsteps(&self) -> &[FootstepEvent] {
        &self.footsteps
    }

    /// How far the walker fell before landing, if it landed during this call to
    /// [`Walker::step`] (T09-007); `None` on every other call. `ss2_game::fall_damage` is the
    /// hook this measurement feeds, once a fall-damage formula exists (T01-436) — it does not
    /// today.
    pub fn landed_fall_height(&self) -> Option<f32> {
        self.landed_fall_height
    }

    /// Advance by `dt` seconds. A non-finite or non-positive `dt` does nothing; more than
    /// [`MAX_FRAME`] counts as [`MAX_FRAME`].
    pub fn step(
        &mut self,
        world: &dyn SpatialQuery,
        cfg: &WalkConfig,
        gravity: Vec3,
        input: &WalkInput,
        dt: f32,
    ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.footsteps.clear();
        self.landed_fall_height = None;
        self.carry += dt.min(MAX_FRAME);
        // The jump press is taken once per frame, by the first substep that stands on ground.
        let mut jump = input.jump;
        while self.carry >= SUBSTEP {
            self.carry -= SUBSTEP;
            self.substep(world, cfg, gravity, input, &mut jump);
        }
    }

    fn substep(
        &mut self,
        world: &dyn SpatialQuery,
        cfg: &WalkConfig,
        gravity: Vec3,
        input: &WalkInput,
        jump: &mut bool,
    ) {
        let h = SUBSTEP;
        let was_grounded = self.grounded;

        // Crouch (T09-007): crouching down is always allowed; standing back up needs a short
        // upward probe to find room, since no "walkable slope"-style constant exists for it in
        // the data either (see the module docs).
        if input.crouch {
            self.crouched = true;
        } else if self.crouched {
            let needed = (cfg.height - cfg.crouch_height).max(0.0);
            let crouch_top = (cfg.crouch_height - cfg.radius).max(cfg.radius);
            let probe_from = self.position + Vec3::Y * crouch_top;
            if needed <= 0.0
                || cast_sphere(world, probe_from, cfg.radius, Vec3::Y * needed).is_none()
            {
                self.crouched = false;
            }
        }
        let height = if self.crouched {
            cfg.crouch_height
        } else {
            cfg.height
        };

        // Horizontal velocity towards the wish; sprinting (T09-007) scales the target, gated off
        // while crouched.
        let wish = input.wish.clamp_length_max(1.0);
        let sprint_factor = if input.sprint && !self.crouched {
            cfg.sprint_multiplier
        } else {
            1.0
        };
        let (sin, cos) = input.yaw.sin_cos();
        let forward = Vec3::new(-sin, 0.0, -cos);
        let right = Vec3::new(cos, 0.0, -sin);
        let target = (right * wish.x + forward * wish.y) * cfg.speed * sprint_factor;
        let horizontal = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
        let horizontal = if self.grounded {
            let rate = if wish == Vec2::ZERO {
                cfg.friction
            } else {
                cfg.accel
            };
            move_towards(horizontal, target, rate * h)
        } else if wish != Vec2::ZERO {
            move_towards(horizontal, target, cfg.air_accel * h)
        } else {
            horizontal
        };
        self.velocity.x = horizontal.x;
        self.velocity.z = horizontal.z;

        let mut jumped = false;
        if self.grounded && *jump {
            self.velocity.y = cfg.jump_speed;
            self.grounded = false;
            *jump = false;
            jumped = true;
        }
        if self.grounded {
            self.velocity.y = 0.0;
        } else {
            self.velocity += gravity * h;
        }

        // Move, sliding along what is hit.
        let start = self.position;
        let delta = self.velocity * h;
        let blocked = self.slide(world, cfg, delta, height);

        // A wall met while walking may be a step: try again from higher up.
        if was_grounded && !jumped && blocked.is_some_and(|n| n.y.abs() < cfg.max_slope_cos) {
            self.try_step_up(world, cfg, start, delta, height);
        }

        // Find the ground under the feet; stick to it when walking down. Skipped while rising
        // out of a jump or off a ledge (there is nothing to stand on yet).
        let skip_probe = jumped || (!was_grounded && self.velocity.y > 0.0);
        let mut grounded_now = false;
        if !skip_probe {
            let probe = if was_grounded {
                cfg.step_height
            } else {
                2.0 * SKIN
            };
            if let Some(hit) = cast_sphere(
                world,
                self.position + Vec3::Y * cfg.radius,
                cfg.radius,
                Vec3::NEG_Y * probe,
            ) && walkable(world, cfg, &hit)
            {
                if !hit.started_overlapping {
                    self.position.y -= (hit.distance - SKIN).max(0.0);
                }
                grounded_now = true;
                self.velocity.y = 0.0;
                self.ground_material = hit.material;
            }
        }
        self.grounded = grounded_now;

        // Fall-height measurement and footstep events (T09-007), from this substep's final
        // grounded state.
        if self.grounded {
            if !was_grounded && let Some(peak) = self.air_peak_y.take() {
                let fall = peak - self.position.y;
                if fall > 1e-3 {
                    self.landed_fall_height = Some(fall);
                }
            }
            let h_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();
            if h_speed > MIN_FOOTSTEP_SPEED && cfg.footstep_stride > 1e-3 {
                self.footstep_phase += h_speed * h;
                while self.footstep_phase >= cfg.footstep_stride
                    && self.footsteps.len() < MAX_FOOTSTEPS_PER_STEP
                {
                    self.footstep_phase -= cfg.footstep_stride;
                    self.footsteps.push(FootstepEvent {
                        position: self.position,
                        material: self.ground_material,
                        speed: h_speed,
                    });
                }
            }
        } else {
            self.air_peak_y = Some(
                self.air_peak_y
                    .map_or(self.position.y, |p| p.max(self.position.y)),
            );
        }
    }

    /// Move by `delta`, sliding along surfaces; `height` is this substep's effective body height
    /// (standing or crouched, T09-007). Returns the normal of the last surface that blocked the
    /// move, if any.
    fn slide(
        &mut self,
        world: &dyn SpatialQuery,
        cfg: &WalkConfig,
        delta: Vec3,
        height: f32,
    ) -> Option<Vec3> {
        let mut delta = delta;
        let mut planes = [Vec3::ZERO; MAX_SLIDES];
        let mut count = 0;
        let mut last = None;
        for _ in 0..MAX_SLIDES {
            let len = delta.length();
            if len < 1e-6 {
                break;
            }
            let Some(hit) = cast_body(world, cfg, &mut self.position, delta, height) else {
                self.position += delta;
                break;
            };
            let dir = delta / len;
            let moved = if hit.started_overlapping {
                // Push out of what we are in, then try the rest of the move.
                self.position += hit.normal * SKIN;
                0.0
            } else {
                let travel = (hit.distance - SKIN).clamp(0.0, len);
                self.position += dir * travel;
                travel
            };
            let n = hit.normal;
            last = Some(n);
            planes[count] = n;
            count += 1;
            let mut rest = clip(delta - dir * moved, n);
            self.velocity = clip(self.velocity, n);
            if count >= 2 {
                // In a crease between two surfaces: only along their shared line.
                let crease = planes[count - 2].cross(n).normalize_or_zero();
                rest = crease * rest.dot(crease);
                self.velocity = crease * self.velocity.dot(crease);
            }
            delta = rest;
        }
        last
    }

    /// From `start`, rise by the step height, redo the horizontal part of `delta` and come back
    /// down; keep the result only if it stands on walkable ground and got further. `height` is
    /// this substep's effective body height (T09-007).
    fn try_step_up(
        &mut self,
        world: &dyn SpatialQuery,
        cfg: &WalkConfig,
        start: Vec3,
        delta: Vec3,
        height: f32,
    ) {
        let flat = Vec3::new(delta.x, 0.0, delta.z);
        let progress = |p: Vec3| (p - start).dot(flat);
        let blocked_at_position = self.position;
        let mut trial = self.clone();
        trial.position = start;
        let up = Vec3::Y * cfg.step_height;
        let rise = match cast_body(world, cfg, &mut trial.position, up, height) {
            Some(hit) if !hit.started_overlapping => (hit.distance - SKIN).max(0.0),
            Some(_) => return,
            None => cfg.step_height,
        };
        trial.position.y += rise;
        trial.slide(world, cfg, flat, height);
        let feet = trial.position + Vec3::Y * cfg.radius;
        match cast_sphere(world, feet, cfg.radius, Vec3::NEG_Y * (rise + SKIN * 2.0)) {
            Some(hit) if !hit.started_overlapping && walkable(world, cfg, &hit) => {
                trial.position.y -= (hit.distance - SKIN).max(0.0);
            }
            _ => return,
        }
        if progress(trial.position) > progress(blocked_at_position) + 1e-4 {
            trial.velocity.y = 0.0;
            *self = trial;
        }
    }
}

/// The body: a capsule standing along +Y from `radius` to `height - radius` above the feet
/// (never shorter than a sphere), and how far above the feet its middle is. `height` is the
/// effective body height for this call (standing or crouched, T09-007).
fn body(cfg: &WalkConfig, height: f32) -> (Shape, f32) {
    let r = cfg.radius;
    let top = (height - r).max(r);
    let shape = Shape::Capsule {
        half_height: (top - r) * 0.5,
        radius: r,
    };
    (shape, (r + top) * 0.5)
}

/// The first thing the body standing at `feet` meets when moved by `delta`, against the static
/// world; a rejected query counts as no hit. `height` is the effective body height for this call
/// (standing or crouched, T09-007).
///
/// A surface the body already touches but moves away from or along does not stop it (standing
/// on a floor does not stop a step up or a walk). Because one cast reports only the nearest
/// contact, such a surface would hide anything behind it, so the body first steps out of it, by
/// its depth plus [`SKIN`] along its normal (moving `feet`), and casts again; at most
/// [`MAX_SLIDES`] times, which bounds the push-out. An overlap it moves into comes back as a hit
/// (one shallower than [`TOUCH`] as a plain touch at distance 0).
fn cast_body(
    world: &dyn SpatialQuery,
    cfg: &WalkConfig,
    feet: &mut Vec3,
    delta: Vec3,
    height: f32,
) -> Option<Hit> {
    let (shape, middle) = body(cfg, height);
    let r = cfg.radius;
    let top = (height - r).max(r);
    let filter = QueryFilter {
        dynamics: false,
        ..QueryFilter::default()
    };
    for _ in 0..MAX_SLIDES {
        let mut hit = world
            .shapecast(&shape, *feet + Vec3::Y * middle, delta, &filter)
            .ok()
            .flatten()?;
        if !hit.started_overlapping {
            return Some(hit);
        }
        // `point` is the nearest point of the surface; the nearest point of the axis is
        // straight across from it, so the depth is the radius less the distance between them.
        let axis = *feet + Vec3::Y * (hit.point.y - feet.y).clamp(r, top);
        let depth = (r - (hit.point - axis).length()).max(0.0);
        if hit.normal.dot(delta) < -1e-6 {
            // Moving into it. Resting exactly on a surface can round to a hair inside it: that
            // is a touch at 0, not something to be pushed out of.
            if depth <= TOUCH {
                hit.started_overlapping = false;
                hit.distance = 0.0;
            }
            return Some(hit);
        }
        *feet += hit.normal * (depth + SKIN);
    }
    None
}

/// One sphere against the static world (the ground probes cast the capsule's bottom end); a
/// rejected query counts as no hit, and so does a surface the sphere already touches but moves
/// away from or along.
fn cast_sphere(world: &dyn SpatialQuery, center: Vec3, radius: f32, delta: Vec3) -> Option<Hit> {
    let filter = QueryFilter {
        dynamics: false,
        ..QueryFilter::default()
    };
    world
        .shapecast(&Shape::Sphere { radius }, center, delta, &filter)
        .ok()
        .flatten()
        .filter(|hit| !(hit.started_overlapping && hit.normal.dot(delta) >= -1e-6))
}

/// Whether a hit below the body is ground. A sphere resting on the rim of a ledge touches the
/// rounded edge, whose contact normal leans; then the surface itself decides, found by a short
/// ray down just past the contact point, onto the ledge.
fn walkable(world: &dyn SpatialQuery, cfg: &WalkConfig, hit: &Hit) -> bool {
    if hit.normal.y >= cfg.max_slope_cos {
        return true;
    }
    if hit.normal.y <= 0.0 {
        return false;
    }
    let inward = -Vec3::new(hit.normal.x, 0.0, hit.normal.z).normalize_or_zero();
    let Some(ray) = Ray::new(hit.point + inward * 0.02 + Vec3::Y * 0.05, Vec3::NEG_Y) else {
        return false;
    };
    let filter = QueryFilter {
        dynamics: false,
        ..QueryFilter::default()
    };
    world
        .raycast(&ray, 0.1, &filter)
        .is_some_and(|h| h.normal.y >= cfg.max_slope_cos)
}

/// Remove the part of `v` that goes into a surface with normal `n`.
fn clip(v: Vec3, n: Vec3) -> Vec3 {
    v - n * v.dot(n).min(0.0)
}

/// Move `from` towards `to` by at most `max`.
fn move_towards(from: Vec3, to: Vec3, max: f32) -> Vec3 {
    let d = to - from;
    let len = d.length();
    if len <= max || len < 1e-9 {
        to
    } else {
        from + d / len * max
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
