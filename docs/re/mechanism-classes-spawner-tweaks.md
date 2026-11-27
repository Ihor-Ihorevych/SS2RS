# Spawner tweak classes and the two tweak resource files (T01-109, part of T01-026)

- **Roadmap task:** T01-109 (part 8 of the class inventory T01-026). Overview and shared rules: [`mechanism-classes.md`](mechanism-classes.md). Companion pages: [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) (the spawner that applies the tweaks), [`mechanism-classes-movers.md`](mechanism-classes-movers.md), [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md), [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the 29 classes, their bases and members, the two resource files (every object counted), the numbering of the tweak types (the code and the data agree: two independent readings), the value tables and, as of T01-166, the owner-id resolution (section 3b: script, 158 of 162); **medium** for what each tweak does to a spawner (read from `DoSpawnerTweaks` and `DoBootTweaks`, once) and for the condition matching; **low** for the boot tweak effects and the remaining open question. Nothing was run.

Prerequisites: [`mechanism-classes.md`](mechanism-classes.md), [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) (the spawner members the tweaks set), `rsc-ctl.md` and `rsc-ctl-schema.md` (the `.rsc` container), `wld-entities.md` (spawner records and `ulEntityIDVis`).

## Summary

The 29 classes are **resource objects**, not entities: they live in two files of `Content/SeriousSam2/Databases`, **`spawnertweaks.rsc`** (198 objects, 31 levels) and **`spawnerboottweaks.rsc`** (35 objects, 11 levels). They are a **per-level patch list for spawners**: a *condition* names the spawners it applies to (by the entity id of the spawnee, by the spawnee's name or by a placement box) and carries one to four *tweaks* that change the spawner (or its spawnee's record) when the spawner boots. About 37 of the 98 tweaks concern the **spawner multiplier** that scales the enemy count in co-op (`mechanism-classes-spawners.md`): "no multiplier", "reduce", "increase multiplier".

What a reader needs to know first:

1. **Structure.** `CGlobalSpawnerTweakParams` (one object, root of `spawnertweaks.rsc`) holds a list of **31 `CSpawnerTweakParams`**, one per level (`ctp_strLevelName`, e.g. `1_4_ApproachingCity`), each with a list of **conditions** (`CSpawnerTweakCondition` and 3 kinds); each condition has a list of **tweaks** (`stp_sttTweakType`: objects of the 16 `CSpawnerTweakType` kinds). `CGlobalSpawnerBootTweaks` (root of `spawnerboottweaks.rsc`) holds **11 `CSpawnerBootTweaks`** (one per level), each with **`CSpawnerBootTweakCondition`** objects (an owner id list and one boot tweak).
2. **Conditions in the file:** 67 conditions with tweaks (**56 by owner id**, **9 by owner name**, **2 by location**) and 11 boot conditions. 98 tweaks (23 "no multiplier", 29 "random cone", 8 "reduce", 6 "increase multiplier", 6 "modify delays", 5 "placement", 5 "increase Y", 4 "null tactics", 4 "ignore can't move", 2 "no auto patrol", 2 "patrol speed", and one each of "barrier collision", "change sense range", "change threat sensitivity" and "ignore max") and 11 boot tweaks (7 "no random", 4 "must be shooter").
3. **The type number** of a tweak is stored in the base class member `stt_sttTweakType` (`SpawnerTweakType`): **0 reduce, 1 random cone, 2 placement, 3 no multiplier, 4 increase multiplier, 5 modify delays, 6 ignore can't move, 7 increase Y, 8 null tactics, 9 no auto patrol, 10 change sense range, 11 barrier collision, 12 patrol speed, 13 change threat sensitivity, 14 ignore max** (the 15th kind, "enemy size", is not in the file; it is used by the boot tweak function). The data and the dispatch in the spawner's code agree on all 15 (**high**).
4. **Developer notes are in the data.** Every condition has a free-text reason (`stc_strReason`, for example a short note that names the enemy or the fix); it has no effect and need not be kept.

## Evidence

- **Export table and vtables.** The 29 classes export 219 functions (16 named methods: `mdGetName` on the condition and tweak classes, `mdGetDataType` and the standard symbols on the others). The spawner's two functions that apply the tweaks (`DoSpawnerTweaks`, 2.5 kB, and `DoBootTweaks`, 1.1 kB) were decompiled and read.
- **Data.** `tools/re/mechanism_classes_verify.py` section 4 reads both files with the container reader of `ep_schema.py` and prints every count and value table below. Members were named by the data (all 29 classes' members are named in the data; the binary's identifiers are the same). `tools/re/spawner_multiplier_verify.py` (T01-166) resolves every owner id against the spawnee it names in its own level (section 3b).
- **Second reading of the type numbers.** The code's dispatch (`DoSpawnerTweaks`: the type number selects which member of the tweak object is copied) and the stored type numbers of the 15 tweak classes present in the file agree for all 15 (script: type number per class).
- **Not checked.** The effect of each tweak on the spawned enemies at run time, the boot tweak application (profiled), the order in which several tweaks of one condition are applied, and the one tweak kind that the file does not use.

## Layout / rules

### 1. The classes

| Class | Base | Members | Objects in the files | Exported methods |
|---|---|---|---|---|
| `CGlobalSpawnerTweakParams` | `CResource` | `gstp_stpSpawnerTweaks` (list of level tables) | 1 | none (7 standard) |
| `CSpawnerTweakParams` | `CResource` | `ctp_strLevelName`, `ctp_astcTweakCondition` | 31 | `mdGetName` |
| `CSpawnerTweakCondition` | `CResource` | `stc_strReason`, `stc_iConditionType`, `stp_sttTweakType` (list of tweaks) | 0 (abstract) | none |
| `CSpawnerTweakConditionOwnerID` | `CSpawnerTweakCondition` | `stoid_aiOwnerIDs` (list of entity ids) | 56 (129 ids in all) | `mdGetName` |
| `CSpawnerTweakConditionOwnerName` | `CSpawnerTweakCondition` | `sten_strOwnerName` | 9 | `mdGetName` |
| `CSpawnerTweakConditionLocation` | `CSpawnerTweakCondition` | `stl_iMinX`, `stl_iMaxX`, `stl_iMinY`, `stl_iMaxY`, `stl_iMinZ`, `stl_iMaxZ` | 2 | `mdGetName` |
| `CSpawnerTweakType` | `CResource` | `stt_sttTweakType` (the type number) | 0 (abstract) | none |
| 15 kinds (below) | `CSpawnerTweakType` | 0 to 5 members each | 98 in all | `mdGetName` on those with members |
| `CSpawnerTweakEnemySize` | (`CSpawnerTweakType`) | not in the data | 0 | `mdGetName` |
| `CGlobalSpawnerBootTweaks` | `CResource` | `gsbt_sbtSpawnerTweaks` | 1 | none |
| `CSpawnerBootTweaks` | `CResource` | `sbt_strLevelName`, `sbt_asbtcTweakCondition` | 11 | `mdGetName` |
| `CSpawnerBootTweakCondition` | `CResource` | `sbtc_strReason`, `sbt_aiOwnerIDs` (33 ids in all), `sbtc_sbttTweakType` | 11 | `mdGetName` |
| `CSpawnerBootTweakType` | `CResource` | `sbtt_iTweakType` | 0 (abstract) | none |
| `CSpawnerTweakNoRandom`, `CSpawnerTweakMustBeShooter` | `CSpawnerBootTweakType` | none | 7, 4 | none |

`stc_iConditionType` is **0 for owner-name conditions, 1 for owner-id conditions and 2 for location conditions** (all 67 conditions; **high**). The classes are in two groups: 15 `CSpawnerTweakType` kinds with numbers 0 to 14, plus the class for the 15th number (enemy size), plus the two boot kinds under the boot base.

### 2. The 15 tweak kinds and what they do (read)

"Member" is the binary's identifier. The effect column is what `DoSpawnerTweaks` does with the tweak when the spawner boots (read, medium); "values" are the file's.

| No. | Class (objects) | Members and values in the file | Effect on the spawner or its spawnee record |
|---|---|---|---|
| 0 | `CSpawnerTweakReduce` (8) | `str_fMultiplier`: **0.5** in all 8 | the multiplier result is scaled down: the spawn count is multiplied by this factor instead of the full multiplier |
| 1 | `CSpawnerTweakRandomCone` (29) | `strc_fMinPitch` (15 in 18; 0, 1, 3, 10, 45, -35), `strc_fMaxPitch` (45 in 12; 2 .. 50), `strc_fAngleH` (1 in 13, 360 in 4, 90, 15, 5, 120, 45), `strc_fMinVelocity` (0.1 .. 50), `strc_fMaxVelocity` (0.2 .. 60) | sets the spawner's **random launch cone** (the five `en_fMinPitch`, `en_fMaxPitch`, `en_fAngleH`, `en_fMinVelocity`, `en_fMaxVelocity`) and turns random-cone launching on; **the spawner's defaults are 15, 45, 90, 10, 15** |
| 2 | `CSpawnerTweakPlacement` (5) | `stp_ptRelative` (`PlacementType` 0 in all 5), `stp_qvPlacement` (a placement, 5), `stp_bNullLauncher` (1 in 4), `stp_bNullSpawnSource` (1 in 3) | **replaces the spawn placement** with `stp_qvPlacement` when it differs from the default, and clears the launcher link and the spawn source link when the flags are set |
| 3 | `CSpawnerTweakNoMulti` (23) | none | the spawner is **not multiplied** by the player count (the no-multiplier flag) |
| 4 | `CSpawnerTweakIncreaseMulti` (6) | `stim_fMultiplier`: 2 in 3, 3 in 2, 20 in 1 | raises the multiplier (adds to the multiplier factor) |
| 5 | `CSpawnerTweakModifyDelays` (6) | `stmd_fMinDelay` (0, 0.05, 0.5, 1, 5, 20), `stmd_fGroupDelay` (0, 0.05, 0.5, 1 in 2, 5), `stmd_fInitialDelay` (0 in 5, 1 in 1) | sets the spawner's minimum delay, **group delay** and **initial delay** (`en_fMinDelay`, `en_fDelayGroup`, `en_fDelayInitial`) |
| 6 | `CSpawnerTweakIgnoreCantMove` (4) | none | clears the spawner's "cant move" handling for the spawnee |
| 7 | `CSpawnerTweakIncreaseY` (5) | `stiy_fAddToY`: 2 in 2, 6, -5, -77 | sets `en_iAddToY`, the vertical offset added to the spawn position |
| 8 | `CSpawnerTweakNullTactics` (4) | none | clears the spawnee record's **tactic links** (the character's patrol/tactic entities) |
| 9 | `CSpawnerTweakNoAutoPatrol` (2) | `stap_bAutoPatrol`: 0 and 1 | sets the spawnee record's **auto patrol flag** from the member |
| 10 | `CSpawnerTweakChangeSenseRange` (1) | `stcsr_fSenseRange`: 200 | sets the spawnee record's sense range |
| 11 | `CSpawnerTweakBarrierCollision` (1) | `stb_bcBarrierCollision` (`BarrierCollision` 1) | sets the spawnee record's barrier collision mode (the enemy may pass through barrier fields) |
| 12 | `CSpawnerTweakPatrolSpeed` (2) | `stms_fSpeed`: 2 in both | sets both patrol speeds of the spawnee record |
| 13 | `CSpawnerTweakChangeThreatSensitivity` (1) | `stts_tsThreatSensitivity` (`EThreatSensitivity` 6) | sets the spawnee record's threat sensitivity |
| 14 | `CSpawnerTweakIgnoreMax` (1) | none | sets `en_bIgnoreMax`: the spawner ignores the chapter's maximum enemy count |
| 15 | `CSpawnerTweakEnemySize` | not in the file | an `EnemySize` choice used by `DoBootTweaks` and `SetDelaysAndType` (name; low) |

