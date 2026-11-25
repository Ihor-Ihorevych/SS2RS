# Level mechanism, spawner and info entity classes of `Sam2Game.dll`: overview (T01-109, part of T01-026)

- **Roadmap task:** T01-109 (part 8 of the class inventory T01-026; the other parts are T01-102 to T01-113). Pages of this part: [`mechanism-classes-movers.md`](mechanism-classes-movers.md) (doors, animated movers, platforms, portals, bouncers, barrier fields, motors, joints, the diamond hand), [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md) (detector areas, killers, locators, environment damagers, alarms, secrets, mission objectives, navigation beacons, cameras), [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) (spawners, multi-spawners, spawn delayers, spawn markers), [`mechanism-classes-spawner-tweaks.md`](mechanism-classes-spawner-tweaks.md) (the 29 spawner tweak, boot-tweak and global tweak classes and the two `.rsc` files), [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md) (`CWorldInfoEntity`, `CChapterInfoEntity`, `ChapterMaxes.txt`).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/mechanism_classes_verify.py` reproduces from the data (which of the 82 classes are saved: 70 under their own or `S`-twin name; the counts of placed entities and records per class; the member lists and their order; the value tables quoted on the pages; the two tweak files; `ChapterMaxes.txt`) and for the **member names** (the binary's declared identifiers, matched to the data by position and checked by the script for 29 classes); **medium** for everything read from the binary once (behaviour rules marked "read"), for "(profiled)" method purposes (strings and callees of the function) and for the net behaviour (two readings: `replication.md` and the exported `GetNetBehavior` functions, 15 of 15 agree); **low** for "(name)" descriptions and for the Open questions. Nothing was run.

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `wld-entities.md` and `wld-entity-schema.md` (the entities and records in worlds, the spawner structure), `level-logic.md` and `level-logic-api.md` (the events and commands the macro scripts use on these classes), `replication.md` (net behaviour and replicated members), `ep.md`, `rsc-ctl.md`, and the sibling parts of this inventory (`weapon-classes.md`, `item-classes.md`, `puppet-classes.md`, `behavior-classes.md`).

## Summary

`Sam2Game.dll` exports **82 classes** for the level's mechanisms, spawners and info entities (**1,206 exported functions**: 726 named methods and constructors besides the standard symbols). **70 are in the data** under their own name or the `S` twin of the exported `C` name (72 if the external joint pair is counted, section 3); **12 exist only in the binary or under another name**: the platform pair, the alarm pair, the spawn delayer pair, `CSpawnerTweakEnemySize`, the three save-state structures (`CSecretEntityData`, `CMissionObjectiveData`, `CNavigationalBeaconData`) and the external joint pair (the data calls it `CTestJoint...`). In the 59 worlds of the retail install the family is **7,651 placed entities** (3,783 of them spawners); the 29 tweak classes live in two `.rsc` files.

| Part | Classes | Page |
|---|---|---|
| movers | 19: doors, animated movers (+ envelope provider), platforms, portals, bouncers, barrier fields, generic motors, external joints, the diamond hand | [`mechanism-classes-movers.md`](mechanism-classes-movers.md) |
| triggers and announcers | 22: detector areas, killers, locators, environment damagers, alarms, secrets, mission objectives, navigation beacons, cameras (with their `...Data` and envelope provider classes) | [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md) |
| spawners | 8: `CSpawnerEntity`, `CMultiSpawnerEntity`, `CSpawnDelayerEntity`, `CSpawnMarkerEntity` and their properties | [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) |
| spawner tweaks | 29: `CSpawnerTweakType` and its kinds, the conditions, the per-level tables, the boot tweaks, the two global holders | [`mechanism-classes-spawner-tweaks.md`](mechanism-classes-spawner-tweaks.md) |
| world and chapter info | 4: `CWorldInfoEntity`, `CChapterInfoEntity` and their properties | [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md) |

What a reader needs to know first:

1. **Entity and record.** Every placed entity of the family has a **property record** of the matching `...Properties` class in the extra section of the world (`wld-entities.md`): entity members are `en_...` (run-time), record members `ep_...` (design). The spawner is the exception: its records are owned by the spawnee. The **tweak classes are not entities**: they are objects of two resource files (`.rsc`).
2. **The data names about half of the members with a number** (`SDoorProperties.5`). The binary declares every member name in a table in the same order; this part matches the two **by position** (the "order rule" of `weapon-classes.md`) and the script checks it: for the 29 classes whose binary table was read, the counts and the position of every member the data names agree in every file variant (one exception: the data renames two members of the mission objective properties; the older spawner layout lacks members, section 4). The names on the pages are the binary's own identifiers.
3. **`SBaseEntity.1` / `SBaseProperties.1` is a handle to a chapter info record** in every `S`-layer entity (section 2).
4. **Spawners are the largest class** (3,783 entities, 74 named methods, 59 members) and are **host-only** (`NEB_NONE`): the spawner decides what exists, the clients only receive the spawned entities.
5. **Net behaviour** of the family (`replication.md` 3.4 and, as a second reading, the exported `GetNetBehavior` functions, each of which is a shared function that returns a constant): **`NEB_STATIC` (1)** for the door, animated mover, barrier field, bouncer, camera, chapter info, diamond hand, mission objective, beacon, portal, secret and world info; **`NEB_DYNAMIC` (2)** for the platform, the generic motor and the external joint; **`NEB_NONE`** (no function exported) for detectors, killers, locators, damagers, alarms, spawners, multi-spawners, spawn markers and spawn delayers.
6. **The data holds uninitialised values**: stray numbers in a net copy of the mission objective state, one NaN in a bouncer, and the world info and the spawner each have several type layouts (the older worlds lack members). The pages list them.

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed and the 932 exported classes grouped by name (the split of T01-026, group "P7 mechanisms/spawners/triggers"). The 82 classes export 1,206 functions: 350 in the movers part (217 named methods besides constructors), 299 (157) in the triggers part, 150 (94) in the spawner part, 219 (16) in the tweak part and 188 (160) in the world part. Names are quoted as exported.
- **Profiles.** **All 726 named methods and constructors** were profiled: for each, the strings the function refers to and the exported functions it calls (a purpose sentence follows from them and the name; event-name strings give the macro events). This is the **"profiled"** basis. About **165 were decompiled and about 60 of them read closely** (the **"read"** basis; the others, mostly constructors and one-line stubs, were looked at for a value only), chosen for the rules a reimplementation needs: the spawner's counters, events, multiplier and boot tweaks, the door states, the killer, bouncer, detector, portal, secret, objective, camera and joint functions, the world info's spawn-slot and limit functions and the chapter info's start and next-chapter functions. Each read rule is **medium** (one reading); where the data or a second function agrees, this is said.
- **Virtual function tables.** The tables of the 24 entity and 24 properties classes were compared slot by slot with `CBaseEntity` (102 slots) and `CBaseProperties` (30 slots). Identical small functions are shared by the linker, so a slot can show another class's name; such names are not used. Every entity class of the family overrides three shared stubs of the `S` layer (`OnSimStart`, `OnSpawn`, `GetChapter`); the number of new virtual functions per class ranges from 0 (the diamond hand, the spawn marker) to 52 (the spawner) and 79 (the world info); the door and the animated mover add 9, the camera 13.
- **Net behaviour, second reading.** The 15 exported `GetNetBehavior` functions were decompiled: each returns the constant 1 (12 classes: the door, animated mover, barrier field, bouncer, camera, chapter info, diamond hand, mission objective, beacon, portal, secret, world info) or 2 (the platform, the generic motor, the external joint), as `replication.md` 3.4 says (**high**, two readings).
- **Member tables.** The binary's member tables (name, type, byte offset, flags) were read for the 27 `S`-layer classes of the family plus `CTestJointEntity` and `CTestJointProperties`, and fragments for most of the others (the offsets are not recorded: `conventions.md`, T01-063); the members the data names itself were checked against them.
- **Data.** `tools/re/mechanism_classes_verify.py` (about 12 minutes: 2 for the type tables of all 8,063 format-8 files, about 8 for the 59 worlds) prints every count and value table of the five pages.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run; no timing, no random sequence; the macro scripts and their use of the commands (`level-logic.md` has the data side), the animations, sounds, effects and the editor tools; the HUD and Netricsa side of secrets and objectives; the AI and puppet classes the spawners create (`behavior-classes.md`, `puppet-classes.md`).

## Layout / rules

### 1. Basis marks used on the five pages

| Mark | Means |
|---|---|
| **read** | the function was decompiled and read; the rule is medium |
| **profiled** | strings and called functions were listed; the purpose sentence follows from them and from the name; medium to low for details |
| **name** | the name only; low |

Confidence words follow `conventions.md` section 3. `#` after a member marks a replicated member (`replication.md`); "stored" means the value occurs in placed entities or records.

