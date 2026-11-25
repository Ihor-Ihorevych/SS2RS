# Legged puppets: `CLeggedPuppetEntity` and the walking kinds (T01-103, part of T01-026)

- **Roadmap task:** T01-103 (overview [`puppet-classes.md`](puppet-classes.md); base class [`puppet-entity-classes.md`](puppet-entity-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class, member and method names, the member list and order of `SLeggedPuppetEntity` (61), `SLeggedPuppetParams` (5), the root-class counts of the parameter files and the data-side counts (`tools/re/puppet_classes_verify.py`); **medium** for the rules of sections 3 to 6 (each is one reading of a short function); **low** for "(name)" descriptions and the open questions. Nothing was run.

Prerequisites: [`puppet-classes.md`](puppet-classes.md) (enumerations: `EPuppetPose`, `LeggedPuppetState`, `MovingAnimation*`, `CharacterEmotion`), [`puppet-entity-classes.md`](puppet-entity-classes.md), `enemies.md` (what the parameter values are), `ep-schema.md` (`SLeggedPuppetParams`, `SLeggedPuppetBodyParams`, `SMovingAbilities`, `SKickingAbilities`, `SKnockOutParams`), `ans.md` and `mdl.md` (the moving-animation scheme), `ai.md`.

## Summary

`CLeggedPuppetEntity` (data `SLeggedPuppetEntity`, parent `CPuppetEntity`) is **every walking character**: all enemies, animals, NPCs and bosses on foot, the player (through `CPlayerPuppetEntity`, T01-102), riders. The placed object is its empty child **`CLeggedCharacterEntity`** (`SLeggedCharacterEntity`, 1,825 placements in 44 worlds). It adds to the base class **121 named methods** (445 virtual slots: 56 overridden, 51 new) and **61 members** (2 replicated: `en_ppPuppetPose`, `en_ceCharacterEmotion`) for: the **pose** (`EPuppetPose`: stand, crouch, dive, swim, fly, fall, roll, knock-out, riding wheel), the **three environment records** (land, air, water: the movement numbers), **jumping and air control**, **diving and drowning**, the **moving-animation chooser**, **knock-out** with a ragdoll, the **generic leap attack**, the **ride-and-rider sequences** and the **spine adjuster** that bends the body towards the look direction.

Findings:

1. **The three environment records are land, air and water** (`SLeggedPuppetParams.lpp_pepLand`, `lpp_pepAir`, `lpp_pepWater`; the order rule of `puppet-classes.md` section 2 names the three numbered members `.1`, `.2`, `.3`). `GetEnvironmentParams` selects: **air for the pose `PP_FLY`, water for `PP_DIVE` and `PP_SWIM`, land for every other pose** (read in full, **medium**). This settles the "probably land, air and water" of `enemies.md`: the seven numbers of the land record (8.5 / 9.5 / 5 for the Kleer) are the speeds in the land pose, and a character swimming uses the third record.
2. **Which kinds exist as classes: seven that are dead** (`CKleerPuppetEntity`, `CFlyingKleerPuppetEntity`, `CWereBullPuppetEntity`, `CZombieSoldierPuppetEntity`, `CSciFiSoldierPuppetEntity`, `CNPCPuppetEntity`, `CManifestedSpawnerPuppetEntity`: `puppet-classes.md` section 3.4) **and two generic ones**. The data's "kinds" (Kleer, Albino, zombie, NPC ...) are `SLeggedPuppetParams` files: 86 files carry the class's type table, of which **81 have it as the root class** (`Cactus.ep` is one of them), 1 is the generic file `Animals/Worm.ep` (`SGenericLeggedPuppetParams`), 2 are the player's (`SPlayerPuppetParams`: `Player/Player.ep` and the cut-scene `NPCs_cutsequences/Santa.ep`) and 2 are `EntityParams/PlayerPuppet.ep` and `PlayerPuppet_DM.ep` (outside `Databases/Puppets`); all legged characters are placed as `SLeggedCharacterEntity` (which accepts any file derived from `SLeggedPuppetParams`).
3. **Knock-out** is a hit with an impulse above a per-file threshold: `CheckImpulseForKnockOut` (read): the file must allow knock-out (`kop_bCanBeInKO`, 4 of the enemy files, `enemies.md`) and the impulse vector (not zero) longer than the file's impulse threshold calls `KnockOut`; `StartKO` ignores a request when the puppet is already knocked out or when less than 0.25 s passed since the last (the clock counts 2^32 ticks per second; the product of the constants 2^32 and 0.25 appears in the function: **medium**). The pose becomes `PP_KO`, the model switches to the **ragdoll mechanism** (`Ragdoll`, else `RagdollSimple`), and `OnSetKOPose` blends the pose back into an animation with `CLerpToAnimAdjuster` when the knock-out ends.
4. **Emotion and shield** are script-set (`macSetEmotionState`: `Frightened` 0, `Brave` 2, `Enrage` 3, `Charge` 4, anything else or no argument 1 `CE_NORMAL`; `macSetShieldState`: `Shielded` -> 1, else 0) and select the animation (`CMovingAnimData` has them as selectors). The emotion member is replicated.

## Evidence

- **Members.** The declared table of `SLeggedPuppetEntity` (61 members) equals the data's 61 members in order, with 12 numbered members named (script section 2: same count, 0 differing names, 0 type mismatches); the table of `SLeggedPuppetParams` (5) and the three records are declared only as members of other tables.
- **Virtual table.** `CLeggedPuppetEntity` 445 slots against 394: 56 overridden, 51 new; its children (the seven kinds, `CGenericLeggedPuppetEntity`, `CLeggedCharacterEntity`) differ from it in 2 to 4 slots.
- **Methods.** All 121 listed; 24 decompiled and read (`R`), 46 from strings and callees (`P`), 51 from the name (`N`); the 14-kilobyte `LerpToDesiredVelocityAndLook` was only profiled (its strings are the debug member names).
- **Data.** 44 worlds define `SLeggedPuppetEntity`; 124 parameter files under `Databases/Puppets`, 84 of them with a legged root or a class derived from it (81 + 1 + 2).
- **Not checked:** nothing was run; the movement step, the animation chooser, the leap and the ride sequence were not read.

## Layout / rules

### 1. Classes

| Class | Parent | Data name (files) | Notes |
|---|---|---|---|
| `CLeggedPuppetEntity` | `CPuppetEntity` | `SLeggedPuppetEntity` (44), 61 members | 121 methods, section 2 |
| `CLeggedCharacterEntity` | `CLeggedPuppetEntity` | `SLeggedCharacterEntity` (44), 0 members | the placed object; `IsGenericPuppet` true |
| `CLeggedPuppetProperties`, `CLeggedCharacterProperties` | `CPuppetProperties` / `CLeggedPuppetProperties` | `SLeggedPuppetProperties`, `SLeggedCharacterProperties` (44 each), 0 members | empty |
| `CLeggedPuppetParams` | `CPuppetParams` | `SLeggedPuppetParams` (86), 5 members | `.1` land, `.2` air, `.3` water (`SPuppetEnvironmentParams`), `lpp_lpbBody` (`SLeggedPuppetBodyParams`), `lpp_kopKnockOutParams` (`SKnockOutParams`) |
| `CLeggedPuppetBodyParams` | - | `SLeggedPuppetBodyParams` (86), 11 members | below |
| `CGenericLeggedPuppetEntity` / `Properties` / `Params` | `CLeggedPuppetEntity` / `CLeggedPuppetProperties`... / `CLeggedPuppetParams` | params `SGenericLeggedPuppetParams` (1 file: `Worm.ep`); entity and properties binary-only | the model is a property |
| seven obsolete kinds | `CLeggedPuppetEntity` | none | `OnCreate` prints the obsolete-class error |
| `CMovingAnimData`, `CAllMovingAnimation`, `CBodyMovingAnimation`, `CPoseTransitionAnimation`, `CMovingAnimationScheme` | - | in the data (44 `.wld`; 48, 17, 25, 60 model files) | the moving-animation chooser |
| `CSpineAdjuster`, `CLerpToAnimAdjuster` | `CMotionHandler` / - | `CSpineAdjuster` (44 `.wld`); the second binary-only | pose adjusters |

`SLeggedPuppetBodyParams` (11 members, 86 files): `lpbp_fMaxValidSlopeForJump` (60 or 70, degrees), `lpbp_fJumpUpVelocity` (10, 11 or 15), `lpbp_fMaxLookAroundAngleH` and `...P` (0 to 180), `lpbp_psdPuppetSpineData` (a link to the spine data: 80 files have one, 6 none), `lpbp_fLookRotationOverride` (-1 or 10,000), `lpbp_fRenderableRotationOverride` (-1), and the view heights `pbp_fViewHeightCrouch` (0.7), `pbp_fViewHeightSwim` (0.4 or 1.5), `pbp_fViewHeightDive` (0 or 1.1) and the breath `pbp_fKeepBreathPeriod` (5 or 60 s). `GetMaxJumpSlopeAngle` converts `lpbp_fMaxValidSlopeForJump` to radians when the parameter file is a legged one (read).

### 2. Members of `SLeggedPuppetEntity` (61, 2 replicated `*`)

| Group | Members | Meaning | Confidence |
|---|---|---|---|
| Pose and body direction | `en_ppPuppetPose`\*, `en_ppOldPuppetPose`, `en_fBodyEulH`, `en_vFrontEul`, `en_vRelLookEul`, `en_bForcingDesiredFront`, `en_vForcedDesiredFrontEul`, `en_bAddingThrust`, `en_mdMoveDirection`, `en_fOldDesTempoToDesLook`, `en_iOldMoveDirection` | the current and previous `EPuppetPose`, the body heading, the front direction and the look direction relative to it (Euler angles), forced front direction, whether thrust is added, the move direction (`EMovingDirection`) and the values of the last step | medium |
| Spine and pose blend | `en_hsaSpineAdjuster`, `en_hltaLerpToAnim` | handles to the spine adjuster and to the knock-out blend helper | medium |
| Jump and air control | `en_bJumpButtonReleased`, `en_bInTheAirAfterJump`, `en_tmLastJump`, `en_bUsesAirControl`, `en_tmLastAirControlDesired`, `en_fAirControlTime`, `en_fAirControlFactor`, `en_fAirControlVelocityMultiplier` | jump button edge, in-air flag, time of the last jump, and the air-control window (`GetAirControlFactor`, `GetAirControlPeriod`, `GetAirControlVelocityMultiplier` read: three 7-byte getters of these members) | medium |
| Fluid | `en_fFluidRatio`, `en_fFluidDensity`, `en_tmDived`, `en_fLastFluidRatio` | how deep in a fluid, its density, when the dive began (for the breath and `InhaleEasy`/`Medium`/`Hard`) | medium |
| Moving animation | `en_mapDesiredPose`, `en_marDesiredReadiness`, `en_bAutoAnimations`, `en_madLast`, `en_mad`, `en_fLastTestedMoveSpeed`, `en_idPoseChangeAllAnim`, `en_idPoseChangeBodyAnim`, `en_idAllAnim`, `en_idBodyAnim`, `en_bLastReverse`, `en_fLastSeenNominalVelocity`, `en_fLastSeenMoveSpeed`, `en_tmPoseChangeEnd`, `en_bLastPlayedMoveAnim` | the desired animation pose and readiness, the last and current selector records (`CMovingAnimData`), the animations in use, the speeds they were chosen for (`PlayMoveAnim` re-chooses when speed or nominal velocity changed) | medium |
| Ride and rider sequence | `en_rrsRideAndRiderSequence`, `en_tmStartEnteringIntoRide`, `en_tmStartExitingFromRide`, `en_qvStartEnteringIntoRideLook`, `en_qvStartExitingFromRideLook`, `en_bLerpRiderPlacement`, `en_tmRiderAnimStart`, `en_qvRiderStart`, `en_fRiderAnimLength`, `en_iCurrRideAndRiderAnimPair` | entering or leaving a ride, with start times, looks and the rider's blend (`SRideAndRiderAnim` pairs) | medium |
| Emotion and shield | `en_cssShieldStatus`, `en_ceCharacterEmotion`\*, `en_tmResetShieldState`, `en_tmResetEmotionState` | `CharacterShieldStatus`, `CharacterEmotion` (replicated) and the times they are reset | medium |
| Knock-out | `en_tmLastKO`, `en_bAddKOForceImpulse`, `en_lpsState` | time of the last knock-out, the pending impulse flag and the `LeggedPuppetState` | medium |
| Leap attack | `en_bGenLeapShouldRunAway`, `en_bGenLeapGroundTouched`, `en_tmLeapAttackStarted`, `en_tmRotateTowardsFoeStart` | the generic leap's bookkeeping | low |

The numbered members the data stores (`.50` front direction, `.51` relative look, `.52` move direction, `.70`, `.71`, `.72` spine adjuster, `.80` pose, `.81`, `.82`, `.83` fluid ratio, `.106` last jump, `.110` auto animations) are named by position in the table of `puppet-classes.md` section 4.

### 3. Environments and poses (`GetEnvironmentParams`; **medium**)

`EPuppetPose` 1 stand, 2 crouch, 3 dive, 4 swim, 5 fly, 6 fall, 7 roll, 8 knock-out, 9 riding wheel. The record used: pose 5 -> air; poses 3 and 4 -> water; all others (stand, crouch, fall, roll, knock-out, riding wheel, invalid) -> land. `GetDeclaredNominalVelocity`, `GetCurrentAcceleration`, `GetCurrentDeceleration` and `CanRotateInPlace` read the chosen record (name and profile). Diving: `GetDipContent` answers land, air or water from the fluid ratio; entering water plays `DiveIn` and `Diving`, leaving plays `DiveOut` and one of `InhaleEasy`, `InhaleMedium`, `InhaleHard` by how long the breath was held (`pbp_fKeepBreathPeriod`), and while diving too long `OnStep` plays the `Drown` effect and sound (names from the code).

### 4. Methods

The 121 methods are in the table at the end of this page (`Basis`: `R` read, `P` profile, `N` name).

### 5. Knock-out, leap and rides (**medium** / **low** where stated)

- **Knock-out** (section Summary 3). `KnockOut` runs on the host with direction and force, then `DoKnockOut` (a remote call) sets the state on every machine; `OnBeginKnockOutState` switches to the ragdoll mechanism; `OnSetKOPose`/`OnKOFinished`/`OnEndKnockOutState` end it and re-activate the spine adjuster. `macKnockOutAbs` and `macKnockOutRel` are the script functions (absolute or relative direction). The knock-out parameters (`SKnockOutParams`: can be in KO, impulse threshold 500 to 10,000,000, sleep time 1 or 2 s) are in `enemies.md`; the sleep time is a minimum duration of the sequence: **specified in [`puppet-legged-sequences.md`](puppet-legged-sequences.md)** (T01-147).
- **Generic leap attack** (`CanPerformLeapOntoFoe` true, `DoGenericLeapOntoFoe`, `GenericLeapAttackState` and the sub-states `GenLeapStartRotate` -> `GenLeapRotating` -> (`GenLeapRunAway` -> `GenLeapAfterRunAway`) -> `GenLeapWaitAfter` -> `GenLeapAttackFinished`; names read from the strings of each routine): turn towards the foe, leap, on landing optionally run away, wait, finish. The leap damage and range are in the `CLeapAttack` records (`enemies.md`: 9 leaps). The order and the numbers are in [`puppet-legged-sequences.md`](puppet-legged-sequences.md) section 2 (T01-147).
- **Rides** (`EnterIntoRide`, `ExitFromRide`, `AttachToSeat`, `DetachFromSeat`, `PlayRideAndRiderSequenceState`, `LerpRiderOntoSeat`): a rider walks to the seat, an `SRideAndRiderAnim` pair plays (rider animation and ride animation, `rra_bLerpRiderPlacement`, `rra_bPlayReversed`), the rider's placement is blended onto the seat attachment and the state `LPS_PLAYING_RIDE_AND_RIDER_SEQUENCE` ends with `RiderLerpingFinished`. **Low**: only the names and the profile were read (T01-144).

## Invariants

1. The 61 members of `SLeggedPuppetEntity` in the data are the declared members in order (44 files, script).
2. The three environment members are `lpp_pepLand`, `lpp_pepAir`, `lpp_pepWater` in this order (86 files, script; the data stores them as `.1`, `.2`, `.3`).
3. Every placed legged character is an `SLeggedCharacterEntity` with 0 members of its own (44 of 44 worlds, script).
4. Environment selection by pose (1 function, **medium**).

## Open questions

1. **The movement step** `LerpToDesiredVelocityAndLook` (14 KB): blending, cones, pose choice, air control, jump and fall: T01-141.
2. **The moving-animation chooser** (`PlayMoveAnim`, `PlayPoseChangedAnim`, `PrepareMovingAnimData`, how `CMovingAnimData` selects a `CAllMovingAnimation` of the model's scheme): **specified in [`puppet-moving-anim.md`](puppet-moving-anim.md)** (T01-148; follow-ups T01-265, T01-266).
3. **Knock-out, leap and ride sequences**: **specified in [`puppet-legged-sequences.md`](puppet-legged-sequences.md)** (T01-147); the numbers of the unnamed `SPuppetEnvironmentParams` members: T01-214 (the slow limit and the move-speed pair are in `puppet-moving-anim.md`).
4. **Diving and drowning**: breath numbers, damage per second, the `Drown` trigger: T01-149.
5. **Run-time checks** of the walking characters on the running original (speeds per pose, knock-out timings): T01-146.

## Test vectors

- Pose to record: `PP_STAND` (1) -> land; `PP_SWIM` (4) -> water; `PP_FLY` (5) -> air; `PP_KO` (8) -> land.
- Emotion script argument: `"Brave"` -> 2, `"Charge"` -> 4, `"Frightened"` -> 0, `"Enrage"` -> 3, `""` or `"xyz"` -> 1; shield: `"Shielded"` -> 1, anything else -> 0.
- `GetQuadrant` of a relative angle a (radians): `|a| < 1.0472` (60 degrees) -> 0; the two other bands (up to 2.0944 = 120 degrees, and beyond) return values that depend on two flag bits of the puppet (not named in the data; **low**).
- Knock-out gate: second `StartKO` 0.2 s after the first -> ignored; 0.3 s after -> accepted (when not already in KO).
- Data: `SLeggedPuppetEntity` 61 members in 44 worlds; `SLeggedPuppetParams` 86 files.

## The 121 methods of `CLeggedPuppetEntity`

Columns: method; what it does; **Basis** (`R` the function was decompiled and read, `P` strings and callees of the function were listed, `N` the name only).

### Life cycle, parameters and mechanism

| Method | What it does | Basis |
|---|---|---|
| `OnCreate` | Runs the base `OnCreate`, then creates the spine adjuster (`CreateSpineAdjuster`) and attaches it to the model as a motion handler. | R |
| `ResetDynamicMembers` | Runs the base reset and clears the legged members (pose, air control, knock-out and sequence state). | P |
| `OnStep` | The base step plus the legged part: diving and drowning checks (the sound `Drown` and the drown effect), fluid ratio, air control and the spine adjustment. | P |
| `GetPuppetParamsClass` | Answers `CLeggedPuppetParams` (the generic legged kind answers the same). | P |
| `SetDefaultMechanism` | Remembers the default mechanism (name). | N |
| `SetCurrentMechanism` | Runs the base switch, then finds the root joint and sets up the pose mechanisms. | P |
| `SetupAspects` | Runs the base setup and requires the root joint (error `Legged puppet model (...) mechanism (...) root joint doesn't exist`). | P |
| `GetEnvironmentParams` | Chooses which of the three environment records of the parameters is used by the pose: **land** for every pose except fly, swim and dive; **air** for `PP_FLY`; **water** for `PP_DIVE` and `PP_SWIM` (section 3). | R |
| `ShouldEvaluateAnimation` | Runs the base test (name). | P |
| `GetDefaultSittingMechanism` | The mechanism a rider uses on a seat (`Default`). | N |
| `GetRagDollMechanism` | Chooses the ragdoll mechanism: `Ragdoll` when the model has it and the developer switch is on, else `RagdollSimple` when the model has it, else none. | R |
| `OnAnimEventRagdoll` | Animation event: turns the corpse into a ragdoll (`corpse` collision). | P |
| `OnAnimEventStep` | Animation event: a footstep; finds the attachment named by the event (error `Character "..." doesn't have attachment ... that should produce step`) and spawns the step effect for the floor material under it. | P |

### Movement, jumping, diving and air control

| Method | What it does | Basis |
|---|---|---|
| `LerpToDesiredVelocityAndLook` | The legged movement step, 14 kilobytes of code: blends the real velocity towards the desired one, the body heading towards the look direction (cone limits `fAllowedConeH`, `fAllowedConeP`), picks the move direction and pose, adds air control and fills the debug record (`en_vFrontEul`, `en_vRelLookEul`, `bDesiresMoving`, `b3DJoint`, `EMovingDirection`, `EPuppetPose`). The rules are not specified here (follow-up task). | P |
| `GetCurrentAcceleration` | The acceleration limit of the pose's environment record (name; the numbers are in the environment parameters). | N |
| `GetCurrentDeceleration` | The deceleration limit of the pose's environment record (name). | N |
| `GetDeclaredNominalVelocity` | The nominal speed of the pose's environment record. | N |
| `GetDesiredMovingSpeed` | The desired speed from the base rule, limited by the pose (name). | P |
| `GetLookRotationVelocity` | The turn speed of the look direction (from the base nominal velocity). | P |
| `GetMaxJumpSlopeAngle` | The steepest slope a jump may start from: the body parameters' `lpbp_fMaxValidSlopeForJump` in radians (degrees times 0.017453) when the parameter file is a legged one, else the base value. | R |
| `GetFrontDirEul` | The front direction of the body. | N |
| `GetFrontDirection` | The front direction as a vector. | N |
| `GetLookDirEul` | The look direction as Euler angles (body heading plus the relative look). | N |
| `GetCurrentLookDirEul` | The same value (two exported names). | N |
| `SetLookDirEul` | Sets the relative look direction. | N |
| `GetDipContent` | The medium the puppet is in, from the fluid ratio (land, air or water). | N |
| `GetOperatorMoveDir` | The operator's movement direction in the body's frame. | N |
| `ProcessOperatorInput` | The base input handling, then the legged jump and crouch buttons. | P |
| `MovingIn3DArea` | True while the pose is fly or swim (name). | N |
| `CanRotateInPlace` | Whether the puppet may turn on the spot (the environment parameters decide). | R |
| `DoJump` | Starts a jump with a velocity, records the time and the direction (replicated through the base call). | R |
| `AutoJump` | Starts a jump over an obstacle or gap by itself (profile: 1,286 bytes, uses the jump parameters). | N |
| `FreezeVelocity` | Holds the velocity (for scripted moves; name). | N |
| `GetAirControlFactor` | The air control factor (the parameter file). | R |
| `GetAirControlPeriod` | The time air control stays active. | R |
| `GetAirControlVelocityMultiplier` | The speed multiplier while air control is active. | R |
| `ResetAirControl` | Clears air control. | N |
| `GetJumpMultiplier` | The jump height multiplier (name). | R |
| `GetMoveDirectionFromRelDir` | Turns a relative direction into `MD_FORWARD`, `MD_LEFT`, `MD_RIGHT`, `MD_BACKWARD` or `MD_STOP`. | N |
| `GetQuadrant` | Classifies an angle into -1, 0, 1 or 2 with the thresholds 60 and 120 degrees (1.0472 and 2.0944 rad) and the left or right flag of the puppet. | R |
| `GetQuadrantName` | The name of a quadrant (`Forward`, `Right`, ...) for animation names. | P |
| `LerpRelativeLook` | Blends the relative look direction. | N |
| `RepositionPuppet` | The base reposition plus the legged pose fix (name). | P |
| `OnStartRiding` | The base start plus the pose change to riding. | P |
| `OnFluidTouched` | Hook: the body touched a fluid. | N |
| `OnStartedDiving` | Starts diving (sounds `DiveIn`, `Diving`). | P |
| `OnStoppedDiving` | Stops diving (sounds `DiveOut`, `InhaleEasy`, `InhaleMedium`, `InhaleHard` by how long the breath was held). | P |
| `OnHullTouchedInternal` | The legged collision part; ground touch during a leap ends the leap (`OnGenGroundTouched`). | P |
| `PrepareToDisableStep` | Called before the step hook is switched off (name). | N |
| `DoLocalMove` | The legged local move (calls the base). | P |
| `OnLocalMoveStarted` | Hook: a local move started (plays the move animation). | P |
| `OnLocalMoveFinished` | Hook: it ended. | P |
| `AdjustClientOrientation` | Corrects the heading of a client copy. | N |
| `StartMovingSound` | Starts the moving sound (calls the base). | P |
| `StartNotMovingSound` | Starts the not-moving sound (calls the base). | P |
| `GetPuppetMovingSoundRatio` | The moving-sound strength (calls the base). | P |

### Moving animation and poses

| Method | What it does | Basis |
|---|---|---|
| `PlayDefaultAnimation` | Plays the default pose animation. | N |
| `AutoAnimationOn` | Turns automatic move animations on. | N |
| `AutoAnimationOff` | Turns them off. | R |
| `PlayMoveAnim` | Chooses and plays the moving animation from the `CMovingAnimData` (pose, speed, direction, readiness, shield, emotion) against the model's moving-animation scheme (debug text `Desired moving speed = ..., animation's nominal velocity = ...`, `Found "all" anim`, `MAS 3D`). | P |
| `PlayPoseChangedAnim` | Plays the transition animation between two poses (`CPoseTransitionAnimation`; debug `Pose change`, ` (reverse)`). | P |
| `PrepareMovingAnimData` | Fills the current moving-animation data. | N |
| `DiscardMovingAnimationData` | Forgets it. | N |
| `GetMovingAnimPose` | The moving-animation pose of the current pose. | N |
| `ConvertMAPToPP` | Converts a moving-animation pose (`MAP_*`) to a puppet pose (`PP_*`). | N |
| `CustomAnimPlayedNotification` | Hook: a custom animation was played. | R |
| `PlayDeathSound` | The base death sound plus a legged one. | P |
| `DoDeathSequence` | Starts the death animation or ragdoll. | N |

### Knock-out, emotion and shield

| Method | What it does | Basis |
|---|---|---|
| `IsInKO` | True when the legged state is knock-out and the knock-out helper handle is empty. | R |
| `CheckImpulseForKnockOut` | A hit with an impulse: if the parameter file allows knock-out, the impulse vector is not zero and its length exceeds the file's knock-out impulse threshold (`kop_` members, `enemies.md`), calls `KnockOut`. | R |
| `RecivesKOForceForceImpulse` | Whether the puppet can be knocked out by an impulse (both spellings are the binary's). | R |
| `PrepareForKOForceImpulse` | The think routine that waits one step before the knock-out impulse is added. | P |
| `AddKOForceImpulse` | Adds the knock-out impulse to the ragdoll. | P |
| `StartKO` | Starts the knock-out unless the puppet is already in one or less than 0.25 s has passed since the last one (the simulation clock counts 2^32 ticks per second: the constants 4.29 x 10^9 and 0.25; medium). | R |
| `KnockOut` | Host-side knock-out with a direction and force (debug `Knock out dir`, `Force impulse:`, sound `KnockOut`). | P |
| `DoKnockOut` | The remote call that starts the knock-out on every machine. | R |
| `KnockOutAbsDir` | Knocks out along an absolute direction. | N |
| `macKnockOutAbs` | Script: `KnockOutAbs`. | N |
| `macKnockOutRel` | Script: knocks out along a direction relative to the puppet. | N |
| `OnBeginKnockOutState` | Enters the knock-out legged state (switches to the ragdoll mechanism). | N |
| `OnSetKOPose` | Sets the knock-out pose with the lerp-to-animation helper (`CLerpToAnimAdjuster`), then sends `OnKOFinished`. | P |
| `OnKOFinished` | Hook: the knock-out animation ended. | R |
| `OnEndKnockOutState` | Leaves the state (re-activates the spine adjuster). | P |
| `OnKicked` | Hook: the puppet was kicked (can start a knock-out). | N |
| `SetLeggedPuppetState` | Changes the legged state (`LPS_*`, section 3 of this page) and schedules the matching think routine (`GenericLeapAttackState`, `PlayRideAndRiderSequenceState`, `PrepareForKOForceImpulse`). | P |
| `SetState` | The base state change. | P |
| `macSetEmotionState` | Script: sets the character emotion by name (`Frightened` 0, `Brave` 2, `Enrage` 3, `Charge` 4, anything else or no argument: `CE_NORMAL` 1). | R |
| `macSetShieldState` | Script: sets the shield status (`Shielded` gives 1, anything else 0). | R |

### Leap attack

| Method | What it does | Basis |
|---|---|---|
| `CanPerformLeapOntoFoe` | True: legged puppets can leap. | N |
| `DoGenericLeapOntoFoe` | The remote call that starts a leap attack. | N |
| `OnBeginGenericLeapAttack` | Hook: it began. | N |
| `GenericLeapAttackState` | The think routine of the leap (error `Leap attack with ID ... doesn't exist in character`). | P |
| `OnEndGenericLeapAttack` | Hook: it ended. | N |
| `GenLeapStartRotate` | The leap sub-state that first turns towards the foe. | P |
| `GenLeapRotating` | The sub-state while turning (then waits, `GenLeapWaitAfter`). | P |
| `GenLeapWaitAfter` | The sub-state that waits after the leap. | P |
| `GenLeapRunAway` | The sub-state that runs away after the leap. | P |
| `GenLeapAfterRunAway` | The sub-state after the run-away. | P |
| `GenLeapAttackFinished` | Ends the leap attack. | N |
| `OnGenGroundTouched` | Hook: the ground was touched during the leap (continues with `GenLeapRunAway`). | P |
| `OnGenLeapPuppetHit` | Hook: a puppet was hit by the leap (empty). | R |

### Rides and the rider sequence

| Method | What it does | Basis |
|---|---|---|
| `AttachToSeat` | Attaches the rider to the seat and sets the riding pose. | N |
| `DetachFromSeat` | Detaches it. | N |
| `EnterIntoRide` | Starts entering (plays the way-to-seat sequence). | N |
| `ExitFromRide` | Starts leaving. | N |
| `FallOutFromTheRide` | Falls out (calls the base). | R |
| `DoFallOutFromRide` | The legged fall-out: ragdoll or knock-out. | P |
| `IsRideFixed` | Whether a ride is fixed (name). | R |
| `PlayEnteringIntoRideSequence` | Plays the enter sequence of the chosen way to the seat. | N |
| `PlayExitingFromRideSequence` | Plays the leave sequence. | N |
| `OnBeginRideAndRiderSequence` | Starts a ride-and-rider sequence (`SRideAndRiderAnim` pairs). | N |
| `PlayRideAndRiderSequenceState` | The think routine that plays the pairs one after the other (event `RiderLerpingFinished`). | P |
| `OnRideAndRiderSequenceFinished` | Hook: the sequence ended. | N |
| `RiderLerpingFinished` | The rider reached the seat placement. | N |
| `LerpRiderOntoSeat` | Blends the rider's placement onto the seat (3 kilobytes of code). | N |
| `LerpToDesiredLookRider` | Turns the rider's head and spine to the look direction (uses the spine adjuster). | P |
| `GetRideEnteringCameraPlacement` | The camera while entering. | N |
| `GetRideExitingCameraPlacement` | The camera while leaving. | N |
| `GetRelativeViewOrigin` | The eye position relative to the body, by pose. | P |
| `GetRelativeViewOriginForPose` | The same for a given pose (uses the view heights `pbp_fViewHeightCrouch`, `...Swim`, `...Dive`). | N |

### Helpers

| Method | What it does | Basis |
|---|---|---|
| `CreateSpineAdjuster` | Creates the `CSpineAdjuster` with its bone list from the spine data of the parameters. | P |
