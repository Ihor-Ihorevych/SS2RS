# Doors, animated movers, platforms and portals: states, timing and rules (T01-167)

- **Roadmap task:** T01-167 (follow-up of `mechanism-classes-movers.md` open question 1). Classes, members and counts: [`mechanism-classes-movers.md`](mechanism-classes-movers.md). The step and timer model: [`main-loop.md`](main-loop.md) and [`spawner-state-machine.md`](spawner-state-machine.md) section 3.
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the enumeration values (`EDoorState`, `DoorFlags`, `AnimatedMoverState`: the binary's own name tables, agreeing with every stored door and mover) and for the data tables (all 59 worlds and the models they name, script `tools/re/door_mover_verify.py`); **medium** for every rule below (one reading of the code each, lost arguments recovered from the instruction listing; the length formulas read in the engine); **low** for the detection area geometry, the sound scheme details and the points marked "open". **Nothing was run**: no door was opened, no time measured.

Prerequisites: [`mechanism-classes-movers.md`](mechanism-classes-movers.md), [`main-loop.md`](main-loop.md) (variable simulation steps), `ans.md` (animations: frames, frame rate), `wld-entities.md`, `replication.md`.

## Summary

1. **`EDoorState`**: **1 `DS_CLOSED`, 2 `DS_CLOSING`, 3 `DS_OPENING`, 4 `DS_OPEN`**; **`DoorFlags`**: bit 0 `DF_CAN_BE_USED`; **`AnimatedMoverState`**: 0 `AMS_IDLE`, 1 `AMS_MOVING` (section 1). This settles the labels left open in `mechanism-classes-movers.md`.
2. **A door is a timer-driven state machine** over one model animation: `DoOpening` (states 1 or 2 to 3) and `DoClosing` (3 or 4 to 2) play the open animation forwards or **backwards**, `SetState` schedules the end of the move after the animation's length, `FinishedMoving2` one simulation step later sets state 1 or 4, and an unlocked open door **closes by itself after `en_fClosingDelayTime`** (1 s in 174 of 176 doors) (sections 3 to 5).
3. **No door stores a close animation** (`ep_idCloseAnim` is none in all 176): **every door closes by playing its open animation in reverse**, at the same speed, even in the models that contain a `Close` animation (section 3).
4. **`Open` and `Lock` are not what their names suggest**: both set the **lock flag** `en_bLocked` and then open or close the door; the lock flag stops touches and the auto-close. `AssureOpened` and `AssureLocked` are the same two functions (section 4).
5. **An animated mover** plays a stored animation (`PlayAnim`, `PlayAnimLoop`, `Start`) and `PlayAnimWait` suspends the script for **(last frame - first frame + 1) / frame rate** seconds measured from the call, by a timer, not by watching the animation; a non-looping animation leaves the mover in state 1 (moving) at its end until `Stop` (section 8).
6. **The platform** (no instance in the retail data) is a slab that moves back and forth: **2 m/s along +x for 6 s, then 2 m/s along -x for 6 s**, started at boot; the record's time and velocity are not used (section 9).
7. **Portals do nothing in single player.** In multiplayer `Activate` / `Deactivate` build the "playerdetector" hull and play the portal animation; a touching player is teleported to **the chapter's start placement shifted by a per-player offset from a 17-entry table of whole metres** (section 10).

## Evidence

- **Code.** All exported methods of `CDoorEntity`, `CAnimatedMoverEntity`, `CPlatformEntity` and `CPortalEntity` that contain behaviour were decompiled and, where arguments were lost (state numbers, delays, scheme-sound names, function names scheduled by the "next think" call), read again in the instruction listing: `SetState`, `DoOpening`, `DoClosing`, `FinishedMoving`, `FinishedMoving2`, `AutoClose`, `Open`, `Lock`, `Unlock`, `OnHullTouched`, `OnBoot`, `OnCreate`, `EnableUsage`, `DisableUsage`, `GetObjectManipulationFlags` and `...Message` of the door; `SetState`, `OnTrigger`, `PlayAnim`, `PlayAnimLoop`, `PlayAnimWait`, `Start`, `Stop`, `StartAnimation`, `WaitMoverToEndMoving`, `OnLatentAnimationFinished`, `OnStep`, `EvalFrame`, `OnBoot`, `ResetDynamicMembers`, `OnCreate` of the mover; `StartMoving`, `SyncMovement`, `OnBoot`, `OnCreate` of the platform; `OnHullTouched`, `Activate`, `Deactivate`, `OnStep`, `StopMoving` of the portal; and in the shared code `samGetPlayerPlacementOnChapter`, `samIsSinglePlayer`, the model renderable's animation length and the animation length function of `Core.dll`/`Engine.dll`. The enumeration name tables were read as data. Build: the retail DLLs of the install used by all T01 pages.
- **Data.** `python -I tools/re/door_mover_verify.py` (about 8 minutes): every door record and entity, the animations of every door model (name, frames, frame rate, lengths), every animated mover's animation set, every portal. Counts below are from that run.
- **Not checked.** The detection area geometry (read only in outline), the sound events and sound scheme tables, the client-side offset correction, the engine's interpolation of keyframes, any timing on the running game.

## Layout / rules

### 1. Enumerations (high)

| Enumeration | Values (the binary's names) | Notes |
|---|---|---|
| `EDoorState` | 1 `DS_CLOSED`, 2 `DS_CLOSING`, 3 `DS_OPENING`, 4 `DS_OPEN` | stored 1 in all 176 doors (and 1 in the replicated copy); 0 is not a valid state |
| `DoorFlags` | 1 `DF_CAN_BE_USED` | stored 0 in all 176; only this bit is ever set (`EnableUsage`) |
| `AnimatedMoverState` | 0 `AMS_IDLE`, 1 `AMS_MOVING` | stored 0 in all 277 movers |

### 2. Door members used

`en_dsState` (the state), `en_dfFlags`, `en_bLocked` (the run-time lock, **"locked" in the sense of section 4**), `en_bInitiallyLocked` (`ep_bLocked`: 62 of 176 records), `en_fClosingDelayTime` (`ep_fClosingDelayTime`), `en_idOpenAnim` (`ep_idOpenAnim`: the name of an animation of the door model; stored as an index into the file's identifier table: `Open` in most, `default` and `Open_Wide` in others, see the census), `en_idCloseAnim` (`ep_idCloseAnim`: **none in all 176**), `en_fAnimLen` (the length in seconds of the animation of the current move), `en_ulCurrentAnimFlags` (0 = played forwards, **4 = played in reverse**; the only two values used), `en_idCurrentAnimation`, `en_phuDetectionArea` (the detection hull: set in 131 of 176), `en_idMechanismName` (the model's mechanism, `Default` in all).

### 3. `DoOpening`, `DoClosing` and the close animation (read; medium)

Let *t* be the **elapsed play time** of the current animation (the time since it started, from the engine's played-animation record; 0 when there is none) and `len(a)` the length of animation *a* as the door uses it: **(last frame - first frame) / frames per second** (no "+ 1": the engine's length function is called with its "include last frame" flag 0 for doors).

**`DoOpening`** (virtual; acts only when the state is **1 or 2**):
1. `en_fAnimLen` := `len(open animation)`; offset := 0.
2. If the state is **2 (closing)**: offset := `max(0, en_fAnimLen - t)`.
3. Clear the playback, **play the open animation forwards** (flags 0) and set its play time to *offset*; remember animation and flags.
4. `SetState(3, en_fAnimLen - offset)`.

**`DoClosing`** (acts only when the state is **3 or 4**):
1. If `en_idCloseAnim` is **none**: animation := the open animation, flags := **4 (reverse)**; else animation := the close animation, flags := 0.
2. `en_fAnimLen` := `len(animation)`; offset := 0; if the state is **3 (opening)**: offset := `max(0, en_fAnimLen - t)`.
3. Clear the playback, play the animation with the flags, set the play time to *offset*.
4. `SetState(2, en_fAnimLen - offset)`.

Meaning: a move **reverses from the current position** (a door opened for *e* seconds closes in *e* seconds; a door closed for *e* seconds out of its full length reopens in *e* seconds), and a full move takes `len`. **In all 176 doors the close animation is none**, so all closings play the open animation backwards, **even for the models that contain a `Close` animation** (`Door.mdl` of the Kronor level: `Open` and `Close`, 1 to 10 at 5 frames per second, 1.8 s each; they are simply not used). The open animation's length `len` over the **176 doors** (36 door models; by the model's `Open` animation, else its first; script section 1): **0.15 s in 15 doors, 0.2 s in 5, 0.45 s in 9, 0.6, 0.7 and 0.9 s in one each, 1.0 s in 87, 1.8 s in 6, 1.967 s in 1, 2.0 s in 41, 2.25 s in 4, 3 s in 1, 4 s in 1, 4.5 s in 1, 5 s in 2**. The closing takes the same time. The record's open animation name is `Open` in 161 doors, `default` or `Default` in 7, `Door` in 4, `Open_Wide` in 2, `Fast` in 1, `Open_Half` in 1; the mechanism name is `Default` in 167 and `Open` in 9.

### 4. Commands, touch and the lock flag (read; medium)

| Entry | Effect |
|---|---|
| script `Open` = `AssureOpened` | `en_bLocked` := 1, then `DoOpening` |
| script `Lock` = `AssureLocked` | `en_bLocked` := 1, then `DoClosing` |
| script `Unlock` | `en_bLocked` := 0; **if the state is 4** schedule `AutoClose` after `en_fClosingDelayTime` |
| `AutoClose` (the timer function) | if `en_bLocked` is 0: `DoClosing`; else nothing |
| **touch** (`OnHullTouched`, host) | ignored while `en_bLocked`; only the **detection hull** of this door counts (the hull test is the only test: players-only comes from the hull's category, section 5); state 4: re-arm `AutoClose` (restart the closing delay); otherwise `DoOpening` (which acts only in states 1 and 2, so a touch during state 3 does nothing). Returns 0 |
| `EnableUsage` / `DisableUsage` | set / clear `DF_CAN_BE_USED` in `en_dfFlags` |
| `GetObjectManipulationFlags` | `(DF_CAN_BE_USED) OR 8`: the value 8 is always set |
| `GetObjectManipulationMessage` (the use prompt) | with `DF_CAN_BE_USED` set: the base class message; else, if the state is 1 and (`en_bLocked` or **no detection hull**): "The door is locked" (`ETRSObjectActionMessage.TheDoorIsLocked`); else empty |

So `Open` **holds the door open** (nothing closes it until `Unlock`), `Lock` **closes and locks it**, and a locked door ignores touches whatever its state. The record's `ep_bLocked` is copied into `en_bLocked` by `OnBoot`. A door whose `ep_bLocked` is 1 therefore starts closed, deaf to touches until a script calls `Unlock` or `Open`.

### 5. `SetState(state, delay)` (read; medium)

`delay` defaults to the sentinel **-1** (meaning `en_fAnimLen`). Let *moving(s)* mean *s* is 2 or 3, and a **"wrong-direction start"** a change from 1 to 2 or from 4 to 3.

1. Switch the per-step update off (`DisableOnStep`).
2. **Current state 2 or 3 (a move in progress):**
   - new state moving (a reversal): enable the per-step update, **stop the kinematic parts of the mechanism**;
   - new state 1 or 4 (the move ends): play the model's **scheme sound `Close`** (play mode 2) or, if the model has none, stop the door's sound channel; **stop the kinematic parts**.
3. **Current state 1 or 4 (at rest), new state moving:** unless it is a wrong-direction start, play the scheme sound **`Moving`** (play mode 3) if the model has one; enable the per-step update.
4. If the new state is moving: schedule the entity's next think with the function **`FinishedMoving`** after `delay` (or `en_fAnimLen` when `delay` is -1).
5. If the new state is **4** and `en_bLocked` is 0: schedule the next think with **`AutoClose`** after `en_fClosingDelayTime`, store state 4 and return.
6. Store the new state.

`FinishedMoving` (the timer function) schedules `FinishedMoving2` after **one simulation step** (a variable-length frame step, `main-loop.md`); `FinishedMoving2`: state 2 -> `SetState(1)`, state 3 -> `SetState(4)` (with the default delay: nothing is scheduled for the rest states). The entity has **one pending think**: a new `SetState` into a moving state replaces a pending `AutoClose`, and the end-of-move timer replaces nothing else. The per-step update (`OnStep`) evaluates the mechanism's animation every step while a move runs (and corrects client offsets on a remote client).

**`OnBoot`** (new level or chapter restore): `en_bLocked` := `en_bInitiallyLocked`; state := 1; `en_fAnimLen`, **`en_dfFlags` (usage bit), the current animation and its flags := 0 / none**; the mechanism is rebuilt (category as in `OnCreate`), the per-step update is switched off and **any pending think is cleared**. A restored door is closed, its use flag is off (a script must call `EnableUsage` again), its lock is the initial one.

### 6. Door timeline (test vector from the code; no run)

Door with the open animation of `Door_destructible.mdl` (`Open`, frames 0 to 90 at 40 per second: `len` = 2.25 s), `en_fClosingDelayTime` 1 s, unlocked; *s* = one simulation step:

| Time | Event |
|---|---|
| 0 | a player enters the detection hull: `DoOpening`: state 1 -> **3**; open animation forwards from 0; `FinishedMoving` due at 2.25 |
| 2.25 | `FinishedMoving` (state 3), `FinishedMoving2` due at 2.25 + *s* |
| 2.25 + *s* | `SetState(4)`: scheme sound `Close` played, kinematic parts stopped, `AutoClose` due 1 s later |
| 3.25 + *s* | `AutoClose`: `DoClosing`: state 4 -> **2**, the open animation in reverse from offset 0, `FinishedMoving` due 2.25 s later |
| 5.5 + *s* | `FinishedMoving`, then at 5.5 + 2*s* the state becomes **1** |

A touch at any time in state 4 restarts the 1 s delay (the pending `AutoClose` is replaced). A touch at 3.8 + *s* (0.55 s into the closing): `DoOpening` from state 2: offset = 2.25 - 0.55 = 1.70; state 3 for **0.55 s**; the door is back to open (state 4) at 4.35 + 2*s* (about) and closes again after 1 s. A touch during state 3 does nothing.

### 7. Detection area, sounds and replication (outline; low)

`OnCreate` builds the door's model renderable from the record (stretch, cull distance, fade length), the mechanism named by `ep_idMechanismName` from the model (a warning "Model configuration for a door with ID ... doesn't have a mechanism." and no mechanism if missing), two sound sources (the door's own and one for the animation-event sounds), the animation event handler, and, when the record asks for it, the **detection hull**: a box derived from the model's bounding box, widened by `ep_fExtraArea` (0 in 110 records, 0.94 in 13, **negative in 28** (-6 in 9, -0.93 in 8, -1.16 in 5 ...), which shrinks it) and `ep_fSafetyOffset` (0 in 150 records, 0.5 in 9, 0.1 in 4 ...), with the sides chosen by `ep_bDetectorFront` / `ep_bDetectorBack` (data: back only 52, front only 33, both 46, **neither 45: exactly the 45 doors without a detection hull**); its collision category is "playerdetector" when `ep_bPlayersOnly` (102 of 176) else "detector". The exact box formula was not recovered (**T01-306**). Sound events of the open animation (`OnAnimEventProduceSound`) play through the second source; those flagged to terminate are stopped when the move ends. The replicated members and the client's start-at-offset rule are in `mechanism-classes-movers.md` and `replication.md`.

### 8. Animated movers (read; medium)

**Members (binary names):** `en_idOrgAnimation` (the default animation, `ep_idAnimation`: the name **`AnimatedMover`** in all 277), `en_qvDefault` (the record's placement), `en_bRelativePath` (`ep_bRelativePath`, 1 in all 277), `en_peacAnimConfig` (the animation configuration, the record's `ep_peacAnimConfig`, a fresh empty one when the record has none), `en_amsState`, `en_bIsAnimLooping`, `en_idCurrentAnimation`, `en_fPlayedAnimationLength` (-1 = none), `en_mlidMacroLatentID` (the script wait), `en_qvLastStep`, `en_qvPlacement` (last evaluated placement).

**Script commands.**

- `PlayAnim(id)`: the omitted argument means the default animation; the animation instance is cleared and the animation queued **once**, `en_bIsAnimLooping` := 0, `en_idCurrentAnimation` := *id*, `SetState(1)`; the body is moved to the animation's start placement at once; `en_fPlayedAnimationLength` := -1 (no wait).
- `PlayAnimLoop(id)` and **`Start(id)`** (= `PlayAnimLoop`): the same with the animation **looping**.
- **`PlayAnimWait(id)`**: as `PlayAnim`, then `en_fPlayedAnimationLength` := **(last frame - first frame + 1) / frames per second** of the animation looked up in the mover's animation configuration (0 if not found) and the script is **suspended** (`BeginLatent`). `SetState(1)` has scheduled `WaitMoverToEndMoving` for the next step; that function, **if the length is not -1**, schedules `OnLatentAnimationFinished` after the length; that function ends the latent wait and the script resumes. The script therefore resumes **about `length` + two 0.00001 s hops after the call**, by timer: it does not watch the playback, and the length uses the "+ 1" form (unlike the doors, the animation's last frame counts).
- **`Stop`**: `SetState(0)`.
- **`OnTrigger`** (a trigger entity that points at the mover): unless already moving, `en_fPlayedAnimationLength` := -1 and `SetState(1)`; it starts **whatever animation is current** (it does not select one: after boot that is none, so a mover that is only ever triggered must have been given an animation by a script first; open, **T01-307**).

**`SetState`**: leaving state 1 sets the body's linear and angular velocity to zero and switches the per-step update off; storing the new state; entering state 1 switches the update on and schedules `WaitMoverToEndMoving` after 0.00001 s. **Nothing returns the state to 0 when a non-looping animation ends**: the mover stays in state 1, its body held at the last evaluated pose with zero velocity, until `Stop`.

**`EvalFrame`** (the pose): the animation's channel named **`AnimatedMover`** is evaluated for position and rotation (`aniEvaluatePosLength`, `aniEvaluateRotation`); with `en_bRelativePath` (always 1 in the data) the pose is **composed with `en_qvDefault`**: rotation := default rotation times the sampled rotation, position := default position + (default rotation applied to the sampled position). The six envelope names the editor offers are `Pos.X`, `Pos.Y`, `Pos.Z`, `Rot.H`, `Rot.P`, `Rot.B` (`mechanism-classes-movers.md` section 3); the interpolation between keys is not recovered (`ans.md`).

**`OnStep`** (state 1, every step): evaluate the pose; **velocity** := (new position - previous position) / step; **angular velocity** := the relative rotation from the previous to the new orientation as axis times angle / step (the shorter way: the quaternion is negated when its dot product with the previous one is negative); both are given to the kinematic body so that things standing on the mover are carried. `en_qvLastStep` keeps the previous pose.

**`OnBoot`** (chapter restore): clear the animation instance, current animation := none, last pose and placement := `en_qvDefault`, the body is placed there, `SetState(0)`: **a restored mover is idle at its default placement**.

**Data (script section 2).** The 277 movers carry animation sets with **1 animation (132 movers), 2 (53), 4 (45), 3 (24), 8 (17), none (3), 5, 7 and 10 (one each)**, 648 animations in all; names `Default` 129, `Down` 58, `Up` 50, `Move01` to `Move03`, `Move_01` to `Move_04`, `Deep`, `Half` ...; frame rates 30 (235), 1 (95), 66.67 (75), 1.33 (51), 0.5 (34) and others down to 0.02; **every one of the 648 animations has exactly two envelopes** (a position and a rotation channel of the `AnimatedMover` target); lengths (last - first + 1) / frame rate from **0.015 s to 1,470 s, median 3 s** (the long ones are slow rides: `First_Ride` and `Second_Ride` of `2_4_woodstock` 300 s, `Slide` of `7_7_mental` 280 s). 

### 9. Platforms (read; medium)

`CPlatformEntity` is **in no retail file**. Rules:

- `OnCreate` builds a kinematic body and a **hull and renderable of a box primitive of 5 x 0.3 x 5 m** (the box is hard-coded), the record supplying only the **colour** and the **stretch**; it **overwrites** `en_fTime` := **6.0 s** and `en_vVelocity` := **(2, 0, 0) m/s** with constants: the record's `ep_fTime` and `ep_vVelocity` are not used (they are in the record's layout only).
- `OnBoot`: `en_fSign` := -1, the body is put at the creation placement `en_qvInitPlacement`, then `StartMoving`.
- `StartMoving` (the function that repeats itself): `en_tmMovementStarted` := now; `en_fSign` := -`en_fSign`; the body's linear velocity := `en_fSign` times `en_vVelocity` (world axes, not rotated by the placement); schedule `StartMoving` again after `en_fTime`.
- Result: **+2 m/s along +x for 6 s, then -2 m/s for 6 s, forever**, a 12 m path starting at boot with sign +1.
- `SyncMovement` (clients; not read in full): re-applies the velocity of the current sign and schedules `StartMoving` after `en_fFTimeLeft` (the replicated remaining time). `NEB_DYNAMIC` (`replication.md`).

