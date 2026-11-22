# Lighting: baked and dynamic light, light types, shadows, ambient (T01-045)

- **Roadmap task:** T01-045
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what is stored (the light members and their copies, the baked shadow buffers, the lightmap chain, the one GI object) and for the size, count and value statistics; **medium** for the meaning of the unnamed light members (taken from the editor's help text, the value types and ranges, and the order of the properties); **low** for the three light flags, for `EntityShadowType` and for everything that says how the renderer *combines* the layers (that is code, not data: see "Open questions").

Prerequisites: `wld-lighting.md` (the entities, sources, the lightmap chain and the light list; this page does not repeat them), `wld-geometry.md` (model instances), `mtr.md` and `shaders.md` (modifiers, `CalcLight` and `FullBright` variants, the nine SH constants), `help-digest.md` (the editor help the property names come from).

## Summary

A level is lit in **two layers**:

1. **Baked light.** The editor bakes every light that has "affects light-map" set, together with its shadows and a directional **ambient occlusion** term, into one **lightmap texture per model instance** (18,392 instances, 18,679 textures, stored inside the world file). A lightmapped instance is almost always drawn with a **constant BOOL shader modifier** next to the lightmap modifier (17,799 of 18,392), which fits the "full bright" shader variant: the baked colour is the lighting, no run-time light is added.
2. **Run-time light for dynamic things** (the player, enemies, props that move, projectiles, effects). All 11,438 lights stay in the world as entities and in the light list, because moving objects cannot be baked. Each light carries an intensity multiplier for dynamic surfaces ("dynamic lighting intensity"), and the shaders of lit materials take **nine spherical-harmonics ambient constants** plus a light direction (`shaders.md`).

**Shadows** are therefore baked for the static world. For *dynamic* objects under a **directional light** the world stores a **baked shadow buffer** (`CShadowBuffer`): a 16-bit depth map of the static scene seen from the light, 40 of the 89 directional lights have one. No data says that run-time shadow maps are rendered each frame; that is a question for the code.

**Ambient** comes from three places: an *ambient colour* on each light (applied inside a directional light's box, scaled in the lightmap bake by an "ambient brightness" setting), the **ambient occlusion** baked into lightmaps (a half-resolution mask is kept as editor data), and the SH ambient constants of dynamic objects. The world holds no per-object or per-cell SH data: the light grid is only a spatial index of the lights, and the single **GI cube** object (`CCubeGISHC`, one in the whole game) holds nine zero vectors. So the SH ambient of a dynamic object must be **computed at run time from the lights** (inferred).

## Evidence

`tools/re/shading_verify.py` (read-only, `SS2_DATA_DIR`; about 20 minutes) over all 59 worlds; the printed result is `lighting-tables.md`. The editor's help (`Help/SeriousEngineTools.chm`, pages "Entities", "Texture oven", "Ambient occlusion", read for this task; described in our own words here) names the light properties and what they do. Where this page maps a stored member to a property name, the **mapping rests on the value type and range and on the order of the help text**, and the confidence says so.

## Light types

| Kind | Entity | Count | How the data tells them apart |
|---|---|---|---|
| Omnidirectional point light | `CPointLightEntity` | **10,263** | cone and soft angle both 360° |
| Spot light | `CPointLightEntity` | **1,086** | cone angle below 360° (pairs of cone and soft angle in degrees, most common: 22/49 in 152 lights, 60/120 in 132, 45/90 in 78, 20/35 in 72, 10/20 in 48) |
| Directional light | `CDirectionalLightEntity` | **89** | a separate entity: parallel rays inside an oriented box |

A "spot" is a point light with a cone; there is no separate spot class. 4,046 point lights have a parent entity (a static model in nearly all) and move with it; the others are root aspects.

### Point light record (`CPointLightProperties`, 11,349 records)

| Member | Type | Meaning (help name) | Values in the game | Confidence |
|---|---|---|---|---|
| `.1` | COLOR | diffuse colour | 40+ distinct | high (copied to the source) |
| `.2` | COLOR | ambient colour | 0 in nearly all | medium |
| `.3` | FLOAT | **hot spot**: metres from the centre with full strength | 0 to 373.8; **0 in 5,018** lights | medium |
| `.4` | FLOAT | **fall-off**: metres beyond the hot spot until the light reaches zero | 0.36 to 3,000; at most 10 in 6,575 lights | medium |
| `.18` | INDEX | **cone angle** in degrees (full strength inside) | 0 to 360 | high (source holds the same angle in radians) |
| `.19` | INDEX | **soft angle** in degrees (zero at its edge) | 8 to 360 | high (as `.18`) |
| `.21` | FLOAT | **near clip plane**: no shadows are cast inside this distance | 0.1 in 9,551, 0.01 in 213, 0.5 in 2; **absent in 1,583** (13 worlds use a 15-member variant with `.8` and `.9` instead) | medium |
| `.7` | BOOL | **dark light** (subtracts light) | 1 in 235 | low (order) |
| `.13` | BOOL | **don't cast shadows** | 1 in 20 | low (order) |
| `.14` | BOOL | **affects light-map** (equals `CLightSource.5` of the source) | 1 in 11,286, 0 in 63 | medium |
| `.16` | FLOAT | **dynamic lighting intensity** (multiplier for moving surfaces) | 1.0 in 5,937, **0.0 in 5,107**, 0.3 in 219, 0.2 in 40, 0.1 in 24, up to 2.0 | medium |
| `.11` | FLOAT | preview sphere size (editor only) | 0.008 to 6.5 | medium |
| `.12` | COLOR | preview colour (editor only, alpha set) | | medium |
| `.20` | IDENT | `empty` | | |

Notes. The unnamed second colour of `wld-lighting.md` is the **ambient colour**; `CPointLightProperties.12` is the **editor preview colour**, not a light colour. A hot spot of 0 means the light starts to fall off at the centre. **`.16` is 0 in 45 % of the lights**: if that is the dynamic intensity, nearly half of all point lights are for baking only and do not light moving objects. The mapping of the three BOOLs follows the help's property order (dark light, don't cast shadows, affects light-map) and the ascending member numbers; it is not proved. The 63 lights whose `.14` is 0 are the only ones the runtime source reports as 0 in `CLightSource.5`.

### Directional light record (`CDirectionalLightProperties`, 89 records)

| Member | Type | Meaning (help name) | Values | Confidence |
|---|---|---|---|---|
| `.1` | COLOR | diffuse colour | | high |
| `.5` | COLOR | ambient colour (applies inside the box) | | medium |
| `.2`, `.3`, `.4` | FLOAT | **width, height, depth of the influence box** (the rays run inside it) | 22.5 to 20,000 | medium |
| `.7` | BOOL | dark light | **0 in all 89** | medium (never used, as expected) |
| `.8` | BOOL | **don't cast shadows** | 1 in 22; **none of those 22 has a shadow buffer, and all 40 buffers sit on lights where it is 0** | medium |
| `.14` | FLOAT | **diffusion bias** (a gamma-like curve on the diffuse term) | 1.0 in 74, 0.0 in 7, others 0.5 to 1.9 | medium |
| `.12` | BOOL | probably **affects light-map** | 1 in 61; it is the near-opposite of `.8` (1 in 60 of the 67 lights with `.8` = 0, and 0 in 21 of the 22 with `.8` = 1) | low (a second member with the same number is the shadow buffer, below) |
| `.21` | FLOAT | dynamic lighting intensity | 1.0 in 86, 0.75 in 2, 3.0 in 1 | low |
| `.20` | INDEX | **occlusion map size** (the resolution of the shadow buffer for dynamic objects) | 0 in 40, 1024 in 37, 512 in 10, 1 in 2 | high for the link to the buffer width (below) |
| `.12` (second) | Ptr&lt;`CShadowBuffer`&gt; | the baked occlusion map | present in 40, null in 49 | high |
| `.9` | Ptr&lt;`CTexture`&gt; | the GI cube-map texture | 1 non-null (an embedded `CStaticTexture` in `6_5_bossarena.wld`) | medium |
| `.19` | Ptr&lt;`CCubeGISHC`&gt; | the GI object | 1 non-null (the same world) | high |
| `.10`, `.11` | FLOAT | GI scale and GI normal skew (help names) | 1 in all 89 | low |

The help says the box is the volume in which the light acts and that the **ambient colour applies to the surfaces inside it**. The boxes are large: 55 lights have all three sizes between 1,000 and 10,000, 19 between 100 and 1,000, 5 above 10,000 and 3 up to 100 (the other 7 mix sizes), so a directional light usually covers a whole area but is bounded.

## Baked light

The chain from a model instance to its lightmap, the texture formats and sizes, the ambient mask and the editor copies are in `wld-lighting.md`. What this task adds:

- **What is baked.** The editor's lightmap baker computes the light **and the shadow** of every light with "affects light-map" on, adds an **ambient brightness** term (−1 = none, 1 = double) and, when switched on, **directional ambient occlusion** (radius, blur, number of samples, fill brightness) into one texture per instance, then optionally blurs and anti-aliases it. The **ambient mask of `CAmbientMaskCache` is that occlusion map**, kept so that a re-bake does not recompute it: the help gives the mask resolution as a fraction of the lightmap size, and the game's masks are exactly **half the width and height** (precision 0.5), which agrees with `wld-lighting.md`.
- **Lightmapped instances are lit by the lightmap alone (inferred).** 18,392 instances have a lightmap; **17,799 of them (96.8 %) also carry one or more `CConstantShaderModifier` that set a BOOL parameter to TRUE** (in the four smallest worlds the parameter ids are 14, 24 and 188: 133, 126 and 95 times; the ids are shader parameter ids whose names are not in the data). 593 have none (592 of them only a single lightmap modifier). The 4,453 instances **without** a lightmap all carry constant modifiers and 174 also a gradient modifier; in the four smallest worlds the constants on them are colour overrides (parameter 187 or 198 with a COLOR). The editor help describes a per-object lightmap as "local texture plus the full-bright flag". Together with the `FullBright` and `CalcLight` shader variants (`shaders.md`) this says: **the BOOL switches the object to full bright, so the shader outputs the lightmap-modulated texture without a run-time lighting term**.
- **`EntityShadowType`** (`CStaticModelProperties.9`, a per-model setting whose value names are not stored; the enumeration has three values): **0 in 24,167 models, 2 in 1,161, 1 in 55**. Models with value 0 have a lightmap in 73 % of the cases, value 2 in 47 %, value 1 in 3 of 55. The value is not explained by the lightmap, nor by `CStaticModelEntity.12` (which spreads 0 to 36 over the value-0 models). Treat it as "how this model takes part in shadows" until the code is analysed.

## Dynamic light

- The light list `CRenderingDomain.3` and the light grid hold **all** 11,438 lights (`wld-lighting.md`), so a dynamic object can find the lights near it without the lightmap data. Each light has a **dynamic lighting intensity**; 5,107 point lights have 0 and therefore only exist for baking, and the 63 lights with "affects light-map" off are the opposite case (run-time only, inferred).
- A point light's reach is **hot spot plus fall-off** (both in metres; the source stores the same two numbers as `CPointLightSource.1` and `.2`), a spot light has a cone with a soft edge. The **shape of the fall-off curve is not stored** (linear, quadratic, ...): that is a question for the renderer code.
- A directional light acts inside its box with parallel rays; a **second, ambient, colour** applies in the same box.

## Shadows

| Receiver | Source of shadow | Stored data |
|---|---|---|
| static geometry | baked into the lightmap (light plus occlusion) | `wld-lighting.md` |
| dynamic objects under a directional light | a **baked shadow buffer** of the static scene | `CShadowBuffer`, below |
| dynamic objects under a point light | not found in the data | the "near clip plane" and "don't cast shadows" members only matter for baking or for a run-time method that is not stored |

### `CShadowBuffer` (class 4; 40 objects in the main section of 36 worlds, 40 references)

```
CShadowBuffer : CResource
  .1  int        width in cells
  .2  int        height in cells
  .3  Matrix44f  16 floats, an affine transform: the last row is (0, 0, 0, 1)
                 (the translation sits in members 3, 7, 11)
  .4  array of width x height ints, each 0 to 65535
```

- **Depth map.** The 40 buffers hold **16-bit values** (the largest value found is 65535 and no value exceeds it); **65535 is the "nothing there" value** (probably the buffer's far plane: it is the value of the first cells of every buffer I looked at). In the 39 non-empty buffers the share of other cells is 10 % or more in 17, between 1 % and 10 % in 13 and below 1 % in 9: the buffers look like a depth image of the static geometry seen from the light (inferred from the help text and the affine matrix; the numbers alone do not prove it).
- **Sizes.** The width is 1024 (34 buffers) or 512 (6); the height equals the width in 29 buffers, and differs in 11 (for example 1024 x 704, 512 x 1280, 1024 x 1642). **The width equals the light's `.20` ("occlusion map size") in all 40 buffers.** One buffer has height 0 and a zero matrix (an unbaked placeholder: a reader must accept an empty buffer).
- **Reference.** There are 40 buffers and 40 references, from the second `.12` member of a directional light record (a record holds two members numbered `.12`: an int and the reference, a reader must not rely on member numbers alone, `ctsemeta-objects.md`). 49 lights have none: the 42 whose `.20` is 0 or 1, and 7 whose `.20` is 512 or 1024 (a size was set but no buffer was baked).
- **Use.** The help calls `.20` the size of the "lightmap used for calculating occluded areas for dynamic objects". The buffer is therefore the **static occluder map for objects that move**: transform the object's position with the matrix, compare its depth with the stored value and treat it as shadowed when the stored value is nearer. This is how our renderer can use it; the original's exact comparison, bias and filtering are not stored (see "Open questions").

## Ambient

| Layer | Where it comes from | Data |
|---|---|---|
| ambient colour of a light | `CPointLightProperties.2`, `CDirectionalLightProperties.5` (and the sources) | per light; inside the directional light's box |
| ambient occlusion in lightmaps | baked once (`CAmbientMaskCache`, half size) | `wld-lighting.md` |
| **SH ambient of dynamic objects** | nine constants per draw in the lit shader families | not stored per object or per cell |
| global illumination cube | `CCubeGISHC` (members: `.1` = **nine `Vector4f`**) | **one object in the game** (`6_5_bossarena.wld`, referenced by one directional light with its cube-map texture); **all nine vectors are 0** |

- **The nine `Vector4f` are a second-order SH** (9 coefficients, the same count as the shader constants of `shaders.md`), computed by the editor from the light's "global illumination cube map" with a scale and a normal skew. In the shipped game the values are zero: the boss arena does not use global illumination, and a reimplementation can **ignore `CCubeGISHC`**.
- Because the light grid and the world carry no SH data, the SH constants of a dynamic object are **computed at run time from the lights around it** (their ambient colours inside their boxes and the directional terms). The exact formula is open.

## What a reimplementation needs (our design, not a parity claim)

1. **Lightmapped static models:** sample the lightmap with the UV set named by the oven modifier (`wld-lighting.md`) and draw full bright: no run-time light. Respect the "affects light-map" flag only in tools.
2. **Dynamic objects:** ambient (SH or a hemisphere term) plus the directional light whose box contains the object plus the nearest point and spot lights, each scaled by its **dynamic lighting intensity**; skip lights with intensity 0.
3. **Shadows of dynamic objects:** look them up in the directional light's baked shadow buffer where one exists (decode as above); otherwise do not shadow, or use a simple contact shadow of our own.
4. **Ignore** `CCubeGISHC`, the editor preview members and the lightmap-baker settings.

## Reader invariants

1. A directional light record holds two members numbered `.12`; take the **reference** one as the shadow buffer.
2. A `CShadowBuffer` may be empty (height 0): do not allocate or sample it.
3. Cell values are unsigned 16-bit; 65535 is "no occluder".
4. The width of a buffer equals the light's `.20`; the height is read from `.2`, not derived.
5. A point light's cone and soft angles are stored in degrees in the record and in radians in the source; 360 degrees means omnidirectional.
6. Lights with dynamic intensity 0 can be dropped from the run-time light list.

## Test vectors

- `content/serioussam2/levels/multiplayer/dm_branchester/dm_branchester.wld`: object #7 is a `CShadowBuffer` of **1024 x 1024** (array length 1,048,576), matrix last row (0, 0, 0, 1), last column (members 3, 7, 11, 15) = (0.011, 0.048, −0.868, 1.0); the first cells are 65535.
- `content/serioussam2/levels/01_mdigbo/1_3_village/1_3_village.wld`: object #19 is a `CShadowBuffer` of **512 x 512** (262,144 cells).
- `content/serioussam2/levels/06_kronor/6_5_bossarena/6_5_bossarena.wld`: object #31 is the one `CCubeGISHC` (nine `Vector4f`, all zeros); the directional light that refers to it also refers to the embedded `CStaticTexture` #8162.

## Open questions (a follow-up task, T01-079: needs code analysis or an oracle capture)

- The fall-off curve of point and spot lights, the shape of the soft angle, how the "diffusion bias" is applied, how a directional light's box limits its effect.
- The exact SH computation of the ambient constants and the combination rule of ambient colour, directional and point lights on dynamic objects.
- Whether lightmapped surfaces receive any run-time light, and which shader parameter ids 14, 24, 188 are (the full-bright switch is **inferred**).
- How the shadow buffer is sampled (bias, filtering, which objects use it), and whether any other run-time shadow exists (`EntityShadowType` 0/1/2 names and effect).
- The three point-light BOOLs and the second `.12` of a directional light by name; the variant of 13 worlds with `.8` and `.9` instead of `.21`.
- The byte order of the packed colours (still unconfirmed, `wld-lighting.md`).
