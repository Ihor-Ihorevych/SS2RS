# SS2RS — Serious Sam 2 engine rewrite in Rust (working title) — Design

Status: **DRAFT — awaiting approval**. Date: 2026-11-21.
Detailed task breakdown lives in [`docs/roadmap/`](../../roadmap/README.md).

## 1. Intent and why

Rewrite the engine that runs *Serious Sam 2* (Croteam, 2005, "Serious Engine 2") as an
open-source Rust project that loads the player's own, legally owned game data and plays the
game like the original does.

Why (in the owner's words): keep the game running on **modern systems (PC first)**, and later
**port it to other platforms (Android first among them)**. Original netcode/sync is considered poor, so
**co-op is a first-class goal and netcode is designed fresh, not replicated**.

## 2. Goals

- G1. Run the original SS2 content (levels, models, textures, sounds, effects) on **PC first**
  (Windows, then Linux and macOS). **Android and other platforms are a later port**, started only after
  the PC campaign works (milestones M9+).
- G2. Behavior parity with the original for single-player gameplay: movement, weapons, enemies,
  level logic, feel. "Replicate behavior, not code."
- G3. **Co-op first**: server-authoritative simulation, client prediction, join-in-progress,
  good behavior with hordes of enemies. Single-player = a local server with one client.
- G4. Many contributors can work in parallel from week one (clear crate boundaries, written
  contracts, spec-then-implement workflow).
- G5. Open source, permissive license, clean provenance.
- G6. Support the shipped mods (Sam2Renovation, InSamnity2) eventually.

Order of delivery (agreed): engine first (see data, render, walk) → co-op foundation →
gameplay → full campaign → polish/mods (all on PC) → **then** Android and other platforms.

## 3. Non-goals

- Compatibility with original multiplayer protocol, servers or clients.
- The level/asset **editor** (Serious Editor 2). Read-only support for data.
- Shipping any Croteam assets, binaries or logos. Players supply their own install.
- Bit-exact reproduction of original physics/AI. Matching feel and numbers is enough.
- VR, consoles (for now), web (for now).
- Android/mobile work (touch UI, data import, mobile quality tier) **before the PC campaign is done**.
  Only the cheap guard rails in §8 apply until then.

## 4. Anti-goals (failure even if it "works")

- Code that is a transliteration of decompiler output or of SS1 source.
- PC code that hard-wires desktop assumptions (absolute paths, always-present mouse/window,
  unbounded RAM) so deeply that the later Android port becomes a rewrite.
- Netcode that quietly depends on lockstep/determinism across CPU architectures.
- A monolith where only one person can touch the renderer/gameplay at a time.

## 5. Constraints

- **Legal / provenance.** SS1 (Serious Engine 1) source is GPL → used for concepts only; no code
  copied. Ghidra analysis is allowed, but findings are written up as behavior/format *specs*
  (`docs/re/`), and implementers write Rust from the specs, not from decompiler output.
  Proprietary files and decompiled code are never committed. Details: roadmap T00-005/T00-006.
- Game data is never redistributed; engine needs a path to the user's install.
- (Later, Android phase) the data (≈3.5 GB, 1+ GB worlds, 1.3 GB video) must be importable from user storage.
- Primary working dir for this repo: `C:\Games\SS2Rewrite` (the retail install sits in
  `Serious Sam 2/` and must be git-ignored).

## 6. What recon found (facts, 2026-11-21)

- `.gro` archives are standard **ZIP** (12,288 file entries across 14 archives; 10,933 distinct paths after patch overrides). Patch archives layer
  on top of `All_PC_*`.
- By count: `.mdl` 3827, `.tex` 3281, `.wav` 1874, `.pfx` 612, `.ep` 394, `.ogg` 392, `.rsc` 391,
  `.dsc` 209, `.bmf` 123, `.wld` 107 (1.06 GB), `.mtr` 102, `.ans` 101, `.ogv` 99 (1.3 GB),
  `.ctl` 88, `.cb` 82, `.srt` 72, `.mat` 65, … plus ~25 rarer types.
- **Almost every asset begins with a `CTSEMETA` container** carrying
  `$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5` and chunk tags (`INFO`, `RFIL`,
  `IDNT`, `EXTY`, `INTY`, `DTTY`, …) — a reflection-style serializer. One parser unlocks most data.
- Gameplay data is data-driven (`.ep` entity params, `.rsc` databases, `.mat` physical materials)
  but **gameplay logic is native code in `Sam2Game.dll` (5.5 MB)**; engine in `Engine.dll`
  (3.5 MB) / `Core.dll`. Built with MSVC 7.1 (`msvcr71.dll`) → RTTI likely available to Ghidra.
- Renderer already has three backends (`GfxD3D`, `GfxOGL`, `GfxSWR`) and a D3D9 shader
  assembly dump (`GPU-Programs.asm`, 83 KB) → shaders are translatable/re-implementable.
- Videos are Ogg Theora (`.ogv`) plus one `.xmv` (Xbox, ignore). Audio is `.wav` + `.ogg`.
- `Help/SeriousEngineTools.chm` documents the tools/formats (Croteam copyright: summarize, don't
  republish).
- `InSamnity2Game.dll` (6 MB native) is a mod's gameplay code → cannot be loaded by a Rust engine;
  mods need a new mod API (see T13).

File-type meanings in the roadmap are **hypotheses from file names** until a spec in `docs/re/`
confirms them.

## 7. Approaches considered

| | Approach | Trade-off | Outcome |
|---|---|---|---|
| A | **Own engine from scratch** (`winit` + `wgpu` + own ECS/sim/net) | Full control, mirrors SS2's entity/material model; slowest to first pixels | **Chosen by owner** |
| B | Bevy as foundation | Fast start, free platform glue/replication; version churn, renderer friction | Rejected |
| C | Deterministic lockstep/rollback netcode | Cheap bandwidth for hordes; cross-arch float determinism, join-in-progress hard | Rejected (netcode: server-authoritative + prediction) |

Safeguard carried over from the discussion: format readers and gameplay logic live in
**engine-independent crates**, so any subsystem (renderer, platform layer) can be swapped.

## 8. Architecture overview (my calls, open to challenge)

Cargo workspace, `ss2-` prefix, tools under `tools/`:

```
ss2-core        ids, errors, logging, time, handles, math re-exports
ss2-vfs         .gro/ZIP + loose dirs + mod layering, case-insensitive paths
ss2-meta        CTSEMETA container + reflection reader
ss2-assets      AssetManager, loaders registry, cooked cache
ss2-format-*    tex, mesh(bmf), skeleton, anim, model, material, world, params(ep/rsc), pfx, font, sound
ss2-platform    window, input, lifecycle, Android/iOS glue, paths
ss2-render      wgpu renderer (frame graph, materials, world, particles, post)
ss2-shaderxlat  D3D9 asm → naga/WGSL (or handwritten replacements, per spike T05-006)
ss2-audio       mixer, 3D audio, music, video-audio sync
ss2-video       Theora cutscene playback
ss2-physics     collision, queries, character controller, rigid bodies
ss2-world       level instance, entities from .wld, navigation, triggers/mechanisms
ss2-sim         entity model, fixed-step loop, server/client split, replication
ss2-net         transport, sessions, snapshots, prediction, lag-comp
ss2-game        gameplay: player, weapons, enemies, pickups, level logic, rules
ss2-ui          UI framework, HUD, menus, fonts, localization
ss2-script      mod scripting (post-1.0 of campaign)
ss2-app         the executable (client/listen server), ss2-server (headless)
ss2-testkit     data-gated tests, fixtures, golden-image/audio helpers
```

Key decisions:
- **Server-authoritative sim**, fixed timestep, snapshot interpolation for remote entities,
  prediction + reconciliation for the local player, server-side lag compensation.
- **Renderer is fed through an "extract" interface** so sim never touches GPU types (threading,
  headless dedicated server).
- **Data-gated tests**: CI cannot hold game data; tests that need it use `SS2_DATA_DIR` and skip
  cleanly when absent. Self-hosted runner for contributors with a legal copy.
- **Cooked cache**: first run converts installed data into engine-friendly formats (faster loads,
  GPU texture formats; also what makes the later Android port practical).
- **PC first, portability guard rails only** (cheap, always on): portable crates take paths and files
  through `ss2-platform` traits (no `std::env::current_dir` tricks), handle surface loss/suspend in the
  lifecycle contract, don't assume a mouse, and keep a non-blocking `cargo check` job for an Android
  target in CI. No Android build pipeline, touch UI or mobile quality tiers until M9.
- **Rust**: stable toolchain pinned, `unsafe` allowed only in audited modules, `sim-safe` coding
  rules for crates in the prediction path (no hash-order iteration, no platform-dependent math).
- **License**: Apache-2.0 (matches the repo's `LICENSE`; patent grant suits many contributors).
  Adding MIT as a dual license later is possible if GPLv2-only embedding ever matters.
  Contributions under DCO sign-off.

## 9. Milestones

M0 repo & contracts → M1 "cat the data" → M2 "window + textured model on PC" →
M3 "walk a level" → M4 "co-op walk (PC)" → M5 "shoot" → M6 "vertical slice (first level, co-op, PC)" →
M7 "campaign complete" → M8 "PC 1.0: mods, polish" → **M9 Android port** → M10 other platforms.
Exit criteria per milestone: `docs/roadmap/README.md`.

## 10. Risks

1. Reimplementing `Sam2Game.dll` behavior from analysis is the largest, least predictable cost.
2. `.wld` format complexity (lighting, visibility) could stall M3.
3. Shader parity (D3D9 era materials) vs. look; mitigated by spike T05-006.
4. Later Android port: 3.5 GB data import, memory, thermal limits, hordes of enemies (mitigated by
   the guard rails now and the cooked cache).
5. Legal: provenance discipline must be enforced from day one (T00-005/006).
6. Contributor coordination at scale: mitigated by contracts + CODEOWNERS + per-track roadmaps.

## 11. Left to the builders

Exact crate choices inside each spike (ECS, transport, rigid-body engine), tick rate, the
internal formats of the cooked cache, UI toolkit internals, and the ordering of weapons/enemies
within M5 are decided by the owning track via RFC (`docs/rfcs/`).
