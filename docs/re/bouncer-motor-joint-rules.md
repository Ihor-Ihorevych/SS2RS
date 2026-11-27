# Bouncers, barrier fields, generic motors, external joints and the diamond hand: the rules (T01-168)

- **Roadmap task:** T01-168 (open question 2 of [`mechanism-classes-movers.md`](mechanism-classes-movers.md); T01-109 listed the classes, members and counts, this page gives the rules)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the member and slot facts that two readings agree on (the template layouts, the command-to-joint mapping, the barrier collision categories, the bouncer's call arguments, the opened-ratio formula of the diamond hand); **medium** for rules that rest on one reading of the original code (the bouncer formula, the air-control rule, the motor row semantics, the break test, the diamond hand's events and beam growth); **low** for the three things marked low (the NaN record in section 2, the navigation flag in section 3, the break test in 5.3). Nothing was run on the original game; no number below was measured at run time.

Prerequisites: [`mechanism-classes-movers.md`](mechanism-classes-movers.md) (the classes, members and data counts), [`puppet-legged-classes.md`](puppet-legged-classes.md), [`puppet-record-members.md`](puppet-record-members.md) (the legged step's force members), `physics.md`, `wld-physics.md`.

## Summary

1. **Bouncer.** A touching entity's velocity `v` is split along the unit direction `d` into `vn` (the part along `d`) and `vt = v - vn`; the new velocity is `vt * ParallelMultiplier - vn * NormalMultiplier + BounceSpeed * d`, capped at `MaxExitSpeed`, and handed to the toucher's `DoJump` together with the air-control time, factor, velocity multiplier and a flag 1 that is stored in the puppet's `en_bDisableFallingDamageOnNextTouch` (**which no code in the game DLL reads**: the member is written and cleared only). All 71 placed bouncers except one have both multipliers 0, so the exit velocity is exactly `BounceSpeed * d`.
2. **Air control** (legged puppets). `DoJump` stores the three air-control numbers and the time of the launch; every step in the air the puppet's acceleration and braking force limits are multiplied by the **air-control factor** while a time window of **air-control time** seconds is open (and the puppet has been asking for movement for more than 0.2 s after leaving the floor), and its wished moving speed is multiplied by the **velocity multiplier** (section 2).
3. **Barrier field.** `Enable` and `Disable` are two virtual functions: enabling gives the hull the collision category `barrier` (`custom_barrier` for the 14 fields with the custom flag), disabling gives it `ignoreall` (section 3).
4. **Generic motor.** Each of the five axes (X, Y, Z, heading H, pitch P) has three numbers: a **force limit** (the axis is a motor only while it is above 0), a **target velocity** and an optional **target position**; with a target position the velocity number is the **maximum approach speed**. Angles are degrees in the script commands and radians inside; positions and velocities are the game's length unit (section 4).
5. **External joint.** The joint templates have **no parameters** except the hinge's limits (degrees); the ball and fixed templates only create the joint. `ep_fBreakForce` is a **force** (-1 = never breaks); a joint that the engine reports as overloaded is **unlinked** and the entity raises the macro event `Broken` (section 5).
6. **Diamond hand.** The hand opens as its health falls: the **desired opened ratio is `1 - health / 500`** (clamped to 0 to 1), the current ratio follows it at **0.5 per second**. Events: `Activated` when the ratio reaches 1, `Deactivated` when it leaves 1, and `HalfOpened` (raised at once and then every 15 s by a repeating timer function `OpenedHandOscilator`) while the ratio is at least 0.5. `ChainPositionType` is `CPT_FAKE` 0, `CPT_FIRST` 1, `CPT_LINK` 2, `CPT_LAST` 3 (section 6).

## Evidence

- **Reading.** Read in full (original game DLLs, the same build as the other specs; nothing run): `CBouncerEntity::OnHullTouched`, `CBaseEntity::DoJump`, the legged `DoJump`, `GetAirControlVelocityMultiplier`, `GetDesiredMovingSpeed`, the air-control part of the legged `LerpToDesiredVelocityAndLook`, `CBarrierFieldEntity::Enable`, `Disable`, `SetupDefaultCollision`, `CancelCollision` and the virtual table, the setters of `CGenericMotorEntity`, the engine's `CGenericMotorJoint` (`SetValues`, the per-axis setters, `GetJik`'s tail) and `CJointLimit` (`AddLinearLimit`, the constructor), the four joint templates' `CreateJoint`, `CExternalJointEntity::OnCreate`, `OnJointOverload` and the end of `LinkTargets`, the engine function that tests the break force, and `CDiamondHandEntity::OnStep`, `OnReceiveDamage`, `GetHealthRatio`, `OpenedHandOscilator`, `SetupOpenedAnimationRatio`, `Disable`. The decisive float decisions (the opened ratio formula and its two thresholds, the strings of the events, the bounce speed unit constants) were checked in the instruction listing.
- **Names.** Template member names come from the engine DLL's own strings (`jt_fLo_H` and the like, section 4.3); the enumeration `ChainPositionType` from its name strings (the declaration order is the reverse of the string pool order, the same rule as in `puppet-record-members.md`).
- **Data.** The counts quoted from `mechanism-classes-movers.md` (71 bouncers, 825 barrier fields, 43 motors, 93 external joints, 14 diamond hands) are those of `tools/re/mechanism_classes_verify.py`; this page adds no new count that needs a script.
- **Not checked:** no run-time check, no solver-level proof of the break test (section 5.3), the unread part of the legged step, `ConnectBeamWithEntity` and `GetRandomBeamPassingDiamond` in detail.

