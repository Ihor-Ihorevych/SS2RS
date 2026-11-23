# T04 — Renderer (`ss2-render`, wgpu)

Design rules: sim never touches GPU types; the renderer consumes a **`RenderWorld` extracted each
frame** (T00-015) so rendering can run on its own thread and the dedicated server has no renderer.
Target: **PC (Vulkan/DX12/Metal)**, 60 fps with hordes. A **mobile tier** (Vulkan/GLES3 floor) is
Phase 2; until then keep quality tiers as plain settings and avoid designs that can't scale down
(e.g. mandatory compute, unbounded per-frame allocation).

## Foundation (Wave 0)
- [ ] **T04-001** · wgpu instance/adapter/device/surface init; backend selection, limits/features report, surface-lost/device-lost recovery (also what the later Android port needs) · `M` · `P0`
- [x] **T04-002** · Frame graph / pass scheduler skeleton (depth pre-pass, opaque, transparent, post, UI), resource aliasing hooks: `ss2_render_api::frame_graph` (`docs/FRAME_GRAPH.md`): stage-then-declaration ordering, read/write dependencies, culling of dead passes, transient lifetimes and a memory-sharing plan, a text dump; **device-free: no wgpu execution, no resource pool across frames, no sharing between kinds** (T04-002b) · `L` · `P0`
- [ ] **T04-002b** · Drive the compiled frame graph with wgpu: create the transient textures and buffers from the `AliasPlan`, a pool that keeps them across frames and rebuilds on resize, per-pass encoders and timestamp hooks (T04-019b), then the real passes as they land · `L` · `P0` · Deps: T04-002, T04-001
- [~] **T04-003** [INPRG @Ihor-Ihorevych/agent · branch `T04-003-gpu-resource-manager` · 2026-11-23] · GPU resource manager: buffers/textures, staging ring, upload budget per frame, streaming API for assets · `L` · `P0`
- [x] **T04-004** · Camera, projection (reverse-Z, infinite far; from the contract T00-015), frustum, culling primitives, jitter slot for AA: `ss2_render_api::culling` (`docs/CULLING.md`) · `M` · `P0`
- [ ] **T04-017** · 2D/UI renderer: batched quads, text glyph quads, scissor, 9-slice, render-to-texture (shared with T10) · `M` · `P0` · 🟢
- [x] **T04-018** · Debug draw (lines/AABB/spheres/text/labels), toggleable overlays: the GPU-free recorder `ss2_render_api::DebugDraw` (`docs/DEBUG_DRAW.md`); the wgpu line pass is T04-018b · `S` · `P1` · 🟢
- [ ] **T04-018b** · Debug draw GPU pass: draw `DebugDraw::lines()` as a line list (depth test on/off) in the debug-overlay stage of the frame graph, labels through the 2D renderer (`project_labels`), the `r_debug <category>` console command · `S` · `P1` · Deps: T04-002, T04-017
- [x] **T04-019** · GPU profiling via timestamp queries + CPU spans; frame time overlay: `ss2_render_api::frame_timing` (`docs/FRAME_TIMING.md`): frame-time history and percentiles, tick-to-millisecond conversion with wrap-around and plausibility checks, pass timings, the overlay text; CPU spans were already `puffin` (T11-011); **no wgpu timestamp queries, nothing draws the overlay, not tried on a real GPU** (T04-019b) · `S` · `P1`
- [ ] **T04-019b** · wgpu timestamp-query set per frame-graph pass (feature check, resolve and non-stalling readback, period from the queue), draw the overlay from `overlay_lines` and a graph from `ordered()`, a key or console variable to toggle it, feed it from the app loop, verify on a real GPU · `S` · `P1` · Deps: T04-019, T04-001, T04-002
- [ ] **T04-026** · `RenderWorld` extract implementation + double-buffering between sim and render thread · `L` · `P0` · Deps: T00-015

## Models & materials (Wave 1) — M1/M2
- [ ] **T04-005** · Static mesh path: vertex layouts from `.bmf`, index buffers, LOD select, per-draw uniforms · `M` · `P0` · Deps: T02-014
- [ ] **T04-006** · GPU skinning, bone palette upload, animation pose → buffer, attachments (weapon on hand) · `L` · `P0` · Deps: T02-015/016
- [ ] **T04-007** · Material system: `MaterialDesc` (from `.mtr`) → pipeline + bind groups; permutation cache; **on-disk pipeline cache** to avoid shader-compile hitches (esp. Android) · `L` · `P0` · Deps: T02-018, T05-*
- [ ] **T04-008** · Texture sampling: filtering, anisotropy, mip bias, sRGB/linear rules, normal-map decode, cubemaps · `M` · `P0`
- [x] **T04-016** · Transparency: sorting, blend modes (additive/alpha/premult), alpha-to-coverage option, soft particles: `ss2_render_api::transparency` (`docs/TRANSPARENCY.md`): blend equations, a deterministic draw sort (opaque by material, blended back to front with layers and bias), the alpha-to-coverage rule, the soft-particle fade and reverse-Z depth conversion; **no pipelines or passes use it yet, per-draw sorting only** (T04-016b) · `M` · `P1`
- [ ] **T04-016b** · Map `BlendFactors` to wgpu pipeline state, the transparent pass and the soft-particle depth read in the frame graph, call `sort_draws` from the renderer, measure the sort on thousands of particles · `M` · `P1` · Deps: T04-016, T04-002, T04-007
- [ ] **T04-023** · Instancing & batching for hordes (hundreds of animated enemies), animation texture/instanced skinning option · `L` · `P1`
- [x] **T04-024** · LOD & distance culling policy (model LODs, impostors for far enemies), quality-tier aware: `ss2_render_api::lod` (`docs/LOD.md`): selection by screen coverage with levels, an impostor, size and distance culling, quality bias and view distance, hysteresis, a per-thing tracker; **nothing feeds it yet (no mesh levels, no impostors, no renderer calls it), no cross-fade** (T04-024b) · `M` · `P1`
- [ ] **T04-024b** · Use the LOD policy in the mesh path (levels from `.bmf`, T04-005), impostor generation and drawing, cross-fade or dithered transitions, per-class distances and shadow-caster LOD, and a benchmark of selection for thousands of things · `M` · `P1` · Deps: T04-024, T04-005

