# Performance budgets and benchmarks

Status: **PROPOSED** (T12-004). Each number is one of:

- **PROPOSED**: chosen here, basis stated. Not binding.
- **MEASURED**: from a repository document that records a measurement (only §5.6).
- **TBD**: no basis in the repository yet; the task that will supply it is named.

The T00-019 draft had memory, load-time and frame-split numbers that no repository document supports.
They are removed and marked TBD. Budgets become binding when T12-004 measures them on the reference PCs
and the owners accept them (`docs/PC_1_0_CHECKLIST.md` §2).

## 1. Purpose and scope

- **Purpose:** define "fast enough" for SS2RS on PC, how it is measured, and when a change is a regression.
- **Scope:** PC first: Windows, then Linux and macOS (design spec §2, G1). Renderer target: "PC
  (Vulkan/DX12/Metal), 60 fps with hordes" (`docs/roadmap/T04-renderer.md`).
- **Mobile is not here.** Memory: T12-005. Perf and thermal: T12-019. Device tiers: T12-006. All PHASE 2, after M8.
- **Not here:** the tick rate (T01-022) and network bandwidth (T08-007).

## 2. Reference PC tiers (PROPOSED, by hardware class)

Classes, not products: the repository names none. T12-004 binds each tier to one physical machine and
records its CPU, GPU, driver, RAM and storage in the report.

| | Low | Mid | High |
|---|---|---|---|
| CPU | 4 cores, about 2017 class | 6 cores, about 2020 class | 8+ cores, recent |
| RAM | 8 GB | 16 GB | 32 GB |
| GPU | integrated or entry discrete, 2 GB class | discrete, 4 GB VRAM class | recent high-end discrete |
| Storage | SATA SSD | NVMe SSD | NVMe SSD |
| Target resolution | 1280 x 720 | 1920 x 1080 | 2560 x 1440 |
| Frame target | 30 fps floor | 60 fps | 144 Hz refresh |

Basis: 1280 x 720 is the menu golden size (`docs/TESTING.md` §3.4); 1920 x 1080 is the UI canvas (T10-030);
60 fps is the renderer target (T04); the Low floor is "still playable" (`docs/PC_1_0_CHECKLIST.md` §2).
The rest is PROPOSED. The original game is 2005-era; its minimum requirements are not in the repository (§8).
HDD is not a reference tier.

## 3. Frame and simulation budgets

### 3.1 Frame budgets

| Metric | Low | Mid | High | Status |
|---|---|---|---|---|
| Frame time at target | 33.3 ms | 16.7 ms | 6.9 ms | PROPOSED (1000 / target fps) |
| 1 % low (mean of slowest 1 % of frames) | at most 50 ms | at most 25 ms | at most 10 ms | PROPOSED (1.5 x target) |
| Hitch (frame over 2 x median) | none in 10 minutes | none | none | PROPOSED |
| Visible draw calls per frame (95th percentile) | 1,000 | 2,000 | 2,000 | PROPOSED (basis §4.2) |
| GPU time per frame | TBD | TBD | TBD | timestamps, T04-019 |
| CPU time per subsystem | TBD | TBD | TBD | spans, T04-019, T11-011 |
| Render extract, 10,000 renderables | no budget | no budget | no budget | MEASURED, §5.6 |

### 3.2 Simulation step

- Tick rate: **TBD**. The design spec leaves it to T01-022 (§11). The original's tick rate is not in
  `docs/re/` (`docs/re/help-digest.md` excludes it).
- Sim budget: a share of one tick, set after T01-022 (share TBD). Workloads already named: 300 AI and
  500 debris on a mid-range CPU (T07-017); a horde of hundreds (T09-017, T04-023).
- **Physics queries of the named workload (MEASURED once, section 5.7):** one tick of 300 sight rays, 300 ground probes, 100 projectile
  casts and 20 blasts over 800 moving colliders and a 131,000-triangle terrain took **about 5.8 ms** plus **0.8 ms** to rebuild the
  moving colliders, on one laptop that is not a reference tier. At 30 ticks per second (a tick is 33 ms) that is about 20 %. **No
  budget is set**: the tick rate and the share are T01-022, and the workload has no AI or gameplay in it yet.

## 4. Memory, load time, and what the original data implies

### 4.1 Memory and load time

