# T01 — Reverse Engineering & Specifications

Output: **specs in `docs/re/`** — never product code. Template (T01-001): *summary · evidence (how
we know) · byte/field layout · invariants · open questions · confidence (low/med/high) · test
vectors (hex + expected values) · reviewer*. Implementers (T02, T07, T09…) code **from specs only**.

Tools: Ghidra (Engine.dll, Core.dll, Sam2Game.dll, GfxOGL.dll, GfxD3D.dll), hex editors, the
shipped `SeriousEngineTools.chm`, our own `meta-dump` (T11-002), instrumentation (T11-008).
Reminder: Ghidra projects and decompiler listings stay **out of git**; only prose + numbers.

## Setup
- [ ] **T01-001** · Spec template, naming scheme, review checklist; shared Ghidra conventions (naming dictionary, how to export notes without code) · `S` · `P0`
- [ ] **T01-002** · Digest `Help/SeriousEngineTools.chm` into a private notes index; produce *summaries* (not verbatim) of every format/tool page; list what it confirms · `M` · `P0`
- [~] **T01-003** [INPRG @Ihor-Ihorevych/agent · branch `re-ctsemeta` · 2026-11-21] · Full archive inventory (counts, dirs, patch overrides, per-type size) → `docs/re/inventory.md` (paths only, no content) + generated machine-readable list kept local · `S` · `P0`

## Container & data formats
- [ ] **T01-004** · SPEC `CTSEMETA` container: header, `$Version` string, chunk tags (`INFO RFIL IDNT EXTY INTY DTTY …`), sizes/endianness/alignment, compression?, string tables, versioning across 2.062 patch levels · `L` · `P0` · Deps: T01-001
- [ ] **T01-005** · SPEC CTSEMETA type system: how types/fields/enums/arrays/references/embedded resources are encoded; is it self-describing? how external references (`RFIL`) resolve · `L` · `P0` · Deps: T01-004
- [ ] **T01-006** · SPEC `.tex` textures: pixel formats, mips, cube/volume, flags, sRGB, sampler state, animation (if any) · `M` · `P0` · Deps: T01-005
- [ ] **T01-007** · SPEC `.bmf` binary meshes: vertex layouts, index formats, LOD, bounds, morph/tangents · `L` · `P0`
- [ ] **T01-008** · SPEC `.mdl` models: meshes, material bindings, LODs, attachments, collision/hit volumes, references to `.skl/.ans/.mtr` · `L` · `P0`
- [ ] **T01-009** · SPEC `.skl` skeletons: bone hierarchy, bind pose, bone flags · `M` · `P0`
- [ ] **T01-010** · SPEC `.ans` animation sets: tracks, keyframe/compression format, events/notifies, root motion, blending hints · `L` · `P0`
- [ ] **T01-011** · SPEC `.mtr` material presets ("ShaderPresets"): parameters, textures slots, shader module selection · `M` · `P0`
- [ ] **T01-012** · SPEC `.wld` world: structure of geometry, lighting data, visibility/sectors, entity records, terrain?, navigation data, embedded vs referenced resources · `XL` → split by discovery (e.g. T01-012a geometry, b lighting, c visibility, d entities, e nav) · `P0` · **highest risk**
- [ ] **T01-013** · SPEC `.mat` physical materials & `.scs` sound collections (impact/footstep mapping) · `M` · `P1`
- [ ] **T01-014** · SPEC `.ep` entity params: schema per entity class, inheritance/overrides, how classes are named · `L` · `P0`
- [ ] **T01-015** · SPEC `.rsc` resource databases (e.g. `HudParams.rsc`) and `.ctl` control/tool definitions · `M` · `P1`
- [ ] **T01-016** · SPEC `.pfx` particle effects: emitters, curves, forces, rendering modes, LOD/budget · `L` · `P1`
- [ ] **T01-017** · SPEC `.lfx` light effects, `.tfx` text effects, `.vfx` (video/visual fx?) · `M` · `P2`
- [ ] **T01-018** · SPEC `.fnt` fonts (+ `Content/Fonts`): glyph metrics, atlases, encodings/languages · `M` · `P1`
- [ ] **T01-019** · Identify unknown types: `.cb .srt .sfc .crt .mac .mch .scp .pss .sca .mas .ctr .preload .bin .dsc .nfo` → one-page spec each as discovered (`.dsc`/`.nfo` look like plain text) · `L` · `P1`
- [ ] **T01-020** · SPEC Shader system: `Bin/Shaders.module`, `GPU-Programs.asm`, `Sources/Shaders`; how materials pick programs, permutations, constants, vertex formats · `L` · `P0`