### 10. Portals (read; medium)

- **Single player.** `Activate` and `Deactivate` (two separate functions) begin with the test `samIsSinglePlayer` (the game info's type field equals **1**) and do **nothing** when it is true. The "playerdetector" mechanism (the hull a player must touch) is created **only** inside these two functions, so **in single player a portal has no hull: it can never be touched** and teleports nobody; it only plays its default animation from `OnCreate` (`Up` in the sample).
- **Multiplayer / co-op.** `Activate` (script command): builds the mechanism if the model has it (`Default` creates it, category `playerdetector`), enables the per-step update (it evaluates the mechanism's animation), plays the animation **`Activate` forwards** (flags 0) and schedules `StopMoving` after the animation's length (the door-style length: last - first over fps). `Deactivate`: the same but the animation **backwards** (flags 4). `StopMoving`: switches the update off and stops the kinematic parts.
- **Touch** (`OnHullTouched`): only a **player puppet with a player brain** counts; the portal needs a **chapter link** (all 54 have one). The player is **teleported** (`Teleport(placement)`) to `samGetPlayerPlacementOnChapter(chapter, slot, player)` where *slot* is the brain's **player slot number** (not recovered further).
- **Arrival placement** (`samGetPlayerPlacementOnChapter`; medium): the chapter entity's own placement (the chapter's "start" marker) with the position shifted by **R * (a, 0, b)** metres, R being the chapter placement's rotation and (a, b) from the table below (x = right, z = back in the chapter's frame); the orientation is the chapter's orientation unchanged. If the chapter record's "fall down" member is set the shifted point is raised **2 m along the up axis** and `samGetFallDownPosition` (a ground search) may replace it with the grounded position for this player; if the search fails the shifted point is used.

