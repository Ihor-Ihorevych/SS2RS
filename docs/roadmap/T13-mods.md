# T13 — Mods & Compatibility (`ss2-script`) — starts after M6, ships by M8

Facts: the install has `Sam2Renovation` and `InSamnity2` mod folders; each has a `.mod`/`.nfo`
descriptor. **`InSamnity2Game.dll` is a 6 MB native C++ game DLL** — a Rust engine cannot load it,
so a mod like that can only be supported by (a) reproducing its gameplay in Rust or (b) a new mod API.
Data-only mods (textures, models, `.ep` tweaks) should load directly through the VFS.

- [ ] **T13-001** · Mod loading semantics: `.mod`/`.nfo`/`DefaultMod.txt`, load order, content overlays on the VFS (T02-004), conflict reporting · `M` · `P1` · Deps: T01-040
- [ ] **T13-002** · **SPIKE/RFC**: mod gameplay strategy — scripting runtime (Lua/Rhai/WASM via an interpreter) vs Rust plugin crates vs "port each mod by hand"; sandboxing; multiplayer implications · `M` · `P1`
- [ ] **T13-003** · Data-only mod support verified: `Sam2Renovation` loads and plays; diff report of what it overrides · `M` · `P1` · Deps: T13-001
- [ ] **T13-004** · Scripting runtime integration per T13-002: sandbox, resource limits, hot reload, error isolation · `L` · `P2`
- [ ] **T13-005** · Mod API v0: register entity classes, subscribe to events, spawn/damage/query, UI hooks, replication attributes; stability policy · `XL` → split · `P2` · Deps: T09 stable entity API
- [ ] **T13-006** · Mod-aware netcode: mod list/hash in handshake (T08-006), mismatch handling, deterministic load order across peers · `M` · `P2`
- [ ] **T13-007** · Mod browser/loader UI (enable/disable, order, per-profile) · `M` · `P2` · Deps: T10-001
- [ ] **T13-008** · Port `InSamnity2` gameplay (classes inventoried from its DLL via T01-040/T01-026-style specs) onto Mod API or core · `XL` · `P2`
- [ ] **T13-009** · Mod validation tool: lints for missing assets, bad references, version mismatch (`ss2-doctor` extension) · `M` · `P2` · 🟢
- [ ] **T13-010** · Modder docs: file formats (from `ss2-format-*`), Mod API reference, example mod · `M` · `P2` · 🟢
- [x] **T13-011** · Policy: what mod content may be redistributed (no Croteam assets), mod licensing guidance · `S` · `P2` · 🟢
