# Chapter restore: boot types, the reset sequence and the boot rule (T01-172)

- **Roadmap task:** T01-172 (follow-up of `mechanism-classes-world-info.md` open question 1). Companion page: [`chapter-restore-players.md`](chapter-restore-players.md) (what the player brain restores for each boot type, the chapter save, level end and `TheEndSequence`). Classes and members: [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md). Iterator classes: `helper-classes-iterators.md`.
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the two enumerations (the binary's value tables, read twice, and every caller agrees), for the 12 x 4 boot type table (read as data) and for the data counts (script `tools/re/chapter_restore_verify.py`); **medium** for every rule below (one reading of the code each; the lost arguments of calls were recovered from the instruction listing); **low** for the points marked "open". **Nothing was run on the original game.**

Prerequisites: [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md), `level-logic.md` (`Chapter.Start`, the events `Started`, `Finished`, `Booted`), `save-game.md` section 6 (the chapter save), `helper-classes-iterators.md` (section 2.3 to 2.5), `players-session-classes.md` (section 7.1: the enumerations were first listed there).

## Summary

1. **A "restore" never reloads the level file.** The world info shuts every entity down, clears all pending thinks, then **boots the entities again from their stored properties**, in dependency order; entities that belong to an **earlier chapter** than the one being booted are shut down and **not** booted (they are booted on demand later). Section 3.
2. **Two enumerations steer it.** The **boot type** (`ChapterBootType`, 12 values 0 to 11) says *who asked* and decides how the player's loadout, lives, score and counters are rebuilt (companion page) and whether secrets, objectives and beacons are restored; the **world resetting type** (`WorldResetingType`, 0 to 3) says *which kind of reset* it is. Section 1.
3. **Who passes what.** New level, first start: boot type from `PrepareChapterToStart` (**0** for a new game or a custom start, **1** for "Continue playing"), resetting type **3**. Arriving from another level: boot type **10**, resetting type **1**. Death in single player: **2** with resetting type **2**. The in-game "Restart chapter": **3**, the in-game "Restart level": **5** (with the first chapter, resetting type 3). Section 2.
4. **`Chapter.Start` is not a restore.** In the retail game it activates the chapter (it becomes the current one, earlier chapters are marked finished, the chapter save is made) and, for a chapter with a next level, changes the level. It does **not** reset the world. Section 5.
5. **Items that are gone for good.** On a respawn after death (boot type 2) the entities whose ids are in the chapter save's "unable to resurrect" list (placed treasure and extra life items that were picked) are **skipped by the boot**: they stay gone; every other placed item comes back. Section 3.3.

## Evidence

- **Code (Sam2Game.dll; the classes' own exported names).** `CWorldInfoEntity::ResetWorldToChapter` (the RPC form), `ResetWorldToChapter_internal`, `ResetWorldToCurrentChapter`, `...ToFirstChapter`, `...ToCustomChapter`, `...ToNextChapter`, `...ToPreviousChapter`, `ResetLevelToChapter`, the three `macResetWorldTo...`, `PrepareForTemplates`, `SetCurrentChapter`, `ActivateChapter`, `IsChapterBefore`, `IsChapterBetween`, `GetChapterIndex`, `GetNextChapter`, `GetPreviousChapter`, `GetChapterByName`, `GetChapterInWorld`, `OnWorldInfoPreShutDown`, `OnWorldInfoPostShutDown`, `OnWorldInfoPostBoot`, `AssureThatIsBooted`, `PrepareChapterToStart`, `OnSimStart`, `PostWorldChange`, `DefaultState`, `ResetCamera`, `StoreFavoriteChapter`, `RestoreFavoriteChapter`, `ShouldRestoreSecrets`, `ShouldRestoreGameObjectives`; `CChapterInfoEntity::Start`, `OnTrigger`, `OnBoot`, `OnShutdown`, `MarkStarted`, `MarkFinished`; `COnBootIterator`, `COnShutDownIterator`, `CDisableMacrosIterator`, `CExecuteMacrosIterator` (`ProcessEntity` of each; the boot iterator's condition was read in the instruction listing, because the decompiler printed a different grouping); the engine's `wldIterateEntitiesSorted`; `CPlayerBrainEntity::OnBoot`, `ExecuteRespawn`; `CGameInfoEntity::OnBoot`, `DisableChapterResurrection`; `CBaseItemEntity::OnPicked`; the menu handlers `CMSInGameMenu::btnRestartChapter_OnYes`, `btnRestartLevel_OnYes`, `btnContinuePlaying_OnYes`; the single-player start function; the `OnBoot` and `OnShutDown` of about 40 entity classes (section 4). The two enumerations are static tables of the DLL (value, name, display text); the 12 x 4 table is plain data in the DLL's data section.
- **Member names** come from the binary's member tables (`en_henFirstChapter`, `en_henCurrentChapter`, `en_henChapterInBooting`, `en_tmWorldStart`, `en_bResetChapterInProgress`, `en_ewrtWorldResetingType`, `en_cbtChapterBootType`, `en_henGameInfoEntity` of the world info; `en_hslsStoredLevelStatus`, `en_sgpGameParams` of the game info; `scs_*`, `sls_*`, `ps_*`, `gs_*`, `sgp_*` of the stored records, found by matching each name to the byte position the code uses).
- **Data.** `python -I tools/re/chapter_restore_verify.py` (about 8 minutes): chapter chains of all 59 worlds, the chapter names, the entities that carry a chapter link by class and by chapter position, the placed treasure and life items.
- **Not checked.** Nothing was run; the macro interpreter's step, the sorted order for real worlds, the exact moment events are delivered, the music crossfade, the per-class `OnBoot` beyond the short descriptions of section 4, who calls `PrepareForTemplates`.

## Layout / rules

### 1. The two enumerations (high)

**`ChapterBootType`** (value, name; the display text of the binary's table is the same as the name in every row that has one):

| Value | Name | Passed by (section 2) |
|---|---|---|
| 0 | `CBT_CUSTOM_LEVEL_CUSTOM_CHAPTER` | `PrepareChapterToStart` for a new game, a custom level, a start chapter named by option, a dedicated start; `ResetWorldToCurrentChapter(0)` from the simulation reset |
| 1 | `CBT_MAIN_MENU_CONTINUE_PLAYING` | `PrepareChapterToStart` when the main menu's "Continue playing" found a stored level status |
| 2 | `CBT_RESPAWN_AFTER_DEATH` | `CPlayerBrainEntity::ExecuteRespawn` when the session is local |
| 3 | `CBT_INGAME_MENU_RESTART_CHAPTER` | in-game "Restart chapter"; the macro `macResetWorldToCurrentChapter` |
| 4 | `CBT_GAME_OVER_MENU_CONTINUE_PLAYING` | **no caller found** (only the table row exists; the menu's "Continue playing" only sets the game info's frozen-statistics flag) |
| 5 | `CBT_GAME_OVER_MENU_RESTART_LEVEL` | in-game "Restart level" (first chapter, resetting type 3) |
| 6 | `CBT_COOP_RESPAWN_PLAYER` | a networked co-op respawn: **only that player's brain is booted** with this type, no world reset |
| 7 | `CBT_COOP_RESTART_LEVEL` | **no caller found** |
| 8 | `CBT_COOP_JOIN` | a player who joins (every player slot but the last, see 2.2) |
| 9 | `CBT_DEVELOPER_CUSTOM_CHAPTER` | `ResetWorldToFirstChapter`, `ResetWorldToCustomChapter` (the developer keys and `ResetLevelToChapter`), `CChapterInfoEntity::Start` when its global flag is set |
| 10 | `CBT_LEVEL_CHANGED` | `PostWorldChange` (arrival from another level) |
| 11 | `CBT_BOOT_TEMPLATES` | `PrepareForTemplates` |

**`WorldResetingType`** (the name is spelled so in the binary): **0 `WRT_INVALID`** ("Invalid"), **1 `WRT_LEVEL_CHANGED`** ("Level changed"), **2 `WRT_CURRENT_CHAPTER`** ("Current chapter"), **3 `WRT_CUSTOM_CHAPTER`** ("Custom chapter"). A reset that is started with 0 logs "Type of world reseting must be set prior to calling ResetWorldToChapter()." and **uses 3**.

**Four per-type lists decide which boot types restore what** (all from small functions; **medium**, they agree with `save-game.md`):

- secrets are restored (`ShouldRestoreSecrets`) for boot types **0, 1, 3, 5, 7, 9, 10** (not for 2, 4, 6, 8, 11);
- mission objectives and navigation beacons are restored (`ShouldRestoreGameObjectives`) for **1, 2, 3, 4** only;
- the game totals are recalculated first (`CalculateGameTotalStats`) for **0, 5, 7, 9, 10**, and for **1 when the chapter being booted is the world's first chapter**;
- the player rebuild table (companion page, section 1): the 12 rows.

So after a **respawn (2)** objectives and beacons come back from the chapter save but **secrets do not** (a found secret stays found: it is simply not touched, because the secret entities were not asked to restore); after a **chapter restart (3)** both are restored; after a **level restart (5)** secrets are restored and objectives are **not** (they are booted fresh from the level file).

### 2. Who starts a reset

#### 2.1 The entry functions

`ResetWorldToChapter(chapter, boot type, resetting type)` is virtual and the **only** route: on the host it sends the call to every machine as a remote call and then runs `ResetWorldToChapter_internal` itself; on a client it runs the internal function **only when it arrives by that remote call** (a client's own call does nothing). The helpers (all pass to it):

| Function | Chapter | Boot type | Resetting type |
|---|---|---|---|
| `ResetWorldToCurrentChapter(boot type)` | `en_henCurrentChapter` | the argument | 2 |
| `ResetWorldToFirstChapter()` | `en_henFirstChapter` | 9 | 3 |
| `ResetWorldToCustomChapter(chapter)` | the argument | 9 | 3 |
| `ResetWorldToNextChapter()` / `...PreviousChapter()` | the chapter after / before the current one in the **cyclic** order of the chain (`GetNextChapter` and `GetPreviousChapter` take the chapter index plus or minus one, modulo the chapter count; a chapter that is not in the chain gives none and nothing happens) | via `ResetWorldToCustomChapter`: 9 | 3 |
| `ResetLevelToChapter(index)` | `GetChapterInWorld(index)`; returns 0 when there is none | 9 | 3 |
| `macResetWorldToChapter(chapter)` | the argument, "Provided chapter is not valid." when none | 9 | 3 |
| `macResetWorldToCurrentChapter()` | current | **3** | 2 |
| `macResetWorldToFirstChapter()` | first | 9 | 3 |
| `PrepareForTemplates()` | (not read) | **11**, then the previous boot type member is put back | 3 |

The macro versions are what level scripts call (`level-logic.md`: `macResetWorldToCurrentChapter` is used, the others are listed in the API). `ResetWorldToNextChapter` and `...Previous...` print "Started chapter: ..." with the chapter's name.

#### 2.2 The callers

| Caller | Call | Condition |
|---|---|---|
| `CWorldInfoEntity::OnSimStart` (the simulation starts) | `ResetWorldToChapter(chapter chosen by PrepareChapterToStart, the boot type it returns, 3)`; first it spawns a game info entity if there is none, collects the environment damagers, objectives, beacons and secrets, reads the level description, then `PrepareChapterToStart`, then `UpdateGameDifficultyParams` (copies one of the five difficulty parameter blocks into the game info) | every level start (new game, continue, load) |
| `PostWorldChange` (the world has changed from another level) | `ResetWorldToChapter(en_henFirstChapter, 10, 1)` after finding the game info entity of the new world by class and collecting the four lists | travel by `simChangeWorld` |
| `CMSInGameMenu::btnRestartChapter_OnYes` | `ResetWorldToCurrentChapter(3)` | after the question "Are you sure you want to restart this chapter? All progress in this chapter will be lost." |
| `CMSInGameMenu::btnRestartLevel_OnYes` | `ResetWorldToChapter(en_henFirstChapter, 5, 3)` | after the level restart question |
| `CPlayerBrainEntity::ExecuteRespawn` | `ResetWorldToCurrentChapter(2)` | the session `netIsLocal` (single player, local); lives are used (single player or co-op with `sgp_bUseLives`) and the puppet has lives left, then one life is taken; **with no life left the respawn does nothing unless the statistics are frozen** |
| the simulation reset function (restart of a test run) | `ResetWorldToCurrentChapter(0)` after `CSimulation::Reset` | a simulation is attached |
| developer keys, in the player controls | `ResetWorldToCurrentChapter`, `...ToFirstChapter`, `...ToPreviousChapter`, `...ToNextChapter`, `StoreFavoriteChapter`, `RestoreFavoriteChapter` (section 6) | only when the cheat level word is not zero |
| `CChapterInfoEntity::Start` | see section 5 | a script's `Chapter.Start` |

`ExecuteRespawn` for a **networked** session does **not** reset the world: it only calls the player brain's `BootPlayerOnChapter(6)` (co-op) or `BootPlayerInDeathmatch(1)`, then one more virtual (the respawn finished hook, not read). The same "boot one player" path exists for a joining player: slot **16** (the last of the 17 player slots) boots with the **world's current boot type**, every other slot with **8** (`CBT_COOP_JOIN`; read in the instruction listing, **medium**).

#### 2.3 `PrepareChapterToStart` (the chapter and boot type of a level start)

It returns a boot type and the chapter to start; **medium** (read once, lost branches checked in the instruction listing).

1. Start with the **first chapter** of the world and boot type **0**.
2. If the session option `sgp_strStartChapterName` is **not empty**: walk the chain from the first chapter and take the chapter with that name (names are compared exactly, `en_strName`); if none matches the first chapter stays; **return 0**. (This is the "start chapter name" of a custom or developer start; it wins over everything below.)
3. Else if `sgp_bContinuePlaying` is 0: **return 0**. The single-player start function sets it to **1 at every single-player start** (new game, continue, any level).
4. Else if the active player profile has **no stored level status** (the profile's chapter save): **return 0**. A **new game** discards the stored status first, so it ends here; "Continue playing" (an empty level name) keeps it.
5. Else if there is **no game info entity**: **return 1** (with the first chapter).
6. Else (a continue with a stored status and a game info entity): the stored level status is **copied** into the session (into the game info entity, in the way `SetLevelStatus` does), the stored game options (`CSamProjectParams`) overwrite the session's and the game info's options, the stored game statistics overwrite the game info's `en_gsGameStats`, and **if the stored chapter id is not -1** the chapter with that entity id (if it is a chapter info) becomes the chapter to start; **return 1**.

In short: "Continue playing" starts the **chapter that was stored when the last chapter start generated the chapter save** (boot type 1); everything else starts the first chapter (0) or the named one. A load of a `.sav` does not use this path for the player data (`save-game.md`).

### 3. `ResetWorldToChapter_internal` step by step (medium)

The arguments are the chapter, the boot type and the resetting type. Order of effects:

1. **Store the types**: `en_cbtChapterBootType` := boot type and `en_ewrtWorldResetingType` := resetting type. This happens **before** any check, so a rejected second call still overwrites both (3.2).
2. **Totals.** If a game info entity exists and (boot type is **1 and the chapter is the first chapter**, or boot type is one of **0, 5, 7, 9, 10**): `CalculateGameTotalStats`. On the host it resets the game statistics block and has **every `SBaseEntity`** add its contribution (kill totals and secret totals of the level; puppets and secrets override the hook).
3. `en_tmWorldStart` := the current simulation time. The global physics parameters are set up (`SetupGlobalPhysicsParams`).
4. If the resetting type is 0: log and set it to 3 (above).
5. If `en_bResetChapterInProgress` is set: log "Can't reset world to certain chapter because world resetting is already in progress. Please wait until first chapter is booted before reseting world to another chapter." and **return** (nothing else is done; the two members of step 1 stay overwritten).
6. `en_bResetChapterInProgress` := 1.
7. **Pre-shutdown hook** (`OnWorldInfoPreShutDown`): empties `en_eaPuppetsInExisting`, `en_eaRidesInExisting`, the three occupied containers `en_eaSpawnSlotsEvil`, `...Good`, `...Misc`, `en_eaEnemiesAlive`, the three waiting lists `en_eaWaitingForSpawnSlotEvil`, `...Good`, `...Misc`, the three render-request lists (`en_eaRequire2DRendering`, `...OpaqueRendering`, `...TranslucentRendering`) and the live debris container; clears `en_henLastSpawnerThatSpawned`, `en_henLastUser`, `en_henBoss` and `en_strBossName`; sets `en_fEnemyLagFactor` to 0; and, if a real-time factor is active, sets the simulation's factor back to 1.0 and `en_fDesiredRTF` to -1. (The timer and counter values are **not** touched here.)
8. **Disable the macros**: every macro entity's interpreter is told to stop executing (`CDisableMacrosIterator`).
9. **Shut every entity down**: every `CBaseEntity` gets its `OnShutDown` and then **bit 0 of its flag word is set** ("is shut down", `COnShutDownIterator`).
10. All **pending thinks** of the simulation are cleared (`simClearThinkers`).
11. The dynamic music is stopped and its sources cleaned (`StopDynamicMusics`, `CleanDynamicMusicsSources`).
12. **Post-shutdown hook** (`OnWorldInfoPostShutDown`): the world info schedules its own state function `DefaultState` **1.0 s** later (the function that, once a second, updates the hostility graph and, when the debug dump option is on, validates the three containers; it re-schedules itself every 1.0 s).
13. `en_henChapterInBooting` := the chapter being booted.
14. **Boot every entity** with the boot iterator over the **sorted** entity list (section 3.1 and 3.3). The iterator is handed the world info, the booting chapter and the chapter save's current-chapter record (when a game info entity and a stored level status exist).
15. The dynamic music is initialised again and crossfaded to type 0 (relaxing); `en_tmLastFoeKilled` := -100 s; the live debris container is freed and `en_bAllowDebrisCreation` := 1; `SetChapterEnemyMaxes` rebuilds the maxima (`mechanism-classes-world-info.md` section 4).
16. **Post-boot hook** (`OnWorldInfoPostBoot`) = `ResetCamera`: `en_henCamera` (the current camera) is cleared; if `en_henAutoStartCamera` is set it becomes the current camera and its `OnTrigger` is called with the world info as the activator (the cutscene camera that starts with the level, `mechanism-classes-triggers.md` section 8).
17. `en_henCurrentChapter` := the chapter.
18. Unless the boot type is **11**: **every macro entity** runs one interpreter step (`CExecuteMacrosIterator`; the interpreter's `OnStep`).
19. **`SetCurrentChapter(current chapter, 1)`** (section 5): from the **first** chapter up to (not including) the booting chapter every chapter gets `MarkFinished` (event `Finished`), then the booting chapter gets `MarkStarted(f)`, where f = 1 when the resetting type is 1 or the boot type is 1, else 0 (event `Started`, game info bookkeeping, **the chapter save is generated**, `companion page` section 3).
20. `en_bResetChapterInProgress` := 0 and `en_ewrtWorldResetingType` := 0. **`en_cbtChapterBootType` stays** at the boot type of this reset.
21. If the current chapter is a chapter info: the macro event **`Booted`** is sent to it (`level-logic.md` lists the event).

Consequences worth keeping (all **medium**): (a) the **`Finished` events of the chapters before the target are raised on every restore**, even a plain respawn; scripts that count `Finished` see it again; (b) `Started` is raised **after** the macro step of step 18, so the target chapter's `Started` handler runs on the **next** interpreter step; (c) a macro that was disabled in step 8 only runs again from step 18 on; (d) the boot type member keeps its value, so a **later** `ActivateChapter` (a scripted chapter start) computes the finished flag f from the **old** boot type: after a "Continue playing" (1) every later chapter start of that session passes f = 1 and **suppresses the `SpreadChapterFinishedEvent`** of that start (low: derived from the code, not observed).

#### 3.1 The sorted order

`wldIterateEntitiesSorted` (engine) first copies the world's entity list into a pending list, then repeatedly takes the first pending entity and calls its **`CollectForSortedIteration`**; an entity's version first collects what it **depends on**, then itself (an entity is added to the result only if it is still pending, so each appears once). Classes with their own version (the dependency each collects first): the chapter info (its parent entity), the flare, the spawner, the static model, the particle effect, the turret puppet, the external joint, the bouncer and the player puppet (the player puppet first collects **all chapter infos**). The result is a **parents-before-children order** of the world's entity list; the booting visits entities in that order. **Medium** (read; not run on a real world).

#### 3.2 What a second reset during a reset does

Nothing except the two member writes of step 1 (and the log line). Because the reset is synchronous on the host, the case only arises through a remote call that arrives during the reset on a client.

#### 3.3 The boot rule of one entity (`COnBootIterator`) (medium, read in the instruction listing)

For every `CBaseEntity` E, in the sorted order:

1. **candidate** := true when **any** of: the booting chapter is not set; E's chapter link (`SBaseEntity.1`) is null or not a chapter info; **`IsChapterBefore(E's chapter, booting chapter)` is false**. `IsChapterBefore(a, b)` walks the chain from the first chapter and returns 1 if it meets `a` before it meets `b` (so **b itself and anything after b give 0**, and a chapter that is not on the chain gives 0).
2. **skip** := true when the resetting type is **2** (`WRT_CURRENT_CHAPTER`), the chapter save record exists, the boot type's **lives column of the table is 0** (that is boot types **2** and **6**, companion page section 1) and E's **entity id is in the record's `scs_ceidUnableToResurrect` list**.
3. If skip: E is **not booted** (it stays shut down; its shutdown bit stays set).
4. Else if candidate and **bit 0 of E's flag word is set**: call E's `OnBoot` and **clear the bit**.

So: entities of the **booting chapter and of later chapters**, and every entity **with no chapter link** (2,902 of 57,529 placed entities have one, `tools/re/chapter_restore_verify.py`), are booted; entities of chapters **before** the booting one are **left shut down**. They are booted later only through `AssureThatIsBooted(entity)` (which does the same bit test, `OnBoot`, bit clear; a null argument logs "Called with NULL entity argument."); the player brain calls it for its own puppet at the start of its `OnBoot`, which is why the puppet is booted before `BootPlayerOnChapter` runs even if it sorts later. Who else calls `AssureThatIsBooted` was not found (**open**, T01-340).

**`scs_ceidUnableToResurrect`** is filled by `CBaseItemEntity::OnPicked` through `CGameInfoEntity::DisableChapterResurrection(item)`: **when an item that has no spawner and a valid entity id is picked and `IsChapterResurrectionEnabled()` is false**, its id is appended (once) to the list of the **current-chapter record**. The two item classes with a false answer in the retail data are **treasure and extra life** (`item-classes-vitals.md`). The list is **emptied whenever the chapter save is generated and the resetting type is not 2** (`GenerateChapterSaveGame`; companion page section 3), so it only lives from a chapter start until the next chapter start or restart. Result: after **dying**, picked health, armour, ammunition and weapon items **reappear** (their entities are booted), but **picked treasure and extra lives do not**; after a chapter restart (3, resetting type 2 but a table lives column of 1) everything is booted, including treasure.

### 4. What each class does when it is booted or shut down

Read from the `OnBoot` and `OnShutDown` of about 40 classes (**medium**; "see" = already specified elsewhere). Classes with no `OnBoot` of their own (the base entity, the generic `S` layer) do nothing.

| Class | `OnShutDown` | `OnBoot` |
|---|---|---|
| puppet (`CPuppetEntity`) | clears its dynamic children, rider lists, animation queue, effects and sound; a puppet that was **created by a spawner or at run time deletes itself**; placed puppets are left "dead-state 3" (state 3 = shut down) | marks its death spawner; sets its birth time and last-step time; returns to its default placement; **registers with the world info** (`CharacterBorn`); shows the default attachments, plays the alive sound and effects, clears its rider list (host) and enters its first state; a dummy foe puppet and a turret puppet add their own settings (a turret re-aims to its default view) |
| player puppet | the puppet base, stops the player's sounds, clears the chat text queue | the puppet base, then clears two effect holders (see the companion page for the brain) |
| player brain | none | **`AssureThatIsBooted(puppet)`**, then `BootPlayerOnChapter(en_cbtChapterBootType)` (or the deathmatch boot), then the brain base (companion page) |
| brain (`CBrainEntity`) | state 0, stops, resets the foe | resets the foe and counters; unless it is in the "ride" state starts the default state |
| game info | none | clears its per-boot state, tells every one of the 17 player puppets `OnGameBooted`, and **restores secrets, objectives and beacons from the chapter save** when the level name and chapter id match (companion page section 3.3) |
| chapter info | none | `en_bFinished` := 0; re-attaches to its parent |
| door, animated mover, platform | see `door-mover-rules.md`: lock flag from `en_bInitiallyLocked`, closed state; a mover returns to its default placement and idles; a platform returns to its creation placement and starts moving | |
| detector area | clears the touching list, deletes its hull and hides the editor view | clears the last activator and **re-arms** (`Recharge`: the hull is created again) |
| killer | none | clears its entity-and-time list; tells its linked mover to start |
| bouncer | none | creates or deletes the hull by its enabled flag, activation time := 0, the current user is cleared |
| barrier field | none | on the host (or a client whose net flag is set) enables the field again |
| camera | none | resets the camera's state and the envelope time |
| external joint, generic motor | none | the joint is built again between its two bodies |
| spawner | state 13 | see `spawner-state-machine.md` (initial counts, `ResetSpawner`, tweaks, multiplier, state 0) |
| items (`CBaseItemEntity`) | a placed item hides itself; a dynamic one is deleted | an item with no spawner is **shown again if `IsItemAllowed`** (game rules and difficulty may remove item kinds), else hidden; the picker mask is cleared. The Serious Bomb pickup additionally hides itself when serious bombs are disabled |
| prop, static model | a spawned prop is deleted; a placed one clears its collision and per-step update | marks its death spawner, resets to its stored model, health, collision and animation; a static model re-appears (`DoAppear`) |
| projectile, debris, simple effect, sound effect, particle effect | deleted (these are always run-time entities; a particle effect also stops) | a placed effect restarts its sound or particles |
| static sound, static vibration, movie player | stop their channel | restart the sound (with a random offset in the sound case), vibration or movie when their autoplay flag says so |
| mission objective, secret, portal | see `mechanism-classes-triggers.md` | see the companion page for the saved states |

Two conclusions that matter for an implementation: **everything created at run time is deleted by a reset** (spawned enemies, projectiles, debris, effects, dropped items) and **everything placed is rebuilt from its record or from the members the boot restores**; the spawners then start again from their initial counts, which is why a restart brings back enemies.

### 5. `SetCurrentChapter`, `MarkStarted`, `MarkFinished`, `OnTrigger`

**`SetCurrentChapter(target, fromFirst)`** (the world info's virtual; `ActivateChapter(chapter)` is `SetCurrentChapter(chapter, 0)`):

1. The start of the walk is the **first chapter** when `fromFirst` is 1; else the **current chapter** if there is one, else the target itself.
2. If the target and the start are both valid: walk from the start along the next-chapter links looking for the target. More than **1,000** steps: log "Inifinite loop found in chapters info chain. Unable to set current chapter info." and **return** (nothing changes). The target not found: log "WorldInfo::SetCurrentChapter() Given chapter can't be started because it can't be found in chain of next chapters." and **return**. **So a script can only move the current chapter forward along the chain** (or stay on the same chapter); going back is done only by a reset.
3. Every chapter from the start up to **but not including** the target gets `MarkFinished`.
4. `en_henCurrentChapter` := target; if it is valid, `MarkStarted(f)` with f as in step 19 above (1 when the resetting type is 1 or the boot type is 1, else 0).

**`MarkFinished`**: sends the macro event **`Finished`** to the chapter and sets `en_bFinished` to 1. **`CChapterInfoEntity::OnBoot`** clears it again.

**`MarkStarted(f)`** (single player with an active profile does all; otherwise only the first two lines): (1) macro event **`Started`**; (2) with no game info entity, or **no active player profile**, stop; (3) a chapter that has a **next chapter** and is **not** named `EndGameChapter`: unless f is 1 or the session is deathmatch, **`SpreadChapterFinishedEvent`** (to every player puppet: its `OnChapterFinished`), then update the profile's current-level statistics (companion page section 5); (4) the **last chapter of a chain** (no next chapter) or a chapter named **`EndGameChapter`**: a chapter named `EndGameChapter` also **marks the profile's "game finished" flag**; then the chapter's **next level path** is added to the profile's list of known levels if it is not on it (and is not an always-open level), the level statistics are finalised and stored (companion page 5), and, unless deathmatch, **`SpreadLevelFinishedEvent`** (to every player puppet: its `OnLevelFinished(level name, difficulty, ...)`); (5) in all cases: **`GenerateChapterSaveGame(profile)`** (the chapter save), the game info's "last save" time stamp := now, and the profile is written to disk.

So the chapter save is made **on every chapter start, including a restore**, and a restore of boot type 2 or 3 rewrites it (with the secrets and objective lists rebuilt from the live entities). The profile list the earlier `save-game.md` calls "the chapter's level" is the chapter's **next level** path (**medium**: the argument is the chapter's `en_strNextLevel`).

**`CChapterInfoEntity::OnTrigger(activator)`** (the chapter's reaction to being triggered, also by a detector area whose target it is, `trigger-area-rules.md`): does nothing when `en_bFinished` is set or when the chapter **already is** the current chapter. Otherwise (a world info exists; else "Can't play on this world because there is no world info."): it looks through the **17 player slots** of the game info for the first one that answers a brain predicate (not identified: some "is a real player" test) and then: `ActivateChapter(this)`; **if a reset is in progress, stop**; if the chapter's **next level path is empty, stop**; **on the host only**: `simChangeWorld(next level path)` (the host changes the level; clients follow), and for an online server the session attributes (level file name, game type description, player counts) are updated for the matchmaking service. **The level change happens at the chapter's activation, not at the end of a movie**: the retail levels reach the end through a chapter named "Next Level Link" (33 of the 307 chapters carry that name, 41 have a next level path) whose trigger the level's last detector fires; the outro movies are played by `TheEndSequence` which scripts call before or around it (companion page section 6).

**`CChapterInfoEntity::Start`** (the script command `Chapter.Start`): in the retail game it calls the chapter's own `OnTrigger(chapter)`, i.e. **the same activation** (a global flag, read only here and written by no code found, selects the other branch: `ResetWorldToChapter(chapter, 9, 3)`; **low** that it is always 0 in the retail game). `JumpToNextLevel` (a developer key) finds the **last chapter of the chain** and calls its `OnTrigger` with the world info as the activator.

`TeleportPlayersToCurentChapter` (a script command, 2 uses): for each of the 17 player slots that has a player, teleports the **player's puppet** to the placement `samGetPlayerPlacementOnChapter(chapter, slot, player)` for the **current chapter** (the same placement function the portal uses, `door-mover-rules.md`).

### 6. The favourite chapter (developer commands, medium)

`StoreFavoriteChapter`: with no current chapter logs "There is no current chapter, can't store favorite chapter." and stops; else stores the **current chapter's name** (`en_strName`) in a **global string that lives across levels** and logs `Chapter "<name>" (<description>) stored as favorite chapter.` `RestoreFavoriteChapter`: an empty global logs "Save favorite chapter first."; else it looks the name up in the chain of the **current world** (`GetChapterByName`; a miss logs "Chapter with name stored as favorite chapter can't be found in chain of chapters."), then `ResetWorldToCustomChapter(that chapter)` (boot type 9, resetting type 3) and logs "Started chapter: ...". Keys: the player-control commands `plcmdStoreChapter` and `plcmdRestoreChapter` (also `plcmdResetToNextChapter`, `plcmdResetToPrevChapter`, `plcmdResetToFirstChapter`, `plcmdNextLevel`), active only when the global cheat level is not 0. They exist for level designers; a port may omit them.

## Invariants

1. Each of the 12 boot types and 4 resetting types has exactly one name and value in the binary's tables; the 12 rows of the boot type table are complete (12 x 4 values).
2. Every world has one chain per first chapter; no chain loops in the 59 worlds, so the 1,000-step guard never fires (data census, `tools/re/chapter_restore_verify.py`).
3. `en_cbtChapterBootType` is only written by the reset (step 1 and `PrepareForTemplates`); `en_bResetChapterInProgress` is 1 only inside the internal function.
4. The `Finished` event reaches every chapter before the target on every reset; the `Booted` event reaches the current chapter once per reset.

## Open questions

1. **Who calls `PrepareForTemplates`, what the boot type 11 is for** (a boot without macros; the name says "templates") and the first argument it passes. **T01-340.**
2. **The callers of `AssureThatIsBooted`**, and whether anything other than the player brain boots an earlier-chapter entity on demand. **T01-340.**
3. **The macro interpreter's step** (what one `OnStep` does after a disable) and the exact delivery time of the `Started`/`Finished`/`Booted` events relative to it. **T01-341** (with `level-logic.md`).
4. Whether the **global flag** read by `Chapter.Start` is ever set (a console variable or an option). **T01-340.**
5. The sort order on a real world (which entities depend on which; the unnamed overrides). **T01-209** (the on-boot iterator's exact condition is now read; what remains is the order).
6. Run-time confirmation of all rules: **T01-174**.

## Test vectors

All **binary readings** unless a data line is named; nothing was run.

1. **Respawn after death, single player.** Boot type 2, resetting type 2. Secrets: not restored. Objectives and beacons: restored from the chapter save. Entities of chapters before the current one: not booted. A treasure picked in this chapter (id in the unable-to-resurrect list): not booted, stays taken. A health pack picked: booted, visible again. The player: health and armour from the **chapter**, lives kept, weapons from the saved player (companion page, row 2: 3, 0, 1, 1).
2. **Restart chapter (menu).** Boot 3, resetting type 2: secrets and objectives restored, every entity of the chapter and later booted (treasure comes back), player rebuilt entirely from the saved player (row 3: 1, 1, 1, 1).
3. **Arrival from another level.** `PostWorldChange`: first chapter, boot 10, resetting 1: secrets restored (from a chapter save whose level name and chapter id match, which a new level does not), totals recalculated, the player keeps current ammunition merged with the chapter's (row 10: 2, 3, 2, 3), and `MarkStarted` gets f = 1 (no `SpreadChapterFinishedEvent`).
4. **Start chapter with a name.** Option `sgp_strStartChapterName` = "Chapter003", a level whose chain contains it: `PrepareChapterToStart` returns that chapter and boot type 0.
5. **`IsChapterBefore`.** Chain A -> B -> C: `IsChapterBefore(A, C)` = 1, `(B, C)` = 1, `(C, C)` = 0, `(C, A)` = 0, a chapter D off the chain: 0. Booting chapter B: entities of A are not booted, entities of B and C and entities with no chapter link are.
6. **`SetCurrentChapter(C, 0)` with current A:** `Finished` for A and B, `Started` for C. `SetCurrentChapter(A, 0)` with current C: the log line "can't be found in chain of next chapters", nothing changes.
7. **Census** (script `tools/re/chapter_restore_verify.py`): see "Data" below.

## Data (census of the 59 worlds)

Script `python -I tools/re/chapter_restore_verify.py` (about 10 minutes), all 59 worlds, the last patch wins:

- **307 chapter infos**: **296 are on a chain** (the chain that starts at the world info's first chapter and follows the next-chapter links) and **11 are on no chain** (they are never current and never reached by `SetCurrentChapter`; 10 placed entities carry a link to one of them). **No chain loops.** Every world that has chapter infos has a first chapter set.
- **Chain lengths** (chapters: worlds): 1: 9, 2: 5, 3: 2, 4: 1, 5: 10, 6: 2, 7: 4, 8: 5, 9: 4, 10: 5, 11: 3, **18: 1** (so `save-game.md`'s "1 to 12" is 1 to 11 plus one level with 18).
- **Names:** `Next Level Link` 33, `EndGameChapter` **1** (in `7_7_mental`, the **last of an 18-chapter chain**, position 17), 273 others. The **41** chapters with a next level path are all **last in their chain**: 33 `Next Level Link` chapters and 8 others; no chapter in the middle of a chain has a next level.
- **2,902 of 57,529 placed entities carry a chapter link** (`SBaseEntity.1`; the total is `wld-entity-schema.md`'s). By class: `SLeggedCharacterEntity` 1,235; `CDetectorAreaEntity` 516; `SAircraftCharacterEntity` 145; health items 189 (S 21, M 92, L 55, XL 13, XS 8); shells 86; turrets 81; bullets 55; portals 54; armour items 146; hand grenades 44; ammo packs 36; rolling balls 32; effects 30; rockets 28; **treasure items 52** (S 25, M 22, L 5); navigation beacons 15; hovercraft 15; doors 13; mission objectives 11; weapon items (rocket launcher 7, plasma 6, auto shotgun, mini gun, cannon, Uzi, double shotgun 5 each = 38), serious bomb 10, grenades 10, cannon balls 17, generic items 19. **By position of the chapter in the chain** (0 = first): 551, 487, 444, 401, 341, 267, 182, 94, 41, 28, 4, 4, 29, 6, 13 (positions 0 to 14). Only entities with a link can be skipped by the earlier-chapter rule, so the rule applies to **2,892** entities (10 point at off-chain chapters: `IsChapterBefore` is false for them, so they are always booted).
- **Treasure and extra life items**: 277 treasure items (S 110, M 128, L 39) and **11 life items** are placed; **52 treasure items and 0 life items carry a chapter link**. The unable-to-resurrect list matters for all 288 of them (the skip rule uses the entity id, not the chapter).
