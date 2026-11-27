# The generic behaviours on the puppet side (T01-152, follow-up of T01-105)

- **Roadmap task:** T01-152 (follow-up of [`behavior-entity-classes.md`](behavior-entity-classes.md) open question 1 and [`behavior-action-classes.md`](behavior-action-classes.md) open question 4)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the data counts and values (`tools/re/behavior_generic_verify.py`: the 13 run-over records, the 10 run-over actions, the 7 local-move actions, the 82 foe-change chances) and for the constants read from the binary (checked as exact floats); **medium** for every rule read from `Sam2Game.dll` (each function was decompiled and read once; the calls whose arguments the decompiler lost were checked on the instruction level where noted; **nothing was run**); **medium-low** where marked.

Prerequisites: [`behavior-entity-classes.md`](behavior-entity-classes.md) (the behaviour entities, section 3), [`behavior-action-classes.md`](behavior-action-classes.md) (the actions that spawn them), [`brain-classes.md`](brain-classes.md) (states, `StartGenericBehavior`), [`puppet-entity-classes.md`](puppet-entity-classes.md) (the 16 puppet states, section 4), [`puppet-attacks.md`](puppet-attacks.md) (shooting chain, melee blasts), [`puppet-legged-sequences.md`](puppet-legged-sequences.md) (the leap), [`behavior-action-choice.md`](behavior-action-choice.md) (who starts an action).

## Summary

A `.cb` action that "takes time" does not act itself: it builds a **generic behaviour** (`behavior-entity-classes.md` section 3) that asks the **puppet** to do one scripted thing and waits for the puppet (or for a timer) to say it is done. This page gives, for each kind, what the puppet is asked, which puppet state runs, how the end is reported and what is cleaned up; the numbers of the **local move** and of the **run-over-foe stages** are in sections 4 and 5.

Findings an implementer needs first:

