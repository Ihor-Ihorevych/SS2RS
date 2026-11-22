//! A network conditioner for tests and scenarios (roadmap T08-017).
//!
//! [`NetSim`] models one direction of a link: packets go in with [`NetSim::send`], and come out
//! of [`NetSim::poll`] after the link's latency, jitter, bandwidth limit, loss, duplication and
//! reordering have been applied. It never reads a clock and never uses a global random source:
//! the caller passes the time (in microseconds on any virtual clock) and the seed, so a run is
//! **exactly repeatable**, which is what a netcode regression test needs. Everything is integer
//! arithmetic and ordered maps (the crate is sim-safe: no `HashMap`, no wall-clock).
//!
//! The in-process simulator is what the tests of prediction and replication use. A socket-level
//! conditioner (a UDP proxy that applies the same [`LinkConfig`] to real traffic) is a separate
//! tool: it needs a real clock, which this crate must not read.

use std::collections::{BTreeMap, VecDeque};

const MILLION: u64 = 1_000_000;

/// What one direction of a link does to the packets that cross it. Probabilities are in parts per
/// million (10,000 = 1 %), times in microseconds, so all the arithmetic is exact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkConfig {
    /// Fixed one-way delay added to every packet.
    pub latency_us: u64,
    /// Extra random delay, uniform from 0 to this value, drawn separately for each packet.
    pub jitter_us: u64,
    /// Chance that a packet is lost on the wire (it still uses up bandwidth).
    pub loss_ppm: u32,
    /// Chance that a packet that is not lost arrives twice.
    pub duplicate_ppm: u32,
    /// Chance that a packet is held back by [`LinkConfig::reorder_delay_us`] so that later
    /// packets overtake it.
    pub reorder_ppm: u32,
    /// The extra delay of a reordered packet.
    pub reorder_delay_us: u64,
    /// The link rate in bytes per second; `None` means no limit.
    pub bandwidth_bytes_per_sec: Option<u64>,
    /// How many bytes may wait for the link before newer packets are dropped (a tail-drop queue);
    /// the packet being transmitted does not count. Only meaningful with a bandwidth limit.
    pub queue_limit_bytes: usize,
}

impl Default for LinkConfig {
    /// A perfect link: no delay, no loss, unlimited bandwidth.
    fn default() -> Self {
        Self {
            latency_us: 0,
            jitter_us: 0,
            loss_ppm: 0,
            duplicate_ppm: 0,
            reorder_ppm: 0,
            reorder_delay_us: 0,
            bandwidth_bytes_per_sec: None,
            queue_limit_bytes: usize::MAX,
        }
    }
}

impl LinkConfig {
    /// A wired local network: under a millisecond, no loss.
    pub fn lan() -> Self {
        Self {
            latency_us: 500,
            jitter_us: 200,
            ..Self::default()
        }
    }

    /// A decent home connection: 15 ms, a little jitter, 0.1 % loss.
    pub fn broadband() -> Self {
        Self {
            latency_us: 15_000,
            jitter_us: 5_000,
            loss_ppm: 1_000,
            ..Self::default()
        }
    }

    /// A mobile connection: 50 ms, noticeable jitter, 1 % loss.
    pub fn mobile() -> Self {
        Self {
            latency_us: 50_000,
            jitter_us: 20_000,
            loss_ppm: 10_000,
            ..Self::default()
        }
    }

    /// Crowded wireless: 30 ms with large jitter, 5 % loss and some reordering.
    pub fn bad_wifi() -> Self {
        Self {
            latency_us: 30_000,
            jitter_us: 40_000,
            loss_ppm: 50_000,
            reorder_ppm: 20_000,
            reorder_delay_us: 30_000,
            ..Self::default()
        }
    }

    /// A satellite link: 300 ms, 0.5 % loss.
    pub fn satellite() -> Self {
        Self {
            latency_us: 300_000,
            jitter_us: 30_000,
            loss_ppm: 5_000,
            ..Self::default()
        }
    }

