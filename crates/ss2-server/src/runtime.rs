//! The dedicated server's main loop (roadmap T08-020): the simulation tick loop driven by a real
//! clock, the console commands an operator types, and the numbers the metrics endpoint serves.
//!
//! Time comes from a [`Clock`] so the loop is tested with a fake one. **There is no network in it
//! yet**: the transport (RFC 0006) does not exist, so the server runs an empty world and nobody
//! can join; what it proves is the loop, the console and the metrics.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ss2_core::console::Console;
use ss2_sim::{Phase, Simulation, TickConfig};

use crate::config::ServerSettings;
use crate::metrics::Metrics;

/// Where the loop gets time and how it waits.
pub trait Clock {
    /// Microseconds since some fixed moment; never goes back.
    fn now_micros(&mut self) -> u64;
    /// Wait about this long.
    fn sleep_micros(&mut self, micros: u64);
}

/// The real clock.
pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    /// A clock that starts now.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now_micros(&mut self) -> u64 {
        u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
    fn sleep_micros(&mut self, micros: u64) {
        std::thread::sleep(Duration::from_micros(micros));
    }
}

/// Why [`Runtime::run`] returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitReason {
    /// An operator typed `quit`.
    Quit,
    /// The clock reached the deadline that was asked for (used by tests and tools).
    DeadlineReached,
}

/// The server's world: nothing yet but a count of the ticks that ran.
#[derive(Default)]
struct ServerWorld {
    ticks: u64,
}

/// What the console commands report.
#[derive(Default)]
struct Status {
    tick: u64,
    uptime_seconds: f64,
    players: u32,
    max_players: u32,
    name: String,
}

/// The server's loop; see the module documentation.
pub struct Runtime {
    sim: Simulation<ServerWorld>,
    console: Console,
    console_enabled: bool,
    metrics: Arc<Mutex<Metrics>>,
    status: Rc<RefCell<Status>>,
    stop: Rc<Cell<bool>>,
    output: Box<dyn FnMut(String)>,
    started_at: Option<u64>,
    last_read: Option<u64>,
    tick_micros_max: u64,
    ticks_dropped: u64,
    period_micros: u64,
    max_players: u32,
}

impl Runtime {
    /// A runtime for `settings` publishing to `metrics`.
    pub fn new(settings: &ServerSettings, metrics: Arc<Mutex<Metrics>>) -> Result<Self, String> {
        let config = TickConfig {
            tick_hz: settings.tick_hz,
            ..TickConfig::default()
        };
        let mut sim = Simulation::new(config, ServerWorld::default()).map_err(|e| e.to_string())?;
        sim.schedule_mut()
            .add(
                Phase::Move,
                "count_ticks",
                0,
                |world: &mut ServerWorld, _| world.ticks += 1,
            )
            .map_err(|e| e.to_string())?;

        let status = Rc::new(RefCell::new(Status {
            max_players: u32::from(settings.max_players),
            name: settings.name.clone(),
            ..Status::default()
        }));
        let stop = Rc::new(Cell::new(false));
        let mut console = Console::new();
        let shown = Rc::clone(&status);
        console
            .register_command(
                "status",
                "show the tick, the players and the uptime",
                move |_, _| {
                    let s = shown.borrow();
                    Ok(format!(
                        "{}: tick {}, players {}/{}, uptime {:.0}s",
                        s.name, s.tick, s.players, s.max_players, s.uptime_seconds
                    ))
                },
            )
            .map_err(|e| e.to_string())?;
        let flag = Rc::clone(&stop);
        console
            .register_command("quit", "stop the server", move |_, _| {
                flag.set(true);
                Ok("stopping".to_owned())
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            sim,
            console,
            console_enabled: settings.console,
            metrics,
            status,
            stop,
            output: Box::new(|text| println!("{text}")),
            started_at: None,
            last_read: None,
            tick_micros_max: 0,
            ticks_dropped: 0,
            period_micros: 1_000_000 / u64::from(settings.tick_hz.max(1)),
            max_players: u32::from(settings.max_players),
        })
    }

