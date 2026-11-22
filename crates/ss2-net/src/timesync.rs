//! Time synchronization between a client and a server (roadmap T08-016).
//!
//! The client pings the server and the server answers with its own clock; from each round trip
//! the client estimates how far its clock is from the server's ([`ClockSync`]), converts server
//! time to simulation ticks ([`TickClock`]), and decides how long to hold back incoming data to
//! smooth out jitter ([`jitter_buffer_delay_us`]).
//!
//! Like the rest of the crate it is clock-free: every function takes the local time as a number
//! of microseconds, so tests drive it with a virtual clock and the network simulator
//! ([`crate::netsim`]).
//!
//! **What the estimate can and cannot do.** A round trip tells the client the *sum* of the two
//! one-way delays, not each. The estimate assumes they are equal; if the path is asymmetric the
//! offset is off by half the difference, and nothing in the data can reveal that. The estimator
//! therefore picks the sample with the **shortest round trip** (the one least disturbed by
//! queueing), which keeps the error to half the asymmetry of the best path.

use std::collections::VecDeque;

const MILLION: u128 = 1_000_000;

/// How [`ClockSync`] filters, smooths and schedules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncConfig {
    /// How many of the latest round trips are kept. A longer window finds a cleaner sample but
    /// follows a changing path more slowly.
    pub window: usize,
    /// How many samples make the clock [`ClockSync::is_synced`].
    pub min_samples: usize,
    /// The most the published offset moves per sample while it follows a change.
    pub max_slew_us: u64,
    /// A change larger than this is taken at once instead of slewed (the clocks were reset or
    /// the first estimate was wrong, rather than drifting).
    pub step_threshold_us: u64,
    /// The time between pings until the clock is synced.
    pub initial_interval_us: u64,
    /// The time between pings once it is.
    pub steady_interval_us: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            window: 16,
            min_samples: 5,
            max_slew_us: 2_000,
            step_threshold_us: 100_000,
            initial_interval_us: 100_000,
            steady_interval_us: 1_000_000,
        }
    }
}

/// Why a round trip was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncError {
    /// The reply arrived (by the local clock) before the request was sent.
    ReplyBeforeRequest,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReplyBeforeRequest => {
                f.write_str("the reply arrived before the request was sent")
            }
        }
    }
}

impl std::error::Error for SyncError {}

#[derive(Clone, Copy, Debug)]
struct Sample {
    rtt_us: u64,
    /// Server clock minus local clock, as this round trip sees it.
    offset_us: i64,
}

/// Estimates the offset between the local clock and the server's, and the round-trip time, from
/// ping and pong exchanges. See the module documentation for what the estimate can promise.
#[derive(Clone, Debug)]
pub struct ClockSync {
    config: SyncConfig,
    samples: VecDeque<Sample>,
    /// How many round trips have been accepted in all (the window forgets old ones).
    accepted: u64,
    /// The offset handed out: it follows the best sample, slewed.
    offset_us: Option<i64>,
    last_ping_us: Option<u64>,
}

