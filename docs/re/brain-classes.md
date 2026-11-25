# Brain classes of `Sam2Game.dll` (T01-104, part of T01-026)

- **Roadmap task:** T01-104 (part 3 of the class inventory T01-026; the other parts are T01-102, T01-103 and T01-105 to T01-113)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** only for the counts and relations that `tools/re/brain_classes_verify.py` reproduces from the data (none of the 61 classes is saved in any file; which data classes mention a brain; which puppet kinds are in the data; which AI moment names the 82 `.cb` files use) and for the two enumerations (`BrainState`, `MoverReason`) and the class and method names, which are read from the export table and a data table of the DLL; **medium** for everything else read from the binary (the inheritance, the member meanings, the rules of section 4, the hooks per state), because each rests on one reading of a handful of functions and **nothing was run**; **low** for what a method does when it is described only from its name (marked "(name)") and for the items under "Open questions".

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `ai.md` (the `.cb` behavior files, moments, movers, foe assignment, patrol and tactics), `misc-types.md` and `misc-schema.md` (`.cb` class tables), `replication.md` (net behaviour, replicated members, "AI decisions exist only on the host"), `players-session-classes.md` (the player brain `CPlayerBrainEntity`, which this page does not repeat), `ep.md` and `ep-schema.md`, `save-game.md`.

## Summary

A **brain** is the entity that decides what a puppet does: it owns the "think" loop, the state of the decision (`BS_` states), the current movement type, the current foe and the link to the foe manager, and it turns the character behavior of the `.cb` file (`ai.md`) into actions of the puppet. A puppet has one brain; a brain controls one puppet. The player's brain is `CPlayerBrainEntity` (`players-session-classes.md` section 4); every other brain is the subject of this page.

`Sam2Game.dll` exports **61 brain classes** with **572 exported symbols** (the `C` name is exported, the data would use the `S` twin, `players-session-classes.md` section 1):

| Group | Classes | Symbols | What they are |
|---|---|---|---|
| **The brain** | `CBrainEntity` | 134 | The only class with behaviour: **127 named methods** (section 3) and the 31 members of `SBrainEntity` (section 5). Every other brain entity inherits it. |
| **Live kind shells** | `CCharacterBrainEntity`, `CDummyFoeBrainEntity`, `CGenericLeggedBrainEntity` | 7, 8, 8 | Subclasses with **no code of their own**: the first adds nothing but its type identity; the other two override one question, `HasFoeManager`, to answer "no" (section 2.2). |
| **Obsolete kind shells** | 16 entity classes: `CAIBrainEntity`, `CAirBikeBrainEntity`, `CAircraftBrainEntity`, `CAutoTurretBrainEntity`, `CBuggyBrainEntity`, `CFlyingKleerBrainEntity`, `CHovercraftBrainEntity`, `CKleerBrainEntity`, `CManifestedSpawnerBrainEntity`, `CMilitaryCarBrainEntity`, `CNPCBrainEntity`, `CSciFiSoldierBrainEntity`, `CTurretBrainEntity`, `CWereBullBrainEntity`, `CWheeledVehicleBrainEntity`, `CZombieSoldierBrainEntity` | 8 each | Their only override is `OnCreate`, which **prints the console error "Class (...) is obsolete and should be removed from level ASAP."** and does nothing else (section 2.1). |
| **Properties** | 20 `C...BrainProperties` classes | 7 each | Empty shells (no member table, no method): the object that spawns a brain (`CreateBrain` builds one in memory; no level stores one). |
| **Parameters** | 21 `C...BrainParams` classes | 7 each | Empty `CResource` shells (no member table): **nothing in the data is typed with them** and no `.ep` file uses them. |

Four findings an implementer needs first:

1. **Nothing of this family is in the data.** None of the 61 classes, under its own name or as an `S` twin, is in any of the 8,063 type tables (script, section 6). A `.wld` never stores a brain: the handles that would point at one (`SPuppetEntity.en_henDefaultBrain` and the unnamed puppet member next to it, `SMoveCharacter.en_henBrain`) are **empty in every stored instance** (`wld-entity-schema.md`, `wld-navigation-schema.md`, `misc-schema.md`, `ep-schema.md`). Brains are made at run time.
2. **In practice every enemy has the plain `CBrainEntity`.** The puppet's `CreateBrain` builds a brain of a per-kind class only for a puppet that has **no character behavior** and whose class name has a partner class; a puppet with a `.cb` behavior (every enemy that has a `.cb`) and any puppet kind without a partner gets the **base class itself** (section 2.3, **medium**). The per-kind classes of the roadmap line ("one brain kind per puppet kind") are therefore almost all dead code: **16 of the 19 are marked obsolete by the binary itself**, and the three live ones add nothing.
3. **The whole AI is data-driven by the `.cb` file and runs in one class.** The brain is a state machine of 16 named states (`BS_`), a "think point" that fires the AI moment handlers, and a mover that is re-chosen every move period. The state names, the 6 mover reasons and the AI moments that the code raises are listed in section 4; the rules that select actions and movers are only outlined (open).
4. **AI exists only on the host** (`replication.md`): the brain class is not replicated (net behaviour inherited, none) except for one member, the link to the puppet.

## Evidence

