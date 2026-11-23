//! The fixed-timestep tick loop (roadmap T08-003).
//!
//! Time **inside** the simulation is the tick counter: tick `n` is at `n / tick_hz` seconds, always.
//! [`TickLoop`] converts the real time that passed between two frames (a number of microseconds
//! that the **caller** measures: this crate never reads a clock) into a whole number of ticks to run
//! now, and the fraction of a tick left over (`alpha`, for the renderer to interpolate with).
//!
//! The conversion is exact integer arithmetic: however the real time is cut into frames, the total
//! number of ticks is `floor(total_time * tick_hz / 1e6)`, so a 30 Hz simulation runs exactly 30
//! ticks per real second at 60 fps, at 144 fps and at 7 fps alike. Pausing, slow motion and
//! single-stepping change how real time is turned into ticks and never the ticks themselves.
//!
//! The tick rate is a setting here: the value the original game uses comes from the main-loop spec
//! (roadmap T01-022).

use std::fmt;

/// One tick is this many accumulator units: a unit is a microsecond times a tick per second times
/// a thousandth of the time scale, so the arithmetic is exact for any tick rate.
const TICK_UNITS: u128 = 1_000_000 * 1000;

/// The most the time scale may ask for (8 times real time).
pub const MAX_TIME_SCALE_PERMILLE: u32 = 8000;

/// The highest tick rate accepted.
pub const MAX_TICK_HZ: u32 = 1000;

/// How the loop turns real time into ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickConfig {
    /// Simulation ticks per second, from 1 to [`MAX_TICK_HZ`].
    pub tick_hz: u32,
    /// The most ticks one call to [`TickLoop::advance`] may return; the rest is given up and
    /// reported, so a stall cannot cause a spiral of ever longer catch-up frames.
    pub max_ticks_per_frame: u32,
    /// A frame longer than this many microseconds is counted as this long (a breakpoint, a window
    /// drag, a suspended laptop).
    pub max_frame_micros: u64,
}

impl Default for TickConfig {
    /// 30 ticks per second, at most 8 ticks and 250 ms per frame. 30 is a placeholder until the
    /// main-loop spec (T01-022) gives the original's rate.
    fn default() -> Self {
        Self {
            tick_hz: 30,
            max_ticks_per_frame: 8,
            max_frame_micros: 250_000,
        }
    }
}

/// Why a [`TickConfig`] is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickConfigError(pub &'static str);

impl fmt::Display for TickConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid tick configuration: {}", self.0)
    }
}

impl std::error::Error for TickConfigError {}

/// What one frame asks of the simulation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameTicks {
    /// How many ticks to run now.
    pub ticks: u32,
    /// How far into the next tick the frame is, in `0.0..1.0`: the blend factor between the last
    /// two states for drawing.
    pub alpha: f32,
    /// Ticks that were due but given up because of the caps (a stall); 0 in normal running.
    pub dropped_ticks: u64,
}

/// The fixed-timestep accumulator; see the module documentation.
#[derive(Clone, Debug)]
pub struct TickLoop {
    config: TickConfig,
    tick: u64,
    accumulator: u128,
    paused: bool,
    pending_steps: u32,
    time_scale_permille: u32,
}

impl TickLoop {
    /// A loop at tick 0, running, at normal speed.
    pub fn new(config: TickConfig) -> Result<Self, TickConfigError> {
        if config.tick_hz == 0 || config.tick_hz > MAX_TICK_HZ {
            return Err(TickConfigError("tick_hz must be from 1 to 1000"));
        }
        if config.max_ticks_per_frame == 0 {
            return Err(TickConfigError("max_ticks_per_frame must be at least 1"));
        }
        if config.max_frame_micros == 0 {
            return Err(TickConfigError("max_frame_micros must be at least 1"));
        }
        Ok(Self {
            config,
            tick: 0,
            accumulator: 0,
            paused: false,
            pending_steps: 0,
            time_scale_permille: 1000,
        })
    }

    /// The settings.
    pub fn config(&self) -> &TickConfig {
        &self.config
    }

