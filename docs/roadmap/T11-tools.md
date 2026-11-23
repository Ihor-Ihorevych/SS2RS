# T11 — Tools (`tools/*`)

Tools multiply every other track: **`meta-dump` (T11-002) is the single most valuable early tool** — it
turns opaque `CTSEMETA` files into readable text so RE and loader work can move in parallel.

- [x] **T11-001** · `gro-tool`: `list`, `extract`, `cat`, `find` (glob/regex), `stats`, `diff` between archive sets; honors mount order · `M` · `P0` · 🟢 · Deps: T02-001
- [ ] **T11-002** · `meta-dump`: print any CTSEMETA file as annotated text/JSON/RON (header, chunks, types, refs, raw spans with offsets); `--unknown` mode highlights undecoded regions · `M` · `P0` · Deps: T02-006 (a hex-only version can precede the full reader)
- [ ] **T11-003** · `meta-diff` / `meta-survey`: compare files of one type to infer layout (constant vs variable bytes), histogram of chunk tags across the install · `M` · `P0` · Deps: T11-002
- [ ] **T11-004** · Asset viewer (desktop, `egui`): VFS tree, search, previews for textures (mips/channels), models (turntable, LOD, skeleton, material slots), animations (scrub), sounds, particles, fonts · `XL` → split by asset type · `P1`
- [ ] **T11-005** · World viewer: free-fly through `.wld`, layer toggles (geometry, collision, lights, sectors, nav, entities), entity inspector · `L` · `P1` · Deps: T04-009
- [ ] **T11-006** · Shader viewer: D3D9 asm disassembly ↔ WGSL side-by-side, permutation explorer · `M` · `P2` · Deps: T05-002
- [ ] **T11-007** · Ghidra helper scripts (Java/Python, `tools/ghidra/`; **no proprietary content**): RTTI class/vtable dumper, string-xref reports, struct-layout export to prose templates · `L` · `P0` · 🟢
- [ ] **T11-008** · Original-game instrumentation harness (debugger/Frida-style, run locally): dump entity positions/velocities/health/tick to CSV for fixed scenarios · `L` · `P0`
- [ ] **T11-009** · Cooker CLI: convert an install into the cooked cache (PC profile now; mobile profile in Phase 2), texture transcode, mesh repack · `L` · `P1` · Deps: T02-011
- [x] **T11-010** · Replay/netsim CLIs: `tools/net-tool` (`docs/NET_TOOL.md`): `replay inspect/verify/dump` and `netsim presets/run` (a seeded one-way packet stream through a preset with overrides, a report of loss, duplicates, reordering and latency percentiles); **it cannot record or play back a session** (no simulation yet; T11-010b) · `M` · `P2`
- [ ] **T11-010b** · net-tool follow-ups: record and play back a session once the simulation and T08-025b exist, a two-way run with a round-trip report, game-shaped traffic (bursty snapshot streams from several clients), JSON output for the progress dashboard (T11-012), scenario files, the socket-level conditioner front-end (T08-017b) · `M` · `P2` · Deps: T11-010, T08-025b
- [x] **T11-011** · Tracy/puffin profiling integration and capture scripts · `S` · `P1` · 🟢
- [~] **T11-012** [INPRG @Ihor-Ihorevych/agent · branch `T11-012-roadmap-progress` · 2026-11-23] · Progress dashboard: auto-generated coverage page — file types loadable, % of levels loading, weapons/enemies implemented; fed by the data-gated runner · `M` · `P2`
- [x] **T11-013** · `ss2-doctor`: validates an install (completeness, version, patch level), prints actionable errors for end users · `S` · `P1` · 🟢
- [ ] **T11-014** · Texture/model exporters (PNG/glTF) for modders and spec reviewers · `M` · `P2`
- [x] **T11-015** · Golden-file update CLI (images/audio/sim traces) with review-diff output · `S` · `P1` · 🟢
- [ ] **T11-016** · Level lister/mapper: levels, entities-by-class counts, which classes are still unimplemented (drives T09-027) · `M` · `P1` · Deps: T02-020d
