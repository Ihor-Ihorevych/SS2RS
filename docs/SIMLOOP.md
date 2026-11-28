# The simulation tick loop

`ss2-sim` (T08-003) has the loop that turns real time into simulation ticks and the schedule of systems that run in each tick. It is
sim-safe: no clock is read (the caller passes the microseconds that passed), no hash map, and the ordering depends only on what was
registered.

## `TickLoop`: real time in, ticks out

```rust
let mut sim = Simulation::new(TickConfig::default(), world)?;
sim.schedule_mut().add(Phase::Move, "walk", 0, |world, ctx| { /* ctx.tick, ctx.dt_seconds() */ })?;

// every frame, with the real time since the last frame:
let frame = sim.update(frame_micros);       // runs the due ticks
draw(sim.world(), frame.alpha);             // alpha in [0, 1): how far into the next tick, for interpolation
```

- **Time inside the simulation is the tick counter.** Tick `n` is at `n / tick_hz` seconds, always (`sim_micros()`).
- **Exact.** The conversion is integer arithmetic in units of microsecond times tick rate times time scale, so however the real time is
  cut into frames the total ticks are `floor(time * tick_hz / 1e6)`. Tests check one second at 20, 30, 60, 64, 100 and 128 Hz with frames
  from 1 ms to 250 ms, and 200 random cuts of 5 to 15 seconds. 30 ticks per second really is 30, at 7 fps and at 1000 fps.
- **Pause** stops the ticks and **does not save up** the time that passes, so resuming does not cause a burst. **`step_once()`** asks for
  one tick while paused (requests add up; it is ignored when running, and resuming forgets an unused request).
- **Time scale** in thousandths (`set_time_scale_permille(500)` is half speed, 2000 double, 0 stops time without pausing), limited to
  8000. A change applies from then on and keeps the leftover time. In a network game only the server may change it.
- **No spiral of death.** A frame longer than `max_frame_micros` (default 100 ms, as the original drops time beyond 100 ms: a breakpoint, a window drag) counts as that long, and one
  call returns at most `max_ticks_per_frame` (default 8) ticks. What is given up is reported in `FrameTicks::dropped_ticks`, and the next
  frame is normal, not paying it back.
- `reset_to_tick(n)` jumps the counter (loading a save, joining a game in progress) and forgets the leftover.
- `tick_hz` is 1 to 1000. **The default is 60** (`DEFAULT_TICK_HZ`, the owner's decision T01-089). The original has no fixed tick
  (`docs/re/main-loop.md`: a variable step of 1 to 100 ms, physics sub-steps of at most 10 ms, time beyond 100 ms dropped); a fixed
  60 Hz keeps co-op and replays deterministic. Physics runs `PHYSICS_SUBSTEPS_PER_TICK` = 2 sub-steps per tick, 120 Hz, under the
  original's 10 ms cap (the walker's 1/120 s step). Nothing else in the loop depends on the value.

## `Schedule`: the order of systems

Systems are named closures `FnMut(&mut World, &TickContext)`, each in a **phase**; phases run in this order every tick:

`Input` → `Ai` → `Move` → `Physics` → `Damage` → `Cleanup` → `Replicate`

Within a phase a lower `order` number runs first, then the system added first. Registration order across phases does not matter. Names are
unique; `set_enabled` switches one off for debugging, `remove` takes it out, `runs` counts how often it ran, and `order()` lists the
running order. `TickContext` carries the tick number (the first tick is 0) and the constant step (`dt_seconds() = 1 / tick_hz`).

`Simulation<W>` is a `TickLoop`, a `Schedule<W>` and the world: `update(real_micros)` runs every system once per due tick with
consecutive tick numbers. Ticks that were given up in a stall do not run their systems, and the clock counts only ticks that ran.

## Tests

Twenty unit tests, each written first and seen to fail. Besides the ones named above: the same total time cut as 120 frames of 16.666 ms,
as three long frames and as 1 ms slivers gives the **same world hash**; phases run in order whatever the registration order; ties go by
the order number and then by registration. Ten deliberate breakages (the leftover thrown away, no frame clamp, no tick cap, the order
number ignored, wrong tick numbers, a disabled system still running, the step guard removed, resume keeping a step) each fail a test; the
last two first **survived** and the step test was extended for them.

## Not covered

- **The main-loop timers and pause semantics** of the original (`main-loop.md`) beyond the tick rate, which is decided (60 Hz, T01-089).
- **Parallel systems.** Systems run one after another. A system graph with declared reads and writes is a later optimisation, and it
  must keep this deterministic order observable (RFC 0005 on the ECS decides what a "world" is).
- **Server and client halves** (`ServerSim`, `ClientSim`, loopback): T08-004. **Replication** of what the `Replicate` phase builds: T08-007.
- A **real-time driver** (reading the clock, sleeping, vsync): it lives in the app, which calls `update` with measured microseconds.