    /// Where command output goes (standard output by default).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn set_output(&mut self, output: Box<dyn FnMut(String)>) {
        self.output = output;
    }

    /// How many ticks have run.
    pub fn ticks_run(&self) -> u64 {
        self.sim.clock().tick()
    }

    /// Run one console command and return what to show; an error becomes text.
    pub fn command(&mut self, line: &str) -> String {
        match self.console.execute(line) {
            Ok(text) => text,
            Err(e) => format!("error: {e}"),
        }
    }

    fn publish(&mut self, uptime_seconds: f64, tick_micros: u64) {
        let tick = self.ticks_run();
        self.tick_micros_max = self.tick_micros_max.max(tick_micros);
        {
            let mut s = self.status.borrow_mut();
            s.tick = tick;
            s.uptime_seconds = uptime_seconds;
        }
        if let Ok(mut m) = self.metrics.lock() {
            *m = Metrics {
                uptime_seconds,
                tick,
                tick_hz: self.sim.clock().config().tick_hz,
                players: 0,
                max_players: self.max_players,
                tick_micros_last: tick_micros,
                tick_micros_max: self.tick_micros_max,
                ticks_dropped: self.ticks_dropped,
            };
        }
    }

    /// Run until an operator types `quit` or, if `deadline` is given, until the clock reaches it.
    /// `input` returns the next console line if there is one (it must not block).
    pub fn run(
        &mut self,
        clock: &mut dyn Clock,
        input: &mut dyn FnMut() -> Option<String>,
        deadline: Option<u64>,
    ) -> ExitReason {
        let started = *self.started_at.get_or_insert_with(|| clock.now_micros());
        loop {
            let before = clock.now_micros();
            let elapsed = before.saturating_sub(self.last_read.unwrap_or(before));
            self.last_read = Some(before);
            let frame = self.sim.update(elapsed);
            self.ticks_dropped += frame.dropped_ticks;
            let after = clock.now_micros();
            self.publish(
                after.saturating_sub(started) as f64 / 1_000_000.0,
                after.saturating_sub(before),
            );

            if self.console_enabled {
                while let Some(line) = input() {
                    let text = self.command(&line);
                    if !text.is_empty() {
                        (self.output)(text);
                    }
                    if self.stop.get() {
                        break;
                    }
                }
            }
            if self.stop.get() {
                return ExitReason::Quit;
            }
            if deadline.is_some_and(|d| after >= d) {
                return ExitReason::DeadlineReached;
            }
            clock.sleep_micros((self.period_micros / 2).max(1_000));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerSettings;
    use crate::metrics::Metrics;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    /// A clock that only moves when the loop sleeps (or when a test jumps it), and that charges
    /// `work_per_read` microseconds each time it is read, to stand for the time the work takes.
    struct FakeClock {
        now: u64,
        sleeps: u32,
        work_per_read: u64,
    }

    impl FakeClock {
        fn new() -> Self {
            Self {
                now: 0,
                sleeps: 0,
                work_per_read: 0,
            }
        }
    }

    impl Clock for FakeClock {
        fn now_micros(&mut self) -> u64 {
            self.now += self.work_per_read;
            self.now
        }
        fn sleep_micros(&mut self, micros: u64) {
            self.sleeps += 1;
            self.now += micros;
        }
    }

    fn runtime(settings: &ServerSettings) -> (Runtime, Arc<Mutex<Metrics>>) {
        let metrics = Arc::new(Mutex::new(Metrics::default()));
        (
            Runtime::new(settings, Arc::clone(&metrics)).unwrap(),
            metrics,
        )
    }

    fn no_input() -> impl FnMut() -> Option<String> {
        || None
    }

    #[test]
    fn one_second_of_clock_time_runs_exactly_the_tick_rate_in_ticks() {
        for hz in [20_u32, 30, 60] {
            let (mut rt, metrics) = runtime(&ServerSettings {
                tick_hz: hz,
                ..ServerSettings::default()
            });
            let mut clock = FakeClock::new();
            let reason = rt.run(&mut clock, &mut no_input(), Some(1_000_000));
            assert_eq!(reason, ExitReason::DeadlineReached);
            assert!(
                (rt.ticks_run() as i64 - i64::from(hz)).abs() <= 1,
                "{hz} Hz: {} ticks",
                rt.ticks_run()
            );
            assert_eq!(metrics.lock().unwrap().tick, rt.ticks_run());
            assert_eq!(metrics.lock().unwrap().tick_hz, hz);
        }
    }

    #[test]
    fn the_loop_sleeps_instead_of_spinning() {
        let (mut rt, _) = runtime(&ServerSettings::default());
        let mut clock = FakeClock::new();
        rt.run(&mut clock, &mut no_input(), Some(2_000_000));
        let ticks = rt.ticks_run() as u32;
        assert!(
            clock.sleeps <= ticks * 3 + 3,
            "{} sleeps for {ticks} ticks",
            clock.sleeps
        );
        assert!(
            clock.sleeps >= ticks / 2,
            "and it does sleep: {}",
            clock.sleeps
        );
    }

    #[test]
    fn quit_stops_the_loop_and_the_input_is_read_in_order() {
        let (mut rt, _) = runtime(&ServerSettings::default());
        let mut clock = FakeClock::new();
        let mut lines: VecDeque<String> =
            VecDeque::from(["status".to_owned(), "quit".to_owned(), "status".to_owned()]);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen2 = Rc::clone(&seen);
        rt.set_output(Box::new(move |text| seen2.borrow_mut().push(text)));
        let reason = rt.run(&mut clock, &mut || lines.pop_front(), None);
        assert_eq!(reason, ExitReason::Quit);
        let out = seen.borrow();
        assert!(out[0].contains("players 0/8"), "{out:?}");
        assert_eq!(
            out.len(),
            2,
            "the second status after quit is not run: {out:?}"
        );
    }

    #[test]
    fn status_reports_the_tick_the_players_and_the_uptime() {
        let (mut rt, _) = runtime(&ServerSettings {
            max_players: 4,
            ..ServerSettings::default()
        });
        let mut clock = FakeClock::new();
        rt.run(&mut clock, &mut no_input(), Some(3_000_000));
        let text = rt.command("status");
        assert!(text.contains("players 0/4"), "{text}");
        assert!(text.contains(&format!("tick {}", rt.ticks_run())), "{text}");
        assert!(text.contains("uptime 3"), "{text}");
    }

    #[test]
    fn unknown_commands_and_bad_input_answer_with_an_error_and_do_not_stop_the_server() {
        let (mut rt, _) = runtime(&ServerSettings::default());
        let unknown = rt.command("frobnicate");
        assert!(
            unknown.starts_with("error:") && unknown.contains("frobnicate"),
            "{unknown}"
        );
        assert!(
            rt.command("status \"unterminated")
                .to_lowercase()
                .contains("error")
        );
        assert_eq!(rt.command(""), "");
        assert!(!rt.command("help").is_empty(), "help lists the commands");
        assert!(rt.command("help").contains("quit") && rt.command("help").contains("status"));
        let mut clock = FakeClock::new();
        assert_eq!(
            rt.run(&mut clock, &mut no_input(), Some(100_000)),
            ExitReason::DeadlineReached
        );
    }

    #[test]
    fn the_console_can_be_switched_off() {
        let (mut rt, _) = runtime(&ServerSettings {
            console: false,
            ..ServerSettings::default()
        });
        let mut clock = FakeClock::new();
        let mut lines = VecDeque::from(["quit".to_owned()]);
        let reason = rt.run(&mut clock, &mut || lines.pop_front(), Some(200_000));
        assert_eq!(
            reason,
            ExitReason::DeadlineReached,
            "input is ignored when the console is off"
        );
    }

    #[test]
    fn a_stall_is_counted_as_dropped_ticks_and_the_server_carries_on() {
        let (mut rt, metrics) = runtime(&ServerSettings::default());
        let mut clock = FakeClock::new();
        rt.run(&mut clock, &mut no_input(), Some(200_000));
        clock.now += 60_000_000; // the machine froze for a minute
        let deadline = clock.now + 200_000;
        rt.run(&mut clock, &mut no_input(), Some(deadline));
        assert!(
            metrics.lock().unwrap().ticks_dropped > 100,
            "{}",
            metrics.lock().unwrap().ticks_dropped
        );
        let ticks_before = rt.ticks_run();
        let deadline = clock.now + 1_000_000;
        rt.run(&mut clock, &mut no_input(), Some(deadline));
        assert!(
            rt.ticks_run() - ticks_before >= 25,
            "normal pace afterwards"
        );
    }

    #[test]
    fn tick_times_and_uptime_are_published() {
        let (mut rt, metrics) = runtime(&ServerSettings::default());
        let mut clock = FakeClock {
            work_per_read: 400,
            ..FakeClock::new()
        };
        rt.run(&mut clock, &mut no_input(), Some(1_000_000));
        let m = metrics.lock().unwrap().clone();
        assert!(
            m.tick_micros_last > 0 && m.tick_micros_max >= m.tick_micros_last,
            "{m:?}"
        );
        assert!(m.uptime_seconds > 0.9 && m.uptime_seconds < 1.2, "{m:?}");
        assert_eq!(m.max_players, 8);
    }

    #[test]
    fn the_system_clock_moves_forward_and_sleeps_for_about_what_it_is_told() {
        let mut clock = SystemClock::new();
        let a = clock.now_micros();
        clock.sleep_micros(5_000);
        let b = clock.now_micros();
        assert!(b >= a + 4_000, "{a} {b}");
        assert!(b < a + 500_000, "and not wildly longer");
    }
}
