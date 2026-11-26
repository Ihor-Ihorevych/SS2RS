# Spawner state machine, procedures and delays (T01-165, part 1)

- **Roadmap task:** T01-165 (follow-up of `mechanism-classes-spawners.md` open question 1). Part 2 (the spawn side: placement, formations, launchers, effects, templates): [`spawner-spawn-side.md`](spawner-spawn-side.md). Overview and counters: [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the numbering of `SpawnerState` and of `en_stSpawningType` (two independent readings: the table in `SetState`, and the call sites in six other functions whose intent matches the function each number selects; no data check is possible, every stored spawner has state 0 and type 0), for the unused members (`en_bChangeToSimple`, scan of all 75 exported methods), for the occupied containers never being filled (section 10, T01-303: two independent readings now agree), and for the "next think" scheduler's replace-on-schedule rule (section 3, T01-301: read down to the engine's own linked-list dedup, not inferred from Sam2Game.dll's side alone), and for `en_bSpawningPaused` and state 14 being genuinely dead (section 5 and invariant 1, T01-302: the empty-function finding and the exhaustive call-site count, not absence-of-evidence); **medium** for every flow rule below (one reading of the code each, the decompiler's lost arguments recovered from the instruction listing; **nothing was run**); **low** for the points marked "open".

Prerequisites: [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) (members, counters and the ten events), [`mechanism-classes-world-info.md`](mechanism-classes-world-info.md) (spawn slot containers), `level-logic.md` and `level-logic-api.md` (the script commands `SpawnSimple`, `SpawnGroup`, ...).

## Summary

A spawner is a small **state machine driven by the entity "next think" timer**. Every step is one of eleven named *state functions*; `SetState(n)` stores the number and schedules the function that belongs to it **0.00001 s** later, a function that waits schedules another function with its delay. There are no other timers: all spawner timing in the original is the delay arithmetic of section 7 plus these 10 microsecond hops.

What a reader needs to know first:

1. **State numbers** (section 2): 0 idle, 1 waiting for the spawn source, 2 finished (terminal), 3 spawn, 5 "spawn procedure" (the slot check), 6 simple entry, 7 single delay, 8 death delay, 9 waiting for a death, 10 maintain-group entry, 11 waiting for all to die, 12 respawn-by-group entry, 14 ammo events, 15 slot retry; 4 and 13 are never entered.
2. **The procedure is chosen by the script command**, not by data: `SpawnSimple` makes type 1, `SpawnGroup` type 2, `SpawnMaintainGroup` type 3, `RespawnByGroup` type 4, `SpawnOne` type 5 (section 4). In a multiplayer game with the enemy multiplier above 1 the spawner's own `en_stMultiSpawningType` (1 or 3, set by the multiplier code) **replaces** the type when the procedure starts. `en_bGroupSpawner` changes only the number of spawnees handed to the formation and the launcher; **`en_bChangeToSimple` is never read** by any spawner method (section 4).
3. **`CommonEntryPoint` is the hub** (section 6): every step of every procedure ends by calling it again; it decides between "finished", "spawn source dead", the four entry procedures and the direct `SpawnOne` path.
4. **The first spawnee waits `en_fDelayInitial`, later ones `en_fDelaySingle` inside a group and `en_fDelayGroup` between groups** (section 7); in the evil container with a multiplier of 2 or more the single delay is recomputed after every spawn from a game-parameter graph.
5. **Spawn slot "wake protocol"** (section 10): a refused spawner **polls every 0.5 s**; the world info's wake call is an extra poke of the first spawner in a waiting container. **Confirmed by a second reading (T01-303):** the *occupied* containers are never filled, so the count limits (`ep_ctMax...SpawneesAtOnce`) never bind and only the enemy lag factor (multiplier above 1) can refuse a spawn — and that lag factor is fully closed-loop: raised on a successful evil spawn, lowered by the same amount when the spawnee dies (`CharacterDied`, section 10).

## Evidence

