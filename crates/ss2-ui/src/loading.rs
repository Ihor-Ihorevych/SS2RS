//! The model behind a **loading screen**: weighted stages with a progress bar that never goes backwards, an estimate of the time left, the current status
//! line, and tips that rotate without repeating (roadmap T10-009, `docs/LOADING.md`).
//!
//! No drawing and no clock: the caller reports work done ([`LoadingTracker::advance`]) and the time that passed ([`LoadingTracker::update`]), and reads what to
//! show ([`LoadingTracker::displayed`], [`LoadingTracker::status`], [`LoadingTracker::eta_seconds`]). Stages can be anything that takes time: reading archives,
//! building the world, uploading textures, **compiling shaders and warming the pipeline cache**.

/// The most stages a tracker holds.
pub const MAX_STAGES: usize = 32;

/// How fast the bar may move, as a share of the whole bar per second: a jump from 10 % to 90 % takes about half a second instead of one frame.
pub const DISPLAY_SPEED: f32 = 1.5;

/// How long (seconds) to watch before an estimate of the time left is shown.
pub const ETA_MIN_SECONDS: f32 = 2.0;

/// A stage of the load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageId(usize);

/// Why a stage was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingError {
    /// The weight was zero, negative or not a number.
    BadWeight,
    /// More than [`MAX_STAGES`] stages.
    TooManyStages,
}

impl std::fmt::Display for LoadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadWeight => f.write_str("a stage's weight must be a positive number"),
            Self::TooManyStages => write!(f, "more than {MAX_STAGES} stages"),
        }
    }
}

impl std::error::Error for LoadingError {}

#[derive(Clone, Debug)]
struct Stage {
    name: String,
    weight: f32,
    /// 0 until known.
    total: u64,
    done: u64,
    finished: bool,
}

impl Stage {
    fn fraction(&self) -> f32 {
        if self.finished {
            1.0
        } else if self.total == 0 {
            0.0
        } else {
            (self.done.min(self.total) as f64 / self.total as f64) as f32
        }
    }
}

/// Progress of a load made of weighted stages.
///
/// ```
/// use ss2_ui::loading::LoadingTracker;
///
/// let mut load = LoadingTracker::new();
/// let world = load.add_stage("Building the world", 3.0).unwrap();
/// let shaders = load.add_stage("Compiling shaders", 1.0).unwrap();
/// load.set_total(world, 200);
/// load.advance(world, 100); // half of the heavy stage, which is 3 of the 4 weights: 3 / 4 x 0.5
/// assert!((load.fraction() - 0.375).abs() < 1e-6);
/// load.finish(world);
/// load.finish(shaders);
/// assert_eq!(load.fraction(), 1.0);
/// ```
#[derive(Clone, Debug, Default)]
pub struct LoadingTracker {
    stages: Vec<Stage>,
    displayed: f32,
    elapsed: f32,
    rate: f32,
    last_fraction: f32,
}

impl LoadingTracker {
    /// A tracker with no stages.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a stage whose share of the bar is `weight` against the others. Stages are worked through in the order added (the one shown in the status line is the first
    /// that is not finished).
    pub fn add_stage(&mut self, name: &str, weight: f32) -> Result<StageId, LoadingError> {
        if !(weight.is_finite() && weight > 0.0) {
            return Err(LoadingError::BadWeight);
        }
        if self.stages.len() >= MAX_STAGES {
            return Err(LoadingError::TooManyStages);
        }
        self.stages.push(Stage {
            name: name.to_owned(),
            weight,
            total: 0,
            done: 0,
            finished: false,
        });
        Ok(StageId(self.stages.len() - 1))
    }

    /// How much work the stage has (files, bytes, shaders): until it is known the stage counts as 0 % done. It may grow as work is discovered.
    pub fn set_total(&mut self, stage: StageId, total: u64) {
        if let Some(s) = self.stages.get_mut(stage.0) {
            s.total = total;
        }
    }

    /// Report `units` more done. Saturates; going past the total counts as complete.
    pub fn advance(&mut self, stage: StageId, units: u64) {
        if let Some(s) = self.stages.get_mut(stage.0) {
            s.done = s.done.saturating_add(units);
        }
    }

    /// The stage is complete, whatever its counts say (a stage with nothing to do, a failure that is not fatal).
    pub fn finish(&mut self, stage: StageId) {
        if let Some(s) = self.stages.get_mut(stage.0) {
            s.finished = true;
        }
    }

