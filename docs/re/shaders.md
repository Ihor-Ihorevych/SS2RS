# Shader system: `GPU-Programs.asm`, the shader classes and how materials reach them

- **Roadmap task:** T01-020
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the file structure of `GPU-Programs.asm`, its naming scheme, its program inventory and the vertex and pixel interfaces that the file itself documents; **medium** for how permutations are assembled and chosen (read from the names and from how the fragments fit together, never from the engine); **low** for what the compiled shader classes in `Bin/Shaders.dll` do beyond the class names, and for the meaning of the `CT` version tags. **T01-417** (open question 6): **medium-low** for assigning three of `CBloomRenderable`'s six numeric members to this file's documented `BloomEffect` colour/threshold/blur-scale register roles, by cross-referencing against binary-read clamp ranges (`environment.md` "Bloom"). **T01-423: medium** for where the code that actually uploads them lives (`Bin/ProcRender.dll`, not `Shaders.dll`), for the camera-box edge-fade gate, and for a strength multiplier and a built-in colour-ramp selector among the members. **T01-424: high** for every `CBloomRenderable`/`CBloomProperties` member's own CTSEMETA number and observed range (read directly from every world's own type table and object data, not inferred); **medium** for each number's functional role and for the `efxBloom` chain's exact mechanics (downscale-loop size, high-pass HDR-only threshold, blur two-tap offset and balance factor, composite blend and alpha handling, three-anchor colour-ramp values), all read from `ProcRender.dll` in full — this corrects T01-417's `.4` (it is the strength multiplier, not the high-pass threshold, which is a different member, `.19`) and settles which renderable number (`.18`, not `.12`) is the colour-ramp selector.

Prerequisites: `mtr.md` (materials, shader class codes, argument classes), `bmf.md` (vertex data), `ctsemeta-container.md` (`EXOB`). Related: `help-digest.md` (the editor help describes what each shader does).

## Summary

The engine ships **no per-material shader source**. A material (`.mtr`) names one of **eight shader classes** (`mtr.md`: codes 1000 to 1070). The GPU code for them lives in one text file, **`GPU-Programs.asm`**: Direct3D 9 **assembly** (vertex shader 1.1; pixel shaders 1.1, 1.4 and 2.0) in **128 named programs**. The same file is stored in the archive as `Sources/Shaders/GPU-Programs.asm` (two versions: 80,623 bytes in `All_PC_03.gro`, 82,917 bytes in `Patch_02_068.gro`) and is also installed beside the engine as `Bin/GPU-Programs.asm`, identical to the patched version.