| Metric | Low | Mid | High | Needs |
|---|---|---|---|---|
| Process memory in a level (high-water mark) | TBD | TBD | TBD | cooked cache T02-011, streaming T02-021 |
| GPU memory in a level (high-water mark) | TBD | TBD | TBD | resource manager T04-003 |
| Cold start to main menu | TBD | TBD | TBD | measurement |
| Warm level load (cache present) | TBD | TBD | TBD | measurement |
| First-run archive indexing and cooking | TBD | TBD | TBD | T02-011, cooker T11-009 |
| Loader throughput (MB/s) and memory | TBD | TBD | TBD | T02-032 |

### 4.2 What the original data implies

Only figures in the cited documents. "derived" means arithmetic on them.

- **Install:** 14 `.gro` archives, 3,012 MB on disk (derived from `docs/re/inventory.md`); the design spec says
  about 3.5 GB (§8). Largest groups: `.wld` 1,059.0 MB (107 files), `.tex` 543.7 MB (3,281), `.mdl` 229.6 MB
  (3,827). Video (`.ogv`, 1,310.8 MB) is streamed, not budgeted (`docs/re/inventory.md`).
- **Worlds:** 59 (`docs/re/wld-root.md`). The reader rules bound a world at 20,000 entities (`docs/re/wld-root.md`, rule 6); that is a bound, not a
  census. All worlds: 73,098 entities, about 1,240 per world (derived; `docs/re/wld-entities.md`).
- **Renderables:** 99,187 in all worlds, about 1,680 per world (derived; `docs/re/wld-visibility.md`).
  The campaign village `1_3_village` has 895 (`docs/re/wld-root.md`).
- **Models:** 25,383 model entities referencing 2,258 `.mdl` files (`docs/re/wld-geometry.md`); 3,517 models,
  7,351 surfaces, 2.85 million triangles (`docs/re/mdl.md`), about 2.1 surfaces per model (derived).
- **Lights:** 11,349 point and 89 directional over 59 worlds; static light is baked into 18,679 lightmap
  modifiers (`docs/re/wld-lighting.md`).
- **Textures:** width up to 4096, height up to 2048; mip-chain size formula in `docs/re/tex.md`.
- **Culling:** 362 vis boxes in 44 worlds; no portal or sector data; run-time use of groups is not stated
  (`docs/re/wld-visibility.md`).
- **Draw calls (derived, not measured):** 895 renderables at about 2 surfaces each could issue about 1,900
  draws if all are visible and unbatched. The Mid target of 2,000 therefore depends on batching and
  instancing (T04-007, T04-009, T04-023).

## 5. How performance is measured

### 5.1 What exists today

- **Micro-benchmarks** (criterion), `crates/ss2-bench`: `src/lib.rs` holds synthetic fixtures (no game data);
  `benches/` covers assets (VFS and loads), render extract and math, platform actions and lifecycle, the
  audio voice limit and physics queries. Run `cargo bench -p ss2-bench`. CI compiles them but does not time them (`docs/TESTING.md` §4).
- **`ss2-app`:** `--smoke-frames <N>` renders N frames and exits (`crates/ss2-app/src/cli.rs`); no frame-time output was found in its source;
  `--screenshot` writes a menu screen.
- **Missing:** scene runs, frame-time capture, GPU timing, memory counters, load-time logs. The only scene
  is a placeholder (T03-032).

### 5.2 Planned

- Scene runs and frame capture: T12-004. GPU timestamps, CPU spans, overlay: T04-019, T10-007. Frame profiling with puffin
  (`docs/PROFILING.md`, T11-011, optional). RenderDoc for GPU capture from M2 (same section). Net profiling
  T08-018; UI draw budget `docs/UI_BUDGET.md` (T10-023, built for the menu screens); frame pacing T03-013.
- Original-game reference numbers: T01-038 (frame times as numbers) with the harness T11-008 and T01-039.

### 5.3 Fixed benchmark scenes (PROPOSED)

The game data is not in the repository; each scene runs only on a machine with its own install.

| Id | Category | Scene | Name source |
|---|---|---|---|
| S1 | large outdoor, campaign | `Levels/01_MDigBo/1_3_Village/1_3_Village.wld` | `docs/re/wld-entities.md`, `docs/re/wld-root.md` |
| S2 | boss fight | `1_8_bossarena`, boss `models/enemies/boss_zumzum/boss_zumzum.mdl` | `docs/re/environment-levels.md`, `docs/re/destruction-models.md` |
| S3 | small arena (floor) | `Levels/Multiplayer/DM_DockingBay/DockingBay.wld` | `docs/re/wld-root.md` |
| S4 | dense interior | TBD, from a per-world census (§8) | none yet |
| S5 | horde | synthetic, hundreds of enemies and debris (not a level) | T07-017, T09-017, T04-023 |

