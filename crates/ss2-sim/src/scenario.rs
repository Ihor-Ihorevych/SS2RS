//! A **scenario harness** for the headless simulation (roadmap T09-029, `docs/SCENARIOS.md`): set up a world and its systems, run it tick by tick without a window or a clock,
//! record numbers while it runs, and ask questions of the result ("how many ticks until the enemy is dead", "how much health was lost", "did two runs end the same").
//!
//! * [`Scenario::new`] wraps a [`Simulation`] with the clock **paused**, so every step is exactly one tick of `1 / tick_hz` seconds whatever the machine does.
//! * [`probe`](Scenario::probe) records a number from the world after every tick; [`Series`] answers questions about it (peak, total drop, the first tick a condition held).
//! * [`run_until`](Scenario::run_until) runs until a condition on the world holds or a tick limit is reached: the shape of **time-to-kill** and **path reached** checks.
//! * With a [`World`], [`Scenario::with_lifecycle_log`] also records every spawn and despawn with its tick.
//! * [`assert_deterministic`] runs a scenario twice from scratch and compares a hash of the final world.
//!
//! Nothing here knows what a weapon or an enemy is: the gameplay systems that will exist (T09) are added as ordinary systems and tested through this.

use crate::schedule::{Phase, ScheduleError, Simulation, TickContext};
use crate::tick::{TickConfig, TickConfigError};
use crate::world::{LifecycleEvent, World};

/// A series of numbers recorded after each tick (index 0 is before the first tick).
#[derive(Clone, Debug, PartialEq)]
pub struct Series(Vec<f64>);

impl Series {
    /// The recorded values.
    pub fn values(&self) -> &[f64] {
        &self.0
    }

    /// How many values.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing was recorded.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The first value (before any tick).
    pub fn first(&self) -> Option<f64> {
        self.0.first().copied()
    }

    /// The latest value.
    pub fn last(&self) -> Option<f64> {
        self.0.last().copied()
    }

    /// The largest value; `None` for an empty series. NaN values are ignored.
    pub fn peak(&self) -> Option<f64> {
        self.0
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .reduce(f64::max)
    }

    /// The smallest value; `None` for an empty series. NaN values are ignored.
    pub fn trough(&self) -> Option<f64> {
        self.0
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .reduce(f64::min)
    }

    /// The sum of all the drops from one sample to the next (always `>= 0`): **damage taken** when the series is a health. Rises do not cancel drops.
    pub fn total_drop(&self) -> f64 {
        self.0.windows(2).map(|w| (w[0] - w[1]).max(0.0)).sum()
    }

    /// The sum of all the rises from one sample to the next (always `>= 0`): **healing**, or **score** gained.
    pub fn total_rise(&self) -> f64 {
        self.0.windows(2).map(|w| (w[1] - w[0]).max(0.0)).sum()
    }

    /// The index (the tick count) of the first value for which `pred` holds: **time-to-kill** is `first_tick_where(|h| h <= 0.0)` on a health series.
    pub fn first_tick_where(&self, pred: impl Fn(f64) -> bool) -> Option<usize> {
        self.0.iter().position(|v| pred(*v))
    }
}

/// How a [`Scenario::run_until`] ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The condition held after this many ticks of the call (0 if it held at once).
    Reached(u64),
    /// The limit was reached first, after this many ticks.
    TimedOut(u64),
}

impl Outcome {
    /// The ticks it took, if the condition was reached.
    pub fn reached(self) -> Option<u64> {
        match self {
            Outcome::Reached(t) => Some(t),
            Outcome::TimedOut(_) => None,
        }
    }
}

/// Why a probe was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenarioError {
    /// A probe with this name exists.
    DuplicateProbe(String),
    /// An empty probe name.
    EmptyName,
    /// An action scheduled for a tick that has already run.
    InThePast {
        /// The tick asked for.
        tick: u64,
        /// How many ticks have run.
        ticks_run: u64,
    },
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateProbe(n) => write!(f, "there is already a probe called {n:?}"),
            Self::EmptyName => f.write_str("a probe needs a name"),
            Self::InThePast { tick, ticks_run } => {
                write!(f, "tick {tick} has already run ({ticks_run} ticks so far)")
            }
        }
    }
}