1. **One end-of-action chain for all kinds (medium).** The puppet tells the brain `OnActionFinished` (shooting, melee, animation, local move, leap) or the behaviour entity ends itself (wait, run-over, go-to-point). The brain passes the report **down** to the deepest running behaviour, which finishes (`BehaviorFinished`); the finish goes **up** (`OnChildBehaviorFinished` of an action list, else the brain's `OnGenericActionBehaviorFinished`), which **ends the puppet's behaviour (`DoEndGenericBehavior`, which sets the puppet default state one instant later), restarts the action timer and re-enters the brain's default state** (section 1). This corrects `behavior-entity-classes.md` section 2, which had the report travelling up.
2. **The action's own numbers for `CBARunOverFoe` are not used (medium, data high).** `Execute` reads only the id of the action and takes **every number from the puppet's `CRunOverFoeAttack` record** (run time drawn between the record's minimum and maximum, run-through distance, kick distance and animation, speed multiplier, wait). The three numbers stored in the action (`rof_fRunOverFoeTimeMin/Max`, `rof_fRunTroughDistance`) differ from the record's in all 10 stored actions and are dead data (section 5.1).
3. **The run-over stages** are a poll loop with a 0.1 s period: turn until the puppet can thrust towards the foe, approach until the foe is nearer than `rof_fRunTroughDistance` (or at once when that is -1), run for the drawn time towards a point straight ahead (or at the foe's position when the distance is -1), kick once if the foe is within the kick distance and within 45 degrees, keep running to the end of the time, turn again, finish (section 5).
4. **The local move** takes a direction (towards the foe, else the puppet's own facing) turned by `balm_fHeading` degrees, scales it by the forward ratio (clamped to `balm_fVelocityRatioClamp`, 1.0 in all 7 stored actions: two stored forward ratios, 1.5 and 2.0, are clamped to 1), and walks towards a goal point at `nominal speed x step time` along it with the puppet's speed multiplier set to the vector length (section 4).
5. **`CBACrouch` calls the puppet's `SetDesiredTempo` with the vector (0, -1, 0)** (medium): it stores that desired velocity and switches the move type to "desired tempo and look"; what the legged movement step makes of a downward desired velocity (the crouch pose) is the movement spec, T01-141 (section 6).
6. **After an attack behaviour (shoot, melee, leap, run-over) the brain may change foe with the probability `fad_fAutoChangeFoeChance`** (not after animation, local move or wait). In the data 58 of 82 behaviours have 0, 4 have 0.25 to 1, and 20 have 1.5 to 10 (**always**) (section 7).
7. **The action-list skip rule** (`SBAList`): the list runs its entries in order; entries with a failing term are skipped; a non-latent entry that started is followed at once by the next; a **latent** entry that started stops the walk until its child behaviour finishes; the end of the list restarts the action timer and finishes the list behaviour (section 3).

## Evidence

- **Code (medium).** `CBAAnimation`, `CBAMelee`, `CBAShoot`, `CBALocalMove`, `CBARunOverFoe`, `CBACrouch` `Execute`; the `OnCreate` of the animation, melee, shoot, local-move, wait, leap and run-over behaviours; the leap behaviour's `OnBoot`, `RotateTowardsFoe`, `LeapOntoFoe`; all 14 methods of the run-over behaviour; `CGBActionListBehaviorEntity::StartNextAction`; the brain's `StartGenericBehavior`, `OnActionFinished`, `OnGenericActionBehaviorFinished`, `OnBehaviorFinished`; the behaviour base's `OnActionFinished`, `BehaviorFinished`, `OnThisBehaviorFinished`, `OnDelete`; the puppet's `DoGenericAnimation`, `GenericActionPlayAnim`, `DoLocalMove` (base and legged), `OnLocalMoveStarted`, `BeforeLocalMove`, `DuringLocalMove`, `AfterLocalMove`, `OnLocalMoveFinished` (base and legged), `DoGenericMeleeAttack`, `SetState`, `SetDesiredTempo`, `StopMoving`, `GetLookTargetAngle`, `GetFrontDirection`, `SetRunTroughPoint`, `SetGoalPoint`, `CanSeeFoe`, `CanAddHorizontalThrustInDirection` (outline). Virtual slots were named from the virtual tables of the brain, the puppet and the behaviours; the shared "return a constant" and "tell the brain" bodies carry the name of whichever sibling the decompiler met first, so a body is quoted by what it does. The arguments of the `SetState` calls of the no-foe branches (the value 0) were checked on the instruction level.
- **Constants.** Read as exact floats from the data section: 0.1 s poll, 1e-5 s scheduling delay, 0.2 s kick lerp, 0.01, 0.001, 0.99 (double), 90 and 45 degrees as 1.5708 and 0.7854 radians, 100.0, the degree-to-radian factor.
- **Data (high).** `python -I tools/re/behavior_generic_verify.py "<install>"` (read-only, under a second).
- **Not checked.** Nothing was run. `CanAddHorizontalThrustInDirection` and `GetLookTargetAngle` were read only in outline (movement step: T01-141); the instruction-level order of the quaternion product in the local move; the player-side `DoAttack` (T01-264).

## Layout / rules

### 1. From action to puppet and back (medium)

**Start (`StartGenericBehavior(behaviour, moveDuring)`).** The action built the behaviour entity from a properties object and spawned it; the entity's `OnCreate` asks the puppet (section 2) and stores whether it accepted (`en_bCanPerformAction`). The action's `Execute` then calls the brain:

1. If the new behaviour is itself an **action list** (the `SBAList` action): it becomes the brain's behaviour and its first entry is started (`StartNextAction`); nothing else changes.
2. Otherwise: the brain state is set to `BS_DEFAULT` (0, which runs the exit hook of the old state); **if `moveDuring` is false the "may move" execution ability (value 4) is cleared, which stops the puppet** (`brain-classes.md` 4.4); if the brain's current behaviour is an action list the new behaviour becomes its **child**, else it replaces the brain's behaviour; finally the brain state becomes `BS_EXECUTING_GENERIC_ACTION` (4), whose entry hook clears the "AI moments may be handled" ability (value 1) and whose exit hook restores all abilities.

`moveDuring` per action: `CBAAnimation` and `CBAWait` pass their `..MoveDuring..` flag (animation: 110 of 112 stored actions 0, 2 are 1; wait: 11 of 14 are 0, 3 are 1), all other kinds pass **false**.

**End.**

1. The puppet reports (section 2 table) with the brain's **`OnActionFinished`**. The brain gives the report to its current behaviour, whose base `OnActionFinished` hands it to its **child** if it has one (so the report reaches the deepest behaviour) and otherwise calls its own `OnActionFinishedInternal`; for the generic kinds that function is the shared one that calls `BehaviorFinished`. The brain then, in brain state `BS_EXITING_FROM_RIDE` (8), `BS_KNOCK_OUT` (11, abilities := all first), `BS_EXECUTE_GENERIC_ATTACK` (12) or `BS_EXECUTING_GENERIC_ACTION` (4), requests `ReconsiderBehavior` again unless the current behaviour is an action list (medium-low: the repeat is harmless).
2. `BehaviorFinished`: with a parent behaviour (an action list) it calls the parent's `OnChildBehaviorFinished` (section 3), without one its own `OnThisBehaviorFinished`.
3. The generic base's `OnThisBehaviorFinished` calls the brain's **`OnGenericActionBehaviorFinished`** and then, **only for the four attack kinds**, the foe-change draw of section 7.
4. `OnGenericActionBehaviorFinished`: (a) the puppet's **`DoEndGenericBehavior`**: it schedules the puppet's `SetDefaultState` for the next instant (1e-5 s), which sets the puppet state `PS_DEFAULT` (1) and so runs the exit hook of the state it was in (4 -> local-move cleanup; 5 -> none; 6 -> `OnEndCGS`; 7 -> `OnEndGenericMeleeAttack`); (b) `ResetActionTime` (the think period restarts, `behavior-action-choice.md` 7); (c) the brain's `OnBehaviorFinished`.
5. `OnBehaviorFinished` (brain): in brain state `BS_ENTERING_RIDE` (3) it sets the "entered ride" flag, tells the puppet (a ride call) and clears the behaviour; in `BS_PATROLLING_MARKERS` (5) it advances the patrol and sends the marker macro events (`brain-movement.md`); **in every other state it requests `ReconsiderBehavior`**: the behaviour is cleared and a new think-point oscillator is spawned one instant later (`brain-classes.md` 3.2).
6. A behaviour that is deleted calls `OnDelete`, which deletes its child.

So after a generic action the puppet is in `PS_DEFAULT` after **1e-5 s** plus the delay of the scheduled call, the action timer restarts at the moment of `OnGenericActionBehaviorFinished`, and the next think point comes one think period (section 7 of `behavior-action-choice.md`) after that.

### 2. The seven kinds: request, acceptance, puppet state, end report

| Kind (behaviour) | `OnCreate` accepts when | Request to the puppet | Puppet state and how it ends | Report to the brain |
|---|---|---|---|---|
| Animation | the id, remapped through the model's animation scheme, exists in the model (else console text, rejected) | `DoGenericAnimation(id)` | `PS_GENERIC_ACTION_PLAY_ANIM` (5), section 2.1 | `OnGenericAnimationFinished` after the animation length; the state is **not** left by the puppet |
| Melee | puppet is legged, **`CanSeeFoe(0)` is true**, and `CanPerformMeleeAttack` (state is not 7) | `DoGenericMeleeAttack(id)` | `PS_GENERIC_MELEE_ATTACK` (7): `puppet-attacks.md` 3 and 7 | `OnMeleeAnimFinished`: `OnActionFinished` and `SetState(PS_DEFAULT)` |
| Shoot | the id exists in the puppet's table of shootings; **`CanSeeFoe(0)` is true unless the process has `sp_bAttackTargetIfCantBeSeen`**; `CanPerformGenericShooting` (state is not 6, a rider's seat allows it) | `DoGenericShooting(id, count)`, then `StopMoving` when the action drew the stop flag | state 6, the chain of `puppet-attacks.md` 4 | `OnEndCGS` (exit hook) |
| Leap onto foe | puppet is legged, the brain **has a foe**, **`CanSeeFoe(0)`**, and `CanPerformLeapOntoFoe` | after turning (section 2.2) `DoGenericLeapOntoFoe(id)` | the legged leap states, `puppet-legged-sequences.md` | `GenLeapAttackFinished`: `OnActionFinished` and `SetState(PS_DEFAULT)` |
| Local move | `DoLocalMove(action)` returns true (always, for a legged puppet) | `DoLocalMove(copy of the action)` | `PS_LOCAL_MOVE` (4), section 4 | `AfterLocalMoveFinished`: `OnActionFinished`; the state is left by `DoEndGenericBehavior` |
| Wait | always | none | none (a timer of the behaviour entity, drawn in [min, max] by the action) | the entity finishes itself |
| Run over foe | the attack record exists (else console text `Attack of type run over foe with ID "<id>" can't be found`, nothing spawned) | none to the puppet beyond what section 5 lists | none (the behaviour drives the puppet) | the entity finishes itself |

`CanSeeFoe(0)` is the puppet's line-of-sight request toward its look target (`en_henLookTarget`); with no look target it is false. The rejection texts are written to the brain's reject string only when the debug dump is on. An **animation** is the one kind whose puppet state is never left by the puppet itself, and for **melee and leap** the puppet returns to `PS_DEFAULT` itself before the brain finishes the behaviour (so `DoEndGenericBehavior` sets the same state a second time).

**2.1 Animation (`DoGenericAnimation`, `GenericActionPlayAnim`; medium).** `DoGenericAnimation(id)` is a remote call (`puppet-entity-classes.md` 6): on the host it is sent to every machine; on each machine it stores the id (`en_idGenericAnimToPlay`) and sets `PS_GENERIC_ACTION_PLAY_ANIM`. The state's entry schedules the think routine after 1e-5 s; the routine: **stops moving**, starts a fresh blend state with the lerp time `en_fAnimLerpTime`, stores the id as the current generic animation, plays the animation **once** (play mode 2, speed 1, weight 1, layer 3) and schedules the end routine after **the animation's length** (the length query with the "full" flag). The end routine only reports to the brain (when the puppet has a brain: host only). The state has no exit hook. The puppet therefore stands still (`StopMoving`) for the whole animation even when `baa_bMoveDuringAnimation` is 1 (the 2 stored cases): the flag only keeps the brain's "may move" ability set (**medium-low**: what a moving puppet could do between the stop and the end was not followed).

**2.2 Leap behaviour (medium).** After acceptance `OnBoot` schedules `RotateTowardsFoe` after 1e-5 s. `RotateTowardsFoe` polls every **0.1 s**: no foe (brain, foe manager or foe missing) -> finish the behaviour; else give the puppet a run-through point at the foe's position and the foe as look target, take the unit vector from the puppet to the foe and ask `CanAddHorizontalThrustInDirection`; **true -> `LeapOntoFoe`** = `DoGenericLeapOntoFoe(leap id)`, false -> poll again. There is **no time limit** on the turn (the behaviour waits as long as the puppet cannot thrust towards the foe). The leap itself and its end are `puppet-legged-sequences.md`.

### 3. The action list (`CGBActionListBehaviorEntity`; medium)

`OnCreate` copies the `SBAList` (only when the properties' object is an `SBAList`) and starts at index 0. `StartNextAction` runs this loop: while the index is below the entry count, take the entry and advance the index; if the entry has a validity term and **the term does not hold** (`IsValid` XOR `vt_bNotTrue`, evaluated against the situation record the brain still holds from the moment that chose the list, i.e. **not rebuilt**), skip it; otherwise call its `Execute(brain)`; **if `Execute` returned true and the action is latent** (`IsLatentAction`: animation, shoot, melee, local move, run-over, leap, wait) **stop and return**; in every other case (Execute false, or a non-latent action such as change state, emotion, play sound, throw macro event, or a nested `SBAList`) go on with the next entry at once. When the entries are used up: `ResetActionTime`, then `BehaviorFinished` of the list. `OnChildBehaviorFinished` (a latent action's behaviour finished) calls `StartNextAction` again. The weights of the entries are **not** used (a list is a script, not a draw); the child of a latent entry is installed by `ExecuteChildBehavior`. A nested `SBAList` is not latent, so the outer list does **not** wait for it (medium-low).

### 4. The local move (`SBALocalMove`, `DoLocalMove`; medium)

Members of the action (13, `behavior-action-classes.md` 2.4). The refusal rule (mover reason must be `MR_AREA`) is in `Execute`. `DoLocalMove(action)` (legged override: calls the base and returns whether it returned non-zero; the base returns **true** always):

1. If the puppet is already in `PS_LOCAL_MOVE` (4), set `PS_DEFAULT` first (this runs the exit hook of the previous move). Store a **copy** of the action (`en_pbaLocalMove`).
2. **Base direction `d`:** with a look target, the unit vector from the puppet to the target (3-D, the vertical part included); without one, the puppet's own forward axis (the negative Z axis rotated by its orientation).
3. Heading `H` = atan2(-d.x, -d.z) (radians; 0 is straight along the negative Z axis, **positive turns towards the negative X axis**); pitch `P` = asin(clamp(d.y, -1, 1)); **when |d.y| is above 0.99 the heading is set to 0**. The orientation `q1` has Euler angles (H, P, 0). `q2` has Euler angles (`balm_fHeading` x 0.01745329 [degrees to radians], 0, 0). The combined orientation is `q1` then `q2` (the product order was not checked on the instruction level: **medium-low**; with pitch 0 it is simply heading `H` plus `balm_fHeading`). `f` = the forward axis (negative Z) of the combined orientation, a unit vector.
4. `v = balm_fFwdVelocityRatio x f + balm_fUpVelocityRatio x (0, 1, 0)`; `L = |v|`. **Effective length `m = min(L, balm_fVelocityRatioClamp)`** (a negative forward ratio flips the direction and is **not** clamped to 0: -0.75 walks backwards). If `m` is not above **0.001** the stored vector is **zero**; else it is `v / L x m`. It is stored in the puppet (the "local move vector") and the puppet state becomes `PS_LOCAL_MOVE`.

**State 4 (entry `OnLocalMoveStarted`, then three timed routines).**

| Step | What happens |
|---|---|
| Entry hook (`OnLocalMoveStarted`, legged) | if `balm_bForcePose`: the desired pose `en_mapDesiredPose` := `balm_mapPose`; if `balm_bForceReadiness`: the desired readiness := `balm_marReadiness`; the puppet's look-ahead flag `en_bLookAhead` := `balm_bLookAhead` (always written, 0 or 1); the move abilities are saved (`en_maMoveAbilitiesTemp`); **if `balm_bDisableStrafing` the abilities lose bits 8 and 32** (`MA_CANSTRAFEH` and `MA_CANMOVEBACKWARD`; strafing vertically, 16, stays) |
| `BeforeLocalMove` (1e-5 s after entry) | if `balm_fWaitBeforeMove` > 0: wait that long, then `DuringLocalMove`; else `DuringLocalMove` at once |
| `DuringLocalMove` | if the stored vector's length is **at most 0.01**: `StopMoving`. Else: the puppet's speed multiplier `en_fSpeedMultiplier` := the vector's length; goal point := puppet position + (unit direction of the vector) x **`GetCurrentNominalVelocity()` x `balm_fAutoOffTime`**; `SetGoalPoint(goal)`. In both cases `AfterLocalMove` is scheduled after **`balm_fAutoOffTime`** seconds |
| `AfterLocalMove` | if `balm_fWaitAfterMove` > 0: run `OnLocalMoveFinished` now (cleanup below), then report after that wait; else report at once |
| Report | the shared routine tells the brain `OnActionFinished` (section 1); the state stays 4 until `DoEndGenericBehavior` |
| Exit hook (leaving state 4, also when the brain interrupts) = `OnLocalMoveFinished` | legged: desired pose and readiness := -1 (none), look-ahead flag := 0, move abilities := the saved ones; base: reset the velocity multipliers (`ResetVelocityMultipliers`), `StopMoving`, delete the stored copy of the action |

So the puppet walks **at `|v|` times its nominal speed** towards a point placed at the full-speed distance for the step time: with ratio 0.75 it covers about three quarters of that distance in `balm_fAutoOffTime` seconds and is still on its way when the step time ends; it goes on towards the goal point until the cleanup's `StopMoving` (right after the step when `balm_fWaitAfterMove` is above 0, else when the behaviour has ended and the state is left, one instant after the report). **The direction is fixed at the call**: the target is not tracked during the step. **Data (7 actions, 2 files, script block 2):** heading 90 (forward 0.75 and -0.75: a step to the left or right of the foe direction, step 1.0 s, no pose change), -45 and 45 (forward -0.75), 0 (forward 1.5, clamped to 1.0, step 0.75 s) in `kleerofski.cb`; heading 30 and -30 (forward 1.0 and 2.0, the 2.0 clamped to 1.0), `balm_bForcePose` 1 with pose 6 (`MAP_ROLL`), step 0.6 s, wait after 0.2 s in `scifi_commander.cb`. `balm_fUpVelocityRatio` 0, `balm_fWaitBeforeMove` 0, `balm_bDisableStrafing`, `balm_bForceReadiness`, `balm_bLookAhead` 0 in all 7. So **5 of 7 move at a ratio of 0.75 to 1.0 and 2 are clamped**; the only forced pose is the roll of the commander's two dodges.

### 5. Run over foe

#### 5.1 The action and the record (medium; data high)

`CBARunOverFoe::Execute` looks the attack up by `rof_idRunOverFoe` in the puppet's table of run-over attacks (`CRunOverFoeAttack`, 13 records in 13 `.ep` files). It fills the behaviour's properties with: the attack id; the **run time** = `min + u x (max - min)` with `u` a fresh random number in [0, 1) (`rof_fRunOverFoeTimeMin`/`Max` **of the record**); the record's `rof_fRunTroughDistance`, `rof_fKickAnimDistance`, `rof_idKickAnim`, `rof_fSpeedMultiplier` and `rof_fPostAttackWait`. The three numbers stored **in the action** are not read. Spawn, `StartGenericBehavior(behaviour, false)` and true; if the behaviour did not accept, delete it and return false. The 13 records (script block 1; time min..max, run-through distance, kick distance, kick animation, speed multiplier, post-attack wait):

| File | Id | Speed x | Time (s) | Run-through | Kick distance | Kick anim | Wait (s) | Blasts |
|---|---|---|---|---|---|---|---|---|
| `dragon.ep`, `zumzum.ep` | `FlyOverGrab` | 2.0 | 2..2 | 30 | 30 | `Bite` | 1 | 1 |
| `kleerofski.ep` | `RunOverFoe` | 1.6 | 2.5..2.5 | **-1** | 1000 | `RunOver` | 1 | 0 |
| `transformer.ep` | `RunOverFoe` | 1.5 | 0.8..0.8 | 50 | 1000 | `FlyOver` | 1 | 0 |
| `cerberus.ep` | `JumpAttack` | 1.1 | 1..1 | 4 | 3.5 | `JumpAttack` | 0.5 | 0 |
| `fido.ep` | `JumpAttack` | 1.25 | 1.5..1.5 | 5 | 4.5 | `JumpAttack` | 1 | 0 |
| `footballer.ep` | `RunOverFoe` | 1.25 | 1..1.5 | 5 | 4 | none | 0 | 1 |
| `harpy.ep` | `RunOverFoe` | 1.5 | 3..3.5 | 7 | 6 | `Air_Melee` | 1 | 1 |
| `neanderthal.ep` | `RunOverFoe` | 1.3 | 1..1.25 | 3.25 | 3 | `Body_Primitive_Melee` | 1 | 1 |
| `neanderthal_noshield.ep` | `RunOverFoe` | 2.0 | 0.25..0.75 | 3.25 | 4 | `Body_Primitive_Melee` | 0.5 | 1 |
| `rhino.ep` | `Kick` | 1.0 | 1.5..2.5 | 12 | 3 | none | 0.5 | 0 |
| `hoverfighter.ep`, `hoverfighter_pyramidfight.ep` | `RunOverFoe` | 1.25 | 1..2 | 15 | 3 | none | 0 | 1 |

The action in `.cb` (10 objects, 7 files) holds 1.5..1.5 (2 and 3 for `footballer`, 1.5..3 for `rhino`) and 7 (5 for `footballer`): **none equals the record's value**, which is what the code predicts.

#### 5.2 Entering and leaving (`OnCreate`, `OnBoot`, `OnDelete`; medium)

`OnCreate` copies the properties, sets the stage to `ROFS_ROTATE_TOWARDS_FOE_BEFORE` (1) and **accepts**; it writes into the puppet: the current attack id (`en_idCurrentAttackID`, which `OnAnimEventMeleeDamage` uses to find the blast list: `puppet-attacks.md` 7), the look-ahead flag `en_bLookAhead` := 1, and it **saves the move abilities and removes bits 8 and 32** (no horizontal strafing, no backward movement); a legged puppet's **emotion** is saved and the "emotion restored" flag cleared. `OnBoot` enters stage 1. **`OnDelete` (also on cancel) restores**: current attack id invalid, `en_fSpeedMultiplier` := 1.0, look-ahead flag 0, the saved abilities, and a legged puppet's saved emotion. `IsThisAttackBehavior` is true.

#### 5.3 The stages (`RunOverFoeStates`; `SetState` schedules the stage's method after 1e-5 s; medium)

`F` is the brain's foe (the foe manager's foe). **Poll period 0.1 s** everywhere below. A missing brain, foe manager or foe in stages 1, 3 and 2 sets stage 0.

| Stage | Method | Does | Next |
|---|---|---|---|
| 0 `ROFS_CANCELED` | `BehaviorFinished` | the behaviour ends | - |
| 1 `ROFS_ROTATE_TOWARDS_FOE_BEFORE` | `RotateTowardsFoeBefore` | gives the puppet a **run-through point at `F`'s position** and `F` as look target; asks `CanAddHorizontalThrustInDirection` for the unit vector to `F`; **false: poll again in 0.1 s** | true: stage 3 |
| 3 `ROFS_APPROACH_CLOSE_ENOUGH` | `StartApproaching`, then `ApproachCloseEnough` | `StartApproaching`: `en_fSpeedMultiplier` := the record's speed multiplier; a legged puppet's **emotion := 4 (`CE_CHARGE`)**; then `ApproachCloseEnough`: run-through point at `F`'s position, look target `F`, `d` = 3-D distance, `a` = absolute heading error of the look target (radians, `GetLookTargetAngle`); **while `a` <= 90 degrees and the run-through distance is not -1 and `d` >= the run-through distance: poll again after `0.02 x d` s for `d` below 100 and 2 s from 100 up** (d <= 0: 0 s); otherwise stage 4 | stage 4 |
| 4 `ROFS_RUN_OVER` | `StartRunOverFoe`, then `RunOverFoe` | `StartRunOverFoe`: records the start time. **Run-through distance not -1:** the run-through point is the puppet's position moved **forward by `GetCurrentNominalVelocity() x run time`** along its front direction (horizontal components only; the height is kept); **-1:** the run-through point is `F`'s current position (no foe: `StopMoving`). Then `RunOverFoe` polls: **when `start + run time` has passed**: `StopMoving`, wait `rof_fPostAttackWait` s (when above 0), then stage 2; **otherwise**: `CheckRestoreEmotionState`; if the record has a kick animation: with `F` missing stage 0, else **if the 3-D distance to `F` is below the kick distance and `a` < 45 degrees: stage 5**; poll again in 0.1 s | 5 or 2 |
| 5 `ROFS_KICK` | `OnBeginKick` (called at once by `SetState`), `Kick`, `OnKickFinished` | `OnBeginKick`: the kick animation id is remapped through the model's scheme; if valid: a blend state with lerp 0.2 s and **play mode 0, speed 1, weight 1, layer 3** (the animation call's mode argument is 0 here against 2, "once", for the generic animation and the melee animation: whether mode 0 loops was not resolved). `Kick`: no animation: finish at once; else finish after **the animation length**. `OnKickFinished`: lerp 0.2 s and **remove the animation**. The blast comes from the animation's `Melee blast (id)` event (`puppet-attacks.md` 7, run-over blasts applied there) | stage 6 |
| 6 `ROFS_CONTINUE_RUN_THROUGH` | `ContinueRunThrough` | polls every 0.1 s: `CheckRestoreEmotionState`; **when `start + run time` has passed**: stage 2 | stage 2 |
| 2 `ROFS_ROTATE_TOWARDS_FOE_AFTER` | `RotateTowardsFoeAfter` | as stage 1 (run-through point at `F`, look target `F`, wait until the puppet can thrust towards `F`); then clears the pending call and `BehaviorFinished` | the behaviour ends (`OnDelete` restores) |

`CheckRestoreEmotionState`: once, when the absolute heading error of the look target exceeds **90 degrees** (the foe is behind), a legged puppet's emotion is set back to the saved value and the restored flag is set (so the charge emotion lasts from the approach until the puppet has run past the foe, or until the end). The run time is counted from the start of stage 4 and **includes the kick**: the kick does not extend it. The stage 4 timeout goes to stage 2 even without a kick. **`rof_fSpeedMultiplier` acts through `en_fSpeedMultiplier` from stage 3 on** (what the movement step does with it: T01-141). A stage 2 turn has no time limit.

**Worked example (`transformer.ep`, record speed 1.5, time 0.8, run-through 50, kick distance 1000, kick `FlyOver`):** the foe is 120 m away: stage 3 polls every 2.0 s while the distance is 100 or more, every 1.2 s at 60 m, and at 49 m (below 50) stage 4 starts; the run-through point lies `nominal speed x 0.8` m ahead; because the kick distance is 1000 the kick starts at the first poll after the turn is within 45 degrees (0.1 s later), plays `FlyOver` through its length (removed at the end), then stage 6 polls until 0.8 s after the start of stage 4 (if the kick already took longer, stage 6 ends at its first poll), the puppet waits 1.0 s stopped, turns, and the behaviour ends.

### 6. `CBACrouch` (medium; effect T01-141)

`Execute` takes the brain's puppet and calls `SetDesiredTempo(0, -1, 0)`: the desired velocity member becomes (0, -1, 0) and the move type becomes `MT_DESIRED_TEMPO_LOOK` (2), return true. Nothing else (no state, no timer, no end report; the action is not latent, so a list continues at once). The 4 stored `CBACrouch` actions (3 files, band `Action` lists); the **crouch pose is the movement step's reading of a downward desired velocity** (`puppet-legged-classes.md`, T01-141); until another order the puppet keeps that tempo.

### 7. After an attack behaviour: change foe (medium)

The generic base's `OnThisBehaviorFinished`, **only when the behaviour is an attack kind** (`IsThisAttackBehavior`: shoot, melee, leap, run-over), with a foe manager present: draw `u` in [0, 1); **when `u` < `fad_fAutoChangeFoeChance` of the character behaviour** the foe manager is asked to `TrySwitchingToAnotherFoe` (`behavior-entity-classes.md` 7). The chance is a probability capped at 1 by the comparison: **0 in 58 of 82 files (never)**, 0.25 in 2, 0.5 in 1, 1.0 in 1, 1.5 in 1, 2.0 in 11, 3.0 in 7 and 10.0 in 1 (script block 4): in **21 files an attack is always followed by a foe-change attempt** (the value is at least 1).

## Invariants

1. For the 10 stored `SBARunOverFoe` actions the action's time pair and run-through distance differ from the record's of the same id: 10 of 10 (script).
2. `balm_fVelocityRatioClamp` is 1.0 and `balm_fUpVelocityRatio` 0 in 7 of 7 stored local-move actions; 2 of 7 forward ratios exceed the clamp (script).
3. Every stored run-over record has a speed multiplier of 1 to 2, a time pair with min <= max and a post-attack wait of 0, 0.5 or 1 (13 of 13); `rof_fRunTroughDistance` is -1 in 1 record (`kleerofski`).
4. `CBACrouch` has no member (4 of 4).

## Open questions

1. **The movement step's reading of** the run-through point (`SetRunTroughPoint` against `SetGoalPoint`, their flag members), `en_fSpeedMultiplier`, the desired tempo of `CBACrouch`, and the thrust cone behind `CanAddHorizontalThrustInDirection` (angle limit, the move-type and strafing conditions) and `GetLookTargetAngle`: **T01-141** (legged movement step).
2. **Run-time confirmation** of the approach distances, the kick moment and the end order for `Kleerofski`, `Neanderthal` and `Footballer`; the local-move walk length against the formula: **T01-158**.
3. **The product order of the two quaternions** of `DoLocalMove` (matters only for a pitched base direction, i.e. when the foe is above or below the puppet) and the exact start of the walk of the local move when the previous one was still running: **T01-280**.
4. **Why `CBAAnimation` has a `moveDuring` flag** when the animation stage stops the puppet itself (section 2.1), and whether a brain state change during an animation can move it: **T01-280**.
5. **The player's own attack entry** (`DoAttack`): T01-264.

## Test vectors

- **Local move** (4): puppet at the origin facing the negative Z axis, foe at (0, 0, -10), action heading 90, forward 0.75, up 0, clamp 1, step 1.0, nominal speed 4: base direction (0, 0, -1); with heading 90 the direction is (-1, 0, 0); `v` = (-0.75, 0, 0), `m` = 0.75; speed multiplier 0.75; goal point (-4, 0, 0) (4 x 1.0 from the origin); the report comes 1.0 s after the move began (plus 0.2 s when `balm_fWaitAfterMove` is 0.2). With forward -0.75 the direction is (+1, 0, 0). With forward 1.5 and clamp 1: `L` = 1.5, `m` = 1.0.
- **Local move, no look target**: the base direction is the puppet's own forward axis.
- **Run over, no run-through** (5.3, record run-through -1, `kleerofski.ep`): stage 3 does not poll; stage 4 sets the run-through point to the foe's current position; the run time is 2.5 s.
- **Run over, kick gate**: distance 2.9, heading error 40 degrees, kick distance 3, record with kick animation: stage 5; heading error 50 degrees: keep running; distance 3.1: keep running.
- **Approach delay**: `d` = 30 m: 0.6 s; `d` = 100 m or more: 2.0 s; `d` = 0: 0 s.
- **Action list** (3): list `[Change state (non-latent), Animation (latent), Shoot (latent)]`: the state change runs and the animation starts in one call; the list waits; when the animation behaviour finishes the shot starts; when the shot finishes the timer restarts and the list finishes.
- **Foe change** (7): behaviour with chance 0.25, an attack just ended: 25 % chance of `TrySwitchingToAnotherFoe`; chance 0: never; chance 2.0: always; no draw after an animation behaviour.
- **Crouch** (6): `SetDesiredTempo(0, -1, 0)` leaves the move type at 2 and the desired velocity (0, -1, 0).
- `tools/re/behavior_generic_verify.py` prints: 13 run-over records, 10 `SBARunOverFoe` actions, 7 `SBALocalMove` (`{'not clamped': 5, 'clamped': 2}`), `CBACrouch` 4, `CBAWait` 14, `SBAAnimation` 112 (`moveDuring=0` 110), `SBALeapOntoFoe` 4, foe-change chances `{0: 58, 0.25: 2, 0.5: 1, 1: 1, 1.5: 1, 2: 11, 3: 7, 10: 1}`.
