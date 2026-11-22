//! Measuring the network for a `net_graph` display and for profiling (roadmap T08-018).
//!
//! [`TrafficMeter`] counts the bytes of each kind of traffic (inputs, snapshots, events, ...) per
//! simulation tick over a sliding window and answers "how much, per second, and what was the
//! worst tick". [`LinkMeter`] keeps the latest round trips and the outcome of the latest packets
//! and answers "what are the round trip, its spread and the loss". Both are plain data
//! collectors: they take numbers and never read a clock, so they are tested with exact values and
//! can be fed from the simulator or from a real transport alike.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Bytes sent or received per tick, by kind of traffic, over a sliding window of ticks.
#[derive(Clone, Debug)]
pub struct TrafficMeter<C: Ord + Copy> {
    window_ticks: usize,
    ticks: VecDeque<BTreeMap<C, u64>>,
    current: BTreeMap<C, u64>,
}

impl<C: Ord + Copy> TrafficMeter<C> {
    /// A meter that remembers the latest `window_ticks` ticks (a window of 0 is read as 1).
    pub fn new(window_ticks: usize) -> Self {
        Self {
            window_ticks: window_ticks.max(1),
            ticks: VecDeque::new(),
            current: BTreeMap::new(),
        }
    }

    /// Count `bytes` of `category` in the tick that is in progress.
    pub fn record(&mut self, category: C, bytes: usize) {
        let slot = self.current.entry(category).or_insert(0);
        *slot = slot.saturating_add(bytes as u64);
    }

    /// Close the tick in progress and start the next; the oldest tick leaves a full window.
    pub fn end_tick(&mut self) {
        self.ticks.push_back(std::mem::take(&mut self.current));
        while self.ticks.len() > self.window_ticks {
            self.ticks.pop_front();
        }
    }

    /// How many closed ticks are in the window.
    pub fn ticks_measured(&self) -> usize {
        self.ticks.len()
    }

    /// The kinds of traffic seen in the window, in order.
    pub fn categories(&self) -> Vec<C> {
        let set: BTreeSet<C> = self.ticks.iter().flat_map(|t| t.keys().copied()).collect();
        set.into_iter().collect()
    }

    /// The bytes of `category` in the window.
    pub fn total(&self, category: C) -> u64 {
        self.ticks.iter().fold(0_u64, |sum, t| {
            sum.saturating_add(t.get(&category).copied().unwrap_or(0))
        })
    }

    /// The bytes of every category in the window.
    pub fn total_all(&self) -> u64 {
        self.ticks
            .iter()
            .flat_map(|t| t.values())
            .fold(0_u64, |sum, b| sum.saturating_add(*b))
    }

    /// The most bytes of `category` in one tick of the window.
    pub fn peak_tick(&self, category: C) -> u64 {
        self.ticks
            .iter()
            .map(|t| t.get(&category).copied().unwrap_or(0))
            .max()
            .unwrap_or(0)
    }

    /// The bytes of `category` for every tick of the window, oldest first (what a graph draws).
    pub fn series(&self, category: C) -> Vec<u64> {
        self.ticks
            .iter()
            .map(|t| t.get(&category).copied().unwrap_or(0))
            .collect()
    }

    fn per_second(&self, bytes: u64, tick_hz: u32) -> u64 {
        let ticks = self.ticks.len() as u128;
        if ticks == 0 {
            return 0;
        }
        u64::try_from(u128::from(bytes) * u128::from(tick_hz) / ticks).unwrap_or(u64::MAX)
    }

    /// The average bytes per second of `category` over the window, at `tick_hz` ticks per second.
    pub fn bytes_per_second(&self, category: C, tick_hz: u32) -> u64 {
        self.per_second(self.total(category), tick_hz)
    }

    /// The average bits per second of all traffic over the window.
    pub fn bits_per_second_all(&self, tick_hz: u32) -> u64 {
        self.per_second(self.total_all(), tick_hz).saturating_mul(8)
    }
}

/// The latest round trips and packet outcomes of one connection.
#[derive(Clone, Debug)]
pub struct LinkMeter {
    rtt_window: usize,
    packet_window: usize,
    rtts: VecDeque<u64>,
    packets: VecDeque<bool>,
}

