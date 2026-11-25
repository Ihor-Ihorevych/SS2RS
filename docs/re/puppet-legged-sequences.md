# Legged puppets: knock-out, leap attack and ride-and-rider sequences (T01-147)

- **Roadmap task:** T01-147 (follow-up of [`puppet-legged-classes.md`](puppet-legged-classes.md) section 5; the records are in `ep-schema.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the counts and value tables (reproduced by `tools/re/legged_ko_leap_ride_verify.py` over all 322 `.ep` files); **medium** for the rules read from `Sam2Game.dll` (one reading of each function); **medium-low / low** where marked. **Nothing was run on the original game.**

Prerequisites: [`puppet-legged-classes.md`](puppet-legged-classes.md), [`puppet-entity-classes.md`](puppet-entity-classes.md) (the 16 puppet states, section 4), [`puppet-attacks.md`](puppet-attacks.md) (`DoAttack`, the melee damage record), [`puppet-moving-anim.md`](puppet-moving-anim.md) (pose-change animations, groups), [`brain-classes.md`](brain-classes.md) (`KnockOut`, `OnActionFinished`).

## Summary

`CLeggedPuppetEntity` has a second small state machine next to the puppet state: **`LeggedPuppetState`** (`LPS_NONE` 0, `LPS_KNOCK_OUT` 1, `LPS_PLAYING_RIDE_AND_RIDER_SEQUENCE` 2, `LPS_GENERIC_LEAP_ATTACK` 3, member `en_lpsState`). Changing it (`SetLeggedPuppetState`) runs the exit hook of the old value, forces the **puppet state to `PS_NONE`** (when the new value is not 0), stores the value, runs the entry hook and installs a think routine **1e-5 s later**:

| New value | Entry hook | Think routine | Exit hook |
|---|---|---|---|
| 1 knock-out | `OnBeginKnockOutState` | `PrepareForKOForceImpulse` | `OnEndKnockOutState` |
| 2 ride sequence | `OnBeginRideAndRiderSequence` | `PlayRideAndRiderSequenceState` | `OnRideAndRiderSequenceFinished` |
| 3 leap | `OnBeginGenericLeapAttack` | `GenericLeapAttackState` | `OnEndGenericLeapAttack` |
| 0 | none | none | (the exit hook of the old value) |

**The legged override of `SetState` calls `SetLeggedPuppetState(0)` before the base `SetState`**: every change of the puppet state ends a running knock-out, ride sequence or leap (this is how all three sequences end: they call `SetState(PS_DEFAULT)`). **Medium** (read in full: the three functions `SetLeggedPuppetState`, `SetState`, `IsInKO`).

- **Knock-out** (section 1): sleep time = a **minimum duration** of the knock-out (the larger of the get-down animation and `kop_fKOSleep`, only for a living puppet); 10 of 82 legged parameter files can be knocked out.
- **Leap** (section 2): the leap is **one launch of the desired tempo plus an animation**, then a chain of waits; its numbers are the 9 `CLeapAttack` records.
- **Ride-and-rider sequence** (section 3): per pair of `SRideAndRiderAnim` the rider and the ride play animations while the rider's placement is **interpolated onto the seat** over the length of the rider animation.

## Evidence

- **Binary:** `Sam2Game.dll` of the install used for the other specs. Read: `SetLeggedPuppetState`, the legged `SetState`, `IsInKO`, `CheckImpulseForKnockOut`, `StartKO`, `KnockOut`, `KnockOutAbsDir`, `macKnockOutAbs`, `macKnockOutRel`, `DoKnockOut`, `DoDeathSequence`, `OnBeginKnockOutState`, `PrepareForKOForceImpulse`, `AddKOForceImpulse`, `OnSetKOPose`, `OnKOFinished`, `OnEndKnockOutState`, `OnAnimEventRagdoll`; `CanPerformLeapOntoFoe`, `DoGenericLeapOntoFoe`, `OnBeginGenericLeapAttack`, `GenericLeapAttackState` (the tail was read in the instruction listing for the arguments), `OnGenGroundTouched`, `GenLeapRunAway`, `GenLeapAfterRunAway`, `GenLeapStartRotate`, `GenLeapRotating` (listing for the end), `GenLeapWaitAfter`, `GenLeapAttackFinished`, `OnEndGenericLeapAttack`, `OnGenLeapPuppetHit`, the leap part of `OnHullTouchedInternal`; `PlayEnteringIntoRideSequence`, `PlayRideAndRiderSequenceState`, `OnBeginRideAndRiderSequence`, `RiderLerpingFinished`, `OnRideAndRiderSequenceFinished`, `LerpRiderOntoSeat`, `AttachToSeat`, `DetachFromSeat`, `DoFallOutFromRide`, `EnterIntoRide`, `ExitFromRide`. All writers and callers of the sequence members and of the virtual calls named here were found by scanning the whole DLL.
- **Data:** `tools/re/legged_ko_leap_ride_verify.py` reads the `.ep` files and prints the tables below and the formulas for the test vectors.
- **Not checked:** nothing was run; the movement step's handling of the three states (T01-141); the seat selection and the ride's own functions (T01-144); the damage the leap does to the victim (T01-140); the second half of the physics of the ragdoll (`CMechanism`).

Member names follow `puppet-legged-classes.md` section 2; the members of the leap and the ride sequence were matched to the code by role and by the order of the declared list (**medium**).

## 1. Knock-out

**Parameters** (`SKnockOutParams`, member `lpp_kopKnockOutParams` of 82 legged parameter files; **high**): `kop_bCanBeInKO` 1 in 10 files, 0 in 72; `kop_fKOKickImpulse` (the impulse threshold; all files: 10,000 x35, 1,200 x21, 100,000 x9, 800 x6, 2,000 x6, 1,500 x2, 500, 1,000,000, 10,000,000); `kop_fKOSleep` 2.0 (53 files), 1.0 (29). The 10 files that can be knocked out: the two chickens (impulse 800, sleep 1.0), the lizard (10,000, 2.0), the zombie agent (500, 2.0), the bride soldier (10,000, 2.0), the simba (1,200, 2.0), the two gnaar cut-scene characters (800, 1.0) and the two dinosaur mounts (100,000, 2.0).

**Triggers** (**medium**):

- `CheckImpulseForKnockOut(impulse vector, flag)`: only for a legged parameter file with `kop_bCanBeInKO`; **the vector must be non-zero and its length greater than `kop_fKOKickImpulse`**; then `StartKO(flag)`. Called by `OnKicked` and by the legged `OnStep`.
- `KnockOutAbsDir(direction, force)` (script `macKnockOutAbs(heading, pitch, force)` and `macKnockOutRel`): **adds `direction x force` to the stored KO impulse vector**, sets the "KO impulse time" to now and calls `StartKO(1)`. `macKnockOutAbs` turns heading `H` and pitch `P` (degrees) into the direction `(-cos P sin H, sin P, -cos P cos H)`. `macKnockOutRel` builds the same local vector and **multiplies it by the minus of the puppet's rotation** (the sign of the result is reversed against the absolute function: **medium-low**, the sign was read from the signs of the matrix terms).
- `DoDeathSequence` (a death): `StopMoving`, `StartKO(1)`: **every legged puppet that dies goes through the knock-out sequence** (without the sleep wait, below).

**`StartKO(addForce)`** (read): **ignored when less than 0.25 s have passed since the last knock-out start** (`en_tmLastKO`, the product of 2^32 and 0.25) **or when `IsInKO` is true** (`IsInKO`: the legged state is `LPS_KNOCK_OUT` **and** the get-up blend helper does not exist yet). Otherwise: `DropObject` (the carried object is dropped), **if the puppet rides: `DetachFromSeat` at its current placement and `UnregisterFromRide`**, `en_tmLastKO` := now, `en_bAddKOForceImpulse` := `addForce`, **`DoKnockOut(addForce, impulse vector)`** and the brain's **`KnockOut`** (brain state `BS_KNOCK_OUT`). `DoKnockOut` is a remote call (host sends, every machine runs): it stores the flag, the vector and the time and calls `SetLeggedPuppetState(LPS_KNOCK_OUT)`.

**The sequence:**

1. `OnBeginKnockOutState`: **execution abilities := none** (0) and two calls on the puppet's body objects with 0.0 and 1.0 (not resolved).
2. `PrepareForKOForceImpulse` -> think `AddKOForceImpulse` -> **applies the stored impulse vector to the body** (`CBody::AddImpulse` at the body's own position) **when `en_bAddKOForceImpulse` is set** and a virtual question (always yes here) allows; then think `KnockOut` after 1e-5 s.
3. `KnockOut()`: a living puppet plays the model's scheme sound **`KnockOut`**; the impulse direction becomes `en_mdMoveDirection` (via `GetMoveDirectionFromRelDir`); **`PlayPoseChangedAnim(from = the old pose, to = PP_KO)`** is called (the chooser of `puppet-moving-anim.md` section 4: the pose-transition record "any to knock-out" of the model's scheme: 34 records in the data) and the pose becomes `PP_KO` (8). **Without such a transition the console prints "Character ... doesn't have "Any-To-Knock Out" pose transition animation in moving animation scheme. It should play it during knock out sequence and set ragdoll mechanism trough animation event." and `OnSetKOPose` runs at once.** Otherwise **the wait is `d = length of the transition animation`, and for a living puppet with legged parameters `d = max(d, kop_fKOSleep)`** (a dead puppet waits only the animation); think `OnSetKOPose` after `d`.
4. **The ragdoll** starts from the animation event `Ragdoll` of that animation (`OnAnimEventRagdoll`): the mechanism named by `GetRagDollMechanism` (`Ragdoll`, else `RagdollSimple`) becomes the current mechanism with the collision category kept for a living puppet and **`corpse` for a dead one**, and **the old mechanism velocity is carried over**. Without the event there is no ragdoll and step 5 finishes at once.
5. `OnSetKOPose`: **a dead puppet only calls `SetState`** (it stays a corpse: the death path, `puppet-entity-classes.md` section 8); a living puppet whose current mechanism is **not** the ragdoll one calls `OnKOFinished` (default state) at once. Otherwise the **get-up**: a `CLerpToAnimAdjuster` is created (fade-in 1.0 s, records the ragdoll bone positions and the old origin), the spine adjuster is deactivated, **the puppet is stood up at the ragdoll body's position facing the horizontal direction of the body's forward axis (reversed when the body lies upside down; the move direction value is 4 normally and 2 when upside down: medium-low)**, the default mechanism is restored, the pose becomes `PP_STAND`, `PlayPoseChangedAnim(0, PP_KO, PP_STAND)` plays the get-up, and **think `OnKOFinished` after `max(1.0, length of that animation - 0.2)` seconds (1.0 s without an animation)**.
6. `OnKOFinished` = `SetState(PS_DEFAULT)`, whose legged override ends the state: `OnEndKnockOutState`: abilities := all, **the stored KO impulse vector is cleared**, the blend helper deleted, and for a living puppet the brain is told `OnActionFinished`, the default mechanism is set, the spine adjuster re-activated.

**A knock-out lasts at least `max(transition length, kop_fKOSleep)` plus the get-up `max(1.0, get-up length - 0.2)`** for a living puppet.

## 2. The generic leap attack

**Records** (`CLeapAttack`, 9 in 8 files; `la_pfPlayerFire` 1 in all; **high**): `la_fForwardVelocityMultiplier` 1.5 (7), 2.0 (cerberus), 4.0 (spider); `la_fUpVelocityImpulse` 0.5 (5), 1.0 (3), 0.4 (kleer); `la_bLoopAnimation` 0 in all; `la_slInflictedDamage` 20 (4), 15 (3), 30 (2); damage type 4 (`DMT_PUNCH`) in all; launch heading/pitch (kick direction): 0/0 in 6, 30/15 for the two kung-fu zombie leaps, 0/20 for the lizard; `la_fFoeKickImpulse` 0 (6), 2,000 (2), 1,500 (lizard); `la_fRunAwayTime` 1.0 (5), 0.5 (4); `la_bRotateTowardsFoeDuringWait` 1 in the 2 kung-fu zombie leaps; before-rotate animation and probability: none stored in any; after-rotate animation: `AfterLeapAttack` (probability 0.5, `MaiGeri` zombie leap) and `LeapAttack_Finish` (1.0) in the two kung-fu zombie leaps, none in the other 7; `la_fWaitAfterAttackTime` 0 (6), 0.5 (3: kleer and the two kung-fu zombie leaps). Leap names (`la_idName`): `JumpAttack` (cerberus, fido, lizard), `Jump` (kleer, kung-fu zombie), `JumpMaiGeri`, `RunningKick`, `Leap` (2).

**Starting** (**medium**): `CanPerformLeapOntoFoe` is true unless the legged state is already the leap. `DoGenericLeapOntoFoe(id)` (remote call) stores the id as the current attack and calls `SetLeggedPuppetState(LPS_GENERIC_LEAP_ATTACK)`. A player's `DoAttack` reaches it with the third attack list (`puppet-attacks.md` section 3). If the leap record is missing, `GenericLeapAttackState` prints "Leap attack with ID ... doesn't exist in character ...", tells the brain `OnActionFinished` and goes to `PS_DEFAULT`.

**Entry (`OnBeginGenericLeapAttack`):** `AutoAnimationOff` (the moving-animation chooser stops, `puppet-moving-anim.md`), **the turning-velocity multiplier `en_fTurningVelocityMultiplier` := 0** (the puppet cannot turn), "ground touched" := false, a puppet flag := 1 (the member that the movement step also reads; not resolved), execution-ability bit 4 cleared, **`en_bGenLeapShouldRunAway` := true**, `en_tmLeapAttackStarted` := now. Exit (`OnEndGenericLeapAttack`): `AutoAnimationOn`, the multiplier := 1.0, the flag := 0, ability bit 4 set.

**The jump (`GenericLeapAttackState`):**

1. The "damage applied" flag := false.
2. **Direction** `d` = the unit vector from the puppet to its target (absolute positions, 3-D); **without a target: the puppet's front direction**.
3. **Tempo** `T = (d.x x m, d.y x m + u, d.z x m)` where `m` = `la_fForwardVelocityMultiplier` and `u` = `la_fUpVelocityImpulse` (the up impulse is **added** to the already scaled y). **The look** is the heading and pitch of `T` (`pitch = asin(T.y / |T|)`; `heading = atan2(-Tx, -Tz)`, 0 when `|T.y / |T||` is above 0.99). `SetDesiredTempoAndLookDir(T, look)` is called: the leap is **a desired tempo vector (a ratio of the nominal velocity, `puppet-entity-classes.md` section 5) with an upward component**; the puppet's movement step performs the jump (T01-141).
4. A new cloned animation state with the lerp time **0.2 s**; the animation `la_idAnimation` is remapped through the model's animation scheme and **played with mode 1 (loop) when `la_bLoopAnimation` else mode 0 (once), speed 1, strength 1, group 3**; a body-related call on one of the puppet's objects wakes it (unresolved).
5. The routine sets **no think point**: the next steps are driven by touches.

**Touches (`OnHullTouchedInternal`, leap part; the legged state must be the leap and the record found):**

- The touched entity is a **puppet** (`CPuppetEntity`-derived): if it is **the puppet's current target** then `en_bGenLeapShouldRunAway` := false. **On the host, if "damage applied" is false: set it to true** (**one hit per leap**) and deliver a damage record to the victim: damage `la_slInflictedDamage`, type `la_dtDamageType`, the attacker, the touch point and normal of the hull contact, the hull, and **an impulse = `Rq x (-cos P sin H, sin P, -cos P cos H) x la_fFoeKickImpulse`** (`H` = `la_fLaunchHeading`, `P` = `la_fLaunchPitch`, `Rq` the puppet's absolute orientation: the same construction as the melee blast, `puppet-attacks.md` section 7); the impulse is also applied to the victim as a manual force impulse (`AddForceImpulseManual`) before `OnReceiveDamage`. (Whether the victim takes it twice is T01-261.)
- The touched entity is **not** a puppet and a puppet touch value (`< 0.8`; a member of the touch group, **not resolved**) holds: **`OnGenGroundTouched`**: the first time and only when **at least 0.5 s** have passed since the leap began (`en_tmLeapAttackStarted`), "ground touched" := true and **think `GenLeapRunAway` after 1e-5 s**.

**After landing (all think routines; the min delay is 1e-5 s):**

| Routine | Rule |
|---|---|
| `GenLeapRunAway` | `AutoAnimationOn` and a second call (not resolved); **if `en_bGenLeapShouldRunAway` and `la_fRunAwayTime` > 0: the desired tempo := the unit front direction** (the puppet runs on forward at the nominal speed) and think `GenLeapAfterRunAway` after **`la_fRunAwayTime`**; otherwise think it after the min delay |
| `GenLeapAfterRunAway` | `StopMoving`; **if `la_idBeforeRotateAnimation` is set and a random number in [0, 1) is below `la_fBeforeAnimationProbability`**: play that animation (remapped; mode 2, group 3) and think `GenLeapStartRotate` after `max(length - lerp time, 1e-5)`; else call `GenLeapStartRotate` |
| `GenLeapStartRotate` | **`la_bRotateTowardsFoeDuringWait` set:** `en_fTurningVelocityMultiplier` := 1.0, the puppet flag := 0, `en_tmRotateTowardsFoeStart` := now, think **`GenLeapRotating` after `la_fWaitAfterAttackTime`**; not set: think **`GenLeapWaitAfter` after `la_fWaitAfterAttackTime`** |
| `GenLeapRotating` | no target or **more than 3.0 s since the rotation began**: think `GenLeapAttackFinished` (min delay). Else **if the angle between the front direction and the direction to the target is below 20 degrees (cosine of 0.349066 rad)**: with `la_idAfterRotateAnimation` set and a random number below `la_fAfterAnimationProbability`, play it (remapped, mode 2, group 3, new state with the lerp time) and think `GenLeapAttackFinished` after `max(length, la_fWaitAfterAttackTime)` (at least the min delay); else think `GenLeapWaitAfter`. **Not yet facing:** think `GenLeapRotating` again after the min delay |
| `GenLeapWaitAfter` | `la_fWaitAfterAttackTime` > 0: think `GenLeapAttackFinished` after it; else call it |
| `GenLeapAttackFinished` | tell the brain `OnActionFinished`, then `SetState(PS_DEFAULT)` (which ends the legged state) |

**Medium** for the table (each routine read; the branch targets of `GenLeapRotating` were taken from the listing). **One flag name is a role guess:** the leap "puppet flag" set at the start and cleared at `GenLeapStartRotate`.

## 3. The ride-and-rider sequence

**Data** (**high**): 22 `SPuppetSeatData` in 22 `.ep` files (seat names `Seat`; attachment `Seat` 21, `SeatBig` 1; rider mechanism none 13, `Sitting` 8, `RidingDinoM` 1; seat flags `SeatFlags`: 0 x12, 17 x5 (rider shoots own weapon + player cannot seat), 136 x2, 32, 81, 16; **no seat has `SF_WAY_POINT_SEAT` (2)**; head turn allowed in 8), 22 `SWayToSeat` (enter point attachment `EnterPointLeft` 20, `EnterPointRight`, `EnterPoint`) and **33 `SRideAndRiderAnim` pairs** in 21 files: rider animation `Ride` 32, `FlyPose` 1; **ride animation none in all 33**; `rra_bLerpRiderPlacement` 1 in all 33; `rra_bPlayReversed` 1 in 11. Per way: **one entering pair in all 22 ways; one exiting pair in 11 ways, none in 11** (the exiting pairs are the reversed ones).

**Starting** (**medium**): `PlayEnteringIntoRideSequence(rider-side ride handle, ride, seat id, way)` and `PlayExitingFromRideSequence(...)` (the same function with direction 1) are remote calls: each machine stores the climbing ride, the ride, the seat id and the way index, **direction := 0 (entering) or 1 (exiting)**, the start time and the view placement (the rider's current look: `en_qvStartEnteringIntoRideLook` or `en_qvStartExitingFromRideLook`) and calls `SetLeggedPuppetState(LPS_PLAYING_RIDE_AND_RIDER_SEQUENCE)`. `EnterIntoRide(ride, seat, ...)` and `ExitFromRide` are the script entries: they ask the **brain** (`PossesRide`, `ExitFromRide`).

**Entry** (`OnBeginRideAndRiderSequence`): the pair index := 0 and the seat data found by name; **for an exit from a seat without `SF_WAY_POINT_SEAT` nothing more is done here; otherwise `AttachToSeat(ride, seat)`** (`AttachToSeat`: only if the puppet's mechanism has no parent yet: the **rider mechanism of the seat** becomes the current mechanism, the pose becomes `PP_RIDING_WHEEL` (9) and the mechanism is attached to the ride's mechanism at the seat's ride part).

**`PlayRideAndRiderSequenceState`** (per pair; read):

1. **Abort** (`SetState`) when the climbing ride, the ride or the puppet's mechanism is missing, or the seat data or the way is not found.
2. **The riding state** `en_rsRidingState` := `RS_ENTERING` (2) for direction 0 or `RS_EXITING` (4) for direction 1. **When the way has no pair for this direction** (the list is empty): entering calls `AttachToSeat` at once, exiting calls `DetachFromSeat` at the current placement (and step 3 then ends the sequence).
3. **When the pair index has reached the number of pairs the sequence ends** (`SetState`).
4. The pair is `sequence[index]` of the way's entering or exiting list; **the rider animation** (`rra_idRiderAnim`, if set): a new cloned animation state with the lerp time, **groups 1 and 2 removed**, played (the pair's reverse flag selects mode 4), its length `L` is kept; **the ride animation** (`rra_idRideAnim`, if set) is played the same way **on the ride's model**.
5. The pair's `rra_bLerpRiderPlacement` is stored as the "lerping rider" flag; **if it is set and the current seat is the one of the sequence, the ride is told `OnRiderStartedEntering` or `OnRiderStartedExiting`**; and, **when the flag is set**, the start time, `L` (`en_fRiderAnimLength`) and the rider's current placement (`en_qvRiderStart`) are stored for the interpolation.
6. Think **`RiderLerpingFinished`** (the delay argument was not recovered from the code; it is most probably `L`, because the interpolation reaches t = 1 at `L`: **low**).

**`LerpRiderOntoSeat`** (the rider's placement while the flag is set; read; per step): `t = min((now - start time) / L, 1)` (`L` = the rider animation length, seconds). **Target** = the seat's absolute placement of the ride when entering, **the exit placement of the seat and way when exiting**. **Position = start + (target - start) x t. Orientation: the start quaternion and the target quaternion are interpolated with the angle taken on the shortest arc (the second is negated when the dot product is negative); when the dot product is at least 0.99 a plain linear blend is used, otherwise the spherical blend `sin((1 - t) a) / sin a` and `sin(t a) / sin a`; the result is normalised (identity when degenerate).** A second quaternion (the rider's look at the start, `en_qvStart...Look`) is interpolated onto the seat's look the same way and handed to `SetLookDirEul`; the player's operator receives the same look. **Medium.**

**`RiderLerpingFinished`:** the lerp flag := 0; **when entering** the rider's placement is set to the exact seat placement; **the pair index +1**; `PlayRideAndRiderSequenceState` is called for the next pair.

**Exit hook (`OnRideAndRiderSequenceFinished`):** without a ride: `DetachFromSeat` at the current placement; with a ride: **when exiting or when the seat is a waypoint seat: `DetachFromSeat`**; the brain is told **`OnActionFinished`**; **the riding state := `RS_RIDING` (3) after an entering sequence and `RS_NOT_RIDING` (0) after an exiting one**. `DetachFromSeat(placement)`: delete the current mechanism (restore the default), pose := `PP_FALL` (6), set the placement. `DoFallOutFromRide` (a crash): the base function, the pose := `PP_FALL`, and **the body gets an impulse of 10 x the puppet's up axis** (`AddForceImpulseManual`).

## Invariants

1. `en_lpsState` is 0 whenever the puppet state changes (legged `SetState`); the three sequences end only through a `SetState` call (**medium**).
2. 10 of 82 legged parameter files have `kop_bCanBeInKO`; their `kop_fKOSleep` is 1.0 (4) or 2.0 (6).
3. All 9 leap records have damage type 4, play once (`la_bLoopAnimation` 0), and have `la_pfPlayerFire` 1; none stores a before-rotate animation; 7 of 9 do not rotate towards the foe.
4. Every way to seat has exactly one entering pair; no seat is a waypoint seat; every pair has `rra_bLerpRiderPlacement` set and no ride animation.

## Open questions

1. **The movement step's part in the leap and the knock-out** (the jump itself, the puppet flag set during the leap, the touch value below 0.8, the two unresolved calls on the body objects): T01-141.
2. **The seat choice, the seat flags and the ride's own functions** (`FindBestSeat`, `AutoFixingRideState`, `ProcessRideRiderRelations`): T01-144.
3. **The delay of `RiderLerpingFinished`, the `.sca` terms and the second half of the get-up** (the direction rule and the unresolved bits): **T01-267**.
4. **Run-time check** of the knock-out timings (the 0.25 s gate, the sleep, the get-up wait) and of one leap's phases: **T01-268** (with T01-146).
5. **The damage path of the leap and the kick** (`OnReceiveDamage`, double impulse): T01-140, T01-261.

## Test vectors

All from `tools/re/legged_ko_leap_ride_verify.py`.

- **Knock-out gate:** `kop_fKOKickImpulse` 800 (chicken): a kick impulse vector of length 799 -> nothing; 801 -> `StartKO`. A second `StartKO` 0.2 s after the first -> ignored; 0.3 s after, not in the knock-out state -> accepted.
- **Knock-out duration (living puppet):** transition length 0.8 s, sleep 2.0 -> wait 2.0 s; transition 3.1 s, sleep 2.0 -> 3.1 s; a dead puppet waits the transition only. Get-up: animation 0.9 s -> wait 1.0 s (`max(1.0, 0.7)`); 2.5 s -> 2.3 s.
- **`macKnockOutAbs(heading, pitch, force)` direction:** (0, 0) -> (0, 0, -1); (90, 0) -> (-1, 0, 0); (0, 45) -> (0, 0.7071, -0.7071); times `force`, added to the stored impulse vector.
- **Leap tempo (`m`, `u`, vector to the foe):** (1.5, 0.4, (0, 0, -10)) -> tempo (0, 0.4, -1.5), look heading 0, pitch 14.93 degrees; (2.0, 0.5, (3, 0, -4)) -> tempo (1.2, 0.5, -1.6), heading -36.87, pitch 14.04; (4.0, 1.0, (0, 2, -8)) -> tempo (0, 1.97, -3.881), pitch 26.92 (spider).
- **Leap phases for the kleer** (`la_fRunAwayTime` 0.5, no rotate, `la_fWaitAfterAttackTime` 0.5): jump at t = 0; the first ground touch at or after t = 0.5 s starts `GenLeapRunAway`; the puppet runs forward for 0.5 s (if it did not hit its target); `GenLeapAfterRunAway` and `GenLeapStartRotate` follow with the 1e-5 s delays, `GenLeapStartRotate` waits 0.5 s, then `GenLeapWaitAfter` waits 0.5 s and `GenLeapAttackFinished` ends the state: about **touch + 1.5 s** (touch + 1.0 s if the leap hit its target, which skips the run).
- **Ride lerp factor** `t = min(elapsed / L, 1)`: elapsed 0 -> 0; elapsed 1.0 with `L` 2.0 -> 0.5; elapsed 2.5 -> 1.
- **Riding state after the sequence:** entering -> 3 (`RS_RIDING`); exiting -> 0 (`RS_NOT_RIDING`).
