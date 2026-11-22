//! Fixed-timestep driver and frame pacing (T03-013).
//!
//! Both types are **pure**: they never read a clock. The platform loop measures how long the
//! last frame took and passes that [`Duration`] in, so the logic is deterministic, trivially
//! testable and safe to use from crates that must not touch wall-clock time.
//!
//! * [`FixedStep`] turns variable frame times into a whole number of fixed simulation ticks plus
//!   an interpolation factor for rendering between the last two ticks.
//! * [`FramePacer`] says how long to sleep and how long to spin to hit a frame-rate cap.

use std::time::Duration;

use crate::{Error, Result};

/// One tick expressed in the accumulator's unit (nanosecond-hertz): `delta_ns * hz` adds up to
/// exactly `1_000_000_000` per tick for any whole tick rate, so no step is ever rounded.
const UNITS_PER_TICK: u128 = 1_000_000_000;

/// What one frame asks the simulation to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepResult {
    /// Whole simulation ticks to run now (at most the catch-up limit).
    pub ticks: u32,
    /// How far the render time is between the last tick and the next one, in `0.0..1.0`:
    /// interpolate the previous and current simulation state by this factor.
    pub alpha: f32,
    /// Time that was discarded because the frame was longer than the catch-up limit allows
    /// (the "spiral of death" guard). Worth logging when it is not zero.
    pub dropped: Duration,
}

/// Accumulator-based fixed-timestep driver.
///
/// ```
/// use ss2_core::time::FixedStep;
/// use std::time::Duration;
///
/// let mut step = FixedStep::new(60, 5).unwrap();
/// let frame = step.advance(Duration::from_millis(20));
/// assert_eq!(frame.ticks, 1); // 20 ms is one 16.7 ms tick and a bit
/// assert!(frame.alpha > 0.0 && frame.alpha < 1.0);
/// ```
#[derive(Clone, Debug)]
pub struct FixedStep {
    rate_hz: u32,
    max_ticks: u32,
    /// Accumulated time in [`UNITS_PER_TICK`] units, always below one tick after `advance`.
    acc: u128,
}

impl FixedStep {
    /// A driver that runs `rate_hz` ticks per second and never more than `max_ticks_per_frame`
    /// ticks in one frame.
    ///
    /// # Errors
    /// [`Error::Invalid`] if either argument is zero.
    pub fn new(rate_hz: u32, max_ticks_per_frame: u32) -> Result<Self> {
        if rate_hz == 0 {
            return Err(Error::invalid("tick rate", "must be at least 1 Hz"));
        }
        if max_ticks_per_frame == 0 {
            return Err(Error::invalid(
                "catch-up limit",
                "must allow at least 1 tick per frame",
            ));
        }
        Ok(Self {
            rate_hz,
            max_ticks: max_ticks_per_frame,
            acc: 0,
        })
    }

    /// Ticks per second.
    pub fn tick_rate(&self) -> u32 {
        self.rate_hz
    }

    /// The length of one tick, rounded to the nearest nanosecond (for display and for scaling
    /// simulation values; the driver itself does not use the rounded value).
    pub fn tick_duration(&self) -> Duration {
        let hz = u64::from(self.rate_hz);
        Duration::from_nanos((1_000_000_000 + hz / 2) / hz)
    }

    /// Add `delta` (the wall-clock time of the last frame) and report how many ticks to run.
    pub fn advance(&mut self, delta: Duration) -> StepResult {
        let hz = u128::from(self.rate_hz);
        self.acc += delta.as_nanos() * hz;

        let max_units = UNITS_PER_TICK * u128::from(self.max_ticks);
        let mut dropped = Duration::ZERO;
        if self.acc > max_units {
            let lost_units = self.acc - max_units;
            self.acc = max_units;
            dropped = Duration::from_nanos(u64::try_from(lost_units / hz).unwrap_or(u64::MAX));
        }

        // `self.acc <= max_units`, so the tick count fits in a u32.
        let ticks = u32::try_from(self.acc / UNITS_PER_TICK).unwrap_or(self.max_ticks);
        self.acc %= UNITS_PER_TICK;
        let alpha = (self.acc as f64 / UNITS_PER_TICK as f64) as f32;
        StepResult {
            ticks,
            alpha,
            dropped,
        }
    }