### 2. Layers and the chapter link

The exported class `CBaseEntity` is the base of every entity. In the data, 18 of the 21 entity classes of this family (including the test joint) derive from **`SBaseEntity`**, the `S` layer (classes built from the editor's script description); the camera, the diamond hand and the spawn marker derive from `CBaseEntity` directly. The `S` layer adds **one member**, the **chapter link** (`SBaseEntity.1`, record side `SBaseProperties.1`: a handle to a `CChapterInfoEntity` or its record), and **three virtual functions** that every such class overrides (`OnSimStart`, `OnSpawn`, `GetChapter`; all shared stubs here, so no behaviour was read). A set link ties the entity to that chapter: every **portal** has one (54 of 54), **516 of 1,077 detector areas**, 13 of 176 doors, 4 of 277 animated movers, 11 of 131 mission objectives, 15 of 174 beacons and 1 secret; no bouncer, barrier field, killer, locator, generic motor or joint has one (script, section 3 "entity linked to a chapter info"). The world info entity uses the link when it restores a chapter (`mechanism-classes-world-info.md`).

### 3. The 82 classes

"Saved as" is the name in the data ("-" = not in the data); "base" is the data's base class; "members" the own members of the data's type table; "files" the number of files with the class (`wld` = worlds, `rsc` = resource files); "symbols (methods)" the exported symbols besides the 7 standard ones and, in brackets, the named methods. `CExternalJoint*` is **identified** with the data's `CTestJoint*` (movers page, section 8): both have the same members and the exported methods use them; `replication.md` lists it so.

| Class | Part | Saved as | Data base | Members | Files | Symbols (methods) |
|---|---|---|---|---|---|---|
| `CAnimatedMoverEntity` | movers | `SAnimatedMoverEntity` | SBaseEntity | 19 | 39 wld | 33 (26) |
| `CAnimatedMoverEnvelopeProvider` | movers | `SAnimatedMoverEnvelopeProvider` | CEntityEnvelopeProvider | 1 | 39 wld | 14 (7) |
| `CAnimatedMoverProperties` | movers | `SAnimatedMoverProperties` | SBaseProperties | 3 | 39 wld | 9 (2) |
| `CBarrierFieldEntity` | movers | `SBarrierFieldEntity` | SBaseEntity | 8 | 54 wld | 23 (16) |
| `CBarrierFieldProperties` | movers | `SBarrierFieldProperties` | SBaseProperties | 5 | 54 wld | 12 (5) |
| `CBouncerEntity` | movers | `CBouncerEntity` | SBaseEntity | 19 | 15 wld | 22 (15) |
| `CBouncerProperties` | movers | `CBouncerProperties` | SBaseProperties | 12 | 15 wld | 10 (3) |
| `CDiamondHandEntity` | movers | `CDiamondHandEntity` | CBaseEntity | 33 | 1 wld | 31 (24) |
| `CDiamondHandProperties` | movers | `CDiamondHandProperties` | CBaseProperties | 9 | 1 wld | 7 (0) |
| `CDoorEntity` | movers | `SDoorEntity` | SBaseEntity | 24 | 33 wld | 39 (32) |
| `CDoorProperties` | movers | `SDoorProperties` | SBaseProperties | 17 | 33 wld | 13 (6) |
| `CExternalJointEntity` | movers | `CTestJointEntity` | SBaseEntity | 13 | 16 .wld | 24 (17) |
| `CExternalJointProperties` | movers | `CTestJointProperties` | SBaseProperties | 7 | 16 .wld | 8 (1) |
| `CGenericMotorEntity` | movers | `CGenericMotorEntity` | SBaseEntity | 11 | 6 wld | 41 (34) |
| `CGenericMotorProperties` | movers | `CGenericMotorProperties` | SBaseProperties | 5 | 6 wld | 8 (1) |
| `CPlatformEntity` | movers | - |  |  | - | 17 (10) |
| `CPlatformProperties` | movers | - |  |  | - | 10 (3) |
| `CPortalEntity` | movers | `CPortalEntity` | SBaseEntity | 7 | 22 wld | 19 (12) |
| `CPortalProperties` | movers | `CPortalProperties` | SBaseProperties | 0 | 22 wld | 10 (3) |
| `CAlarmEntity` | triggers | - |  |  | - | 11 (4) |
| `CAlarmProperties` | triggers | - |  |  | - | 8 (1) |
| `CCameraEntity` | triggers | `SCameraEntity` | CBaseEntity | 24 | 28 wld | 51 (44) |
| `CCameraEnvelopeProvider` | triggers | `SCameraEnvelopeProvider` | CEntityEnvelopeProvider | 1 | 28 wld | 17 (10) |
| `CCameraProperties` | triggers | `SCameraProperties` | CBaseProperties | 8 | 28 wld | 12 (5) |
| `CDetectorAreaEntity` | triggers | `CDetectorAreaEntity` | SBaseEntity | 14 | 41 wld | 21 (14) |
| `CDetectorAreaProperties` | triggers | `CDetectorAreaProperties` | SBaseProperties | 7 | 41 wld | 10 (3) |
| `CEnvironmentDamagerEntity` | triggers | `CEnvironmentDamagerEntity` | SBaseEntity | 4 | 2 wld | 11 (4) |
| `CEnvironmentDamagerProperties` | triggers | `CEnvironmentDamagerProperties` | SBaseProperties | 3 | 2 wld | 10 (3) |
| `CKillerEntity` | triggers | `SKillerEntity` | SBaseEntity | 9 | 31 wld | 14 (7) |
| `CKillerProperties` | triggers | `SKillerProperties` | SBaseProperties | 6 | 31 wld | 10 (3) |
| `CLocatorEntity` | triggers | `SLocatorEntity` | SBaseEntity | 5 | 3 wld | 17 (10) |
| `CLocatorProperties` | triggers | `CLocatorProperties` | SBaseProperties | 4 | 3 wld | 10 (3) |
| `CMissionObjectiveData` | triggers | - |  |  | - | 3 (0) |
| `CMissionObjectiveEntity` | triggers | `SMissionObjectiveEntity` | SBaseEntity | 26 | 40 wld | 28 (21) |
| `CMissionObjectiveProperties` | triggers | `SMissionObjectiveProperties` | SBaseProperties | 18 | 40 wld | 8 (1) |
| `CNavigationalBeaconData` | triggers | - |  |  | - | 3 (0) |
| `CNavigationalBeaconEntity` | triggers | `SNavigationalBeaconEntity` | SBaseEntity | 3 | 29 wld | 14 (7) |
| `CNavigationalBeaconProperties` | triggers | `SNavigationalBeaconProperties` | SBaseProperties | 1 | 29 wld | 8 (1) |
| `CSecretEntity` | triggers | `CSecretEntity` | SBaseEntity | 14 | 32 wld | 22 (15) |
| `CSecretEntityData` | triggers | - |  |  | - | 3 (0) |
| `CSecretProperties` | triggers | `CSecretProperties` | SBaseProperties | 6 | 32 wld | 8 (1) |
| `CMultiSpawnerEntity` | spawners | `SMultiSpawnerEntity` | SBaseEntity | 2 | 15 wld | 12 (5) |
| `CMultiSpawnerProperties` | spawners | `SMultiSpawnerProperties` | SBaseProperties | 4 | 15 wld | 13 (6) |
| `CSpawnDelayerEntity` | spawners | - |  |  | - | 9 (2) |
| `CSpawnDelayerProperties` | spawners | - |  |  | - | 7 (0) |
| `CSpawnMarkerEntity` | spawners | `CSpawnMarkerEntity` | CBaseEntity | 2 | 16 wld | 12 (5) |
| `CSpawnMarkerProperties` | spawners | `CSpawnMarkerProperties` | CBaseProperties | 0 | 16 wld | 8 (1) |
| `CSpawnerEntity` | spawners | `SSpawnerEntity` | SBaseEntity | 58 | 46 wld | 81 (74) |
| `CSpawnerProperties` | spawners | `SSpawnerProperties` | SBaseProperties | 14 | 46 wld | 8 (1) |
| `CGlobalSpawnerBootTweaks` | tweaks | `CGlobalSpawnerBootTweaks` | CResource | 1 | 1 rsc | 7 (0) |
| `CGlobalSpawnerTweakParams` | tweaks | `CGlobalSpawnerTweakParams` | CResource | 1 | 1 rsc | 7 (0) |
| `CSpawnerBootTweakCondition` | tweaks | `CSpawnerBootTweakCondition` | CResource | 3 | 1 rsc | 8 (1) |
| `CSpawnerBootTweakType` | tweaks | `CSpawnerBootTweakType` | CResource | 1 | 1 rsc | 7 (0) |
| `CSpawnerBootTweaks` | tweaks | `CSpawnerBootTweaks` | CResource | 2 | 1 rsc | 8 (1) |
| `CSpawnerTweakBarrierCollision` | tweaks | `CSpawnerTweakBarrierCollision` | CSpawnerTweakType | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakChangeSenseRange` | tweaks | `CSpawnerTweakChangeSenseRange` | CSpawnerTweakType | 1 | 1 rsc | 7 (0) |
| `CSpawnerTweakChangeThreatSensitivity` | tweaks | `CSpawnerTweakChangeThreatSensitivity` | CSpawnerTweakType | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakCondition` | tweaks | `CSpawnerTweakCondition` | CResource | 3 | 1 rsc | 7 (0) |
| `CSpawnerTweakConditionLocation` | tweaks | `CSpawnerTweakConditionLocation` | CSpawnerTweakCondition | 6 | 1 rsc | 8 (1) |
| `CSpawnerTweakConditionOwnerID` | tweaks | `CSpawnerTweakConditionOwnerID` | CSpawnerTweakCondition | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakConditionOwnerName` | tweaks | `CSpawnerTweakConditionOwnerName` | CSpawnerTweakCondition | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakEnemySize` | tweaks | - |  |  | - | 8 (1) |
| `CSpawnerTweakIgnoreCantMove` | tweaks | `CSpawnerTweakIgnoreCantMove` | CSpawnerTweakType | 0 | 1 rsc | 7 (0) |
| `CSpawnerTweakIgnoreMax` | tweaks | `CSpawnerTweakIgnoreMax` | CSpawnerTweakType | 0 | 1 rsc | 7 (0) |
| `CSpawnerTweakIncreaseMulti` | tweaks | `CSpawnerTweakIncreaseMulti` | CSpawnerTweakType | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakIncreaseY` | tweaks | `CSpawnerTweakIncreaseY` | CSpawnerTweakType | 1 | 1 rsc | 7 (0) |
| `CSpawnerTweakModifyDelays` | tweaks | `CSpawnerTweakModifyDelays` | CSpawnerTweakType | 3 | 1 rsc | 8 (1) |
| `CSpawnerTweakMustBeShooter` | tweaks | `CSpawnerTweakMustBeShooter` | CSpawnerBootTweakType | 0 | 1 rsc | 7 (0) |
| `CSpawnerTweakNoAutoPatrol` | tweaks | `CSpawnerTweakNoAutoPatrol` | CSpawnerTweakType | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakNoMulti` | tweaks | `CSpawnerTweakNoMulti` | CSpawnerTweakType | 0 | 1 rsc | 7 (0) |
| `CSpawnerTweakNoRandom` | tweaks | `CSpawnerTweakNoRandom` | CSpawnerBootTweakType | 0 | 1 rsc | 7 (0) |
| `CSpawnerTweakNullTactics` | tweaks | `CSpawnerTweakNullTactics` | CSpawnerTweakType | 0 | 1 rsc | 7 (0) |
| `CSpawnerTweakParams` | tweaks | `CSpawnerTweakParams` | CResource | 2 | 1 rsc | 8 (1) |
| `CSpawnerTweakPatrolSpeed` | tweaks | `CSpawnerTweakPatrolSpeed` | CSpawnerTweakType | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakPlacement` | tweaks | `CSpawnerTweakPlacement` | CSpawnerTweakType | 4 | 1 rsc | 8 (1) |
| `CSpawnerTweakRandomCone` | tweaks | `CSpawnerTweakRandomCone` | CSpawnerTweakType | 5 | 1 rsc | 8 (1) |
| `CSpawnerTweakReduce` | tweaks | `CSpawnerTweakReduce` | CSpawnerTweakType | 1 | 1 rsc | 8 (1) |
| `CSpawnerTweakType` | tweaks | `CSpawnerTweakType` | CResource | 1 | 1 rsc | 7 (0) |
| `CChapterInfoEntity` | world info | `CChapterInfoEntity` | SBaseEntity | 20 | 51 wld | 23 (16) |
| `CChapterInfoProperties` | world info | `CChapterInfoProperties` | CBaseProperties | 18 | 51 wld | 10 (3) |
| `CWorldInfoEntity` | world info | `CWorldInfoEntity` | SBaseEntity | 77 | 59 wld | 145 (138) |
| `CWorldInfoProperties` | world info | `CWorldInfoProperties` | CBaseProperties | 22 | 59 wld | 10 (3) |

### 4. The order rule and its exceptions

For each class the script lists the data's members in file order against the binary's declared names. **All 28 checked classes agree** (counts equal, every named member at its position, every data type fits the Hungarian prefix of the binary's name) except: (a) `SMissionObjectiveProperties`: the data names two members `ep_strObjAccomplished` and `ep_strObjFailed` where the binary declares `ep_strObjAccomplishedToSay` and `ep_strObjFailedToSay` (same positions: a rename in one of the two builds; the binary's names are used); (b) `SSpawnerEntity` has **three layouts** in the data: 59 members (9 worlds, all of the binary's), 58 (22 worlds, the last member `en_bIgnoreMax` missing) and 33 (15 worlds, the oldest levels: it lacks `en_stMultiSpawningType` and the 25 members after `en_bDeathSpawner`); a reader must default the missing members to the constructor's value. `CWorldInfoEntity` has four layouts (67, 70, 76 and 77 members) and `CWorldInfoProperties` two (19 and 22), see the world page.

### 5. Macro events raised by the family

The classes raise **named events** on themselves that level scripts wait on (`level-logic.md` counts their uses). Events found as strings in the methods read or profiled: spawner `OneSpawned`, `GroupSpawned`, `AllSpawned`, `OneKilled`, `GroupKilled`, `AllKilled`, `OnePicked`, `GroupPicked`, `AllPicked` and `SpawneeAvailable`; detector `Activated` and `CustomDetection`; external joint `Broken`; camera `MovingStarted` and `AnimationFinished`; diamond hand `Activated`, `Deactivated`, `CircleClosed` and `HalfOpened`; chapter info `Started` and `Finished`; world info `TimerFinished`, `CounterFinished`, `Booted`, `LevelFinished` and `PlayerBorn`-style events (profiled; details on the world page). Items raise `Picked` (`item-classes.md`). Events are **names hashed to identifiers**; the numbers are the engine's string-to-id function and are not needed.

## Invariants

1. 82 exported classes; 70 are in the data under their own or the `S` twin name, the external joint pair under the data name `CTestJoint...`, and 10 are in no file (script section 1): a reader must not look for the platform, alarm, spawn delayer, `CSpawnerTweakEnemySize` or the three `...Data` structures in a world or resource file.
2. Every placed entity of the family has exactly one property record of the matching class, except spawners (records owned by the spawnee) and multi-spawners (one world record and one owned record each): script section 3.
3. The 29 tweak classes occur only in `spawnertweaks.rsc` and `spawnerboottweaks.rsc` (script section 4).
4. The names of members are the binary's; where the data names a member itself the two agree.

## Open questions

Ranked by what blocks an implementer. IDs are **new** (appended to `docs/roadmap/T01-reverse-engineering.md`); each page lists its own.

1. **Spawners** (state machine, procedures, spawn effects, launchers, formations; the multiplier and the tweak application): [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md), [`mechanism-classes-spawner-tweaks.md`](mechanism-classes-spawner-tweaks.md). **T01-165**, **T01-166.**
2. **Movers** (door states and timing, platform, portals, animated movers; bouncers, barrier fields, motors, joints, the diamond hand): [`mechanism-classes-movers.md`](mechanism-classes-movers.md). **T01-167**, **T01-168.**
3. **Triggers and announcers** (detector, killer, damager and alarm details; secret, objective and beacon state values and save structures; cameras): [`mechanism-classes-triggers.md`](mechanism-classes-triggers.md). **T01-169**, **T01-170**, **T01-171.**
4. **World info and chapter flow** (chapter restore, the end sequence; enemy limits, lag, music, timers, hostility graph): [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md). **T01-172**, **T01-173.**
5. **Run-time checks on the original** (door, spawner and multiplier timing, secret and objective flows): numbers only. **T01-174.**

## Test vectors

All from the retail install via `tools/re/mechanism_classes_verify.py`, unless a binary reading is named.

1. **Counts.** 82 exported classes, 1,206 exported functions; 70 in the data (26 under an `S` twin, 44 under the `C` name); the 59 worlds hold 3,783 spawners (in 46 worlds), 1,077 detector areas (41), 825 barrier fields (54), 307 chapter infos (51) and one world info each.
2. **Order rule.** `SDoorProperties` has 17 members; the data's ids `.1 .11 .2 .3 .5 .18 .8 .9 .10 .12 .13 .14 .15 .16 .17` are, in order, `ep_pmcModel`, `ep_vStretch`, `ep_bDetectorFront`, `ep_bDetectorBack`, `ep_fExtraArea`, `ep_fSafetyOffset`, `ep_fClosingDelayTime`, `ep_idOpenAnim`, `ep_idCloseAnim`, `ep_idMechanismName`, `ep_bLocked`, `ep_smlModifiers`, `ep_fCullDistance`, `ep_fFadeLength`, `ep_bPlayersOnly`, then the two named sound members.
3. **Net.** A door, a camera, a portal and a world info entity are `NEB_STATIC` (a function returning 1); a generic motor is `NEB_DYNAMIC` (a function returning 2); a spawner has no net behaviour function (`NEB_NONE`).
