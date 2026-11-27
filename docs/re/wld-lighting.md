# `.wld` lighting: lights, baked lightmaps, flares (T01-012b)

- **Roadmap task:** T01-012b (part b of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the light entities, their property records and sources, the light list and grid, the baked lightmap chain (model instance → modifier → texture → bake record → ambient mask) and for how flares attach; for, as of T01-061 (`Engine.dll`, read in full: `CTextureOvenShaderModifier::{ApplyModifications,Bake}`), how the oven modifier's two shader parameter ids reach the shader-argument override mechanism and for that `Bake` being editor-only; and, as of T01-406 (same two functions re-read, plus `CModelRenderable::Bake`, `mdlBakeModel` and `shaExecutePreset`, `Engine.dll`, all read in full, and a data script over all 59 worlds and the meshes of the 8 smallest), for the three ways the lightmap UV map id is **not** sourced (not a mesh UV id, not a per-instance or per-world stored stream, not an editor remap table) and for the id being a **per-world, not per-instance, constant**; **medium** for the roles of the individual light and bake parameters (read from value ranges and from the copy relation between record and source) and for the runtime path that T01-406 traced but could not pin down (`ApplyModifications`'s own mechanism, and that it runs in the shipped game via `CModelRenderable::Bake`, are medium; where the UV value is actually consumed is **open**, see below); **low** for the exact per-class meaning of the two shader parameter ids and the colour byte order (not newly resolved: lights are converted to spherical-harmonics coefficients before reaching the vertex shader, `CDefaultShader::Exec`, not passed as a classic per-pixel light colour this reading could trace further). Scope: the 59 distinct format-8 `.wld` worlds: 11,349 point lights, 89 directional lights, 18,679 baked lightmaps, 2,501 flares.

Prerequisites: `wld-root.md` (entities, aspects, domains, grids), `wld-geometry.md` (model instances), `mtr.md` (shader modifiers), `tex.md`.

## Summary

Lighting in a world has two layers. **Dynamic lights** are entities (`CPointLightEntity`, `CDirectionalLightEntity`) with a runtime light source and an editor property record; all light sources are listed in the rendering domain and in the **light grid**. **Static lighting is baked into textures**: every lit model instance carries a **`CTextureOvenShaderModifier`** in its modifier list that overrides one texture slot of the model's material with a **lightmap texture stored inside the world file**, together with a UV map id; a bake record and an ambient mask are kept in the extra section (editor data). Flares (`SFlareEntity`) attach a flare renderable to a model next to a light.

## Evidence

`tools/re/wld_lighting_verify.py` (read-only, `SS2_DATA_DIR`; about 20 minutes) over all 59 worlds. Facts below are over all 59 worlds unless a smaller set is named; **T01-061** additionally decompiled and read, in `Engine.dll`: `CTextureOvenShaderModifier::{ApplyModifications, Bake}`, cross-referenced against `shaModifyValue` (`mtr.md`, T01-059). **T01-406** (the lightmap UV source, below) adds `tools/re/wld_lightmap_uv_verify.py` (read-only, `SS2_DATA_DIR`; about 5 minutes) over all 59 worlds for the id-distribution and editor-class counts, and over the 8 smallest worlds' instances for the mesh check (it resolves each lit instance's external `.mdl` and checks every surface of every LOD, all 8 UV slots); and re-reads, in `Engine.dll`: `CTextureOvenShaderModifier::ApplyModifications` (again, confirming T01-061), `CModelRenderable::Bake`, `mdlBakeModel`, `shaExecutePreset`, `CBaseTexture::IsLightMap`, `CTextureModifier::IsLightMap`, `CDefaultShader::Exec` (re-read from T01-059), and searches (by name, not full decompile) `mdlGetSurfaceUVMaps`/`mdlGetSurfaceUVMapsCount`/`rmGetUVMaps`:

