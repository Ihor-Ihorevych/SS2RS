# Level logic: triggers, doors, movers, chapters, secrets, objectives, cutscene triggers (T01-030)

- **Roadmap task:** T01-030
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the **interface** between scripts and entities (which entity class raises which events and receives which actions, with counts: every action, call, event, send and run of every macro program resolved to an entity class by the variable bindings, `tools/re/wld_logic_verify.py`), for the chapter chain and for the data of the logic entities; **medium** for what a name means (every reading below is **from the name and from where it is used**, not from run-time behaviour); **low** for timing and ordering (when an event fires, how long an action takes, how `Recharge` re-arms a detector): that needs the instrumentation tasks (T01-038/039). Scope: all 59 worlds and the 8 `.mac` files.

Prerequisites: `wld-macro.md` (the language), `wld-macro-vocab.md` (all names), `wld-entities.md` (the entities), `wld-physics.md` (doors and movers are mechanisms), `ai.md` (spawners and characters), `cutscenes.md` (movies), `help-digest.md` (the editor documentation of chapters).

## Summary

Level logic has **three layers**, all data:

1. **Sensors**: entities that raise named **events**: detector areas (`Activated`), spawners (`SpawneeAvailable`, `AllKilled`, …), characters (`Died`, `Damaged`, marker events), usable and destructible models (`Used`, `Destroyed`), chapters (`Started`, `Finished`), animation events of movers and cameras, and **user events** that macro entities send to each other (`Chapter002Ended`, `StartNextLevelLink`).
2. **Macro programs** (`wld-macro.md`): `On (event(Sensor.Event)) do { … }` handlers and function bodies that **wait** and call **actions** on entities.
3. **Actuators**: entities with **actions**: spawners, doors, barrier fields, movers, models, effects, sounds, objectives, secrets, the world info (music, timers, boss registration) and the player (Netricsa messages).

There is **no separate trigger system**: a "trigger" is a detector area entity raising `Activated` that a handler waits for (963 uses in 40 worlds), and "scripted sequences" are macro functions that wait and call actions. The complete interface (68 entity classes, every name with its use counts) is generated in **`level-logic-api.md`**.

## Evidence

`tools/re/wld_logic_verify.py` (read-only, `SS2_DATA_DIR`; about 20 minutes) over the retail install with all patches:

- **Resolution of the interface.** Object expressions resolve to an entity class through the macro entity's variable bindings (`bound`), through `This` (the function's class or the macro entity), or through a declared variable type (`hint`, shown as `~Class` and only a declared base-class name such as `CLeggedCharacterEntity`): of 5,453 actions 4,075 are bound (75 %), 490 `This`, 592 hinted and 296 unresolved; of 3,120 async actions 2,734 / 276 / 71 / 39; of 5,470 events 4,937 / 448 / 58 / 27; of 1,375 calls 1,114 / 50 / 22 / 105 unresolved and 84 on `NULL`.
- **Names.** Of 6,807 variables that hold one entity handle, 2,837 (42 %) are named like the entity's editor name (`CEntityProperties.4`): scripts do not rely on names matching.
- **Chapters.** 51 worlds have chapter info entities (307 in all, 1 to 12 per world); in all 51 the world info names the first chapter (`CWorldInfoEntity.2`) and the chapters form a **linear chain** (each is the next of at most one chapter, `CChapterInfoEntity.2`); 296 of the 307 are reached from the first by the next handle.

## Sensors and the events they raise

Counts are uses in scripts / worlds that use them (`level-logic-api.md` has all names).

| Entity class | Events (uses/worlds) |
|---|---|
| `CDetectorAreaEntity` (1,077 entities) | `Activated` 963/40, `CustomDetection` 101/17 |
| `SSpawnerEntity` (3,783) | `SpawneeAvailable` 512/41, `AllKilled` 330/35, `OneKilled` 196/30, `OnePicked` 154/31, `GroupKilled` 97/25, `GroupSpawned` 49/14, `AllSpawned` 41/15, `OneSpawned` 27/12, `AllPicked` 18/10 |
| `CChapterInfoEntity` (307) | `Started` 680/42, `Finished` 455/32, `Ended` 71/7, `LevelFinished` 13/7, `Booted` 12/2, `Interrupted` 8/1, and many level-specific names |
| legged characters (declared type `CLeggedCharacterEntity`, class `SLeggedCharacterEntity`) | `Died` 57+22, `Damaged` 35+7, `LastPatrolMarkerReached` 31, `LastTacticMarkerReached` 22+12, `MarkerReached` 21, `TacticMarkerReached` 11, `Idle`, `UpClose`, `HealthUnder75/50/10`, `Invulnerable`, `Recharging`, `RechargingOver` |
| `CStaticModelEntity` (25,383) | `Used` 55/22, `Destroyed` 44/16, `Damaged` 18/7, `BossStartedRecharging` 4, `BossStoppedRecharging` 4 |
| `SAnimatedMoverEntity` (277) | level-specific **animation events** (`Lift_Down` 22, `Lift_Up` 16, `AnimFinished` 8, `Lift_Start` 6, …): names set in the mover's animation (`CSamMacroEvent`, `wld-sound-effects.md`) |
| `SCameraEntity` (76) | `AnimationFinished` 10/6, `UserBreak` 6/4 |
| `CWorldInfoEntity` (59) | `TimerFinished` 27/3, `DiamondDestroyed`, `PlayerBorn` |
| `SMissionObjectiveEntity` (131) | `ActivateMe` 25/9, `DeactivateMe` 21/9 (sent by scripts) |
| `CSecretEntity` (110) | `Found`, `Ended` (1 use each) |
| macro entities (`CMacroEntity`) | user events: `Chapter001Ended` … `Chapter008Ended`, `StartChapter002` …, `StartNextLevelLink` 9/9, `Crush` 14, `Idle` 14 |