impl LinkMeter {
    /// A meter that keeps the latest `rtt_window` round trips and `packet_window` packet outcomes.
    pub fn new(rtt_window: usize, packet_window: usize) -> Self {
        Self {
            rtt_window: rtt_window.max(1),
            packet_window: packet_window.max(1),
            rtts: VecDeque::new(),
            packets: VecDeque::new(),
        }
    }

    /// Add a round trip time in microseconds.
    pub fn record_rtt(&mut self, rtt_us: u64) {
        self.rtts.push_back(rtt_us);
        while self.rtts.len() > self.rtt_window {
            self.rtts.pop_front();
        }
    }

    /// Add the outcome of one packet: `true` if it was acknowledged, `false` if it was given up on.
    pub fn record_packet(&mut self, acknowledged: bool) {
        self.packets.push_back(acknowledged);
        while self.packets.len() > self.packet_window {
            self.packets.pop_front();
        }
    }

    /// The mean round trip of the window.
    pub fn rtt_mean_us(&self) -> Option<u64> {
        (!self.rtts.is_empty()).then(|| self.rtts.iter().sum::<u64>() / self.rtts.len() as u64)
    }

    /// The shortest round trip of the window.
    pub fn rtt_min_us(&self) -> Option<u64> {
        self.rtts.iter().copied().min()
    }

    /// The longest round trip of the window.
    pub fn rtt_max_us(&self) -> Option<u64> {
        self.rtts.iter().copied().max()
    }

    /// The `percent`-th percentile of the round trips (nearest rank; above 100 is read as 100).
    pub fn rtt_percentile_us(&self, percent: u32) -> Option<u64> {
        if self.rtts.is_empty() {
            return None;
        }
        let mut sorted: Vec<u64> = self.rtts.iter().copied().collect();
        sorted.sort_unstable();
        let rank = (sorted.len() as u64 * u64::from(percent.min(100)))
            .div_ceil(100)
            .max(1);
        Some(sorted[rank as usize - 1])
    }

    /// The mean deviation of the round trips from their mean: how much the delay varies.
    pub fn jitter_us(&self) -> Option<u64> {
        let mean = self.rtt_mean_us()?;
        Some(self.rtts.iter().map(|r| r.abs_diff(mean)).sum::<u64>() / self.rtts.len() as u64)
    }

    /// The round trips of the window, oldest first.
    pub fn rtt_series(&self) -> Vec<u64> {
        self.rtts.iter().copied().collect()
    }

    /// The share of the latest packets that were not acknowledged, in parts per thousand.
    pub fn loss_permille(&self) -> Option<u32> {
        if self.packets.is_empty() {
            return None;
        }
        let lost = self.packets.iter().filter(|acked| !**acked).count() as u64;
        Some((lost * 1000 / self.packets.len() as u64) as u32)
    }

    /// How many packet outcomes are in the window.
    pub fn packets_measured(&self) -> usize {
        self.packets.len()
    }
}

/// Everything a `net_graph` overlay shows, gathered in one place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetGraph {
    /// Average traffic of all kinds, bytes per second.
    pub bytes_per_second_total: u64,
    /// Mean round trip, microseconds.
    pub rtt_mean_us: Option<u64>,
    /// 95th percentile round trip, microseconds.
    pub rtt_p95_us: Option<u64>,
    /// Mean deviation of the round trips, microseconds.
    pub jitter_us: Option<u64>,
    /// Packet loss in parts per thousand.
    pub loss_permille: Option<u32>,
    /// How many ticks the traffic figures cover.
    pub ticks_measured: usize,
    /// The latest round trips, oldest first, for the graph.
    pub rtt_series: Vec<u64>,
}