- **Exported names.** The 11,610 exports of `Sam2Game.dll` were listed and the 61 classes of this family taken with their exported symbols (the roadmap survey of T01-026: 61 classes, 572 symbols; recounted here: 285 on the 20 entity classes, 140 on the 20 properties classes, 147 on the 21 parameter classes). Names are quoted as exported.
- **Inheritance.** For every class the **copy constructor** was read to see which parent constructor it calls first, and for the entity classes the **virtual function table** was compared slot by slot with that of `CBrainEntity` and of `CAIBrainEntity`. Two readings; **medium**, because identical small functions are shared by the linker (the decompiler then shows another class's name for a slot, so slot names are not used as evidence).
- **Member table.** The DLL's static tables of class members (227 read, see `players-session-classes.md`) contain one for `SBrainEntity` (31 members, names and flags) and none for any other class of this family. Byte offsets and sizes were read but are **not recorded** (`conventions.md`; T01-063).
- **Methods.** The 127 named methods of `CBrainEntity` were decompiled and the strings and the exported functions each refers to were listed; about 50 of them were read closely (`ThinkPoint`, `SetState`, `SetNewState`, `HandleAIMoment`, `StartCharacterAction`, `MoveCharacter`, `SetCurrentMover`, `OnCreatedByPuppet`, `OnFoeAssigned`, `OnBehaviorFinished`, the patrol, ride and generic-action methods, the constructors), the rest are one-liners that forward to a virtual call, or (the ride, goal-point and debug methods) were described from their names and the functions they call. The partner puppet function `CPuppetEntity::CreateBrain` was read the same way. Each rule is **medium** (one reading).
- **Enumerations.** The names and values of `BrainState` and `MoverReason` come from the DLL's enumeration tables (value, name pairs), found with a script that looks for the name strings and the words around them, and agree with how the code compares the values (for example the "is idle" test and the dead-state rule).
- **Data.** `tools/re/brain_classes_verify.py` (about 3 minutes) reads the type tables of all 8,063 format-8 files and the 82 `.cb` files and prints the counts of section 6; the numbers it quotes from other specs (null handles) are from their scripts.
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared.
- **Not checked.** Nothing was run. No function was read in full except those named above; the movers, behaviors, foe manager and goal-point entities the brain creates are T01-105; puppet code is T01-103. A sentence for a method that is described only from its name is marked "(name)".

## Layout / rules

### 1. The 61 classes, by role

"Parent" is the C++ parent read from the copy constructor (medium); "Saved as" is the data name (script: all "not saved").

| Class | Kind | Parent (C++) | Symbols | Notes |
|---|---|---|---|---|
| `CBrainEntity` | entity | `CBaseEntity` (data name `SBaseEntity` appears in the member table) | 134 (127 named) | section 3; 211 virtual slots (102 inherited from the base entity, 14 of those overridden, 109 new) |
| `CCharacterBrainEntity` | entity | `CBrainEntity` | 7 | no override; the only name that suggests "the brain of a character" |
| `CDummyFoeBrainEntity` | entity | `CBrainEntity` | 8 | overrides `HasFoeManager` (answers no) |
| `CGenericLeggedBrainEntity` | entity | `CBrainEntity` | 8 | overrides `HasFoeManager` (answers no) |
| 16 obsolete entity classes (list in the Summary) | entity | `CBrainEntity` (all 16 copy constructors call it directly) | 8 each | override `OnCreate` (the obsolete-class error) |
| 20 `...BrainProperties` | properties | `CBaseProperties` (all 20 copy constructors run the base chain and add nothing) | 7 each | no member table, no method |
| `CBrainParams` | params | `CEntityParams` | 7 | `CResource` shell |
| `CAIBrainParams` | params | `CBrainParams` | 7 | |
| `CAircraftBrainParams`, `CFlyingKleerBrainParams`, `CHovercraftBrainParams`, `CKleerBrainParams`, `CNPCBrainParams`, `CSciFiSoldierBrainParams`, `CTurretBrainParams`, `CWereBullBrainParams`, `CWheeledVehicleBrainParams` | params | `CAIBrainParams` | 7 each | |
| `CAirBikeBrainParams` | params | `CHovercraftBrainParams` | 7 | |
| `CBuggyBrainParams`, `CMilitaryCarBrainParams` | params | `CWheeledVehicleBrainParams` | 7 each | |
| `CAutoTurretBrainParams` | params | `CTurretBrainParams` | 7 | |
| `CNPCBomberBrainParams`, `CNPCCivilianBrainParams`, `CNPCKamikazeBrainParams`, `CNPCShooterBrainParams` | params | `CNPCBrainParams` | 7 each | four NPC flavours |
| `CGenericLeggedBrainParams`, `CManifestedSpawnerBrainParams` | params | `CEntityParams` directly | 7 each | siblings of `CBrainParams`, not children |

Counts: 20 entity + 20 properties + 21 parameter classes = 61. The 21 parameter classes are the base `CBrainParams`, 16 kinds and the 4 NPC flavours; the three kinds without parameters are `CCharacterBrain`, `CDummyFoeBrain` and `CZombieSoldierBrain` (there is no `CCharacterBrainParams`, `CDummyFoeBrainParams` or `CZombieSoldierBrainParams`). The parameter classes mirror the puppet kinds, not the entity hierarchy, which is flat (medium; the entity parent column is from the constructor, and the slots agree: a kind shell differs from `CBrainEntity` in 1 to 3 slots out of 211 and the 16 obsolete ones are identical to `CAIBrainEntity` except for the type identity).

**Standard symbols.** Every class exports the same 7: default and copy constructor, destructor, assignment, stream-read operator, one further special member, and `mdGetDataType`; the data type object `md_pdtDataType` is also exported (as in `players-session-classes.md` section 1). 7 x 61 = 427 of the 572 symbols; the other 145 are `CBrainEntity`'s 127 named methods, 16 `OnCreate`, and 2 `HasFoeManager`.

### 2. The kinds

#### 2.1 The 16 obsolete kinds (**medium**: the same small function is the body of all 16)

The 16 entity classes named in the Summary share one `OnCreate` (identical code): it asks for the object's class name and prints on the console `Class (<name>) is obsolete and should be removed from level ASAP.` (the binary's own text), then returns. The same function is also the `OnCreate` of 11 puppet kinds (`CAirBikePuppetEntity`, `CAutoTurretPuppetEntity`, `CBuggyPuppetEntity`, `CFlyingKleerPuppetEntity`, `CKleerPuppetEntity`, `CManifestedSpawnerPuppetEntity`, `CMilitaryCarPuppetEntity`, `CNPCPuppetEntity`, `CSciFiSoldierPuppetEntity`, `CWereBullPuppetEntity`, `CZombieSoldierPuppetEntity`), the two spawn effect classes, a foe manager and behavior class, and three item classes (`CBladesItemEntity`, `CCircularSawItemEntity`, `CSerious_BombItemEntity`). So the puppet kinds Kleer, flying Kleer, were-bull, zombie, sci-fi soldier, NPC, air bike, buggy, military car, auto turret and manifested spawner are, by this reading, **obsolete in this build together with their brain** (the puppet side is T01-103; the items and effects named are other parts). The kinds `Aircraft`, `Hovercraft`, `Turret` and `WheeledVehicle` have an obsolete **brain** but a puppet class that is **not** in that list; whether their puppets are live is T01-103 (the data has `SAircraftPuppetEntity` in 29 files, `SHovercraftPuppetEntity` 10, `STurretPuppetEntity` 21, no wheeled-vehicle entity: section 6).

