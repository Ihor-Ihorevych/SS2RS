//! Level of detail and distance culling: **which version of a model to draw, or none** (roadmap T04-024, `docs/LOD.md`).
//!
//! Pure functions over a [`Camera`] and a bounding sphere; no GPU. A model has a [`LodSet`]: the screen **coverage** at which each lower level takes over, an
//! optional **impostor** (a flat stand-in for a far enemy) and a **cull** limit; an optional farthest distance as well. The [`LodPolicy`] applies the player's
//! quality settings ([`QualitySettings::lod_bias`] and `view_distance`) and a **hysteresis** so a thing at the edge of a threshold does not flicker between two
//! levels as the camera moves a little. [`LodTracker`] remembers the last choice of each [`RenderId`].
//!
//! *Coverage* is the bounding sphere's radius as a share of **half the screen height** at its distance: 1.0 means the sphere is as tall as the screen, 0.1 a tenth
//! of it. It does not depend on the window size, so one set of thresholds serves every resolution (a phone and a monitor choose the same levels for the same
//! picture; the *quality tier* is what asks for coarser levels).
//!
//! ```
//! use ss2_core::math::Vec3;
//! use ss2_render_api::lod::{LodChoice, LodPolicy, LodSet};
//! use ss2_render_api::{Camera, Sphere};
//!
//! // Three levels (switching at 25 % and 8 % of the screen), an impostor below 2 %, gone below 0.5 %.
//! let set = LodSet::new(vec![0.25, 0.08], Some(0.02), 0.005, None).unwrap();
//! let camera = Camera::default(); // at the origin looking down -Z
//! let policy = LodPolicy::default();
//! let at = |z: f32| policy.select(&set, &camera, Sphere::new(Vec3::new(0.0, 0.0, -z), 1.0), None);
//! assert_eq!(at(2.0), LodChoice::Level(0));
//! assert_eq!(at(60.0), LodChoice::Level(2)); // coverage 2.9 %
//! assert_eq!(at(100.0), LodChoice::Impostor); // 1.7 %
//! assert_eq!(at(10_000.0), LodChoice::Culled);
//! ```

use std::collections::BTreeMap;

use crate::quality::QualitySettings;
use crate::{Camera, Projection, RenderId, Sphere};

/// What to draw for one thing this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LodChoice {
    /// A model level: 0 is the most detailed.
    Level(u8),
    /// The flat stand-in.
    Impostor,
    /// Nothing.
    Culled,
}

/// Why a [`LodSet`] or [`LodPolicy`] was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodError(pub &'static str);

impl std::fmt::Display for LodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for LodError {}

fn positive(x: f32) -> bool {
    x.is_finite() && x > 0.0
}

/// How one model is detailed at a distance.
#[derive(Clone, Debug, PartialEq)]
pub struct LodSet {
    /// `switch[k]` is the coverage below which level `k + 1` takes over from level `k`: strictly descending; `switch.len() + 1` levels in all.
    switch: Vec<f32>,
    /// The coverage below which the impostor takes over from the last level.
    impostor_below: Option<f32>,
    /// The coverage below which nothing is drawn.
    cull_below: f32,
    /// Nothing is drawn farther than this (before the policy's view-distance scale).
    max_distance: Option<f32>,
}

impl LodSet {
    /// A set. `switch` is the descending coverage list (empty for a model with one level), `impostor_below` and `cull_below` follow it in order.
    pub fn new(
        switch: Vec<f32>,
        impostor_below: Option<f32>,
        cull_below: f32,
        max_distance: Option<f32>,
    ) -> Result<Self, LodError> {
        if switch.iter().any(|s| !positive(*s)) || !(cull_below.is_finite() && cull_below >= 0.0) {
            return Err(LodError(
                "coverage thresholds must be positive and finite (the cull limit may be 0)",
            ));
        }
        let mut last = f32::INFINITY;
        for s in switch
            .iter()
            .copied()
            .chain(impostor_below)
            .chain(Some(cull_below))
        {
            // Equal values are refused too: a level that is never chosen is a mistake in the data.
            if s >= last {
                return Err(LodError(
                    "thresholds must descend: level switches, then the impostor, then the cull limit",
                ));
            }
            last = s;
        }
        if switch.len() >= 255 {
            return Err(LodError("too many levels"));
        }
        if max_distance.is_some_and(|d| !positive(d)) {
            return Err(LodError("the maximum distance must be positive"));
        }
        Ok(Self {
            switch,
            impostor_below,
            cull_below,
            max_distance,
        })
    }