impl NetGraph {
    /// Gather the figures from the two meters at `tick_hz` ticks per second.
    pub fn summarize<C: Ord + Copy>(
        traffic: &TrafficMeter<C>,
        link: &LinkMeter,
        tick_hz: u32,
    ) -> Self {
        let bytes_per_second_total = traffic.per_second(traffic.total_all(), tick_hz);
        Self {
            bytes_per_second_total,
            rtt_mean_us: link.rtt_mean_us(),
            rtt_p95_us: link.rtt_percentile_us(95),
            jitter_us: link.jitter_us(),
            loss_permille: link.loss_permille(),
            ticks_measured: traffic.ticks_measured(),
            rtt_series: link.rtt_series(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Kind {
        Input,
        Snapshot,
        Event,
    }

    fn feed(meter: &mut TrafficMeter<Kind>, ticks: &[&[(Kind, usize)]]) {
        for tick in ticks {
            for (kind, bytes) in *tick {
                meter.record(*kind, *bytes);
            }
            meter.end_tick();
        }
    }

    #[test]
    fn an_empty_meter_reports_zeros() {
        let meter = TrafficMeter::<Kind>::new(60);
        assert_eq!(meter.total(Kind::Input), 0);
        assert_eq!(meter.bytes_per_second(Kind::Input, 60), 0);
        assert_eq!(meter.peak_tick(Kind::Input), 0);
        assert_eq!(meter.ticks_measured(), 0);
        assert!(meter.categories().is_empty());
        assert!(meter.series(Kind::Input).is_empty());
    }

    #[test]
    fn bytes_are_added_per_category_over_the_window() {
        let mut meter = TrafficMeter::new(10);
        feed(
            &mut meter,
            &[
                &[(Kind::Input, 30), (Kind::Snapshot, 400)],
                &[(Kind::Input, 30)],
                &[(Kind::Snapshot, 600), (Kind::Event, 50)],
            ],
        );
        assert_eq!(meter.ticks_measured(), 3);
        assert_eq!(meter.total(Kind::Input), 60);
        assert_eq!(meter.total(Kind::Snapshot), 1_000);
        assert_eq!(meter.total(Kind::Event), 50);
        assert_eq!(meter.total_all(), 1_110);
        assert_eq!(
            meter.categories(),
            [Kind::Input, Kind::Snapshot, Kind::Event]
        );
        assert_eq!(meter.peak_tick(Kind::Snapshot), 600);
    }

    #[test]
    fn the_rate_is_per_second_at_the_given_tick_rate() {
        let mut meter = TrafficMeter::new(60);
        for _ in 0..60 {
            meter.record(Kind::Snapshot, 500);
            meter.end_tick();
        }
        assert_eq!(meter.bytes_per_second(Kind::Snapshot, 60), 30_000);
        // The same window read at 30 ticks per second is half a window's worth of seconds.
        assert_eq!(meter.bytes_per_second(Kind::Snapshot, 30), 15_000);
        assert_eq!(meter.bits_per_second_all(60), 240_000);
    }

    #[test]
    fn the_rate_of_a_part_filled_window_uses_the_ticks_measured() {
        let mut meter = TrafficMeter::new(600);
        for _ in 0..30 {
            meter.record(Kind::Input, 40);
            meter.end_tick();
        }
        assert_eq!(
            meter.bytes_per_second(Kind::Input, 60),
            2_400,
            "40 bytes a tick at 60 ticks a second, after only half a second"
        );
    }

    #[test]
    fn old_ticks_leave_the_window() {
        let mut meter = TrafficMeter::new(3);
        feed(
            &mut meter,
            &[
                &[(Kind::Input, 1_000)],
                &[(Kind::Input, 1)],
                &[(Kind::Input, 2)],
                &[(Kind::Input, 3)],
            ],
        );
        assert_eq!(meter.ticks_measured(), 3);
        assert_eq!(
            meter.total(Kind::Input),
            6,
            "the 1,000 byte tick has been forgotten"
        );
        assert_eq!(meter.peak_tick(Kind::Input), 3);
        assert_eq!(
            meter.series(Kind::Input),
            [1, 2, 3],
            "oldest first, ready to draw"
        );
    }

    #[test]
    fn bytes_recorded_in_the_current_tick_count_when_it_ends_not_before() {
        let mut meter = TrafficMeter::new(10);
        meter.record(Kind::Event, 99);
        assert_eq!(meter.total(Kind::Event), 0);
        meter.end_tick();
        assert_eq!(meter.total(Kind::Event), 99);
    }

    #[test]
    fn a_category_missing_from_some_ticks_counts_zero_there() {
        let mut meter = TrafficMeter::new(10);
        feed(
            &mut meter,
            &[&[(Kind::Event, 10)], &[], &[(Kind::Event, 20)]],
        );
        assert_eq!(meter.series(Kind::Event), [10, 0, 20]);
        assert_eq!(meter.peak_tick(Kind::Event), 20);
    }

    #[test]
    fn huge_counts_saturate_instead_of_wrapping() {
        let mut meter = TrafficMeter::new(4);
        meter.record(Kind::Input, usize::MAX);
        meter.record(Kind::Input, usize::MAX);
        meter.end_tick();
        assert_eq!(meter.peak_tick(Kind::Input), u64::MAX);
        assert_eq!(meter.total(Kind::Input), u64::MAX);
    }

    #[test]
    fn a_window_of_zero_ticks_is_read_as_one() {
        let mut meter = TrafficMeter::new(0);
        feed(&mut meter, &[&[(Kind::Input, 5)], &[(Kind::Input, 7)]]);
        assert_eq!(meter.ticks_measured(), 1);
        assert_eq!(meter.total(Kind::Input), 7);
    }

    #[test]
    fn a_link_meter_with_nothing_reports_nothing() {
        let link = LinkMeter::new(32, 100);
        assert_eq!(link.rtt_mean_us(), None);
        assert_eq!(link.rtt_percentile_us(95), None);
        assert_eq!(link.jitter_us(), None);
        assert_eq!(link.loss_permille(), None);
    }

    #[test]
    fn round_trip_statistics_use_the_latest_samples() {
        let mut link = LinkMeter::new(5, 100);
        for rtt in [40_000, 60_000, 40_000, 60_000, 50_000] {
            link.record_rtt(rtt);
        }
        assert_eq!(link.rtt_mean_us(), Some(50_000));
        assert_eq!(link.rtt_min_us(), Some(40_000));
        assert_eq!(link.rtt_max_us(), Some(60_000));
        assert_eq!(link.rtt_percentile_us(50), Some(50_000));
        assert_eq!(link.rtt_percentile_us(100), Some(60_000));
        assert_eq!(link.rtt_percentile_us(0), Some(40_000));
        assert_eq!(
            link.jitter_us(),
            Some(8_000),
            "the mean deviation: 10, 10, 10, 10 and 0 ms around the 50 ms mean"
        );
        // One more sample pushes the oldest out of the window of five.
        link.record_rtt(500_000);
        assert_eq!(link.rtt_max_us(), Some(500_000));
        assert_eq!(
            link.rtt_min_us(),
            Some(40_000),
            "the second 40 ms sample is still inside the window"
        );
        assert_eq!(link.rtt_series(), [60_000, 40_000, 60_000, 50_000, 500_000]);
    }

    #[test]
    fn loss_is_the_share_of_recent_packets_that_were_not_acknowledged() {
        let mut link = LinkMeter::new(8, 10);
        for acked in [true, true, false, true, true, true, true, true, true, false] {
            link.record_packet(acked);
        }
        // The window keeps the latest 10 packets: 2 lost of 10.
        assert_eq!(link.loss_permille(), Some(200));
        for _ in 0..7 {
            link.record_packet(true);
        }
        assert_eq!(
            link.loss_permille(),
            Some(100),
            "the first loss (the third packet) has left the window, the second has not"
        );
        for _ in 0..3 {
            link.record_packet(true);
        }
        assert_eq!(
            link.loss_permille(),
            Some(0),
            "both losses have left the window"
        );
        assert_eq!(link.packets_measured(), 10);
    }

    #[test]
    fn percentiles_stay_inside_the_samples() {
        let mut link = LinkMeter::new(100, 10);
        for rtt in 1..=100_u64 {
            link.record_rtt(rtt * 1_000);
        }
        assert_eq!(link.rtt_percentile_us(50), Some(50_000));
        assert_eq!(link.rtt_percentile_us(95), Some(95_000));
        assert_eq!(link.rtt_percentile_us(99), Some(99_000));
        assert_eq!(
            link.rtt_percentile_us(250),
            Some(100_000),
            "a percentile above 100 is read as 100"
        );
    }

    #[test]
    fn the_graph_summary_collects_what_a_hud_draws() {
        let mut traffic = TrafficMeter::new(60);
        let mut link = LinkMeter::new(30, 60);
        for tick in 0..60_u64 {
            traffic.record(Kind::Snapshot, 400);
            traffic.record(Kind::Input, 30);
            traffic.end_tick();
            link.record_rtt(80_000 + (tick % 2) * 20_000);
            link.record_packet(tick % 10 != 0);
        }
        let g = NetGraph::summarize(&traffic, &link, 60);
        assert_eq!(g.bytes_per_second_total, 25_800);
        assert_eq!(g.rtt_mean_us, Some(90_000));
        assert_eq!(g.rtt_p95_us, Some(100_000));
        assert_eq!(g.loss_permille, Some(100));
        assert_eq!(g.ticks_measured, 60);
        assert_eq!(g.rtt_series.len(), 30);
    }
}
