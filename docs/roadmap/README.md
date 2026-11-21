# SS2RS Roadmap — Index & Rules of Engagement

Working title **SS2RS**: open-source Rust engine that plays *Serious Sam 2* from the player's own data.
Design: [`../superpowers/specs/2026-11-21-ss2rs-design.md`](../superpowers/specs/2026-11-21-ss2rs-design.md).

> **Honesty note.** File-type meanings (`.ep` = entity params, `.rsc` = resource database, …) are
> *hypotheses from file names and tiny header peeks* until a spec in `docs/re/` confirms them.
> Task lists will grow as the RE track (T01) produces specs. Sizes are rough.

## 1. Tracks (one file each → fewer merge conflicts)

| Track | File | Scope | Main crates |
|---|---|---|---|
| T00 | [T00-repo-infra.md](T00-repo-infra.md) | Repo, CI, governance, contracts (RFCs) | workspace, `ss2-core`, `ss2-testkit` |
| T01 | [T01-reverse-engineering.md](T01-reverse-engineering.md) | Format & behavior **specs** (no product code) | `docs/re/` |
| T02 | [T02-data-formats.md](T02-data-formats.md) | VFS, CTSEMETA, all asset loaders | `ss2-vfs`, `ss2-meta`, `ss2-assets`, `ss2-format-*` |
| T03 | [T03-platform.md](T03-platform.md) | Window, input, desktop platforms, config; **Android/iOS = Phase 2** | `ss2-platform`, `ss2-app` |
| T04 | [T04-renderer.md](T04-renderer.md) | wgpu renderer | `ss2-render` |
| T05 | [T05-shaders.md](T05-shaders.md) | D3D9 asm → WGSL / material shaders | `ss2-shaderxlat` |
| T06 | [T06-audio-video.md](T06-audio-video.md) | Audio, music, cutscene video | `ss2-audio`, `ss2-video` |
| T07 | [T07-world-physics.md](T07-world-physics.md) | Collision, movement, physics, nav, mechanisms | `ss2-physics`, `ss2-world` |
| T08 | [T08-sim-netcode.md](T08-sim-netcode.md) | Sim core + **co-op netcode** | `ss2-sim`, `ss2-net` |
| T09 | [T09-gameplay.md](T09-gameplay.md) | Player, weapons, enemies, level logic | `ss2-game` |
| T10 | [T10-ui.md](T10-ui.md) | UI, HUD, menus, fonts, localization | `ss2-ui` |
| T11 | [T11-tools.md](T11-tools.md) | gro-tool, meta-dump, viewers, Ghidra helpers | `tools/*` |
| T12 | [T12-qa-release.md](T12-qa-release.md) | Test strategy, perf, packaging, release | CI, `ss2-testkit` |
| T13 | [T13-mods.md](T13-mods.md) | Mod loading, scripting, Renovation/InSamnity2 | `ss2-script` |

## 2. Task format

```
- [ ] **T02-006** · CTSEMETA container reader · `M` · `P0` · crate `ss2-meta`
  Deps: T01-004 · Do: … · Done when: …
```

- **ID** `Txx-nnn`: stable; never renumber, append only.
- **Size**: `S` ≤1 day · `M` 2–5 days · `L` 1–2 weeks · `XL` = must be split before claiming.
- **Priority**: `P0` on the critical path of the current milestone · `P1` needed for the milestone ·
  `P2` needed before 1.0 · `P3` nice-to-have.
- 🟢 = good first task (self-contained, little domain knowledge).
- **`PHASE 2`** = Android/mobile/other-platform work; not scheduled until M8 (replaces a P-level).
- **Spike** (`SPIKE`) = time-boxed decision task; output is an RFC in `docs/rfcs/`, not product code.
- **Spec task** (`SPEC`) = RE output; lives in `docs/re/`.

## 3. Workflow rules (so 30+ people can commit at once)

1. **One owner per crate** (`CODEOWNERS`). Work inside your crate freely; touch another crate's
   internals only through a PR it owns approves. Cross-crate changes = RFC first.