**The multiplier tweaks** (0, 3, 4) and the delay and cone tweaks (1, 5) are the ones that **change how many enemies come and how fast** in co-op: 23 + 8 + 6 = 37 of the 98 tweaks concern the multiplier; this page gives the numbers, the rule is in `mechanism-classes-spawners.md`.

### 3. Conditions and levels

- **Owner id** (56 conditions, 129 ids): `stoid_aiOwnerIDs` lists entity ids; the tweak applies to the spawner whose spawnee (`en_henOwner`) has that id (`SBaseProperties`' `ep_ulEntityIDVis` equals the serial: `wld-visibility.md`; the match with the spawnee's id was read, medium). A condition can name 1 to 5 ids.
- **Owner name** (9 conditions): `sten_strOwnerName` is compared with the **name of the spawnee's class or record** (read: it is compared with the spawner's `en_strSpawningPuppetName`-like name string, for example the class name of the enemy; medium).
- **Location** (2 conditions): six integers `stl_iMinX .. stl_iMaxZ` bound a box in the level; the spawner whose placement is inside it matches (name; low).
- **Levels with a table (31):** `1_1_Jungle` (1 condition, 1 tweak), `1_2_ApproachingVillage` (2, 2), `1_3_Village` (1, 2), `1_4_ApproachingCity` (5, 9), `1_5_Mdigbo_CitySuburbs` (2, 6), `1_6_Inside_Temple` (2, 3), `1_7_City_Metropolis` (6, 6), `2_1_Swamp` (2, 3), `2_3_Branchester` (3, 3), `2_4_Woodstock` (3, 3), `2_5_Land_Of_Giants` (2, 4), `2_6_BossArena` (1, 1), `3_1_ChiFang_Suburbs` (1, 1), `3_2_ChiFang_City` (1, 1), `3_3_Chaolin_Chambers` (1, 1), `3_4_Chans_Palace` (3, 8), `4_1_Wasteland` (2, 3), `4_2_Canyon` (3, 8), `4_3_Bone_Yard` (4, 7), `5_1_Village` (1, 1), `5_2_SeasideTownApproaching` (2, 2), `5_4_Town_Sewers` (2, 2), `5_5_Town_Garden` (2, 2), `5_6_SkyIslands` (1, 1), `6_2_Military_Base` (2, 3), `6_3_Snow_Base` (4, 6), `7_1_City_Part_I` (1, 1), `7_2_City_Part_II` (1, 1), `7_4_Arenas_JungleStadium` (1, 1), `7_6_City_Part_III` (3, 4), `7_7_Mental` (2, 2). The level name is matched against the world's name (the spawner asks the game info entity for the current level name, read).
- **Boot tweaks (11 levels, one condition each):** `1_1_Jungle`, `1_2_ApproachingVillage`, `1_3_Village`, `1_5_Mdigbo_CitySuburbs`, `2_2_Deserted_Base`, `3_1_ChiFang_Suburbs`, `3_2_ChiFang_City`, `3_4_Chans_Palace`, `4_1_Wasteland`, `6_3_Snow_Base`, `6_4_Alliance_Headquarters`; 33 owner ids in all; **7 "no random"** and **4 "must be shooter"** tweaks. `DoBootTweaks` (read in structure and by its messages, **medium to low**) logs "Spawner with ID '...' is using a no random tweak" or "... a must be shooter tweak" and sets the matching spawner flags: no random = the spawner's random choice (of the spawnee type or the position among a multi-spawner's templates) is switched off; must be shooter = the spawned enemy must be one that shoots (the choice among a multi-spawner's templates). The "enemy size" tweak is looked up by the string "EnemySize" with the message "Spawner with ID '...' is using an Enemy Size tweak '...'".

### 3b. Which spawner each owner id selects (T01-166, data task)

`tools/re/spawner_multiplier_verify.py` walks every world's `SSpawnerEntity` objects, resolves `.120` (`en_henOwner`) to the spawnee and records the spawnee's own id (`CEntity.1`, equal to `ep_ulEntityIDVis`) against its class, then looks each tweak file's owner ids up **in the one world named by the condition's own level table** (ids are per-world serials, not global). **158 of the 162 ids (129 + 33) resolve**: overwhelmingly `SLeggedCharacterEntity` (most conditions, matching the developer notes: "wallpushers", "door smashers", cutscene enemies stuck in geometry), with `SAircraftCharacterEntity` where the note names helicopters (`1_7_City_Metropolis`'s "helicopters get stuck", `5_1_Village`'s "need more heli's", `7_1_City_Part_I`'s "heli's stackin up"), `SMultiSpawnerEntity` where it names a multi-spawner directly (`1_5_Mdigbo_CitySuburbs`'s "end multispawners had some issues", `6_3_Snow_Base`'s eight-id "Group of one with custom behavior", `7_6_City_Part_III`, `7_7_Mental`), and one `SRollingBallCharacterEntity` (`5_2_SeasideTownApproaching`). **Four ids do not resolve** to any spawner in their own level's world: `6_3_Snow_Base` 9725, 9726 (`spawnerboottweaks.rsc`) and 9967 (`spawnertweaks.rsc`), and `1_7_City_Metropolis` **-842150451** (`spawnertweaks.rsc`) — read as an unsigned 32-bit value this is `0xCDCDCDCD`, MSVC's debug-heap "uninitialised memory" fill pattern, so that one id was never actually written by whoever authored the condition (the other two are plain stale ids, most likely from an entity later removed or renumbered by a patch).

### 4. How the spawner uses the tables

On boot the spawner (`CSpawnerEntity::OnBoot`) asks the global game parameters for the tables (`DoSpawnerTweaks`, `DoBootTweaks`; the tables are the `CGlobalSpawnerTweakParams` and `CGlobalSpawnerBootTweaks` objects that the game parameters link), finds the level's `CSpawnerTweakParams` by name (**exact string comparison with the current level name**), walks its conditions and, for each condition that matches the spawner (owner id, owner name or location), applies the tweaks **in list order**. Three **hard-coded level cases** are in the code in addition: in `7_3_Arenas_DesertStadium`, `7_4_Arenas_JungleStadium` and `7_5_Arenas_LavaStadium` every spawner gets the "no multiplier" result and the "delays modified" result before the table is read (the three arena levels; the file has a table only for the jungle stadium). Which of the mutually exclusive effects wins when two tweaks set the same member is the **last** (list order: read). Medium.

## Invariants

1. `spawnertweaks.rsc` has 1 root, 31 level tables, 67 conditions (56 / 9 / 2) and 98 tweaks; `spawnerboottweaks.rsc` has 1 root, 11 level tables, 11 conditions and 11 boot tweaks (script).
2. The type number stored in a tweak equals the dispatch number of the spawner's code for all 15 kinds in the file.
3. Every condition has 1 to 4 tweaks (43 with 1, 19 with 2, 3 with 3, 2 with 4); every boot condition has exactly 1.
4. Tables exist for 31 level names of the 59 worlds; the names are story levels and one arena level (`7_4_Arenas_JungleStadium`).
5. The abstract classes (`CSpawnerTweakCondition`, `CSpawnerTweakType`, `CSpawnerBootTweakType`) have no objects of their own.

## Open questions

1. **Exact conditions and application order:** the owner id match, the name comparison, the location box test, the effect of each of the 15 tweaks on every spawner member, the boot tweak effects and the enemy size tweak: still open at the level of detail of section 2 above. **Answered by T01-166:** which spawner each owner id selects (section 3b below: `tools/re/spawner_multiplier_verify.py` resolves 158 of the 162 ids against a real spawnee in its own level). **Still open** (T01-166 read `DoMultiplier` and its seven procedures in full — [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) section 6 — but none of them visibly reads `str_fMultiplier` or `stim_fMultiplier`; the "reduce"/"increase multiplier" arithmetic most likely sits in `CommonEntryPoint`'s `total = ctInitialTotal * multiplier` step, which remains profiled, not read line by line): where exactly the stored tweak values enter that formula.

## Test vectors

All from the retail install via `tools/re/mechanism_classes_verify.py`, unless a binary reading is named.

1. **Counts.** `spawnertweaks.rsc`: 198 objects (31 level tables, 56 + 9 + 2 conditions, 98 tweaks, 1 root, 1 resource file object); `spawnerboottweaks.rsc`: 35 objects (11 level tables, 11 conditions, 11 tweaks, 1 root, 1 resource file object).
2. **Type numbers.** Every `CSpawnerTweakRandomCone` has type 1 (29 of 29), every `CSpawnerTweakNoMulti` type 3 (23 of 23), every `CSpawnerTweakReduce` type 0 (8 of 8).
3. **A tweak.** `1_2_ApproachingVillage` has a condition by owner id with one tweak: the spawner whose spawnee has that id is changed at boot (a developer note says it is a fix for wall pushers); with a no-multiplier tweak the spawner makes exactly its stored count in co-op (binary reading).
4. **Owner ids (T01-166, `tools/re/spawner_multiplier_verify.py`).** 158 of 162 owner ids resolve to a real spawnee in their own level's world; the id `-842150451` of `1_7_City_Metropolis` is `0xCDCDCDCD` as unsigned, MSVC's uninitialised-heap fill pattern.
