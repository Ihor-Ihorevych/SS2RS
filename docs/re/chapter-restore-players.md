# Chapter restore: the player rebuild, the chapter save, level end and the end sequence, and the single-player/co-op start (T01-172, T01-210, T01-411)

- **Roadmap task:** T01-172 (second part); section 2.8/2.9 is T01-210; section 2.10 is T01-411 (leftovers of T01-210). First part: [`chapter-restore.md`](chapter-restore.md) (boot types, reset sequence, the boot rule, chapter activation). The chapter save's record layout is in `save-game.md` section 6 and `players-session-classes.md` section 8. The co-op placement offset table is in [`door-mover-rules.md`](door-mover-rules.md) section 10; the deathmatch spawn marker rule, and the solo-spawn marker collector, are in [`helper-classes-iterators.md`](helper-classes-iterators.md) section 2.5.
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the 12 x 4 table (plain data of the DLL), for the member names and byte-position matches of the saved records (the binary's member tables, all names found by position), for the worlds census of section 2.9 (`tools/re/player_start_verify.py`, all 59 worlds) and for section 2.10's `en_henParent` census (`tools/re/chapter_start_ground_verify.py`, all 307 chapter info entities); **medium** for every rule (one reading each, the lost arguments recovered from the instruction listing), including section 2.8 (`GotoCurrentChapter`, `samGetPlayerPlacementOnChapter`, `samGetPlayerSpawnPlacement`, `samGetFallDownPosition`, each read in full) and section 2.10's reading of `samGetFallDownPosition`'s own ray query; **low** for the points marked "open", including the two callers T01-411 still could not find and the exact name of the solo-spawn marker class. **Nothing was run on the original game.**

Prerequisites: [`chapter-restore.md`](chapter-restore.md), `players-session-classes.md` (`CPlayerInventory`, `CPlayerSave`, `CGameStats`), `inventory.md` and `weapons.md` (weapon and ammunition indices), `save-game.md`, `cutscenes.md` (the `.nfo` movie lists), `misc-types.md` (the `.nfo` keys).

## Summary

1. **One 12 x 4 table of small numbers drives the whole player rebuild.** Row = boot type, column = what is rebuilt: **A** health, armour and ammunition; **B** lives (and score and secret count); **C** weapons; **D** kill count. A cell says where the new value comes from: 0 keep the current one, 1 take it from the saved player, 2 merge the current and the chapter's values (the larger), 3 take it from the chapter info or a default, 4 reset to the inventory default (section 1).
2. **Respawn after death keeps the lives and takes the chapter's health, armour and ammunition**; the weapons come from the saved player (row 2 = 3, 0, 1, 1). A **chapter restart** (row 3) and a **continue** (row 1) rebuild everything from the saved player (1, 1, 1, 1). A **new game, a level restart, a custom start and a developer reset** (rows 0, 5, 9) take everything from the chapter (3, 3, 3, 3); a **level change** (row 10) keeps the weapons and the larger of current and chapter ammunition (2, 3, 2, 3).
3. **The chapter save** (`CStoredLevelStatus`) is rebuilt at **every chapter start**; for the **last** chapter of a level it stores the **next level's path and no chapter id**, which is how "Continue playing" starts the next level (section 3).
4. **Level end score** (section 5): difficulty bonus 1,000 / 3,000 / 5,000 / 7,000 / 10,000, **1,000 per remaining life, 10 per kill, 10 per percent of secrets found, 1 per second under the level's `ETA` time**, plus the score; **0 bonus when cheating**.
5. **`TheEndSequence(movie)`** queues, in this order, a start marker, the level's outro movies, the statistics menu (local sessions), the optional extra movie and an end marker (section 6).
6. **No `CSpawnMarkerEntity` is ever used for a single-player or co-op start: that class exists only in the 16 deathmatch worlds** (0 exceptions, all 59 worlds checked). A single-player or co-op start is the **world's own first chapter, found by following the chain from the world info, not by taking the first `CChapterInfoEntity` in the world's entity list** — the two disagree in **31 of the 51 worlds that have a chain** (3 of those 31 even put an off-chain chapter first in entity order). The chapter's own placement (position and orientation) is the start, unshifted for the "slot 16" player and offset per slot for a co-op player beside them (section 2.8, section 2.9).
7. **A loader must not test only static models for the ground under a chapter.** `samGetFallDownPosition`'s own ray query is the engine's generic line-of-sight check (section 2.10), which hits movers, doors and any other collidable entity, not just `CStaticModelEntity`. Two single-player starts (`5_1_Village`, `5_2_SeasideTownApproaching`) land far from static ground precisely because their chapter is not parented to anything (`en_henParent` is null, ruling that out) but sits near non-static geometry instead (section 2.10).

## Evidence

- **Code.** `CPlayerBrainEntity::BootPlayerOnChapter` (2 kB, decompiled and read in full; the boot-type table accesses and the branches on each column verified in the listing), `AddWeaponsFromPastChapters`, `StoreChapterSaveData`, `OnBoot`, `GotoCurrentChapter`, `BootPlayerInDeathmatch`, `ExecuteRespawn`; `CAddPastChaptersWeaponsIterator::ProcessEntity`; `CFindSpawnMarkerIterator::ProcessEntity`; `CPlayerInventory::ResetAllToDefault`, `ClearWeapons`, `ClearAmmo`, `SetAmmoFromChapter` (instruction listing for the constant), `CGameInfoEntity::GenerateChapterSaveGame`, `StorePlayersData`, `SetLevelStatus`, `RestoreSavedSecrets`, `RestoreSavedObjectivesAndNavBeacons`, `DisableChapterResurrection`, `OnBoot`, `SpreadChapterFinishedEvent`, `SpreadLevelFinishedEvent`, `UpdateGameDifficultyParams`, `CalculateGameTotalStats`, `CChapterInfoEntity::MarkStarted`, `CGameInfoEntity::CreatePlayer`, `PlayerAdd`; `samGetPlayerPlacementOnChapter`, `samGetPlayerSpawnPlacement`, `samGetFallDownPosition` (all three read in full for T01-210); `CChapterInfoEntity::GetAbsPlacement`, `OnUpdate` and `CBaseProperties::GetParentAspect` (read in full for T01-411, section 2.10); the two helper functions `samGetFallDownPosition` calls into, which carry the engine's own `"LOS()"` profiler tag (T01-411); the profile's level statistics function and the level information lookups it uses; `CWorldInfoEntity::TheEndSequence`, `macTheEndSequence`; the script-call stubs of both.
- **Member names** by byte position in the binary's member tables: `CGameStats` (`gs_ctSecrets`, `gs_ctSecretsTotal`, `gs_ctKills`, `gs_ctKillsTotal`, `gs_slScore`, `gs_tmPlayingTime`, `gs_slPlayingTime`, `gs_bCheating`, `gs_bSaving`, `gs_bFrozenStatistic`), `CStoredLevelStatus` (`sls_scsFirstChapter`, `sls_scsCurrentChapter`, `sls_sgpGameParams`), `CStoredChapterStatus` (`scs_strWorld`, `scs_ulChapterEntityID`, `scs_gsGameStats`, `scs_ahpsPlayerData`, `scs_ceidUnableToResurrect`, `scs_anbdNavigationalBeacons`, `scs_asedSecretEntities`), `CPlayerSave` (`ps_*`), the player puppet's `en_slHealth`, `en_slArmor`, `en_piPlayerInventory`, the session options `sgp_*`.
- **Data.** The `.nfo` keys behind the level `ETA` and the movie lists (`misc-types.md`); counts in `chapter-restore.md`; `python -I tools/re/player_start_verify.py` (about 3 minutes; T01-210, section 2.9): every world's spawn marker count and whether its chapter chain's own first chapter is the first `CChapterInfoEntity` in entity-list order, over all 59 worlds. The two worked examples' exact positions and headings were read with `ss2-format-wld`'s own `World::from_bytes` (already tested by `crates/ss2-format-wld/tests/install.rs`), as a one-off check against the retail install, not a new committed tool. `python -I tools/re/chapter_start_ground_verify.py` (T01-411, section 2.10; about 3 minutes): `en_henParent` over all 307 chapter info entities of the install, and the entities placed near the two odd worlds' first chapter.
- **Not checked.** Nothing run; the profile's medal computation, the effect of the `OnChapterFinished` and `OnLevelFinished` player hooks, the movie sequencer's actions, who calls `GotoCurrentChapter` for a brand new game and who assigns a player's slot number before `CGameInfoEntity::CreatePlayer` (section 2.8; T01-411 widened the search to `Sam2.exe`, `Engine.dll`, `Core.dll`, `Sam2GameGui.dll` and `EngineGui.dll` without finding either caller — see section 2.10 and open question 6); the exact collision geometry under the two odd worlds' chapters (no hull/mesh data read, section 2.10).

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

### 2.8 Placement of a booted player: the single-player and co-op start (T01-210)

**No "player start" entity exists.** The full class inventory (`class-inventory.md`, T01-026) has no `CPlayerStartEntity` or equivalent; the only placement-only marker class is `CSpawnMarkerEntity` (`mechanism-classes-spawners.md`), and it is used by exactly one function, `CFindSpawnMarkerIterator`'s user, which is deathmatch-only (`helper-classes-iterators.md` section 2.5; confirmed again below). A single-player or co-op level has no spawn marker to read.

**The start is the world's own first chapter, found by its chain, not by file order.** `CWorldInfoEntity`'s first-chapter handle (`CWorldInfoEntity.2`, `en_henFirstChapter`) names the `CChapterInfoEntity` that `PrepareChapterToStart` starts a new game at (`chapter-restore.md` section 2.3); it is **not**, in general, the first `CChapterInfoEntity` a reader meets while walking the world's entity list (`CWorld.1`) — the order `ss2-format-wld`'s `World.chapters` keeps today, and the order the playable MVP's loader falls back to (`crates/ss2-world/src/load.rs`). Section 2.9 gives the full-data count: the two orders disagree in most worlds.

**Position and heading are the chapter's own placement, unmoved; a slot offset shifts only the position.** The one function that places a player at a chapter, `samGetPlayerPlacementOnChapter(chapter, slot, puppet)` (read in full), has exactly three call sites in `Sam2Game.dll`: the player brain's own `GotoCurrentChapter`, the portal's touch handler and the script command `TeleportPlayersToCurentChapter` — already read for `door-mover-rules.md` section 10, which this page now treats as the source of the placement rule rather than duplicating it:

1. It reads the chapter's own world placement (position and orientation; the same placement every other entity has).
2. It shifts the **position only** by `R * (a, 0, b)` metres, `R` the chapter's own rotation and `(a, b)` a fixed 17-entry table keyed by the player's **slot number** (0 to 16; `door-mover-rules.md` section 10's table, read again for this page and found to match exactly). **Slot 16 gets `(0, 0)`: that player stands exactly at the chapter's own position**, facing the chapter's own heading, unshifted. Every other slot rings around it (an inner ring of 8 at slots 0–7, an outer ring of 7 at 8–14; 15 and anything else gets the ring's last spot).
3. The **orientation is always the chapter's own**, never shifted by the slot or by anything else.
4. If the chapter's `en_bAutoFallDown` flag is set, the shifted point is raised 2 m along the up axis and a ground search (`samGetFallDownPosition`, read in full: casts a ray down from the raised point along the navigation mesh / collision world, not detailed further here) may replace it with the grounded position for that specific player; if the search fails, the raised point is used as is.

**Reader rule for a loader (medium confidence), single player and co-op alike:**

1. Read the world info entity's first-chapter handle and follow it to the `CChapterInfoEntity` (one world info per world, `wld-root.md`; a world with no first chapter and no chapters has no scripted start at all — out of scope here).
2. The spawn **position** is that chapter's world translation.
3. The spawn **heading** is the yaw of that chapter's world rotation (the same convention `load.rs`'s existing `yaw_of` already uses for other markers).
4. For more than one player (co-op), place every other player beside the same chapter, offset by their own slot's `(a, b)` pair (above); the single local/host player (and, in co-op, the slot that boots with the world's own boot type rather than `CBT_COOP_JOIN` — `chapter-restore.md` section 2.2 already names slot 16 as that slot) needs no offset at all.
5. `CSpawnMarkerEntity` plays no part in any of this; a loader that prefers it over the chapter (as `load.rs` does today) will never actually read one in a single-player or co-op level (section 2.9), but should not look for one there either.

