# `.wld` world root (T01-012f)

- **Roadmap task:** T01-012f (part f of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the list and ownership relations, the placement hierarchy, the grids' pool structure, the entity class codes and the property records (all checked in every one of the 59 worlds); **medium** for what several lists are *for* (`CRenderingDomain.8`, `CPhysicsDomain.2`, the dynamic light list) and for the `Time` encoding; **low** for the flag words and counters listed under "Open questions". Scope: the 59 distinct format-8 `.wld` worlds (73,098 entities, 217,366 aspects); the contents of the individual entity classes, lights, scripts and so on belong to the other parts (T01-012a…e, g, h, i).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md` (especially the `CMetaHandle` rule and the `EDOB` section).

## Summary

A world is an object graph with **one `CWorld` root**. The root owns the **entity list** (`CWorld.1`), the **entity property records** (`CWorld.5`, stored in the extra section after `EDOB`), five **domain objects** that index the rest (rendering, physics, collision, scripting, macro), three **sync lists** and a few settings (gravity, grid size). Everything placed in the world hangs off an entity as an **aspect** (a renderable or a hull) with a local and a world placement, arranged in a tree. Spatial indexes (**grids**) are pools of nodes with free lists. A **state manager** holds scheduled entries for entities. There is no per-world geometry format here: geometry is placed models (part a).

## Evidence

`tools/re/wld_root_verify.py` (read-only, `SS2_DATA_DIR`; about 20 minutes for all worlds, or pass a number to check only the smallest worlds) over the retail install with all patches. Every check below was run on **all 59 worlds** unless a smaller count is given:

- exactly one `CWorld` per world; the world is in `CResourceFile.rf_crsShared`; exactly one `CWorldInfoEntity`, referenced by `CWorld.7`; exactly one `CStateManager`, referenced by the scripting domain;
- `CWorld.1` lists exactly the objects whose class derives from `CEntity` (73,098), without duplicates;
- 69,177 property records in `CWorld.5`: all are extra records, each names one entity of the world once; the other 3,921 entities have no property record (listed below);
- `CRenderingDomain.1` lists exactly the objects deriving from `CRenderable`; `CCollisionDomain.1` exactly those deriving from `CHull`;
- of 217,366 aspects: 66,886 roots with world placement equal to local placement, **150,480 children with world placement = parent world placement × local placement, in every case**; the parent's child/sibling chain reaches each child; every aspect's owner is an entity of the world or null (12,303);
- the entity class code table (below) has 94 distinct codes and **no code maps to two classes and no class has two codes**;
- the 177 two-dimensional grids and 59 collision grids have valid node child indices and free lists.

## File frame

`CResourceFile` (object 0) has two lists: `rf_crsShared` always contains the `CWorld` (plus, in some worlds, `CMacroProgram` 550, `CModelConfiguration` 245 and one `CEffect` object in total) and `rf_crsExposed` lists objects other files may refer to: mostly the baked lightmap textures (`CStaticTexture`, 18,677 over all worlds), also `CFlareConfiguration` (1,074), `CSpawnEffectConfiguration` (824), `CModelConfiguration` (511), `CMacroProgram` (495), `CSamMaterial` (450), `CEntityAnimConfiguration` (353), `CMesh` (245) and the `CNavMesh`. Which of these a world loader must read is the business of the parts that own those classes.

## `CWorld`

| Member | Meaning |
|---|---|
| `.1` | list of references: **all entities** |
| `.5` | list of references: the **property records** of the entities (extra records, see below) |
| `.7` | → the `CWorldInfoEntity` (an element of `.1`) |
| `.2` / `.3` / `.4` / `.8` | → `CRenderingDomain` / `CPhysicsDomain` / `CCollisionDomain` / `CScriptingDomain` |
| `wo_pmdMacro` | → `CMacroDomain` |
| `.12` / `.13` / `.14` | → `CSoundSyncList` / `CMovieSyncList` / `CVibroSyncList` |
| `.9` | int: **highest entity serial used**: equal to the highest serial in 49 worlds, greater in 9 and below it in 1 (so not a strict rule) |
| `.10` | `Vector3f`: **(0, −30, 0) in all 59 worlds**: almost certainly the gravity vector (m/s², y up) |
| `.11` | float: 0.5 in all 59 worlds |
| `.16` / `.17` | int / float: **grid depth hint** 12 (43 worlds) or 11 (16) and **grid extent** 4096.0 (all); the grids' own depth is within ±1 of `.16` (the 2D grids: 162 equal, 12 deeper, 3 shallower; the collision grids: 54 equal, 4 deeper, 1 shallower), their extent equals `.17` in all |
| `.6` | list, empty in all 59 |
| `.18` | reference, null in all 59 |
| `.19` | int, 3 to 263 and more; a counter (not the number of entities, objects or exposed objects): **unknown** |

## Entities

An entity is an object of a class deriving from `CEntity`; the **class is the object's type**. Common members (all entities): `CEntity.1` and `CEntity.4` (always equal), `CEntity.2`, `CEntity.5`, `CEntity.6` (absent in 17,246 entities, 0 in the others), `CBaseEntity.1` (flags) and, for classes derived from the game's base class, `SBaseEntity.1` (null; absent for the engine's own classes such as `CPointLightEntity`, `CFormationMarkerEntity`, `CMacroEntity`).

- **Serial.** `CEntity.1` (= `.4`) is the entity's **serial number**, unique inside the world, with gaps. The value **`0xFFFFFFFF` means "no serial"** and occurs exactly on the entities that have **no property record** in all 59 worlds: 3,783 `SSpawnerEntity`, 133 `SBulletShooterEntity` and 5 `CEffectEntity`. `CWorld.9` is the highest serial (see above).
- **Class code.** `CEntity.2` is an **external reference to the game module** `Bin/Sam2Game.module` (a native library, not a file). The `EXOB` flag word of that reference is the **entity class code**: 94 codes over the 59 worlds, each belonging to exactly one class, for example 2 `CWorldInfoEntity`, 10 `CStaticModelEntity`, 14 `CCubeBackgroundEntity`, 17 `CDirectionalLightEntity`, 18 `CPointLightEntity`, 19 `CHazeEntity`, 20 `CDetectorAreaEntity`, 21 `CBloomEntity`, 25 `CChapterInfoEntity`, 27 `CGenericItemEntity`, 30 `CEffectEntity`, 40 `CMacroEntity`, 50 `CFluidEntity` (the full table is printed by the script). A reader gets the class from the type table anyway; the code is a cross-check and the identity a game module would use.
- **Flags.** `CBaseEntity.1` is 0 for all but 541 entities, which have the value `0x2`: `CStaticModelEntity` (492), `SBarrierFieldEntity` (42), `CFluidEntity` (7). Meaning unknown.
- **Property records.** For every entity with a serial there is one extra record whose class derives from `CEntityProperties`: `CEntityProperties.1` = the module reference, `.2` → the entity, `.3` = a placement (`QuatVect`); more members per class. The record class name is the entity class name with `Entity` replaced by `Properties` for 69,099 of 69,177 records; the exceptions are `CForceFieldEntity` → `CForceFieldEntityProperties` (38), `CFluidEntity` → `CFluidEntityProperties` (23) and `SLocatorEntity` → `CLocatorProperties` (17). The entity object holds the **runtime state**; the property record holds the **editor-set parameters** (which feed the `.ep` parameter schemas, T01-014 and part d). A world loader must read both.

## Domains

| Object | Member | Meaning (observed) |
|---|---|---|
| `CRenderingDomain` | `.1` | all `CRenderable` objects of the world (59 of 59) |
| | `.8` | a **subset** of `.1`. In 4 of the 5 smallest worlds it is exactly the second renderable grid's list (`.1` minus the haze effect); in the campaign world `1_3_village` it holds only 157 of 895 renderables, and over all worlds the missing ones are mostly editor primitives and models: the renderables that are **in no vis group** (all of them except the haze effect, `wld-visibility.md`) |
| | `.3` | the dynamic **light sources**: `CPointLightSource` (11,349 over all worlds), `CDirectionalLightSource` (89) |
| | `.9` | the haze effect renderable (`CHazeEffectRenderable`, 65 over all worlds) |
| | `.5` | → the `CLightGrid` |
| | `.6`, `.7` | → two different `CRenderableGrid` objects |
| `CCollisionDomain` | `.1` | all `CHull` objects (59 of 59) |
| | `.3` | a subset: `CPrimitiveHull` 3,346, `CModelHull` 439, `CFluidHull` 3, `CForceFieldHull` 2, `CDummyHull` 1 |
| | `.2` | → the `CCollisionGrid` |
| `CPhysicsDomain` | `.1` | the **bodies**: `CDynamicBody` 2,076, `CHybridBody` 1,259, `CKinematicBody` 1,194 |
| | `.2` | a subset of `.1` (identical in 30 worlds, a proper subset in 29): probably the bodies that are simulated |
| `CScriptingDomain` | `.4` | → the `CStateManager` |
| | `.2` | list of steppers: pairs (→ entity of the world, int); 1 to 11 per world |
| | `.3` | always 1 (not the stepper count) |
| `CMacroDomain` | `md_ateEvents`, `md_ulUniqueEventID` | empty list and 0 in the sample (part g) |
| sync lists | | `CSoundSyncList` (`.3` has 0 to 53 entries, `.4` always 0 entries, `.7` 0 to 9, plus `Time` values), `CMovieSyncList.1`, `CVibroSyncList` (`.1`, `.3`, `.4`): contents not analysed |

## Aspects (the placement tree)

Renderables (`CRenderable`) and hulls (`CHull`) derive from `CAspect`:

| Member | Meaning |
|---|---|
| `CAspect.1` | → **parent aspect** or null |
| `CAspect.2` | → **first child** (null if none) |
| `CAspect.3` | → **next sibling** |
| `CAspect.4` | `QuatVect`: **local placement** (rotation quaternion x,y,z,w; translation), relative to the parent |
| `CAspect.7` | `QuatVect`: **world placement** |
| `CAspect.5` | → the **owning entity** (or null: 12,303 aspects) |
| `CAspect.6` | `AspectFlags` (4 bytes): common values `0x5` (104,284), `0x1` (71,419), `0x800005` (24,604), `0x7`, `0x40005`, `0x400005`, `0x420003`, `0x440005`; **bit meanings unknown** |

**Placement.** World = parent world × local with the same convention as skeletons (`skl.md`): rotation matrix of the quaternion, translation as a column vector, `world(child) = world(parent) × local(child)`. It holds for all 150,480 children (tolerance 1e-3); for a root aspect `.7` equals `.4` (all 66,886). The renderables carry in addition `CRenderable.1`, an index that is the list position only in the smallest world: treat it as a serial. The model renderable (`CModelRenderable`) holds a `CModelInstance` (`mdl.md`).

## Grids

All grids are **pools with free lists**; a reader can take the lists as stored.

- **`CGenericGrid2D`** (base of `CRenderableGrid` and `CLightGrid`): `.1` nodes, each `CGridNode2D` = four ints (**child node indices**, −1 for none: a quadtree) and an int (index of the first **link**, −1 for none); `.3` links, each `CGridLink2D` = two ints (entry index, next link); `.5` entries, each `CGridEntry2D` = five ints; `.2`, `.4`, `.6` are the **free lists** (unique valid indices into nodes, links, entries); `.7` depth (11 or 12) and `.8` extent (4096.0). The number of **used entries** (pool size − free list) equals the number of objects in the grid's own list (`CRenderableGrid.1` or `CLightGrid.1`) for all 177 grids. The light grid's list holds only light sources (`CPointLightSource`, `CDirectionalLightSource`), the renderable grids' lists only renderables of the world. Of the two renderable grids of a world, one has an empty own list in 15 of the 59 worlds; in the five smallest worlds the other holds exactly the renderables of `CRenderingDomain.8`.
- **`CCollisionGrid`**: `.1` nodes (`CGridNode`: four child indices + int), `.2` free nodes, `.3` links (`CGridLink`: **hull reference** (null possible) + next link), `.4` free links, `.5` depth, `.6` extent. Every used link points at a hull of the world, and **every hull appears in at least one used link** (all 59 worlds).

How the cells of the quadtree map to the plane and how it is walked is in `wld-visibility.md` (entry = cell rectangle plus level, child slot = `bx + 2*bz`).

## State manager and `Time`

`CStateManager` holds a few `Time` values, a `CSMEntry` structure (`.9`), a `CEntryList` (`.10`: `CEntryList.1` the list of `CSMEntry`, plus `.2`, `.3`, `.4`) and further lists. An entry has a `CMetaHandle` (an object id, null or an entity of the world in all 59 worlds), a `Time` and a set of small ints (−1 mostly). In every world the entry list has fewer than 50 entries. The semantics (scheduled "steps" of entities) are unknown.

`Time` is a structure with one member `Time.1`, a signed 64-bit integer. The data supports **seconds × 2³²** (fixed point with 32 fractional bits): of 164,223 values 159,012 are whole multiples of 2³²; the value `−429,496,729,600` = **−100.0 s** (71,369 times) and `0` (84,966 times) are the common sentinels, `−1` s and `−1000` s occur too; a cluster of values near `1.7016×10⁹ s` decodes to a Unix timestamp in late 2023 (perhaps a modification time). Raw `−1` (211 times) is a separate marker. Confidence medium.

## Reader invariants

1. Require exactly one `CWorld`; take the entity list, property records and domains from its members, never by scanning classes.
2. Every reference is null, an object of the main section, an extra record, or an external reference; an unknown id is an error.
3. Entities with serial `0xFFFFFFFF` have no property record: do not treat that as an error.
4. Build the aspect tree from `.1`, `.2`, `.3`; world placements are stored, so there is no need to recompute them (but the relation above is a free consistency check).
5. Treat the free lists as authoritative: an entry or link index in a free list is unused.
6. Cap every count by the bytes remaining (a world has up to 20,000 entities and 200,000 aspects).

## Test vector

`Content/SeriousSam2/Levels/Multiplayer/DM_DockingBay/DockingBay.wld` (33,217 bytes): 60 main objects and 5 extra records. `CWorld` is object 1: entities `[#4 CWorldInfoEntity, #5 CStaticModelEntity, #6 CChapterInfoEntity]` (serials 0, 1, 2), property records `[#7, #8, #9]` (extra: `CWorldInfoProperties`, `CStaticModelProperties`, `CChapterInfoProperties`), `.7` = #4; domains #10 (rendering), #11 (physics), #12 (collision), #13 (scripting), #14 (macro); sync lists #15, #16, #17; `.9` = 2, `.19` = 6, `.10` = (0, −30, 0), `.11` = 0.5, `.16` = 12, `.17` = 4096.0. `CRenderingDomain.1` lists 7 renderables (#19, #20, #21, #22, #62, #68, #69); `CRenderingDomain.8` the same 7; the light grid is #70, the renderable grids #71 (own list empty) and #72 (own list the 7 renderables); `CCollisionDomain.1` lists the model hull #73; the collision grid is #74 with two used links, both pointing at #73. The aspect tree: #68 (a `CEditorPrimitiveRenderable`, owner #6) has the child #69; #68 has rotation (0, −0.7071, 0, 0.7071) and world translation (22.4866, 0, 30.4261); #69 has local rotation (−0.7071, 0, 0, 0.7071), local translation (0, 0, −2.5) and **world translation (24.9866, 0, 30.4261)**, which is the parent's translation plus the local translation rotated by −90° about y: (2.5, 0, 0).

## Open questions

- Bit meanings of `CAspect.6` and `CBaseEntity.1` (`0x2`); `CEntity.5`/`.6` (always 0); `CWorld.19`, `CWorld.11` and the exact role of `CWorld.9`.
- Exact meaning of `CRenderingDomain.8`, `CPhysicsDomain.2` and `CCollisionDomain.3`; contents of the sync lists, the state manager and the steppers.
- The quadtree geometry of the grids; whether a loader can rebuild the grids from the placements (they look like pure acceleration structures, but this was not proven).
- Everything below the entity level is in the other parts: models and placement (a), lights and baked lightmaps (b), visibility boxes (c, `wld-visibility.md`), entity classes and their properties (d), navigation (e), scripting (g), sound and effects (h), physics bodies (i).
