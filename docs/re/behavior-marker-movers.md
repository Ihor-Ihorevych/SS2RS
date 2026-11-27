# The marker movers in full: patrol, random and closest-to-foe (T01-155, follow-up of T01-105)

- **Roadmap task:** T01-155 (follow-up of [`behavior-mover-classes.md`](behavior-mover-classes.md) open question 2; the navigation moving type, `CachePath` and `RecheckWaypoints` are in the companion page [`behavior-navigation-mover.md`](behavior-navigation-mover.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for every stored value, count and cross-table (`tools/re/marker_movers_verify.py`, all 59 worlds); **medium** for every rule below that comes from the code of `Sam2Game.dll` (each function of the three movers was read in full once and every branch that decides something was re-checked in the instruction listing, so the polarities are firm; the effect of the puppet's own goal-point handling, which these movers call, is only summarised); **medium-low** where marked. **Nothing was run on the original game**; no run-time check exists yet (T01-158).

Prerequisites: [`behavior-mover-classes.md`](behavior-mover-classes.md) (classes, members, `CharacterMoverResult`, `BehaviorOnLastMarker`), [`behavior-mover-steering.md`](behavior-mover-steering.md) (the base `MoveCharacter`, `ValidateAndSetGoalPoint`, the destination point), [`brain-movement.md`](brain-movement.md) (how the brain keeps a tactic mover, the patrol state, `TacticEnded`), [`worldcontent-markers-debug.md`](worldcontent-markers-debug.md) (the marker entity), `wld-navigation.md` (marker graph and tactics in the data).

## Summary

A **marker mover** walks a puppet along the **path markers** of the world: a marker has at most one **next** link and at most one **back** link (data `en_henNextMarker`, `en_henPrevMarker`). Three movers use them. All three keep the marker they are heading for in the puppet's `en_henCurrentPathMarker`, hand its position to `ValidateAndSetGoalPoint` as a goal point **every move period**, and are told by the puppet when the goal point is reached.

| Mover | Idea | Data (tactic movers in 59 worlds) |
|---|---|---|
| `CMCMarkersMovingOrdered` | patrol: go along next (or back) links, wait, react to the last marker | 755 (264 with a marker on the tactic) |
| `CMCMarkersMovingRandom` | after each marker wait, then go to a random other marker **of the same chain** | 24 (18 with a marker) |
| `CMCMarkersMovingClosestToFoe` | go to the chain marker nearest to the foe, along the shorter way | 36 (21 with a marker) |

Findings an implementer needs first:

1. **A marker's position is its editor primitive, 0.25 m along the marker's up axis above the record position**: 10,588 of 10,639 markers (the other 51 lie in 6 worlds and are placed elsewhere than their record says). The movers read the primitive's placement (section 1.1).
2. **`GetPathLength` of the closest-to-foe mover returns the summed 3-D length of the straight segments minus 1.0**, and -1.0 means "invalid", so a distinct marker pair whose path is shorter than 1 m reads as invalid (39 of 8,974 next-link segments are shorter than 1 m). There is **no circle case**: the word "circle" appears only in a warning text (section 4.4).
3. **The patrol mover's macro events are `TacticMarkerReached` and `LastTacticMarkerReached`**, sent to the puppet when a marker is reached (the last one when the marker is the last in the current direction). They are not the brain's `MarkerReached` and `LastPatrolMarkerReached` (correction of `behavior-mover-classes.md` 4.1). The random and closest-to-foe movers send no event (section 2.5).
4. **At the last marker of a patrol the three "ending" values of `BehaviorOnLastMarker` act at once, not after the wait**: `Stop using tactic` (1), `Disappear` (3) and `Die` (4) act at the next move step after the marker is reached; the wait drawn for that marker is ignored. Only `Go back` (0) and `Stay forever` (2) wait (section 2.4; correction of `behavior-mover-classes.md` 4.1).
5. **The random mover chooses among the markers of its current marker's single path** (the next chain from the current marker, then the back chain; the current marker is entry 0): the index is `trunc(r x n)` with `r` in [0, 1) and **index 0 is moved to index 1**, so the marker after the current one has probability 2/n, the others 1/n and the current marker 0 (for n of at least 2) (section 3).
6. **None of the three marker movers ever asks for navigation**: they can operate without a foe, which switches the base `MoveCharacter`'s path test off, and they ignore the result of `ValidateAndSetGoalPoint` (sections 2.2 and 5).
7. **The closest-to-foe mover re-evaluates every `mmctf_fBestMarkerCheckPeriod` seconds** (2 s in 27 of 36), measures the forward and backward way along the chain from the marker nearest to the puppet to the best marker, takes the shorter (ties and a missing forward way go backward) and walks marker by marker, passing through every marker but the best one (section 4.5).

## Evidence

- **Code (medium).** `CMCMarkersMovingOrdered::{FirstTimeExecuted, MoveCharacter, UpdateGoalPoint, ContinueMarkersReaching, ChangeMovingDirection, IsMarkerLastInChain, ReachMarker, OnGoalPointReached}`, `CMCMarkersMovingRandom::{FirstTimeExecuted, MoveCharacter, UpdateGoalPoint, ReachMarker, OnGoalPointReached, ContinueMarkersReaching}`, `CMCMarkersMovingClosestToFoe::{FirstTimeExecuted, GetPathMarkerClosestToPoint, MoveCharacter, GetPathLength, OnGoalPointReached, UpdateGoalPoint, ReachMarker, CacheBestMarker, SendCharacterAlongBestPath}`; the marker's `GetSinglePath`, `GetNextMarker`, `GetViewOrigin`, `OnCreate` (position of the primitive); the puppet's `StopMoving`, `SetGoalPoint`, `SetRunTroughPoint`, `GoalPointCommonReset`, `GetClosestMarker`, `ObtainFirstPathMarker`; the line-of-sight fill-in used by `CacheBestMarker`. The virtual slots were named from the vtables (so `UpdateGoalPoint`, `ContinueMarkersReaching`, `ChangeMovingDirection`, `IsMarkerLastInChain`, `ReachMarker` are the movers' five own hooks; the closest-to-foe mover has six). Every comparison that decides a branch (the `>= period` and `<= 0` tests, the last-marker tests, the direction choice of section 4.5, the random coin) was read in the instruction listing.
- **Constants (high).** 0.5 and 0.0 (the coin and the wait tests), 2^-32 (uniform random), 1.0 (`GetPathLength` offset, "closest" 3.0e38 start), 1001 steps, -100.0 s (initial "last time"). Member default values from the constructors.
- **Data (high).** `python -I tools/re/marker_movers_verify.py "<install>"` (about 15 minutes; `LIMIT` smallest worlds for a quick run). It prints every number of section 6. It reads the placed tactic movers; the `.cb` and `.ep` files hold no marker mover (`behavior-mover-classes.md`).
- **Not checked.** Nothing was run. The puppet's movement step (when it calls the mover's `OnGoalPointReached`, T01-141) was not read; the sign and unit of nothing here depend on it, but **when** a goal is reached does. The effect of `DoDisappear` and `ApplyLethalDamage` on the puppet is not specified here.

## Layout / rules

### 1. Shared facts

**1.1 The marker.** `en_henNextMarker` (data `.2`) and `en_henPrevMarker` (`.4`) are entity handles. The marker's `OnUpdate` makes the next link of its record the entity link and sets that next marker's back link to itself (`worldcontent-markers-debug.md`); the back link therefore names **one** predecessor (597 of 8,974 next links have a target whose back link names another marker). **Position:** the movers call the marker's `GetAbsPlacement`, which is the placement of the marker's editor primitive (`en_hre`). The primitive is created by `OnCreate` at the record placement plus **0.25 m along the rotated local Y axis** (an unrotated marker: 0.25 m up), and the saved primitive keeps that placement. Data: **10,588 of 10,639 markers** have the primitive at record position + 0.25 x (the rotated up axis of the record's quaternion) within 0.001 m; the other 51 markers lie in `3_2_ChiFang_City` (16), `4_4_BossArena` (14), `7_2_City_Part_II` (6), `7_7_Mental` (6), `5_6_SkyIslands` (5) and `2_1_Swamp` (4) and have a primitive that is not at the record position (those records hold a template placement; the primitive is the live one). **Implementers use the primitive's position** (the placement stored with the marker object), not the record's.

**1.2 The single path** (`CPathMarkerEntity::GetSinglePath(array)`): the array is cleared; the marker itself is added, then its next marker, and so on until the next link is empty or the marker is already in the array (loop-safe); then the **back chain of the starting marker** is added (its back link, that marker's back link, ...) with the same stop rule. Entry 0 is the marker itself, the next chain follows in order, the back chain after it. In a closed loop the back chain adds nothing. Data (all 10,639 markers): single path sizes 1 (247), 2 (1,112), 3 (1,171), 4 (1,245), 5 (1,116) ... (script).

**1.3 The goal point call.** Every `ReachMarker` calls the mover's `ValidateAndSetGoalPoint(marker position, limit = false, pass-through = p)` (`behavior-mover-steering.md` 1.4). With `p` true the puppet gets a **run-through point** (it does not stop at it), with `p` false a **goal point** (it stops there and the arrival ends the leg). **All three movers ignore the returned value.**

**1.4 The first call.** The base `MoveCharacter` runs first in every mover (`behavior-mover-steering.md` 1.2): on the first call it runs `FirstTimeExecuted`; it sets the look target and the look-ahead flag; the navigation test is skipped because the movers answer "yes" to `CanOperateWithoutTargetEntity`; steps 4 and 5 (auto-off time and distance, stop-tactic-when-foe) apply to a tactic mover with the tactic's values. Only when the base returns 0 does the marker code run.

**1.5 Random numbers.** The engine's global generator (`behavior-action-choice.md` 7; `hitscan-shot.md` section 4): `r` = uniform in [0, 1), the 32-bit value times 2^-32 evaluated in extended precision (so **r < 1 strictly** here, unlike the 32-bit-float `r` of other pages).

### 2. `CMCMarkersMovingOrdered` (the patrol mover)

**2.1 Members** (declared order and data names; defaults are the constructor's, the data always overwrites them): `mm_imdInitMarkersDir` (`InitialMarkersDirection`: `IMD_FORWARD` 0, `IMD_BACKWARD` 1, `IMD_RANDOM` 2; default 0), `mm_fChangeDirectionPeriod` (default -1), `mm_fWaitOnMarkerMin`, `mm_fWaitOnMarkerMax` (default 0, 0), `mm_bolmBehaviorOnLastMarker` (default 2), run-time `mm_bMovingTowardNextMarker` (default true), `mm_tmLastDirectionChange`, `mm_tmMarkerReached`, `mm_bMarkerReached` (0), `mm_bLastMarkerReached` (0), `mm_fWaitOnMarker` (0). A freshly copied mover has all run-time members at these values: **all 755 stored movers hold exactly (1, 0, 0, 0.0)** in the four scalar run-time members (script).

**2.2 `FirstTimeExecuted`** (runs once, from the base):

1. the base part (random `mc_fRnd`, start time, start position);
2. `mm_tmLastDirectionChange` := now; `mm_tmMarkerReached` := **-100 s**;
3. `mm_bMovingTowardNextMarker`: `IMD_FORWARD` -> true; `IMD_BACKWARD` -> false; `IMD_RANDOM` -> **true when a fresh random value `r` is above 0.5**, else false (the test is strict: a value of exactly 0.5 gives false);
4. `ReachMarker(en_henCurrentPathMarker)` (2.6). Result 0.

The puppet's current marker comes from the tactic or from `ObtainFirstPathMarker` (`brain-movement.md` 3.3 and 5.4): the tactic's marker, else the puppet's auto-patrol marker id, else the **closest path marker** (section 5.2).

**2.3 `MoveCharacter`** (called once per move period by the brain), in this order:

1. The base result; a non-zero result (a tactic's auto-off, stop-when-foe) is returned.
2. **No current marker -> return 1** (`CMR_REPLACE_ME`).
3. If `mm_bLastMarkerReached` is true, act on `mm_bolmBehaviorOnLastMarker`:
   - 3 (`BOLM_DISAPPEAR`): the puppet's `DoDisappear`; return 0;
   - 4 (`BOLM_DIE`): the puppet's `ApplyLethalDamage` with direction (0, 0, 0), damage type -1, weapon -1, no inflictor; return 0;
   - 1 (`BOLM_DESTROY_MOVER`): **return 1**;
   - 2 (`BOLM_STAY_FOREVER`): skip steps 4 and 5, go to step 6;
   - 0 (`BOLM_GO_BACK`) and any other value: continue with step 4.
4. **Direction change by time.** When `mm_fChangeDirectionPeriod` is above 0 **and** `now - mm_tmLastDirectionChange >= mm_fChangeDirectionPeriod`: `ChangeMovingDirection` (2.7). (The test is on the period alone: it also fires while the puppet waits on a marker and after the last marker with `BOLM_GO_BACK`.)
5. **End of the wait.** When `mm_bMarkerReached` is true **and** `mm_fWaitOnMarker` is above 0 **and** `now - mm_tmMarkerReached >= mm_fWaitOnMarker`: `mm_fWaitOnMarker` := 0, `ContinueMarkersReaching` (2.8), **return 0** (steps 6 skipped).
6. `UpdateGoalPoint` = `ReachMarker(en_henCurrentPathMarker)`: the goal point is re-issued towards the current marker. Return 0.

So while a puppet waits, step 6 keeps issuing the goal point of the marker it stands on (a goal point, not a run-through point, because the wait is above 0, 2.6).

**2.4 `OnGoalPointReached`** (the puppet tells the mover that the goal point is reached; the brain routes it, `brain-movement.md` 5.3):

1. **Nothing at all when `mm_bMarkerReached` is already true** (a marker is handled once).
2. `mm_tmMarkerReached` := now; `mm_bMarkerReached` := true.
3. **Wait draw:** `mm_fWaitOnMarker` := `min + (max - min) x r` with a fresh `r` in [0, 1) (so min and max may be equal, or reversed: then the wait lies between them; one stored mover has -1 and -1).
4. The puppet's `en_henLastReachedMarker` := its current marker.
5. `last` := `IsMarkerLastInChain(current marker)` (2.9); when `last`, `mm_bLastMarkerReached` := true. Let `ending` := `mm_bolmBehaviorOnLastMarker` is 1, 3 or 4.
6. **Wait or go on:** when `mm_fWaitOnMarker <= 0`: **if not (last and ending)**, `ContinueMarkersReaching` at once (2.8); when last and ending nothing is called here (the next `MoveCharacter` acts on `mm_bLastMarkerReached`, step 3 above). When `mm_fWaitOnMarker > 0`: the puppet's **`StopMoving`**.
7. **Macro event** sent to the puppet (`macSendEvent`): **`LastTacticMarkerReached` when `last`, else `TacticMarkerReached`.** Sent after steps 1 to 6, also when the mover has just asked for the end.

**Consequence of 3 and 6 (medium):** with `BOLM_DESTROY_MOVER`, `BOLM_DISAPPEAR` or `BOLM_DIE` the drawn wait is never honoured at the last marker (the next `MoveCharacter` returns, disappears or kills first), while a wait at an ordinary marker or at the last marker with `BOLM_GO_BACK` or `BOLM_STAY_FOREVER` is. With `BOLM_STAY_FOREVER` the puppet after the last marker only re-issues the last goal point (step 6) for ever; `mm_bLastMarkerReached` is never cleared (also not by `BOLM_GO_BACK`, which does not read it).

**2.5 Events and the brain.** The brain's own patrol state sends `MarkerReached` and `LastPatrolMarkerReached` (`brain-movement.md` 5.3); the mover's two events are separate and belong to **tactic patrols**. `BOLM_DESTROY_MOVER` returning 1 makes the brain send `TacticEnded` (reason tactic) and take the tactic's next tactic (`brain-movement.md` 4.2). `BOLM_DISAPPEAR` and `BOLM_DIE` end the puppet, not the tactic. The random and closest-to-foe movers send **no** macro event.

**2.6 `ReachMarker(m)`**: nothing when `m` is empty. Otherwise: `last` := `IsMarkerLastInChain(m)`; if the puppet's current marker is **not** `m`, `mm_bMarkerReached` := false and the current marker := `m`; **pass-through** `p` := (`mm_fWaitOnMarker` equals 0.0); **but `p` := false when `last` and `mm_bolmBehaviorOnLastMarker` is 2**; then `ValidateAndSetGoalPoint(position of m, false, p)`. Hence: a puppet with no wait runs through every marker without stopping and stops only at a last marker with `BOLM_STAY_FOREVER`; with `BOLM_DESTROY_MOVER`, `BOLM_DISAPPEAR`, `BOLM_DIE` and `BOLM_GO_BACK` the last goal is a run-through point too (the puppet stops only because the mover ends or turns). `mm_fWaitOnMarker` is the wait **of the marker just reached** until step 5 of 2.3 clears it, so `p` of the next marker is true again after a wait.

**2.7 `ChangeMovingDirection`**: `mm_bMovingTowardNextMarker` := not itself; `mm_tmLastDirectionChange` := now; `ReachMarker(neighbour of the current marker in the new direction)` (its next marker when the flag is now true, its back marker when false; an empty neighbour does nothing). It does not clear `mm_bMarkerReached` itself (`ReachMarker` does, because the marker changes).

**2.8 `ContinueMarkersReaching`**: `m` := the current marker's next marker when `mm_bMovingTowardNextMarker`, else its back marker. **If `m` exists: `ReachMarker(m)`. Otherwise: if `mm_bolmBehaviorOnLastMarker` is 0 (`BOLM_GO_BACK`): `ChangeMovingDirection`** (turn round and walk to the neighbour on the other side, if any; at a chain of one marker nothing happens). For the other values nothing happens.

**2.9 `IsMarkerLastInChain(m)`**: when moving towards the next marker: `m` has **no next marker**; when moving back: `m` has **no back marker**. So the "last" marker depends on the **current direction**, and on a closed loop no marker is ever last.

**2.10 Edge cases (medium).**

- *First marker is the last* (28 of the 264 stored patrols with a marker start on a marker that has no next link: 26 with `IMD_FORWARD`, 2 with `IMD_BACKWARD`; a chain of one marker is the extreme case): the first `ReachMarker` aims at the marker itself; on arrival `last` is true (with `IMD_FORWARD` at the end of a chain). With `BOLM_GO_BACK` `ContinueMarkersReaching` turns round immediately.
- *Wait at a go-back turning point:* `OnGoalPointReached` draws the wait; if it is above 0 the puppet stops; after the wait `ContinueMarkersReaching` finds no marker and turns round (so the wait is honoured before the turn).
- *A merge:* the back link names one predecessor; walking backwards from a merge follows that one.
- *A closed loop* never has a last marker (`IsMarkerLastInChain` is false everywhere), so `BehaviorOnLastMarker` is inert on the 62 stored patrols whose marker lies on a loop (they only end by the tactic's auto-off members, a foe, or a direction change by time).

### 3. `CMCMarkersMovingRandom`

**Members:** `mmr_fWaitOnMarkerMin` (default 1.0), `mmr_fWaitOnMarkerMax` (default 2.0), run-time `mmr_fWaitOnMarker` (default 0), `mmr_tmMarkerReached`, `mmr_bMarkerReached` (default false). All 24 stored movers hold wait 0 and reached false.

**`FirstTimeExecuted`**: base part; `mmr_tmMarkerReached` := -100 s; `mmr_bMarkerReached` := false. (It does **not** send the mover to a marker: the first goal point is issued by step 3 below.)

**`MoveCharacter`**: base result (non-zero returned); **no current marker -> return 1**; then, when `mmr_bMarkerReached` is true **and** `mmr_fWaitOnMarker` is above 0 **and** `now - mmr_tmMarkerReached >= mmr_fWaitOnMarker`: `ContinueMarkersReaching`; **always** `UpdateGoalPoint` = `ReachMarker(current marker)`; return 0.

**`ReachMarker(m)`**: nothing when empty; if the puppet's current marker is not `m`: `mmr_bMarkerReached` := false and the current marker := `m`; `ValidateAndSetGoalPoint(position of m, false, pass-through = (mmr_fWaitOnMarker equals 0.0))`.

**`OnGoalPointReached`**: nothing when `mmr_bMarkerReached` is already true; else `mmr_tmMarkerReached` := now, `mmr_bMarkerReached` := true, `mmr_fWaitOnMarker` := `min + (max - min) x r`; **when the wait is `<= 0`: `ContinueMarkersReaching` at once; otherwise the puppet's `StopMoving`**. (No last-marker logic, no event, the puppet's last reached marker is **not** updated.)

**`ContinueMarkersReaching`**: no current marker -> nothing. Otherwise the current marker's single path (1.2) of `n` markers; `k` := `trunc(r x n)` (`r` in [0, 1), so `0 <= k < n`); **if `k` equals the index of the current marker (always 0, the current marker is entry 0) then `k` := `(k + 1) mod n`**; `ReachMarker(entry k)`.

| `n` | P(entry 0, the current) | P(entry 1, the next) | P(each of entries 2 to n-1) |
|---|---|---|---|
| 1 | 1 | - | - |
| 2 | 0 | 1 | - |
| 3 | 0 | 2/3 | 1/3 |
| 4 | 0 | 1/2 | 1/4 |
| n | 0 | 2/n | 1/n |

The entries are the markers **ahead along the next links first**, then the markers behind: the random mover can jump to any marker of the chain, not only a neighbour (`behavior-mover-classes.md` 4.2 said "connected ones"). Data: the 18 stored movers with a tactic marker have single paths of 3 (10), 5 (2), 8 (2), 1, 4, 7 and 20 markers; waits 0.5 to 1.5 as minimum, 1.0 to 5.0 as maximum (minimum 1.0 in 16 of 24, maximum 2.0 in 16 of 24; two movers have 0 and 0).

### 4. `CMCMarkersMovingClosestToFoe`

**4.1 Members:** `mmctf_bTestLOS` (default false), `mmctf_bMustFollowPath` (default false), `mmctf_fBestMarkerCheckPeriod` (default 2.0), run-time `mmctf_tmLastCheckTime`, `mmctf_henCurrentBestMarker`, `mmctf_henLastReachedMarker`, `mmctf_bMovingForwardAlongPath`. All 36 stored movers have both handles empty and the direction flag 0. Stored flags (test LOS, must follow path): (1, 1) 21, (1, 0) 7, (0, 0) 5, (0, 1) 3; period 2.0 (27), 1.0 (3), 10.0 (2), 5.0 (2), 8.0 (2).

**4.2 `FirstTimeExecuted`:** base part; `mmctf_tmLastCheckTime` := -100 s; **when `mmctf_bMustFollowPath`: `SendCharacterAlongBestPath`** (4.5). Result 0.

**4.3 `MoveCharacter`:**

1. The base result (non-zero returned); **no current marker -> return 1**.
2. If `now - mmctf_tmLastCheckTime` is **less than** the period: `UpdateGoalPoint` (4.7); return 0.
3. Otherwise `mmctf_tmLastCheckTime` := now and
   - `mmctf_bMustFollowPath`: `SendCharacterAlongBestPath`; return 0;
   - else: `CacheBestMarker`, then `ReachMarker(best marker)`; return 0.

(The first call runs both the check of `FirstTimeExecuted` and, because the last check time is -100 s, the check of step 3 again.)

**4.4 `GetPathLength(a, b, forward)`** (path length along the chain from `a` to `b`):

1. `a` equals `b`: return **0.0**.
2. Otherwise start a sum at **-1.0** and repeat at most **1001** times: take the neighbour `n` of the current marker (next marker when `forward`, back marker otherwise); **no neighbour: return -1.0**; add the 3-D distance between the positions of the current marker and `n` to the sum; **`n` equals `a`** (the way came back to the start): warning `Invalid path provided to GetPathLength()` and return -1.0; **`n` equals `b`: return the sum**; else continue from `n`. After 1001 repetitions without success: warning `GetPathLength(): Markers %1 and %2 are not properly connected (neither a straight line, nor a circle).` (with the two entity ids) and return -1.0.

So the result is **the sum of the segment lengths minus 1.0**, and -1.0 means invalid. A valid way of less than 1.0 m in total reads **negative** (invalid): the data have 39 next-link segments shorter than 1 m (2 shorter than 0.01 m); it matters only when such a segment is the whole way. Because both directions carry the same offset, the comparison of 4.5 is unaffected otherwise. **There is no circle case**: only straight segments between consecutive markers are summed; the message text is a leftover.

**4.5 `SendCharacterAlongBestPath`** (the heart of the mover):

1. `CacheBestMarker` (4.6) sets the best marker `B` (`mmctf_henCurrentBestMarker`). **`B` empty: the puppet's `StopMoving` and stop.**
2. The single path of the puppet's current marker (1.2). `C` := the marker of that array **nearest (3-D distance) to the puppet's own position**; with equal distances the first in array order (strict comparison; initial best 3.0e38).
3. `F` := `GetPathLength(C, B, forward)` and `K` := `GetPathLength(C, B, backward)`.
4. **Direction** `mmctf_bMovingForwardAlongPath`:
   - `F` below 0: `K` below 0 too -> warning `Invalid situation occured in <closest to foe> character mover: both forward and backward paths are invalid` and **stop without any change** (no goal point is issued this time); otherwise **backward**;
   - `F` at least 0: `K` below 0 -> **forward**; both at least 0: **forward only when `F < K`**, else (`F >= K`, including a tie) **backward**.
5. Let `L` be `mmctf_henLastReachedMarker`:
   - `L` equals `B`: the puppet's `StopMoving`;
   - `C` equals `L`: `ReachMarker(` the next marker of `L` when forward, the back marker of `L` when backward `)`;
   - otherwise `ReachMarker(C)`.

**4.6 `CacheBestMarker`:** `B` := empty. When the puppet has a current marker **and** the movement record has a foe (the foe's puppet): the array := the single path of the puppet's current marker; `D` := the destination point (`GetDestinationPoint`: the foe's live position when seen on the last test, else the last seen position); then
- **no line-of-sight test:** `B` := `GetPathMarkerClosestToPoint(D, array)`;
- **`mmctf_bTestLOS`:** `B` := the closest marker to `D`; while `B` exists: run a line-of-sight request **from the marker `B` to the foe's puppet**, purpose `LRP_LOOK` (0), both categories invalid (the "bullet" category, `brain-line-of-sight.md` 1: it is the marker that looks, its view origin is its position), executed at once; **if the foe is seen, stop with this `B`**; otherwise remove `B` from the array and take the closest of the rest; the loop ends with `B` empty when the array is empty.

**`GetPathMarkerClosestToPoint(point, array)`:** the entry with the smallest 3-D distance between its marker position and `point`, initial best 3.0e38, **strictly smaller replaces** (ties keep the first); empty when the array is empty or holds no valid marker.

**4.7 `ReachMarker(m)`:** `m` empty: the puppet's `StopMoving`. Otherwise the puppet's current marker := `m` (this mover has no "reached" flag to reset) and `ValidateAndSetGoalPoint(position of m, false, pass-through = (m is not the best marker))`, so **only the best marker is a goal point, every other marker a run-through point**. `UpdateGoalPoint`: when the best marker exists, `ReachMarker(current marker)`, else nothing. **`OnGoalPointReached`:** `mmctf_henLastReachedMarker` := the puppet's current marker; when `mmctf_bMustFollowPath`, `SendCharacterAlongBestPath`. Without the follow flag the puppet walks **straight to the best marker** (step 3 of 4.3: `ReachMarker(B)`), not along the chain.

**4.8 Behaviour in words (medium).** Every period the mover finds the marker `B` of its chain nearest to the foe (and in sight of it with the LOS flag). With "must follow path" the puppet first goes to the chain marker `C` nearest to itself, then one marker at a time along the shorter direction until it stands on `B`; when it stands on `B` it stops and keeps checking every period (the foe moves; `B` changes). Without the flag it runs straight to `B`. The chain is always **the single path of the puppet's current marker**, so after the first marker change the choice is limited to one chain; on a closed loop the "shorter way" is whichever direction is shorter (the lengths of the two ways differ by the loop).

### 5. Interactions

**5.1 Brain and puppet.** The brain keeps the mover while its reason is tactic (`brain-movement.md` 3.1), calls `MoveCharacter` each move period (`CanOperateWithoutTargetEntity` is true for all three, so a puppet without a foe also runs it, `brain-movement.md` 4.1) and routes `OnGoalPointReached` to it (5.3 there). A tactic that is stopped while a patrol runs leaves the puppet's `en_henCurrentPathMarker` as it was.

**5.2 The first marker.** The tactic's `en_henTacticMarker` becomes the puppet's current marker when set (306 of 993 tactics); else, when the puppet has none, `ObtainFirstPathMarker()`: the entity named by `en_ulAutoPatrolMarkerEntityID` when that is not -1 (and a path marker), else **`GetClosestMarker`: the path marker nearest to the puppet's own position by 3-D Euclidean distance, no distance limit** (every entity of class `CPathMarkerEntity` or a subclass is tested; the first strictly smaller distance wins, so a tie keeps the entity seen first; the initial best is 3.0e38; **medium**; this is the rule T01-242 asked for). `GetGoalMarker` (not used by the movers) returns the closest marker, or its next marker when the puppet is within 1.0 m of it. The puppet that has no marker at all and no marker in the world gets an empty marker: the mover returns 1 at once (step 2 of 2.3 / 3 / 4.3).

**5.3 What the goal-point call does** (summary of the puppet side, **medium-low**; T01-141): `SetGoalPoint(p)` and `SetRunTroughPoint(p)` end in the puppet's `GoalPointCommonReset(p, 1)`: **when the same point is already the active goal nothing changes** (this is why the movers can re-issue the goal every period); **else when `p` is within 0.01 of the puppet's last reached goal point and the puppet is within **half of its current nominal speed** (metres, `GetCurrentNominalVelocity` x 0.5) of `p`** (3-D, or horizontal only when the puppet does not move in 3-D)**: the puppet stops (`StopMoving`) and the call reports "already there" (a goal that is not set up: the mover's `OnGoalPointReached` is **not** called by this path); otherwise the point becomes the active goal. The puppet's movement step calls the brain's `OnGoalPointReached` on arrival (T01-141). A puppet that **starts on top of its first marker** (or re-issues a goal at the point it just reached) therefore may stand still without a mover callback; whether the movement step also reports it was not read (**open**, T01-141).

### 6. Data (script `tools/re/marker_movers_verify.py`, all 59 worlds; high)

- **Markers:** 10,639; primitive at record + 0.25 up in 10,588 (exceptions listed in 1.1); next-link segments 8,974 with 3-D length min 0.000, median 15.76, max 2,473.8 m; shorter than 1 m: 39; shorter than 0.01 m: 2.
- **Ordered movers:** 755 (264 with a tactic marker, 491 without): initial direction forward 746, random 6, backward 3; change period -1 in 745, 20 s (5), 10 s (2), 15 s (2), 6 s (1) (10 above 0); wait minimum 0 in 677, 1.0 (28), 2.0 (20), 0.5 (15), 1.5 (10), 100 (2), -1, 10, 4 (1 each); wait maximum 0 in 651, 2.0 (41), 1.5 (13), 5.0 (11), 1.0 (9), 4.0 (8), 10 (6), 3 (6), 2.5 (5), 150 (2), 0.5, 15 and -1 (1 each); **a wait is possible (maximum above 0) in 103 movers**, **26 have a minimum at or below 0 with a maximum above 0**, one has -1 and -1; behaviour on last marker `Stop using tactic` 412, `Stay forever` 172, `Go back` 160, `Die` 9, `Disappear` 2; speed multiplier 1.0 (593), 0.35 (57), 0.25 (21), 1.25 (18), 0.75 (15) and nine more values.
- **Ordered movers with a marker (264):** the marker lies on a **closed loop** in 62, is the **first** of an open chain (no back link) in 155, the **last** (no next) in 28, in the **middle** in 19. Behaviour on last marker x chain: open chain `Stop using tactic` 96, `Stay forever` 54, `Go back` 45, `Die` 6, `Disappear` 1; **closed loop** `Go back` 43, `Stay forever` 12, `Stop using tactic` 6, `Die` 1 (on a loop the last marker is never reached, so those 62 never end). Initial direction x chain: forward at the first marker 155, on a loop 61, at a last marker 26, in the middle 16; random in the middle 3, on a loop 1; **backward at a last marker 2** (the only sensible use of `IMD_BACKWARD`; the third backward mover has no marker). Single path size of the tactic marker: 2 (48), 3 (43), 4 (36), 5 (27), 1 (24), 7 (16), 9 (14), 6 (13), 10 (11) ... (the 491 ordered movers without a marker: `Stop using tactic` 310, `Stay forever` 106, `Go back` 72, `Die` 2, `Disappear` 1: their marker is the puppet's own, 5.2).
- **Closest-to-foe:** 36 (21 with a marker: 16 on a closed loop, 5 on an open chain; single path sizes 10 (6), 2 (4), 3 (3), 9 (3), 4 (2) ...).
- **Random:** 24 (18 with a marker).

## Invariants

1. All 755 + 36 + 24 stored marker movers hold the default run-time members (2.1, 3, 4.1) (script).
2. Every next link targets a path marker (8,974 of 8,974, `worldcontent-markers-debug.md`); the single path of a marker never repeats a marker.
3. The wait of an ordered or random mover is `min + (max - min) x r`; it is **not** clamped; a value at or below 0 means no wait.
4. `GetPathLength` of a marker to itself is 0.0 and of a distinct reachable pair the segment sum minus 1.0.
5. The ordered mover handles each marker once (`mm_bMarkerReached` guard) until the puppet's current marker changes.

## Open questions

1. **When the puppet's movement step calls `OnGoalPointReached`** for a run-through point and for a point the puppet already stands on (5.3): T01-141 and the run-time check **T01-158**.
2. **What `DoDisappear` and `ApplyLethalDamage` (type -1, weapon -1) do to the puppet and whether `BOLM_DIE` kills a puppet that is invulnerable or in a ride:** T01-400.
3. **Whether the macro events `TacticMarkerReached` and `LastTacticMarkerReached` have handlers in the levels' scripts** (the script files were not searched): T01-401.
4. **The meaning of the 6 worlds' 51 markers whose primitive is not at the record position** (whether they are moved by script or by the level data at load): T01-402.

## Test vectors

- **Direction coin** (2.2): `IMD_RANDOM` with the 32-bit random value 2^31 (r = 0.5): backward; 2^31 + 1: forward.
- **Ordered, no wait, go back** (markers A -> B -> C, A first, `mm_imdInitMarkersDir` 0, wait 0 and 0, `BOLM_GO_BACK`, period -1): goal A (run-through, wait is 0); on arrival at A: `TacticMarkerReached`, `ContinueMarkersReaching` -> B; at B the same; at C `last` (no next), `mm_bLastMarkerReached` true, event `LastTacticMarkerReached`, wait 0 and `ending` false -> `ContinueMarkersReaching`: no next marker, `BOLM_GO_BACK` -> `ChangeMovingDirection`: direction back, goal B; arrival at B: B is not last in the back direction -> A; at A **last** (no back): turn round again. The puppet walks A, B, C, B, A, B, C ... for ever; the flag `mm_bLastMarkerReached` stays true and is ignored.
- **Ordered, die** (`BOLM_DIE` at C, wait 0.7 drawn): arrival at C: wait above 0 -> `StopMoving`; event `LastTacticMarkerReached`; **the next `MoveCharacter` (the next move period) applies lethal damage**: the wait is not awaited.
- **Ordered, stay forever:** at the last marker `ReachMarker` makes the goal a goal point (pass-through false), so the puppet stops exactly there; `MoveCharacter` re-issues that goal point every period.
- **Random pick** (3.): single path of 4 markers, `r` = 0.1: `k` = 0 -> 1 (the next marker); `r` = 0.3: `k` = 1; `r` = 0.6: `k` = 2; `r` = 0.99: `k` = 3.
- **Path length** (4.4): markers A (0, 0, 0), B (3, 4, 0), C (3, 4, 12), `GetPathLength(A, C, forward)` = 5 + 12 - 1 = 16.0; `GetPathLength(A, A, ...)` = 0.0; A and B only 0.4 apart: `GetPathLength(A, B, forward)` = 0.4 - 1.0 = -0.6 (invalid).
- **Closest-to-foe direction** (4.5): `F` = 20, `K` = 35: forward; `F` = 35, `K` = 35: backward; `F` = -1, `K` = 12: backward; `F` = 5, `K` = -1: forward; both -1: warning, nothing changes.
- **Pass-through** (4.7): best marker `B`; goals for any marker other than `B` are run-through points, for `B` a goal point.
- `tools/re/marker_movers_verify.py` prints, among others: `path markers: 10639`, `markers whose primitive sits 0.25 m along the up axis ...: 10588`, `ordered: behaviour on last marker: {1: 412, 2: 172, 0: 160, 4: 9, 3: 2}`, `ordered: marker lies on: {first 155, loop 62, last 28, middle 19}`, `closest: (test LOS, must follow path): {(1, 1): 21, (1, 0): 7, (0, 0): 5, (0, 1): 3}`.
