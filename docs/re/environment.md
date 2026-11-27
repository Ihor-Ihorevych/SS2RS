# Environment effects per level: sky, haze, bloom, fluids, shakers (T01-044)

- **Roadmap task:** T01-044 (feeds the renderer tasks T04-012 and T04-015); the sky cube's face directions and image orientation are T01-405 (feeds T04-039); whether and how the sky cube's placement rotation is applied is T01-409 (feeds T04-039)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for which entities a level uses, how many, their stored values and how each runtime renderable copies its property record (checked on all 59 worlds with `tools/re/wld_env_verify.py`; members from the files' own type tables, see `environment-schema.md`; the per-level values are in `environment-levels.md`); **high** for the sky's face-to-face topology and image orientation (which `CCubeRenderable` face member borders which, and how, from decoded face pixels on every sky, independent of the binary: `tools/re/sky_faces_verify.py`); **medium** for which compass direction, and so which engine axis, each face member is, and for the roles of the haze and bloom numbers (the compass reading is from the decompiled `Engine.dll`, cross-checked against the pixel topology, T01-405; the haze/bloom numbers are known to be copied one to one from the record, but their names are numbers); **high** for the distribution of the sky entity's placement rotation over all 58 skies (`tools/re/wld_env_verify.py`, T01-409) and **medium** for whether and how the renderer applies it (decompiled `Engine.dll`, T01-409); **low** for how the renderer uses the rest of the sky's numbers (formulas, blending, order). Scope: all 59 distinct worlds.

Prerequisites: `wld-lighting.md` (lights, flares, lightmaps), `wld-entities.md`, `wld-physics.md` (fluid and force field hulls), `mtr.md` (water and reflection shaders), `wld-sound-effects.md` (effect entities).

## Summary

- A level's **environment** is a handful of single entities, almost all optional: **one sky cube** (58 of 59 worlds), **0 to 5 haze volumes** (65 in 45 worlds), **0 to 2 bloom entities** (34 in 33 worlds), **fluid volumes** (23 in 15 worlds), force fields (38 in 7), shakers (12 in 7), environment damagers (23 in 2).
- **There is no separate fog or water entity.** Fog-like and haze effects are **haze volumes** (a box with a colour and four numbers); water is a **fluid hull** (a box volume with a density) plus materials that use the water shader (`mtr.md`: 52 `CWaterShaderArgs`); the post effects in the data are **bloom** and camera **shakers** only. Underwater overlays, fades and damage flashes are not stored in the worlds (T04-031).
- Every effect entity has a **runtime renderable** that **copies** the member values of its property record; the record is what an editor shows and a reader should use.

## Evidence

`tools/re/wld_env_verify.py` and `wld_entity_schema.py --match …` (read-only, `SS2_DATA_DIR`) over the retail install with all patches:

- **Sky cube:** the record's size equals the renderable's size and its colour equals the renderable's colour in 58 of 58, and its `.4` equals the renderable's `CCubeRenderable.2` in 58 of 58; 55 skies name an external `.tex` and have face textures on the renderable (five faces, a sixth in 11); 3 skies have none.
- **Haze:** 65 of 65 records are matched member for member by the renderable (table below); **bloom:** 34 of 34; **fluids:** the record's density and second value equal the hull's in 23 of 23.
- **Sky cube faces (T01-405):** `tools/re/sky_faces_verify.py` decodes the 53 skies whose face textures are format-8 (two more use pre-format-8 "legacy" textures, `tex-legacy.md`, out of scope here) and compares every pair of face edges. Four members form a consistent four-face ring (edge errors 4.2–5.0 on a 0–255 scale, in 40–41 of 53 skies with both faces, against 17–23 for the two ring diagonals that are not really adjacent); a fifth member borders all four ring faces (errors 2.6–10.7 in 43–44 of 53) and a sixth, present in only 10 of the 53, borders two of the four convincingly (2.7 and 6.0) and is never adjacent to the fifth (55.8): the fifth and sixth members are the cube's top and bottom, the fifth is the one that is (almost) always present.
- **Sky cube placement (T01-409):** `tools/re/wld_env_verify.py` reads `CCubeBackgroundEntity.en_qvPlacement` on all 58 skies. 44 of 58 (76%) carry a **non-identity rotation**; every one of the 58 (the 44 rotated and the 14 identity) is either the identity or a **pure rotation about +Y** (heading only — the renderable's up axis is never tilted, i.e. pitch and bank are always exactly zero, within 1e-3 on the matrix). Of the 44 non-identity headings, only 6 are within 0.5° of a multiple of 90°; the rest spread over nearly the whole -180°..180° range (`1_3_village` itself is -110.0°, matching this task's starting observation exactly). The **position** component of `en_qvPlacement` is never (0, 0, 0) on any of the 58 skies and has no relation to the level's playable area (values from a few metres to tens of thousands, e.g. -37,346 on the Y axis in `dm_chanolin`): it looks like wherever the level editor happened to drop the entity, not a value the renderer could sensibly use to position a cube that must always surround the camera.

## Sky: `CCubeBackgroundEntity`

```
CCubeBackgroundEntity (58 worlds, one each; record CCubeBackgroundProperties)
  .1  -> CCubeRenderable      .3 -> CSimpleBackgroundViewerRenderable
  en_fSize     FLOAT 64 (56 worlds), 32 (1), 6 (1)        en_fViewerY  FLOAT -6.66 .. 4.52     en_qvPlacement QuatVect     en_bActive 1
CCubeBackgroundProperties
  .1  -> CTexture: an external .tex (55; null in 3)   .6 COLOR (6 distinct values; 0xFFFFFF in most)   .2 = en_fSize   .3 = en_fViewerY   .4 FLOAT 0 or 1   .5 BOOL
CCubeRenderable
  .1 size,  .2 FLOAT 0 or 1,  .3 COLOR,  .10 .11 .12 .13 .15 -> CTexture (five faces), .14 -> CTexture (a sixth face, set in 11 skies)
CSimpleBackgroundViewerRenderable   .1 FLOAT 5.22 or 55.68
```

A sky is a **cube of up to six textures drawn around the viewer** at size 64, with a colour tint and a vertical viewer offset: the textures are named per level (`mdigbo_village_bckg_0002.tex`, `kronor_snowmnt_00.tex`, `lavaarena_01.tex`, …). The one world without a sky entity is `dockingbay`.

### Which face member is which direction (T01-405)

| `CCubeRenderable` member | Compass name | Engine axis | Present |
|---|---|---|---|
| `.10` | North | **−Z** | 53 of 53 decoded skies |
| `.11` | West | **−X** | 53 of 53 |
| `.12` | South | **+Z** | 53 of 53 |
| `.13` | East | **+X** | 53 of 53 |
| `.14` | Floor (down) | **−Y** | 10 of 53 (11 of 55 counting the legacy-texture sky below) |
| `.15` | Ceiling (up) | **+Y** | 53 of 53 |

The compass names (**medium**, from the engine binary) come from `SeriousEngine::CCubeRenderable::AutoSetTextures` and the helper it calls, `txuMakeCubeFileNames` (`Engine.dll`, read-only, never copied into this repo): given one base texture name ending in a recognised letter, it builds six file names by appending the literal suffixes `"N"`, `"W"`, `"S"`, `"E"`, `"C"`, `"F"` (or, for a base name ending in a digit, `"1"`…`"6"`) — the letters stand for **N**orth, **W**est, **S**outh, **E**ast, **C**eiling and **F**loor — and assigns them, in that order, to the renderable's six texture-pointer members in ascending offset, which is member order `.10, .11, .12, .13, .14, .15`.

The **ring order** (N next to W next to S next to E, not N next to S) and the **top/bottom identity** (one member touches all four ring faces and is almost always present; a second member, rarely present, touches two of the four convincingly and is never adjacent to the first) are **high confidence**, found independently from the decoded face pixels (`tools/re/sky_faces_verify.py`, evidence below) without reading the binary at all — and the two results agree exactly: North→West→South→East→North is the ordinary counter-clockwise compass cycle. Which engine axis each compass direction is remains a **convention, not a measurement**: `−Z` for North (forward) and `+X` for East follow the standard right-handed assignment (`GEOMETRY.md`: right-handed, Y up, −Z forward; `Right = Forward × Up`, so facing North along −Z puts East at +X) and match the engine's own documented heading convention elsewhere (`behavior-validity-terms.md`: heading 0 is along −Z, a positive heading turns towards −X, i.e. West — consistent with North = −Z, East = +X in a right-handed frame).

### How the face images tile (the orientation rule)

A face texture decodes as in `tex.md`: rows top to bottom, columns left to right. The renderer draws all six faces with the one identical unit-square UV mapping (`CCubeRenderable::RenderObject`, binary, **medium**: only the bound texture differs per face in the drawing loop), so the only per-sky choice is how the artist drew each image to meet its neighbours; `tools/re/sky_faces_verify.py` finds this **from the pixels**, independent of the binary, by matching every pair of face edges (top, bottom, left, right; read forward or reversed) across every sky that has both faces:

| Faces (member) | Edge of the first | Edge of the second | Mirrored | Agreeing skies | Mean edge error (0–255) |
|---|---|---|---|---|---|
| North–West (`.10`–`.11`) | North's left | West's right | no | 41/53 | 4.2 |
| North–East (`.10`–`.13`) | North's right | East's left | no | 40/53 | 4.3 |
| West–South (`.11`–`.12`) | West's left | South's right | no | 40/53 | 4.2 |
| South–East (`.12`–`.13`) | South's left | East's right | no | 40/53 | 5.0 |
| North–Ceiling (`.10`–`.15`) | North's top | Ceiling's bottom | no | 44/53 | 3.0 |
| West–Ceiling (`.11`–`.15`) | West's top | Ceiling's left | no | 44/53 | 2.6 |
| South–Ceiling (`.12`–`.15`) | South's top | Ceiling's top | **yes** | 43/53 | 10.7 |
| East–Ceiling (`.13`–`.15`) | East's top | Ceiling's right | **yes** | 44/53 | 8.9 |
| North–Floor (`.10`–`.14`) | North's bottom | Floor's top | no | 7/10 | 2.7 |
| East–Floor (`.13`–`.14`) | East's bottom | Floor's right | no | 4/10 | 6.0 |

North–South, West–East (the two ring diagonals) and Ceiling–Floor never matched consistently (mean error 17.0–55.8, and the best-matching edges differed from sky to sky instead of agreeing): they are the cube's three **opposite** pairs, which never share an edge, exactly as a cube's geometry requires. West–Floor and South–Floor did not match convincingly either (best errors 27–30, no consistent winner among only 10 skies with a floor face at all) and are **not checked**; by the symmetry of the other three ring faces (each borders the ceiling along its own top edge) they are expected to be West's bottom–Floor's left, no flip, and South's bottom–Floor's bottom, mirrored, the same pattern that the ceiling shows one quarter and one half of the way around the ring from North.

**Reader rule:** place North (`.10`) at −Z, West (`.11`) at −X, South (`.12`) at +Z and East (`.13`) at +X, Floor (`.14`) at −Y and Ceiling (`.15`) at +Y. Going around the horizon the order is North, West, South, East (**not** North, East, South, West); each ring face's left edge continues unflipped into its western neighbour's right edge (so reading the four images in texture-row order, left to right, continues the horizon moving west). Each ring face's top edge continues into the ceiling: unflipped for North and West, mirrored for South and East (and, expected by symmetry, into the floor the same way).

### Worked example: `1_3_village`'s sky

| Member | Direction | File | Size |
|---|---|---|---|
| `.10` | North | `mdigbo_village_bckg_0000.tex` | 1024 × 512 |
| `.11` | West | `mdigbo_village_bckg_0003.tex` | 1024 × 512 |
| `.12` | South | `mdigbo_village_bckg_0002.tex` | 1024 × 512 |
| `.13` | East | `mdigbo_village_bckg_0001.tex` | 1024 × 512 |
| `.14` | Floor | _(none; a 5-face sky)_ | — |
| `.15` | Ceiling | `mdigbo_village_bckg_0004.tex` | 1024 × 1024 |

(`tools/re/sky_faces_verify.py --dump DIR` writes these five as PNGs for a visual check; the ring faces show a starfield over a cloud horizon and tile seamlessly left to right in North, West, South, East order, and the ceiling shows a planet overhead.)

### `.1`: which face it is

The record's own texture (`CCubeBackgroundProperties.1`, `mdigbo_village_bckg_0002.tex` in the world above) is **not a seventh, separate image**: in all 55 named skies it is byte-identical to one of the renderable's own face textures — North (`.10`) in 38, Ceiling (`.15`) in 13, South (`.12`) or East (`.13`) in 2 each (`tools/re/sky_faces_verify.py` cross-check against `wld_env_verify.py`'s record reader, **high**, 55 of 55). It reads as a left-over "representative" pointer, perhaps what a level editor's sky picker shows; a reader only needs the renderable's own six (or five, or in 3 worlds, zero) face members and can ignore `.1`.

### Size, viewer height and colour

`CCubeBackgroundProperties.2` equals the entity's `en_fSize` (the cube's half-extent: **64** world units in 56 of 58 skies, 32 in one, 6 in one) and `.3` equals `en_fViewerY` (**−6.66 to 4.52**, 0 in most): together they place a cube of that size centred on the viewer but offset vertically by `en_fViewerY`, so the cube moves with the camera horizontally but not (fully) vertically — the usual trick to keep a sky cube's horizon level while letting the player fly without the sky "sliding". `.6` is the ARGB colour tint copied onto the renderable (`0xFFFFFF`, i.e. no tint, in most skies; 6 distinct values across 58 worlds) — multiplied over every face's pixels, by analogy with the bloom colour (`CBloomEntity`, above) which is read the same way. `.4` (FLOAT, 1 in 54 of 58, 0 in 4) equals the renderable's `.2` in all 58 skies (same high-confidence record-to-renderable copy as `.2`→size, `.3`→viewer Y and `.6`→colour); its meaning is not named in the data. `.5` (BOOL, 1 in 16 of 58) does not consistently equal anything on the renderable this task checked and remains **not established** (open question, unchanged from T01-044).

### Placement: the cube is rotated, never translated (T01-409)

`CCubeBackgroundEntity.en_qvPlacement` is **not editor-only**: the binary (`SeriousEngine::CCubeRenderable::RenderObject` and `SeriousEngine::CSimpleBackgroundViewerRenderable::AddToRendering`, decompiled `Engine.dll`, read-only, never copied into this repo) builds the drawn cube's model matrix from a quaternion using the full, standard quaternion-to-matrix formula (all four components, the same construction already used by `tools/re/wld_root_verify.py` and `tools/re/skl_verify.py` for every other placement in this codebase) — so the renderer is **structurally applying the full rotation**, not a reduced heading-only angle, and the sky's placement is genuine input to rendering, read every frame, not a value the engine only shows an editor. The quaternion consumed there is the renderable's own placement, which (as for every renderable in this engine, `wld-root.md`) is kept in sync with the owning entity's placement — here `en_qvPlacement` — through the ordinary entity-to-aspect sync, not something special-cased for the sky.

The same function (`AddToRendering`) composes that rotation against the **inverse of the current camera's rotation**, while its translation is taken from the camera/view's own position fields, not from the renderable's own placement position. Net effect, medium confidence: the drawn cube's **orientation follows `en_qvPlacement`'s rotation and stays fixed in world space** regardless of which way the camera looks (the standard skybox trick: cancel the camera's rotation so the object's own rotation is what remains on screen), while its **position always tracks the camera**, not the entity's placement position — consistent with the position evidence above (every one of the 58 skies has a non-zero, seemingly arbitrary `en_qvPlacement` position that cannot be what actually places the cube, since the cube must always surround the player). `en_fViewerY`'s own combination with the camera position (e.g. whether it is added before or after this step) is script logic (`CCubeBackgroundEntity` is a scripted entity, not in the native decompiled tree) and was **not traced further**: treat it as an additive Y offset on top of the camera position, per the data evidence already in the previous section.

`CCubeRenderable.1` (the size) is read directly by both `GetBoundingBox` and `RenderObject` to build the drawn cube's extents (medium confidence, binary) — it is not just stored, consistent with the already-high-confidence record/renderable equality. `CCubeRenderable.2` (the record's `.4`) enters the same extent formula as the size in `RenderObject` (medium-low: based on reading default-constructor values to tell its storage offset apart from neighbouring members, not on a full trace of the formula) — that reads as a second geometric term, not a plain on/off visibility switch as previously guessed; its exact role is still **not established**.

