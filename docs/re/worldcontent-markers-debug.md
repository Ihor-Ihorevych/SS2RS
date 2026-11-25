# Markers and debug helpers of `Sam2Game.dll`: path markers, visibility boxes, navigation-mesh markers, debug view and dump classes (T01-110, part of T01-026)

- **Roadmap task:** T01-110 (overview [`worldcontent-classes.md`](worldcontent-classes.md); this page covers `CPathMarkerEntity`, `CVisBoxEntity`, `CNavMeshMarkerEntity` with their properties (6 classes) and the 18 `CDebug...` classes)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member tables, the counts, the link relations and the flag checks (`tools/re/worldcontent_classes_verify.py`); **medium** for the hooks, the debug class tree and `ShouldDebug` (one reading each; **nothing was run**); **low** for methods marked `P` or `N`, for the meaning of the hard-coded marker move and for the open questions.

Prerequisites: [`worldcontent-classes.md`](worldcontent-classes.md), `wld-navigation.md` (path marker graph, nav mesh, tactics), `wld-visibility.md` (vis boxes, groups), `ai.md` (what the AI does with the markers).

## Summary

| Class | Entities in the data | What it is |
|---|---|---|
| `CPathMarkerEntity` (`SPathMarkerEntity`) | **10,639** in 47 worlds (all with a record) | a point of a patrol or movement path with a next and a previous link |
| `CVisBoxEntity` (`SVisBoxEntity`) | **362** in 44 worlds | the box that names a visibility group (`wld-visibility.md`) |
| `CNavMeshMarkerEntity` | **54** in 15 worlds | an editor marker for the navigation-mesh bake; tells the world info entity when it changes |
| 18 `CDebug...` classes | 0 | in-game debug dumps and drawings (text, boxes, arrows, spheres), created by the `samDbg...` functions |

1. **A path marker links forward and backward by itself.** `OnUpdate` copies the next marker from the record and sets the next marker's *previous* link to this marker; it clears its own previous link when the previous marker no longer points at it. Of 8,974 marker pairs with a next link, **8,377** have the back link pointing here and 597 do not (a marker has one back link, so a second predecessor is not recorded; script).
2. **A hard-coded move.** At boot the marker with entity id 3,325 in the world whose name contains `1_2_ApproachingVillage` is placed at a fixed horizontal position (section 1).
3. **Vis boxes and nav-mesh markers have no run-time logic of their own**: they are editor objects whose data other systems read. The nav-mesh marker's `OnUpdate` and `OnDelete` call a function of the world info entity (the mesh needs a rebake).
4. **The debug classes are a two-level tree**: `CDebugObject` (4 virtual slots) with 8 dump classes and 8 view classes as direct children; nothing in a world file references them.

## Evidence

- **Export table.** Markers: path 18 + 8, vis box 12 + 10, nav-mesh marker 13 + 8 = **69** (27 named); debug: 18 classes, **161** symbols (36 named).
- **Member tables.** `SPathMarkerEntity` (5), `SPathMarkerProperties` (2), `SVisBoxEntity` (1), `SVisBoxProperties` (3), `CNavMeshMarkerEntity` (1) match the data (script section 2); `CNavMeshMarkerProperties` has 0 members. The debug classes have no member table.
- **Methods.** Read: the path marker's `GetNextMarker`, `GetViewOrigin`, `OnUpdate`, `OnBoot`, `GetSinglePath`; the nav-mesh marker's `OnUpdate`, `OnDelete`; the debug `CreateDumpText`, `GetDumpText`, `SetDescriptionText`, `SetLifeTime`, the constructors, `CDebugDumpConfig::ShouldDebug`. Profiled: `OnCreate` of the three marker classes, `RenderRanges`, the debug `Render*` functions.
- **Class tree of the debug helpers.** The vtable of each class (4 slots) was compared with `CDebugObject`'s: the 8 dump classes override slot 1 (`GetDumpText`) and share slots 2 and 3; the 8 view classes override slot 2 (`Render3DTranslucent`) and some slot 3 (`Render3DOpaque`) and share slot 1. A view class could also sit under a dump class (the vtables of `CDebugViewP` and `CDebugV3f` do not decide it); the **flat tree is medium**.
- **Creators** of the debug classes: the callers of `CDebugObject`'s constructor (the `samDbg...` functions, section 4).
- **Not checked:** nothing was run; the nav mesh bake and the world info entity's function were not read.