    /// A model with one level, drawn until `cull_below` and never replaced.
    pub fn single(cull_below: f32) -> Result<Self, LodError> {
        Self::new(Vec::new(), None, cull_below, None)
    }

    /// How many model levels there are.
    pub fn levels(&self) -> usize {
        self.switch.len() + 1
    }

    /// Whether there is an impostor.
    pub fn has_impostor(&self) -> bool {
        self.impostor_below.is_some()
    }

    /// The choice for `coverage` with every threshold scaled by `scale` (a bigger scale makes things coarser sooner), as a rank: levels first, then the
    /// impostor, then culled.
    fn rank(&self, coverage: f32, scale: f32) -> usize {
        let levels = self.levels();
        let mut rank = self
            .switch
            .iter()
            .take_while(|s| coverage < **s * scale)
            .count();
        if rank == levels - 1 {
            // On the last level: the impostor, then nothing.
            let mut next = levels;
            if let Some(i) = self.impostor_below {
                if coverage < i * scale {
                    rank = next;
                }
                next += 1;
            }
            if coverage < self.cull_below * scale {
                rank = next;
            }
        }
        rank
    }

    fn choice(&self, rank: usize) -> LodChoice {
        let levels = self.levels();
        if rank < levels {
            LodChoice::Level(rank as u8)
        } else if rank == levels && self.impostor_below.is_some() {
            LodChoice::Impostor
        } else {
            LodChoice::Culled
        }
    }

    fn to_rank(&self, c: LodChoice) -> usize {
        // A choice this set cannot give (a level it does not have, an impostor it lacks) lands out of range and is clamped by the caller.
        let levels = self.levels();
        match c {
            LodChoice::Level(l) => usize::from(l),
            LodChoice::Impostor => levels,
            LodChoice::Culled => levels + 1,
        }
    }
}

/// The player's view of LOD: quality settings and hysteresis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodPolicy {
    /// Added to the level: each +1 halves the coverage a thing is judged at (so a lower level is used at half the distance); -1 doubles it. -1 to 3.
    pub lod_bias: f32,
    /// Multiplies every set's maximum distance. 0.5 to 2.
    pub view_distance: f32,
    /// How far past a threshold a thing must go before it changes level, as a share of the threshold (0 to 0.5): the camera has to move that much to switch.
    pub hysteresis: f32,
}

impl Default for LodPolicy {
    /// No bias, normal view distance, 10 % hysteresis.
    fn default() -> Self {
        Self {
            lod_bias: 0.0,
            view_distance: 1.0,
            hysteresis: 0.1,
        }
    }
}

impl LodPolicy {
    /// A checked policy.
    pub fn new(lod_bias: f32, view_distance: f32, hysteresis: f32) -> Result<Self, LodError> {
        let p = Self {
            lod_bias,
            view_distance,
            hysteresis,
        };
        p.validate()?;
        Ok(p)
    }

    /// The policy a quality setting asks for (the default hysteresis).
    pub fn from_quality(q: &QualitySettings) -> Self {
        Self {
            lod_bias: q.lod_bias,
            view_distance: q.view_distance,
            ..Self::default()
        }
    }

    /// Whether the values are in range.
    pub fn validate(&self) -> Result<(), LodError> {
        if !(self.lod_bias.is_finite() && (-1.0..=3.0).contains(&self.lod_bias)) {
            return Err(LodError("lod_bias must be -1 to 3"));
        }
        if !(self.view_distance.is_finite() && (0.5..=2.0).contains(&self.view_distance)) {
            return Err(LodError("view_distance must be 0.5 to 2"));
        }
        if !(self.hysteresis.is_finite() && (0.0..=0.5).contains(&self.hysteresis)) {
            return Err(LodError("hysteresis must be 0 to 0.5"));
        }
        Ok(())
    }

    /// The screen coverage of a sphere from a camera, before the bias: its radius as a share of half the screen height at its distance (a perspective
    /// camera) or of the view height (an orthographic one: distance does not matter).
    pub fn coverage(camera: &Camera, sphere: Sphere) -> f32 {
        match camera.projection {
            Projection::Perspective { fov_y, .. } => {
                let distance = (sphere.center - camera.position).length();
                let half_height_per_metre = (fov_y * 0.5).tan();
                sphere.radius / (distance * half_height_per_metre).max(f32::MIN_POSITIVE)
            }
            Projection::Orthographic { height, .. } => {
                sphere.radius / (height * 0.5).max(f32::MIN_POSITIVE)
            }
        }
    }