- **Code.** The spawner's state functions (`SetState`, `CommonEntryPoint`, `SimpleSpawnerEntryPoint`, `MaintainGroupSpawnerEntryPoint`, `RespawnByGroupSpawnerEntryPoint`, `SpawnEntityProcedure`, `WaitSpawneeDeathDelay`, `WaitAllSpawneesToDie`, `DelaySingleAndSpawn`, `StartSpawning`, `WaitThenSendAmmoItemEvents`, `SkipSpawning`, `WaitFreeSpawnSlot`, `OnSpawnSlotAvailable`), the script commands (`SpawnSimple`, `SpawnGroup`, `SpawnMaintainGroup`, `RespawnByGroup`, `SpawnOne`), the counting and event functions, `EntitySpawnedInternalNotification`, `OnSpawneeKilledInternalNotification`, `OnSpawnSourceDied`, `CancelSpawningInProgress`, `ResetSpawner`, `OnBoot`, `DeathSpawn`, `SetDelaysAndType` and the world info's `IsSpawningAllowed`, `INeedSpawnSlot`, `IDontNeedSpawnSlot`, `RegisterSpawnSlot`, `UnregisterSpawnSlot`, `GetWaitArray`, `GetOccupiedArray`, `CheckSpawnSlotValidity` and `CharacterDied` (the last two, T01-303) were decompiled and, wherever the decompiler lost an argument (state numbers, delays, string identifiers), read again in the instruction listing. Build: the retail `Sam2Game.dll` of the install used for all T01 pages (no version string was recorded by T01-109; **not re-checked here**). **T01-301** additionally decompiled and read, in `Engine.dll`: `CEntity::SetNextThink`/`ClearNextThink`, `CStateManager::SetThinkAt`/`SetNextThink`/`ClearNextThink` and the three unnamed helpers they share (the entity-handle lookup, the queue-entry removal, the 64-bit time add) — same build, not re-checked for version either.
- **Scans.** All 75 exported `CSpawnerEntity` methods were searched for reads of the members named below (`en_bChangeToSimple`: none). All callers of the world info's slot functions were searched, by direct reference and by virtual-call displacement over the whole DLL; T01-303 re-checked this by reading each call site's actual callee body (which `Get...Array` it asks for) rather than vtable-offset proximity alone, since the latter is easy to get wrong between sibling functions. T01-302 searched the whole DLL for any `SetState(14)` call site and for any write of `en_bSpawningPaused` to a nonzero value, and cross-checked `PauseSpawning`/`UnpauseSpawning`'s shared empty-function address against every other exported method that shares it.
- **Data.** Every stored spawner has `en_ssState` 0, `en_stSpawningType` 0, `en_bSpawningPaused` 0 and zero counters (`mechanism-classes-spawners.md`): the state machine is **not** observable in the data. The counts of commands per level are in `level-logic-api.md`.
- **Not checked.** Nothing was run: no timing, no event order was observed, and the numbers of the game-parameter graphs belong to T01-166 (answered, `mechanism-classes-spawners.md` section 6). `CheckSpawnSlotValidity`'s own caller was not found. The "next think" scheduler itself is now read (T01-301, `Engine.dll`: `CEntity::SetNextThink` → `CStateManager::SetNextThink` and its handle-keyed dedup, `_global_011.c`/`_global_001.c`), but whether its absolute-time queue is actually drained once per simulation step (making 0.00001 s behave as "next step") was not run.

## Layout / rules

### 1. Members used (the binary's own names)

`en_ctGroup`, `en_ctTotal` (copies of the record; **`en_ctTotal` is overwritten** by `en_ctInitialTotal * en_iMultiplier` at the first entry), `en_ctInitialGroup`, `en_ctInitialTotal` (copied from the record once, by `OnBoot`: `max(value, 1)`), `en_fDelayInitial`, `en_fDelaySingle`, `en_fDelayGroup`, `en_fSpawneeDeathDelay`, `en_fMinDelay` (default 1), `en_ctSpawned`, `en_ctGroupsToSpawn`, `en_ctKilledSpawnees`, `en_ssState`, `en_stSpawningType`, `en_stMultiSpawningType`, `en_bRBGSpawnerIsSpawningGroup` (the flag of the respawn-by-group procedure; "RBG flag" below), `en_bMultiplied`, `en_iMultiplier`, `en_bNoMulti`, `en_bDeathSpawner`, `en_bDeathSpawningEnded`, `en_bIgnoreMax`, `en_bIgnoreAmmoCheck`, `en_bSpawningPaused`, `en_tmTriggered`, `en_scltSpawneeCountLimitType` ("limit type": 2 good, 3 evil, 4 misc), `en_fPuppetLagFactor`.

### 2. `SpawnerState`: numbers and what each does

`SetState(n)` is a virtual function of the spawner. The table is the complete list of numbers the code can act on (**high**).

| n | State function run 0.00001 s after entering | What it does | Entered by |
|---|---|---|---|
| 0 | none (idle) | nothing is pending; the spawner waits for a script command or a poke. All data records are saved in this state | `OnBoot`, `ResetSpawner`, `CommonEntryPoint` ("done", "paused", "no properties", "group count invalid") |
| 1 | none | **waiting for the spawn source's reply** (`SpawnReady`, part 2 section 2) | `SpawnSourceAndSpawnEffect` |
| 2 | none, **terminal** | finished: `SetState` ignores every further call (section 3); only `ResetSpawner` leaves it | `OnSpawnSourceDied` only |
| 3 | `SpawnSourceAndSpawnEffect` | compute origin and velocity, make the spawnee or start the effect, then call `CommonEntryPoint` (part 2) | `SpawnEntityProcedure` |
| 4 | `WaitFreeSpawnSlot` | **never entered** by any code found (its function would only call `CommonEntryPoint`); `SetState` still releases the slot registration when it leaves state 4 | none |
| 5 | `SpawnEntityProcedure` | the slot check of section 9, then state 3 or state 15 | `StartSpawning` (the timer function of every entry procedure) |
| 6 | `SimpleSpawnerEntryPoint` | choose the delay (section 7), schedule `StartSpawning` | `CommonEntryPoint` for types 1 and 2 |
| 7 | `DelaySingleAndSpawn` | schedule `StartSpawning` after `en_fDelaySingle` | `MaintainGroupSpawnerEntryPoint` |
| 8 | `WaitSpawneeDeathDelay` | schedule `CommonEntryPoint` after `en_fSpawneeDeathDelay` | `OnSpawneeKilledInternalNotification` |
| 9 | none | **waiting for a spawnee to die** (maintain-group procedure) | `MaintainGroupSpawnerEntryPoint` |
| 10 | `MaintainGroupSpawnerEntryPoint` | choose the next step of the maintain-group procedure | `CommonEntryPoint` for type 3 |
| 11 | `WaitAllSpawneesToDie` | if all spawned are dead: set the RBG flag and call `CommonEntryPoint`; else wait | `RespawnByGroupSpawnerEntryPoint` |
| 12 | `RespawnByGroupSpawnerEntryPoint` | choose the next step of the respawn-by-group procedure | `CommonEntryPoint` for type 4 |
| 13 | - | not a state any code selects or tests | none |
| 14 | `WaitThenSendAmmoItemEvents` | schedule `SendAmmoItemEvents` after **15 s** | **none** — confirmed dead code (T01-302): no `SetState(14)` call site exists anywhere in the DLL, and neither this function nor `SendAmmoItemEvents` has any other caller |
| 15 | `SkipSpawning` | **retry**: after **0.5 s** call `CommonEntryPoint` | `SpawnEntityProcedure` (spawn refused) |