## Engine behavior (from `Engine.dll`/`Core.dll`)
- [ ] **T01-021** · Recover C++ class hierarchy & vtables via MSVC RTTI for all three game DLLs → `docs/re/classes.md` (names, inheritance, sizes) · `L` · `P0`
- [ ] **T01-022** · SPEC main loop & time: tick rate, fixed/variable step, interpolation, timers, pause semantics · `M` · `P0`
- [ ] **T01-023** · Identify physics/collision middleware from imports/strings; list features used (rigid bodies, ragdolls, destruction) · `S` · `P0`
- [ ] **T01-024** · SPEC player movement: speeds, acceleration, friction, air control, jump, gravity, step height, slope limit, crouch, swim, ladders — as **measured numbers + test vectors** · `L` · `P0` · Deps: T01-038/039
- [ ] **T01-025** · SPEC AI/navigation: perception, pathfinding (navmesh/waypoints), steering, group behavior, spawners · `L` · `P1`
- [ ] **T01-026** · SPEC entity class inventory of `Sam2Game.dll`: every class, its properties, events, relation to `.ep` · `XL` · `P0`
- [ ] **T01-027** · SPEC player rules: health/armor, inventory, ammo caps, pickups, powerups, death/respawn, Netricsa · `L` · `P1`
- [ ] **T01-028** · SPEC weapons — **one spec per weapon** (list from `.ep`/classes; `AutoShotgun` confirmed present): fire modes, rate, damage, spread, projectile physics, ammo use, animation events, effects · `L` each · `P1` · *parallelizable per weapon*
- [ ] **T01-029** · SPEC enemies — **one spec per enemy type**: stats, attacks, movement, AI states, spawn/death effects, damage modifiers · `L` each · `P1` · *parallelizable per enemy*
- [ ] **T01-030** · SPEC level logic: triggers, mechanisms (doors, elevators), scripted sequences, checkpoints, secrets, cutscene triggers · `L` · `P1`
- [ ] **T01-031** · SPEC what state original MP replicates (to design *our* replication; not protocol parity) · `M` · `P2`
- [ ] **T01-032** · SPEC save-game contents (our format will differ; informs what must persist) · `M` · `P2`
- [ ] **T01-033** · SPEC console variables/commands, profile/config files, default key binds · `M` · `P2`
- [ ] **T01-034** · SPEC audio semantics: 3D attenuation curves, priorities, music system, EAX/reverb zones · `M` · `P1`
- [ ] **T01-035** · SPEC cutscene/movie system: how in-engine cutscenes and `.ogv` are triggered/skipped · `M` · `P2`
- [ ] **T01-036** · SPEC GUI system (`EngineGui.dll`): widget model, layout, skin, input focus rules; menu flow from `Content/SeriousSam2/Menus` · `L` · `P1`
- [ ] **T01-037** · SPEC game modes, difficulty tables, scoring, co-op/MP rules as shipped · `M` · `P1`
- [ ] **T01-038** · Oracle capture: record reference gameplay (screen + frame times + known actions) for movement/weapon/enemy validation; store **numbers only** · `L` · `P0`
- [ ] **T01-039** · Instrumentation harness for the original game (debugger/Frida-style scripts reading positions, velocities, health, tick) → CSV oracle data · `L` · `P0` · Deps: T11-008
- [ ] **T01-040** · SPEC mod interface: how `.mod`/`.nfo`, game DLL replacement, content overlays work; what `Sam2Renovation` and `InSamnity2` change · `M` · `P2`
- [ ] **T01-041** · SPEC localization: languages shipped, string tables, subtitle sync · `M` · `P2`
- [ ] **T01-042** · SPEC camera & FOV rules, weapon view models, head bob/sway constants · `M` · `P1`
- [ ] **T01-043** · SPEC decals, gibs, blood, hit effects pipeline (what spawns when) · `M` · `P2`
- [ ] **T01-044** · SPEC water, fog, sky, post effects usage per level (feeds T04-012/015) · `M` · `P1`
- [ ] **T01-045** · SPEC lighting: baked vs dynamic, light types, shadow rules, ambient · `L` · `P0` · Deps: T01-012b
- [ ] **T01-046** · SPEC destruction system (`EditKit_Destruction` hint): breakable objects, debris · `M` · `P2`
- [ ] **T01-047** · SPEC vehicles/turrets/special mechanics if present · `M` · `P2`
- [ ] **T01-048** · SPEC boss fights (per boss) · `L` each · `P2`
- [ ] **T01-049** · Review & QA of specs: independent re-read, cross-check test vectors against `meta-dump` output · rolling · `P0`