    /// What to draw. `previous` is what was drawn last frame (for the hysteresis); `None` for a thing seen the first time.
    pub fn select(
        &self,
        set: &LodSet,
        camera: &Camera,
        sphere: Sphere,
        previous: Option<LodChoice>,
    ) -> LodChoice {
        let distance = (sphere.center - camera.position).length();
        // A thing's own size does not move the far limit: it is about the centre, so a big monster does not vanish later than a small one at the same place.
        let was_culled = previous == Some(LodChoice::Culled);
        if let Some(max) = set.max_distance {
            let limit = max
                * self.view_distance
                * if was_culled {
                    1.0 - self.hysteresis
                } else {
                    1.0
                };
            if distance.is_nan() || distance > limit {
                return LodChoice::Culled;
            }
        }
        // Each +1 of bias judges the thing at half its coverage.
        let coverage = Self::coverage(camera, sphere) * (-self.lod_bias).exp2();
        // A sphere at the camera (coverage infinite) or with a NaN radius (coverage NaN) fails every "below" comparison, so it gets the finest level.
        let exact = set.rank(coverage, 1.0);
        let Some(prev) = previous else {
            return set.choice(exact);
        };
        // The finest level the thing may keep and the coarsest, with every threshold moved by the hysteresis; the previous level holds while inside.
        let finest = set.rank(coverage, 1.0 - self.hysteresis);
        let coarsest = set.rank(coverage, 1.0 + self.hysteresis);
        set.choice(set.to_rank(prev).clamp(finest, coarsest))
    }
}

/// How many things were given each choice in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LodStats {
    /// Things drawn as a model level, indexed by level (up to 8 levels counted; deeper ones are added to the last).
    pub levels: [u32; 8],
    /// Things drawn as an impostor.
    pub impostors: u32,
    /// Things not drawn.
    pub culled: u32,
}

/// Remembers the last choice of each thing, so the hysteresis has something to hold on to.
///
/// Call [`begin_frame`](Self::begin_frame), [`select`](Self::select) for each thing, then [`end_frame`](Self::end_frame): things not selected this frame
/// (gone from the world) are forgotten, so the memory does not grow.
#[derive(Debug, Default)]
pub struct LodTracker {
    last: BTreeMap<RenderId, LodChoice>,
    current: BTreeMap<RenderId, LodChoice>,
    stats: LodStats,
}

impl LodTracker {
    /// An empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a frame: the counts reset.
    pub fn begin_frame(&mut self) {
        self.current.clear();
        self.stats = LodStats::default();
    }

    /// Choose for one thing and remember it.
    pub fn select(
        &mut self,
        policy: &LodPolicy,
        set: &LodSet,
        id: RenderId,
        camera: &Camera,
        sphere: Sphere,
    ) -> LodChoice {
        let choice = policy.select(set, camera, sphere, self.last.get(&id).copied());
        match choice {
            LodChoice::Level(l) => self.stats.levels[usize::from(l).min(7)] += 1,
            LodChoice::Impostor => self.stats.impostors += 1,
            LodChoice::Culled => self.stats.culled += 1,
        }
        self.current.insert(id, choice);
        choice
    }

    /// End the frame: what was chosen becomes "last frame", and what was not chosen is forgotten. Returns the counts.
    pub fn end_frame(&mut self) -> LodStats {
        std::mem::swap(&mut self.last, &mut self.current);
        self.stats
    }

    /// How many things are remembered.
    pub fn tracked(&self) -> usize {
        self.last.len()
    }

