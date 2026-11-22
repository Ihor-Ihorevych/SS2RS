# Vehicles, turrets and flying characters (T01-047)

- **Roadmap task:** T01-047 (feeds the player/AI/physics tasks that move these characters)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for which classes exist, their parameter values, which files the worlds place and how often (all 126 puppet parameter files and all 59 worlds, `tools/re/vehicles_verify.py`; members from the files' own type tables); **medium** for the roles of the numbers (the names are in the data, the units are not); **low** for the run-time rules: how a rider enters a ride, how the operator's input drives the movement, collision and weapon behaviour. Scope: 40 parameter files, 503 placed entities.

Prerequisites: `ep.md` (the puppet parameter file), `puppet-stats.md` (health and score), `ai.md` (brains: flyers use altitude graphs), `wld-entities.md` (the entity records), `level-logic.md` and `level-logic-api.md` (rider events and calls), `player-rules.md` (cameras).

## Summary

- **There are no separate vehicle classes of the world.** A vehicle, a turret and a flying enemy are all **puppet characters**: a `SPuppetParams` file (`.ep`) whose root class chooses the movement model, placed by an entity of the matching class. Four movement models exist beyond the walking `SLeggedPuppetParams`:

| Parameter class | Placed entity class | Files | Placed (worlds) | Model |
|---|---|---|---|---|
| `SAircraftPuppetParams` | `SAircraftCharacterEntity` | 19 | 254 | free flight with leaning: helicopters, fighters, bombers, flying enemies |
| `STurretPuppetParams` | `STurretCharacterEntity` | 14 | 164 | a fixed gun that turns |
| `SHovercraftPuppetParams` | `SHovercraftCharacterEntity` | 5 | 41 | spring-suspended ground hover with jumps |
| `SRollingBallPuppetParams` | `SRollingBallCharacterEntity` | 2 | 44 | a rolling ball with a seat |

- **A player can ride a file when it has a ride icon** (`CMiscPupeptParams.mpp_ptxRideIcon`, set in 18 of the 40 files: 6 aircraft, 7 turrets, 4 hovercraft, 1 rolling ball). The same file serves as an **AI-driven** enemy or turret when no player rides it; the enemies and automatic turrets have no ride icon.
- Riders are tracked on the **placed entity**: `SPuppetEntity` has the rider list (`en_eaRiders`), the default riders (`en_eaDefaultRiders`), the ride a puppet sits in (`en_henRide`), the seat names (`en_idSeat`, `en_idCurrentSeat`), the operator (`en_henOperator`) and the way to the seat (`en_iWayToSeat`). Scripts react to `AllRidersSittingInRide`, `RiderDied` and `RiderLost`, and call `GetRide`, `GetRiderOnSeat` and `macMountRide`.

## Evidence

`tools/re/vehicles_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes) over the retail install with all patches:

- **Files:** 19 + 14 + 5 + 2 = 40 `.ep` files have one of the four root classes; the per-file table is `vehicles-table.md` (generated: health, ride icon, seat, ride camera, class numbers, number of entities and worlds).
- **Worlds:** 503 placed entities of these four classes, **all 503** name one of the 40 files; they occur in 37 of 59 worlds.
- **Mounted tools:** the mounted-tool lists (`pp_amtMountedTools`) of all 40 files are **empty**, and the generic mount handle of the turret probe is null: the weapons of vehicles are not stored as mounted tools in the parameter files (they come from the attack parameters and the model, `ai.md`).

## Aircraft: `SAircraftPuppetParams`

The 24 members are, by name, a **velocity and acceleration limit per axis**, **angular limits** and **lean** settings:

```
app_apcPlayerControls   SAircraftPlayerControls: five AircraftAction values (MoveLR, MoveFB, MoveUD, LookLR, LookFB): which player input drives which axis
app_fMaxFrontVel 15..100   app_fFrontAcceleration 10..500   app_fFrontDeceleration 10..500
app_fMaxUpVel 5..100       app_fUpAcceleration 5..400       app_fUpDeceleration 5..400
app_fMaxRightVel 1..200    app_fRightAcceleration 10..2000  app_fRightDeceleration 10..2000
app_fAngAccelH 500..2000   app_fAngAccelP 250..2000         app_fAngAccelB 500..2000
app_fAngVelH 20..240       app_fAngVelP 10..200             app_fAngVelB 10..200   (heading, pitch, bank, degrees per second)
app_bFrontDirInXZPlane (1 in all)   app_bSlowDownWhenTurning   app_bLeanPWhenMovingFB   app_fMaxLeanP 0..45
app_bLeanBWhenMovingFBRotatingH (1 in all)   app_fMaxLeanB 25..75   app_fAirResistanceForLeaningP 0 or 2   app_fAirResistanceForLeaningB 0 or 0.5
```

Examples (`vehicles-table.md`): the `helicopter` (health 380, front velocity 30, up 30, heading 80 per second; 94 placed in 17 worlds, the most used vehicle), `fighter01_mental` (300; front 50, side 1: a fighter that does not strafe; 33 in 13 worlds), `fighter02_mental` (300; front 100, acceleration 500; 21 in 7), `dropship_mental` (health 999,999: not meant to be killed), the player's `playerhelicopter` (300) and `airfighter_rockets` (200) and the four coloured `sirius_car05_*` (200; ride icon). Five aircraft files are **flying enemies** (`fly`, `kungfumaster`, `levitator_air`, `robotfloater`, `witch_flying`: health 28 to 140, no ride icon; their brains have altitude limits, `ai.md`).

## Turrets: `STurretPuppetParams`

Three members: `.1` is the turning speed or limit of the gun (**180** in 13 files, **300** in `turret_machinegun`), and `tpp_fGunAcceleration` and `tpp_fGunDecceleration` are **400** in all. Health ranges from 140 (`shooter_mask`, `spawnermask`) over 200 to 700 (`turret_cannondm`) and 10,000 (`kwongocatapult`, a boss). **Seven have a ride icon** (the player-operated `turret_cannon`, `turret_cannondm`, `turret_machinegun`, `turret_machinegun_ctf`, `turret_plasma` and the boss turrets `kwongo_machinegun` and `turret_crossbow`); the `autoturret_*` files (cannon, plasma, plasma laser, rocket: health 255; 118 placed in all, the rocket turret 72 of them in 10 worlds) and the two mask turrets are automatic. Four turret files use the ride camera `TurretsCamera` (the others `3rdPersonRideCamera`).

## Hovercraft and rolling ball

- **`SHovercraftPuppetParams`** (18 members): `hpp_fMaxVelForward` 9 to 20, `…Backward` 7 to 20, `…Side` 10 to 12, accelerations 10 to 50, a heading and bank acceleration, `hpp_fMaxLeanBanking` 5 or 15, a **spring suspension** (`hpp_fSpringCoef` 30 or 50, `hpp_fDampingCoef` 0.1 or 0.25) and **jumping** (`hpp_fMinJumpingPeriod` 0.5, `hpp_fMaxValidSlopeForJump` 70, jump velocity 5 to 20). Files: the player's `hoverfighter` (health 500; 19 placed in 6 worlds), its `_pyramidfight` and `_slowrate` variants, the `hoversaucer` (camera `FlyingSaucer`) and the enemy `levitator` (health 120, no ride icon).
- **`SRollingBallPuppetParams`** (9 members): maximum linear velocity 15, acceleration 7 or 15, jump velocity 10 to 20, a slope limit 60 for jumping, `rbpp_fSeatRotationVelocity` 360 and an impulse rule (`rbpp_slAccumulatedImpulseNonBraking` 70 or 100, `rbpp_fNonBrakingPeriodAfterImpulse` 0.1 or 0.25: after a hit the ball is not braked for a short time). Files: `rollingball` (325; 38 placed in 8 worlds) and `rollingball_player` (500, ride icon, camera `RollingBallCamera`).

## Seats, operators and cameras (shared by all four)

- `SOperatingParams` (`pp_opOperatingParams`): the seat identifier `op_idOperatorSeat` (`Seat` in 12 of the 40 files, `empty` in the others), a camera offset `op_fCameraOriginOffsetY` (0, 1.2 or 2), `op_bPreferesTurningControl` and `op_bRequiresIgnition` (0 in all).
- `SCameras` (`pp_cCameras`): a camera identifier for riding, for the normal third-person view and for death; the riding ones seen here are `3rdPersonRideCamera`, `HelicopterCamera`, `HoverFighterCamera`, `HoverFighterCameraPyramid`, `FlyingSaucer`, `TurretsCamera` and `RollingBallCamera` (the camera behaviours are defined in the camera data: `player-rules.md`, task T01-042).
- The effect, sound and attack parameters are the common puppet ones (`ep.md`).

## Reader invariants

1. Decide the movement model from the **root class of the `.ep` file** (aircraft, turret, hovercraft, rolling ball); the placed entity's class matches it in all 503 placed entities.
2. A file whose `mpp_ptxRideIcon` is set is **rideable by a player**; any file can also be driven by an AI brain (`ai.md`).
3. Use the numeric members as stored; units are not named in the data (velocities are probably metres per second and the angular values degrees per second, by the magnitudes and the model sizes; not verified).
4. Treat the rider bookkeeping as part of the **entity** state (`SPuppetEntity.en_*`), not the parameter file; scripts must be able to query a ride's rider on a seat.
5. Fail on a puppet root class that is not one of the five movement classes of `ep-schema.md`.

## Test vector

`puppets/vehicles/hoverfighter.ep`: root `SHovercraftPuppetParams`, health 500, ride icon set, seat `Seat`, ride camera `HoverFighterCamera`, forward 20, side 12, jump velocity 10, spring 50; placed 19 times in 6 worlds. `puppets/turrets/turret_machinegun.ep`: root `STurretPuppetParams`, health 500, ride icon set, camera `TurretsCamera`, `.1` 300, gun acceleration and deceleration 400; placed 4 times in 2 worlds. `puppets/vehicles/helicopter.ep`: root `SAircraftPuppetParams`, health 380, no ride icon, camera `HelicopterCamera`, front velocity 30, up 30, heading 80; placed 94 times in 17 worlds.

## Open questions

- **Run-time rules** (task T01-072): the exact meaning and units of the movement numbers, how input maps to the axes (`SAircraftPlayerControls` values), how a rider boards (`en_iWayToSeat`, `macMountRide`, the seat animation), what ends a ride (`RiderLost`), damage and collision of vehicles, how the player's weapon relates to a turret's: measurement (T01-038/039) and the game module.
- Whether the `.1` member of the turret parameters is a turning speed or an angle limit; the roles of `hpp_fMaxHVelocity` and `…BVelocity`.
- The 22 non-rideable files in the table (enemy flyers, automatic turrets, the enemy levitator, the plain rolling ball) use the same classes with a brain; which brains drive them is `ai.md`.