impl std::error::Error for ScenarioError {}

struct Probe<W> {
    name: String,
    read: Box<dyn Fn(&W) -> f64>,
    samples: Vec<f64>,
}

type LifecycleCollector<W> = fn(&mut W, u64, &mut Vec<(u64, LifecycleEvent)>);

type Action<W> = Box<dyn FnOnce(&mut W)>;

/// A simulation to run in a test; see the module documentation.
///
/// ```
/// use ss2_sim::scenario::Scenario;
/// use ss2_sim::{Phase, TickConfig};
///
/// // A health that drains 10 per second, in a world that is just a number.
/// let mut s = Scenario::new(TickConfig::default(), 100.0_f32).unwrap(); // 30 ticks a second
/// s.system(Phase::Damage, "drain", 0, |health, ctx| *health -= 10.0 * ctx.dt_seconds()).unwrap();
/// s.probe("health", |h| f64::from(*h)).unwrap();
/// let outcome = s.run_until(10_000, |h| *h <= 0.0);
/// let ticks = outcome.reached().unwrap();
/// assert!((299..=301).contains(&ticks), "ten seconds is about 300 ticks: {ticks}");
/// assert!((s.seconds_of(ticks) - 10.0).abs() < 0.1);
/// assert!(s.series("health").unwrap().total_drop() > 99.0);
/// ```
pub struct Scenario<W> {
    sim: Simulation<W>,
    probes: Vec<Probe<W>>,
    lifecycle: Vec<(u64, LifecycleEvent)>,
    collector: Option<LifecycleCollector<W>>,
    /// Scripted actions: the tick they run before, and the action, in the order they were added.
    script: Vec<(u64, Action<W>)>,
    ticks: u64,
}

impl<W> Scenario<W> {
    /// A scenario at tick 0. The clock is **paused** so that [`step`](Self::step) is exactly one tick.
    pub fn new(config: TickConfig, world: W) -> Result<Self, TickConfigError> {
        let mut sim = Simulation::new(config, world)?;
        sim.clock_mut().pause();
        Ok(Self {
            sim,
            probes: Vec::new(),
            lifecycle: Vec::new(),
            collector: None,
            script: Vec::new(),
            ticks: 0,
        })
    }

    /// Add a system (see [`Schedule::add`](crate::Schedule::add)).
    pub fn system(
        &mut self,
        phase: Phase,
        name: &str,
        order: i32,
        system: impl FnMut(&mut W, &TickContext) + 'static,
    ) -> Result<(), ScheduleError> {
        self.sim.schedule_mut().add(phase, name, order, system)
    }

    /// The world.
    pub fn world(&self) -> &W {
        self.sim.world()
    }

    /// The world, to set up between steps (spawn the player, place the enemy).
    pub fn world_mut(&mut self) -> &mut W {
        self.sim.world_mut()
    }

    /// The simulation underneath (to switch systems, read the clock).
    pub fn simulation(&mut self) -> &mut Simulation<W> {
        &mut self.sim
    }

    /// How many ticks have run.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Simulated seconds for `ticks` ticks at this scenario's rate.
    pub fn seconds_of(&self, ticks: u64) -> f64 {
        ticks as f64 / f64::from(self.sim.clock().config().tick_hz)
    }

    /// Simulated seconds elapsed.
    pub fn seconds(&self) -> f64 {
        self.seconds_of(self.ticks)
    }

