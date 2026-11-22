//! Showing remote entities smoothly from snapshots that arrive a few times per second
//! (roadmap T08-008).
//!
//! The server sends the state of an entity every few ticks. A client does not draw the newest
//! snapshot: it draws the entity as it was a little in the past (the **interpolation delay**),
//! blending between the two snapshots around that moment ([`SnapshotBuffer::sample`]). When the
//! next snapshot is late the buffer **extrapolates** for a short time and then holds the last
//! state; a **teleport** (a respawn, a portal) is shown as a jump, not a streak; a **despawn**
//! ends the entity's life instead of freezing it. [`InterpolationDelay`] picks the delay from
//! the measured jitter: it grows at once when the network gets worse and shrinks slowly when it
//! gets better, so the picture does not hitch.
//!
//! The state type is generic ([`Interpolate`]); times are microseconds of server time (see
//! `timesync`). Presentation only: nothing here feeds back into the simulation.

use std::collections::BTreeMap;

const MILLION: u64 = 1_000_000;

/// A state that can be shown between two snapshots.
pub trait Interpolate: Clone {
    /// The state `fraction_ppm` parts per million of the way from `self` to `to` (0 is `self`,
    /// 1,000,000 is `to`).
    fn lerp(&self, to: &Self, fraction_ppm: u32) -> Self;

    /// A guess at the state `ahead_us` after `self`, given the `previous` snapshot that was
    /// `previous_to_self_us` before it; `None` when the state cannot be extrapolated (a flag, a
    /// weapon choice), in which case the last state is held.
    fn extrapolate(
        &self,
        _previous: &Self,
        _previous_to_self_us: u64,
        _ahead_us: u64,
    ) -> Option<Self> {
        None
    }
}

/// How a [`SnapshotBuffer`] behaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotConfig {
    /// The most snapshots kept; the oldest are forgotten.
    pub capacity: usize,
    /// How long past the newest snapshot the state is extrapolated before it is held still.
    pub max_extrapolation_us: u64,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            capacity: 64,
            max_extrapolation_us: 100_000,
        }
    }
}

/// Facts about a snapshot that change how it is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnapshotFlags {
    /// The entity jumped to this state (respawn, portal): do not blend towards it.
    pub teleport: bool,
}

/// What to draw for an entity at a render time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sample<S> {
    /// The entity does not exist at this time (nothing received yet, or it despawned).
    Gone,
    /// Before the first snapshot, across a teleport, or past the newest state that cannot be
    /// extrapolated: the nearest known state.
    Held(S),
    /// Blended between two snapshots (also exactly on one).
    Interpolated(S),
    /// Past the newest snapshot, guessed forward for up to `max_extrapolation_us`.
    Extrapolated(S),
    /// Past the newest snapshot by more than the extrapolation limit: the data is stale and the
    /// entity holds still (a sign of a stalled connection).
    Stale(S),
}

#[derive(Clone, Debug)]
struct Entry<S> {
    time_us: u64,
    state: S,
    flags: SnapshotFlags,
}

/// The recent snapshots of one entity.
#[derive(Clone, Debug)]
pub struct SnapshotBuffer<S> {
    config: SnapshotConfig,
    /// By tick, so a snapshot that arrives twice or out of order lands in the right place.
    entries: BTreeMap<u64, Entry<S>>,
    despawn_at_us: Option<u64>,
}

impl<S: Interpolate> SnapshotBuffer<S> {
    /// An empty buffer.
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            config,
            entries: BTreeMap::new(),
            despawn_at_us: None,
        }
    }

    /// Add the snapshot of `tick`, taken at server time `time_us`. A tick that is already held is
    /// ignored (the first copy stays).
    pub fn push(&mut self, tick: u64, time_us: u64, state: S, flags: SnapshotFlags) {
        self.entries.entry(tick).or_insert(Entry {
            time_us,
            state,
            flags,
        });
        while self.entries.len() > self.config.capacity.max(2) {
            self.entries.pop_first();
        }
    }

    /// The entity ceases to exist at server time `time_us`.
    pub fn despawn(&mut self, time_us: u64) {
        self.despawn_at_us = Some(time_us);
    }

    /// Whether the entity has despawned by server time `time_us`.
    pub fn is_gone_at(&self, time_us: u64) -> bool {
        self.despawn_at_us.is_some_and(|at| time_us >= at)
    }

    /// How many snapshots are held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no snapshot is held.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// What to draw at server time `render_time_us` (the current server time minus the
    /// interpolation delay).
    pub fn sample(&self, render_time_us: u64) -> Sample<S> {
        if self.is_gone_at(render_time_us) {
            return Sample::Gone;
        }
        let Some(first) = self.entries.values().next() else {
            return Sample::Gone;
        };
        if render_time_us < first.time_us {
            return Sample::Held(first.state.clone());
        }
        let mut previous: Option<&Entry<S>> = None;
        for entry in self.entries.values() {
            if entry.time_us >= render_time_us {
                let Some(before) = previous else {
                    return Sample::Interpolated(entry.state.clone());
                };
                if entry.flags.teleport && render_time_us < entry.time_us {
                    return Sample::Held(before.state.clone());
                }
                let span = (entry.time_us - before.time_us).max(1);
                let fraction = ((render_time_us - before.time_us) * MILLION / span).min(MILLION);
                return Sample::Interpolated(before.state.lerp(&entry.state, fraction as u32));
            }
            previous = Some(entry);
        }
        // Past the newest snapshot.
        let mut newest_first = self.entries.values().rev();
        let last = newest_first.next().expect("the buffer is not empty");
        let ahead = render_time_us - last.time_us;
        if ahead > self.config.max_extrapolation_us {
            return Sample::Stale(last.state.clone());
        }
        let guess = newest_first
            .next()
            .filter(|_| !last.flags.teleport)
            .and_then(|before| {
                last.state
                    .extrapolate(&before.state, last.time_us - before.time_us, ahead)
            });
        match guess {
            Some(state) => Sample::Extrapolated(state),
            None => Sample::Held(last.state.clone()),
        }
    }
}