    /// The acceptance profile of the roadmap (`docs/PC_1_0_CHECKLIST.md`): 100 ms and 2 % loss,
    /// without jitter or reordering, so that a failure points at loss and latency alone.
    pub fn acceptance_100ms_2pct() -> Self {
        Self {
            latency_us: 100_000,
            loss_ppm: 20_000,
            ..Self::default()
        }
    }

    /// Check the values.
    ///
    /// # Errors
    /// A message naming the first value that is out of range: a probability above 1,000,000 or a
    /// bandwidth of zero.
    pub fn validate(&self) -> Result<(), String> {
        for (name, ppm) in [
            ("loss_ppm", self.loss_ppm),
            ("duplicate_ppm", self.duplicate_ppm),
            ("reorder_ppm", self.reorder_ppm),
        ] {
            if u64::from(ppm) > MILLION {
                return Err(format!("{name} is {ppm}, above 1,000,000 (100 %)"));
            }
        }
        if self.bandwidth_bytes_per_sec == Some(0) {
            return Err("bandwidth_bytes_per_sec is 0 (use None for no limit)".to_owned());
        }
        Ok(())
    }

    /// A copy that [`NetSim`] can run: probabilities capped at 100 %, a zero bandwidth read as
    /// one byte per second.
    fn sanitized(&self) -> Self {
        let cap = |ppm: u32| ppm.min(MILLION as u32);
        Self {
            loss_ppm: cap(self.loss_ppm),
            duplicate_ppm: cap(self.duplicate_ppm),
            reorder_ppm: cap(self.reorder_ppm),
            bandwidth_bytes_per_sec: self.bandwidth_bytes_per_sec.map(|b| b.max(1)),
            ..self.clone()
        }
    }
}

/// What happened to a packet handed to [`NetSim::send`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    /// Accepted. It may still be lost on the wire; see [`NetStats::dropped_loss`].
    Queued,
    /// Lost on the wire (it used up its share of the bandwidth).
    DroppedLoss,
    /// Refused because the link queue was full.
    DroppedQueueFull,
}

/// Counters of what a link did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetStats {
    /// Packets handed to `send`.
    pub sent: u64,
    /// Packets returned by `poll`, duplicates included.
    pub delivered: u64,
    /// Packets lost on the wire.
    pub dropped_loss: u64,
    /// Packets refused by a full queue.
    pub dropped_queue: u64,
    /// Extra copies created.
    pub duplicated: u64,
    /// Packets held back to be overtaken.
    pub reordered: u64,
}

/// A small deterministic random generator (SplitMix64): the same seed always gives the same
/// sequence, on every platform.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// `true` with probability `ppm` per million (never for 0, always for 1,000,000).
    fn chance(&mut self, ppm: u32) -> bool {
        match ppm {
            0 => false,
            p if u64::from(p) >= MILLION => true,
            p => self.next_u64() % MILLION < u64::from(p),
        }
    }

    /// A value from 0 to `max` inclusive.
    fn up_to(&mut self, max: u64) -> u64 {
        if max == 0 {
            0
        } else {
            self.next_u64() % (max + 1)
        }
    }
}

/// One direction of a conditioned link carrying packets of type `T`.
#[derive(Clone, Debug)]
pub struct NetSim<T> {
    config: LinkConfig,
    rng: Rng,
    seq: u64,
    /// When the link finishes the last accepted packet.
    busy_until_us: u64,
    /// Accepted packets that have not started transmitting: (start time, size).
    waiting: VecDeque<(u64, usize)>,
    in_flight: BTreeMap<(u64, u64), T>,
    stats: NetStats,
}

impl<T: Clone> NetSim<T> {
    /// A link with the given behaviour and random seed. A config that does not pass
    /// [`LinkConfig::validate`] is made usable (probabilities capped, zero bandwidth read as one
    /// byte per second) rather than refused.
    pub fn new(config: LinkConfig, seed: u64) -> Self {
        Self {
            config: config.sanitized(),
            rng: Rng(seed),
            seq: 0,
            busy_until_us: 0,
            waiting: VecDeque::new(),
            in_flight: BTreeMap::new(),
            stats: NetStats::default(),
        }
    }

