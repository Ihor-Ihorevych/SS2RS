# `.wld` geometry: placed models, hulls and the collision grid (T01-012a)

- **Roadmap task:** T01-012a (part a of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for how a static model is placed (every relation below was checked on all 25,383 static model entities of the 59 worlds), for the hull and collision-grid cell relation (all 30,085 hulls) and for which entity classes carry which geometry, and, as of T01-060 (`Core.dll`'s `CPrimitiveDesc::{GetVolume,GetBoundingBox,GetInnerRadius}` and `Engine.dll`'s `CEditorPrimitiveRenderable::AddToRendering`, read in full), for the names of the four collision `PrimitiveType` shapes and for editor renderables being conditionally drawn, not unconditionally skipped; **medium** for `CStaticModelProperties.5`/`.6` (now named, not just guessed, via their setters' own names) and for `PrimitiveType` 4's identity; **low** for the remaining integer members listed under "Open questions". The world contains **no brushes or terrain meshes**: the level architecture is placed models.

Prerequisites: `wld-root.md` (root, entities, aspects, grids), `mdl.md` (models), `ctsemeta-objects.md`.

## Summary

Almost all visible geometry is a **`CStaticModelEntity`**: an entity that places one model (`.mdl`) in the world. The entity owns a **renderable** (`CModelRenderable`, which holds the model instance with the model name and a scale), optionally a **mechanism** (`CMechanism`, the physics wrapper) and one or more **collision hulls**. A hull is registered in the **collision grid** over a rectangle of cells that is stored on the hull. Other entity classes (pickups, props, doors, barrier fields) carry the same classes in other combinations; markers, lights and characters carry editor-only primitive or model renderables.

## Evidence

`tools/re/wld_geometry_verify.py` (read-only, `SS2_DATA_DIR`; about 20 minutes for all worlds) over the retail install with all patches; **T01-060** additionally decompiled and read, in full: `Core.dll`'s `CPrimitiveDesc::{GetVolume,GetBoundingBox,GetInnerRadius,mdPostChange}`, `Engine.dll`'s `CEditorPrimitiveRenderable::AddToRendering`, `Engine.dll`'s `CModelRenderable::{SetCullDistance,SetFadeLength}` and their caller in `Sam2Game.dll`'s `CStaticModelEntity` construction. All counts are over **all 59 worlds**:

- **25,383 static model entities.** Every renderable is owned by its entity (25,383); the entity's model (`CStaticModelEntity.6`) equals the model named by its renderable's instance (25,383) and by its property record (`CStaticModelProperties.1`, 25,383); **all 25,274 external model references resolve to an existing `.mdl`** (2,258 distinct models); 109 entities embed a `CModelConfiguration` instead.
- **Hulls.** Of the entities, 21,420 have a mechanism and **exactly those have at least one hull** (21,420 both ways; the other 3,963 have none); the mechanism's renderable is the entity's renderable (21,420). For **all 30,085 hulls**: the cell rectangle is well-formed, the number of used collision-grid links that point at the hull **equals the number of cells of the rectangle**, and the level never exceeds the grid depth.
- **Placement.** For the 20,605 entities without a parent entity the top aspect is a root aspect (20,605), and the entity's property placement equals its world placement for 20,499 of them; for the 4,778 child entities the property placement is the top aspect's local placement in 4,714.

## Placing a model

```
CStaticModelEntity                     (CEntity.2 = class code 10, see wld-root.md)
  .1   -> CModelRenderable             the renderable (its CAspect.5 owner is the entity)
  .10  -> CMechanism                   null for entities without hulls
  .6   -> the model                    external .mdl (25,274) or embedded CModelConfiguration (109)
  .21  -> parent entity or null        4,778 entities have a parent (CStaticModelEntity or SAnimatedMoverEntity)
  .11  int                             1 exactly when the entity has a hull (all 25,383): a collision flag
  .12, .20  int                        copies of CStaticModelProperties.12 / .8 (all 25,383): see "Open questions"
CModelRenderable                       (an aspect, see wld-root.md)
  .1   -> CModelInstance
CModelInstance
  .1   Vector3f                        scale
  .2   CModelAnimQueue                 animation queue (as in mdl.md)
  .3   -> the model                    external .mdl or embedded configuration; equals the entity's .6
  .4   -> CShaderModifierList or null  per-instance material modifiers: present in 22,103 of 25,383 instances
```

- **Scale** is per axis, `1,1,1` in 2,683 instances, **uniform** in 16,377, **non-uniform** in 5,057 and with a **negative axis** (a mirrored model) in 1,266. A reader must apply it and handle mirroring (it flips the triangle winding).
- **Modifiers.** `CModelInstance.4` is a `CShaderModifierList` as in `mtr.md`: parameter animations (gradients, graphs) applied to the model's materials for this instance only.
- **Embedded models.** An embedded `CModelConfiguration` is a complete model object of the format in `mdl.md` stored in the world; the 109 cases are not analysed further.
- **Names.** The property record (`CStaticModelProperties`, extra section) carries the editor name in `CEntityProperties.4` (`Column`, `mdigbo_fence_01`, …) and the same model reference.

### Entity placement and the aspect subtree

The entity's **property record** holds its placement in `CEntityProperties.3` (`QuatVect`: rotation x,y,z,w and translation). An entity's aspects (renderable, hulls, sound sources) form **one subtree** for 25,229 of 25,383 entities; the other 154 have several tops (164 entities have two or more hulls). The **top** of the subtree is the **hull when the entity has exactly one** (checked on the three smallest worlds: 212 of 212), the renderable when it has none (68 of 68 in the same worlds).

- **No parent entity** (`.21` null): the top is a root aspect and the property placement **equals its world placement** (20,605 of 20,605 roots; the property placement matches in 20,499).
- **Parent entity** (`.21` set): the top aspect's parent is an aspect of the parent entity (4,756 of 4,778), `CEntityProperties.5` is the parent's property record (4,778 of 4,778), and the property placement is the top aspect's **local** placement (4,714 of 4,778). The tree rules of `wld-root.md` then give the world placement.

## Mechanisms and hulls

`CMechanism` is the physics wrapper of an entity: `.6` → the renderable, `.1` a list of parts (`CMechanismPart`, one per hull in 21,338 of 21,420 mechanisms), `.10`/`.12` placement data, `.3` an optional `CMechanismMotionHandler` (moving platforms and doors), `.7` an optional external reference. Its contents are part i of the world spec.

A **hull** (`CHull`, an aspect) is a collision shape:

| Class | Count | Shape | Body (`CHull.1`) |
|---|---|---|---|
| `CModelHull` | 21,194 (static model entities) + 825 (barrier fields) | the **model's own collision mesh** (`CModelHull.1` → a second `CModelInstance` naming the same model as the entity: 21,194 of 21,194) | null (21,599) or `CKinematicBody` (420) |
| `CPrimitiveHull` | 7,804 | a **primitive** described by `CPrimitiveHull.1` (`CPrimitiveDesc`: `PrimitiveType` + three floats) | `CDynamicBody` 3,247, `CKinematicBody` 1,708, `CHybridBody` 1,302, null 1,547 |
| `CDummyHull`, `CForceFieldHull`, `CFluidHull` | 200 / 36 / not counted | markers, force fields and fluids | null |

`CHull.9` points to the owning entity's mechanism (checked for all 22,094 hulls of entities with a mechanism: no exception). A primitive hull with a **body** is a physics object (pickups, props, doors); a model hull without a body is static level geometry.

**Primitive descriptors.** `PrimitiveType` values over all primitive hulls: 1 (4,534), 0 (1,410), 3 (959), 2 (901). Type 0 has only its first parameter set in 1,272 of 1,410 (a one-parameter shape: most likely a **sphere** of that radius; the pickups of the three smallest worlds all use 2.0), the other types use three parameters. The editor primitive renderables use the same descriptor class (types 1: 24,091, 4: 18,403, 0: 14,940, 3: 1,098, 2: 8).

**The shapes are now confirmed** (T01-060: `Core.dll`'s `CPrimitiveDesc::GetVolume`, `::GetBoundingBox` and `::GetInnerRadius`, read in full — all three switch on the type and their arithmetic names the shape). Calling the three floats (in member order, presumably `CPrimitiveDesc.2`/`.3`/`.4` after the type at `.1`) first/second/third:

| `PrimitiveType` | Shape | Parameters | Evidence |
|---|---|---|---|
| 0 | **sphere** | first = radius | `GetVolume`: `r³ × 4⁄3π` from the first float alone; `GetBoundingBox`: a cube of side `2r` built from it alone; `GetInnerRadius`: the first float itself |
| 1 | **box** | first, second, third = three independent half-extents | `GetVolume`: `first × second × third` (plain volume, no `π`); `GetBoundingBox` uses all three as three different axis extents; `GetInnerRadius`: the smallest of the three |
| 2 | **capsule** (cylinder with hemispherical caps) | first = radius, second = half-length of the cylindrical part | `GetVolume`: `r² × (r × 4⁄3π + second × π)` — exactly a hemisphere-plus-cylinder sum rewritten; `GetBoundingBox` uses the first float for the two radial axes and the second for the long axis |
| 3 | **cylinder** (flat caps) | first = radius, second = half-height | `GetVolume`: `r² × second × π` (plain cylinder volume, no sphere-cap term); same bounding-box shape as the capsule |

`PrimitiveType` 4 (editor-only: it never appears in an actual `CPrimitiveHull`, only in editor primitive renderables) shares the capsule/cylinder's radial bounding-box shape in `GetBoundingBox`, but has **no case in `GetVolume`** (it falls to a fixed default volume) — consistent with it being a shape the game never needs to integrate physically, most likely a **cone**. Not confirmed further.

### The hull in the collision grid

`CHull.3`, `.4`, `.5`, `.6` are the **cell rectangle** `xmin, xmax, ymin, ymax` (inclusive) and `CHull.7` the **level**, with the grid of `wld-root.md` (`CCollisionGrid`, extent `E` = `CWorld.17` = 4096). The cell size at level `L` is `E / 2^L`, and a cell index is `floor((coordinate + E/2) / size)` with the **x** range taken from the world x and the **y** range from the world **z** (the plane is x–z, y is up). For all 30,085 hulls:

1. the rectangle is well-formed (`min ≤ max`);
2. the number of **used links** of the collision grid that point at this hull equals `(xmax − xmin + 1) × (ymax − ymin + 1)`, i.e. one link per cell;
3. `CHull.7` is not larger than the grid depth;
4. the cell of the hull's own origin lies inside the rectangle for 29,736 hulls (99 %; the origin of a shape need not be in the middle).

Example from the smallest world: the hull has `CHull.3..6 = 8, 8, 7, 8` at level 4: one cell column and two cell rows, two links, cell size 256; its origin at (0, 0, 0) is the cell (8, 8). So the grid is **derivable from the hulls** (each hull gives its cells) and a loader can rebuild it instead of reading the pools.

## Who carries geometry

Owner classes of the geometry-bearing aspect classes (counts over all 59 worlds; first entries):

| Aspect class | Owners |
|---|---|
| `CModelRenderable` (28,641) | `CStaticModelEntity` 25,383; `SPropEntity` 1,690; `SDoorEntity` 176; pickups (`SShellsItemEntity` 143, `SHealthItemXSEntity` 119, …) |
| `CModelHull` | `CStaticModelEntity` 21,194; `SBarrierFieldEntity` 825 |
| `CPrimitiveHull` | `SPropEntity` 3,160; `CDetectorAreaEntity` 1,077; `SDoorEntity` 1,036; `CStaticModelEntity` 859; pickups |
| `CEffectRenderable` | `CEffectEntity` 2,352; pickups (each pickup has a model, an effect and a primitive hull) |
| `CFlareRenderable` | `SFlareEntity` 2,501; `CDiamondHandEntity` 14 |
| `CCrumbsRenderable` / `CCubeRenderable` / `CVisBoxRenderable` | `SCrumbsEntity` 186 / `CCubeBackgroundEntity` 58 / `SVisBoxEntity` 362 |
| `CEditorPrimitiveRenderable` (58,540) | `SPathMarkerEntity` 21,278; `CPointLightEntity` 11,349; `CFormationMarkerEntity` 5,264; `SLeggedCharacterEntity` 2,923; `SFlareEntity` 2,501; `CEffectEntity` 2,352; `STacticEntity` 1,986; `SStaticSoundEntity` 1,220 |
| `CEditorModelRenderable` (4,632) | `SLeggedCharacterEntity` 1,805; `SBarrierFieldEntity` 825; `SAircraftCharacterEntity` 254; `STurretCharacterEntity` 135; items |

The **editor renderables** (`CEditor…`) show markers (path points, lights, formations) and proxy models of characters in the editor: a box, sphere or similar (`CPrimitiveRenderable.1` = a `CPrimitiveDesc`, `.2` a colour such as `0xFFFF0000`, `.3` 0 or 1). They are 63,000 of the 217,000 aspects.

**Whether the game draws them is now confirmed** (T01-060: `Engine.dll`'s `CEditorPrimitiveRenderable::AddToRendering`, read in full): it is **conditionally** drawn, gated by one bit of a global render-state flag (checked first, before anything else) — so there genuinely is a runtime switch, consistent with these being editor/debug visualisation that a normal play session turns off; a reader that always skips `CEditor…` renderables therefore matches the common case, but is not representing something the engine itself always suppresses unconditionally. Incidentally, the same function shows the colour's alpha byte (`CPrimitiveRenderable.2`) choosing between two different draw-command types when it is not exactly `0xFF` (a "see-through"-style mode versus the normal one) — a confirmed, if narrow, meaning for that alpha byte.

## Reader invariants

1. For a static model entity: take the model from the instance (`CModelInstance.3`); fail on an external reference that is not an `.mdl`.
2. Apply the instance scale per axis; negative scale mirrors the model.
3. Place the entity's aspect subtree from the stored world placements (`wld-root.md`); use the property placement (`CEntityProperties.3`) only as a cross-check or to rebuild a child entity relative to its parent.
4. Register each hull in the collision grid for the cells of its rectangle, or take the stored pools; never walk links that are in the free list.
5. A hull without a body is static; with a body it is dynamic or kinematic (part i).
6. Skip `CEditor…` renderables for the game view.

## Test vector

`dockingbay.wld` (`Content/SeriousSam2/Levels/Multiplayer/DM_DockingBay/`, 33,217 bytes): `CStaticModelEntity` #5 (serial 1) with renderable #62, mechanism #63 and no parent entity; its model is the external `DockingBay.mdl`; renderable #62 → `CModelInstance` #88 (scale (1, 1, 1), model = `DockingBay.mdl`); the hull is `CModelHull` #73 with instance #91 (also `DockingBay.mdl`), body null, mechanism #63; the mechanism has one part; **the hull is the top of the subtree** and the renderable #62 (`CAspect.1` → #73) is its child; the entity's property placement and every placement in the subtree is the identity; `CStaticModelEntity.11` = 1; the hull has `CHull.3..6 = 8, 8, 7, 8` and `CHull.7 = 4` and two used links in the collision grid #74, both pointing at #73.

## Open questions

- The meaning of `CHull.2`, `CHull.8`, `CHull.10` (values 0, 1, −1 and small numbers), `CPrimitiveHull.3` (0.0 mostly; 1.0 and small fractions) and `.4` (0), and of `CMechanism.10`/`.12`/`.8`: **not resolved by T01-060**; no reader of these specific members was found in this reading.
- ~~Names and shapes of the `PrimitiveType` values and the meaning of the three parameters per type.~~ **Answered by T01-060** (see "Primitive descriptors" above): 0 = sphere, 1 = box, 2 = capsule, 3 = cylinder; type 4 is editor-only and most likely a cone, not confirmed further.
- `CStaticModelEntity.12` (values 0 to 9, 0 in 9,082) and `.20` (0, 1, 104 …): not resolved. `CStaticModelProperties.5`/`.6` (−1 or distances such as 170/20): **confirmed** to be the model renderable's cull distance and fade length (T01-060: the property record's two floats at this position are passed straight into `CModelRenderable::SetCullDistance`/`SetFadeLength` during entity setup, in that member order). `.9` (`EntityShadowType` 0 in 24,167, 2 in 1,161, 1 in 55) is the same member `lighting.md` (T01-045) already covers from the data side; no additional code reading was done on it here. `.14` (0 or 1) stays open.
- How the 154 entities with several separate hull roots are laid out, and the 82 mechanisms whose part count differs from the hull count: not attempted (data-only, not revisited this task).
- The embedded `CModelConfiguration` cases (109) and the world's use of `CModelConfiguration` objects in `rf_crsShared`: not attempted.
- ~~Whether the game draws the editor renderables.~~ **Answered by T01-060**: yes, conditionally, gated by a render-state flag bit (see "Who carries geometry" above).
