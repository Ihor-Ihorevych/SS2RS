# T05 — Shaders (`ss2-shaderxlat` and material shader library)

Context: the install ships `Bin/GPU-Programs.asm` (≈83 KB, D3D9-era vertex/pixel shader assembly) and
`Bin/Shaders.module`; materials (`.mtr`, `.mdl`) reference `Bin/Shaders.module`. Two ways to get
the same look in WGSL: **translate** the original assembly, or **re-write** shader families by hand
from the behavior spec. The spike T05-006 chooses (likely: translate for coverage + hand-tune hot ones).

- [ ] **T05-001** · Parser for D3D9 shader assembly (`vs_1_1/2_0/3_0`, `ps_1_x/2_0/3_0`) → typed AST/IR; declarations, defs, modifiers, swizzles, write masks, predicates, loops/if · `L` · `P0` · 🟢-ish · Deps: T01-020 for samples
- [ ] **T05-002** · IR → `naga` IR → WGSL/SPIR-V emitter; scalar/vector ops, saturate, source modifiers, dst masks · `L` · `P0` · Deps: T05-001
- [ ] **T05-003** · Register/constant model: `c#`, `i#`, `b#`, sampler `s#`, `v#`/`t#`/`o#` semantics → stable binding layout (bind groups, UBO std140-compatible, push consts avoided on GLES) · `M` · `P0`
- [ ] **T05-004** · Instruction coverage matrix & tests (texld/texldp/dp3/dp4/mad/lrp/cmp/exp/log/pow/rcp/rsq/sincos/nrm/…); CPU reference interpreter for the IR · `L` · `P0` · 🟢
- [ ] **T05-005** · Understand/describe shader module composition: how `.mtr` params choose program + permutation; build `ShaderLibrary` index (spec T01-020 → code) · `M` · `P0`
- [ ] **T05-006** · **SPIKE**: translate vs hand-write per shader family; measure parity and mobile cost; RFC with recommendation · `M` · `P0`
- [ ] **T05-007** · Numerical validation: render a probe quad with CPU-interpreted vs GPU-executed shader across a parameter grid, max error report · `M` · `P1` · Deps: T05-002/004
- [ ] **T05-008** · Pipeline/shader cache keyed by (shader id, permutation, target features); precompile on first run with progress (avoids hitching on PC too) · `M` · `P1`
- [ ] **T05-009** · Cross-backend validation: naga output through Vulkan/Metal/DX12 validators in CI (GLES added in Phase 2); limits report (samplers per stage, UBO size) · `M` · `P1`
- [ ] **T05-010** · Fixed-function-era behavior emulation (alpha test, fog, texture stage states) when materials depend on it · `M` · `P1`
- [ ] **T05-011** · Hand-written WGSL "base library": unlit, lit-lightmapped, skinned-lit, additive particle, water, sky, glass, decal, post effects — as per T05-006 outcome · `XL` → split by family · `P0`
- [ ] **T05-012** · Mobile variants: reduced-precision, fewer lights/taps, no dynamic branching hot spots · `L` · `PHASE 2`
- [ ] **T05-013** · Shader debug tooling: disassembly viewer, permutation explorer, overdraw/heat views (viewer in T11-006) · `M` · `P2`
- [ ] **T05-014** · Documentation: "how SS2 materials map to our shaders" for artists/modders · `S` · `P2` · 🟢