impl ClockSync {
    /// A synchronizer with no samples.
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            samples: VecDeque::new(),
            accepted: 0,
            offset_us: None,
            last_ping_us: None,
        }
    }

    /// Feed one answered ping: `sent_us` is the local time the ping left, `server_time_us` the
    /// server clock stamped on the reply, `received_us` the local time the reply arrived.
    ///
    /// # Errors
    /// [`SyncError::ReplyBeforeRequest`] if `received_us < sent_us`; the sample is ignored.
    pub fn on_pong(
        &mut self,
        sent_us: u64,
        server_time_us: u64,
        received_us: u64,
    ) -> Result<(), SyncError> {
        let rtt_us = received_us
            .checked_sub(sent_us)
            .ok_or(SyncError::ReplyBeforeRequest)?;
        // The server stamped the reply about half a round trip before it was received.
        let offset = i128::from(server_time_us) + i128::from(rtt_us / 2) - i128::from(received_us);
        let offset_us =
            i64::try_from(offset).unwrap_or(if offset < 0 { i64::MIN } else { i64::MAX });
        self.samples.push_back(Sample { rtt_us, offset_us });
        while self.samples.len() > self.config.window.max(1) {
            self.samples.pop_front();
        }
        self.accepted += 1;
        let target = self.best().offset_us;
        self.offset_us = Some(match self.offset_us {
            // Until enough round trips are in, take the best estimate as it is: the first ones are
            // the least reliable, and slewing would only slow the start down.
            None => target,
            Some(_) if !self.is_synced() => target,
            Some(current) => {
                let diff = i128::from(target) - i128::from(current);
                if diff.unsigned_abs() > u128::from(self.config.step_threshold_us) {
                    target
                } else {
                    let step = diff.clamp(
                        -i128::from(self.config.max_slew_us),
                        i128::from(self.config.max_slew_us),
                    );
                    // `current + step` stays between `current` and `target`, so it fits.
                    i64::try_from(i128::from(current) + step).unwrap_or(target)
                }
            }
        });
        Ok(())
    }

    /// The sample with the shortest round trip; the latest of equals, so a changed path wins.
    fn best(&self) -> Sample {
        let mut best = self.samples[0];
        for sample in &self.samples {
            if sample.rtt_us <= best.rtt_us {
                best = *sample;
            }
        }
        best
    }

    /// Whether enough round trips have been seen for the estimate to be trusted.
    pub fn is_synced(&self) -> bool {
        self.accepted >= self.config.min_samples as u64
    }

    /// Server clock minus local clock in microseconds (positive when the server is ahead), once
    /// at least one round trip has been seen.
    pub fn offset_us(&self) -> Option<i64> {
        self.offset_us
    }

    /// The shortest round trip in the window.
    pub fn rtt_us(&self) -> Option<u64> {
        (!self.samples.is_empty()).then(|| self.best().rtt_us)
    }

    /// The mean deviation of the round trips in the window from their mean: how much the delay
    /// varies. It is what sizes the jitter buffer.
    pub fn jitter_us(&self) -> Option<u64> {
        if self.samples.is_empty() {
            return None;
        }
        let n = self.samples.len() as u64;
        let mean = self.samples.iter().map(|s| s.rtt_us).sum::<u64>() / n;
        Some(
            self.samples
                .iter()
                .map(|s| s.rtt_us.abs_diff(mean))
                .sum::<u64>()
                / n,
        )
    }

    /// The server's clock at local time `local_us`, never below zero.
    pub fn server_time_us(&self, local_us: u64) -> Option<u64> {
        let offset = self.offset_us?;
        let t = i128::from(local_us) + i128::from(offset);
        Some(u64::try_from(t.max(0)).unwrap_or(u64::MAX))
    }

    /// Whether it is time to send the next ping: at once for the first, then every
    /// `initial_interval_us` until synced and every `steady_interval_us` afterwards.
    pub fn ping_due(&self, local_now_us: u64) -> bool {
        let Some(last) = self.last_ping_us else {
            return true;
        };
        let interval = if self.is_synced() {
            self.config.steady_interval_us
        } else {
            self.config.initial_interval_us
        };
        local_now_us.saturating_sub(last) >= interval
    }

    /// Record that a ping went out at `local_now_us`.
    pub fn note_ping_sent(&mut self, local_now_us: u64) {
        self.last_ping_us = Some(local_now_us);
    }
}

/// Converts between server time and simulation ticks at a fixed tick rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickClock {
    hz: u32,
}

impl TickClock {
    /// A clock for `hz` ticks per second (a rate of 0 is read as 1).
    pub fn new(hz: u32) -> Self {
        Self { hz: hz.max(1) }
    }

    /// The tick that contains server time `server_us`.
    pub fn tick_at(&self, server_us: u64) -> u64 {
        u64::try_from(u128::from(server_us) * u128::from(self.hz) / MILLION).unwrap_or(u64::MAX)
    }

    /// The first moment of `tick`, so that `tick_at(time_of_tick(n)) == n`.
    pub fn time_of_tick(&self, tick: u64) -> u64 {
        let hz = u128::from(self.hz);
        u64::try_from((u128::from(tick) * MILLION).div_ceil(hz)).unwrap_or(u64::MAX)
    }

    /// The length of a tick, rounded to the nearest microsecond.
    pub fn tick_period_us(&self) -> u64 {
        (MILLION as u64 + u64::from(self.hz) / 2) / u64::from(self.hz)
    }
}