`SkipSpawning`, `WaitFreeSpawnSlot` and `OnSpawnSlotAvailable` are the same code: "call `CommonEntryPoint`". The names are those of the binary's own exports and of the string table inside `SetState`.

### 3. How a state runs, and `SetState`'s own rules

- **Stepping.** `SetState(n)` for n in {3,4,5,6,7,8,10,11,12,14} stores n, then calls the virtual "set next think" of the entity with a delay of **0.00001 s** and the identifier of the state function's name; n = 15 does the same with a delay of **0.5 s**. States 0, 1, 2, 9 and 13 store the number and schedule nothing. The entity has **one pending think**: scheduling replaces a pending one, and the engine also has a "clear next think". **T01-301 (`Engine.dll`, read in full): both the delay numbers and the replace-on-schedule behaviour are now confirmed at the engine level, not just inferred.** `CBaseEntity::SetNextThink(float, IDENT)` forwards through `CEntity::SetNextThink` to `CStateManager::SetNextThink`, which (1) builds an absolute due time by adding the given delay to the state manager's own cached current time (a 64-bit add, carry included — `0.00001` and `0.5` are literal IEEE-754 float immediates at the call sites, `0x3727c5ac` and `0x3f000000`, not derived from any variable tick rate) and (2) **before** inserting the new queue entry, walks the manager's linked list of pending entries and removes any existing one for the **same entity handle** (`CMetaHandle::operator==`, matched on the entity alone — the `IDENT` argument is stored in the entry for later dispatch, but plays no part in the lookup). So **a pending think never survives any later `SetState`/`SetNextThink` call on the same spawner, regardless of which state or ident it was for** — not an inference, read in the scheduler itself. Whether a 0.00001 s delay is indistinguishable from "the next simulation step" was not settled by reading the scheduler alone (it depends on how often the main loop drains due entries, `main-loop.md`): with the engine's variable 1 to 100 ms step, any due time this far in the past by the time the next step runs will fire there, so in practice it is — but this remains a **run-time check**, not re-ticketed. The functions that wait (`DelaySingleAndSpawn`, `WaitSpawneeDeathDelay`, entry procedures) schedule `StartSpawning` or `CommonEntryPoint` with their delay as the next think, **without changing the stored state number** (the number says which function last ran, not what is pending).
- **`StartSpawning`** is a timer target, not only a script command: its whole body is `SetState(5)`.
- **State 2 is terminal.** When the current state is 2, `SetState` changes nothing; if the limit type is 3 and the spawner is not a death spawner it first **releases the waiting registration** (`IDontNeedSpawnSlot`, section 10). **When the current state is 4** it releases the registration, then continues as normal.
- `SetState` never validates: a number outside the table is stored and does nothing.

### 4. Spawning types and who sets them

`en_stSpawningType` (0 at boot and after `ResetSpawner`; **high**):

| Type | Set by | Meaning |
|---|---|---|
| 0 | `ResetSpawner` / boot | none: `CommonEntryPoint` ends in state 0 |
| 1 | script `SpawnSimple` | make `en_ctTotal` spawnees one by one |
| 2 | script `SpawnGroup` | make **groups**: one group per command (`en_ctGroupsToSpawn` counts the commands) |
| 3 | script `SpawnMaintainGroup` | keep `en_ctGroup` spawnees alive until `en_ctTotal` have been made |
| 4 | script `RespawnByGroup` | make a group, wait until all are dead, make the next |
| 5 | script `SpawnOne` | make **one** spawnee at once and return it (no delays, no states) |

The script command bodies (read):