Events are **names**: the sensor entity decides when it raises one (the detector when a player enters its primitive, the spawner when the spawnee exists or the group is dead, a mover when its animation passes the event). Scripts also raise events: `Send event Obj.Name` (884 uses, `wld-macro-vocab.md`; most used `Finished` 43, `AllKilled` 33, `Ended` 28, `ActivateMe` 27, `DeactivateMe` 23, `AllEnemiesKilled` 19). `any`/`all` combine several events (`wld-macro.md`).

## Actuators and their actions

| Entity class | Actions (sync + async uses) |
|---|---|
| `SSpawnerEntity` | `SpawnSimple` 882 + 588, `SpawnMaintainGroup` 253 + 191, `SpawnGroup` 228 + 226, `SpawnOne` 48 + 18, `RespawnByGroup` 39 + 13, `PauseSpawning` 13 + 3, `SetSpawnSource` 9 + 38; call `GetLastSpawned` 514 |
| `CDetectorAreaEntity` | `Recharge` 147 + 136 (re-arm), `RegisterCustomDetection` 121 + 5, `Deactivate`, `UnregisterCustomDetection` |
| `SDoorEntity` (176) | `Unlock` 38 + 42, `Lock` 26 + 15, `AssureOpened` 22 + 13, `Open` 18 + 12, `AssureLocked` 5 + 14, `EnableUsage` 4 |
| `SBarrierFieldEntity` (825) | `Disable` 77 + 18, `Enable` 25 + 6 |
| `SAnimatedMoverEntity` | `PlayAnim` 115 + 25, `PlayAnimWait` 38 + 101, `PlayAnimLoop` 22 + 53, `Start` 8, `Stop` 7 + 54 |
| `CStaticModelEntity` | `macDestroyOnePhase` 149 + 48, `Disappear` 60 + 42, `PlayAnimStay` 49 + 32, `EnableUsage` 37 + 12, `PlayAnimLoop` 28 + 11, `Appear`, `StopAllAnims` |
| `CGenericMotorEntity` (43) | `SetForceX/Y`, `SetPositionX/Y/P`, `SetVelocityX/Y`, `SetTorqueP`, `SetAngVelocityP` (motors of mechanisms, `wld-physics.md`) |
| `CEffectEntity` | `Start` 144 + 78, `Stop` 78 + 23, `DisableEmitting` 10, `Terminate` |
| `SStaticSoundEntity` | `PlayOnce` 69 + 192, `PlayLooping` 27 + 37, `Stop` 15 + 30, `StopLooping` 14 + 7, `PlayOnceWait` |
| `SMissionObjectiveEntity` | `Activate` 55 + 56, `MarkDone` 48 + 58 |
| `CSecretEntity` | `MarkFulfilled` 88 + 23 |
| `CPortalEntity` (54) | `Activate` 41 + 15, `Deactivate` 5 |
| `CChapterInfoEntity` | `Start` 249 + 6 |
| `CWorldInfoEntity` | `SetMusic` 11, `ForceMusic` 10 + 18, `SetMusicVolume` 3, `ActivateTimer` 12, `StopTimer` 3, `RegisterBoss` 6 + 1, `macAddScore` 6, `TeleportPlayersToCurentChapter` 2, `macResetWorldToCurrentChapter`, `macTheEndSequence` 2; calls `GetAllPlayersInRange` 243, `GetClosestPlayer` 83, `GetCharacters` 32 |
| `SCameraEntity` (76) | `PlayAnimWait` 16 + 39, `EnableUserBreak` |
| player puppet (declared type `CPlayerPuppetEntity`) | `SayNetricsa` 86, `HideNetricsaIcon` 82, `ShowNetricsaIcon` 82, `SayPlayer` 54 + 7, `SayPuppet`, `ClearConversation`, `macTeleport`, `DropDead` 13 |
| characters (declared `CLeggedCharacterEntity`) | `macSetThreatSensitivity` 58 + 29, `BeInvulnerable` 33 + 41, `ReportDamage` 32 + 33, `macSetHealth` 32 + 19, `SetTacticEntity` 26 + 27, `ChangeState` 23, `macSetLookTarget` 21, `PlayAnimWait` 20, `PatrolMarkers` 9 + 20, `AttackMelee`, `AttackShoot`, `macPickTool` 18, `macDrawTool`, `macThrowToolAtTargetEntity` |
| macro entity | `StartFunction` 189 + 214 (run a function of another macro entity) |