Rules: fixed start, camera or input path stored as text, fixed seed, 60 s warm-up discarded, and a
10-minute run on Mid for hitches (`docs/PC_1_0_CHECKLIST.md` §2). Cold shader and pipeline compiles are
timed apart from warm runs (T04-007).

### 5.4 What is recorded (PROPOSED)

Per scene, tier and build: frame-time histogram (fixed buckets), p50, p95, p99, 1 % low; hitch count; draw
calls (max, p95); CPU per subsystem once spans exist; process and GPU memory high-water marks; cold and
warm load times; commit; OS; GPU and driver; resolution; settings (T03-014). Results are numbers, CSV or text
only, with no game files, game text or game screenshots (`docs/PC_1_0_CHECKLIST.md` §3). Storage location is
undecided; until T12-002 defines it, results go in the PR or issue as text.

### 5.5 Determinism and noise

- Micro-benchmarks: `black_box` inputs and results; compare on one machine against a saved baseline
  (`--save-baseline`, `--baseline`); re-run before trusting a change under about 5 %. Close other programs.
- Scene runs: fixed seed and input path; three runs per scene; median of medians; repeat the set if
  run-to-run spread exceeds 5 % (PROPOSED). Vsync off (T03-013).
- Movement code that feeds prediction follows the sim-safe rules (`docs/CODING_STANDARDS.md`).

### 5.6 Micro-benchmark baseline (MEASURED)

T00-019 author's run: release profile, criterion 1 s warm-up and 2 s measurement, Intel Core Ultra 5 225U
laptop, 15 GB RAM, Windows 11. **Not re-run for this document.** Order of magnitude only.

| Benchmark | Time |
|---|---|
| `assets/load_cached_100` | 57.1 us |
| `render_extract/reset_and_fill/10000` | 147 us |
| `render_extract/validate/10000` | 674 us |
| `render_extract/submit_null/10000` | 715 us |
| `platform/action_query_frame` | 155 ns |

Finding: `RenderWorld::validate` costs about 5x more per item at 10,000 than at 1,000 (a `BTreeSet`
duplicate check). At 2,000 renderables it is about 0.1 ms, so no change now.

### 5.7 Physics query baseline (MEASURED, T07-017)

`cargo bench -p ss2-bench --bench physics -- --warm-up-time 1 --measurement-time 2`, release profile, 12th Gen Intel Core i5-12450HX (8
cores, 12 threads), 23.7 GB RAM, Windows 11, one run by the author of T07-017. A laptop, not a reference tier; the numbers are the
**middle value** of criterion's interval and are an order of magnitude, not a budget. Synthetic scenes (no game data): boxes scattered
over a 200 by 200 arena, a hilly terrain of two triangles per cell, capsules and spheres as moving colliders.

| Benchmark | Time | Per item |
|---|---|---|
| `physics_bvh/build/1000` | 0.76 ms | 0.76 us per box |
| `physics_bvh/build/10000` | 19.3 ms | 1.9 us per box |
| `physics_bvh/build/100000` | 303 ms | 3.0 us per box |
| `physics_bvh/nearest_hit_256_rays/1000` | 1.90 ms | 7.4 us per ray |
| `physics_bvh/nearest_hit_256_rays/10000` | 2.31 ms | 9.0 us per ray |
| `physics_bvh/nearest_hit_256_rays/100000` | 1.64 ms | 6.4 us per ray |
| `physics_mesh/raycast_256/8192` (triangles) | 1.15 ms | 4.5 us per ray |
| `physics_mesh/raycast_256/131072` | 1.02 ms | 4.0 us per ray |
| `physics_mesh/sphere_sweep_256/8192` | 1.91 ms | 7.5 us per sweep |
| `physics_mesh/sphere_sweep_256/131072` | 2.13 ms | 8.3 us per sweep |
| `physics_horde_tick/rebuild_800_colliders` | 0.78 ms | 1.0 us per collider |
| `physics_horde_tick/queries_300_sight_300_ground_100_projectiles_20_blasts` | 5.77 ms | 8 us per query (720 queries) |

The workload of T07-017 is the last two rows: 300 enemies (capsules) and 500 debris (small spheres), a 131,000-triangle terrain, every
enemy looking at the player (a ray) and probing the ground (a sphere cast down), 100 projectiles (sphere casts) and 20 explosions
(sphere overlaps). **One tick of it costs about 6.5 ms of physics queries**, single threaded.