- `SpawnSimple`: only if the type is 0: type := 1; then, **unless `ShouldntSpawnAmmoItems` returns true**, `CommonEntryPoint`. A second `SpawnSimple` on a running spawner does nothing.
- `SpawnGroup`: if the type is already 2: `en_ctGroupsToSpawn` += 1 and `ReActivateInactiveSpawner` (which calls `CommonEntryPoint` only when the state is 0); if the type is 0: `en_ctGroupsToSpawn` += 1, type := 2 and, unless `ShouldntSpawnAmmoItems`, `CommonEntryPoint`. In any other type nothing happens.
- `SpawnMaintainGroup`: only if type 0: unless `ShouldntSpawnAmmoItems`: type := 3; **if `en_bNoMulti` is 0, `en_iMultiplier` is above 1 and `en_ctGroup` is 1, `en_ctGroup` := `en_ctInitialTotal`** (the whole initial count becomes the group that is kept alive); `CommonEntryPoint`. (The type is set only when the ammo test passes; the other commands set it first.)
- `RespawnByGroup`: only if type 0: type := 4, unless `ShouldntSpawnAmmoItems` then `CommonEntryPoint`.
- `SpawnOne`: sets `en_ctInitialTotal`, `en_ctInitialGroup`, `en_iMultiplier` := 1, `en_bMultiplied` := 1, `en_bNoMulti` := 1, type := 5, calls `CommonEntryPoint` and returns the spawner's last spawned entity (null on failure). No ammo test, no multiplier, no delay.
- **The multiplier override.** On its first `CommonEntryPoint` (section 6) a spawner that is **not** a death spawner and has `en_bNoMulti` 0 and `en_stMultiSpawningType` non-zero takes `en_stSpawningType` := `en_stMultiSpawningType`, whatever the script asked for. The multiplier code sets it to **1** (simple) or **3** (maintain group) (`SetDelaysAndType`: 1 normally, 3 when the count passes the graph's threshold; the multi-spawner, hovercraft and aircraft multipliers: 1 or 3; the exact conditions are T01-166).
- `en_bGroupSpawner` (data: 154 spawners, `ep_bGroupSpawner`) is copied from the record by `OnCreate` and read in two places only: type 5's spawnee count and the formation / launcher "count" argument when the multiplier is 1 (part 2 section 2). It does **not** select a procedure. **`en_bChangeToSimple` is read by no exported spawner method** (75 scanned; only the constructor and the copy constructor touch it): a design-time leftover (**medium**: other classes were not scanned).
- `ShouldntSpawnAmmoItems` (read): false when `en_bIgnoreAmmoCheck`; otherwise true when the spawnee entity is a `CSeriousBombItemEntity` and the game info's mode is 2 with the "no serious bombs" setting on; for a **death spawner** it also sets `en_bDeathSpawningEnded` and returns true on the second call. (The mode number and the setting's name were not recovered: **low**.)

### 5. `PauseSpawning`, `UnpauseSpawning`, `ResetSpawner`, `ReActivateInactiveSpawner`

- `PauseSpawning` and `UnpauseSpawning` are **empty functions** in the DLL (one instruction); `CommonEntryPoint` tests `en_bSpawningPaused`, but **no code that sets it to 1 was found**. **T01-302: confirmed, not merely a scan's absence.** The empty body is not a decompiler gap: it is one of a large identical-code-folding group the linker shares across dozens of unrelated classes' own genuinely-empty virtual overrides (`CBaseEntity::OnBoot`, `SBaseEntity::OnDelete`, and about forty more, found via the same shared address) — a class whose override is in that group has no code of its own at all, by construction, not a "maybe inlined" case. And `level-logic-api.md`'s own "action"/"action (async)" tables, read across the many other entity families they cover, are consistently **direct calls of the named virtual method** (`macSetHealth`, `AttackMelee`, ... one command name, one method, no generic "set member by name" layer in between) — so the level scripts' 13 + 3 `PauseSpawning` commands (`level-logic-api.md`) really do call this exact empty method and nothing else. **The feature is confirmed dead in the retail binary**, not merely unobserved: nothing anywhere sets `en_bSpawningPaused` to 1, and the one command that is supposed to has no body.
- `ResetSpawner` (chapter restore): `en_bSpawningPaused`, type, `en_ctSpawned`, `en_ctGroupsToSpawn`, `en_ctKilledSpawnees`, the RBG flag, `en_bMultiplied`, `en_bIgnoreAmmoCheck`, `en_bDeathSpawningEnded` and **`en_bUseRandomCone`** := 0, `en_tmTriggered` := -100 s; the spawn source (if any) is told about the spawner again. **It does not change the state number**: a spawner in state 2 stays terminal after a reset, and one in a waiting state keeps its pending think. Whether the chapter restore also calls `OnBoot` (which would set the state to 0 and re-apply the cone) is T01-172.
- `OnBoot`: if the initial-values flag is set (always, in the 58 and 59 member layouts): `en_ctInitialTotal` := `max(en_ctTotal, 1)`, `en_ctInitialGroup` := `max(en_ctGroup, 1)`, flag := 0; then `ResetSpawner`, the spawnee record's `SetPlacement` with the stored owner placement, the tweaks (`DoSpawnerTweaks`), and, when both the world info and the game info exist, `DoMultiplier`; finally `SetState(0)`.
- `CancelSpawningInProgress` (called when a spawnee could not be created): releases the slot (`UnregisterSpawnSlot`, if not a death spawner), subtracts `en_fPuppetLagFactor` from the world's lag factor, then runs `CommonEntryPoint`. A failed spawn is therefore **retried at the next step of the procedure**, not abandoned (**medium**).

### 6. `CommonEntryPoint`, step by step (read; medium)

It returns the spawner's last spawned entity (used by `SpawnOne`; every other path returns null). Steps in order:

