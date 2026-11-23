//! The gameplay event bus (roadmap T08-028): how things in the simulation tell each other that
//! something happened, with replication built in.
//!
//! A door opens, a boss dies, a trigger fires: code **emits** an event; the bus **delivers** it to
//! every subscriber at a fixed point of the tick ([`EventBus::flush`]), in a fixed order. Events can
//! be **delayed** ("fire this output in 2 seconds": [`EventBus::emit_after`]) and cancelled. Each
//! event has an **audience**: [`Audience::Local`] stays in the simulation, the others are also
//! recorded for the network layer, which sends them to clients (`ss2_net::events` delivers them
//! exactly once, in order, with late-join suppression: see `docs/EVENTS.md`).
//!
//! Deterministic by construction: every event gets a sequence number; delivery order is by due
//! tick, then sequence number; subscribers run in registration order; nothing reads a clock or uses a
//! hash map. Bounded: the queue has a capacity and a flush has a round limit, so a handler that
//! emits an event that emits an event... cannot hang the tick.

use std::fmt;

/// Who an event is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    /// The simulation only: not sent to anyone.
    Local,
    /// Every client.
    All,
    /// One client, by player slot.
    Player(u8),
}

/// The limits of a bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusConfig {
    /// The most events waiting (delivered or delayed) at once; more are refused and counted.
    pub max_queued: usize,
    /// The most rounds of delivery in one flush: a round is one pass over the events that were
    /// waiting, and what handlers emit goes to the next round.
    pub max_rounds: u32,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            max_queued: 4096,
            max_rounds: 8,
        }
    }
}

/// Why a subscriber could not be added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusError {
    /// A subscriber of this name exists.
    DuplicateSubscriber(String),
    /// The name is empty.
    EmptyName,
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSubscriber(name) => write!(f, "a subscriber called {name:?} exists"),
            Self::EmptyName => write!(f, "a subscriber needs a name"),
        }
    }
}

impl std::error::Error for BusError {}

/// A way to cancel a delayed event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventHandle(u64);

/// An event recorded for the network layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replicated<E> {
    /// The sequence number, increasing in emission order.
    pub seq: u64,
    /// The tick at which it was delivered.
    pub tick: u64,
    /// Who it is for.
    pub audience: Audience,
    /// The event.
    pub event: E,
}

/// What one flush did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlushReport {
    /// Events delivered.
    pub delivered: usize,
    /// Rounds run.
    pub rounds: u32,
    /// Events that were due but left for the next flush because the round limit was reached.
    pub carried_over: usize,
}

/// What a handler may emit while it runs; it goes to the next round.
#[derive(Debug)]
pub struct Outbox<E> {
    items: Vec<(u64, E, Audience)>,
}

impl<E> Outbox<E> {
    /// Emit an event for the next round.
    pub fn emit(&mut self, event: E, audience: Audience) {
        self.items.push((0, event, audience));
    }

    /// Emit an event to be delivered `delay_ticks` ticks from now.
    pub fn emit_after(&mut self, delay_ticks: u64, event: E, audience: Audience) {
        self.items.push((delay_ticks, event, audience));
    }
}

struct Queued<E> {
    due: u64,
    seq: u64,
    audience: Audience,
    event: E,
}

type Handler<W, E> = Box<dyn FnMut(&mut W, &E, &mut Outbox<E>)>;

/// The bus; `W` is the world the handlers may change and `E` the event type (usually an enum).
///
/// ```
/// use ss2_sim::{Audience, BusConfig, EventBus};
///
/// let mut bus: EventBus<Vec<u32>, u32> = EventBus::new(BusConfig::default());
/// bus.subscribe("log", |world: &mut Vec<u32>, event: &u32, _| world.push(*event)).unwrap();
/// bus.begin_tick(0);
/// bus.emit(1, Audience::All);
/// bus.emit_after(2, 2, Audience::Local); // two ticks from now
/// let mut world = Vec::new();
/// bus.flush(&mut world);
/// assert_eq!(world, [1]);
/// bus.begin_tick(2);
/// bus.flush(&mut world);
/// assert_eq!(world, [1, 2]);
/// assert_eq!(bus.take_replicated().len(), 1, "only the audience event is for the network");
/// ```
pub struct EventBus<W, E> {
    config: BusConfig,
    tick: u64,
    next_seq: u64,
    queue: Vec<Queued<E>>,
    subscribers: Vec<(String, Handler<W, E>)>,
    replicated: Vec<Replicated<E>>,
    dropped: u64,
}

