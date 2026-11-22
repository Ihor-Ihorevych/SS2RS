# `.wld` entities and their property records (T01-012d)

- **Roadmap task:** T01-012d (part d of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class list and counts, the entity/record model, the spawner structure, the parent/child links between records and the links to `.ep` files (all checked on every world with `tools/re/wld_entity_verify.py`; the member lists are generated from the files' own type tables, see `wld-entity-schema.md`); **medium** for the family descriptions (class names and value ranges, not behaviour); **low** for what individual members do (about 60 % of the members have only numbers for names). Scope: all 59 distinct worlds of the retail install with patches.

Prerequisites: `wld-root.md` (the entity list, serials, class codes, the property record layout), `wld-geometry.md`, `wld-lighting.md`, `wld-visibility.md`, `ep.md` (the parameter files the entities link to).

## Summary

- A world holds **73,098 entities of 94 classes**. Each entity is a runtime object (members prefixed `en_`); its **property record** is the editor's description of the same thing (members prefixed `ep_`, a class `…Properties`). **69,177 entities have a record** in `CWorld.5` that names them; the other 3,921 have none: **3,783 spawners**, **133 bullet shooters** and 5 effect entities.
- **A spawner is not a free-standing object.** The 3,783 `SSpawnerEntity` have no serial and no world record. Each belongs to exactly one other entity (a character, an item, a multi-spawner or a prop): the spawner entity points at that entity (`.120`) and at a **copy of its property record** (`.21`), and the entity's own record points back at a `SSpawnerProperties` record (the spawner's settings). What a spawner does at run time is not stored.
- **Entities link to `.ep` parameter files** through six members: puppet parameters (`ep_pppPuppetParams` / `en_pppPuppetParams`), item parameters (`SBaseItemEntity.503`), weapon pickup parameters (`en_pbwpWeaponParams`, `en_pbwpWeaponRenderParams`) and the resource lists of the world and chapter info entities. 11,420 links, all resolve to an `.ep` file of the install.
- **The editor hierarchy of entities** (parent, first child, next sibling) is stored in the records (`CEntityProperties.5/.6/.7`) and is consistent in every world.

## Evidence

`tools/re/wld_entity_schema.py` (generates `wld-entity-schema.md` and counts the classes) and `tools/re/wld_entity_verify.py` (relations), read-only, over the retail install with all patches, 59 worlds:

- **94 entity classes** (73,098 entities, equal to the length of all `CWorld.1` lists) and **93 property record classes** (77,011 record objects in all: 69,177 named by an entity, 3,913 owned records in the extra section without an entity (3,783 `SSpawnerProperties` and 130 others), and 3,921 in the main object section). The schema reference lists 241 classes (entities, records, their bases and the 27 structures nested in them) with every member.
- **Spawners:** 3,783 of 3,783 have no serial; 3,783 `.120` targets are entities that have a record in `CWorld.5`; 3,783 `.21` targets are records that are not in `CWorld.5` and have the class of the target entity's own record; all 3,783 `SSpawnerProperties` records are referenced exactly once and have `CEntityProperties.2` null.
- **Records, parents and children:** 13,766 records have a parent (`.5`). For the 3,958 parents that are world records, the chain `.6` (first child) then `.7` (next sibling) lists exactly the records that name that parent, in every case; no chain is cyclic.
- **`.ep` links:** 2,328 of 2,328 puppet entities have the same `.ep` target as their record; 228 of 228 weapon pickups follow the naming rule below; 2,140 of 2,730 item entities carry an item parameter link.

## The entity and its record

| | Entity (`CEntity`) | Property record (`CEntityProperties`) |
|---|---|---|
| Section | the world's objects (listed in `CWorld.1`) | extra section after `EDOB` (listed in `CWorld.5`) |
| Members | runtime state, `en_…` (`CEntity.1` serial, `.2` class code, …) | design data, `ep_…`; header `CEntityProperties.1` (class code), `.2` → the entity, `.3` `QuatVect`, `.4` name, `.5/.6/.7` hierarchy, `.8` entity id, `.9` platform; `CBaseProperties.ep_ulEntityIDVis` |
| Count | 73,098 | 69,177 world records |

- The **header of every record** is the same (`wld-root.md`); `ep_ulEntityIDVis` equals the serial in 51,703 of 51,704 records that have it (`wld-visibility.md`).
- The **record class** is the entity class with `Entity` replaced by `Properties` in 89 of the 92 classes that have world records; the exceptions are `SLocatorEntity` (`CLocatorProperties`), `CForceFieldEntity` (`CForceFieldEntityProperties`) and `CFluidEntity` (`CFluidEntityProperties`). `SSpawnerEntity` and `SBulletShooterEntity` have no world record class; `SSpawnerProperties` is only used for owned records.
- **Values appear twice.** The entity holds the runtime copy of what the record holds (the light source and its record in `wld-lighting.md`; a puppet's `.ep` link below). A reader that only needs the design data can read the records; one that needs runtime state reads the entities.
- **Parent, child, sibling.** `CEntityProperties.5` → the parent record, `.6` → the first child, `.7` → the next sibling (all null when absent): 13,766 children, 3,966 parents. These are links between world records (3,958 of the 3,966 parents are world records). The editor hierarchy follows the aspect tree closely but not exactly: of 33,870 aspects of child entities that have a parent aspect, 12,714 have a parent aspect owned by the parent entity and 32,705 (96.6 %) have an ancestor aspect owned by it.
- **`SBaseProperties.1`** is a handle to a **chapter info record** (`CChapterInfoProperties`): 3,279 non-null references (in the village world all from characters and one item; chapters are below).

## Spawners

```
SSpawnerEntity (3,783)        no serial (0xFFFFFFFF), no record in CWorld.5
  .120  -> the spawnee entity           (a character, an item, a multi-spawner or a prop; has a record in CWorld.5)
  .21   -> a property record of the same class as the spawnee's own record, not in CWorld.5 (a template copy)
  en_…  runtime settings: delay, a launch cone (heading, pitch) and velocity range, health, multiplier, state, counters, …
spawnee's own world record
  SPuppetProperties.10 | SBaseItemProperties.3 | SMultiSpawnerProperties.1 | SPropProperties.50
        -> SSpawnerProperties (owned record, CEntityProperties.2 null; ep_bGroupSpawner, ep_bDisableSpawnSound, ep_sfcSpawnEffectConfig, …)
```

- **Spawnees:** 2,195 characters, 1,492 items, 60 multi-spawners (`SMultiSpawnerEntity`) and 36 props; the 3,783 `SSpawnerProperties` records are referenced exactly once each, by that member of a world record.
- **The template copy is a copy.** Of the 135,480 members of the 3,783 template records, 113,685 (83.9 %) equal the spawnee's own record; the differences are the entity pointer (`.2`), the platform (`.9`), the spawner link itself, `SBaseProperties.1` (2,236) and, for all 2,195 characters, four puppet members (`SPuppetProperties.34`, `ep_maPatrolMovingAbilities`, `ep_bPassTroughBarriers`, `.10`); 1,041 also differ in the tactic entity. Which of the two a spawn uses is a run-time question not answered by the data.
- **Bullet shooters** (`SBulletShooterEntity`, 133) have no record at all; their members are in the entity.

## The entity classes

Counts are entities over all 59 worlds. The schema reference has every member; the text describes only what the data supports.

| Family | Classes | Entities | Notes |
|---|---|---|---|
| Placed models and moving parts | 14 | 29,022 | `CStaticModelEntity` 25,383 (the level geometry, `wld-geometry.md`), `SPropEntity` 1,726, `SBarrierFieldEntity` 825, `SAnimatedMoverEntity` 277, `SCrumbsEntity` 186, `SDoorEntity` 176, `SKillerEntity` 104, `CTestJointEntity` 93, `CBouncerEntity` 71, `CPortalEntity` 54 (a model with a mechanism and animation members, see `wld-visibility.md`), `CGenericMotorEntity` 43, `CForceFieldEntity` 38, `CFluidEntity` 23, `CEnvironmentDamagerEntity` 23 |
| Lights, effects and sound | 11 | 17,220 | `CPointLightEntity` 11,349 and `CDirectionalLightEntity` 89 (`wld-lighting.md`), `SFlareEntity` 2,501, `CEffectEntity` 2,352 (particle effects, `pfx.md`), `SStaticSoundEntity` 610, `CEnvSFXEntity` 136, `CHazeEntity` 65, `CCubeBackgroundEntity` 58, `CBloomEntity` 34, `CTextEffectEntity` 14, `SShakerEffectEntity` 12 |
| Markers, navigation and tactics | 7 | 14,592 | `SPathMarkerEntity` 10,639, `CFormationMarkerEntity` 2,538, `STacticEntity` 993, `CSpawnMarkerEntity` 177, `SNavigationalBeaconEntity` 174, `CNavMeshMarkerEntity` 54, `SLocatorEntity` 17 |
| Characters (puppets) | 5 | 2,328 | `SLeggedCharacterEntity` 1,825, `SAircraftCharacterEntity` 254, `STurretCharacterEntity` 164, `SRollingBallCharacterEntity` 44, `SHovercraftCharacterEntity` 41; all derive from `SPuppetEntity` |
| Spawners | 3 | 3,976 | `SSpawnerEntity` 3,783, `SBulletShooterEntity` 133, `SMultiSpawnerEntity` 60 |
| Triggers, scripting and level logic | 6 | 2,850 | `CMacroEntity` 1,094 (script programs, part g), `CDetectorAreaEntity` 1,077, `SVisBoxEntity` 362, `SMissionObjectiveEntity` 131, `CSecretEntity` 110, `SCameraEntity` 76 |
| World and chapter | 2 | 366 | `CChapterInfoEntity` 307, `CWorldInfoEntity` 59 (exactly one per world) |
| Items (pickups) | 45 | 2,730 | health 687 (5 sizes), armor 478 (6), ammunition 922 (9), weapons 228 (12), treasure and score 290 (4), power-ups and others 125 (9); all derive from `SBaseItemEntity` |
| Other | 1 | 14 | `CDiamondHandEntity` (one world) |

Features the data shows for the families (details in the schema):

- **Characters.** The record has `ep_pppPuppetParams` (the `.ep`), a puppet name, behaviour and default state identifiers (12 states in the data), movement overrides (speed multipliers, allowed range, fall height, step up/down), sense and sight overrides, tactic and death-spawner handles, mounts and flags such as `ep_bDisableNavigation`; the spawner link is `SPuppetProperties.10`.
- **Items.** `SBaseItemProperties` has only the spawner link (`.3`) and three flags; the entity carries the item parameter link (`.503`). Weapon pickups (`SWeaponItemEntity`: 228) add the two weapon links.
- **Detector areas** (1,077) hold a primitive shape (`CPrimitiveDesc`), colours and an event flag (`ep_bCustomEventDetectorArea`); a detector can name a base record (3 do). **Barrier fields** (825) hold a model, a part name and a stretch. **Tactic entities** (993) hold a character-mover handle, the next tactic, a marker, an auto-off time and distance (−1 for none) and a colour. **Path markers** hold only the next marker handle and a colour; **formation markers** a formation pointer (70 set).
- **Chapter info** (`CChapterInfoEntity`, 307 in 51 worlds) describes what the player starts with: description, health 50 to 200, armor, a weapon flag set, a preferred weapon, an ammunition container and a list of weapon resource holders. **World info** (`CWorldInfoEntity`) holds the runtime containers of the world (enemies alive, spawn slots, entities that need rendering) and the path-finding service.

## Links to `.ep` files

| Member | Where | Links | Target root class (`ep.md`) |
|---|---|---|---|
| `SPuppetProperties.ep_pppPuppetParams` | character records | 4,783 (incl. spawner templates) | `SLeggedPuppetParams`, `SAircraftPuppetParams`, `STurretPuppetParams`, `SRollingBallPuppetParams`, `SHovercraftPuppetParams`, `SGenericLeggedPuppetParams`, `SPlayerPuppetParams` |
| `SPuppetEntity.en_pppPuppetParams` | character entities | 2,328 | the same: equal to the record's link in 2,328 of 2,328 |
| `SBaseItemEntity.503` | item entities | 2,140 of 2,730 items | `CBaseItemParams` (all) |
| `SWeaponItemEntity.en_pbwpWeaponParams`, `.en_pbwpWeaponRenderParams` | weapon pickups | 228 each | `S<name>WeaponParams` and `S<name>WeaponRenderingParams` for the pickup class `S<name>ItemEntity` (228 of 228) |
| `CResourceOnDemandHolder.rodh_prsResource` | in `CWorldInfoEntity` and `CChapterInfoEntity` lists | 1,713 (of 2,067 holders) | weapon parameter and rendering parameter files: the weapons a world or chapter loads |

The link members are the only `.ep` references in worlds (11,420 in all: 9,707 in entities and records, 1,713 in the holders); all resolve. The link is an **external reference** of the world file (an `EXOB` entry with a resource file); the class of the root object of the target is the one in the table.

## Reader invariants

1. Take entities from `CWorld.1` and records from `CWorld.5`; the entity's class is its object type.
2. An entity can lack a record: spawners and bullet shooters always, 5 effect entities. Do not treat that as an error.
3. A spawner is read through its spawnee: follow `.120`; the spawner's settings are in the spawner entity (`en_`) and in the `SSpawnerProperties` referenced from the spawnee's world record.
4. Read the hierarchy from `CEntityProperties.5/.6/.7` of world records; do not rebuild it from the aspect tree.
5. For `.ep` links use the entity or its record (equal for puppets); for weapon pickups derive nothing from names: read both links.
6. Fail on an unknown entity class; the 94 classes of the retail data are the full list.

## Test vector

`Content/SeriousSam2/Levels/01_MDigBo/1_3_Village/1_3_Village.wld`: 32 `SSpawnerEntity`, each with serial `0xFFFFFFFF`; the first has `.120` → an `SLeggedCharacterEntity` (object 134) and `.21` → an `SLeggedCharacterProperties` record (object 2481, not in `CWorld.5`), `en_iHealth` 100, `en_fMinDelay` 1.0, `en_fMinPitch` 15, `en_fMaxPitch` 45, `en_fAngleH` 90, `en_fMinVelocity` 10, `en_fMaxVelocity` 15, `en_strSpawningPuppetName` `No Name`; of the 32 spawners 29 point at `SLeggedCharacterEntity`, 2 at `SHealthItemMEntity` and 1 at `SShellsItemEntity`. The `SSpawnerProperties` of the first is an extra record with `CEntityProperties.2` null, `.2` = 1, `.1` = 1, `ep_bDisableSpawnSound` = 1. The same world has 600 world records, 7 of them `SVisBoxProperties` (`wld-visibility.md`).

## Open questions

- What each member of the 241 classes does: the schema gives names, types and value ranges only; about 60 % of the members have number-only names. Tracked as T01-063 (gameplay semantics per family, with the spawn, script and AI specs T01-027, T01-030, T01-034).
- When the engine uses a spawner's template record instead of the live record, and how spawners re-create entities.
- The 130 owned records in the extra section that are neither spawner records nor world records (their class mix was not analysed), the 8 parent links whose target is not a world record, and the 5 effect entities without a record.
- `CEntityProperties.8` (equals the serial where present) and `CBaseEntity.1` flags (`wld-root.md`).