## Layout / rules

### 1. Bouncer launch (`CBouncerEntity::OnHullTouched`; **medium**, the arguments **high**)

1. The touching hull's owner must be an entity of the base entity class; a touch by something else does nothing.
2. **Same-entity guard.** If the owner equals `en_henLastBounced` and fewer than `en_fIgnoreSameEntityPeriod` seconds (0.1 in all 71) passed since `en_tmLastBounce`, nothing happens. Otherwise `en_henLastBounced` := the owner and `en_tmLastBounce` := now.
3. **Velocity.** `v` = the owner's velocity (`GetVelocity`). `d` = the bounce direction `en_vBounceDir` (from `ep_fRelH`, `ep_fRelP`; **it is not normalised by the code, the length is used**). If `|d|` is at least a tiny constant: `vn = d * (v . d) / |d|^2` and `vt = v - vn`; otherwise `vn` and `vt` are both zero. The new velocity is **`w = vt * ParallelMultiplier - vn * NormalMultiplier + BounceSpeed * d`**; if `|w| > MaxExitSpeed` it is scaled to length `MaxExitSpeed` (`ParallelMultiplier` = `en_fParallelComponentMultiplier`, `NormalMultiplier` = `en_fNormalComponentMultiplier`, `BounceSpeed` = `en_fBounceSpeed`). Units: the game's length unit per second.
4. **Hand-over.** The bouncer calls the owner's **`DoJump(w, en_fAirControlTime, en_fAirControlFactor, en_fAirControlVelocityMultiplier, 1)`** (a virtual function of the base entity class: for every class except the legged puppets it keeps only `w` and calls `SetVelocity(w)`; the three air-control numbers and the flag are ignored).
5. **The flag.** The legged `DoJump` stores its last argument in **`en_bDisableFallingDamageOnNextTouch`**. The member is cleared when the puppet's dynamic members are reset and initialised to 0 by the constructor; **no function of the game DLL reads it** (searched in the puppet classes and the player classes), so the flag has no effect that was found (T01-384: the falling-damage rule as a whole is T01-140).
6. **Data.** 70 of 71 bouncers have both multipliers 0 (and one has 0.6 for the normal part): so `w = BounceSpeed * d` and any incoming speed is ignored; with `d` not of unit length the real speed is `BounceSpeed * |d|`.

### 2. Air control (legged puppets; **medium**; the legged step's other parts are T01-141)

**Members** of the legged puppet: `en_fAirControlTime` (class default and value after reset **0.5 s**), `en_fAirControlFactor` (**0.5**), `en_fAirControlVelocityMultiplier` (**1.0**), `en_bUsesAirControl` (0), `en_tmLastJump`, `en_tmLastAirControlDesired`, and the floor-contact time `en_tmLastFloorTouch`.