**Reader rule:** apply `en_qvPlacement`'s **full rotation** to the cube (rotate all six face quads by it around the camera); **ignore `en_qvPlacement`'s position** (and `en_fSize`'s role is unaffected: it still sets the cube's half-extent around the camera, offset vertically by `en_fViewerY`, as in the previous section). Applying only the yaw (heading) component would give bit-identical results for every one of the 58 retail skies, because none of them have a pitch or bank component — but that is a fact about this data set, not about what the renderer does, so a reader that aims to match the engine should not rely on it.

## Haze: `CHazeEntity`

`CHazeEntity` (65 in 45 worlds; 14 worlds have none, 31 one, 11 two, one each three, four and five): `.1` → `CHazeEffectRenderable`, optional parent entity (5 are attached). The property record and the renderable hold the **same numbers** (checked on all 65):

| Record `CHazeProperties` | Renderable `CHazeEffectRenderable` | Range |
|---|---|---|
| `.1` COLOR | `.3` | any (e.g. `0x2a383d` in the village world) |
| `.2`, `.3`, `.4` FLOAT | the extents of the box `.1` (`Box3f`) | 10 to 10,000 (the volume's size) |
| `.5` | `.2` | 0 to 999.8 |
| `.6` | `.4` | −300 to 500 |
| `.7` | `.5` | 0 to 5,900 |
| `.10` | `.7` | 0 to 2,500 |

So a haze is **an axis-aligned box volume with a colour and four numbers**; the renderable is `CRenderingDomain.9` (`wld-lighting.md`). The roles of the four numbers (density, distances, height; the middle one can be negative) are not named in the data; `environment-levels.md` lists every haze of every level so the values can be compared with the original's look.

## Bloom: `CBloomEntity`

`CBloomEntity` (34 in 33 worlds: 0 in 26, 1 in 32, 2 in 1; record `CBloomProperties`): `.1` → `CBloomRenderable`. The renderable copies the record: colour `.1` → `.3`, `.5` → `.2` (0 to 100), `.6` → `.4` (0.3 to 1.5), `.8` → `.6` (0.5 to 6), `.13` → `.12`, `.16` → `.13` (0 to 1), `.21` → `.18` (0.5 to 3); the remaining members are constants (1.0, 0, 3) and a texture pointer that is **null in all 34** (no bloom mask). `.2` to `.4` of the record (96 to 10,000) look like a box size as in the haze (not checked). Colours are stored ARGB, for example `0xff463c26` in the village world and `0xff1b1b1b` in a deathmatch level.

## Water: fluids, force fields and shaders

- **`CFluidEntity`** (23 in 15 worlds; record `CFluidEntityProperties`: `.2` a `CPrimitiveDesc`, `.3` density, `.4` a second value, `.11` BOOL): `.1` → a **`CFluidHull`** (`wld-physics.md`) holding the volume `Box3f`; the record's `.3` equals the hull's `.1` (**1000**, 500 or 980) and `.4` equals its `.2` (0.85 or 1.25): 23 of 23. A fluid is a **box of water or similar liquid** with a buoyancy density; where the surface material and the overlay come from is not in the entity.
- **`CForceFieldEntity`** (38 in 7 worlds; hull `CForceFieldHull`): `.4` 2,000 to 100,000, `.5` 20 to 500, a `Vector3f` and a box: their meaning is not named in the data (a force on bodies inside the volume is likely).
- **Water surfaces** are materials with the **water shader** (52 `CWaterShaderArgs`, textures `waterreflection`, `cavereflection`, bump maps: `mtr.md`); which models use them in which level is a model-to-material question (`mdl.md`), not an entity of the world.

## Shakers and damagers

- **`SShakerEffectEntity`** (12 in 7 worlds): a `CShakerRenderable` with `CShakingParams`: hot spot 5 to 1,000 and fall-off 30 to 1,500 (the two-radius model of `audio.md`), damping (0.75 to 2), Y, Z and bank (B) intensity (0 to 4) and frequency (0 to 6,000) and a constant-intensity period (0, 1 or 5): probably a **camera shake around a point**, started by scripts (`level-logic.md`).
- **`CEnvironmentDamagerEntity`** (23 in 2 worlds): a box, a type (always 0) and a healing flag: a damaging or healing volume.

## Per level

`environment-levels.md` (generated) has four tables: the sky (texture, size, viewer offset, colour), every haze (colour and the members above), every bloom, and per world the fluids (density, second value, box), force fields, shakers, damagers and portals. Counts per world: haze 0 to 5, bloom 0 to 2, sky 1 (58 worlds).

## Reader invariants

1. Read the sky, haze, bloom and fluid values from the **property records**; the renderables repeat them.
2. A world without a sky entity, haze or bloom simply has none: do not substitute defaults from another level.
3. A haze is a box in world coordinates; several can overlap; their order is the list order (`CRenderingDomain.9` lists the haze renderables).
4. A fluid's box and density come from the hull; the visual water comes from materials, not from the fluid entity.
5. Fail on an unknown environment class; the classes in `environment-schema.md` are the complete list of the retail data.
6. **The sky cube's six face members are fixed directions, not an arbitrary list** (T01-405): `.10` North (−Z), `.11` West (−X), `.12` South (+Z), `.13` East (+X), `.14` Floor (−Y) and `.15` Ceiling (+Y); place each face's quad accordingly and ignore `CCubeBackgroundProperties.1` (it duplicates one of the six, not a seventh texture). A sky with five faces is missing the floor (`.14`); render it as an open bottom (no quad), never a default or repeated texture.
7. **The sky cube is rotated by `CCubeBackgroundEntity.en_qvPlacement`'s full rotation, around the camera, and never translated by its position** (T01-409): apply the rotation to the six face directions above before placing them around the viewer, offset vertically by `en_fViewerY`; `en_qvPlacement`'s position is not used by the renderer (evidence above) and must be ignored.

## Test vector

`Content/SeriousSam2/Levels/01_MDigBo/1_3_Village/1_3_Village.wld`: sky texture `mdigbo_village_bckg_0002.tex`, size 64, viewer offset 0, colour 0xFFFFFF, faces North `mdigbo_village_bckg_0000.tex`, West `..._0003.tex`, South `..._0002.tex` (the same file as the record's own texture), East `..._0001.tex`, Ceiling `..._0004.tex`, no floor face (a five-face sky); `en_qvPlacement` is a **pure yaw of -110.0°** at position (145.7, -1374.4, 49.6) (T01-409: the rotation is applied around the camera, the position is not); one haze (colour `0x2a383d`; record members `.2` to `.10`: 300, 100, 400, 0, −150, 1,860, 0) and one bloom (colour `0xff463c26`; members 400, 148, 400, 25). `DM_Yodeller`: sky `kronor_snowmnt_00.tex`; `en_qvPlacement` a pure yaw of 60.0°; haze `0xffffff` (750 × 750 × 750 box, 0, −100, 500, 0); bloom `0xff1b1b1b` (1,000, 1,000, 1,000, 25).

## Open questions

- The meaning and formulas of the haze members `.5`, `.6`, `.7`, `.10` and of the bloom members; whether the haze colour's alpha byte is used; how the renderer blends several hazes and the bloom (T04-012, T04-015).
- Where underwater and fog overlays come from, and how the fluid hull switches the underwater state (the game rules or the renderer, not the world).
- The three skies without any face texture (`1_4_approachingcity`, `dm_cockaleekie`, `dm_hole`: the record's own texture is null too) and the world without a sky entity at all (`dockingbay`) are identified (T01-405) but not explained: whether the renderer falls back to a flat colour, the `.6` tint alone, or simply draws nothing is not in the data. Two further skies (`dm_deserttemple`, `dm_dockingbay`) name face textures that are pre-format-8 "legacy" `.tex` files (`tex-legacy.md`), which `tools/re/sky_faces_verify.py` does not decode.
- Whether the floor (`.14`) really borders the west and south ring faces the way the geometry requires: only its seams with north and east were confirmed by pixel matching (too few of the 10 full-cube skies have well-matched floor art to confirm the other two, T01-405).
- The meaning of `CCubeBackgroundProperties.4` and `.5` (`.4` is confirmed copied onto the renderable's `CCubeRenderable.2`, both FLOAT 0 or 1; T01-409 found it enters the same extent formula as the size in `RenderObject`, leaning away from a plain visibility flag, but its exact role is still not established; `.5` does not consistently equal anything checked) and of the viewer renderable's own `.1` (5.22 or 55.68).
- How `en_fViewerY` is actually combined with the camera position (T01-409 traced the cube's position in `AddToRendering` to the camera/view's own fields, not the entity's placement, but `CCubeBackgroundEntity` is a scripted entity and the exact point where the Y offset is added is not in the native decompiled tree).
