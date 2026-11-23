# Frame timing and the frame-time overlay

`ss2_render_api::frame_timing` (T04-019) is the **number-crunching and text** behind a frame-time overlay and GPU pass timings. It is device-free: no GPU, no clock. The caller supplies frame times and timestamp ticks.

The **CPU side** of profiling (scopes, a live viewer) already exists as `puffin` in `ss2-app` (T11-011, [PROFILING.md](PROFILING.md)). This module is what a renderer and an overlay add on top of it.

## Frame-time history

`FrameTimeHistory::new(capacity)` is a ring of the last N frame times (1 to 100,000; 240 is four seconds at 60 fps). `push(ms)` records a frame; a time that is not a finite, non-negative number is **ignored** so a stall on a window event cannot poison the numbers. `ordered()` gives the times oldest first (for a graph) and `clear()` forgets them (a level change: loading frames would skew everything).

`stats()` returns `FrameTimeStats`: count, average, min, max, the median, 95th and 99th percentiles (**nearest rank**: the p-th percentile of n values is the `ceil(n x p / 100)`-th smallest), `fps_avg = 1000 / avg`, `fps_1pct_low = 1000 / p99` (the frame rate of the slowest one percent: what a player feels as stutter, and what an average hides) and `hitches`: frames **longer than twice the median**.

`classify(ms, target_ms)` colours a time: `Good` up to the target, `Warn` up to 1.5 times it, `Bad` beyond, and `Bad` for any number that cannot be trusted (NaN, negative, a zero target): an overlay must not show green for garbage.

## GPU pass timings

A graphics API gives **timestamp ticks**, not milliseconds, and a period in nanoseconds per tick. `ticks_to_ms(begin, end, period_ns)`:

- takes the difference **with wrap-around** (the counter is 64 bits; `end < begin` is a wrap, not a negative time);
- returns `None` for a tick of 0 ("not written"), a period that is not positive and finite, and any duration over **1,000 ms** (`MAX_PLAUSIBLE_PASS_MS`), which is what a disjoint timer (a power-state change, a device reset) looks like. An unknown time is shown as `?`, never as a nonsense number.

`resolve_pass_timings(names, ticks, period)` takes `names.len() + 1` ticks (one between each pair of passes) and returns each pass's time; a count mismatch is refused, and one bad timestamp spoils only the two passes next to it.

## The overlay text

`overlay_lines(stats, gpu, target_ms)` returns the rows:

```
frame 10.0 ms avg  10.0 max  (100 fps, 1% low 100)
p50 10.0  p95 10.0  p99 10.0 ms   hitches 0
gpu 4.75 ms
  depth        1.25
  opaque       3.50
```

plus `over the 16.7 ms budget` when the average misses the target, `gpu: no timestamp queries` when the device lacks them, and `gpu ? ms (some timings unavailable)` when any pass is unknown (no passes is not a time of zero). The caller draws the rows and may colour each by `classify`.

## How it was tested

Sixteen unit tests and two doctests: percentiles on a known series (1 to 100) and on counts that are not a multiple of 100 (where rounding direction shows), a single frame, unsorted input, hitches at exactly twice the median, the ring evicting the oldest and ordering the rest, a cleared ring refilling in order, bad frame times, the tick conversion at two periods, the counter wrapping, unwritten and implausible timestamps, pass pairing, and the overlay text compared as exact strings. **Twenty-one deliberate breakages: sixteen failed a test at once and five survived my first tests**, all five real gaps (the exact 1.5x boundary, percentile rounding, an unwritten end tick when the period is tiny, a cleared ring that kept its write position, and a negative-zero frame time); I added a test for each and they now fail. No equivalent mutants remain.

## Not covered (follow-up T04-019b)

- **Nothing in the renderer takes timestamps**: the wgpu timestamp-query set, resolving and reading them back without stalling, and the device feature check belong to the device and frame graph tasks (T04-001, T04-002). The tick conversion is tested on invented numbers, **not on a real GPU's counter**, and the period source (`Queue::get_timestamp_period`) is not wired.
- **Nothing draws the overlay** (a `DrawList` of text rows and a graph from `ordered()`) and no key or console variable toggles it; the app does not feed it frame times.
- **Percentiles sort a copy** of the history each call (fine for a few hundred frames once per frame; not measured).
- **CPU spans stay in puffin**: there is no engine-side span tree, by design.
