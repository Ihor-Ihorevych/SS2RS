# Reliable events

`ss2_net::events` (T08-012) carries the things that must happen **exactly once** and cannot be rebuilt from a snapshot:
explosions, pickups, chat lines, sounds. It is generic over the event type, clock-free, and tested over the network simulator
(`docs/TESTING.md` section 3.7).

## The pieces

| Type | Where | Job |
|---|---|---|
| `EventLog<E>` | server, one for all clients | appends events with sequence numbers 1, 2, 3 ...; bounded by `max_events` (256); `prune(now_tick)` drops events older than `max_age_ticks` (600) |
| `EventCursor` | server, one per client | how far that client has acknowledged (never goes backwards) |
| `EventPacket<E>` | the wire | `events` the client has not acknowledged (oldest first, at most `max_events_per_packet` = 16) and `oldest_seq`, where the log now starts |
| `EventReceiver<E>` | client | drops duplicates, delivers each event once and **in order**, reports `ack_seq()` |

## How it works

- **Redundancy instead of resends.** `log.packet(&cursor)` returns every event after the client's acknowledgement, so each packet
  repeats the ones not yet confirmed; a lost packet is repaired by the next one. Put the packet in the snapshot (or beside it).
- **Dedupe.** The receiver ignores a sequence number it has already delivered or buffered and counts it (`duplicates()`); the
  network simulator's duplicated packets and the redundancy both land here.
- **Order.** An event behind a gap waits until the gap is filled.
- **Events that are too old to matter** (an explosion from ten seconds ago) leave the log. The packet's `oldest_seq` tells the
  client the log starts later, and it **gives up on the missing ones** (`skipped()`), so it never waits forever.
- **Late join.** `log.join_cursor()` gives a client that connects now the cursor "everything so far is acknowledged": it receives
  only later events (and ignores an old packet that reaches it). It gets the state of the world from a full snapshot (T08-013),
  not by replaying every sound that was played.

```rust
let mut log = EventLog::new(EventConfig::default());
let mut cursor = log.join_cursor();                      // per client, when it joins
let mut client = EventReceiver::new(cursor.clone());     // on the client
log.emit(tick, Event::Explosion { .. });                 // server, when it happens
send(log.packet(&cursor));                               // server, every snapshot
for e in client.receive(&packet) { play(e.event) }       // client, on a packet
cursor.ack(reply.ack);                                   // server, from the client's ack_seq()
log.prune(tick);                                         // server, every tick
```

## Measured (simulated link)

500 events over a link with 40 ms latency, 30 ms jitter, **25 % loss, 10 % duplicated and 10 % reordered packets**, 4,000 ticks,
the acknowledgement travelling back over an equally bad link: all 500 arrived **exactly once and in order**; the client dropped
3,426 repeated copies; a packet carried **1.34 events on average**. (The server sent a packet on every tick that had something
unacknowledged, 3,501 packets, because the acknowledgement takes several ticks to come back; in the game the events ride on the
snapshots.) A simulation, not a real network.

## Not covered

- **Unreliable events** that may be lost (muzzle flashes, footsteps): send them in the snapshot without a sequence number.
- The **wire format** (T08-030) and the connection (T08-006); the events themselves belong to the game code and to the event bus
  (T08-028), including which events are client-predicted.
- Replication-aware **suppression for clients that cannot see the event** (relevancy) is T08-007.
- The late-join **full snapshot** is T08-013.
