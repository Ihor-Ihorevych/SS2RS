# The gameplay event bus

`ss2_sim::EventBus` (T08-028) is how things in the simulation tell each other that something happened, and how the simulation decides what
the clients hear about. A door opens, a boss dies, a trigger fires: code **emits** an event; the bus **delivers** it to every subscriber at
a fixed point of the tick, in a fixed order, and **records** the ones that clients must hear.

```rust
let mut bus: EventBus<World, GameEvent> = EventBus::new(BusConfig::default());
bus.subscribe("doors", |world, event, out| {
    if let GameEvent::ButtonPressed(id) = event {
        world.open_door(*id);
        out.emit_after(60, GameEvent::DoorClose(*id), Audience::All);   // an output with a delay: close it in 2 s at 30 Hz
    }
})?;

// every tick:
bus.begin_tick(tick);
bus.emit(GameEvent::ButtonPressed(7), Audience::All);     // from gameplay code
bus.flush(&mut world);                                     // e.g. from a system in Phase::Damage (docs/SIMLOOP.md)
for e in bus.take_replicated() { net_log.emit(e.tick, e.event); }   // to ss2_net::events (docs/EVENTS.md)
```

## Rules

- **Order is fixed.** Every event gets a sequence number when it is emitted. Delivery is by **due tick, then sequence number**; each
  event goes to the subscribers in **registration order**. Nothing reads a clock or uses a hash map.
- **Rounds.** `flush` delivers what is due in rounds: a round is one pass over the events that were waiting, and what handlers emit
  (through the `Outbox`) goes to the **next round**, not depth first. So an event never interrupts another event's delivery, and the order is
  easy to reason about.
- **No runaway.** A flush runs at most `max_rounds` rounds (default 8). A handler that re-emits its own event is stopped there, the flush
  reports `carried_over`, and the rest is delivered by the next flush.
- **Bounded.** At most `max_queued` events wait (default 4096); more are refused (`emit` returns `None`) and counted (`dropped()`).
- **Delays and cancelling.** `emit_after(ticks, ..)` delivers on tick `now + ticks`; `cancel(handle)` withdraws it until it is delivered.
  If the flush runs late (a skipped tick, a loaded save), everything due is delivered, in order.
- **Audience.** `Local` stays in the simulation. `All` and `Player(slot)` are also **recorded** when they are delivered, stamped with the
  tick of delivery, and `take_replicated()` hands them to the network layer in delivery order. The bus does not send anything itself:
  `ss2_net::events::EventLog` repeats them until acknowledged, delivers them once and in order, and suppresses what a late joiner must not
  see (`docs/EVENTS.md`).

## Triggers and outputs

"This entity's output fires that entity's input after a delay" is `emit_after` with an event that names its target. What the names mean
(entities, classes, the outputs a level designer wires up) is the entity model's job (T08-002) and the level data's; the bus only
guarantees the delivery.

## Tests

Twelve unit tests, written first and seen to fail on the missing API: delivery order and subscriber order, unique names and removal,
follow-up events in the next round (not depth first), the round limit with the rest carried over to the next flush, delayed events on
their tick and not before, equal due ticks by emission order and a late flush catching up, cancelling, the audience filter with tick and
order, recording at the tick of **delivery** (a late delivery is stamped when it arrived, not when it was due), the capacity limit, handlers
changing the world, and two runs giving the same log and the same sequence numbers. Nine deliberate breakages each fail a test (no round
limit: the test hangs and the harness's timeout stops it; the queue not ordered by due tick; local events recorded; no capacity limit; the
delay not relative to the tick; subscribers in reverse; cancel doing nothing; carried-over not reported; the recorded tick being the due
tick: the last one **survived at first** and I added the late-delivery test for it).

## Not covered

- **Typed event routing** (subscribers to one kind of event only): a subscriber filters in its handler for now; a router can be added without
  changing the bus.
- **Per-client filtering of `All`** (relevancy: a client far away need not hear a footstep) and **priorities** between events: the
  replication framework (T08-007).
- **Wiring into `Simulation`** (a flush system in a phase) and the **entity model's** outputs and inputs (T08-002).
- **Handlers that fail:** a panic in a handler propagates; there is no isolation between subscribers.
- **Parallel delivery**: delivery is sequential on purpose.
