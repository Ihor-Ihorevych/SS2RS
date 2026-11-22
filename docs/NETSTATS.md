# Net statistics for a `net_graph` and for profiling

`ss2_net::netstats` (T08-018) collects the numbers a network overlay and a profiler need. It is plain data: it takes numbers and never
reads a clock, so it is tested with exact values and can be fed from `ss2_net::netsim` or from a real transport alike.

## The pieces

| Type | What it measures |
|---|---|
| `TrafficMeter<C>` | the bytes of each kind of traffic (`C` is your enum: inputs, snapshots, events, ...) per simulation tick over a sliding window: `record(kind, bytes)` during a tick, `end_tick()` after it; `total`, `bytes_per_second(kind, tick_hz)`, `bits_per_second_all`, `peak_tick`, `series` (per tick, oldest first, for drawing) |
| `LinkMeter` | the latest round trips and packet outcomes: `record_rtt`, `record_packet(acknowledged)`; mean, min, max, any percentile (nearest rank), the jitter (mean deviation) and the loss in parts per thousand |
| `NetGraph` | `NetGraph::summarize(&traffic, &link, tick_hz)` gathers what an overlay shows: bytes per second, mean and 95th percentile round trip, jitter, loss, the round trip series |

Rates use the ticks actually measured, so half a second into a session the figure already means "per second"; windows are bounded
(a window of 0 is read as 1); byte counts saturate instead of wrapping. Round trips can come from `ClockSync` (`docs/TIMESYNC.md`);
a packet is "lost" when the sender gives up on it (no acknowledgement within the repetition window of `docs/INPUTSTREAM.md` or
`docs/EVENTS.md`).

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Traffic { Input, Snapshot, Event }

let mut traffic = TrafficMeter::new(120);               // two seconds at 60 ticks a second
// every tick:
traffic.record(Traffic::Snapshot, packet.len());
traffic.end_tick();
// every frame, for the overlay:
let graph = NetGraph::summarize(&traffic, &link, 60);   // graph.bytes_per_second_total, graph.rtt_p95_us, ...
```

## Not covered (the rest of T08-018)

- **The overlay itself** (the `net_graph` that draws these numbers): it needs the in-game overlay of the UI (T10-007).
- **Prediction error** (how far the client's prediction was from the server's state) can only be measured once prediction exists (T08-009).
- **Profiler scopes:** the optional puffin scopes of `docs/PROFILING.md` are in `ss2-app` only; `ss2-net` is sim-safe and does not
  link a profiler. A scope around the send and receive paths belongs in the code that calls `ss2-net` (the transport crate, RFC 0006).
- **Per-component traffic**: the meter counts by whatever kind you pass; breaking snapshots down by component needs the replication
  framework (T08-007) to say which component a byte belongs to.