**What this page could not pin down (medium to low; follow-up T01-411).** `GotoCurrentChapter` is the only function found that calls `samGetPlayerPlacementOnChapter` for a live player rather than a portal or a script command, but it is **not** called by `CPlayerBrainEntity::OnBoot` or by `BootPlayerOnChapter` (both read in full: `OnBoot` only calls `AssureThatIsBooted` and `BootPlayerOnChapter`/`BootPlayerInDeathmatch`, section 2 above; `BootPlayerOnChapter` sets inventory, health, weapons and kills but never a position) — so the exact moment a brand-new game's player is first placed is not demonstrated end to end inside `Sam2Game.dll`; it is plausibly a call from the engine itself (`Core.dll`/`Engine.dll`, not decompiled for this page) once the puppet is spawned, or the puppet's placed position is simply left alone until the first boot or teleport moves it. Which slot a given player is assigned (`CGameInfoEntity::CreatePlayer(slot, name)` takes the slot as a plain argument; its own caller, which presumably picks slot 16 for the local/host player and a free slot in 0–15 for anyone else, was not found) is open too. `BootPlayerInDeathmatch`'s own placement path, `samGetPlayerSpawnPlacement`, has a second rule not previously recorded: with **exactly one player** in the session it does **not** run `CFindSpawnMarkerIterator` at all — it instead collects candidates of a (not fully identified) marker class by a different iterator and picks **one at random**, falling back to a hard-coded default placement with a console warning if none exists; `CFindSpawnMarkerIterator`'s farthest-from-nearest-player rule (`helper-classes-iterators.md` section 2.5) only runs with **two or more** players. Neither path was run.

