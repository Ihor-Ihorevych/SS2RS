# The steering of the movers (T01-154, follow-up of T01-105)

- **Roadmap task:** T01-154 (follow-up of [`behavior-mover-classes.md`](behavior-mover-classes.md) open question 1; the markers movers and the navigation moving type are T01-155)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the stored values, counts and graph curves (`tools/re/mover_steering_verify.py`) and for every constant (read as an exact float); **medium** for the rules read from `Sam2Game.dll` and `Engine.dll` (each steering function was decompiled and read once; calls whose arguments the decompiler lost were checked on the instruction level; **nothing was run**); **medium-low** where marked (the sign convention of the turning matrix, the flying attack's details).

Prerequisites: [`behavior-mover-classes.md`](behavior-mover-classes.md) (the classes, the protocol, `CharacterMoverResult`), [`brain-movement.md`](brain-movement.md) (how the brain picks and keeps a mover, the movement record, the band lookup), [`brain-classes.md`](brain-classes.md), [`behavior-action-choice.md`](behavior-action-choice.md) (the situation record and the random generator), [`puppet-legged-classes.md`](puppet-legged-classes.md), `rsc-ctl.md` (global game parameters).

## Summary

A mover is called once per **move period** and answers with a goal point for the puppet; the puppet's movement step does the walking (T01-141). This page gives, for the movers that steer relative to the foe, **how the goal point is computed**: the shared machinery (destination point, altitude, validation) and the formulas of `CMCTowardsFoe`, `CMCAwayFromFoe`, `CMCDontMove`, `CMCCircleAroundFoe`, `CMCStayInArea`, `CMCPatternMoving`, `CMCDiamond`, `CMCMellon` and `CMCAirAttack`, with every constant.

Findings an implementer needs first:

1. **The destination is the foe's live position when the foe was seen on the last line-of-sight test, else the last seen position** (`en_bFoeSeenOnLastTest`), not "always the last seen position" (correction of `brain-movement.md` 1.1). **`CMCTowardsFoe`, `CMCAwayFromFoe` and `CMCStayInArea` pass that same flag as "pass through"** (a run-through point that the puppet does not stop at); **`CMCCircleAroundFoe`, `CMCPatternMoving`, `CMCDiamond`, `CMCMellon` and `CMCAirAttack` always pass true** (section 1.4).
2. **The base `MoveCharacter` ends a mover (`CMR_REPLACE_ME`) when the foe is nearer than `mc_fAutoOffDistance`**, not when the puppet is that far from its start (correction of `behavior-mover-classes.md` 3.1 and 5): the tactic's `en_fAutoOffDistance` ("10 in 238 tactics") therefore means "stop the tactic when the foe comes within 10" (section 1.2).
3. **The goal point of a step mover is one move-period step away**: nominal speed times the move period (`en_mcd` period, 0.1 s for the flying attack and for the 10 behaviours with a period override) from the puppet along the wanted direction; the movers never hand out the foe's own position except `CMCTowardsFoe` and the closing phase of the diamond.
4. **Circle and stay-in-area use the band, not their own ranges, when they are band movers**: the circling radius lies inside `[band start, band end]` (a fixed fraction chosen per puppet from its random member: `start + width x (0.25 + 0.5 r)`), and the `mc_fInnerRange`/`mc_fOuterRange` stored in the 26 + 7 `.cb` movers are **never read**; they matter only for tactic movers (sections 3 and 4).
5. **The pattern mover's weave is a sine of the phase**: heading offset `A = scale x sin(2 pi x phase)` degrees, the phase running over `P = pm_fPatternPeriod .. pm_fPatternPeriodMax` seconds (chosen per puppet), scale 20 to 80 degrees in the stored graphs, negative in 7 (section 5).
6. **The flying attack** keeps a frozen target and front direction to fly a straight pass over the foe (altitude by a graph over the signed horizontal distance), with 12 stored movers, all with a move period of 0.1 s (section 8).
7. **`SDynamicMoveData` is an empty class** (a virtual table and nothing else, 4 bytes); the brain's movement record is a different class, `CMoveCharacterData` (section 9).

## Evidence

- **Code (medium).** `SMoveCharacter::MoveCharacter`, `FirstTimeExecuted`, `IsNavigationAllowed`, `GetDestinationPoint`, `AdjustGPAltitude`, `ValidateAndSetGoalPoint`, `UsesNavigation`; the `MoveCharacter` of the nine movers above, `CMCCircleAroundFoe::FirstTimeExecuted` and `OnWallHit`; the puppet functions they call (`GetCurrentNominalVelocity`, `MovingIn3DArea`, `GetFrontDirection`, `SetGoalPoint`, `SetRunTroughPoint`, `StopMoving`); `CSineGraph::GetValue`, `CStepDownGraph::GetValue` (also the value of a step-up graph), `CLinearGraph::GetValue` and `CValueGraph::GetValue` of `Engine.dll`. Virtual slots were named from the virtual tables. The decompiler lost several local variables of the big functions (the circle, stay-in-area, diamond, mellon and air-attack movers); where a value could not be read from the decompiled text the instruction listing was read and the result is stated here in words (the final `ValidateAndSetGoalPoint` flags of every mover were checked that way).
- **Constants.** Exact floats from the data section: 0.1, 0.01, 0.001, 0.5, 1, pi/4, the degree-to-radian factor, 2 pi (for the sine graph), the up vector (0, 1, 0); the global game parameters `ggp_fStayInAreaCoreRatio` 0.5, `ggp_fStayInAreaHysteresisRatio` 0.25, `ggp_fStayInAreaShellRatio` 0.25, `ggp_fCircleAroundFoeCoreRatio` 0.5 (`helper-classes-params.md`).
- **Data (high).** `python -I tools/re/mover_steering_verify.py "<install>"` (about 5 minutes with the worlds; `--no-wld` about 15 seconds).
- **Not checked.** Nothing was run. The movement step that turns a goal point into walking (T01-141), the markers movers and the navigation mover (T01-155), the sign convention of the turning matrix (section 3), the unused members.

## Layout / rules

### 1. The shared machinery (`SMoveCharacter`; medium)

**1.1 The movement record** (`CMoveCharacterData`, the brain's `en_mcd`, filled before every `MoveCharacter`, `brain-movement.md` 1.1). Read by the movers: the brain; the **foe** (the foe's brain, null when the brain has none); the puppet's **own position** (x, y, z); the foe's **last seen position** (x, y, z, `en_vFoeLastSeenPos`); **`en_bFoeSeenOnLastTest`**; the **move period** `T` in seconds (`GetMovePeriod`, never below 1e-5). Also read from the situation record (`behavior-action-choice.md` 1): the band start, band end and the "next band distance" (asd fields 10, 11, 12), the area ratio (field 7) and the brain's mover reason.

**Destination point** (`GetDestinationPoint`): when the foe exists **and** `en_bFoeSeenOnLastTest` is true, the foe puppet's **current** position (its brain's placement); otherwise the **last seen position**. **Nominal speed** `v` = `GetCurrentNominalVelocity()` of the puppet: its declared nominal velocity (the legged one reads a curve of the puppet's random member `en_fRnd`; 1.0 when none) times its current velocity multiplier (`GetCurrentVelocityMultiplier`, T01-141). The **step** below is always `v x T`.

**1.2 The base `MoveCharacter`** (runs first in every mover; returns a `CharacterMoverResult`; 0 `CMR_OK`, 1 `CMR_REPLACE_ME`, 2 `CMR_NEED_NAVIGATION`, 3 `CMR_NAVIGATION_FAILED`):

1. On the first call: clear the first-time flag and run `FirstTimeExecuted` (base: draw `mc_fRnd` in [0, 1], store the start time and the start position; the circle, marker and navigation movers add their own); a non-zero result of `FirstTimeExecuted` is returned at once (no stored mover returns one).
2. If the brain has a foe: set the puppet's look target to the foe puppet, **or to the ride the foe puppet is on (`en_henRide`) when it has one**, and set the puppet's look-ahead flag `en_bLookAhead` to **not `mc_bLookAtFoe`** (the mover decides whether the puppet faces the foe or its walking direction).
3. If **navigation is allowed** (below) and the world info entity says the straight line from the puppet's own position to the **last seen position** is not free (the world info entity's straight-line test `CanMoveFromPointAToPointB`; its last two arguments are the puppet's `en_fStepDownHeight` and `en_fStepUpHeight`, the permitted height drop and climb, T01-155, `behavior-navigation-mover.md` 3.5; it answers true when either point has no mesh node): return **`CMR_NEED_NAVIGATION`** (2). (The test uses the last seen position even when the destination is the live one.)
4. If `mc_fAutoOffTime` is not -1 and `mc_tmStarted + mc_fAutoOffTime` has passed: return **`CMR_REPLACE_ME`** (1).
5. If the brain has a foe: `mc_bStopTacticWhenHasFoe` true -> `CMR_REPLACE_ME`; `mc_fAutoOffDistance` not -1 and **the brain's distance to the foe (field 8 of the situation record, horizontal) is below `mc_fAutoOffDistance`** -> `CMR_REPLACE_ME`.
6. Return `CMR_OK`.

All stored movers have `mc_fAutoOffTime` and `mc_fAutoOffDistance` -1 and `mc_bStopTacticWhenHasFoe` 0 (`behavior-mover-classes.md`), so steps 4 and 5 act only on **tactic movers**, which receive the tactic's values (`brain-movement.md` 3.3: 936 of 993 tactics have no time, 485 no distance, 73 stop when the brain has a foe).

**Navigation allowed** (`IsNavigationAllowed`): all of: the world has a navigation mesh; the mover's `UsesNavigation` is true (base: the puppet is **not** moving in a 3-D area; `CMCAirAttack` and `CMCDontMove` answer false); the puppet's `en_bDisableNavigation` is false; the puppet's allowed moving range `en_fAllowedMovingRange` is **-1** (unrestricted); the brain's mover reason is **not** `MR_NAVIGATION` (4); the mover does **not** `CanOperateWithoutTargetEntity`; and the puppet is not moving in a 3-D area again (the same legged test: flying or swimming poses and abilities).

**1.3 `AdjustGPAltitude(point)`** (only for a puppet that moves in a 3-D area, i.e. flying and swimming characters, and only when the behaviour has `cb_pgrAltitudeMin`): `dist` = the 3-D distance from the puppet to the destination; `h` = `cb_pgrAltitudeMin(dist)`; **with `cb_pgrAltitudeMax` set: `h` = (`cb_pgrAltitudeMax(dist)` + `h`) x 0.5**. Let `u` be the unit vector from the puppet to `point` (3-D); **if the horizontal part of `u` is longer than 0.1**: `point.x, point.z` := the puppet's position plus `u.x, u.z` times `v x T`, and **`point.y` := the destination's y + `h`**; otherwise the point is left unchanged. (So a flyer hovers `h` above the foe's height and its horizontal goal is one step away.) Data: 15 of 82 behaviours have a minimum graph (11 linear, 2 step-up, 1 value graph of 2.0, 1 combine graph) and 7 a maximum graph (6 linear, 1 step-up; script block 3); minimum curves such as (0, 1) to (20, 6) (4 behaviours) or (4, 0) to (100.6, 78.2), maximum curves such as (0, 1) to (20, 8).

**1.4 `ValidateAndSetGoalPoint(point, limit, passThrough)`** returns a result (0 or `CMR_NEED_NAVIGATION`):

1. If `limit` is true and the point is farther than 0.1 from the puppet: shorten it to **`v x T`** along the direction from the puppet. **No stored or read mover passes `limit` true** (the nine movers of this page pass false).
2. **Allowed moving range.** If the puppet's `en_fAllowedMovingRange` is 0 or more **and** the mover cannot operate without a foe: the range centre is the puppet's own allowed-area centre, **or the position the brain had when it got its foe** (`en_qvFoeAssignPlacement`) when the brain has a foe. If the point is closer than 0.01 to the puppet, or the range is below 0.01: `StopMoving`, return 0. If the point is farther from the centre than the range: the point becomes `centre + (unit vector from the puppet to the point) x range`, **pass-through becomes false**, and if that point is closer than 0.01 to the puppet: `StopMoving`, return 0.
3. If navigation is allowed and the world info entity says the straight line from the puppet to the point is not free: return `CMR_NEED_NAVIGATION` (2).
4. Pass-through false: the puppet's **`SetGoalPoint(point)`** (stop at the point); true: **`SetRunTroughPoint(point)`** (the point is a direction, the puppet does not stop at it; `puppet-generic` behaviours use the same call). Return 0.

**`CMCTowardsFoe`**: base; destination `D`; `AdjustGPAltitude(D)`; `ValidateAndSetGoalPoint(D, false, en_bFoeSeenOnLastTest)`; the result is returned. **`CMCAwayFromFoe`**: base; `d` = destination minus own position (3-D); the point is `own - unit(d) x a` with **`a` = the "next band distance"** (asd field 12: the next band's outer distance, or the behaviour's range for the last band, 0 without a band); `AdjustGPAltitude`; `ValidateAndSetGoalPoint(point, false, en_bFoeSeenOnLastTest)`. **`CMCDontMove`**: base; if the puppet's move type is not "none" (0): `StopMoving`; return the base result. **`CMCDummy`**: nothing.

### 2. The random numbers the movers use

`mc_fRnd` (the mover's own, drawn once at its first call, uniform in [0, 1], used by the diamond, mellon, air-attack movers as a coin and a blend) and **the puppet's `en_fRnd`** (a replicated float of the puppet, fixed for its life: the puppet's constructor draws it from the engine generator, uniform in [0, 1); the player puppet's `OnCreate` sets it to 0; whether a placed puppet's world data overwrites it was not checked, the member is in the puppet's data table) used as a per-puppet blend by the circle, stay-in-area, pattern and diamond movers and by the think period (`behavior-action-choice.md` 7). "`blend(a, b)`" below is `a + (b - a) x en_fRnd`.

### 3. `CMCCircleAroundFoe` (medium; the matrix sign: medium-low)

Members: `mc_fInnerRange`, `mc_fOuterRange`, `en_fChangeDirectionTimeMin/Max`, `en_icdInitialDirection`, `mc_bCircleRight` and run-time `mc_tmLastDirectionChange`, `mc_fCirclingRnd`, `mc_tmLastWallHitTime`.

- **`FirstTimeExecuted`**: after the base: `en_icdInitialDirection` 0 (`ICD_RANDOM`): draw `u`; **`u` <= 0.5 -> `mc_bCircleRight` false, else true**; 1 (`ICD_LEFT`): false; 2 (`ICD_RIGHT`): true; 3 (`ICD_NOT_ALTERED`): unchanged. Then `mc_tmLastDirectionChange` := now and `mc_fCirclingRnd` := a fresh draw.
- **Direction change.** When both `en_fChangeDirectionTimeMin` and `Max` are above 0 and `now - mc_tmLastDirectionChange` exceeds `Min + (Max - Min) x mc_fCirclingRnd`: draw a new `mc_fCirclingRnd`, restart the clock and **flip `mc_bCircleRight`**. (A change time of 0 means never: 3 of 28 stored movers; the others change every 2 to 16 s.)
- **`OnWallHit`**: flips `mc_bCircleRight` when **more than 1.0 s** has passed since the previous wall hit (the first hit always flips), then stores the time; returns 0.
- **Radius** `R`: `g` = `ggp_fCircleAroundFoeCoreRatio` (0.5): `f` = `blend(0.5 - 0.5 g, 0.5 + 0.5 g)` = `0.25 + 0.5 en_fRnd` (the core of the band, a fixed fraction per puppet); the interval is `[inner, outer]` = the **mover's own** `[mc_fInnerRange, mc_fOuterRange]` when the brain's mover reason is `MR_TACTIC` (3) and **the band's `[start, end]` (asd fields 10 and 11) otherwise**; `R = inner + (outer - inner) x f`. For the 26 + 2 stored movers in `.cb` and `.ep` files (band or shooting-process movers) the ranges 8 / 16 (25 of 28), 4 / 16, 10 / 20, 30 / 50 are therefore **unused**.
- **Step**: `s = v x T`; the angle `theta = -(s / R) x dir` with `dir` = +1 for `mc_bCircleRight` true, -1 for false (radians; the arc length of one step at radius `R`). `radial` = the unit vector from the foe (destination) to the puppet (3-D; zero when they coincide); the **circle point** `C` = destination + `rot(theta) x (radial x R)` where `rot` is the rotation about the vertical axis by the engine's heading convention (`theta` is the heading: positive turns the negative Z axis towards the negative X axis; **medium-low**: the matrix routine was not read, so which way `mc_bCircleRight` really circles on the screen is open, T01-158). The goal point is the puppet's position plus the unit vector from the puppet to `C` times `s` (one step towards the point on the circle), with the circle point's height; then `AdjustGPAltitude`; **`ValidateAndSetGoalPoint(point, false, true)`**; the result is returned.
- So the puppet converges on the radius `R` around the foe while advancing along the circle at one step per period; it **always passes through** (never stops at the goal point).

### 4. `CMCStayInArea` (medium)

Members `mc_fInnerRange`, `mc_fOuterRange` (read only for tactic movers, as in 3); global `ggp_fStayInAreaCoreRatio` (0.5) `c`, `ggp_fStayInAreaHysteresisRatio` (0.25, **read only by the debug drawing**), `ggp_fStayInAreaShellRatio` (0.25) `sh`.

1. Base `MoveCharacter` (result returned when not 0).
2. Position fraction `p`: for a tactic mover (reason `MR_TACTIC`) the horizontal distance `d` from the puppet to the destination: `p` = 0 when `d` <= the mover's inner range, 1 when `d` >= its outer range, else `(d - inner) / (outer - inner)`; for a band mover `p` = **the area ratio** (asd field 7: 1.0 at the band's inner edge, 0.0 at its outer edge). (The two conventions run in opposite directions; the test of step 3 is symmetric, so it does not matter.)
3. **If `sh x 0.5` <= `p` <= `1 - sh x 0.5` (0.125 to 0.875: the middle of the area): `StopMoving`** (the puppet stands). **Otherwise** move to the desired range: `D = inner + (outer - inner) x blend(0.5 - 0.5 c, 0.5 + 0.5 c)` (0.25 to 0.75 of the width, fixed per puppet; the interval as in 3), the goal point is **the destination plus the unit vector from the foe to the puppet times `D`** (the point on the ray from the foe through the puppet, at distance `D`), `AdjustGPAltitude`, **`ValidateAndSetGoalPoint(point, false, en_bFoeSeenOnLastTest)`**.
4. The result of `ValidateAndSetGoalPoint` is **ignored**: the mover always returns `CMR_OK` (it never asks for navigation after the first step).

### 5. `CMCPatternMoving` (medium)

Members: `pm_fPatternPeriod` `P0` (the minimum, 3 in 9 of 10 `.cb` movers, 5 in 1), `pm_fPatternPeriodMax` `P1` (6, 8 or 10), graphs `pm_pgrHAngleMin`, `pm_pgrHAngleMax`, `pm_pgrPatternMultiplier`.

1. Base. `t` = now - the mover's start; **period** `P` = `P0 + (P1 - P0) x en_fRnd`; **phase** `x` = (`t` mod `P`) / `P` in [0, 1).
2. Heading offset `A` (degrees): both angle graphs set: `A` = `blend(HAngleMin(x), HAngleMax(x))`; only `HAngleMin`: `A = HAngleMin(x)`; only `HAngleMax`: `HAngleMax(x)`; neither: 0.
3. **Multiplier**: when `pm_pgrPatternMultiplier` is set: `q` = for a tactic mover (reason 3) the horizontal distance from the puppet to the foe divided by the 3-D distance from the mover's **start position** to the foe (clamped to [0, 1]; 0 when the distance is 0), else the area ratio (field 7); **`A := PatternMultiplier(q) x A`**.
4. Direction `w`: the **horizontal** unit vector from the puppet to the destination, **rotated by `A` degrees about the vertical axis** (the engine's heading convention as in 3). Goal point = the puppet's position + `w x max(v, 0.1) x max(T, 0.1)` (one step, both factors at least 0.1), `AdjustGPAltitude`, **`ValidateAndSetGoalPoint(point, false, true)`**.

Data (**38 movers: 10 in 9 `.cb` files and 28 in 16 worlds**; script blocks 1 and 3): `pm_fPatternPeriod` 5 (16), 3 (12), 2 (4), 2.5 (4), 1, 1.5; `pm_fPatternPeriodMax` 10 (11), 6 (9), 5 (9), 8 (7), 3, 15. `pm_pgrHAngleMin` is set in all 38 and is a `CSineGraph` through (0, 0) and (1, 1) in 35 of them (two have other points and one is a value graph): **value = scale x sin(2 pi x)**, a sine of the phase with the amplitude `scale` degrees: scale 30 (6), **80 (6)**, 20 (5), **-80 (5)**, 45 (4), 25 (2), **-45 (2)**, 75 (2), 70, 50, 10, 1 (a negative scale starts the weave to the other side). `pm_pgrHAngleMax` in 10 (sine, scale 70 in 5, 50 in 2, 80 in 1, others), null in 28. `pm_pgrPatternMultiplier` in 35: step-up 0.5 -> 1 over 0..1 (17: the swing grows from half to full as `q` rises), step-down 1 -> 0 (6, plus 1 over 0..0.7), step-up 0 -> 1 (2), a sine (6), linear (3) (the multiplier is over `q`, the distance or area ratio, not over the phase). So a puppet weaves around the straight line to the foe with a heading swing of 20 to 80 degrees every 1 to 15 s.

### 6. `CMCDiamond` (medium)

Members: `mc_amdAllovedMovingDirections` `AMD` (0 left, 1 right, 2 both), `mc_fMinOffset`, `mc_fMaxOffset`, `mc_fRandomOffset`, `mc_fRandomizeOffsetPeriod`, `mc_fDispersionDistance` `Dd`, `mc_fFocusingDistance` `Fd`, run-time `mc_fOffsetRnd`, `mc_tmLastOffsetRandomization`. Let `S` = the mover's **start position** (the puppet's position at its first call), `F` = the destination, `L` = |`F - S`| (3-D), `u` = unit(`F - S`), **`p` = `u` x (0, 1, 0)** (the horizontal perpendicular, of length 1 when `u` is horizontal; which side of the line a positive offset is on was not checked).

1. **Side offset** `O` = `blend(mc_fMinOffset, mc_fMaxOffset)`; **negated** when `AMD` is 0, or when `AMD` is 2 **and `mc_fRnd` > 0.5** (the mover's own coin: both sides are used by different movers).
2. When `mc_fRandomizeOffsetPeriod` is above 0: every that many seconds `mc_fOffsetRnd` := a fresh draw minus 0.5 (in [-0.5, 0.5)); `O := O + mc_fOffsetRnd x mc_fRandomOffset`.
3. `d` = |puppet - `S`| (3-D distance covered from the start). **Phase A (`d` < `Dd`, the dispersion)**: goal = `S + u x Dd + p x O` (fan out sideways while keeping the line direction). **Phase B (`d` >= `Dd`)**: the progress along the line `a = (puppet - S) . u` (**horizontal components only**: the height difference is ignored); if **`a` <= `L - Fd`** (not yet within the focusing distance of the foe): `t = clamp(d / L + 0.1, 0, 1)` (1 when `d` >= `L`), goal = `S + (F - S) x t + p x O`; **else (inside the focusing distance): goal = `F`**.
4. `AdjustGPAltitude`; **`ValidateAndSetGoalPoint(goal, false, true)`**; the result is returned.

So several puppets sharing the mover but with different side offsets spread out to a diamond around the line from their start to the foe, close in at the pace of the progress, and converge on the foe within `Fd`. Data: **19 stored in 9 worlds** (tactic movers), `AMD` 2 in all 19, `mc_fMinOffset` 0 to 25, `mc_fMaxOffset` 4 to 60, `mc_fRandomOffset` 0 to 10, period -1 in 13 of 19 (3, 4, 5 otherwise), `Dd` 5 to 60, `Fd` 5 to 75 (`behavior-mover-classes.md` 3.4; script block 2).

### 7. `CMCMellon` (medium)

Members: `mc_amdAllovedMovingDirections` `AMD`, `mc_fMinAngle`, `mc_fMaxAngle`, `mc_bFadeOutOnTimer`, `mc_bFadeOutOnDistance`; the base's `mc_fAutoOffTime` `To` and `mc_fAutoOffDistance` `Do` are read as the fade time and the fade distance.

1. Base. `S` = start position, `F` = destination, `L` = |`F - S`|, `u` = unit(`F - S`), `w` = unit(`F` - own position) (3-D).
2. **Distance fade** `fd`: 1; if `mc_bFadeOutOnDistance` and `Do` > 0: `fd = 1 - clamp(((own - S) . u - Do) / L, 0, 1)`.
3. **Time fade** `ft`: 1; if `mc_bFadeOutOnTimer` and `To` > 0: `ft = 1 - clamp((now - start) / To, 0, 1)`.
4. Angle `A` (degrees) = `(mc_fMinAngle + (mc_fMaxAngle - mc_fMinAngle) x mc_fRnd) x ft x fd`; **negated** when `AMD` is 0 or (`AMD` is 2 and `mc_fRnd` > 0.5) (note: the same `mc_fRnd` picks the angle and the side, so with `AMD` 2 the right-hand movers get the larger angles).
5. `w` is rotated by `A` degrees about the vertical axis (the engine's heading convention), the goal point is **the puppet's position + the rotated vector x `v`** (one second of travel at nominal speed: **no move-period factor**), `AdjustGPAltitude`, **`ValidateAndSetGoalPoint(point, false, true)`**.

So a Mellon swings its approach direction away from the foe by 0 to 90 degrees to one side and **fades the swing out** with time or distance only when the base auto-off values are positive: **in the stored movers `To` and `Do` are -1, so `ft` and `fd` are 1 and the angle stays constant**; only a tactic that copies a time or distance into the mover (`brain-movement.md` 3.3) makes the fade work. 14 stored (all in worlds), `AMD` 2 in 11, 0 or 1 in 3, `mc_fMinAngle` 0, 10, 20, `mc_fMaxAngle` 20 to 90, `mc_bFadeOutOnDistance` 1 in 11.

### 8. `CMCAirAttack` (medium-low)

Members: `mc_pgrAltitude`, `mc_pgrMaxAltitude`, `mc_pgrHorizontalOffset` (null in all 12), `mc_fAltitudeCorrectingFactor` `K` (0.5 in 11, 0.9 in 1), `mc_fFreezeDirectionDistance` `Z` (20 in 9, 50 in 2, 75 in 1), `mc_fMaxFlyAwayDistanceDistance` `M` (-1 in 10, 250 in 2); run-time `mc_bTargetFrozen`, `mc_vFrozenFoe`, `mc_bFrontFrozen`, `mc_vFrozenFront`, `mc_bWeWereMostlyFront`; the base's `mc_pgrMovingPeriodOverride` is set in all 12 and is the constant **0.1 s**. `UsesNavigation` is false. The data are **12 movers in 8 `.ep` files** (the shooting processes of flying enemies) and 1 in a world. Notation: `P` the puppet's position, `h` the horizontal distance to the foe, `T` = `GetMovePeriod(distance)` (0.1 s), `v` the nominal speed, **`alt(e)`** = `blend(mc_pgrAltitude(e), mc_pgrMaxAltitude(e))` (the first alone when no maximum graph).

0. Base; **without a foe return `CMR_REPLACE_ME`**.
1. **Front frozen** (`mc_bFrontFrozen`, a straight pass already committed): if `M` is not -1 and `h` > `M`: clear `mc_bFrontFrozen` and `mc_bWeWereMostlyFront` and continue with step 2. Else: with `f` = `mc_vFrozenFront` (x, z used), `s` = the sign of `(F - P) . f` (horizontal; -1, 0 or +1), the goal is `P + 2 x v x T x (f.x, 0, f.z)` with its height **`F.y + alt(s x h)`**; `ValidateAndSetGoalPoint(goal, false, true)`; return. The altitude graph is read over the **signed** horizontal distance: positive before the foe is passed, negative after (the stored graphs have points at x < 0 in 5 of the 12).
2. **Freeze the target.** If `mc_bTargetFrozen` is false and `h` < `Z`: `mc_vFrozenFoe` := the foe's position, `mc_bTargetFrozen` := true. The target `Tg` = `mc_vFrozenFoe` when frozen, else the destination.
3. `f0` = the puppet's front direction (unit, `GetFrontDirection`: from its heading and pitch); `d` = `Tg - P`; `T'` = `GetMovePeriod(|d|)`; `Q` = `P + f0 x v x T'` (where it would be after a step straight ahead); `e` = |`Q - Tg`| (3-D); `a` = `alt(e)`; `ho` = `mc_pgrHorizontalOffset(e)` when set (never).
4. The goal: horizontally **`P` + 2 x (unit horizontal direction of `d`) x `v` x `T'`** (+ the perpendicular `(-d.z, 0, d.x)` normalised times `ho`); vertically **`P.y + (Tg.y + a - P.y) x K`** (the height moves a fraction `K` of the way to the wanted altitude per step).
5. **Commit rule.** While `mc_bWeWereMostlyFront` is false: it becomes true when the puppet's front direction and the horizontal direction to the foe make an angle below **45 degrees** (cos 45 < `f0 . unit(horizontal F - P)`); until then the goal of step 4 is used as is. Once true and the front is not frozen: when `f0 . (F - P)` > 0 (the foe is ahead) **and** `(F - (P + 0.5 x v x f0 horizontal)) . f0` < 0 (half a step later the foe would be behind), `mc_vFrozenFront` := the front direction (x, y, z) and `mc_bFrontFrozen` := true.
6. `ValidateAndSetGoalPoint(goal, false, true)`; return its result.

So the flyer approaches and descends along the altitude graph (the stored curves rise from about 10 to 40 over 0 to 100 m, e.g. 10 at 0, 14.9 at 50, 14.6 at 100; or 30 to 42), aims at a **frozen** point once within `Z` of the foe, and as it is about to pass over, locks its heading and flies straight on, its altitude following the graph over the negative distance, until it is `M` (250 in 2 of 12, else unlimited) away; then it turns for a new pass. `mc_bTargetFrozen` is **not** cleared by that reset (a fresh mover copy is made whenever the brain re-chooses its mover). **Medium-low** for the step 5 geometry (decompiled text with lost variables).

### 9. `SDynamicMoveData` and `CMoveCharacterData`

`SDynamicMoveData` (3 exported functions: two constructors and `mdGetDataType`) stores **only its virtual table**: no data member, no member table, no stored instance, no caller found: it is **empty and unused** (medium). The brain's `en_mcd` is the run-time record `CMoveCharacterData` (named in `AddDebugInfo`'s signature), laid out as in 1.1: **correction** of `behavior-mover-classes.md` 4.5 ("probably this class").

### 10. Data (script)

See the `Layout` sections for the numbers per mover; the script prints per class the files and objects per extension, every member's values and every graph. Totals (script block 1): `CMCCircleAroundFoe` 26 objects in 24 `.cb` files, 2 in 1 `.ep` file and **32 in 9 worlds** (the tactic copies, which do use their own ranges: inner 4 to 40, outer 10 to 50); `CMCStayInArea` 7 in 5 `.cb` files and 14 in 8 worlds (inner 8 to 50, outer 16 to 70); `CMCPatternMoving` 10 in 9 `.cb` files and 28 in 16 worlds; `CMCDiamond` 19 in 9 worlds; `CMCMellon` 14 in 7 worlds; `CMCAirAttack` 12 in 8 `.ep` files and 1 in a world. `mc_fAutoOffTime` and `mc_fAutoOffDistance` are -1 and `mc_bStopTacticWhenHasFoe` 0 in all.

## Invariants

1. The speed multiplier `mc_fSpeedMultiplier` is 1.0 in 59 of 60 circle movers (0.75 in 1), 19 of 21 stay-in-area movers (1.3 in 2), 38 of 38 pattern, 18 of 19 diamond (1.25 in 1), 14 of 14 mellon and 13 of 13 air-attack movers (script block 2).
2. The 26 band-stored circle movers (and the 2 of the `.ep` files) and the 7 band-stored stay-in-area movers have ranges that the code never reads for a band mover (sections 3 and 4): inner 8 and outer 16 in 25 of the 28 circle and in 7 of 7 stay-in-area objects.
3. The pattern minimum-angle graphs are sine graphs in 36 of 38 (35 through (0, 0) and (1, 1)); the scale is the amplitude in degrees (20 to 80, 7 negative, 2 of 1.0).
4. All 12 air-attack movers have a constant move period of 0.1 s; none has a horizontal-offset graph; 2 have a maximum altitude graph.
5. `cb_pgrThinkPeriodOveride` (read by the brain's `GetMovePeriod`, not by the oscillator) is a constant 0.1 s in 9 behaviours and 0.01 s in 1 (script block 3).

## Open questions

1. **The movement step** that consumes the goal and run-through points, `en_fSpeedMultiplier`, the look-ahead flag and the thrust cone: **T01-141**.
2. **The turning matrix sign** (which screen direction `mc_bCircleRight` circles; the side of the diamond's `p`), and the run-time check of the circle radius, the pattern weave and the air-attack pass: **T01-158**.
3. **Whether a placed puppet keeps the constructor's `en_fRnd`** (the member is stored in the world's puppet record; the stored values were not counted): **T01-281**.
4. **The markers movers, `CachePath` and `RecheckWaypoints`**: **T01-155**.
5. **The air attack's exact step-5 geometry** (decompiler text with lost variables), the stored `en_fRnd` of placed puppets (3 above) and the turning matrix's product order: **T01-281**.

## Test vectors

- **Destination** (1.1): foe seen on the last test -> the live position; not seen -> the last seen position.
- **Auto-off distance** (1.2): `mc_fAutoOffDistance` 10, foe at horizontal distance 9.9: `CMR_REPLACE_ME`; at 10.0: `CMR_OK`.
- **Towards** (1.4): puppet at the origin, foe seen at (30, 0, 0): goal point (30, 0, 0) as a run-through point.
- **Away** (1.4): puppet at the origin, foe at (10, 0, 0), next band distance 70: goal (-70, 0, 0).
- **Circle radius** (3): band 8 .. 16, `en_fRnd` 0.5: `f` = 0.5, `R` = 12; `en_fRnd` 0: `R` = 10; 1: 14. With `v` 3 and `T` 2 the step `s` = 6: `theta` = -(6 / 12) x (+1) = -0.5 rad for `R` 12 and `mc_bCircleRight` true (+0.5 rad when false).
- **Stay in area** (4): band 8 .. 16, area ratio 0.5 (puppet in the middle): `StopMoving`; ratio 0.05: move to `D` = 8 + 8 x (0.25 + 0.5 `en_fRnd`).
- **Pattern** (5): `P0` 3, `P1` 3, sine graph scale 30, 0.75 s into the cycle (`x` = 0.25): `A` = 30 x sin(pi/2) = 30 degrees; at `x` = 0.5: 0; with a step-down multiplier 1 -> 0 over [0, 1] and area ratio 0.5: `A` x 0.5.
- **Diamond** (6): start (0, 0, 0), foe (0, 0, -100), `Dd` 20, `Fd` 30, `O` 10: phase A goal (0, 0, -20) + `p` x 10; at 10 m progress and 40 m: `t` = clamp(40/100 + 0.1) = 0.5 -> (0, 0, -50) + `p` x 10; at progress 75 (>= 100 - 30): goal = the foe.
- **Mellon** (7): `mc_fMinAngle` 10, `mc_fMaxAngle` 30, `mc_fRnd` 0.25, `AMD` 2: `A` = 10 + 20 x 0.25 = 15 degrees, **not negated** (`mc_fRnd` <= 0.5), no fade (`To`, `Do` -1): goal = position + (the unit vector to the foe turned by 15 degrees) x `v`; with `mc_fRnd` 0.75 the angle is 25 degrees and negated.
- **Air attack** (8): `h` 15 with `Z` 20: the target freezes; altitude graph (0 -> 10, 100 -> 14.6) at `e` 50: 12.3; `K` 0.5: the height moves half of the way per step.
- `tools/re/mover_steering_verify.py` prints, among others: `CMCPatternMoving` 10 objects, `pm_pgrHAngleMin` x10 sine graphs with scale 30 (6), 25 (2), 70 (1), 1 (1); `CMCAirAttack` 12 objects, `mc_fFreezeDirectionDistance` {20: 9, 50: 2, 75: 1}; the 12 altitude curves.
