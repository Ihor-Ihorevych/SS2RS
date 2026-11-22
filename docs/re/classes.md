# Class hierarchy of the engine and the game (T01-021)

- **Roadmap task:** T01-021
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list, the inheritance and the member counts of every class that is serialized in the data (1,043 classes, 8,063 files); **medium** for which module defines a class (a name string is present in that module's binary); **not delivered**: class **sizes**, virtual-function tables and the classes that never appear in data (see "What RTTI would have given").

Prerequisites: `ctsemeta-types.md` (type tables). The full catalogue is **`classes-catalog.md`** (generated, 1,043 lines: the whole hierarchy as an indented tree).

## Summary

The roadmap asked for the class hierarchy to be recovered from the MSVC run-time type information (RTTI) of the three game DLLs. **That route is closed**: RTTI is compiled out. A search of every module for the RTTI type-descriptor marker finds only the exception classes and `type_info` itself (`Engine.dll`: 5 names, `Core.dll`: 4, `Sam2Game.dll`: 4, `EngineGui.dll`: 4, `Shaders.dll`: 2, `ProcRender.dll`: 2, `GfxD3D.dll`: 3), no game class.

The engine does not need RTTI because it has something better: its own **meta-data system** (`help-digest.md`: "data knows its own structure"), and every CTSEMETA file carries the **definitions of the classes it uses, with base class and member lists** (`ctsemeta-types.md`). Merging those definitions over the whole install gives the hierarchy of **every class that can be saved**, which is what readers and writers of data need. That is delivered here.

## Evidence

- `tools/re/class_catalog.py` reads only the type tables of the data files (format-8 files of all archives, patches override). Reproduces the numbers and writes `classes-catalog.md`.
- The **module** letters in the catalogue (E Engine, C Core, G Sam2Game, U EngineGui, P ProcRender, S Shaders, D GfxD3D, O GfxOGL, K the EditKit editor DLLs) record whether the class name occurs as a string in that module's file. They were measured locally with a throw-away script; per `CLEANROOM.md` scripts that read the original binaries are not kept in the repository, and only the resulting names go into the catalogue. The method is: take every run of letters, digits and underscores of at least three characters in the file and look the class name up.
- No disassembly, decompiler output or address was used.

## Numbers

| | |
|---|---|
| Classes serialized in data | **1,043** (types that have members; primitives, containers, pointers excluded) |
| Files read | 8,063 (format 8, patches merged) |
| Root classes (no base) | 275 (value structs such as `Vector3f`, `Box3f`, `Time`, and helper classes) |
| Depth | at most 7 levels; classes per level: 275, 178, 137, 159, 125, 66, 103 |
| Members | 4,865 own members in all |
| `DTTY` flags word | 5 for every class |
| Classes whose definition differs between files | **37** (see below); only one changes its base class |

### Prefixes

| Prefix | Classes | Where they live |
|---|---|---|
| `C…` | 755 | engine and game: 357 only in `Sam2Game`, 150 only in `Engine`, 51 in both, 38 only in editor modules, 33 only in `ProcRender` |
| `S…` | 261 | **all 261 only in `Sam2Game`**: the game's own classes (entities, parameters, behaviors, HUD) |
| other | 27 | math and helper value types (`Vector3f`, `Matrix44f`, `Quaternion4f`, `QuatVect`, `Box3f`, `Time`, `GFXHANDLE`) and editor transfer types |

The `S` classes derive from `S` classes (210) or from a `C` base (32), and 19 are roots; every other layer of the game builds on the engine's `C` classes.

### Where classes are defined (module of the name string)

| Module(s) | Classes | What they are |
|---|---|---|
| `Sam2Game` only | 618 | the game: entities, parameters, behavior, HUD, menus |
| `Engine` only | 154 | the engine: resources, aspects, renderables, macro language, animation, mechanisms |
| `Engine` + `Sam2Game` | 51 | engine classes the game extends or uses directly |
| editor modules only | 48 | `Edit…`, `Packed…`, `…Map`, transfer types: **editor-only**, never in a shipped game file except as embedded tool data |
| `Engine` + editor | 42 | engine classes the editor edits |
| `ProcRender` only | 33 | particle effects, text effects (`CTextEffect`, `CLetterEffect`, `CParticleEffect`, `CEmitterShape`, …) |
| the rest | 97 | spread over `Core`, `EngineGui`, `Shaders` and combinations |

(Only 2 of 1,043 class names occur in no module.)

## The main families

`CObject` is the **root of 474 of the 1,043 classes** (14 direct children). The big subtrees:

| Root (module) | Subtree | Notes |
|---|---|---|
| `CObject` → `CSmartObject` → `CResource` | 127 | every resource type: textures, meshes, models, animation sets, skeletons, worlds, parameters, schemes, effects (54 direct children) |
| `CObject` → `CEntity` → `CBaseEntity` (G) → `SBaseEntity` | 109 / 95 | **entities**: every placed object of a world; `SBaseEntity` has 33 children (items, puppets, spawners, markers, …). Each entity class has a matching **properties** class in the parallel tree `CEntityProperties` → `CBaseProperties` (108) → `SBaseProperties` (92), which holds the editable values (`wld-root.md`, `ep.md`) |
| `CMacroCodeSegment` (E) | 65 | the macro language classes (statements, expressions, handlers; `CMacroStatement` alone has 20 subclasses) (`help-digest.md`, T01-012g) |
| `CMenuGadgetLayout` (G) | 45 | menu gadgets (T01-036) |
| `CAspect` (E/G/P/K) | 39 | the aspects of an entity: renderable, collision, physics, sound; `CRenderable` has 18 children |
| `CBehaviorAction` (G) + `CBAValidityTerm` | 28 + 8 | AI behavior actions and conditions (`misc-types.md`, `.cb`) |
| `CAnimEvent` / `CAnimEventHandler` | 24 + 5 | animation events (`ans.md`) |
| `SHudElementParams` (G) | 47 | HUD elements (`rsc-ctl.md`) |
| `SBaseWeaponParams` (G) | 19 | weapon parameters (T01-028) |
| `CJointTemplate` + `CJoint` | 13 + 9 | physics joints (`.mch`, T01-057) |
| `CGraph` (E/G/U/P) | 9 | graph classes shared by effects, shaders and databases (`rsc-ctl.md`) |

Large classes (members in the current definition): `SPuppetEntity` 242, `CNetricsaHudElementParams` 96, `CGlobalGameParams` 80, `CWorldInfoEntity` 77, `SLeggedPuppetEntity` 61, `SSpawnerEntity` 58, `CTerrainShaderArgs` 56, `CStaticModelEntity` 51.

## Classes that exist in two versions

37 classes have more than one definition in the install. Two different causes show in the archives:

- **Entity classes grow with patches.** `CWorldInfoEntity` has 67 members in the base archives (`All_PC_02/03`), 70 in `Patch_02_068`, 76 and 77 in `Patch_02_090` and 77 in `Patch_02_100`; `SPuppetEntity` 234 in the base archives and 242 from `Patch_02_090` on; `SSpawnerEntity` 33 (base) to 58 and 59; `CPointLightProperties` 14 to 15. A world saved by the base tools and one saved after the patches therefore differ in their class tables.
- **Resource classes exist in two forms inside the base archives themselves**: `CMesh` has 2 members in 1,895 files of `All_PC_02` and 3 members in 23 files of the same archive (similar for `CAnimSet` 2/3, `CModelConfiguration` 11/12, `CFont` 7/8, `CResource` 4/5): files exported by different tool versions.

`CPieSliceRenderable` is the one class whose base changed (`CRenderable` or `CEditorPrimitiveRenderable`). A reader **must take the class definition from each file's own type table, never from a built-in table**: the file is the authority (`ctsemeta-types.md`), and members cannot be skipped without knowing their types.

## What RTTI would have given and what is missing

- **Sizes** (`sizeof`): not in any data file and not in the string tables. Needed only to reimplement binary layouts of in-memory structures, which we do not (our structures are our own). Open: T01-063.
- **Virtual functions** and **methods** (what the game does with the data): behavior comes from specs by observation (T01-024 and later), not from class dumps.
- **Classes that are never serialized** (helpers inside the engine, the 660 string matches in `Engine.dll` of class-like names that are not in the catalogue, the `Core` threads, tasks, the file system): their names are not in data, so they are not listed. They carry no layout that a data reader needs.

## Reader invariants

1. Treat the catalogue as **documentation**, not as a table to compile in: the per-file type table decides member lists.
2. `CObject` is the root of everything saved as an object; a class without a base is a value struct stored by value.
3. Entity classes and their properties classes come in pairs; the `.wld` and `.ep` specs say how they are linked.
4. Editor-only classes (modules K only) appear in shipped data only as embedded editor data (mesh sources, destruction tools); a runtime may skip those members by type-table size.

## Open questions

1. Sizes and vtable layouts: T01-063 (needs binary analysis with the Ghidra conventions of `conventions.md`).
2. Whether names that exist in the DLL strings but not in data (about 600 in `Engine.dll`, about 2,200 `C…`/`S…`-shaped strings in `Sam2Game.dll`) are classes without a saved form, enumerations, functions or log text; the catalogue lists only the first group that data confirms.
3. The mod DLL `InSamnity2Game.dll` adds classes: none of them appears in data of the base game, so it is out of scope until T13 (mods).

## Test vectors

`classes-catalog.md` lines to compare with `class_catalog.py` output: `CAnimEnvelope` (5 members, base none) with the three children `CFloatAnimEnvelope`, `CQuatAnimEnvelope`, `CVec3LenAnimEnvelope`; `CAIMomentHandler` (1 member, 105 files, in `.cb` and `.wld`); `SBaseEntity` has 33 direct children; the total is 1,043 classes in 8,063 files.