### 2.9 Two worked single-player starts, and the full-world count (T01-210)

`tools/re/player_start_verify.py` (about 3 minutes, all 59 worlds; high confidence, full data) confirms section 2.8's two data claims over every world:

- **Every world with a `CSpawnMarkerEntity` is a deathmatch level.** 16 of 59 worlds have at least one (4 to 20 each, 177 total, matching `mechanism-classes-spawners.md`); **all 16 file names start with `dm_`**; the other 43 worlds, single player and co-op alike, have none.
- **The chain's own first chapter is usually not the first `CChapterInfoEntity` met in the world's entity list.** Of the 51 worlds that have a chapter chain, the two agree in only **20**; in the other **31** they disagree, and in **3** of those 31 (`7_1_City_Part_I`, `7_2_City_Part_II`, `1_5_Mdigbo_CitySuburbs`) the entity-order "first" chapter is not even on the chain.

Two worked examples (chain from the script above; position and heading read with `ss2-format-wld`'s existing `World::from_bytes`, a one-off check against the retail install, no new product code):

| World | Chain (`CChapterInfoEntity.7`, in order) | Chain's first chapter | Its rank in entity-list order | Position (m, engine axes) | Heading (yaw) |
|---|---|---|---|---|---|
| `1_1_Jungle` | Chapter001 → Chapter002 → Chapter003 → Chapter004 → NextLevelLink (5 chapters, no branch) | `Chapter001` (serial 3619) | **4th of 5** (`Chapter003`, serial 2532, is 1st in entity order) | (263.2, −0.6, −242.0) | ≈ 104.5° |
| `7_1_City_Part_I` | Chapter001 → Chapter002 → Chapter003 → Chapter004 → Chapter005 → Chapter006 → Chapter007 → Next Level Link (8 on the chain; `Portal_Chapter005` and `temporary portal chapter` are off it) | `Chapter001` (serial 3250) | **8th of 10** (`Portal_Chapter005`, serial 9438, is 1st in entity order, and is off the chain) | (−54.7, 3.4, 39.6) | ≈ 0.0° |

So for `1_1_Jungle` the loader's current fallback (the first `CChapterInfoEntity` of `world.chapters`, i.e. `ss2-format-wld`'s entity-list order) would place the single-player start at **Chapter003**'s placement, with a different heading (Chapter003's own yaw is ≈165°); for `7_1_City_Part_I` it would pick an **off-chain** chapter never reached by any chapter activation. Neither world's `world.spawn_markers` has anything in it (consistent with the full-world count above).

