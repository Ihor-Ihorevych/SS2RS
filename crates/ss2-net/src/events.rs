//! Reliable events from the server to the clients (roadmap T08-012): explosions, pickups, chat,
//! sounds, anything that must happen once and cannot be reconstructed from a snapshot.
//!
//! The server appends each event to an [`EventLog`] with a sequence number. For each client it
//! repeats every event the client has not yet acknowledged in every packet ([`EventLog::packet`]),
//! so a lost packet is repaired by the next one with no resend round trip. The client
//! ([`EventReceiver`]) drops the copies it has already seen, hands the events to the game exactly
//! once and **in order**, and reports how far it got ([`EventReceiver::ack_seq`]).
//!
//! Two limits keep it honest. An event that is **too old to matter** (an explosion effect from
//! ten seconds ago) is dropped from the log, and the packet says where the log now starts so a
//! client does not wait forever for it. A client that **joins late** starts after the events
//! already in the log ([`EventLog::join_cursor`]): it gets the world from a full snapshot, not
//! the replay of every sound that was ever played.
//!
//! Generic over the event type; clock-free and deterministic like the rest of the crate.

use std::collections::BTreeMap;

/// The most out-of-order events a receiver buffers, whatever a server sends.
const MAX_PENDING: u64 = 4_096;

/// The limits of the event stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventConfig {
    /// The most events the log keeps; the oldest are forgotten first.
    pub max_events: usize,
    /// The most events in one packet (the oldest unacknowledged ones go first, so order holds).
    pub max_events_per_packet: usize,
    /// An event older than this many ticks is dropped from the log by [`EventLog::prune`].
    pub max_age_ticks: u64,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            max_events: 256,
            max_events_per_packet: 16,
            max_age_ticks: 600,
        }
    }
}

/// One event with its sequence number and the tick it happened on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEntry<E> {
    /// The sequence number, increasing by one per event from 1.
    pub seq: u64,
    /// The simulation tick of the event.
    pub tick: u64,
    /// The event itself.
    pub event: E,
}

/// What the server sends: the events the client has not acknowledged, and where the log starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventPacket<E> {
    /// The oldest sequence number the server still has. A client that is waiting for an older
    /// one gives up on it.
    pub oldest_seq: u64,
    /// The events, oldest first.
    pub events: Vec<EventEntry<E>>,
}

/// How far one client has acknowledged.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventCursor {
    acked: u64,
}

impl EventCursor {
    /// A cursor that has acknowledged nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// The client has every event up to and including `seq` (an acknowledgement never goes back).
    pub fn ack(&mut self, seq: u64) {
        self.acked = self.acked.max(seq);
    }

    /// The highest acknowledged sequence number.
    pub fn acked(&self) -> u64 {
        self.acked
    }
}

/// The server's record of recent events, shared by all clients (each has its own [`EventCursor`]).
#[derive(Clone, Debug)]
pub struct EventLog<E> {
    config: EventConfig,
    entries: BTreeMap<u64, EventEntry<E>>,
    next_seq: u64,
}

impl<E: Clone> EventLog<E> {
    /// An empty log.
    pub fn new(config: EventConfig) -> Self {
        Self {
            config,
            entries: BTreeMap::new(),
            next_seq: 1,
        }
    }

    /// Append an event that happened on `tick`; returns its sequence number.
    pub fn emit(&mut self, tick: u64, event: E) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.insert(seq, EventEntry { seq, tick, event });
        while self.entries.len() > self.config.max_events.max(1) {
            self.entries.pop_first();
        }
        seq
    }

    /// Drop the events older than `max_age_ticks` at tick `now_tick`.
    pub fn prune(&mut self, now_tick: u64) {
        let age = self.config.max_age_ticks;
        self.entries
            .retain(|_, e| e.tick.saturating_add(age) >= now_tick);
    }

    /// The sequence number of the latest event, 0 if there was none.
    pub fn latest_seq(&self) -> u64 {
        self.next_seq - 1
    }

    /// How many events are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no event is kept.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The oldest sequence number still available (the next one to come if the log is empty).
    fn oldest_seq(&self) -> u64 {
        self.entries.keys().next().copied().unwrap_or(self.next_seq)
    }

    /// The packet for a client: every event after its acknowledgement, oldest first, at most
    /// `max_events_per_packet`. Asking again repeats them: that is the redundancy.
    pub fn packet(&self, cursor: &EventCursor) -> EventPacket<E> {
        let events = self
            .entries
            .range(cursor.acked.saturating_add(1)..)
            .take(self.config.max_events_per_packet.max(1))
            .map(|(_, e)| e.clone())
            .collect();
        EventPacket {
            oldest_seq: self.oldest_seq(),
            events,
        }
    }

    /// The cursor for a client that joins now: it has "acknowledged" everything so far, so it
    /// only receives events that happen after it joined.
    pub fn join_cursor(&self) -> EventCursor {
        EventCursor {
            acked: self.latest_seq(),
        }
    }
}