    /// Forget accumulated time, for example after a loading screen or when resuming from
    /// suspend, so the next frame does not try to catch up.
    pub fn reset(&mut self) {
        self.acc = 0;
    }
}

/// How the presentation rate is limited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePacing {
    /// The display's vertical sync paces the loop; the loop itself never waits.
    Vsync,
    /// Render as fast as possible; the loop never waits.
    Uncapped,
    /// Cap at `fps` frames per second by waiting.
    Capped {
        /// Target frames per second (at least 1).
        fps: u32,
    },
}

/// How long to wait at the end of a frame: sleep first (cheap, but the OS may oversleep by a
/// millisecond or more), then spin for the final stretch (precise, but burns a core).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PacePlan {
    /// Time to give back to the OS with a sleep.
    pub sleep: Duration,
    /// Time to busy-wait after the sleep.
    pub spin: Duration,
}

/// Turns "this frame took `elapsed`" into a [`PacePlan`] for a [`FramePacing`] mode.
#[derive(Clone, Copy, Debug)]
pub struct FramePacer {
    mode: FramePacing,
    spin_margin: Duration,
}

impl FramePacer {
    /// `spin_margin` is the part of the wait that is spun rather than slept (about the OS
    /// sleep granularity: 1 to 2 ms is typical).
    ///
    /// # Errors
    /// [`Error::Invalid`] for a frame-rate cap of 0.
    pub fn new(mode: FramePacing, spin_margin: Duration) -> Result<Self> {
        if matches!(mode, FramePacing::Capped { fps: 0 }) {
            return Err(Error::invalid("frame rate cap", "must be at least 1 fps"));
        }
        Ok(Self { mode, spin_margin })
    }

