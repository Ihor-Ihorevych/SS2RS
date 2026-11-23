# T02 — Data Layer: VFS, CTSEMETA, Asset Loaders

Crates: `ss2-vfs`, `ss2-meta`, `ss2-assets`, `ss2-format-{tex,mesh,skeleton,anim,model,material,world,params,pfx,font,sound,...}`.
Rules: pure Rust, no GPU/window deps, `#![forbid(unsafe_code)]` unless justified, zero panics on
malformed input, borrowed/zero-copy where possible, every loader has a synthetic-fixture unit test
**and** a data-gated "load everything" test.

## Virtual file system (does **not** need RE — ZIP is standard) — Wave 0
- [x] **T02-001** · `ss2-vfs`: open `.gro` (ZIP) archives, central-directory index, stored+deflate reads, big-file safe (>2 GB offsets, ZIP64) · `M` · `P0` · Deps: T00-014
  Done: lists all 12,288 file entries across the 14 archives of a real install (10,933 distinct paths after overrides) in <1 s (data-gated).
- [x] **T02-002** · Mount stack & override semantics: `Patch_*` over `All_PC_*`, ordering by name/version, later wins, case-insensitive normalized paths (`/` and `\`), directory listing merge · `M` · `P0`
- [x] **T02-003** · Loose-directory mount (dev, mods, cooked cache) with same API · `S` · `P1` · 🟢
- [x] **T02-004** · `.mod`/`.nfo`/`DefaultMod.txt` descriptor parsing and mod mount ordering · `S` · `P1` · 🟢 · Deps: T01-040 — `ss2-vfs::mods` (`ModManifest`, `Nfo`, `selected_mod`, `discover_mods`, `mount_mod`)
- [ ] **T02-005** · IO backends behind a file-provider trait: memory-mapped (desktop) + buffered/streaming fallback, read-ahead for 1 GB+ worlds. The trait is what lets Phase 2 add Android content URIs/SAF without touching loaders · `M` · `P0`
- [ ] **T02-005b** · Archive scanning caches (index cache to avoid re-reading ZIP directories on each launch) · `S` · `P2`

## CTSEMETA container — Wave 1 (needs T01-004/005)
- [x] **T02-006** · `ss2-meta`: container reader (header, `$Version`, chunk iterator, string/ref tables) · `M` · `P0` · Deps: T01-004 — `ss2_meta::Container::parse` (versions 4–8; all 8,141 containers of the retail install parse)
  Done: reads every file in the install that starts with `CTSEMETA` without error; reports unknown chunk tags.
- [x] **T02-007** · `ss2-meta`: type system / reflection decoder into a generic `Value` tree (structs, arrays, enums, refs) · `L` · `P0` · Deps: T01-005, T02-006 — `ss2_meta::{TypeSystem, Value, Container::decode_objects}` (all 8,141 containers of the retail install decode: 989,763 objects, 231,643 extra records)
- [x] **T02-008** · `ss2-meta`: writer + round-trip tests (needed for fixtures, cooked cache, tools) · `M` · `P1` — `ss2_meta::write` (all 8,141 containers of the retail install, 1.39 GB, are rewritten byte for byte)
- [x] **T02-008b** · `serde`-style derive layer: typed structs from `Value` (`#[derive(MetaDeserialize)]`) · `L` · `P1` — `ss2_meta::FromValue`, `Decoded::typed`, new crate `ss2-meta-derive`
- [x] **T02-009** · `ss2-assets`: `AssetManager` (handles, refcount, async load, dependency graph from `RFIL`, error reporting) · `L` · `P0` · Deps: T00-014 — `AssetOptions` (worker pool, eviction), `Handle::wait/wait_all`, `LoadContext::load_any`, `dependency_graph`, `stats`, `failures`; decisions in RFC 0001
- [x] **T02-010** · Loader registry & extension → loader mapping; plugin-style registration per format crate · `S` · `P0`
- [ ] **T02-011** · Cooked cache: processed-asset store with content hash, invalidation on archive/version change, background cooking · `L` · `P1`
- [ ] **T02-011b** · Hot reload of loose files (dev mode) · `S` · `P2` · 🟢

## Asset format loaders (each: crate + spec ID + fuzz target + data-gated full-install test)
- [~] **T02-012** [INPRG @lxstyo · branch `T02-012-tex-cpu-image-mip` · 2026-11-23] · `.tex` → CPU image + mip chain + metadata · `M` · `P0` · Deps: T01-006
- [ ] **T02-013** · Texture transcoding: BCn upload path + software decode fallback, normal-map handling (ETC2/ASTC targets added in Phase 2) · `L` · `P1`
- [ ] **T02-014** · `.bmf` mesh loader (vertex streams, LODs, bounds) · `M` · `P0` · Deps: T01-007
- [~] **T02-015** [INPRG @lxstyo/agent · branch `T02-015-skl-skeleton-loader` · 2026-11-23] · `.skl` skeleton loader · `S` · `P0` · Deps: T01-009
- [ ] **T02-016** · `.ans` animation loader + sampling API (pose evaluation, events) · `L` · `P0` · Deps: T01-010
- [ ] **T02-017** · `.mdl` model loader (links mesh/skeleton/materials/LOD/collision) · `L` · `P0` · Deps: T01-008, T02-014/015
- [ ] **T02-018** · `.mtr` material preset loader → engine-neutral `MaterialDesc` · `M` · `P0` · Deps: T01-011
- [ ] **T02-019** · `.mat` physical material loader, `.scs` sound collection loader · `S` · `P1` · Deps: T01-013
- [ ] **T02-020** · `.wld` world loader — split: **a** geometry, **b** lighting/lightmaps, **c** visibility/sectors, **d** entity records, **e** navigation data · `XL` · `P0` · Deps: T01-012a-f (f = the world root, the others hang off it)
- [ ] **T02-021** · World streaming/lazy resource resolution (avoid loading 100+ MB at once; matters most on low-RAM PCs and later mobile) · `M` · `P1`
- [ ] **T02-022** · `.ep` entity params → typed `EntityParams` (inheritance merge) · `M` · `P0` · Deps: T01-014
- [ ] **T02-023** · `.rsc` databases and `.ctl` loaders · `S` · `P1`
- [ ] **T02-024** · `.pfx` particle loader; `.lfx`, `.tfx`, `.vfx` loaders · `M` · `P1`
- [ ] **T02-025** · `.fnt` font loader (+ glyph atlas data) · `S` · `P1`
- [ ] **T02-026** · Sound loaders: `.wav` (PCM/ADPCM?), `.ogg` (Vorbis, streaming) · `M` · `P1` · 🟢
- [ ] **T02-027** · Text/localization loaders (`.srt`?, `.dsc`, `.nfo`, string tables) · `S` · `P2`
- [ ] **T02-028** · `.preload` lists → prefetch plan · `S` · `P2` · 🟢
- [ ] **T02-029** · Config/script-ish text types (`.cfg .mac .scp …` per T01-019) · `M` · `P2`

## Quality
- [ ] **T02-030** · `cargo-fuzz` targets for every parser, corpus from synthetic fixtures, nightly CI run · `M` · `P1` · 🟢
- [ ] **T02-031** · "Load everything" smoke tool & test: open every file in an install with its loader, print coverage table, fail on unknown/errored · `M` · `P0`
- [x] **T02-032** · Loader benchmarks (MB/s) and memory profile on PC (Android device in Phase 2) · `S` · `P2`
  PC measurements and reproduction commands are in [Performance budgets](../PERF_BUDGETS.md#58-loader-throughput-and-working-set-t02-032).
- [ ] **T02-033** · Public docs & examples for every `ss2-format-*` crate; publishable standalone (valuable to modders) · `M` · `P2`
- [ ] **T02-034** · Version-tolerance: handle files from older/newer `$Version` strings and patch deltas, explicit error when unsupported · `M` · `P1`