## Patterns (readings from names and usage)

- **Chapters and level flow.** A chapter is started by `Chapter.Start` (249 uses) and raises `Started` and `Finished`; the macro entity of the chapter chain sends `ChapterNNNEnded` and `StartChapterNNN` (`Chapter001Ended` in 9 worlds), and the user event **`StartNextLevelLink`** (9 worlds) is, by its name, the end of the level; what actually loads the next level is not stated by the data (`CPortalEntity`, 54, receives `Activate` 41 times). The world info names the first chapter; macro functions named `ChapterControl` (25) or `Chapter001` (15) are the usual chapter bodies. Level movies (`cutscenes.md`) are played by the game after a level ends, not by these scripts.
- **Triggers.** A detector area `Activated` starts a handler; the handler usually calls `Recharge` on the detector (147 + 136 uses) to re-arm it, so a detector is one-shot unless recharged; `CustomDetection` with `RegisterCustomDetection` lets a script choose what a detector senses (`en_bCustomEventDetectorArea`, `en_eaCustomDetections`).
- **Doors and barriers.** Doors are `SDoorEntity` mechanisms (`wld-physics.md`) with `Lock`/`Unlock`/`Open`/`AssureOpened`/`AssureLocked` and a usage event `Used` (5); barrier fields are enabled or disabled (`Disable` 95 uses): the way scripts block and open areas.
- **Movers and elevators.** Elevators and platforms are `SAnimatedMoverEntity` playing a named animation (`PlayAnim`, `PlayAnimWait` 139 uses, `PlayAnimLoop`); their level-specific events (`Lift_Down`, `Lift_Up`, `WallClose`, …) come from the animation; free motion uses `CGenericMotorEntity` (`SetForce…`, `SetVelocity…`).
- **Spawning waves.** A handler waits on `SpawneeAvailable` and takes `GetLastSpawned`, then configures the spawnee (threat sensitivity, health, tactic, invulnerability) and runs a function on it (the most used run target is the function `Fight`: 132 uses); `AllKilled`/`OneKilled`/`GroupKilled` end waves (`ai.md`).
- **Secrets, objectives, music.** `CSecretEntity` (message, speech sound, "happy music" members) is filled by `MarkFulfilled` (111 uses); objectives by `Activate` and `MarkDone` (`wld-macro-vocab.md`); `SetMusic`/`ForceMusic` on the world info select the dynamic music (`audio.md`); Netricsa messages and icons are player actions (`SayNetricsa`).
- **Cutscene triggers.** In-engine cutscenes are `SCameraEntity` animations (`PlayAnimWait`, `EnableUserBreak`, events `AnimationFinished` and `UserBreak`) started by handlers; the camera count is 76 over the levels (`cutscenes.md`).
- **Bosses.** `RegisterBoss` on the world info and the events `BossStartedRecharging`/`BossStoppedRecharging` of the boss model mark boss fights; the per-boss logic is T01-048.

## The logic entities' own data (`wld-entity-schema.md`)

`CDetectorAreaEntity` (a primitive hull, a renderable, an event flag, a container of custom detections), `SDoorEntity` (a model renderable, a mechanism, a hull, door flags, current animation), `CSecretEntity` (message text, message stay time 3 to 6, a speech sound, happy music, state, finder handle), `SMissionObjectiveEntity` (texts and sounds for added and accomplished, a state, scanner visibility), `SKillerEntity` (a hull, instant kill, damage period and amount 1 to 100,000, an "abyss" flag), `CBouncerEntity` (bounce direction and speed 10 to 500, air control, an auto-activate flag), `CChapterInfoEntity` (start inventory, `wld-entities.md`).

## Reader invariants

1. Treat scripts as the only trigger system: build the interface from the sensor events and actuator actions; keep names as strings.
2. Resolve an object expression through the macro entity's bindings; `This` is the function's class or the macro entity; an unbound variable with a declared type is only a type hint.
3. A detector raises `Activated` once until a script calls `Recharge` (reading; confirm by measurement).
4. Load the chapter chain from the world info's first chapter and `CChapterInfoEntity.2`; 11 chapters of the retail data are not on that chain (not analysed).
5. An entity class that gets an action it does not implement must fail loudly in tests: the API file lists what the retail data uses.

## Open questions

- **Semantics and timing:** when and how often each event fires, what `Recharge`, `Assure…`, `MarkDone` and `macDestroyOnePhase` do, whether `Wait` conditions are re-evaluated every tick (T01-038/039, T01-022).
- The 11 chapters off the chain, the 296 unresolved actions and 207 unresolved runs (expressions through calls and loops), and the declared-type classes (`~CLeggedCharacterEntity` are base class names of the editor and map to `SLeggedCharacterEntity` and relatives).
- The per-entity details of the weapons, enemies, bosses and vehicles are T01-028, T01-029, T01-047 and T01-048.