    /// What a thing was given last frame.
    pub fn last_choice(&self, id: RenderId) -> Option<LodChoice> {
        self.last.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::math::Vec3;

    fn cam() -> Camera {
        Camera::default() // at the origin, 60 degrees vertical field of view, looking down -Z
    }

    /// A sphere of radius 1 `z` metres in front of the default camera. Its coverage is `1 / (z * tan 30 degrees)`.
    fn ahead(z: f32) -> Sphere {
        Sphere::new(Vec3::new(0.0, 0.0, -z), 1.0)
    }

    fn z_for(coverage: f32) -> f32 {
        1.0 / (coverage * (30f32.to_radians()).tan())
    }

    fn set3() -> LodSet {
        LodSet::new(vec![0.25, 0.08], Some(0.02), 0.005, None).unwrap()
    }

    fn pick(p: &LodPolicy, s: &LodSet, z: f32) -> LodChoice {
        p.select(s, &cam(), ahead(z), None)
    }

    // ---- coverage ------------------------------------------------------------------------------

    #[test]
    fn coverage_is_the_radius_over_half_the_screen_height_at_that_distance() {
        // At 1 / tan(30 deg) = 1.732 m a radius-1 sphere is exactly as tall as half the screen: coverage 1.
        let c = LodPolicy::coverage(&cam(), ahead(1.0 / (30f32.to_radians()).tan()));
        assert!((c - 1.0).abs() < 1e-5, "{c}");
        // Twice as far: half.
        let c2 = LodPolicy::coverage(&cam(), ahead(2.0 / (30f32.to_radians()).tan()));
        assert!((c2 - 0.5).abs() < 1e-5, "{c2}");
        // Direction does not matter, only distance (the frustum test is the culling's job).
        let side = LodPolicy::coverage(&cam(), Sphere::new(Vec3::new(3.0, 0.0, 0.0), 1.0));
        let front = LodPolicy::coverage(&cam(), Sphere::new(Vec3::new(0.0, 0.0, -3.0), 1.0));
        assert!((side - front).abs() < 1e-6);
        // A wider field of view makes things smaller on screen.
        let wide = Camera {
            projection: Projection::Perspective {
                fov_y: 100f32.to_radians(),
                near: 0.1,
            },
            ..cam()
        };
        assert!(LodPolicy::coverage(&wide, ahead(5.0)) < LodPolicy::coverage(&cam(), ahead(5.0)));
    }

    #[test]
    fn an_orthographic_camera_ignores_distance() {
        let ortho = Camera {
            projection: Projection::Orthographic {
                height: 10.0,
                near: 0.1,
                far: 100.0,
            },
            ..cam()
        };
        let near = LodPolicy::coverage(&ortho, ahead(2.0));
        let far = LodPolicy::coverage(&ortho, ahead(80.0));
        assert!((near - 0.2).abs() < 1e-6 && near == far, "{near} {far}");
    }

    // ---- choosing --------------------------------------------------------------------------------

    #[test]
    fn levels_impostor_and_culling_follow_the_thresholds_in_order() {
        let (p, s) = (LodPolicy::default(), set3());
        assert_eq!(pick(&p, &s, z_for(0.9)), LodChoice::Level(0));
        assert_eq!(pick(&p, &s, z_for(0.26)), LodChoice::Level(0));
        assert_eq!(pick(&p, &s, z_for(0.24)), LodChoice::Level(1));
        assert_eq!(pick(&p, &s, z_for(0.09)), LodChoice::Level(1));
        assert_eq!(pick(&p, &s, z_for(0.07)), LodChoice::Level(2));
        assert_eq!(pick(&p, &s, z_for(0.021)), LodChoice::Level(2));
        assert_eq!(pick(&p, &s, z_for(0.019)), LodChoice::Impostor);
        assert_eq!(pick(&p, &s, z_for(0.006)), LodChoice::Impostor);
        assert_eq!(pick(&p, &s, z_for(0.004)), LodChoice::Culled);
        assert_eq!(s.levels(), 3);
        assert!(s.has_impostor());
    }

    #[test]
    fn a_set_without_an_impostor_goes_from_the_last_level_to_culled() {
        let s = LodSet::new(vec![0.2], None, 0.01, None).unwrap();
        let p = LodPolicy::default();
        assert_eq!(pick(&p, &s, z_for(0.1)), LodChoice::Level(1));
        assert_eq!(pick(&p, &s, z_for(0.009)), LodChoice::Culled);
        assert!(!s.has_impostor());
    }

    #[test]
    fn a_single_level_model_is_drawn_until_it_is_too_small() {
        let s = LodSet::single(0.01).unwrap();
        let p = LodPolicy::default();
        assert_eq!(s.levels(), 1);
        assert_eq!(pick(&p, &s, z_for(0.5)), LodChoice::Level(0));
        assert_eq!(pick(&p, &s, z_for(0.011)), LodChoice::Level(0));
        assert_eq!(pick(&p, &s, z_for(0.009)), LodChoice::Culled);
        // A cull limit of 0 never culls by size.
        let always = LodSet::single(0.0).unwrap();
        assert_eq!(pick(&p, &always, 1.0e9), LodChoice::Level(0));
    }

    #[test]
    fn moving_away_never_gives_a_finer_choice() {
        let (p, s) = (
            LodPolicy {
                hysteresis: 0.0,
                ..LodPolicy::default()
            },
            set3(),
        );
        let rank = |c: LodChoice| match c {
            LodChoice::Level(l) => l as u32,
            LodChoice::Impostor => 3,
            LodChoice::Culled => 4,
        };
        let mut last = 0;
        let mut z = 0.5f32;
        let mut seen = [false; 5];
        while z < 5_000.0 {
            let r = rank(pick(&p, &s, z));
            assert!(r >= last, "at {z} m the choice got finer");
            seen[r as usize] = true;
            last = r;
            z *= 1.001;
        }
        assert!(seen.iter().all(|s| *s), "every choice is reached: {seen:?}");
    }

    #[test]
    fn a_bigger_thing_keeps_its_detail_farther_away() {
        let (p, s) = (LodPolicy::default(), set3());
        let at30 = |radius: f32| {
            p.select(
                &s,
                &cam(),
                Sphere::new(Vec3::new(0.0, 0.0, -30.0), radius),
                None,
            )
        };
        assert_eq!(at30(0.3), LodChoice::Impostor, "coverage 0.017");
        assert_eq!(at30(0.5), LodChoice::Level(2), "coverage 0.029");
        assert_eq!(at30(8.0), LodChoice::Level(0));
    }

    #[test]
    fn a_sphere_at_the_camera_is_the_finest_level_and_a_nan_position_does_not_panic() {
        let (p, s) = (LodPolicy::default(), set3());
        assert_eq!(
            p.select(&s, &cam(), Sphere::new(Vec3::ZERO, 1.0), None),
            LodChoice::Level(0)
        );
        let nan = p.select(
            &s,
            &cam(),
            Sphere::new(Vec3::new(f32::NAN, 0.0, 0.0), 1.0),
            None,
        );
        assert!(matches!(
            nan,
            LodChoice::Level(_) | LodChoice::Impostor | LodChoice::Culled
        ));
        let with_far = LodSet::new(vec![], None, 0.0, Some(50.0)).unwrap();
        assert_eq!(
            p.select(
                &with_far,
                &cam(),
                Sphere::new(Vec3::new(f32::NAN, 0.0, 0.0), 1.0),
                None
            ),
            LodChoice::Culled,
            "NaN is not within range"
        );
    }

    #[test]
    fn a_value_exactly_on_a_threshold_is_the_coarser_side_exactly() {
        // An orthographic view of height 2: coverage is exactly radius, so thresholds can be hit to the bit.
        let ortho = Camera {
            projection: Projection::Orthographic {
                height: 2.0,
                near: 0.1,
                far: 100.0,
            },
            ..cam()
        };
        let p = LodPolicy {
            hysteresis: 0.0,
            ..LodPolicy::default()
        };
        let s = LodSet::new(vec![0.5, 0.25], Some(0.125), 0.0625, None).unwrap();
        let at = |radius: f32| {
            p.select(
                &s,
                &ortho,
                Sphere::new(Vec3::new(0.0, 0.0, -5.0), radius),
                None,
            )
        };
        assert_eq!(
            at(0.5),
            LodChoice::Level(0),
            "exactly on the first switch is still level 0"
        );
        assert_eq!(at(0.4999), LodChoice::Level(1));
        assert_eq!(at(0.25), LodChoice::Level(1));
        assert_eq!(at(0.2499), LodChoice::Level(2));
        assert_eq!(
            at(0.125),
            LodChoice::Level(2),
            "exactly on the impostor edge is still the last level"
        );
        assert_eq!(at(0.1249), LodChoice::Impostor);
        assert_eq!(
            at(0.0625),
            LodChoice::Impostor,
            "exactly on the cull edge is still drawn"
        );
        assert_eq!(at(0.0624), LodChoice::Culled);
    }

    #[test]
    fn a_thing_exactly_at_the_maximum_distance_is_still_drawn() {
        let s = LodSet::new(vec![], None, 0.0, Some(100.0)).unwrap();
        let p = LodPolicy {
            hysteresis: 0.0,
            ..LodPolicy::default()
        };
        assert_eq!(
            p.select(
                &s,
                &cam(),
                Sphere::new(Vec3::new(0.0, 0.0, -100.0), 1.0),
                None
            ),
            LodChoice::Level(0)
        );
        assert_eq!(
            p.select(
                &s,
                &cam(),
                Sphere::new(Vec3::new(0.0, 0.0, -100.01), 1.0),
                None
            ),
            LodChoice::Culled
        );
    }

    #[test]
    fn a_set_has_at_most_255_levels() {
        let descending = |n: usize| {
            (1..=n)
                .rev()
                .map(|i| i as f32 / 1000.0)
                .collect::<Vec<f32>>()
        };
        assert!(LodSet::new(descending(254), None, 0.0, None).is_ok());
        assert!(LodSet::new(descending(255), None, 0.0, None).is_err());
    }

    // ---- quality -------------------------------------------------------------------------------

    #[test]
    fn each_step_of_bias_judges_a_thing_at_half_its_coverage() {
        let s = set3();
        let at = |bias: f32, coverage: f32| {
            LodPolicy::new(bias, 1.0, 0.0)
                .unwrap()
                .select(&s, &cam(), ahead(z_for(coverage)), None)
        };
        assert_eq!(at(0.0, 0.3), LodChoice::Level(0));
        assert_eq!(at(1.0, 0.3), LodChoice::Level(1), "0.3 judged as 0.15");
        assert_eq!(
            at(1.0, 0.45),
            LodChoice::Level(1),
            "0.45 is judged as 0.225, below 0.25"
        );
        assert_eq!(
            at(1.0, 0.55),
            LodChoice::Level(0),
            "0.55 is judged as 0.275, above it"
        );
        assert_eq!(
            at(-1.0, 0.15),
            LodChoice::Level(0),
            "a negative bias is finer: 0.15 is judged as 0.3"
        );
    }

    #[test]
    fn the_view_distance_scales_the_maximum_distance_and_culls_beyond_it() {
        let s = LodSet::new(vec![], None, 0.0, Some(100.0)).unwrap();
        let at = |view: f32, z: f32| {
            LodPolicy::new(0.0, view, 0.0)
                .unwrap()
                .select(&s, &cam(), ahead(z), None)
        };
        assert_eq!(at(1.0, 99.0), LodChoice::Level(0));
        assert_eq!(at(1.0, 101.0), LodChoice::Culled);
        assert_eq!(at(0.5, 51.0), LodChoice::Culled);
        assert_eq!(at(0.5, 49.0), LodChoice::Level(0));
        assert_eq!(at(2.0, 199.0), LodChoice::Level(0));
        assert_eq!(at(2.0, 201.0), LodChoice::Culled);
    }

    #[test]
    fn quality_settings_become_a_policy_and_a_lower_tier_draws_coarser() {
        use crate::quality::QualityTier;
        let low = LodPolicy::from_quality(&QualitySettings::preset(QualityTier::Low));
        let ultra = LodPolicy::from_quality(&QualitySettings::preset(QualityTier::Ultra));
        let preset = QualitySettings::preset(QualityTier::Ultra);
        assert_eq!(
            (ultra.lod_bias, ultra.view_distance),
            (preset.lod_bias, preset.view_distance)
        );
        assert!(low.validate().is_ok() && ultra.validate().is_ok());
        let s = LodSet::new(vec![0.25, 0.08], Some(0.02), 0.005, Some(500.0)).unwrap();
        let rank = |c: LodChoice| match c {
            LodChoice::Level(l) => l as u32,
            LodChoice::Impostor => 3,
            LodChoice::Culled => 4,
        };
        let mut z = 1.0f32;
        while z < 800.0 {
            assert!(
                rank(low.select(&s, &cam(), ahead(z), None))
                    >= rank(ultra.select(&s, &cam(), ahead(z), None)),
                "at {z} m low is not coarser than ultra"
            );
            z *= 1.05;
        }
        assert_eq!(
            low.select(&s, &cam(), ahead(400.0), None),
            LodChoice::Culled,
            "low: 300 m limit"
        );
        assert_ne!(
            ultra.select(&s, &cam(), ahead(400.0), None),
            LodChoice::Culled,
            "ultra: 750 m limit"
        );
    }

    // ---- hysteresis ------------------------------------------------------------------------------

    #[test]
    fn a_thing_on_a_threshold_does_not_flicker_but_a_real_move_switches() {
        let s = set3();
        let p = LodPolicy::default(); // 10 % hysteresis
        let mut prev = None;
        let mut changes = 0;
        let mut last = None;
        // Wobble around the 0.25 threshold by about 4 %: inside the 10 % band.
        for i in 0..200 {
            let coverage = 0.25 * (1.0 + 0.04 * ((i as f32) * 0.9).sin());
            let c = p.select(&s, &cam(), ahead(z_for(coverage)), prev);
            if last.is_some_and(|l| l != c) {
                changes += 1;
            }
            last = Some(c);
            prev = Some(c);
        }
        assert_eq!(changes, 0, "no flicker inside the band");
        // Without the hysteresis the same wobble flickers.
        let none = LodPolicy {
            hysteresis: 0.0,
            ..LodPolicy::default()
        };
        let (mut prev, mut changes, mut last) = (None, 0, None);
        for i in 0..200 {
            let coverage = 0.25 * (1.0 + 0.04 * ((i as f32) * 0.9).sin());
            let c = none.select(&s, &cam(), ahead(z_for(coverage)), prev);
            if last.is_some_and(|l| l != c) {
                changes += 1;
            }
            last = Some(c);
            prev = Some(c);
        }
        assert!(changes > 10, "the control flickers: {changes}");
        // Past the band it does switch, in both directions.
        let down = p.select(
            &s,
            &cam(),
            ahead(z_for(0.25 * 0.85)),
            Some(LodChoice::Level(0)),
        );
        assert_eq!(down, LodChoice::Level(1));
        let up = p.select(
            &s,
            &cam(),
            ahead(z_for(0.25 * 1.15)),
            Some(LodChoice::Level(1)),
        );
        assert_eq!(up, LodChoice::Level(0));
        // Just inside the band on each side it holds.
        assert_eq!(
            p.select(
                &s,
                &cam(),
                ahead(z_for(0.25 * 0.95)),
                Some(LodChoice::Level(0))
            ),
            LodChoice::Level(0)
        );
        assert_eq!(
            p.select(
                &s,
                &cam(),
                ahead(z_for(0.25 * 1.05)),
                Some(LodChoice::Level(1))
            ),
            LodChoice::Level(1)
        );
    }

    #[test]
    fn hysteresis_applies_to_the_impostor_and_to_culling_and_a_jump_skips_levels() {
        let s = set3();
        let p = LodPolicy::default();
        // Held as the impostor just above its upper edge (0.02): coverage 0.021 is level 2 exactly, impostor inside the band.
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.021)), Some(LodChoice::Impostor)),
            LodChoice::Impostor
        );
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.021)), Some(LodChoice::Level(2))),
            LodChoice::Level(2)
        );
        // A big jump goes straight to the right choice rather than one step at a time.
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.5)), Some(LodChoice::Culled)),
            LodChoice::Level(0)
        );
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.001)), Some(LodChoice::Level(0))),
            LodChoice::Culled
        );
        // A thing culled for being too small stays culled just above the cull edge (coverage 0.0052 against 0.005), and comes back as the impostor past the band.
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.0052)), Some(LodChoice::Culled)),
            LodChoice::Culled
        );
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.0052)), None),
            LodChoice::Impostor
        );
        assert_eq!(
            p.select(&s, &cam(), ahead(z_for(0.0056)), Some(LodChoice::Culled)),
            LodChoice::Impostor
        );
        // A thing that was culled by distance comes back only inside the limit minus the band.
        let far = LodSet::new(vec![], None, 0.0, Some(100.0)).unwrap();
        assert_eq!(
            p.select(&far, &cam(), ahead(95.0), Some(LodChoice::Culled)),
            LodChoice::Culled
        );
        assert_eq!(
            p.select(&far, &cam(), ahead(89.0), Some(LodChoice::Culled)),
            LodChoice::Level(0)
        );
        assert_eq!(
            p.select(&far, &cam(), ahead(99.0), Some(LodChoice::Level(0))),
            LodChoice::Level(0)
        );
    }

    #[test]
    fn a_remembered_choice_that_the_set_does_not_have_is_mapped_not_trusted() {
        // The set changed (a model without an impostor replaced one with): a remembered Impostor or a high level must not panic or index out of range.
        let one = LodSet::single(0.01).unwrap();
        let p = LodPolicy::default();
        for prev in [
            LodChoice::Impostor,
            LodChoice::Level(7),
            LodChoice::Culled,
            LodChoice::Level(0),
        ] {
            let c = p.select(&one, &cam(), ahead(z_for(0.5)), Some(prev));
            assert_eq!(c, LodChoice::Level(0), "{prev:?}");
        }
    }

    // ---- the tracker -----------------------------------------------------------------------------

    #[test]
    fn the_tracker_remembers_each_thing_counts_and_forgets_what_left() {
        let s = set3();
        let p = LodPolicy::default();
        let mut t = LodTracker::new();
        t.begin_frame();
        assert_eq!(
            t.select(&p, &s, RenderId(1), &cam(), ahead(z_for(0.9))),
            LodChoice::Level(0)
        );
        assert_eq!(
            t.select(&p, &s, RenderId(2), &cam(), ahead(z_for(0.05))),
            LodChoice::Level(2)
        );
        assert_eq!(
            t.select(&p, &s, RenderId(3), &cam(), ahead(z_for(0.01))),
            LodChoice::Impostor
        );
        assert_eq!(
            t.select(&p, &s, RenderId(4), &cam(), ahead(z_for(0.001))),
            LodChoice::Culled
        );
        let stats = t.end_frame();
        assert_eq!(
            (
                stats.levels[0],
                stats.levels[1],
                stats.levels[2],
                stats.impostors,
                stats.culled
            ),
            (1, 0, 1, 1, 1)
        );
        assert_eq!(t.tracked(), 4);
        assert_eq!(t.last_choice(RenderId(3)), Some(LodChoice::Impostor));
        // Next frame: thing 1 sits just past its threshold: it holds by hysteresis; things 2 to 4 are gone.
        t.begin_frame();
        assert_eq!(
            t.select(&p, &s, RenderId(1), &cam(), ahead(z_for(0.24))),
            LodChoice::Level(0),
            "held"
        );
        // A thing seen for the first time has no memory: the same coverage gives level 1.
        assert_eq!(
            t.select(&p, &s, RenderId(9), &cam(), ahead(z_for(0.24))),
            LodChoice::Level(1)
        );
        let stats = t.end_frame();
        assert_eq!((stats.levels[0], stats.levels[1]), (1, 1));
        assert_eq!(
            t.tracked(),
            2,
            "the things that were not selected were forgotten"
        );
        assert_eq!(t.last_choice(RenderId(2)), None);
        // The counts start again every frame.
        t.begin_frame();
        assert_eq!(t.end_frame(), LodStats::default());
        assert_eq!(t.tracked(), 0);
    }

    // ---- bad data ---------------------------------------------------------------------------------

    #[test]
    fn bad_sets_and_policies_are_refused() {
        for (switch, imp, cull, max) in [
            (vec![0.1, 0.2], None, 0.01, None),  // ascending
            (vec![0.2, 0.2], None, 0.01, None),  // equal
            (vec![0.2], Some(0.3), 0.01, None),  // impostor above the last switch
            (vec![0.2], Some(0.01), 0.01, None), // cull equal to the impostor
            (vec![0.2], None, 0.2, None),        // cull equal to the last switch
            (vec![0.2], Some(0.05), 0.06, None), // cull above the impostor
            (vec![f32::NAN], None, 0.0, None),
            (vec![0.0], None, 0.0, None),
            (vec![-0.1], None, 0.0, None),
            (vec![0.2], None, f32::NAN, None),
            (vec![0.2], None, -0.1, None),
            (vec![0.2], None, 0.01, Some(0.0)),
            (vec![0.2], None, 0.01, Some(f32::INFINITY)),
        ] {
            assert!(
                LodSet::new(switch.clone(), imp, cull, max).is_err(),
                "{switch:?} {imp:?} {cull} {max:?}"
            );
        }
        assert!(LodSet::new(vec![], None, 0.0, None).is_ok());
        assert!(LodSet::single(f32::NAN).is_err());
        for (b, v, h) in [
            (-1.1, 1.0, 0.1),
            (3.1, 1.0, 0.1),
            (f32::NAN, 1.0, 0.1),
            (0.0, 0.4, 0.1),
            (0.0, 2.1, 0.1),
            (0.0, 1.0, -0.1),
            (0.0, 1.0, 0.6),
            (0.0, f32::NAN, 0.1),
        ] {
            assert!(LodPolicy::new(b, v, h).is_err(), "{b} {v} {h}");
        }
        assert!(LodPolicy::new(-1.0, 0.5, 0.0).is_ok() && LodPolicy::new(3.0, 2.0, 0.5).is_ok());
        assert!(LodError("x").to_string() == "x");
    }
}
