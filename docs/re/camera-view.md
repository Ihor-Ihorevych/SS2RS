# Camera, field of view and the first-person weapon view (T01-042)

- **Roadmap task:** T01-042 (feeds the renderer and player tasks that set up the view)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what is stored: the 18-camera database with its graphs, the camera names every puppet uses, the weapon view placement of the 16 weapons, the player's view parameters and the sniper zoom (every file read with `tools/re/camera_verify.py`; members from the files' own type tables); **medium** for the roles and units of numbers that only have numbers or short names; **low** for the run-time rules: how the engine turns a camera graph into a view, how the field of view and the weapon field of view combine, and whether the game moves the view while walking (**no head bob or sway constants exist in the data**). Scope: `Cameras.rsc`, all `.ep` files, all worlds' camera entities by reference to `cutscenes.md`.

Prerequisites: `rsc-ctl.md` (the `.rsc` container and the graph classes), `ep.md` and `ep-schema.md` (puppet parameters), `player-rules.md` (the player's numbers), `vehicles.md` (which vehicle uses which camera), `cutscenes.md` (camera entities), `environment.md` (camera shakers).

## Summary

- **First person is the default view.** The player's field of view is **80 degrees** (`CGlobalPlayerParams.gpp_fFOV`, the only value; `player-rules.md`). The weapon in the player's hand is drawn with **its own field of view and placement per weapon**: 16 weapon rendering files give a position, an angle and a field of view (0.531 to 1.228, probably radians: **30.4 to 70.4 degrees**) for holding one weapon and a second position for holding two.
- **Third-person and vehicle cameras are data, not code constants.** `Databases/Cameras.rsc` holds **18 named cameras**; each gives how far **behind** and how far **above** its target the camera sits **as a function of the pitch angle** (two graphs) and the allowed pitch range. Puppets choose a camera by name for three cases: ridden by a player, third person, third-person death (`SCameras`).
- **Camera effects** are separate: `SShakerEffectEntity` and destruction shakers (`environment.md`, `destruction.md`) and the player's view damping (below). **No head bob, weapon sway or walking camera motion constants are stored**; if the original has them they are in code (task T01-073).

## Evidence

`tools/re/camera_verify.py` (read-only, `SS2_DATA_DIR`; about 3 minutes) over the retail install with all patches; the tables are `camera-tables.md` (generated):

- **Cameras:** 18 `CCameraHolder` → `CCameraControl`, 18 distinct names; their graphs are 17 constant (`CValueGraph`) and 19 piecewise-linear (`CLinearGraph`) (a constant graph has one control point).
- **Puppet usage:** each of the 126 puppet parameter files has the three camera slots; the ride camera slot names (108 `3rdPersonRideCamera`, 7 `HoverFighterCamera`, 4 `TurretsCamera`, 2 `HelicopterCamera`, one each `Dino_M_Camera`, `Dino_S_Camera`, `HoverFighterCameraPyramid`, `FlyingSaucer`, `RollingBallCamera`); the third-person slot: 121 `3rdPersonCamera`, 2 `3rdPersonCamera_Close`, 2 `HelicopterCamera`, 1 `RollingBallCamera`; the death slot: **123 `3rdPersonDeathCamera`, which is not in the database** (two `HelicopterCamera`, one `RollingBallCamera`). So the death camera is built by the game or comes from another source.
- **Weapon view:** 16 of 16 files have the same member for the field of view for one weapon (`.3`) and for two (`bwrp_fFOVDoubleWeapon`); the angle for two weapons equals the angle for one in 15 of 16.
- **Player:** the four player parameter files (`player.ep`, `playerpuppet.ep`, `playerpuppet_dm.ep`, `santa.ep`) have identical view values.

## The camera database

```
CCameraDatabase.cd_cchCameraHolders   list of 18 CCameraHolder
CCameraHolder.ch_pccCameraControl     -> CCameraControl
CCameraControl
  cc_idName                    IDENT   the camera name (compare as strings)
  cc_pgrBehindDistanceOverPitch  -> CGraph   distance behind the target for a pitch angle (x: degrees, y: distance)
  cc_pgrAboveDistanceOverPitch   -> CGraph   height above the target for a pitch angle
  cc_fMinPitch, cc_fMaxPitch   FLOAT   -90, -45 or -30 .. 30 to 70 degrees
```

A `CValueGraph` is a constant (its one point and `CGraph.12` give the value); a `CLinearGraph` interpolates linearly between its control points (`rsc-ctl.md`). Examples from `camera-tables.md`:

| Camera | Pitch range | Behind | Above |
|---|---|---|---|
| `3rdPersonCamera` | −90 .. 70 | bell shaped over pitch: 0 at ±90°, 0.05 at ±45°, 0.4 at ±30°, 0.8 at ±15°, about 1.03 at 0° | 1.5 at ±90°, about 1.87 at 0° |
| `3rdPersonCamera_Close` | −90 .. 70 | constant 4 | constant 1.424 |
| `3rdPersonRideCamera` | −30 .. 35 | constant 5.85 | constant 1.646 |
| `HoverFighterCamera` | −45 .. 45 | constant 5 | constant 0.5 |
| `HelicopterCamera` | −45 .. 45 | about 23 → 22.3 (0° → 84°) | about 8.5 → 8.1 |
| `TurretsCamera` | −30 .. 45 | constant 4.68 | 2.066 at 0°, 2.3 at 30° |
| `RollingBallCamera` | −45 .. 40 | constant 6.57 | constant 2.3 |
| `Transformer_Camera` | −45 .. 45 | constant 22 | constant 4 |

The units of the distances are not named (the magnitudes fit metres: a camera 5.85 behind and 1.65 above a vehicle). The ride cameras of vehicles are described per vehicle in `vehicles.md`; seven cameras are named by no `.ep` file of the retail data (`3rdPersonStrafeRideCamera`, `3rdPersonRideCameraAlternate`, `HoverFighter2Camera`, `HoverFighter3Camera`, `HoverBikeCamera`, `CorrectedHoverBikeCamera`, `Transformer_Camera`): game code or scripts may select them, or they are unused.

## The player's view

- **Field of view:** `CGlobalPlayerParams.gpp_fFOV` **80** (degrees, the horizontal field of view per `player-rules.md`); the AI's `ggp_fCharacterFOV` of 360 is the sight cone of characters (`ai.md`), not a camera.
- **`SPlayerPuppetParams` view members** (identical in all four files): `ppp_fAddedPitchDeg` 10 (an added pitch), `ppp_fViewDampingK` **160** and `ppp_fViewDampingD` **10** (a damped spring on the view: stiffness and damping), `ppp_fMaxMoveBanking` 0.25 and `ppp_fMaxSideStepBanking` 1 (the view rolls when moving or side-stepping, probably in degrees), `ppp_fViewAutoCenterDelay` 0.5 s and `ppp_fViewAutoCenterMultiplier` 3 (the view returns to the centre after a delay). View heights (crouch 0.7, swim 1.5, dive 1.1) and the breath timer are in `player-rules.md`.
- **Sniper zoom:** `SSniperWeaponParams`: `swp_fMinZoomFactor` **2**, `swp_fMaxZoomFactor` **4** (6 in the deathmatch weapon), `swp_fFullZoomingTime` **0.8 s** (1 s in deathmatch) and `swp_slBulletDamageInZoomMode` 150 (300 in deathmatch): zooming from 2× to the maximum takes the full zooming time (reading; the interpolation is not stated).
- **Camera offset for operators:** `op_fCameraOriginOffsetY` 0, 1.2 or 2 on vehicles and turrets (`vehicles.md`).

## The weapon in the first-person view

Each of the 16 weapons has a rendering parameter file (`EntityParams/<weapon>weaponrendering.ep`: autoshotgun, beamgun, cannon, circular saw, Colt, double shotgun, flamer, grenade launcher, Klodovik gun, minigun, plasma rifle, rocket launcher, Serious Bomb, sniper, Uzi, zap gun) with the members of `SBaseWeaponRenderingParams`:

| Member | Meaning (observed) | Range |
|---|---|---|
| `.1` (`Vector3f`) | position of the weapon model relative to the camera for one weapon | x 0.09 to 0.28, y −0.59 to −0.16, z −0.77 to 0.08 |
| `.2` (`Vector3f`) | angle of the model (three values; the third is 0 in all) | −7.1 to 24 degrees (first two values) |
| `.3` | field of view used to draw the weapon (a separate projection) | 0.531 to 1.228 (30.4° to 70.4° if radians) |
| `bwrp_vPositionDoubleWeapon`, `bwrp_vAngleDoubleWeapon`, `bwrp_fFOVDoubleWeapon` | the same for holding **two** weapons (the second one at x larger by up to 0.13; equal for the Colt, the Uzi and the Serious Bomb) | |
| `.5` | probably a scale (0.4 to 1) | 0.4 .. 1 |
| `.4` | 0.1 in all files; role unknown (a near distance is likely) | 0.1 |

Examples: `sniper` position (0.136, −0.372, −0.767), angle (1.8, 1.4, 0), field of view 0.571 (32.7°); `minigun` (0.277, −0.277, −0.744), 0.928 (53.2°); `zapgun` z +0.082 (it sits closer to the camera), 0.531 (30.4°); `beamgun` 1.228 (70.4°). The full table is in `camera-tables.md`. The weapon **model, animations and tool placement** come from the `.ctl` and `.mdl` files (`rsc-ctl.md`, `mdl.md`).

## Cutscene cameras

In-engine cutscenes use **camera entities** that play a camera animation with position, rotation, field of view and aspect channels (`cutscenes.md`, `ans.md`); they are started by scripts (`level-logic.md`) and replace the player's view while they play.

## Reader invariants

1. Use the **camera database** for third-person and ride cameras: look up the camera by its string name (identifiers are per-file indices: compare strings); clamp the pitch to the camera's range; sample both graphs at the pitch (linear interpolation between points, constant for a value graph, clamp outside the range).
2. The first-person field of view is 80 degrees; draw the weapon with the weapon's own field of view and placement, using the two-weapon values when two weapons are held.
3. The death camera `3rdPersonDeathCamera` is **not** a database entry: handle a missing name as a request for the default death view (to be defined by the game rules).
4. Apply the view spring (stiffness 160, damping 10), the banking limits and the auto-centre rule only as parameters of the player; there is no stored head bob or sway.
5. Fail on a camera graph class other than `CValueGraph` and `CLinearGraph`; the step graphs of `rsc-ctl.md` do not occur in the camera database.

## Test vector

`Databases/Cameras.rsc`: `3rdPersonRideCamera` — pitch −30 to 35, behind constant 5.85, above constant 1.646. `3rdPersonCamera` — pitch −90 to 70; behind points (−90, 0), (−45, 0.05), (−30, 0.4), (−15, 0.8), (≈0.12, ≈1.027), (15, 0.8), (30, 0.4), (45, 0.05), (90, 0); above points (−90, 1.5), (≈−0.016, ≈1.870), (90, 1.5). `EntityParams/sniperweaponrendering.ep`: position (0.136, −0.372, −0.767), angle (1.8, 1.4, 0), field of view 0.571. `EntityParams/playerpuppet.ep`: view damping 160 and 10, banking 0.25 and 1, auto-centre 0.5 s and 3, added pitch 10.

## Open questions

- **Run-time rules** (task T01-073): the actual projection set-up (is 80° horizontal or vertical, how the weapon field of view is applied and whether the weapon uses its own near plane), how the spring and banking act, **head bob, sway, recoil and idle motion** (absent from the data), the death camera, and the zoom interpolation: measurement (T01-038/039) and the game module.
- The units and roles of `SBaseWeaponRenderingParams.4` and `.5` and whether `.3` is in radians (the range suggests it).
- Which code or script selects the seven cameras that no `.ep` file names (listed above).