- **Lights.** For all 11,349 point lights and 89 directional lights the light source holds copies of the members of the property record (table below). The light list `CRenderingDomain.3` equals the set of all light sources in 59 of 59 worlds, and the light grid's own list equals it in 59 of 59. Every light source has **no owner** (`CAspect.5` is null, 11,438 of 11,438) and a **child aspect owned by its light entity** (the editor marker, 11,438 of 11,438).
- **Lightmaps.** 18,679 oven modifiers in the main section; each lightmap texture is used by **exactly one** instance in every world; 18,676 of the 18,679 textures are `CStaticTexture` objects of the main section and **all of those are exposed** through `rf_crsExposed`.
- **Flares.** All 2,501 flare renderables are owned by their flare entity.

## Light entities

```
CPointLightEntity                      (class code 18)
  .2   -> CPointLightSource            the runtime light (an aspect, owner null)
  .3   -> parent entity or null        4,048 point lights have a parent: a CStaticModelEntity; 2 more have a CGenericItemEntity
  .4   int                             (as the entity serial in wld-root.md)
CDirectionalLightEntity                (class code 17)
  .2   -> CDirectionalLightSource
  .3   -> null in all 89
CPointLightProperties / CDirectionalLightProperties   (extra records, one per entity, see wld-root.md)
```

The light source is an **aspect**. It is a **root aspect exactly when the light has no parent entity** (11,438 of 11,438); a light with a parent entity is a child of one of the parent's aspects (the parent's renderable): its parent aspect belongs to the parent entity in 4,050 of 4,050 cases and `CEntityProperties.5` is the parent's property record (4,050 of 4,050). Placement follows the rule of `wld-geometry.md`: for a light **with** a parent the property placement is the source's local placement (4,048 of 4,050); for a light without one it is the source's world placement (7,216 of the 7,388 lights; the remaining 172 differ and are not analysed).

The **editor marker** is a `CEditorPrimitiveRenderable` that is a child aspect of the light source (11,349 under point lights, 89 under directional lights). Directional lights also carry a `CShadowBufferBaker` child (89) and a `CShadowBuffer` reference (record `.12`, source `.15`, for the lights with shadows).

### Source and property record

The runtime source is a **copy of the editor record** (all of these equalities hold for every light unless a count is given):

| Source member | Record member | Observed values |
|---|---|---|
| `CLightSource.1` | `CPointLightProperties.1` / `CDirectionalLightProperties.1` | **colour** as a 24-bit integer (for example `0xFF9555` in the test vector; the byte order is not confirmed) |
| `CLightSource.2` | `.2` (point) / `.5` (directional) | second colour: 0 for nearly all point lights |
| `CLightSource.3`, `.4` | `.7`, `.8` (directional) | small ints (0, 1) |
| `CLightSource.5` | `.14` (point) / `.12` (directional) | 1 = enabled in 108 of 109 sample lights |
| `CLightSource.6` | `.16` (point) / `.14` (directional) | float, 1.0 |
| `CPointLightSource.1` | `CPointLightProperties.3` | float: 0.0 or an intensity-like value (for example 1.52, 90.0 in the three smallest worlds) |
| `CPointLightSource.2` | `CPointLightProperties.4` | float, **range-like**: 4.0 to 150.0 or more |
| `CPointLightSource.6`, `.7` | `CPointLightProperties.18`, `.19` | **cone angles**: the source stores **radians**, the record **degrees** (exact for all 11,349); 360° / 2π means an omnidirectional light; spot values: 22, 60, 45, 20, 90, 50 … |
| `CPointLightSource.9` | `CPointLightProperties.21` | 0.1: equal in 9,766 of 11,349 lights, so **not** a copy rule |

`CPointLightProperties.12` holds a second colour with alpha (`0xFFFFFF00`, `0xFFFF7200`, …); the roles of `.11`, `.13`, `.20` (0) and `.7` are not determined. The directional light's properties carry range-like floats (`.2`, `.3`, `.4` = 1,000 to 10,000) and a member `.20` (0, 512 or 1024; perhaps a shadow-map size).