Because none of the 16 holds data or code, an implementer **does not need them**; a level that asks for one would only get the error message above.

#### 2.2 The three live kind shells

`CCharacterBrainEntity` overrides nothing (its vtable differs from `CBrainEntity` only in the type identity), so a character brain **is** a `CBrainEntity` (**medium**). `CDummyFoeBrainEntity` and `CGenericLeggedBrainEntity` override one slot, `HasFoeManager`, with a function that returns **false** (the body is the shared "return 0" function). `HasFoeManager` decides whether the brain creates a **foe manager** when its puppet is attached (section 4.2): so these two kinds **never get a foe manager, never assign a foe and therefore never run the `.cb` decision loop**; they are brains for puppets that only need a placeholder brain (a dummy foe target, a generic legged puppet driven by something else; the use is **not established**, T01-125). The base class answers "yes" exactly when the puppet has a character behavior (section 4.2).

#### 2.3 Which class a puppet gets (`CPuppetEntity::CreateBrain`, **medium**)

When a puppet is created or booted and has no brain yet:

1. If the puppet **has a character behavior** (its `.cb`, which the brain calls `StartCharacterAction` on), the brain is the **plain `CBrainEntity`**: a brain properties object of the base class is built in memory, given the puppet's placement, and spawned.
2. If the puppet has **no** character behavior, the puppet's **class name** is taken, the text `PuppetEntity` in it is replaced by `BrainProperties`, and a class of that name is looked up (`CKleerPuppetEntity` -> `CKleerBrainProperties`, `CPlayerPuppetEntity` -> `CPlayerBrainProperties`, `CPuppetEntity` -> `CBrainProperties`). If it exists and derives from the brain properties class, **it is spawned** (and its entity is the brain). The pairing exists for 18 of the 19 kinds (all but `CCharacterBrain` and `CAIBrain`) and for the player.
3. If nothing was created, the plain `CBrainProperties` is built and spawned: the brain is the base class. This is what a `CLeggedPuppetEntity` and a `CRollingBallPuppetEntity` without a behavior get (their names have no partner class).
4. The puppet then stores the brain in both its brain and default-brain links, copies one of its own members (an initial sight or perception mode, **name unknown**) into another (the member that `GetSeeDistance` reads), and calls the brain's `OnCreatedByPuppet`.

Consequences: the player gets `CPlayerBrainEntity` (`players-session-classes.md`); **every puppet of the shipped data that has a `.cb` behavior gets `CBrainEntity`** (the puppet classes the data uses are the six of section 6); a puppet without a behavior and of a kind with an obsolete brain would print the obsolete-class error (either this does not happen in the shipped data or the rule is wrong; **not verified**, T01-125).

### 3. `CBrainEntity`: the 127 named methods

Grouped by role. "(name)" = only the name was used. Parameter lists are not recorded. All methods are `public`; 109 are virtual (the new slots).

**3.1 Life cycle, links and debug**

