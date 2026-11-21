# Performance budgets and benchmarks

Status: **proposed**. The targets below are starting points set from the design goals, not
measurements of the real game. They become binding when T12-004 measures them on the reference PCs
and the owners accept them. What *is* measured today is the micro-benchmark baseline in §5.

Mobile budgets are Phase 2 (after M8); see §7.

## 1. How we measure

Two kinds of measurement, never mixed:

| Kind | Tool | Where | Purpose |
|---|---|---|---|
| **Micro-benchmarks** | `criterion` in `crates/ss2-bench` | any dev machine, release profile | catch regressions in hot paths; compare two commits on the **same machine** |
| **Scene runs** | fixed scenes + frame-time capture (T12-004, needs the renderer and game data) | the reference PCs (§2) | check the frame/memory/load budgets below |

CI **compiles** the benchmarks (`clippy --all-targets`) but does **not** time them: shared runners are
too noisy for a regression gate. Timing comparisons are run by a person (or a future dedicated runner,
T12-004) on a quiet machine.

### Running

```
cargo bench -p ss2-bench                            # everything (about 2 minutes)
cargo bench -p ss2-bench --bench render_extract     # one file
cargo bench -p ss2-bench -- --test                  # smoke: run every benchmark once, no timing
```

Compare a change against `main` on the same machine:

```
git checkout main            && cargo bench -p ss2-bench -- --save-baseline main
git checkout my-branch       && cargo bench -p ss2-bench -- --baseline main
```

Criterion prints the change per benchmark and flags statistically significant ones. Close other
programs, plug in the laptop, and re-run before believing a result under about 5 %.

### Adding a benchmark

- Put shared synthetic scenes in `ss2-bench/src/lib.rs` (so every benchmark measures the same
  thing) and the benchmark itself in `ss2-bench/benches/<area>.rs`; register it under `[[bench]]`
  with `harness = false`.
- **No game data, no files on disk, no wall clock, no threads you do not control.** Deterministic
  inputs only.
- Wrap inputs *and* results in `std::hint::black_box`. A result in the picoseconds means the
  optimizer deleted the work (this happened to `lifecycle_cycle` in the first draft).
- Name groups `<area>` and cases `<what>[/<size>]`; keep each case under about 5 seconds.
- Benchmark code a frame runs many times, not code that runs once per level.

## 2. Reference machines (to be fixed by T12-004)

| Tier | Intent | Sketch (to be replaced by real hardware) |
|---|---|---|
| **Low** | "still playable" floor | 4-core CPU from about 2017, integrated GPU or entry GPU, 8 GB RAM, SATA SSD |
| **Mid** | the target experience | 6-core CPU from about 2020, GPU with about 4 GB VRAM, 16 GB RAM, NVMe |
| **High** | headroom and high refresh | recent 8+ core CPU, recent GPU, 32 GB RAM, NVMe |

The original game targets 2005-era hardware, so even **Low** is far above its minimum; the budgets
are about not wasting that headroom on a modern engine, not about squeezing.

## 3. Frame budgets (proposed)

Resolution 1920 x 1080 unless noted; Android and other mobile tiers are Phase 2.

| | Low | Mid | High |
|---|---|---|---|
| Target frame rate | 30 fps floor, no drops below | 60 fps | 144 fps at 1440p |
| Frame time (p50) | 33.3 ms | 16.7 ms | 6.9 ms |
| Frame time (p99) | 40 ms | 20 ms | 9 ms |
| Hitches (> 2 x median frame) | none in a 10-minute run | none | none |

Split of the **Mid** (16.7 ms) CPU main-thread budget, as a starting allocation for the owners to
challenge:

| Subsystem | Budget | Note |
|---|---|---|
| Simulation tick (gameplay, AI, physics) | 4.0 ms | worst case with the horde scene (hundreds of enemies, T04-023) |
| Render extract (`RenderWorld` build + validate + submit) | 0.5 ms | 2,000 renderables; see §5 for the measured cost |
| Render CPU (culling, batching, command recording) | 4.0 ms | can move to a render thread |
| UI / HUD | 0.5 ms | |
| Audio (control side: voices, listener, bus) | 0.3 ms | the mixer thread is separate |
| Net (serialize/deserialize snapshots) | 0.5 ms | |
| Slack | about 6.9 ms | thermal, OS, spikes; do not allocate it |

GPU time for **Mid**: 12 ms per frame (leaves margin for the same 16.7 ms frame). Draw calls: aim for
under 2,000 visible per frame through batching and instancing; this is a budget for the batching work
(T04-007, T04-023), not a hard limit.

