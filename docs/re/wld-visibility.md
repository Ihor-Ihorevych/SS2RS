# `.wld` visibility: vis boxes, vis groups and the renderable grids (T01-012c)

- **Roadmap task:** T01-012c (part c of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the vis box entity, its property record and renderable, the group membership relation, the active list and the grid structure (checked on all 59 worlds, `tools/re/wld_vis_verify.py`); **medium** for the roles of the two numeric vis box members and of the group flag (read from value ranges only); **low** for how the engine uses groups at run time (nothing in the data says when a group is shown). Scope: all 59 distinct worlds of the retail install with patches.

Prerequisites: `wld-root.md` (entities, aspects, domains and the grid pool layout), `wld-geometry.md`.

## Summary

- **There are no portals, sectors, zones or occlusion data in the worlds.** A scan of all 484 classes in the type tables of the 59 worlds finds exactly three visibility classes: `SVisBoxEntity`, `CVisBoxRenderable` and `CVisGroupRenderable`. (`CPortalEntity`, 54 instances in 22 worlds, is a model entity with a mechanism and animation members: a teleporter-like object, not visibility; `CChapterInfoProperties.ep_bCoopPortalMarker` is a cooperative-play marker.)
- **A vis box is a hand-placed box that names a group of renderables.** 362 vis boxes in 44 worlds (15 worlds have none). Membership is **explicit**: a renderable's `CRenderable.2` points to its group's box and the box lists its members in `CVisGroupRenderable.1`; the two relations are exact inverses in every world. 71,086 of the 99,187 renderables belong to a group.
- **Spatial culling uses 2D quadtree grids** (`CRenderableGrid`, `CLightGrid`): every renderable has a rectangle in the X-Z plane stored as grid cells. The vis boxes live in their own grid (`CRenderingDomain.6`); all other renderables are in the second grid (`.7`). This spec gives the exact cell layout (the pool layout is in `wld-root.md`).
- What the engine does with the groups at run time (show or hide by distance, camera position or script) is not stored and is not specified here.

## Evidence

`tools/re/wld_vis_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes for all worlds) over the retail install with all patches, 59 worlds:

- **Every vis box entity owns exactly one `CVisBoxRenderable`** (`SVisBoxEntity.1`) whose `CAspect.5` points back: 362 of 362. Each has an `SVisBoxProperties` record in the extra section, and the record's three members equal the renderable's three: 362 of 362. All 362 are named (`CEntityProperties.4`, for example `Village_Approach_01`).
- **`CRenderable.2` and `CVisGroupRenderable.1` are exact inverses** in all 59 worlds (each renderable in at most one group; every set `CRenderable.2` points at a `CVisBoxRenderable`).
- **Grid structure:** for all 118 renderable grids and 59 light grids, every cell of every entry has exactly one link at the node reached by the walk below, and the used links are exactly those (one per entry and cell), the used nodes exactly the nodes on those paths. All 99,122 renderable entries and 11,438 light entries cover at most 2 cells per axis.
- **`CRenderable.1` is the index of the renderable's grid entry**: a used entry of its grid in all 118 grids, unique within a grid (it equals the list position in 113 of 118 grids).
- **Vis box rectangles** equal the rule below in 360 of 362 boxes (the other two are not analysed).

## The vis box

```
SVisBoxEntity           (base SBaseEntity -> CBaseEntity; 362 over all worlds)
  SVisBoxEntity.1  -> CVisBoxRenderable
SVisBoxProperties       (extra record; base CEntityProperties + CBaseProperties)
  CEntityProperties.4    string: the box's name
  SVisBoxProperties.1    Vector3f: size of the box (full edge lengths)
  SVisBoxProperties.2    float: 3.0e38 in 153 boxes, otherwise 70 .. 400 (a distance?)
  SVisBoxProperties.3    int: 1 in 347 boxes, 0 in 15
CVisBoxRenderable       (CAspect -> CRenderable -> CVisGroupRenderable -> CVisBoxRenderable)
  CVisGroupRenderable.1  list of -> renderables: the members of the group
  CVisBoxRenderable.1/.2/.3   copies of SVisBoxProperties.1/.2/.3
  CRenderable.2          -> the box this box is itself a member of (nesting; below)
  CAspect.6              AspectFlags: 0x80005 in all 362
```

- **Placement and size.** The box is centred on the renderable's world placement (`CAspect.7`) with edge lengths `.1`; 311 of 362 have an identity rotation; the other 51 are rotated. The members' origins lie inside their box in 71,059 of 71,086 cases (27 outside, in a few worlds); the origin test is `|local position| <= size/2` per axis with the box's rotation.
- **`.2`** is 3.0e38 (effectively "no limit") in 153 boxes and a round number between 70 and 400 in the rest (100: 20, 200: 19, 120: 10, 170: 8, 150: 8, 400: 8, …): by its range a **distance**; its use is not stated in the data. **`.3`** is a flag, 1 in almost all.
- **Nesting.** 52 vis boxes are themselves members of another box (`CRenderable.2` of a `CVisBoxRenderable` is set): a nesting depth of exactly 1 in the data (310 top-level boxes, 52 nested, no cycles).
- **Membership is not spatial.** 4,254 renderables that are in no group have their origin inside some box, so a group is the list stored in the file, not a query a reader can repeat. 71,086 members: editor primitives 45,790, models 18,069, editor model proxies 3,801, effects 1,814, flares 1,480, and so on; their owners are mostly path markers, static models, lights, formation markers and characters.
- **Group sizes** range from under 50 renderables (117 groups) to 400 and more (56 groups).
- **Entity visibility id.** Every property record of the newer class variant has `CBaseProperties.ep_ulEntityIDVis` equal to the entity's own serial (`CEntity.1`) and to `CEntityProperties.8`: 51,703 of 51,704 records (the exception is one force field entity whose value is 1). 17,473 records come from an older class variant without the member. The member is therefore **not** the id of the entity's group; groups are only given by `CRenderable.2`.

## The active list `CRenderingDomain.8`

`wld-root.md` left `CRenderingDomain.8` (a subset of all renderables) open. It is: **all renderables that are in no group, except the haze effect renderable** (which has its own list `.9`). Checked in all 59 worlds: `.8` contains no group member, is a subset of the ungrouped renderables, and the ungrouped renderables missing from it are exactly the 65 haze renderables (in 14 worlds `.8` equals the full ungrouped set). Consequence: the renderables the engine draws without a group decision are in `.8`; everything grouped (71,086) is outside it, including the 52 nested boxes.

## The grids: cells and entries

All renderable grids and the light grid are the same structure (`CGenericGrid2D`, `wld-root.md`): `.7` the depth (11 or 12) and `.8` the extent (4096.0 in all 177 grids). This part adds the geometry:

```
level L has 2^L x 2^L cells; cell size = extent / 2^L        (level 0 = the whole grid; L <= depth)
the grid covers x and z in [-extent/2, +extent/2)             (world translation; y is ignored)
cell index along an axis: floor((p + extent/2) / cell size)

CGridEntry2D = (x0, x1, z0, z1, L):  the object covers cells x0..x1 and z0..z1 at level L   (x1-x0 <= 1, z1-z0 <= 1)
CGridLink2D  = (entry index, next link)          CGridNode2D = (4 child nodes, first link)

for each cell (cx, cz) of an entry: start at node 0; for s = 1..L:
    slot = ((cx >> (L-s)) & 1) + 2 * ((cz >> (L-s)) & 1);   node = children[slot]
  the link list of that node holds exactly one link for this entry.
```

- **The level of an entry** is the finest level whose cell size is at least the larger side of the object's rectangle, so it spans one or two cells per axis. For a vis box the rectangle is the rotated box (centre +- the projected half sizes in x and z): 360 of 362 boxes. For the other renderables and lights the object's bounding rectangle is not stored, but the same bound holds (all 110,560 entries cover at most 2 x 2 cells), and the renderable's origin lies inside its rectangle in 98,817 of 99,122 renderables (the origin of a model can lie outside its bounds).
- **Which grid holds what:** `CRenderingDomain.6` holds all vis boxes and nothing else (44 worlds) or is empty (the 15 worlds without boxes); `.7` holds all other renderables of the world except the haze effect (118 grids in 59 worlds). The light grid (`.5`) holds the light sources in the same layout (11,438 entries in total).
- The entry index of a renderable is its `CRenderable.1`.

## Reader invariants

1. There is no portal or sector system to implement. Visibility data is only: vis boxes and groups, the renderable grids, the active list.
2. Take group membership from `CRenderable.2` / `CVisGroupRenderable.1` (inverse of each other); do not infer it from positions.
3. A box can be a member of one other box (depth 1 in the data); expect no cycles and fail on one.
4. Treat `CRenderingDomain.8` as "ungrouped renderables"; when `.8` and the group relation disagree the file is not a retail world.
5. A grid reader walks the quadtree as above; a missing link for a cell, or a level above the grid depth, is an error.
6. Do not interpret `SVisBoxProperties.2` and `.3` or the render-time use of groups before the renderer spec says what they do.

## Test vector

`Content/SeriousSam2/Levels/01_MDigBo/1_3_Village/1_3_Village.wld`: 7 vis boxes, all top-level. The first (entity serial 1976, renderable object 2986, name `Village_Approach_01`): size (33.404, 36.876, 88.462) at (126.471, 21.022, −95.277) with an identity rotation, `.2` = 3.0e38, `.3` = 1, one member (an editor primitive). The seven boxes have member counts 1, 158, 142, 59, 14, 199 and 164 (737 members); 663 members are in a group whose `.3` is 0 and 74 in groups whose `.3` is 1; the members are exactly the renderables outside `CRenderingDomain.8` (157 are in `.8`, 737 are in groups and 1 is the haze effect renderable, 895 in all). The vis grid (object 4548): depth 12, extent 4096, 16 entry slots of which 7 are used, entries `(16,17,14,15,5)`, `(34,35,27,28,6)`, `(35,36,27,28,6)`, `(17,17,14,15,5)`, `(17,18,14,15,5)`, `(35,36,28,29,6)`, `(34,35,28,29,6)` in the order of the box list; 26 used links (one per entry cell), 25 used nodes of 32.

## Open questions

- What a group does at run time: the meaning of `SVisBoxProperties.2` (a distance?) and `.3`, and whether groups are shown or hidden by position, by script or by distance (T04 renderer, T01-030 scripting).
- How the level of non-box entries is chosen exactly (the bounding rectangles of models and lights are not stored), and the two vis boxes whose entry differs from the rule.
- Whether the macro language can address a group (nothing visibility-related appears among the classes; groups are addressed by entity serial at most).
- `AspectFlags` bit meanings (shared with `wld-root.md`).