2. **Contracts before code.** Public traits/APIs between crates are defined in T00-014…T00-018 and
   frozen as `v0` stubs. Consumers code against stubs/mocks; they never wait for the real thing.
3. **Spec → implement → verify.** RE people write specs (`docs/re/<topic>.md` with byte layouts,
   invariants, confidence, test vectors). Implementers write Rust *from the spec*, never from
   decompiler output. A spec marked `confidence: low` needs a second reviewer.
4. **Clean-room discipline** (T00-005): no copying of SS1 or decompiled code; no proprietary files
   in git; contributors declare provenance in PR template; DCO sign-off.
5. **Data-gated tests**: tests needing game data use `ss2_testkit::require_game_data!()`
   (`SS2_DATA_DIR`) and skip when absent. Everything else must run on plain CI with
   synthetic fixtures.
6. **Small PRs** (<400 lines when possible), one task ID per PR, title `T02-006: CTSEMETA reader`.
7. **No hidden global state**; every subsystem is constructible in tests without a window/GPU.
8. **Sim-safe crates** (`ss2-sim`, `ss2-physics`, `ss2-net`, movement in `ss2-game`): no
   `HashMap` iteration order dependence, no wall-clock, no thread-order dependence.

## 4. Definition of Done (every task)

- Code compiles on Win/Linux/macOS CI; the non-blocking Android `cargo check` job shows no *new* desktop-only assumptions in portable crates.
- `cargo fmt`, `clippy -D warnings`, `cargo deny` clean.
- Tests included (unit + data-gated where relevant); parsers also fuzz-targeted (T02-030).
- Public items documented; spec/RFC updated if behavior or contract changed.
- Task checkbox ticked in the track file in the same PR.

## 5. Milestones and exit criteria

| M | Name | Exit criteria (demonstrable) |
|---|---|---|
| **M0** | Repo & contracts | Workspace builds on 3 OSes + Android check; CI green; contract stubs v0 merged; clean-room guard live |
| **M1** | Cat the data | `gro-tool` lists/extracts; `meta-dump` dumps any CTSEMETA file; all `.tex`, `.mdl`, `.bmf`, `.skl`, `.ans`, `.mtr` parse without error across the full install; asset viewer shows a textured model (headless render OK) |
| **M2** | Window + textured model (PC) | One binary renders a textured, animated model in a window on **Windows** (Linux/macOS follow via CI builds); input works (kbd/mouse/gamepad); audio plays a sound |
| **M3** | Walk a level | A real `.wld` level renders; free-fly then collision-based walking with the original movement model; sky/lighting basically right; music plays |
| **M4** | Co-op walk (PC) | Dedicated or listen server; 2+ PC clients walk around the same level; prediction smooth at 100 ms/2 % loss in netsim; join-in-progress works |
| **M5** | Shoot | 2+ weapons and 2+ enemy types, projectiles, damage, death, pickups; fight in a single arena in co-op |
| **M6** | Vertical slice | First level playable start→finish in co-op on PC with HUD, menus, saves/checkpoints, cutscene |
| **M7** | Campaign complete | All campaign levels completable; all weapons/enemies/bosses; difficulty levels; saves; cutscenes/video |
| **M8** | PC 1.0 | Mods (Renovation, InSamnity2 ported), polish, PC perf budgets met, packaged PC releases (Win/Linux/macOS), docs |
| **M9** | **Android port** *(starts only after M8)* | Same campaign runs on an Android device: data import, touch controls, mobile quality tier, perf/memory budgets, signed APK; co-op cross-play with PC |
| **M10** | Other platforms | iOS, handheld Linux, etc. as decided |

**PC-first rule:** tasks marked `PHASE 2` (Android/mobile/other platforms) are *not scheduled* until M8.
Only the portability guard rails (design §8) apply before then.

## 6. Waves — what can start immediately (no one waits for anyone)