    /// The true progress, 0 to 1: the weighted mean of the stages. With no stages it is 0. It can go down if a stage discovers more work; [`displayed`](Self::displayed) does not.
    pub fn fraction(&self) -> f32 {
        let total_weight: f32 = self.stages.iter().map(|s| s.weight).sum();
        if self.stages.is_empty() || total_weight <= 0.0 {
            return 0.0;
        }
        // Each term is at most its weight and floating-point addition is monotonic, so the sum never exceeds the total weight: no clamp is needed,
        // and with every stage finished the two sums are the same number, so the result is exactly 1.
        let sum: f32 = self.stages.iter().map(|s| s.weight * s.fraction()).sum();
        sum / total_weight
    }

    /// Advance time by `dt` seconds (a `dt` that is not positive and finite is ignored): the displayed bar chases the true progress at most [`DISPLAY_SPEED`] a second
    /// and never moves back, and the rate used for the estimate is updated.
    pub fn update(&mut self, dt: f32) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let f = self.fraction();
        self.elapsed += dt;
        let instant = (f - self.last_fraction) / dt;
        // Smooth the speed, and ignore a drop in the true progress (more work discovered) rather than reading it as negative speed.
        self.rate = 0.9 * self.rate + 0.1 * instant.max(0.0);
        self.last_fraction = f;
        let target = f.max(self.displayed);
        self.displayed = (self.displayed + DISPLAY_SPEED * dt).min(target);
    }

    /// The bar to draw, 0 to 1.
    pub fn displayed(&self) -> f32 {
        self.displayed
    }

    /// Whether everything is finished **and** the bar has reached the end, so the screen can close without cutting off the last of the animation.
    pub fn is_done(&self) -> bool {
        !self.stages.is_empty()
            && self.stages.iter().all(|s| s.finished)
            && self.displayed >= 1.0 - 1e-6
    }

    /// An estimate of the seconds left, from the smoothed speed; `None` until [`ETA_MIN_SECONDS`] have passed, while nothing is moving, or when the work is done.
    pub fn eta_seconds(&self) -> Option<f32> {
        let f = self.fraction();
        if self.elapsed < ETA_MIN_SECONDS || self.rate < 1e-4 || f >= 1.0 {
            return None;
        }
        Some((1.0 - f) / self.rate)
    }

    /// Seconds since the first update.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// The status line: the first stage that is not finished and how far it is (`"Building the world: 120 / 200"`, or just its name while its size is not known).
    /// `None` when every stage is finished or there are none.
    pub fn status(&self) -> Option<String> {
        let s = self.stages.iter().find(|s| !s.finished)?;
        Some(if s.total > 0 {
            format!("{}: {} / {}", s.name, s.done.min(s.total), s.total)
        } else {
            s.name.clone()
        })
    }
}

/// Tips shown while loading: each once before any repeats, in an order fixed by the seed, changing every `seconds`.
///
/// ```
/// use ss2_ui::loading::TipRotator;
///
/// let tips = ["Reload behind cover.", "Secrets hide in vents.", "Save often."].map(String::from).to_vec();
/// let rotator = TipRotator::new(tips, 5.0, 42);
/// let first_cycle: Vec<&str> = (0..3).map(|i| rotator.tip_at(i as f32 * 5.0).unwrap()).collect();
/// let mut sorted = first_cycle.clone();
/// sorted.sort_unstable();
/// sorted.dedup();
/// assert_eq!(sorted.len(), 3); // every tip once before any repeats
/// ```
#[derive(Clone, Debug)]
pub struct TipRotator {
    tips: Vec<String>,
    order: Vec<usize>,
    seconds: f32,
}

impl TipRotator {
    /// A rotator over `tips`, changing every `seconds` (limited to at least 1; a value that is not a number counts as 5). The order is a shuffle fixed by `seed`.
    pub fn new(tips: Vec<String>, seconds: f32, seed: u64) -> Self {
        let mut order: Vec<usize> = (0..tips.len()).collect();
        let mut state = seed | 1; // xorshift needs a non-zero state
        for i in (1..order.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            order.swap(i, (state % (i as u64 + 1)) as usize);
        }
        let seconds = if seconds.is_nan() {
            5.0
        } else {
            seconds.max(1.0)
        };
        Self {
            tips,
            order,
            seconds,
        }
    }

