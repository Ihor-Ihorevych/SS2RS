# `.wld` navigation: nav mesh, path markers, tactics and movers, formations (T01-012e)

- **Roadmap task:** T01-012e (part e of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the nav mesh layout (nodes, links, link chains, the grid), for the path marker graph, the tactic and mover classes and the formation classes and counts (all checked on every world with `tools/re/wld_nav_verify.py`; members come from the files' own type tables, see `wld-navigation-schema.md`); **medium** for the roles of a few mesh members that are only numbers; **low** for how the game searches and follows paths (the algorithm is not in the data). Scope: all 59 distinct worlds.

Prerequisites: `wld-root.md`, `wld-visibility.md` (the quadtree grid layout), `wld-entities.md` (marker entities), `ai.md` (what the AI does with this data: patrol, tactics, foe assignment), `help-digest.md` (the editor's description of the nav mesh).

## Summary

- **The navigation mesh is a set of axis-aligned rectangles on the ground plane** (80,695 nodes in 48 non-empty meshes, 53 meshes in all) **joined by directed links on their shared edges** (335,264 links). Every link has a reverse link; each node owns a chain of links; a 2D grid of the same quadtree layout as the renderable grids finds the node at a point.
- **Path markers** (`SPathMarkerEntity`, 10,639) form **chains** by a next handle (and a back handle): patrol routes. **Tactics** (`STacticEntity`, 993) bind a mover to a start marker; **movers** (`CMC…` classes, 12 kinds) say how a character moves along the markers or around a foe.
- **Formations** (`CFormation` and four subclasses plus `CMarkersFormation`) place groups of characters around **formation markers** (`CFormationMarkerEntity`, 2,538).
- The search itself (A\*, costs, smoothing) and the use of the mesh by characters are **not** in the data (`ai.md`).

## Evidence

`tools/re/wld_nav_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes) over the retail install with all patches:

- **Nav mesh objects:** 53 in 53 worlds (every world but 6), one `CNavMeshRenderable` each pointing at the mesh, and in all 59 worlds the world info entity holds a `CPathFindingService` (its pointer is the mesh in the 53 worlds that have one); 5 meshes are empty.
- **Links:** in all 48 non-empty meshes the link, link search and node lists are consistent: every link has a **reverse link** that is its own reverse and has the opposite direction (335,264 of 335,264); the owner and target nodes **touch on the side of the direction and overlap there** (335,264 of 335,264); the link point (`.10`) and height (`.6`) follow the formulas below (335,264 of 335,264).
- **Nodes:** every node's first link (`.1`) is owned by the node and the chain through `CNMLinkSearch.2` lists exactly the node's links (80,695 of 80,695); 409 nodes have no links.
- **Grid:** one used entry per node (80,695), each named by exactly one entry through `CNavigationGrid.4`; each entry's cells contain the node's rectangle and cover at most 2 × 2 cells.
- **Path markers:** `.2` targets a marker of the world in 8,974 of 8,974 cases and `.4` in 8,377 of 8,377; a `.4` target's `.2` points back in all 8,377.

## The navigation mesh

```
CNavMesh (derives CResource)
  .26  list of CNMNode        .28  list of CNMLinkSearch        .29  list of CNMLink        (the last two have one item per link)
  .30  CNavigationGrid
CNMNode
  .5 .6   INTEGER corner (x, z) of the rectangle, in world units;     .7 .8  width and depth (1 .. 200 or so)
  .4      three floats: the first is a height of the order of the heights of the node's links (a plane height with two slopes is likely, not checked)
  .1      first link of the node (index into the links; -1 without links);     .2 INDEX (equal to the node's own index in 66,441 of 80,695: not determined)
  .9      SWORD: a node index or -1 (80,695 of 80,695: not determined);        .10 UBYTE flags (0 in 65,207 nodes, 4, 130, 26, 126, 22, 8, 162, ...);   .11 UBYTE 0
CNMLink       (one directed link from the owner node to the target node)
  .4  direction 0 = +z, 1 = +x, 2 = -z, 3 = -x          .8  length of the shared edge segment (cells)
  .5 .7  the cell where the segment starts: dir 0 first cell in x (z = .7), dir 2 last cell in x, dir 1 last cell in z (x = .5), dir 3 first cell in z
  .10 Vector3f  the centre of the segment: dir 0 x = .5 + n/2, dir 2 x = .5 + 1 - n/2, dir 1 z = .7 + 1 - n/2, dir 3 z = .7 + n/2 (n = .8); the other coordinate is the shared edge, y = .6
  .6  FLOAT height at the link (always equal to the y of .10);   .3 FLOAT (opposite signs on the two links of a pair in the sample: probably the slope between the nodes)
  .9  INDEX the reverse link (its own reverse, opposite direction)
CNMLinkSearch (item j belongs to link j)
  .1  the TARGET node of link j        .2  the next link of the same OWNER node (-1 at the end of the chain)
```

- **Owner and target.** The owner of link `j` is the target of its reverse link (`CNMLinkSearch[CNMLink[j].9].1`); the target is `CNMLinkSearch[j].1`. A node's links are the chain from `CNMNode.1` through `CNMLinkSearch.2`.
- **Geometry.** Node rectangles are `[x, x + w) × [z, z + d)` in integer world units; two linked nodes touch on a side (`+z`: owner's `z + d` equals the target's `z`, and so on) and the segment of the link is the overlap of the two sides (its length is `.8`). The largest meshes have more than 4,000 nodes.
- **`CNavigationGrid`** derives `CGenericGrid2D` (`wld-root.md`; depth 11 in 38 meshes, 12 in 10; extent 4096) and adds `.4`, a list **indexed by entry**: the node the entry belongs to (-1 for unused entries). An entry holds the cell rectangle and level that contain the node's rectangle (`wld-visibility.md`: slot = `bx + 2*bz`, level `L` has `2^L` cells per axis); the level is the finest one whose cell is at least the larger side in 28,067 nodes and **one level finer** in 52,628; every entry covers at most 2 × 2 cells.
- **Renderables and the service.** `CNavMeshRenderable` (53) is the debug renderable (`.2` → the mesh; its owner is the world info entity); `CPathFindingService` (a structure of each world info entity) holds the pointer to the mesh.
- **Baking.** The mesh is baked in the editor from `CNavMeshMarkerEntity` (54 in 15 worlds) and the chapter markers (`help-digest.md`); the bake data is only the mesh above.

## Path markers, tactics and movers

- **`SPathMarkerEntity`** (10,639 in 47 worlds; record `SPathMarkerProperties`): `.2` the **next marker** (set in 8,974), `.4` a **back link** (8,377; always a marker whose `.2` is this marker), `.333` a name (`PathMarker001`, …), two editor renderables; the record's single handle (`SPathMarkerProperties.1`) is the next marker's record (set exactly when `.2` is set: 10,639 of 10,639) and a colour. 516 markers are the next of two or more markers (paths merge). Following `.2` from markers that are nobody's next gives **2,262 chains** (lengths 1 to 21 and more: 2 (457), 3 (358), 4 (280), 1 (247), 5 (215), …) covering 9,203 markers; the other 1,436 markers lie on **closed loops** (patrols may loop, `ai.md`).
- **`STacticEntity`** (993 in 38 worlds): a mover (`en_hmcCharacterMover`, 990), a start marker (`en_henTacticMarker`, 306: all path markers), a next tactic (113), an auto-off time and distance (−1 = never), a flag "stop when the character has a foe". The record `STacticProperties` repeats them.
- **Movers** (`SMoveCharacter` and its `CMC…` subclasses; `ai.md`): each tactic's mover is stored **twice** (in the entity and in the record: every class count below is 2 × the tactics that use it).

| Mover class | Tactics | Own members (from the schema) |
|---|---|---|
| `CMCMarkersMovingOrdered` | 755 | initial direction (0 to 2), direction change period (−1 to 20), wait on marker min/max (−1 to 150), behaviour on the last marker (0 to 4) |
| `CMCMarkersMovingClosestToFoe` | 36 | test line of sight, must follow path, best marker check period (1 to 10) |
| `CMCDontMove` | 32 | none |
| `CMCCircleAroundFoe` | 32 | inner and outer range, direction change times, initial direction, circling flags |
| `CMCTowardsFoe` | 30 | none |
| `CMCPatternMoving` | 28 | pattern period and maximum, three graph pointers (angles, multiplier) |
| `CMCMarkersMovingRandom` | 24 | wait on marker min/max (0 to 5) |
| `CMCDiamond` | 19 | allowed directions, offsets (min, max, random), dispersion and focusing distance |
| `CMCMellon` | 14 | allowed directions, min/max angle, fade-out flags |
| `CMCStayInArea` | 14 | inner range 15 to 50, outer range 20 to 70 |
| `CMCAwayFromFoe` | 5 | none |
| `CMCAirAttack` | 1 | altitude graphs, correction factor 0.8, freeze distance 20 |

The base class `SMoveCharacter` holds the speed multiplier (0.1 to 4), look-at-foe, an optional period graph, auto-off time and distance, and run-time fields (start time and position, brain handle: empty in the stored state).

## Formations

- **`CFormationMarkerEntity`** (2,538 in 41 worlds): a placement and, in 70 markers, **its own formation** (`.2`: line 31, pie slice 19, rectangle 11, V 8, markers 1); the record holds the same pointer. A few markers attach to a hull and an entity (7).
- **Formation classes** (`CFormation`: five BOOLs and an INDEX 1 to 5; objects over all worlds): `CMarkersFormation` 1,760 (a `CWeightedFormationMarkers` list of up to 21 items, each naming a formation marker entity, its record and a weight 1 or 2; 2,311 items name a marker), `CLineFormation` 638 (length 1.6 to 43, an editor renderable), `CRectFormation` 196 (four sizes, four editor renderables), `CPieSliceFormation` 68 (three floats: 0 to 20, 5 to 62, 30 to 357) and `CVFormation` 68 (two floats). The objects are shared by markers, spawners and records (`ai.md`: spawners may hold a formation handle).
- **Other navigation entities.** `SNavigationalBeaconEntity` (174 in 29 worlds; `en_bDirectionOnly`, `en_bActive` 0; their purpose is not determined here), `CNavMeshMarkerEntity` (54), `SLocatorEntity` (17 in 3 worlds) and `CSpawnMarkerEntity` (177 in 16 worlds; `wld-entities.md`).

## Reader invariants

1. Read the mesh as nodes, link search and links by index; take owner/target as above; a node's links are its chain.
2. Never assume the node count equals the entry count of the grid pool: map entries to nodes through `CNavigationGrid.4`.
3. A link's reverse always exists and has the opposite direction; treat a violation as a corrupt mesh.
4. A path marker's `.2` is the next marker; do not require a chain to end (closed loops exist); a back link is optional.
5. Movers are dispatched on their class; all 12 classes in `wld-navigation-schema.md` are the complete list of the retail data.
6. Unknown nav, marker or formation classes fail loudly.

## Test vector

`Content/SeriousSam2/Levels/01_MDigBo/1_3_Village/1_3_Village.wld`: a `CNavMesh` with 791 nodes, 3,328 links and a grid of depth 11. Node 0: corner (192, −226), size 30 × 24, first link 0. Link 0: direction 0 (+z), cell (192, −203), length 14, height 6.8835, point (199, 6.88, −202), reverse link 1; link 1 (direction 2, from node 2 to node 0) has the point (199, 6.88, −202). `CNMLinkSearch[0]` = (target 2, next link 2). Node 0's grid entry is `(70, 70, 56, 57, level 7)`. The smallest world with markers in the sample (6 smallest worlds): 137 path markers, 98 with a next marker, 88 with a back link, 24 tactics (23 `CMCMarkersMovingOrdered`).

## Open questions

- **Search and following:** read from the binary by T01-155 ([`behavior-navigation-mover.md`](behavior-navigation-mover.md): node lookup, the A\* costs, waypoints at the link centres, the straight-line test, the look-ahead of the navigation mover; medium-low); what remains (open-list ties, the compared heights) is T01-403; how feelers and the mesh interact and every timing: T01-038/039 measurement; `ai.md`.
- **`CNMNode.4`** is read as a plane `height = f0 + f1 x + f2 z` of **world** coordinates (T01-155, medium; the three floats in that order).
- `CNMNode.2`, `.9`, `.10`, `.11` and `CNMLink.3`, and the length of `CNavigationGrid.4` compared with the entry pool, and why the node entry level is one finer than the larger-side rule in 65% of the nodes: measured as far as data decides in [`wld-navigation-leftovers.md`](wld-navigation-leftovers.md) (T01-068); none of the five members is read by the shipped path-finding code (T01-155), so each is very likely bake-time/editor-only, and the length and level questions stay open.
- How formations pick their members and how a spawner uses a formation: answered in [`spawner-spawn-side.md`](spawner-spawn-side.md) section 3 (T01-165). What `SNavigationalBeaconEntity` is for: answered in [`secrets-objectives-beacons.md`](secrets-objectives-beacons.md) section 4 (T01-170). The three tactics without a mover: answered in [`brain-movement.md`](brain-movement.md) section 3.3 step 4 (T01-127) — see [`wld-navigation-leftovers.md`](wld-navigation-leftovers.md) for the cross-reference and which three they are (T01-068).
