# AI and navigation: what the data defines (T01-025)

- **Roadmap task:** T01-025
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the structure of the behavior data (`.cb`), the vocabulary of movers, actions and moments, and the counts (all 82 behaviors read; 6 worlds read for spawners, tactics, markers and navigation); **medium** for the roles of the numeric members that have readable names (alignment, ranges, band distances, weights); **low** for anything about *how the game evaluates* the data (think rate, perception tests, weight selection, steering, path search): none of that is in a data file, and it is listed as open questions with the measurements that would answer them.

Prerequisites: `misc-types.md` (`.cb`, `.sfc`, `.pss`, `.sca`/`.mas`; class tables in `misc-schema.md`), `classes.md`, `help-digest.md` (what the editor help says about patrolling and navigation), `wld-root.md` (entities), `ep.md` (puppet parameters).

## Summary

An enemy's decisions are **data-driven by one file per character, `.cb`** ("character behavior"): a few **states**, and in every state **distance bands** in front of the current **foe**; each band has a **movement** (how to move relative to the foe) and **action lists** keyed by **moments** (events). Actions are weighted alternatives. Behavior is thus a **table you can read**, not code: the numbers (ranges, bands, weights, shot counts) are all in the files, which makes a faithful reimplementation of the decision layer mostly a matter of **implementing the evaluator**, whose rules we must measure.

Around the brain sit: the **foe assignment** rules, **patrol and tactics** (path markers with movers), **spawners** (entities with limits, delays and effects), **formations**, and the **navigation mesh** with a path-finding service. Movement physics (speeds, acceleration) belongs to the puppet parameters (`ep.md`, T01-024/029).

## Evidence

`tools/re/ai_verify.py` (read-only) decodes all 82 `.cb` files and prints the counts below (`--table` prints the per-behavior table; `--worlds N` reads the N smallest worlds). The help file's pages on enemy behavior, patrolling and the navigation mesh are summarized in `help-digest.md`. Nothing here comes from binaries.

## The behavior file (`.cb`)

Root `CCharacterBehavior` (member list in `misc-schema.md`). Reading order:

1. **Identity and alignment.** `cb_idName` (the character name used by puppets; 76 names in 82 files: a few files are variants such as `…_pyramid`, `…_pyramid_kamikaze`) and `cb_baAligment` (`EBrainAligment`): **1 in 72 files** (all the hostile enemies), 0 in 5 (`NPC_Warrior`, `NPC_Gunner`, `Simba_Chief`, `Simba_Shaman`, `NPC_Civilian_Good`: friendly characters), 3 in 3 (the civilians `NPC_Civilian`, `NPC_Civilian_Static` and the semi-neutral amoeba) and 2 in 2 (`Ameba`, `Mask_Watcher`). This matches the editor's spawn limits "evil / good / misc" (chapter info, `help-digest.md`): **1 = evil, 0 = good, 2 and 3 = misc**; the names of the values are not in the data (inference).
2. **Ranges** (probably meters; the unit is not stated): `cb_fActiveRange` **250 in 67 files**, 2,500 in 13, 400 once (`Barbarian`), 25,000 once (`pyramid.cb`); `cb_fSenseRange` and `cb_fThreatRange` **10 in 64–66 files**, 50 (8), 20, 25, 30 or 5,000 for a few, 0 for the three amoeba files (no states at all). The active range is the distance from a player within which the character is simulated (compare `ActivatePhysicsInRange` in `physics.md`); the sense and threat ranges are the short distances at which an unaware character notices a player or counts as threatened; both readings are **inferred from the names**, not proven.
3. **Misc header.** `cb_fInitialReactionPeriod` −1 in all (disabled), optional graphs `cb_pgrThinkPeriodOveride` (10 files), `cb_pgrAltitudeMin` (15) and `…Max` (7) for flyers, `cb_pcbBasedOnCharacter` **null in all 82** (the inheritance mechanism exists but is unused), `cb_fadFoeAsigningDescriptor.fad_fAutoChangeFoeChance` (0 to 10).
4. **Behavior-wide moment handler** `gdc_ahActionHandler` (a `CAIMomentHandler`): action lists for moments that do not depend on distance (see below).
5. **States** `cb_acsCharacterStates`: 0 to 8 per file (54 files have exactly one, `Default`; 12 have two; the maximum is `NPC_Civilian_Static` with 8). 36 state names occur (`Default`, `Shooter`, `Agressive`, `RunAndGun`, `Frightened`, `ThrowItem`, …); a `SBAChangeState` action switches state (36 such actions).
6. **Bands.** Each state has three medium sets (`land`, `air`, `water`; each a list of `CCustomBehaviorArea`): the behavior depends on the medium the character is in. A band has a **name** (`Slash`, `Jump`, `Close`, `Middle`, `Far`, … 31 names), the **outer distance** (2 … 10⁸; the bands of one set are ordered by it, the nearest first), the **mover**, a graph of **action frequency over the position inside the band**, a **random multiplier of the action period** (0 to 0.5; 0.25 in most), a velocity adjuster (speed 0.5 to 2, other multipliers 1) and its own moment handler. Totals over all files: 381 bands (several per file; `Tank` has 21).
7. **Movers** (`SMoveCharacter` family) used by the 381 bands: `CMCTowardsFoe` 227, none 57 (a null mover: stand and act), `CMCDontMove` 26, `CMCCircleAroundFoe` 26, `CMCAwayFromFoe` 24, `CMCPatternMoving` 10, `CMCStayInArea` 7, `CMCDummy` 4. `CMCCircleAroundFoe` has an inner and outer range (8/16, 10/20, 30/50 …), a direction change time of 2 to 5 s (up to 16) and a start direction; the other movers' members are in the schema.
8. **Actions** (`CBehaviorAction` and subclasses), found in the lists of a moment: `CBANoAction` 192, `CBADefaultAction` 131, `SBAShoot` 106, `SBAAnimation` 94, `SBAMelee` 65, `CBAPlaySound` 47, `SBAList` 42 (a named group of actions), `SBAChangeState` 27, `SBARunOverFoe` 10, `CBAForcePlayerFoe` 6, `SBALocalMove` 5, `SBADie` 5, `SBALeapOntoFoe` 4, `CBACrouch` 4. Every action has a **weight** (0 to 100,000; 1 is the usual value; sounds use 10), an optional **validity term** (health, foe angle, player distance, tool held, time in state, combined with `CBAVTTermCombiner`; each can be negated) and three optional weight-multiplier graphs (over health, distance and area ratio; **all null in the shipped files**). A shot names a **shot definition** in the puppet parameters (`as_idShoot`: `Shoot`, `Laser`, `TankRocket`, `Chainballs`, … 38 names) with a minimum and maximum number of shots; a melee names an attack; an animation names an animation of the model's scheme (`mat.md`, `misc-types.md`).
9. **Moments** (the keys of the action lists, with the number of lists in all files): `Action` 384, `FoeAssigned` 53, `Damage` 51, `BeginState` 14, `Idle` 13, `EndState` 11, `AllFoesDied` 8, `Born` 6, `ToolPicked` 6, `StartRiding` 2, `StopRiding` 2. `Action` is the periodic decision inside a band; the others are events.