    /// The number of ticks run so far: the simulation's clock.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Simulation time at the current tick, in microseconds (exact, from the tick counter).
    pub fn sim_micros(&self) -> u64 {
        self.sim_micros_at(self.tick)
    }

    /// Simulation time at `tick`, in microseconds.
    pub fn sim_micros_at(&self, tick: u64) -> u64 {
        u64::try_from(u128::from(tick) * 1_000_000 / u128::from(self.config.tick_hz))
            .unwrap_or(u64::MAX)
    }

    /// The length of a tick in microseconds, cut to a whole number (for display; the loop itself
    /// never uses it).
    pub fn tick_micros(&self) -> u64 {
        1_000_000 / u64::from(self.config.tick_hz)
    }

    /// Stop running ticks. Real time that passes while paused is not saved up.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Run ticks again.
    pub fn resume(&mut self) {
        self.paused = false;
        self.pending_steps = 0;
    }

    /// Whether ticks are stopped.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// While paused, ask for one more tick at the next [`TickLoop::advance`] (frame advance for
    /// debugging). Requests add up. Ignored when not paused.
    pub fn step_once(&mut self) {
        if self.paused {
            self.pending_steps = self.pending_steps.saturating_add(1);
        }
    }

    /// Run slower or faster than real time: 1000 is normal, 500 half speed, 2000 double. Limited
    /// to [`MAX_TIME_SCALE_PERMILLE`]. 0 stops time without pausing.
    pub fn set_time_scale_permille(&mut self, permille: u32) {
        self.time_scale_permille = permille.min(MAX_TIME_SCALE_PERMILLE);
    }

    /// The current time scale in thousandths.
    pub fn time_scale_permille(&self) -> u32 {
        self.time_scale_permille
    }

    /// Jump the counter to `tick` (loading a save, joining a game in progress) and forget the
    /// leftover time.
    pub fn reset_to_tick(&mut self, tick: u64) {
        self.tick = tick;
        self.accumulator = 0;
        self.pending_steps = 0;
    }

    fn alpha(&self) -> f32 {
        // 24 bits of fraction are exact in an f32 and keep the result below 1.0.
        ((self.accumulator * (1 << 24) / TICK_UNITS) as f32) / (1 << 24) as f32
    }