## Layout / rules

### 1. `CPathMarkerEntity`

**Members** (5): `en_hre` (the marker's editor primitive), `en_hreDir` (a second one that shows the direction), `en_henNextMarker` (data `.2`: set 8,974, null 1,665; **all 8,974 targets are path markers**), `en_henPrevMarker` (`.4`: set 8,377), `en_strName` (`.333`, "PathMarker001" ...; the most common names are `PathMarker001` 737, `PathMarker002` 579). Record (2): `ep_hepNextMarker` "Next marker" (set 8,974, equals the entity's link), `ep_colColor` "Color" (215 distinct values). No member is replicated; `GetNetBehavior` is inherited `NEB_NONE`.

**Behaviour.**

| Function | Rule (basis) |
|---|---|
| `OnCreate` | builds the two editor primitives and places the marker (P, 1,236 instructions) |
| `OnUpdate(record)` | next = the record's next-marker entity if it is a path marker; if next exists, set **its previous link to this marker**; if the previous link is set but the previous marker's next is not this marker, clear the previous link (R) |
| `GetNextMarker` | the next link (R) |
| `GetSinglePath(array)` | collect the chain: follow next links from this marker until a marker is seen twice or the chain ends (loop safe), then follow **previous links from this marker** the same way; the result is the whole path through this marker (R) |
| `GetViewOrigin` | the marker's placement (R) |
| `GetBoundingBox`, `GetAbsPlacement`, `SetAbsPlacement` | placement of the marker's primitive (P) |
| `RenderRanges` | draws the path with arrows in the editor (P) |
| `OnBoot` | **hard-coded fix** (below) |

**The hard-coded fix (R, medium).** If the entity id (`CEntity.1`) is **3,325 (0xCFD)** *and* the world's name contains `1_2_ApproachingVillage`, `OnBoot` reads the current placement, replaces its horizontal position with **x = 59.5, z = −19.75** (the height is kept) and sets the placement. The record's own position in that world is (59.113, 9.183, −20.108), so the code moves the marker by about **(+0.39, 0, +0.36) m**; the script reports the marker in that world (and shows that the id 3,325 is a different marker, "DropShip02Marker", in `1_4_ApproachingCity`, which the name test excludes). Why the fix exists is unknown (it looks like a late correction of a path that a patch could not re-bake; **low**, T01-182).

### 2. `CVisBoxEntity`

