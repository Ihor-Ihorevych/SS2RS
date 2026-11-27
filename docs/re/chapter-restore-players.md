# Chapter restore: the player rebuild, the chapter save, level end and the end sequence (T01-172)

- **Roadmap task:** T01-172 (second part). First part: [`chapter-restore.md`](chapter-restore.md) (boot types, reset sequence, the boot rule, chapter activation). The chapter save's record layout is in `save-game.md` section 6 and `players-session-classes.md` section 8.
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the 12 x 4 table (plain data of the DLL) and for the member names and byte-position matches of the saved records (the binary's member tables, all names found by position); **medium** for every rule (one reading each, the lost arguments recovered from the instruction listing); **low** for the points marked "open". **Nothing was run on the original game.**

Prerequisites: [`chapter-restore.md`](chapter-restore.md), `players-session-classes.md` (`CPlayerInventory`, `CPlayerSave`, `CGameStats`), `inventory.md` and `weapons.md` (weapon and ammunition indices), `save-game.md`, `cutscenes.md` (the `.nfo` movie lists), `misc-types.md` (the `.nfo` keys).

## Summary

1. **One 12 x 4 table of small numbers drives the whole player rebuild.** Row = boot type, column = what is rebuilt: **A** health, armour and ammunition; **B** lives (and score and secret count); **C** weapons; **D** kill count. A cell says where the new value comes from: 0 keep the current one, 1 take it from the saved player, 2 merge the current and the chapter's values (the larger), 3 take it from the chapter info or a default, 4 reset to the inventory default (section 1).
2. **Respawn after death keeps the lives and takes the chapter's health, armour and ammunition**; the weapons come from the saved player (row 2 = 3, 0, 1, 1). A **chapter restart** (row 3) and a **continue** (row 1) rebuild everything from the saved player (1, 1, 1, 1). A **new game, a level restart, a custom start and a developer reset** (rows 0, 5, 9) take everything from the chapter (3, 3, 3, 3); a **level change** (row 10) keeps the weapons and the larger of current and chapter ammunition (2, 3, 2, 3).
3. **The chapter save** (`CStoredLevelStatus`) is rebuilt at **every chapter start**; for the **last** chapter of a level it stores the **next level's path and no chapter id**, which is how "Continue playing" starts the next level (section 3).
4. **Level end score** (section 5): difficulty bonus 1,000 / 3,000 / 5,000 / 7,000 / 10,000, **1,000 per remaining life, 10 per kill, 10 per percent of secrets found, 1 per second under the level's `ETA` time**, plus the score; **0 bonus when cheating**.
5. **`TheEndSequence(movie)`** queues, in this order, a start marker, the level's outro movies, the statistics menu (local sessions), the optional extra movie and an end marker (section 6).

## Evidence

- **Code.** `CPlayerBrainEntity::BootPlayerOnChapter` (2 kB, decompiled and read in full; the boot-type table accesses and the branches on each column verified in the listing), `AddWeaponsFromPastChapters`, `StoreChapterSaveData`, `OnBoot`, `GotoCurrentChapter`, `ExecuteRespawn`; `CAddPastChaptersWeaponsIterator::ProcessEntity`; `CPlayerInventory::ResetAllToDefault`, `ClearWeapons`, `ClearAmmo`, `SetAmmoFromChapter` (instruction listing for the constant), `CGameInfoEntity::GenerateChapterSaveGame`, `StorePlayersData`, `SetLevelStatus`, `RestoreSavedSecrets`, `RestoreSavedObjectivesAndNavBeacons`, `DisableChapterResurrection`, `OnBoot`, `SpreadChapterFinishedEvent`, `SpreadLevelFinishedEvent`, `UpdateGameDifficultyParams`, `CalculateGameTotalStats`, `CChapterInfoEntity::MarkStarted`; the profile's level statistics function and the level information lookups it uses; `CWorldInfoEntity::TheEndSequence`, `macTheEndSequence`; the script-call stubs of both.
- **Member names** by byte position in the binary's member tables: `CGameStats` (`gs_ctSecrets`, `gs_ctSecretsTotal`, `gs_ctKills`, `gs_ctKillsTotal`, `gs_slScore`, `gs_tmPlayingTime`, `gs_slPlayingTime`, `gs_bCheating`, `gs_bSaving`, `gs_bFrozenStatistic`), `CStoredLevelStatus` (`sls_scsFirstChapter`, `sls_scsCurrentChapter`, `sls_sgpGameParams`), `CStoredChapterStatus` (`scs_strWorld`, `scs_ulChapterEntityID`, `scs_gsGameStats`, `scs_ahpsPlayerData`, `scs_ceidUnableToResurrect`, `scs_anbdNavigationalBeacons`, `scs_asedSecretEntities`), `CPlayerSave` (`ps_*`), the player puppet's `en_slHealth`, `en_slArmor`, `en_piPlayerInventory`, the session options `sgp_*`.
- **Data.** The `.nfo` keys behind the level `ETA` and the movie lists (`misc-types.md`); counts in `chapter-restore.md`.
- **Not checked.** Nothing run; the profile's medal computation, the effect of the `OnChapterFinished` and `OnLevelFinished` player hooks, the movie sequencer's actions, the placement of a booted player.

## Layout / rules

### 1. The boot type table (high)

Twelve rows of four 32-bit numbers, one row per boot type, in the order of `ChapterBootType` (`chapter-restore.md` section 1). The player brain reads **all four columns**; the world's boot iterator reads **column B** only.

| Boot type | Name | A: health, armour, ammunition | B: lives, score, secrets | C: weapons | D: kills |
|---|---|---|---|---|---|
| 0 | `CBT_CUSTOM_LEVEL_CUSTOM_CHAPTER` | 3 | 3 | 3 | 3 |
| 1 | `CBT_MAIN_MENU_CONTINUE_PLAYING` | 1 | 1 | 1 | 1 |
| 2 | `CBT_RESPAWN_AFTER_DEATH` | 3 | **0** | 1 | 1 |
| 3 | `CBT_INGAME_MENU_RESTART_CHAPTER` | 1 | 1 | 1 | 1 |
| 4 | `CBT_GAME_OVER_MENU_CONTINUE_PLAYING` | 3 | 4 | 2 | 4 |
| 5 | `CBT_GAME_OVER_MENU_RESTART_LEVEL` | 3 | 3 | 3 | 3 |
| 6 | `CBT_COOP_RESPAWN_PLAYER` | 3 | **0** | 2 | **0** |
| 7 | `CBT_COOP_RESTART_LEVEL` | 3 | 3 | 2 | 3 |
| 8 | `CBT_COOP_JOIN` | 3 | 3 | 3 | 3 |
| 9 | `CBT_DEVELOPER_CUSTOM_CHAPTER` | 3 | 3 | 3 | 3 |
| 10 | `CBT_LEVEL_CHANGED` | 2 | 3 | 2 | 3 |
| 11 | `CBT_BOOT_TEMPLATES` | 4 | 4 | 4 | 4 |

(The numbers are the cell values; the meaning of the digits depends on the column, sections 2.1 to 2.4. A value that is not listed for a column does nothing.)

**Fallback.** When there is **no stored level status**, or no saved player whose name equals this player's name, every cell **1 becomes 3** before it is used (so a continue without a saved player rebuilds from the chapter). **Boot type 3 on a client** (not `netIsLocal`) sets column B to **0**.

### 2. `BootPlayerOnChapter(boot type)` (medium)

Called by the player brain's `OnBoot` with `en_cbtChapterBootType` (not in deathmatch; deathmatch uses `BootPlayerInDeathmatch`, which places the player at a deathmatch spawn, resets the whole inventory to the default and, when its flag is 1, re-boots the puppet), by the join path (8 or the world's type) and by a networked co-op respawn (6). It needs a **player puppet** and a **game info entity** (else it logs "BootPlayerOnChapter() called but puppet is NULL." or "... Game info entity doesn't exist." and stops).

