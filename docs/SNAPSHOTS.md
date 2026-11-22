# Snapshot buffer and interpolation

`ss2_net::snapshots` (T08-008) shows **remote entities** smoothly from snapshots that arrive a few times per second. It is
presentation only: nothing in it feeds back into the simulation. Times are microseconds of server time (`docs/TIMESYNC.md`).

## The pieces

| Type | Job |
|---|---|
| `Interpolate` (trait) | what a state must offer: `lerp(&to, fraction_ppm)`, and optionally `extrapolate(previous, previous_to_self_us, ahead_us)` (`None` for a state that cannot be guessed forward) |
| `SnapshotBuffer<S>` | the recent snapshots of one entity, by tick (a repeated or out-of-order snapshot lands in the right place, the first copy of a tick stays); bounded by `capacity` (64) |
| `Sample<S>` | what `sample(render_time)` returns: `Gone`, `Held`, `Interpolated`, `Extrapolated`, `Stale` |
| `InterpolationDelay` | picks the delay from the snapshot interval and the measured jitter: **rises at once, falls by at most a tenth per update** |
| `RenderClock` | `server_now - delay`, never going backwards when the delay changes |

## What `sample` does

- **Between two snapshots:** blends them (exactly on a snapshot, it is that snapshot).
- **Before the first snapshot:** holds the first state.
- **Teleport** (`SnapshotFlags { teleport: true }` on the snapshot the entity jumped to): holds the old place until the new
  snapshot's time, then shows the new place, instead of streaking across the map. Normal blending resumes after it.
- **Past the newest snapshot:** extrapolates for up to `max_extrapolation_us` (100 ms) when the state can (`Extrapolated`),
  holds the last state when it cannot (`Held`), and after the limit reports `Stale` so the game can show a connection warning
  instead of a silently frozen world. A teleport snapshot is never extrapolated from.
- **Despawn** (`despawn(time)`): from that time `sample` returns `Gone`; the entity ends instead of freezing.

```rust
let mut clock = RenderClock::default();
let mut delay = InterpolationDelay::new(DelayConfig::default());
// when a snapshot arrives:
buffer.push(tick, server_time_us, state, SnapshotFlags::default());
// every rendered frame:
let wanted = delay.update(jitter_us, snapshot_interval_us);
let render_time = clock.render_time(server_now_us, wanted);
match buffer.sample(render_time) { Sample::Interpolated(s) | Sample::Extrapolated(s) | Sample::Held(s) => draw(s), Sample::Stale(s) => { draw(s); warn() }, Sample::Gone => {} }
```

## Measured (simulated link)

A 20 Hz snapshot stream over 30 ms latency, up to 60 ms of jitter and 2 % loss, drawn at 60 Hz for 30 s:

| Delay | Frames not interpolated | Mean delay |
|---|---|---|
| fixed 100 ms (two snapshots, the usual rule of thumb) | **38.0 %** | 100 ms |
| `InterpolationDelay` fed with four times the smoothed arrival-gap deviation | **0.0 %** | **264 ms** |

So the adaptive delay removes the stalls **at the price of a bigger delay**, and that link is a bad one. The default
`jitter_factor` (2) is conservative; tune it, and the jitter measure you feed in, against real traces before relying on it
(the test feeds a smoothed deviation of the time between arrivals times four, which is not a recommendation). A simulation, not
a real network.

## Not covered

- Nothing calls it yet: there is no snapshot protocol (T08-007) or transport (T08-005). The jitter measure comes from the caller
  (for example `ClockSync::jitter_us`, or the spread of snapshot arrival times).
- Smoothing of the **local player** is prediction and reconciliation (T08-009), not this module.
- A rise of the delay holds the picture for the difference (the render clock does not go back); a fall skips a little forward.
  Slowing the render clock down instead of holding it is a possible refinement.