### Worked example: `Kleer` (kleer.cb)

Alignment 1, active range 250, sense and threat 10, initial reaction −1. Behavior-wide: `Action` → shoot `Chainballs` (1 shot, weight 1); `Damage` → play animation `Wound`; `FoeAssigned` → animation `Sight` and sound `Sight` (weights 1 and 10). **State `Default`**, land bands: `Slash` at 3 (no mover; melee `Slash`), `Jump` at 6 (mover `CMCTowardsFoe`; action `SBALeapOntoFoe`), `Close` at 25, `Middle` at 70 and `Far` at 500 (all `CMCTowardsFoe`, speed multiplier 1; actions `CBANoAction`, and at `Middle` also `CBADefaultAction`). **State `Shooter`**: `Slash` 3 (melee), `Middle` at 150 (mover `CMCDontMove`; `CBADefaultAction` weight 4 and `CBANoAction` weight 1; band period randomization 0.5) and `Far` at 500 (`CMCDontMove`). The author of a behavior therefore writes: "walk toward the foe; at 6 leap; at 3 slash; as a shooter, stand at 150 and shoot most of the time".

## Foe assignment

A character has a **foe** (the character it fights). `CFoeAssigning` (in the puppet parameters, 126 files) holds a table interpreter and a table of identifiers (empty in the sample); the behavior holds `fad_fAutoChangeFoeChance`. Actions can force a player foe (`CBAForcePlayerFoe` with a hostility 0, 6 or 9 and an auto-off time, −1 = never) or constrain it (`CBAPlayerAssigning`: allow a player or a non-player to be the foe). Global defaults (`rsc-ctl.md`): standard see distance 100 and a short-sighted distance 40. The rules for picking a foe (nearest, random, threat) are **not** in the data.

## Patrol and tactics

From the world data (6 smallest worlds, `ai_verify.py --worlds 6`): **path markers** (`SPathMarkerEntity`: 137 entities in 6 worlds, names like `PathMarker001`) carry two handles to other markers (`.2`: 98 set, 39 null; `.4`: 88 set, 49 null); the help file says a patrol path cannot branch, each marker has one target, circular paths are allowed and patrol markers throw a macro event when reached. **Tactics** (`STacticEntity`, 24) tie a **mover** (handle to a `SMoveCharacter`), a **marker** (the start of the path), an optional **next tactic** (9 of 24 set), an auto-off time (−1 = never) and distance (−1; one has 10) and a flag "stop the tactic when the character has a foe" (10 true, 14 false). The mover most used for patrols is **`CMCMarkersMovingOrdered`** (46 instances in these worlds): speed multiplier **1 (40), 0.35 (4) or 1.2 (2)**, look at foe (42 of 46), wait on a marker (`mm_fWaitOnMarkerMin/Max`: 0 to 15 s; 0 in most), a direction-change period (−1 = never in 44, 20 s in 2) and a behavior on the last marker (`BehaviorOnLastMarker` 1 in 42, 0 in 4). The patrol speed 0.35 matches the editor help (patrol speed multiplier default 0.35; 0.7 or less plays the walk animation). Other markers movers: `CMCMarkersMovingRandom`, `CMCMarkersMovingClosestToFoe`, `CMCPatternMoving`, `CMCDiamond`, `CMCMellon`, `CMCStayInArea`. **Formations** (`CMarkersFormation`, `CLineFormation`; formation marker entities in 41 world files) place groups of characters on markers.