1. If the state is **2**: return (nothing else).
2. **First entry** (`en_bMultiplied` 0): `en_iMultiplier` := 1; unless a death spawner: if `en_bNoMulti` is 0, `en_iMultiplier` := `GetMultiplier()`; `en_ctTotal` := `en_ctInitialTotal * en_iMultiplier`; `en_tmTriggered` := now; the world info's `INeedSpawnSlot(spawner)` (section 10); if `en_bNoMulti` is 0 and `en_stMultiSpawningType` is non-zero, the type := `en_stMultiSpawningType`. Then `en_bMultiplied` := 1. (A death spawner sets only the multiplier to 1 and the flag.)
3. The spawn origin `en_qvSpawnOrigin` := the stored owner placement `en_qvOwnerProperties` (the spawner's own placement, or a tweak's placement); `en_vSpawnVelocity` := zero.
4. If `en_ctInitialGroup` is not positive: log "Group count peroperty is invalid (<=0). Can't spawn." (sic), `SetState(0)`, return.
5. If `en_bSpawningPaused`: log "Spawning paused.", `SetState(0)`, return.
6. If there is no spawn properties record: log "No spawn properties. Can't spawn.", release the slot (`UnregisterSpawnSlot`) if the limit type is 3, `SetState(0)`, return.
7. If `en_ctSpawned` is not below `en_ctTotal`: `SetState(0)`, return.
8. **Spawn source dead.** If a spawn source exists and its liveness query (a virtual of the source entity; named only by its use here, **low**) returns false: for every missing spawnee (`en_ctSpawned` .. `en_ctTotal`-1) count a kill and raise `OneKilled` / `GroupKilled` by the rules of `mechanism-classes-spawners.md` section 4; then `AllKilled` if `en_ctKilledSpawnees` = `en_ctTotal`; `en_ctSpawned` := `en_ctTotal`, `en_ctGroupsToSpawn` := 0; release the slot if limit 3 and not a death spawner; `SetState(0)`, return. **No spawned-side events** are raised on this path (`OnSpawnSourceDied` is the same loop but ends in state **2** instead of 0).
9. **Choose the procedure** by `en_stSpawningType`: 1 → `SetState(6)`; 2 → `SetState(6)` if `en_ctGroupsToSpawn` is above 0, else `SetState(0)`; 3 → `SetState(10)`; 4 → `SetState(12)`; 5 → the direct path below; any other value → `SetState(0)`.
10. **Type 5 (`SpawnOne`) direct path.** n := `en_ctTotal` (or `en_ctGroup` if `en_bGroupSpawner`). If a spawn source exists and has a spawn effect configuration: the origin := the source's attachment placement whose name is the configuration's `sfxc_idNoAnimAttachment` (the identifier `Barrel01` when it is unset), when the source has one. If a formation exists, the origin := the formation's placement for the closest player, with index `en_ctSpawned` and count n (part 2 section 3); if a launcher exists, the velocity := the launcher's velocity (same arguments, pitch fixed at 0.7853982 rad = 45 degrees, part 2 section 4). Then `DoSpawnEffect` (part 2 section 6), `SetState(0)`, return the last spawned entity.

### 7. The four entry procedures and the delay arithmetic (read; medium)

Let *spawned* = `en_ctSpawned`, *killed* = `en_ctKilledSpawnees`, *G* = `en_ctInitialGroup` (note: the initial value, not `en_ctGroup`, except where stated). All delays are seconds of simulation time; "then" means after the 0.00001 s hop that ran the state function. Every procedure ends the delay by running `StartSpawning` (state 5).

**Simple (state 6)**, used for types 1 and 2:

| Condition when the function runs | Delay before the next spawn attempt |
|---|---|
| spawned = 0 | `en_fDelayInitial` |
| spawned is a multiple of G | `en_fDelayGroup` |
| otherwise | `en_fDelaySingle` |

So **the initial delay is paid only before the very first spawnee** of the spawner's life (also for type 2: later groups pay the group delay), a group delay between groups, a single delay between the members of a group. The spawner makes `en_ctTotal` spawnees in all (type 1) or stops after each group (type 2, by the group counter of section 8).

**Maintain group (state 10)**, type 3, with *C* = `en_ctGroup`:

| Condition | Action |
|---|---|
| spawned = 0 | wait `en_fDelayInitial`, then spawn |
| spawned < C | wait `en_fDelaySingle`, then spawn (the first group is built member by member) |
| spawned - killed < C | state 7: wait `en_fDelaySingle`, then spawn (refill) |
| otherwise | **state 9**: wait for a death |

A death in state 9 sets state 8 (section 8): after `en_fSpawneeDeathDelay` `CommonEntryPoint` runs again and the refill rule applies. Note that a refill pays `en_fDelaySingle` and the death delay **in series** (death delay first). The procedure ends when spawned reaches `en_ctTotal` (step 7 of section 6).

**Respawn by group (state 12)**, type 4, with the RBG flag:

| Condition | Action |
|---|---|
| spawned = 0 | RBG := 1, wait `en_fDelayInitial`, then spawn |
| RBG = 0 | **state 11**: `WaitAllSpawneesToDie` |
| spawned is a multiple of G | wait `en_fDelayGroup`, then spawn (first member of a new group) |
| otherwise | wait `en_fDelaySingle`, then spawn |