impl<W, E: Clone> EventBus<W, E> {
    /// An empty bus at tick 0.
    pub fn new(config: BusConfig) -> Self {
        Self {
            config,
            tick: 0,
            next_seq: 0,
            queue: Vec::new(),
            subscribers: Vec::new(),
            replicated: Vec::new(),
            dropped: 0,
        }
    }

    /// Add a subscriber: it is called for every event, after the subscribers added before it.
    pub fn subscribe(
        &mut self,
        name: &str,
        handler: impl FnMut(&mut W, &E, &mut Outbox<E>) + 'static,
    ) -> Result<(), BusError> {
        if name.is_empty() {
            return Err(BusError::EmptyName);
        }
        if self.subscribers.iter().any(|(n, _)| n == name) {
            return Err(BusError::DuplicateSubscriber(name.to_owned()));
        }
        self.subscribers.push((name.to_owned(), Box::new(handler)));
        Ok(())
    }

    /// Remove a subscriber; `false` if there is none of that name.
    pub fn unsubscribe(&mut self, name: &str) -> bool {
        let before = self.subscribers.len();
        self.subscribers.retain(|(n, _)| n != name);
        self.subscribers.len() != before
    }

    /// Tell the bus which tick it is; delayed events count from here.
    pub fn begin_tick(&mut self, tick: u64) {
        self.tick = tick;
    }

    /// How many events are waiting.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// How many events were refused because the queue was full.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    fn enqueue(&mut self, delay: u64, event: E, audience: Audience) -> Option<EventHandle> {
        if self.queue.len() >= self.config.max_queued {
            self.dropped += 1;
            return None;
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        let due = self.tick.saturating_add(delay);
        let at = self.queue.partition_point(|q| (q.due, q.seq) < (due, seq));
        self.queue.insert(
            at,
            Queued {
                due,
                seq,
                audience,
                event,
            },
        );
        Some(EventHandle(seq))
    }

    /// Emit an event for the next flush; `None` if the queue is full.
    pub fn emit(&mut self, event: E, audience: Audience) -> Option<EventHandle> {
        self.enqueue(0, event, audience)
    }

    /// Emit an event to be delivered `delay_ticks` ticks from now; `None` if the queue is full.
    pub fn emit_after(
        &mut self,
        delay_ticks: u64,
        event: E,
        audience: Audience,
    ) -> Option<EventHandle> {
        self.enqueue(delay_ticks, event, audience)
    }

    /// Cancel an event that has not been delivered; `false` if it already was (or never existed).
    pub fn cancel(&mut self, handle: EventHandle) -> bool {
        let before = self.queue.len();
        self.queue.retain(|q| q.seq != handle.0);
        self.queue.len() != before
    }

    /// Deliver every event that is due, in rounds, to every subscriber. What handlers emit
    /// is delivered in the next round (up to [`BusConfig::max_rounds`]); what is left stays for the
    /// next flush.
    pub fn flush(&mut self, world: &mut W) -> FlushReport {
        let (mut delivered, mut rounds) = (0, 0);
        while rounds < self.config.max_rounds {
            let due_now = self.queue.partition_point(|q| q.due <= self.tick);
            if due_now == 0 {
                break;
            }
            rounds += 1;
            let batch: Vec<Queued<E>> = self.queue.drain(..due_now).collect();
            let mut outbox = Outbox { items: Vec::new() };
            for queued in batch {
                if queued.audience != Audience::Local {
                    self.replicated.push(Replicated {
                        seq: queued.seq,
                        tick: self.tick,
                        audience: queued.audience,
                        event: queued.event.clone(),
                    });
                }
                for (_, handler) in &mut self.subscribers {
                    handler(world, &queued.event, &mut outbox);
                }
                delivered += 1;
            }
            for (delay, event, audience) in outbox.items {
                self.enqueue(delay, event, audience);
            }
        }
        let carried_over = self.queue.partition_point(|q| q.due <= self.tick);
        FlushReport {
            delivered,
            rounds,
            carried_over,
        }
    }