**Preparation.**

1. The brain's per-boot fields are reset: a flag cleared, the look direction restored to its start value, **the boot time stamp := now**, the "last action" time := -1 s, and **`en_bInvulnerablePeriodOnBootActive` := 1**.
2. The **saved player** is looked up (only on a local session): in the current-chapter record's `scs_ahpsPlayerData` (64 entries) the first `CPlayerSave` whose `ps_strPlayerName` equals the brain's `en_strPlayerName`. None found is not an error (see the fallback).
3. For boot types **2 and 6** the puppet is **shut down and booted again** (`OnShutDown`, `OnBoot`) before anything else.
4. For boot types **1 and 3** with a record: `en_tmWorldStart` of the world info is moved by the record's `gs_tmPlayingTime` and the game info's `gs_tmPlayingTime` := the record's (the playing time continues from the saved value; the direction of the adjustment was not read, **low**). For boot type **2**: `en_tmWorldStart` is moved by the **game info's current** `gs_tmPlayingTime`.
5. The **chapter** used for the loadout is the **booting chapter** while a reset is in progress and the **current chapter** otherwise (`en_henChapterInBooting` or `en_henCurrentChapter`). **If there is no chapter the inventory is just reset to its default** and the rest is skipped.
6. A fresh inventory is created and reset to the default (`ResetAllToDefault`, 2.5); the four table cells of the boot type are read; the fallback of section 1 is applied.