- **`DoJump(v, T, f, m, flag)`** (legged): `en_fAirControlTime` := `T`, `en_fAirControlFactor` := `f`, `en_fAirControlVelocityMultiplier` := `m`, `en_tmLastJump` := now, `en_tmLastAirControlDesired` := now, `en_bDisableFallingDamageOnNextTouch` := `flag`, then `SetVelocity(v)`.
- **The force limits every step.** The step sets the motion joint's acceleration force limit to `mass x pep_fAcceleration x en_fAccelMultiplier x (a per-puppet factor)` and the braking force limit to `mass x pep_fDecceleration x en_fDecelMultiplier x (a per-step factor)` (`puppet-record-members.md` section 8), **and then**, for a puppet that is **not touching the floor** (and not in the pose branch that the step handles separately, T01-141):
  1. `en_bUsesAirControl` := 0.
  2. The window is open when **`now - en_tmLastFloorTouch <= T`** or, failing that, **`now - en_tmLastJump <= T`**, with `T = en_fAirControlTime` in seconds (the clock counts 2^32 ticks per second).
  3. The puppet has air control only if the window is open **and** `en_tmLastAirControlDesired - en_tmLastFloorTouch` is **more than 0.2 s**. `en_tmLastAirControlDesired` is set to now by `DoJump` and, in the jump logic of the step, every step in the air in which the puppet is adding thrust (`en_bAddingThrust`). So a puppet that is only flying through the air without trying to move has no control; one that pushes for more than 0.2 s after leaving the floor has.
  4. **Without air control both force limits are 0** (the puppet cannot steer or brake). **With air control `en_bUsesAirControl` := 1 and both limits are multiplied by `en_fAirControlFactor`.**
  5. Finally, when the member at the place of `en_bTouchedFloorOnce` (the floor has never been touched in this life; identified by its position) is 0 and the puppet is not in the other branch, both limits are 0.
- **Speed.** `GetDesiredMovingSpeed` (legged) is the base wished speed **times `en_fAirControlVelocityMultiplier` while `en_bUsesAirControl` is 1**, else the base speed. The multiplier is set back to **1.0** by the reset of the dynamic members, by `SetupAspects` and by the pose change that leaves the fall pose.
- **Data meaning** of the bouncer's members (`mechanism-classes-movers.md` section 6): `ep_fAirControlTime` 10,000 (12 bouncers) means "the whole flight", 0 (10) means no window, 2 to 50 s otherwise; `ep_fAirControlFactor` 0 (10) means that the puppet can neither steer nor brake in the air, 1 or 0.5 (26 each) full or half force, 10 (5) ten times the force; `ep_fAirControlVelocityMultiplier` 1 in 50, 0 to 2 otherwise. **The NaN record** (one bouncer stores NaN, one 1.4e-45, a denormal): a NaN multiplier makes the wished speed NaN while air control is on; **what the movement code does with a NaN speed was not established** (**low**; the data is stray, T01-384).

### 3. Barrier fields (**high** for the two functions, **low** for the navigation flag)