| Method | What it does |
|---|---|
| `OnCreate` | Sets the "last action" time to zero and enters the default state (`BS_DEFAULT`). |
| `OnBoot` | Clears the "foe assigned before" and "immediate think" flags; unless the brain is patrolling markers, calls `ReconsiderBehavior`. |
| `OnShutDown` | Sets the state to default without running hooks, cancels pending thinking and sets all execution abilities. |
| `OnDelete` | Deletes the foe manager entity and the current behavior entity, drops the current mover, and tells the world info entity to forget the brain (when the puppet was on its list). |
| `OnCreatedByPuppet` | When `HasFoeManager` is true, spawns a foe manager (`CFoeManagerProperties` with this brain as its brain) and stores it (a spawned entity that is not a foe manager is discarded); makes the puppet the brain's default puppet; for a puppet that answers a certain question (not resolved), adds the brain to the world info entity's list of brains (the member `en_eaBrainsWithoutFoe` of `CWorldInfoEntity`: the name says "brains without foe"); enters the `.cb` state that the puppet stores as its initial state (`SetNewState`); raises one more moment whose name was not resolved (the data's `Born`, 6 lists, is the likely one; section 4.3). |
| `OnConnectedToPuppet`, `OnDisconnectFromPuppet` | Empty hooks (5 bytes each) for subclasses (name). |
| `SwitchToDefaultPuppet` | Gives the controlled puppet back (tells it to release its operator) and makes the default puppet use this brain again. |
| `PostReceiveUpdate` | After a network update, writes this brain into the puppet it points at, so that the puppet's brain link follows the replicated link. |
| `GetDefaultPlacement`, `GetAbsPlacement`, `GetVelocity` | The brain has no body: they return the puppet's placement and velocity (a fixed default when there is no puppet). |
| `mdGetDataType` | Standard (returns the type descriptor). |
| `AddDebugEntites`, `AddDebugInfo`, `DebugAIMomentHandlingData` | Debug display: lists the entities the brain relates to; shows current state, mover, mover reason, area, content (land, air, water), periods and the action-selection record; shows why the last AI moment was handled, rejected or canceled. |

**3.2 The decision loop and the `.cb` state**

| Method | What it does |
|---|---|
| `ThinkPoint` | The periodic decision point. First replays a forced AI moment that was queued while moments could not be handled; then, if a different `.cb` state was requested, switches to it (`SetNewState`); then, **with a foe**, handles the current action set (`Action` at the start) with `HandleAIMoment`, **without a foe** handles one other moment (name not resolved; the data's `Idle` is the likely one), and when that returns false calls `ResetActionTime` (**medium-low**: the order is read from a short function whose moment names the decompiler did not resolve). |
| `HandleAIMoment` | Runs one AI moment against the character behavior: needs a puppet, the "AI moments may be handled" ability and a behavior; first offers the moment to a per-instance behavior of the puppet (recorded as handled by `Instance`), otherwise starts the moment through `StartCharacterAction`; when an action started, resets the action time and tells the puppet; writes debug text when enabled. Returns whether an action started. |
| `ForceHandleAIMoment` | Handles a moment now when allowed, otherwise queues it (one slot) for the next think point. |
| `StartCharacterAction` | Rolls a random number (kept for the action selection) and asks the `.cb` behavior to choose and start an action for the current situation (moment, area, distance, health, content); with no behavior it records the text "Unknown starting character behavior is set." as the reject reason and starts nothing. |
| `CacheAreaData` | Gathers the situation record used for the selection: current `.cb` state and moment, the puppet's health ratio, the distance to the foe, the medium (land, air, water), the random value and the puppet's active ranges. |
| `SetNewState`, `SetDesiredState` | Switch the `.cb` character state (`SetNewState`: raises the `EndState` moment for the old and `BeginState` for the new one, restarts the state timer) or only request one (`SetDesiredState`, applied at the next think point). |
| `SetExecutionAbilities` | Stores the ability flags; when the "generic action" ability is removed while it was set, cancels what the puppet was doing for it (section 4.4). |
| `SetState` | Changes the **brain state** (`BS_`), running the exit hook of the old state and the entry hook of the new one (section 4.1). |
| `IsIdle`, `IsAlive` | Idle = state default or default-character-moving; alive = the puppet says so (false without a puppet). |
| `BeIdle` | Enters `BS_IDLE` for a given time and tells the puppet to stand idle; the think routine that ends it is `StopIdle`. |
| `ResetActionTime` | Restarts the timer of the periodic action. |
| `ReconsiderBehavior`, `ReconsiderBehavior2` | The first clears the current behavior and schedules the second for the next instant; the second spawns a think-point oscillator behavior (T01-105) and goes to `BS_DEFAULT_CHARACTER_MOVING`, or to `BS_DEFAULT` when the puppet has no `.cb` behavior. |
| `DefaultCharacterMovingState` | The think routine of normal movement: makes sure a think-point oscillator behavior exists, then schedules the next move period (`GetMovePeriod`). |
| `SetBehavior` | Replaces the current behavior entity (the old one is told to finish). |
| `OnDamage`, `RouteOnDamage` | The puppet reports damage; the brain tells the foe manager (damage dealer), the current behavior and raises the `Damage` moment (only for positive damage). |
| `OnBehaviorFinished`, `OnBehaviorCanceled`, `OnActionFinished`, `OnActionFinishedInternal`, `OnGenericActionBehaviorFinished` | Completion hooks of behaviors and actions: after a patrol step sends the macro events `MarkerReached` or `LastPatrolMarkerReached`; after a ride entry sets the "entered ride" flag; otherwise re-enters the default state (`ReconsiderBehavior`). `OnBehaviorCanceled` and `OnActionFinishedInternal` are very small (name). |
| `OnShouldDie` | The puppet died: finishes the brain (drops the foe manager and the behavior, emits a threat sound at the puppet, and enters `BS_DEAD`). |
| `GetMovePeriod` | The time until the next movement update: the current mover's own period when it has one, else the behavior's, else the global graph of move periods evaluated for the foe distance; never below 0.00001 s; a fixed default without a foe (values of the graph: T01-126). |

**3.3 Foes and senses**

| Method | What it does |
|---|---|
| `HasFoeManager` | True when the puppet has a character behavior; the two live shells override it with false (section 2.2). |
| `SetFoeManager`, `AssignFoe`, `LoseFoe` | Replace the foe manager (the old one is deleted); ask it to pick a foe; ask it to give up the foe. |
| `HasFoe` | The foe manager has a foe that still exists. |
| `OnFoeAssigned` | A foe was given: leaves the patrol (or the "look at a sound" state), stores the brain's placement at that moment, tells the puppet its foe, marks the first assignment (sets "foe assigned before" and "immediate think") and raises the `FoeAssigned` moment. |
| `OnFoeLost` | Tells the puppet to stop what it was doing for the foe and clears the puppet's foe. |
| `OnITouchSomebody`, `OnTouchedByBrain` | Contact hooks: touching the foe raises the `FoeTouched` moment; being touched by another brain is forwarded to the foe manager. |
| `IBecomeFoe` | Another brain has made this one its foe (name; the body was not read). |
| `OnIKilledFoe` | Tells the foe manager the foe was killed (it then raises the celebrate calls below). |
| `CelebrateIKillSomeFoe`, `CelebrateIKillMyFoe`, `CelebrateFoeDied`, `CelebrateAllFoesDied` | Raise the moments `IKilledSomeFoe`, `IKilledMyFoe`, `MyFoeDied`, `AllFoesDied`. |
| `GetSeeDistance` | The sight distance in world units: the global standard see distance (**100**, `rsc-ctl.md`); a puppet that carries a special sight setting can replace it by its own value or by the global short-sighted distance (**40**) and in one setting uses a fixed larger constant (which setting and which constant: T01-126). |
| `GetFoeSeekingFOVDeg` | The field of view in degrees for seeking a foe: the puppet's own value when it is at least a small threshold, otherwise the global character field of view (**360**); a fixed value while examining a sound. |
| `ExamineSoundSource` | A sound was heard at a position: unless a puppet setting (not resolved) says to ignore it, tells the puppet to look toward the position and enters `BS_EXAMINE_THREAT_SOUND`. |
| `OnToolPicked` | Raises the moment `ToolPicked`. |
| `OnStartRiding`, `OnStopRiding`, `OnTurnedOn`, `OnTurnedOff` | Raise the moments `StartRiding`, `StopRiding`, `TurnedOn`, `TurnedOff` (queued when moments cannot be handled right now). |

**3.4 Movement**

| Method | What it does |
|---|---|
| `MoveCharacter` | One movement update, returns the next period: needs the foe manager and a behavior; applies a pending state change; fills the movement record (this brain, own position, foe handle, foe position, foe distance); keeps the current mover unless there is none, or its reason is "area" and the area changed, or the puppet has a **tactic** pending; then chooses a new one in this order: the pending tactic (its mover with the tactic's settings copied; error text "Tactic entity has invalid (NULL) mover!" when it has none), else the mover of the current distance band of the `.cb` state, else a newly built default "towards the foe" mover; the don't-move and navigation movers are also built here (the navigation fallback and the end of the function, which sends the macro event `TacticEnded`, were read less closely: **low**; mover reasons in section 4.2). |
| `SetCurrentMover`, `DeleteCurrentMover` | Take a copy of a mover as the current one with a reason and a start time (and tell the puppet the mover's speed multiplier) / delete it and restore the speed multiplier 1. |
| `ForceMovingType`, `UnforceMovingType` | Force a mover (reason `MR_FORCED`) for a given time / cancel it. |
| `OnWallHit` | Asks the current mover whether it accepts the hit (true without a mover). |
| `OnAreaChanged` | The foe crossed into another distance band: tells the current behavior. |

**3.5 Patrol, goal points and macros**

| Method | What it does |
|---|---|
| `StartPatrolingMarkers`, `StartMacroPatrolingMarkers` | Start a path-marker patrol (only without a foe): store the first marker and the mover data on the puppet and enter `BS_PATROLLING_MARKERS`; the macro variant first marks the patrol as started by a macro and begins a latent macro call. |
| `ContinuePatrolingMarkers` | Resume a patrol that has a marker. |
| `StopPatroling` | Leaves the patrol (clears the behavior, tells the puppet) and reconsiders. |
| `SetPatrolingSpeedMultiplier`, `SetPatrolingRotateSpeedMultiplier` | Set the puppet's patrol speed and turn-speed multipliers (patrol state only). |
| `SendPuppetTowardsCurrentMarker` | The think routine of the patrol state: turns the puppet toward the marker. |
| `OnBeginPatrollingMarkers`, `OnEndPatrollingMarkers` | Entry and exit hooks of `BS_PATROLLING_MARKERS`. |
| `OrderPuppetToGoToGoalPoint` | Spawns a go-to-point behavior entity (T01-105) for a goal point. |
| `RouteOnGoalPointReached`, `OnGoalPointReached` | Arrival: tells the mover and the behavior; in the "examine a sound" state ends it and reconsiders. |

**3.6 Generic behaviors, scripted actions and waiting**

| Method | What it does |
|---|---|
| `StartGenericBehavior` | Installs a generic action-list behavior (`CGBActionListBehaviorEntity`, T01-105) as the current behavior, or hands the new one to the running one. |
| `PrepareForGenericBehavior`, `GenericBehaviorNotStarted` | Enter `BS_PREPARED_FOR_GENERIC_BEHAVIOR` / reconsider when the behavior did not start. |
| `StartGenericAction`, `CommandGenericShooting`, `CommandGenericMelee` | Start a scripted action, a scripted shooting (asks the puppet to start it, begins a latent call, enters `BS_EXECUTE_GENERIC_ATTACK`) or a scripted melee. |
| `ExecutingGenericActionState`, `ExecutingGenericAttackState` | The think routines of the two generic states: keep re-arming the think point while the ability is set. |
| `OnBeginGenericAction`, `OnBeginGenericAttack`, `OnEndExecuteGenericAction`, `OnEndExecuteGenericAttack` | Entry and exit hooks of the two states (entry removes the "AI moments may be handled" ability, exit restores it and ends the latent call; `OnEndExecuteGenericAttack` is small, name). |
| `PuppetDoesSpawnSource`, `PuppetDoesSpawneeAppearAnim` | The puppet is playing a spawn animation: set the generic-action ability and enter `BS_EXECUTING_GENERIC_ACTION`. |
| `WaitAnimToEndFunc`, `WaitingAnimToEndState`, `WaitingAnimFinished`, `OnBeginWaitingAnimToEnd`, `OnEndWaitingAnimToEnd` | Wait for the current animation: begin a latent call and `BS_WAITING_ANIM_TO_END` for the animation length, end it and reconsider (the two hooks are name). |
| `DoDisable`, `DoEnable` | Disable: drop the behavior, set the abilities to none and enter `BS_DISABLED`; enable: return to the default state, set all abilities and reconsider. |
| `DisableAI`, `EnableAI` | Only for a brain with a foe manager: disable (or enable) the foe manager, then drop the behavior, set the abilities to none and enter the default state (or set all abilities and reconsider). |
| `KnockOut`, `OnBeginKnockOutState`, `OnEndKnockOutState`, `OnBeginDeadState`, `OnEndDeadState`, `OnBeginIdleState`, `OnEndIdleState` | Enter `BS_KNOCK_OUT`; entry and exit hooks of the knock-out, dead and idle states (the dead state sets the abilities to none on entry and to all on exit; the knock-out and idle hooks are small, name). |

**3.7 Rides** (the brain boards and leaves vehicles of `vehicles.md`)

| Method | What it does |
|---|---|
| `GetAvailableSeat` | Chooses a free seat among the ride's seats (name; it walks the ride's seat array). |
| `CantPosessRide` | Raises the moment `CantPossesRide` and a macro event. |
| `PossesRide`, `PossesCharacter` | Take control of a ride (a seat and the way to it are stored in the members of section 5) / of a character puppet. |
| `ShouldTeleportToEnterRide` | Whether to teleport instead of walking to the seat (name; the player brain overrides it). |
| `ExitFromRide`, `OnBeginExitingFromRide`, `OnEndExitingFromRide` | Leave the ride: spawn the exit behavior, enter `BS_EXITING_FROM_RIDE` and its hooks (the client side skips it for remote entities). |
| `OnBeginEnteringRide`, `OnEndEnteringRide`, `PossessingRideState` | Entering: wait for the riders to climb, then start the ride possession behavior (T01-105) and raise `StartRiding`; `PossessingRideState` is the think routine that spawns the possession behavior. |
| `WaitRiderToClimb`, `OnAllRidersClimbed` | `BS_WAITING_RIDERS_TO_CLIMB` and its end: all riders on board, go on. |

### 4. How the brain works (**medium**)

#### 4.1 The brain state (`BrainState`, the member `en_bsState`)

16 values, read from the DLL's table (high for names and values); every state except the default has an "on begin" and an "on end" hook that `SetState` calls, and the dead state cannot be left by `SetState` (leaving it does nothing: the state stays `BS_DEAD`; a revived brain goes through `DoEnable`, **medium**).

| Value | Name | Hooks, think routine or what enters it |
|---|---|---|
| -1 | `BS_INVALID` | while `SetState` runs the old state's exit hook |
| 0 | `BS_DEFAULT` | state after construction and `OnCreate`; "idle" for `IsIdle` |
| 1 | `BS_CHARACTER_NOT_MOVING` | (name) |
| 2 | `BS_DEFAULT_CHARACTER_MOVING` | think routine `DefaultCharacterMovingState`; also "idle" for `IsIdle` |
| 3 | `BS_ENTERING_RIDE` | `OnBeginEnteringRide` / `OnEndEnteringRide` |
| 4 | `BS_EXECUTING_GENERIC_ACTION` | `OnBeginGenericAction` / `OnEndExecuteGenericAction`; think routine `ExecutingGenericActionState` |
| 5 | `BS_PATROLLING_MARKERS` | `OnBeginPatrollingMarkers` / `OnEndPatrollingMarkers`; think routine `SendPuppetTowardsCurrentMarker` |
| 6 | `BS_WAITING_ANIM_TO_END` | `OnBeginWaitingAnimToEnd` / `OnEndWaitingAnimToEnd`; think routine `WaitingAnimToEndState` |
| 7 | `BS_DEAD` | `OnBeginDeadState` / `OnEndDeadState` |
| 8 | `BS_EXITING_FROM_RIDE` | `OnBeginExitingFromRide` / `OnEndExitingFromRide` |
| 9 | `BS_DISABLED` | `DoDisable` |
| 10 | `BS_EXAMINE_THREAT_SOUND` | `ExamineSoundSource`; left by `OnFoeAssigned` or `OnGoalPointReached` |
| 11 | `BS_KNOCK_OUT` | `KnockOut`; `OnBeginKnockOutState` / `OnEndKnockOutState` |
| 12 | `BS_EXECUTE_GENERIC_ATTACK` | `OnBeginGenericAttack` / `OnEndExecuteGenericAttack`; think routine `ExecutingGenericAttackState` |
| 13 | `BS_WAITING_RIDERS_TO_CLIMB` | `WaitRiderToClimb`, `OnAllRidersClimbed` |
| 14 | `BS_IDLE` | `BeIdle`; `OnBeginIdleState` / `OnEndIdleState` |
| 15 | `BS_PREPARED_FOR_GENERIC_BEHAVIOR` | `PrepareForGenericBehavior` |

Which think routine belongs to which state is read from the state change function and from the strings each routine refers to (**medium-low**: the decompiler resolves identical small functions to one name).

**The `.cb` character state** (`en_idState`, `en_idDesiredState`) is separate from the brain state: it is the state name of the character behavior (`ai.md`: `Default`, `Shooter`, ...), starts as the identifier `Default` (as in the data), and changes through `SetNewState`, which raises the moments `EndState` then `BeginState`.

#### 4.2 Mover choice and reasons (`MoverReason`, the member `en_mrMoverReason`; high for names and values)

| Value | Name | When |
|---|---|---|
| 0 | `MR_NONE` | no mover |
| 1 | `MR_AREA` | the mover of the current distance band of the `.cb` state (the area is stored so that a change of area re-chooses) |
| 2 | `MR_FORCED` | `ForceMovingType` |
| 3 | `MR_TACTIC` | a tactic's mover (patrol and tactics, `ai.md`) |
| 4 | `MR_NAVIGATION` | the navigation moving type (`CMNavigationMovingType`, T01-105) |
| 5 | `MR_NAVIGATION_FAILED` | the navigation attempt failed |

The default mover when a band has none (57 of the 381 bands in the data have a null mover, `ai.md`) is **towards the foe** (`CMCTowardsFoe`, medium).

#### 4.3 The AI moments the brain raises

A moment is an identifier that selects an action list of the `.cb` behavior. The brain raises: **`Action`** (the periodic one; the brain's initial action set), **`BeginState`**, **`EndState`**, **`FoeAssigned`**, **`Damage`**, **`AllFoesDied`**, **`ToolPicked`**, **`StartRiding`**, **`StopRiding`**, and seven that **no `.cb` file uses**: `IKilledSomeFoe`, `IKilledMyFoe`, `MyFoeDied`, `TurnedOn`, `TurnedOff`, `FoeTouched`, `CantPossesRide` (script, section 6: 9 of 16 names are used; the 82 files use 12 identifiers). The data also uses **`Idle`** (13 lists), **`Born`** (6) and the empty identifier (3) which the brain code does not name as a string (T01-126: another path or a computed name). Besides moments the brain sends **macro events** to the level script: `MarkerReached`, `LastPatrolMarkerReached`, `TacticEnded` and one for a failed ride (`CantPossesRide` is both).

#### 4.4 Execution abilities

The member `en_ulExecutionAbilities` is a set of flags, **all set** after construction. Two flags are used: **bit 0 "AI moments may be handled"** (without it `HandleAIMoment` does nothing and `ForceHandleAIMoment` queues) and **bit 2 "a generic action may run"** (the two generic states only act with it). Entering a generic action or attack clears bit 0, leaving restores it (`SetExecutionAbilities(all)`); `OnBeginDeadState` clears all, `OnEndDeadState` sets all. The binary that was read has no names for the flags (the bits are described here by use, **ours**).

#### 4.5 Net behaviour

`CBrainEntity` and its kinds do **not** override `GetNetBehavior`, `GetRelevantDistance` or `GetUpdatePriority`: they inherit **`NEB_NONE`**, default relevance and priority 2 (`replication.md`; the player brain is the only brain with overrides: predicted, unlimited, priority 10). The table of `SBrainEntity` has **1 replicated member of 31: `en_henPuppet`**; `en_hmcCharacterMover` is flagged owned and `en_asd` temporary (`replication.md` 3.4). All decisions are taken on the host; `PostReceiveUpdate` re-links the puppet to a received brain link. Whether a client creates brains for the puppets it receives, or runs `CreateBrain` at all, is **not read** (T01-125). `ExitFromRide` checks the remote flag, so some brain logic is skipped on clients (name-level, **low**).

### 5. Members (`SBrainEntity`, 31; confidence per row)

Flags as in `replication.md`: NETUPDATE (replicated), OWNED, TEMPORARY. Meanings are from the names and the methods that use them.

| Member | Meaning | Confidence |
|---|---|---|
| `en_henFoeManager` | the foe manager entity of this brain (empty for brains without one) | medium |
| `en_henPuppet` (NETUPDATE) | the puppet this brain controls | medium |
| `en_henDefaultPuppet` | the puppet the brain was made for (`SwitchToDefaultPuppet` returns control to it) | medium |
| `en_henBehavior` | the current behavior entity (think-point oscillator, generic action list, go-to-point, possess ride, ...) | medium |
| `en_mlidLatentID`, `en_bMacroStartedPatroling` | the latent macro call that waits for an animation, a patrol or a scripted action, and whether the patrol was started by a macro | medium |
| `en_fAnimToWaitLen` | the time to wait for an animation (seconds) | medium |
| `en_asd` (TEMPORARY) | the action-selection record: areas, distance ratios, health ratio, content, area id, moment id, selected action, random value | medium |
| `en_tmLastActionTime` | time of the last periodic action | medium |
| `en_bFoeAssignedBefore`, `en_bImmediateThinkPoint` | set by the first `OnFoeAssigned`, cleared by `OnBoot` | medium |
| `en_strBAHandledBy`, `en_strBARejectInfo` | debug strings: what handled the last moment (`Instance` or a file name), why it was rejected | medium |
| `en_bResetTimerAfterBehaviorAction` | read by `OnGenericActionBehaviorFinished` | low |
| `en_hmcCharacterMover` (OWNED) | the current mover | medium |
| `en_mrMoverReason`, `en_tmMoverStarted`, `en_idAreaForLastMover` | why and when the mover was set, and for which distance band | medium |
| `en_mcd` | the movement record passed to movers (this brain, position, foe, foe position, foe distance) | medium |
| `en_idCurrentActionSet` | the current action set (moment) id, `Action` after construction | medium |
| `en_qvFoeAssignPlacement` | the brain's placement when the foe was assigned | medium |
| `en_idState`, `en_idDesiredState`, `en_tmAIStateStarted` | the `.cb` character state, the requested one and when the state began | medium |
| `en_ulExecutionAbilities` | section 4.4 | medium |
| `en_bSuccessfullyEnteredRide`, `en_henRideToPossess`, `en_idSeatToPosess`, `en_iWayToSeatToPosess` | ride boarding data (names) | low |
| `en_bsState` | the brain state (section 4.1) | high |
| `en_idForceHandlingOfAIMoment` | the moment queued by `ForceHandleAIMoment`, replayed by `ThinkPoint` | medium |

After construction (**medium**): state `BS_DEFAULT`, `.cb` state `Default`, action set `Action`, desired state invalid, all execution abilities set, mover reason `MR_NONE`, "mover started" time -100 s, last action time 0.

**Relation to the puppet.** The puppet has a brain link, a default-brain link and (player only) an operator link (`SPuppetEntity.en_henOperator`, handle of a player brain); the brain has the puppet link and the default puppet. The mover (`SMoveCharacter`, data class) holds `en_henBrain` back. All of these are empty in the stored data.

**Relation to the data.** The brain reads `.cb` data that `ai.md` specifies (states, bands, movers, action lists, moments) through the puppet's behavior; nothing of this family has a `.ep` schema class, and no `.ep` or `.rsc` member names a brain parameter class. The `.cb` field `cb_baAligment` (`EBrainAligment`, 0 to 3) is read by the puppet, not by the brain (the member is on `SPuppetEntity.en_baAlignment`, `wld-entity-schema.md`).

### 6. What the script reproduces

`python -I tools/re/brain_classes_verify.py "<install>"` (about 3 minutes, data only, read-only) prints four blocks: (1) the 61 class names are absent from the 8,063 type tables (the names come from the export table; the script quotes them); (2) the three data classes with a member whose name contains `Brain` and the number of files that define each (`en_eaBrainsWithoutFoe` in `CWorldInfoEntity`, `en_henBrain` in `SMoveCharacter`, `en_henDefaultBrain` in `SPuppetEntity`); (3) which of the 21 exported puppet entity classes are in the data and in how many files; (4) the moment identifiers of the 82 `.cb` files against the moment names that the brain code raises. What it does **not** reproduce: everything read from the binary (the methods, the enumerations, the member table, the construction rule).

## Invariants

1. None of the 61 classes appears in any type table of the 8,063 format-8 files, under its own name or as an `S` twin (checked on all 61, script). A reader of the shipped data can ignore them.
2. All 20 entity classes of the family derive directly from `CBrainEntity` in their copy constructors (20 of 20 read; checked against the vtable for 6, **medium**); the parameter classes form the tree of section 1 (21 of 21 read).
3. The 16 obsolete entity classes share one `OnCreate` that only prints the obsolete-class error (16 of 16 read as one function, **medium**).
4. A brain with `BS_DEAD` stays dead through `SetState` (1 function, **medium**).
5. The brain's own members are initialised as listed after "Members" (the constructor, **medium**).
6. A reader must reject nothing here: no file format is defined by this family.

## Open questions

1. **Which brain class each shipped puppet kind really gets, and when `CreateBrain` runs** (T01-125): the rule in section 2.3 is read from one function; confirm on the running original that placed enemies run `CBrainEntity`, what a puppet without a `.cb` (rideable vehicles, `SRollingBallPuppetEntity`) gets, whether a client creates brains, and why `Aircraft`, `Hovercraft`, `Turret` and `WheeledVehicle` have an obsolete brain while their puppets are live; and how a puppet gets its behavior from its character name.
2. **The decision loop in full** (T01-126): the think period and its graph, the order inside `ThinkPoint`, the exact use of `GetMovePeriod`, the sight setting behind `GetSeeDistance`, the moments `Idle`, `Born` and the empty one, the meaning and names of the execution ability flags, the think routine of each state. Ties to T01-076 (enemy run time) and T01-038.
3. **Mover choice, patrol and tactics** (T01-127): the full `MoveCharacter` selection (band lookup, tactic copy, navigation fallback, speed multipliers), patrol start and resume rules, goal points and the macro events; with T01-105 for the mover and behavior entities themselves.
4. **Rides** (T01-128): seat choice, the way to a seat, teleport to enter, possession and exit; ties to `vehicles.md` and to the player's own ride rules (T01-114).
5. **Foe and sense rules** (T01-129): what `OnCreatedByPuppet` registers in the world info's list of brains, how the foe manager and the brain split the work (the foe manager is T01-105), the sound examination, the hearing thresholds.
6. **Run-time checks on the original** (T01-130): which brain classes exist in a running level, the state sequence of one enemy (`Kleer`, `Albino`), the moment order, the see distance and field-of-view numbers, whether the obsolete-class error ever appears; numbers only (T01-038, T01-039).

## Test vectors

- **Partner class by name** (section 2.3): a puppet of class `CKleerPuppetEntity` with no behavior looks for `CKleerBrainProperties` (exists; its entity prints the obsolete-class error); `CGenericLeggedPuppetEntity` looks for `CGenericLeggedBrainProperties` (exists; no foe manager); `CLeggedPuppetEntity` looks for `CLeggedBrainProperties` (does not exist) and gets the base `CBrainEntity`; any puppet **with** a behavior gets `CBrainEntity` without a lookup.
- **Brain state values** (section 4.1): `BS_DEFAULT` 0, `BS_ENTERING_RIDE` 3, `BS_PATROLLING_MARKERS` 5, `BS_DEAD` 7, `BS_KNOCK_OUT` 11, `BS_IDLE` 14, `BS_INVALID` -1; `IsIdle` is true exactly for 0 and 2.
- **Mover reason values**: `MR_NONE` 0, `MR_AREA` 1, `MR_FORCED` 2, `MR_TACTIC` 3, `MR_NAVIGATION` 4, `MR_NAVIGATION_FAILED` 5.
- **After construction**: brain state 0, `.cb` state `Default`, action set `Action`, execution abilities all set.
- **Dead stays dead**: `SetState(BS_DEAD)` followed by `SetState(BS_IDLE)` leaves `en_bsState` at 7.
- **Moments**: a brain that gets a foe raises `FoeAssigned`; one that is hit for 5 or more health raises `Damage`; one that is hit for 0 or less raises nothing (`RouteOnDamage` tests for positive damage).
- **Distances** (data, `rsc-ctl.md`): the standard see distance is 100 and the short-sighted distance 40; the character field of view is 360.
- `tools/re/brain_classes_verify.py` prints: 61 of 61 classes absent; classes with a "Brain" member: `CWorldInfoEntity` (59 files), `SMoveCharacter` (120), `SPuppetEntity` (45); puppet kinds in the data: `SPuppetEntity`, `SLeggedPuppetEntity`, `SAircraftPuppetEntity`, `SHovercraftPuppetEntity`, `SRollingBallPuppetEntity`, `STurretPuppetEntity`; 82 `.cb` files; moment identifiers `Action` 384, `FoeAssigned` 53, `Damage` 51, `BeginState` 14, `Idle` 13, `EndState` 11, `AllFoesDied` 8, `Born` 6, `ToolPicked` 6, empty 3, `StartRiding` 2, `StopRiding` 2.