## 4. Memory and load time (proposed)

| Metric | Low | Mid | High |
|---|---|---|---|
| Process memory in a level (working set) | 3 GB | 4 GB | 6 GB |
| GPU memory in a level | 2 GB | 3 GB | 6 GB |
| Cold start to main menu | 8 s | 5 s | 3 s |
| Level load (from menu, warm cooked cache) | 25 s | 15 s | 8 s |
| First-ever level load (cooks assets, T02-011) | no limit, but shows progress | | |

The install is about 3.5 GB, with 1 GB+ worlds, which is why asset streaming and the cooked cache
exist. Working-set budgets include the asset cache, so they are the main input for T02-011 sizing.

## 5. Micro-benchmark baseline (measured)

Release profile, criterion with a 1 s warm-up and 2 s measurement per case (shorter than the
default, so expect a few percent of noise). Machine: Intel Core Ultra 5 225U laptop CPU, 15 GB RAM,
Windows 11. **Indicative only**: use it to see order of magnitude and scaling, and compare against
your own run of the same commit, never against these absolute numbers.

| Benchmark | Time | What it tells us |
|---|---|---|
| `vfs/path_normalize` | 340 ns | normalizing a typical game path; paid on every `load` and lookup |
| `vfs/stat` | 64 ns | in-memory VFS lookup |
| `vfs/read_4k` | 268 ns | read 4 KiB from memory |
| `vfs/list_100` | 24.6 us | `MemoryVfs` scans all keys: a **mock** limitation, not a budget for the real VFS |
| `assets/load_cold_100` | 118.8 us | 100 first-time loads (~1.2 us each, trivial loader) |
| `assets/load_cached_100` | 57.1 us | 100 cache hits (~570 ns each, mostly path normalization) |
| `render_extract/reset_and_fill/1000` | 13.0 us | per-tick extract of 1,000 renderables (reused allocation) |
| `render_extract/reset_and_fill/10000` | 147 us | scales linearly |
| `render_extract/validate/1000` | 52.6 us | `RenderWorld::validate` on submit |
| `render_extract/validate/10000` | 674 us | **about 5x the per-item cost of 1,000**: see findings |
| `render_extract/submit_null/10000` | 715 us | validate + swap; extract of 10,000 is about 4 % of a 60 fps frame |
| `render_math/transform_interpolate` | 19 ns | per object per frame |
| `render_math/camera_view_projection` | 33 ns | once per frame |
| `platform/action_map_lookup` | 11 ns | |
| `platform/action_query_frame` | 155 ns | 12 actions x (pressed + just_pressed) |
| `platform/lifecycle_cycle` | 3.6 ns | five lifecycle events |
| `audio/play_256_with_32_voice_limit` | 63-68 us | 256 plays with constant culling (~260 ns each) |
| `audio/update_32_voices` | 310 ns | moving 32 emitters |

### Findings from the baseline

1. **`RenderWorld::validate` grows faster than linearly** (about 50 ns per renderable at 1,000,
   67 ns at 10,000) because duplicate-id detection uses a `BTreeSet`. At the Mid allocation of 2,000
   renderables it costs about 0.1 ms, well inside 0.5 ms, so **no change now**; revisit if extracts
   routinely exceed about 10,000 or the renderer starts calling it on every frame instead of every
   tick.
2. **Path normalization dominates a cached asset load** (340 ns of about 570 ns). If asset lookups
   become hot (per-frame resolve by path), cache `VfsPath` in the caller or key the cache by an
   interned id. Today lookups are per spawn, not per frame, so this is fine.
3. The first draft of `lifecycle_cycle` measured 326 ps because the inputs were constants: the
   optimizer removed the work. The rule in §1 ("`black_box` inputs and results") exists because of it.

## 6. Regression policy

- A change that makes a tracked micro-benchmark **more than 10 % slower** on the same machine needs a
  sentence in the PR explaining why it is worth it (or a fix).
- A change that breaches a frame, memory or load budget in §3/§4 once those are accepted blocks the
  milestone it affects, not the PR that discovers it: open an issue against the owner of the
  subsystem from §3's table.
- Budgets move only by PR to this file, with measurements, reviewed by the owners of the subsystems
  named in the table.

## 7. Phase 2: mobile

Mobile memory budgets (T12-005), mobile perf and thermal protocol (T12-019), the 4 GB-class memory
cap, texture streaming limits and the sustained 20-minute thermal runs are **not scheduled until M8**
and have no numbers here on purpose. The PC micro-benchmarks already run on any target that can run
`cargo bench`, so a first Android data point can be added without new infrastructure.