/// How [`InterpolationDelay`] chooses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelayConfig {
    /// How many snapshot intervals of data to keep in hand (2 means a late snapshot can be
    /// covered by the one before it).
    pub snapshots_in_buffer: u64,
    /// How many times the measured jitter is added.
    pub jitter_factor: u64,
    /// The delay is never below this.
    pub min_delay_us: u64,
    /// The delay is never above this (beyond it, play looks too laggy to be worth smoothing).
    pub max_delay_us: u64,
}

impl Default for DelayConfig {
    fn default() -> Self {
        Self {
            snapshots_in_buffer: 2,
            jitter_factor: 2,
            min_delay_us: 20_000,
            max_delay_us: 400_000,
        }
    }
}

/// Chooses the interpolation delay from the snapshot interval and the measured jitter: it rises
/// at once and falls by at most a tenth per update, so the picture does not hitch.
#[derive(Clone, Debug)]
pub struct InterpolationDelay {
    config: DelayConfig,
    current_us: u64,
}

impl InterpolationDelay {
    /// A chooser that starts at the minimum.
    pub fn new(config: DelayConfig) -> Self {
        let current_us = config.min_delay_us;
        Self { config, current_us }
    }

    /// Update with the latest jitter (see `ClockSync::jitter_us`) and the time between
    /// snapshots; returns the delay to use.
    pub fn update(&mut self, jitter_us: u64, snapshot_interval_us: u64) -> u64 {
        let wanted = self
            .config
            .snapshots_in_buffer
            .saturating_mul(snapshot_interval_us)
            .saturating_add(self.config.jitter_factor.saturating_mul(jitter_us))
            .clamp(
                self.config.min_delay_us,
                self.config.max_delay_us.max(self.config.min_delay_us),
            );
        self.current_us = if wanted >= self.current_us {
            wanted
        } else {
            let step = (self.current_us / 10).max(1);
            self.current_us.saturating_sub(step).max(wanted)
        };
        self.current_us
    }
}

/// Turns the server time and the delay into a render time that never goes backwards, so that a
/// rise of the delay holds the picture for a moment instead of rewinding it.
#[derive(Clone, Debug, Default)]
pub struct RenderClock {
    last_us: u64,
}