### 2.10 Leftovers of T01-210 (T01-411)

**1. Who calls `GotoCurrentChapter` for a brand-new game, and who assigns a player's slot before `CreatePlayer`.** Both stay open. `GotoCurrentChapter` (virtual, takes a `ChapterBootType`) and `CGameInfoEntity::CreatePlayer(slot, name)` (virtual too) have **no caller anywhere in `Sam2Game.dll`** — not by name (every function of the DLL was already searched for T01-210) and not, as far as a text search of the decompiled output can show, in `Sam2.exe`, `Engine.dll`, `Core.dll`, `Sam2GameGui.dll` or `EngineGui.dll` either (none of the five names either symbol, not even as a quoted string for a script or console command). That rules out a caller findable by name, but **not** a caller that reaches either function through a plain virtual-table slot with no symbol attached to the call site, which a decompiled-text search cannot find; the five other binaries are also only partially decompiled locally (`Sam2GameGui.dll` has 3 of its functions available). Both therefore stay exactly where T01-210 left them: **plausibly an engine-side call once a session creates its players**, not demonstrated. **Low** confidence (an absence of a textual match is not a proof of absence).

**2. The marker class `samGetPlayerSpawnPlacement` collects with exactly one player.** `samGetPlayerSpawnPlacement` (read again in full) does **not** construct `CFindSpawnMarkerIterator` at all on this path (confirmed again: the constructor call is inside the `GetPlayersCount() != 1` branch only). On the one-player path it instead builds, in place, a second, unnamed iterator with no exported constructor of its own (its constructor is a small compiler-generated helper, not a named class method — the same situation as `CAddAllWeaponsIterator`, `CCurrentStateInfoIterator` and `CStateCallsInfoIterator`, `helper-classes-iterators.md` section 2.6): one member is a class identifier (default "invalid", like `CFindFirstEntityOfTypeIterator`'s `eit_idEntityClass`, section 2.5) and four more are shaped exactly like `CEntityArray` (capacity, pointer, count, growth step 16). Before running it, the class identifier is set from a name resolved by `strConvertStringToID` — **the literal string argument of that call is not recoverable from the decompiled output at this or any of the other ~20 call sites of `strConvertStringToID` in the DLL** (a systematic limitation of this decompile, not specific to this call). The iterator therefore **collects every entity of one exact class, by name, into a growable array** (the same "exact class" rule as `CFindFirstEntityOfTypeIterator`, not "derived classes count"); afterwards one entry is picked **uniformly at random** by index, and the pick is only accepted once the engine's own type-derivation check confirms it derives from **`CSpawnMarkerEntity`** — the same class the two-or-more-player path uses. The most economical reading is that the resolved name **is** `"CSpawnMarkerEntity"` itself (making the final check a defensive, always-true validation of the lookup, not a second filter) — but this is **not confirmed**: the lookup could just as well name a narrower sibling or script (`S`) twin that still derives from `CSpawnMarkerEntity`. The random pick itself (a generic "random in range" helper used across many unrelated functions) and the one-entity-array shape are read in full, **medium**; the exact class name is **low** (open, same as `helper-classes-iterators.md` section 2.5's note).

**3. Why `5_1_Village` and `5_2_SeasideTownApproaching` land far from static ground.** `tools/re/chapter_start_ground_verify.py` checked all four candidates against the data:

- **Not a parented chapter.** `en_henParent` is **null** on both worlds' own first chapter (read directly; **high**). Across the whole install exactly **14 of 307** chapter info entities have a parent (confirming `mechanism-classes-world-info.md`'s count independently), and the **two worlds whose own first chapter is one of the 14** are `2_4_Woodstock` and `5_6_SkyIslands` — **neither** of the two worlds this task asks about. So section 2.8's placement rule (read the chapter's own world placement) is not in question here, and a loader does not need to special-case a parent for these two starts.
- **Not a scripted or vehicle start specifically.** Both chapters have `en_bAutoFallDown` set (1), the ordinary case (304 of 307); nothing distinguishes them as "no fall-down" chapters, and no vehicle entity (`SAircraftCharacterEntity`, `SHovercraftCharacterEntity`) sits anywhere near either chapter's position. This does not rule out a macro reacting to "Started" moving the player later, but nothing at the chapter's own placement itself is scripted differently from the other 40 starts.
- **The engine's own ground search is not limited to static models.** `samGetFallDownPosition` (`chapter-restore-players.md` section 2.8) resolves, through further helpers, into the engine's **generic line-of-sight query** — the helper that performs the actual cast is tagged, in the engine's own profiler, as a line-of-sight check, the same primitive the engine uses for sight and hit checks everywhere else, which tests the **whole physical/collision world**, not `CStaticModelEntity` instances specifically. A ground search limited to static models is therefore **not** what the original game does.
- **Both chapters have non-static, collidable neighbours within a few metres.** In `5_1_Village`, the first chapter (serial 2586, `Chapter001`, at (10.07, 171.68, 494.42) m) sits inside a room that also holds an `SDoorEntity` (5.4 m away, 2.9 m above) and 18 other non-static entities within 1 m of its own height, within a 15 m radius; the nearest `CStaticModelEntity` at essentially the same height is 8.0 m away. In `5_2_SeasideTownApproaching`, the first chapter (serial 1864, `Chapter001`, at (172.67, −26.72, 1740.63) m) has a `CStaticModelEntity` only 1.4 m away but 1.2 m below, and the same scarcity of static geometry directly under its own position that the product's ground search already reported. Neither world's immediate neighbourhood was traced down to actual hull/mesh geometry (no `.bmf`/`.mdl` collision data read for this task), so the **exact** piece of geometry the original engine's line-of-sight cast would hit first is not pinned down (**low**); what the data does show, at **high** confidence, is that both starts sit in a neighbourhood with non-static collidable entities (doors, and elsewhere in the same levels, `SAnimatedMoverEntity`) that a static-model-only search skips over entirely.