    /// The wait for a frame that has already taken `elapsed`.
    pub fn plan(&self, elapsed: Duration) -> PacePlan {
        let FramePacing::Capped { fps } = self.mode else {
            return PacePlan::default();
        };
        let target = Duration::from_nanos(1_000_000_000 / u64::from(fps));
        let remaining = target.saturating_sub(elapsed);
        let spin = remaining.min(self.spin_margin);
        PacePlan {
            sleep: remaining - spin,
            spin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn rejects_a_zero_rate_or_a_zero_catch_up_limit() {
        assert!(FixedStep::new(0, 5).is_err());
        assert!(FixedStep::new(60, 0).is_err());
        assert!(FixedStep::new(60, 5).is_ok());
    }

    #[test]
    fn one_exact_step_is_one_tick_with_no_remainder() {
        let mut s = FixedStep::new(100, 5).unwrap(); // 10 ms per tick
        let r = s.advance(ms(10));
        assert_eq!(r.ticks, 1);
        assert_eq!(r.alpha, 0.0);
        assert_eq!(r.dropped, Duration::ZERO);
    }

    #[test]
    fn partial_frames_accumulate_into_whole_ticks() {
        let mut s = FixedStep::new(100, 5).unwrap(); // 10 ms
        let a = s.advance(ms(4));
        assert_eq!((a.ticks, a.alpha), (0, 0.4));
        let b = s.advance(ms(8)); // 12 ms in total
        assert_eq!(b.ticks, 1);
        assert!((b.alpha - 0.2).abs() < 1e-6, "{}", b.alpha);
    }

    #[test]
    fn a_whole_second_of_odd_frames_is_exactly_the_tick_rate() {
        // 60 Hz is not a whole number of nanoseconds per tick (16_666_666.67 ns): the driver must
        // not truncate the step, or the simulation drifts against the wall clock.
        let mut s = FixedStep::new(60, 10).unwrap();
        let mut ticks = 0;
        for _ in 0..1000 {
            ticks += s.advance(ms(1)).ticks;
        }
        assert_eq!(ticks, 60);
        let mut s = FixedStep::new(7, 10).unwrap();
        let mut ticks = 0;
        for _ in 0..3000 {
            ticks += s.advance(Duration::from_micros(333)).ticks;
        }
        // 3000 * 333 us = 0.999 s at 7 Hz: 6 whole ticks and 0.993 of the next.
        assert_eq!(ticks, 6);
    }

    #[test]
    fn a_long_frame_is_clamped_and_the_lost_time_is_reported() {
        let mut s = FixedStep::new(100, 5).unwrap(); // 10 ms, at most 5 ticks per frame
        let r = s.advance(Duration::from_secs(1));
        assert_eq!(r.ticks, 5);
        assert_eq!(r.alpha, 0.0);
        assert_eq!(r.dropped, ms(950));
        // the next frame starts from a clean slate: no further catch-up
        assert_eq!(s.advance(ms(10)).ticks, 1);
    }

    #[test]
    fn alpha_stays_below_one_and_ticks_never_exceed_the_limit() {
        let mut s = FixedStep::new(30, 3).unwrap();
        let mut seed = 12345_u64;
        for _ in 0..10_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let r = s.advance(Duration::from_micros((seed >> 40) % 200_000));
            assert!((0.0..1.0).contains(&r.alpha), "alpha {}", r.alpha);
            assert!(r.ticks <= 3, "ticks {}", r.ticks);
        }
    }

    #[test]
    fn a_zero_delta_changes_nothing() {
        let mut s = FixedStep::new(100, 5).unwrap();
        s.advance(ms(4));
        let r = s.advance(Duration::ZERO);
        assert_eq!((r.ticks, r.alpha), (0, 0.4));
    }

    #[test]
    fn reset_drops_the_accumulated_time() {
        let mut s = FixedStep::new(100, 5).unwrap();
        s.advance(ms(9));
        s.reset();
        let r = s.advance(ms(1));
        assert_eq!((r.ticks, r.alpha), (0, 0.1));
    }

    #[test]
    fn tick_duration_is_the_rounded_step() {
        assert_eq!(FixedStep::new(100, 1).unwrap().tick_duration(), ms(10));
        assert_eq!(
            FixedStep::new(60, 1).unwrap().tick_duration(),
            Duration::from_nanos(16_666_667)
        );
        assert_eq!(FixedStep::new(60, 1).unwrap().tick_rate(), 60);
    }

    #[test]
    fn uncapped_and_vsync_never_ask_the_loop_to_wait() {
        for mode in [FramePacing::Uncapped, FramePacing::Vsync] {
            let p = FramePacer::new(mode, ms(2)).unwrap();
            assert_eq!(p.plan(Duration::ZERO), PacePlan::default());
            assert_eq!(p.plan(ms(5)), PacePlan::default());
        }
    }

    #[test]
    fn a_capped_frame_sleeps_most_of_the_slack_and_spins_the_rest() {
        let p = FramePacer::new(FramePacing::Capped { fps: 100 }, ms(2)).unwrap(); // 10 ms frames
        assert_eq!(
            p.plan(ms(4)),
            PacePlan {
                sleep: ms(4),
                spin: ms(2)
            }
        );
    }

    #[test]
    fn a_small_slack_is_all_spin_and_a_late_frame_waits_for_nothing() {
        let p = FramePacer::new(FramePacing::Capped { fps: 100 }, ms(2)).unwrap();
        assert_eq!(
            p.plan(ms(9)),
            PacePlan {
                sleep: Duration::ZERO,
                spin: ms(1)
            }
        );
        assert_eq!(p.plan(ms(10)), PacePlan::default());
        assert_eq!(p.plan(ms(50)), PacePlan::default());
    }

    #[test]
    fn a_zero_frame_rate_cap_is_rejected() {
        assert!(FramePacer::new(FramePacing::Capped { fps: 0 }, ms(1)).is_err());
    }
}