| slot | 16 | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | other |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| a | 0 | -1 | -1 | 0 | 1 | 1 | 1 | 0 | -1 | -2 | -2 | -2 | -2 | -1 | 0 | 1 | 2 |
| b | 0 | 0 | 1 | 1 | 1 | 0 | -1 | -1 | -1 | -1 | 0 | 1 | 2 | 2 | 2 | 2 | 2 |

Slots 0 to 7 form a ring of eight spots around the chapter's start point, 8 to 14 continue on an outer ring, **16 is the start point itself**; **15 and every number above 16 (or negative) get (2, 2)** (the last spot). The co-op portal marker (`ep_bCoopPortalMarker`, 0 in all 307 chapters, `mechanism-classes-world-info.md`) is not read by these functions.

## Invariants

1. The only door states are 1 to 4; every door in the data is in state 1 with `DoorFlags` 0 (176 of 176).
2. `ep_idCloseAnim` is none in all 176 records (`DoClosing` therefore always reverses the open animation).
3. Every animated mover in the data has state 0, relative path 1, the default animation `AnimatedMover` and at least one animation of two envelopes (script).
4. A portal's chapter link is set in 54 of 54 (`mechanism-classes-movers.md`).

## Open questions

1. **The detection hull geometry** (the exact box from the model's bounding box, extra area and safety offset), the sound scheme tables and the client offset correction. **T01-306.**
2. **Mover animations**: what `OnTrigger` plays when no animation is current, the interpolation between keys, which scripts call `Start` / `Stop`, what a mover does at the end of a looping or one-shot animation in the retail levels (all by run time). **T01-307.**
3. The value names of the game info's type field behind `samIsSinglePlayer` (1) and the mode used by `ShouldntSpawnAmmoItems` (2). **T01-307.**
4. Run-time confirmation of the door and platform timings and of the portal offsets. T01-174.

## Test vectors

1. **Door lengths** (from the data, script section 1): `Door_destructible.mdl` `Open` 0..90 at 40: 2.25 s; `Door.mdl` (Kronor) `Open` and `Close` 1..10 at 5: 1.8 s; `Door_03.mdl` `Open` 0..60 at 30: 2.0 s; `Door.mdl` (Pechina fence) `Open` 0..30 at 30: 1.0 s and `Open_Wide` 0..30 at 100: 0.3 s.
2. **Door timeline** of section 6; the distribution of door open lengths is in section 3.
3. **Mover wait.** A mover whose animation has frames 0 to 420 at 30 per second (the `Up` animation of `6_3_snow_base`): `PlayAnimWait` suspends the script for **(420 - 0 + 1) / 30 = 14.03 s**.
4. **Platform.** Created at x = 0 and booted at t = 0: x(t) = 2 t for t up to 6 (12 m), then 12 - 2 (t - 6) until t = 12, and so on.
5. **Portal slots.** Chapter start at the origin facing -z with the identity rotation: slot 3 arrives at (1, 0, 1) metres, slot 8 at (-2, 0, -1), slot 16 at (0, 0, 0), slot 20 at (2, 0, 2).
