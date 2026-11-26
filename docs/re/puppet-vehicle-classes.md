# Rolling ball, turret, aircraft, hovercraft, wheeled vehicle and the other puppet models (T01-103, part of T01-026)

- **Roadmap task:** T01-103 (overview [`puppet-classes.md`](puppet-classes.md); base class [`puppet-entity-classes.md`](puppet-entity-classes.md); the parameter values are in `vehicles.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class, member and method names, the member lists (declared against stored, script), the placement counts and the absence of the wheeled vehicle, the dummy foe and the generic legged entity from the data; **medium** for the method descriptions marked `R` or `P` and for the member meanings; **low** for those marked `N` (name only) and for the open questions. Nothing was run; no movement step was read.

Prerequisites: [`puppet-classes.md`](puppet-classes.md), [`puppet-entity-classes.md`](puppet-entity-classes.md), `vehicles.md` and `vehicles-table.md` (all parameter values and the 40 files), `enemies.md`, `brain-classes.md` (rides by AI), `replication.md`.

## Summary

Besides the walking kind, five movement models and two special classes derive from `CPuppetEntity`. Each overrides the **movement step** (`LerpToDesiredVelocityAndLook`), `SetCurrentMechanism`, `GetPuppetParamsClass` and a few getters (acceleration, deceleration, nominal velocity), and none stores more than a handful of members. Each has a **Character** child that is the placed object.

| Model | Entity (data name) | Placed as | Param class (files) | Methods | Own members | Placed (worlds) |
|---|---|---|---|---|---|---|
| Rolling ball | `CRollingBallPuppetEntity` (`SRollingBallPuppetEntity`) | `SRollingBallCharacterEntity` | `SRollingBallPuppetParams` (2) | 21 | 6 | 44 (10) |
| Turret | `CTurretPuppetEntity` | `STurretCharacterEntity` | `STurretPuppetParams` (14) | 18 | 5 | 164 (21) |
| Aircraft | `CAircraftPuppetEntity` | `SAircraftCharacterEntity` | `SAircraftPuppetParams` (19) | 16 | 2 | 254 (29) |
| Hovercraft | `CHovercraftPuppetEntity` | `SHovercraftCharacterEntity` | `SHovercraftPuppetParams` (5) | 13 | 3 | 41 (10) |
| Wheeled vehicle | `CWheeledVehiclePuppetEntity` | `CWheeledVehicleCharacterEntity` (binary only) | `CWheeledVehiclePuppetParams` (**none**) | 13 | 3 | **0** |
| Dummy foe | `CDummyFoePuppetEntity` | - | `CDummyFoePuppetParams` (none; loads a built-in file that is not in the install) | 13 | 4 (properties) + 4 | **0** |
| Generic legged | `CGenericLeggedPuppetEntity` | - | `SGenericLeggedPuppetParams` (1: `Animals/Worm.ep`) | 2 | 1 + 3 (properties) | 0 |

(The placement counts and file counts are from `vehicles.md` and `wld-entities.md`; the world counts are the `.wld` files that define the class, script section 1 of `puppet-classes.md`.) The four obsolete kinds that derive from these models (`CAirBikePuppetEntity` from the hovercraft, `CAutoTurretPuppetEntity` from the turret, `CBuggyPuppetEntity` and `CMilitaryCarPuppetEntity` from the wheeled vehicle) are empty shells (`puppet-classes.md` section 3.4). **The data has no wheeled vehicle at all** (no entity and no parameter file): the wheeled-vehicle code (wheel data, steering torque) exists in the binary only.

What an implementer needs first:

1. **The movement model is chosen by the Character class and checked by the parameter class** (`puppet-classes.md` section 3.2): a rolling-ball entity given an aircraft file prints `Parameters for character ... are not of proper class`.
2. **The turret is the one model that does not move**: `GetDeclaredNominalVelocity` answers 0, `OnStep` is empty, the gun turns on two joints named `GunLR` (left-right) and `GunUD` (up-down) with the speed `tpp_fRotationSpeed` (180 in 13 files, 300 in one) and acceleration and deceleration `tpp_fGunAcceleration`, `tpp_fGunDecceleration` (400 in all), limited by the instance members `tpe_fUpJointLimitEul` and `tpe_fDnJointLimitEul`; it can be **attached to a parent entity** (`tpe_henParent`, `tpe_qvRelToParent`, both replicated; `macSetParent` is the script function; errors: `Cannot attach turret (%1) to parent (%2), parent doesn't provide a mechanism.` and `... only turrets with valid mechanism and joint in root part can be attached to a parent entity.`). `ShouldExamineThreatSound` answers no: a turret does not turn towards sounds. **Medium** (`AttachToParent` read; the rest profile).
3. **Aircraft are always "in the air"** (`GetDipContent` answers `DC_AIR`, `MovingIn3DArea` yes); their step uses per-axis limits (`app_fMaxFrontVel`, `app_fMaxUpVel`, `app_fMaxRightVel`) and accelerations, three angular accelerations and velocities and leaning (`vehicles.md`).
4. **The hovercraft is not a flyer** (`MovingIn3DArea` no, `CanRotateInPlace` no): it hovers on a spring joint (`CHovercraftJoint`, error `Can't find main hovercraft joint`) with jumps; its `OnStep` throws the riders out in some state.
5. **The rolling ball is driven by torque**: `GetTorque` returns zero until `rbpp_fNonBrakingPeriodAfterImpulse` has passed since the last force impulse and then a torque along the look direction (or along the velocity when it has none), scaled by the body mass and the ball radius (the radius is half the size of the first round hull of the body, default 1); the rider's seat is a hybrid body with a ball joint, turned to the rider's heading with `rbpp_fSeatRotationVelocity` (360). **Medium** (`GetTorque`, `GetRollingBallRadius` read; the formula is not written down).

## Evidence

- **Members.** Declared tables of `STurretPuppetEntity` (5), `SAircraftPuppetEntity` (2), `SHovercraftPuppetEntity` (3), `SRollingBallPuppetEntity` (6), `SWheeledVehiclePuppetEntity` (3), `SDummyFoePuppetEntity` (4), `SGenericLeggedPuppetEntity` (1) and the matching parameter tables (3, 24, 18, 9, 5) were compared with the data: for the five classes that are in the data both entity and parameter lists are equal in count and order (script section 2: 0 differing names, except 2 spellings in `SAircraftPuppetParams`: `app_bLeanPWhenMovingFB` and `app_bLeanBWhenMovingFBRotatingH` in the data against `app_bLeanPWhenMoving`, `app_bLeanBWhenMoving` declared; the dummy foe, the generic legged and the wheeled-vehicle tables are not in the data).
- **Virtual tables.** Compared with `CPuppetEntity`'s 394 slots: rolling ball 20 overridden and 2 new, turret 17, aircraft 18, hovercraft 14, wheeled vehicle 11 and 3 new, dummy foe 14; the Character children 2 to 3 (destructor, `mdGetDataType`, `IsGenericPuppet`).
- **Methods.** All 126 methods of these classes and records are in the tables below (26 `R` read, 70 `P` profile, 30 `N` name). The movement steps (aircraft 2.9 KB, turret 2.5 KB, hovercraft 2.0 KB, rolling ball 1.4 KB, wheeled vehicle 0.6 KB) were only profiled.
- **Data.** `tools/re/puppet_classes_verify.py` sections 1 to 4.

## Layout / rules

### 1. Members of the model classes

| Class | Members (data order) | Meaning | Confidence |
|---|---|---|---|
| `STurretPuppetEntity` | `tpe_fUpJointLimitEul` (.1), `tpe_fDnJointLimitEul` (.2), `tpe_henParent`\* (.3), `tpe_qvRelToParent`\* (.4), `en_bLastRotating` | the upper and lower limit of the up-down joint (Euler angle), the parent entity and the placement relative to it (replicated), whether the gun turned in the last step (name) | medium |
| `SAircraftPuppetEntity` | `en_vVelocityLast`, `en_vLastDesiredVelocity` | the velocity and the desired velocity of the last step | medium |
| `SHovercraftPuppetEntity` | `en_vVelocityLast`, `en_tmLastJump`, `en_fLastError` | the velocity of the last step, the time of the last jump (`hpp_fMinJumpingPeriod` is the minimum between two) and the last spring error | medium |
| `SRollingBallPuppetEntity` | `en_fRadius`, `en_tmLastJump`, `en_hboSeat`, `en_fDesiredSeatHeadingEul`, `en_tmLastForceImpulse`, `en_fDesiredVelocity` | the ball radius, time of the last jump, the seat body, the seat heading the rider wants, the time of the last force impulse and the desired velocity (read by `GetDesiredMovingSpeed`) | medium |
| `SWheeledVehiclePuppetEntity` (not in the data) | `en_awdWheelData`, `en_ctWheels`, `en_ctDrivingWheels` | the wheel records (`CWheelData`), the number of wheels and of driving wheels | medium |
| `SDummyFoePuppetEntity` (not in the data) | `en_bImmortal`, `en_bFlying`, `en_fVelocity`, `en_slCustomHealth` | cannot die; flies (read: `MovingIn3DArea` returns it); constant velocity (read: `GetDeclaredNominalVelocity` returns it); health override | medium |
| `SDummyFoePuppetProperties` | `ep_baAligment`, `ep_bFlying`, `ep_fVelocity`, `ep_slHealth` | the placed values of the above (the spelling `Aligment` is the binary's) | medium |
| `SGenericLeggedPuppetEntity` / `Properties` | `en_pmcModel` / `ep_pmcModel`, `ep_vStretch`, `ep_glppPuppetParamsObsolete` | the model and its scale, an obsolete parameter link | medium |

### 2. Parameters of the models

All values and units are in `vehicles.md`; the member lists were verified against the declared tables (**high**): `SAircraftPuppetParams` (24 members, with `app_apcPlayerControls`), `SHovercraftPuppetParams` (18), `SRollingBallPuppetParams` (9: `rbpp_fMaxLinearVelocity`, `rbpp_fLinearAccel`, `rbpp_fMinJumpVelocity`, `rbpp_fMaxJumpVelocity`, `rbpp_fMaxValidSlopeForJump`, `rbpp_fSeatRotationVelocity`, `rbpp_slAccumulatedImpulseNonBraking`, `rbpp_fNonBrakingPeriodAfterImpulse`, `rbpp_bDiscardVelocityUponImpulse`), `STurretPuppetParams` (3: `tpp_fRotationSpeed`, `tpp_fGunAcceleration`, `tpp_fGunDecceleration`; the data stores the first as `.1`) and, **declared but unused**, `SWheeledVehiclePuppetParams` (5: `wwpp_fMaxSpeed`, `wwpp_fVehicleAcceleration`, `wwpp_fVehicleDecceleration`, `wwpp_fWheelSteeringTorque`, `wwpp_fWheelSteeringSpeed`). How the methods use them: `GetDeclaredNominalVelocity` (the maximum linear velocity, the front velocity limit, the forward velocity limit or the maximum speed), `GetCurrentAcceleration` and `GetCurrentDeceleration` (the matching acceleration members; the three warnings `<Kind> puppet %1 has invalid parameters.` appear when the file is not of the model's class) were profiled by the members they read (**medium**).

### 3. Seats and rides (shared by all models)

Rides use three records. **`SPuppetSeatData`** (`CPuppetSeatData`, 11 members, 22 `.ep` files; stored in the list `pp_csoSeats` of `SPuppetParams`): `psd_idSeatName`, `psd_idSeatAttachment`, `psd_idOperatedFromSeat`, `psd_sfSeatFlags` (`SeatFlags`: `SF_RIDER_CAN_SHOOT_OWN_WEAPON` 1, `SF_WAY_POINT_SEAT` 2, `SF_PUPPETS_CANT_SEAT` 4 ("disabled for non-players"), `SF_RIDER_DIES_WITH_RIDE` 8, `SF_PLAYER_CANT_SEAT` 16, `SF_PLAYER_CAN_BE_DAMAGED` 32, `SF_NON_PLAYER_IS_INVOULNERABLE` 64, `SF_PLAYER_CANT_EXIT` 128), `psd_idRiderMechanism`, the sitting animation (`psd_idSittingAnim` in the data, `psd_idRiderAnimRideNotMoving` in the binary: the same member), `psd_idRiderAnimRideMoving`, `psd_idApproachFromSeat`, `psd_idRidePartToAttachTo`, `psd_bRiderCanTurnHead` and the list of ways to the seat `psd_csoWaysToSeat`. **`SWayToSeat`** (3 members, 21 files): `wts_idEnterPointAttachment`, `wts_csoEnteringAnimSequence` and `wts_csoExitingAnimSequence` (lists of **`SRideAndRiderAnim`**: `rra_idRiderAnim`, `rra_idRideAnim`, `rra_bLerpRiderPlacement`, `rra_bPlayReversed`; 4 members, 21 files). The editor name of a seat (`mdGetScriptName`) is the seat name plus ` (NA)` when non-players cannot use it or ` (WP)` for a way-point seat (profile strings). The riding state of a puppet is `RidingState` (`RS_NOT_RIDING` ... `RS_EXITING`, `puppet-classes.md` section 3.3) and the brain's own phases `PRS_*`. **Medium**; the rules of choosing a seat are in [`puppet-carry-rides.md`](puppet-carry-rides.md) 2 (`FindBestSeat`, T01-144; `GetWayToSeat` and its error `Can't find way to sit in ride.` were not read).

## Invariants

1. Every placed aircraft, turret, hovercraft and rolling ball is a Character entity of its class and names a parameter file of the matching class (**high**, `wld-entities.md`: 503 entities of the four classes, all 503 name one of the 40 files).
2. No world contains a wheeled-vehicle, dummy-foe or generic-legged entity and no parameter file is of the wheeled-vehicle class (script).
3. The declared member lists of the five data model classes equal the stored ones (script).

## Open questions

1. **Movement steps** of the aircraft, hovercraft, rolling ball, turret and wheeled vehicle (the formulas that turn the parameters into motion): T01-142.
2. **The wheeled vehicle**: whether any shipped content ever used it (none found in the data) and its rules: T01-142 (low value; parked with the movement steps).
3. **Seats, rides and the AI**: T01-144 and T01-128.
4. **The dummy foe**: who creates one and what its built-in file contains (not in the install): T01-150.
5. **Run-time checks** of the models (top speeds, turret turn rates, ball torque): T01-146.

## Test vectors

- **Class to model**: `STurretCharacterEntity` with `STurretPuppetParams` ok; with `SAircraftPuppetParams` -> warning, no parameters.
- **Turret limits** (data): `tpp_fRotationSpeed` 180 (13 files) or 300 (`turret_machinegun`), `tpp_fGunAcceleration` = `tpp_fGunDecceleration` = 400 in all 14 files.
- **Placement counts** (`vehicles.md`): 503 entities, 37 of 59 worlds; 254 aircraft, 164 turrets, 44 rolling balls, 41 hovercraft.
- **Seat flags**: a seat with `SF_PLAYER_CANT_SEAT | SF_PUPPETS_CANT_SEAT` = 20 is for nobody; `SF_WAY_POINT_SEAT` = 2 marks an intermediate seat.

## The methods

Columns: method; what it does; **Basis** (`R` the function was decompiled and read, `P` strings and callees listed, `N` the name only).

### `CRollingBallPuppetEntity` (Rolling ball, 21 methods)

| Method | What it does | Basis |
|---|---|---|
| `AdjustSeatForRiderEntering` | Moves the seat joint for a rider that enters (turns the seat to the entry side). | N |
| `ApplyClientPositionCorrection` | Passes to the base (a ball is corrected like any puppet; name). | P |
| `EnforcePuppetMoveLook` | Applies the desired move and look (does nothing of its own: the ball is driven by torque). | N |
| `GetAbsPlacement` | The placement of the ball body (base for the seat). | P |
| `GetCurrentAcceleration` | The linear acceleration `rbpp_fLinearAccel` (warning `Rolling ball puppet %1 has invalid parameters` when the parameters are not the rolling-ball class). | P |
| `GetCurrentDeceleration` | The same value. | P |
| `GetDeclaredNominalVelocity` | The maximum linear velocity `rbpp_fMaxLinearVelocity` (same warning). | P |
| `GetDesiredMovingSpeed` | Reads the desired velocity member (`en_fDesiredVelocity`). | R |
| `GetPuppetMovingSoundRatio` | The rolling sound strength from the ball's velocity (329 bytes). | N |
| `GetPuppetParamsClass` | Answers `CRollingBallPuppetParams`. | P |
| `GetRollingBallRadius` | The radius of the first round hull of the body (default 1; half of the hull's size otherwise). | R |
| `GetTorque` | The torque for a desired acceleration: zero before the minimum time since the last force impulse (`rbpp_fNonBrakingPeriodAfterImpulse`), otherwise along the look direction or the current velocity direction, scaled by the body mass and the ball radius (826 bytes; the formula is not written down). | R |
| `IsRideFixed` | Answers yes (a ball is never broken; name). | R |
| `LerpToDesiredVelocityAndLook` | The ball's movement step: applies torque from `GetTorque` to the angular-motion joint towards the desired velocity (error `Rolling ball puppet %1 has invalid parameters`). | P |
| `MovingIn3DArea` | No. | R |
| `OnRiderStartedEntering` | A rider enters: reads the seated rider (turns the ball seat towards the entry). | P |
| `OnRiderStartedExiting` | A rider leaves. | P |
| `OnStep` | The base step plus the rolling-ball part (jump, seat heading `rbpp_fSeatRotationVelocity`, time of the last impulse). | P |
| `ReceiveForceImpulse` | Applies the impulse to the ball unless the player brain disables impulses (`rbpp_bDiscardVelocityUponImpulse`, `rbpp_slAccumulatedImpulseNonBraking`). | P |
| `ResetDynamicMembers` | Base reset plus the ball's members. | P |
| `SetCurrentMechanism` | Base switch plus the ball setup: the root joint must be an angular-motion joint (error `... root joint doesn't exist or is not of CAngularMotionJoint class`), the part needs a hybrid body with a ball joint (`should have part with hybrid body and ball joint`). | P |

### `CTurretPuppetEntity` (Turret, 18 methods)

| Method | What it does | Basis |
|---|---|---|
| `AdjustClientOrientation` | Corrects a client copy's gun orientation (name). | N |
| `AttachToParent` | Attaches the turret to the parent entity's mechanism (errors `Cannot attach turret (%1) to parent (%2), parent doesn't provide a mechanism` and `only turrets with valid mechanism and joint in root part can be attached`). | R |
| `CollectForSortedIteration` | Hook for the sorted entity iteration (name). | N |
| `GetCurrentLookDirEul` | The gun's current aim as Euler angles (from the two gun joints). | N |
| `GetDeclaredNominalVelocity` | 0 (a turret does not move). | R |
| `GetLookRotationVelocity` | The gun turning speed `tpp_fRotationSpeed`. | P |
| `GetPuppetParamsClass` | Answers `CTurretPuppetParams`. | P |
| `GetShootDirection` | The aim direction of a shot from the gun joints (joint names `GunLR` and `GunUD`). | P |
| `LerpToDesiredVelocityAndLook` | The gun step: turns the two joints (`GunLR` left-right, `GunUD` up-down) towards the desired look with `tpp_fGunAcceleration` and `tpp_fGunDecceleration`, limited by `tpe_fUpJointLimitEul` and `tpe_fDnJointLimitEul` (error `Turret puppet has invalid parameters`). | P |
| `OnBoot` | Base boot, then attaches to the parent when `tpe_henParent` is set. | P |
| `OnCreate` | Base create. | P |
| `OnStep` | Empty (the gun is driven by the movement step). | R |
| `OnUpdate` | Base update (properties changed). | P |
| `ResetDynamicMembers` | Base reset. | P |
| `SetCurrentMechanism` | Base switch, then re-attaches to the parent. | P |
| `SetDesiredTempoAndLookDir` | Base call (a turret ignores the velocity). | P |
| `ShouldExamineThreatSound` | No: turrets do not turn towards sounds. | R |
| `macSetParent` | Script: sets the parent entity and attaches. | P |

### `CAircraftPuppetEntity` (Aircraft, 16 methods)

| Method | What it does | Basis |
|---|---|---|
| `CanRotateInPlace` | True for a puppet whose parameters declare angular velocities (warning `Aircraft puppet %1 has invalid parameters`). | P |
| `EnforcePuppetMoveLook` | Applies the desired move and look (shared with the rolling ball and wheeled vehicle: no code of its own). | N |
| `GetCurrentAcceleration` | The front acceleration `app_fFrontAcceleration` (same warning). | P |
| `GetCurrentDeceleration` | The front deceleration `app_fFrontDeceleration`. | P |
| `GetDeclaredNominalVelocity` | The front velocity limit `app_fMaxFrontVel`. | P |
| `GetDesiredMovingSpeed` | The desired speed (name). | N |
| `GetDipContent` | `DC_AIR` (an aircraft is always in the air). | R |
| `GetFrontDirEul` | The front direction (flattened into the horizontal plane when `app_bFrontDirInXZPlane`). | P |
| `GetMaxAxesVelocities` | The three per-axis velocity limits (`app_fMaxFrontVel`, `app_fMaxUpVel`, `app_fMaxRightVel`) times the velocity multiplier. | P |
| `GetPuppetParamsClass` | Answers `CAircraftPuppetParams`. | P |
| `LerpToDesiredVelocityAndLook` | The flight step: per-axis acceleration and deceleration towards the desired velocity, angular acceleration and velocity limits for heading, pitch and bank, and the leaning (`app_bLeanPWhenMoving...`, `app_fMaxLeanP`, `app_fMaxLeanB`, air resistance for leaning). | P |
| `MovingIn3DArea` | Yes. | R |
| `OnCreate` | Base create. | P |
| `OnStep` | Empty (the movement step does the work). | R |
| `ResetDynamicMembers` | Base reset plus the last velocity members. | P |
| `SetCurrentMechanism` | Base switch plus the flight setup (error `Aircraft puppet model (...) mechanism (...) ...`). | P |

### `CHovercraftPuppetEntity` (Hovercraft, 13 methods)

| Method | What it does | Basis |
|---|---|---|
| `CanRotateInPlace` | No. | R |
| `EnforcePuppetMoveLook` | Applies the desired move and look to the hover joint. | N |
| `GetCurrentAcceleration` | The front acceleration `hpp_fFrontAcceleration` (warning `Hovercraft puppet %1 has invalid parameters`). | P |
| `GetCurrentDeceleration` | The front deceleration `hpp_fFrontDecceleration`. | P |
| `GetDeclaredNominalVelocity` | The forward velocity limit `hpp_fMaxVelForward`. | P |
| `GetPuppetParamsClass` | Answers `CHovercraftPuppetParams`. | P |
| `LerpToDesiredVelocityAndLook` | The hover step: drives the main hover joint (error `Can't find main hovercraft joint`) with the forward, side and heading limits, banking, spring and damping and jumping. | P |
| `MovingIn3DArea` | No. | R |
| `OnCreate` | Base create. | P |
| `OnStep` | Base step, then throws the riders out when the craft is upside down or similar (`FallOutAllRidersFromRide`; name-level). | P |
| `PostReceiveUpdate` | Base net update plus the hover joint state. | P |
| `ResetDynamicMembers` | Base reset plus the jump members. | P |
| `SetCurrentMechanism` | Base switch plus the hover setup (error `... root joint doesn't exist or is not of CHovercraftJoint class`). | P |

### `CWheeledVehiclePuppetEntity` (Wheeled vehicle, 13 methods)

| Method | What it does | Basis |
|---|---|---|
| `DebugDumpAllWheelsData` | Prints the number of driving and steering wheels and every wheel (debug). | P |
| `DebugDumpWheelData` | Prints one wheel: driving or steering, inverted (debug). | P |
| `EnforcePuppetMoveLook` | Shared with the aircraft and rolling ball. | N |
| `GetCurrentAcceleration` | The vehicle acceleration `wwpp_fVehicleAcceleration` (warning `Wheeled vehicle puppet %1 has invalid parameters`). | P |
| `GetCurrentDeceleration` | The deceleration `wwpp_fVehicleDecceleration`. | P |
| `GetDeclaredNominalVelocity` | The maximum speed `wwpp_fMaxSpeed`. | P |
| `GetPuppetParamsClass` | Answers `CWheeledVehiclePuppetParams`. | P |
| `LerpToDesiredVelocityAndLook` | Drives the wheels towards the desired speed and steering (`wwpp_fWheelSteeringTorque`, `wwpp_fWheelSteeringSpeed`). | P |
| `MovingIn3DArea` | No. | R |
| `OnStep` | Base step, then throws riders out when the vehicle is wrecked (name-level). | P |
| `ReceiveForceImpulse` | Applies an impulse unless the player brain disables them. | P |
| `SetCurrentMechanism` | Base switch. | P |
| `SetupWheels` | Finds the wheel hulls and builds the wheel data (errors `Can't find wheel hull!`, `Wheel hull is not a round primitive!`). | P |

### `CDummyFoePuppetEntity` (Dummy foe, 13 methods)

| Method | What it does | Basis |
|---|---|---|
| `CallShutDown` | Calls `OnShutDown` (used by the death sequence). | R |
| `DoDeathSequence` | Calls `CallShutDown` at once: a dummy foe has no death animation. | P |
| `GetCurrentAcceleration` | A constant (value not read; the three movement getters are 7-byte functions). | R |
| `GetCurrentDeceleration` | A constant (value not read; shares code with the acceleration). | R |
| `GetDeclaredNominalVelocity` | Returns the velocity member `en_fVelocity` (read: one load). | R |
| `GetPuppetModel` | The fixed model `Content/SeriousSam2/Models/Enemies/DummyFoe/DummyFoe.mdl`. | P |
| `GetVelocity` | The velocity member `en_fVelocity` along the heading when it is not zero (name and size). | N |
| `LerpToDesiredVelocityAndLook` | A short movement step (540 bytes). | N |
| `MovingIn3DArea` | Returns `en_bFlying` (read: one load). | R |
| `OnBoot` | Base boot. | P |
| `OnCreate` | Copies the four properties (`ep_baAligment`, `ep_bFlying`, `ep_fVelocity`, `ep_slHealth`) to `en_baAlignment`, `en_bFlying`, `en_fVelocity` and `en_slCustomHealth`, runs the base create and loads the built-in parameter file `Puppets/Objects/DummyFoe.ep` when no parameters are set; **that file and the model are not in the install**. | R |
| `OnReceiveDamage` | Base damage unless the dummy foe is immortal (`en_bImmortal`; name-level). | P |
| `SetCurrentMechanism` | Base switch with a dummy body. | P |

### `CGenericLeggedPuppetEntity` (Generic legged, 2 methods)

| Method | What it does | Basis |
|---|---|---|
| `GetPuppetModel` | The model of the generic properties (`ep_pmcModel`, member `en_pmcModel`) when set, else the base choice. | P |
| `OnCreate` | Runs the legged create with the model and stretch of the properties (`ep_pmcModel`, `ep_vStretch`; the obsolete `ep_glppPuppetParamsObsolete` is ignored) and loads the file `...GenericLegged...` parameters. | P |

### `CGenericLeggedPuppetProperties` (Generic legged properties, 1 methods)

| Method | What it does | Basis |
|---|---|---|
| `GetBoundingBox` | The bounding box from the model of the properties, for the editor. | N |

### `CPuppetProperties` (Puppet properties (editor and spawner links), 9 methods)

| Method | What it does | Basis |
|---|---|---|
| `BreakMultiSpawnerLinks` | Breaks the links to multi-spawners. | R |
| `BreakSpawnerLinks` | Breaks the links to spawners. | N |
| `GetBoundingBox` | The bounding box of the model for the editor. | N |
| `GetEffectiveEntity` | The entity that stands in for these properties (the puppet). | N |
| `GetExecutionCommand_EDT` | Editor: the command `Command.SwitchToEditor` of the mesh editor. | P |
| `GetModelRenderable` | The renderable of the preview. | N |
| `GetSpawneeCountLimitType` | How a spawner counts the spawnee (`SpawneeCountLimitType`). | N |
| `IsMultiSpawnerTemplate` | Returns `ep_bMultiSpawnerTemplate`. | R |
| `MarkAsSpawnee` | Marks the properties as spawned (`ep_bIsSpawned`). | R |

### `CPuppetSeatData` (Seat data, 2 methods)

| Method | What it does | Basis |
|---|---|---|
| `GetWayToSeat` | Chooses one of the ways to the seat by index (error `Can't find way to sit in ride.`). | P |
| `mdGetScriptName` | The editor name of the seat: the seat name with the suffix ` (NA)` when not for non-players or ` (WP)` for a way-point seat. | P |

### `CMovingAnimData` (Moving-animation selectors, 2 methods)

| Method | What it does | Basis |
|---|---|---|
| `GetMovingAnimDataDescription` | Builds the text of the six selectors (`CharacterEmotion`, `CharacterShieldStatus`, `MovingAnimationDirection`, `MovingAnimationPose`, `MovingAnimationReadiness`, `MovingAnimationSpeed`) for debug display. | P |
| `IsEqualTo` | Compares two animation-selector records. | N |

### `CAllMovingAnimation` (All-body moving animation, 1 methods)

| Method | What it does | Basis |
|---|---|---|
| `mdGetName` | The editor caption (adds ` (scheme)`). | P |

### `CBodyMovingAnimation` (Body moving animation, 1 methods)

| Method | What it does | Basis |
|---|---|---|
| `mdGetName` | The editor caption. | N |

### `CPoseTransitionAnimation` (Pose transition animation, 1 methods)

| Method | What it does | Basis |
|---|---|---|
| `mdGetName` | The editor caption. | P |

### `CLerpToAnimAdjuster` (Lerp-to-animation adjuster, 6 methods)

| Method | What it does | Basis |
|---|---|---|
| `Evaluate` | Blends the model's pose from where it was to where the animation puts it, over the fade-in time (2.8 kilobytes). | N |
| `RecordPositions` | Records the bone positions at the start. | N |
| `SetFadeInTime` | Sets the fade-in time. | N |
| `SetNewOrigin` | Sets the new origin. | N |
| `SetOldOrigin` | Sets the old origin. | N |
| `SetRootBone` | Sets the root bone. | N |

### `CSpineAdjuster` (Spine adjuster, 7 methods)

| Method | What it does | Basis |
|---|---|---|
| `Activate` | Turns the adjuster on. | R |
| `AddBone` | Adds a bone with its share of the angle. | N |
| `Deactivate` | Turns it off. | R |
| `Evaluate` | Applies the angle to all bones (position part). | P |
| `EvaluateRotation` | Applies the rotation part. | N |
| `GetBoneTransformMatrix` | The transform of one bone. | N |
| `SetAngle` | Sets the angle the spine bends by. | N |