## Spawners

`SSpawnerEntity` (in 46 world files; the properties record `SSpawnerProperties` has 14 members) with `SMultiSpawnerEntity` (15 files) and `CSpawnMarkerEntity` (16 files): a spawner holds a **spawnee template** (a puppet), counts and limits, delays and a **spawn effect configuration** (`.sfc`, `misc-types.md`). Over the 40 spawners of the 6 sampled worlds: initial total count 1 to 5 (1 in 13), initial group count 1 to 5, a **count-limit type** (`ESpawneeCountLimitType`: values 2, 3 and 4, with 4 and 3 the usual), two time members (values 0 to 6 and 0 to 60; the roles are unconfirmed), a "group spawner" flag (0 in all 40 here), sound disabled in 30 of 40, and optional formation and launcher handles. `rsc-ctl.md` lists the **spawner tweaks** (`CSpawnerTweak…`: change sense range to 200, change threat sensitivity, modify delays, patrol speed 2, no auto patrol, placement, random cone, reduce, increase multi, …) that adjust spawners by difficulty or by rule.

## Navigation and path finding

Each world has a **`CNavMesh`** (3 of the 4 meshes in the 6 sampled worlds have 559 to 791 nodes and 2,400 to 3,328 links, plus a navigation grid; the fourth, in the tiny `dockingbay`, is empty), a `CPathFindingService` (59 of 59 worlds, a pointer to the mesh) and renderables for debugging. The mesh is **baked in the editor** from the chapter markers and navigation markers by casting rays in the four directions and merging neighbours (`help-digest.md`); characters use it when a straight line is blocked, and use "feelers" to avoid dynamic objects. The data layout of nodes, links and the grid is the subject of T01-012e; the **search algorithm** (A* or other, costs, smoothing) is not in the data.

## What the data does not give (needs measurement, T01-038/039)

1. How often the brain thinks (the think period; `cb_pgrThinkPeriodOveride` overrides it for 10 files) and the period of the `Action` moment.
2. How an action is picked: the formula from weights, validity terms, frequency graph and randomization; whether `ba_fWeight` is normalized per list.
3. How "foe", "sense" and "threat" are tested: line of sight, distance, field of view, hearing (alarms, `help-digest.md`: Alarm entity), memory time.
4. Steering: acceleration, turning rates, feelers; how `SAreaVelocityAdjuster` multiplies puppet speeds.
5. Path following: replanning, how markers movers and the nav mesh interact, stuck handling.
6. Group behavior: formations, spawn limits by alignment, how `AllFoesDied` fires.
7. Difficulty scaling: where reaction times and damage change (T01-037).

A cheap way to approach 1 to 3 is an instrumented run (T01-039) that logs one behavior (`Kleer`, `Albino`: single-state melee; `SciFi_Grunt`: shooter with bands) over time and compares its decisions with the table above.

## Reader invariants

1. `.cb` files are format-8 CTSEMETA with the classes of `misc-schema.md`; resolve every pointer (states, bands, movers, actions, terms are separate objects) and dispatch on their class; an unknown class is an error.
2. Bands of one medium set are in increasing distance order in every file; the first band whose distance is above the foe's distance applies (inferred; verify with the instrumented run).
3. `cb_pcbBasedOnCharacter` is null in all shipped files; a reader may reject non-null until inheritance is specified.
4. Identifiers (`IDENT`) are indices into the file's identifier table: the state, band, animation, shot and sound names are strings there.

## Open questions

The list in "What the data does not give", plus: the unit of the ranges (meters assumed), the exact meaning of the `EBrainAligment` values, the meaning of `FoeHostility` 6 and 9, how `SBAList` groups are evaluated, and which behaviors the multiplayer bots use (none of the `.cb` is MP-specific).

## Test vectors

`tools/re/ai_verify.py` prints: 82 files; alignment 1 ×72, 0 ×5, 3 ×3, 2 ×2; active range 250 ×67, 2,500 ×13, 25,000 ×1, 400 ×1; states per behavior 1 ×54, 2 ×12, 3 ×5, 4 ×4, 0 ×3, 6 ×2, 5 ×1, 8 ×1; movers used in bands `CMCTowardsFoe` 227, none 57, `CMCDontMove` 26, `CMCCircleAroundFoe` 26, `CMCAwayFromFoe` 24, `CMCPatternMoving` 10, `CMCStayInArea` 7, `CMCDummy` 4; and the `Kleer` structure above.