/// How long incoming data is held back to smooth out arrival-time variation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JitterBufferConfig {
    /// The delay with no jitter at all.
    pub min_delay_us: u64,
    /// The most delay worth adding; beyond it late data is dropped instead.
    pub max_delay_us: u64,
    /// How many times the measured jitter is added to the minimum.
    pub jitter_factor: u64,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            min_delay_us: 20_000,
            max_delay_us: 250_000,
            jitter_factor: 3,
        }
    }
}

/// The delay to hold incoming data for, given the measured jitter: `min + factor * jitter`,
/// capped at the maximum (but never below the minimum, even for an inconsistent config).
pub fn jitter_buffer_delay_us(jitter_us: u64, config: &JitterBufferConfig) -> u64 {
    let wanted = config
        .min_delay_us
        .saturating_add(config.jitter_factor.saturating_mul(jitter_us));
    wanted.min(config.max_delay_us).max(config.min_delay_us)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::{Duplex, LinkConfig};

    /// The server's clock is `local + TRUE_OFFSET` for the client.
    const TRUE_OFFSET: i64 = 1_234_567;

    fn config() -> SyncConfig {
        SyncConfig::default()
    }

    /// One ping answered over a path with the given one-way delays: returns what the client
    /// feeds into the estimator.
    fn exchange(sync: &mut ClockSync, t0: u64, up_us: u64, down_us: u64) {
        let server_time = (t0 + up_us).checked_add_signed(TRUE_OFFSET).unwrap();
        let t1 = t0 + up_us + down_us;
        sync.on_pong(t0, server_time, t1).unwrap();
    }

    #[test]
    fn a_symmetric_path_gives_the_exact_offset_and_round_trip() {
        let mut sync = ClockSync::new(config());
        assert!(!sync.is_synced());
        assert_eq!(sync.offset_us(), None);
        for i in 0..5 {
            exchange(&mut sync, 1_000_000 + i * 100_000, 20_000, 20_000);
        }
        assert!(sync.is_synced());
        assert_eq!(sync.offset_us(), Some(TRUE_OFFSET));
        assert_eq!(sync.rtt_us(), Some(40_000));
        assert_eq!(sync.jitter_us(), Some(0));
    }

    #[test]
    fn an_asymmetric_path_is_off_by_half_the_difference_and_no_more() {
        let mut sync = ClockSync::new(config());
        for i in 0..8 {
            exchange(&mut sync, 1_000_000 + i * 100_000, 10_000, 30_000);
        }
        // up 10 ms, down 30 ms: the estimate assumes 20 ms each way, so it is 10 ms too small.
        assert_eq!(sync.offset_us(), Some(TRUE_OFFSET - 10_000));
    }

    #[test]
    fn the_shortest_round_trip_wins_over_the_noisy_ones() {
        let mut sync = ClockSync::new(config());
        // Eight pings with queueing delay on the way back, one clean one.
        for (i, extra) in [30_000, 80_000, 55_000, 0, 70_000, 20_000, 90_000, 45_000]
            .iter()
            .enumerate()
        {
            exchange(
                &mut sync,
                1_000_000 + i as u64 * 100_000,
                20_000,
                20_000 + extra,
            );
        }
        assert_eq!(
            sync.offset_us(),
            Some(TRUE_OFFSET),
            "the clean sample decides"
        );
        assert_eq!(
            sync.rtt_us(),
            Some(40_000),
            "the round trip is the best one in the window"
        );
    }

    #[test]
    fn one_huge_round_trip_does_not_move_the_estimate() {
        let mut sync = ClockSync::new(config());
        for i in 0..6 {
            exchange(&mut sync, 1_000_000 + i * 100_000, 20_000, 20_000);
        }
        let before = sync.offset_us().unwrap();
        exchange(&mut sync, 2_000_000, 20_000, 520_000);
        assert_eq!(sync.offset_us(), Some(before));
    }

    #[test]
    fn jitter_is_the_mean_deviation_of_the_round_trips() {
        let mut sync = ClockSync::new(config());
        for (i, rtt) in [40_000_u64, 60_000, 40_000, 60_000, 40_000, 60_000]
            .iter()
            .enumerate()
        {
            exchange(&mut sync, 1_000_000 + i as u64 * 100_000, rtt / 2, rtt / 2);
        }
        assert_eq!(
            sync.jitter_us(),
            Some(10_000),
            "values 40 and 60 ms around the mean 50 ms"
        );
    }

    #[test]
    fn a_round_trip_that_ends_before_it_began_is_refused() {
        let mut sync = ClockSync::new(config());
        assert_eq!(
            sync.on_pong(5_000, 9_000, 4_000),
            Err(SyncError::ReplyBeforeRequest)
        );
        assert!(
            sync.offset_us().is_none(),
            "a refused sample changes nothing"
        );
    }

    #[test]
    fn server_time_is_local_time_plus_the_offset_and_never_negative() {
        let mut sync = ClockSync::new(config());
        assert_eq!(sync.server_time_us(5_000_000), None, "not synced yet");
        for i in 0..5 {
            exchange(&mut sync, 1_000_000 + i * 100_000, 20_000, 20_000);
        }
        assert_eq!(
            sync.server_time_us(5_000_000),
            Some(5_000_000 + TRUE_OFFSET as u64)
        );
        // A server clock that is behind the client's: the result stops at zero.
        let mut behind = ClockSync::new(config());
        for i in 0..5 {
            let t0 = 10_000_000 + i * 100_000;
            behind.on_pong(t0, 1_000 + 20_000, t0 + 40_000).unwrap();
        }
        assert_eq!(behind.server_time_us(0), Some(0));
    }

    #[test]
    fn a_small_change_is_followed_slowly_and_a_big_one_at_once() {
        let cfg = SyncConfig {
            window: 4,
            ..config()
        };
        let mut sync = ClockSync::new(cfg.clone());
        for i in 0..6 {
            exchange(&mut sync, 1_000_000 + i * 100_000, 20_000, 20_000);
        }
        let start = sync.offset_us().unwrap();
        // The client clock is stepped back by 8 ms: the true offset grows by 8 ms.
        let step = 8_000;
        let mut last = start;
        for i in 0..4_u64 {
            let t0 = 2_000_000 + i * 100_000;
            let server_time = (t0 + 20_000)
                .checked_add_signed(TRUE_OFFSET + step)
                .unwrap();
            sync.on_pong(t0, server_time, t0 + 40_000).unwrap();
            let now = sync.offset_us().unwrap();
            assert!(
                now - last <= cfg.max_slew_us as i64,
                "moved {} at once",
                now - last
            );
            last = now;
        }
        assert!(last > start, "it follows the change");
        // A jump of a second is taken at once (the clocks were reset, not drifting).
        let t0 = 3_000_000_u64;
        let server_time = (t0 + 20_000)
            .checked_add_signed(TRUE_OFFSET + 1_000_000)
            .unwrap();
        for k in 0..4 {
            sync.on_pong(
                t0 + k * 100_000,
                server_time + k * 100_000,
                t0 + k * 100_000 + 40_000,
            )
            .unwrap();
        }
        assert_eq!(sync.offset_us(), Some(TRUE_OFFSET + 1_000_000));
    }

    #[test]
    fn the_estimate_follows_a_path_change_once_old_samples_age_out() {
        let cfg = SyncConfig {
            window: 6,
            ..config()
        };
        let mut sync = ClockSync::new(cfg);
        for i in 0..6 {
            exchange(&mut sync, 1_000_000 + i * 100_000, 10_000, 10_000);
        }
        assert_eq!(sync.rtt_us(), Some(20_000));
        for i in 0..6 {
            exchange(&mut sync, 2_000_000 + i * 100_000, 50_000, 50_000);
        }
        assert_eq!(
            sync.rtt_us(),
            Some(100_000),
            "the old, shorter round trips are gone"
        );
    }

    #[test]
    fn pings_are_frequent_at_first_and_then_settle() {
        let cfg = config();
        let mut sync = ClockSync::new(cfg.clone());
        assert!(sync.ping_due(0), "the first ping goes out at once");
        sync.note_ping_sent(0);
        assert!(!sync.ping_due(cfg.initial_interval_us - 1));
        assert!(sync.ping_due(cfg.initial_interval_us));
        let mut now = 0;
        let mut sent = 1;
        while !sync.is_synced() {
            now += cfg.initial_interval_us;
            assert!(sync.ping_due(now));
            sync.note_ping_sent(now);
            exchange(&mut sync, now, 5_000, 5_000);
            sent += 1;
            assert!(sent < 100, "never synced");
        }
        sync.note_ping_sent(now);
        assert!(!sync.ping_due(now + cfg.steady_interval_us - 1));
        assert!(sync.ping_due(now + cfg.steady_interval_us));
    }

    #[test]
    fn the_estimate_is_good_over_a_lossy_jittery_link_with_a_drifting_clock() {
        // Server time = local time * (1 + 100 ppm) + TRUE_OFFSET, over 150 s, with 40 ms of jitter
        // each way and 5 % loss, pinged by the schedule. Round trips come from the simulator.
        let link_config = LinkConfig {
            latency_us: 25_000,
            jitter_us: 40_000,
            loss_ppm: 50_000,
            ..LinkConfig::default()
        };
        let mut link = Duplex::<(u64, u64)>::symmetric(link_config, 2024);
        let mut sync = ClockSync::new(config());
        let step = 10_000_u64; // the test loop advances 10 ms at a time
        let mut worst_after_warmup: i64 = 0;
        let mut now = 0_u64;
        while now < 150_000_000 {
            if sync.ping_due(now) {
                sync.note_ping_sent(now);
                link.a_to_b.send(now, (now, 0), 32);
            }
            for (at, (t0, _)) in link.a_to_b.poll(now) {
                let server_clock = at + at / 10_000 + TRUE_OFFSET as u64; // 100 ppm fast
                link.b_to_a.send(at, (t0, server_clock), 32);
            }
            for (at, (t0, server_clock)) in link.b_to_a.poll(now) {
                sync.on_pong(t0, server_clock, at).unwrap();
            }
            if now > 20_000_000
                && let Some(estimate) = sync.server_time_us(now)
            {
                let truth = now + now / 10_000 + TRUE_OFFSET as u64;
                worst_after_warmup = worst_after_warmup.max((estimate as i64 - truth as i64).abs());
            }
            now += step;
        }
        assert!(sync.is_synced());
        assert!(
            worst_after_warmup < 20_000,
            "worst error {worst_after_warmup} us"
        );
    }

    #[test]
    fn ticks_follow_server_time_and_round_trip() {
        for hz in [20_u32, 30, 60, 64, 128] {
            let clock = TickClock::new(hz);
            assert_eq!(clock.tick_at(1_000_000), u64::from(hz));
            for tick in [0_u64, 1, 7, 59, 60, 12_345, 1_000_000_000] {
                let at = clock.time_of_tick(tick);
                assert_eq!(clock.tick_at(at), tick, "{hz} Hz tick {tick}");
                assert!(
                    at == 0 || clock.tick_at(at - 1) < tick,
                    "{hz} Hz: time_of_tick is the first moment of the tick"
                );
            }
        }
        assert_eq!(TickClock::new(60).tick_period_us(), 16_667);
        assert_eq!(
            TickClock::new(0).tick_at(5_000_000),
            5,
            "a zero rate is read as 1 Hz"
        );
    }

    #[test]
    fn the_jitter_buffer_grows_with_jitter_inside_its_limits() {
        let cfg = JitterBufferConfig {
            min_delay_us: 20_000,
            max_delay_us: 200_000,
            jitter_factor: 3,
        };
        assert_eq!(jitter_buffer_delay_us(0, &cfg), 20_000);
        assert_eq!(jitter_buffer_delay_us(10_000, &cfg), 50_000);
        assert_eq!(jitter_buffer_delay_us(1_000_000, &cfg), 200_000, "capped");
        let mut last = 0;
        for jitter in (0..200_000).step_by(5_000) {
            let d = jitter_buffer_delay_us(jitter, &cfg);
            assert!(d >= last, "the delay never shrinks as jitter grows");
            last = d;
        }
        let odd = JitterBufferConfig {
            min_delay_us: 300_000,
            max_delay_us: 100_000,
            jitter_factor: 2,
        };
        assert_eq!(
            jitter_buffer_delay_us(5_000, &odd),
            300_000,
            "a minimum above the maximum wins"
        );
    }
}