**The rule for a loader.** Reproduce `samGetPlayerPlacementOnChapter` and `samGetFallDownPosition` as read, not a static-model approximation of them: take the chapter's (slot-shifted) position; when `en_bAutoFallDown` is set, raise it 2 m and cast straight down through the **whole collision world** — static models **and** the collision hulls of non-static entities (movers, doors, and anything else with a `CModelHull`/`CPrimitiveHull`) — stopping at the first hit; if nothing is hit, use the raised point as is, exactly as the original falls back (**not** an error, and **not** a search that keeps going past a non-static floor looking for a static one). A loader that tests only `CStaticModelEntity` instances will, for the handful of starts placed over a door, mover or other non-static piece of geometry, either find the wrong, much lower static floor underneath (`5_1_Village`: the apparent "53 m below" is a static floor the real ground search in the original never has to reach, because something non-static stops the ray first) or find nothing at all (`5_2_SeasideTownApproaching`). **Worked values:** `5_1_Village`'s first chapter (`Chapter001`, serial 2586) at (10.07, 171.68, 494.42) m, heading 0°; `5_2_SeasideTownApproaching`'s first chapter (`Chapter001`, serial 1864) at (172.67, −26.72, 1740.63) m, heading 0°; both read with `ss2-format-wld`'s existing `World::from_bytes`, cross-checked against the same values read independently from the raw data by `tools/re/chapter_start_ground_verify.py`.

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
6. **Placement of a booted player: resolved by section 2.8/2.9 (T01-210); the ground search and the parent hypothesis resolved by section 2.10 (T01-411).** Still open: who calls `GotoCurrentChapter` for a brand-new game, who assigns a player's slot number before `CreatePlayer` (no caller found in any of the six binaries searched, only a virtual-dispatch caller cannot be ruled out), and the exact class name `samGetPlayerSpawnPlacement`'s solo-player path resolves before picking one at random (the mechanism and its "exact class, `CSpawnMarkerEntity`-derived" shape are now read in full; only the literal name string is unrecovered, a decompiler limitation affecting every call of that one string-to-id function in the DLL). **T01-414.**
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
8. **Single-player start, `1_1_Jungle`.** Position (263.2, −0.6, −242.0), heading ≈104.5°: `Chapter001`'s own placement, slot 16's offset `(0, 0)` applied. The world's five `CChapterInfoEntity`s have no `CSpawnMarkerEntity` to compete with (`tools/re/player_start_verify.py`).
9. **Co-op join beside the same chapter, slot 2.** `door-mover-rules.md` section 10's table gives slot 2 the offset `(0, 1)`: the joining player stands 1 m behind `Chapter001`'s own position along its own backward axis (rotated by `Chapter001`'s orientation, not world axes), facing the same heading as the slot-16 player.
10. **Single-player start, `5_1_Village` (section 2.10).** `Chapter001` (serial 2586) at (10.07, 171.68, 494.42) m, heading 0°, `en_henParent` null: a loader using only `CStaticModelEntity` instances for a ground search finds the nearest one at essentially the same height 8.0 m away and the nearest one strictly below much farther down, because the room's actual floor by the chapter includes an `SDoorEntity` 5.4 m away that such a search skips.
11. **Single-player start, `5_2_SeasideTownApproaching` (section 2.10).** `Chapter001` (serial 1864) at (172.67, −26.72, 1740.63) m, heading 0°, `en_henParent` null: the nearest `CStaticModelEntity` is 1.4 m away but 1.2 m below the chapter, not level with or under it, consistent with the product's own ground search finding no static model below at all.