#### 2.1 Column A: health, armour, ammunition

| Cell | Effect |
|---|---|
| 3 | the puppet's `en_slHealth` := the chapter's `en_slHealth`, `en_slArmor` := the chapter's `en_slArmor`; the ammunition is set **from the chapter** (`SetAmmoFromChapter`, 2.6) |
| 1 | health and armour := the saved player's `ps_slHealth` and `ps_slArmor`; the 12 ammunition counts := the saved player's `ps_aslAmmoQuantity` (the copy takes `sam_ctAmmoTypes` entries from the 11-entry saved array, `players-session-classes.md` open question 2) |
| 2 | health := the **larger** of the chapter's and the puppet's current health; armour likewise; each ammunition count := the **larger** of the current count and the count `SetAmmoFromChapter` gives |
| 0 | the ammunition is copied from the puppet's current inventory; health and armour untouched |
| 4 | nothing is set: the new inventory keeps the default (**zero ammunition**), health and armour untouched |

#### 2.2 Column B: lives, score, secrets

| Cell | Effect |
|---|---|
| 1 | lives := the saved player's `ps_ctLives`; the game info's `gs_slScore` := the record's `gs_slScore` and `gs_ctSecrets` := the record's `gs_ctSecrets` |
| 0 | lives := the puppet's current lives; score and secret count unchanged |
| 3 | lives := the **game info's copy of `sgp_ctMaxLives`** (default 3, `helper-classes-params.md`); **unless the boot type is 6 or 8**: `gs_slScore` := 0 and `gs_ctSecrets` := 0 |
| 2, 4 | nothing is set: the inventory keeps its default lives, which is the **global session options' `sgp_ctMaxLives`** (`ResetAllToDefault`) |

#### 2.3 Column C: weapons

The inventory starts with the **default weapons** (`samGetDefaultWeapons`) and the default pair flags (`samGetDefaultDoubleWeaponFlags`); **the pair (double-wielded) flags are never restored** from the save (`CPlayerSave` has `ps_ulDoubledWeapons`, but `StoreChapterSaveData` does not fill it and `BootPlayerOnChapter` does not read it; **medium**).

| Cell | Effect |
|---|---|
| 3 | when the chapter's `en_bLoseWeapons` is **0**: `AddWeaponsFromPastChapters` (2.7); then the weapon set |= the chapter's `en_ulWeaponsAddedToPlayer` |
| 1 | the weapon set := the saved player's `ps_ulWeapons` |
| 2 | when `en_bLoseWeapons` is 0 **or the boot type is not 10**: `AddWeaponsFromPastChapters` and the weapon set |= the puppet's current weapons; then, always, the weapon set |= the chapter's `en_ulWeaponsAddedToPlayer`. (So with a lose-weapons chapter and boot type 10 the current weapons are **dropped**.) |
| 0 | the weapon set := the puppet's current weapons |
| 4 | nothing: the default weapons only |

After the inventory is assigned to the puppet: for cell 1 or 2 with a saved player the puppet selects the saved `ps_wiLastSelectedWeapon`; for any cell except 1, when the chapter exists and its `en_wiPreferredWeapon` is not -1 the puppet selects the **chapter's preferred weapon** (this wins over the saved one for cell 2, which runs second); if the puppet then still has no current weapon (its weapon query returns -1) two calls with weapon index 0 are made (a can-select test and, if the test says no, a select call; their exact meaning was not read, T01-114).