Members: `en_hre` (the `CVisBoxRenderable`, `wld-visibility.md`). Record (3, resolving two numeric members of `wld-visibility.md`): `ep_vSize` "Size" (`.1`, the edge lengths), `ep_fCullDistance` "Cull distance" (`.2`, 10..3.0e38; **3.0e38 means no limit**), `ep_bOcclusionTest` "Occlusion test" (`.3`: **1 in 347 boxes, 0 in 15**; the data's flag). Defaults (record constructor): size 10 × 10 × 10, cull distance 3.0e38, occlusion test 1. `OnCreate` builds the renderable at the record's placement (P); `OnDelete` deletes it; `GetAbsPlacement`, `SetAbsPlacement`, `GetBoundingBox` use the box (P). 362 entities; no replication; nothing else runs. **Medium** for the names (declared by the DLL), **high** for the counts.

### 3. `CNavMeshMarkerEntity`

Member: `en_hre` (an editor primitive, drawn when selected). The record has no member. `OnCreate` builds the primitive (P); **`OnUpdate` and `OnDelete` each call a function of the world info entity** (R: one virtual call, `samGetWorldInfo`; the function is the navigation mesh's "markers changed" notification, **P**). 54 placed in 15 worlds; the bake that uses them is the editor's (`wld-navigation.md`). Hooks: `OnCreate`, `OnUpdate`, `OnDelete`, `SetAbsPlacement`, `GetAbsPlacement`, `GetBoundingBox`.

### 4. The debug helpers

**Tree (medium).** `CDebugObject` (4 virtual slots: data type, `GetDumpText`, `Render3DTranslucent`, `Render3DOpaque`; 12 named methods) with direct children:

| Kind | Classes | What they show |
|---|---|---|
| dump (override `GetDumpText`) | `CDebugBool` ("True"/"False"), `CDebugFloat`, `CDebugIndex`, `CDebugString`, `CDebugAngle`, `CDebugAngle3f`, `CDebugV3f` ("%1 %2 %3"), `CDebugQV` ("Pos: %1 %2 %3 Euler: %1 %2 %3") | one value as text |
| view (override the 3D render hooks) | `CDebugViewP` (a point or sphere; `SetRadius`), `CDebugViewPV` (point and vector), `CDebugViewPR` (point and rotation), `CDebugViewQV` (placement as an arrow), `CDebugViewBB` (a bounding box), `CDebugViewAngleH`, `CDebugViewAngleP` (a horizontal and a pitch angle), `CDebugViewEntityPath` (the path an entity walked, with a start time) | a drawing in the 3D view, optionally with the text of the value |
| `CDebugDumpConfig` (not a `CDebugObject`; 9 symbols) | `ShouldDebug`, `DumpThinkersOn`, `DumpThinkersOff` | which entities to dump |

**`CDebugObject`** (R): constructor defaults: empty name and description texts, **life time 3.0e38** (never expires), start time = now, flag 1, two colors, **ink 0xffffffff and paper 0x80000000** (by the setter names); `SetFlags`, `SetInkColor`, `SetPaperColor`, `SetLifeTime`, `SetDescriptionText`, `DisplayTextIn3DView` are setters; `CreateDumpText` formats `%1: %2\n` or, when a class name is attached, `(%1) %2: %3\n` (class name, name, value); `Render2D` draws the dump text lines on the 2D overlay with the current font and the two colors; `RenderTextIn3DView` draws the text at a world point. **`CDebugDumpConfig::ShouldDebug(entity)`** (R): a mode member (0, 1 or 2; some modes are host-only) is tested first (P); then it asks the entity (virtual `AddDebugEntites`, slot 100) for the entities to be dumped together with it (itself, its brain, its behaviours: the class names `CPuppetEntity`, `CBrainEntity`, `CFoeManagerEntity`, `CBehaviorEntity` are looked up once) and returns true when one of them is the configured target entity or, when no target is set, is derived from the configured class name. `DumpThinkersOn` writes, via the debug string function, one line per entity that has a think scheduled (class name and id; P); `DumpThinkersOff` clears it.

**Creation.** Every instance is made by a `samDbg...` function (the callers of `CDebugObject`'s constructor): `samDbgI` (index), `samDbgB` (bool), `samDbgF` (float), `samDbgStr` (string), `samDbgV3f`, `samDbgEul` and `samDbgEul3f` (angles), `samDbgQV`, `samDbgViewP`, `samDbgViewSphere`, `samDbgViewPV`, `samDbgViewPR`, `samDbgViewQV`, `samDbgViewBB`, `samDbgViewAngleH`, `samDbgViewAngleP` and the entity path view. The AI classes call them through the debug-dump switch (`ShouldDebug`); no placed entity is a debug object. Nothing of this is saved or replicated.

## Invariants

1. 10,639 path markers have records; each next link (8,974) targets a path marker; 8,377 back links are the inverse of a next link; **597 next links have a target whose back link names another marker** (script section 3).
2. 362 vis boxes, 44 worlds; 347 have the occlusion test; every box's cull distance is 10..3.0e38.
3. 54 navigation-mesh markers in 15 worlds; no record member.
4. No debug class and no movement of a placed marker is stored in a file; the only code that moves a marker at boot is the fix of section 1.

## Open questions

1. The consumers of `GetSinglePath` and `GetNextMarker` (patrol, formation, tactic), the hard-coded fix and the nav-mesh marker's notification: **T01-182**.
2. The `samDbg...` functions, `CDebugDumpConfig` in full (modes, target entity, thinker dump) and the 2D overlay: **T01-183**.

## Test vectors

1. **Path.** Markers A → B → C (next links) with back links B → A, C → B: `GetSinglePath(B)` returns B, C then A (the forward chain from B, then the backward chain from B, each stopping at a repeat).
2. **Update.** Setting A's next to B sets B's previous to A; if B's next is not A, a stale previous link of A is cleared.
3. **Fix.** In `1_2_approachingvillage.wld` the marker with serial 3,325 (record position 59.113, 9.183, −20.108) is at x 59.5, y 9.183, z −19.75 after boot.
4. **Counts** (script): 10,639 path markers, 362 vis boxes, 54 nav-mesh markers.
