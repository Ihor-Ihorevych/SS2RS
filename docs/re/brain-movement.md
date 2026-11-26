# Mover choice, patrol and tactics in the brain (T01-127)

- **Roadmap task:** T01-127 (follow-up of [`brain-classes.md`](brain-classes.md) open question 3; the movers and tactics themselves are [`behavior-mover-classes.md`](behavior-mover-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for what `tools/re/brain_mover_verify.py` reproduces from the data (the band lists of the 82 `.cb` files: counts, order, last distance, the velocity adjuster values; the tactic and patrol data of the 59 worlds); **medium** for every rule read from the binary (the selection in `MoveCharacter`, the band lookup, the patrol functions, the macro events), because each rests on one reading of a function that was decompiled once and cross-checked with the virtual tables and the callers; **low** for what is marked "(name)" or "(not read)". **Nothing was run.**

Prerequisites: [`brain-classes.md`](brain-classes.md) (brain members, `BrainState`, `MoverReason`), [`behavior-mover-classes.md`](behavior-mover-classes.md) (mover classes, `CharacterMoverResult`, tactics), `ai.md`, [`brain-assignment.md`](brain-assignment.md) (who has a brain), [`puppet-entity-classes.md`](puppet-entity-classes.md).

## Summary

`CBrainEntity::MoveCharacter` is called once per move period by the think loop. It does **four things in this order**: apply a pending `.cb` state change, look up the **distance band** the foe is in, **choose or keep the current mover**, and **run the mover** (or stop the puppet). This page gives the rules exactly. Numbers that matter to an implementer:

1. **Band lookup** (section 2): the bands of the current `.cb` state and medium (land, air or water of the puppet) are scanned **in file order** and the **first band whose distance is greater than the horizontal distance to the foe** is the current band. All **132 non-empty band lists** of the 82 files are **strictly ascending** (high), so this is "the narrowest band that still contains the foe". With no foe (distance stored as -1), or a foe beyond the last band, there is **no band** and the mover is the default "towards the foe".
2. **Mover choice** (section 3): a pending **tactic** wins over everything (even a forced mover); otherwise the mover of the band; otherwise `CMCTowardsFoe`. The current mover is kept while its reason is a tactic, forced, navigation or navigation-failed, and while its reason is "area" **and the band id has not changed** (the comparison is by band **identifier**, not by band object).
3. **Mover results** (section 4): `CMR_REPLACE_ME` deletes the mover and tries again after **0.01 s**; `CMR_NEED_NAVIGATION` installs the navigation mover (only when the world has a navigation mesh and the puppet does not move in 3-D) and the next call is after **0.01 s**; `CMR_NAVIGATION_FAILED` installs a "don't move" mover with reason `MR_NAVIGATION_FAILED` and the next call is after a **random time in [1, 3) s**. The macro event `TacticEnded` is sent to the puppet when a **tactic** mover asks to be replaced.
4. **Patrol** (section 5): a patrol is the brain state `BS_PATROLLING_MARKERS`; every marker is a "go to goal point" behaviour; the macro events `MarkerReached` (every marker) and `LastPatrolMarkerReached` (no further marker) are sent to the puppet when the behaviour finishes. A foe stops the patrol.
5. **Defaults of the data** (section 6): 1,094 of the 2,328 placed puppets have a default tactic; 792 of them have a tactic **without** a marker and no auto-patrol flag, so the marker (if the mover needs one) is found by the puppet, not stored (section 5.4).

Two corrections to earlier pages (section 8): the brain's movement record holds the foe's **last seen position and a "seen on the last test" flag**, not a foe distance; and `CBAStopUsingTactic` and `CBAReuseTactic` act on the **desired** tactic member.

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Functions decompiled and read in full: `CBrainEntity::MoveCharacter`, `SetCurrentMover`, `DeleteCurrentMover`, `ForceMovingType`, `UnforceMovingType`, `CacheAreaData`, `StartPatrolingMarkers`, `StartMacroPatrolingMarkers`, `ContinuePatrolingMarkers`, `StopPatroling`, `SetPatrolingSpeedMultiplier`, `SetPatrolingRotateSpeedMultiplier`, `SendPuppetTowardsCurrentMarker`, `OnBeginPatrollingMarkers`, `OnEndPatrollingMarkers`, `OrderPuppetToGoToGoalPoint`, `RouteOnGoalPointReached`, `OnGoalPointReached`, `OnBehaviorFinished`, `OnAreaChanged`, `OnWallHit`, `SetState`, `OnFoeAssigned`; the character behavior's band lookup (an unexported method of `CCharacterBehavior`, called by `CacheAreaData`); `CPuppetEntity::PatrolMarkers`, `StopPatrolingMarkers`, `ContinuePatrolingMarkers`, `StartAutoPatroling`, `ObtainFirstPathMarker`, `GetClosestMarker` (outline), `DoStartPatroling`, `DoStopPatroling`, `PrepareForAutoPatroling`, `OnGoalPointReached`, `SetPatrolingSpeedMultiplier`, `StopMoving`, `SetCurrentPath`, `SetTacticEntity`, `ResetDynamicMembers` (the tactic and marker members), the end of `OnBoot`, the tactic copy in `OnUpdate`; `CBAStopUsingTactic`, `CBAReuseTactic`, `CBAStopTactic`; `CPathMarkerEntity::GetNextMarker`. Member identities were assigned by comparing the byte offsets that the code uses with the DLL's member tables (names and order, offsets not recorded here, `conventions.md`): the member names below are the declared ones. A few virtual calls were identified by position in the virtual tables of 13 classes (the names of identical shared stubs are not used as evidence).
- **Callers.** Every function of the DLL was scanned for direct virtual calls through the table positions of the brain's patrol methods and of `ObtainFirstPathMarker`; the results are in section 5.
- **Data.** `python -I tools/re/brain_mover_verify.py "<install>"` (the `.cb` part about 1 minute, the world part about 20 minutes; `--cb-only` skips it) prints sections 1 to 3 quoted below.
- **Not checked.** Nothing was run on the original (no instrumentation): every timing, the order of calls and every number here is read from code or data, not measured (T01-130, T01-038). The mover classes' own `MoveCharacter` bodies (geometry) are T01-154; the path-finding service T01-155; the think period T01-126.

## Layout / rules

### 1. State used (members; **medium**)

Brain members (`SBrainEntity`, `brain-classes.md` section 5): `en_hmcCharacterMover` (the current mover, a **copy**), `en_mrMoverReason`, `en_tmMoverStarted`, `en_idAreaForLastMover` (the band id when the reason is `MR_AREA`, else invalid), `en_mcd` (the movement record, 1.1), `en_idState`/`en_idDesiredState` (`.cb` states), `en_bsState`, `en_ulExecutionAbilities` (bit 0 value 1: AI moments may be handled; bit 2 value 4: the puppet may move and the generic states keep re-arming; 4.1), `en_asd` (the action-selection record that `CacheAreaData` fills: it holds the current state id, the current band id and band, the distance to the foe, the medium and the two ratios of 2.2).

Puppet members read or written by the brain: `en_henCurrentTacticEntity`, `en_henDesiredTacticEntity`, `en_henDefaultTacticEntity`, `en_henCurrentPathMarker`, `en_henLastReachedMarker`, `en_bRotateOnLastPatrolingMarker`, `en_ulAutoPatrolMarkerEntityID` and `en_bAutoPatrolMarkers`, `en_fMoverMultiplier`, `en_fSpeedMultiplier`, `en_fTurningVelocityMultiplier`, `en_fAccelMultiplier`, `en_fDecelMultiplier`, `en_fThrustAddingAngleOverride`, `en_fPatrolSpeedMultiplier`, `en_fPatrolRotateSpeedMultiplier`, `en_fPatrolThrustAddingAngle`, `en_maMoveAbilities`, `en_maMoveAbilitiesTemp`, `en_maPatrolingMoveAbilities`, `en_ava` (the five velocity-adjuster floats), `en_bTouchedFloorOnce`.

**1.1 The movement record `en_mcd`** (filled at the start of every `MoveCharacter`, **medium**; this corrects "foe distance" in `brain-classes.md` section 3.4): the brain (handle), the brain's own position (3 floats), the foe **puppet** (handle; invalid when there is no foe), the foe's **last seen position** (3 floats, taken from the foe manager's `en_vFoeLastSeenPos`, zero when there is no foe), the foe manager's **`en_bFoeSeenOnLastTest`** flag, and the move period just computed by `GetMovePeriod` (seconds). The movers read the foe position from here, so **a mover steers towards where the foe was last seen when the foe was not seen on the last line-of-sight test, and towards the foe's live position when it was** (corrected by T01-154, `behavior-mover-steering.md` 1.1: `GetDestinationPoint` uses `en_bFoeSeenOnLastTest`) (the foe manager updates the position when it sees the foe; `behavior-entity-classes.md` 7).

### 2. The band lookup (`CacheAreaData` and the behavior's lookup; **medium**, data **high**)

**2.1 Inputs.** The behavior is the puppet's `.cb` (`en_pcbCharacterBehavior`). `CacheAreaData` first fills the record: the brain, the current `.cb` state id, the puppet's **health ratio** (`GetHealth / GetMaxHealth` as floats; `brain-classes.md` 3.2), a **random number in [0, 1)** (the engine's generator, scaled by 2^-32; the same one `StartCharacterAction` uses; T01-126), the **medium** (`GetDipContent`: 0 `DC_LAND`, 1 `DC_AIR`, 2 `DC_WATER`; legged puppets answer from their pose, aircraft answer air, every other class answers land; `puppet-vehicle-classes.md`), and the **horizontal distance to the foe**: the Euclidean distance in the **X and Z plane** (the vertical axis is ignored) between the brain's position (the puppet's) and the **live** position of the foe's brain (not the last seen one); **-1** when the brain has no foe manager or the manager has no foe.

**2.2 The lookup** (a method of the character behavior, called with the record):

1. Find the state whose id equals the current `.cb` state id in the behavior's state list (first match); none: no state, no band.
2. If the distance is **negative** (no foe): no band; the record's area id is the invalid id, the band is null, and the five ratio members are set to -1.
3. Take the state's band list for the medium: `cs_dcLand` for any value except 1 (air: `cs_dcAir`) and 2 (water: `cs_dcWater`).
4. Walk the list in order; the **first band whose `cba_fDistance` is greater than the distance** is the current band (a distance exactly equal to a band's distance belongs to the **next** band). If no band qualifies (the list is empty, or the distance is beyond the last one) there is **no band**: the area id stays invalid and the band null.
5. With a band: the record gets its id (`cba_idArea`) and the band. The **band start** is the previous band's distance (0.0 for the first band) and the **band end** is the band's own distance. The record also gets: the distance divided by one of the behavior's ranges (the first of its three range members in declared order, `cb_fActiveRange`, **medium-low**: identified by member order), and an **area ratio** `1 - (d - start) / (end - start)`: **1.0 at the inner edge, 0.0 at the outer edge** (for d not above the start the ratio is 1.0); the next band's distance (or the behavior's range member when the band is last) is stored as well. These ratios feed the action selection (`cba_pgrActionFrequencyOverAreaRatio`, T01-126).

**2.3 After the lookup.** When a band was found, `CacheAreaData` copies the band's **five velocity-adjuster floats** (`ava_fAccelerationMultiplier`, `ava_fDeccelerationMultiplier`, `ava_fSpeedMultiplier`, `ava_fMovingRotationMultiplier`, `ava_fStandingRotationMultiplier`) into the puppet's `en_ava`; with no band, `en_ava` is set to **1, 1, 1, 1, 1**. Data: of the 381 bands all but `ava_fSpeedMultiplier` are 1.0 (**381 of 381** for the other four members); the speed multiplier is 1.0 in 294 bands and 0.5 (13), 0.7 (1), 0.75 (1), 1.1 (4), 1.125 (1), 1.2 (26), 1.25 (21), 1.4 (1), 1.5 (15), 2.0 (4) (script, section 2). How the puppet uses `en_ava` is the movement spec (T01-141).

**2.4 Data** (script, section 1): 82 files, 134 states, 402 band lists (134 x 3 media), **270 empty** and 132 non-empty with 1 band (36), 2 (19), 3 (23), 4 (32) or 5 (22): 381 bands. **132 of 132** non-empty lists are strictly ascending by distance. The first distance of a list is 2 to 500 (2.0 in 20 lists, 3.0 in 13, 8.0 in 10); the last distance is 100 to 10,000 in 129 lists and **100,000,000 in 3** (so for 129 of 132 lists a foe farther than the last distance has **no band**). 57 of the 381 bands have no mover (`ai.md`).

**2.5 The area id is compared, not the band** (see 3.1): two bands of different states or media with the same identifier (`Close`, `Far` ... 31 distinct identifiers in 381 bands) are the **same area** for the purpose of keeping the mover.

### 3. Choosing the mover (`MoveCharacter`, **medium**)

**3.0 Start.** Return **1.0** (no further work) when the brain has no foe manager, or the puppet has no `.cb` (the same test as `HasFoeManager`; a brain without a foe manager never moves on its own, `brain-assignment.md` 4). First, if a `.cb` state change is **desired** (`en_idDesiredState` valid and different from the current state) **and** ability bit 0 is set, call `SetNewState` (the same step `ThinkPoint` takes). Then compute the move period with `GetMovePeriod` (`brain-classes.md` 3.2; this is the number returned at the end unless a step below replaces it), fill `en_mcd` (1.1) and call `CacheAreaData` (2). Remember the band before the call (to detect a change at the end).

**3.1 Keep or choose.** The band's mover is the band's `cba_pmcMoveCharacter` (null when the band is missing or has none). The current mover is **kept** when it exists and:

- its reason is `MR_AREA` (1) **and** `en_idAreaForLastMover` equals the new area id (the band id, 2.5), **or**
- its reason is anything else (`MR_FORCED` 2, `MR_TACTIC` 3, `MR_NAVIGATION` 4, `MR_NAVIGATION_FAILED` 5),

**and in both cases no tactic is pending** (`en_henDesiredTacticEntity` empty). A pending tactic therefore replaces **any** mover, including a forced or navigation mover. The mover is **re-chosen** when there is none (`MR_NONE` 0), when the reason is `MR_AREA` and the area id changed, or when a tactic is pending.

**3.2 Re-choosing.** If **no** tactic is pending:

- band mover present: `SetCurrentMover(band mover, MR_AREA)`;
- otherwise (no band, or a band without mover, 57 of 381 in the data): build a new **`CMCTowardsFoe`** and `SetCurrentMover(it, MR_AREA)`. (The mover that was built is a temporary object and is deleted afterwards; the brain holds a copy.)

If a tactic **is** pending (3.3).

**3.3 Taking the pending tactic.** In order:

1. `en_henCurrentTacticEntity` := `en_henDesiredTacticEntity`; the desired member becomes empty.
2. If the tactic has a **path marker** (`en_henTacticMarker`): `SetCurrentPath(marker)` makes it the puppet's current path marker (`en_henCurrentPathMarker`).
3. If the puppet's current path marker is **still empty** (the tactic had no marker and none was set earlier): it becomes `ObtainFirstPathMarker()` (5.4).
4. Take the tactic's mover (`en_hmcCharacterMover`). When it is **empty** the console error `Tactic entity has invalid (NULL) mover!` is printed and **nothing else happens** (the previous mover stays and the pending tactic is consumed). Otherwise copy three tactic members **into the tactic's own mover object**: `en_fAutoOffTime` -> `mc_fAutoOffTime`, `en_fAutoOffDistance` -> `mc_fAutoOffDistance`, `en_bStopTacticWhenHasFoe` -> `mc_bStopTacticWhenHasFoe` (so the **tactic's values override the mover's own**, which are -1, -1 and 0 in every stored mover), and `SetCurrentMover(mover, MR_TACTIC)`.

**3.4 `SetCurrentMover(mover, reason)`** (**medium**): first `DeleteCurrentMover`; with a non-null mover: make a **copy** of it (the engine's generic object copy, external handles copied), store the copy as `en_hmcCharacterMover`, write the brain into the copy's brain link, store the reason, `en_tmMoverStarted` := now, and set the puppet's **`en_fMoverMultiplier`** to the copy's `mc_fSpeedMultiplier`. Then `en_idAreaForLastMover` := the current band id when the reason is `MR_AREA`, else the invalid id. The copy is fresh: `mc_bFirstTimeExecuted` is true again, so a mover that is re-chosen (area change, tactic) starts over (start time and position, random value), while a kept mover keeps its state. **`DeleteCurrentMover`**: puppet `en_fMoverMultiplier` := **1.0**; when a mover exists it is deleted, the member emptied and the reason set to `MR_NONE`.

**3.5 `ForceMovingType(mover, time)`** (called by `CBAForceMovingType`): `DeleteCurrentMover`, `SetCurrentMover(mover, MR_FORCED)`, then the copy's `mc_fAutoOffTime` := `time`: **a forced mover ends itself after `time` seconds** (it returns `CMR_REPLACE_ME`, which deletes it; with reason forced no `TacticEnded` is sent). `UnforceMovingType` = `DeleteCurrentMover`. The tactic actions: `CBAStopTactic` sets the reason to `MR_NONE` **only if it is** `MR_TACTIC` (the mover object stays but is re-chosen at the next call); `CBAStopUsingTactic` empties the **desired** tactic (not the current one) and, if the reason is `MR_TACTIC`, `DeleteCurrentMover`; `CBAReuseTactic` copies the **default** tactic (`en_henDefaultTacticEntity`) into the **desired** tactic (so the next call takes it, 3.3).

### 4. Running the mover (the second half of `MoveCharacter`, **medium**)

**4.1 Whether the mover runs.** The mover runs only when **all** hold: ability bit 2 (value 4) is set (it is the "may move" ability: `SetExecutionAbilities` stops the puppet when it removes bit 2 that was set; `DoDisable` and the dead state clear all bits; `brain-classes.md` 4.4 described it only as "a generic action may run", it also gates the two generic-state think loops); a current mover exists; the brain **has a foe**, or the mover answers "yes" to `CanOperateWithoutTargetEntity` (the patrol and navigation movers do, `behavior-mover-classes.md`); and the puppet **has touched the floor once** (`en_bTouchedFloorOnce`) **or moves in 3-D** (`MovingIn3DArea`: aircraft yes; the base puppet no; legged puppets decide by pose; hovercraft, rolling ball and turret inherit "no"). Otherwise the puppet is **stopped** (`StopMoving`: move type none, desired tempo zero) and the call returns the move period.

**4.2 The call.** The puppet's `en_fMoverMultiplier` is set again to the mover's `mc_fSpeedMultiplier`, then the mover's `MoveCharacter` runs and returns a `CharacterMoverResult`. What the brain does with it (the order is exact):

| Condition | Action | Next call after |
|---|---|---|
| the current **reason** is `MR_NAVIGATION_FAILED` (5), whatever the result | try navigation again (4.3) | the move period, or 0.01 s when a navigation mover was installed |
| result 3 `CMR_NAVIGATION_FAILED` | build a `CMCDontMove`, `SetCurrentMover(it, MR_NAVIGATION_FAILED)`; the next call's period is `1.0 + 2 * u` with `u` a random number in [0, 1): **1 to 3 s** | 1 to 3 s |
| result 1 `CMR_REPLACE_ME` | if the reason is `MR_TACTIC`: send the macro event **`TacticEnded`** to the puppet, then (when no tactic is pending) set the desired tactic to the finished tactic's **next tactic** (`en_henNextTactic`, possibly empty) and empty the current tactic; in every case `DeleteCurrentMover` | **0.01 s** |
| result 2 `CMR_NEED_NAVIGATION` | try navigation (4.3) | as 4.3 |
| result 0 `CMR_OK` | nothing | the move period |

(The order of the table is the order of the tests: the reason test comes **before** the result test; result 1 takes the branch above even for a reason other than a tactic, then the common tail deletes the mover.)

**4.3 Trying navigation.** Only when the world info entity says navigation can be used (`CanUseNavigation`: the world has a navigation mesh, `wld-navigation.md`) **and** the puppet does **not** move in 3-D: build a `CMNavigationMovingType` (`behavior-mover-classes.md` 4.4) and `SetCurrentMover(it, MR_NAVIGATION)`, and the call's period becomes **0.01 s**. Otherwise nothing is changed (the mover stays and will return the same result at the next call). So a navigation mover is installed only for ground characters in worlds with a mesh; a flier or a world without a mesh keeps walking its own mover. While the reason is `MR_NAVIGATION_FAILED` the "don't move" mover runs each period and navigation is re-tried each time (a stuck character keeps standing and keeps retrying).

**4.4 End of the call.** If the current band differs from the one before `CacheAreaData`, `OnAreaChanged` runs (it tells the think-point oscillator, the behavior that calls `ThinkPoint`, to re-evaluate; `brain-classes.md` 3.4). The call returns the period (4.2).

**4.5 `OnWallHit`** asks the current mover whether it accepts the hit (`CMCCircleAroundFoe` flips direction); with no mover it answers true.

### 5. Patrol, goal points and macro events (**medium**)

**5.1 Starting, continuing, stopping** (brain methods; the puppet macros that call them are in 5.3):

| Method | Rule |
|---|---|
| `StartPatrolingMarkers(marker, rotateOnLast)` | does nothing when the brain **has a foe** (foe manager and foe both exist), or when the marker is empty. Otherwise the puppet's `en_henCurrentPathMarker` := marker, `en_bRotateOnLastPatrolingMarker` := the flag, and the brain enters **`BS_PATROLLING_MARKERS`** (5) |
| `StartMacroPatrolingMarkers(marker, flag)` | `en_bMacroStartedPatroling` := true, **begin a latent macro call** (`en_mlidLatentID`: the script that called waits), then `StartPatrolingMarkers`. When that refuses (a foe) the latent call has **already begun** and no state change will end it: the script waits until something else ends the latent id (not read, **low**) |
| `ContinuePatrolingMarkers()` | when the puppet has a current marker: macro flag := true, begin a latent call, enter state 5 (**no foe test**) |
| `StopPatroling()` | `SetBehavior(none)` (the running goal-point behaviour is told to finish), the puppet's `DoStopPatroling`, then `ReconsiderBehavior` |
| `SetPatrolingSpeedMultiplier(x)`, `SetPatrolingRotateSpeedMultiplier(x)` | in state 5 only: puppet `en_fSpeedMultiplier` := x, resp. `en_fTurningVelocityMultiplier` := x |

**5.2 The patrol state.** Entering state 5 (`SetState`, **after** the exit hook of the old state; the new state is stored first as -1, so a hook that changes state again wins): `OnBeginPatrollingMarkers` = puppet `DoStartPatroling`, then the think routine `SendPuppetTowardsCurrentMarker` is scheduled **1e-5 s** later (the engine's smallest period, the same one `ReconsiderBehavior` uses). Leaving state 5: `OnEndPatrollingMarkers` ends the latent call when the macro flag is set (the flag is **not cleared**: later patrols started by the engine itself also end a latent call, the engine tolerates it, **medium-low**) and calls `DoStopPatroling`.

- **`DoStartPatroling`** (puppet; it is a script-RPC function: on a host it forwards the call, the body runs where the call is received; **medium-low**): `en_fSpeedMultiplier` := `en_fPatrolSpeedMultiplier`, `en_fTurningVelocityMultiplier` := `en_fPatrolRotateSpeedMultiplier`; `en_fAccelMultiplier` and `en_fDecelMultiplier` := the two patrolling multipliers of the puppet's moving abilities (`ma_fPatrolingAccelMultiplier`, `ma_fPatrolingDeccelMultiplier`), or 1.0 and 1.0 without parameters; `en_fThrustAddingAngleOverride` := `en_fPatrolThrustAddingAngle`; `en_maMoveAbilitiesTemp` := `en_maMoveAbilities`; `en_maMoveAbilities` := `en_maPatrolingMoveAbilities` **and** `en_maMoveAbilities` (a bit mask: patrolling can only remove abilities). **`DoStopPatroling`**: reset the velocity multipliers (`ResetVelocityMultipliers`), `StopMoving`, `en_maMoveAbilities` := `en_maMoveAbilitiesTemp`, puppet state `PS_DEFAULT` (1).
- **`SendPuppetTowardsCurrentMarker`** (think routine of the state): reads the puppet's current marker, its position, and the marker's **next marker**. With a next marker the brain orders a goal point **at the marker's position with `pass through` = true, `rotate on point` = false and a zero rotation**; with **no** next marker (the last one): `pass through` = false, `rotate on point` = **`en_bRotateOnLastPatrolingMarker`**, and the **rotation = the marker's own orientation** (Euler angles). The order is `OrderPuppetToGoToGoalPoint(position, pass through, rotate, rotation)`.
- **`OrderPuppetToGoToGoalPoint`** builds a `CGoToPointEntity` behaviour (a properties object with this brain, the point, the two flags and the rotation; `behavior-entity-classes.md`) and makes it the brain's current behaviour with `SetBehavior` (a non-behaviour result clears it). `ExamineSoundSource` uses the same call (T01-129).

**5.3 Arrival and the macro events.** When the puppet reaches its goal point the puppet's `OnGoalPointReached` copies the goal point into its last reached goal point and calls the brain's `OnGoalPointReached`: in state `BS_EXAMINE_THREAT_SOUND` (10) the brain stops the puppet and `ReconsiderBehavior`s; otherwise `RouteOnGoalPointReached` tells the **current mover** (its `OnGoalPointReached`; the ordered-marker mover marks its marker reached) and the **current behaviour** (the go-to-point behaviour finishes and calls `OnBehaviorFinished`).

`OnBehaviorFinished` in state 5 (**medium**), in this order: (a) no current marker: nothing at all (the state stays 5); (b) `en_henLastReachedMarker` := the current marker; the current marker := its **next marker**; (c) next marker empty: `StopPatroling`, else `SendPuppetTowardsCurrentMarker` (the next goal point at once); (d) send the macro event **`MarkerReached`** to the puppet; (e) when there was **no** next marker also send **`LastPatrolMarkerReached`**. (d) is the unconditional one and (e) the conditional one (the names are the two strings the function refers to; which of the two sends is first is by exclusion, **medium-low**.) States 3 (entering a ride) and all others: see `brain-classes.md` 3.2 (state 3 sets the "entered ride" flag and starts the ride; every other state reconsiders). `TacticEnded` is sent from `MoveCharacter` (4.2). The three events go to the **puppet entity** (a script entity: a level script's handlers named `MarkerReached` and so on, `wld-macro.md`; whether the data contains handlers with these names is T01-129's open question, not counted here).

**5.4 The puppet's macros and the first marker.**

- `PatrolMarkers(marker, rotate)` (script): ignored while the puppet's entity flag bit 0 (a "shut down" bit of the base entity, name not resolved) is set. When the **marker argument is omitted** it is `ObtainFirstPathMarker()`-like: the entity whose id is `en_ulAutoPatrolMarkerEntityID` when that is not -1 and is a path marker, else the **closest marker** (`GetClosestMarker`). `rotate` omitted = 0. Then, if the puppet has a brain and the marker exists: brain `StartMacroPatrolingMarkers(marker, rotate)`. `StopPatrolingMarkers` and `ContinuePatrolingMarkers` call the brain methods of the same name (same shut-down test).
- `ObtainFirstPathMarker()`: `en_ulAutoPatrolMarkerEntityID` = -1 -> `GetClosestMarker`; else the entity with that id when it is a `CPathMarkerEntity`, else none. `GetClosestMarker` builds an entity iterator carrying the puppet's position and a class identifier (given by name) and iterates the world's entities with it; **the closest one is the path marker with the smallest 3-D Euclidean distance from the puppet's own position, no distance limit, the first of equal distances winning (T01-155, `behavior-marker-movers.md` 5.2)**.
- **Auto patrol.** At the end of the puppet's `OnBoot`: when the puppet has a brain **and** `en_bAutoPatrolMarkers` is set, the puppet enters `PS_START_AUTO_PATROLING` (10), otherwise `PS_DEFAULT` (1). State 10's think routine `PrepareForAutoPatroling`: unless the puppet flag `PNF_DISABLE_STOP_BEFORE_PATROL` (set by the macro `macDisableStopBeforePatrol`) is set, it waits (rechecking every **0.1 s**) while the **body's speed is 0.01 or more**, then calls `StartAutoPatroling` (after 1e-5 s): `ObtainFirstPathMarker()` and, when found, the brain's `StartPatrolingMarkers(marker, **rotateOnLast = true**)`. An auto patrol therefore **rotates on the last marker**; a macro patrol rotates only when the macro says so.
- **The tactic members at boot.** `ResetDynamicMembers` (called by `OnBoot`): `en_henCurrentTacticEntity` := empty, **`en_henDesiredTacticEntity` := `en_henDefaultTacticEntity`**, `en_henCurrentPathMarker` and `en_henLastReachedMarker` := empty. The default tactic comes from the placed record (`ep_henTacticEntity` -> `en_henDefaultTacticEntity`, copied by `OnUpdate`). So **a puppet with a default tactic takes it at its first `MoveCharacter`** (3.3). The macro `SetTacticEntity(t)` sets only the desired member.

### 6. Data (script sections 1 to 3)

- Bands: 2.4 and 2.3.
- **Tactics** (59 worlds): 993 tactic entities; 306 with a marker, 113 with a next tactic (105 distinct tactics are the next of another; none is its own next); mover classes: `CMCMarkersMovingOrdered` 755, `CMCMarkersMovingClosestToFoe` 36, `CMCDontMove` 32, `CMCCircleAroundFoe` 32, `CMCTowardsFoe` 30, `CMCPatternMoving` 28, `CMCMarkersMovingRandom` 24, `CMCDiamond` 19, `CMCMellon` 14, `CMCStayInArea` 14, `CMCAwayFromFoe` 5, `CMCAirAttack` 1, none 3.
- **Puppets** (2,328 placed): **1,094 have a default tactic** (190 tactics are shared by two or more puppets); **364 have the auto-patrol flag** and **80 an auto-patrol marker id** (not -1); 77 have both a default tactic and the flag. Of the 1,094, **866 have a default tactic without a marker** (792 of them without the auto-patrol flag): their marker, if the mover needs one, is the puppet's own (the id above, or the closest marker, 5.4).

### 7. What follows for an implementer

1. Keep per brain: current mover (copy), reason, start time, last area id; per puppet: current, desired and default tactic, current and last marker, the patrol flags. Re-select exactly by 3.1 (area **id** compared, tactic overrides all).
2. A band lookup returns no band for a negative distance, an empty list or a foe beyond the last band; use `CMCTowardsFoe` then, and `en_ava` = all ones.
3. The periods: 0.01 s after replace/navigation, a random 1 to 3 s after a navigation failure, else `GetMovePeriod`.
4. Patrol = state 5 with one go-to-point behaviour per marker; events per 5.3; a foe ends it (`OnFoeAssigned` calls `StopPatroling` in state 5, `brain-classes.md` 3.3).

### 8. Corrections to earlier pages

1. `brain-classes.md` 3.4 `MoveCharacter`: it needs the foe manager **and** the `.cb` (both tests are there), the movement record holds the **foe's last seen position and a seen flag** (1.1), the tactic's three auto-off values are copied **into the mover** (3.3), the puppet's `en_fMoverMultiplier` (not the speed multiplier) is the one the mover sets (3.4).
2. `behavior-action-classes.md` 2.3 (`CBAStopUsingTactic`, `CBAReuseTactic`): they act on the **desired** tactic member (3.5), not the current one.
3. `behavior-mover-classes.md` 4.1 / `brain-classes.md` 3.5: the events `MarkerReached` and `LastPatrolMarkerReached` are sent by the **brain's** `OnBehaviorFinished` (5.3); the mover also marks its marker reached; the two are separate.

## Invariants

1. All 132 non-empty band lists are strictly ascending by distance (script, **high**).
2. 270 of the 402 band lists are empty (script); an empty list gives "no band".
3. A pending tactic is always taken at the next `MoveCharacter` (3.1; 1 function, **medium**).
4. After `DeleteCurrentMover` the reason is `MR_NONE` and the puppet's mover multiplier 1.0 (1 function, **medium**).
5. `StartPatrolingMarkers` never changes state while the brain has a foe (1 function, **medium**).

## Open questions

1. **Closest marker** (T01-242; the distance rule and the absence of a limit are answered by T01-155, `behavior-marker-movers.md` 5.2): which markers a `CMCMarkersMovingOrdered` tactic without a marker gets in the 866 puppets of section 6 (the movers' own `FirstTimeExecuted`, T01-155).
2. **Run-time check** of the periods and orders of 4.2 and 5.3 (T01-130, T01-038).
3. **What `MarkerReached` handlers exist** in the levels' scripts and what the latent call of `StartMacroPatrolingMarkers` does when a foe refuses the patrol (T01-243).
4. **`en_ava` use and the speed multipliers** in the puppet's movement step (T01-141, T01-142).
5. **The distance divisor** (`cb_fActiveRange` or another range of the behavior) and the other two fields of the record (T01-126).

## Test vectors

- **Lookup**, `Kleer.cb`, state `Default`, land bands (distance, id) = (3, `Slash`), (6, `Jump`), (25, `Close`), (70, `Middle`), (500, `Far`): distance -1 (no foe): no band; 0 and 2.99: `Slash`; 3 and 3.01: `Jump`; 499: `Far`; 500 and above: no band (default mover). Area ratio at d = 4.5 in `Jump`: start 3, end 6: `1 - (4.5-3)/(6-3)` = 0.5.
- **Keep or choose**: reason `MR_AREA`, area id `Close`, new band `Close` -> kept; new band `Middle` -> new copy of the band's mover (or `CMCTowardsFoe`), reason `MR_AREA`; reason `MR_FORCED` and a pending tactic -> the tactic's mover, reason `MR_TACTIC`.
- **Tactic end**: reason `MR_TACTIC`, mover returns `CMR_REPLACE_ME`: the event `TacticEnded` is sent, the next tactic (if any) becomes desired, the mover is deleted, the next call is in 0.01 s.
- **Navigation failure**: mover returns `CMR_NAVIGATION_FAILED`: a `CMCDontMove` with reason 5 becomes current; next call in [1, 3) s.
- **Patrol**: markers A -> B -> C (C has no next), rotate flag 1: goal point A (pass through, no rotation), B (pass through), C (stop, rotate to C's orientation); events: `MarkerReached` at A, B, C and `LastPatrolMarkerReached` at C; after C the patrol ends (`StopPatroling`).
- **Auto patrol**: a puppet with a brain and the auto flag, at boot: state `PS_START_AUTO_PATROLING`; when its body speed is below 0.01 the brain starts a patrol on the first marker with rotate-on-last = true.
- `tools/re/brain_mover_verify.py` prints: 82 files, 134 states, 402 lists (270 empty), 381 bands (57 without a mover), 132 of 132 non-empty lists ascending, last distance 1e8 in 3 lists; the velocity adjuster values of 2.3; 993 tactics, 1,094 puppets with a default tactic, 364 with the auto-patrol flag, 80 with a marker id, 866 default tactics without a marker.
