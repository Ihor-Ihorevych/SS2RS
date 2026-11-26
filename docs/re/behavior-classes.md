# AI behaviour and action classes of `Sam2Game.dll`: overview (T01-105, part of T01-026)

- **Roadmap task:** T01-105 (part 4 of the class inventory T01-026; the other parts are T01-102 to T01-104 and T01-106 to T01-113). Detail pages: [`behavior-action-classes.md`](behavior-action-classes.md) (actions and validity terms), [`behavior-mover-classes.md`](behavior-mover-classes.md) (movers, navigation moving type, tactics), [`behavior-entity-classes.md`](behavior-entity-classes.md) (behaviour entities, rides, think-point oscillator, foe manager)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class, method, member and enumeration names (export table and the DLL's own tables), for the class inventory counts and for everything the data side says (`tools/re/behavior_classes_verify.py`: which of the 95 classes are saved, their members, files, objects and stored values); **medium** for the inheritance, the net behaviour and what each method does (each rests on one reading of a short function; **nothing was run**); **low** for every method marked `N` or `P` on the detail pages and for the open questions.

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `ai.md` (the `.cb` behaviour files: states, bands, movers, actions, moments), [`brain-classes.md`](brain-classes.md) (the brain that runs everything on these pages), `misc-schema.md`, `ep-schema.md`, `wld-entity-schema.md`, [`replication.md`](replication.md), [`puppet-classes.md`](puppet-classes.md).

## Summary

The brain (`brain-classes.md`) decides *when*; this family is *what*: the **actions** a `.cb` file lists, the **validity terms** that gate them, the **behaviour entities** that carry out an action over time, the **movers** that say how to walk, the **tactic** that binds a mover to a patrol, and the **foe manager** that picks the brain's foe. `Sam2Game.dll` exports **95 classes with 955 functions** for it, in six groups:

| Group | Classes | Functions | Role | Detail page |
|---|---|---|---|---|
| Actions (`CBA*`, `SBAList`, `SBehaviorAction`) | 30 | 284 | one data object per `.cb` action; `Execute(brain)` | actions |
| Validity terms (`CBAVT*`) | 11 | 99 | one data object per condition; `IsValid(selection record)` | actions |
| Behaviour entities and properties | 32 (16 + 16) | 304 | the base, 7 generic action kinds, the list, go-to-point (2), possess and exit ride, think-point oscillator, 2 obsolete | entities |
| Foe managers | 4 (2 + 2) | 61 | the foe state machine; one subclass is obsolete | entities |
| Movers | 16 | 186 | `SMoveCharacter` and 13 movers, the navigation moving type, one record class | movers |
| Tactic | 2 | 21 | the placed object that holds a patrol mover | movers |

The 955 functions are **664 standard ones** (default and copy constructor, destructor, assignment, stream operator, one more special function and `mdGetDataType`; 94 classes have all, `SDynamicMoveData` has no destructor) and **291 named methods** of 112 distinct names. 95 data type objects (`md_pdtDataType`) are also exported, making 1,050 symbols.

**45 of the 95 classes are saved in files** (counted by exported name or `S` twin, as the survey did): 29 actions and terms, 13 movers, `SMoveCharacter`, the tactic entity and properties. One more is saved under another name (`CBAFoeAssigning` as `CBAPlayerAssigning`, **medium**). Every behaviour entity and the foe manager is binary only: the brain makes them at run time.

Findings an implementer needs first:

1. **An action is a factory or a setter, not a procedure.** Of the 28 actions that have an `Execute`, 8 spawn a generic behaviour entity and hand it to the brain, 12 set one thing on the brain or puppet, 8 control the foe, tactic or mover; the stored data of all of them is a handful of members (`behavior-action-classes.md`).
2. **The data classes carry the engine's own names for the values.** `EBrainAligment` is 0 Good, 1 Evil, 2 Neutral, 3 Semi neutral; `FoeHostility` runs 0 to 9; `BehaviorOnLastMarker` is Go back, Stop using tactic, Stay forever, Disappear, Die; `HealthObservation` has six modes of which one ("every N damage") is used by 46 of 51 stored health terms (section 4).
3. **Movers, tactics and bands close exactly**: 324 movers in the `.cb` files for the 324 bands that have one, 990 movers in worlds for the 990 tactics that have one, 17 in `.ep` files for the 17 shooting processes that point at one (`behavior-mover-classes.md`).
4. **Two entity classes of the line are obsolete** (`CTurretBehaviorEntity`, `CRideFoeManagerEntity`; their properties classes are empty) and print the obsolete-class error; **the navigation moving type and all behaviours exist only at run time**.
5. **Nothing here is replicated**: AI actions, terms, movers, behaviours and the foe manager are host-only (`NEB_NONE`, none of the 151 declared members has the replication flag).

## Evidence

- **Exported names.** The 11,610 exports of `Sam2Game.dll` were listed and the 95 classes of the family taken from the T01-026 split (`CBA*`, `CBAVT*`, `CGBA*`, `CBaseGenericBehavior*`, `CBehavior*`, `CGoToPoint*`, `CTurretBehavior*`, `CPossessRide*`, `CExitFromRide*`, the foe managers, `CTactic*`, `CMC*`, `CMNavigationMovingType`, `CThinkPointOscilator*`, `SBAList`, `SBehaviorAction`, `SDynamicMoveData`, `SMoveCharacter`). Names are quoted as exported.
- **Inheritance.** For every class the constructors were listed with the functions they call (two readings with the data's or the DLL table's base class, and the virtual-table shape); the column "source" below says which: `D` the base class in the data's type table, `T` the base named in the DLL's own member table, `C` the first parent constructor called, `V` only the shape of the virtual table (actions all share 7 slots, terms 5), which proves a sibling relation but not the parent's name (**medium**).
- **Member tables.** The DLL declares 38 tables for the family (14 for saved classes, 24 for classes no file stores); the 14 equal the data in names and order (script section 3). Offsets, sizes and flags beyond replication are not recorded.
- **Methods.** All 28 `Execute`, 11 `IsValid` and 113 behaviour and foe-manager methods were read or profiled; the movers' `MoveCharacter` bodies only partly (`behavior-mover-classes.md`). The decompiler resolves identical small functions to the name of one class, so a slot or a callee is only quoted when the class itself exports a function of that name.
- **Data.** `tools/re/behavior_classes_verify.py` (about 10 minutes, data only, read-only): the type tables of all 8,063 format-8 files, and every object of the 82 `.cb`, 322 `.ep` and 59 `.wld` files.
- **Enumerations.** Names, values and editor labels come from the DLL's enumeration tables (name, value, label triples).
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared.
- **Not checked.** Nothing was run. The brain's choice among actions, the steering geometry of the movers, the numbers inside the foe manager and the puppet side of the generic behaviours are open (section 6).

## Layout / rules

### 1. The 95 classes

All classes are defined in `Sam2Game.dll`. "Symbols / named" counts the exported functions and, of them, the named methods (the others are the standard functions). "Saved as" gives the data type name when the class or its `S` twin is in the data. Parent sources: see Evidence.


**Actions (30)**

| Class | Parent (source) | Symbols / named methods | Saved as (files, data members) |
|---|---|---|---|
| `CBAAnimation` | CBehaviorAction (D) | 11 / 4 | `SBAAnimation`, 58 files, 2 m. |
| `CBAChangeAligment` | SBehaviorAction (T) | 9 / 2 | - |
| `CBAChangeMovingAbilities` | CBehaviorAction (D) | 8 / 1 | same name, 4 files, 2 m. |
| `CBAChangeState` | SBehaviorAction (D) | 10 / 3 | `SBAChangeState`, 5 files, 1 m. |
| `CBACrouch` | SBehaviorAction (D) | 9 / 2 | same name, 3 files, 0 m. |
| `CBADie` | SBehaviorAction (D) | 9 / 2 | `SBADie`, 3 files, 0 m. |
| `CBAEmotion` | SBehaviorAction (D) | 9 / 2 | `SBAEmotion`, 3 files, 2 m. |
| `CBAFoeAssigning` | CBehaviorAction (V) | 9 / 2 | - |
| `CBAForceMovingType` | CBehaviorAction (T) | 9 / 2 | - |
| `CBAForcePlayerFoe` | CBehaviorAction (D) | 9 / 2 | same name, 3 files, 2 m. |
| `CBALeapOntoFoe` | CBehaviorAction (D) | 11 / 4 | `SBALeapOntoFoe`, 3 files, 1 m. |
| `CBALocalMove` | CBehaviorAction (D) | 12 / 5 | `SBALocalMove`, 2 files, 13 m. |
| `CBALoseFoe` | CBehaviorAction (D) | 9 / 2 | same name, 3 files, 0 m. |
| `CBAMelee` | CBehaviorAction (D) | 11 / 4 | `SBAMelee`, 42 files, 1 m. |
| `CBAPlaySound` | SBehaviorAction (D) | 10 / 3 | same name, 45 files, 5 m. |
| `CBAReuseTactic` | CBehaviorAction (D) | 9 / 2 | same name, 3 files, 0 m. |
| `CBARunOverFoe` | CBehaviorAction (D) | 11 / 4 | `SBARunOverFoe`, 7 files, 4 m. |
| `CBASetActionSet` | SBehaviorAction (T) | 10 / 3 | - |
| `CBAShieldStatus` | SBehaviorAction (D) | 9 / 2 | `SBAShieldStatus`, 2 files, 2 m. |
| `CBAShoot` | CBehaviorAction (D) | 11 / 4 | `SBAShoot`, 82 files, 7 m. |
| `CBAStopMoving` | SBehaviorAction (D) | 9 / 2 | same name, 3 files, 0 m. |
| `CBAStopTactic` | CBehaviorAction (V) | 9 / 2 | - |
| `CBAStopUsingTactic` | CBehaviorAction (D) | 9 / 2 | same name, 3 files, 0 m. |
| `CBAThrowMacroEvent` | SBehaviorAction (D) | 10 / 3 | `SBAThrowMacroEvent`, 1 files, 1 m. |
| `CBAThrowPickedTool` | SBehaviorAction (D) | 10 / 3 | same name, 3 files, 1 m. |
| `CBATryRiding` | CBehaviorAction (V) | 7 / 0 | - |
| `CBAUnforceMovingType` | CBehaviorAction (V) | 8 / 1 | - |
| `CBAWait` | CBehaviorAction (D) | 10 / 3 | same name, 5 files, 3 m. |
| `SBAList` | CBAList (D) | 10 / 3 | same name, 10 files, 1 m. |
| `SBehaviorAction` | CBehaviorAction (D) | 7 / 0 | same name, 46 files, 0 m. |

**Validity terms (11)**

| Class | Parent (source) | Symbols / named methods | Saved as (files, data members) |
|---|---|---|---|
| `CBAVTCurrentStateDuration` | CBAValidityTerm (D) | 9 / 2 | same name, 3 files, 1 m. |
| `CBAVTFoeAngle` | CBAValidityTerm (D) | 9 / 2 | same name, 2 files, 4 m. |
| `CBAVTFoeIsPlayer` | CBAValidityTerm (D) | 9 / 2 | same name, 4 files, 0 m. |
| `CBAVTHasFoe` | CBAValidityTerm (V) | 9 / 2 | - |
| `CBAVTHasFoeCandidate` | CBAValidityTerm (V) | 9 / 2 | - |
| `CBAVTHasTool` | CBAValidityTerm (D) | 9 / 2 | same name, 3 files, 1 m. |
| `CBAVTHealth` | CBAValidityTerm (D) | 9 / 2 | `SBAVTHealth`, 51 files, 3 m. |
| `CBAVTKnockOut` | CBAValidityTerm (V) | 9 / 2 | - |
| `CBAVTMeleeSuccess` | CBAValidityTerm (V) | 9 / 2 | - |
| `CBAVTPlayerDistance` | CBAValidityTerm (D) | 9 / 2 | same name, 5 files, 1 m. |
| `CBAVTWarMeasure` | CBAValidityTerm (V) | 9 / 2 | - |

**Behaviour entities and properties (32)**

| Class | Parent (source) | Symbols / named methods | Saved as (files, data members) |
|---|---|---|---|
| `CBaseGenericBehaviorEntity` | CBehaviorEntity (T,V) | 10 / 3 | - |
| `CBaseGenericBehaviorProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CBehaviorEntity` | CBaseEntity (data twin `SBaseEntity`) (C,T) | 22 / 15 | - |
| `CBehaviorProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CExitFromRideEntity` | CBehaviorEntity (T,V) | 10 / 3 | - |
| `CExitFromRideProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CGBAAnimationBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 8 / 1 | - |
| `CGBAAnimationBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGBALeapOntoFoeBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 12 / 5 | - |
| `CGBALeapOntoFoeBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGBALocalMoveBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 8 / 1 | - |
| `CGBALocalMoveBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGBAMeleeBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 9 / 2 | - |
| `CGBAMeleeBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGBARunOverFoeBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 24 / 17 | - |
| `CGBARunOverFoeBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGBAShootBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 9 / 2 | - |
| `CGBAShootBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGBAWaitBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 9 / 2 | - |
| `CGBAWaitBehaviorProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CGBActionListBehaviorEntity` | CBaseGenericBehaviorEntity (C) | 12 / 5 | - |
| `CGBActionListBehaviorProperties` | CBaseGenericBehaviorProperties (T) | 7 / 0 | - |
| `CGoToPointByPathEntity` | CBehaviorEntity (T,V) | 11 / 4 | - |
| `CGoToPointByPathProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CGoToPointEntity` | CBehaviorEntity (T,V) | 10 / 3 | - |
| `CGoToPointProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CPossessRideEntity` | CBehaviorEntity (T,V) | 18 / 11 | - |
| `CPossessRideProperties` | CBehaviorProperties (T) | 7 / 0 | - |
| `CThinkPointOscilatorEntity` | CBehaviorEntity (T,V) | 12 / 5 | - |
| `CThinkPointOscilatorProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CTurretBehaviorEntity` | CBehaviorEntity (T,V) | 8 / 1 | - |
| `CTurretBehaviorProperties` | CBaseProperties (C) | 7 / 0 | - |

**Foe managers (4)**

| Class | Parent (source) | Symbols / named methods | Saved as (files, data members) |
|---|---|---|---|
| `CFoeManagerEntity` | CBaseEntity (data twin `SBaseEntity`) (C,T) | 39 / 32 | - |
| `CFoeManagerProperties` | CBaseProperties (C) | 7 / 0 | - |
| `CRideFoeManagerEntity` | CFoeManagerEntity (C) | 8 / 1 | - |
| `CRideFoeManagerProperties` | CBaseProperties (C) | 7 / 0 | - |

**Movers (16)**

| Class | Parent (source) | Symbols / named methods | Saved as (files, data members) |
|---|---|---|---|
| `CMCAirAttack` | SMoveCharacter (D) | 10 / 3 | same name, 9 files, 11 m. |
| `CMCAwayFromFoe` | SMoveCharacter (D) | 9 / 2 | same name, 26 files, 0 m. |
| `CMCCircleAroundFoe` | SMoveCharacter (D) | 11 / 4 | same name, 34 files, 9 m. |
| `CMCDiamond` | SMoveCharacter (D) | 9 / 2 | same name, 9 files, 9 m. |
| `CMCDontMove` | SMoveCharacter (D) | 10 / 3 | same name, 35 files, 0 m. |
| `CMCDummy` | SMoveCharacter (D) | 9 / 2 | same name, 3 files, 0 m. |
| `CMCMarkersMovingClosestToFoe` | SMoveCharacter (D) | 18 / 11 | same name, 18 files, 7 m. |
| `CMCMarkersMovingOrdered` | SMoveCharacter (D) | 17 / 10 | same name, 38 files, 11 m. |
| `CMCMarkersMovingRandom` | SMoveCharacter (D) | 15 / 8 | same name, 11 files, 5 m. |
| `CMCMellon` | SMoveCharacter (D) | 9 / 2 | same name, 7 files, 5 m. |
| `CMCPatternMoving` | SMoveCharacter (D) | 9 / 2 | same name, 25 files, 5 m. |
| `CMCStayInArea` | SMoveCharacter (D) | 9 / 2 | same name, 13 files, 2 m. |
| `CMCTowardsFoe` | SMoveCharacter (D) | 9 / 2 | same name, 82 files, 0 m. |
| `CMNavigationMovingType` | SMoveCharacter (C) | 17 / 10 | - |
| `SDynamicMoveData` | none read (-) | 6 / 0 | - |
| `SMoveCharacter` | CMoveCharacter (D) | 19 / 12 | same name, 120 files, 11 m. |

**Tactic (2)**

| Class | Parent (source) | Symbols / named methods | Saved as (files, data members) |
|---|---|---|---|
| `CTacticEntity` | CBaseEntity (data twin `SBaseEntity`) (C,D) | 13 / 6 | `STacticEntity`, 38 files, 8 m. |
| `CTacticProperties` | CBaseProperties (data `SBaseProperties`) (C,D) | 8 / 1 | `STacticProperties`, 38 files, 7 m. |

Reading the tables: `CBAFoeAssigning` is saved as `CBAPlayerAssigning` (inference, **medium**: the binary registers a table of that name whose two booleans are the two arguments `CBAFoeAssigning::Execute` uses); `SBAList` and `SBehaviorAction` are exported with the `S` name; the data's `CBehaviorAction` (5 members), `CBAValidityTerm` (1), `CBAList` (1) and `CMoveCharacter` (0) are parents that no function exports. `GoToPoint*` member tables are registered without the leading `S` (quoted as found on the entity page).

### 2. Data classes of the same subject that the DLL does not export

| Data class | Base | Members | Files | Objects | Role |
|---|---|---|---|---|---|
| `CBehaviorAction` | none | 5 (`ba_pcValidityTerm`, `ba_fWeight`, three weight graphs) | 98 (76 `.cb`, 22 `.wld`) | 0 (abstract) | base of every action |
| `CBADefaultAction` | `CBehaviorAction` | 0 | 46 `.cb` | 131 | the "default action" entry (weights 1 .. 100,000; no function exported) |
| `CBANoAction` | `CBehaviorAction` | 0 | 54 `.cb` | 192 | the "do nothing" entry (weight 0.75 .. 10; 163 of 192 are 1) |
| `CBAList` | `CBehaviorAction` | 1 | 10 `.cb` | 0 (`SBAList` is stored) | base of `SBAList` |
| `CBehaviorActionList` | none | 2 (`bal_idMoment`, `bal_abaActions`) | 98 | 523 counted, all in worlds | one moment's weighted actions |
| `CAIMomentHandler` | none | 1 | 105 (82 `.cb`, 23 `.wld`) | 564 counted, all in worlds | the lists of a behaviour, state or band |
| `CCustomBehaviorArea` | none | 7 | 79 `.cb` | 381 | a distance band: mover, moment handler, velocity adjuster |
| `CBAValidityTerm`, `CBAVTTermCombiner` | none / `CBAValidityTerm` | 1 / 3 | 54 / 3 `.cb` | 0 / 3 | term base and the AND/OR combiner |
| `CBAPlayerAssigning` | `CBehaviorAction` | 2 | 3 `.cb` | 12 | the saved `CBAFoeAssigning` |

(The script's object walk reaches the handler and list objects of the worlds but not those of the `.cb` files, whose actions it does find as top-level objects; so 523 and 564 are the world-stored ones and the `.cb` list counts stay those of `ai.md`: `Action` 384, `FoeAssigned` 53, `Damage` 51, ...) `CBADefaultAction` and `CBANoAction` are the two commonest `.cb` actions (131 and 192 of 853): their effect is not in any exported function (**open, T01-153**).

### 3. How the pieces work together (**medium**, assembled from the detail pages)

1. A **moment** (`Action`, `Damage`, `FoeAssigned`, ... `brain-classes.md` 4.3) selects a `CBehaviorActionList` of the `.cb` behaviour for the current state, distance band and medium. The brain's `StartCharacterAction` chooses one action by weight and validity (the formula: [`behavior-action-choice.md`](behavior-action-choice.md), T01-153) and calls its `Execute(brain)`.
2. **Direct actions** (change state, play sound, die ...) act at once. **Behaviour-spawning actions** build a generic behaviour entity (`CGBA*BehaviorEntity`) and hand it to `StartGenericBehavior`; the behaviour asks the puppet to do the thing (shoot, melee, play an animation, step, wait, leap, run over) and, when it ends, tells the brain `OnBehaviorFinished`, which returns the brain to its think loop (the think-point oscillator, `CThinkPointOscilatorEntity`, calls `ThinkPoint` at a computed period).
3. The **mover** of the current distance band (`cba_pmcMoveCharacter`) or of a **tactic** (`STacticEntity`) is copied into the brain and `MoveCharacter` is called once per move period. A mover can ask for the **navigation moving type**, which the brain builds at run time.
4. The **foe manager** keeps the foe (`FoeHostility` ranks, the 60 s loss rule); `CBAFoeAssigning`, `CBAForcePlayerFoe` and `CBALoseFoe` act on it through the brain.
5. A **ride** is boarded by `CPossessRideEntity` and left by `CExitFromRideEntity` (states and members on the entity page).

### 4. The engine's own names for the values (enumerations of the DLL; the editor label is in quotes)

| Enumeration | Values |
|---|---|
| `EBrainAligment` | 0 `BA_GOOD` "Good", 1 `BA_EVIL` "Evil", 2 `BA_NEUTRAL` "Neutral", 3 `BA_SEMI_NEUTRAL` "Semi neutral" (stored in the `.cb` files: 1 in 72, 0 in 5, 3 in 3, 2 in 2, `ai.md`) |
| `FoeHostility` | 0 `FH_LIGHT`, 1 `FH_HEARD`, 2 `FH_SEEN`, 3 `FH_SENSED`, 4 `FH_DAMAGED`, 5 `FH_NEAR`, 6 `FH_FORCED`, 7 `FH_TOUCHED`, 8 `FH_HUNTING`, 9 `FH_AUTO_CHANGED_FOE` "Always" |
| `FoeManagerState` | 0 `FMS_DISABLED`, 1 `FMS_LOOKING_FOR_FOE`, 2 `FMS_TRACKING_CURRENT_FOE` |
| `HealthObservation` | 0 `HO_BELOW` "Is below", 1 `HO_ABOVE`, 2 `HO_DROPPED_BELOW`, 3 `HO_HEALED_ABOVE`, 4 `HO_DROPPED_EVERY` "Every N damage", 5 `HO_HEALED_EVERY` |
| `TermCombiner` | 0 `TC_AND` "And", 1 `TC_OR` "Or" |
| `AttackType` | 0 `ATT_NONE` "Invalid", 1 `ATT_SHOOT`, 2 `ATT_MELEE`, 3 `ATT_LEAP`, 4 `ATT_ROF` "Run over foe" |
| `MoveAbilities` (flags) | 1 `MA_WALK`, 2 `MA_FLY`, 4 `MA_SWIM` "Diving", 8 `MA_CANSTRAFEH`, 16 `MA_CANSTRAFEV`, 32 `MA_CANMOVEBACKWARD` |
| `CharacterMoverResult` | 0 `CMR_OK`, 1 `CMR_REPLACE_ME`, 2 `CMR_NEED_NAVIGATION`, 3 `CMR_NAVIGATION_FAILED` |
| `BehaviorOnLastMarker` | 0 `BOLM_GO_BACK`, 1 `BOLM_DESTROY_MOVER` "Stop using tactic", 2 `BOLM_STAY_FOREVER`, 3 `BOLM_DISAPPEAR`, 4 `BOLM_DIE` |
| `InitialMarkersDirection`, `AllowedMovingDirections`, `InitialCirclingDirection` | 0 forward, 1 backward, 2 random; 0 left, 1 right, 2 both; 0 random, 1 left, 2 right, 3 not altered |
| `PossesRideState`, `RunOverFoeStates` | 4 and 7 values (entity page) |
| `CharacterEmotion`, `CharacterShieldStatus` | -2 `CE_ANY`, -1 invalid, 0 `CE_FRIGHTENED`, 1 `CE_NORMAL`, 2 `CE_BRAVE`, 3 `CE_ENRAGE`, 4 `CE_CHARGE`; `CSS_NOT_SHIELDED` 0, `CSS_SHIELDED` 1 (`puppet-legged-classes.md`) |

The AI debug flags the binary names (bit sets the console can switch on) show what the engine counts as its parts: `DD_BEHAVIOR` "Behavior start/stop (log)", `DD_PUPPET_GOAL_POINT`, `DD_ALLOWED_MOVING_AREA`, `DD_PUPPET_PATH`, and `DD2_CURRENT_STATE`, `DD2_THINKERS`, `DD2_CURRENT_MOVER`, `DD2_GENCHA_ACTION_SELECTING_DATA`, `DD2_FOE_M_LOS` "Foe manager LOS (log)", `DD2_FOE_LINKS`, `DD2_HOSTILITY_GRAPH`, `DD2_LOS`, `DD2_FOV` (names only).

### 5. Net behaviour and replicated members (`replication.md`)

None of the 36 entity and property classes overrides `GetNetBehavior`, `GetRelevantDistance` or `GetUpdatePriority` (virtual-table comparison); the actions, terms and movers are data objects with no such hooks: **NEB_NONE, default priority**. Of the 38 member tables the DLL declares for the family (151 members) **none has the replication flag**; 12 members are flagged "owned or temporary" and 1 editor-only. The brain's puppet link is the AI's only replicated member (`brain-classes.md` 4.5). `CExitFromRideEntity` is the one class that tests for the host. So **clients do not run actions, terms, movers, behaviours or the foe manager**; they see the puppet's replicated pose, animation and movement.

### 6. Relation to `.cb`, `.wld` and `.ep` (counts from the data, script sections 1 to 5)

- **`.cb`** (82 files): 853 action objects of 25 classes, 381 bands (324 with a mover, 57 without), 110 term objects (104 attached to actions: `SBAAnimation` 50, `SBAChangeState` 18, `CBALoseFoe` 12, `SBAShoot` 7, `CBAForcePlayerFoe` 6, `CBAPlaySound` 4, `SBAMelee` 4, `SBAList` 3; 6 are operands of the 3 combiners). Term classes used: `SBAVTHealth` 51, `CBAVTFoeIsPlayer` 29, `CBAVTHasTool` 9, `CBAVTPlayerDistance` 9, `CBAVTFoeAngle` 6, `CBAVTCurrentStateDuration` 3, `CBAVTTermCombiner` 3 (operands: 3 `CBAVTPlayerDistance`, 3 `CBAVTFoeIsPlayer`).
- **`.wld`** (59 worlds): 993 tactics with 990 movers (755 ordered patrols); 614 `SBAShoot` and 16 `SBAAnimation` inside the moment handlers of 23 worlds (owned by the puppet's own concrete character record — `S{Aircraft,Legged}Character{Entity,Properties}` — not by `SPuppetEntity`/`SPuppetProperties`; [`behavior-leftovers.md`](behavior-leftovers.md), T01-159).
- **`.ep`** (322 files): 17 movers in 9 files (12 `CMCAirAttack`); no behaviour, action or term. `CFoeAssigning` (126 `.ep` files) is the puppet parameter that configures the foe assignment (`ai.md`; not a class of this family).
- **Matching schema pages**: movers in `ep-schema.md` (`CMCAirAttack`, `CMCCircleAroundFoe`, `CMCDontMove`, `CMCTowardsFoe`), actions and terms in `misc-schema.md`, the tactic in `wld-entity-schema.md`.

### 7. What the script reproduces

`python -I tools/re/behavior_classes_verify.py "<install>"` (about 10 minutes, data only) prints six blocks: (1) the 95 names against the type tables (45 saved by own name or `S` twin) with base, members and files; (2) the data classes of section 2; (3) the member-order check of 14 classes against the binary's declared names; (4) objects and files per class and extension; (5) the stored values of every member; (6) the term class attached to each action class, the combiner operands, the movers of the bands and the action weights. It does not reproduce anything read from the binary (methods, enumerations, inheritance, net behaviour).

## Invariants

1. 45 of the 95 classes are in the data under their own name or `S` twin (script section 1); the 24 binary-only member tables are in none (24 of 24).
2. The 14 data classes whose table the binary declares have the binary's member names in the same order (14 of 14).
3. `ba_pgrWeightOverHealthMultiplier`, `...DistanceMultiplier` and `...AreaRatioMultiplier` are null in all 1,483 stored actions (853 + 630); `mc_fAutoOffTime` and `mc_fAutoOffDistance` are -1 in all 1,331 stored movers.
4. No entity or property class of the family overrides a net hook (36 of 36 compared).
5. `CTurretBehaviorEntity` and `CRideFoeManagerEntity` print the obsolete-class error and do nothing else (2 of 2).

## Open questions

Each has a roadmap task (appended after T01-151):

1. **T01-152**: the generic-behaviour protocol on the puppet side (`CanPerformGenericShooting`, `DoGenericShooting`, `DoGenericMeleeAttack`, `DoGenericAnimation`, `DoLocalMove`, the leap and run-over states) and the numbers inside the run-over-foe states.
2. **T01-153** (answered, [`behavior-action-choice.md`](behavior-action-choice.md)): the choice among actions: weights, validity (`vt_bNotTrue`, the combiner), the frequency graphs, the behaviour-wide list, `CBADefaultAction` and `CBANoAction`.
3. **T01-154**: the steering numbers of the movers (circle, stay-in-area, pattern, diamond, mellon, air attack), the base `MoveCharacter` returns, the `ValidateAndSetGoalPoint` limits.
4. **T01-155**: the marker movers and the navigation moving type in full (path length, waits, `CachePath`, `RecheckWaypoints`, failure).
5. **T01-156**: the foe manager's periods and constants, the line-of-sight request, the sorting key, the world-info "may be foes" test, the oscillator's think period.
6. **T01-157**: the exact semantics of the validity terms (angle convention, health modes 2, 3 and 5, the five terms the data does not use) and of the seven actions no file uses (`CBATryRiding`, `CBAChangeAligment`, `CBASetActionSet`, `CBAForceMovingType`, `CBAStopTactic`, `CBAUnforceMovingType`, and `CBAFoeAssigning` by its other name).
7. **T01-158**: run-time checks of the behaviours on the running original: which classes exist in a running level, the order of action start and finish for one enemy (`Kleer`, `Albino`), mover results, patrol macro events; numbers only.
8. **T01-159** (answered, [`behavior-leftovers.md`](behavior-leftovers.md)): data leftovers: which object class owns the 564 world-stored `CAIMomentHandler`s (614 `SBAShoot`), whether any mod data creates the obsolete classes, and how a tactic without a marker gets one.

## Test vectors

- **Counts**: 95 classes, 955 functions = 664 standard + 291 named; 45 saved; actions 30, terms 11, behaviours 32, foe managers 4, movers 16, tactic 2.
- **Enumerations** (section 4): `BA_EVIL` 1, `FH_FORCED` 6, `HO_DROPPED_EVERY` 4, `TC_OR` 1, `CMR_NEED_NAVIGATION` 2, `BOLM_DIE` 4.
- **Chain** (section 3): a band with mover `CMCTowardsFoe` and an `Action` list holding `CBAShoot` `Chainballs`: the brain calls `Execute`, which spawns `CGBAShootBehaviorEntity`; if the puppet refuses (no line of sight), `Execute` returns false and the brain is told the behaviour did not start.
- **Closure**: the 324 movers of the `.cb` files equal the 324 bands with a mover; the 990 world movers equal the 990 tactics with a mover (755 of them `CMCMarkersMovingOrdered`).
- `tools/re/behavior_classes_verify.py` prints: `45 of 95`; `SBAShoot 726 objects (cb:112, wld:614)`; `STacticEntity 993 objects`; `CBAVTTermCombiner` 3 objects, operation AND.