    /// Turn `real_micros` of real time since the last call into the ticks to run now.
    pub fn advance(&mut self, real_micros: u64) -> FrameTicks {
        if self.paused {
            let ticks = std::mem::take(&mut self.pending_steps);
            self.tick += u64::from(ticks);
            return FrameTicks {
                ticks,
                alpha: self.alpha(),
                dropped_ticks: 0,
            };
        }
        let counted = real_micros.min(self.config.max_frame_micros);
        let units_per_micro =
            u128::from(self.config.tick_hz) * u128::from(self.time_scale_permille);
        let mut dropped_units = u128::from(real_micros - counted) * units_per_micro;
        self.accumulator += u128::from(counted) * units_per_micro;
        let mut due = self.accumulator / TICK_UNITS;
        let cap = u128::from(self.config.max_ticks_per_frame);
        if due > cap {
            dropped_units += (due - cap) * TICK_UNITS;
            due = cap;
        }
        self.accumulator %= TICK_UNITS;
        let ticks = due as u32;
        self.tick += u64::from(ticks);
        FrameTicks {
            ticks,
            alpha: self.alpha(),
            dropped_ticks: u64::try_from(dropped_units / TICK_UNITS).unwrap_or(u64::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(hz: u32) -> TickConfig {
        TickConfig {
            tick_hz: hz,
            max_ticks_per_frame: 8,
            max_frame_micros: 250_000,
        }
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    #[test]
    fn a_bad_configuration_is_refused() {
        assert!(TickLoop::new(config(0)).is_err());
        assert!(TickLoop::new(config(1001)).is_err());
        assert!(
            TickLoop::new(TickConfig {
                max_ticks_per_frame: 0,
                ..config(30)
            })
            .is_err()
        );
        assert!(
            TickLoop::new(TickConfig {
                max_frame_micros: 0,
                ..config(30)
            })
            .is_err()
        );
        assert!(TickLoop::new(config(1)).is_ok());
        assert!(TickLoop::new(config(1000)).is_ok());
    }

    #[test]
    fn a_whole_second_gives_exactly_tick_hz_ticks_whatever_the_frame_rate() {
        for hz in [20_u32, 30, 60, 64, 100, 128] {
            for frame_us in [1_000_u64, 6_944, 16_666, 16_667, 33_333, 100_000, 250_000] {
                // The cap on ticks per frame is for stalls; a 250 ms frame at 128 Hz is 32 ticks.
                let cap = TickConfig {
                    max_ticks_per_frame: 64,
                    ..config(hz)
                };
                let mut l = TickLoop::new(cap).unwrap();
                let (mut total, mut ticks) = (0_u64, 0_u64);
                // Whole frames, then the rest, adding up to exactly one second.
                while total + frame_us <= 1_000_000 {
                    ticks += u64::from(l.advance(frame_us).ticks);
                    total += frame_us;
                }
                ticks += u64::from(l.advance(1_000_000 - total).ticks);
                assert_eq!(ticks, u64::from(hz), "{hz} Hz at {frame_us} us frames");
                assert_eq!(l.tick(), u64::from(hz));
            }
        }
    }

    #[test]
    fn the_total_ticks_do_not_depend_on_how_the_time_is_cut_into_frames() {
        let mut rng = Lcg(5);
        for _ in 0..200 {
            let total: u64 = 5_000_000 + rng.next() % 10_000_000;
            let hz = [24_u32, 30, 60, 90][(rng.next() % 4) as usize];
            let mut l = TickLoop::new(TickConfig {
                max_ticks_per_frame: 1000,
                max_frame_micros: 10_000_000,
                ..config(hz)
            })
            .unwrap();
            let mut left = total;
            while left > 0 {
                let frame = (1 + rng.next() % 60_000).min(left);
                l.advance(frame);
                left -= frame;
            }
            assert_eq!(
                l.tick(),
                total * u64::from(hz) / 1_000_000,
                "{total} us at {hz} Hz"
            );
        }
    }

    #[test]
    fn alpha_is_the_fraction_of_a_tick_left_over() {
        let mut l = TickLoop::new(config(10)).unwrap(); // a tick is 100 ms
        let f = l.advance(50_000);
        assert_eq!(f.ticks, 0);
        assert!((f.alpha - 0.5).abs() < 1e-6, "{}", f.alpha);
        let f = l.advance(75_000); // 125 ms in all
        assert_eq!(f.ticks, 1);
        assert!((f.alpha - 0.25).abs() < 1e-6, "{}", f.alpha);
        for step in 0..1000 {
            let f = l.advance(1 + (step * 37) % 90_000);
            assert!((0.0..1.0).contains(&f.alpha), "{}", f.alpha);
        }
    }

    #[test]
    fn simulation_time_is_the_tick_counter() {
        let mut l = TickLoop::new(config(30)).unwrap();
        assert_eq!(l.sim_micros(), 0);
        l.advance(100_000);
        assert_eq!(l.tick(), 3);
        assert_eq!(l.sim_micros(), 100_000);
        assert_eq!(l.sim_micros_at(30), 1_000_000);
        assert_eq!(
            l.tick_micros(),
            33_333,
            "a tick rounded to whole microseconds, for display only"
        );
    }

    #[test]
    fn pausing_stops_the_ticks_and_the_time_that_passes_is_not_saved_up() {
        let mut l = TickLoop::new(config(30)).unwrap();
        l.advance(50_000);
        let before = l.tick();
        l.pause();
        assert!(l.is_paused());
        for _ in 0..100 {
            let f = l.advance(100_000);
            assert_eq!(f.ticks, 0);
        }
        assert_eq!(l.tick(), before);
        l.resume();
        let f = l.advance(33_334);
        assert_eq!(f.ticks, 1, "no burst of catch-up ticks after a pause");
    }

    #[test]
    fn single_stepping_runs_one_tick_per_request_while_paused() {
        let mut l = TickLoop::new(config(30)).unwrap();
        l.pause();
        assert_eq!(l.advance(16_000).ticks, 0);
        l.step_once();
        assert_eq!(l.advance(16_000).ticks, 1);
        assert_eq!(
            l.advance(16_000).ticks,
            0,
            "only the one that was asked for"
        );
        l.step_once();
        l.step_once();
        assert_eq!(l.advance(0).ticks, 2, "requests add up");
        assert_eq!(l.tick(), 3);
        // Stepping is for a paused game: when running it is ignored rather than adding a tick.
        l.resume();
        l.step_once();
        assert_eq!(l.advance(0).ticks, 0);
        // ...and does not wait around to fire when the game is paused later.
        l.pause();
        assert_eq!(
            l.advance(0).ticks,
            0,
            "a step asked for while running is not saved up"
        );
        // A step asked for and then not used is dropped by resuming.
        l.step_once();
        l.resume();
        l.pause();
        assert_eq!(l.advance(0).ticks, 0, "resuming forgets the unused step");
    }

    #[test]
    fn time_scale_changes_how_many_ticks_per_real_second() {
        for (permille, expect) in [(500_u32, 15_u64), (1000, 30), (2000, 60), (250, 7), (0, 0)] {
            let mut l = TickLoop::new(config(30)).unwrap();
            l.set_time_scale_permille(permille);
            let mut ticks = 0;
            for _ in 0..100 {
                ticks += u64::from(l.advance(10_000).ticks);
            }
            assert_eq!(ticks, expect, "scale {permille}");
        }
        let mut l = TickLoop::new(config(30)).unwrap();
        l.set_time_scale_permille(1_000_000);
        assert_eq!(
            l.time_scale_permille(),
            8000,
            "the scale is limited so one frame cannot ask for a flood of ticks"
        );
    }

    #[test]
    fn a_change_of_scale_applies_from_then_on_and_keeps_the_leftover() {
        let mut l = TickLoop::new(config(10)).unwrap(); // 100 ms a tick
        l.advance(60_000); // 0.6 tick in the bag
        l.set_time_scale_permille(500);
        let f = l.advance(80_000); // 40 ms of simulated time: 1.0 tick in all
        assert_eq!(f.ticks, 1);
        assert!(f.alpha < 1e-6, "{}", f.alpha);
    }

    #[test]
    fn a_stall_cannot_cause_a_spiral_of_catch_up_ticks() {
        let mut l = TickLoop::new(config(30)).unwrap();
        let f = l.advance(10_000_000); // ten seconds frozen
        assert!(f.ticks <= 8, "{}", f.ticks);
        // The frame is clamped to 250 ms first: that is 7 ticks at 30 Hz.
        assert_eq!(f.ticks, 7);
        let next = l.advance(33_334);
        assert_eq!(
            next.ticks, 1,
            "and then it is back to normal, not paying the rest back"
        );

        let mut l = TickLoop::new(TickConfig {
            max_frame_micros: 10_000_000,
            ..config(30)
        })
        .unwrap();
        let f = l.advance(10_000_000);
        assert_eq!(f.ticks, 8, "the tick cap still applies");
        assert!(
            f.dropped_ticks > 200,
            "{} ticks were given up and reported",
            f.dropped_ticks
        );
        assert_eq!(l.advance(33_334).ticks, 1);
    }

    #[test]
    fn jumping_to_a_tick_resets_the_leftover_time() {
        let mut l = TickLoop::new(config(30)).unwrap();
        l.advance(50_000);
        l.reset_to_tick(1000);
        assert_eq!(l.tick(), 1000);
        let f = l.advance(10_000);
        assert_eq!(f.ticks, 0, "the leftover of before the jump is gone");
        assert!((f.alpha - 0.3).abs() < 0.01, "{}", f.alpha);
    }
}