The shader **classes** (`CShadeShader`, … ) are compiled code in `Bin/Shaders.dll` (278 KB), loaded as "the shader module" (the `Bin/Shaders.module` name that every material's external reference uses). The classes decide *which* programs to combine and which constants to upload; the programs are the shared building blocks. Where this spec says "the engine chooses", that is inferred from names, not read from the class code.

## Evidence

- `tools/re/shader_verify.py` (read-only, standard library) parses the file and prints counts, never program text. Reproduces every number below.
- The register interfaces come from the **comment block that the authors put above each family** (inputs `v0…`, constants `c…`, outputs `oT…`); the table below restates them. Program bodies were read to confirm the structure (fragments, tags, opcodes) and are **not** reproduced or paraphrased line by line.
- Nothing from binaries.

## File structure

- ASCII text, LF line ends, 3,389 lines. Starts with a copyright comment, then **20 section banners** of the form `// - - - - - Name shader vertex programs - - - - -` (a vertex banner and a pixel banner for each of the ten families).
- A **program** is a **label** at the start of a line, `Name:` (optionally followed by a `//` comment on the same line), up to the next label. Labels are unique (128), always of the shape **`<Family>_<VP|PP><model digits>_<variant>`**:

  | Part | Values |
  |---|---|
  | Family | `BillBoardShader`, `BlendLayersShader`, `BloomEffect`, `PolyBumpShader`, `ReflectionShader`, `RefractionShader`, `ShadeShader`, `SpecularShader`, `TerrainShader`, `WaterShader` |
  | `VP` / `PP` | vertex program (59) / pixel program (69) |
  | model digits | none, `1`, `2`, `11`, `14`, `20`: the pixel-shader generation the program is written for (none = the base program; `1`/`11` = 1.1 or 1.x, `14` = 1.4, `2`/`20` = 2.0); `VP1`/`VP2` appear once each, in the bloom family |
  | variant | a free name of the permutation (`CalcLight`, `FullBright`, `2shades1add`, `L0_M1S_L1_S`, `4lyr1shd`, …) |

- Block comments (`/* … */`, 76 of the programs have one) carry the register documentation; `//` comments follow instructions. A block comment in the middle of the file explains the billboard rotation geometry (its first line looks like a label, `ROTATION:`, but lies inside a comment).
- **Every program that has a body ends with `END`** (127 of 128). The one exception, `BloomEffect_PP1_downmax4x4`, is an empty stub (a label and nothing else); a loader must accept it or the engine never asks for it.
- **Version tags.** A program that is complete starts with a version line: `CTvs.1.1` (15 vertex programs), `CTps.1.1` (41 pixel), `CTps.1.4` (6), `CTps.2.0` (19), or the plain Direct3D form `ps.1.1` (2). The prefix `CT` is not a Direct3D token; the engine's own assembler front end evidently consumes it (probably "Croteam", marking programs for its preprocessor); what it changes is unknown.
- **Vertex programs are fragments.** Of the 59 vertex programs only the **15 headers carry a version tag**; the other **44 have none**, and are the fragments that a header program is completed with. Every family has one header (some two: `ShadeShader` and `ReflectionShader`, and the four bloom programs) followed by 2 to 10 fragments (the four bloom programs are complete headers without fragments), for example for the shade family: header `CalcLight` or `FullBright` (the two lighting modes), fragments `ColorMap` (optional vertex colour), `Output` (distance fade, projection, fog), and `1Shade` / `2Shades` / `3Shades` (the shade texture coordinates). So a final vertex program is **a header plus a chosen set of fragments concatenated in file order** (per family: lighting, optional colour map, output, optional extras). The names show the permutation axes; the exact order and the rule for combining fragments belongs to the shader classes (open).
- **Pixel programs are complete**: all 68 that have a body carry their own version tag; the variants cover the combinations of textures (layers, masks, shade textures, additive overlays).
- **Pseudo-opcodes.** Besides real Direct3D opcodes the file uses three that are not Direct3D: `col vN` (3 uses, always commented "RGBA->ARGB": it converts the byte order of the vertex colour), `txp tN` (2 uses: a projective texture fetch in the 1.1 pixel model) and `expp` (1 use, probably the partial-precision form of exp). A consumer must treat them as **macros of the engine's assembler**.
- Opcodes seen: `add`, `col`, `dcl`, `dcl_2d`, `dcl_cube`, `def`, `dp3`, `dp4`, `expp`, `frc`, `lit`, `lrp`, `m3x3`, `m4x3`, `m4x4`, `mad`, `max`, `min`, `mov`, `mul`, `nrm`, `phase`, `pow`, `rcp`, `rsq`, `sge`, `slt`, `sub`, `tex`, `texcoord`, `texld`, `texldp`, `texm3x2pad`, `texm3x2tex`, `texm3x3pad`, `texm3x3vspec`, `txp`, with the usual modifiers (`_pp`, `_sat`, `_x2`, `_x4`). Inline constants use `def` (23 programs; 1 to 4 each).
- The programs are short: 68 of 128 have fewer than 10 instructions, 36 have 10 to 19, the longest has 48.

## The ten families

| Family | Programs (VP + PP) | What it renders (from the family's own notes) | Material class (`mtr.md` code) |
|---|---|---|---|
| `ShadeShader` | 14 (7 + 7) | the basic textured, lit surface: base texture, up to three shade (overlay) textures and additive overlays, per-vertex lighting, fog, distance fade | `CShadeShader` 1000 (6,213 configs) |
| `SpecularShader` | 5 (3 + 2) | shade plus specular highlight with a detail and gloss texture | `CSpecularShader` 1010 |
| `ReflectionShader` | 20 (12 + 8) | cube-map reflection with Fresnel term, flat, bump-mapped or offset (parallax) variants, with or without a shade layer | `CReflectionShader` 1020 |
| `RefractionShader` | 6 (3 + 3) | bump-distorted projective refraction of a texture | `CRefractionShader` 1025 |
| `BlendLayersShader` | 19 (7 + 12) | up to four textured layers blended by masks (`L` layer, `M` mask, `S` shade in the variant names) | `CBlendLayersShader` 1030 |
| `TerrainShader` | 24 (8 + 16) | terrain painted from up to five layers plus up to two shades; three pixel-program generations (11, 14, 20) per combination | `CTerrainShader` 1040 |
| `PolyBumpShader` | 10 (4 + 6) | normal-mapped lighting with optional specular and offset mapping | `CPolyBumpShader` 1050 (with the "Pixelight" arguments) |
| `WaterShader` | 9 (6 + 3) | animated water: four waves summed from Taylor-series sine and cosine, Fresnel, projective reflection | `CWaterShader` 1070 |
| `BillBoardShader` | 7 (5 + 2) | camera-facing quads with rotation and colour map (particles, flares); has no class code among those found in materials, so it is presumably used by the effect renderers directly | none (no billboard class among the codes materials use) |
| `BloomEffect` | 14 (4 + 10) | the post-process bloom: 4 × 4 downscale, high-pass (contrast), separable blur, add; with 1.x and 2.0 generations | none (post effect, `help-digest.md`: "Bloom" entity) |

## Vertex and register interface (as documented by the file)

All families share one **vertex layout vocabulary** (the engine builds the vertex streams from `bmf.md` data):

| Input | Meaning |
|---|---|
| `v0.xyz` | position (billboards: centre; `v0.w` rotation angle) |
| `v1.xyz` | normal |
| `v2` / `v2.xyz`+`w` | colour map (vertex colour) in the simple families, or tangent with `w` = binormal sign in the bump families |
| next `vN.xy` | texture coordinate sets in the order of the family (diffuse, bump, detail, shade, layer 1 … 4, mask) |
| the last two inputs | **bone indices** and **bone weights**: the engine skins on the GPU |

Common **constants** (documented in each family's note; indexes differ per family): `c0` a set of small numeric constants (0, 0.5, 1, 2 or 3; the programs also define their own with `def`); `c1…c4` the **model-view-projection** matrix (billboards: projection only plus the model-view in `c9…c11`); `c5` the constant colour and alpha of the material; a stretch/offset or 2 × 2 **mapping matrix per texture** (scale, offset, rotation); the **fog scale and bias for opaque and for translucent objects** (a linear fog, `-1/(end-start)` and `end/(end-start)`); the viewer position in object space; a **distance fade** scale and bias; the light direction (in the specular and bump families); and **nine spherical-harmonics coefficients** (`c14…c22` in shade and blend-layers, `c13…c21` in specular, `c12…c20` in polybump, `c21…c29` in water) that carry the **ambient light from the baked light grid** (second-order SH of the normal: constant, 3 linear, 5 quadratic terms; confirmed by the arithmetic of the `CalcLight` fragments). Bone matrices follow after the fixed constants (`c15+` in billboards).

**Outputs:** `oPos`, a diffuse colour/alpha `oD0` (bump families also `oD1` = N·L), texture coordinates `oT0…oT7`, with tangent-space light, view and half vectors in `oT1…oT5` for the bump families. **Pixel inputs** are named in each family's second note (`t0` base, `t1` mask or normalisation cube map, `t2` layer or reflection, `t3` shade mask or cube map, `s0…` samplers in the 2.0 programs, `c0…` light colour, specular colour and power, gloss attenuation, bump strength).

Examples of the family notes: the bump families need **tangents** (`v2` with binormal sign) in the mesh; reflection needs an object-to-absolute matrix for the environment cube map; refraction needs a **projective texture** of the scene already rendered; terrain uses up to eight texture coordinate sets, one per layer; water takes **per-vertex wave weights** and four wave parameter sets.

## How materials reach the programs (what is known, what is inferred)

1. A `.mtr` config references a shader class and carries its arguments (`mtr.md`). Known.
2. The class turns the arguments into **a vertex program (header plus fragments) and a pixel program**, uploads the constants above and binds the textures. Inferred from the names: the permutation axes are the **number and kind of textures** (shade textures: `1shade`, `2shades`, `3shades`; additive: `1add`; layers: `2lyr`, `3lyr` … `5lyr`), the lighting mode (`CalcLight` vs `FullBright`: the "full bright" flag of the shader), the optional vertex colour (`ColorMap`), the bump or offset mapping, specular, and for water and terrain the hardware tier.
3. **Hardware tiers.** The terrain, reflection, refraction, water and bloom families exist in several pixel-program generations (1.1, 1.4, 2.0). The engine presumably picks the best generation the graphics card supports; the shipped game's baseline is 1.1, and **1.1 programs exist for the shade, blend, billboard, specular and polybump-base families**. A reimplementation that targets modern GPUs only needs the **semantics** of each program, not the generations: see below.
4. The compiled form: the install carries three versions of the Direct3D shader compiler library (`d3dx9_25/26/27.dll`), so the assembly is presumably **assembled at load time** by the Direct3D runtime after the engine's front end has resolved the `CT` tags and pseudo-opcodes. Unconfirmed.

**What a reimplementation needs from this spec**: for each family, the list of inputs, the lighting model (SH ambient, optional directional/specular), the texture combination rules the variant names describe, the fog and distance-fade formulas, and the effect-specific maths (water waves, billboard rotation, bloom kernels). The programs themselves are the authoritative statement of those formulas; T05-001 (a parser of this assembly) and T05 tasks may read them. This spec deliberately stops at the structure so that no program text is copied into the repository.

## Reader invariants

1. Labels are unique and the file ends after the last program's `END`; accept the one empty stub.
2. Split programs at labels; strip `//` comments and block comments before looking at instructions.
3. A program is a vertex program iff its label contains `_VP`; vertex programs without a version tag are **fragments**; they only make sense appended to a header of the same family.
4. Treat `CTvs.x.y`, `CTps.x.y` as the versions `vs_x_y` and `ps_x_y`; accept the bare `ps.1.1`.
5. Implement `col` (colour byte-order), `txp` (projective fetch) and `expp` as macros; reject any other unknown opcode.
6. The line ends are LF; text is ASCII; do not depend on the code page.

## Open questions

1. The exact rule for combining fragments (order, which are optional) and the mapping from material arguments to program names for each class: needs the class code (`Shaders.dll`; T01-059 with this task's successors).
2. What the `CT` prefix does, and the exact semantics of `col`, `txp` and `expp`.
3. The register map of the pixel programs for the families whose pixel note is short (shade, blend: partly documented). **A pattern noted but not reconciled** (T01-408, `wld-lighting.md` "Baked lightmaps"): across the shade family's "full bright, N shade textures" pixel programs, each additional shade texture scales the overall output twice as hard as one fewer (identical on the Direct3D and OpenGL code paths, which reach it through different instruction sequences); net of that, each shade texture itself is combined with the base texture by a plain, unscaled multiply. Read naively this looks like a ×2 "overbright" brightness factor for a lightmapped draw, but a direct real-game visual comparison (T03-039b) found ×1, not ×2, matches the retail game, and T01-408 could not reconcile the two — treat the pixel-program pattern as an open question, not an answer. The full per-input register role (which input is the lightmap versus a detail or bump-adjacent "shade" texture in each family) also stays open.
4. Which pixel generation the shipped PC game selected on which hardware (the loader), and how the 1.x programs differ visually from the 2.0 ones (quality tiers).
5. The shadow and sky handling that is not material-based (T01-044, T01-045). **Fog is answered for the non-material half** (T01-412, `environment.md` "Combining several hazes into one screen fog"): the several-haze combination into one per-frame colour/near/far triple happens in `Engine.dll` (`CViewRenCmd::ActivateExecution`), not in a shader class, and the vertex program's own fog term (the `Output` fragment, `ShadeShader` family) uses **clip-space depth**, not Euclidean view distance — a separate, unrelated "distance fade" term in the same fragment is the one that uses true 3-D distance, for translucency near the camera, not for fog. Whether the sky cube is itself fogged by render-state leakage (its own draw call never arms or disarms fog, instead bypassing the material pipeline entirely via `gfuUseSimpleShader`) is **narrowed but still open** (T01-416, `environment.md` "The sky cube's fog state"): the GPU's actual fog-enable bit is armed or disarmed only by `shaGetFogFactors` (called once per material draw), not by `shaEnableFog` itself, and every view disarms it unconditionally at its own end, so the answer turns on whether the sky sorts before or after a fogged material in the same view's per-frame render-command list — a question the decompiled C's `qsort` comparator does not answer (T01-422).
6. The compiled `Shaders.dll` also holds the **class implementations**; whether any shader work happens there beyond choosing programs (software skinning, the bloom chain) is open. **Narrowed for bloom, then settled by T01-423 (`environment.md` "Bloom"):** `CBloomRenderable`'s own field layout and clamp ranges were confirmed from a `Sam2Game.dll` routine (T01-417), giving a medium/medium-low-confidence mapping of colour, a high-pass threshold and a blur spread/scale to three of the six numeric members. **T01-423 found the per-frame code is not in `Shaders.dll` at all** — `CBloomRenderable`'s own virtual overrides and the `efxBloom` free function that actually runs the chain (downscale, high-pass, two-pass blur, additive composite, directly against the current back buffer) live in a fifth, previously-undecompiled module, `Bin/ProcRender.dll` (binary, medium): `Sam2Game.dll`'s own import table resolves the class's constructor/destructor there, not to `Engine.dll`. So `Shaders.dll`'s role for bloom is limited to owning the `BloomEffect` GPU programs themselves (`GPU-Programs.asm`); the CPU-side logic that picks constants and drives the chain is entirely outside it. Whether `Shaders.dll` still does any *other* shader work beyond selecting per-material programs (for example software skinning) stays open; the bloom-specific half of this question is closed. **T01-424 read `efxBloom` in full and corrected the member-role mapping** (`environment.md` "Bloom"): the "high-pass threshold" T01-417/this file attributed to renderable `.4` is actually renderable `.19` (always authored 3, and only read under HDR rendering); `.4` is in fact the per-entity strength multiplier `efxBloom` combines with the camera fade and the global `efx_fBloomBias`. The downscale/blur stages are fused into one ping-ponged loop (quarter both dimensions per iteration until ≤128 texels, with a two-tap ±1.5-texel offset-accumulate blur folded into each iteration) rather than two separate passes.

## Test vectors

`tools/re/shader_verify.py` on the retail install prints (among others): 128 programs; 59 `VP` and 69 `PP`; 10 families; version tags `CTps.1.1` 41, `CTps.2.0` 19, `CTvs.1.1` 15, `CTps.1.4` 6, `ps.1.1` 2, none 45 (the 44 vertex fragments and the one stub); 127 programs end with `END`; the longest program has 48 instructions; 20 section banners.

The first program in the file is `BloomEffect_VP_downscale4x4`: tag `CTvs.1.1`, 10 non-blank non-comment lines (tag, one `def`, three arithmetic instructions, four coordinate adds, `END`), documented as taking a 2D position in texture-coordinate space and producing four sample coordinates `oT0…oT3`. The first fragment-only vertex program is `BillBoardShader_VP_constColor` (no version tag; one instruction and `END`); a typical file-order group of the shade family is header `ShadeShader_VP_CalcLight`, fragments `ShadeShader_VP_ColorMap` and `ShadeShader_VP_Output`.