#### 2.4 Column D: kills

Cell 3: the game info's `gs_ctKills` := 0. Cell 1: := the record's `gs_ctKills`. Other cells: unchanged.

#### 2.5 `CPlayerInventory::ResetAllToDefault` (medium)

Weapons := the default weapon set; pair flags := the default pair set; all `sam_ctAmmoTypes` ammunition counts := 0; the power-ups are reset (`ResetPowerUps`: **the power-up timers are not carried across a boot**); lives := the global `sgp_ctMaxLives`.

#### 2.6 `SetAmmoFromChapter(chapter)` (medium)

1. `AssureAmmoRatioForAllPossessedWeapons(chapter ratio)` with the chapter's `en_fAmmoRatio`: every ammunition type of a weapon the inventory holds is raised to **ratio x that type's maximum** (`samGetMaxAmmoQuantity`; the rounding was not read). (T01-115 owns the ammunition numbers.)
2. **Hand grenades (index 4) are raised to at least `trunc(maximum(4) x ratio)`**; since `ResetAllToDefault` made them 0 this means "ratio x max".
3. `SetAmmoValues(chapter's ammunition container)` (`en_hcaiCustomAmmoItemsAddedToPlayer`): the container's counts are **added** by the inventory's own routine (details: T01-115).

#### 2.7 `AddWeaponsFromPastChapters(inventory, chapter)` (medium)

1. Walk the chain from the **first chapter** up to, **not including**, `chapter`: for each chapter with `en_bLoseWeapons` set, **first** clear the inventory's weapons (to the default set) and ammunition (to 0); then OR the chapter's `en_ulWeaponsAddedToPlayer` into the inventory's weapon set. A chapter that loses weapons therefore keeps **its own** additions but nothing from before it.
2. **Weapon pickups placed in the earlier chapters count too.** Let L be the **last chapter with `en_bLoseWeapons`** among the chapters from the first up to and **including** `chapter` (none if there is none). Every placed **weapon item entity** (`CWeaponItemEntity` and its subclasses) that has a chapter link gives its weapon to the player when: **L is none** and the item's chapter is **strictly before** `chapter` (`IsChapterBefore`), or **L exists, is not `chapter` itself**, and the item's chapter is **strictly between L and `chapter`** (`IsChapterBetween`, both ends excluded) **or is L itself**. When L is `chapter` itself no item gives anything. Items with no chapter link never count.
3. Both steps only add weapons, never ammunition.

### 3. The chapter save (medium)

#### 3.1 Records

`CStoredLevelStatus` = `sls_scsFirstChapter` + `sls_scsCurrentChapter` (two `CStoredChapterStatus`) + `sls_sgpGameParams` (the session options). A `CStoredChapterStatus` (`scs_`) holds:

