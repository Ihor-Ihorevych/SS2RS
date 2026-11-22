//! The stream of player inputs from a client to the server (roadmap T08-011).
//!
//! A client stamps each input with the simulation tick it is meant for and sends the last few
//! unacknowledged ones in every packet ([`InputSender`]): a lost packet is then repaired by the
//! next one without a resend round trip. The server ([`InputBuffer`]) checks every frame (not too
//! old, not too far ahead, not faster than a player can generate them, not a second, different
//! input for a tick it already has), keeps the accepted ones, hands one input to the simulation
//! per tick, and tells the client how far it has everything ([`InputBuffer::ack_tick`]).
//!
//! The input type is generic (the movement and weapon state of the player come with the game
//! code); like the rest of the crate this is clock-free and deterministic.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The limits of the input stream. The defaults suit a 60 Hz game over an ordinary connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputConfig {
    /// How many of the latest unacknowledged frames each packet carries (at least 1).
    pub redundancy: usize,
    /// The most frames in one packet, whatever the redundancy.
    pub max_frames_per_packet: usize,
    /// The most unacknowledged frames the sender keeps; older ones are dropped.
    pub max_unacked: usize,
    /// How far ahead of the tick being simulated the server accepts input.
    pub max_ahead_ticks: u64,
    /// How long the server waits for a missing tick before it stops counting it as outstanding.
    pub max_late_ticks: u64,
    /// How many ticks the last input is repeated for when the next one is missing.
    pub max_repeat_ticks: u64,
    /// The most new frames the server accepts at once (the size of the rate limit's bucket).
    pub burst_frames: u32,
    /// New frames the server accepts per simulated tick on average (a player makes one).
    pub frames_per_tick: u32,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            redundancy: 3,
            max_frames_per_packet: 8,
            max_unacked: 64,
            max_ahead_ticks: 30,
            max_late_ticks: 4,
            max_repeat_ticks: 5,
            burst_frames: 16,
            frames_per_tick: 2,
        }
    }
}

/// One input, stamped with the tick it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFrame<T> {
    /// The simulation tick the input applies to.
    pub tick: u64,
    /// The input itself.
    pub input: T,
}

/// What a client sends: the latest few frames, oldest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPacket<T> {
    /// The frames, in increasing tick order.
    pub frames: Vec<InputFrame<T>>,
}

/// The client side: remembers the unacknowledged frames and builds redundant packets.
#[derive(Clone, Debug)]
pub struct InputSender<T> {
    config: InputConfig,
    history: VecDeque<InputFrame<T>>,
    last_tick: Option<u64>,
}

impl<T: Clone> InputSender<T> {
    /// A sender with nothing recorded.
    pub fn new(config: InputConfig) -> Self {
        Self {
            config,
            history: VecDeque::new(),
            last_tick: None,
        }
    }

    /// Record the input for `tick`. Ticks must increase: a tick that is not newer than the last
    /// one is refused (`false`).
    pub fn push(&mut self, tick: u64, input: T) -> bool {
        if self.last_tick.is_some_and(|last| tick <= last) {
            return false;
        }
        self.last_tick = Some(tick);
        self.history.push_back(InputFrame { tick, input });
        while self.history.len() > self.config.max_unacked.max(1) {
            self.history.pop_front();
        }
        true
    }

    /// The packet to send now: the latest `redundancy` unacknowledged frames, at most
    /// `max_frames_per_packet`. `None` when everything has been acknowledged.
    pub fn packet(&self) -> Option<InputPacket<T>> {
        if self.history.is_empty() {
            return None;
        }
        let count = self
            .config
            .redundancy
            .max(1)
            .min(self.config.max_frames_per_packet.max(1))
            .min(self.history.len());
        let frames = self
            .history
            .iter()
            .skip(self.history.len() - count)
            .cloned()
            .collect();
        Some(InputPacket { frames })
    }

    /// The server has everything up to and including `tick`: stop repeating those frames.
    pub fn ack(&mut self, tick: u64) {
        while self.history.front().is_some_and(|f| f.tick <= tick) {
            self.history.pop_front();
        }
    }