- **`Enable`** is a one-call forwarder to the virtual **`SetupDefaultCollision`**; **`Disable`** to **`CancelCollision`** (the two slots are named in the class's virtual table).
- `SetupDefaultCollision`: without a hull it only sets `en_bEnabled` := 1; with a hull it sets the hull's category to **`barrier`**, or **`custom_barrier` when `en_bCustom` is set** (14 of 825), and `en_bEnabled` := 1. `CancelCollision`: with a hull the category becomes **`ignoreall`** (nothing collides with it); `en_bEnabled` := 0 in both cases. (`OnBoot` calls the setup on the host: `mechanism-classes-movers.md`.)
- **`ep_bIgnoredByNavigation`** (1 in 42 fields): **no reader exists in the game DLL**; the path-finder that would honour it is in the engine and was not searched. **Low** (T01-386).

### 4. Generic motors (`CGenericMotorEntity`, the engine's `CGenericMotorJoint`; **medium**)

**4.1 The command values.** Each axis X, Y, Z (linear) and H, P (angular; **no roll axis**) holds three numbers (the joint stores them per axis; a `CGenericMotorJointValues` record holds all of them: 5 forces, 5 velocities, 5 positions and one byte of five bits that mean "no target position" for the axis):

| Number | Script commands | Meaning |
|---|---|---|
| **force** | `SetForceX/Y/Z`, `SetTorqueH/P` | the **limit of the motor's force** (or torque) on the axis; **the axis is a motor only while it is greater than 0**; default 0 |
| **velocity** | `SetVelocityX/Y/Z`, `SetAngVelocityH/P` | the **target velocity** of the axis; with a target position set it is the **largest approach speed** instead |
| **position** | `SetPositionX/Y/Z/H/P`, `SetNoDesiredPosX/Y/Z/H/P` | the **target position** or angle; `SetNoDesiredPos` stores the "none" value **-3e38** (the default); the joint treats any value other than -3e38 as a target |

All linear numbers are in the game's length unit (per second); **the angular commands take degrees (and degrees per second) and the entity multiplies them by 0.017453292 before they reach the joint, so the joint works in radians**; torques are not converted. Every setter except `SetNoDesiredPos` calls `ActivateBodies` (wakes both bodies); the joint's `SetValues` wakes them only when a value changed.

**4.2 What the joint does with them** (`GetJik`, `CJointLimit::AddLinearLimit`; the angular rows are the same in structure). For each axis the joint adds, to the solver's constraint set, up to two rows along the joint's own axis (the placement of the joint relative to the source body):

1. **The limit row** (always, from the template's minimum and maximum, section 4.3): when the position along the axis is outside `[min, max]` a row pushes it back; the corrective speed is `(limit - position) x phy_fLimitSp / step` with `phy_fLimitSp` = **0.2** at start (a console variable of the engine), i.e. 20% of the error per solver step, with a softness of 1e-5. With the template defaults (-3e38, +3e38) the row never acts.
2. **The motor row** (only if the **force limit is greater than 0**): the row's force bounds are `-force` and `+force`; its target speed is the **velocity number**, unless a **target position** is set: then the target speed is **`(target - position) x phy_fLimitSp / step`, clamped to `[-velocity, +velocity]`**, and **the row is dropped when the result is smaller than 0.001 in size**. Consequences: **a target position with velocity 0 does nothing** (the clamp makes the speed 0 and the row is skipped); the clamp assumes a velocity of 0 or more; and a force of 0 switches the whole motor row off whatever the other two numbers are.

All 43 placed motors store the template defaults, so they have **no position limits at all**; their behaviour is what the scripts command.

**4.3 The template** (`CGenericMotorJointTemplate`, 10 floats; the data's numbered members `.1` to `.10` are in this order, **high**: the engine's own member names and the order of `CreateJoint` agree): `.1` `jt_fLo_H` and `.2` `jt_fHi_H` (heading limits, **degrees**, converted to radians), `.3` `jt_fLo_P`, `.4` `jt_fHi_P` (pitch, degrees), `.5` `jt_fLo_X`, `.6` `jt_fHi_X`, `.7` `jt_fLo_Y`, `.8` `jt_fHi_Y`, `.9` `jt_fLo_Z`, `.10` `jt_fHi_Z` (units). Defaults and data in all 43 motors: **-3e38 and +3e38**. (The data's order puts the angular pair before the linear ones; `mechanism-classes-movers.md` lists the five axes as X, Y, Z, H, P for the commands, which is a different thing.)

**4.4 Other facts.** `macGetLinearVelocity` returns the relative linear velocity of the two bodies. `LinkTargets` with one entity joins it to the world. The entity is `NEB_DYNAMIC` and replicates the joint values (`PostReceiveUpdate`).

### 5. External joints and the joint templates (**medium**)

**5.1 Templates** (`ep_pjtJoint`): the **ball** template (`CBallJointTemplate`, 69 of 93 joints) **has no members**: its `CreateJoint` only builds a ball joint (three rotational degrees free, no limits); the **fixed** template (16) has no members and builds a fixed joint (the joint gets the manipulator limits of 5.2); the **hinge** template (1) has two numbers, `jt_fLoLimit` and `jt_fHiLimit` (**degrees**, converted to radians for the joint's angle limits; the player-model hinges in the data use -30 to 30 and -65 to 80). With no template (7 joints) the entity builds a plain ball joint itself (`OnCreate`).

**5.2 Linking** (`LinkTargets`): the joint is initialised between the two bodies (the part bodies of the two entities; the second may be the world). For a **fixed** joint, which is a manipulator joint in the engine, the entity sets **maximum linear velocity 10, maximum angular velocity 2 pi (6.2831855 radians per second), maximum force and maximum torque 3e38 (unlimited)** (read; **medium-high**). The break force is stored in the joint at creation (`OnCreate`: `SetBreakForce(ep_fBreakForce)`); the joint's own default is **-1** (never breaks), and the data's -1 (76 of 93) keeps it.

**5.3 Breaking** (**medium-low**). The engine tests every joint with a break force above 0 once per solver cycle, but only when a per-joint iteration counter exceeds 2. It computes for each of the two bodies **`mass x |g|`** where `g` is a vector of the solver's island data (**probably the gravity vector, so the body's weight; not established**), takes the larger of the two and, **when the joint's break force is smaller than it**, stores that value in the joint and puts the joint on the engine's overload list. The engine then calls the owner's **`OnJointOverload(joint, force)`**. The break-force **unit is therefore a force** (mass times length unit per second squared, the same as the motors' force limits); the test is **not** a measurement of the constraint force in the sense of the solver's reaction, which differs from the usual reading of "break force" (T01-383 re-checks it). If the vector is gravity, the 8 joints with 5,000, the 4 with 50, the one with 10 and the two with 1e7 are weights of the attached bodies (not established).

**5.4 `OnJointOverload`.** If the reported joint is the entity's own joint it is **unlinked** (`CJoint::Unlink`, which releases both bodies) and the entity sends the **macro event `Broken`** to its script object. Nothing else (no sound, no effect) happens in this function; `ExecuteDestruction` (name only) is the destruction system's entry.

### 6. The diamond hand (`CDiamondHandEntity`; **medium**, the opened-ratio part **high**)

**6.1 Health.** Maximum **500** (`OnReceiveDamage` clamps to 0 .. 500, so a negative damage heals up to 500). `OnReceiveDamage` (host only): **damage of type 7 is ignored**; damage dealt by a **puppet whose alignment is 1 (evil) is ignored**; otherwise `en_slHealth` := `clamp(en_slHealth - amount, 0, 500)`, and only while the health is above 0. `GetHealthRatio` = health x 0.002; `SetHealthRatio(r)` sets the health to the integer of `r` times 500.

**6.2 The opened ratio** (`OnStep`, every step; all 14 hands). `desired` := `clamp(1 - health / 500, 0, 1)`; the stored current ratio `c` moves toward it by at most **`simulation step x 0.5`** per step (so a full opening takes 2 s). Then:
- **`Activated`**: when `c` was below 1 and becomes exactly 1; **`Deactivated`**: when `c` was exactly 1 and becomes smaller (macro events on the entity's script object).
- When `c` rises from below 0.5 to at least 0.5 the entity starts the timer function **`OpenedHandOscilator`** (first call after 1e-5 s); when it falls from at least 0.5 to below 0.5 the timer is cancelled. `OpenedHandOscilator` raises the macro event **`HalfOpened`** (as `mechanism-classes-movers.md` says: the string was read in the listing) and **re-arms itself after `ep_fActivatedOscilatorPeriod` seconds (15 in the data)**.
- Both hand-half mechanisms play their `Opening` animation at the offset ratio `c` every step.

**6.3 The beams.** Only the hand with `en_cptType` = `CPT_FIRST` (1) runs the beam update. It sums `en_fDistanceToNext` over the chain hand by hand following `en_henNext` **while each hand's current ratio is exactly 1** (a chain stops growing at the first hand that is not fully open), giving the **target beam length** `L`. A **static (shared by all chains) smoothed length** approaches `L` by at most `step x 40` per step (**40 units per second**). For each hand of the chain the beam's z stretch is `clamp(smoothed - en_fPathLength, 0, en_fDistanceToNext)`: **the beams grow from the first hand at 40 units per second**, each over its own `en_fDistanceToNext`; a hand whose stretch is 0 hides its beam model. **`CircleClosed`**: for the `CPT_LAST` (3) hand, once, when its beam model is visible and its stretch is within a small tolerance (0.2) of its full length `en_fDistanceToNext` (`en_bCircleClosedSent` set so that it fires once). `en_fPathLength` is the distance of the hand from the first one along the chain (-1 in 6 hands: the `CPT_FAKE` ones, 44.7 to 316 in the others).

**6.4 `ChainPositionType`**: 0 `CPT_FAKE`, 1 `CPT_FIRST`, 2 `CPT_LINK`, 3 `CPT_LAST` (**medium-high**: the names in declaration order; the roles agree with the code above). The 14 hands: 6 fake, 6 link, 1 first, 1 last.

**6.5 Not established:** `ConnectBeamWithEntity` (it orients a beam model from a hand to a target entity's placement with a heading and pitch from the direction and scales its z stretch to the distance; the angle limits it tests were not resolved), `GetRandomBeamPassingDiamond` (valid only on the first hand), `SetupAspects` and the flare, who sets `en_fPathLength` and `en_fDistanceToNext` (stored in the world), the meaning of damage type 7, and what the hide flags replicate (T01-385).

## Invariants

1. Bouncers: `d` is used with its stored length; with both multipliers 0 the exit velocity is `BounceSpeed * d` (70 of 71 records; the 71st has a normal multiplier 0.6).
2. Motors: every placed motor has the template defaults, no limits (43 of 43).
3. Fixed joints get a maximum linear velocity of 10 and a maximum angular velocity of 2 pi (the only joints that do: the code tests the joint class).
4. A diamond hand's health is always in 0 to 500; its desired ratio is in 0 to 1; its current ratio moves at most 0.5 per second.
5. `Activated` and `Deactivated` alternate (the ratio equals 1 or not).

## Open questions

1. **The break test** (section 5.3): which vector is multiplied by the masses, the meaning of the iteration gate, and the run-time behaviour of a ball joint loaded beyond its break force: **T01-383** (needs the instrumentation harness for a numerical answer).
2. **Legged air control leftovers** (section 2): the pose branch the force part skips, the virtual functions that give a normal jump its air-control time and factor, the NaN speed, and whether the "disable falling damage" flag is read by any code (the falling-damage rule belongs to T01-140): **T01-384**.
3. **The diamond hand's remaining functions** (6.5): **T01-385**.
4. **Barrier field navigation flag** and the engine's path-finder: **T01-386**.
5. **Run-time checks**: launch velocities with and without multipliers, the air-control window, motor target-position behaviour with velocity 0, the opened-ratio speed and the beam growth speed, the break of a ball joint: **T01-387** (with T01-174).

## Test vectors

- **Bouncer, plain pad:** `BounceSpeed` 30, `d` = (0, 1, 0), both multipliers 0, the toucher's velocity (5, -12, 3): `vn` = (0, -12, 0), `vt` = (5, 0, 3); `w` = (0, 30, 0). With `MaxExitSpeed` 20 the result is (0, 20, 0).
- **Bouncer with the 0.6 record:** toucher velocity (0, -10, 0), `d` = (0, 1, 0), speed 30, normal multiplier 0.6, parallel 0: `vn` = (0, -10, 0); `w` = `-vn x 0.6 + 30 d` = (0, 6 + 30, 0) = (0, 36, 0) (below the cap 200).
- **Guard:** the same puppet touching again after 0.05 s: ignored; after 0.15 s: launched again.
- **Air control:** a legged puppet launched with (T = 10, f = 0.5, m = 1.0) at time 0, its last floor contact at -0.1 s: the window is open until 10 s of flight; `en_tmLastAirControlDesired - en_tmLastFloorTouch` is 0.1 s at launch (not above 0.2), so **no control** at first; once the puppet adds thrust in the air at a time more than 0.2 s after the floor contact (from t = 0.1 s on) the difference exceeds 0.2 s, both force limits are half of the normal ones and the wished speed is multiplied by 1.0. With f = 0 the puppet never steers or brakes in the air.
- **Motor:** force X 100, velocity X 5, no target: the row pushes the relative X velocity to 5 with at most 100 force. Target position 8 at X = 0, velocity 5: target speed = `(8 - 0) x 0.2 / step` clamped to 5: at a step of 1/60 s that is 96, clamped to **5**; at X = 7.99 the speed is `0.01 x 0.2 x 60 = 0.12`. Velocity 0 with a target: the row is skipped.
- **Angular command:** `SetPositionH(90)` stores 1.5707964 radians in the joint.
- **Diamond hand:** health 500: desired 0, closed. Health 250: desired 0.5; the ratio rises 0.5 per second, so after 1 s it equals 0.5 and the oscillator timer starts (event `HalfOpened` at once, then every 15 s). Health 0: desired 1: after 2 s `Activated`. A hit of 600 from a player weapon: health 0 (clamped); from an evil puppet's attack or damage type 7: no effect.
- **Beams:** a first hand and two links, `en_fDistanceToNext` 10 each, all fully open: `L` = 30 (the sum along the chain); the smoothed length grows 40 per second: after 0.25 s it is 10 and the first beam is full; the second beam starts at `en_fPathLength` 10.
