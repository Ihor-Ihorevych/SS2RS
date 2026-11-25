# Behaviour actions and validity terms of `Sam2Game.dll` (T01-105, part of T01-026)

- **Roadmap task:** T01-105 (overview [`behavior-classes.md`](behavior-classes.md); the other parts are [`behavior-mover-classes.md`](behavior-mover-classes.md) and [`behavior-entity-classes.md`](behavior-entity-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class, member and enumeration names, for every count and stored value of the 29 saved action and term classes and for the member order against the binary's tables (`tools/re/behavior_classes_verify.py`); **medium** for what each action's `Execute` and each term's `IsValid` does (every one was read, once, **nothing was run**); **low** for the cases marked `P`/`N` (profiled or from the name), for the formula the brain uses to *choose* among actions (now in [`behavior-action-choice.md`](behavior-action-choice.md), T01-153) and for the open questions.

Prerequisites: [`behavior-classes.md`](behavior-classes.md), `ai.md` (the `.cb` file, moments, weights, bands), `misc-schema.md` (the same members as a generated table), [`brain-classes.md`](brain-classes.md) (the brain calls an action through `StartCharacterAction`), [`behavior-entity-classes.md`](behavior-entity-classes.md) (the behaviours the actions start), [`puppet-legged-classes.md`](puppet-legged-classes.md).

## Summary

A `.cb` file lists, per **moment**, weighted **actions**. An action is a small data object with an `Execute(brain)` function: it **does one thing to the brain or its puppet** and says whether it did it. Some actions carry a **validity term** (`IsValid`) that decides whether they may be chosen at all. This page covers the 30 action classes (27 `CBA*` classes, `SBAList` and the two bases `SBehaviorAction`) and the 11 term classes (`CBAVT*`) that `Sam2Game.dll` exports, with the data-only classes that go with them (`CBADefaultAction`, `CBANoAction`, `CBAList`, `CBAValidityTerm`, `CBAVTTermCombiner`).

Findings an implementer needs first:

1. **The 30 action classes split into four kinds** (section 2): **12 that act directly** on the brain or puppet (change state, set the action set, change abilities or alignment, emotion, shield, crouch, stop moving, die, play a sound, throw a macro event, throw a picked tool); **eight that spawn a generic behaviour** (`CBAAnimation`, `CBAShoot`, `CBAMelee`, `CBALocalMove`, `CBARunOverFoe`, `CBALeapOntoFoe`, `CBAWait`, `SBAList`; they take time, and the seven but `SBAList` answer `IsLatentAction` true); **eight that control the foe, the tactic and the mover**; and `CBATryRiding`, which has no function of its own. (The 30 include the base `SBehaviorAction`.)
2. **`SBAShoot` is the commonest stored action**: 726 objects, **112 in 62 `.cb` files and 614 in 20 worlds** (embedded in the moment handlers of placed objects, section 1); 38 distinct shot ids (`Shoot` 597, `TankRocket` 53, `LaunchRockets` 12, `ThrowBall` 9, `Laser` 6 ...); the number of shots is drawn from `sp_ctMinShoots` .. `sp_ctMaxShoots`; the two **count multiplier graphs are null in every stored action** (726 of 726), so only the "shoot while moving" graph decides anything (643 set over distance ratio, 22 over area ratio).
3. **`CBAFoeAssigning` is saved as `CBAPlayerAssigning`** (inference, **medium**): the binary's table registers the class under that name with exactly the two booleans (`pa_bAllowPlayerToBeFoe`, `pa_bAllowNonPlayerToBeFoe`) that `CBAFoeAssigning::Execute` turns into "enable or disable player foes" and "enable or disable non-player foes" on the foe manager. The survey count of "45 classes in the data" counts by name and therefore misses this one.
4. **Validity terms are small tests of the situation**; the data uses 7 of the 11 classes (`SBAVTHealth` 51 objects, `CBAVTFoeIsPlayer` 29, `CBAVTHasTool` 9, `CBAVTPlayerDistance` 9, `CBAVTFoeAngle` 6, `CBAVTCurrentStateDuration` 3, and the combiner 3) and **46 of 51 `SBAVTHealth` use the mode "every N damage"** (`HO_DROPPED_EVERY`): the wound reaction of most enemies is a damage threshold repeated every N health, not a one-off.
5. **`HealthObservation`, `EBrainAligment` and `FoeHostility` have names in the binary** that settle the "names of the values are not in the data" of `ai.md`: alignment 0 `BA_GOOD` ("Good"), 1 `BA_EVIL` ("Evil"), 2 `BA_NEUTRAL` ("Neutral"), 3 `BA_SEMI_NEUTRAL` ("Semi neutral"); `TermCombiner`: 0 `TC_AND`, 1 `TC_OR` (all 3 stored combiners are AND).

## Evidence

- **Exported names.** 41 classes with 383 exported functions (284 on the 30 action classes, 99 on the 11 terms); 28 `Execute`, 11 `IsValid`, 37 `mdGetNameScript`, 12 `GetDescription`, 7 `IsLatentAction`, `mdGetLMActionDescription` (`behavior-classes.md` section 1).
- **Data.** `tools/re/behavior_classes_verify.py` (about 10 minutes; reads 82 `.cb`, 322 `.ep`, 59 `.wld`) prints for each saved class: base class, member count, files, objects, and for every member the distinct values, range and commonest values; and that the member lists of the 11 saved action and term classes whose table the binary declares equal the binary's names in the same order (11 of 11; with `CBAPlayerAssigning` and the two tactic classes 14 of 14).
- **Methods.** All 28 `Execute` and all 11 `IsValid` were decompiled and read (`R`); `CBAVTFoeAngle::IsValid` and the long `CBAShoot::Execute` and `CBARunOverFoe::Execute` were read in outline (`R-`). The editor strings (`mdGetNameScript`, `GetDescription`, `mdGetLMActionDescription`) were extracted as text.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared.
- **Not checked.** Nothing was run; the engine's choice among the actions of a list (weights, validity, frequency graphs) is not in these classes (specified in [`behavior-action-choice.md`](behavior-action-choice.md), T01-153). `CBATryRiding` has no function of its own (only the 7 standard ones) and no member table.

## Layout / rules

### 1. Common to all actions

Every action derives from the data class `CBehaviorAction` (**the data's base, not exported**) with five members, stored for every action:

| Member | Meaning | Values in the data (all action objects) |
|---|---|---|
| `ba_pcValidityTerm` | a pointer to a validity term, or null | in the `.cb` files null in 749 of 853, set in 104; in the 630 world-stored actions always null |
| `ba_fWeight` | the weight in the random choice | 0 .. 100,000; 1.0 in the large majority (`CBAShoot` 705 of 726, `SBAAnimation` 121 of 128); 10 for 3 sounds; see `ai.md` |
| `ba_pgrWeightOverHealthMultiplier`, `ba_pgrWeightOverDistanceMultiplier`, `ba_pgrWeightOverAreaRatioMultiplier` | optional weight multiplier graphs | **null in all 853 `.cb` actions and in all 630 world actions** |

(853 is the number of action objects in the 82 `.cb` files, `misc-schema.md`; the script's `.cb` counts of section 4 add up to the same 853. The worlds hold 630 more, all `SBAShoot` (614) and `SBAAnimation` (16), inside `CAIMomentHandler` objects (564 in 23 worlds, 523 `CBehaviorActionList`s in 22 worlds); the same five members, the same all-null graphs, term null in every one of them.) The base's exported part is `SBehaviorAction` (7 standard functions, saved in 46 `.cb` files as the parent of `CBACrouch`, `CBAPlaySound`, `CBAStopMoving`, `CBAThrowPickedTool`, `SBAChangeState`, `SBADie`, `SBAEmotion`, `SBAShieldStatus`, `SBAThrowMacroEvent`: a second level of the data hierarchy that adds no member). The five virtual hooks of an action (read from the table of `CBAShoot` and `SBAList`): `Execute(brain)`, `GetDescription` (the editor line), `mdGetNameScript` (the editor name), `IsLatentAction` and the destructor; the base `IsLatentAction` is false.

`Execute` takes the entity that runs the action, **casts it to a brain** (`CBrainEntity`) and returns **true when the action did something, false when it could not**; the brain's `StartCharacterAction` records the action as started or rejected (`brain-classes.md` 3.2). **Every action acts through the brain's `en_henPuppet`** (the puppet it controls), except those that act on the brain itself.

**Net behaviour.** Actions are data objects, not entities; they run on the host with the brain (`replication.md`: "AI decisions exist only on the host"). Nothing here is replicated.

### 2. The action classes (`Execute`, **medium**)

"Basis": `R` read in full, `R-` read in outline, `P` profile, `N` name. "Data" gives the saved name, the number of objects and the files (`.cb` unless said). All actions return false when the brain has no puppet.

**2.1 Actions that spawn a generic behaviour** (they ask the brain to `PrepareForGenericBehavior` or not, build the properties object of the matching behaviour class, spawn it, and hand it to `StartGenericBehavior(behaviour, moveDuring)`; `moveDuring` is the second argument below). If the spawned behaviour reports that the puppet refused (`en_bCanPerformAction` false, `behavior-entity-classes.md` 3.1), the brain is told `GenericBehaviorNotStarted`, the behaviour is deleted and `Execute` returns false. `IsLatentAction` is true for the seven in **bold**.

| Class (data) | Members (data names) and values | What `Execute` does | Basis |
|---|---|---|---|
| **`CBAAnimation`** (`SBAAnimation`, 128 objects: 112 in 56 `.cb` files, 16 in 2 worlds) | `baa_idAnimation` (20 distinct: `Wound` 46, `Sight` 21, `Happy_Jump` 16, `Threat` 14, `BangingChest` 3, ...), `baa_bMoveDuringAnimation` (0 in 126, 1 in 2) | spawns `CGBAAnimationBehavior` with the animation id (a name of the model's **scheme**: `mdl.md`, `mat.md`); `moveDuring` = `baa_bMoveDuringAnimation` | R |
| **`CBAShoot`** (`SBAShoot`, 726 objects: 112 in 62 `.cb` files, 614 in 20 worlds) | `as_idShoot`, `sp_ctMinShoots`, `sp_ctMaxShoots`, two "shoot while moving" graphs, two count multiplier graphs (section 2.5) | spawns `CGBAShootBehavior` with the shot id, a drawn shot count and the stop-to-shoot flag (2.5) | R- |
| **`CBAMelee`** (`SBAMelee`, 67 objects, 42 files) | `mel_idMeleeAttack` (11 distinct: `Melee` 26, `HandKick` 13, `Kick` 11, `Slap_LeftHand` 3, `Slap_RightHand` 3, ...) | spawns `CGBAMeleeBehavior`; `moveDuring` false | R |
| **`CBALocalMove`** (`SBALocalMove`, 7 objects, 2 files) | 13 members (2.4) | refuses unless the brain's **mover reason is `MR_AREA`** (1): a local move cannot interrupt a forced, tactic or navigation mover; then spawns `CGBALocalMoveBehavior` with a pointer to itself; `moveDuring` false | R |
| **`CBARunOverFoe`** (`SBARunOverFoe`, 10 objects, 7 files) | `rof_idRunOverFoe` (`RunOverFoe` 7, `JumpAttack` 2, `Kick` 1), `rof_fRunOverFoeTimeMin` (1.5 or 2), `rof_fRunOverFoeTimeMax` (1.5 or 3), `rof_fRunTroughDistance` (7 in 7, 5 in 3) | looks the attack up by id in the puppet's parameters (a `CRunOverFoeAttack` record, `ep-schema.md`; when missing: console text `Attack of type run over foe with ID "<id>" can't be found`), copies its numbers into the properties of `CGBARunOverFoeBehavior` with a run time drawn between the record's minimum and maximum, and starts it; `moveDuring` false | R- |
| **`CBALeapOntoFoe`** (`SBALeapOntoFoe`, 4 objects, 3 files) | `lof_idLeapAttack` (`Jump` 2, `JumpMaiGeri` 1, `JumpAttack` 1) | spawns `CGBALeapOntoFoeBehavior` | R |
| **`CBAWait`** (`CBAWait`, 14 objects, 5 files) | `baw_bMoveDuringWait` (0 in 11, 1 in 3), `baw_fMinPeriod` (0.5 in 14), `baw_fMaxPeriod` (0.5 in 9, 1.5 in 5) | draws a wait time uniformly in [min, max], spawns `CGBAWaitBehavior`; `moveDuring` = `baw_bMoveDuringWait` | R |
| `SBAList` (`SBAList`, 42 objects, 10 files; base `CBAList`) | `bal_strName` (editor name: 26 empty, `Stop uising tactic and set frightened emotion` 4 [sic], `Reuse tactic and set normal emotion` 4, `Throw health and switch to default state` 3, `MeleeAndPause` 2, ...), `bal_abaActions` (lists of 2 in 22, 3 in 12, 1 in 5, 6 in 2, 7 in 1) | spawns `CGBActionListBehavior` holding a copy of the list; `moveDuring` false. **Not latent** (no `IsLatentAction`), although it takes time | R |

**2.2 Actions that change the brain or the puppet directly** (all return true unless said):

| Class (data) | Members and values | Effect | Basis |
|---|---|---|---|
| `CBAChangeState` (`SBAChangeState`, 36, 5 files) | `bacs_idNewState` (9 ids: `Default` 11, `ThrowItem` 9, `Frightened` 6, `GreetPlayer` 3, `IgnorePlayer` 3, ...) | the brain's `SetDesiredState(id)`: the `.cb` state changes at the next think point (`brain-classes.md` 3.2) | R |
| `CBASetActionSet` (binary only; table `SBASetActionSet`: `sas_idNewActionSet`) | the moment name | sets the brain's current action set (the moment `Action` is replaced) | R |
| `CBAChangeMovingAbilities` (`CBAChangeMovingAbilities`, 5, 4 files) | `bacma_maToAdd` (stored 0 in 3, 2 in 1, 1 in 1), `bacma_maToRemove` (40 in 3, 1, 2) | the puppet's movement abilities become `(abilities OR add) AND NOT remove`; `MoveAbilities` bits: `MA_WALK` 1, `MA_FLY` 2, `MA_SWIM` 4 ("Diving"), `MA_CANSTRAFEH` 8, `MA_CANSTRAFEV` 16, `MA_CANMOVEBACKWARD` 32: **40 = strafe horizontally + move backwards** | R |
| `CBAChangeAligment` (binary only; `SBAChangeAligment`: `ca_baNewAligment`) | `EBrainAligment` | the puppet's `SetAlignment` | R |
| `CBAEmotion` (`SBAEmotion`, 8, 3 files) | `bae_ceEmotion` (`CE_FRIGHTENED` 0 in 4, `CE_NORMAL` 1 in 4), `bae_fAutoOffTime` (-1 in 8) | sets the legged puppet's emotion member and, when `bae_fAutoOffTime` is above 0, the time at which it is reset (`en_tmResetEmotionState`, `puppet-legged-classes.md`); for a puppet that is not legged it prints `Emotion state behavior action is valid only for legged puppets.` and returns false | R |
| `CBAShieldStatus` (`SBAShieldStatus`, 3, 2 files) | `bass_cssChieldStatus` (sic; `CSS_SHIELDED` 1 in 2, `CSS_NOT_SHIELDED` 0 in 1), `bass_fAutoOffTime` (-1) | the same for the shield status (`en_cssShieldStatus`, `en_tmResetShieldState`); text `Shield state behavior action is valid only for legged puppets.` | R |
| `CBACrouch` (`CBACrouch`, 4, 3 files) | none | one puppet call with the arguments (0, -1.0, 0); the slot resolves to `SetDesiredTempo`, the effect "crouch" is the editor name | R-/N (**low**) |
| `CBAStopMoving` (`CBAStopMoving`, 3, 3 files) | none | the puppet's `StopMoving` | R |
| `CBADie` (`SBADie`, 5, 3 files) | none | the puppet's `ApplyLethalDamage` with the default damage-type constants (the puppet dies) | R |
| `CBAPlaySound` (`CBAPlaySound`, 47, 45 files) | `bacs_idSoundInScheme` (`Sight` 42, `War` 2, `Wound` 1, `Taunt` 1, empty 1), `bacs_pbsSound` (null in 47), `bacs_fVolume` (1), `bacs_fPitch` (1), `bacs_fPitchRnd` (0 in 46, 0.25 in 1) | when the scheme id is valid: `SaySchemeSound(id)` (a sound of the model's sound scheme, `mdl.md`); else, with an explicit sound resource, `SaySound(sound, volume, pitch)` | R |
| `CBAThrowMacroEvent` (`SBAThrowMacroEvent`, 1, 1 file) | `tme_idMAcroEvent` (sic; `Behavior_ToolThrown`) | sends that macro event with the puppet as source (`macSendEvent`, `level-logic.md`) | R |
| `CBAThrowPickedTool` (`CBAThrowPickedTool`, 3, 3 files) | `batt_idThrowDescription` (`ThrowItem` in 3) | with no id: the puppet's `ThrowPickedTool`; with an id: `ThrowPickedToolAtTargetEntity` (a `CThrowDescription` of the character tool) | R |

**2.3 Foe, tactic and mover control**:

| Class (data) | Members and values | Effect | Basis |
|---|---|---|---|
| `CBAFoeAssigning` (data `CBAPlayerAssigning`, 12 objects, 3 files) | `pa_bAllowPlayerToBeFoe` (1 in 9, 0 in 3), `pa_bAllowNonPlayerToBeFoe` (1 in 9, 0 in 3) | the brain's foe manager `EnablePlayerFoes`/`DisablePlayerFoes` and `EnableNonPlayerFoes`/`DisableNonPlayerFoes` by the two flags; false when the brain has no foe manager | R |
| `CBAForcePlayerFoe` (`CBAForcePlayerFoe`, 9, 3 files) | `bafpf_fhFoeHostility` (`FoeHostility` 0, 6, 9, 3 objects each), `bafpf_fAutoOffTime` (-1) | takes the **closest player at any distance** to the puppet and calls the brain's `AssignFoe(player, hostility, autoOffTime)`; false when no player exists. Editor name: `Force closest player as foe` | R |
| `CBALoseFoe` (`CBALoseFoe`, 12, 3 files) | none | the brain's `LoseFoe` | R |
| `CBAForceMovingType` (binary only; `SBAForceMovingType`: `bafmt_pmcForcedMovingType`, `bafmt_fAutoOffTime`) | a mover and a time | the brain's `ForceMovingType(mover, time)`: reason `MR_FORCED`; editor line `Force moving type: <mover name or NULL>` | R |
| `CBAUnforceMovingType` (binary only) | none | the brain's `UnforceMovingType` | R |
| `CBAStopTactic` (binary only) | none | when the brain's mover reason is `MR_TACTIC` (3) the reason becomes `MR_NONE` (0); nothing else | R |
| `CBAStopUsingTactic` (`CBAStopUsingTactic`, 7, 3 files) | none | clears the puppet's **desired**-tactic member (`en_henDesiredTacticEntity`) and, when the mover reason is `MR_TACTIC`, deletes the brain's current mover (corrected by T01-127, `brain-movement.md` 3.5) | R |
| `CBAReuseTactic` (`CBAReuseTactic`, 7, 3 files) | none | copies the puppet's **default**-tactic member into its **desired**-tactic member, so the next `MoveCharacter` takes it (corrected by T01-127) | R |
| `CBATryRiding` (binary only) | none declared | no function of its own; **not resolved** | N |

**2.4 `SBALocalMove` members** (13, the binary's names in the data's order; 7 objects in 2 files): `balm_fHeading` (6 values: 90 ×2, -45, 45, 0, 30), `balm_fFwdVelocityRatio` (-0.75 ×3, 0.75, 1.5, 1.0, 2.0), `balm_fUpVelocityRatio` (0), `balm_fVelocityRatioClamp` (1), `balm_bForcePose` (1 in 2), `balm_mapPose` (`MovingAnimationPose`: 0 `MAP_STAND` in 5, 6 `MAP_ROLL` in 2), `balm_bForceReadiness` (0), `balm_marReadiness` (`MAR_NOT_READY` 0), `balm_bDisableStrafing` (0), `balm_bLookAhead` (0), `balm_fWaitBeforeMove` (0), `balm_fAutoOffTime` (0.6 in 2, 0.75, 1.0 in 4), `balm_fWaitAfterMove` (0 in 5, 0.2 in 2): **a short scripted step** (read from the names, **low**): move at a heading and a forward velocity ratio for `balm_fAutoOffTime` seconds (a negative forward ratio, -0.75 in 3 of the 7 stored actions, would be a step back). The editor line is `Local move (<description>)` with the pose, `look ahead` and `no strafing` words. `MovingAnimationPose` values 0 `MAP_STAND`, 1 `MAP_CROUCH`, 2 `MAP_DIVE`, 3 `MAP_SWIM`, 4 `MAP_FLY`, 5 `MAP_FALL`, 6 `MAP_ROLL`, 7 `MAP_KO`, 8 `MAP_LADDER`, 9 `MAP_PARACHUTE`, 10 `MAP_DEATH`; `MovingAnimationReadiness` 0 `MAR_NOT_READY`, 1 `MAR_READY` (the `-1` and `-2` wildcards `INVALID` and `ANY` exist).

**2.5 `CBAShoot::Execute` (`R-`)**: reads like this. The shot count is a random integer from `sp_ctMinShoots` to `sp_ctMaxShoots` (at least 1; **the count multiplier graphs, when set, scale it by the value at the distance ratio and the area ratio of the brain's action-selection record; all 726 stored graphs are null, so this branch is unused**). The **stop-to-shoot** flag is drawn at random: a "shoot while moving" graph (over the distance ratio, else over the area ratio) is evaluated, its value clamped to [0, 1] and called `p`; the puppet **stops** to shoot with probability `1 - p` (the draw is compared with `1 - p`: **medium-low**; the default when both graphs are null was not resolved: in the data 83 stored shoot actions have a null distance graph, 704 a null area graph). `moveDuring` passed to the brain is the opposite of the stop flag. The behaviour is spawned with the shot id, the count and that flag.

### 3. The validity terms (`IsValid(selection record)`, **medium**)

Every term derives from the data class `CBAValidityTerm` (not exported; 1 member, `vt_bNotTrue`: **whoever evaluates a term negates its `IsValid` when the flag is 1**: the list selection and the combiner; `IsValid` itself never reads it, T01-153 `behavior-action-choice.md` section 5). `IsValid` takes the brain's **action-selection record** (`CActionSelectionData`; its first member is the brain) and returns whether the term holds. The classes:

| Class (data) | Members and values | Test | Editor name | Basis |
|---|---|---|---|---|
| `CBAVTHealth` (`SBAVTHealth`, 51 objects, 51 files) | `vth_hoObservation` (`HealthObservation`: **4 in 46**, 2 in 5), `vth_slValue` (23 distinct, 1 .. 10,000: 10 ×11, 20 ×7, 30 ×5, 200 ×3, 80 ×3), `vth_bAccumulative` (0 in 50, 1 in 1) | by mode, with `h` = the puppet's health, `v` = value, `d` = the damage of the last hit (the puppet's last-damage member), `N` = value: `HO_BELOW` 0: `h < v`; `HO_ABOVE` 1: `v < h`; `HO_DROPPED_BELOW` 2: a time test against the puppet's last-damage time (polarity not resolved), `h < v` and `v < h + d` (the last hit crossed the value downward); `HO_HEALED_ABOVE` 3: read as `v < h` plus a second test against `h + d` whose polarity was not resolved; `HO_DROPPED_EVERY` 4: with `vth_bAccumulative` the accumulated damage counter is above `v`; without it, the number of whole `N`-steps lost from the nominal health grew with the last hit (a new multiple of `N` of lost health was crossed); `HO_HEALED_EVERY` 5: `(h + d) mod N < h mod N` (crossing a multiple upward). Labels: `Is below`, `Is above`, `Dropped below`, `Healed above`, `Every N damage`, `Every N healed` | `HealthObservation` | R- (modes 2, 3 and 5 were read once; the exact sign of `d` in the modes **low**) |
| `CBAVTCurrentStateDuration` (3 objects) | `csd_fCurrentStateDuration` (10.0 in 3) | the time since the brain's `.cb` state began (`en_tmAIStateStarted`) is above the value | `Current state duration` | R |
| `CBAVTFoeAngle` (6 objects, 2 files) | `fa_fHeadingMin` (-180 ×4, -90 ×2), `fa_fHeadingMax` (180 ×4, 90 ×2), `fa_fPitchMin` (-90 ×6), `fa_fPitchMax` (-55 ×4, 90 ×2); `vt_bNotTrue` 1 in 4 | the direction from the puppet to its foe, turned into the puppet's frame, has its heading and pitch in the two ranges (degrees; the sign convention was not resolved); false without a foe | `Foe angle (%1<H<%2 %3<P<%4)` | R- (**medium-low**) |
| `CBAVTFoeIsPlayer` (29 objects, 4 files) | none; `vt_bNotTrue` 1 in 16, 0 in 13 | the brain has a foe and the foe's puppet is a player puppet | `Is player foe` | R |
| `CBAVTHasTool` (9 objects, 3 files) | `csd_idTool` (`GenericEntity` 6, `GenericItem` 3) | the puppet has an attached tool with that id | `Has tool: <id>` | R |
| `CBAVTPlayerDistance` (9 objects, 5 files) | `pd_fPlayerDistance` (8 ×3, 30 ×2, 10 ×2, 50, 20); `vt_bNotTrue` 1 in 1 | a player puppet exists within that distance of the puppet (the closest-player query with that radius) | `Closest player distance` | R |
| `CBAVTTermCombiner` (data only; 3 objects) | `tc_pTermA`, `tc_pTermB` (both set in 3), `vtr_tcTermCombineOperation` (`TermCombiner`: `TC_AND` 0, `TC_OR` 1; stored 0 in 3) | both sub-terms are evaluated, each negated by its own `vt_bNotTrue`; AND / OR; false when a sub-term is missing or the operation is neither (`behavior-action-choice.md` section 5) | - | R (T01-153, **medium**) |
| `CBAVTHasFoe` (not in data) | - | the foe manager has a foe | `Has foe` | R |
| `CBAVTHasFoeCandidate` (not in data) | - | the foe manager's `en_bHasFoeCandidates` | `Has foe candidates` | R |
| `CBAVTKnockOut` (not in data) | - | the foe's puppet is in knock-out (`IsInKO`) | `Foe is in KO` | R |
| `CBAVTMeleeSuccess` (not in data) | - | reads one boolean member of the foe's puppet (what it records: not resolved) | `Melee success` | P (**low**) |
| `CBAVTWarMeasure` (not in data) | two members | the world info entity's "war measure" for the second member's id is at least the first member | `War measure` | P (**low**) |

The data's terms: of 104 non-null `ba_pcValidityTerm`, the script prints which term class hangs on which action class (section 6 of its output).

### 4. Editor strings (the binary's own text)

`GetDescription` (12 classes) is the one-line description in the editor's action list; `mdGetNameScript` (37) the short name. They confirm the actions' roles: `Foe assigning`, `Force moving type: <name>`, `Force closest player as foe`, `Leap: "<id>"`, `Local move (<...>)`, `Lose foe`, `Melee: "<id>"`, `Reuse tactic`, `Run over foe`, `Set action set: <id>`, `Shoot: "<id>"`, `Stop tactic`, `Stop using tactic`, `Throw macro event: <id>`, `Throw picked tool: <id>`; for the animation, change-state and play-sound actions the id or the sound's file name; `CBAWait` and `CBAAnimation` print their parameters. They are not part of play.

## Invariants

1. 29 of the 41 classes of this page are saved: 17 under the exported name (including `SBAList` and `SBehaviorAction`, which are exported with the `S` name) and 12 as the `S` twin of a `C` name (`SBAAnimation`, `SBAChangeState`, `SBADie`, `SBAEmotion`, `SBAShieldStatus`, `SBAThrowMacroEvent`, `SBALeapOntoFoe`, `SBALocalMove`, `SBAMelee`, `SBARunOverFoe`, `SBAShoot`, `SBAVTHealth`); 12 are binary only (7 actions, 5 terms; the script prints each).
2. For the 11 saved classes whose table the binary declares, the data's member names equal the binary's in name and order (11 of 11).
3. `ba_pgrWeightOver...Multiplier` is null in every stored action, `ba_fWeight` is positive, `bafpf_fAutoOffTime` and `bae_fAutoOffTime` and `bass_fAutoOffTime` are -1 in every stored object.
4. `sp_pgrShootCountMultiplierOverDistanceRatio` and `...AreaRatio` are null in all 726 stored shoot actions.
5. Every `CBAVTTermCombiner` is an AND of two non-null terms (3 of 3).

## Open questions

1. **The choice among actions**: **answered by T01-153** in [`behavior-action-choice.md`](behavior-action-choice.md) (the three handler levels, the weighted draw, `vt_bNotTrue` and the combiner, the two special actions, the band period graph; **medium**, nothing run). Still open there: the run-time confirmation (T01-158), the empty moment id (T01-126), the instance handlers (T01-159).
2. **The generic-behaviour protocol** (what the puppet does for each request; `behavior-entity-classes.md`): **T01-152**.
3. **Validity term details**: the sign convention of `CBAVTFoeAngle`, modes 2, 3 and 5 of `CBAVTHealth`, the four terms the data does not use, and the use of `CBATryRiding`, `CBAChangeAligment`, `CBASetActionSet`, `CBAForceMovingType`, `CBAStopTactic`, `CBAUnforceMovingType` (none occurs in a stored file): **T01-157**.
4. **What `CBACrouch` calls** (the puppet method behind its slot) and the exact `CBARunOverFoe` data flow (action members against the puppet's attack record): **T01-152**.

## Test vectors

- **Hit counter**: an `SBAVTHealth` with mode 4 (`HO_DROPPED_EVERY`), value 20, not accumulative, nominal health 100: a hit that takes the puppet from 85 to 75 raises the lost health from 15 to 25, which crosses the multiple 20 (15 / 20 = 0 < 25 / 20 = 1 in whole steps), so the term holds; a hit from 79 to 75 (lost 21 -> 25) does not.
- **Abilities**: abilities 7 (walk, fly, swim), add 0, remove 40: result 7 (bits 8 and 32 were clear). Abilities 1, add 2 remove 1: result 2.
- **Alignment**: `BA_GOOD` 0, `BA_EVIL` 1, `BA_NEUTRAL` 2, `BA_SEMI_NEUTRAL` 3.
- **Shoot**: a `CBAShoot` with min 2, max 4, a shoot-while-moving graph value 0.75 draws a count in {2, 3, 4} and stops to shoot in about 1 of 4 cases.
- **Local move refusal**: a `CBALocalMove` while the mover reason is `MR_TACTIC` (3) returns false and starts nothing.
- `tools/re/behavior_classes_verify.py` prints, among others: `SBAShoot` 726 objects, `as_idShoot` 38 distinct values with `Shoot` 597; `SBAVTHealth` 51 with `HO` mode 4 in 46 and 2 in 5; `CBAPlayerAssigning` 12 objects; `CBANoAction` 192, `CBADefaultAction` 131.
