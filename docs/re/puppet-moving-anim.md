# The moving-animation chooser of legged puppets (T01-148)

- **Roadmap task:** T01-148 (follow-up of [`puppet-legged-classes.md`](puppet-legged-classes.md) open question 2; the data side is generated in `misc-schema.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the counts and value tables of the schemes (reproduced by `tools/re/moving_anim_verify.py` over all `.mas` and `.mdl` files); **medium** for every rule read from `Sam2Game.dll` (one reading of each function; the selection loops were checked against the instruction listing where noted); **low** where marked. **Nothing was run on the original game.**

Prerequisites: [`puppet-legged-classes.md`](puppet-legged-classes.md) (the members and enumerations), [`puppet-classes.md`](puppet-classes.md) section 3.3 (`MovingAnimationPose` ...), `mat.md` (the `.mas` and `.sca` files), `mdl.md` (the custom model data that names the scheme), `ans.md` (animation sets), [`puppet-attacks.md`](puppet-attacks.md) (animation groups and play modes).

## Summary

A walking character does not name its animations. Every step the puppet builds a small **selector record** (`CMovingAnimData`: pose, speed class, direction, readiness, shield, emotion and up to four held tool ids) from its state, and the model's **moving-animation scheme** (`CMovingAnimationScheme`, files `.mas` or a scheme embedded in the model) answers with up to two animation identifiers: an **"all" animation** (the whole body: group 1) and an optional **body override** (the upper body for a held weapon: group 2). A third list holds **pose-transition animations** (stand to crouch, knock-out, death). The chooser is a scoring search over the records of the scheme and of its base schemes.

- The selector and its construction: section 2. The three record kinds: section 1. The choice rules: sections 3 to 5. Playing, speed matching and when it is re-run: section 6. `samRemapSchemeAnim` (the "term" animations): section 7.
- Of the 438 models with custom model data **110 name a scheme** (70 as one of 19 `.mas` files, 40 with a scheme embedded in the model); 328 have none and are not moved by this chooser.
- Two things an implementer will not guess: the search keeps the **first** best record on ties in the "all" and body choosers but the **last** one in the pose-transition chooser; and a record whose pose, direction or speed does **not** match is still chosen when nothing better exists (the "all" chooser never rejects on those three; only a tool mismatch rejects).

## Evidence

- **Binary:** `Sam2Game.dll` of the install used for the other specs. Read: `PrepareMovingAnimData`, `GetMovingAnimPose`, `GetEnvironmentParams`, `GetDesiredMovingSpeed` (base and legged), `GetDeclaredNominalVelocity`, `PlayMoveAnim`, `PlayPoseChangedAnim`, `PlayDefaultAnimation`, `DiscardMovingAnimationData`, `AutoAnimationOn/Off`, `CMovingAnimData` (constructor, `IsEqualTo`, the description function), the constructors of the three record classes, `CLPEnvironmentParams::GetInstanceMoveSpeed` and its constructor, and the engine-side functions `samGetMovingAnims`, `samGetPoseTransitionAnims`, the body-record search, the tool-match scorer, `samGetSchemeAnim`, `samRemapSchemeAnim`. The end of `LerpToDesiredVelocityAndLook` (the movement step, T01-141) was read **only** for the calls to the functions above and the values it passes. The selection loops were checked in the instruction listing for the tool scorer and the comparison order.
- **Data:** `tools/re/moving_anim_verify.py` reads the 20 `.mas` files and the 40 embedded schemes and prints every count; it also contains a **reference implementation of the choosers** that the test vectors come from.
- **Not checked:** no run; the movement step's own logic (T01-141); the animation-event side of the animations; the contents of the `.sca` animation schemes beyond the lookup rule.

## 1. The records (data; **high**)

Positions are the data's stored order; identifiers are animation names of the model; enumerations are those of `puppet-classes.md` (the data stores `-2` as 4,294,967,294: "ANY" for pose, direction, speed, readiness and emotion; **shield uses -1 for ANY**).

**`CAllMovingAnimation`** (`.mas` list 1; 167 records in the 60 schemes), 12 members: animation id; **"term" flag** (the id is a name of the model's animation scheme to be remapped: true in 6 of 167); **nominal velocity** (m/s the animation covers at speed 1; -1 = none; range -1 to 25, 80 records negative, one 0); **reverse flag** (true in 1); pose (`MAP_*`); direction (`MAD_*`); speed class (`MAS_*`); **required tool ids** (0 in 142, 1 in 19, 2 in 6; names such as `PlayerWeapon_MiniGun_R`, `NeanderthalClub`, `ChicheWeapon_Uzi_L`, and the wildcard `<Any>` in 2); readiness (`MAR_*`); shield; emotion; **"no body override"** flag (true in 34). Values over the 167: pose ANY 72, stand 71, fly 9, crouch 8, fall 2, swim 2, dive 2, roll 1; direction ANY 94, forward 31, turn-left 11, turn-right 11, backward 7, left 5, right 5, not-moving 3; speed idle 47, fast 44, ANY 44, slow 32; readiness ANY 137, ready 28, not ready 2; shield ANY 165, not shielded 2; emotion ANY 163, frightened 3, normal 1.

**`CBodyMovingAnimation`** (list 2; 90 records), 6 members: animation id; required tool ids (a tool name or `<Any>`; 28 distinct names); pose (ANY 56, swim 17, dive 17); readiness (ANY 53, not ready 26, ready 11); shield (ANY 90); emotion (ANY 89, frightened 1).

**`CPoseTransitionAnimation`** (list 3; 58 records), 11 members: animation id; **reverse flag** (true in 1); **from pose**; **to pose**; direction; speed; tool ids (none stored); readiness; shield; emotion; "no body override". (from, to) pairs: (ANY, knock-out) 34, (ANY, death) 16, (knock-out, stand) 2, (fall, fly) 2, and one each (stand, crouch), (crouch, stand), (fall, stand), (stand, fall).

**`CMovingAnimationScheme`**: the three lists and a **base scheme** (a reference to another scheme; 25 of the 60 have one: 15 of the 20 `.mas` files and 10 of the 40 embedded schemes (9 name `BipedMoving.mas`, 1 an embedded scheme)). Sizes: `BipedMoving.mas` 24 / 6 / 12 records, `Player.mas` 22 / 56 / 0, `NPCs.mas` 2 / 9 / 0, `Kleer.mas` 3 / 0 / 0 (idle, run, walk; every pose, direction and readiness ANY), `Tank.mas` 1 / 0 / 1, `Snowman.mas` 2 / 0 / 0; the 40 embedded schemes hold 0 to 7 "all" records each (0 in 10).

The model names its scheme in the fourth member of its custom model data (`CSamCustomModelData`): 70 models point at a `.mas` (Player 16, NPCs 15, BipedMoving 14, the rest 1 to 3 each), 40 embed one, 328 have none.

**Defaults of the records (constructors; medium).** A fresh selector has pose, speed, direction, readiness and emotion ANY (-2), shield ANY (-1) and four invalid tool ids; a fresh "all" record has ANY everywhere and shield -1; a fresh transition has both poses ANY.

## 2. The selector (`CMovingAnimData`) and `PrepareMovingAnimData(speed)`

The selector is 10 words: pose, speed class, direction, readiness, shield, emotion, then four tool ids. It is rebuilt by `PrepareMovingAnimData(speed)` (read) from the puppet's members, where `speed` is the animation speed value of section 6 (0 when the puppet is not moving):

1. **Pose** := `GetMovingAnimPose(en_ppPuppetPose)`: `PP_STAND` 1 to `PP_KO` 8 map to `MAP_STAND` 0 to `MAP_KO` 7 in order (crouch 1, dive 2, swim 3, fly 4, fall 5, roll 6, ko 7); **any other value (0 and 9 `PP_RIDING_WHEEL`) gives `MAP_STAND`**.
2. **Speed class** := `MAS_IDLE` (2). If the puppet has environment parameters (`GetEnvironmentParams`, below) and `speed` is not 0: **`MAS_SLOW` (1) when `speed` is below the record's slow limit, else `MAS_FAST` (0)**. Without environment parameters the class stays idle whatever the speed.
3. **Direction** from the puppet's move direction `en_mdMoveDirection` (`MD_*`): left 1 -> `MAD_LEFT` 2, forward 2 -> `MAD_FORWARD` 0, right 3 -> `MAD_RIGHT` 3, backward 4 -> `MAD_BACKWARD` 1, stop 5 -> `MAD_NOT_MOVING` 8, turn-left 6 -> `MAD_TURNLEFT` 4, turn-right 7 -> `MAD_TURNRIGHT` 5, up 8 -> `MAD_UP` 6, down 9 -> `MAD_DOWN` 7; **any other value (0) leaves the previous direction value in the record unchanged**.
4. **Readiness** := the member `en_marDesiredReadiness` when it is not -1 (a script can force it); otherwise **ready (1) when the puppet's brain exists and `HasFoe` is true, else not ready (0)**.
5. **Shield** := `en_cssShieldStatus`, **emotion** := `en_ceCharacterEmotion` (the script functions `macSetShieldState` and `macSetEmotionState`: `puppet-legged-classes.md`).
6. **Tool ids:** all four set to invalid, then the first up to four **attached tools' ids** (the list `en_aactAttachedTools`, in list order) are copied.

**Environment parameters** (`GetEnvironmentParams`, read): the legged parameter file's three records chosen by the pose: **fly (`PP_FLY` 5): the second (air) record; dive or swim (3, 4): the third (water) record; every other pose: the first (land) record**; none without a legged parameter file. The **slow limit** is the member the data stores as number 7 of `SPuppetEnvironmentParams` (default 2.5; range 0 to 99; Kleer land 5, air 3; Kleerofski land 30), the fifth float of the record (**medium**: matched by the constructor's defaults against the stored Kleer record). The two members before it (numbers 3 and 4: default 6, 6) are the **minimum and maximum move speed**: `GetInstanceMoveSpeed(t) = min + (max - min) x t`, and the legged `GetDeclaredNominalVelocity` is that at the puppet's per-instance random fraction (1.0 without environment parameters). T01-214 owns the other members.

`DiscardMovingAnimationData`: invalidates the last chosen ids, clears the last speed and reverse values, and sets the puppet's pose and move direction members to 0 (invalid). `AutoAnimationOn` calls it once when automatic animation was off; `AutoAnimationOff` only clears the flag `en_bAutoAnimations`. `CMovingAnimData::IsEqualTo` compares all 10 words (pose, speed, direction, readiness, shield, emotion and the four tool ids).

## 3. Choosing the "all" animation (`samGetMovingAnims`; read)

Inputs: the model's scheme chain (the scheme, then its base scheme, then that one's base, ...), the selector. Outputs: the **all-animation id**, its **term flag**, its **reverse flag**, its **nominal velocity** and the **body-animation id** (or invalid).

**Tool score of a record** (`n` = the number of valid tool ids in the selector, at most 4; checked in the listing): for each **valid** required tool id of the record in order: if it equals one of the four selector tool ids, add **3** (once per required id); else if it is the wildcard **`<Any>`** and fewer than `n` wildcards were used so far, add **2** and count it; **otherwise the record is rejected (score -1)**. An empty tool list scores 0.

**Level of a field** (record value `r`, selector value `s`): **2 if `r == s`, 1 if `r` is the wildcard (ANY: -2 for pose, direction, speed, readiness and emotion; -1 for shield), else 0.**

For every scheme of the chain in turn and every "all" record **not rejected by the tool score**: compute the levels of pose, direction, speed, then the tool score, then readiness, shield, emotion. **The record replaces the best so far only when this 7-tuple is strictly greater, compared in the order pose, direction, speed, tool score, readiness, shield, emotion (lexicographic); on a complete tie the earlier record stays.** The best is carried over to the base schemes (the base scheme's records compete in the same search; a base record wins only by being strictly better). After each scheme: **if the best record equals the selector exactly in pose, direction, speed, readiness, shield and emotion (a literal match, not a wildcard) and its tool score is `3 x n`, the search stops** and that record is returned with **no body animation**.

After the last scheme (no exact record): if a best record exists it is returned (id, term flag, nominal velocity, reverse flag). **A best record that mismatches pose, direction or speed (level 0) is still returned**: the chooser never rejects on those; a scheme without any non-rejected record returns nothing (both ids invalid; the puppet then plays no moving animation). **Body animation:** looked up only when the best record's "no body override" flag is clear **and** at least one of these differs: tool score is not `3 x n`, readiness, shield or emotion of the record is not literally equal to the selector's. The body search (below) then runs over the same chain.

**Body search** (the body-record search; read). For every body record (all of the chain, base last): its tool score (the same function; -1 does **not** exclude it from the comparison), the levels of pose, readiness, shield, emotion. **Replace the best when the tuple (tool score, pose, readiness, shield, emotion) is strictly greater (lexicographic, tool score first); ties keep the earlier record.** At the end the best is accepted only if **its tool score is not -1 and none of its four levels is 0**; otherwise there is **no body animation**. (The body list has no direction or speed: it overlays the upper body for whatever legs the "all" animation plays.)

**The coverage of real schemes** (script section 5, reference implementation over every combination of 8 poses, 9 directions, 3 speeds, 2 readiness, 2 shield, 5 emotions with no tools): every scheme of the 20 returns an "all" record for all 4,320 selectors (none returns nothing); in the biped family (the base `BipedMoving.mas`) 240 of the 4,320 selectors match pose, direction and speed with level 2 on all three; `Kleer.mas`, `Tank.mas`, `Snowman.mas` and `MovingAnimationScheme.mas` never do (their records use wildcards). The player's, NPCs' and zombies' schemes add a body override for held weapons (for example `Player.mas` for all 4,320 selectors).

## 4. Choosing a pose-transition animation (`samGetPoseTransitionAnims(model, selector, to pose)`; read)

`PlayPoseChangedAnim(speed, from pose, to pose, playBody)` first runs `PrepareMovingAnimData(speed)` and then **sets the selector's pose to `GetMovingAnimPose(from pose)`**; the target `to` is `GetMovingAnimPose(to pose)`. The search is **strict**: a record is **skipped when any of its fields mismatches without being a wildcard** (from pose against the selector's pose, to pose against the target, direction, speed, readiness, shield, emotion) and also when its tool score is -1. Among the rest the key is **(level of to pose, level of from pose, direction, speed, tool score, readiness, shield, emotion)** compared lexicographically, and **a record replaces the best when its key is greater or equal: on a tie the later record wins** (the opposite of section 3). The scheme chain is walked the same way. The result is the id and the reverse flag of the best record; the body animation is searched as in section 3 under the same conditions (record's "no body override" clear and a difference in tool score, readiness, shield or emotion). **Nothing is returned when no record passes** (a puppet whose scheme chain has no matching transition plays none; of the 20 `.mas` files only `BipedMoving.mas` (12 records) and `Tank.mas` (1) have transitions; of the 40 embedded schemes 23 have them, 45 records in all).

## 5. The tool list and `<Any>`

The puppet's tools are the `CCharacterTool` ids of the attached tools (weapons and held objects, `puppet-entity-classes.md`). A record that names a tool applies only while that tool is among the first four attached ones. `<Any>` is the identifier spelled `<Any>`: it matches **as many held tools as there are wildcards in the record, up to the number of tools** the puppet has (so a record with `<Any>` applies to a puppet holding at least one tool, at a lower score than a named tool: 2 against 3). Examples in the data: the player's weapon poses name the weapon's right-hand tool; `NPCs.mas` bodies name `NPC_Bazooka_R`, `NPC_Shotgun_R`, `NPC_Zapgun_R`; 7 body records and 2 "all" records use `<Any>`.

## 6. Playing: `PlayMoveAnim`, `PlayPoseChangedAnim`, `PlayDefaultAnimation`

**Animation groups and modes** (the fifth and second arguments of the model's `PlayAnimation(id, mode, speed, strength, group)`; also in `puppet-attacks.md`): **group 1 = the "all" animation, group 2 = the body override, group 3 = attack and action animations**; mode values are flags: 0 once, 1 loop, 4 reversed (5 = loop reversed), 2 used by melee (meaning not resolved).

**`PlayMoveAnim(speed)`** (read), only while `en_bAutoAnimations` is set and the puppet's parameters are legged parameters:

1. Store `speed` in `en_fLastTestedMoveSpeed`; ask the chooser (sections 2 and 3) for the all-animation, term flag, reverse flag, body animation and nominal velocity `V`.
2. **Nothing** if both ids are invalid.
3. **Same animation as before** (all id and body id equal the last ones, the "last move animation was playing" flag set, the reverse flag equal): only if `V` differs from the last `V` **or `|speed - last speed| >= 0.25`**: store both and **adjust the playing animation's speed to the factor `f`** (the played animation is found in the animation queue and `aniAdjustSpeed` applied with the current time); otherwise nothing happens.
4. **Otherwise** (new animation): new cloned animation state with the lerp time `en_fAnimLerpTime`, **remove groups 1 and 2**, set the "last move animation" flag, **if the term flag is set remap the id through the model's animation scheme (section 7)**, store the ids, `V`, the reverse flag and `speed`, then **play the all animation: mode 1 (loop), or 5 when the reverse flag is set, speed factor `f`, strength 1, group 1**, and **the body animation: mode 1, speed 1, strength 1, group 2**.
5. **Speed factor** `f = (speed / V) / stretch` when both `V` and `speed` are above 0, else 1; `stretch` is the puppet's stretch (`GetPuppetStretch`). So an animation whose nominal velocity is 6.4 m/s plays at 0.5 when the puppet moves at 3.2 m/s (stretch 1).

**`PlayPoseChangedAnim(speed, from, to, playBody)`** (read; `en_bAutoAnimations` and legged parameters required; returns 1 when an animation was started): clears the stored pose-change ids; chooses as in section 4; **if either id is valid**: clear the "last move animation" flag, new cloned state with the lerp time, **remove groups 1 and 2**; the transition animation is played **once (mode 0, or 4 when its reverse flag is set), speed 1, strength 1, group 1**, its id is stored as the pose-change and as the last all-animation id with its reverse flag, and **`en_tmPoseChangeEnd` := now + max(length - 0.2 s, 0.1 s)**; **only when `playBody` is set** and a body animation was chosen it is played once (mode 0, group 2). The step passes `playBody` 1 and the old and new poses.

**`PlayDefaultAnimation`** (read; called by `ResetDynamicMembers`, `SetupAspects` and `ThreatSoundHeard`): only when the puppet is **not riding**: set auto animations on, pose := `PP_STAND` (1), move direction := stop (5), clear the last-animation flag, prepare the selector with speed 0, call `PlayMoveAnim(0)`, **then set the pose member to `PP_FALL` (6)** (a stored oddity: the next step sees a pose change), new cloned state with lerp 0, and **play the last all and body animations in loop mode (1) at speed 1** in groups 1 and 2.

**When the movement step calls them** (`LerpToDesiredVelocityAndLook`, read only for these calls; its own rules are T01-141):

- **The animation speed value** `v`: 0 unless the desired moving speed is above 0.01 and the puppet is moving; then the **desired moving speed** (`GetDesiredMovingSpeed`: **the current nominal velocity x the length of the desired tempo vector** (horizontal; 3-D when the puppet moves in a 3-D area; for move type `MT_DRIVE_STEER_LOOK` the absolute forward tempo component), and for a puppet in air control multiplied by the air-control velocity multiplier), **halved when the pose is crouch** (`PP_CROUCH` 2).
- **Direction** `en_mdMoveDirection` of the step: **when moving: the quadrant of the desired tempo in the body frame (a quadrant function of the puppet: 0 gives `MD_FORWARD`, 1 `MD_LEFT`, -1 `MD_RIGHT`, anything else `MD_BACKWARD`; **medium-low**); when standing (v = 0): by the body's turn angle of this step in radians: above 0.001 `MD_TURNLEFT`, below -0.001 `MD_TURNRIGHT`, else `MD_STOP`** (the turn angle's sign convention: a positive step turns left).
- **Pose change:** when the new pose differs from the old one the step calls `PlayPoseChangedAnim` (after a few special cases that stop or restart the physics buoyancy for dive/swim poses) and remembers whether it started an animation.
- **Move animation:** the selector is saved as the "last" selector, `PrepareMovingAnimData(v)` rebuilds it, and **if the speed class changed but `|v - the last PlayMoveAnim speed| < 0.25` the old speed class is kept** (hysteresis on the class). Then **`PlayMoveAnim(v)` is called unless a pose-change animation was started in this step, when at least one holds: `en_tmPoseChangeEnd` lies in the last simulation step (from now minus one step up to now), `v` differs by more than 0.01 from the previous step's value, or the new selector differs from the last one (`IsEqualTo` false)**.

## 7. `samRemapSchemeAnim` and the animation scheme

An id with the **term flag** (or any scheme animation name used by script, melee and shooting: `puppet-attacks.md`) is looked up in the model's **animation scheme** (`CAnimationScheme`, the `.sca` file or embedded, the third member of the custom model data). `samGetSchemeAnim(model, term)`: **no custom data, no scheme or an invalid id: invalid; search the scheme's term list by id; if absent continue in the scheme's base scheme (recursively)**; a found entry is a **weighted list** of animation names (`CWeightedAnimInScheme`: weight and name): one entry is returned as is (no random draw); several: one number is drawn from the global generator (`mth_ulRandom`) in [0, total weight) and **the first entry whose running weight sum exceeds it is returned** (the stored weights are all 1: `misc-types.md`). `samRemapSchemeAnim(model, id)` returns the **scheme result, or the original `id` when the term is not found** (so a plain animation name passes through unchanged); with no model or an invalid id it returns invalid. (**Medium**; the draw's exact scaling was read in outline.)

## Invariants

1. `CMovingAnimData` is 10 words and `IsEqualTo` compares them all; the four tool slots are filled in the attached-tool order and cut at 4.
2. The "all" search never returns a record with a tool score of -1; the pose-transition search never returns a record with any mismatching field; the body search never returns a record with a level 0 or tool score -1.
3. Of the 167 stored "all" records none has more than 2 tool ids; 6 have the term flag; 1 has the reverse flag; 34 disable the body override.
4. In the data every scheme has at least one record of the "all" list or a base that has (the 20 `.mas` files: `HellChick.mas`, `MartialArts.mas` and `SciFiSolider_Destroyer.mas` have no "all" record of their own and depend on `BipedMoving.mas`).

## Open questions

1. **The movement step's own rules** (the direction and speed value, pose-change special cases, buoyancy multipliers, jump, air control): T01-141.
2. **The meaning of the play modes** other than 0, 1, 4 (the melee value 2) and of the strength argument: **T01-265**.
3. **The `.sca` scheme contents** (11 term names, the weighted lists per model) and the draw's scaling: **T01-265**.
4. **Run-time check** of the hysteresis constants (0.25, 0.01), the 0.2 s and 0.1 s of the pose-change duration, and the "first best wins / last wins" tie rules on a real puppet: **T01-266** (with T01-146).
5. The members of `SPuppetEnvironmentParams`: T01-214 (the slow limit and the min/max move speed are given here).

## Test vectors

From `tools/re/moving_anim_verify.py` (scheme `BipedMoving.mas`, no tools, normal emotion, no shield):

- stand, forward, fast, not ready -> **`Run`** (nominal 6.416; its direction is ANY, so the match is partial); stand, forward, slow, ready -> **`Walk`** (2.017); stand, backward, fast -> **`BackPedalRun`** (6.2); stand, not moving, idle -> **`Idle`** (term flag set); crouch, forward, slow -> **`CrouchWalk`** (1.037); stand, left, fast -> `StrafeRunLeft`.
- Pose change stand -> crouch: **`Crouch`** forward; crouch -> stand: **`Crouch`** with the reverse flag (played in mode 4).
- Selector from state (`PrepareMovingAnimData`): standing, not moving (`MD_STOP`), land slow limit 5, no foe -> pose 0, speed class 2 (idle), direction `MAD_NOT_MOVING` 8, readiness 0, tools none. Walking at 3 m/s with a foe, direction forward -> speed class **1 (slow)**, readiness **1**. Walking at **exactly 5.0** (the slow limit) backward -> speed class **0 (fast)**, direction 1.
- Speed factor: `Run` (6.416) at speed 3.208, stretch 1 -> `f` = 0.5; the same puppet with stretch 2 -> 0.25; `Idle` (nominal -1) -> 1.
- Hysteresis: last `PlayMoveAnim` speed 3.0, now 3.2, same animation -> the stored speed stays and **nothing is adjusted** (difference 0.2 < 0.25); now 3.3 -> the animation's speed is adjusted.
- Pose change end: `Crouch` of length 1.0 s -> `en_tmPoseChangeEnd` = now + 0.8 s; of length 0.25 s -> now + 0.1 s.