The RBG flag is cleared by the counting code when the last member of a group has been spawned (section 8) and set again by `WaitAllSpawneesToDie` (when killed >= spawned) or by the death handler of state 11. Between two groups the spawner therefore waits for the last death, then `en_fSpawneeDeathDelay` (state 8), then `en_fDelayGroup`.

**`WaitAllSpawneesToDie`** (state 11, the function): if killed >= spawned: RBG := 1 and `CommonEntryPoint` at once (no death delay), otherwise nothing; the death handler (below) takes over.

**Death handler** (`OnSpawneeKilledInternalNotification`, runs after the kill counting of `mechanism-classes-spawners.md` section 4): if the state is 9: `SetState(8)`. If the state is 11 and killed >= spawned: RBG := 1 and `SetState(8)`. In any other state nothing.

**`en_fDelaySingle` is not a constant for a multiplied evil spawner** (`EntitySpawnedInternalNotification`, after every spawn): if the limit type is 3 and `en_iMultiplier` is at least 2, then `en_fDelaySingle` := `max(0, G1(t) * en_fMinDelay)` where *t* is the time in seconds since `en_tmTriggered` and G1 is a graph of the global game parameters (`CGlobalSpawningParams` in the binary; its values were not extracted: **T01-166**). `SetDelaysAndType` (called by the multiplier code) sets `en_fDelaySingle`/`en_fDelayGroup`/`en_fMinDelay` from other graphs indexed by the spawnee count: `en_fMinDelay` := the smaller (larger when G is 1) of the graph value and the old single delay, `en_fDelayGroup` := a second graph value (when the count is above a threshold: a third graph's value, a second group count and the type 3), `en_ctGroup` := the resulting group size, `en_fSpawneeDeathDelay` := 0. The numbers are T01-166.

### 8. After a spawn: counters, group flags and the kill side

`DoSpawnEffect` (part 2) ends with `CountOneSpawned` (events of `mechanism-classes-spawners.md` section 4; nothing at all for a death spawner), whose last act is `EntitySpawnedInternalNotification`:

1. **Type 2**: if spawned is a multiple of G, `en_ctGroupsToSpawn` -= 1 (so `CommonEntryPoint` ends in state 0 after the current group).
2. **Type 4**: if the RBG flag is set and spawned is a multiple of G, the flag := 0.
3. Delay recomputation of section 7 (limit type 3 and multiplier of at least 2).

Kill side: `OnSpawneeKilled` (called by the dying puppet or destroyed prop through its spawner handle; part of the puppet death, `puppet-classes.md`) counts and raises events, then `OnSpawneeKilledInternalNotification` (above). `OnSpawneePicked` (an item spawnee picked up) counts as a kill, releases the slot (`UnregisterSpawnSlot`, not for a death spawner), but **does not call the notification**: a maintain-group spawner of items does not refill on a pick (**medium**).

`SendAmmoItemEvents` (the end of state 14, **not reachable by any found path**): `en_ctGroup` := `max(en_ctGroup, 1)`; for i = 0 .. `en_ctTotal` inclusive (**total + 1 iterations**) raise `OnePicked` and, when `en_ctTotal` is a multiple of `en_ctGroup`, `GroupPicked`; then `AllPicked`.

### 9. `SpawnEntityProcedure` (state 5)

1. If the limit type is not 3: `SetState(3)`.
2. Otherwise, unless `en_bIgnoreMax`: ask the world info `IsSpawningAllowed(spawner)`; **not allowed**: `SetState(15)` (retry after 0.5 s, section 2); allowed: `SetState(3)`.

`IsSpawningAllowed` (`mechanism-classes-world-info.md` section 3, re-read here): returns true if there is no current chapter record or the limit type is 1; with the session enemy multiplier above 1 it **(for every limit type except 1, not only the evil one) adds the spawner's `en_fPuppetLagFactor` to the world's lag factor and refuses when the sum is above the chapter's limit (the chapter's maximum from `ChapterMaxes.txt`, else the evil limit, else 30), otherwise stores the sum and allows**; with a multiplier of 1 and limit 3 it **adds the lag factor unconditionally** and compares the **number of entries of the evil occupied container** with the chapter's evil limit; limits 2 and 4 compare the good / misc occupied container's count with the chapter's limit. The lag factor is subtracted again by `CancelSpawningInProgress` and (a call at the start of puppet death, not read) when an enemy dies.

### 10. Wait and wake protocol with the world info (read; medium)

Containers (members of the world info): per limit type one **waiting** container (`en_eaWaitingForSpawnSlotEvil`, `...Good`, `...Misc`: spawners) and one **occupied** container (`en_eaSpawnSlotsEvil`, `...Good`, `...Misc`).