/// The client's end: drops duplicates, delivers in order, reports progress.
#[derive(Clone, Debug)]
pub struct EventReceiver<E> {
    delivered_through: u64,
    pending: BTreeMap<u64, EventEntry<E>>,
    duplicates: u64,
    skipped: u64,
}

impl<E: Clone> EventReceiver<E> {
    /// A receiver that starts after `cursor` (a late joiner's cursor from
    /// [`EventLog::join_cursor`], or [`EventCursor::new`] for one that starts at the beginning).
    pub fn new(cursor: EventCursor) -> Self {
        Self {
            delivered_through: cursor.acked,
            pending: BTreeMap::new(),
            duplicates: 0,
            skipped: 0,
        }
    }

    /// Take a packet; returns the events that can now be handed to the game, each exactly once
    /// and in sequence order. Events behind a gap wait for it to be filled, unless the server
    /// says it no longer has the missing ones.
    pub fn receive(&mut self, packet: &EventPacket<E>) -> Vec<EventEntry<E>> {
        if packet.oldest_seq > self.delivered_through + 1 {
            self.skipped += packet.oldest_seq - self.delivered_through - 1;
            self.delivered_through = packet.oldest_seq - 1;
            self.pending = self.pending.split_off(&packet.oldest_seq);
        }
        for entry in &packet.events {
            if entry.seq <= self.delivered_through || self.pending.contains_key(&entry.seq) {
                self.duplicates += 1;
            } else if entry.seq - self.delivered_through <= MAX_PENDING {
                self.pending.insert(entry.seq, entry.clone());
            }
        }
        let mut out = Vec::new();
        while let Some(entry) = self.pending.remove(&(self.delivered_through + 1)) {
            self.delivered_through += 1;
            out.push(entry);
        }
        out
    }

    /// The sequence number up to which every event has been delivered (or given up on): send it
    /// back to the server.
    pub fn ack_seq(&self) -> u64 {
        self.delivered_through
    }

    /// How many repeated copies were dropped.
    pub fn duplicates(&self) -> u64 {
        self.duplicates
    }