    /// The tip to show `elapsed` seconds in (a negative or non-finite time shows the first); `None` if there are no tips.
    pub fn tip_at(&self, elapsed: f32) -> Option<&str> {
        if self.tips.is_empty() {
            return None;
        }
        let step = if elapsed.is_finite() && elapsed > 0.0 {
            (elapsed / self.seconds) as usize
        } else {
            0
        };
        Some(&self.tips[self.order[step % self.order.len()]])
    }

    /// How many tips there are.
    pub fn len(&self) -> usize {
        self.tips.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.tips.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn two_stages() -> (LoadingTracker, StageId, StageId) {
        let mut t = LoadingTracker::new();
        let a = t.add_stage("Archives", 1.0).unwrap();
        let b = t.add_stage("World", 3.0).unwrap();
        (t, a, b)
    }

    // ---- fraction -----------------------------------------------------------------------------

    #[test]
    fn progress_is_the_weighted_mean_of_the_stages() {
        let (mut t, a, b) = two_stages();
        assert_eq!(t.fraction(), 0.0);
        t.set_total(a, 10);
        t.set_total(b, 100);
        t.advance(a, 10); // the light stage done: 1/4
        assert!(close(t.fraction(), 0.25));
        t.advance(b, 50); // half of the heavy one: + 3/4 x 0.5
        assert!(close(t.fraction(), 0.25 + 0.375));
        t.finish(b);
        assert!(
            close(t.fraction(), 1.0),
            "the first stage is also complete by its counts: {}",
            t.fraction()
        );
    }

    #[test]
    fn a_stage_with_an_unknown_size_counts_as_nothing_done_and_work_past_the_total_as_complete() {
        let (mut t, a, _) = two_stages();
        t.advance(a, 5);
        assert_eq!(t.fraction(), 0.0, "no total yet");
        t.set_total(a, 4);
        assert!(
            close(t.fraction(), 0.25),
            "5 of 4 is complete: the weight of the first stage"
        );
        t.advance(a, u64::MAX);
        assert!(close(t.fraction(), 0.25), "saturating");
    }

    #[test]
    fn finishing_every_stage_gives_exactly_one_and_finishing_one_stage_fills_its_share() {
        let (mut t, a, b) = two_stages();
        t.finish(a);
        assert!(close(t.fraction(), 0.25));
        t.finish(b);
        assert_eq!(t.fraction(), 1.0);
        // An empty tracker is not done and shows nothing.
        let empty = LoadingTracker::new();
        assert_eq!(
            (empty.fraction(), empty.is_done(), empty.status()),
            (0.0, false, None)
        );
    }

    #[test]
    fn bad_stages_are_refused_and_unknown_ids_are_ignored() {
        let mut t = LoadingTracker::new();
        for w in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(t.add_stage("x", w), Err(LoadingError::BadWeight), "{w}");
        }
        for _ in 0..MAX_STAGES {
            t.add_stage("s", 1.0).unwrap();
        }
        assert_eq!(
            t.add_stage("one more", 1.0),
            Err(LoadingError::TooManyStages)
        );
        assert!(
            LoadingError::BadWeight.to_string().contains("weight")
                && LoadingError::TooManyStages.to_string().contains("32")
        );
        // Ids from another tracker point past the end here, or at someone else's stage: they must not panic.
        let ghost = StageId(999);
        t.set_total(ghost, 5);
        t.advance(ghost, 5);
        t.finish(ghost);
        assert_eq!(t.fraction(), 0.0);
    }

    // ---- the displayed bar --------------------------------------------------------------------------------

    #[test]
    fn the_bar_chases_the_progress_without_jumping_or_going_back() {
        let (mut t, a, b) = two_stages();
        t.set_total(a, 10);
        t.set_total(b, 10);
        t.finish(a);
        t.update(0.1); // true progress is 0.25, but the bar may move only 0.15 in 0.1 s
        assert!(close(t.displayed(), 0.15));
        t.update(0.1);
        assert!(
            close(t.displayed(), 0.25),
            "it has caught up and stops there"
        );
        t.advance(b, 10);
        for _ in 0..3 {
            t.update(0.1);
        }
        assert!(t.displayed() > 0.25 && t.displayed() < 1.0);
        // Work is discovered late: the true progress drops, the bar stays.
        let before = t.displayed();
        t.set_total(b, 1000);
        assert!(t.fraction() < before);
        t.update(0.1);
        assert_eq!(t.displayed(), before, "never backwards");
    }