- **Registration.** At its first `CommonEntryPoint` every non-death spawner calls `INeedSpawnSlot`: it is **appended to the waiting container of its type** (types 2, 3, 4; nothing for type 1; "INeedSpawnSlot() called for an entity that is already registered for waiting on a free spawn slot." if it is already in). This is **not** conditional on being refused: the waiting container is the list of **all active spawners of that type**, in order of arrival.
- **De-registration** (`IDontNeedSpawnSlot`: removes it if it is a member): only for **limit type 3 and a non-death spawner**, at: all spawnees made (`CountOneSpawned`), spawn source dead (both paths), leaving state 2 attempts, leaving state 4. Spawners of type 2 and 4 are **never removed** from their waiting container by these paths (they stay until the world is unloaded).
- **Wake.** `UnregisterSpawnSlot(spawner)` removes the spawner from the **occupied** container of its type and then calls `OnSpawnSlotAvailable` (= `CommonEntryPoint`) on the **first member of the waiting container** of that type (log "Spawner that was registered for spawn slot waiting is now NULL." if it is a null handle). Callers: `OnSpawneePicked` (an item picked up), `CancelSpawningInProgress`, and the "no spawn properties" path of `CommonEntryPoint`. The woken spawner is **not necessarily one that was refused**: for the misc type it is the same first registered spawner every time. A poke restarts that spawner's current wait (it re-runs `CommonEntryPoint`, which schedules the procedure's delay anew); on a spawner in state 0 with type 0 it does nothing.
- **Polling.** A refused spawner (state 15) calls `CommonEntryPoint` itself every **0.5 s**, so a lost wake costs at most 0.5 s.
- **Occupied container (T01-303: confirmed by a second, independent reading).** `RegisterSpawnSlot` (adds the spawner itself — not its spawnees — to `GetOccupiedArray`) is a distinct function from `INeedSpawnSlot` (adds to `GetWaitArray`): the two bodies are easy to conflate from their vtable proximity alone, which is why this is called out explicitly. Re-reading every call site that touches the world info's spawn-slot slots (`CommonEntryPoint`'s first-entry call, `CountOneSpawned`'s all-spawned call, the "no spawn properties" error path) against each callee's own body (which container it asks `Get...Array` for) finds the same result as before: every one of them is `INeedSpawnSlot`, `IDontNeedSpawnSlot` or `UnregisterSpawnSlot`, each operating on a **waiting** container or removing from one; `RegisterSpawnSlot` itself is still not reached from anywhere. So the **occupied containers genuinely stay empty**, the count tests of `IsSpawningAllowed` compare 0 with the limits and **never refuse**, and only the lag-factor test (multiplier above 1) can — confirmed, not merely a scan's negative result.
- **`CheckSpawnSlotValidity`** (T01-303: read in full): sweeps all **six** containers (occupied and waiting, each of evil/good/misc) with `GetCountOfNullEntries`/`ClearNullEntries`, logging one of six distinct "Deleted (NULL) entities detected ... in '<evil|good|misc>' container" / "... waiting for the free spawn slot in ..." messages per container that had any. Since the occupied containers are never filled, this half of it never logs in practice; the waiting containers (which genuinely do fill, per the bullet above) are the ones this actually prunes, when a waiting spawner is deleted from the world without ever being woken. Its own caller was not found in this reading (a virtual call with no decompiled body nearby pointed to its displacement).
- **The lag factor's other half: who decreases it (T01-303, answers `mechanism-classes-world-info.md`'s open "what resets `en_fEnemyLagFactor`").** `CWorldInfoEntity::CharacterDied(puppet)` (read in full): removes the dying puppet from `en_eaPuppetsInExisting`; then, only for a puppet whose own "counted against the evil limit" flag is set, resolves the puppet's own spawner handle (the entity's `en_henSpawnerWhoSpawnedMe`-like link) and, if it still exists, **subtracts that spawner's own `en_fPuppetLagFactor` back out of `en_fEnemyLagFactor`**, floored at 0 (never negative). So the lag factor genuinely is a live **load estimate**: raised by `IsSpawningAllowed` on every successful evil spawn above multiplier 1, lowered by the matching amount when that spawnee dies — closing the loop T01-173 left open. The puppet also leaves a further container at a different offset from the three spawn-slot pairs (not matched to a named member in this reading) under the same flag.

### 11. Death spawner and spawn source

- **Death spawner.** `DeathSpawn(placement)` is called by a dying puppet, destroyed prop or static model that has a death spawner. It runs only on the host and only while `en_bDeathSpawningEnded` is 0 and `ShouldntSpawnAmmoItems` is false: `en_qvSpawnOrigin` := the given placement, `en_vSpawnVelocity` := the launcher's velocity for it (zero without a launcher; launcher arguments: the spawner, the placement, pitch 45 degrees, `ep_fRndBase`, index 0, count 0), then `DoSpawnEffect`. A death spawner does **no counting, raises no events and has no states** (`CountOneSpawned`, `CommonEntryPoint` steps 2 and the slot calls are skipped by `en_bDeathSpawner`); `MarkAsDeathSpawner` sets the flag.
- **Spawn source** (`SetSpawnSource`): stores the handle and tells the source about the spawner. When a spawn source dies, its puppet calls the exploiting spawner's `OnSpawnSourceDied` (section 6 step 8, ending in state 2). `SetSpawneeExploitingSpawner` stores a spawner handle in `en_henSpawnerExploiter`; after each spawn `SpawnOneSpawnee` writes the new spawnee into that spawner's spawn source (a spawner that spawns from the thing another spawner spawned, part 2 section 7).

## Invariants

