//! The systems that run each tick, in a fixed, deterministic order (roadmap T08-003).
//!
//! A [`Schedule`] holds named systems, each in a [`Phase`]; every tick they run phase by phase
//! (input, AI, movement, physics, damage, cleanup, replication), and inside a phase by their
//! `order` number and then by the order they were added. The order depends only on those three
//! things, so the same registrations give the same order on every machine and every run.
//! [`Simulation`] puts a [`Schedule`] together with a [`TickLoop`](crate::tick::TickLoop) and a
//! world.

use std::fmt;

use crate::tick::{FrameTicks, TickConfig, TickConfigError, TickLoop};

/// The stages of a tick, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Read the inputs that apply to this tick.
    Input,
    /// Decide what the computer-controlled entities do.
    Ai,
    /// Move entities (player controllers, scripted motion).
    Move,
    /// Physics and collision.
    Physics,
    /// Apply damage, deaths and what they trigger.
    Damage,
    /// Remove what is gone, flush spawns and despawns.
    Cleanup,
    /// Build what is sent to clients.
    Replicate,
}

impl Phase {
    /// Every phase, in running order.
    pub const ALL: [Self; 7] = [
        Self::Input,
        Self::Ai,
        Self::Move,
        Self::Physics,
        Self::Damage,
        Self::Cleanup,
        Self::Replicate,
    ];
}

/// What a system is told about the tick it runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickContext {
    /// The number of the tick being simulated (the first tick is 0).
    pub tick: u64,
    /// Ticks per second.
    pub tick_hz: u32,
}

impl TickContext {
    /// The step of every tick in seconds: constant, so systems may use it freely.
    pub fn dt_seconds(&self) -> f32 {
        1.0 / self.tick_hz as f32
    }

    /// Simulation time at the start of this tick, in microseconds.
    pub fn sim_micros(&self) -> u64 {
        u64::try_from(u128::from(self.tick) * 1_000_000 / u128::from(self.tick_hz))
            .unwrap_or(u64::MAX)
    }
}

/// Why a system could not be added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    /// Another system has this name.
    DuplicateName(String),
    /// The name is empty.
    EmptyName,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateName(name) => write!(f, "a system called {name:?} already exists"),
            Self::EmptyName => write!(f, "a system needs a name"),
        }
    }
}

impl std::error::Error for ScheduleError {}

type SystemFn<W> = Box<dyn FnMut(&mut W, &TickContext)>;

struct Entry<W> {
    name: String,
    phase: Phase,
    order: i32,
    seq: u64,
    enabled: bool,
    runs: u64,
    run: SystemFn<W>,
}

/// The systems of a simulation over a world of type `W`; see the module documentation.
pub struct Schedule<W> {
    systems: Vec<Entry<W>>,
    next_seq: u64,
}

impl<W> Default for Schedule<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W> Schedule<W> {
    /// An empty schedule.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            next_seq: 0,
        }
    }

    /// Add a system to `phase`. Within the phase, lower `order` runs first, then the one added first.
    pub fn add(
        &mut self,
        phase: Phase,
        name: &str,
        order: i32,
        system: impl FnMut(&mut W, &TickContext) + 'static,
    ) -> Result<(), ScheduleError> {
        if name.is_empty() {
            return Err(ScheduleError::EmptyName);
        }
        if self.systems.iter().any(|e| e.name == name) {
            return Err(ScheduleError::DuplicateName(name.to_owned()));
        }
        let entry = Entry {
            name: name.to_owned(),
            phase,
            order,
            seq: self.next_seq,
            enabled: true,
            runs: 0,
            run: Box::new(system),
        };
        self.next_seq += 1;
        let at = self
            .systems
            .partition_point(|e| (e.phase, e.order, e.seq) < (entry.phase, entry.order, entry.seq));
        self.systems.insert(at, entry);
        Ok(())
    }

    /// Remove a system; `false` if there is none of that name.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.systems.len();
        self.systems.retain(|e| e.name != name);
        self.systems.len() != before
    }

    /// Switch a system on or off (for debugging); `false` if there is none of that name.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        match self.systems.iter_mut().find(|e| e.name == name) {
            Some(entry) => {
                entry.enabled = enabled;
                true
            }
            None => false,
        }
    }

    /// How many times a system has run.
    pub fn runs(&self, name: &str) -> Option<u64> {
        self.systems.iter().find(|e| e.name == name).map(|e| e.runs)
    }

    /// How many systems there are.
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// Whether there are no systems.
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// The phase and name of every system, in running order.
    pub fn order(&self) -> Vec<(Phase, &str)> {
        self.systems
            .iter()
            .map(|e| (e.phase, e.name.as_str()))
            .collect()
    }

    /// Run every enabled system once, in order.
    pub fn run(&mut self, world: &mut W, ctx: &TickContext) {
        for entry in self.systems.iter_mut().filter(|e| e.enabled) {
            (entry.run)(world, ctx);
            entry.runs += 1;
        }
    }
}

