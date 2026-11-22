# Input command stream

`ss2_net::inputstream` (T08-011) carries the player's inputs from a client to the server. It is generic over the input type
(the movement and weapon state come with the game code) and clock-free like the rest of the crate, so it is tested with the
network simulator (`docs/TESTING.md` section 3.7).

## The client: `InputSender<T>`

- `push(tick, input)` records the input for a simulation tick. Ticks must increase; a tick that is not newer is refused.
- `packet()` returns the **latest `redundancy` (default 3) unacknowledged frames**, oldest first, at most
  `max_frames_per_packet`. A lost packet is repaired by the next one: no resend round trip.
- `ack(tick)` drops the frames the server confirmed; `unacked()` is bounded by `max_unacked` (64).

## The server: `InputBuffer<T>` (one per client)

- `receive(&packet)` checks each frame and returns a `ReceiveReport` with the counts: `accepted`, `duplicates` (the redundancy
  at work), `conflicting` (a **different** input for a tick that already has one: the first stays, so a client cannot rewrite
  its past), `too_old` (the tick has been simulated), `too_far_ahead` (beyond `max_ahead_ticks`, default 30: bounds the memory)
  and `rate_limited`.
- **Rate limit:** a token bucket in simulation ticks: new frames cost one token, the bucket holds `burst_frames` (16) and refills
  `frames_per_tick` (2) per simulated tick. A player makes one frame per tick, so a real client is never limited (tested over
  2,000 ticks); a flood of 500 frames in one packet keeps 8. `violations()` counts the refused frames for a kick policy
  (T08-021).
- `take(tick)` gives the simulation its input: `Received(input)`, or `Repeated(input)` (the last real input, a held key, for up to
  `max_repeat_ticks` = 5 ticks), or `Missing` (use the neutral input).
- `ack_tick()` is the tick up to which the server has every frame **or has given up waiting** (`max_late_ticks` = 4 behind the
  simulation). Send it back to the client (in the snapshot) so it can `ack` and stop repeating older frames.

```rust
let mut sender = InputSender::new(InputConfig::default());
let mut buffer = InputBuffer::new(InputConfig::default(), server_tick_when_client_joined);
// client, every tick (it runs a few ticks ahead of the server):
sender.push(tick + lead, input);
send(sender.packet().unwrap());
// server, on a packet:   buffer.receive(&packet);
// server, every tick:    let input = buffer.take(server_tick);   reply.ack = buffer.ack_tick();
// client, on the reply:  if let Some(ack) = reply.ack { sender.ack(ack); }
```

## Measured (simulated link)

Over a link with 40 ms latency, 10 ms jitter and **20 % packet loss**, 3,000 ticks at 60 Hz, the client running 6 ticks ahead:
**20.0 %** of the ticks had no real input at the server with one frame per packet, **0.9 %** with three. (A simulation from
`ss2_net::netsim`, not a real network.)

## Not covered

- **The wire format** of the packet (T08-030) and the connection and session (T08-006): this module deals in Rust values.
- How far ahead the client runs (the input lead) and how the server adapts its buffering: that depends on the time sync
  (`docs/TIMESYNC.md`) and belongs with prediction (T08-009).
- Validation of the **content** of an input (an impossible aim angle or speed) is the game's (T08-021); this module validates the
  stream, not the meaning.