Findings (and what they do not say):

- **A query costs 4 to 9 microseconds almost whatever the size of the scene** (a ray over 1,000 boxes costs about the same as over
  100,000, and over 8,000 triangles about the same as over 131,000). So the cost is not the depth of the tree: it is **per-query
  overhead** (the traversal stack is a fresh `Vec` per query, every box is padded and tested with the general slab routine, and the
  results go through closures). That is a suspicion from reading the code, **not a profile**; nobody has profiled it. A query that
  costs a few hundred nanoseconds should be possible.
- **The BVH build is not linear:** 0.76, 1.9 and 3.0 microseconds per box at 1,000, 10,000 and 100,000. The median split sorts at every level
  (`O(n log^2 n)`); a selection instead of a sort would remove a log. Rebuilding 800 colliders each tick (0.78 ms) is cheap enough now.
- **No mobile numbers and no thread scaling**: the task asks for a re-run on mobile CPUs in Phase 2, and queries are single-threaded here.
- The workload has **no gameplay** in it (no AI decisions, no animation, no character controller), so the real tick will cost more.

## 6. Regression policy

1. **Micro-benchmarks (in force):** a change making a tracked benchmark more than 10 % slower on one machine
   needs one sentence in the PR (`docs/CODING_STANDARDS.md`). Review enforces it; CI does not time.
2. **Scene runs (PROPOSED):** baseline is the median of the last five accepted runs of the scene on the same
   reference machine. Warn at +5 % p50, or +10 % in 1 % low or memory. Alert at +10 % p50, or any breach of a
   number in §3 or §4 once it is not TBD. Two alerts in a row open an issue.
3. **Load time (PROPOSED):** warn at +20 %.
4. **Who is told:** the subsystem owner in `.github/CODEOWNERS` and the performance owner (`docs/PC_1_0_CHECKLIST.md`
   §9; TBD). Until automation exists, whoever ran the scene files the issue and tags both.
5. **Filing:** `.github/ISSUE_TEMPLATE/bug.md` (no performance template exists). Title
   `Perf: <scene> <tier> <metric> +<n> % vs <baseline commit>`. Body: commit range, scene, machine record, run
   count, histogram as text or CSV, suspected subsystem. Labels per `docs/TRIAGE.md` §2 plus `track:T12`; a
   maintainer creates a `perf` label.
6. **Severity** (`docs/TRIAGE.md` §3): a load-time miss is S3 (the TRIAGE example). A Mid 60 fps breach on the
   1.0 scene set is S2 (PROPOSED), since it blocks M8. Memory exhaustion as a crash uses the crash template, S1.
7. **Triage:** `docs/TRIAGE.md` §4 and §7 (a regression needing design work becomes a task).
8. **Budgets move only by PR to this file, with measurements** (`docs/PC_1_0_CHECKLIST.md` §2).
9. **No automated alert yet.** GitHub Actions cannot start jobs (billing; `AGENTS.md` §1c); the scheduled
   workflows are audit, portability and claims (`docs/TESTING.md` §5). Runs happen on a local or contributor
   machine until the data-gated runner (T12-002) and then T12-004 automation exist.

## 7. Phase 2: mobile

No mobile numbers here: memory T12-005, thermal and sustained runs T12-019, device tiers T12-006, Android
frame pacing T03-022. All start after M8 (`docs/roadmap/README.md`).

## 8. Gaps and open questions

1. Tick rate and sim share: T01-022.
2. Original frame times, load times and memory are not in the repository (T01-038 would give frame times).
3. Per-world maxima for entities, renderables and draw calls: only totals exist. The census scripts in
   `tools/re/` print counts that are not committed. The dense-interior scene (S4) needs a numbers-only table.
4. Install size: design spec about 3.5 GB; the inventory archive table sums to 3,012 MB. One is wrong.
5. No physical machine is assigned to any tier; owner and location TBD.
6. GPU budgets and the frame-time split need T04-019 and T11-011.
7. Working-set and load budgets depend on T02-011 (cache) and T02-021 (streaming).
8. The performance owner (`docs/PC_1_0_CHECKLIST.md` §9) is not named.
9. Storage of results and alert delivery: T12-002 and T12-004.
10. Linux and macOS have no reference machines or scene runs. Windows is the primary platform
    (`docs/DEV_TOOLS.md` §4).
11. The original game's minimum hardware is not in the repository.