/// A world, its systems and the tick loop that drives them.
///
/// ```
/// use ss2_sim::{Phase, Simulation, TickConfig};
///
/// let mut sim = Simulation::new(TickConfig::default(), 0_u64).unwrap(); // the world is a counter
/// sim.schedule_mut().add(Phase::Move, "count", 0, |world, _ctx| *world += 1).unwrap();
/// let frame = sim.update(1_000_000); // one second of real time; the frame is clamped to 250 ms (7 ticks at 30 Hz)
/// assert_eq!(frame.ticks, 7);
/// assert_eq!(*sim.world(), 7);
/// ```
pub struct Simulation<W> {
    clock: TickLoop,
    schedule: Schedule<W>,
    world: W,
}

impl<W> Simulation<W> {
    /// A simulation at tick 0 with no systems.
    pub fn new(config: TickConfig, world: W) -> Result<Self, TickConfigError> {
        Ok(Self {
            clock: TickLoop::new(config)?,
            schedule: Schedule::new(),
            world,
        })
    }

    /// The tick loop (pause, time scale, the tick counter).
    pub fn clock(&self) -> &TickLoop {
        &self.clock
    }

    /// The tick loop, to pause, step or change the time scale.
    pub fn clock_mut(&mut self) -> &mut TickLoop {
        &mut self.clock
    }

    /// The systems.
    pub fn schedule(&self) -> &Schedule<W> {
        &self.schedule
    }

    /// The systems, to add or switch.
    pub fn schedule_mut(&mut self) -> &mut Schedule<W> {
        &mut self.schedule
    }

    /// The world.
    pub fn world(&self) -> &W {
        &self.world
    }

    /// The world, to change between frames (loading, spawning from outside the tick).
    pub fn world_mut(&mut self) -> &mut W {
        &mut self.world
    }

