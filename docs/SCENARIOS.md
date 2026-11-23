# The scenario harness

`ss2_sim::scenario` (T09-029) runs the simulation **headless, tick by tick, in a test**: set up a world and its systems, script what happens when, record numbers while it runs, and ask questions of the result. It has no window and no clock. It knows nothing about weapons or enemies; the gameplay systems that T09 will build are added as ordinary systems and tested through it.

```rust
let mut s = Scenario::new(TickConfig::default(), world)?;      // the clock is paused: one step is exactly one tick
s.system(Phase::Damage, "burn", 0, |w, ctx| { /* ... */ })?;   // any system, in the usual phases
s.probe("health", move |w| health_of(w, enemy))?;              // record a number after every tick
s.at(30, |w| { /* spawn the second wave */ })?;                // script a change before tick 30's systems
let outcome = s.run_until(10 * 30, move |w| health_of(w, enemy) <= 0.0);
let ticks = outcome.reached().expect("the enemy dies within 10 seconds");
assert!(s.seconds_of(ticks) < 4.0);                            // time-to-kill
assert!(s.series("health").unwrap().total_drop() > 90.0);      // damage dealt
```

## What it gives

- **Exact stepping.** The scenario pauses the simulation's clock and asks for one tick per `step`, so a tick is always `1 / tick_hz` seconds (tested at 1, 7, 30, 60 and 1000 Hz). `ticks()`, `seconds()` and `seconds_of(n)` convert.
- **Probes** (`probe(name, |world| number)`): recorded once when added (index 0, the state before any tick) and after every tick. `series(name)` returns a `Series` with `first`, `last`, `peak`, `trough` (NaN skipped), **`total_drop`** (the sum of the drops, rises not cancelling them: damage taken from a health series), `total_rise` (healing, score), and **`first_tick_where`** (time-to-kill is `first_tick_where(|h| h <= 0.0)`).
- **`run_until(max_ticks, condition)`**: the condition is tested before the first tick (already true: `Reached(0)`, nothing runs) and after each; `Reached(n)` or `TimedOut(max)`. Reaching it exactly at the limit counts as reached. This is the shape of a **time-to-kill** or **path reached** check.
- **Scripted actions** (`at(tick, action)`): run once, before that tick's systems, in the order added; for a tick that has already run it is refused (`InThePast`). A player can be moved or a wave spawned at a chosen moment.
- **A lifecycle log** (for a `World`: `with_lifecycle_log()`): every spawn and despawn with the number of ticks run when it was collected (setup is tick 0), and `net_spawned()`.
- **`assert_deterministic(make, ticks, hash)`**: builds the scenario twice from scratch, runs both, and panics if the hashes of the final worlds differ (tested with a scenario that deliberately depends on something outside itself).

The world stays whatever type you like (`SimpleWorld`, a counter in a test).

## How it was tested

Seventeen unit tests and a doctest: exact stepping at five rates, phase order and tick numbers seen by systems, duplicate system names, a world changed between steps and a system switched off through the simulation, probes from the start and added late, the series statistics (drops and rises not cancelling, NaN, empty, a single sample), time-to-kill and damage on a burning `SimpleWorld` mob, `run_until` on the three outcomes and at the limit, scripted actions in order and once, in the past and for the current tick, a scripted spawn, the lifecycle log at the right ticks, and determinism both ways. **Twenty-one deliberate breakages: nineteen failed a test** (two of them, the NaN filters in `peak` and `trough`, only after I added a test for a series of nothing but NaN; one more did not apply the first time because rustfmt had reflowed the lines and failed a test when redone); **two are equivalent**: a count that only guarded a branch the next line repeated (I removed it), and `==` against `<=` in the script partition, which cannot differ because an action for a past tick is refused.

## Not covered (follow-up T09-029b)

- **It has only run against toy systems.** There is no player, weapon or enemy to put in a scenario yet, so the "scripted player and enemy setups" are the means (`at`, systems, probes), not actual setups; and nothing proves the harness fits the gameplay systems until they exist. T09-029b is to write the first real scenarios (a weapon's time-to-kill table, an enemy reaching a point) when they do.
- **Input is not scripted by name**: there is no recorded-input player (T07-019 plans a movement validation harness from recorded inputs); a scenario drives the world by systems and actions.
- **Probes are `f64` numbers only**: a position is three probes. No assertion library beyond `Series` and `Outcome` (plain `assert!` does the rest), no failure report showing the series, and no multi-client scenario (a server and two clients over the simulated network, T08-029).
