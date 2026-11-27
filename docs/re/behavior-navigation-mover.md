# The navigation moving type: paths, waypoints, failure and fallback (T01-155, follow-up of T01-105)

- **Roadmap task:** T01-155 (follow-up of [`behavior-mover-classes.md`](behavior-mover-classes.md) open question 2; the three marker movers are in [`behavior-marker-movers.md`](behavior-marker-movers.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the data side (mesh counts of `wld-navigation.md`, the constants read as exact floats); **medium** for the mover's own rules (`MoveCharacter`, `CachePath`, `RecheckWaypoints`, `OnGoalPointReached`, `DestroyPath`: each read in full once, every deciding comparison re-checked in the instruction listing); **medium-low** for the path service (node lookup, search order, waypoint construction, the straight-line test: read once, with the decompiler's lost variables) and for what the puppet does with the goal points the mover hands out (summarised, T01-141). **Nothing was run on the original game.**

Prerequisites: [`behavior-mover-classes.md`](behavior-mover-classes.md), [`behavior-mover-steering.md`](behavior-mover-steering.md) (base `MoveCharacter`, navigation conditions, destination point), [`brain-movement.md`](brain-movement.md) (4.2 and 4.3: when the brain installs this mover and what it does with its results), `wld-navigation.md` (the mesh in the data), [`brain-line-of-sight.md`](brain-line-of-sight.md).

## Summary

`CMNavigationMovingType` is the mover a brain installs (reason `MR_NAVIGATION`) when another mover returned `CMR_NEED_NAVIGATION`: it asks the world's path service for a **list of waypoints** from the puppet to the destination point, then walks the puppet along them. It is never stored in a file and has no members of its own in the DLL's tables; it exists only at run time, one copy per brain, only for ground puppets in a world whose navigation mesh has nodes (**48 of the 59 worlds**).

Findings an implementer needs first:

1. **The mover plans at most once per second.** It re-plans when the destination (the foe) has **moved by at least 0.02 x the horizontal distance from the puppet to the destination since the path was made** (both distances in metres, 3-D for the move, horizontal for the distance) **and the path is older than 1.0 s**; otherwise it keeps the path (section 2.1).
2. **Waypoints are looked ahead, six at a time.** `RecheckWaypoints` tests the next up to **6** waypoints from the current index and heads for the **farthest of the consecutive ones that the straight-line test accepts**; it fails when not even the waypoint at the index is accepted (section 2.3).
3. **The mover ends (`CMR_REPLACE_ME`) as soon as its index reaches the last waypoint**, which is the destination point itself; the brain then re-chooses the band's mover after 0.01 s (section 2.1). The last stretch is therefore walked by the original mover.
4. **Failure** is `CMR_NAVIGATION_FAILED`: no path with at least 2 waypoints, or a recheck that fails right after a fresh plan. The brain then installs a `CMCDontMove` with reason `MR_NAVIGATION_FAILED` and asks again after 1 to 3 s (`brain-movement.md` 4.2). A path that goes wrong while walking is **re-planned on the spot** (`OnGoalPointReached`, section 2.4) before failure is reported.
5. **Fallback for points off the mesh:** when the start or the end point has no mesh node under it, the "path" is the straight line start -> end (2 waypoints) and every straight-line test answers yes: the puppet walks straight at the destination (section 3.3).
6. **The path service is a search over the rectangles of the mesh** (`wld-navigation.md`): node centres, costs as sums of per-axis distances, a squared per-axis distance as the heuristic, step limits from the puppet's `en_fStepDownHeight` and `en_fStepUpHeight`, waypoints at the link centres pushed 0.01 m into the next node (section 3, medium-low).
7. **The mover's own goal is re-issued only when the index jumps**: after `OnGoalPointReached` has incremented the index, the puppet's goal is renewed only if `RecheckWaypoints` moves the index further; the play-out when it does not depends on the puppet's goal bookkeeping (section 2.4, **open**).

## Evidence

- **Code (medium).** `CMNavigationMovingType::{CMNavigationMovingType, FirstTimeExecuted, CachePath, RecheckWaypoints, OnGoalPointReached, MoveCharacter, DestroyPath, CreateDebugPathRenderable, DestroyDebugPathRenderable, ~CMNavigationMovingType}`; `CWorldInfoEntity::{FindPath, CanMoveFromPointAToPointB, CanUseNavigation}` and the path service functions behind them (node lookup, search set-up, expansion, open-list handling, waypoint construction, the walk of the straight-line test); the puppet's `ForceRunTroughPoint`, `UpdateGoalPoint`, `GoalPointCommonReset`, `StopMoving`; the base `SMoveCharacter::MoveCharacter`. The vtable of the mover has 14 slots: the 11 of the base and `DestroyPath`, `CreateDebugPathRenderable`, `DestroyDebugPathRenderable`.
- **Constants (high).** 0.02 (re-plan ratio), 1.0 s (minimum path age), 6 (look-ahead), 0.01 (waypoint push and "same goal" tolerance), 1.0 (cell size), 1e-5 (link-match tolerance), 3.0e38 (initial best height distance), 15 (capacity of the sorted part of the open list), 4.0 and 2.0 (defaults of the two step limits before `FindPath` sets them), the four rows of the push table (0, 0, -0.01), (-0.01, 0, 0), (0, 0, 0.01), (0.01, 0, 0), indexed by the direction of the link that points back towards the start (the row is the opposite of that direction, i.e. the direction of travel).
- **Data (high).** `wld-navigation.md` (48 non-empty meshes, 335,264 links, plane members); no stored `CMNavigationMovingType` exists (`behavior_classes_verify.py`). `python -I tools/re/marker_movers_verify.py` does not cover this page; the counts of mesh worlds are those of `wld_nav_verify.py`.
- **Not checked.** Nothing was run. The puppet's movement step (T01-141) and the run-time order of goal-reached callbacks; the exact open-list bookkeeping for ties (section 3.4); the two points whose height difference is compared by the search and by the straight-line test (3.4, 3.5).

## Layout / rules

### 1. State and construction

The mover's state (**names are ours**; the DLL declares no table): the **path** (handle to a path object, empty at the start), the **debug path renderable** (handle), the **waypoint index** `i` (0), the **current goal waypoint** `W` (a point), the **cached destination** `D0` and the **cache time** `t0`. The base `SMoveCharacter` members have their constructor defaults (auto-off time and distance -1, stop-when-foe false), so the base's steps 4 and 5 never end this mover. `CanOperateWithoutTargetEntity` is true. The copy the brain makes (`SetCurrentMover`) starts with `mc_bFirstTimeExecuted` true; `FirstTimeExecuted` is the base's only (random value, start time, start position). The destructor destroys the path and the renderable.

The base `MoveCharacter` of this mover **never asks for navigation** (`IsNavigationAllowed` is false while the brain's mover reason is `MR_NAVIGATION`), so the result of the base is 0 unless the base ends the mover.

### 2. The mover

**2.1 `MoveCharacter`** (one call per move period; the brain calls it after 0.01 s the first time, `brain-movement.md` 4.3), in order:

1. The base result; a non-zero result is returned.
2. **No path** (the handle is empty): `CachePath`; **false -> return 3** (`CMR_NAVIGATION_FAILED`); then `RecheckWaypoints`; **false -> return 3**.
3. `RecheckWaypoints` again (so it runs twice in the call that made the path); **false -> return 3**.
4. When the path exists and **`i >= waypoint count - 1`: return 1** (`CMR_REPLACE_ME`).
5. `D` := the destination point (`GetDestinationPoint`: the foe's live position when seen on the last test, else the last seen position). `M` := the 3-D distance from `D0` to `D`; `H` := the horizontal (x, z) distance from the brain's own position (the movement record) to `D`. **When `0.02 x H <= M`** and `now - t0 > 1.0 s`: **`DestroyPath`**, return 0 (the next call plans again). (Both conditions are needed; the test is `<=`; a path younger than or exactly 1.0 s is never dropped.)
6. The puppet's **`UpdateGoalPoint(W)`**: when it answers non-zero (the puppet is already at `W`, section 5), call this mover's **`OnGoalPointReached`**. Return 0.

**2.2 `CachePath`:** `DestroyPath`; start `S` := the brain's own position (movement record); `D` := the destination point; `path` := the service's `FindPath(S, D, en_fStepDownHeight, en_fStepUpHeight)` (both of the puppet; section 3); `D0` := `D`; `t0` := now. **When the path exists and has at least 2 waypoints:** `i` := 0, `W` := waypoint 1, `CreateDebugPathRenderable`, return **true**; **otherwise return false** (the path object, if any, is kept until the next `DestroyPath`; no renderable). (Waypoint 0 is the start point `S`.)

**2.3 `RecheckWaypoints`** (the look-ahead): let `n` be the waypoint count, `P` the brain's own position, and the two step limits as in 2.2. For `j` = 0 to 5 in turn: `k` := `i + j`; stop when `k >= n`; test `CanMoveFromPointAToPointB(P, waypoint k, stepDown, stepUp)` (section 3.5); **stop at the first test that fails**; otherwise `last` := `k`. Then:

- **`last < i`** (nothing accepted, also when the loop could not start): return **false**;
- else `c` := `last` clamped to [0, n - 1]; **when `c != i`: `i` := `c`, `W` := waypoint `c`, and the puppet's `ForceRunTroughPoint(W)`** (a run-through goal that the puppet does not stop at); return **true**.

Note the order of effect: nothing is issued to the puppet when the index does not change.

**2.4 `OnGoalPointReached`** (called by the brain when the puppet reports that it reached its goal point, or by 2.1 step 6): `i` := `i + 1`. When the path exists and `i < n`: `RecheckWaypoints`; if it **fails**: the puppet's `StopMoving`, then `CachePath`; if that succeeded, `RecheckWaypoints` again, and **if that fails too: `StopMoving`** (the failure surfaces as `CMR_NAVIGATION_FAILED` at the next `MoveCharacter`, 2.1 step 2 or 3). When the path is gone or `i >= n`: `StopMoving`. (So a path that stops being walkable is replanned from the puppet's current position at once; only a plan that fails right away gives up.)

**What the puppet is told, step by step (medium-low).** After `CachePath` the first `RecheckWaypoints` usually moves the index from 0 to a waypoint `c >= 1` and issues `ForceRunTroughPoint`. When the puppet reaches it, `i := c + 1`; `RecheckWaypoints` then tests from waypoint `c + 1`; **if the farthest accepted one is `c + 1` itself no goal is issued** (the index did not change after the increment), and the puppet's goal stays the one just reached: the next `MoveCharacter` calls `UpdateGoalPoint(W)` with that old `W`, which the puppet answers "already there" (section 5), which calls `OnGoalPointReached` again and increments the index again; the chain ends when a `RecheckWaypoints` moves the index (a new goal) or fails (a replan). Whether this really happens, or whether the puppet's own movement step keeps the previous goal active so that the puppet walks on to the next waypoint meanwhile, depends on goal bookkeeping that was not traced: **open, T01-141 and T01-403**.

**2.5 `DestroyPath`:** destroys the path object when there is one and then the debug renderable (`DestroyDebugPathRenderable`). **`CreateDebugPathRenderable`** (called by `CachePath` on success) makes an editor-style path renderable owned by the brain and fills it from the path; it is a drawing, with no effect on behaviour (**medium-low**: whether it is drawn in a release session was not checked).

### 3. The path service (`CWorldInfoEntity::FindPath` and friends; medium-low)

**3.1 The three calls.**

| Call | Meaning |
|---|---|
| `CanUseNavigation()` | **true** when the world has a path-finding service and its mesh has **at least one node** (`0 < node count`); 53 meshes in the 59 worlds, 5 of them empty: **48 worlds can use navigation** |
| `FindPath(start, end, stepDown, stepUp)` | a path object with a list of waypoints (3.4); **an empty handle when there is no service**, an object with 0 waypoints when the search finds no route; with a service but a start or end point without a node the list is `[start, end]` (3.3) |
| `CanMoveFromPointAToPointB(a, b, stepDown, stepUp)` | the straight-line test (3.5); **false when there is no service** |

**3.2 The node under a point** (`x`, `y`, `z`): the mesh's grid is asked for the nodes that overlap the unit cell with corner (`floor(x / 1.0)`, `floor(z / 1.0)`) (the node rectangles have integer corners and sizes, so this is containment of the cell). **No node: none** (index -1). **One node: that node** (no height test). **Several (stacked levels): the node whose plane height at (x, z) is nearest to `y`**, the first of equal distance winning (initial best distance 3.0e38). The plane of a node: `height = f0 + f1 x x + f2 x z` with the node's three floats (`CNMNode.4`, `wld-navigation.md`) in that order and **world** coordinates (**medium**: code; data check on one world: the median difference between a link's stored height and the owner node's plane is 5 mm, the maximum 1.3 m). The node's **centre** is the corner plus half the size in x and z, and its height there is the plane value.

**3.3 Set-up and fallbacks** (`FindPath`): the step limits are stored (the search defaults 4.0 and 2.0 are overwritten by the call); start node and end node by 3.2. **When either is missing: the result is the two-waypoint list [start point, end point]** and no search runs. **When both exist the search runs** (3.4); when the two nodes are the same node the search succeeds at once and the list is `[start, end]` as well.

**3.4 The search (medium-low).** An A* over mesh nodes:

- **Entry costs.** The start entry has cost `g` = 0. Moving from a node to a linked neighbour adds the **sum of the absolute differences in x, in height and in z between the two node centres** (a per-axis, "Manhattan" distance in 3-D, not the Euclidean one) to the parent's `g`. The heuristic `h` of a node is **the square of the sum of the absolute per-axis differences between the node's centre (x, plane height, z) and the end point**. The entries are ordered by **`g + h`**, smallest first. (The squared heuristic is much larger than any single-step cost: the search is strongly goal-directed and its result is **not** the shortest path in general.)
- **Expansion.** The best entry is taken from the open list and its node is closed; **the search ends successfully when that node is the end node**; it ends in failure (no waypoints) when the open list is empty. For every link of the node (the link chain of the node): the neighbour is expanded only if the **height change** between the two link points that bound the crossing is **at most `stepUp` when upwards and at most `stepDown` when downwards** (which two points exactly: not determined, 3.6), and each neighbour node is considered **once per expansion** (a scratch mark, cleared when the expansion ends; several links to one neighbour count once); a neighbour already open with a **larger** `g` gets the new parent and `g` and is re-sorted; a closed neighbour with a larger `g` gets the new parent and `g` but is **not reopened**.
- **Open list.** The open list is a sorted part of at most **15** entries plus an overflow list; entries move between them when the sorted part is full or empty (this affects only the order among near-equal keys; **not specified further**, T01-403).
- **Waypoints** (the path object): the end point, then for every pair of consecutive nodes of the route (from the end node back to the start node) the **centre point of the link** between them (its stored point, `wld-navigation.md` `.10`) **pushed by 0.01 m in the direction of travel** (link direction 0 = +z, 1 = +x, 2 = -z, 3 = -x: the push is (0, 0, +0.01), (+0.01, 0, 0), (0, 0, -0.01), (-0.01, 0, 0) along the direction **of travel**, i.e. into the node nearer the end point), then the start point; the list is reversed. **Waypoint 0 is the start point, the last waypoint is the end point, and between them lies one waypoint per node boundary crossed**: a route over `m` nodes has `m - 1 + 2` waypoints.

**3.5 The straight-line test** (`CanMoveFromPointAToPointB(a, b, down, up)`, **medium-low**): the nodes under `a` and under `b` are found (3.2); **true when either is missing or they are the same node**. Otherwise the segment from `a` to `b` is walked node by node: the exit point of the segment from the current node's rectangle (the box is unlimited in height) is found; the walk **fails** when the exit lies beyond the segment length plus 0.1 or before the previous exit; the neighbour is the node reached by the link of the current node whose centre segment contains the exit point (on the shared edge within 1e-5 and within half the link length along it); **no such link -> false**; the **height change across the crossing must not exceed `up` upwards or `down` downwards**; the walk **succeeds when the neighbour is the node under `b`** and fails when a node would be visited twice or a link leads nowhere. Note the consequence for `RecheckWaypoints`: a waypoint is accepted only when the straight line from the puppet to it stays on connected mesh nodes with acceptable steps.

**3.6 What is not determined.** Which two heights are compared for the step limits in the search and in 3.5; the exact tie rules of the open list; whether the plane heights or the link heights enter the compared change. (T01-403.)

### 4. Results seen by the brain (summary; `brain-movement.md` 4.2 and 4.3 are the full rules)

| Mover result | Brain action | Next call |
|---|---|---|
| 0 `CMR_OK` | nothing | the move period |
| 1 `CMR_REPLACE_ME` (last waypoint reached) | not a tactic reason: `DeleteCurrentMover`; the band's mover (or `CMCTowardsFoe`) is chosen at the next call | 0.01 s |
| 3 `CMR_NAVIGATION_FAILED` | `CMCDontMove` with reason `MR_NAVIGATION_FAILED` | 1.0 + 2u s (u uniform in [0, 1)) |
| (reason `MR_NAVIGATION_FAILED` current) | a new navigation mover is tried again at every call | the move period, 0.01 s after a new mover |

A pending tactic replaces the navigation mover (`brain-movement.md` 3.1).

### 5. The puppet's side of the goal points (summary; **medium-low**; T01-141)

`ForceRunTroughPoint(p)` installs `p` as the active goal as a run-through point (no stop, no test). `UpdateGoalPoint(p)` is `GoalPointCommonReset(p, 1)`: when `p` is already the active goal it changes nothing and returns 0; else when `p` is within **0.01 m** of the puppet's last reached goal point and the puppet is within **half of its current nominal speed in metres** of `p`, the puppet stops and the call returns **1** ("already there"); otherwise `p` is installed as the active goal (a goal point: the puppet stops there) and the call returns 0. This is the only way this mover learns that the puppet stands on `W` without a callback from the movement step.

## Invariants

1. The nav mover is never stored (0 objects in all files) and never returns 2 (`CMR_NEED_NAVIGATION`).
2. A path has either 0 waypoints (failure) or at least 2; waypoint 0 is the start point and the last is the end point.
3. `RecheckWaypoints` never looks more than 6 waypoints ahead; the index never decreases except by `CachePath` (reset to 0).
4. `MoveCharacter` returns 1 only when the index has reached the last waypoint (or the base ended the mover); 3 only from steps 2 and 3 of 2.1.
5. The re-plan test needs both conditions of 2.1 step 5; with a destination that does not move (`M` = 0) a path is never dropped (`0.02 x H <= 0` only when `H` = 0).

## Open questions

1. **The goal bookkeeping between `OnGoalPointReached`, `RecheckWaypoints` and the puppet's movement step** (2.4): whether a puppet that arrives at a waypoint and gets no new goal keeps walking: T01-141 and the run-time check **T01-158**.
2. **The remaining path-service details** (3.6: the compared heights, the open-list ties and overflow), and **the cost of the search on the largest meshes (4,000 nodes)**: **T01-403**.
3. **Whether the debug path renderable is drawn in a release session:** T01-404.
4. **Why the first recheck of a fresh path can leave the index at 0 with the goal `W` = waypoint 1** (the constructor of the state): harmless in all readings.

## Test vectors

- **Re-plan test** (2.1 step 5): the puppet at (0, 0, 0), the path made 1.5 s ago for a destination at (50, 0, 0); now the destination is at (50.5, 0, 0): `M` = 0.5, `H` = 50.5, `0.02 x H` = 1.01 > 0.5: no re-plan. At (52, 0, 0): `M` = 2.0 >= 1.04: re-plan (age 1.5 s > 1.0). Same move 0.9 s after the plan: no re-plan.
- **Look-ahead** (2.3): 8 waypoints, `i` = 2; the straight line is accepted to waypoints 2, 3, 4 and refused at 5 (waypoint 6 would have been accepted): `last` = 4, `c` = 4, `i` := 4, `W` := waypoint 4, run-through goal issued. With the first test already refused (waypoint 2): `last` = -1 < `i`: false. Accepted for 2 to 7 (all six): `last` = 7, `c` = 7 = `n - 1`: the next `MoveCharacter` returns 1 after step 3.
- **End of path** (2.1 step 4): 5 waypoints, `i` = 4: return 1; `i` = 3: continue.
- **Fallback** (3.3): the puppet stands on the mesh, the foe on a ledge without a node: the path is [puppet position, foe position]; `RecheckWaypoints` accepts both (a missing node means "yes"); `i` becomes 1 = `n - 1`; the next call returns 1 and the original mover walks on.
- **Waypoint construction** (3.4): route over nodes N1 -> N2 -> N3, N1 -> N2 through a link of direction 1 (+x) with its centre at (10, 5, 3) and N2 -> N3 through direction 0 (+z) with its centre at (14, 5, 8): waypoints [start, (10.01, 5, 3), (14, 5, 8.01), end].
- **Failure** (2.1 step 2): `CachePath` finds no route (both nodes exist, the search ends with an empty open list): return 3; the brain installs `CMCDontMove` with reason 5 for 1 to 3 s.