## World & lighting (needs T01-012/045) — M3
- [ ] **T04-009** · World geometry renderer: static geometry chunks, sector/portal or PVS culling per `.wld` spec, material batching · `XL` → split (batches, culling, streaming) · `P0` · Deps: T02-020
- [ ] **T04-010** · Lighting: baked lightmaps (if used) + dynamic lights (point/spot), light culling/clustering, light budget per tier · `XL` → split · `P0` · Deps: T01-045
- [ ] **T04-011** · Dynamic shadows (cascades/cube/spot), filtering, bias, tier toggles · `L` · `P1`
- [ ] **T04-015** · Sky (box/dome/procedural?), fog (distance/height), water (reflection/refraction/flow), glass, heat haze · `L` · `P1`
- [ ] **T04-014** · Decals (bullet holes, blood, scorches), lifetime/pool, projection on static world · `M` · `P2`
- [ ] **T04-025** · Occlusion culling (HZB/software) optional, only if profiling shows need · `L` · `P3`

## Effects & post
- [ ] **T04-013** · Particle renderer (CPU sim → GPU draw first; GPU sim later), `.pfx` features, sorting, budgets · `L` · `P1` · Deps: T02-024
- [ ] **T04-012** · Post: HDR target, tonemap, bloom, color grading, vignette, motion blur?, DOF? (as used per T01-044), quality tiers · `L` · `P1`
- [ ] **T04-029** · Anti-aliasing: MSAA (mobile-friendly), FXAA, optional TAA · `M` · `P2`
- [ ] **T04-030** · Weapon viewmodel pass (separate FOV/depth range), muzzle flash, shell ejection · `M` · `P1`
- [ ] **T04-031** · Screen-space effects: damage flashes, underwater overlay, fades, letterbox for cutscenes · `S` · `P1` · 🟢

## Quality, tooling, mobile
- [x] **T04-020** · Quality presets (Ultra→Low), dynamic resolution scale, per-setting toggles exposed to settings: `ss2_render_api::quality` (`docs/QUALITY.md`): four presets, eleven named settings with strict parsing, a deterministic dynamic-resolution controller; **nothing in the renderer reads them yet, the preset values are unmeasured guesses, and there is no settings file or screen** (T04-020b) · `M` · `P1`
- [ ] **T04-020b** · Make the renderer honour `QualitySettings` (render scale first, then each pass as it lands), feed `DynamicResolution` from a real frame timer, pick a default tier from the GPU, tune the presets on real GPUs (a phone in Phase 2) · `M` · `P1` · Deps: T04-020, T04-002
- [x] **T04-021** · Golden-image test harness: `ss2_render::golden` (`docs/GOLDEN_IMAGES.md`): a two-limit tolerance, failure evidence (rendered and difference images), the update workflow (with T11-015), `SS2_FORCE_SOFTWARE_ADAPTER`; **no CI job with a software adapter yet** (T04-021b) · `M` · `P0`
- [ ] **T04-021b** · CI job for the golden tests with a software adapter (Mesa lavapipe on Linux, WARP on Windows) and `SS2_FORCE_SOFTWARE_ADAPTER=1`; measure whether goldens made on one platform pass on the other and set per-check tolerances · `S` · `P1` · Deps: T04-021, T12-001
- [ ] **T04-022** · Visual parity workflow: compare to oracle screenshots (T01-038) side by side; checklist per level · `M` · `P2`
- [ ] **T04-027** · Mobile optimization pass: tile-based friendly passes, ASTC/ETC2, limited MRT, half precision, thermal-adaptive scaling · `L` · `PHASE 2`
- [x] **T04-028** · Screenshot/capture, photo mode hooks · `S` · `P3` · 🟢
  F12 writes a PNG. Photo tooling can request capture without the UI draw list.
- [ ] **T04-032** · GLES3 fallback validation (naga output limits, UBO sizes, no compute) and device blocklist · `M` · `PHASE 2`
- [ ] **T04-033** · Render settings hot-reload, shader hot-reload · `S` · `P2`
- [x] **T04-034** · Multi-view split-screen (couch co-op)? decision RFC only · `S` · `P3`
  Decision recorded in [RFC 0011](../rfcs/0011-split-screen.md).

## Menu slice (PC, placeholder skin)
- [x] **T04-035** · wgpu context (windowed + offscreen), 2D quad batch and `glyphon` text with an embedded OFL font; partial T04-001/T04-017 · `L` · `P1` · Deps: T10-032
- [x] **T04-036** · Offscreen render to RGBA image, image diff, golden menu tests (skip without GPU adapter); seed of T04-021 · `M` · `P1` · Deps: T04-035