    /// Take the events recorded for the network since the last call, in delivery order.
    pub fn take_replicated(&mut self) -> Vec<Replicated<E>> {
        std::mem::take(&mut self.replicated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Bus = EventBus<Vec<String>, String>;

    fn bus() -> Bus {
        EventBus::new(BusConfig::default())
    }

    fn log_handler(
        label: &'static str,
    ) -> impl FnMut(&mut Vec<String>, &String, &mut Outbox<String>) + 'static {
        move |world, event, _| world.push(format!("{label}:{event}"))
    }

    #[test]
    fn events_are_delivered_in_emission_order_to_subscribers_in_registration_order() {
        let mut bus = bus();
        bus.subscribe("a", log_handler("a")).unwrap();
        bus.subscribe("b", log_handler("b")).unwrap();
        bus.emit("one".into(), Audience::Local);
        bus.emit("two".into(), Audience::Local);
        let mut world = Vec::new();
        let report = bus.flush(&mut world);
        assert_eq!(world, ["a:one", "b:one", "a:two", "b:two"]);
        assert_eq!(
            (report.delivered, report.rounds, report.carried_over),
            (2, 1, 0)
        );
        assert!(
            bus.flush(&mut world).delivered == 0,
            "nothing is delivered twice"
        );
    }

    #[test]
    fn subscriber_names_are_unique_and_a_subscriber_can_be_removed() {
        let mut bus = bus();
        bus.subscribe("a", log_handler("a")).unwrap();
        assert_eq!(
            bus.subscribe("a", log_handler("again")),
            Err(BusError::DuplicateSubscriber("a".into()))
        );
        assert_eq!(
            bus.subscribe("", log_handler("x")),
            Err(BusError::EmptyName)
        );
        assert!(bus.unsubscribe("a") && !bus.unsubscribe("a"));
        bus.emit("e".into(), Audience::Local);
        let mut world = Vec::new();
        bus.flush(&mut world);
        assert!(
            world.is_empty(),
            "no subscriber, no delivery, and the event is gone"
        );
    }

    #[test]
    fn what_a_handler_emits_is_delivered_in_the_next_round_of_the_same_flush() {
        let mut bus = bus();
        bus.subscribe(
            "chain",
            |world: &mut Vec<String>, event: &String, out: &mut Outbox<String>| {
                world.push(event.clone());
                if event == "start" {
                    out.emit("second".into(), Audience::Local);
                } else if event == "second" {
                    out.emit("third".into(), Audience::Local);
                }
            },
        )
        .unwrap();
        bus.emit("start".into(), Audience::Local);
        bus.emit("other".into(), Audience::Local);
        let mut world = Vec::new();
        let report = bus.flush(&mut world);
        assert_eq!(
            world,
            ["start", "other", "second", "third"],
            "a round at a time, not depth first"
        );
        assert_eq!(report.rounds, 3);
    }

    #[test]
    fn a_runaway_chain_stops_at_the_round_limit_and_carries_the_rest_over() {
        let mut bus = EventBus::new(BusConfig {
            max_rounds: 4,
            ..BusConfig::default()
        });
        bus.subscribe(
            "loop",
            |world: &mut Vec<String>, event: &String, out: &mut Outbox<String>| {
                world.push(event.clone());
                out.emit(event.clone(), Audience::Local);
            },
        )
        .unwrap();
        bus.emit("ping".into(), Audience::Local);
        let mut world = Vec::new();
        let report = bus.flush(&mut world);
        assert_eq!(
            (world.len(), report.rounds, report.carried_over),
            (4, 4, 1),
            "it ends, and says one is left"
        );
        bus.flush(&mut world);
        assert_eq!(
            world.len(),
            8,
            "the next flush carries on, four more rounds"
        );
    }

    #[test]
    fn a_delayed_event_arrives_on_its_tick_and_not_before() {
        let mut bus = bus();
        bus.subscribe("log", log_handler("l")).unwrap();
        bus.begin_tick(10);
        bus.emit_after(3, "later".into(), Audience::Local);
        bus.emit_after(0, "now".into(), Audience::Local);
        let mut world = Vec::new();
        for tick in 10..=13 {
            bus.begin_tick(tick);
            bus.flush(&mut world);
            let expect: &[&str] = match tick {
                10 => &["l:now"],
                13 => &["l:now", "l:later"],
                _ => &["l:now"],
            };
            assert_eq!(world, expect, "tick {tick}");
        }
        assert_eq!(bus.pending(), 0);
    }

    #[test]
    fn events_due_at_the_same_tick_go_by_emission_order_and_a_late_flush_catches_up() {
        let mut bus = bus();
        bus.subscribe("log", log_handler("l")).unwrap();
        bus.begin_tick(0);
        bus.emit_after(5, "a".into(), Audience::Local);
        bus.emit_after(2, "b".into(), Audience::Local);
        bus.emit_after(5, "c".into(), Audience::Local);
        let mut world = Vec::new();
        bus.begin_tick(9); // a tick was skipped (for example after loading a save): everything due is delivered, in order
        bus.flush(&mut world);
        assert_eq!(
            world,
            ["l:b", "l:a", "l:c"],
            "by due tick, then by emission order"
        );
    }

    #[test]
    fn a_delayed_event_can_be_cancelled_until_it_is_due() {
        let mut bus = bus();
        bus.subscribe("log", log_handler("l")).unwrap();
        bus.begin_tick(0);
        let timer = bus.emit_after(5, "boom".into(), Audience::Local).unwrap();
        let keep = bus.emit_after(5, "keep".into(), Audience::Local).unwrap();
        assert!(bus.cancel(timer));
        assert!(!bus.cancel(timer), "already cancelled");
        let mut world = Vec::new();
        bus.begin_tick(5);
        bus.flush(&mut world);
        assert_eq!(world, ["l:keep"]);
        assert!(!bus.cancel(keep), "already delivered");
    }

    #[test]
    fn only_events_with_an_audience_are_recorded_for_the_network_with_their_tick_and_order() {
        let mut bus = bus();
        bus.subscribe("log", log_handler("l")).unwrap();
        let mut world = Vec::new();
        bus.begin_tick(7);
        bus.emit("secret".into(), Audience::Local);
        bus.emit("boom".into(), Audience::All);
        bus.emit("for you".into(), Audience::Player(2));
        bus.flush(&mut world);
        let sent = bus.take_replicated();
        assert_eq!(sent.len(), 2);
        assert_eq!(
            (sent[0].tick, sent[0].audience, sent[0].event.as_str()),
            (7, Audience::All, "boom")
        );
        assert_eq!(
            (sent[1].tick, sent[1].audience, sent[1].event.as_str()),
            (7, Audience::Player(2), "for you")
        );
        assert!(sent[0].seq < sent[1].seq);
        assert!(bus.take_replicated().is_empty(), "taken once");
        // A delayed event is recorded when it is delivered, stamped with that tick.
        bus.emit_after(4, "later".into(), Audience::All).unwrap();
        bus.begin_tick(11);
        bus.flush(&mut world);
        let sent = bus.take_replicated();
        assert_eq!((sent.len(), sent[0].tick), (1, 11));
    }

    #[test]
    fn a_late_delivery_is_recorded_at_the_tick_it_was_delivered_not_the_one_it_was_due() {
        let mut bus = bus();
        bus.begin_tick(0);
        bus.emit_after(2, "due at 2".into(), Audience::All).unwrap();
        bus.begin_tick(9);
        bus.flush(&mut Vec::new());
        assert_eq!(
            bus.take_replicated()[0].tick,
            9,
            "clients are told when it happened to them"
        );
    }

    #[test]
    fn the_queue_is_bounded_and_says_what_it_refused() {
        let mut bus = EventBus::new(BusConfig {
            max_queued: 3,
            ..BusConfig::default()
        });
        bus.subscribe("log", log_handler("l")).unwrap();
        for i in 0..3 {
            assert!(bus.emit(format!("e{i}"), Audience::Local).is_some());
        }
        assert_eq!(bus.emit("e3".into(), Audience::Local), None, "full");
        assert_eq!(
            bus.emit_after(2, "late".into(), Audience::Local),
            None,
            "delayed events count too"
        );
        assert_eq!(bus.dropped(), 2);
        let mut world = Vec::new();
        bus.flush(&mut world);
        assert_eq!(world.len(), 3);
        assert!(
            bus.emit("again".into(), Audience::Local).is_some(),
            "room again"
        );
    }

    #[test]
    fn handlers_can_see_the_world_and_change_it() {
        let mut bus: EventBus<u32, u32> = EventBus::new(BusConfig::default());
        bus.subscribe("sum", |world: &mut u32, event: &u32, _| *world += *event)
            .unwrap();
        for n in 1..=4 {
            bus.emit(n, Audience::Local);
        }
        let mut total = 0;
        bus.flush(&mut total);
        assert_eq!(total, 10);
    }

    #[test]
    fn the_same_emissions_give_the_same_log_and_the_same_sequence_numbers() {
        let run = || {
            let mut bus = bus();
            bus.subscribe(
                "echo",
                |world: &mut Vec<String>, event: &String, out: &mut Outbox<String>| {
                    world.push(event.clone());
                    if event.starts_with('x') {
                        out.emit_after(1, format!("echo-{event}"), Audience::All);
                    }
                },
            )
            .unwrap();
            let mut world = Vec::new();
            for tick in 0..6 {
                bus.begin_tick(tick);
                if tick % 2 == 0 {
                    bus.emit(format!("x{tick}"), Audience::All);
                }
                bus.flush(&mut world);
            }
            let seqs: Vec<u64> = bus.take_replicated().iter().map(|r| r.seq).collect();
            (world, seqs)
        };
        assert_eq!(run(), run());
    }
}