    /// Record a number from the world: once now (index 0, before any tick) and after every tick from here on.
    ///
    /// # Errors
    /// [`ScenarioError`] for an empty or repeated name.
    pub fn probe(
        &mut self,
        name: &str,
        read: impl Fn(&W) -> f64 + 'static,
    ) -> Result<(), ScenarioError> {
        if name.is_empty() {
            return Err(ScenarioError::EmptyName);
        }
        if self.probes.iter().any(|p| p.name == name) {
            return Err(ScenarioError::DuplicateProbe(name.to_owned()));
        }
        let first = read(self.sim.world());
        self.probes.push(Probe {
            name: name.to_owned(),
            read: Box::new(read),
            samples: vec![first],
        });
        Ok(())
    }

    /// Script a change to the world: `action` runs **before the systems of tick `tick`** (the first tick is 0), so a player can be moved or an enemy spawned at a chosen moment.
    /// Actions for one tick run in the order they were added, each once.
    ///
    /// # Errors
    /// [`ScenarioError::InThePast`] if that tick has already run.
    pub fn at(
        &mut self,
        tick: u64,
        action: impl FnOnce(&mut W) + 'static,
    ) -> Result<(), ScenarioError> {
        if tick < self.ticks {
            return Err(ScenarioError::InThePast {
                tick,
                ticks_run: self.ticks,
            });
        }
        self.script.push((tick, Box::new(action)));
        Ok(())
    }

    /// What a probe recorded, or `None` for an unknown name.
    pub fn series(&self, name: &str) -> Option<Series> {
        self.probes
            .iter()
            .find(|p| p.name == name)
            .map(|p| Series(p.samples.clone()))
    }

    /// Run exactly one tick: every system in order, then the probes and the lifecycle log.
    pub fn step(&mut self) {
        let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.script)
            .into_iter()
            .partition(|(t, _)| *t == self.ticks);
        self.script = later;
        for (_, action) in now {
            action(self.sim.world_mut());
        }
        self.sim.clock_mut().step_once();
        let frame = self.sim.update(0);
        debug_assert_eq!(
            frame.ticks, 1,
            "a paused clock gives exactly one tick per step"
        );
        self.ticks += u64::from(frame.ticks);
        for p in &mut self.probes {
            let v = (p.read)(self.sim.world());
            p.samples.push(v);
        }
        if let Some(collect) = self.collector {
            collect(self.sim.world_mut(), self.ticks, &mut self.lifecycle);
        }
    }

    /// Run `ticks` ticks.
    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// Run until `done(world)` holds or `max_ticks` ticks have run. The condition is tested **before** the first tick (so a condition that already holds returns
    /// `Reached(0)`) and after every tick.
    pub fn run_until(&mut self, max_ticks: u64, done: impl Fn(&W) -> bool) -> Outcome {
        for ran in 0..=max_ticks {
            if done(self.sim.world()) {
                return Outcome::Reached(ran);
            }
            if ran == max_ticks {
                break;
            }
            self.step();
        }
        Outcome::TimedOut(max_ticks)
    }
}

impl<W: World> Scenario<W> {
    /// Also record every spawn and despawn with the tick it happened in (the number of ticks run when it was collected: 0 for what happened during setup). Returns the scenario.
    #[must_use]
    pub fn with_lifecycle_log(mut self) -> Self {
        self.collector =
            Some(|w, tick, log| log.extend(w.drain_lifecycle().into_iter().map(|e| (tick, e))));
        // What happened during setup is part of the log, at tick 0.
        let setup = self.sim.world_mut().drain_lifecycle();
        self.lifecycle.extend(setup.into_iter().map(|e| (0, e)));
        self
    }

    /// The recorded spawns and despawns, oldest first (empty unless [`with_lifecycle_log`](Self::with_lifecycle_log) was used).
    pub fn lifecycle(&self) -> &[(u64, LifecycleEvent)] {
        &self.lifecycle
    }

    /// Spawns minus despawns recorded so far.
    pub fn net_spawned(&self) -> i64 {
        self.lifecycle
            .iter()
            .map(|(_, e)| {
                if matches!(e, LifecycleEvent::Spawned(_)) {
                    1
                } else {
                    -1
                }
            })
            .sum()
    }
}

