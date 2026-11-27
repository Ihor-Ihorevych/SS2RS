//! A **provisional** walking controller (slice of T07-005 for the playable MVP; RFC 0013).
//!
//! The numbers in [`WalkConfig::PROVISIONAL`] are tuned by feel. They are not measurements of
//! the original game; those come from T01-024, and this controller is then replaced or retuned.
//!
//! The body is three spheres stacked along +Y (capsule casts are not available yet, T07-004b),
//! swept against the static world of a [`SpatialQuery`]. [`Walker::step`] is a pure function of
//! (state, input, world, dt): it runs fixed substeps of [`SUBSTEP`] seconds and carries the
//! remainder to the next call, so frame rate does not change the path. No clock, no hash maps.
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
//! let input = WalkInput { wish: Vec2::new(0.0, 1.0), yaw: 0.0, jump: false };
//! for _ in 0..60 {
//!     walker.step(&world, &cfg, Vec3::new(0.0, -30.0, 0.0), &input, 1.0 / 60.0);
//! }
//! assert!(walker.grounded && walker.position.z < -3.0);
//! ```

use ss2_core::math::{Vec2, Vec3};

use crate::geometry::Ray;
use crate::query::{Hit, QueryFilter, Shape, SpatialQuery};

/// Length of one internal step, seconds.
pub const SUBSTEP: f32 = 1.0 / 120.0;
/// The longest frame [`Walker::step`] simulates; longer frames are cut to this.
pub const MAX_FRAME: f32 = 0.1;
/// How far the body keeps from surfaces, metres.
const SKIN: f32 = 0.01;
/// Slide iterations per move.
const MAX_SLIDES: usize = 4;

/// The walker's size and feel. Units: metres, seconds, radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WalkConfig {
    /// Radius of the body's spheres.
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
}

impl WalkConfig {
    /// Numbers tuned by feel for the playable MVP; **not** the original game's (T01-024).
    pub const PROVISIONAL: Self = Self {
        radius: 0.4,
        height: 1.8,
        eye_height: 1.65,
        step_height: 0.5,
        max_slope_cos: 0.707,
        speed: 10.0,
        accel: 80.0,
        friction: 60.0,
        air_accel: 15.0,
        jump_speed: 9.0,
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
}

/// A walking body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walker {
    /// Feet position.
    pub position: Vec3,
    /// Velocity, metres per second.
    pub velocity: Vec3,
    /// Standing on walkable ground.
    pub grounded: bool,
    /// Time not yet simulated (less than one substep).
    carry: f32,
}

impl Walker {
    /// A walker standing still at `position` (feet), not yet on ground.
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            grounded: false,
            carry: 0.0,
        }
    }

    /// Where the eyes are.
    pub fn eye(&self, cfg: &WalkConfig) -> Vec3 {
        self.position + Vec3::Y * cfg.eye_height
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

        // Horizontal velocity towards the wish.
        let wish = input.wish.clamp_length_max(1.0);
        let (sin, cos) = input.yaw.sin_cos();
        let forward = Vec3::new(-sin, 0.0, -cos);
        let right = Vec3::new(cos, 0.0, -sin);
        let target = (right * wish.x + forward * wish.y) * cfg.speed;
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
        let blocked = self.slide(world, cfg, delta);

        // A wall met while walking may be a step: try again from higher up.
        if was_grounded && !jumped && blocked.is_some_and(|n| n.y.abs() < cfg.max_slope_cos) {
            self.try_step_up(world, cfg, start, delta);
        }

        // Find the ground under the feet; stick to it when walking down.
        if jumped || (!was_grounded && self.velocity.y > 0.0) {
            self.grounded = false;
            return;
        }
        let probe = if was_grounded {
            cfg.step_height
        } else {
            2.0 * SKIN
        };
        match cast_sphere(
            world,
            self.position + Vec3::Y * cfg.radius,
            cfg.radius,
            Vec3::NEG_Y * probe,
        ) {
            Some(hit) if walkable(world, cfg, &hit) => {
                if !hit.started_overlapping {
                    self.position.y -= (hit.distance - SKIN).max(0.0);
                }
                self.grounded = true;
                self.velocity.y = 0.0;
            }
            _ => self.grounded = false,
        }
    }

    /// Move by `delta`, sliding along surfaces; returns the normal of the last surface that
    /// blocked the move, if any.
    fn slide(&mut self, world: &dyn SpatialQuery, cfg: &WalkConfig, delta: Vec3) -> Option<Vec3> {
        let mut delta = delta;
        let mut planes = [Vec3::ZERO; MAX_SLIDES];
        let mut count = 0;
        let mut last = None;
        for _ in 0..MAX_SLIDES {
            let len = delta.length();
            if len < 1e-6 {
                break;
            }
            let Some(hit) = cast_body(world, cfg, self.position, delta) else {
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
    /// down; keep the result only if it stands on walkable ground and got further.
    fn try_step_up(
        &mut self,
        world: &dyn SpatialQuery,
        cfg: &WalkConfig,
        start: Vec3,
        delta: Vec3,
    ) {
        let flat = Vec3::new(delta.x, 0.0, delta.z);
        let progress = |p: Vec3| (p - start).dot(flat);
        let blocked_at = *self;
        let mut trial = *self;
        trial.position = start;
        let up = Vec3::Y * cfg.step_height;
        let rise = match cast_body(world, cfg, start, up) {
            Some(hit) if !hit.started_overlapping => (hit.distance - SKIN).max(0.0),
            Some(_) => return,
            None => cfg.step_height,
        };
        trial.position.y += rise;
        trial.slide(world, cfg, flat);
        let feet = trial.position + Vec3::Y * cfg.radius;
        match cast_sphere(world, feet, cfg.radius, Vec3::NEG_Y * (rise + SKIN * 2.0)) {
            Some(hit) if !hit.started_overlapping && walkable(world, cfg, &hit) => {
                trial.position.y -= (hit.distance - SKIN).max(0.0);
            }
            _ => return,
        }
        if progress(trial.position) > progress(blocked_at.position) + 1e-4 {
            trial.velocity.y = 0.0;
            *self = trial;
        }
    }
}

/// The nearest hit of the body's spheres moved by `delta`.
fn cast_body(world: &dyn SpatialQuery, cfg: &WalkConfig, feet: Vec3, delta: Vec3) -> Option<Hit> {
    let r = cfg.radius;
    let top = (cfg.height - r).max(r);
    let mut best: Option<Hit> = None;
    for y in [r, (r + top) * 0.5, top] {
        if let Some(hit) = cast_sphere(world, feet + Vec3::Y * y, r, delta) {
            let better = match &best {
                None => true,
                Some(b) => {
                    (hit.started_overlapping && !b.started_overlapping)
                        || (hit.started_overlapping == b.started_overlapping
                            && hit.distance < b.distance)
                }
            };
            if better {
                best = Some(hit);
            }
        }
    }
    best
}

/// One sphere against the static world; a rejected query counts as no hit, and so does a
/// surface the sphere already touches but moves away from or along (standing on a floor does not
/// stop a step up or a walk).
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