    /// How many events were given up on because the server no longer had them.
    pub fn skipped(&self) -> u64 {
        self.skipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::{LinkConfig, NetSim};

    fn config() -> EventConfig {
        EventConfig::default()
    }

    fn seqs(packet: &EventPacket<u32>) -> Vec<u64> {
        packet.events.iter().map(|e| e.seq).collect()
    }

    #[test]
    fn events_get_increasing_sequence_numbers_from_one() {
        let mut log = EventLog::<u32>::new(config());
        assert_eq!(log.emit(10, 100), 1);
        assert_eq!(log.emit(10, 101), 2);
        assert_eq!(log.emit(12, 102), 3);
        assert_eq!(log.latest_seq(), 3);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn everything_unacknowledged_is_repeated_oldest_first_until_acknowledged() {
        let mut log = EventLog::<u32>::new(config());
        let mut cursor = EventCursor::new();
        for i in 0..4 {
            log.emit(1, i);
        }
        assert_eq!(seqs(&log.packet(&cursor)), [1, 2, 3, 4]);
        assert_eq!(
            seqs(&log.packet(&cursor)),
            [1, 2, 3, 4],
            "asking again changes nothing: it is repeated"
        );
        cursor.ack(2);
        assert_eq!(seqs(&log.packet(&cursor)), [3, 4]);
        cursor.ack(4);
        assert!(log.packet(&cursor).events.is_empty());
    }

    #[test]
    fn an_acknowledgement_never_goes_backwards() {
        let mut cursor = EventCursor::new();
        cursor.ack(5);
        cursor.ack(3);
        assert_eq!(cursor.acked(), 5);
    }

    #[test]
    fn a_packet_is_limited_and_carries_the_oldest_events_first() {
        let mut log = EventLog::<u32>::new(EventConfig {
            max_events_per_packet: 3,
            ..config()
        });
        let cursor = EventCursor::new();
        for i in 0..10 {
            log.emit(1, i);
        }
        assert_eq!(
            seqs(&log.packet(&cursor)),
            [1, 2, 3],
            "the oldest first, so the order is kept"
        );
    }

    #[test]
    fn the_client_delivers_each_event_once_and_in_order() {
        let mut log = EventLog::<u32>::new(config());
        let mut client = EventReceiver::<u32>::new(EventCursor::new());
        for i in 0..5 {
            log.emit(1, i * 10);
        }
        let packet = log.packet(&EventCursor::new());
        let got = client.receive(&packet);
        assert_eq!(
            got.iter().map(|e| e.event).collect::<Vec<_>>(),
            [0, 10, 20, 30, 40]
        );
        assert_eq!(client.ack_seq(), 5);
        assert!(
            client.receive(&packet).is_empty(),
            "the same packet again delivers nothing"
        );
        assert_eq!(client.duplicates(), 5);
    }

    #[test]
    fn a_gap_holds_later_events_until_it_is_filled() {
        let mut client = EventReceiver::<u32>::new(EventCursor::new());
        let entry = |seq| EventEntry {
            seq,
            tick: 1,
            event: seq as u32,
        };
        let first = EventPacket {
            oldest_seq: 1,
            events: vec![entry(1), entry(3), entry(4)],
        };
        let got = client.receive(&first);
        assert_eq!(
            got.iter().map(|e| e.seq).collect::<Vec<_>>(),
            [1],
            "2 is missing, so 3 and 4 wait"
        );
        assert_eq!(client.ack_seq(), 1);
        let second = EventPacket {
            oldest_seq: 1,
            events: vec![entry(2), entry(3), entry(4)],
        };
        let got = client.receive(&second);
        assert_eq!(got.iter().map(|e| e.seq).collect::<Vec<_>>(), [2, 3, 4]);
        assert_eq!(client.ack_seq(), 4);
    }

    #[test]
    fn an_event_the_server_has_dropped_is_skipped_not_waited_for() {
        let mut log = EventLog::<u32>::new(EventConfig {
            max_age_ticks: 100,
            ..config()
        });
        for i in 0..3 {
            log.emit(10, i); // seq 1..=3 at tick 10
        }
        log.emit(500, 99); // seq 4, much later
        log.prune(500);
        assert_eq!(log.len(), 1, "the old events were dropped");
        let mut client = EventReceiver::<u32>::new(EventCursor::new());
        let packet = log.packet(&EventCursor::new());
        assert_eq!(packet.oldest_seq, 4);
        let got = client.receive(&packet);
        assert_eq!(got.iter().map(|e| e.event).collect::<Vec<_>>(), [99]);
        assert_eq!(
            client.ack_seq(),
            4,
            "the client moved on over the lost ones"
        );
        assert_eq!(client.skipped(), 3);
    }

    #[test]
    fn a_client_that_joins_late_gets_only_later_events() {
        let mut log = EventLog::<u32>::new(config());
        for i in 0..6 {
            log.emit(1, i);
        }
        let cursor = log.join_cursor();
        let mut client = EventReceiver::<u32>::new(cursor.clone());
        log.emit(2, 100);
        log.emit(2, 101);
        let got = client.receive(&log.packet(&cursor));
        assert_eq!(got.iter().map(|e| e.event).collect::<Vec<_>>(), [100, 101]);
        // Even if an old packet reaches the new client, the old events are ignored.
        let old = EventPacket {
            oldest_seq: 1,
            events: vec![EventEntry {
                seq: 2,
                tick: 1,
                event: 1_u32,
            }],
        };
        assert!(client.receive(&old).is_empty());
    }

    #[test]
    fn the_log_is_bounded() {
        let mut log = EventLog::<u32>::new(EventConfig {
            max_events: 10,
            ..config()
        });
        for i in 0..50 {
            log.emit(1, i);
        }
        assert_eq!(log.len(), 10);
        assert_eq!(log.latest_seq(), 50);
        assert_eq!(log.packet(&EventCursor::new()).oldest_seq, 41);
    }

    #[test]
    fn every_event_arrives_exactly_once_and_in_order_over_a_lossy_duplicating_link() {
        let link_config = LinkConfig {
            latency_us: 40_000,
            jitter_us: 30_000,
            loss_ppm: 250_000,
            duplicate_ppm: 100_000,
            reorder_ppm: 100_000,
            reorder_delay_us: 40_000,
            ..LinkConfig::default()
        };
        let mut down = NetSim::<EventPacket<u32>>::new(link_config.clone(), 7);
        let mut up = NetSim::<u64>::new(link_config, 8);
        let mut log = EventLog::<u32>::new(config());
        let mut cursor = EventCursor::new();
        let mut client = EventReceiver::<u32>::new(EventCursor::new());
        let mut delivered = Vec::new();
        let (mut sent_packets, mut carried) = (0_usize, 0_usize);
        for tick in 0..4_000_u64 {
            let now = tick * 16_667;
            if tick % 7 == 0 && tick < 3_500 {
                log.emit(tick, tick as u32);
            }
            let packet = log.packet(&cursor);
            if !packet.events.is_empty() {
                sent_packets += 1;
                carried += packet.events.len();
                down.send(now, packet, 200);
            }
            for (_, packet) in down.poll(now) {
                delivered.extend(client.receive(&packet));
                // The client acknowledges what it has, as a value that travels back and can be lost.
                up.send(now, client.ack_seq(), 16);
            }
            for (_, ack) in up.poll(now) {
                cursor.ack(ack);
            }
            log.prune(tick);
        }
        let expected: Vec<u32> = (0..3_500_u64)
            .filter(|t| t % 7 == 0)
            .map(|t| t as u32)
            .collect();
        assert_eq!(
            delivered.iter().map(|e| e.event).collect::<Vec<_>>(),
            expected,
            "all events, once, in order"
        );
        assert!(client.duplicates() > 0, "the redundant copies were dropped");
        assert!(
            carried / sent_packets < 8,
            "the repetition stays small: {} events per packet",
            carried / sent_packets
        );
    }
}
