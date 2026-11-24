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
- [x] **T13-009** · Mod validation tool: lints for missing assets, bad references, version mismatch (`ss2-doctor` extension): `ss2_vfs::check_mods` / `check_install_mods` and `ss2-doctor [FOLDER] --mod NAME | --mod-dir DIR | --all-mods` (`docs/MOD_VALIDATION.md`): 37 lints with stable codes MOD001 to MOD092 (descriptors, `REQUIRES_PATCH` and update-pack version checks, native game module, `.nfo` references that resolve nowhere or are unsafe, file-name problems, override summary and critical overrides, conflicts between mods in a load order), deterministic ASCII report and exit code, no panic on damaged input; run on the real `Sam2Renovation` and `InSamnity2` (0 errors, no false positives; found one dangling thumbnail in `InSamnity2`). **Not done (T13-009b):** references inside the binary formats (`.ep`, `.rsc`, `.wld`, `.mdl`...), case checks against the stored name, a real version source (the original formats have none; `REQUIRES_PATCH` is our own optional key), loose files in the mod folder, references between mods, any machine-readable output · `M` · `P2` · 🟢
- [ ] **T13-009b** · Mod validation, the rest: read the references inside `.ep`/`.rsc`/`.ctl`/`.wld`/`.mdl`/`.mat` through their format crates (`ss2-format-ep::referenced_files` and the T02 readers; needs an RFC for the cross-crate API) and report the ones that resolve nowhere, add the mod's loose files to the checks, resolve references into mods that are required together, decide the version source with the mod authors (`REQUIRES_PATCH`, T13-001), JSON output for tools and the mod browser (T13-007), run the checks on Linux and macOS · `M` · `P2` · Deps: T13-009, T13-001
- [~] **T13-010** [INPRG @Ihor-Ihorevych/agent · branch `T13-010-modder-docs` · 2026-11-24] · Modder docs: file formats (from `ss2-format-*`), Mod API reference, example mod · `M` · `P2` · 🟢
- [x] **T13-011** · Policy: what mod content may be redistributed (no Croteam assets), mod licensing guidance · `S` · `P2` · 🟢
