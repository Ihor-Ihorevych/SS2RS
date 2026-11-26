# The validity terms and the rarely used actions in detail (T01-157)

- **Roadmap task:** T01-157 (follow-up of [`behavior-action-classes.md`](behavior-action-classes.md) open question 3; the choice among the actions is [`behavior-action-choice.md`](behavior-action-choice.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the data counts and values of `tools/re/validity_terms_verify.py` (51 health terms, 6 angle terms, 12 foe-assigning objects, none of the unused classes in any file) and for the enumeration values; **medium** for every rule read from the binary (each is one decompiler reading with the comparisons of the health and angle terms checked in the instruction listing; **nothing was run**); **medium-low** for the Euler and heading conventions of the angle term and for the recording of the last damage when armor absorbs part of it (marked); **low** for the open questions.

Prerequisites: [`behavior-action-classes.md`](behavior-action-classes.md) (the term and action tables), [`behavior-action-choice.md`](behavior-action-choice.md) (who evaluates a term and negates it), [`foe-manager-numbers.md`](foe-manager-numbers.md) (the foe manager members the terms read), [`brain-movement.md`](brain-movement.md) (movers and reasons), [`puppet-legged-classes.md`](puppet-legged-classes.md) (knock-out).

## Summary

1. **`CBAVTHealth` is a test on the puppet's last hit** (section 2). With `h` the current health, `d` the **last damage amount** (`en_slLastDamage`, recorded with its time `en_tmLastDamage`), `before = h + d` and `v` the value: `HO_BELOW` `h < v`; `HO_ABOVE` `v < h`; **`HO_DROPPED_BELOW` (2): the hit happened at this very simulation instant and `h < v < before`**; `HO_HEALED_ABOVE` (3): `v < h` and `v < before` (no heal is looked at: the second part is implied by the first, so it is **just "above" with an extra condition that is always true**); **`HO_DROPPED_EVERY` (4): the number of whole `v`-steps lost since the nominal health grew by the last hit** (`(nominal - h) / v > (nominal - before) / v` in integer division, **false when `before` equals the nominal health**), or, with `vth_bAccumulative`, **the damage accumulated in the current burst is above `v`**; **`HO_HEALED_EVERY` (5): `h mod v > before mod v`**, i.e. the last hit **crossed a multiple of `v` downward** (it is the damage test of mode 4 in absolute multiples, not a heal test). The data uses **only modes 4 (46 objects) and 2 (5 objects)**.
2. **`CBAVTFoeAngle` compares the foe's direction with the puppet's own heading and pitch** (section 3): direction heading `atan2(-x, -z)`, pitch `asin(y)` of the world direction from the puppet to its foe (heading forced to 0 when the direction is within 0.99 of vertical), both **minus the puppet's own Euler heading and pitch**, wrapped to [-180, 180) degrees; true when **min <= heading <= max** and **min <= pitch <= max** (both ends inclusive). A foe straight ahead is (0, 0); the sign is **positive for a foe on the puppet's left** (the world's -X side when the puppet faces -Z). Used by 6 objects: the helicopter's "foe far below" (pitch -90 to -55) and the transformer's "foe in front" (heading +-90).
3. **The five terms the data does not use** (section 4) are small and exact: `CBAVTHasFoe` (the foe manager has a foe), `CBAVTHasFoeCandidate` (`en_bHasFoeCandidates`, **stale while tracking**), `CBAVTKnockOut` (the foe's legged puppet is in knock-out and not yet getting up), `CBAVTMeleeSuccess` (the **foe's** last generic melee attack hit its own foe), `CBAVTWarMeasure` (the highest "war measure" of the last N seconds is at least M; the measure is the **number of non-player puppets that currently have a foe**, sampled once per second into a 60-entry history).
4. **The seven actions no file stores under their own names** (section 5): `CBATryRiding` **does nothing and reports success**; `CBAChangeAligment` stores the new alignment in the puppet (no list is updated); `CBASetActionSet` stores the id the brain raises as its foe moment (default `Action`) for the rest of its life; `CBAForceMovingType` and `CBAUnforceMovingType` and `CBAStopTactic` are as in `brain-movement.md` 3.5; **`CBAFoeAssigning` is the class the data calls `CBAPlayerAssigning`** (two readings, section 5.2).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs. Read (decompiled; the comparisons of `CBAVTHealth::IsValid` and `CBAVTFoeAngle::IsValid` re-checked in the instruction listing, including the float constants): the eleven `IsValid`, the six actions' `Execute` and the vtable of `CBATryRiding`, `CPuppetEntity::OnReceiveDamage` (the parts that write the last-damage members), `OnStep` (the reset of the accumulated damage), `ResetDynamicMembers`, `OnAnimEventMeleeDamage`, `GenericMeleeAttackState`, `SetAlignment`, `GetHealth`, `GetNominalHealth`, `CLeggedPuppetEntity::IsInKO`, `CWorldInfoEntity::GetMaxWarMeasureLately`, `UpdateHostilityGraph`, `DefaultState`, `CBrainEntity::ForceMovingType`, `UnforceMovingType`, `ThinkPoint` (the action-set member), `CFoeManagerEntity::EnablePlayerFoes` and its three siblings. Member identities by offset against the DLL's member tables (names and order only). Six float constants (0.25, 0.99, 6.2831855, 3.1415927, 57.295776 and the time scale) were read from the data section.
- **Data.** `python -I tools/re/validity_terms_verify.py "<install>"` (about 1 second): section 1 to 4 of the data section below and the test vectors.
- **Not checked.** Nothing was run. The damage record `OnReceiveDamage` was read only where it writes the last-damage members; which damage kinds skip the accumulation and what damage kinds 19 to 21 are (damage-kind enumeration) was not resolved. `mthQuaternionToEuler`'s axis order (taken as heading, pitch, bank) and `CPlayerBrainEntity` terms were not read.

## Layout / rules

### 1. Evaluation (**medium**, `behavior-action-choice.md` 5)

`IsValid(record)` takes the action-selection record, **casts its first member to a brain** and returns **false** when that is not a brain (every term below says so once). `vt_bNotTrue` is **not** read by any `IsValid`: the evaluator (list selection, combiner) negates the answer. A term is evaluated when its moment is handled, so **terms without a time test stay true until the state they read changes**: this matters for modes 3, 4 and 5 of the health term.

### 2. `CBAVTHealth` (**medium**; the polarities checked in the listing)

Inputs (all of the brain's puppet): `h` = `GetHealth()` (an integer), `d` = `en_slLastDamage`, `before = h + d`, `nominal` = `GetNominalHealth()`, `t_d` = `en_tmLastDamage`, `a` = `en_slAccuDamage`; `v` = `vth_slValue`, `mode` = `vth_hoObservation`, `acc` = `vth_bAccumulative`. Everything is **integer arithmetic**; `/` truncates toward zero and `mod` is the C remainder.

| Mode | Result (strict comparisons) |
|---|---|
| 0 `HO_BELOW` | `h < v` |
| 1 `HO_ABOVE` | `v < h` |
| 2 `HO_DROPPED_BELOW` | `now == t_d` (the **same 64-bit time value**) **and** `h < v` **and** `v < before` |
| 3 `HO_HEALED_ABOVE` | `v < h` **and** `v < before` (the second part cannot fail when `d >= 0`) |
| 4 `HO_DROPPED_EVERY` | `acc` set: `v < a`; else: **false when `nominal == before`**, else `(nominal - h) / v > (nominal - before) / v` |
| 5 `HO_HEALED_EVERY` | `(h mod v) > (before mod v)` |
| other | false |

- **Division by zero**: modes 4 (not accumulating) and 5 divide by `v`; `v = 0` raises the processor's divide error (the data's smallest value is 1: 3 objects).
- **Mode 4 in words.** Without accumulation it counts how many **whole multiples of `v` the puppet's health has lost relative to its nominal health** before and after the last hit and is true when that count grew: `v = 20`, nominal 100: a hit 85 -> 75 (d = 10) takes the count from 0 to 1 (**true**); 79 -> 75 (d = 4) leaves it at 1 (false). **The first hit out of full health never triggers it** (`before == nominal` is the false case), however large (e.g. 100 -> 70).
- **Mode 4 accumulating** (1 stored object: `Cerberus`, value 50): true while the **damage added up in the current burst** is above `v`. `en_slAccuDamage` grows in `OnReceiveDamage` by the hit's damage amount (set to **1,000,000** when the hit is the lethal marker 2,147,483,647), except when a flag of the puppet's parameters is set **and** the damage kind is one of three kinds (the numeric kinds 19, 20 and 21 of the damage enumeration: not resolved), and is **reset to 0 by `OnStep` when more than 0.25 s have passed since `t_d`** (strict; also reset by `ResetDynamicMembers`). So a burst = hits less than 0.25 s apart.
- **Mode 5 in words.** `(h mod v) > ((h + d) mod v)` is true when a multiple of `v` lies in `(h, h + d]`: **the last hit crossed a multiple of `v` downward** (absolute health, not relative to the nominal health). `v = 20`: 25 -> 15 (true), 35 -> 25 (false). Its editor label ("Every N healed") is misleading; no stored object uses it.
- **Where `d` and `t_d` come from** (`OnReceiveDamage`, host only): the damage amount `a0` of the hit (unless it is the lethal marker) is multiplied by a puppet multiplier (the receive-damage multiplier, truncated toward zero, at least 1; **medium-low**); when the result `r` is positive: `t_d` := now, `d` := `r`, then **two thirds of `r` (integer division) go to armor and the rest to health**; armor that cannot take its share moves the deficit to health. So **`d` is the whole damage, not the health lost**: the terms' `before = h + d` equals the true health before the hit **only when the puppet has no armor** (121 of the 124 puppet files have maximum armor 0; with armor the terms over-estimate `before`; **medium-low**: the armor split was read in outline). `t_d` and `d` are not touched by healing.
- **Data** (script section 2): 51 objects, **mode 4 in 46** (values 1 x 3, 10 x 6, 20 x 7, 30 x 5, 35 x 2 ... 10,000 x 1; 1 accumulating), **mode 2 in 5** (value 10 in all five: `Kamikaze`, `SciFi_Destroyer`, `SciFi_Grunt`, `SciFi_Guard`, `SciFi_Gunner`, puppets with 15 to 40 health: "the hit that took it below 10"). No stored term uses modes 0, 1, 3 or 5; `vt_bNotTrue` is 0 in all 51. 50 of the 51 hang on an `SBAAnimation` (the wound animations) and 1 on a `CBAPlaySound`.

### 3. `CBAVTFoeAngle` (**medium**; the formula and constants checked in the listing; the Euler order **medium-low**)

False when the brain has no foe manager or no foe. Otherwise:

1. `p` = the brain's puppet placement (position and orientation), `f` = the foe brain's placement (the same as its puppet's: `brain-classes.md` 3). The puppet's orientation is turned into Euler angles `(H, P)` (heading, pitch; the bank is ignored).
2. `u` = `f - p` normalised (a zero vector stays zero).
3. Direction pitch `Pd = asin(clamp(u.y, -1, 1))`. Direction heading `Hd = atan2(-u.x, -u.z)`, **but `Hd = 0` when `u.y > 0.99` or `u.y < -0.99`**.
4. `rh = wrap(Hd - H)` and `rp = wrap(Pd - P)` with `wrap(a) = fmod(a + pi, 2 pi)`, `+ 2 pi` when negative, `- pi`: the range **[-pi, pi)**; both are then multiplied by 57.295776 (degrees).
5. **True iff `fa_fHeadingMin <= rh <= fa_fHeadingMax` and `fa_fPitchMin <= rp <= fa_fPitchMax`** (inclusive).

Convention: a direction straight ahead of a puppet with Euler heading `H` has direction heading `H`, so `rh = 0` ahead; with forward = -Z, `Hd` is **positive towards -X**, so **positive `rh` = the foe is to the left**; positive `rp` = the foe is above the puppet's pitch. A foe at the puppet's own position (zero direction) gives `Hd = -180` (`atan2(-0, -0)`) and pitch 0. **Data** (script section 2): 6 objects: `Helicopter` x 4 (heading -180 to 180, pitch -90 to -55, `vt_bNotTrue` 0, 1, 0, 1: "foe far below" and its opposite) and `Transformer` x 2 (heading -90 to 90, pitch -90 to 90, `vt_bNotTrue` 1: "foe is **not** in front"). Test vectors in section 8.

### 4. The five terms no file uses (**medium**; `R`)

| Class | Editor name | Rule |
|---|---|---|
| `CBAVTHasFoe` | `Has foe` | the brain's foe manager exists and its `en_henFoe` is set |
| `CBAVTHasFoeCandidate` | `Has foe candidates` | the foe manager exists and `en_bHasFoeCandidates` is true. **The member is written only by `LookForFoe`** (`foe-manager-numbers.md` 2.1): while the manager tracks a foe it keeps the value of the last search |
| `CBAVTKnockOut` | `Foe is in KO` | the brain has a foe manager and a foe, the foe's puppet is a **legged** puppet, and `IsInKO` is true: the legged state is knock-out (`LPS_KNOCK_OUT`) **and** the get-up helper does not exist yet (`puppet-legged-sequences.md`) |
| `CBAVTMeleeSuccess` | `Melee success` | same chain, then the foe's puppet's **`en_bMeleeHitFoe`**: cleared when its generic melee attack state begins and on a reset, **set when one of its melee damage events hit the entity that is its own foe** (`OnAnimEventMeleeDamage`). Read from the evaluating brain: "my foe's last melee attack has hit its target" |
| `CBAVTWarMeasure` | `War measure` | `wm_iWarMeasure <= GetMaxWarMeasureLately(wm_fPastPeriod)` of the world info entity |

**War measure.** The world info entity's default state runs `UpdateHostilityGraph` **once per second** (the next call is scheduled 1.0 s later): it counts the puppets of its list of existing puppets whose **brain has a foe manager with a current foe** (so players, whose brain has no foe manager, never count; allies and neutral puppets that fight do) and stores the count in a **60-entry ring** (the oldest sample is overwritten). `GetMaxWarMeasureLately(period)`: the period is truncated to an integer and clamped to **1 .. 60**, then the **maximum of that many most recent samples** (starting with the newest) is returned; a period above 60 prints `War measure is calculated only for last minute, so testing values before that time is not alowed` and one below 1.0 prints `War measure is updated each second, so testing period can't be smaller than 1 sec.` (the value is still computed with the clamp). (The initial contents of the ring were not read.)

### 5. The seven actions (**medium**; none occurs under its own name in a file: the script counts 0 of 10 names)

| Action (editor text) | `Execute` | Data member |
|---|---|---|
| `CBATryRiding` | **inherited and does nothing: it returns true.** Its vtable's `Execute` slot holds the shared "return 1" function (the function that every other action's `Execute` calls first and whose answer it ignores); it has no members, no `IsLatentAction`. The name suggests boarding a ride, but the class carries no behaviour: a list that picks it counts as handled | none |
| `CBAChangeAligment` (`SBAChangeAligment`) | the puppet's `SetAlignment(ca_baNewAligment)`: **one store into `en_baAlignment`** (replicated). The world info lists are **not** updated (the evil-puppet list is filled at birth only) and the foe manager's cached checks see the new value at their next test | `ca_baNewAligment` (`EBrainAligment`: 0 good, 1 evil, 2 neutral, 3 semi-neutral) |
| `CBASetActionSet` (`Set action set: <id>`) | stores the id in the brain's `en_idCurrentActionSet`. The brain's think point with a foe raises **that** moment (`HandleAIMoment`, `behavior-action-choice.md` 3); the default after construction is `Action`; **nothing resets it** (neither a new foe nor a state change): it lasts for the brain's life | `sas_idNewActionSet` |
| `CBAForceMovingType` (`Force moving type: <mover or NULL>`) | the brain's `ForceMovingType(mover, time)`: `DeleteCurrentMover`, `SetCurrentMover(mover, MR_FORCED)` and the copy's `mc_fAutoOffTime` := `time` (`brain-movement.md` 3.5) | `bafmt_pmcForcedMovingType`, `bafmt_fAutoOffTime` |
| `CBAUnforceMovingType` | the brain's `UnforceMovingType` = `DeleteCurrentMover` | none |
| `CBAStopTactic` (`Stop tactic`) | **only** when the mover reason is `MR_TACTIC` (3) the reason is set to `MR_NONE` (0); **the mover itself is not deleted** (unlike `CBAStopUsingTactic`, which also clears the desired tactic and deletes the mover) | none |
| `CBAFoeAssigning` (`Foe assigning`; data class `CBAPlayerAssigning`) | the brain's foe manager gets `EnablePlayerFoes` or `DisablePlayerFoes` by the first flag and `EnableNonPlayerFoes` or `DisableNonPlayerFoes` by the second (flag set = enable); returns true, **false when the brain has no foe manager** | `pa_bAllowPlayerToBeFoe`, `pa_bAllowNonPlayerToBeFoe` |

**5.2 The identity `CBAFoeAssigning` = data `CBAPlayerAssigning`** (**medium**, two independent arguments): (a) **by elimination**: the 29 saved action and term classes match exported classes by name or by the `S` twin except this one, and exactly one exported action class has no stored name, `CBAFoeAssigning`; its instance size has room for exactly two booleans after the base members, and the data class has exactly those two; (b) **by meaning**: the first stored member is named "allow player to be foe" and the code's first flag selects the `PlayerFoes` pair, the second selects the `NonPlayerFoes` pair (flag set = enable), and the foe manager's two members `en_bCantAssignPlayerAsFoe` / `en_bCantAssignNonPlayerAsFoe` are what `AssignFoe` and `SwitchToClosestPlayer` read (`foe-manager-numbers.md` 3 and 4). The two binary-side names are adjacent strings of the DLL's type table; the C++ class was probably renamed after the data class was fixed (**low**). Data (script section 3): 12 objects in the three civilian behaviours (`NPC_Civilian`, `NPC_Civilian_Good`, `NPC_Civilian_Static`), four each: (1, 1) x 2, **(0, 1)**: players may not be foes, **(1, 0)**: non-players may not.

### 6. What the data shows about the rarely used classes

None of the five terms and ten action names (the six unused actions with their `S` twins and `CBAFoeAssigning`) occurs in any of the 82 `.cb` files (script section 1: 0). Of the seven terms in use the script prints the carrier classes (section 1): `SBAAnimation` carries 50 health terms, `CBALoseFoe` 12 `FoeIsPlayer`, `SBAChangeState` 9 `HasTool`, 6 `FoeIsPlayer` and 3 `CurrentStateDuration`, `SBAShoot` 4 `FoeAngle`, 2 `FoeIsPlayer` and 1 `PlayerDistance`, `SBAMelee` 2 `PlayerDistance` and 2 `FoeAngle`, `CBAForcePlayerFoe` 3 + 3, `CBAPlaySound` 3 + 1, `SBAList` 3 combiners.

## Invariants

1. A term's `IsValid` never reads `vt_bNotTrue` (11 of 11 read); modes 2, 3, 4 and 5 of the health term test integers only.
2. In the data no health term has mode 0, 1, 3 or 5, none has `vt_bNotTrue` set, and the five mode-2 objects all have value 10 (51 objects).
3. All six angle terms have pitch minimum -90 and maxima -55 (4) or 90 (2); headings -180 to 180 (4) or -90 to 90 (2).
4. The three stored foe-assigning variants per civilian behaviour are (1, 1) twice, (0, 1) and (1, 0) (12 of 12).

## Open questions

1. **The damage kinds that skip the accumulated-damage counter** (the three numeric damage kinds 19 to 21 and the parameter flag that the accumulation test reads in `OnReceiveDamage`), and the armor split's exact rounding: **T01-364** (with `puppet-entity-classes.md` section 7, T01-140).
2. **`mthQuaternionToEuler`'s axis order** and the engine's forward axis (the angle term assumes heading first; a run-time check settles it): T01-130, T01-038.
3. **The names of the two data/code identities** (`CBAFoeAssigning` against `CBAPlayerAssigning`): a second, independent confirmation would come from the type-registration tables of `Sam2Game.dll` (not read): T01-364.
4. **Run-time check** of `CBATryRiding` being a true no-op in a list (it stops the list, `behavior-action-choice.md` 3) and of the same-instant condition of mode 2: T01-158.

## Test vectors

- **Mode 4** (value 20, nominal 100): 85 -> 75 (d = 10): **true**; 79 -> 75 (d = 4): false; 100 -> 70 (d = 30): **false** (the first hit from full health); 70 -> 55 (d = 15): `(100 - 55) / 20 = 2 > (100 - 70) / 20 = 1`: true.
- **Mode 2** (value 50): 60 -> 40 (d = 20) evaluated at the instant of the hit: **true**; the same record one simulation step later: false; 60 -> 55: `h < v` fails: false.
- **Mode 5** (value 20): 25 -> 15 (d = 10): `15 > 25 mod 20 = 5`: true; 35 -> 25 (d = 10): `25 mod 20 = 5 > 35 mod 20 = 15`: false.
- **Accumulating mode 4** (value 50): hits of 30 at t = 0 and 30 at t = 0.2: the sum 60 is above 50 at the second hit: true; the same hits 0.3 s apart: the counter was reset in between: false.
- **Foe angle** (limits heading -45 to 45, pitch -90 to 90; the puppet at the origin facing -Z, Euler heading and pitch 0): foe at (0, 0, -10): heading 0, pitch 0: **true**; at (-10, 0, 0): heading **+90**: false; at (10, 0, 0): heading **-90**: false; at (0, 0, 10): heading -180: false; at (0, 5, -8.66): pitch +30: true; puppet heading 90 and foe at (0, 0, -10): `rh = -90`: false.
- **War measure:** 5 non-player puppets fight from t = 10 s to 20 s: `GetMaxWarMeasureLately(5)` at t = 25 is 0 if the samples are the last five seconds (t = 20 to 24), `GetMaxWarMeasureLately(30)` is 5.
- **Set action set:** `CBASetActionSet` `Attack` once: every later think point with a foe raises `Attack` until another set action runs.
- **Foe assigning (0, 1):** the foe manager refuses player foes: `AssignFoe` for a player returns at the refusal step and `SwitchToClosestPlayer` returns false.
- `python -I tools/re/validity_terms_verify.py "<install>"` prints the term table, the six angle objects, the 51 health terms by mode and the 12 foe-assigning objects.
