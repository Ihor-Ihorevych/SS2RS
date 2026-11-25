# Entity iterators, entity arrays and handles of `Sam2Game.dll` (T01-113, part 12 of T01-026)

- **Roadmap task:** T01-113 (the last part of the class inventory T01-026; overview and table of all 59 classes: [`helper-classes.md`](helper-classes.md); the animation events: [`helper-classes-events.md`](helper-classes-events.md); the parameter classes: [`helper-classes-params.md`](helper-classes-params.md); the inventory of all 932 classes: [`class-inventory.md`](class-inventory.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the export counts and names, for the member names of the six iterator classes whose member tables the binary declares (read from the tables; one reading plus the constructors' member order), and for the statement that none of the 17 iterators is in the data; **medium** for what each iterator does per entity (each was read once, in full, and each is 94 to 493 bytes long) and for who runs it (found from the callers of its constructor and type record); **low** for the three iterators that no exported function constructs, for the boot iterator's condition and for the spawn marker iterator's last test.

Prerequisites: [`helper-classes.md`](helper-classes.md) (basis marks, groups), `mechanism-classes-world-info.md` (`CWorldInfoEntity`, `CChapterInfoEntity`), `players-session-classes.md` (`CGameInfoEntity`, `CPlayerBrainEntity`, `CPlayerInventory`), `save-game.md` (chapter restore).

## Summary

An **entity iterator** is a small object that the world hands to its "iterate all entities" service: the service calls the iterator's one virtual function, `ProcessEntity`, once for every entity of the world, and the iterator does one job with it (collect it, call a hook on it, remember the best one). `Sam2Game.dll` exports **17** of them (`C...Iterator`, 8 exported functions each: the standard seven plus `ProcessEntity`; 136 functions). They derive from the engine's `CEntityIterator` (named by the binary's member tables of six of them; the others share the constructor pattern). **None is in the data** (no type table names an iterator or an `S` twin of one); six have a member table in the DLL (section 3). The two other classes of this page are the **entity array** `CEntityArray` (a growable list of entity handles with 10 named methods, 16 functions) and the **entity handle** `CEntityHandle` (4 functions, a 4-byte handle value).

| Fact | Value | Basis |
|---|---|---|
| Iterator classes / exported functions | 17 / 136 (7 standard + `ProcessEntity` each) | export table |
| Iterators in the data | 0 of 17 | `tools/re/helper_classes_verify.py`, section 1 |
| Iterators with a member table in the DLL | 6, all with parent `CEntityIterator` (4, 1, 2, 3, 1 and 1 members; section 3) | declared tables (names), medium for the parent of the rest |
| Iterators constructed by an exported function | 14; the other 3 (`CAddAllWeaponsIterator`, `CCurrentStateInfoIterator`, `CStateCallsInfoIterator`) are referred to only by their own type registration | callers of the constructor and of the type record, **medium** |
| Users | `CWorldInfoEntity` (10 iterators), `CPlayerBrainEntity` (2), `CGameInfoEntity` (1), `CPlayerPuppetEntity` (1) | callers, medium |
| Entity array / handle | `CEntityArray` 16 functions (10 named), `CEntityHandle` 4 | export table |

**Headline findings.**

1. **The world info entity runs most iterators**: it collects four kinds of entity into its own lists (environment damagers, mission objectives, navigation beacons, secrets) at the start of a world, finds the game info entity, executes and disables the macro entities, shuts every entity down and boots every entity again on a chapter restore, and draws the debug health numbers over the puppets (section 2). **Medium** (each caller read once).
2. **A chapter carries the weapons of the player.** When a player arrives at a later chapter, the brain gives the inventory the weapons of all earlier chapters: the chapters' own weapon sets are added and every weapon pickup placed in the chapters since the last "lose weapons" chapter is received (`CAddPastChaptersWeaponsIterator`, section 2.2). This explains the `ep_ulWeaponsAddedToPlayer` and `ep_bLoseWeapons` members of the chapter info (`mechanism-classes-world-info.md`). **Medium**.
3. **A deathmatch spawn marker is chosen to be as far as possible from the nearest other player**: for every marker the squared distance to each of the up to 17 players is taken, the smallest is the marker's score, and the marker with the largest score wins (section 2.5). **Medium** for the rule, **low** for one test of the loop.
4. **The three debug string iterators** only append text (the state of an entity or the call stack of its last state change) for entities selected by the debug dump configuration; **no exported code constructs two of them**, so they may be run from the console (section 2.6, open).

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were listed; the 932 exported classes were grouped by name (T01-026 split, group "P11 helpers/global params/events/iterators", 59 classes). The 17 iterators export 136 functions; names are quoted as exported.
- **Reading.** All 17 `ProcessEntity` functions and all 17 default constructors were decompiled and **read in full** (basis **R**), as were the ten `CEntityArray` functions and the constructors of the two arrays' users. The callers of each constructor, of each class's virtual table and of each class's type record were listed (so the users in section 2 were found by what refers to the class, not guessed); six of the users were read in full: `AddWeaponsFromPastChapters`, `CollectSecretEntities`, `PostWorldChange`, `BootPlayerInDeathmatch`, `CalculateGameTotalStats` and `RenderPuppetsHealth`; the others were listed only (basis **P**).
- **Member tables.** The binary's declared member tables (names and parent) of the six `S...Iterator` classes were read; byte offsets were read to order the members but are **not recorded** (`conventions.md`).
- **Data.** `tools/re/helper_classes_verify.py` (section 1) confirms that none of the 17 iterator names, nor an `S` twin, is in any of the 8,063 type tables.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run, so no iterator was observed working; the engine's iteration service itself (`Engine.dll`: its order, whether entities created during iteration are visited); the exact semantics of two numeric arguments (section 2.8).

## Layout / rules

### 1. The iterator interface

Every iterator has a virtual function table with one function of its own, `ProcessEntity(entity handle)` (**R**, **medium**). The engine's iteration service (not exported by this DLL: `wldIterateEntities` and `wldIterateEntitiesSorted` are engine functions the DLL calls) calls it once per entity. All 17 start the same way: resolve the handle to an entity pointer, test the entity's class against one class (`mdIsDerivedFrom`: **derived classes count**, except where stated), and do nothing for every other entity. The iterator's own state is a few members after the table pointer; the creator sets them before the walk and reads the result afterwards. A user creates the iterator, sets its members, calls the service, reads the result, and deletes the iterator; the six users read in full all do exactly this.

### 2. The 17 iterators

"Members" gives the declared names where a table exists (section 3). "User" is the exported function that constructs the iterator.

#### 2.1 Collecting entities into the world info (4)

`CCollectEnvironmentDamagersIterator`, `CCollectMissionObjectivesIterator`, `CCollectNavigationalBeaconsIterator`, `CCollectSecretsIterator` (**R**, **medium**): one member, the world info entity to fill. For an entity of class `CEnvironmentDamagerEntity`, `CMissionObjectiveEntity`, `CNavigationalBeaconEntity` or `CSecretEntity` (derived classes included) the iterator appends the entity (a pointer, not a handle) to the matching list of the world info and grows the list when it is full. The user of each is `CWorldInfoEntity::CollectEnvironmentDamagers`, `CollectMissionObjectives`, `CollectNavigationalBeacons`, `CollectSecretEntities`; all four are called by `CWorldInfoEntity::PostWorldChange` (read) and, by `mechanism-classes-world-info.md`, by `OnSimStart`. `CollectSecretEntities` first frees and empties its list (read); the other three were not read for this. The lists are the entity arrays of the world info; their member names are not recovered (`T01-215`). The lists are what the objective, secret and beacon features of the world info read later (`mechanism-classes-triggers.md`).

#### 2.2 Weapons of earlier chapters (2)

- **`CAddPastChaptersWeaponsIterator`** (4 members: `eit_penWorldInfo`, `eit_henStartFromChapter`, `eit_henCurrentChapter`, `eit_ppiPlayerInventory`; **R**, **medium**). For each entity that is a weapon pickup (`CWeaponItemEntity` and derived), it asks for the chapter the pickup belongs to. If that chapter exists and the start chapter differs from the current chapter: when the start chapter is **not set**, the pickup's weapon is received when the pickup's chapter lies **before** the current chapter; when the start chapter **is set**, it is received when the pickup's chapter lies **between** the start and the current chapter or **is** the start chapter. "Received" means `CPlayerInventory::ReceiveWeapon` with the pickup's weapon index. The two chapter tests are the world info's `IsChapterBefore` and `IsChapterBetween` (two of its virtual functions); whether `IsChapterBetween` includes its upper end was not read. User: `CPlayerBrainEntity::AddWeaponsFromPastChapters(inventory, current chapter)` (**R**): it first walks the chapter chain of the world info from the first chapter up to (not including) the current one; for every chapter it **clears the inventory's weapons and ammunition when the chapter's lose-weapons flag is set** and **adds the chapter's weapon set to the inventory's weapon flags**; then it walks the chain again to find the start chapter (the **last** chapter with the flag, up to and including the current chapter) and runs the iterator over the whole world. The chapter members used are the chapter info's weapon set (`ep_ulWeaponsAddedToPlayer`) and lose-weapons flag (`ep_bLoseWeapons`); they are identified by what the code does with them (the first is OR-ed into a weapon bit set, the second clears the inventory), **medium**.
- **`CAddAllWeaponsIterator`** (one member: the player inventory; no member table; **R**, **medium**). For each weapon pickup it receives the pickup's weapon; for each chapter info entity it OR-s the chapter's weapon set into the inventory's weapon flags; it does nothing when no inventory is set. **No exported function constructs it** (only its type registration refers to it): it may be for a console command or a cheat that gives every weapon of the level. `player-rules.md` records a different "give all weapons" source (`ggp_wfGiveAllWeaponsFlags`, 49,151 = 15 of 16 weapon bits); how the two relate is open (`T01-208`).

#### 2.3 Hooks on every entity (4)

- **`CContributeToGameTotalStatsIterator`** (member `eit_pgsGameStats`, the statistics block to fill; **R**, **medium**): for every entity of class `SBaseEntity` (that is, every entity of the game's own layer) it calls the entity's `ContributeToGameTotalStats` hook with the block and the two constants 1 and 0 (their meaning was not read). The hook is new in the S layer and overridden by `CPuppetEntity` (`worldcontent-classes.md`). User: `CGameInfoEntity::CalculateGameTotalStats` (**R**): **host only** (`netIsHost`); it resets the game statistics block, runs the iterator and deletes it. This is how the level's totals (enemy count and so on) are computed (`players-session-classes.md`, section 8).
- **`COnShutDownIterator`** (no members; **R**, **medium**): for every `CBaseEntity` it calls the entity's shut-down hook and then **sets bit 0 of the entity's flag word**.
- **`COnBootIterator`** (members `eit_penWorldInfo`, `eit_henBootingChapterInfo`, `eit_pscsStoredChapterStatus`; **R**, **low** for the condition): for every `CBaseEntity` it decides whether to call the entity's boot hook and, when it does, calls it and **clears bit 0** of the entity's flag word (the bit that `COnShutDownIterator` sets: "has been shut down"). The decision combines: the world info's chapter test `IsChapterBefore(the entity's chapter, the booting chapter)`; whether the entity's flag bit is set; whether the world info's boot type is the one numbered 2 and a stored chapter status was given; and one more per-entity test (a function that was not identified). The reading supports only this: an entity that **is in an earlier chapter than the booting one is not rebooted when a stored chapter status is restored**, every other entity that was shut down is booted. User: `CWorldInfoEntity::ResetWorldToChapter_internal`, which also uses the next two (`T01-209`; the whole restore belongs to T01-172).
- **`CPuppetsHealthDebugIterator`** (no members; **R**, **medium**): debug drawing. For each puppet (`CPuppetEntity`) that the debug dump configuration selects, and whose handle at one member is empty and whose counter at another is zero (two members the reading could not name), it asks the puppet for its **health** (`GetHealth`), its bounding box and its colour, and draws the number in 3D text at the puppet's position raised along a fixed direction by a multiple of the box height (three static floats give the direction). User: `CWorldInfoEntity::RenderPuppetsHealth` (**R**), which runs only while a simulation exists.

#### 2.4 The macro entities (2)

`CExecuteMacrosIterator` and `CDisableMacrosIterator` (no members; **R**, **medium**): for every entity of one class (the class of the macro entities, `CMacroEntity` of `wld-macro.md`; identified by the data class that stores the macros and by the iterator names, **medium**; the class is an engine class, not exported by this DLL) the first runs the entity's macro and the second disables it, each through one non-virtual function. Both are created by `CWorldInfoEntity::ResetWorldToChapter_internal`. `save-game.md` says that the restore "finally runs every macro entity (skipped when the boot type is 11)"; this page adds that **disabling** is done by the same iterator family, so macros running at the moment of a restore are stopped before the entities are shut down (order inferred from the existence of both, **low**).

#### 2.5 Finding one entity (3)

- **`CFindFirstEntityOfTypeIterator`** (members `eit_henEntity`, `eit_idEntityClass`; **R**, **medium**): remembers the **first** entity whose class identifier **equals** the stored identifier (an **exact** class, not derived classes: it compares the entity's own class name identifier); later entities are ignored. User: `CWorldInfoEntity::PostWorldChange` (**R**) sets the identifier from a class name string, runs it, casts the result to `CGameInfoEntity` and stores it as the world info's game info entity handle; before that it prints the warning "Game info entity ptr is not NULL and should be." when one is already stored. So **after every world change the world info looks up the game info entity by class** (**medium**; the string was not read).
- **`CFindSpawnMarkerIterator`** (members: the player's own handle to exclude, the best marker, the best score; **R**, **medium**, the last test **low**): for every `CSpawnMarkerEntity` it takes the marker's position and, over the **17 player handles** of the game info entity (`en_ahenPlayers`), the squared distance to the position of every other live player; the smallest of them is the marker's score (a very large number, 3 x 10^38, when nobody else exists). The marker whose score is **not smaller** than the best so far becomes the best, so **later markers win ties**. One more pointer test in the same condition (on the loop's last handle) was not understood. User: the deathmatch start-up path of `CPlayerBrainEntity::BootPlayerInDeathmatch`, through the function that picks the spawn placement (`samGetPlayerSpawnPlacement`, internal) (**P**). This is the rule for **deathmatch only**; the co-op and single player start markers are chosen elsewhere (`T01-210`).
- **`CCollectChaptersForSortingIterator`** (no members; **R**, **medium**): for every `CChapterInfoEntity` it calls the entity's `CollectForSortedIteration` hook (slot 24 of the base entity). User: `CPlayerPuppetEntity::CollectForSortedIteration` (**P**): when the player puppet is asked to take part in a sorted iteration it first has the chapters collected, so that chapters come before the entities that depend on them in a sorted walk (**low**).

#### 2.6 Debug text (2)

`CCurrentStateInfoIterator` (member `eit_strCurrentStates`, a string) and `CStateCallsInfoIterator` (member `eit_strStateChangeInfo`) (**R**, **medium** for what they append, **low** for who runs them): for every `CBaseEntity` that the **debug dump configuration** (the global `sam_ddc`) selects, the first appends to the string a line with the entity's current **state name**, the text ` state: ` and the entity's **class name**; the second appends a line with the state name, the text ` enters state: `, the class name and then the **call stack** of the state change. The order of the pieces inside a line was not verified. **No exported function constructs either of them** (nor `CAddAllWeaponsIterator`); the strings are meant for a log or a debug display and the classes are probably created by name from the console (`T01-208`).

### 3. Member tables of the iterators

All six tables have the parent `CEntityIterator`. The first column is the class's saved name in the table.

| Table | Members in declaration order | Basis |
|---|---|---|
| `SAddPastChaptersWeaponsIterator` | `eit_penWorldInfo`, `eit_henStartFromChapter`, `eit_henCurrentChapter`, `eit_ppiPlayerInventory` | table; order checked against the constructor and `AddWeaponsFromPastChapters` (high) |
| `SContributeToGameTotalStatsIterator` | `eit_pgsGameStats` | table |
| `SFindFirstEntityOfTypeIterator` | `eit_henEntity`, `eit_idEntityClass` | table; order checked against `ProcessEntity` |
| `SOnBootIterator` | `eit_penWorldInfo`, `eit_henBootingChapterInfo`, `eit_pscsStoredChapterStatus` | table |
| `SStateCallsInfoIterator` | `eit_strStateChangeInfo` | table |
| `SCurrentStateInfoIterator` | `eit_strCurrentStates` | table |

The names say what is held: `pen` = pointer to entity, `hen` = handle to entity, `ppi` = pointer to player inventory, `pgs` = pointer to game stats, `pscs` = pointer to stored chapter status, `id` = identifier, `str` = string. The other 11 iterators have no table and (the collectors, the macro and shut-down iterators) no state beyond one pointer.

### 4. `CEntityArray` and `CEntityHandle`

**`CEntityHandle`** (4 exported functions: the default constructor, two operators and `mdGetDataType`; **R**, **high** for the layout): a plain value of **4 bytes**, an entity handle (the constructor stores zero = no entity). The engine resolves a handle to an entity with `hvHandleToPointer` and back with `hvPointerToHandle`; a handle whose entity was deleted resolves to nothing, which is why the code tests every handle before use. In the data a handle is stored as one object id (`ctsemeta-objects.md`: `Handle<X>`, `0xFFFFFFFF` = null); the class name `CEntityHandle` occurs once in the data, as the element type of `CEntityContainer.ea_aehEntities` (59 `.wld`).

**`CEntityArray`** (16 exported functions: 6 standard + 10 named; **R**, **medium**): a growable list of `CEntityHandle` values, the same shape as the engine's static stack array: **capacity, pointer to the elements, count, growth step**; the constructor starts it empty with a growth step of **16** (its constructor is the same machine code as the one of `CAmmoIcons`: the linker shared them). The members of other classes that hold entity lists carry the prefix `en_ea` or `ea_` (for example `en_eaBrainsWithoutFoe` of the world info; the data's `CEntityContainer.ea_aehEntities`); `CEntityArray` is constructed by the constructors of `CPuppetEntity`, `CPuppetParams`, `CPuppetSeatData`, `CWayToSeat`, `CBaseWeaponEntity`, `CDoorEntity`, `CStaticModelAnimEventsData` and by `CMultiSpawnerEntity::OnCreate` (callers of the constructor), so the entity lists of the puppet, the seats, the weapon, the door, the multi-spawner and the static model events data are of this class (**medium**; which member is which is open, `T01-215`).

| Method | What it does (all **R**) |
|---|---|
| `Add(handle)` | appends at the end; when the count would exceed the capacity the storage grows by the growth step |
| `Clear` | when storage exists, frees it and resets capacity, pointer and count to zero |
| `ClearNullEntries` | walks the list and removes, **keeping the order**, every entry whose handle no longer resolves to an entity |
| `Count` | the count |
| `GetCountOfNullEntries` | how many entries no longer resolve |
| `GetMember(i)` | the handle at index `i`; **no range check** |
| `Index(handle)` | the index of the first entry equal to `handle`, or **-1** |
| `IsMember(handle)` | whether an equal entry exists |
| `RemoveByValue(handle)` | removes the **first** equal entry by moving the **last** entry into its place (**order not kept**); nothing when absent |
| `RemoveOrderedByValue(handle)` | removes the first equal entry and shifts the rest down (**order kept**) |

Comparison is by the raw handle value (equal values are equal entities). Whether the array is replicated depends on the owner's member flags (`replication.md`); the class itself has no net behaviour.

## Invariants

1. None of the 17 iterator names is in any type table of the install, under its own name or as an `S` twin (script, 8,063 files).
2. All 17 iterators have exactly the standard seven functions plus `ProcessEntity` (export table: 17 of 17 have 8).
3. Of the six member tables, every member name starts with `eit_` and the parent is `CEntityIterator` (6 of 6).
4. `CEntityHandle` stores one 32-bit value and `CEntityArray`'s elements are `CEntityHandle`: the same size as the data's object id (4 bytes).

## Open questions

1. **Who runs `CAddAllWeaponsIterator`, `CCurrentStateInfoIterator` and `CStateCallsInfoIterator`**, and the debug dump configuration's switches: **T01-208**.
2. **The boot iterator's exact condition**, the per-entity test it calls, and the order of events of a chapter restore: **T01-209** (with T01-172).
3. **Which spawn marker a single player or co-op start uses**, and the last test of `CFindSpawnMarkerIterator`: **T01-210**.
4. **Which member of each owner is a `CEntityArray`** (and which are replicated): **T01-215**.
5. The engine's iteration service (order, entities created during a walk): **T01-211**.

## Test vectors

1. The player being placed excludes himself. Two other players stand at B = (0, 0, 0) and C = (30, 0, 0); two markers are M1 = (5, 0, 0) and M2 = (0, 0, 100). For M1 the squared distances are 25 (to B) and 625 (to C): score 25. For M2 they are 10,000 (to B) and 10,900 (to C): score 10,000. M2 has the larger score and wins. With only B present the scores are 25 and 10,000, the same winner; with nobody else present both scores are 3 x 10^38 and, because ties go to the later marker, the last marker in iteration order wins. (Hand computation of the rule in section 2.5; **medium**.)
2. `CEntityArray`: after `Add(a)`, `Add(b)`, `Add(c)`, `RemoveByValue(a)` the order is `c`, `b` and `Count` is 2; after `Add(a)`, `Add(b)`, `Add(c)`, `RemoveOrderedByValue(a)` the order is `b`, `c`. `Index(x)` of an absent handle is -1.