    /// How many frames are waiting for an acknowledgement.
    pub fn unacked(&self) -> usize {
        self.history.len()
    }
}

/// What the server counted while it went through a packet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReceiveReport {
    /// New frames kept.
    pub accepted: u32,
    /// Frames already held (the redundancy at work).
    pub duplicates: u32,
    /// A different input for a tick that already has one (the first stays).
    pub conflicting: u32,
    /// Frames for a tick that has already been simulated.
    pub too_old: u32,
    /// Frames further ahead than the look-ahead allows.
    pub too_far_ahead: u32,
    /// New frames refused by the rate limit.
    pub rate_limited: u32,
}

/// What the simulation gets for a tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputTake<T> {
    /// The input the client sent for this tick.
    Received(T),
    /// None arrived; the last real input is repeated (a held key) for a limited time.
    Repeated(T),
    /// None arrived and nothing is worth repeating: use the neutral input.
    Missing,
}

/// The server side: validates, rate-limits and buffers the inputs of one client.
#[derive(Clone, Debug)]
pub struct InputBuffer<T> {
    config: InputConfig,
    payloads: BTreeMap<u64, T>,
    /// The next tick the simulation will ask for.
    next_tick: u64,
    last_taken: Option<(u64, T)>,
    /// Ticks received (or given up) at or after `done_below`, for the acknowledgement.
    received: BTreeSet<u64>,
    /// Every tick below this one is received or given up.
    done_below: u64,
    start_tick: u64,
    tokens: u32,
    violations: u64,
}

impl<T: Clone + PartialEq> InputBuffer<T> {
    /// A buffer for a client that joins while the simulation is at `start_tick`.
    pub fn new(config: InputConfig, start_tick: u64) -> Self {
        let tokens = config.burst_frames;
        Self {
            config,
            payloads: BTreeMap::new(),
            next_tick: start_tick,
            last_taken: None,
            received: BTreeSet::new(),
            done_below: start_tick,
            start_tick,
            tokens,
            violations: 0,
        }
    }

    /// Check and keep the frames of one packet.
    pub fn receive(&mut self, packet: &InputPacket<T>) -> ReceiveReport {
        let mut report = ReceiveReport::default();
        let limit = self.next_tick.saturating_add(self.config.max_ahead_ticks);
        for frame in &packet.frames {
            if frame.tick < self.next_tick {
                report.too_old += 1;
            } else if frame.tick > limit {
                report.too_far_ahead += 1;
            } else if let Some(held) = self.payloads.get(&frame.tick) {
                if *held == frame.input {
                    report.duplicates += 1;
                } else {
                    report.conflicting += 1;
                }
            } else if self.tokens == 0 {
                report.rate_limited += 1;
                self.violations += 1;
            } else {
                self.tokens -= 1;
                self.payloads.insert(frame.tick, frame.input.clone());
                self.received.insert(frame.tick);
                report.accepted += 1;
            }
        }
        self.advance_acknowledgement();
        report
    }

    /// The input for `tick`, which the simulation is about to run. Ticks should be asked for in
    /// order; asking for one tick consumes it and everything before it.
    pub fn take(&mut self, tick: u64) -> InputTake<T> {
        self.payloads = self.payloads.split_off(&tick);
        let result = if let Some(input) = self.payloads.remove(&tick) {
            self.last_taken = Some((tick, input.clone()));
            InputTake::Received(input)
        } else {
            match &self.last_taken {
                Some((at, input)) if tick > *at && tick - *at <= self.config.max_repeat_ticks => {
                    InputTake::Repeated(input.clone())
                }
                _ => InputTake::Missing,
            }
        };
        let next = tick.saturating_add(1).max(self.next_tick);
        let elapsed = next - self.next_tick;
        self.next_tick = next;
        let refill = u32::try_from(elapsed.saturating_mul(u64::from(self.config.frames_per_tick)))
            .unwrap_or(u32::MAX);
        self.tokens = self
            .tokens
            .saturating_add(refill)
            .min(self.config.burst_frames);
        self.advance_acknowledgement();
        result
    }