    /// Hand a packet of `size_bytes` bytes to the link at time `at_us`. Times should not go
    /// backwards from one call to the next.
    pub fn send(&mut self, at_us: u64, payload: T, size_bytes: usize) -> SendOutcome {
        self.stats.sent += 1;
        // The time the packet leaves the sender: at once, or after the link has sent what is ahead.
        let mut departs_us = at_us;
        if let Some(rate) = self.config.bandwidth_bytes_per_sec {
            while self
                .waiting
                .front()
                .is_some_and(|&(start, _)| start <= at_us)
            {
                self.waiting.pop_front();
            }
            let start_us = at_us.max(self.busy_until_us);
            if start_us > at_us {
                let waiting: usize = self.waiting.iter().map(|&(_, size)| size).sum();
                if waiting.saturating_add(size_bytes) > self.config.queue_limit_bytes {
                    self.stats.dropped_queue += 1;
                    return SendOutcome::DroppedQueueFull;
                }
                self.waiting.push_back((start_us, size_bytes));
            }
            let transmit_us = (size_bytes as u64).saturating_mul(MILLION).div_ceil(rate);
            departs_us = start_us.saturating_add(transmit_us);
            self.busy_until_us = departs_us;
        }
        if self.rng.chance(self.config.loss_ppm) {
            self.stats.dropped_loss += 1;
            return SendOutcome::DroppedLoss;
        }
        let copies = if self.rng.chance(self.config.duplicate_ppm) {
            self.stats.duplicated += 1;
            2
        } else {
            1
        };
        let reordered = self.rng.chance(self.config.reorder_ppm);
        if reordered {
            self.stats.reordered += 1;
        }
        for copy in 0..copies {
            let mut deliver_us = departs_us
                .saturating_add(self.config.latency_us)
                .saturating_add(self.rng.up_to(self.config.jitter_us));
            if reordered && copy == 0 {
                deliver_us = deliver_us.saturating_add(self.config.reorder_delay_us);
            }
            self.seq += 1;
            self.in_flight
                .insert((deliver_us, self.seq), payload.clone());
        }
        SendOutcome::Queued
    }

    /// Take every packet due at `now_us` or earlier, as (delivery time, packet), earliest first;
    /// packets due at the same time come out in the order they were sent.
    pub fn poll(&mut self, now_us: u64) -> Vec<(u64, T)> {
        let mut out = Vec::new();
        while let Some(entry) = self.in_flight.first_entry() {
            let (deliver_us, _) = *entry.key();
            if deliver_us > now_us {
                break;
            }
            out.push((deliver_us, entry.remove()));
        }
        self.stats.delivered += out.len() as u64;
        out
    }

    /// The time the next packet is due, if any is on its way.
    pub fn next_delivery_us(&self) -> Option<u64> {
        self.in_flight.keys().next().map(|&(t, _)| t)
    }

    /// How many packets are on their way.
    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// What the link has done so far.
    pub fn stats(&self) -> NetStats {
        self.stats
    }
}

/// Both directions of a link, each with its own behaviour and its own random stream.
#[derive(Clone, Debug)]
pub struct Duplex<T> {
    /// Packets from side A to side B.
    pub a_to_b: NetSim<T>,
    /// Packets from side B to side A.
    pub b_to_a: NetSim<T>,
}

impl<T: Clone> Duplex<T> {
    /// A link whose directions behave differently (a typical home connection is asymmetric).
    pub fn new(a_to_b: LinkConfig, b_to_a: LinkConfig, seed: u64) -> Self {
        Self {
            a_to_b: NetSim::new(a_to_b, seed),
            b_to_a: NetSim::new(b_to_a, seed ^ 0xA5A5_A5A5_5A5A_5A5A),
        }
    }