### Light list and light grid

`CRenderingDomain.3` lists **all light sources** (11,438; equal to the union of point and directional sources in every world). The `CLightGrid` (the quadtree of `wld-root.md`) holds the same set in its own list (`CLightGrid.1`) in every world, so the grid is an index over lights.

## Flares

`SFlareEntity` (2,501): `.1` → `CFlareRenderable`, `.2` → parent entity (a `CStaticModelEntity` in 2,288, null in 213), `.4` a list of two. The flare renderable's parent aspect is the parent model's renderable (2,278), null (215) or a `CDynamicBody` (8); its first member names the **flare configuration**: an embedded `CFlareConfiguration` (1,074) or an external file (1,427; the `.lfx` flare configuration, now specified in full in `fx.md`, T01-017 — confirmed: `.lfx` is referenced only from worlds, 62 `RFIL` entries to all 37 files). The property record links the flare to another entity in `CEntityProperties.7`: a `CPointLightProperties` (616), a `SFlareProperties` (609), a `CEffectProperties` (439), a `CStaticModelProperties` (65), one sound entity, or null (771). When it links to a point light, the **flare renderable's next sibling aspect (`CAspect.3`) is that light's source** in 599 of the 616 cases: the flare and its light are siblings under the same model.

## Environment entities

`CBloomEntity` (0 to 2 per world), `CHazeEntity` (0 to 5; its renderable `CHazeEffectRenderable` is `CRenderingDomain.9`), `CCubeBackgroundEntity` (1 in 58 of the 59 worlds; the sky cube, `CCubeRenderable`): single post-processing and background effects. Their members were not analysed here (part h).

## Baked lightmaps

The chain, for each lit instance of a model (`wld-geometry.md`):

```
CModelInstance.4          -> CShaderModifierList (main section)
  CShaderModifierList.1   list of modifiers; one of them is
CTextureOvenShaderModifier  (main section)
  CShaderModifier.1       EPlatform
  .1, .2                  int, int: two shader parameter ids (the texture and the UV map slot being overridden)
  .4                      CShaderArgValTexture -> the LIGHTMAP TEXTURE (a CStaticTexture in the main section, exposed)
  .3                      CShaderArgValUVMap   -> the UV map id to use for it
  .6                      -> CTextureBakerLightMap (extra section)
CTextureBakerLightMap     (extra section, editor data)
  .9                      -> CAmbientMaskCache (extra section)
CAmbientMaskCache
  .1, .2                  width, height
  .3                      the mask bytes (width x height, one byte per cell)
```