    /// Feed the real time of a frame; runs every due tick (all systems, in order, per tick) and
    /// returns what was run and the `alpha` for drawing.
    pub fn update(&mut self, real_micros: u64) -> FrameTicks {
        let frame = self.clock.advance(real_micros);
        let first = self.clock.tick() - u64::from(frame.ticks);
        let tick_hz = self.clock.config().tick_hz;
        for i in 0..u64::from(frame.ticks) {
            self.schedule.run(
                &mut self.world,
                &TickContext {
                    tick: first + i,
                    tick_hz,
                },
            );
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::{TickConfig, TickLoop};

    #[derive(Default)]
    struct Log {
        names: Vec<String>,
        ticks: Vec<u64>,
        hash: u64,
    }

    fn logger(label: &'static str) -> impl FnMut(&mut Log, &TickContext) + 'static {
        move |w, ctx| {
            w.names.push(label.to_string());
            w.ticks.push(ctx.tick);
            w.hash = w
                .hash
                .wrapping_mul(31)
                .wrapping_add(ctx.tick ^ label.len() as u64);
        }
    }

    #[test]
    fn systems_run_by_phase_whatever_order_they_were_added_in() {
        let mut s = Schedule::new();
        s.add(Phase::Replicate, "replicate", 0, logger("replicate"))
            .unwrap();
        s.add(Phase::Input, "input", 0, logger("input")).unwrap();
        s.add(Phase::Cleanup, "cleanup", 0, logger("cleanup"))
            .unwrap();
        s.add(Phase::Physics, "physics", 0, logger("physics"))
            .unwrap();
        s.add(Phase::Ai, "ai", 0, logger("ai")).unwrap();
        s.add(Phase::Damage, "damage", 0, logger("damage")).unwrap();
        s.add(Phase::Move, "move", 0, logger("move")).unwrap();
        let mut w = Log::default();
        s.run(
            &mut w,
            &TickContext {
                tick: 0,
                tick_hz: 30,
            },
        );
        assert_eq!(
            w.names,
            [
                "input",
                "ai",
                "move",
                "physics",
                "damage",
                "cleanup",
                "replicate"
            ]
        );
    }

    #[test]
    fn inside_a_phase_the_order_number_then_the_order_of_adding_decides() {
        let mut s = Schedule::new();
        s.add(Phase::Move, "late", 10, logger("late")).unwrap();
        s.add(Phase::Move, "first_added", 0, logger("first_added"))
            .unwrap();
        s.add(Phase::Move, "early", -5, logger("early")).unwrap();
        s.add(Phase::Move, "second_added", 0, logger("second_added"))
            .unwrap();
        let order: Vec<_> = s.order().iter().map(|(_, name)| name.to_string()).collect();
        assert_eq!(order, ["early", "first_added", "second_added", "late"]);
        let mut w = Log::default();
        s.run(
            &mut w,
            &TickContext {
                tick: 3,
                tick_hz: 30,
            },
        );
        assert_eq!(w.names, ["early", "first_added", "second_added", "late"]);
    }

    #[test]
    fn names_are_unique_and_a_system_can_be_switched_off_or_removed() {
        let mut s = Schedule::new();
        s.add(Phase::Ai, "brain", 0, logger("brain")).unwrap();
        assert_eq!(
            s.add(Phase::Move, "brain", 0, logger("again")),
            Err(ScheduleError::DuplicateName("brain".into()))
        );
        assert_eq!(
            s.add(Phase::Move, "", 0, logger("x")),
            Err(ScheduleError::EmptyName)
        );
        s.add(Phase::Move, "walk", 0, logger("walk")).unwrap();
        let mut w = Log::default();
        assert!(s.set_enabled("brain", false));
        assert!(!s.set_enabled("nobody", false));
        s.run(
            &mut w,
            &TickContext {
                tick: 0,
                tick_hz: 30,
            },
        );
        assert_eq!(w.names, ["walk"]);
        assert_eq!(s.runs("brain"), Some(0));
        assert_eq!(s.runs("walk"), Some(1));
        s.set_enabled("brain", true);
        s.run(
            &mut w,
            &TickContext {
                tick: 1,
                tick_hz: 30,
            },
        );
        assert_eq!(s.runs("brain"), Some(1));
        assert!(s.remove("brain"));
        assert!(!s.remove("brain"));
        assert_eq!(s.runs("brain"), None);
        assert_eq!(s.len(), 1);
        // The name is free again.
        s.add(Phase::Ai, "brain", 0, logger("brain2")).unwrap();
    }

    #[test]
    fn the_context_carries_the_tick_and_a_constant_step() {
        let ctx = TickContext {
            tick: 9,
            tick_hz: 50,
        };
        assert!((ctx.dt_seconds() - 0.02).abs() < 1e-9);
        assert_eq!(ctx.sim_micros(), 180_000);
    }

    #[test]
    fn the_simulation_runs_every_system_once_per_tick_with_consecutive_tick_numbers() {
        let mut sim = Simulation::new(TickConfig::default(), Log::default()).unwrap();
        sim.schedule_mut()
            .add(Phase::Move, "m", 0, logger("m"))
            .unwrap();
        sim.schedule_mut()
            .add(Phase::Input, "i", 0, logger("i"))
            .unwrap();
        let f = sim.update(100_000); // 3 ticks at 30 Hz
        assert_eq!(f.ticks, 3);
        assert_eq!(sim.world().names, ["i", "m", "i", "m", "i", "m"]);
        assert_eq!(sim.world().ticks, [0, 0, 1, 1, 2, 2]);
        sim.update(34_000);
        assert_eq!(*sim.world().ticks.last().unwrap(), 3);
        assert_eq!(sim.clock().tick(), 4);
    }

    #[test]
    fn the_same_time_cut_differently_gives_the_same_world() {
        let run = |frames: &[u64]| {
            let mut sim = Simulation::new(
                TickConfig {
                    // Lumpy frames are long on purpose: the stall caps must not apply.
                    max_ticks_per_frame: 1000,
                    max_frame_micros: 100_000_000,
                    ..TickConfig::default()
                },
                Log::default(),
            )
            .unwrap();
            sim.schedule_mut()
                .add(Phase::Move, "a", 0, logger("a"))
                .unwrap();
            sim.schedule_mut()
                .add(Phase::Ai, "bb", 0, logger("bb"))
                .unwrap();
            for f in frames {
                sim.update(*f);
            }
            (sim.world().hash, sim.world().names.len())
        };
        let steady = vec![16_666_u64; 120]; // about 2 s at 60 fps
        let total: u64 = steady.iter().sum();
        let lumpy = vec![total / 3, total / 3, total - 2 * (total / 3)];
        let tiny: Vec<u64> = (0..total / 1000)
            .map(|_| 1000)
            .chain([total % 1000])
            .collect();
        assert_eq!(run(&steady), run(&lumpy));
        assert_eq!(run(&steady), run(&tiny));
    }

    #[test]
    fn a_paused_simulation_runs_nothing_and_a_step_runs_one_tick() {
        let mut sim = Simulation::new(TickConfig::default(), Log::default()).unwrap();
        sim.schedule_mut()
            .add(Phase::Move, "m", 0, logger("m"))
            .unwrap();
        sim.clock_mut().pause();
        sim.update(1_000_000);
        assert!(sim.world().names.is_empty());
        sim.clock_mut().step_once();
        let f = sim.update(0);
        assert_eq!(f.ticks, 1);
        assert_eq!(sim.world().ticks, [0]);
    }

    #[test]
    fn ticks_given_up_in_a_stall_do_not_run_their_systems() {
        let mut sim = Simulation::new(
            TickConfig {
                max_frame_micros: 100_000_000,
                ..TickConfig::default()
            },
            Log::default(),
        )
        .unwrap();
        sim.schedule_mut()
            .add(Phase::Move, "m", 0, logger("m"))
            .unwrap();
        let f = sim.update(60_000_000);
        assert_eq!(f.ticks, 8);
        assert!(f.dropped_ticks > 1000);
        assert_eq!(sim.world().names.len(), 8);
        assert_eq!(
            sim.clock().tick(),
            8,
            "the clock counts only ticks that ran"
        );
    }

    #[test]
    fn a_standalone_loop_and_schedule_can_be_driven_by_hand() {
        let mut clock = TickLoop::new(TickConfig::default()).unwrap();
        let mut schedule = Schedule::new();
        schedule.add(Phase::Move, "m", 0, logger("m")).unwrap();
        let mut world = Log::default();
        let f = clock.advance(70_000);
        let first = clock.tick() - u64::from(f.ticks);
        for i in 0..u64::from(f.ticks) {
            schedule.run(
                &mut world,
                &TickContext {
                    tick: first + i,
                    tick_hz: clock.config().tick_hz,
                },
            );
        }
        assert_eq!(world.ticks, [0, 1]);
    }
}