    #[test]
    fn the_screen_is_done_only_when_everything_is_finished_and_the_bar_is_full() {
        let (mut t, a, b) = two_stages();
        t.finish(a);
        t.finish(b);
        assert!(!t.is_done(), "the bar has not moved yet");
        t.update(0.1);
        assert!(!t.is_done());
        t.update(1.0);
        assert!(t.is_done());
        assert_eq!(t.displayed(), 1.0);
    }

    #[test]
    fn bad_time_steps_are_ignored() {
        let (mut t, a, _) = two_stages();
        t.finish(a);
        for bad in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            t.update(bad);
        }
        assert_eq!((t.displayed(), t.elapsed()), (0.0, 0.0));
    }

    // ---- the estimate ------------------------------------------------------------------------------------------

    #[test]
    fn the_estimate_waits_two_seconds_then_follows_the_speed() {
        let mut t = LoadingTracker::new();
        let s = t.add_stage("Work", 1.0).unwrap();
        t.set_total(s, 1000);
        // 10 % a second, steady.
        let mut etas = Vec::new();
        for i in 1..=100 {
            t.advance(s, 10); // 1 % per 0.1 s
            t.update(0.1);
            if i == 10 || i == 19 {
                etas.push(t.eta_seconds());
            }
            if i == 50 {
                let eta = t.eta_seconds().unwrap();
                assert!((eta - 5.0).abs() < 0.6, "5 s to go at 50 %, said {eta}");
            }
        }
        assert_eq!(etas[0], None, "after 1 s");
        assert_eq!(etas[1], None, "after 1.9 s");
        assert_eq!(t.eta_seconds(), None, "complete: nothing left to estimate");
    }

    #[test]
    fn the_estimate_is_smoothed_and_survives_work_being_discovered_late() {
        let mut t = LoadingTracker::new();
        let s = t.add_stage("Work", 1.0).unwrap();
        t.set_total(s, 1000);
        for _ in 0..21 {
            t.advance(s, 10);
            t.update(0.1);
        }
        // 2.1 s in at 10 % a second: the smoothed speed is 0.1 x (1 - 0.9^21) = 0.0891, so 79 % left takes 8.9 s (the true answer is 7.9: the speed ramps up).
        let early = t.eta_seconds().unwrap();
        assert!((8.5..9.3).contains(&early), "{early}");
        // Ten times the work turns up: the progress collapses from 21 % to 1 %, and that must not be read as a large negative speed: ten steps later there is still an estimate.
        t.set_total(s, 20_000);
        for _ in 0..10 {
            t.advance(s, 10);
            t.update(0.1);
        }
        let later = t.eta_seconds();
        assert!(later.is_some_and(|e| e > 0.0 && e.is_finite()), "{later:?}");
    }

    #[test]
    fn no_estimate_while_nothing_moves() {
        let mut t = LoadingTracker::new();
        let s = t.add_stage("Work", 1.0).unwrap();
        t.set_total(s, 100);
        for _ in 0..50 {
            t.update(0.1);
        }
        assert_eq!(
            t.eta_seconds(),
            None,
            "5 s passed but no progress: no speed to extrapolate"
        );
        // A drop in the true progress is not read as negative speed.
        t.advance(s, 50);
        t.update(0.1);
        t.set_total(s, 100_000);
        for _ in 0..10 {
            t.update(0.1);
        }
        assert!(t.eta_seconds().is_none_or(|e| e > 0.0));
    }

    // ---- status ------------------------------------------------------------------------------------------------------

    #[test]
    fn the_status_names_the_first_unfinished_stage_and_its_counts() {
        let (mut t, a, b) = two_stages();
        assert_eq!(
            t.status().as_deref(),
            Some("Archives"),
            "size unknown: just the name"
        );
        t.set_total(a, 40);
        t.advance(a, 12);
        assert_eq!(t.status().as_deref(), Some("Archives: 12 / 40"));
        t.advance(a, 1000);
        assert_eq!(
            t.status().as_deref(),
            Some("Archives: 40 / 40"),
            "never shows more than the total"
        );
        t.finish(a);
        assert_eq!(t.status().as_deref(), Some("World"));
        t.set_total(b, 7);
        assert_eq!(t.status().as_deref(), Some("World: 0 / 7"));
        t.finish(b);
        assert_eq!(t.status(), None);
    }

    // ---- tips -------------------------------------------------------------------------------------------------------

    fn tips(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("tip {i}")).collect()
    }

    #[test]
    fn every_tip_is_shown_once_before_any_repeats_and_the_cycle_then_starts_again() {
        let r = TipRotator::new(tips(7), 5.0, 1234);
        let cycle: Vec<&str> = (0..7)
            .map(|i| r.tip_at(i as f32 * 5.0 + 0.1).unwrap())
            .collect();
        let mut sorted = cycle.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 7);
        let second: Vec<&str> = (7..14)
            .map(|i| r.tip_at(i as f32 * 5.0 + 0.1).unwrap())
            .collect();
        assert_eq!(cycle, second);
        assert_eq!((r.len(), r.is_empty()), (7, false));
    }

    #[test]
    fn a_tip_stays_for_its_whole_interval_and_the_order_depends_on_the_seed() {
        let order = |seed: u64| -> Vec<String> {
            let r = TipRotator::new(tips(5), 4.0, seed);
            (0..5)
                .map(|i| r.tip_at(i as f32 * 4.0).unwrap().to_owned())
                .collect()
        };
        let r = TipRotator::new(tips(5), 4.0, 7);
        assert_eq!(r.tip_at(0.0), r.tip_at(3.99));
        assert_ne!(r.tip_at(3.99), r.tip_at(4.0));
        let a = order(7);
        assert_eq!(a, order(7), "the same seed gives the same order");
        assert!(
            (0..20u64).any(|seed| order(seed) != a),
            "a different seed can give a different order"
        );
    }

    #[test]
    fn the_shuffle_can_leave_a_tip_where_it_was_so_it_is_not_a_forced_rotation() {
        // A shuffle that cannot leave any element in place (a cyclic permutation) would never show tip 2 third, say. Over many seeds some order has a fixed point.
        let has_fixed_point = |seed: u64| {
            let r = TipRotator::new(tips(5), 1.0, seed);
            (0..5).any(|i| r.tip_at(i as f32) == Some(format!("tip {i}").as_str()))
        };
        assert!((0..200u64).any(has_fixed_point));
        // And some order is not a rotation of the original either.
        let all_distinct = (0..50u64).all(|seed| {
            let r = TipRotator::new(tips(5), 1.0, seed);
            let mut seen: Vec<&str> = (0..5).map(|i| r.tip_at(i as f32).unwrap()).collect();
            seen.sort_unstable();
            seen.dedup();
            seen.len() == 5
        });
        assert!(all_distinct, "every seed still gives each tip exactly once");
    }

    #[test]
    fn odd_inputs_give_sensible_tips() {
        let none = TipRotator::new(Vec::new(), 5.0, 1);
        assert_eq!((none.tip_at(3.0), none.is_empty()), (None, true));
        let one = TipRotator::new(tips(1), 5.0, 1);
        assert_eq!(one.tip_at(1000.0), Some("tip 0"));
        let r = TipRotator::new(tips(4), 5.0, 9); // 4 tips: usize::MAX is not a multiple of 4, so a bad time that saturated would land on another tip
        let first = r.tip_at(0.0);
        for bad in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(r.tip_at(bad), first, "{bad}");
        }
        // An interval below 1 s is raised to 1 s (a flickering tip is unreadable); NaN counts as 5 s.
        let fast = TipRotator::new(tips(4), 0.01, 9); // 99 steps of 0.01 s would be tip 99 % 4 = 3 against tip 0
        assert_eq!(fast.tip_at(0.0), fast.tip_at(0.99));
        assert_ne!(fast.tip_at(0.99), fast.tip_at(1.0));
        let nan = TipRotator::new(tips(3), f32::NAN, 9);
        assert_eq!(nan.tip_at(4.99), nan.tip_at(0.0));
        assert_ne!(nan.tip_at(5.0), nan.tip_at(0.0));
        // Seed 0 is fine, and behaves as seed 1 (a zero state would make the shuffle degenerate).
        let order = |seed: u64| -> Vec<String> {
            let r = TipRotator::new(tips(6), 1.0, seed);
            (0..6)
                .map(|i| r.tip_at(i as f32).unwrap().to_owned())
                .collect()
        };
        assert_eq!(order(0), order(1));
    }
}
