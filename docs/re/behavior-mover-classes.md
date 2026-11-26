# Movers, the navigation moving type and tactics of `Sam2Game.dll` (T01-105, part of T01-026)

- **Roadmap task:** T01-105 (overview [`behavior-classes.md`](behavior-classes.md); the other parts are [`behavior-action-classes.md`](behavior-action-classes.md) and [`behavior-entity-classes.md`](behavior-entity-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class and method names, the editor names of the movers, the data-side member names and values and every count (`tools/re/behavior_classes_verify.py`: all 13 saved mover classes, the base, the tactic, their member order against the binary's table where one exists); **medium** for the common mover protocol (section 3) and the rules marked `R`; **low** for what a mover's `MoveCharacter` does beyond its data members (marked `P` or `N`: profiled or from the name) and for the open questions. **Nothing was run.**

Prerequisites: [`behavior-classes.md`](behavior-classes.md), [`brain-classes.md`](brain-classes.md) (the brain's `MoveCharacter`, `SetCurrentMover`, `MoverReason`), `ai.md` (bands, patrol, tactics), `wld-navigation.md`, `ep-schema.md` and `misc-schema.md` (member lists of the same classes), `wld-entity-schema.md`.

## Summary

A **mover** says *how a character moves relative to its foe or to markers*. It is a small data object (`SMoveCharacter` and 13 subclasses) that sits in three places: in the distance **bands** of a `.cb` character behaviour, in a **tactic** entity of a world, and in a few `.ep` files. The brain copies a mover when it becomes the current one and calls its `MoveCharacter` once per move period; the result (`CharacterMoverResult`) tells the brain whether to keep it. This page covers the 16 classes of the roadmap line that are movers (`SMoveCharacter`, 13 `CMC*`, `CMNavigationMovingType`, `SDynamicMoveData`) and the **tactic** entity and properties (2).

| Class | Editor name (`GetMoveTypeName`) | Data members | Files (`.cb` / `.wld` / `.ep`) | Objects (`.cb` / `.ep` / `.wld`) |
|---|---|---|---|---|
| `SMoveCharacter` (base, data twin of the same name) | `<invalid>` | 11 | 120 (73 / 38 / 9) | in subclasses |
| `CMCTowardsFoe` | `Towards foe` | 0 | 82 (68 / 13 / 1) | 258 (227 / 1 / 30) |
| `CMCDontMove` | `Don't move` | 0 | 35 (14 / 19 / 2) | 60 (26 / 2 / 32) |
| `CMCCircleAroundFoe` | `Circle around foe` | 9 | 34 (24 / 9 / 1) | 60 (26 / 2 / 32) |
| `CMCAwayFromFoe` | `Away from foe` | 0 | 26 (23 / 3 / 0) | 29 (24 / 0 / 5) |
| `CMCPatternMoving` | `Patern moving` (sic) | 5 | 25 (9 / 16 / 0) | 38 (10 / 0 / 28) |
| `CMCStayInArea` | `Stays in area` | 2 | 13 (5 / 8 / 0) | 21 (7 / 0 / 14) |
| `CMCDummy` | `Dummy mover` | 0 | 3 (3 / 0 / 0) | 4 (4 / 0 / 0) |
| `CMCAirAttack` | `Air attack` | 11 | 9 (0 / 1 / 8) | 13 (0 / 12 / 1) |
| `CMCDiamond` | `Diamond` | 9 | 9 (0 / 9 / 0) | 19 (0 / 0 / 19) |
| `CMCMellon` | `Mellon` | 5 | 7 (0 / 7 / 0) | 14 (0 / 0 / 14) |
| `CMCMarkersMovingOrdered` | `Ordered moving using markers` | 11 | 38 (0 / 38 / 0) | 755 (0 / 0 / 755) |
| `CMCMarkersMovingClosestToFoe` | `Move on marker closest to foe` | 7 | 18 (0 / 18 / 0) | 36 (0 / 0 / 36) |
| `CMCMarkersMovingRandom` | `Move on random marker` | 5 | 11 (0 / 11 / 0) | 24 (0 / 0 / 24) |
| `CMNavigationMovingType` | `Navigation moving` | - | not in the data | - |
| `SDynamicMoveData` | - | - | not in the data | - |
| `CTacticEntity` (data `STacticEntity`) / `CTacticProperties` | - | 8 / 7 | 38 `.wld` | 993 / 0 |

Findings:

1. **Every mover is a subclass of the data class `SMoveCharacter`, whose own 11 members are the same for all** (speed multiplier, look at foe, move-period override, a random value, start time and position, auto-off time and distance, stop-tactic flag, first-time flag, brain link; section 2). Movers differ only in `MoveCharacter` and in a handful of hooks (section 3).
2. **The editor names are the movers' own labels**: `CMCMellon` is called "Mellon", `CMCDiamond` "Diamond", `CMCPatternMoving` "Patern moving" (the binary's own typo); the roles of the first two are known only from their data members (section 3.4).
3. **The mover is how a `.cb` band, a patrol and a tactic are the same thing, and the counts close exactly**: the 324 movers in the `.cb` files are the movers of the 324 bands that have one (57 of 381 bands carry none); the 990 movers stored in worlds are the movers of the 990 tactics that have one (`en_hmcCharacterMover` is null in 3 of 993), 755 of them `CMCMarkersMovingOrdered` (the patrol mover); the 17 movers in `.ep` files are the 17 set `sp_pmcCharacterMover` links of `CShootingProcess` (`ep-schema.md`, 106 null).
4. **`BehaviorOnLastMarker` is `Go back` 0, `Stop using tactic` 1, `Stay forever` 2, `Disappear` 3, `Die` 4**; the patrols of the shipped worlds mostly **end the tactic** (412 of 755) or **stay forever** (172) or **go back** (160); 9 die and 2 disappear (script). This settles the unnamed `BehaviorOnLastMarker` values of `ai.md`.
5. **Not all movers are present in the shipped `.cb`**: `CMCAirAttack`, `CMCDiamond`, `CMCMellon` and the three marker movers appear only in worlds or `.ep` files (the flying enemies and tactics), the navigation moving type nowhere: **it is made by the brain at run time** when a mover returns `CMR_NEED_NAVIGATION` (section 4).

## Evidence

- **Exported names.** 16 mover classes with 186 of the family's 955 functions (75 of them named methods) and the 2 tactic classes with 21 (7 named methods) (`behavior-classes.md` section 1).
- **Data.** `tools/re/behavior_classes_verify.py` (about 10 minutes) walks every object of the 82 `.cb`, 322 `.ep` and 59 `.wld` files: counts, files, and the stored values of every member (section 4 and 5 of its output); the member order of `STacticEntity` and `STacticProperties` against the binary's tables (the only two classes of this page with a DLL table) is identical in name and order (section 3 of its output: 2 of 2).
- **Virtual tables.** All 14 mover classes were compared with `SMoveCharacter`'s 11-slot table: the 11-slot movers differ from it in 4 to 6 slots (destructor, `mdGetDataType`, `GetMoveTypeName` and `MoveCharacter` always); `CMCMarkersMovingClosestToFoe` (17 slots), `...Ordered` (16), `...Random` (14) and `CMNavigationMovingType` (14) add 6, 5, 3 and 3 hooks of their own (section 4). **medium**.
- **Methods.** The 75 named methods of the movers (15 `MoveCharacter`, 15 `GetMoveTypeName`, 6 `FirstTimeExecuted` ...): the 4 simplest `MoveCharacter` (`TowardsFoe`, `AwayFromFoe`, `DontMove`, the base), the marker movers' `OnGoalPointReached`, `ReachMarker`, `ChangeMovingDirection`, `FirstTimeExecuted` and the base's `ValidateAndSetGoalPoint` were read (`R`); the other `MoveCharacter` bodies (circle, stay-in-area, pattern, diamond, mellon, air attack, navigation) were read only far enough to list their strings and the members they use (`P`).
- **Not checked.** Nothing was run. The geometry inside the steering functions (angles, offsets, graph use) was not extracted: it is the content of T01-154 and T01-155.

## Layout / rules

### 1. Classes and the common table

All mover classes have the C++ parent `SMoveCharacter` (all 14 constructors read; data twin `CMoveCharacter` is its data parent, 120 files, no members). Every class exports the standard seven functions; the named ones are listed in sections 3 and 4. The mover interface has **11 virtual slots**:

| Slot | Hook | Basis |
|---|---|---|
| 0 | destructor | - |
| 1 | `mdGetDataType` | - |
| 2 | `CanOperateWithoutTargetEntity` | R: a constant. The base answers one value and `CMCMarkersMovingOrdered`, `...ClosestToFoe`, `...Random` and `CMNavigationMovingType` the other: **a mover that can run without a foe is a patrol or navigation mover**. The base value is "no" (**medium**, inferred from which classes override) |
| 3 | `GetMoveTypeName` | R: the editor name of the table above |
| 4 | `GetMovePeriod(distance)` | R: when `mc_pgrMovingPeriodOverride` is set, the graph evaluated at the argument; otherwise the brain's default (`brain-classes.md` 3.2 `GetMovePeriod`) |
| 5 | `FirstTimeExecuted` | R: base draws a random number in [0,1) into `mc_fRnd`, stores the start time and start position (`mc_tmStarted`, `mc_vStarted`); overridden by `CircleAroundFoe`, the three marker movers and the navigation mover, all of which call or repeat the base |
| 6 | `OnGoalPointReached` | base: empty (a 3-byte function); overridden by the marker movers and the navigation mover |
| 7 | `OnWallHit` | base: constant; `CMCCircleAroundFoe` overrides (flips the direction, 3) |
| 8 | `MoveCharacter` | every concrete mover |
| 9 | `ValidateAndSetGoalPoint(x,y,z, flag, flag)` | R: the base routine every mover uses to give the puppet a goal point (section 3.2) |
| 10 | `UsesNavigation` | R: base: true when the puppet does not move in a 3-D area; `CMCAirAttack` and `CMCDontMove` override with a constant (**medium-low**: false, since flyers and standing characters need no path) |

(The slot order was read from the table of the most derived class; the decompiler shows shared stubs under other classes' names, so only methods the class itself exports are quoted.)

### 2. The base data class `SMoveCharacter` (11 members, 120 files; confidence per row)

The DLL declares no table for it; the data names are the file's own (`ep-schema.md`). Values over every stored object of the 14 classes (script):

| Member | Type | Meaning | Values in the data | Confidence |
|---|---|---|---|---|
| `mc_fSpeedMultiplier` | FLOAT | multiplier of the puppet's speed while this mover is current (the brain tells the puppet at `SetCurrentMover`, restores 1 at `DeleteCurrentMover`; `brain-classes.md` 3.4) | 1.0 in almost all; markers movers 0.1 .. 4 (1.0 ×593, 0.35 ×57, 0.25 ×21, 1.25 ×18, 0.75 ×15 of 755); 2.0 once (TowardsFoe) | medium |
| `mc_bLookAtFoe` | BOOL | the puppet faces the foe while moving (base `MoveCharacter` sets the look target) | 1 in 561 of 755 ordered-marker movers, 1 in most others | medium |
| `mc_pgrMovingPeriodOverride` | `CGraph*` | the move period override of slot 4 | null in every mover except 12 of 13 `CMCAirAttack` and 1 `CMCTowardsFoe` | high |
| `mc_fRnd` | FLOAT | random value drawn at the first execution | 0 in every stored object (it is a run-time member) | medium |
| `mc_tmStarted`, `mc_vStarted` | Time, Vector3f | start time and position (set at the first execution) | run-time members | medium |
| `mc_fAutoOffTime` | FLOAT | seconds after which the mover ends itself; **-1 = never** | **-1 in every stored object** (all 14 classes) | high (value), medium (meaning) |
| `mc_fAutoOffDistance` | FLOAT | **the mover ends when the foe is nearer than this distance** (corrected by T01-154, `behavior-mover-steering.md` 1.2: the base compares it with the brain's horizontal distance to the foe, not with the distance from the start); -1 = never | -1 in every stored mover (the tactic has its own, section 5) | medium |
| `mc_bStopTacticWhenHasFoe` | BOOL | end a tactic mover as soon as the brain has a foe | 0 in every stored mover (the tactic has its own flag, section 5) | medium |
| `mc_bFirstTimeExecuted` | BOOL | "not run yet" flag, cleared by the first `MoveCharacter` | 1 (stored) | medium |
| `en_henBrain` | handle | the brain that owns the copy | **null in all** (it is set when the brain copies the mover) | high |

### 3. The common protocol (**medium**)

**3.1 `SMoveCharacter::MoveCharacter`** (the base part every mover calls first; read in outline) returns a `CharacterMoverResult`: `CMR_OK` 0, `CMR_REPLACE_ME` 1, `CMR_NEED_NAVIGATION` 2, `CMR_NAVIGATION_FAILED` 3 (names and values from the DLL's table). On its first run it clears `mc_bFirstTimeExecuted` and calls `FirstTimeExecuted`. It then sets the puppet's look target to the foe (the puppet is told to face the foe unless `mc_bLookAtFoe` is false), tests whether **navigation is needed**: when navigation is allowed (`IsNavigationAllowed`) and the world info entity says the straight line from the puppet to the destination is blocked, the mover returns `CMR_NEED_NAVIGATION` and the brain installs the navigation mover (`MR_NAVIGATION`, `brain-classes.md` 4.2); and it ends the mover (`CMR_REPLACE_ME`) when the auto-off time or the auto-off distance is exceeded, or when `mc_bStopTacticWhenHasFoe` is set and the brain has a foe (the order and the exact tests of the returns were not read, T01-154).

**3.2 `ValidateAndSetGoalPoint(point, ..)`** (R; **fully specified by T01-154, `behavior-mover-steering.md` 1.4**: the limit flag is never set, the allowed-range clipping, the navigation request and the goal-point or run-through call; the following text is the first outline): shortens a goal point that is farther than a limit to the puppet's current nominal speed times the mover's speed multiplier, clips the point to the puppet's **allowed moving area** (a centre and radius the puppet carries; drawn by the debug flag `Puppet allowed moving area`), stops the puppet when the point is too close, and finally calls the puppet's **`SetGoalPoint`** (or **`SetRunTroughPoint`** when the flag says the puppet should pass through without stopping). Numeric limits were not extracted (T01-154).

**3.3 The simple movers** (`R`):

| Mover | `MoveCharacter` after the base |
|---|---|
| `CMCTowardsFoe` | goal point = the foe's position (`GetDestinationPoint`), its altitude adjusted for the puppet's medium (`AdjustGPAltitude`), then `ValidateAndSetGoalPoint` |
| `CMCAwayFromFoe` | goal point = own position plus the unit vector pointing away from the foe times a distance read from one member of the brain's entity (which member: not resolved), then `ValidateAndSetGoalPoint` |
| `CMCDontMove` | stops the puppet when it is moving (`StopMoving`); returns `CMR_OK` |
| `CMCDummy` | does nothing (its `MoveCharacter` is the shared empty function); the 4 stored objects are in 3 `.cb` files |

**3.4 The steering movers** (`P`: strings and members only; **all formulas and constants are in [`behavior-mover-steering.md`](behavior-mover-steering.md), T01-154, which also corrects two statements below: the circle and stay-in-area ranges are read only for tactic movers, and a band mover uses the band's start and end**): `CMCCircleAroundFoe` circles between an **inner range** (`mc_fInnerRange` 4 .. 50, 8 in 35 of 60) and an **outer range** (`mc_fOuterRange` 10 .. 100, 16 in 34 of 60): the debug overlay names the radii `inner core`, `outer core` and `desired range`; every `en_fChangeDirectionTimeMin..Max` (0 .. 10 and 0 .. 20; 0 in 32 of 60 = never) seconds it flips `mc_bCircleRight` (a random point of the interval is drawn), and `OnWallHit` flips it too (`mc_tmLastWallHitTime`); `en_icdInitialDirection` is `InitialCirclingDirection` (`ICD_RANDOM` 0, `ICD_LEFT` 1, `ICD_RIGHT` 2, `ICD_NOT_ALTERED` 3; stored 0 in 59, 2 in 1). `CMCStayInArea` keeps the foe distance between its two ranges (`mc_fInnerRange` 8 .. 50, `mc_fOuterRange` 16 .. 70) with hysteresis; the overlay names `inner core`, `outer core`, `desired range`, `inner hysteresis`, `outer hysteresis`, and it reads three numbers of the global game parameters (not extracted). `CMCPatternMoving` moves the goal point along a pattern over time: `pm_fPatternPeriod` (1 .. 5, 5 in 16 of 38) to `pm_fPatternPeriodMax` (3 .. 15) pick the period (interpolated by the puppet's random member), `pm_pgrHAngleMin`/`Max` give the heading offsets and `pm_pgrPatternMultiplier` scales it (graphs: Min set in 38 of 38, Max in 10, multiplier in 35). `CMCDiamond` keeps the character at a **diamond-shaped offset** from the foe; overlay names `Diamond: dispersion`, `Diamond: offset`, `Diamond: focus`; members `mc_amdAllovedMovingDirections` (`AllowedMovingDirections`: `AMD_LEFT` 0, `AMD_RIGHT` 1, `AMD_LEFT_AND_RIGHT` 2; stored 2 in all 19), `mc_fMinOffset` (0 .. 25), `mc_fMaxOffset` (4 .. 60), `mc_fRandomOffset` (0 .. 10), `mc_fRandomizeOffsetPeriod` (-1 in 13 of 19), `mc_fDispersionDistance` (5 .. 60), `mc_fFocusingDistance` (5 .. 75), run-time `mc_fOffsetRnd`, `mc_tmLastOffsetRandomization`. `CMCMellon` has `mc_amdAllovedMovingDirections` (stored 2 in 11 of 14), `mc_fMinAngle` (0 .. 20), `mc_fMaxAngle` (20 .. 90), `mc_bFadeOutOnTimer`, `mc_bFadeOutOnDistance` (1 in 11 of 14): a sweep within an angle band that can fade out with time or distance. `CMCAirAttack` is the **flying attack**: altitude over a graph (`mc_pgrAltitude`, set in 12 of 13), an optional maximum altitude graph, an altitude-correction factor `mc_fAltitudeCorrectingFactor` (0.5 in 11 of 13, 0.8, 0.9), a **freeze distance** `mc_fFreezeDirectionDistance` (20 in 10 of 13, 50, 75) inside which the target point and the front direction are frozen (`mc_bTargetFrozen`, `mc_vFrozenFoe`, `mc_bFrontFrozen`, `mc_vFrozenFront`, `mc_bWeWereMostlyFront`), and a fly-away limit `mc_fMaxFlyAwayDistanceDistance` (-1 in 11 of 13, 250 in 2). These descriptions are **medium-low**: names plus strings; no geometry was read (T01-154).

### 4. Marker movers and navigation

**Answered in full by T01-155: [`behavior-marker-movers.md`](behavior-marker-movers.md) (the three marker movers) and [`behavior-navigation-mover.md`](behavior-navigation-mover.md) (the navigation moving type and the path service); where the text below disagrees, those pages win (corrections: the patrol mover's events are `TacticMarkerReached` / `LastTacticMarkerReached`; the ending values of `BehaviorOnLastMarker` act at once and do not wait; the random mover picks any marker of the chain; the path length is a sum of straight segments minus 1.0).**

**4.1 `CMCMarkersMovingOrdered`** (patrol; `R` for the functions named): members `mm_imdInitMarkersDir` (`InitialMarkersDirection`: `IMD_FORWARD` 0, `IMD_BACKWARD` 1, `IMD_RANDOM` 2; stored 0 in 746, 2 in 6, 1 in 3), `mm_fChangeDirectionPeriod` (-1 = never in 745; 20 s in 5, 10 s 2, 15 s 2, 6 s 1), `mm_fWaitOnMarkerMin`/`Max` (0 in 677/651 of 755; up to 100 and 150 s), `mm_bolmBehaviorOnLastMarker` (finding 4) and the run-time `mm_bMovingTowardNextMarker`, `mm_tmLastDirectionChange`, `mm_tmMarkerReached`, `mm_bMarkerReached`, `mm_bLastMarkerReached`, `mm_fWaitOnMarker`. Behaviour: `FirstTimeExecuted` sets `mm_bMovingTowardNextMarker` from `mm_imdInitMarkersDir` (`IMD_FORWARD` -> toward the next marker, `IMD_BACKWARD` -> the other way, `IMD_RANDOM` -> a coin flip) and then reaches the puppet's current marker (`ReachMarker`). `OnGoalPointReached` (`R`) records the arrival time, marks the marker reached, draws the wait time uniformly between `mm_fWaitOnMarkerMin` and `Max`, copies the reached marker into the puppet's last-marker member, asks whether the marker is the last of the chain in the current direction (`IsMarkerLastInChain`: no further marker) and **sends the macro event** `TacticMarkerReached` (or `LastTacticMarkerReached` for the last marker in the current direction) to the puppet (`macSendEvent`; T01-155: these are the mover's own events, not the brain's `MarkerReached` and `LastPatrolMarkerReached`). When the last marker was reached `MoveCharacter` (`R`) acts on `BehaviorOnLastMarker`: **`BOLM_DISAPPEAR`** (3) calls the puppet's `DoDisappear` and **`BOLM_DIE`** (4) applies lethal damage to the puppet; (corrected by T01-155: the three ending values act at the next move step, the wait is not awaited; only go-back and stay-forever wait): **`BOLM_GO_BACK`** (0) reverses the direction (`ChangeMovingDirection` flips `mm_bMovingTowardNextMarker` and re-aims, `R`), **`BOLM_DESTROY_MOVER`** (1, editor label "Stop using tactic") ends the mover (the brain then chooses a mover again; inferred) and **`BOLM_STAY_FOREVER`** (2) never leaves (name only). Independently, a positive `mm_fChangeDirectionPeriod` makes `MoveCharacter` reverse the direction whenever that many seconds have passed since the last change (`R`). While waiting on a marker the puppet is stopped (`StopMoving`); when the wait is over it continues to the next marker (`ContinueMarkersReaching`).

**4.2 `CMCMarkersMovingRandom`** (24 objects, 11 worlds): `mmr_fWaitOnMarkerMin` (0 .. 2) and `Max` (0 .. 5), run-time `mmr_fWaitOnMarker`, `mmr_tmMarkerReached`, `mmr_bMarkerReached`: after a marker it waits, then picks **another marker at random** among the markers of the chain (`ContinueMarkersReaching`; T01-155: the single path of the current marker, the next marker twice as likely as any other, `behavior-marker-movers.md` 3).

**4.3 `CMCMarkersMovingClosestToFoe`** (36 objects, 18 worlds): `mmctf_bTestLOS` (1 in 28), `mmctf_bMustFollowPath` (1 in 24), `mmctf_fBestMarkerCheckPeriod` (2 s in 27 of 36, 1 .. 10), run-time `mmctf_henCurrentBestMarker`, `mmctf_henLastReachedMarker`, `mmctf_bMovingForwardAlongPath`: every check period it finds the **path marker closest to the foe** (`GetPathMarkerClosestToPoint`, `CacheBestMarker`), measures the path lengths forward and backward (`GetPathLength`: the strings say two markers must be joined by "a straight line or a circle"; warning `GetPathLength(): Markers %1 and %2 are not properly connected (neither a straight line, nor a circle).`) and sends the character along the shorter one (`SendCharacterAlongBestPath`; error text `Invalid situation occured in <closest to foe> character mover: both forward and backward paths are invalid`). **Path markers are connected by straight lines or circles** (`wld-navigation.md`). `P`/`R`.

**4.4 `CMNavigationMovingType`** (no member declared, not in the data; **specified in full by T01-155 in `behavior-navigation-mover.md`**; the following is the first outline): 10 named methods: `CachePath`, `RecheckWaypoints`, `DestroyPath`, the debug renderable pair, `MoveCharacter`, `OnGoalPointReached`, `FirstTimeExecuted` (a one-line call of the base), `GetMoveTypeName`, `CanOperateWithoutTargetEntity`. The brain installs it (reason `MR_NAVIGATION` 4) when a mover returned `CMR_NEED_NAVIGATION`; it asks the world info entity for a path to the goal (`CachePath`), walks way points (`OnGoalPointReached` advances), rechecks the next way points (`RecheckWaypoints`) and gives up with `CMR_NAVIGATION_FAILED` (brain reason `MR_NAVIGATION_FAILED` 5). The path search is the world info entity's path service (`behavior-navigation-mover.md` 3).

**4.5 `SDynamicMoveData`** (6 symbols, no destructor): a class with **only a virtual table** (no member, no member table, no caller found: empty and unused, T01-154, `behavior-mover-steering.md` 9); the brain's movement record `en_mcd` is the different run-time class `CMoveCharacterData`.

### 5. Tactics

`STacticEntity` (data name of `CTacticEntity`; 8 members; 993 objects in 38 worlds) and `STacticProperties` (7 members; its type table is in the same 38 worlds but **no object of that class is stored**; the world stores the entity). Member order is identical in the data and in the binary's declared tables (script section 3). A tactic is a **placed object** that ties a mover to a path marker for a patrolling puppet:

| Member (`STacticEntity`) | Meaning | Values in the 993 stored tactics |
|---|---|---|
| `en_hre`, `en_hreChild` | handles to the two editor renderables (the arrow between tactic and marker; the binary's table flags them "owned or temporary") | a reference is stored in all 993 |
| `en_hmcCharacterMover` | the mover to run (an embedded `SMoveCharacter` subclass; "owned") | set in 990, null in 3 |
| `en_fAutoOffTime` | the tactic ends after this many seconds; **-1 = never** | -1 in 936; 10 (14), 20 (7), 30 (5), 8 (4) ... up to 45 |
| `en_fAutoOffDistance` | the tactic ends when the foe comes **nearer** than this distance (T01-154; was read as the distance from the start); -1 = never | -1 in 485; **10 in 238**, 20 in 88, 5 in 37, 15 in 32 ... (25 values, -15 .. 100) |
| `en_bStopTacticWhenHasFoe` | the tactic ends as soon as the puppet has a foe | 1 in 73, 0 in 920 |
| `en_henNextTactic` | the tactic that follows when this one ends | set in 113 |
| `en_henTacticMarker` | the path marker where the tactic starts | set in 306, null in 687 |

(The ordered-marker mover of a null-marker tactic gets its marker from the puppet or the spawner; **not resolved**.) `CTacticEntity` adds `GetAbsPlacement`/`SetAbsPlacement` (**the placement of a tactic is its marker's**; a missing marker prints `Tactic entity returns invalid placement`), `GetBoundingBox`, `OnCreate` (builds the two editor renderables: a primitive and its child, the arrow) and `OnUpdate` (keeps them in step; `P`) and `OnDelete`: **all editor presentation**. At run time the brain reads a tactic's mover and flags through the puppet's current-tactic member (`MoveCharacter` selection, `brain-classes.md` 3.4; actions `CBAStopUsingTactic`, `CBAReuseTactic`, `CBAStopTactic`, `behavior-action-classes.md`). Tactics are `NEB_NONE`: not replicated.

### 6. Net behaviour and `.ep`

None of the 16 mover classes and the tactic overrides a net hook; movers are copied into the brain on the host only. The DLL declares no replicated member for them. In `.ep` files a mover occurs 17 times in 9 files (`CMCAirAttack` 12 in 8 files, `CMCCircleAroundFoe` 2, `CMCDontMove` 2, `CMCTowardsFoe` 1): each is the mover that a shooting process points at (`ep-schema.md`: `CShootingProcess.sp_pmcCharacterMover`, 17 set and 106 null: the same number).

## Invariants

1. Every stored mover class is a subclass of `SMoveCharacter` and its stored base members are equal to the 11 of section 2 (13 of 13 saved classes; the script prints the member lists).
2. `mc_fAutoOffTime` and `mc_fAutoOffDistance` are -1 and `mc_bStopTacticWhenHasFoe` is 0 in **every** stored mover object (all 14 classes); only the tactic carries non-trivial auto-off values (script).
3. `mc_pgrMovingPeriodOverride` is set in 13 of the 1,331 stored mover objects (12 `CMCAirAttack`, 1 `CMCTowardsFoe`; script).
4. The binary's tables for `STacticEntity` (8) and `STacticProperties` (7) equal the data's in names and order (2 of 2).
5. `BehaviorOnLastMarker` values in the data are 0 .. 4 only (755 objects).

## Open questions

1. **The steering geometry** (**answered by T01-154**, [`behavior-mover-steering.md`](behavior-mover-steering.md)) of the circle, stay-in-area, pattern, diamond, mellon and air-attack movers;  the original wording:   the numbers they read from the global game parameters, the return conditions of the base `MoveCharacter`, the goal-point limits of `ValidateAndSetGoalPoint`, `GetDestinationPoint` and `AdjustGPAltitude` (**T01-154**).
2. **Markers movers and the navigation moving type in full**: **answered by T01-155** ([`behavior-marker-movers.md`](behavior-marker-movers.md), [`behavior-navigation-mover.md`](behavior-navigation-mover.md)): path length (a sum of straight segments minus 1.0, no circle), the wait and direction rules, `CachePath`/`RecheckWaypoints`, failure and the fallback.
3. **Which marker a tactic without a marker uses**: **answered by T01-155** (the puppet's auto-patrol marker id, else the closest path marker by 3-D distance without a limit; `behavior-marker-movers.md` 5.2), confirmed by **T01-159** ([`behavior-leftovers.md`](behavior-leftovers.md)) to hold in both installed mods and for 65% of `CMCMarkersMovingOrdered` tactics, not an edge case; the original question: how the puppet gets its first tactic (spawner, `.cb`, macro) and when `en_hmcCharacterMover` is null (**T01-127** for the brain side).
4. **The meaning of `CMCMellon`** beyond its name (a measurement of an enemy that uses it, T01-158).
5. **`SDynamicMoveData`**: answered by T01-154 (an empty class).

## Test vectors

- **Editor names**: `GetMoveTypeName` of `CMCTowardsFoe` is `Towards foe`; of `CMCDontMove` `Don't move`; of `CMCPatternMoving` `Patern moving`; of `SMoveCharacter` `<invalid>`.
- **`CharacterMoverResult`**: `CMR_OK` 0, `CMR_REPLACE_ME` 1, `CMR_NEED_NAVIGATION` 2, `CMR_NAVIGATION_FAILED` 3.
- **`BehaviorOnLastMarker`**: 0 Go back, 1 Stop using tactic, 2 Stay forever, 3 Disappear, 4 Die. A patrol with value 4 ends with the puppet dying at the last marker (9 patrols in the data).
- **Ordered patrol**: `mm_imdInitMarkersDir` 0, `mm_fWaitOnMarkerMin` 0, `Max` 2, one marker pair: the puppet walks to the next marker, on arrival sends the marker's macro event, waits a time drawn uniformly in [0, 2] seconds, continues.
- **Away from foe**: puppet at (0,0,0), foe at (10,0,0): the goal point lies on the negative X axis (horizontal direction away from the foe).
- `tools/re/behavior_classes_verify.py` prints, for the movers (sections 1, 4, 5): the 14 classes with 120 files for the base (73 `.cb`, 38 `.wld`, 9 `.ep`), `CMCMarkersMovingOrdered` 755 objects in 38 worlds, `STacticEntity` 993 objects in 38 worlds, `bolm` values {1: 412, 2: 172, 0: 160, 4: 9, 3: 2}.