| Member | Content |
|---|---|
| `scs_strWorld` | the level to continue in |
| `scs_ulChapterEntityID` | the entity id of the chapter to continue at, or **-1** |
| `scs_gsGameStats` | a `CGameStats` copy (secrets, secret total, kills, kill total, score, playing time, cheating, saving, frozen) |
| `scs_ahpsPlayerData` | up to **64** handles to `CPlayerSave` (`ps_strPlayerName`, `ps_ulWeapons`, `ps_ulDoubledWeapons`, `ps_wiLastSelectedWeapon`, `ps_aslAmmoQuantity`, `ps_slHealth`, `ps_slArmor`, `ps_ctLives`) |
| `scs_ceidUnableToResurrect` | the entity ids of picked placed items with `IsChapterResurrectionEnabled` false (`chapter-restore.md` section 3.3) |
| `scs_anbdNavigationalBeacons` | pairs (entity id, one value: the beacon's active state) |
| the objectives array | triples (entity id, the objective's state, one more value; `CMissionObjectiveData`) |
| `scs_asedSecretEntities` | pairs (entity id, the secret's `SecretEntityState` value) |

The numeric values of `SecretEntityState` and `MissionObjectiveState` and the fields behind the three arrays are the subject of **T01-170**.

#### 3.2 Generating it (`GenerateChapterSaveGame(profile)`, from `MarkStarted`)

Does nothing without a profile or without a current chapter. Otherwise:

1. If the game info has no stored level status one is created. The session options are copied into `sls_sgpGameParams`. **`first` := the current chapter is the world's first chapter.** The previous current-chapter record is kept as `old`.
2. `scs_gsGameStats` := the game info's `en_gsGameStats`.
3. **The chapter id and level.** If the current chapter has a **next chapter**: `scs_ulChapterEntityID` := the chapter's entity id and `scs_strWorld` := the running level's name. If it is the **last chapter** (no next chapter): `scs_ulChapterEntityID` := **-1** and `scs_strWorld` := the chapter's **next level path** (`en_strNextLevel`; may be empty). That is why a continue after a finished level starts the **next level**, at its first chapter (`PrepareChapterToStart`: the chapter id -1 selects nothing).
4. `StorePlayersData(record, old)`: for each of the 17 live player slots that has a player brain a new `CPlayerSave` is created and filled by the brain's `StoreChapterSaveData` (name; weapons set; the last selected weapon; the 12 ammunition counts copied by `sam_ctAmmoTypes`; health `en_slHealth`; armour `en_slArmor`; lives from the inventory; **not** the pair flags, the power-ups, the position or the score); then every saved player of `old` **whose name is not among the live players** is copied after them, up to 64 entries.
5. If the resetting type is **not 2**: `scs_ceidUnableToResurrect` is emptied.
6. The beacons, objectives and secrets arrays are **rebuilt from the world info's collected entities** (`en_cenNavBeacons`, `en_cenMissionObjectives`, `en_cenSecrets`): one entry per entity, in container order.
7. When `first`: the new record is also copied into `sls_scsFirstChapter`.
8. **Single player with a profile:** the profile's stored level status (created if missing) gets the session options and a copy of the current-chapter record; **when `first`, the whole stored level status is copied** (so the profile keeps the first-chapter record as well).

#### 3.3 Applying it (`CGameInfoEntity::OnBoot`, during the entity boot)

The game info entity is booted with every other entity. It clears its transient state (`gs_*` times, a text), tells each of the 17 player puppets `OnGameBooted`, and then applies the saved lists **only if all are true**: the simulation and the world info exist; the world info's `en_henChapterInBooting` is set; the game info has a stored level status; the current-chapter record's **`scs_strWorld` equals the running level's name**; and its **`scs_ulChapterEntityID` equals the booting chapter's entity id**. Then:

- if the boot type is in the objectives list (1, 2, 3, 4): for each beacon entry the entity with that id, if it is a navigation beacon, gets the saved value; for each objective entry the entity with that id, if it is a mission objective, gets the two saved values;
- if the boot type is in the secrets list (0, 1, 3, 5, 7, 9, 10): for each secret entry the entity with that id, if it is a secret, gets the saved state.

**Order does not matter for the restored values.** The secret, mission objective and navigation beacon classes export **no `OnBoot`** (only an `OnShutDown`, which runs in step 9 of the reset, before any boot), so nothing overwrites the saved values after the game info entity has applied them, whatever the sorted order (**medium**; what their `OnShutDown` resets: T01-170). Entities of **earlier chapters**, which are not booted, get their saved values all the same, because the lookup is by entity id.

#### 3.4 Life-cycle summary

*Level start:* `PrepareChapterToStart` -> boot (restore uses the record of the **previous** chapter start of this level: none for a new game, so nothing is restored) -> `SetCurrentChapter` -> `MarkStarted` -> **record rebuilt**. *Scripted chapter start:* `ActivateChapter` -> `MarkStarted` -> record rebuilt. *Respawn:* boot with the record -> `MarkStarted` -> record rebuilt (the unable-to-resurrect list kept). *Level change:* the record's world name no longer matches, the new world starts with boot type 10 and a record that is rebuilt for its first chapter.

### 4. `UpdateGameDifficultyParams` and `CalculateGameTotalStats`

`UpdateGameDifficultyParams` (called at every `OnSimStart`): copies one of **five** difficulty parameter blocks (7 values each, from the global game parameters object) into the game info by `en_gdDifficulty`: 1 -> block 1 ... 5 -> block 5, any other value -> block 1 (the tourist block). The five blocks are the members `gd_dpTourist` ... `gd_dpSerious` of the global `CGameDifficulties` (`players-session-classes.md`). `CalculateGameTotalStats`: `chapter-restore.md` step 2.

### 5. Level end statistics and score (medium)

`MarkStarted` of a **last chapter** (or the `EndGameChapter`) calls the profile's statistics function with "final"; every other chapter start calls it with "current". Inputs: the profile's difficulty (a value 1 `GD_TOURIST` to 5 `GD_SERIOUS` read from the profile record; the member is most probably `pp_gdDifficulty`, **medium**), the **lives left** of the local player (the scan over the 17 slots keeps the last local player puppet and stops at a slot whose puppet is missing), the game info's `gs_ctKills`, `gs_ctKillsTotal`, `gs_ctSecrets`, `gs_ctSecretsTotal`, `gs_slScore`, `gs_slPlayingTime` (seconds) and `gs_bCheating`, and the level's **par time**: the level's `ETA` from its `.nfo` (a `hh:mm:ss` string; `00:00:00` when the level has no entry) converted to seconds.

If **`gs_bCheating` is 0**:

| Part | Value |
|---|---|
| difficulty bonus | 1 -> 1,000; 2 -> 3,000; 3 -> 5,000; 4 -> 7,000; 5 -> 10,000; any other -> 0 |
| lives bonus | **1,000 x lives left** |
| kills bonus | **10 x `gs_ctKills`** |
| secrets bonus | when `gs_ctSecretsTotal` is not 0: **10 x trunc(floor(100 x found / total + 0.5))**, i.e. 10 per rounded percent; else 0 |
| time bonus | **trunc(max(0, par seconds - playing seconds))**: 1 per second under the par time (the playing time is the integer `gs_slPlayingTime`) |
| total | time + secrets + lives + kills + difficulty + `gs_slScore` |

With **`gs_bCheating` 1** every bonus is 0 and the total is just `gs_slScore`. The record built holds (level path, score, kills, kill total, secrets, secret total, playing time, medal 0, difficulty, lives, total, cheating). **"Current"** only stores it as the profile's running statistics; **"final"** also hands it to the profile's per-level best list (the medal is decided there; **not read**, T01-342) and **zeroes** the running statistics' playing time and score. **Example:** Normal, 2 lives left, 61 of 63 kills, 4 of 5 secrets, score 12,300, played 700 s with `ETA` 00:15:00: 5,000 + 2,000 + 610 + 800 + 200 + 12,300 = **20,910**.

### 6. `TheEndSequence(movie name)` and its script wrapper (medium)

`macTheEndSequence(movie)` is the level script's command (`level-logic-api.md`: 2 uses); it forwards to `TheEndSequence`. `TheEndSequence` first runs on the host as a **remote call** to every machine (clients run it only when called that way). Then, only if the project instance exists (otherwise a "guard breach" log):

1. **A "start" action** is appended to the project's sequence queue (a marker object; its effect is not read).
2. Unless `sam_bSkipMovies` is set: **the level's outro movies** (the movie list of the level's `.nfo` entry, looked up by the running world's name; the level record holds three lists and kind 1 is the one used; that it is the `OUTRO_MOVIE` list is inferred from the call and the key names, **medium**) are queued as movie actions, **the first one at a 0.5 volume argument**, the others at the default (-1: "not specified"); non-empty names only.
3. **In a local session** (`netIsLocal`): the UI variable `LevelFinished` is set to the world's name and the menu **`CMSStatistics`** is queued (with a 0.5 fade argument). In a networked session the statistics menu is **not** queued here.
4. If the argument is a non-empty name: **that movie** is queued after them as a movie action (the volume argument -1.0 and the other flags were read but their meaning was not).
5. **An "end" action** (a second marker) is appended.

The queue plays in order; leaving the last action returns to the game (the world is **not** changed by this function; the level change is the chapter trigger of `chapter-restore.md` section 5). A script that calls it right before triggering the "Next Level Link" chapter gets: outro movies, the statistics screen, then the level change.

### 7. What a port needs

A restore needs: the chapter chain and the entity-to-chapter links; a **shutdown / boot** pair on every entity class that returns the entity to its stored state; the **sorted boot** with the earlier-chapter skip and the unable-to-resurrect skip; the **12 x 4 table** for the player; the **chapter save** rebuilt at every chapter start with the world-name and chapter-id guard; and the rules for secrets, objectives and beacons by boot type (`chapter-restore.md` section 1).

## Invariants

1. The 12 rows have only the digits 0 to 4; column B is 0 exactly in rows 2 and 6; column D is 0 exactly in row 6; rows 0, 5, 8 and 9 are all 3; rows 1 and 3 are all 1; row 11 is all 4 (12 x 4 = 48 values, read as data).
2. No boot type uses cell 1 where there is no saved player (the fallback turns it into 3).
3. The chapter save is rebuilt by `MarkStarted` only; no other function writes the record's lists.
4. A chapter whose `en_bLoseWeapons` is set drops all weapons and ammunition granted by earlier chapters at the moment `AddWeaponsFromPastChapters` passes it.

## Open questions

1. **The profile's medal and per-level best rules** (`pls_mtMedal`), the unlocking test for levels and what the "always open" test (a level list function) is. **T01-342.**
2. **The player hooks `OnChapterFinished` and `OnLevelFinished`** (what each does to a player puppet: award, heal, reset?). **T01-343.**
3. **The movie sequencer's actions** (the two markers of the end sequence, the queue's play and skip behaviour, the effect of the volume argument). **T01-344.**
4. **Whether the game info entity's boot order lets its restore stick** (section 3.3) and the exact `OnBoot` of secrets, objectives and beacons. **T01-170** (the secrets and objectives page) and **T01-209**.
5. **The playing-time arithmetic of boot types 1, 2 and 3** (sign), and the respawn invulnerability arithmetic (`sgp_iRespawnInvulnerability` and a constant of 120 s in the same function). **T01-340.**
6. **Placement of a booted player** (`GotoCurrentChapter`, `samGetPlayerPlacementOnChapter`, fall-down position, respawn in place, `T01-210`).
7. **`SetAmmoValues` and `AssureAmmoRatioForAllPossessedWeapons` rounding**, `SelectWeapon` details. **T01-115, T01-114.**

## Test vectors

All binary readings; nothing was run. Chapter loadouts: health 100, armour 0, weapons added 0x1, ratio 0.5, lose-weapons 0, preferred weapon -1, unless stated.

1. **New game (boot 0).** Row 3,3,3,3: health 100, armour 0, ammunition from the chapter, lives = `sgp_ctMaxLives` (3), score and secrets 0, kills 0, weapons = default | past chapters (none) | 0x1.
2. **Respawn after death (boot 2), 2 lives left before death.** One life is taken first (`ExecuteRespawn`) -> 1 left. Row 3,0,1,1: health and armour from the **chapter**; lives := the **current** inventory's lives (1); weapons := the saved player's set; kills from the record; score and secrets unchanged.
3. **Restart chapter (boot 3).** Row 1,1,1,1: everything from the saved player (health, armour, 12 ammunition counts, lives, weapons, last weapon), score and secret count from the record, kills from the record.
4. **Level change (boot 10), chapter with lose-weapons 1 and added weapons 0x4, the player holds 0x1 | 0x2 and 40 bullets, the chapter ammunition container gives 100 bullets at ratio 0.5:** row 2,3,2,3: health := max(chapter, current) (and armour likewise); each ammunition count := max(40, chapter's); weapons: cell 2 with lose-weapons 1 **and** boot type 10 -> the current weapons are **dropped** -> default | 0x4; lives := `sgp_ctMaxLives`; score and secrets := 0 (boot type 10 is not 6 or 8); kills 0.
5. **Level score** (section 5): Normal, 2 lives, 61 kills, 4 of 5 secrets, score 12,300, 700 s against `ETA` 00:15:00 = 900 s -> 5,000 + 2,000 + 610 + 800 + 200 + 12,300 = 20,910. The same with cheating: 12,300.
6. **Last chapter of a level.** The chapter has no next chapter and the next level path `Content/SeriousSam2/Levels/<set>/<level>/<level>`: the record stores chapter id -1 and that path; "Continue playing" then starts that level's first chapter with boot type 0 (no chapter id to find) after `PrepareChapterToStart` step 6: the chapter of id -1 is not looked up.
7. **`AddWeaponsFromPastChapters`.** Chain A (adds 0x1) -> B (lose-weapons, adds 0x2) -> C (target): step 1 gives 0x1, then at B clears to the default set and adds 0x2; the result before C's own is default | 0x2. A weapon pickup placed in A is **not** counted (L = B: only items in (B, C) or in B count); one placed in B counts.
