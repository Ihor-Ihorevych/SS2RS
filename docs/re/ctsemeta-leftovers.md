# CTSEMETA leftovers: old format versions 4–7, `INFO[0]`, `RFIL.flags`, `EXOB.flags`, `EDTY`, extra records (T01-051)

- **Roadmap task:** T01-051
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the old-format differences, the `EDTY` rule, the shader and entity-class codes (each checked on every file, scripts below); **medium** for the roles of the `EXOB` code values (the mapping is exact, what the numbers mean inside the game is not in the data); **low / open** for `INFO[0]` and `RFIL.flags` (a reader never needs them: only correlations are known).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`. Generated companion: `ctsemeta-entity-codes.md`. Reproduce with `tools/re/ctse_leftovers_verify.py` (read-only, `SS2_DATA_DIR`).

## Summary

1. **Old versions 4–7 are the version-8 frame with two small differences**: versions 4 to 7 have a 4-word `INFO` (no `INFO[0]`), and version 4 also puts a member-number word in front of every struct member. With those two rules the 155 old `.tex` files (all development-build leftovers inside patches 2.090 and 2.100) decode with the unchanged version-8 reader, each exactly to the footer.
2. **`EDTY` is the type table of the extra records**: the pairs `(object id, type id)` of the records after `EDOB`, in the same order. Equal counts in all 8,063 files.
3. **`EXOB.flags` is a class code**: `1` for ordinary resources; `1000`–`1070` identify the concrete shader class; for `CEntityClass` entries it is a stable per-entity-class number (94 codes for 143 classes, identical in all 59 worlds).
4. **`RFIL.flags = 0x00040000`** is set on references that come from the data-driven parameter layer (weapon, player, HUD and character databases; observed, the engine meaning is not in the data); a reader can ignore it.
5. **`INFO[0]`** is 0 in every file that has extra records and 1 only in files without them; its meaning is not in the data, a reader can ignore it.

## Evidence

`tools/re/ctse_leftovers_verify.py` over the retail install with all patches: 8,063 version-8 files (the newest version-8 copy of every path) plus all 155 files of versions 4–7 read from the archives. Of the 8,141 paths of the install that hold a container, the game reads 8,016 version-8 files and 125 old-version files: for 47 paths a newer old-version file overrides the version-8 copy, which the 8,063 still include. Nothing from binaries except the name of one engine switch mentioned in the `INFO[0]` notes (`conventions.md` §5).

## Old format versions 4–7 (155 files)

All 155 are `.tex` files with the `CResourceFile` root and one `CStaticTexture` (154) or `CColorTexture` (1) object; none has extra records and only one has a file reference (`Textures/Generic/effects/Noise_02.tex` refers to one texture, flags 0). They sit in `Patch_02_090_593613.gro` (114: version 4 ×20, 5 ×4, 6 ×69, 7 ×17) and `Patch_02_100_800000.gro` (45: version 6 ×44, 7 ×1); **125 of them are the file the game actually reads** (the effective copy; the other 30 are overridden by a later version-8 file).

| Version | Files | Writer builds seen (examples) |
|---:|---:|---|
| 4 | 20 | `2.005.00:24818` to `:28882` (`Development`, `2004-05-dev`, `2004-06-dev`) |
| 5 | 4 | `2.005.00:30678` to `:31940` (`2004-06-dev`) |
| 6 | 113 | `2.016.00:32799` to `:37656` (`2004-10-dev`), a few `2.023.00:38026` to `:39339` (`2005-02/03-dev`) |
| 7 | 18 | `2.023.00:39826` to `:40514`, `2.027.00:41340` to `:41637`, `2.029.00:43908` to `:44905` (`2005-03/05-dev`) |
| 8 | 9,269 | the gold build `2.062.00:57284` and later |

**Differences from version 8 (all that was found, every file checked):**

1. **`INFO` has four words, not five.** The fields are those of version-8 `INFO[1..4]` (counts of `RFIL`, `IDNT`, types, objects), so a version-8 reader that inserts one word before them (any value) reads the rest unchanged. The word `INFO[0]` was added in version 8.
2. **Version 4 only: every member entry of a struct (`STMB` block) starts with one extra `u32`, a member number**, before the name length: `u32 number · u32 len · name · u32 typeId`. The numbers are small, unique inside the class and **not in member order** (for example `CResourceFile`: 1, 2, 4; `CResource`: 1, 3, 2; `CBaseTexture`: 1, 2, 4, 5, 6, 17, 18, 8, 9, 10, 14). A reader drops the number and keeps the member order of the file; object data follows that order, exactly as in version 8 (checked: all 20 files decode to the footer after dropping the numbers).
3. **Names of two members differ**: in version 4 the first two members of `CResourceFile` still have their source names (`rf_iNextFreeID`, `rf_iDefaultID`), from version 5 on they are the numeric names `1`, `2` (the "unnamed member" form used in version 8, `ctsemeta-types.md`).

Everything else is identical: the section order, `RFIL`/`IDNT`/`EXTY` entries (`RFIL` entry = index, flags, length, path), the type-table blocks, `EXOB`/`OBTY`/`EDTY` entry sizes, the `OBJS` value rules and the `EDOB` + `METAEND ` tail. The `EXTY` table of versions 5–7 lists `CResourceCustomData` (and, in version 7, `UBYTE`) as external types.

**Reader rule.** Accept versions 4 to 8. For 4 to 7 read `INFO` as four words; for 4 also skip the leading member number. Nothing else changes. (A loader that wants to ignore these files entirely loses 125 textures of the shipped game (level, model, effect and menu textures): they are not optional.) `tex.md` describes what the objects mean.

## `INFO[0]`

Values and counts over the 8,063 version-8 files: **0** in 769 files, **1** in 7,294 files. Observations:

| `INFO[0]` | Files | With extra records after `EDOB` |
|---:|---:|---:|
| 0 | 769 | 119 (all 59 worlds, 29 `.mdl`, 18 `.bmf`, 8 `.fnt`, 4 `.ans`, 1 `.skl`) |
| 1 | 7,294 | 0 |

- **Every file that has extra records has `INFO[0] = 0`**, and so has every file whose type table defines the editor classes; the converse is false (650 files with 0 have none: 462 `.tex`, 108 `.rsc`, 61 `.ep`, 10 `.tfx`, …).
- It is **not** a function of the writer build: the gold build number 57284 wrote 1 in all 6,529 of its files, but builds 587968 to 593613 wrote both values (for example build 588273: 261 files with 1 and 21 with 0), and every file of build 729441 and later has 0.
- It is **not** a function of the extension or of the version string alone.
- The core module has a switch named `md_bSaveDiscardsEditData` (a name only; `console-config.md`): "save without edit data" would explain why files without extra records exist, but the data does not tie the word to it.

A reader must ignore `INFO[0]`; a writer may store 1 (no edit data) or 0.

## `RFIL.flags`

`flags` is `0` or `0x00040000` (bit 18): **37,746 clear and 2,044 set** over the 8,063 version-8 files (5 %); counting every archive copy, including the overridden ones, the totals are 49,376 and 3,743 (`ctsemeta-container.md`). Facts:

- Only four container kinds ever carry the flag: **worlds** (1,630 of 11,367 entries), **`.ep`** (287 of 596), **`.rsc`** (65 of 270) and **`.ctl`** (62 of 68). No model, mesh, material, animation, effect or font file has a flagged entry.
- The flag is a property of the **entry in one file**, not of the target: 177 paths are flagged in every file that lists them, 165 paths are flagged in some files and not in others (for example `Fonts/SeriousZurich.fnt`, `Models/Weapons/AutoShotgun/AutoShotgun_FP.mdl`).
- Which entries are flagged: **the same 24 paths are flagged in every one of the 59 worlds** and never clear there: 17 weapon parameter files (`ColtWeapon.ep`, `CircularSawWeapon.ep`, `ZapGunWeapon.ep` and 14 `*WeaponRendering.ep` files), `PlayerPuppet.ep`, `GenericEntity.ctl`, `HudParams.rsc`, the player model `Models/Player/SeriousSam/SeriousSam.mdl`, `Models/Projectiles/HotBullet/HotBullet.mdl` and two bullet-trail textures. Other flagged entries: **62 of the 62** model references of `.ctl` files that have a model; **253 of 256** `CModelConfiguration` references of `.ep` files; the HUD, power-up effect and HUD-bar textures of three `.rsc` files. A reference from a world to a placed model or texture is almost never flagged (123 of 5,106 models, 118 of 771 textures).
- The `RFIL` entries without an `EXOB` entry do not exist: every entry is used by at least one external object.

Reading: the flag marks **references made by the data-driven parameter layer** (databases and parameter files that the game module loads by itself), as opposed to references made by placed level objects. It has no effect on how the target is found. A reader ignores it; a **dependency collector** (for example a pre-loader) may treat flagged entries as "global" resources. The exact engine meaning (a loading hint, an "exposed resource", a sharing flag) is not in the data.

## `EXOB.flags`

`EXOB` entries are `objectId · fileRef · flags · typeId`. The `flags` word is one of three kinds, decided by the target:

| Value(s) | Used for | Meaning |
|---|---|---|
| **1** (36,000 of 42,927 entries; 84 %) | resource files: `CStaticTexture` 14,339, `CModelConfiguration` 6,929, `CSamMaterial` 4,231, `CStaticSound` 2,760, `CEffect` 1,923, parameter structs such as `CBaseItemParams` 737 | an ordinary external resource: "object `objectId` of file `fileRef`" |
| **1000, 1010, 1020, 1025, 1030, 1040, 1050, 1070** (4,428 entries; the file is always `Bin/Shaders.module`) | shader classes | the **concrete shader class** of the referenced object: **1000** `CShadeShader`, **1010** `CSpecularShader`, **1020** `CReflectionShader`, **1025** `CRefractionShader`, **1030** `CBlendLayersShader`, **1040** `CTerrainShader`, **1050** `CPolyBumpShader`, **1070** `CWaterShader`. An entry of the static type `CShader` carries the code of its real subclass (for example 2,668 entries of type `CShader` have 1000), so the code tells which shader class to instantiate; **no other value between 1000 and 1099 occurs** (1060 is unused) |
| **2 to 139,526** other values (2,499 entries; the file is `Bin/Sam2Game.module`) | `CEntityClass` | the **entity-class code** (below) |

**Entity-class codes.** An entity or its properties record in a world (or a `.ctl` file) refers to an external `CEntityClass` object; the entry's flags are a number that is **constant per class**: over all 59 worlds, **143 classes (every entity class and its `…Properties` class) have exactly one code each**, **94 distinct codes**, a class and its properties class share one code, and every external class object is referenced by at least one object. The full list is `ctsemeta-entity-codes.md`. Structure: the 31 classes with codes **2 to 80** are mostly engine classes (a few game classes too: `SMissionObjectiveEntity` 23, `SNavigationalBeaconEntity` 24) (`CWorldInfoEntity` 2, `CStaticModelEntity` 10, `CDirectionalLightEntity` 17, `CPointLightEntity` 18, `CHazeEntity` 19, `CBloomEntity` 21, `CChapterInfoEntity` 25, `CEffectEntity` 30, the power-ups 32–37, `CMacroEntity` 40, `CFluidEntity` 50, `CForceFieldEntity` 70, `CFormationMarkerEntity` 80); the 112 game classes have codes from **0x13002** (`SFlareEntity` 77,826) to **0x22106** (`CArmorItemXXLEntity` 139,526), grouped in blocks: ammunition items `0x15003..0x15014`, health and life items `0x15101..0x15106`, repair `0x15200`, weapon items `0x17005..0x17035` (every second or fourth number), markers, spawners and barriers `0x18000..0x1801B`, props and sounds `0x19000..0x19006`, characters `0x1C003..0x1C008`, treasure `0x21002..0x21004`, armour `0x22101..0x22106`. The grouping suggests a code of the form *group · 4096 + index*, but **nothing in the data confirms that reading**: treat the code as an opaque identifier of the class.

Why it matters: the entry has no name of its own; the class is known from the object that refers to it. A loader **resolves a class by name** (the referring record's own type name) and may ignore the code; it can check it against the table above as a consistency test.

## `EDTY`

`EDTY` (the `u32 count` and the 8-byte entries) is the **type table of the extra records**: the entry *i* is `(object id, type id)` of the *i*-th extra record after `EDOB`, in the same order (all 60 non-world files with extras: identical; the **count equals the `EDOB` count in all 8,063 files**, worlds included, the ids of worlds were not compared one by one). The ids are disjoint from `OBTY` (60 of 60 checked). It plays for the extra records the role `OBTY` plays for the main objects, which lets a reader locate and skip them without decoding. The earlier statement in `ctsemeta-container.md` ("meaning unknown") is replaced by this rule.

## Extra records after `EDOB`

What the extra classes are (all 60 distinct non-world files; worlds are in `ctsemeta-objects.md` and `wld-*.md`):

| Container | Records | Classes (count) | What they are |
|---|---:|---|---|
| `.bmf` meshes | 593 | `SurfacePolygonMapTransferData` 87, `CShaderPreset` 82, `CFloatGraphShaderModifier` 65, `MorphMapTransferData` 51, `CSineGraph` 45, `CShadeShaderArgs` 43, `CPlacementData` 29, `CPixelightShaderArgs` 28, `RenderLayerMapTransferData` 26, `CCustomDataHolder` 18, `CEditMesh` 18, `CPackedEditMesh` 18 | the editor's source mesh and the maps used to rebuild the baked mesh, shader presets of the source materials, graph modifiers, and the viewport placement |
| `.mdl` models | 1,237 | `CEditChannel` 162, `CWeightVertexMap` 117, `CEdtDestructionEffect` 92, `CDFXDistribution` 67, `CDFXDebris` 48, `CPlacementData` 43, `CEditMechanismShapeTemplate` 41, `CPrimitiveHullTemplate` 41, `CEditMechanismPartTemplate` 37, `CSurfacePolygonMap` 35, `CCustomDataHolder` 31, … | editor models of animation, vertex weights, destruction effects, mechanism templates (physics source data) |
| `.ans` animation sets | 755 | `CEditChannel` 570, `CEditEnvelope` 95, anim-event classes (`CHidePlayerWeaponAttachmentAnimEvent` 21, `CShowPlayerWeaponAttachmentAnimEvent` 12, `CSamSoundAnimEvent` 5, `CSamEffectAnimEvent` 5, `CRenderMuzzleFlashAnimEvent` 3), `CEditAnimation` 20, `CAnimEventHolder` 20, `CEditAnimSet` 4 | the editor's keyframe channels and envelopes, and the animation events |
| `.fnt` fonts | 19 | `CEditFont` 8, `CStaticTexture` 8, `CFontFilterOutline` 3 | the font's source description and outline filter |
| `.skl` skeleton | 2 | `CEditSkeleton`, `CEditSkeletonLOD` | the editor skeleton |

**References from the main section** (checked, all 60 files): the main objects point into the extra records through exactly these members, by id: `CResource.4 → CCustomDataHolder` (49), `CModelConfiguration.13 → CEditModelConfig` (28), `CMesh.3 → CEditMesh` (31), `CAnimHost.2 → CEditAnimHost` (14), `CDestructionHolder.2 → CEdtDestructionHolder` (14), `CFont.10 → CEditFont` (8), `CSkeleton.2 → CEditSkeleton` (9), `CMechanismTemplateHolder.2 → CEditMechanismTemplateHolder` (6), `CAnimSet.2 → CEditAnimSet` (6). These are the **edit-data pointers** of the runtime classes. **A runtime reader can follow them lazily or ignore them**: the runtime classes read in `mdl.md`, `bmf.md`, `ans.md`, `skl.md` and `fnt.md` are in the main section; the destruction effects and mechanism templates that look like game data (`CDFXDebris`, `CPrimitiveHullTemplate`) are reached through the destruction and mechanism holders and are therefore needed only if the runtime uses the editor-side copies (the specs of `destruction.md` and `physics.md` use the main-section configuration). `CCustomDataHolder` is a list of custom data objects; the sample objects are **editor viewport placements** (`CPlacementData`: a view position, angles and a distance), not game data. The previous question of `mdl.md` ("not checked whether any other main-section member refers to an extra record") is thereby answered: these nine members are all.

**Worlds** keep their extra records for a different reason: the main section refers to them (`ctsemeta-objects.md`), so a world reader must keep them.

## What is not in the data

1. The meaning of `INFO[0]` and of `RFIL.flags` beyond the correlations above; whether the engine changes behaviour for flagged references.
2. How entity-class codes are produced (a registration order, a hash, a packed group and index) and whether they are stable across game versions: they match in all 59 worlds of one install, and `InSamnity2Game.dll` (a different module) shares the file format but its worlds were not read.
3. Whether other shader codes exist in mods (1060, 1080 and up: unused in the shipped files).
4. The reason the 30 overridden old-version textures were kept in the patches.

## Reader invariants

1. Accept versions 4–8 (4 words `INFO` for 4–7; member-number words in version 4 only).
2. Ignore `INFO[0]` and the `0x40000` bit; keep them when copying a file.
3. Resolve an external class by the **name** (entity/properties type or shader static type) and, when present, check `EXOB.flags` against the code tables as a consistency check only.
4. `EDTY` count must equal the `EDOB` count; ids must be disjoint from `OBTY`.
5. Treat the members in the table "References from the main section" as optional edit-data pointers.

## Test vectors

- `Textures/Levels/01a_Aisha/Trim_Horizontal_01.tex` (version 4, `2.005.00:24818`): `INFO` = `0, 1, 20, 2` (four words: `RFIL` 0, `IDNT` 1, types 20, objects 2); the first struct is `CResourceFile` with members numbered 1, 2, 4 (`rf_iNextFreeID`, `rf_iDefaultID`, `rf_crsShared`).
- `Textures/Menu/background_gradient.tex` (version 7, `2.023.00:40633`): `INFO` = `0, 1, 20, 2`.
- Counts: versions 4/5/6/7 = 20/4/113/18 files (155), 125 effective; `INFO[0]` 0/1 = 769/7,294; 119 files with extra records, all with `INFO[0]` 0; `RFIL.flags` set in 2,044 of 39,790 entries (version-8 files); `EXOB.flags` 1 in 36,000 of 42,927 entries; 94 entity-class codes for 143 classes; `CWorldInfoEntity` = 2, `CStaticModelEntity` = 10, `SDoubleShotgunItemEntity` = 94,229, `SHealthItemLEntity` = 86,275; shader codes 1000, 1010, 1020, 1025, 1030, 1040, 1050, 1070.