- **One lightmap per instance.** 18,392 instances have oven modifiers (18,105 exactly one, the others two or more); every lightmap texture is used by exactly one instance.
- **Texture.** The lightmap is a `CStaticTexture` object stored **in the world file** (18,679). The formats are DXT1 (17,299), 24-bit (1,123), 32-bit (251), and a few others; sizes are powers of two from 16 to 2,048 (64: 7,601, 128: 7,185, 256: 2,287, 32: 906, 512: 452, 1,024: 129, 16: 111, 2,048: 5), square in 18,265 of 18,676 cases. The `TexFlags` of the lightmaps are `0x71` (15,812), `0x31` (2,325), `0x30` (525), `0x70` (13) and `0x20` (1): **bit `0x10` is set on 18,675 of 18,676**, while `tex.md` found it on only three standalone textures (`0x71` twice, `0x60` once); so `0x10` marks a generated/baked texture (T01-054). The wrap modes are `0x51`/`0x51` (14,625), `0x50`/`0x50` (4,050) and `0x52`/`0x52` (1).
- **Editor copy.** For each main-section oven modifier there is one oven modifier in the extra section (18,680 against 18,679; equal in 58 of 59 worlds); the copies are referred to by the property records of the entities that were baked: `CStaticModelProperties` (18,481), `SDoorProperties` (115) and `SPropProperties` (84).
- **Bake record and mask.** The bake record lives only in the extra section: 18,495 of 18,679 modifiers point to one; 181 records have no member `.9`. Where present, the **ambient mask is half the lightmap size** (width and height halved, one byte per cell) for 16,589 of the lightmaps checked; the others have a different size. The bake record's parameters are nearly constant: two profiles in the sample of four worlds (`.10` = 10.0 with `.11` = 2 and `.12` = 20 in 722 records, 25.0 / 12 / 36 in 52); `.2`, `.7`, `.14` = 0.5, `.13` = 0.3, `.20` = 0.4. They are editor settings.
- **UV map id (T01-406: not a mesh id, not per-instance, not a stored stream — a per-world constant; where it is consumed stays open).** The id in `.3` is **not** one of the model's mesh UV ids: not just in a four-world sample, but checked directly against the actually-referenced mesh of every lit instance in the 8 smallest worlds (732 instances; the mesh check resolves the instance's external `.mdl` and scans **every** UV slot of **every** surface of **every** LOD, not only the slots a material uses) — **0 of 732** have their lightmap id anywhere in their own mesh's UV id list. This rules out "a mesh UV stream under a different id" directly, not just statistically.

  **T01-406 also found that the id is not per-instance at all: it is (almost always) a single constant for the whole world.** Over all 59 worlds, grouping each world's oven modifiers by UV id: **31 of 59 worlds use exactly one id for every one of their lit instances**, 25 use two, 2 use three (`tools/re/wld_lightmap_uv_verify.py`); **18,468 of 18,679 modifiers (99 %)** carry their world's single most common id, and that id is **55 of 59 worlds'** only id. The two overridden shader-parameter ids (`.1`/`.2`) follow the same pattern — a handful of large, distinct pairs (e.g. `(378, 379)`, `(320, 322)`, `(23, 24)` — some consecutive, some not), each reused 700–1,700 times, consistent with one pair per world (or bake session), not per instance. So the id cannot be a per-instance generated unwrap, nor an index into a per-instance or per-world array keyed by instance (a shared id would collide every mesh against every other mesh in the world); the earlier text's "probably generated for the instance" is **superseded** by this finding.

  **Ruled out, not just "not found": a stored UV stream in the oven-modifier chain, and a remap/layout table.** The full oven-modifier → bake-record (`CTextureBakerLightMap`, all 12 members dumped for the test vector) → ambient-mask chain, and the extra-section property-record copy (`CStaticModelProperties`), hold only scalar bake-tool settings (floats, small ints) and the ambient-mask byte array — no float array of the right shape for UV coordinates anywhere reachable from the modifier. Classes named for UV maps in `classes-catalog.md` (`CUVMapLayout`, `CForegroundUVMapList`, `CTexCoordVertexMap`, `CUVMapViewCustomData`) are rare and do not track the oven-modifier count: `CUVMapLayout` and `CForegroundUVMapList` occur in **0 of the 59 worlds**; `CTexCoordVertexMap` in 41 of 59 but 1 to 69 per world, uncorrelated with that world's (constant) lightmap count; all four are strings found only in the EditKit modules per `classes-catalog.md`, not in `Engine.dll`/`Core.dll`/`Sam2Game.dll` — editor mesh-import bookkeeping, not the universal mechanism.

  **The mechanism that pushes the override is confirmed (T01-061, re-confirmed T01-406) to run in the shipped game, not only the editor.** `CTextureOvenShaderModifier::ApplyModifications` (`Engine.dll`, read in full again) calls the generic `shaModifyValue` override-stack push (`mtr.md`) **twice**, once for the parameter id paired with the texture wrapper and once for the id paired with the UV-map wrapper, each time passing the engine's own "invalid identifier" as the search key — so each push always creates a **fresh** stack entry; `ApplyModifications` itself takes no placement or geometry argument, so it cannot be computing or generating UV coordinates. **Newly traced (T01-406):** this virtual call is driven by `CModelRenderable::Bake()`, which builds a placement matrix and calls the free function `mdlBakeModel(CModelInstance*, const Matrix34f&)` — a **different, shipped** function from the modifier's own `Bake`; it walks the instance's shader-modifier list and invokes each modifier's virtual apply. So the override genuinely happens every time a lit renderable bakes in the real game (this refines, not contradicts, the existing finding: it is `CTextureOvenShaderModifier::Bake` — the pixel-painting function below, confirmed **editor-only** — that never runs in the shipped game, not the modifier's `ApplyModifications`/`CModelRenderable::Bake` path).

  **Where the overridden value is actually read back and matched to a vertex stream was not found in `Engine.dll`.** `shaExecutePreset` (read in full) dispatches each shader config straight to the shader's `Exec` with an already-built `CShaderArgs`; it contains no mesh, UV or stream-matching code, so the general rule for ordinary materials ("match the UV map id against the surface's declared ids", `mtr.md`) is not implemented there — it must happen earlier, while a surface's `CShaderArgs`/vertex declaration is assembled, a step this reading did not locate by name (`CModelRenderable::RenderObject` → `mdlRenderModel` → unnamed callees). The surface's **own** material (checked on the mesh of the test vector's model) never uses member numbers `12`/`13` for its own texture/UV-map argument pair (it uses `.10`/`.11`, `.20`/`.21`, …, the per-class slot numbers of `mtr.md`), so the oven modifier's two overridden ids are not simply "the diffuse slot's own numbers": they come from some other, evidently shared/global id space. `CBaseTexture::IsLightMap` / `CTextureModifier::IsLightMap` (both read in full) exist as a predicate on the texture (matching the `TexFlags 0x10` bit already found in the data) but this reading found no caller of either by name in `Engine.dll`, `Core.dll`, `Sam2Game.dll` or `GfxD3D.dll` (both are virtual; a caller through the vtable would not show up in a name search). No unwrap, planar- or box-projection, or "generate UV" function was found among the named functions of `Engine.dll` either (`mdlGetSurfaceUVMaps`/`rmGetUVMaps` only **enumerate** a mesh's existing declared ids, for example for an editor UI; they do not create one).

  **Best remaining hypothesis (not confirmed, for a follow-up task):** given the id is a per-world constant that never matches any mesh and that no stream-matching or UV-generation code was found in `Engine.dll`, the runtime most plausibly special-cases a texture flagged `IsLightMap()` and samples it through an already-bound channel (most likely the instance's own base/diffuse UV, which — unlike the oven modifier's id — **is** one of the mesh's declared ids) rather than resolving `.3`'s value as a lookup key at all; the numeric id would then be inert editor/serialization bookkeeping. Confirming or refuting this needs the shader module (`Shaders.dll`, not yet decompiled for this project, T01-020) or a closer trace of the per-surface `CShaderArgs` assembly in `Engine.dll`; see the follow-up task below.

- **`Bake(instance, placement)` on the modifier itself** (read in full, T01-061) is confirmed to be **editor-only**: it does nothing unless the modifier has a bake record (`.6`/`.9` chain), and when it runs it prepares the texture and asks the bake record's own virtual method to fill the pixels — the shipped game loads the already-baked texture directly and never calls this. (This is `CTextureOvenShaderModifier::Bake`, distinct from `CModelRenderable::Bake` above, which does run in the shipped game.)

## Reader invariants

1. For each `CModelInstance`, apply the modifiers of its list in order; a `CTextureOvenShaderModifier` replaces a texture slot by the lightmap texture.
2. Read the lightmap texture from the world's own `CStaticTexture` objects (zlib blob, formats of `tex.md`); never expect it as an external file.
3. **Do not look up the oven modifier's UV map id (`.3`) in the mesh's own per-surface UV id list** (`bmf.md` member 12/14): it is confirmed (T01-406) never to be one of them, and it is a per-world constant, not a per-instance or per-mesh value — treat it as opaque. Until the real consumption rule is confirmed (see "Baked lightmaps" above and the follow-up task), sample the lightmap through the instance's own base/diffuse UV map (the UV id already used by texture group 1 of its material, `mtr.md`) as a pragmatic approximation; this is **not** a confirmed engine behavior.
4. A reader may ignore the extra-section bake records and ambient masks (editor data).
5. Use the light list (or the light grid) for dynamic lights; the light source holds the runtime values, the record is the editor copy.
6. A light source has no owner: reach it through its entity's member `.2`.
7. Convert cone angles from radians (source) or degrees (record) consistently.

## Test vector

`dm_stadium.wld` (`Content/SeriousSam2/Levels/Multiplayer/DM_Stadium/`, 2,920,647 bytes, the second smallest world): the first point light entity is #412 (serial 138) with source #1885 and parent entity #389; the source has `CLightSource.1` = `0xFF9555`, `CLightSource.2` = 0, `CPointLightSource.1` = 90.0, `.2` = 150.0, `.6` = 0.7854, `.7` = 1.5708, and its property record `PointLight001` has the cone angles 45 and 90 (degrees). `CModelInstance` #2435 has the modifier list #3137 = [#3553 `CTextureOvenShaderModifier`, #3554 `CConstantShaderModifier`]; #3553 has parameter ids 12 and 13, UV map id 10, the texture #11 (a 128 × 128 `CStaticTexture`, format 7 = DXT1, `TexFlags 0x71`, exposed) and the bake record #3821 (extra section, `.2` = 0.5, `.6` = 1, `.7` = 0.5, `.20` = 0.4) which refers to a 64 × 64 ambient mask of 4,096 bytes. **T01-406:** instance #2435's model is the external `Fence.mdl` (two surfaces, 79 and 336 vertices), whose own declared UV ids are only `{3, 4}` — `10` is absent even as an unused slot; and UV id 10 is in fact used by **all 266 of 266** oven modifiers in `dm_stadium.wld` (the parameter ids 12/13 are likewise this world's only pair), confirming the id is a per-world constant here, not a value tied to instance #2435 or to `Fence.mdl`.

## Open questions

- The shader parameter ids `.1`/`.2` of the oven modifier and the UV map id `.3`: **the mechanism that pushes them is answered by T01-061/T01-406** (section "Baked lightmaps" above: each id is pushed through `shaModifyValue` as a fresh override, and it runs in the shipped game via `CModelRenderable::Bake`/`mdlBakeModel`, not only in the editor). **T01-406 ruled out** the id being a mesh UV id, a per-instance/per-world stored stream, or an editor remap table, and found it is instead a **per-world constant**; **still open** (needs `Shaders.dll`, T01-020, or a closer trace of the per-surface `CShaderArgs` assembly in `Engine.dll`): where the value is read back and which UV channel a lightmapped draw actually samples — the leading hypothesis is that it samples the instance's ordinary base/diffuse UV channel and the id itself is not resolved as a lookup key at all, but this is not confirmed. **T01-407** continues this.
- The roles of `CPointLightProperties.3` (intensity?), `.11`, `.12`, `.13`, `.7`, `CPointLightSource.9`, and the directional light's parameters; byte order of the packed colours.
- The 172 lights without parent whose property placement differs from the world placement; the 184 oven modifiers without a bake record in the extra section; the lightmaps whose ambient mask has another size; the 411 non-square lightmaps.
- ~~Which lights cast shadows and how the `CShadowBuffer` is used.~~ Answered in `lighting.md` (T01-045): the baked shadow buffer, the flags, ambient and GI; the run-time use stays open (T01-079).
- ~~The external flare configuration files (`.lfx`).~~ **Answered**: `fx.md` (T01-017) specifies the format in full. The effect entities themselves (`CBloomEntity`, `CHazeEntity`, `CCubeBackgroundEntity`) stay open (part h).
- How the baked lightmaps combine with dynamic lights in the shader: `lighting.md` gives the data-side answer (lightmapped instances carry a full-bright style BOOL modifier); the shader-side rule needs code analysis (T01-079).
