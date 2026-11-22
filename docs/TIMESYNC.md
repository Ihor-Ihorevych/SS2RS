# Time synchronization

`ss2_net::timesync` (T08-016) lets a client agree with the server on what time it is, to a few milliseconds, so that it can
place server snapshots on its own timeline, stamp its inputs with the right tick and decide how long to buffer incoming data.
It is plain functions and structs with **no clock of its own**: the caller passes the local time in microseconds, which makes
it testable with a virtual clock and the network simulator (`docs/TESTING.md` section 3.7).

## The pieces

| Type | Job |
|---|---|
| `ClockSync` | turns ping and pong exchanges into an estimate of **server clock minus local clock** (`offset_us`), the shortest **round trip** (`rtt_us`) and the **jitter** (`jitter_us`); schedules the pings (`ping_due`, `note_ping_sent`) |
| `TickClock` | converts server time to a simulation tick and back at a fixed rate; `tick_at(time_of_tick(n)) == n` |
| `jitter_buffer_delay_us` | how long to hold incoming data: `min + factor x jitter`, capped |

## How a client uses it

```rust
let mut sync = ClockSync::new(SyncConfig::default());
if sync.ping_due(now_us) {                       // every 100 ms at first, every second once synced
    sync.note_ping_sent(now_us);
    send_ping(now_us);                           // the server answers with its own clock
}
// when the reply arrives:
sync.on_pong(sent_us, server_time_us, now_us)?;  // ReplyBeforeRequest if the local clock went backwards
if let Some(server_now) = sync.server_time_us(now_us) {
    let tick = TickClock::new(60).tick_at(server_now);
    let hold = jitter_buffer_delay_us(sync.jitter_us().unwrap_or(0), &JitterBufferConfig::default());
}
```

## What to expect (and what it cannot do)

- **Shortest round trip wins.** Of the last 16 round trips the one with the smallest delay (the least queueing) decides the
  offset; a single huge round trip changes nothing.
- **Asymmetric paths are off by half the difference.** A round trip only gives the sum of the two one-way delays. With 10 ms up
  and 30 ms down the estimate is 10 ms off, and no amount of data can show that. Do not use it to measure one-way latency.
- **Smoothing.** Once `is_synced` (5 accepted round trips) the published offset moves at most 2 ms per sample toward the best
  estimate, so the game clock does not jump; a change over 100 ms is taken at once (the clocks were reset). Before that the best
  estimate is used directly, so the start is quick.
- **Measured in the tests:** over 150 s with 40 ms of jitter each way, 5 % loss and a server clock running 100 ppm fast, the
  estimate stayed within **6.7 ms** of the truth after 20 s of warm-up (the test asserts 20 ms). That is a simulation, not a
  measurement of a real network.

## Not covered

- Nothing calls it yet: the transport (T08-005) and the snapshot buffer (T08-008) that would use the jitter buffer do not
  exist. The jitter buffer here only computes the delay; the buffer itself is T08-008.
- The wire format of the ping and pong (T08-030).
- Smoothing of the **local game clock** against the estimate (slewing the tick rate) is left to the client loop; this module
  hands out the offset.
