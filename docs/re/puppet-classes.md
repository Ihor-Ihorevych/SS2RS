# Puppet classes of `Sam2Game.dll`: overview (T01-103, part of T01-026)

- **Roadmap task:** T01-103 (part 2 of the class inventory T01-026; the other parts are T01-102, T01-104 to T01-113). Companion pages: [`puppet-entity-classes.md`](puppet-entity-classes.md) (the base class `CPuppetEntity`), [`puppet-entity-methods.md`](puppet-entity-methods.md) (its 519 methods), [`puppet-legged-classes.md`](puppet-legged-classes.md) (the walking kinds and their helpers), [`puppet-vehicle-classes.md`](puppet-vehicle-classes.md) (rolling ball, turret, aircraft, hovercraft, wheeled vehicle and the kinds derived from them).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/puppet_classes_verify.py` reproduces from the data: which of the 101 classes are saved (50), under which name, their base class, member counts and file counts; the **member-order rule** (for 16 data classes the data's members, in file order, are the members that the binary declares, in the same order; every number-only member therefore has a name); the root classes of the 124 puppet parameter files and which Character entity class goes with which parameter class; and the absence from the install of every entity of the 11 obsolete kinds, of the dummy foe and of the generic legged kind. **High** also for the class, method and enumeration names, which are read from the export table and the DLL's own tables. **Medium** for everything else read from the binary (parents of the binary-only classes, the virtual tables, the member meanings, the rules of the sections called "read"), because each rests on one reading of a function and **nothing was run**. **Low** for what a method does when it is described only from its name (marked "(name)") and for the items under "Open questions".

Prerequisites: `classes.md` and `classes-catalog.md` (the classes of the data), `ep.md` and `ep-schema.md` (the parameter files and their member lists), `enemies.md` and `vehicles.md` (what the parameter values are), `puppet-stats.md`, `ai.md` and `brain-classes.md` (the brains: this page picks up the shared hook that `brain-classes.md` found), `replication.md` (net behaviour), `wld-entities.md` and `wld-entity-schema.md` (the placed entities), `players-session-classes.md` (the player puppet `CPlayerPuppetEntity`, which is **not** part of this family), `conventions.md`.

## Summary

A **puppet** is an entity that lives, moves and can be controlled: every enemy, animal, NPC, turret, aircraft, hovercraft and rolling ball of the game, and (as a subclass) the player. `Sam2Game.dll` exports **101 classes** with **1,565 exported symbols** (1,464 functions and 101 `md_pdtDataType` data symbols; **783 named methods** besides the standard set) that make up the puppets, **without the player's own classes** (`CPlayerPuppet*`, part T01-102). They are:

| Role | Classes | Symbols | Named methods | In the data |
|---|---|---|---|---|
| **Entities** (the live object) | 26 | 961 | 753 | 11 |
| **Properties** (what a world stores for a placed entity) | 26 | 218 | 10 | 11 |
| **Parameters** (the `.ep` file of a character) | 24 | 192 | 0 | 7 |
| **Records and helpers** (parts of the parameter files, animation selectors, adjusters) | 25 | 194 | 20 | 21 |
| **Total** | **101** | **1,565** | **783** | **50** |

**50 of the 101 are saved in the data** (49 by the usual rule: the exported `C` name has an `S` twin in the type tables; plus `CMiscPuppetParams`, which the data spells `CMiscPupeptParams`, with a typo that is the game's own). **51 exist only in the binary.**

Seven findings an implementer needs first:

1. **The classes of the 11 "kinds" are dead code; the kinds are parameter files.** The world places one of **five Character entity classes** (`SLeggedCharacterEntity` 1,825 times, `SAircraftCharacterEntity` 254, `STurretCharacterEntity` 164, `SRollingBallCharacterEntity` 44, `SHovercraftCharacterEntity` 41: 2,328 entities, `wld-entities.md`), and a sixth, `CWheeledVehicleCharacterEntity`, exists only in the binary. Each answers "yes" to `IsGenericPuppet` and takes its whole identity (model, speeds, attacks, health) from the **`.ep` file** that its `en_pppPuppetParams` member names. The 11 old per-kind classes (`CKleerPuppetEntity`, `CFlyingKleerPuppetEntity`, `CWereBullPuppetEntity`, `CZombieSoldierPuppetEntity`, `CSciFiSoldierPuppetEntity`, `CNPCPuppetEntity`, `CManifestedSpawnerPuppetEntity`, `CAirBikePuppetEntity`, `CAutoTurretPuppetEntity`, `CBuggyPuppetEntity`, `CMilitaryCarPuppetEntity`) have no code but one `OnCreate` that prints `Class (...) is obsolete and should be removed from level ASAP.` and are in no world (section 3.4). This is the "shared creation hook" that `brain-classes.md` found. **High** for the data side, **medium** for the reading of the binary.
2. **The parameter file decides everything and its class decides the movement model.** The five parameter classes with a data presence are `SLeggedPuppetParams` (81 root files), `SAircraftPuppetParams` (19), `STurretPuppetParams` (14), `SHovercraftPuppetParams` (5), `SRollingBallPuppetParams` (2), plus one `SGenericLeggedPuppetParams` (`Animals/Worm.ep`) and the player's `SPlayerPuppetParams` (2: `Player/Player.ep`, `NPCs_cutsequences/Santa.ep`): 124 files. A Character entity refuses a file that is not of its class (`GetPuppetParamsClass`, section 3.2).
3. **The unnamed members of the puppet files get their names.** The data stores 37 members of `SPuppetEntity`, 7 of `SPuppetProperties`, 7 of `SPuppetParams`, 3 of `SLeggedPuppetParams`, 12 of `SLeggedPuppetEntity`, 4 of `STurretPuppetEntity` and 1 of `STurretPuppetParams` with a bare number. The binary declares the same members, in the same order, with names; the order rule (script, section 2) assigns every number its name (section 4). Among them: **the three environment records of every legged character are `lpp_pepLand`, `lpp_pepAir` and `lpp_pepWater`**, which settles the "probably land, air and water" of `enemies.md`.
4. **`CPuppetEntity` is the largest class of the game's gameplay code**: 519 named methods, 394 virtual slots (102 inherited from `CBaseEntity`, 55 of those overridden, 292 new) and 245 members (`SPuppetEntity`; 242 in the data, 55 replicated). Every movement model is a subclass that overrides a handful of virtuals, above all `LerpToDesiredVelocityAndLook`.
5. **The puppet has its own 16-state machine** (`PS_DEFAULT`, `PS_IDLE`, `PS_LOCAL_MOVE`, `PS_CONSECUTIVE_GENERIC_SHOOTING`, `PS_GENERIC_MELEE_ATTACK`, `PS_EXPLODING`, `PS_FADE_OUT`, ...; section 3.3), independent of the brain's 16 states (`brain-classes.md`).
6. **Net behaviour**: a puppet is `NEB_DYNAMIC` unless a spawner made it (then `NEB_STATIC`), its update priority is 8.0 while alive and 2.0 otherwise, 55 of its 245 members are replicated (`replication.md`, confirmed here by a second reading).
7. **What is not here.** No wheeled vehicle, no dummy foe, no generic legged puppet and none of the 11 obsolete kinds is in any world or `.ep` file of the install (script, sections 1 and 4); the file `Puppets/Objects/DummyFoe.ep` that the dummy foe's code loads is not in the install either.

## Evidence

- **Exported names.** The 11,610 exports of `Sam2Game.dll` were listed and the 932 exported classes grouped by name (the split of T01-026); the 101 classes of this family are those with a puppet, character, moving-animation, ride or seat name that no other part claims. Their exported functions total **1,464**, which equals the survey of T01-026. All names are quoted as exported (the `C` name; the data uses the `S` twin, `players-session-classes.md` section 1).
- **Parents.** For every class the **copy constructor** was read to see which parent constructor it calls (the entity and parameter classes), and for the entity and properties classes the **virtual function table** was compared slot by slot with its parent's. Two readings; **medium**, because identical small functions are shared by the linker (a slot can then show another class's name, so slot names are not used as evidence), and a constructor can inline its parents' constructors (the properties classes then show only the top parent; the data's own base classes, which the script prints, are used where the data has the class).
- **Member tables.** The DLL's static tables of class members (`replication.md` section 3.4: 229 read) contain 22 tables of this family (for example `SPuppetEntity` 245 members, `SPuppetProperties` 40, `SPuppetParams` 18, `SLeggedPuppetEntity` 61); names and flags are quoted, **byte offsets and sizes were read but are not recorded** (`conventions.md`). No table exists for the records `SPuppetHealthParams`, `SPuppetBodyParams`, `SMovingAbilities`, `SKickingAbilities`, `SPuppetDeathParams`, `SPuppetEffectParams`, `SPuppetEnvironmentParams`, `CMiscPuppetParams`: their numbered members stay numbered (section 4).
- **Enumerations.** Name, value pairs of about 30 puppet-related enumerations were read from the DLL's tables with a script that finds the name strings and the words around them (not committed: it reads the binary) and checked against the type names that the data uses for the same members (script, section 5).
- **Methods.** For all 783 named methods the size, the strings, the numbers read from constants and the exported functions each refers to were listed (a profile). In the three method tables (766 rows) the column **Basis** says how each sentence is known: **`R`** 153 rows (the function was decompiled and read: the parameter loading, the net hooks, the damage, health and death routines, `SetState`, `OnStep` and `OnBoot` in outline, the knock-out and environment functions, the rolling-ball, turret and dummy-foe functions, and every function of 12 bytes or less that answers a question), **`P`** 297 rows (the strings, constants and callees of the function were listed and the sentence follows them), **`N`** 316 rows (the name alone: **low**). The other 17 methods are the six `IsGenericPuppet` and eleven `OnCreate` functions of the Character and obsolete classes.
- **Data.** `tools/re/puppet_classes_verify.py` (about 5 minutes) reads the type tables of all 8,063 format-8 files and the 124 puppet parameter files and prints the counts of sections 1 to 5; the numbers quoted from other specs (placed entities) are from their scripts (`vehicles_verify.py`, `wld_entity_verify.py`).
- **Binary build.** The `Sam2Game.dll` of the install used for the other specs; no other build was compared. **Not checked:** nothing was run; no movement step, damage routine or attack sequence was read in full (they are follow-up tasks); the editor classes of other DLLs were not looked at.

## Layout / rules

### 1. The 101 classes

`Parent` is the C++ parent (constructor and virtual table, **medium**; where the data has the class its own base is the script's **high**); `Data name` is the type name in the 8,063 files (script section 1) with the number of files that carry it (`.wld` files for entities and properties, `.ep` for parameters); "-" = binary only.

**1.1 Entities (26)**

| Class | Parent | Data name (files) | Methods | Virtual slots (overridden) | Role |
|---|---|---|---|---|---|
| `CPuppetEntity` | `CBaseEntity` | `SPuppetEntity` (45) | 519 | 394 (55 of the 102 inherited; 292 new) | the base of every puppet: [`puppet-entity-classes.md`](puppet-entity-classes.md) |
| `CLeggedPuppetEntity` | `CPuppetEntity` | `SLeggedPuppetEntity` (44) | 121 | 445 (56 overridden, 51 new) | the walking, swimming and jumping character: [`puppet-legged-classes.md`](puppet-legged-classes.md) |
| `CRollingBallPuppetEntity` | `CPuppetEntity` | `SRollingBallPuppetEntity` (10) | 21 | 396 (20, 2 new) | a ball driven by torque, with a seat |
| `CTurretPuppetEntity` | `CPuppetEntity` | `STurretPuppetEntity` (21) | 18 | 394 (17) | a fixed gun with two joints |
| `CAircraftPuppetEntity` | `CPuppetEntity` | `SAircraftPuppetEntity` (29) | 16 | 394 (18) | free flight with leaning |
| `CHovercraftPuppetEntity` | `CPuppetEntity` | `SHovercraftPuppetEntity` (10) | 13 | 394 (14) | spring-suspended hover with jumps |
| `CWheeledVehiclePuppetEntity` | `CPuppetEntity` | - | 13 | 397 (11, 3 new) | wheels with driving and steering |
| `CDummyFoePuppetEntity` | `CPuppetEntity` | - | 13 | 394 (14) | a test target with a built-in model (section 3.5) |
| `CGenericLeggedPuppetEntity` | `CLeggedPuppetEntity` | - | 2 | 445 (4) | a legged puppet whose model is a property (section 3.5) |
| `CLeggedCharacterEntity` | `CLeggedPuppetEntity` | `SLeggedCharacterEntity` (44) | 1 | 445 (3) | **the placed enemy, NPC, animal and boss** |
| `CAircraftCharacterEntity` | `CAircraftPuppetEntity` | `SAircraftCharacterEntity` (29) | 1 | 394 (2) | the placed aircraft |
| `CTurretCharacterEntity` | `CTurretPuppetEntity` | `STurretCharacterEntity` (21) | 1 | 394 (3) | the placed turret |
| `CHovercraftCharacterEntity` | `CHovercraftPuppetEntity` | `SHovercraftCharacterEntity` (10) | 1 | 394 (2) | the placed hovercraft |
| `CRollingBallCharacterEntity` | `CRollingBallPuppetEntity` | `SRollingBallCharacterEntity` (10) | 1 | 396 (2) | the placed rolling ball |
| `CWheeledVehicleCharacterEntity` | `CWheeledVehiclePuppetEntity` | - | 1 | 397 (2) | would be the placed wheeled vehicle (none in any world) |
| `CKleerPuppetEntity`, `CFlyingKleerPuppetEntity`, `CWereBullPuppetEntity`, `CZombieSoldierPuppetEntity`, `CSciFiSoldierPuppetEntity`, `CNPCPuppetEntity`, `CManifestedSpawnerPuppetEntity` | `CLeggedPuppetEntity` | - | 1 each (`OnCreate`) | 445 (3 each) | obsolete shells (section 3.4) |
| `CAirBikePuppetEntity` | `CHovercraftPuppetEntity` | - | 1 | 394 (2) | obsolete shell |
| `CAutoTurretPuppetEntity` | `CTurretPuppetEntity` | - | 1 | 394 (3) | obsolete shell |
| `CBuggyPuppetEntity`, `CMilitaryCarPuppetEntity` | `CWheeledVehiclePuppetEntity` | - | 1 each | 397 (2 each) | obsolete shells |

Counts: 9 + 6 + 11 = 26. "Overridden" counts the slots whose target differs from the parent's; **every class overrides the destructor slot and `mdGetDataType`**, so the 2 or 3 of a Character or obsolete class are those two plus `IsGenericPuppet` / `OnCreate` (a class with 3 has one more slot that the table builder shows as internal). Virtual slot numbers are not recorded.

**1.2 Properties (26).** One properties class per entity class, named `C<Kind>PuppetProperties` or `C<Kind>CharacterProperties`. Only `CPuppetProperties` (`SPuppetProperties`, 40 members, 45 files; 9 named methods, mostly editor and spawner-link functions) has members and methods; `CGenericLeggedPuppetProperties` adds one method (`GetBoundingBox`) and three members (`ep_pmcModel`, `ep_vStretch`, `ep_glppPuppetParamsObsolete`), `CDummyFoePuppetProperties` four (`ep_baAligment`, `ep_bFlying`, `ep_fVelocity`, `ep_slHealth`; the spelling is the binary's); all others are empty shells with the 7 standard symbols (`SAircraftPuppetProperties` 29 files, `SHovercraftPuppetProperties` 10, `SLeggedPuppetProperties` 44, `SRollingBallPuppetProperties` 10, `STurretPuppetProperties` 21, and the five Character twins, all 0 members). Parent chain: `CCharacterProperties` of a kind derive from that kind's puppet properties (data); every kind's puppet properties derive from `CPuppetProperties` ← `CBaseProperties`.

**1.3 Parameters (24)**

| Class | Parent | Data name (files) | Members | Notes |
|---|---|---|---|---|
| `CPuppetParams` | `CEntityParams` | `SPuppetParams` (126) | 18 | the common parts; section 4 |
| `CLeggedPuppetParams` | `CPuppetParams` | `SLeggedPuppetParams` (86 files: 81 roots, `Worm.ep`, the 2 player files and the 2 `EntityParams/PlayerPuppet*.ep`) | 5 | three environment records, body, knock-out |
| `CAircraftPuppetParams` | `CPuppetParams` | `SAircraftPuppetParams` (19) | 24 | `vehicles.md` |
| `CTurretPuppetParams` | `CPuppetParams` | `STurretPuppetParams` (14) | 3 | `vehicles.md` |
| `CHovercraftPuppetParams` | `CPuppetParams` | `SHovercraftPuppetParams` (5) | 18 | `vehicles.md` |
| `CRollingBallPuppetParams` | `CPuppetParams` | `SRollingBallPuppetParams` (2) | 9 | `vehicles.md` |
| `CGenericLeggedPuppetParams` | `CLeggedPuppetParams` | `SGenericLeggedPuppetParams` (1: `Animals/Worm.ep`) | 0 | an empty subclass; the one file is of this class |
| `CWheeledVehiclePuppetParams` | `CPuppetParams` | - | 5 (declared: `wwpp_fMaxSpeed`, `wwpp_fVehicleAcceleration`, `wwpp_fVehicleDecceleration`, `wwpp_fWheelSteeringTorque`, `wwpp_fWheelSteeringSpeed`) | no file uses it |
| `CAirBikePuppetParams` | `CHovercraftPuppetParams` | - | 0 | obsolete |
| `CBuggyPuppetParams`, `CMilitaryCarPuppetParams` | `CWheeledVehiclePuppetParams` | - | 0 | obsolete |
| `CAutoTurretPuppetParams` | `CPuppetParams` | - | 0 | obsolete |
| `CKleerPuppetParams`, `CFlyingKleerPuppetParams`, `CWereBullPuppetParams`, `CZombieSoldierPuppetParams`, `CSciFiSoldierPuppetParams`, `CManifestedSpawnerPuppetParams` | `CLeggedPuppetParams` | - | 0 | obsolete |
| `CNPCPuppetParams` and its four flavours `CNPCBomberPuppetParams`, `CNPCCivilianPuppetParams`, `CNPCKamikazePuppetParams`, `CNPCShooterPuppetParams` | `CLeggedPuppetParams` (the flavours: read from the constructors as `CLeggedPuppetParams`; whether they derive from `CNPCPuppetParams` was not read) | - | 0 | obsolete; **no `.ep` file is of a NPC class**: the 22 NPC files are `SLeggedPuppetParams` |
| `CDummyFoePuppetParams` | `CPuppetParams` | - | 0 | belongs to the dummy foe |

The player's `SPlayerPuppetParams` (`CPlayerPuppetParams`, T01-102) is a child of `SLeggedPuppetParams` in the data (2 files in `Databases/Puppets`: `Player/Player.ep` and `NPCs_cutsequences/Santa.ep`).

**1.4 Records and helpers (25).** Parts of the parameter files and run-time helpers; the 21 saved ones are in the data under the names below.

| Class | Data name (files) | Members | Role |
|---|---|---|---|
| `CPuppetHealthParams` | `SPuppetHealthParams` (126; 2 variants) | 8 | health, healing and the health bar (`php_` members) |
| `CPuppetBodyParams` | `SPuppetBodyParams` (126) | 16 | model, aim points, hull flags, shields, auto-aim switch |
| `CMovingAbilities` | `SMovingAbilities` (126) | 17 | steering, slope and fall limits, flocking, step heights |
| `CKickingAbilities` | `SKickingAbilities` (126) | 15 | kick damage and impulse (`ka_` members) |
| `CPuppetDeathParams` | `SPuppetDeathParams` (126) | 10 | exploding damage, corpse sink config |
| `CPuppetEffectParams` | `SPuppetEffectParams` (126) | 12 | step and alive effects, sound ranges |
| `CMiscPuppetParams` | `CMiscPupeptParams` (126; 2 variants) | 6 | score, ride icons, boss name, sniper multiplier |
| `CLeggedPuppetBodyParams` | `SLeggedPuppetBodyParams` (86) | 11 | jump, look-around, view heights, spine data |
| `CPuppetSeatData` | `SPuppetSeatData` (22) | 11 | one seat of a ride |
| `CWayToSeat` | `SWayToSeat` (21) | 3 | one way to enter a seat |
| `CRideAndRiderAnim` | `SRideAndRiderAnim` (21) | 4 | a pair of animations of ride and rider |
| `CShieldDescription` | `CShieldDescription` (2) | 2 | a shield tool and part |
| `CCorpseSinkConfig` | `CCorpseSinkConfig` (2) | 5 | how a corpse waits and sinks (`rsc-ctl.md`) |
| `CDamageMultipliers` | `CDamageMultipliers` (1, `.rsc`) | 5 | global multipliers by damage kind (`msr_` members; use unresolved) |
| `CPuppetSoundRanges` | `CPuppetSoundRanges` (1, `.rsc`) | 6 | global sound fall-off ranges |
| `CMovingAnimData` | `CMovingAnimData` (44, `.wld`) | 7 | the six selectors of a moving animation of a placed puppet |
| `CAllMovingAnimation`, `CBodyMovingAnimation`, `CPoseTransitionAnimation`, `CMovingAnimationScheme` | same names (48, 17, 25, 60; `.mdl`/`.mas`) | 12, 6, 11, 4 | the moving-animation scheme of a model |
| `CSpineAdjuster` | `CSpineAdjuster` (44, `.wld`) | 3 | bends the spine bones towards the look direction |
| `CLerpToAnimAdjuster` | - | - | blends a pose into an animation (knock-out) |
| `CMoveCharacterData`, `CRideAndSeat`, `CWheelData` | - | - | run-time records (movement, a ride and its seat, a wheel); empty shells in the export table |

### 2. What the script reproduces about the members

For **16 of the 22 declared tables** whose class is in the data, the data's members in file order are exactly the declared members in order (a declared member the data does not store is skipped: the three camera handles of `SPuppetEntity`; 14 older world files store 234 members, the first 234 of the 242 in the same order, so `en_henFlame`, `en_tmSpawned`, `en_strName` and five numbered members are missing there). Every named data member is at the position of the same declared name, with **7 exceptions of spelling** (the data and the binary disagree on a name at the same position, with a plausible type: `en_pfFlags` against `en_pnfNetFlags`, `ep_idPuppet` against `ep_idPuppetObsolete`, `ep_idBehavior` against `ep_idBehaviorObsolete`, `ep_bPassTroughBarriers` against `ep_bcBarrierCollision`, `app_bLeanPWhenMovingFB` against `app_bLeanPWhenMoving`, `app_bLeanBWhenMovingFBRotatingH` against `app_bLeanBWhenMoving`, `psd_idSittingAnim` against `psd_idRiderAnimRideNotMoving`: the reason is not known; the types agree) and **1 type surprise** (`en_bLastFrameMoving` is a `FLOAT` in the data although its name says `bool`). The rule is **high** as a statement about the files (script); the names given to the numbered members are **high** where the position, the count and the type all agree and the class is one of the 16 (listed in section 4).

### 3. The kinds, the parameter loading and the enumerations

#### 3.1 Character classes and generic puppets (**medium**; data side **high**)

A **generic puppet** is a puppet class whose character is chosen by data. The base `IsGenericPuppet` (a virtual function) answers **false**; the six `C<Kind>CharacterEntity` classes override it with a one-line function that answers **true** (read twice: the function bodies and the virtual tables, where the six differ from their parents in exactly this slot). Every placed enemy, animal, NPC, boss, turret, aircraft, hovercraft and rolling ball of the shipped worlds is such a Character entity (script section 3: the five Character classes with a data presence are in 44, 29, 21, 10 and 10 world files; the wheeled-vehicle one in none), and its `en_pppPuppetParams` equals the link of the world's record in **2,328 of 2,328** cases (`wld-entities.md`). The consequences an implementer needs:

- **The class of the entity is the movement model, the parameter file is the character.** Pairing (script section 3, all from the data): `SLeggedCharacterEntity` ↔ `SLeggedPuppetParams` (81 root files, and the derived worm, player and Santa files), `SAircraftCharacterEntity` ↔ `SAircraftPuppetParams` (19), `STurretCharacterEntity` ↔ `STurretPuppetParams` (14), `SHovercraftCharacterEntity` ↔ `SHovercraftPuppetParams` (5), `SRollingBallCharacterEntity` ↔ `SRollingBallPuppetParams` (2). A wheeled vehicle would need `SWheeledVehiclePuppetParams`; none exists. The one `SGenericLeggedPuppetParams` file (`Worm.ep`) is accepted by `SLeggedCharacterEntity` because its class derives from `SLeggedPuppetParams`; the generic legged entity is binary-only.
- **The brain rule of `brain-classes.md` section 2.3 can never pick a per-kind brain for these entities**: it replaces the text `PuppetEntity` in the class name by `BrainProperties` and looks the result up, and the names `CLeggedCharacterEntity` and so on contain no `PuppetEntity`. A Character entity with a `.cb` file gets the plain `CBrainEntity` (rule step 1) and one without gets it too (step 3). This explains in one line why the 16 per-kind brain classes are dead: the 11 puppet classes they pair with are dead as well (section 3.4). **Medium**: one reading; the running original was not observed (T01-125).
- A Character class has **no methods of its own** besides `IsGenericPuppet` (1 named method each), no members (the data's `SLeggedCharacterEntity` has 0) and no properties of its own (`SLeggedCharacterProperties` has 0 members).

#### 3.2 Loading the parameters (`GetPuppetParameters`, `GetPuppetParamsClass`; **medium**)

`CPuppetEntity::GetPuppetParameters` takes the character identifier and returns the parameter resource (or nothing):

1. For a **generic puppet** (the six Character classes): the class asks `GetPuppetParamsClass` which parameter class it needs (the base answers "invalid"; the legged class answers `CLeggedPuppetParams`, the rolling ball `CRollingBallPuppetParams`, the turret `CTurretPuppetParams`, the aircraft `CAircraftPuppetParams`, the hovercraft `CHovercraftPuppetParams`, the wheeled vehicle `CWheeledVehiclePuppetParams`; the dummy foe and the generic legged class inherit), turns the identifier into the resource name, loads it and **accepts it only if its class is derived from the required class**; otherwise it prints the warning `Parameters for character "<id>" are not of proper class ("<class>")` and returns nothing.
2. For a **non-generic puppet** (the old per-kind classes and the dummy foe): the name comes from the entity itself; **in deathmatch the text `.ep` in the name is replaced by `_DM.ep`** and that resource is loaded (the deathmatch variant of a character; the same convention as the `_dm` weapon files of `weapons.md`).
3. `OnCreate` stores the result in `en_pppPuppetParams` (a replicated member) and the entity then reads every number from it; `SPuppetProperties.ep_pppPuppetParams` is the placed value.

The `_DM.ep` rule is in the non-generic branch only; **the generic branch has no deathmatch variant**, the identifier of a generic character is looked up in a table of the `Puppets` folder, and only the player puppet's file has a `_DM` twin: all of this, the missing-file case and the order inside `OnCreate` are in [`puppet-params-loading.md`](puppet-params-loading.md) (T01-139).

#### 3.3 Enumerations declared by the binary

Value and name as the DLL's tables give them (quoted as exported). The first column is the type name the **data** uses for the same members, where it was found (script section 5), else the prefix of the binary names. **Medium** (one table each, read twice: strings and a table parse); the data confirms the type names, not the value lists, except where an observed stored value is quoted in another spec.

| Data type (prefix) | Values |
|---|---|
| `PuppetState` (`PS_`; member `en_psState`) | 0 `PS_NONE`, 1 `PS_DEFAULT`, 2 `PS_IDLE`, 3 `PS_SHUT_DOWN`, 4 `PS_LOCAL_MOVE`, 5 `PS_GENERIC_ACTION_PLAY_ANIM`, 6 `PS_CONSECUTIVE_GENERIC_SHOOTING`, 7 `PS_GENERIC_MELEE_ATTACK`, 8 `PS_EXPLODING`, 9 `PS_FADE_OUT`, 10 `PS_START_AUTO_PATROLING`, 11 `PS_AUTO_FIXING_RIDE`, 12 `PS_BEING_SPAWN_SOURCE`, 13 `PS_DISAPPEARING`, 14 `PS_APPEARING`, 15 `PS_SPAWNEE_RISING` |
| `EPuppetPose` (`PP_`) | 0 `PP_INVALID`, 1 `PP_STAND`, 2 `PP_CROUCH`, 3 `PP_DIVE`, 4 `PP_SWIM`, 5 `PP_FLY`, 6 `PP_FALL`, 7 `PP_ROLL`, 8 `PP_KO`, 9 `PP_RIDING_WHEEL` |
| `LeggedPuppetState` (`LPS_`) | 0 `LPS_NONE`, 1 `LPS_KNOCK_OUT`, 2 `LPS_PLAYING_RIDE_AND_RIDER_SEQUENCE`, 3 `LPS_GENERIC_LEAP_ATTACK` |
| `RidingState` (`RS_`) | 0 `RS_NOT_RIDING`, 1 `RS_APPROACHING`, 2 `RS_ENTERING`, 3 `RS_RIDING`, 4 `RS_EXITING` |
| `RideAndRiderSequence` (`RRS_`) | 0 `RRS_ENTERING`, 1 `RRS_EXITING` |
| (`PRS_`, a ride phase of a player brain; name not found in the data) | 0 `PRS_REACH_DESTINATION_SEAT`, 1 `PRS_REACH_WAYPOINT_SEAT`, 2 `PRS_ENTER_ONTO_SEAT`, 3 `PRS_WAIT_RIDE_IDLE` |
| (`RT_`) | 0 `RT_RIDDING_ACTIVELY`, 1 `RT_RIDDING_PASSIVELY` |
| `PuppetFlags` (`PNF_`, replicated, member `en_pfFlags` in the data) | 1 `PNF_INVULNERABILITY`, 4 `PNF_CAN_BE_USED`, 8 `PNF_DISALLOW_RECHAGING`, 16 `PNF_LAUNCHED`, 32 `PNF_UPSIDEDOWN`, 64 `PNF_DISABLE_STOP_BEFORE_PATROL`, 128 `PNF_MINI_BOSS` (bit 2 is unused) |
| `PuppetLocalFlags` (`PLF_`, not replicated) | 1 `PLF_CURRENTLY_RIDING`, 2 `PLF_ALL_RIDERS_RIDING`, 4 `PLF_EXPLODED`, 8 `PLF_ON` ("ride is on"), 16 `PLF_REPORT_DAMAGE`, 32 `PLF_NOTIFY_IDLE`, 64 `PLF_NOTIFY_AI_ENABLED`, 128 `PLF_UPSIDEDOWN`, 256 `PLF_MINI_BOSS`, 65,536 `PLF_USES_MOVING_SOUNDS` |
| `PuppetExecutionAbilities` (`PEAF_`) | 1 `PEAF_MACRO_ACTIONS`, 2 `PEAF_LERP_TO_DESIRED`, 4 `PEAF_APPLY_OPERATOR_MOVE` |
| `MoveAbilities` (`MA_`, flags) | 1 `MA_WALK`, 2 `MA_FLY`, 4 `MA_SWIM` ("Diving"), 8 `MA_CANSTRAFEH`, 16 `MA_CANSTRAFEV`, 32 `MA_CANMOVEBACKWARD` |
| `EMoveType` (`MT_`) | 0 `MT_NONE`, 1 `MT_GOAL_POINT`, 2 `MT_DESIRED_TEMPO_LOOK`, 3 `MT_DRIVE_STEER_LOOK` |
| `EMovingDirection` (`MD_`) | 0 `MD_INVALID`, 1 `MD_LEFT`, 2 `MD_FORWARD`, 3 `MD_RIGHT`, 4 `MD_BACKWARD`, 5 `MD_STOP`, 6 `MD_TURNLEFT`, 7 `MD_TURNRIGHT`, 8 `MD_UP`, 9 `MD_DOWN` |
| `BarrierCollision` (`BC_`) | 0 `BC_BLOCKED_BY_BARRIER`, 1 `BC_PASS_THROUGH_BARRIER`, 2 `BC_BLOCKED_BY_CUSTOM_BARRIER` |
| `SeatFlags` (`SF_`) | 1 `SF_RIDER_CAN_SHOOT_OWN_WEAPON`, 2 `SF_WAY_POINT_SEAT`, 4 `SF_PUPPETS_CANT_SEAT`, 8 `SF_RIDER_DIES_WITH_RIDE`, 16 `SF_PLAYER_CANT_SEAT`, 32 `SF_PLAYER_CAN_BE_DAMAGED`, 64 `SF_NON_PLAYER_IS_INVOULNERABLE`, 128 `SF_PLAYER_CANT_EXIT` |
| `EBrainAligment` (`BA_`; the spelling is the binary's) | 0 `BA_GOOD`, 1 `BA_EVIL`, 2 `BA_NEUTRAL`, 3 `BA_SEMI_NEUTRAL` |
| `EThreatSensitivity` (`TS_`) | 0 `TS_INVALID`, 1 `TS_DEADHORSE`, 2 `TS_DAMAGEONLY`, 3 `TS_DAMAGEANDSENSE`, 4 `TS_SHORTSIGHTED`, 5 `TS_STANDARD`, 6 `TS_LONGSIGHTED`, 7 `TS_HEARTHROUGHWALLS`, 8 `TS_SEETHROUGHWALLS` |
| `EnemySize` (`ES_`; member `CMiscPupeptParams.8`) | 0 `ES_EX_SMALL`, 1 `ES_SMALL`, 2 `ES_MEDIUM`, 3 `ES_LARGE`, 4 `ES_X_LARGE`, 5 `ES_BOSS` |
| `MovingAnimationPose` (`MAP_`) | -2 `MAP_ANY`, -1 `MAP_INVALID`, 0 `MAP_STAND`, 1 `MAP_CROUCH`, 2 `MAP_DIVE`, 3 `MAP_SWIM`, 4 `MAP_FLY`, 5 `MAP_FALL`, 6 `MAP_ROLL`, 7 `MAP_KO`, 8 `MAP_LADDER`, 9 `MAP_PARACHUTE`, 10 `MAP_DEATH` |
| `MovingAnimationSpeed` (`MAS_`) | -2 `MAS_ANY`, -1 `MAS_INVALID`, 0 `MAS_FAST`, 1 `MAS_SLOW`, 2 `MAS_IDLE` |
| `MovingAnimationDirection` (`MAD_`) | -2 `MAD_ANY`, -1 `MAD_INVALID`, 0 `MAD_FORWARD`, 1 `MAD_BACKWARD`, 2 `MAD_LEFT`, 3 `MAD_RIGHT`, 4 `MAD_TURNLEFT`, 5 `MAD_TURNRIGHT`, 6 `MAD_UP`, 7 `MAD_DOWN`, 8 `MAD_NOT_MOVING` |
| `MovingAnimationReadiness` (`MAR_`) | -2 `MAR_ANY`, -1 `MAR_INVALID`, 0 `MAR_NOT_READY`, 1 `MAR_READY` |
| `CharacterShieldStatus` (`CSS_`) | -1 `CSS_ANY` and `CSS_INVALID` (both -1), 0 `CSS_NOT_SHIELDED`, 1 `CSS_SHIELDED` |
| `CharacterEmotion` (`CE_`) | -2 `CE_ANY`, -1 `CE_INVALID`, 0 `CE_FRIGHTENED`, 1 `CE_NORMAL`, 2 `CE_BRAVE`, 3 `CE_ENRAGE`, 4 `CE_CHARGE` |
| (`DC_`, the three environments) | 0 `DC_LAND`, 1 `DC_AIR`, 2 `DC_WATER` |
| (`PSST_`, a puppet sound source) | 0 `PSST_MOUTH`, 1 `PSST_MOVE`, 2 `PSST_AVAILABLE` |
| (`CTA_`, the state of a character tool; section 9 of `puppet-entity-classes.md`) | 0 `CTA_READY_FOR_DRAWING`, 1 `CTA_DRAWING`, 2 `CTA_DRAWING_AND_RESIZING`, 3 `CTA_DRAWN`, 4 `CTA_READY_FOR_STORING`, 5 `CTA_STORING`, 6 `CTA_STORING_AND_RESIZING`, 7 `CTA_STORED`, 8 `CTA_READY_FOR_PICKING`, 9 `CTA_PICKING`, 10 `CTA_PICKING_AND_LERPING`, 11 `CTA_PICKED`, 12 `CTA_READY_FOR_THROWING`, 13 `CTA_THROWING`, 14 `CTA_THROWN` |
| (`TDT_`) | 0 `TDT_AUTO`, 1 `TDT_STORE`, 2 `TDT_THROW` |
| (`ROFS_`, the stages of a run-over attack) | 0 `ROFS_CANCELED`, 1 `ROFS_ROTATE_TOWARDS_FOE_BEFORE`, 2 `ROFS_ROTATE_TOWARDS_FOE_AFTER`, 3 `ROFS_APPROACH_CLOSE_ENOUGH`, 4 `ROFS_RUN_OVER`, 5 `ROFS_KICK`, 6 `ROFS_CONTINUE_RUN_THROUGH` |
| (`PUPPET_ANIM_GROUP_`) | 0 `LIFE`, 1 `ALL`, 2 `BODY`, 3 `ACTION`, 4 `MACRO_ALL`, 5 `MACRO_BODY`, 6 `MACRO_HEAD`, 7 `MACRO_FACE` (the animation layers: life, all, body, action and four macro layers) |

`PuppetExecutionAbilities` and `PuppetFlags` are different from the brain's execution flags (`EAF_`, `brain-classes.md` section 4.4). The damage types (`EDamageType`, `DMT_`) belong to the weapon and item parts.

#### 3.4 The 11 obsolete kinds and the shared hook (**medium**)

`CKleerPuppetEntity`, `CFlyingKleerPuppetEntity`, `CWereBullPuppetEntity`, `CZombieSoldierPuppetEntity`, `CSciFiSoldierPuppetEntity`, `CNPCPuppetEntity` and `CManifestedSpawnerPuppetEntity` derive from `CLeggedPuppetEntity`; `CAirBikePuppetEntity` from `CHovercraftPuppetEntity`; `CAutoTurretPuppetEntity` from `CTurretPuppetEntity`; `CBuggyPuppetEntity` and `CMilitaryCarPuppetEntity` from `CWheeledVehiclePuppetEntity`. Each class's virtual table equals its parent's except for the destructor slot, `mdGetDataType` and **one overridden `OnCreate`**; all 11 point at the **same function**, the one that `brain-classes.md` section 2.1 describes: it asks for the object's class name and prints the console error `Class (<name>) is obsolete and should be removed from level ASAP.`, then returns (so a world that still contained one would get the error and an entity without a model, mechanism or parameters; the base `OnCreate` is **not** called). The same function is the `OnCreate` of the 16 obsolete brain classes and of a few effect, foe manager, behavior and item classes (other parts). The properties and parameter classes of the 11 kinds are empty shells with no member.

Evidence that none is used: no world, parameter file or other data file has any of the 11 entity, properties or parameter classes (script section 1: "none"), the 22 NPC characters, the Kleer, the were-bull and the others are `SLeggedPuppetParams` files (`enemies.md`), and no `.ep` file has a root class named for a kind. **High** for the data side. That the engine ever created one is not known: the classes may be left over from an earlier design in which every kind had its own class (the names of the parameter classes `CNPCBomberPuppetParams`, `CNPCCivilianPuppetParams`, `CNPCKamikazePuppetParams` and `CNPCShooterPuppetParams` suggest four kinds of NPC characters; the data has one NPC family under `Puppets/NPCs`).

#### 3.5 The dummy foe and the generic legged puppet (**medium**)

Two classes are **live in the binary but not in the data**:

- **`CDummyFoePuppetEntity`** (`CPuppetEntity` child, 13 methods): a target dummy. Its properties (`ep_baAligment`, `ep_bFlying`, `ep_fVelocity`, `ep_slHealth`) are copied by `OnCreate` into `en_baAlignment`, `en_bFlying`, `en_fVelocity` and `en_slCustomHealth`; its model is fixed (`GetPuppetModel` returns the path `Content/SeriousSam2/Models/Enemies/DummyFoe/DummyFoe.mdl`) and `OnCreate` loads the built-in parameter file `Puppets/Objects/DummyFoe.ep` when it has none. **Neither file is in the install** (script section 4: of 12,342 archive entries the only match for "dummyfoe" or `Puppets/Objects` is `Cactus.ep`). Its movement getters are constant-returning stubs (one returns `en_fVelocity`; the others were not resolved), its death sequence calls `OnShutDown` at once and it has an immortality member (`en_bImmortal`). The brain pairing is `CDummyFoeBrainEntity` (no foe manager; `brain-classes.md` section 2.2). Who creates a dummy foe (an editor tool, a script or a test level) was not found.
- **`CGenericLeggedPuppetEntity`** (`CLeggedPuppetEntity` child, 2 methods): a legged puppet whose **model and stretch come from its own properties** (`ep_pmcModel`, `ep_vStretch`), so a level designer can place any model as a walking puppet without a character file; its parameter class is `SGenericLeggedPuppetParams` (an empty `SLeggedPuppetParams` child; the install has one such `.ep` file, `Databases/Puppets/Animals/Worm.ep`, no world places one). Its brain pairing is `CGenericLeggedBrainEntity`. No world places one.

### 4. The names of the numbered members (the order rule)

The data's type tables store a member either by its name or, when the name is a number in the game's own tables, by that number. For every class in the list below the data's members, in file order, are the declared members in order (script section 2), so **the position gives the name**. **High** where the class is one of the 16 checked and the data type fits the Hungarian prefix of the declared name (all 71 numbered members below fit); the meaning of the name is a separate matter (the member tables of the entity pages). `.n` is the number the data stores.

| Class (data name) | Numbered member -> declared name (data type) |
|---|---|
| `SPuppetEntity` (37) | `.3` = `en_henBrain` (`Handle<CBrainEntity>`); `.30` = `en_henSpawner` (`Handle<CSpawnerEntity>`); `.31` = `en_henSpawnerWhoSpawnedMe` (`Handle<CSpawnerEntity>`); `.20` = `en_vDesiredTempoAbs` (`Vector3f`); `.21` = `en_vDesiredLookDirEul` (`Vector3f`); `.61` = `en_vGoalPointRotationEul` (`Vector3f`); `.33` = `en_bForceDesiredLookDir` (`BOOL`); `.40` = `en_bMultiSpawnerTemplate` (`BOOL`); `.81` = `en_maLastMoveAbilities` (`MoveAbilities`); `.100` = `en_hmech` (`Handle<CMechanism>`); `.101` = `en_hbo` (`Handle<CBody>`); `.102` = `en_hjo` (`Handle<CJoint>`); `.103` = `en_hre` (`Handle<CModelRenderable>`); `.300` = `en_idLastSetMechanism` (`IDENT`); `.301` = `en_idLastSetCollisionCategory` (`IDENT`); `.902` = `en_fLastJointVelocity` (`FLOAT`); `.1101` = `en_ulAutoPatrolMarkerEntityID` (`ULONG`); `.1102` = `en_bAutoPatrolMarkers` (`BOOL`); `.1200` = `en_tmLastNonIdle` (`Time`); `.1300` = `en_tsDefaultThreatSensitivity` (`EThreatSensitivity`); `.28` = `en_pbaLocalMove` (`CBALocalMove*`); `.42` = `en_hmechCarried` (`Handle<CMechanism>`); `.60` = `en_bShouldRotateOnGP` (`BOOL`); `.22` = `en_henLookTarget` (`Handle<CBaseEntity>`); `.23` = `en_vGoalPoint` (`Vector3f`); `.24` = `en_mtMoveType` (`EMoveType`); `.50` = `en_bAlive` (`BOOL`); `.80` = `en_maMoveAbilities` (`MoveAbilities`); `.200` = `en_qvOriginalPlacement` (`QuatVect`); `.900` = `en_fSpeedMultiplier` (`FLOAT`); `.901` = `en_fPatrolSpeedMultiplier` (`FLOAT`); `.1000` = `en_fRnd` (`FLOAT`); `.302` = `en_fMaxDotTouchNormalAndRelVelocity` (`FLOAT`); `.303` = `en_hhuStrongestTouch` (`Handle<CHull>`); `.304` = `en_vStrongestTouchNormal` (`Vector3f`); `.305` = `en_bDisableFallingDamageOnNextTouch` (`BOOL`); `.306` = `en_vAccuDamage` (`Vector3f`) |
| `SPuppetProperties` (7) | `.10` = `ep_hspSpawner` (`Handle<CSpawnerProperties>`); `.20` = `ep_bMultiSpawnerTemplate` (`BOOL`); `.30` = `ep_hepAutoPatrolMarker` (`Handle<CPathMarkerProperties>`); `.33` = `ep_bAutoPatrolMarkers` (`BOOL`); `.31` = `ep_ulAutoPatrolMarkerEntityID` (`ULONG`); `.32` = `ep_bIsSpawned` (`BOOL`); `.34` = `ep_tsDefaultThreatSensitivity` (`EThreatSensitivity`) |
| `SPuppetParams` (7) | `.1` = `pp_phpHealth` (`SPuppetHealthParams`); `.2` = `pp_pbpBody` (`SPuppetBodyParams`); `.7` = `pp_maMovingAbilities` (`SMovingAbilities`); `.3` = `pp_pppProjectileParams` (`SPuppetProjectileParams`); `.4` = `pp_csoSeats` (`CScriptObjectContainer`); `.5` = `pp_kaKickingAbilities` (`SKickingAbilities`); `.6` = `pp_pdpDeath` (`SPuppetDeathParams`) |
| `SLeggedPuppetParams` (3) | `.1` = `lpp_pepLand` (`SPuppetEnvironmentParams`); `.2` = `lpp_pepAir` (`SPuppetEnvironmentParams`); `.3` = `lpp_pepWater` (`SPuppetEnvironmentParams`) |
| `SLeggedPuppetEntity` (12) | `.80` = `en_ppPuppetPose` (`EPuppetPose`); `.50` = `en_vFrontEul` (`Vector3f`); `.51` = `en_vRelLookEul` (`Vector3f`); `.52` = `en_mdMoveDirection` (`EMovingDirection`); `.70` = `en_fOldDesTempoToDesLook` (`FLOAT`); `.71` = `en_iOldMoveDirection` (`INDEX`); `.72` = `en_hsaSpineAdjuster` (`Handle<CSpineAdjuster>`); `.81` = `en_bJumpButtonReleased` (`BOOL`); `.82` = `en_bInTheAirAfterJump` (`BOOL`); `.83` = `en_fFluidRatio` (`FLOAT`); `.106` = `en_tmLastJump` (`Time`); `.110` = `en_bAutoAnimations` (`BOOL`) |
| `STurretPuppetParams` (1) | `.1` = `tpp_fRotationSpeed` (`FLOAT`) |
| `STurretPuppetEntity` (4) | `.1` = `tpe_fUpJointLimitEul` (`FLOAT`); `.2` = `tpe_fDnJointLimitEul` (`FLOAT`); `.3` = `tpe_henParent` (`Handle<CBaseEntity>`); `.4` = `tpe_qvRelToParent` (`QuatVect`) |

The numbered members of the records `SPuppetHealthParams` (`.1`, `.2`), `SPuppetBodyParams` (`.1`, `.3`, `.4`, `.6`, `.7`, `.10` to `.12`), `SPuppetDeathParams` (`.1` to `.4`), `SPuppetEffectParams` (`.1`, `.2`), `CMiscPupeptParams` (`.8`), `SKickingAbilities`, `SMovingAbilities`, `SPuppetEnvironmentParams` and `SKnockOutParams` have **no declared table** in the DLL (they are not entity-like classes), so their numbers stay numbers (`ep-schema.md`, `enemies.md`); follow-up T01-151.

### 5. How the classes relate to the `.ep` and `.wld` files

- **`.wld`**: a placed puppet is a record of a Character class plus a record of the matching properties class (`SLeggedCharacterEntity` + `SLeggedCharacterProperties` and so on): `wld-entities.md` (2,328 placed Character entities, with `en_pppPuppetParams` equal to the record's link in 2,328 of 2,328). The entity record stores the 242 members of `SPuppetEntity` and the class's own members; the properties record stores the 40 members of `SPuppetProperties`.
- **`.ep`**: one file per character under `Databases/Puppets` (124), root class `SLeggedPuppetParams` 81, `SAircraftPuppetParams` 19, `STurretPuppetParams` 14, `SHovercraftPuppetParams` 5, `SRollingBallPuppetParams` 2, `SPlayerPuppetParams` 2, `SGenericLeggedPuppetParams` 1. The 18 members of `SPuppetParams` (`pp_phpHealth` `.1`, `pp_pbpBody` `.2`, `pp_pppProjectileParams` `.3`, `pp_csoSeats` `.4`, `pp_kaKickingAbilities` `.5`, `pp_pdpDeath` `.6`, `pp_maMovingAbilities` `.7`, `pp_pfxEffectParams`, `pp_opOperatingParams`, `pp_omObjectManipulation`, `pp_mppMiscPuppetParams`, `pp_cCameras`, `pp_paPuppetAttack`, `pp_atdThrowDescs`, `pp_hcmGenericMounts`, `pp_faaFoeAssigning`, `pp_amtMountedTools`, `pp_aiShownAttachments`) are the same in all 126 files that carry the class (the declared table of `SPuppetParams` gives the names of the seven numbered ones). `ep.md` and `ep-schema.md` give the values; `enemies.md` and `vehicles.md` explain them.
- **`.mdl` and `.mas`**: the moving-animation scheme (`CMovingAnimationScheme` with `CAllMovingAnimation`, `CBodyMovingAnimation`, `CPoseTransitionAnimation` lists) is part of the model files (60 files), and `CMovingAnimData` and `CSpineAdjuster` are stored in placed legged puppets (44 worlds).
- **`.rsc`**: `CCorpseSinkConfig` (global values: `CGlobalGameParams`, `rsc-ctl.md`), `CDamageMultipliers` (5 global multipliers `msr_f...`: cutting, explosion, kicking, non-player against model, puppet against puppet; **their use was not found** among the functions read, T01-140) and `CPuppetSoundRanges` (6 sound fall-off and hot-spot ranges for puppet, moving and step sounds) are in one `.rsc` file each.

## Invariants

1. Of the 101 classes **50 are in the data** (11 entity, 11 properties, 7 parameter, 21 record); the other 51 are binary only (script section 1).
2. No entity, properties or parameter class of the 11 obsolete kinds, the wheeled vehicle, the dummy foe or the generic legged puppet is in any of the 8,063 type tables (script sections 1 and 3); the only parameter file of the generic legged class is `Animals/Worm.ep`.
3. For 16 classes whose declared table is in the data, count and order of the members agree and every named member has the declared name except 7 spelled differently (script section 2).
4. All 124 puppet parameter files have one of seven root classes (script section 3).
5. A Character entity answers true to `IsGenericPuppet`, and the six Character classes are the only classes that do (6 of 6 read).

## Open questions

Each open item has a follow-up task in `docs/roadmap/T01-reverse-engineering.md`.

1. **`GetPuppetParameters` in generic puppets and the `_DM` rule** (T01-139): whether a generic puppet has a deathmatch variant and how the identifier becomes a resource name.
2. **Damage, hits and kills** (T01-140): the rest of `OnReceiveDamage`, `OnHullTouched`, `ApplyDamageImpact`, the use of the co-op multiplier and of `CDamageMultipliers`, kick damage.
3. **Movement steps** of the legged kind (T01-141) and of the other models (T01-142).
4. **Attacks** (T01-143, done: [`puppet-attacks.md`](puppet-attacks.md)): the shooting chain, melee and run-over blasts, projectile spawning, `LaunchTool`.
5. **Tools, carrying, rides and seats** (T01-144).
6. **Spawning, appearing and the spawn-source states** (T01-145).
7. **Run-time checks on the running original** (T01-146): numbers for every rule marked **medium** here (co-op multiplier, deathmatch health cap, corpse timings, knock-out gate, the order of the step).
8. **Knock-out, leap and ride sequences** (T01-147), **the moving-animation chooser** (T01-148), **diving and drowning** (T01-149).
9. **The dummy foe** (T01-150) and **the names of the numbered members of the record classes** (T01-151).
10. **Which brain each placed puppet really gets** (T01-125, already open): section 3.1 gives the class-name argument.

## Test vectors

- **Counts**: 101 classes = 26 + 26 + 24 + 25; 1,565 exported symbols = 1,464 functions + 101 data symbols; 50 in the data (11 + 11 + 7 + 21); 783 named methods (753 + 10 + 0 + 20).
- **Member naming**: `SPuppetEntity.3` is `en_henBrain`; `SPuppetEntity.50` is `en_bAlive`; `SLeggedPuppetParams.2` is `lpp_pepAir`; `STurretPuppetEntity.4` is `tpe_qvRelToParent`.
- **Class pairing**: a placed `SLeggedCharacterEntity` with `Databases/Puppets/Enemies/Kleer.ep` (`SLeggedPuppetParams`) is valid; with `Aircraft` files it is refused with the warning of section 3.2.
- **Obsolete kind**: creating `CKleerPuppetEntity` prints `Class (CKleerPuppetEntity) is obsolete and should be removed from level ASAP.` and builds nothing.
- **`tools/re/puppet_classes_verify.py` prints** (section 1): `in the data: 50 of 101 (49 by the own-name / S-twin rule + 1 saved under a misspelled name ...)`, `by role {'entity': 11, 'properties': 11, 'params': 7, 'record': 21}`; section 2: `classes checked: 16 ... 16`; section 3: 124 files with roots `SLeggedPuppetParams` 81, `SAircraftPuppetParams` 19, `STurretPuppetParams` 14, `SHovercraftPuppetParams` 5, `SRollingBallPuppetParams` 2, `SPlayerPuppetParams` 2, `SGenericLeggedPuppetParams` 1; section 4: 12,342 archive entries, one match (`Cactus.ep`).