    /// The tick up to which the server has every frame or has stopped waiting for it: the number
    /// to send back to the client so it stops repeating older frames. `None` before the first.
    pub fn ack_tick(&self) -> Option<u64> {
        (self.done_below > self.start_tick).then(|| self.done_below - 1)
    }

    /// How many frames are waiting to be taken.
    pub fn buffered(&self) -> usize {
        self.payloads.len()
    }

    /// How many frames the rate limit has refused so far (a kick policy can use it).
    pub fn violations(&self) -> u64 {
        self.violations
    }

    fn advance_acknowledgement(&mut self) {
        // Ticks that are far enough in the past are given up: the client need not resend them.
        let floor = self.next_tick.saturating_sub(self.config.max_late_ticks);
        if self.done_below < floor {
            self.done_below = floor;
        }
        while self.received.contains(&self.done_below) {
            match self.done_below.checked_add(1) {
                Some(next) => self.done_below = next,
                None => break,
            }
        }
        self.received = self.received.split_off(&self.done_below);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::{Duplex, LinkConfig};

    type Sender = InputSender<u32>;
    type Buffer = InputBuffer<u32>;

    fn config() -> InputConfig {
        InputConfig::default()
    }

    fn frame(tick: u64, input: u32) -> InputFrame<u32> {
        InputFrame { tick, input }
    }

    fn ticks(packet: &InputPacket<u32>) -> Vec<u64> {
        packet.frames.iter().map(|f| f.tick).collect()
    }

    #[test]
    fn a_packet_repeats_the_latest_unacknowledged_frames_oldest_first() {
        let mut sender = Sender::new(InputConfig {
            redundancy: 3,
            ..config()
        });
        assert!(sender.packet().is_none(), "nothing to send yet");
        for t in 1..=5 {
            assert!(sender.push(t, t as u32 * 10));
        }
        let packet = sender.packet().unwrap();
        assert_eq!(ticks(&packet), [3, 4, 5]);
        assert_eq!(packet.frames[2], frame(5, 50));
    }

    #[test]
    fn an_acknowledgement_trims_what_is_repeated() {
        let mut sender = Sender::new(InputConfig {
            redundancy: 4,
            ..config()
        });
        for t in 1..=6 {
            sender.push(t, 0);
        }
        sender.ack(4);
        assert_eq!(ticks(&sender.packet().unwrap()), [5, 6]);
        sender.ack(6);
        assert!(sender.packet().is_none(), "everything is acknowledged");
        assert_eq!(sender.unacked(), 0);
    }

    #[test]
    fn frames_must_come_in_increasing_tick_order() {
        let mut sender = Sender::new(config());
        assert!(sender.push(10, 1));
        assert!(!sender.push(10, 2), "the same tick twice");
        assert!(!sender.push(9, 3), "going back");
        assert!(sender.push(11, 4));
        assert_eq!(ticks(&sender.packet().unwrap()), [10, 11]);
    }

    #[test]
    fn a_packet_never_exceeds_the_frame_limit_and_the_history_is_bounded() {
        let mut sender = Sender::new(InputConfig {
            redundancy: 100,
            max_frames_per_packet: 4,
            max_unacked: 10,
            ..config()
        });
        for t in 1..=50 {
            sender.push(t, 0);
        }
        assert_eq!(ticks(&sender.packet().unwrap()), [47, 48, 49, 50]);
        assert!(sender.unacked() <= 10, "unacked {}", sender.unacked());
    }

    #[test]
    fn the_server_accepts_frames_hands_them_out_per_tick_and_counts_duplicates() {
        let mut buffer = Buffer::new(config(), 100);
        let report = buffer.receive(&InputPacket {
            frames: vec![frame(100, 1), frame(101, 2), frame(102, 3)],
        });
        assert_eq!(report.accepted, 3);
        let again = buffer.receive(&InputPacket {
            frames: vec![frame(101, 2), frame(102, 3), frame(103, 4)],
        });
        assert_eq!((again.accepted, again.duplicates), (1, 2));
        assert_eq!(buffer.take(100), InputTake::Received(1));
        assert_eq!(buffer.take(101), InputTake::Received(2));
        assert_eq!(buffer.take(102), InputTake::Received(3));
        assert_eq!(buffer.take(103), InputTake::Received(4));
        assert_eq!(buffer.buffered(), 0);
    }

    #[test]
    fn a_missing_tick_repeats_the_last_input_for_a_while_and_then_gives_up() {
        let mut buffer = Buffer::new(
            InputConfig {
                max_repeat_ticks: 2,
                ..config()
            },
            10,
        );
        buffer.receive(&InputPacket {
            frames: vec![frame(10, 7)],
        });
        assert_eq!(buffer.take(10), InputTake::Received(7));
        assert_eq!(buffer.take(11), InputTake::Repeated(7));
        assert_eq!(buffer.take(12), InputTake::Repeated(7));
        assert_eq!(
            buffer.take(13),
            InputTake::Missing,
            "a held key does not last forever"
        );
        let mut fresh = Buffer::new(config(), 10);
        assert_eq!(
            fresh.take(10),
            InputTake::Missing,
            "no input was ever received"
        );
    }

    #[test]
    fn frames_for_ticks_already_simulated_or_too_far_ahead_are_refused() {
        let mut buffer = Buffer::new(
            InputConfig {
                max_ahead_ticks: 20,
                ..config()
            },
            100,
        );
        buffer.take(100);
        buffer.take(101);
        let report = buffer.receive(&InputPacket {
            frames: vec![
                frame(99, 1),
                frame(101, 1),
                frame(102, 1),
                frame(122, 1),
                frame(123, 1),
                frame(1_000_000, 1),
            ],
        });
        assert_eq!(report.too_old, 2, "ticks 99 and 101 are in the past");
        assert_eq!(
            report.too_far_ahead, 2,
            "123 and 1,000,000 are beyond 102 + 20"
        );
        assert_eq!(report.accepted, 2, "102 and 122");
        assert!(
            buffer.buffered() <= 21,
            "memory is bounded by the look-ahead"
        );
    }

    #[test]
    fn a_second_different_input_for_a_tick_is_refused_and_the_first_stands() {
        let mut buffer = Buffer::new(config(), 5);
        buffer.receive(&InputPacket {
            frames: vec![frame(5, 1)],
        });
        let report = buffer.receive(&InputPacket {
            frames: vec![frame(5, 999)],
        });
        assert_eq!(
            (report.accepted, report.conflicting, report.duplicates),
            (0, 1, 0)
        );
        assert_eq!(buffer.take(5), InputTake::Received(1));
    }

    #[test]
    fn a_flood_is_cut_to_the_burst_and_the_rate_refills_with_the_ticks() {
        let cfg = InputConfig {
            burst_frames: 8,
            frames_per_tick: 2,
            max_ahead_ticks: 10_000,
            ..config()
        };
        let mut buffer = Buffer::new(cfg, 0);
        let flood = InputPacket {
            frames: (0..500).map(|t| frame(t, 1)).collect(),
        };
        let report = buffer.receive(&flood);
        assert_eq!(report.accepted, 8);
        assert_eq!(report.rate_limited, 492);
        assert_eq!(buffer.violations(), 492);
        // Ten ticks later the bucket holds 8 again (10 x 2 refill, capped at the burst).
        for t in 0..10 {
            buffer.take(t);
        }
        let more = buffer.receive(&InputPacket {
            frames: (500..520).map(|t| frame(t, 1)).collect(),
        });
        assert_eq!(more.accepted, 8);
    }

    #[test]
    fn a_normal_player_is_never_rate_limited() {
        let mut buffer = Buffer::new(config(), 0);
        let mut sender = Sender::new(config());
        for tick in 0..2_000_u64 {
            sender.push(tick + 3, tick as u32); // a client that runs 3 ticks ahead
            buffer.receive(&sender.packet().unwrap());
            buffer.take(tick);
            if let Some(ack) = buffer.ack_tick() {
                sender.ack(ack);
            }
        }
        assert_eq!(buffer.violations(), 0);
    }

    #[test]
    fn the_acknowledged_tick_advances_over_contiguous_frames_and_skips_given_up_gaps() {
        let mut buffer = Buffer::new(
            InputConfig {
                max_late_ticks: 3,
                ..config()
            },
            0,
        );
        assert_eq!(buffer.ack_tick(), None);
        buffer.receive(&InputPacket {
            frames: vec![frame(0, 1), frame(1, 1), frame(3, 1)],
        });
        assert_eq!(buffer.ack_tick(), Some(1), "tick 2 is still awaited");
        buffer.receive(&InputPacket {
            frames: vec![frame(2, 1)],
        });
        assert_eq!(buffer.ack_tick(), Some(3));
        // Tick 4 never arrives; once the server is 3 ticks past it the wait is over.
        buffer.receive(&InputPacket {
            frames: vec![frame(5, 1)],
        });
        assert_eq!(buffer.ack_tick(), Some(3));
        for t in 0..9 {
            buffer.take(t);
        }
        buffer.receive(&InputPacket {
            frames: vec![frame(9, 1)],
        });
        assert_eq!(buffer.ack_tick(), Some(5), "the gap at 4 is abandoned");
    }

    #[test]
    fn redundancy_repairs_lost_packets_over_a_lossy_link() {
        let missing = |redundancy: usize| {
            let link_config = LinkConfig {
                latency_us: 40_000,
                jitter_us: 10_000,
                loss_ppm: 200_000,
                ..LinkConfig::default()
            };
            let mut link = Duplex::<InputPacket<u32>>::symmetric(link_config, 31);
            let cfg = InputConfig {
                redundancy,
                ..config()
            };
            let mut sender = Sender::new(cfg.clone());
            let mut buffer = Buffer::new(cfg, 0);
            let tick_us = 16_667_u64;
            let (mut missing, mut taken) = (0_u32, 0_u32);
            for tick in 0..3_000_u64 {
                let now = tick * tick_us;
                sender.push(tick + 6, tick as u32);
                link.a_to_b.send(now, sender.packet().unwrap(), 64);
                for (_, packet) in link.a_to_b.poll(now) {
                    buffer.receive(&packet);
                }
                if tick > 30 {
                    taken += 1;
                    if !matches!(buffer.take(tick), InputTake::Received(_)) {
                        missing += 1;
                    }
                }
            }
            f64::from(missing) / f64::from(taken)
        };
        let none = missing(1);
        let three = missing(3);
        assert!(
            none > 0.10,
            "without redundancy about 20 % of the ticks lack input: {none}"
        );
        assert!(
            three < 0.03,
            "with 3 frames per packet very few do: {three}"
        );
    }

    #[test]
    fn the_same_packets_give_the_same_result() {
        let run = || {
            let mut buffer = Buffer::new(config(), 0);
            let mut out = Vec::new();
            for t in 0..50_u64 {
                let report = buffer.receive(&InputPacket {
                    frames: vec![frame(t + 1, t as u32), frame(t, t as u32)],
                });
                out.push((report.accepted, report.duplicates, buffer.take(t)));
            }
            out
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn ticks_near_the_top_of_the_range_do_not_overflow() {
        let mut buffer = Buffer::new(config(), u64::MAX - 5);
        let report = buffer.receive(&InputPacket {
            frames: vec![frame(u64::MAX, 1), frame(u64::MAX - 5, 2)],
        });
        assert_eq!(report.accepted, 2);
        assert_eq!(buffer.take(u64::MAX - 5), InputTake::Received(2));
        let mut sender = Sender::new(config());
        assert!(sender.push(u64::MAX, 3));
    }
}
