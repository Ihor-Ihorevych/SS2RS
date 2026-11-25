# How the brain chooses among the actions of a moment (T01-153, part of the T01-105 follow-ups)

- **Roadmap task:** T01-153 (follow-up of [`behavior-action-classes.md`](behavior-action-classes.md) open question 1; the classes are in [`behavior-classes.md`](behavior-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for every count of the data side (`tools/re/action_choice_verify.py`, all 82 `.cb` files: handlers, lists, weights, terms, the two special actions, the band period graph); **medium** for every rule read from `Sam2Game.dll` (the selection routine, the three handler levels, the weight formula, the term evaluation, the special actions, the period formula: each is one reading of a short routine that agrees with the data's layout and with the editor's own labels; **nothing was run**); **low** where marked.

Prerequisites: `ai.md` (the `.cb` file: states, bands, moments, action lists), [`behavior-action-classes.md`](behavior-action-classes.md) (what each action's `Execute` does and the validity terms), [`brain-classes.md`](brain-classes.md) (`ThinkPoint`, `HandleAIMoment`, `StartCharacterAction`), [`brain-movement.md`](brain-movement.md) (the band lookup that fills the situation record), [`hitscan-shot.md`](hitscan-shot.md) (the engine's random generator).

## Summary

At every **think point** the brain raises a **moment** (`Action` when it has a foe, `Idle` when it has none; also `Damage`, `FoeAssigned`, `BeginState` ... raised by events). For a moment the `.cb` behaviour can hold action lists at **three levels**, tried in a fixed order: the **band** (the distance band the foe is in), then the **state** (`Default`, `Shooter` ...), then the **behaviour-wide** handler. The first level whose list for that moment **starts an action** ends the search. Inside one list the action is a **weighted random draw** among the actions whose validity term holds. Two special actions steer the search: **`CBADefaultAction`** ("Pass on handling") always fails, so the search goes on to the next level; **`CBANoAction`** ("No action") always succeeds without doing anything, so the search stops and the brain simply waits for the next think point.

Findings an implementer needs first:

1. **Order of the levels (medium):** band list, then state list, then behaviour-wide list; the instance handler of a placed puppet comes before all of them (section 2). A level is left for the next one when it has no list for the moment, **or** its list yields no action, **or** the chosen action's `Execute` returns false.
2. **Selection inside a list (medium):** the weight of each action is `ba_fWeight` times up to three optional graph values (health ratio, distance ratio, area ratio; **all null in all 1,483 stored actions**), clamped at 0; actions with weight 0 or a failed term are dropped; one random number `r` in [0, 1) is compared with the running sum of `weight / total`: the first action whose running sum is **greater than `r`** is chosen, in list order. **Chosen means executed once: there is no second draw if `Execute` fails** (section 4).
3. **`vt_bNotTrue` and the combiner (medium):** the negation flag of a term is applied by whoever evaluates the term (the selection, or the combiner for its two sub-terms); `IsValid` itself never looks at it. The combiner evaluates **both** sub-terms (no short cut), negating each by its own flag, and returns false when either sub-term is missing (section 5).
4. **`CBADefaultAction` and `CBANoAction` (medium; the data fits):** `Execute` returns **false** for the first and **true** for the second and nothing else happens. In the data they only occur in band `Action` lists, as a pair in 98 of 381 bands (so a band can say "with probability `wD / (wD + wN)` do what the state or behaviour-wide `Action` list says, otherwise wait"), alone in 86 (64 with `CBANoAction`, 22 with `CBADefaultAction`; section 6).
5. **The "frequency graph" is a period in seconds, not a frequency:** `cba_pgrActionFrequencyOverAreaRatio` is the **time between two think points** while the foe is in the band (381 of 381 are constant graphs, 0 to 20 s; 83 bands have 0, which means "as fast as the engine allows"), randomised per puppet by `cba_fActionPeriodRandomizeMultiplier`, divided by the difficulty's think multiplier (section 7). The oscillator does **not** use `cb_pgrThinkPeriodOveride` (correction of `behavior-entity-classes.md` section 6).
6. **A successful action resets the action timer and marks the puppet non-idle; a failed search only resets the timer** (section 3).

## Evidence

- **Code (medium).** `CBrainEntity::HandleAIMoment`, `StartCharacterAction`, `CacheAreaData`, `ThinkPoint`, `ResetActionTime`, `GetMovePeriod`, the oscillator's `ExecuteBehavior`, the behaviour's lookup and the list-selection routine that they call (neither of the last two is exported; they were found by following the calls), the `Execute` of `CBADefaultAction` and `CBANoAction` and the `IsValid` of the combiner and of `CBAVTCurrentArea` (found through the class registration tables of the DLL, which also give the editor labels), `CValueGraph::GetValue` of `Engine.dll`. Each was decompiled and read once; the call order of the three handler levels was confirmed on the instruction level. The layouts (which member sits where) were matched against the member order of the data classes (`misc-schema.md`): the weight graphs, the band's handler and graph, the state's handler and the velocity adjuster all fall on the offsets the code uses, which is a second, independent reading of the layout.
- **Data (high).** `python -I tools/re/action_choice_verify.py "<install>"` (read-only, under a second): all 82 `.cb` files; 5 blocks of counts, quoted below.
- **Binary build.** The `Sam2Game.dll` and `Engine.dll` of the install used for the other specs.
- **Not checked.** Nothing was run, so no probability was measured (`T01-038`/`T01-039` do not exist as instrumented runs yet; the run-time confirmation is T01-158). The 564 world-stored handlers (section 2) were not walked by the script (T01-159).

## Layout / rules

### 1. The situation record (`CActionSelectionData`, member `en_asd` of the brain; medium)

`HandleAIMoment` rebuilds it before every moment (section 2); the selection and the terms read it. Fields, in the order of the record (16 words; the engine's own member names are `asd_...`, only `asd_idAIMoment` was seen as a string, the others are described by use):

| # | Content | Filled by |
|---|---|---|
| 0 | the brain | `CacheAreaData`; the first argument of every `Execute` |
| 1 | the current `.cb` state id | `CacheAreaData` (the brain's `en_idState`) |
| 2 | the **moment id** being handled | `HandleAIMoment` (after `CacheAreaData`) |
| 3 | the id of the current band (`cba_idArea`), invalid when there is none | the band lookup (`brain-movement.md` 2.2) |
| 4 | the state object whose id equals field 1 (null when none) | the band lookup |
| 5 | the current band object (null: no foe, or the foe is beyond the last band, or the list is empty) | the band lookup |
| 6 | the action chosen last | the selection (cleared at its start) |
| 7 | the **area ratio**: `1 - (d - start) / (end - start)`, 1.0 at the inner edge and 0.0 at the outer edge; **-1 with no foe**; **0 with a foe but no band** | the band lookup |
| 8 | the horizontal distance `d` to the foe (X and Z only), **-1 with no foe** | `CacheAreaData` |
| 9 | the **distance ratio**: `d` divided by the behaviour's first range member (`cb_fActiveRange`, **medium-low**: identified by member order); -1 with no foe; 0 with a foe but no band | the band lookup |
| 10, 11, 12 | band start, band end, and the next band's distance (or the range for the last band) | the band lookup |
| 13 | the **health ratio**: the puppet's health divided by its maximum health | `CacheAreaData` |
| 14 | the **random number** `r` in [0, 1] (section 4) | `CacheAreaData`, then **drawn again** by `StartCharacterAction` |
| 15 | the medium: 0 land, 1 air, 2 water | `CacheAreaData` |

The record is reset to its defaults (ids invalid, numbers 0) at the start of each `CacheAreaData`. With no foe the band lookup sets the ratios to -1 and finds no band; with a foe but no matching band the two ratios stay 0.

### 2. Entering the selection (`HandleAIMoment`, `StartCharacterAction`; medium)

`HandleAIMoment(moment)` (called by `ThinkPoint`, `ForceHandleAIMoment` and the event hooks, `brain-classes.md` 3.2, 3.3):

1. Return false (nothing happens) when the brain has no puppet, or bit 0 of the execution abilities ("AI moments may be handled", `brain-classes.md` 4.4) is clear, or the puppet has no character behaviour.
2. Rebuild the situation record (`CacheAreaData`: section 1), store the moment id in field 2, clear the "rejected because" debug text.
3. **Instance handler first.** If the puppet carries a moment handler of its own (its member `en_hmhMomentHandler`, a handle; the selection routine is applied to the object it points at as if it were a `CAIMomentHandler`, and the 564 world-stored `CAIMomentHandler` objects of `behavior-classes.md` section 3 are the likely targets: **medium-low**, T01-159 owns the link), run the list selection of section 4 on that handler alone; if it yields a started action the moment is "handled by Instance" and steps 4 and 5 are skipped.
4. Otherwise `StartCharacterAction(behaviour)`: draw a **fresh** random number (field 14; the draw of step 2 is overwritten), then run the three-level search of section 3 on the behaviour. If it fails and the behaviour has a based-on behaviour (`cb_pcbBasedOnCharacter`, null in all 82 files), the same call is repeated with that behaviour. With no behaviour at all the brain records the reject text `Unknown starting character behavior is set.` and starts nothing.
5. The result of the search is the result of `HandleAIMoment`. **When an action started**, the brain restarts its action timer (`ResetActionTime`: the time of the last action becomes "now", the origin of the think period of section 7) and tells the puppet (`MarkNonIdle`, through `GenericActionStarted`).

So the instance handler uses the random number of the `CacheAreaData` draw and the behaviour search the second one; both are uniform, the sequence is the engine's global generator (`hitscan-shot.md` section 4: two states, multiplier 1,220,703,125; `r` = the exclusive-or of the two states times 2^-32, converted to a 32-bit float, so `r` can reach exactly 1.0 in about one case in 2^25).

`ThinkPoint` (`brain-classes.md` 3.2) with a foe calls `HandleAIMoment(en_idCurrentActionSet)` (`Action` unless a `CBASetActionSet` changed it; no stored file uses that action); with no foe it calls `HandleAIMoment("Idle")` (the string was read from the call: **medium**). **When it returns false the think point calls `ResetActionTime` itself**, so a failed search also restarts the period.

### 3. The three levels (the search; medium, instruction level read)

Given the situation record:

1. **Band level.** If field 5 (the band) is not null, run the list selection (section 4) on the **band's** handler (`cba_mhAIMomentHandler`). If it started an action: done, true.
2. **State level.** If field 4 (the state) is not null, run it on the **state's** handler (`cs_mhAIMomentHandler`). If it started an action: done, true.
3. **Behaviour-wide level.** Run it on the behaviour's `gdc_ahActionHandler`. Its result is the result of the search.

The two earlier levels are tried only while the earlier ones **returned false** (the state level is skipped when the band started an action; the behaviour level when either did). A band with no list for `Action` (63 of 381) therefore behaves like a band whose list always passes on. In the data the levels hold: behaviour-wide 171 lists (`Action` 52, `Damage` 51, `FoeAssigned` 53, `AllFoesDied` 5, `Born` 3, `ToolPicked` 3, `StartRiding` 2, `StopRiding` 2), state 61 lists (`Action` 14, `BeginState` 14, `Idle` 13, `EndState` 11, `AllFoesDied` 3, `Born` 3, `ToolPicked` 3), band 321 lists (`Action` 318 plus 3 whose moment id is **empty**; script block 1). The 3 empty-id lists can only match a moment raised with an invalid id; no stored file reaches them (not resolved, T01-126). The moments `Damage` and `FoeAssigned` therefore exist only behaviour-wide, `Idle` and `BeginState`/`EndState` only per state, **and only the `Action` moment is ever per band** (script).

**What a "no foe" moment sees.** With no foe the band lookup finds no band (field 5 null), so `Idle`, `Born`, `ToolPicked` ... are searched at the state and behaviour-wide levels only.

### 4. The list selection (one handler; medium)

The routine runs on one handler (a list of `CBehaviorActionList`, each with a moment id and a list of actions) with the situation record. It returns true exactly when it **started** an action (`Execute` returned true). Steps:

1. Clear the record's "chosen action" (field 6).
2. Walk the handler's lists in stored order. For a list whose moment id differs from field 2 go on to the next list. (No stored handler repeats a moment: **0 of 597**; only the first matching list can matter, except in the rounding case of step 7.)
3. **A matching list with no action ends the whole selection with false** (it does not try later lists). In the data: 1 band list (`kwongo.cb`).
4. For every action of the list, in order, evaluate **all** of them (nothing is skipped after a success):
   - the validity term: no term (`ba_pcValidityTerm` null) = holds; otherwise the term's result is `IsValid(record)` when its `vt_bNotTrue` is 0 and the **opposite** of `IsValid(record)` when it is 1 (section 5). An action whose term fails is dropped.
   - the weight `w = ba_fWeight`; then, if the **distance** graph is set, `w = w * graph(field 9)`; if the **health** graph is set, `w = w * graph(field 13)`; if the **area-ratio** graph is set, `w = w * graph(field 7)` (members `ba_pgrWeightOverDistanceMultiplier`, `...HealthMultiplier`, `...AreaRatioMultiplier`; the three multiply, so the order is immaterial). **All three are null in all 1,483 stored actions** (853 in `.cb`, 630 in worlds), so in shipped data `w = ba_fWeight`.
   - `w` below 0 becomes 0; an action with `w = 0` is dropped (the routine contains a branch for a weight below zero that "starts the action at once", **which can never be taken** after the clamp: **medium**). The surviving actions go into a temporary array (weight, action) in list order that grows on demand (**medium-low**: the growth routine was read in outline; the longest stored list has 7 actions).
5. The total `T` is the sum of the surviving weights. **If `T` is 0 (no action survives) the selection ends with false** (like step 3, later lists are not tried). In the data: 1 list (`fido.cb`, every weight 0).
6. Walk the surviving actions in list order keeping a running sum `S` (starts at 0): `S = S + w / T`; the **first action for which `r < S`** (with `r` = field 14) is the chosen one. It is stored in field 6 and **its `Execute(brain)` is called once; the selection ends with the value `Execute` returned** (true: started; false: not started and **no other action is tried**).
7. If no action satisfied `r < S` (only by rounding when `r` is within about 1e-7 of 1) the loop of step 2 continues with the next list of the handler, and ends with false after the last one.

So the probability of action `i` is `w_i / T` among the actions whose term holds (not normalised per list in the data: weights are relative inside one list only). The weights are 32-bit floats; `T` and `S` are accumulated in float.

The selection has **no memory**: no cool-down, no "not the same action twice", no use of earlier outcomes; the only state that carries over is the think period of section 7.

### 5. Validity terms, `vt_bNotTrue` and the combiner (medium)

Every term derives from `CBAValidityTerm` with one data member `vt_bNotTrue` (stored 0 or 1) and one virtual `IsValid(record)`; the base never calls `IsValid` itself:

- **In a list** (section 4 step 4): `holds = IsValid(record) XOR vt_bNotTrue`.
- **`CBAVTTermCombiner`** (members `tc_pTermA`, `tc_pTermB`, `vtr_tcTermCombineOperation`: `TC_AND` 0, `TC_OR` 1): its `IsValid` returns **false** when either sub-term pointer is null; otherwise it evaluates **both** sub-terms (no short cut; the terms have no side effects) each as above (`IsValid XOR its own vt_bNotTrue`), and returns `A and B` for operation 0, `A or B` for operation 1, **false for any other value**. The combiner's own `vt_bNotTrue` is then applied by the list selection like for any term.
- The negation therefore **nests**: a negated combiner of two negated terms is the opposite of `(not A) op (not B)`.
- **`CBAVTCurrentArea`** (a 12th term class; the binary's registration, editor label `Current area`, one member `vtr_idCurrentArea`): holds when `vtr_idCurrentArea` equals the id of the current band (field 3); **no stored file has it** (0 of 82 `.cb` files; T01-157 for the other terms).

Data (script block 3): of the 853 actions of the `.cb` files (749 placed directly in a handler list, 104 more inside the `SBAList` groups) 104 carry a term, none of the 630 world-stored actions does. The 110 term objects (the 104 plus the 6 sub-terms of the 3 combiners) are: `SBAVTHealth` 51 (on `SBAAnimation` 50 and `CBAPlaySound` 1 in handler lists; `vt_bNotTrue` 0 in all 51), `CBAVTFoeIsPlayer` 29 (`vt_bNotTrue` 1 in 16, 0 in 13; includes 3 sub-terms), `CBAVTHasTool` 9, `CBAVTPlayerDistance` 9 (negated in 1; includes 3 sub-terms), `CBAVTFoeAngle` 6 (negated in 4; on `SBAMelee` 2 and `SBAShoot` 4), `CBAVTCurrentStateDuration` 3 (on `SBAChangeState`), `CBAVTTermCombiner` 3 (all on `SBAList`, all AND of a `CBAVTPlayerDistance` and a `CBAVTFoeIsPlayer`, both sub-terms not negated, the combiner not negated). `vt_bNotTrue` is 1 on 21 of the 110 term objects (`CBAVTFoeIsPlayer` 16, `CBAVTFoeAngle` 4, `CBAVTPlayerDistance` 1) and 0 on the other 89. Lists in which **every** action carries a term (so they may yield nothing): 72 (`Damage` behaviour-wide 51, state `Action` 6 and `Idle` 3, band `Action` 5, behaviour-wide `FoeAssigned` 3 and `ToolPicked` 3, behaviour-wide `Action` 1; counted over the actions placed directly in a handler list). The meaning of each term's `IsValid` is in [`behavior-action-classes.md`](behavior-action-classes.md) section 3 (open details: T01-157).

### 6. The two special actions (medium; labels high)

Both derive from `CBehaviorAction`, have **no member of their own**, a default weight of 1, no term and null graphs, and are registered with the editor labels **`Do nothing`** (class `CBANoAction`, description `No action`) and **`Pass on handling`** (class `CBADefaultAction`, description `Pass on handling`). Their only behaviour is the result of `Execute`:

- **`CBANoAction.Execute` returns true** and does nothing. The search ends here: a started action, the action timer restarts, the puppet is marked non-idle. The brain waits for the next think point.
- **`CBADefaultAction.Execute` returns false** and does nothing. The search goes on to the next level (section 3). At the behaviour-wide level a false result ends the moment: `ThinkPoint` only restarts the timer.

Consequence for the data (script block 4 and 4b, all **high** as counts):

| `Action` list of the band | Bands | Meaning |
|---|---|---|
| `CBANoAction` only | 64 | the character does nothing in this band (never reaches the state or behaviour-wide `Action`) |
| `CBADefaultAction` only | 22 | always passes on |
| both | 98 | passes on with probability `wD / (wD + wN)`: 1/3 in 7, 5/12 in 1, 1/2 in 27, 3/5 in 1, 2/3 in 15, 5/7 in 8, 3/4 in 8, 4/5 in 10, 5/6 in 11, 6/7 in 7, 8/9 in 2, and about 1 in 1 (weight 100,000 against 1) |
| no `Action` list | 63 | passes on |
| other actions (with or without the two) | 133 | draws among them as in section 4; one of them (`fido.cb`) has every weight 0 and ends the selection with false |
| a list with no action (`kwongo.cb`) | 1 | ends the selection with false |

`CBANoAction` also fills the 3 band lists with an empty moment id and the state lists `AllFoesDied`, `Born` and `ToolPicked` (3 each): "react by doing nothing" (the moment is then handled at the state level and the behaviour-wide list for the same moment is not consulted, wherever both exist).

**Worked example (`kleer.cb`, state `Default`, foe 50 m away).** The distance lies in band `Middle` (outer distance 70; `Close` ends at 25). Its `Action` list is `CBANoAction` (1) and `CBADefaultAction` (1). With `r` below 0.5 the first (running sum 0.5) is chosen: nothing happens until the next think point. With `r` of 0.5 or more the default action is chosen: false, no state list exists, so the behaviour-wide `Action` list is tried: one action, `SBAShoot` (shot `Chainballs`, weight 1), which starts a shot. So the Kleer shoots in about half of its think points at that distance; in band `Close` (25) it never does (a list with only `CBANoAction`), in band `Slash` (3) it slashes. In state `Shooter` the `Middle` band (outer distance 150) passes on with weights 4 and 1: 80% shoot, 20% wait.

### 7. The think period (what the "frequency graph" is; medium, T01-156 owns the remaining numbers)

The **think-point oscillator** (`behavior-entity-classes.md` section 6) calls `ThinkPoint`, then schedules the next call. Its period rule, read from `ExecuteBehavior`:

1. If the brain's "immediate think point" flag is set: clear it and schedule the next call for the next instant (**medium-low**: the delay argument of that call was not resolved; `behavior-entity-classes.md` section 2 gives 1e-5 s for such calls).
2. Otherwise start with `x = 1 + u`, `u` a fresh random number in [0, 1) (so a brain **without a foe, or with a foe and no band**, thinks every 1 to 2 s).
3. If the brain has a foe **and** the current band (field 5) is not null: if the band's `cba_pgrActionFrequencyOverAreaRatio` is set, `x` becomes **its value at the area ratio (field 7)**; then multiply by `1 + 2 k` with `k = (en_fRnd - 0.5) * cba_fActionPeriodRandomizeMultiplier` (`en_fRnd` is the puppet's own random member, a float, replicated, assumed uniform in [0, 1): so the factor is in `1 +- cba_fActionPeriodRandomizeMultiplier`, **fixed per puppet**, not redrawn); then divide by the current difficulty's `dif_fThinkFrequencyMultiplier` (`player-rules.md`: Tourist and Easy 0.75, Normal 1, Hard 1.125, Serious 1.75; the game-info entity holds the current difficulty).
4. The next think point is at `(time of the last action) + x`, i.e. the delay is `x` minus the time already elapsed since the last action reset (`ResetActionTime`), **never less than 1e-5 s**.

In the data every band has the graph (381 of 381 are `CValueGraph`, i.e. constants: the value of a `CValueGraph` is its constant times the graph's scale, scale 1.0 in all): 0 s in 83 bands, 0.1 (5), 0.25 (2), 0.5 (8), 1 (58), 1.25 (9), 1.5 (19), 1.75 (10), 2 (32), 2.5 (16), 3 (91), 4 (9), 5 (19), 6 (8), 8 (1), 10 (5), 12 (2), 20 (3), 100,000 (1: a band that effectively never thinks again); `cba_fActionPeriodRandomizeMultiplier` is 0.25 in 337, 0 in 33, 0.5 in 4, 0.2 in 4, 0.4 in 2, 0.125 in 1. A band with period 0 gives `x = 0` and so a delay of 1e-5 s after the last action: **the character thinks at the engine's fastest rate while in that band** (`Slash` and `Jump` of the Kleer). Because a started action restarts the timer, the delay counts from the **last started action or failed search**, not from the previous think point.

**Correction.** `behavior-entity-classes.md` section 6 says the oscillator takes its period from `cb_pgrThinkPeriodOveride`. The oscillator does not read that graph; it reads the band's graph above. `cb_pgrThinkPeriodOveride` is read by `GetMovePeriod` (the period between mover updates, `brain-classes.md` 3.2), evaluated at the 3-D distance to the foe, after the mover's own override (`mc_pgrMovingPeriodOverride`) and before the global graph (T01-126).

## Invariants

1. The 82 `.cb` files hold 597 handlers (82 behaviour-wide, 134 state, 381 band); **none** repeats a moment id inside one handler (script, 0 of 597).
2. Of the 853 stored `.cb` actions **852 have a weight above 0 and 1 has weight 0** (`fido.cb`); the weight graphs are null in all (script, and `behavior-classes.md`).
3. `CBADefaultAction` (131) and `CBANoAction` (192) occur only in band `Action` lists, except 2 behaviour-wide and 9 state-level `CBANoAction` (script block 4).
4. Every band's frequency graph is a `CValueGraph` with scale 1.0; its constant equals the y of its one control point (381 of 381).
5. A moment id found in a band list is `Action` or empty (321 of 321).

## Open questions

1. **Run-time confirmation** (the order of the levels, the measured probability of `Default` against `No action`, the think period of one enemy): T01-158 (`Kleer`: bands `Slash`, `Middle`, `Shooter` state).
2. **The empty moment id** (3 band lists), the moment raised by `OnCreatedByPuppet`, `Born`: T01-126.
3. **The instance handlers** (`en_hmhMomentHandler`, 564 stored `CAIMomentHandler` objects): which placed objects carry them and what they contain beyond `SBAShoot` (614) and `SBAAnimation` (16): T01-159.
4. **The rest of the think period**: the immediate-think case in detail, the unit and range of `en_fRnd`, the oscillator's `1 + u` branch (T01-156); `GetMovePeriod` and the global period graph (T01-126).
5. **`IsValid` of the terms** (angle convention of `CBAVTFoeAngle`, health modes 2, 3 and 5, the terms the data does not use): T01-157.

## Test vectors

- **Weights** (4.4 to 4.6): a list with `SBAShoot` weight 1 (no term) and `SBAMelee` weight 3 (term fails): only the shot survives, `T = 1`, any `r` below 1 chooses it. Same list with the term holding: `T = 4`, running sums 0.25 and 1.0: `r = 0.1` shoots, `r = 0.25` melees (the test is `r < S`, so exactly 0.25 does **not** choose the first), `r = 0.9` melees.
- **Pass-through** (3, 6): band list `[No action 1, Pass on 1]`, state list absent, behaviour-wide `Action` = `[SBAShoot 1]`: `r = 0.3` starts nothing and returns true; `r = 0.7` returns the shot's result; if the puppet refuses the shot (`Execute` false) `HandleAIMoment` returns false and the think point restarts the timer (no second draw).
- **Weights 0** (4.5): a list whose actions all have weight 0 (or whose terms all fail) returns false at once and the search moves on to the next level.
- **Negated combiner** (5): combiner AND of A (true) and B (false) with `vt_bNotTrue` 1 on the combiner: the combiner returns false, the list sees `false XOR 1` = true.
- **Period** (7): band with graph 3.0 and randomiser 0.25, puppet `en_fRnd` 0.9, Normal difficulty: `k = 0.4 * 0.25 = 0.1`, factor 1.2, `x = 3.0 * 1.2 / 1 = 3.6 s` after the last action; on Serious `3.6 / 1.75` = 2.057 s. Band with graph 0: delay 1e-5 s.
- **Level counts** (script): behaviour-wide lists 171, state 61, band 320; band `Action` shapes: both special actions 98, No action only 64, no list 63, Default only 22.