    /// A link that behaves the same in both directions (with different random draws).
    pub fn symmetric(config: LinkConfig, seed: u64) -> Self {
        Self::new(config.clone(), config, seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(sim: &mut NetSim<u32>, until_us: u64) -> Vec<(u64, u32)> {
        sim.poll(until_us)
    }

    fn lossless(latency_us: u64) -> LinkConfig {
        LinkConfig {
            latency_us,
            ..LinkConfig::default()
        }
    }

    #[test]
    fn a_default_link_delivers_at_once_and_in_order() {
        let mut sim = NetSim::new(LinkConfig::default(), 1);
        for i in 0..5 {
            sim.send(100, i, 100);
        }
        assert_eq!(
            drain(&mut sim, 100),
            [(100, 0), (100, 1), (100, 2), (100, 3), (100, 4)]
        );
        assert_eq!(sim.in_flight(), 0);
    }

    #[test]
    fn latency_delays_delivery_exactly() {
        let mut sim = NetSim::new(lossless(50_000), 1);
        sim.send(1_000, 7, 100);
        assert!(
            drain(&mut sim, 50_999).is_empty(),
            "not before the latency has passed"
        );
        assert_eq!(sim.next_delivery_us(), Some(51_000));
        assert_eq!(drain(&mut sim, 51_000), [(51_000, 7)]);
        assert_eq!(sim.next_delivery_us(), None);
    }

    #[test]
    fn jitter_stays_inside_its_bounds_and_actually_varies() {
        let config = LinkConfig {
            latency_us: 40_000,
            jitter_us: 10_000,
            ..LinkConfig::default()
        };
        let mut sim = NetSim::new(config, 9);
        for i in 0..2_000 {
            sim.send(0, i, 10);
        }
        let got = drain(&mut sim, u64::MAX);
        assert_eq!(got.len(), 2_000);
        let times: Vec<u64> = got.iter().map(|(t, _)| *t).collect();
        assert!(
            times.iter().all(|t| (40_000..=50_000).contains(t)),
            "{:?}",
            (times.iter().min(), times.iter().max())
        );
        assert!(
            times.iter().min() != times.iter().max(),
            "no jitter was applied"
        );
    }

    #[test]
    fn the_same_seed_gives_the_same_run_and_another_seed_a_different_one() {
        let config = LinkConfig {
            latency_us: 20_000,
            jitter_us: 15_000,
            loss_ppm: 100_000,
            duplicate_ppm: 50_000,
            reorder_ppm: 100_000,
            reorder_delay_us: 30_000,
            ..LinkConfig::default()
        };
        let run = |seed| {
            let mut sim = NetSim::new(config.clone(), seed);
            for i in 0..500_u32 {
                sim.send(u64::from(i) * 1_000, i, 200);
            }
            sim.poll(u64::MAX)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(43));
    }

    #[test]
    fn loss_follows_the_configured_rate() {
        let sent = 100_000;
        let config = LinkConfig {
            loss_ppm: 100_000,
            ..LinkConfig::default()
        };
        let mut sim = NetSim::new(config, 5);
        for i in 0..sent {
            sim.send(0, i, 10);
        }
        let delivered = drain(&mut sim, 0).len() as u32;
        let lost = sent - delivered;
        assert!(
            (9_000..=11_000).contains(&lost),
            "lost {lost} of {sent} at 10 %"
        );
        assert_eq!(sim.stats().dropped_loss, u64::from(lost));
    }

    #[test]
    fn zero_loss_loses_nothing_and_total_loss_delivers_nothing() {
        let mut keep = NetSim::new(LinkConfig::default(), 1);
        let mut drop_all = NetSim::new(
            LinkConfig {
                loss_ppm: 1_000_000,
                ..LinkConfig::default()
            },
            1,
        );
        for i in 0..1_000 {
            keep.send(0, i, 10);
            drop_all.send(0, i, 10);
        }
        assert_eq!(drain(&mut keep, 0).len(), 1_000);
        assert!(drain(&mut drop_all, u64::MAX).is_empty());
        assert_eq!(drop_all.stats().dropped_loss, 1_000);
    }

    #[test]
    fn duplication_delivers_a_packet_twice() {
        let mut sim = NetSim::new(
            LinkConfig {
                duplicate_ppm: 1_000_000,
                latency_us: 5,
                ..LinkConfig::default()
            },
            3,
        );
        sim.send(0, 9, 10);
        let got = drain(&mut sim, u64::MAX);
        assert_eq!(got.iter().map(|(_, p)| *p).collect::<Vec<_>>(), [9, 9]);
        assert_eq!(sim.stats().duplicated, 1);
    }

    #[test]
    fn reordering_delays_some_packets_past_later_ones_and_only_when_asked() {
        let order = |reorder_ppm| {
            let config = LinkConfig {
                latency_us: 10_000,
                reorder_ppm,
                reorder_delay_us: 25_000,
                ..LinkConfig::default()
            };
            let mut sim = NetSim::new(config, 11);
            for i in 0..1_000 {
                sim.send(u64::from(i) * 1_000, i, 10);
            }
            sim.poll(u64::MAX)
                .into_iter()
                .map(|(_, p)| p)
                .collect::<Vec<_>>()
        };
        let in_order: Vec<u32> = (0..1_000).collect();
        assert_eq!(order(0), in_order, "no reordering was asked for");
        let shuffled = order(200_000);
        assert_ne!(shuffled, in_order);
        let mut sorted = shuffled.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, in_order, "reordering loses nothing");
    }

    #[test]
    fn a_bandwidth_limit_spaces_packets_by_their_transmission_time() {
        // 125,000 bytes per second: a 1,250 byte packet takes 10,000 microseconds.
        let config = LinkConfig {
            bandwidth_bytes_per_sec: Some(125_000),
            latency_us: 1_000,
            ..LinkConfig::default()
        };
        let mut sim = NetSim::new(config, 1);
        for i in 0..4 {
            sim.send(0, i, 1_250);
        }
        let times: Vec<u64> = drain(&mut sim, u64::MAX).iter().map(|(t, _)| *t).collect();
        assert_eq!(times, [11_000, 21_000, 31_000, 41_000]);
    }

    #[test]
    fn an_idle_link_does_not_save_up_bandwidth() {
        let config = LinkConfig {
            bandwidth_bytes_per_sec: Some(125_000),
            ..LinkConfig::default()
        };
        let mut sim = NetSim::new(config, 1);
        sim.send(0, 1, 1_250);
        sim.send(1_000_000, 2, 1_250);
        let times: Vec<u64> = drain(&mut sim, u64::MAX).iter().map(|(t, _)| *t).collect();
        assert_eq!(times, [10_000, 1_010_000]);
    }

    #[test]
    fn a_full_queue_drops_the_newest_packets() {
        let config = LinkConfig {
            bandwidth_bytes_per_sec: Some(125_000),
            queue_limit_bytes: 3_000,
            ..LinkConfig::default()
        };
        let mut sim = NetSim::new(config, 1);
        let results: Vec<SendOutcome> = (0..6).map(|i| sim.send(0, i, 1_250)).collect();
        let queued = results
            .iter()
            .filter(|r| **r == SendOutcome::Queued)
            .count();
        assert_eq!(
            queued, 3,
            "the first packet is on the wire and two more fit in 3,000 bytes: {results:?}"
        );
        assert!(
            results[3..]
                .iter()
                .all(|r| *r == SendOutcome::DroppedQueueFull)
        );
        assert_eq!(sim.stats().dropped_queue, 3);
        let delivered: Vec<u32> = drain(&mut sim, u64::MAX).iter().map(|(_, p)| *p).collect();
        assert_eq!(delivered, [0, 1, 2], "the oldest packets survive");
    }

    #[test]
    fn packets_with_the_same_delivery_time_keep_their_send_order() {
        let mut sim = NetSim::new(lossless(1_000), 1);
        for i in 0..50 {
            sim.send(0, i, 10);
        }
        let got: Vec<u32> = drain(&mut sim, u64::MAX).iter().map(|(_, p)| *p).collect();
        assert_eq!(got, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn polling_in_steps_returns_each_packet_once() {
        let mut sim = NetSim::new(lossless(10_000), 1);
        for i in 0..10_u32 {
            sim.send(u64::from(i) * 5_000, i, 10);
        }
        let mut all = Vec::new();
        let mut now = 0;
        while sim.in_flight() > 0 {
            now += 7_000;
            all.extend(sim.poll(now));
        }
        assert_eq!(
            all.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
        assert!(all.iter().all(|(t, _)| *t <= now));
    }

    #[test]
    fn statistics_add_up() {
        let config = LinkConfig {
            loss_ppm: 200_000,
            duplicate_ppm: 100_000,
            reorder_ppm: 100_000,
            reorder_delay_us: 5_000,
            ..LinkConfig::default()
        };
        let mut sim = NetSim::new(config, 77);
        for i in 0..10_000 {
            sim.send(0, i, 10);
        }
        let delivered = drain(&mut sim, u64::MAX).len() as u64;
        let s = sim.stats();
        assert_eq!(s.sent, 10_000);
        assert_eq!(s.delivered, delivered);
        assert_eq!(
            s.delivered,
            s.sent - s.dropped_loss - s.dropped_queue + s.duplicated
        );
        assert!(s.reordered > 0);
    }

    #[test]
    fn a_config_is_validated() {
        assert!(LinkConfig::default().validate().is_ok());
        assert!(
            LinkConfig {
                loss_ppm: 1_000_001,
                ..LinkConfig::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LinkConfig {
                duplicate_ppm: 2_000_000,
                ..LinkConfig::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LinkConfig {
                reorder_ppm: 1_000_001,
                ..LinkConfig::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LinkConfig {
                bandwidth_bytes_per_sec: Some(0),
                ..LinkConfig::default()
            }
            .validate()
            .is_err()
        );
        for preset in [
            LinkConfig::lan(),
            LinkConfig::broadband(),
            LinkConfig::mobile(),
            LinkConfig::bad_wifi(),
            LinkConfig::satellite(),
        ] {
            assert!(preset.validate().is_ok(), "{preset:?}");
        }
    }

    #[test]
    fn the_presets_are_ordered_from_good_to_bad() {
        assert!(LinkConfig::lan().latency_us < LinkConfig::broadband().latency_us);
        assert!(LinkConfig::broadband().latency_us < LinkConfig::mobile().latency_us);
        assert!(LinkConfig::mobile().latency_us < LinkConfig::satellite().latency_us);
        assert!(LinkConfig::bad_wifi().loss_ppm > LinkConfig::broadband().loss_ppm);
    }

    #[test]
    fn the_acceptance_profile_of_the_roadmap_is_100_ms_and_2_percent_loss() {
        let c = LinkConfig::acceptance_100ms_2pct();
        assert_eq!(c.latency_us, 100_000);
        assert_eq!(c.loss_ppm, 20_000);
    }

    #[test]
    fn a_duplex_link_has_independent_directions_and_seeds() {
        let mut link = Duplex::<u32>::new(lossless(10_000), lossless(30_000), 5);
        link.a_to_b.send(0, 1, 10);
        link.b_to_a.send(0, 2, 10);
        assert_eq!(link.a_to_b.poll(u64::MAX), [(10_000, 1)]);
        assert_eq!(link.b_to_a.poll(u64::MAX), [(30_000, 2)]);
        let mut x = Duplex::<u32>::symmetric(LinkConfig::bad_wifi(), 5);
        let mut y = Duplex::<u32>::symmetric(LinkConfig::bad_wifi(), 5);
        for i in 0..200 {
            x.a_to_b.send(0, i, 100);
            x.b_to_a.send(0, i, 100);
            y.a_to_b.send(0, i, 100);
            y.b_to_a.send(0, i, 100);
        }
        assert_eq!(
            x.a_to_b.poll(u64::MAX),
            y.a_to_b.poll(u64::MAX),
            "same seed, same run"
        );
        assert_ne!(
            x.a_to_b.poll(u64::MAX),
            x.b_to_a.poll(u64::MAX),
            "the two directions use different random streams"
        );
    }

    #[test]
    fn an_invalid_config_is_made_usable_not_fatal() {
        let config = LinkConfig {
            loss_ppm: u32::MAX,
            duplicate_ppm: u32::MAX,
            reorder_ppm: u32::MAX,
            bandwidth_bytes_per_sec: Some(0),
            ..LinkConfig::default()
        };
        assert!(config.validate().is_err());
        let mut sim = NetSim::new(config, 1);
        for i in 0..100_u32 {
            sim.send(u64::from(i), i, 1_000);
        }
        assert!(sim.poll(u64::MAX).is_empty(), "100 % loss drops everything");
        assert_eq!(sim.stats().dropped_loss, 100);
    }
}