impl RenderClock {
    /// The server time to draw at: `server_now - delay`, but never earlier than before.
    pub fn render_time(&mut self, server_now_us: u64, delay_us: u64) -> u64 {
        self.last_us = self.last_us.max(server_now_us.saturating_sub(delay_us));
        self.last_us
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::{LinkConfig, NetSim};

    /// A one-dimensional position with an optional velocity estimate for extrapolation.
    #[derive(Clone, Debug, PartialEq)]
    struct Pos(i64);

    impl Interpolate for Pos {
        fn lerp(&self, to: &Self, fraction_ppm: u32) -> Self {
            Pos(self.0 + (to.0 - self.0) * i64::from(fraction_ppm) / 1_000_000)
        }

        fn extrapolate(
            &self,
            previous: &Self,
            previous_to_self_us: u64,
            ahead_us: u64,
        ) -> Option<Self> {
            let velocity_num = self.0 - previous.0;
            Some(Pos(self.0
                + velocity_num * ahead_us as i64
                    / previous_to_self_us.max(1) as i64))
        }
    }

    fn buffer() -> SnapshotBuffer<Pos> {
        SnapshotBuffer::new(SnapshotConfig::default())
    }

    fn snap(buffer: &mut SnapshotBuffer<Pos>, tick: u64, time_us: u64, value: i64) {
        buffer.push(tick, time_us, Pos(value), SnapshotFlags::default());
    }

    #[test]
    fn an_empty_buffer_has_nothing_to_show() {
        assert_eq!(buffer().sample(1_000), Sample::Gone);
    }

    #[test]
    fn it_blends_between_the_two_snapshots_around_the_render_time() {
        let mut b = buffer();
        snap(&mut b, 1, 100_000, 0);
        snap(&mut b, 2, 150_000, 100);
        assert_eq!(b.sample(100_000), Sample::Interpolated(Pos(0)));
        assert_eq!(b.sample(125_000), Sample::Interpolated(Pos(50)));
        assert_eq!(b.sample(150_000), Sample::Interpolated(Pos(100)));
        assert_eq!(b.sample(110_000), Sample::Interpolated(Pos(20)));
    }

    #[test]
    fn before_the_first_snapshot_it_shows_the_first_state() {
        let mut b = buffer();
        snap(&mut b, 5, 500_000, 7);
        assert_eq!(b.sample(100_000), Sample::Held(Pos(7)));
    }

    #[test]
    fn snapshots_may_arrive_out_of_order_and_twice() {
        let mut b = buffer();
        snap(&mut b, 3, 200_000, 200);
        snap(&mut b, 1, 100_000, 0);
        snap(&mut b, 2, 150_000, 100);
        snap(&mut b, 2, 150_000, 999); // a repeat of tick 2: the first one stays
        assert_eq!(b.len(), 3);
        assert_eq!(b.sample(125_000), Sample::Interpolated(Pos(50)));
        assert_eq!(b.sample(175_000), Sample::Interpolated(Pos(150)));
    }

    #[test]
    fn a_teleport_is_a_jump_not_a_streak() {
        let mut b = buffer();
        snap(&mut b, 1, 100_000, 0);
        b.push(2, 150_000, Pos(10_000), SnapshotFlags { teleport: true });
        assert_eq!(
            b.sample(149_999),
            Sample::Held(Pos(0)),
            "no blending towards the new place"
        );
        assert_eq!(b.sample(150_000), Sample::Interpolated(Pos(10_000)));
        snap(&mut b, 3, 200_000, 10_100);
        assert_eq!(
            b.sample(175_000),
            Sample::Interpolated(Pos(10_050)),
            "normal blending resumes after it"
        );
    }

    #[test]
    fn a_late_snapshot_is_covered_by_extrapolation_for_a_short_time_only() {
        let mut b = buffer();
        snap(&mut b, 1, 100_000, 0);
        snap(&mut b, 2, 150_000, 100);
        // 2 units per ms: 20 ms past the last snapshot is 40 units on.
        assert_eq!(b.sample(170_000), Sample::Extrapolated(Pos(140)));
        let limit = SnapshotConfig::default().max_extrapolation_us;
        assert!(matches!(b.sample(150_000 + limit), Sample::Extrapolated(_)));
        assert_eq!(
            b.sample(150_000 + limit + 1),
            Sample::Stale(Pos(100)),
            "after that the entity holds still"
        );
    }

    #[test]
    fn a_state_that_cannot_extrapolate_holds_the_last_state() {
        #[derive(Clone, Debug, PartialEq)]
        struct Flag(bool);
        impl Interpolate for Flag {
            fn lerp(&self, to: &Self, f: u32) -> Self {
                if f < 500_000 {
                    self.clone()
                } else {
                    to.clone()
                }
            }
        }
        let mut b = SnapshotBuffer::<Flag>::new(SnapshotConfig::default());
        b.push(1, 100_000, Flag(false), SnapshotFlags::default());
        b.push(2, 150_000, Flag(true), SnapshotFlags::default());
        assert_eq!(b.sample(160_000), Sample::Held(Flag(true)));
    }

    #[test]
    fn a_despawned_entity_disappears_instead_of_freezing() {
        let mut b = buffer();
        snap(&mut b, 1, 100_000, 0);
        snap(&mut b, 2, 150_000, 100);
        b.despawn(150_000);
        assert!(
            matches!(b.sample(140_000), Sample::Interpolated(_)),
            "still alive until then"
        );
        assert_eq!(b.sample(150_000), Sample::Gone);
        assert_eq!(b.sample(900_000), Sample::Gone);
        assert!(b.is_gone_at(150_000));
        assert!(!b.is_gone_at(149_999));
    }

    #[test]
    fn the_buffer_is_bounded_and_forgets_the_oldest() {
        let mut b = SnapshotBuffer::<Pos>::new(SnapshotConfig {
            capacity: 8,
            ..SnapshotConfig::default()
        });
        for t in 0..100_u64 {
            snap(&mut b, t, t * 50_000, t as i64);
        }
        assert_eq!(b.len(), 8);
        assert_eq!(b.sample(99 * 50_000), Sample::Interpolated(Pos(99)));
        assert_eq!(
            b.sample(0),
            Sample::Held(Pos(92)),
            "the oldest kept snapshot"
        );
    }

    #[test]
    fn the_delay_grows_at_once_shrinks_slowly_and_stays_in_bounds() {
        let cfg = DelayConfig::default();
        let mut delay = InterpolationDelay::new(cfg.clone());
        let steady = delay.update(0, 50_000);
        assert_eq!(
            steady,
            cfg.snapshots_in_buffer * 50_000,
            "with no jitter: the snapshots to keep in hand"
        );
        let bad = delay.update(80_000, 50_000);
        assert!(bad > steady, "bad jitter raises the delay at once");
        let mut last = bad;
        for _ in 0..200 {
            let d = delay.update(0, 50_000);
            assert!(d <= last, "the delay only falls while the network is good");
            assert!(last - d <= last / 10 + 1, "and slowly: {last} to {d}");
            last = d;
        }
        assert_eq!(last, steady, "it settles back");
        assert_eq!(delay.update(10_000_000, 50_000), cfg.max_delay_us, "capped");
        let mut tiny = InterpolationDelay::new(cfg.clone());
        assert_eq!(
            tiny.update(0, 1_000),
            cfg.min_delay_us,
            "never below the minimum"
        );
    }

    #[test]
    fn the_render_clock_never_goes_backwards_when_the_delay_changes() {
        let mut clock = RenderClock::default();
        assert_eq!(clock.render_time(1_000_000, 100_000), 900_000);
        assert_eq!(clock.render_time(1_010_000, 100_000), 910_000);
        // The delay jumps up by 50 ms: the render time holds instead of jumping back.
        assert_eq!(clock.render_time(1_020_000, 150_000), 910_000);
        assert_eq!(clock.render_time(1_100_000, 150_000), 950_000);
        assert_eq!(
            RenderClock::default().render_time(10, 500),
            0,
            "never below zero"
        );
    }

    #[test]
    fn adaptive_delay_removes_the_stalls_that_a_fixed_two_snapshot_delay_has() {
        // 20 snapshots per second over 30 ms of latency and up to 60 ms of jitter, 2 % loss, drawn at 60 Hz.
        let stalls = |adaptive: bool| {
            let link = LinkConfig {
                latency_us: 30_000,
                jitter_us: 60_000,
                loss_ppm: 20_000,
                ..LinkConfig::default()
            };
            let mut net = NetSim::<(u64, i64)>::new(link, 99);
            let mut b = buffer();
            let mut delay = InterpolationDelay::new(DelayConfig::default());
            let mut clock = RenderClock::default();
            let mut jitter_us = 0_u64;
            let mut last_arrival: Option<u64> = None;
            let mut bad = 0_u32;
            let mut frames = 0_u32;
            let step = 16_667_u64;
            let mut next_send = 0_u64;
            let mut tick = 0_u64;
            let mut now = 0_u64;
            while now < 30_000_000 {
                while next_send <= now {
                    net.send(next_send, (next_send, tick as i64 * 10), 100);
                    tick += 1;
                    next_send += 50_000;
                }
                for (arrival, (server_time, value)) in net.poll(now) {
                    b.push(
                        server_time / 50_000,
                        server_time,
                        Pos(value),
                        SnapshotFlags::default(),
                    );
                    if let Some(prev) = last_arrival {
                        // The variation of the time between arrivals around the 50 ms they were sent at.
                        let gap = arrival.abs_diff(prev);
                        jitter_us = (jitter_us * 7 + gap.abs_diff(50_000)) / 8;
                    }
                    last_arrival = Some(arrival);
                }
                let wanted = if adaptive {
                    delay.update(jitter_us * 4, 50_000)
                } else {
                    100_000
                };
                // The server clock is known exactly here: the time sync is tested on its own.
                let render = clock.render_time(now, wanted);
                if now > 2_000_000 {
                    frames += 1;
                    if !matches!(b.sample(render), Sample::Interpolated(_)) {
                        bad += 1;
                    }
                }
                now += step;
            }
            f64::from(bad) / f64::from(frames)
        };
        let fixed = stalls(false);
        let adaptive = stalls(true);
        assert!(
            fixed > 0.05,
            "a fixed 100 ms delay (two snapshots) over this link shows stalls: {fixed}"
        );
        assert!(
            adaptive < 0.02,
            "the adaptive delay almost never does: {adaptive}"
        );
    }
}