/// Run a scenario twice from scratch for `ticks` ticks and check the two final worlds hash the same. `make` builds a fresh scenario each time (so it must not share state);
/// `hash` summarises a world. Returns the common hash.
///
/// # Panics
/// If the two hashes differ: the simulation is not deterministic.
pub fn assert_deterministic<W>(
    make: impl Fn() -> Scenario<W>,
    ticks: u64,
    hash: impl Fn(&W) -> u64,
) -> u64 {
    let run = || {
        let mut s = make();
        s.run(ticks);
        hash(s.world())
    };
    let (a, b) = (run(), run());
    assert_eq!(
        a, b,
        "two runs of the same scenario ended differently: the simulation is not deterministic"
    );
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflect::PropertyValue;
    use crate::world::{SimpleWorld, conformance};

    fn mob_world() -> (SimpleWorld, crate::EntityId) {
        let mut w = SimpleWorld::new(conformance::registry());
        let mob = w.registry().class_id("Mob").unwrap();
        let id = w.spawn(mob, None).unwrap();
        (w, id)
    }

    fn health(w: &SimpleWorld, id: crate::EntityId) -> f64 {
        match w.get(id, "health").unwrap() {
            PropertyValue::F32(h) => f64::from(h),
            other => panic!("{other:?}"),
        }
    }

    /// A scenario with a mob that loses `dps` health per second.
    fn burning(dps: f32) -> (Scenario<SimpleWorld>, crate::EntityId) {
        let (w, id) = mob_world();
        let mut s = Scenario::new(TickConfig::default(), w).unwrap();
        s.system(Phase::Damage, "burn", 0, move |w, ctx| {
            if let Ok(PropertyValue::F32(h)) = w.get(id, "health") {
                let _ = w.set(
                    id,
                    "health",
                    PropertyValue::F32((h - dps * ctx.dt_seconds()).max(0.0)),
                );
            }
        })
        .unwrap();
        (s, id)
    }

    // ---- stepping ------------------------------------------------------------------------------------

    #[test]
    fn every_step_is_exactly_one_tick_whatever_the_rate() {
        for hz in [1, 7, 30, 60, 1000] {
            let mut s = Scenario::new(
                TickConfig {
                    tick_hz: hz,
                    max_ticks_per_frame: 8,
                    max_frame_micros: 250_000,
                },
                0u64,
            )
            .unwrap();
            s.system(Phase::Move, "count", 0, |w, _| *w += 1).unwrap();
            s.run(25);
            assert_eq!((s.ticks(), *s.world()), (25, 25), "{hz} Hz");
            assert!((s.seconds() - 25.0 / f64::from(hz)).abs() < 1e-12);
        }
    }

    #[test]
    fn systems_run_in_phase_order_and_see_the_tick_number() {
        let mut s = Scenario::new(TickConfig::default(), Vec::<String>::new()).unwrap();
        s.system(Phase::Damage, "b", 0, |w, c| {
            w.push(format!("damage@{}", c.tick))
        })
        .unwrap();
        s.system(Phase::Input, "a", 0, |w, c| {
            w.push(format!("input@{}", c.tick))
        })
        .unwrap();
        s.run(2);
        assert_eq!(s.world(), &["input@0", "damage@0", "input@1", "damage@1"]);
        assert!(
            s.system(Phase::Input, "a", 0, |_, _| {}).is_err(),
            "a duplicate system name is refused"
        );
    }

    #[test]
    fn a_bad_configuration_is_refused_and_zero_ticks_do_nothing() {
        assert!(
            Scenario::new(
                TickConfig {
                    tick_hz: 0,
                    ..TickConfig::default()
                },
                0u8
            )
            .is_err()
        );
        let mut s = Scenario::new(TickConfig::default(), 0u8).unwrap();
        s.run(0);
        assert_eq!((s.ticks(), s.seconds()), (0, 0.0));
        assert_eq!(s.seconds_of(30), 1.0);
    }

    #[test]
    fn the_world_can_be_changed_between_steps() {
        let mut s = Scenario::new(TickConfig::default(), 0u64).unwrap();
        s.system(Phase::Move, "add", 0, |w, _| *w += 10).unwrap();
        s.run(1);
        *s.world_mut() = 1000;
        s.run(1);
        assert_eq!(*s.world(), 1010);
        s.simulation().schedule_mut().set_enabled("add", false);
        s.run(3);
        assert_eq!(
            *s.world(),
            1010,
            "the system was switched off through the simulation"
        );
    }

    // ---- probes and series -----------------------------------------------------------------------------------

    #[test]
    fn a_probe_records_the_start_and_every_tick_and_time_to_kill_is_the_first_dead_sample() {
        let (mut s, id) = burning(20.0); // 100 health at 20 per second: five seconds, 150 ticks at 30 Hz
        s.probe("health", move |w| health(w, id)).unwrap();
        s.run(200);
        let h = s.series("health").unwrap();
        assert_eq!(
            h.len(),
            201,
            "one sample before the first tick and one after each"
        );
        assert_eq!(h.first(), Some(100.0));
        assert_eq!(h.last(), Some(0.0), "it stops at zero");
        let ttk = h.first_tick_where(|v| v <= 0.0).unwrap();
        assert!((149..=151).contains(&ttk), "{ttk}");
        assert!((h.total_drop() - 100.0).abs() < 1e-3, "{}", h.total_drop());
        assert_eq!(h.total_rise(), 0.0);
        assert_eq!((h.peak(), h.trough()), (Some(100.0), Some(0.0)));
        assert!(s.series("nope").is_none());
    }

    #[test]
    fn a_probe_added_late_starts_with_the_value_it_has_then() {
        let mut s = Scenario::new(TickConfig::default(), 0u64).unwrap();
        s.system(Phase::Move, "add", 0, |w, _| *w += 1).unwrap();
        s.run(5);
        s.probe("n", |w| *w as f64).unwrap();
        s.run(2);
        assert_eq!(s.series("n").unwrap().values(), [5.0, 6.0, 7.0]);
    }

    #[test]
    fn probe_names_must_be_new_and_not_empty() {
        let mut s = Scenario::new(TickConfig::default(), 0u8).unwrap();
        s.probe("a", |_| 0.0).unwrap();
        assert_eq!(
            s.probe("a", |_| 1.0),
            Err(ScenarioError::DuplicateProbe("a".into()))
        );
        assert_eq!(s.probe("", |_| 1.0), Err(ScenarioError::EmptyName));
        assert!(
            ScenarioError::EmptyName.to_string().contains("name")
                && ScenarioError::DuplicateProbe("a".into())
                    .to_string()
                    .contains("\"a\"")
        );
    }

    #[test]
    fn series_statistics_handle_rises_drops_nan_and_empty() {
        let s = Series(vec![10.0, 7.0, 9.0, 4.0, 4.0, 6.0]);
        assert_eq!(
            s.total_drop(),
            3.0 + 5.0,
            "drops 10 to 7 and 9 to 4; the rises do not cancel them"
        );
        assert_eq!(s.total_rise(), 2.0 + 2.0);
        assert_eq!(
            (s.peak(), s.trough(), s.first(), s.last()),
            (Some(10.0), Some(4.0), Some(10.0), Some(6.0))
        );
        assert_eq!(s.first_tick_where(|v| v < 5.0), Some(3));
        assert_eq!(s.first_tick_where(|v| v > 100.0), None);
        let nan = Series(vec![f64::NAN, 3.0, f64::NAN, 1.0]);
        assert_eq!(
            (nan.peak(), nan.trough()),
            (Some(3.0), Some(1.0)),
            "NaN is skipped"
        );
        let only_nan = Series(vec![f64::NAN, f64::NAN]);
        assert_eq!(
            (only_nan.peak(), only_nan.trough()),
            (None, None),
            "a series of nothing but NaN has no peak"
        );
        let empty = Series(vec![]);
        assert!(empty.is_empty());
        assert_eq!(
            (
                empty.peak(),
                empty.trough(),
                empty.first(),
                empty.last(),
                empty.total_drop(),
                empty.total_rise()
            ),
            (None, None, None, None, 0.0, 0.0)
        );
        assert_eq!(
            Series(vec![1.0]).total_drop(),
            0.0,
            "one sample has no changes"
        );
    }

    // ---- run_until ---------------------------------------------------------------------------------------------

    #[test]
    fn run_until_reports_the_ticks_it_took_or_that_it_timed_out() {
        let (mut s, id) = burning(100.0); // one second to live: 30 ticks
        let outcome = s.run_until(1000, move |w| health(w, id) <= 0.0);
        let ticks = outcome.reached().expect("it dies");
        assert!((29..=31).contains(&ticks), "{ticks}");
        assert_eq!(s.ticks(), ticks);
        // Already true: no tick runs.
        let again = s.run_until(1000, move |w| health(w, id) <= 0.0);
        assert_eq!((again, s.ticks()), (Outcome::Reached(0), ticks));
        // A condition that never holds times out after exactly the limit.
        let (mut slow, id2) = burning(0.0);
        let timed = slow.run_until(50, move |w| health(w, id2) <= 0.0);
        assert_eq!(
            (timed, timed.reached(), slow.ticks()),
            (Outcome::TimedOut(50), None, 50)
        );
        assert_eq!(
            slow.run_until(0, |_| false),
            Outcome::TimedOut(0),
            "a limit of zero runs nothing"
        );
    }

    #[test]
    fn the_condition_that_holds_exactly_at_the_limit_counts_as_reached() {
        let mut s = Scenario::new(TickConfig::default(), 0u64).unwrap();
        s.system(Phase::Move, "add", 0, |w, _| *w += 1).unwrap();
        assert_eq!(s.run_until(5, |w| *w >= 5), Outcome::Reached(5));
        let mut t = Scenario::new(TickConfig::default(), 0u64).unwrap();
        t.system(Phase::Move, "add", 0, |w, _| *w += 1).unwrap();
        assert_eq!(t.run_until(4, |w| *w >= 5), Outcome::TimedOut(4));
    }

    // ---- scripted actions ----------------------------------------------------------------------------

    #[test]
    fn a_scripted_action_runs_once_before_the_systems_of_its_tick() {
        let mut s = Scenario::new(TickConfig::default(), Vec::<String>::new()).unwrap();
        s.system(Phase::Move, "log", 0, |w, c| {
            w.push(format!("tick{}", c.tick))
        })
        .unwrap();
        s.at(2, |w| w.push("action".to_owned())).unwrap();
        s.at(0, |w| w.push("first".to_owned())).unwrap();
        s.run(4);
        assert_eq!(
            s.world(),
            &["first", "tick0", "tick1", "action", "tick2", "tick3"]
        );
        s.run(2);
        assert_eq!(
            s.world().iter().filter(|x| *x == "action").count(),
            1,
            "it ran once"
        );
    }

    #[test]
    fn actions_for_one_tick_run_in_the_order_they_were_added_and_later_ones_wait() {
        let mut s = Scenario::new(TickConfig::default(), Vec::<u8>::new()).unwrap();
        s.at(1, |w| w.push(1)).unwrap();
        s.at(1, |w| w.push(2)).unwrap();
        s.at(5, |w| w.push(9)).unwrap();
        s.at(1, |w| w.push(3)).unwrap();
        s.run(3);
        assert_eq!(s.world(), &[1, 2, 3]);
        s.run(3);
        assert_eq!(s.world(), &[1, 2, 3, 9]);
    }

    #[test]
    fn an_action_for_a_tick_that_has_run_is_refused_and_the_current_tick_is_allowed() {
        let mut s = Scenario::new(TickConfig::default(), 0u8).unwrap();
        s.run(3);
        let err = s.at(2, |_| {}).unwrap_err();
        assert_eq!(
            err,
            ScenarioError::InThePast {
                tick: 2,
                ticks_run: 3
            }
        );
        assert!(err.to_string().contains("already run"));
        assert!(
            s.at(3, |w| *w = 7).is_ok(),
            "the next tick to run is tick 3"
        );
        s.run(1);
        assert_eq!(*s.world(), 7);
    }

    #[test]
    fn scripted_setup_can_spawn_an_enemy_mid_scenario_and_time_to_kill_counts_from_the_run() {
        let (w, _) = mob_world();
        let mut s = Scenario::new(TickConfig::default(), w)
            .unwrap()
            .with_lifecycle_log();
        s.at(10, |w| {
            let mob = w.registry().class_id("Mob").unwrap();
            let _ = w.spawn(mob, None);
        })
        .unwrap();
        s.run(12);
        assert_eq!(s.net_spawned(), 2);
        let spawn_tick = s
            .lifecycle()
            .iter()
            .filter(|(_, e)| matches!(e, LifecycleEvent::Spawned(_)))
            .map(|(t, _)| *t)
            .max();
        assert_eq!(
            spawn_tick,
            Some(11),
            "spawned before tick 10's systems, collected after it: 11 ticks had run"
        );
    }

    // ---- lifecycle ---------------------------------------------------------------------------------------------

    #[test]
    fn spawns_and_despawns_are_logged_with_their_tick() {
        let (w, id) = mob_world(); // spawned during setup
        let mut s = Scenario::new(TickConfig::default(), w)
            .unwrap()
            .with_lifecycle_log();
        s.system(Phase::Cleanup, "reaper", 0, move |w, ctx| {
            if ctx.tick == 3 {
                let _ = w.despawn(id);
            }
            if ctx.tick == 5 {
                let mob = w.registry().class_id("Mob").unwrap();
                let _ = w.spawn(mob, None);
            }
        })
        .unwrap();
        s.run(8);
        let log = s.lifecycle();
        assert_eq!(log.len(), 3, "{log:?}");
        assert_eq!(
            log[0],
            (0, LifecycleEvent::Spawned(id)),
            "setup is logged at tick 0"
        );
        assert_eq!(
            (log[1].0, log[1].1),
            (4, LifecycleEvent::Destroyed(id)),
            "despawned during tick 3, collected after it: 4 ticks have run"
        );
        assert!(matches!(log[2], (6, LifecycleEvent::Spawned(_))));
        assert_eq!(s.net_spawned(), 1);
        // Without the log nothing is recorded.
        let (w2, _) = mob_world();
        let mut plain = Scenario::new(TickConfig::default(), w2).unwrap();
        plain.run(3);
        assert!(plain.lifecycle().is_empty());
    }

    // ---- determinism --------------------------------------------------------------------------------------------

    #[test]
    fn a_deterministic_scenario_gives_the_same_hash_and_the_hash_depends_on_the_run() {
        let make = || {
            let (mut s, _) = burning(7.5);
            s.system(Phase::Move, "mix", 0, |w, ctx| {
                let _ = w.entities();
                let _ = ctx.tick;
            })
            .unwrap();
            s
        };
        let hash = |w: &SimpleWorld| match w.get(w.entities()[0], "health").unwrap() {
            PropertyValue::F32(h) => u64::from(h.to_bits()),
            _ => 0,
        };
        let a = assert_deterministic(make, 40, hash);
        let b = assert_deterministic(make, 41, hash);
        assert_ne!(a, b, "one more tick gives a different world");
        assert_eq!(assert_deterministic(make, 40, hash), a);
    }

    #[test]
    #[should_panic(expected = "not deterministic")]
    fn a_scenario_that_depends_on_something_outside_it_is_caught() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static RUNS: AtomicU64 = AtomicU64::new(0);
        let make = || {
            let offset = RUNS.fetch_add(1, Ordering::SeqCst); // each run starts differently
            Scenario::new(TickConfig::default(), offset).unwrap()
        };
        assert_deterministic(make, 3, |w| *w);
    }
}
