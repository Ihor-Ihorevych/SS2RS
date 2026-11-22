# Environment effects per level: sky, haze, bloom, fluids, shakers (T01-044)

- **Roadmap task:** T01-044 (feeds the renderer tasks T04-012 and T04-015)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for which entities a level uses, how many, their stored values and how each runtime renderable copies its property record (checked on all 59 worlds with `tools/re/wld_env_verify.py`; members from the files' own type tables, see `environment-schema.md`; the per-level values are in `environment-levels.md`); **medium** for the roles of the haze and bloom numbers (they are known to be copied one to one from the record, but their names are numbers); **low** for how the renderer uses them (formulas, blending, order). Scope: all 59 distinct worlds.

Prerequisites: `wld-lighting.md` (lights, flares, lightmaps), `wld-entities.md`, `wld-physics.md` (fluid and force field hulls), `mtr.md` (water and reflection shaders), `wld-sound-effects.md` (effect entities).

## Summary

- A level's **environment** is a handful of single entities, almost all optional: **one sky cube** (58 of 59 worlds), **0 to 5 haze volumes** (65 in 45 worlds), **0 to 2 bloom entities** (34 in 33 worlds), **fluid volumes** (23 in 15 worlds), force fields (38 in 7), shakers (12 in 7), environment damagers (23 in 2).
- **There is no separate fog or water entity.** Fog-like and haze effects are **haze volumes** (a box with a colour and four numbers); water is a **fluid hull** (a box volume with a density) plus materials that use the water shader (`mtr.md`: 52 `CWaterShaderArgs`); the post effects in the data are **bloom** and camera **shakers** only. Underwater overlays, fades and damage flashes are not stored in the worlds (T04-031).
- Every effect entity has a **runtime renderable** that **copies** the member values of its property record; the record is what an editor shows and a reader should use.

## Evidence

`tools/re/wld_env_verify.py` and `wld_entity_schema.py --match …` (read-only, `SS2_DATA_DIR`) over the retail install with all patches:

- **Sky cube:** the record's size equals the renderable's size and its colour equals the renderable's colour in 58 of 58; 55 skies name an external `.tex` and have face textures on the renderable (five faces, a sixth in 11); 3 skies have none.
- **Haze:** 65 of 65 records are matched member for member by the renderable (table below); **bloom:** 34 of 34; **fluids:** the record's density and second value equal the hull's in 23 of 23.

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

## Test vector

`Content/SeriousSam2/Levels/01_MDigBo/1_3_Village/1_3_Village.wld`: sky texture `mdigbo_village_bckg_0002.tex`, size 64, viewer offset 0, colour 0xFFFFFF; one haze (colour `0x2a383d`; record members `.2` to `.10`: 300, 100, 400, 0, −150, 1,860, 0) and one bloom (colour `0xff463c26`; members 400, 148, 400, 25). `DM_Yodeller`: sky `kronor_snowmnt_00.tex`; haze `0xffffff` (750 × 750 × 750 box, 0, −100, 500, 0); bloom `0xff1b1b1b` (1,000, 1,000, 1,000, 25).

## Open questions

- The meaning and formulas of the haze members `.5`, `.6`, `.7`, `.10` and of the bloom members; whether the haze colour's alpha byte is used; how the renderer blends several hazes and the bloom (T04-012, T04-015).
- Where underwater and fog overlays come from, and how the fluid hull switches the underwater state (the game rules or the renderer, not the world).
- The three skies without textures and the world without a sky.
