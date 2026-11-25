# Behaviour entities and the foe manager of `Sam2Game.dll` (T01-105, part of T01-026)

- **Roadmap task:** T01-105 (overview [`behavior-classes.md`](behavior-classes.md); the other parts are [`behavior-action-classes.md`](behavior-action-classes.md) and [`behavior-mover-classes.md`](behavior-mover-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class, method, member and enumeration names (read from the export table and the DLL's own tables), for the parents of the 38 classes whose parent the constructors name, and for the statement that no class of this page is stored in any file (`tools/re/behavior_classes_verify.py`); **medium** for the rules of sections 2 to 7 (each is one reading of a short function, **nothing was run**); **low** for every method marked `N` (described from its name) and for the open questions.

Prerequisites: [`behavior-classes.md`](behavior-classes.md) (what a behaviour is, the marks `R`, `P`, `N`), [`brain-classes.md`](brain-classes.md) (the brain that creates and runs these entities, its states and moments), `ai.md` (the `.cb` files), [`replication.md`](replication.md), [`puppet-legged-classes.md`](puppet-legged-classes.md) (the puppet side of the generic attacks).

## Summary

A **behaviour** (`CBehaviorEntity`) is a small entity that a brain spawns to do one job with its puppet and that tells the brain when it is done. This page covers the 32 classes of that family (16 entity and 16 properties classes) and the **foe manager** (4 classes). Nothing here is saved in a file: the brain makes these entities at run time, from properties objects that exist only in memory.

| Group | Classes | What they do |
|---|---|---|
| Base | `CBehaviorEntity`, `CBaseGenericBehaviorEntity` (+ Properties) | the finish and cancel protocol; the "generic behaviour" base that asks the puppet to perform one scripted action |
| Generic action behaviours (7 kinds) | `CGBAAnimation`, `CGBALeapOntoFoe`, `CGBALocalMove`, `CGBAMelee`, `CGBARunOverFoe`, `CGBAShoot`, `CGBAWait` `...BehaviorEntity` (+ Properties), and the list `CGBActionListBehaviorEntity` | one behaviour per kind of `.cb` action; **the action classes of [`behavior-action-classes.md`](behavior-action-classes.md) are factories for them** |
| Movement behaviours | `CGoToPointEntity`, `CGoToPointByPathEntity`, `CPossessRideEntity`, `CExitFromRideEntity` | walk to a point, walk a navigation path, board a ride, leave a ride |
| The think loop | `CThinkPointOscilatorEntity` | calls the brain's `ThinkPoint` over and over; the default behaviour of a brain that has a `.cb` |
| Obsolete | `CTurretBehaviorEntity`, `CRideFoeManagerEntity` (+ Properties) | their only code prints the obsolete-class error |
| Foe manager | `CFoeManagerEntity` (+ Properties) | a three-state machine that chooses, keeps and loses the brain's foe |

Findings an implementer needs first:

1. **The `.cb` actions do not act, they start behaviours.** Each of the six actions that does something with the body (`CBAAnimation`, `CBAShoot`, `CBAMelee`, `CBALocalMove`, `CBARunOverFoe`, `CBALeapOntoFoe`), plus `CBAWait` and `SBAList`, spawns the matching generic behaviour and hands it to the brain (`StartGenericBehavior`). The behaviour asks the puppet to perform the action (`CanPerformGenericShooting`, `DoGenericMeleeAttack`, `DoGenericAnimation`, `DoLocalMove` ...), remembers whether the puppet accepted (`en_bCanPerformAction`), and the action's `Execute` returns **false** (and the brain is told `GenericBehaviorNotStarted`) when the puppet refused (section 3, **medium**).
2. **A brain in its normal moving state has one running behaviour, the think-point oscillator**, which calls the brain's `ThinkPoint` at a computed period; a generic behaviour replaces it for as long as it runs and the brain makes a new oscillator when it is done (section 6; `brain-classes.md` 3.2).
3. **The foe manager is a state machine with three states and a hostility ranking** (`FoeHostility` 0 to 9): a candidate replaces the current foe only if it ranks higher, with three special cases (section 7). A foe that has not been seen for 60 s is lost, except a hunted one.
4. **Two classes of the roadmap line are dead**: `CTurretBehaviorEntity` and `CRideFoeManagerEntity` print `Class (<name>) is obsolete and should be removed from level ASAP.` and do nothing else (the same function as the 16 obsolete brain kinds, `brain-classes.md` section 2.1).
5. **No class of this page is replicated** (net behaviour inherited `NEB_NONE`, 0 of the 151 members declared for the family have the replication flag). The only host-only check is in `CExitFromRideEntity` (section 5).

## Evidence

- **Exported names.** The 95 classes of the family and their 955 exported functions (`behavior-classes.md` section 1); this page covers 36 classes (18 entity and 18 properties classes) with 365 of them: the 32 behaviour classes 304, the foe manager and its obsolete subclass 61. Of the 365, 113 are named methods (all on the entity classes); the rest are the standard functions.
- **Parents.** The constructors of the properties classes call `CBaseProperties` (all 18 read). The constructors of the entity classes call `CBaseEntity` directly (the intermediate constructors are inlined) except the seven generic kinds, the list and the ride foe manager, which call their parent's constructor. The behaviour parent (`CBehaviorEntity`, and `CBaseGenericBehaviorEntity` for the kinds) is therefore taken from two other readings that agree: the base class named in the DLL's member tables (where one exists) and the virtual table, whose first 117 (118 for the generic kinds) slots are the parent's with the parent's methods in the hook positions (**medium**).
- **Virtual tables.** For the 16 behaviour entity classes (and the foe manager and its subclass) the table was compared with the parent's slot by slot; the overridden and new hooks of section 1 come from that comparison (**medium**: the linker shares identical small functions, so a slot is only named when the class itself exports a method of that name).
- **Member tables.** The DLL declares 21 tables for classes of this page that no file stores (`behavior-classes.md` section 4), with 93 members; the member names are quoted as declared. None of the 93 has the replication flag; three carry the "owned or temporary" flag (`en_hnpPathToFollow`, `en_henRenderable`, `en_pbalActionList`) and one the "editor only" flag (`ep_pbaLocalMove`).
- **Methods.** Of the 113 named methods: 83 decompiled and read (`R`; many are 3 to 12 lines), 29 profiled from their strings and callees (`P`), 1 described from the name (`N`: `CGoToPointByPathEntity::OnDelete`). The long routines `LookForFoe`, `TrackCurrentFoe`, `GetSortedFoeCandidates`, `ThreatSoundHeard` and the thirteen run-over-foe methods were only profiled.
- **Enumerations.** `FoeHostility` (`FH_`), `FoeManagerState` (`FMS_`), `PossesRideState` (`PRS_`), `RunOverFoeStates` (`ROFS_`) and `CharacterMoverResult` (`CMR_`) are quoted from the DLL's enumeration tables (name, value, editor label).
- **Not checked.** Nothing was run. The numeric constants the foe manager and the oscillator read (periods, sleep times) were not extracted (open, T01-156). Data: this family is in no file, so there is nothing to count; the script checks that (`behavior_classes_verify.py` section 1).

## Layout / rules

### 1. The classes

"Parent" is the C++ parent. "Table" = members the DLL declares (the data name is the `S` twin; `GoToPoint*` are registered without a first-letter prefix, quoted as found). None is in the data.

| Class | Parent | Symbols | Table (members) | Hooks it overrides (new virtual methods in **bold**) |
|---|---|---|---|---|
| `CBehaviorEntity` | `CBaseEntity` (via `SBaseEntity`) | 22 | `SBehaviorEntity` (3) | `OnCreate`, `OnDelete`, `AddDebugEntites`; **`DoDelete`, `CallBehavior`, `OnChildBehaviorFinished`, `OnChildBehaviorCanceled`, `OnThisBehaviorFinished`, `OnThisBehaviorCanceled`, `BehaviorFinished`, `BehaviorCanceled`, `OnDamage`, `OnActionFinished`, `OnActionFinishedInternal`, `OnGoalPointReached`** (12 new, 117 slots) |
| `CBehaviorProperties` | `CBaseProperties` | 7 | `SBehaviorProperties` (1) | none |
| `CBaseGenericBehaviorEntity` | `CBehaviorEntity` | 10 | `SBaseGenericBehaviorEntity` (1) | `OnThisBehaviorFinished`, `OnActionFinishedInternal`; **`IsThisAttackBehavior`** |
| `CBaseGenericBehaviorProperties` | `CBaseProperties` | 7 | none | none |
| `CGBAAnimationBehaviorEntity` / `Properties` | `CBaseGenericBehaviorEntity` / `CBaseProperties` | 8 / 7 | `SGBAAnimationBehaviorProperties` (1) | `OnCreate` |
| `CGBALeapOntoFoeBehaviorEntity` / `Properties` | same | 12 / 7 | `SGBALeapOntoFoeBehaviorEntity` (2), `...Properties` (1) | `OnCreate`, `OnBoot`; **`RotateTowardsFoe`, `LeapOntoFoe`**, `IsThisAttackBehavior` |
| `CGBALocalMoveBehaviorEntity` / `Properties` | same | 8 / 7 | `SGBALocalMoveBehaviorProperties` (1) | `OnCreate` |
| `CGBAMeleeBehaviorEntity` / `Properties` | same | 9 / 7 | `SGBAMeleeBehaviorProperties` (1) | `OnCreate`, `IsThisAttackBehavior` |
| `CGBARunOverFoeBehaviorEntity` / `Properties` | same | 24 / 7 | `SGBARunOverFoeBehaviorEntity` (13), `...Properties` (7) | `OnCreate`, `OnBoot`, `OnDelete`, `IsThisAttackBehavior`; **13 new** (section 3.4) |
| `CGBAShootBehaviorEntity` / `Properties` | same | 9 / 7 | `SGBAShootBehaviorProperties` (3) | `OnCreate`, `IsThisAttackBehavior` |
| `CGBAWaitBehaviorEntity` / `Properties` | same | 9 / 7 | none | `OnCreate`; `AfterWaitingPeriod` (a scheduled method) |
| `CGBActionListBehaviorEntity` / `Properties` | same | 12 / 7 | `SGBActionListBehaviorEntity` (2), `...Properties` (1) | `OnCreate`, `OnChildBehaviorFinished`, `OnActionFinishedInternal`; **`StartNextAction`, `ExecuteChildBehavior`** |
| `CGoToPointEntity` / `Properties` | `CBehaviorEntity` / `CBehaviorProperties` | 10 / 7 | `GoToPointEntity` (4), `...Properties` (4) | `OnCreate`, `OnGoalPointReached`; **`SendPuppetTowardsGoalPoint`** |
| `CGoToPointByPathEntity` / `Properties` | same | 11 / 7 | `GoToPointByPathEntity` (10), `...Properties` (6) | `OnCreate`, `OnDelete`, `OnGoalPointReached`; **`GoingTowardsPathPoint`** |
| `CPossessRideEntity` / `Properties` | same | 18 / 7 | `SPossessRideEntity` (9), `...Properties` (2) | `OnCreate`, `OnActionFinished`, `OnGoalPointReached`; **`PossesRide`, `SetState`, `OnReachDestinationSeatBegin`, `ReachDestinationSeat`, `EnterOntoSeat`** (+ `ReachWaypointSeatLoop`, `SendPuppetTowardsClimbPoint`, `WaitRideIdleLoop` as scheduled methods) |
| `CExitFromRideEntity` / `Properties` | same | 10 / 7 | none | `OnCreate`, `OnDelete`; **`ExitFromRide`** |
| `CThinkPointOscilatorEntity` / `Properties` | same | 12 / 7 | none | `OnCreate`; **`OnFoeAssigned`, `OnAreaChanged`, `ExecuteBehavior`** (+ `CallThinkPoint`) |
| `CTurretBehaviorEntity` / `Properties` | same | 8 / 7 | none | `OnCreate` (the obsolete-class error) |
| `CFoeManagerEntity` | `CBaseEntity` (via `SBaseEntity`) | 39 | `SFoeManagerEntity` (20) | `OnCreate`, `OnDelete`, `OnSeeTarget`, `OnNotSeeTarget`, `AddDebugEntites`; **19 new** (section 7), 124 slots |
| `CFoeManagerProperties` | `CBaseProperties` | 7 | `SFoeManagerProperties` (1) | none |
| `CRideFoeManagerEntity` / `Properties` | `CFoeManagerEntity` / `CBaseProperties` | 8 / 7 | none | `OnCreate` (the obsolete-class error) |

The brain refers to behaviours as `en_henBehavior` (`brain-classes.md` section 5); the data class `SPuppetEntity` has no link to a behaviour entity.

### 2. `CBehaviorEntity`: the finish protocol (**medium**)

A behaviour has three links (`SBehaviorEntity`): `en_henBrain` (its brain), `en_henChildBehavior` and `en_henParentBehavior`. Its properties carry the brain (`ep_henBrain`); creating the entity stores the brain handle. A brain spawns a behaviour by building a properties object of the right class in memory, giving it the brain and the data, and calling `SpawnEntity`.

**Scheduling.** Almost every behaviour does its first step by *scheduling one of its own methods for the next instant*: the entity's "call this method after `t`" service is used with `t` = **1 x 10^-5 s** and a method name (`CallThinkPoint`, `SendPuppetTowardsGoalPoint`, `PossesRide`, `ExitFromRide`, `AfterWaitingPeriod`, `DoDelete` ...). This is how the brain's code can return before the behaviour acts (`R`: the same constant and call in the oscillator, go-to-point, ride and exit behaviours).

| Method | What it does | Basis |
|---|---|---|
| `OnCreate` | Stores the brain from the properties; when the debug flag "Behavior start/stop (log)" is on, prints `Brain <id> starts behavior <class>`. | R |
| `OnDelete` | Cancels pending scheduling, prints `... stops behavior ...` under the same flag, deletes the child behaviour if there is one. | R |
| `CallBehavior(child)` | Deletes the current child, stores the new one and sets the child's parent link to this behaviour (a behaviour can call a sub-behaviour; only the action list does). | R |
| `DoDelete` | Schedules deletion of this entity for the next instant (name `DoDelete`). | P |
| `BehaviorFinished`, `BehaviorCanceled` | Without a parent: calls `OnThisBehaviorFinished` / `OnThisBehaviorCanceled`. With a parent: calls the parent's `OnChildBehaviorFinished` / `OnChildBehaviorCanceled`. | R |
| `OnThisBehaviorFinished`, `OnThisBehaviorCanceled` | Tell the brain `OnBehaviorFinished` / `OnBehaviorCanceled` (`brain-classes.md` section 3.2: after a patrol step the brain sends `MarkerReached`; otherwise it re-enters its default state). | R |
| `OnActionFinished`, `OnActionFinishedInternal` | The puppet reports that the action it was doing is over: without a parent the entity calls its own `OnActionFinishedInternal` (empty in the base, so the base ignores it), with a parent it hands the report up. | R/P |
| `OnGoalPointReached`, `OnDamage` | Same routing to the parent (reports from the puppet: the goal point was reached; damage was taken). | P |
| `OnChildBehaviorFinished`, `OnChildBehaviorCanceled` | Empty in the base (for a parent to override). | R |
| `AddDebugEntites` | Debug display: adds the entities it relates to. | N |

### 3. The generic behaviours

#### 3.1 The base (`CBaseGenericBehaviorEntity`, **medium**)

The member `en_bCanPerformAction` is set by each kind's `OnCreate` to whether the puppet accepted the request. `IsThisAttackBehavior` is true for shoot, melee, leap and run-over (the four attack kinds, `ATT_SHOOT`, `ATT_MELEE`, `ATT_LEAP`, `ATT_ROF` of the `AttackType` enumeration) and false for animation, local move, wait and list. `OnThisBehaviorFinished` (**R**): tells the brain `OnGenericActionBehaviorFinished`; then, if the brain has a foe manager, draws a random number and, when it is **below the character behaviour's `fad_fAutoChangeFoeChance`** (`ai.md`: 0 to 10 in the 82 files, a probability with the scale of the engine's random number; the unit is open), asks the foe manager to `TrySwitchingToAnotherFoe` (section 7). So **an enemy re-chooses its foe after finishing a scripted action, with a per-behaviour chance**.

#### 3.2 The seven kinds (`OnCreate`; rejection texts are the binary's own)

| Kind | Properties members (DLL names) | What `OnCreate` asks the puppet | When it refuses (sets `en_bCanPerformAction` false) | Basis |
|---|---|---|---|---|
| `CGBAAnimationBehavior` | `ep_idAnimation` | `DoGenericAnimation(animation)` after mapping the scheme animation through the model (`samRemapSchemeAnim`) | the model has no such animation: console text `Can't find animation: "<id>" in model: "<file>"` | R |
| `CGBAMeleeBehavior` | `ep_idMeleeAttack` | `DoGenericMeleeAttack(attack id)` | the puppet is not legged (`Melee action can only be requested for legged puppets.`), the foe cannot be seen (`Melee action of type "<id>" is rejected because foe can't be seen`), or the puppet's own can-perform test for generic melee says no | R |
| `CGBAShootBehavior` | `ep_idShoot`, `ep_ctConsecutiveShootings`, `ep_bStopToShoot` | `DoGenericShooting(shooting id, count)`; with `ep_bStopToShoot` the puppet is told to stop moving first | the id is not in the puppet's table of shootings (`Shooting type "<id>" is rejected because it is not defined in table of shootings.`), the foe cannot be seen (`Shooting action of type "<id>" is rejected because foe can't be seen`), or `CanPerformGenericShooting` says no | R |
| `CGBALeapOntoFoeBehavior` | `ep_idLeapAttack` | rotates toward the foe, then `LeapOntoFoe` | the puppet is not legged (`Leap action can only be requested for legged puppets.`); the foe is NULL (`Melee action of type "<id>" is rejected because foe is NULL`, the binary's own copied text) or cannot be seen | P |
| `CGBALocalMoveBehavior` | `ep_pbaLocalMove` (a pointer to the `SBALocalMove` action, flagged) | `DoLocalMove(action)`; the result of the call is the acceptance | `DoLocalMove` returned false | R |
| `CGBAWaitBehavior` | none declared | nothing: accepts at once and schedules `AfterWaitingPeriod` after the wait time carried in the properties | never | R |
| `CGBARunOverFoeBehavior` | 7 (section 3.4) | the state machine of 3.4 | the attack is not found | P |

Entity-side members the DLL declares: `CGBALeapOntoFoe` `en_idLeapAttack`, `en_tmBehaviorStarted`; the other kinds keep the properties values in members of the same name without a table.

`AfterWaitingPeriod` (the wait is over) is the shared one-call function that calls the behaviour's own `BehaviorFinished` (section 2), so **after a `CBAWait` the behaviour reports itself finished and the brain re-enters its default state** (`OnBehaviorFinished`, `brain-classes.md` 3.2); the same function is the body of `CGoToPointEntity::OnGoalPointReached` and the base class's `OnActionFinishedInternal` (**R**).

#### 3.3 `CGBActionListBehaviorEntity` (the `SBAList` action)

Members: `en_pbalActionList` (owned: a copy of the `SBAList`, made with the engine's meta copy) and `en_iCurrentAction`. `StartNextAction` walks the list from the current index, advancing it for every entry: it asks the entry to start against the brain (the action's `Execute`) and, when that started a child behaviour, stops and waits (`OnChildBehaviorFinished` calls `StartNextAction` again); entries whose `Execute` starts no child are passed over in the same loop; when the list is exhausted it calls `ResetActionTime` on the brain and finishes itself with `BehaviorFinished` (**medium-low**: read in outline; the exact test that decides "started a child" is not resolved). `ExecuteChildBehavior(handle)` hands a spawned child to `CallBehavior`.

#### 3.4 `CGBARunOverFoeBehaviorEntity` (state machine, **medium-low**)

The state is `en_rofsState` (`RunOverFoeStates`): `ROFS_CANCELED` 0, `ROFS_ROTATE_TOWARDS_FOE_BEFORE` 1, `ROFS_ROTATE_TOWARDS_FOE_AFTER` 2, `ROFS_APPROACH_CLOSE_ENOUGH` 3, `ROFS_RUN_OVER` 4, `ROFS_KICK` 5, `ROFS_CONTINUE_RUN_THROUGH` 6. The sequence the method names give: turn to the foe, approach until close enough, run over the foe for a time between the action's minimum and maximum (`rof_fRunOverFoeTimeMin/Max`, drawn at random when the action is executed) with the speed multiplier `ep_fSpeedMultiplier`, when the kick distance (`ep_fKickAnimDistance`) is reached play the kick animation (`ep_idKickAnim`, term `en_idKickTerm`), continue running through the foe for `ep_fRunTroughDistance`, turn again, and wait `ep_fPostAttackWait`. The behaviour saves the puppet's emotion and moving abilities before the attack (`en_ceBeforeROF`, `en_maBeforeAttack`) and restores them (`CheckRestoreEmotionState`; `en_bCharacterEmotionRestored`). Its `SetState` function is the only one with strings (`Kick`, `ContinueRunThrough`, `RotateTowardsFoeAfter`: the names of scheduled methods). The numbers inside the states were not extracted (T01-152).

### 4. Movement behaviours

#### 4.1 `CGoToPointEntity` (**medium**)

Properties and entity carry `ep_vPoint`, `ep_bRotateOnGoalPoint`, `ep_bPassThroughGoalPoint`, `ep_vGoalPointRotationEul`. `OnCreate` copies them and schedules `SendPuppetTowardsGoalPoint`, which gives the puppet a goal point (`SetGoalPoint`) or, with "pass through", a run-through point (`SetRunTroughPoint`), and when rotation is requested the goal-point rotation; `OnGoalPointReached` ends with `BehaviorFinished` (the shared one-call function). The brain creates it in `OrderPuppetToGoToGoalPoint` (`brain-classes.md` section 3.5).

#### 4.2 `CGoToPointByPathEntity` (**medium-low**)

Properties `ep_vStartPoint`, `ep_vEndPoint`, `ep_henLookTarget`, plus the three goal-point members of 4.1. `OnCreate` asks the **world info entity for a navigation path** between the two points (the world info entity answers path queries; `wld-navigation.md`), keeps it (`en_hnpPathToFollow`, owned) and creates a debug renderable (`en_henRenderable`); `GoingTowardsPathPoint` walks the path from its **end** backwards (`en_iCurrentWayPoint` counts down to -1) and, before sending the puppet to a way point, tries to skip up to **5** following points whenever the world info entity says the straight line between the current point and that point is free (a walkability test with two radii of 2.0 and 2.0), counting the points it skipped in `en_ctPointsProcessed`. `OnGoalPointReached` decrements the index and counts the point; when the index has passed the first point the behaviour finishes (`BehaviorFinished`), otherwise it calls `GoingTowardsPathPoint` for the next step.

#### 4.3 `CPossessRideEntity` (boarding a ride, **medium-low**)

Its state is `en_prsState` (`PossesRideState`): `PRS_REACH_DESTINATION_SEAT` 0, `PRS_REACH_WAYPOINT_SEAT` 1, `PRS_ENTER_ONTO_SEAT` 2, `PRS_WAIT_RIDE_IDLE` 3. Members (`SPossessRideEntity`): `en_henRide`, `en_bTeleport`, `en_idDestSeat`, `en_idSeatToReach`, `en_idReachedSeat`, `en_iWayToDestSeat`, `en_henSeatToReachOwner`, `en_henReachedSeatOwner`, `en_prsState`. `OnCreate` copies the ride and the teleport flag from the properties, reads the puppet's seat bookkeeping (the seat it should take and the way to it) and schedules `PossesRide`. `PossesRide` finishes the behaviour at once when the ride or puppet is missing; **with teleport** it places the puppet at the seat's enter point (`RepositionPuppet`) and sets the state `PRS_ENTER_ONTO_SEAT`; otherwise it starts at `PRS_REACH_DESTINATION_SEAT`. `ReachDestinationSeat` looks the seat up by name (`FindSeatDataByName` of the ride) and, when the seat has a way point seat (`SWayToSeat`) that is not yet reached, makes that the seat to reach and switches to `PRS_REACH_WAYPOINT_SEAT`; entering that state sets a puppet flag that is cleared on leaving it, and `ReachWaypointSeatLoop` and `SendPuppetTowardsClimbPoint` (profiled) walk to the climb point. `EnterOntoSeat` stops the puppet and tells it to enter the ride (ride, seat, way). `PRS_WAIT_RIDE_IDLE` asks the puppet to idle for 2.0 s and polls (`WaitRideIdleLoop`); `OnActionFinished` in `PRS_ENTER_ONTO_SEAT` copies the reached seat into the seat-to-reach members, places the puppet on the seat and goes back to `PRS_REACH_DESTINATION_SEAT`; `OnGoalPointReached` (puppet alive, not already waiting) goes to `PRS_WAIT_RIDE_IDLE`. Seat choice (`GetAvailableSeat`) is in the brain (T01-128).

#### 4.4 `CExitFromRideEntity` (**medium-low**)

`OnCreate` schedules `ExitFromRide`; on the **host only** (`netIsHost`) it tells the puppet to stop riding (`DoStopRiding`). `OnDelete` clears the puppet's "leaving a ride" flag and calls the base.

### 5. Net behaviour of the behaviours (**medium**)

None of the classes overrides `GetNetBehavior`, `GetRelevantDistance` or `GetUpdatePriority` (the slot comparison), so all are `NEB_NONE` (`replication.md` section 3.1): a behaviour exists on the host only. The 21 tables of this page have **0 replicated members** (flag 0x4000) among their 93 members (the whole family: 0 of 151). The one place that distinguishes host from client is `CExitFromRideEntity::ExitFromRide`. The brain's own link to the puppet is the only replicated member of the AI family (`brain-classes.md` section 4.5).

### 6. `CThinkPointOscilatorEntity` (**medium-low**)

No member table. The brain spawns it in `ReconsiderBehavior2` and in `DefaultCharacterMovingState` (`brain-classes.md` 3.2) as the brain's current behaviour. `OnCreate` schedules `CallThinkPoint` for the next instant. `CallThinkPoint` calls the brain's **`ThinkPoint`** (`R`). `OnFoeAssigned` and `OnAreaChanged` (both are the same function) reschedule `CallThinkPoint` for the next instant: **a new foe or a change of distance band causes an immediate think** (`R`). `ExecuteBehavior` schedules the next `CallThinkPoint` after a period that it computes (`P`/`R` outline): it draws one random number; **corrected by T01-153 (`behavior-action-choice.md` section 7):** the period comes from the **current band's** `cba_pgrActionFrequencyOverAreaRatio` (a period in seconds), randomised by the band's `cba_fActionPeriodRandomizeMultiplier` with the puppet's own random member and divided by the difficulty's `dif_fThinkFrequencyMultiplier`; `cb_pgrThinkPeriodOveride` is read by the brain's `GetMovePeriod`, not here; the result is reduced by the time already elapsed since the last think and never below **1 x 10^-5 s**. The default period without the graph, the exact factors and what clears `en_bImmediateThinkPoint` were not extracted (T01-126 reads the brain side; the oscillator numbers are T01-156).

### 7. The foe manager (`CFoeManagerEntity`)

#### 7.1 Members (`SFoeManagerEntity`, 20; **medium**, from the names and from the functions that write them in the order of the table)

| Member | Meaning |
|---|---|
| `en_henBrain` | the brain this manager serves (`OnCreate` prints `SFoeManager::OnCreate() Brain that uses foe manager is NULL.` when it is empty) |
| `en_henToBeFoe` | the candidate chosen but not yet confirmed as the foe |
| `en_tmAutoAssignFoe` | time at which the manager may assign a foe on its own |
| `en_fAutoLoseFoePeriod` | the time after which the foe is dropped again; **the third argument of `AssignFoe`**, -1 meaning never |
| `en_henFoe` | the current foe |
| `en_fhFoeHostility` | the hostility rank the foe was assigned with (`FoeHostility`, 7.2) |
| `en_tmFoeLastSeenTime`, `en_vFoeLastSeenPos`, `en_bFoeSeenOnLastTest` | when and where the foe was last seen, and the result of the last line-of-sight test (`OnSeeTarget` sets them, `OnNotSeeTarget` clears the last) |
| `en_henLastTestedFoe` | the candidate tested last |
| `en_tmFoeAssigned`, `en_bScreamed`, `en_tmNearFoeAssigned` | when the foe was assigned; whether the "sight" scream of the first assignment was done (cleared by `AssignFoeNow`); when a `FH_NEAR` foe was assigned |
| `en_bCantAssignPlayerAsFoe`, `en_bCantAssignNonPlayerAsFoe` | set by `DisablePlayerFoes` / `DisableNonPlayerFoes`, cleared by the `Enable...` pair; **driven by the action `CBAFoeAssigning` (data name `CBAPlayerAssigning`)** |
| `en_bHasFoeCandidates` | read by the validity term `CBAVTHasFoeCandidate` |
| `en_fmsState` | the state (7.3) |
| `en_lafLastAssignedFoe` | an enumerated "last assigned foe" kind (1 or 2, set by `AssignFoeNow`: probably player or not; **not resolved**) |
| `en_bAutoChangesFoes` | set at creation when the character behaviour's auto-change chance is above zero (`OnCreate`, **R**) |
| `en_bHasHadFoe` | set by the first assignment |

#### 7.2 `FoeHostility` (the rank of a reason to fight; values and editor labels from the DLL)

| Value | Name | Label | Set by (**R**) |
|---|---|---|---|
| 0 | `FH_LIGHT` | Light | (name); the lowest rank; one of the three values the data's `CBAForcePlayerFoe` uses (3 of 9) |
| 1 | `FH_HEARD` | Heard | (sound examination) |
| 2 | `FH_SEEN` | Seen | `OnSeeTarget` in the looking state |
| 3 | `FH_SENSED` | Sensed | (name) |
| 4 | `FH_DAMAGED` | Damaged | `OnBrainDamaged` (only when a condition on the damager that was not resolved holds) |
| 5 | `FH_NEAR` | Near | (name; has a special case below) |
| 6 | `FH_FORCED` | Forced | `ForceFoe`; the data's other `CBAForcePlayerFoe` value (3 of 9) |
| 7 | `FH_TOUCHED` | Touched | `OnTouchedByBrain` |
| 8 | `FH_HUNTING` | Hunting | `HuntFoe` |
| 9 | `FH_AUTO_CHANGED_FOE` | Always | the label says "always": `AssignFoe` accepts this rank whatever the current foe; the data's `CBAForcePlayerFoe` value 9 (3 of 9); the value `TrySwitchingToAnotherFoe` passes was not resolved (probably this one, **low**) |

The three data values of `CBAForcePlayerFoe.bafpf_fhFoeHostility` are **0, 6 and 9** (3 objects each, script): this settles the open question "the meaning of `FoeHostility` 6 and 9" of `ai.md`: 6 is `FH_FORCED` and 9 is `FH_AUTO_CHANGED_FOE` (label "Always").

#### 7.3 States and the assignment rule

`FoeManagerState`: `FMS_DISABLED` 0, `FMS_LOOKING_FOR_FOE` 1, `FMS_TRACKING_CURRENT_FOE` 2. `OnCreate` starts in `FMS_LOOKING_FOR_FOE`. `SetState` cancels the pending call; entering looking drops the brain's current mover (`DeleteCurrentMover`) and starts looking (`BeginLookingForFoe`, which tells the world info entity the brain has no foe); entering tracking schedules the tracking routine. `Disable` loses the foe and goes to `FMS_DISABLED`; `Enable` goes to looking; `IsDisabled` is state 0.

**`AssignFoe(candidate, hostility, autoLosePeriod)`** (**R**, **medium**; the long routine was read once): nothing happens in `FMS_DISABLED`, nor when the candidate is the same as the current foe at the same hostility (only a higher rank updates the stored hostility and the auto-lose period), nor when the candidate is a player and players are disabled or a non-player and non-players are disabled. Otherwise the candidate **becomes the foe when**: there is no foe, **or** the new hostility is 9, **or** the current and the new hostility are both `FH_NEAR` (5, then at most once per fixed interval of the clock), **or** the new rank is higher than the current one; a `FH_DAMAGED` (4) assignment has one further refusal condition (it depends on a kind recorded by `en_lafLastAssignedFoe` and on the current rank being 6, 8 or 9; not resolved). On acceptance: the manager stores the hostility and period, sets `en_bHasHadFoe`, and calls **`AssignFoeNow`**, which stores the foe, tells the brain `OnFoeAssigned` and `AdjustClientOrientation`, notes that the world info entity has the brain, records the time, clears `en_bScreamed` and goes to `FMS_TRACKING_CURRENT_FOE`. Every route into `AssignFoe` first asks the world info entity whether the two brains may be foes (`OnBrainDamaged`, `OnTouchedByBrain`, `HuntFoe`; the test itself is not read, T01-156).

**Seeing and losing.** In the looking state `OnSeeTarget` (a line-of-sight answer) assigns the target with `FH_SEEN` (2) and period -1. In the tracking state it records the time and place of the sighting. `OnNotSeeTarget` clears `en_bFoeSeenOnLastTest`; in the tracking state, **when 60 s have passed since the foe was last seen and the hostility is not 8 (`FH_HUNTING`), the foe is lost** (`FoeLost`: clears foe and candidate, tells the brain `OnFoeLost`, goes to looking). `LoseFoe` is `FoeLost` through the brain. `OnIKilledFoe` (tracking only): when the dead one was this brain's foe it raises the brain's three celebrate calls (`CelebrateIKillMyFoe`, `CelebrateIKillSomeFoe`, `CelebrateAllFoesDied`). `CalculateSleepTime` returns the pause before the next foe search as a linear function of the distance from the brain's puppet to the nearest candidate, between two constants (not extracted): **far brains look less often**.

#### 7.4 Methods

| Method | What it does | Basis |
|---|---|---|
| `OnCreate`, `OnDelete` | `OnCreate`: store the brain, set `en_bAutoChangesFoes`, enter the looking state. `OnDelete`: tell the world info entity the brain is gone. | R |
| `SetState`, `Enable`, `Disable`, `IsDisabled` | the state machine of 7.3 (`SetState` also names the entity's debug label `My foe`) | R |
| `AssignFoe`, `AssignFoeNow`, `ForceFoe`, `HuntFoe`, `OnBrainDamaged`, `OnTouchedByBrain`, `LoseFoe`, `FoeLost`, `GetFoe` | 7.2 and 7.3; `ForceFoe` = `AssignFoe(x, FH_FORCED, period)`, `GetFoe` returns the handle | R |
| `EnablePlayerFoes`, `DisablePlayerFoes`, `EnableNonPlayerFoes`, `DisableNonPlayerFoes` | write the two flags of 7.1 | R |
| `OnSeeTarget`, `OnNotSeeTarget` | line-of-sight answers (7.3) | R |
| `OnIKilledFoe` | celebrate calls | R |
| `BeginLookingForFoe` | tell the world info entity the brain has no foe, schedule `LookForFoe` | R |
| `LookForFoe` | the periodic search: sorts candidate puppets (`GetSortedFoeCandidates`), asks for a line of sight to one at a time, debug text `... LookingForAFoe state, trying to see foe: ` | P |
| `TrackCurrentFoe` | the periodic check of the current foe's visibility and distance, with a debug text `... TrackingCurrentFoe state, trying to see foe: ` | P |
| `GetSortedFoeCandidates` | fills a list of candidate puppets up to a distance, nearest first (arguments: distance, whether players are included, whether non-players are) | P |
| `TrySwitchingToAnotherFoe` | gets sorted candidates within a multiple of the distance to the current foe and, for each in turn, runs a line-of-sight request and assigns the first that passes (the rank passed was not resolved) | P |
| `SwitchToClosestPlayer` | when players are not disabled, assigns the closest player | R |
| `ThreatSoundHeard` | reaction to a sound (hearing thresholds, `FH_HEARD`): T01-129 | P |
| `CalculateSleepTime` | 7.3 | R |
| `CheckSelfCombustion` | returns false (a 3-byte function; a hook) | R |
| `AddDebugEntites` | debug | N |

## Invariants

1. None of the 36 classes of this page is stored in any of the 8,063 format-8 files, under its own name or as an `S` twin (script section 1: 0 of 36; 21 of the family's 24 binary-only member tables belong to this page).
2. The 16 entity classes of the behaviours derive from `CBehaviorEntity` (the 7 generic kinds and the list from `CBaseGenericBehaviorEntity`); `CRideFoeManagerEntity` derives from `CFoeManagerEntity` (constructors, 17 of 17 read).
3. `CTurretBehaviorEntity` and `CRideFoeManagerEntity` have one override each, the obsolete-class error (2 of 2 read, shared function).
4. `FoeHostility` has 10 values 0 to 9, `FoeManagerState` 3 values, `PossesRideState` 4 and `RunOverFoeStates` 7 (tables read).
5. The data values of `CBAForcePlayerFoe.bafpf_fhFoeHostility` are exactly {0, 6, 9} (9 objects in 3 files, script).

## Open questions

1. **The generic-behaviour protocol on the puppet side**: what `CanPerformGenericShooting`, `DoGenericShooting`, `DoGenericMeleeAttack`, `DoGenericAnimation`, `DoLocalMove` and the run-over and leap states do and how the puppet reports completion, the numbers inside `ROFS_*`, the action list's skip rule (**T01-152**; with T01-103's puppet pages).
2. **Foe manager numbers**: the search and tracking periods, `CalculateSleepTime` constants, the line-of-sight request, the sorting key, the world info "may be foes" test, `en_lafLastAssignedFoe`, the `FH_NEAR` interval, the auto-lose period use (**T01-156**; sound and field-of-view rules stay with T01-129).
3. **Think period**: the formula and its factors are now in [`behavior-action-choice.md`](behavior-action-choice.md) section 7 (T01-153, **medium**); what remains for **T01-156**: the immediate-think delay, the range of the puppet's `en_fRnd`, and the sleep and search periods of the foe manager; T01-126 for the brain side.
4. **Boarding and leaving rides** (T01-128): seat choice and the way to a seat; this page only gives the states.
5. **Which of the two obsolete classes the shipped mods use**: none appears in the data, so no file can need them; whether a mod's level could create one is untested (**T01-159**).

## Test vectors

- **Hostility ranking** (7.3): a brain with no foe is damaged: `OnBrainDamaged` calls `AssignFoe(damager, 4, -1)` (period -1 = never lose) and, when the world info entity allows, the damager becomes the foe. While the foe has hostility 5 and a candidate with hostility 7 is offered, the candidate replaces it; with hostility 3 offered against a foe of 4, it does not.
- **Foe loss** (7.3): tracking state, hostility 2, foe not seen for 60 s: `FoeLost`, state looking (1). Same with hostility 8: the foe is kept.
- **Obsolete**: creating a `CTurretBehaviorEntity` prints `Class (CTurretBehaviorEntity) is obsolete and should be removed from level ASAP.`
- **Scheduling**: the first act of a `CGoToPointEntity` happens 1 x 10^-5 s after creation (the next instant), not inside `OnCreate`.
- **Enumeration values**: `FH_FORCED` 6, `FH_AUTO_CHANGED_FOE` 9, `FMS_TRACKING_CURRENT_FOE` 2, `PRS_WAIT_RIDE_IDLE` 3, `ROFS_KICK` 5, `CMR_NEED_NAVIGATION` 2.
- `python -I tools/re/behavior_classes_verify.py "<install>"` prints (section 1) that none of the 36 classes of this page is in the data and (section 5) the three hostility values 0, 6 and 9 of `CBAForcePlayerFoe`.