1. The numbers 0-3, 5-12, 14, 15 are the only ones `SetState` acts on (table of section 2); 4, 13 **and 14** (T01-302: confirmed by the same exhaustive accounting) are never selected by any call site found (call sites: `CommonEntryPoint` 0, 6, 10, 12; `SpawnEntityProcedure` 3, 15; `SpawnSourceAndSpawnEffect` 1; `StartSpawning` 5; the entry procedures 7, 9, 11; the death handler 8; `OnSpawnSourceDied` 2; `OnBoot` 0).
2. Every stored spawner is in state 0 with type 0 (3,783 of 3,783).
3. A spawner never leaves state 2 (except by `ResetSpawner`, which does not change the state: it stays 2 until `OnBoot`).
4. Delays are always read from the `en_` copies; the record's design numbers reach them only through `OnBoot` (counts) and the copy constructor.

## Open questions

1. **Tick granularity and think semantics** of the engine's "set next think": **answered by T01-301** (`Engine.dll`'s `CStateManager::SetNextThink`, read in full) for the half that does not need a run — a pending think is always replaced by the next `SetState`/`SetNextThink` call on the same entity (the scheduler dedups by entity handle alone, not by ident), and the delays are literal, fixed float immediates (not a configurable rate). **Still open, needs a run:** whether the main loop's step cadence actually makes 0.00001 s indistinguishable from "next step" in practice.
2. **Who sets `en_bSpawningPaused`:** **answered by T01-302** (section 5): nobody, confirmed rather than merely unobserved — `PauseSpawning`/`UnpauseSpawning` are members of a DLL-wide identical-code-folding group of genuinely empty overrides (so the emptiness is not a decompiler gap), and `level-logic-api.md`'s "action" commands are, consistently across every other entity family it covers, direct calls of the named method with no generic by-name setter in between. The feature is dead in the retail binary.
3. **State 14 and `SendAmmoItemEvents`:** **answered by T01-302** (section 2, invariant 1): confirmed dead code by the same exhaustive call-site accounting used for states 4 and 13 — no `SetState(14)` call site exists, and neither `WaitThenSendAmmoItemEvents` nor `SendAmmoItemEvents` has any caller at all.
4. **The occupied containers and the full lag-factor bookkeeping:** **answered by T01-303** (section 10): the occupied containers are confirmed never filled (a second, independent reading agrees with the first); the lag factor's missing other half is `CharacterDied`, which subtracts a dying evil-tracked puppet's own spawner's lag factor back out, floored at 0. Still open: `CheckSpawnSlotValidity`'s own caller, and the exact name/meaning of the puppet's "counted against the evil limit" flag and the further container it is removed from on death.
5. **Run-time confirmation** of the delay chains of section 7 (event times for the test vectors below, with a multiplier above 1). T01-174.
6. The graphs behind `SetDelaysAndType` and the single-delay recomputation: **answered by T01-166**, [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) section 6.

## Test vectors

All from the code reading (**no run**; times are simulation seconds from the command, plus a few 0.00001 s hops per spawn that are omitted):

1. **Simple, no multiplier.** `ep_ctTotal` 4, `ep_ctGroup` 2, `ep_fDelayInitial` 1, `ep_fDelaySingle` 0.5, `ep_fDelayGroup` 2, command `SpawnSimple` at t = 0: spawnees at **1.0, 1.5, 3.5, 4.0 s** (initial; single inside the group; group delay before the third; single). `OneSpawned` at each, `GroupSpawned` at 1.5 and 4.0, `AllSpawned` at 4.0.
2. **Maintain group.** total 6, group 2, delays initial 0, single 1, death 3: spawnees at 0, 1 (the first group); then the spawner enters state 9; when the first one dies at t = 10, a refill is made at **10 + 3 + 1 = 14 s** (death delay, then single delay).
3. **Respawn by group.** total 4, group 2, delays initial 0, single 0.5, group 2, death 1: spawnees at 0, 0.5; the spawner then waits; if both die at t = 10 (the second death at 10), the third is made at **10 + 1 + 2 = 13 s**, the fourth at 13.5 s.
4. **Group commands.** `SpawnGroup` issued three times in a row on a spawner with group 2, total 6: the first call starts the procedure (groups to spawn 1), the second and third add to the counter; two members are made per group, the counter returns to 0 after the last group.
5. **Retry.** A spawner with limit type 3 refused by `IsSpawningAllowed` tries again at 0.5 s intervals until allowed.
6. **Lag factor round trip (T01-303, code reading).** An evil spawner with `en_fPuppetLagFactor` 5 raises `en_fEnemyLagFactor` by 5 on a successful spawn under multiplier 2; when that one spawnee dies, `CharacterDied` lowers it back by the same 5 (floored at 0, so two deaths of spawnees from the same spawner never drive it negative even if the first subtraction already reached 0 through some other spawner's activity).
7. **Replace, not stack (T01-301, code reading).** A spawner in `WaitSpawneeDeathDelay` (a pending think for `CommonEntryPoint` already queued) that is then driven through `SetState(3)` (spawn) has its *first* pending think removed by `CStateManager::SetNextThink`'s own handle lookup before the second is queued — at no point are both pending at once, regardless of the two thinks' idents.
8. **Dead features (T01-302, code reading).** A level script's `PauseSpawning` command on any spawner calls a function with zero instructions of its own: the spawner keeps spawning exactly as if the command had never run. No sequence of script commands or engine events can make a spawner reach state 14; `SendAmmoItemEvents` can only run if something outside the 75 exported `CSpawnerEntity` methods calls it directly, which this reading did not find.