**Wave 0 — day 1, zero dependencies** (assign these first):
- T00: 001–013, 019, 020, 022, 023, 026 (tools list: everyone installs the same toolset first)
- T01: 001–005, 020 (survey), 021 (RTTI recovery), 023 (physics middleware ID), 038, 039
- T02: 001–005 (ZIP/VFS doesn't need RE), 030 (fuzz harness skeleton)
- T03: 001–003, 005, 012, 013, 014, 018, 019
- T04: 001–004, 017 (2D) · T05: 001 (asm parser) · T06: 001, 002, 013
- T07: 001, 006 (SPIKE) · T08: 001, 005 (SPIKES), 017 · T10: 001, 002 (framework without game data)
- T11: 001 (gro-tool), 007 (Ghidra helpers) · T12: 001, 002

**Wave 1 — after contracts (T00-014…018) and container spec (T01-004/005):**
T02-006+ loaders, T04 mesh/material, T06 SFX/music, T07 collision extraction, T08 sim loop, T11-002.

**Wave 2 — after M1:** renderer ↔ assets integration, world loading, character controller,
replication, gameplay framework.

**Wave 3 — after M3/M4:** weapons, enemies, UI screens, levels, mods.

## 7. Critical path (watch these)

`T01-004/005 (container spec) → T02-006/007 (meta reader) → T02-012/014/017 (tex/mesh/model) →
T04-005…007 (render models) → M1/M2` and in parallel
`T01-012 (world spec) → T02-020 (wld loader) → T04-009 (world render) + T07-002 (collision) → M3 →
T08-004/007/009 (server/client/replication/prediction) → M4 → T01-026…029 → T09 → M5/M6`.

The single biggest unknown is **T01-012 (.wld spec)** and the **gameplay reverse engineering
(T01-026…029)**: staff them with your best reverse engineers first.

## 8. Suggested staffing (30–40 people; scale down by merging tracks)

| Track | Min | Ideal | Profile |
|---|---|---|---|
| T00 repo/infra | 1 | 2 | build/CI engineer |
| T01 RE/specs | 3 | 6 | Ghidra, C++/MSVC RTTI, binary formats; **at least two never write product code** (clean room) |
| T02 data | 2 | 4 | parsers, nom/binrw, fuzzing |
| T03 platform | 1 | 2 | winit, input, packaging (+1–2 Android/NDK people from M8 on) |
| T04 renderer | 2 | 4 | wgpu, real-time graphics |
| T05 shaders | 1 | 2 | D3D9 asm, naga, HLSL/WGSL |
| T06 audio/video | 1 | 2 | cpal, codecs |
| T07 world/physics | 2 | 3 | collision, character controllers |
| T08 sim/net | 2 | 4 | netcode, prediction |
| T09 gameplay | 3 | 8 | gameplay programmers (weapons/enemies parallelize well) |
| T10 UI | 1 | 3 | UI/UX, text rendering |
| T11 tools | 1 | 2 | egui, scripting |
| T12 QA/release | 1 | 2 | CI, packaging, device lab |
| T13 mods | 0 | 2 | scripting runtimes (starts after M6) |
| Phase 2 Android port | 0 | 3 | Android NDK, mobile GPU perf, touch UX (starts after M8) |

## 9. Cross-track dependency summary

```
T00 ──contracts──► everyone
T01 specs ─► T02 loaders ─► T04 render / T06 audio / T07 collision / T09 gameplay
T03 platform ─► T04 (surface) , T06 (audio device), T10 (input)   [touch/Android = Phase 2]
T05 shaders ─► T04 materials
T07 movement ─► T08 prediction ─► T09 player
T08 sim/net ◄─► T09 gameplay (entity & replication contract)
T10 UI ◄─ T02 (fonts/rsc) , T03 (input)
T11 tools support everyone (meta-dump unblocks T01 and T02)
T13 mods ◄─ T09 stable entity API
```

## 10. How to claim work

1. Pick an unchecked task whose deps are ✅ (or whose deps have merged stubs/mocks).
2. Comment/assign on the matching GitHub issue (issues are generated from these files, T00-008).
3. Branch `T02-006-meta-reader`; PR title starts with the ID; tick the box in the same PR.
4. If a task is bigger than its size label, split it and append new IDs (never renumber).
