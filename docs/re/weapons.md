# Weapons: parameters of the 16 weapons (T01-028)

- **Roadmap task:** T01-028 (feeds the player, projectile and balance tasks; per-weapon run-time behaviour needs the oracle capture T01-038)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what is stored: which files and classes define each weapon, the named members (bullet damage, pellet counts, zoom, sounds, models, tools), the projectile files each weapon fires and every single-player versus deathmatch difference (all 17 weapon parameter files and 12 projectile files, `tools/re/weapons_verify.py`; members from the files' own type tables); **medium** for the readings of numbered members (**the fire period, the spread and range members, and the second set of projectile numbers have no names**: the readings below follow the values and the pairs); **low** for run-time behaviour: ammo use, reloading, damage falloff, how the combo (dual wielding) works. Scope: `Databases/EntityParams/*weapon*.ep`, `Databases/Tools/PlayerWeapon_*.ctl` by reference.

Prerequisites: `ep.md` and `ep-schema.md` (the parameter file), `player-rules.md` (weapon items, ammo types, the cheat weapon set), `camera-view.md` (the weapon view model placement), `effects-pipeline.md` (hit effects), `rsc-ctl.md` (`.ctl` tools), `vehicles.md`.

## Summary

- **A weapon is two to four files.** For each weapon `<w>`: `EntityParams/<w>weapon.ep` (the parameters), `<w>weapon_dm.ep` (the **deathmatch** variant: same class, rebalanced numbers; 17 of 17 weapons have one), `<w>weaponrendering.ep` (first-person placement: `camera-view.md`) and the pickup `EntityParams/Items/Weapons/<w>item.ep` (`player-rules.md`). There are **16 weapons plus the circular saw's blade projectile weapon**: auto shotgun, beam gun, cannon, circular saw (and blades), Colt, double shotgun, flamer, grenade launcher, Klodovik gun, minigun, plasma rifle, rocket launcher, Serious Bomb, sniper, Uzi and zap gun; the pickup list also has a **combo weapons** item (dual wielding, no parameter file of its own).
- **Three kinds of weapon by their data:** **bullet weapons** (Colt, Uzi, minigun, auto and double shotgun, sniper, beam gun: damage and pellet counts are members of the weapon file), **projectile weapons** (cannon, grenade launcher, rocket launcher, plasma rifle, Klodovik gun, zap gun, flamer, circular saw blades: member `b.23` points to a **projectile parameter file** with the damage, radii and flight) and **special weapons** (circular saw: a cutting volume with engine and cutting sounds; Serious Bomb: no parameters beyond the base).
- **The base** (`SBaseWeaponParams`, 22 members shared by all) holds the model (`b.24`, a first-person `.mdl`), the **two tools** (`b.35` left, `b.36` right: `PlayerWeapon_<w>_L.ctl` and `_R.ctl`, the hand animations: `rsc-ctl.md`) and numbers; **`b.30` is, by its values, the time between shots** (below).
- Ammunition amounts, ammunition per shot, pickup quantities and magazine rules are **not in the weapon files** (`player-rules.md`: not in the data).

## Evidence

`tools/re/weapons_verify.py` (read-only, `SS2_DATA_DIR`; about 1 minute); the full member tables are `weapons-tables.md` (generated: every member of every weapon in single player and deathmatch, and every referenced projectile file):

- **Files:** 34 weapon files (17 weapons × single player and deathmatch), 12 projectile files; weapons that differ between the two modes: **12 of 17** (not the circular saw, its blades, the Klodovik gun, the Serious Bomb and the zap gun); members that differ: 0 to 7 per weapon; the flamer's deathmatch file has one extra member (`w.4`, the fire effect `Flamethower_Fire.pfx`, spelling as in the file).
- **Classes:** `SColtWeaponParams`, `SUziWeaponParams`, `SMiniGunWeaponParams`, `SSniperWeaponParams`, `SAutoShotgunWeaponParams`, `SDoubleShotgunWeaponParams`, `SBeamGunWeaponParams`, `SZapGunWeaponParams`, `SPlasmaRifleWeaponParams`, `SRocketLauncherWeaponParams`, `SGrenadeLauncherWeaponParams`, `SCannonWeaponParams`, `SKlodovikGunWeaponParams`, `CFlamerWeaponParams`, `SSeriousBombWeaponParams`, `SCircularSawWeaponParams` and `SCircularSawBladesWeaponParams`; the bullet weapons derive from **`SBaseScatterWeaponParams`** (a scatter pattern `bswp_vPatternStretch`, bullet impulse `bswp_fBulletImpulse`, −1 = use the default, and six numbered members `s.1` to `s.6`). Numbered members are written with the class level as a prefix: `b.N` is member N of `SBaseWeaponParams`, `s.N` of `SBaseScatterWeaponParams`, `w.N` of the weapon's own class, `p.N` of `SBaseProjectileParams` (the numbers repeat across levels).

## Bullet weapons

| Weapon | Damage per bullet (SP → DM) | Bullets per shot | Time between shots `.30` (SP → DM) | Other |
|---|---|---|---|---|
| Colt | 10 → 15 | 1 | 0.5 s | `b.5` is 6 (0 in all other weapons) |
| Uzi | 10 → 13 | 1 | 0.12 → 0.2 s | `b.34` 0.08 → 0.1 |
| Minigun | 10 → 12 | 1 | **0.035 → 0.1 s** | sounds: fire, click, spin, spin up, spin down |
| Auto shotgun | 12 → 10 | **7** (`aswp_ctBullets`) | 1.0 → 0.819 s | |
| Double shotgun | 12 → 7 | **14** (`dswp_ctBullets`) | 1.3 → 1.078 s | |
| Sniper | 75 → 25 (zoomed **300 → 150**) | 1 | 0 → 1.0 s | zoom 2× to 4× (6× DM) in 0.8 s (1 s DM): `camera-view.md` |
| Beam gun | `w.1` 14 → 7 | hitscan beam | 0.1 s | `w.4` 100 → 50, `b.20` 0.25 → 0.5, `b.21` 0.75 → 3; loop model `BeamLooping.mdl`, miss effect `Beam_NoHit.pfx` (both modes) |

- **Single player versus deathmatch.** Deathmatch lowers shotgun pellet damage (12 → 10 and 12 → 7) and the sniper's damage, raises Colt, Uzi and minigun damage; it **slows** the minigun (0.035 → 0.1 s), the Uzi (0.12 → 0.2 s), the sniper (0 → 1.0 s) and the rocket launcher (0.6 → 0.665 s) and **speeds up** the auto shotgun (1.0 → 0.819 s), the double shotgun (1.3 → 1.078 s) and the plasma rifle (0.15 → 0.1 s). The scatter members change by large factors: `s.1` (Uzi 100 → 500, minigun 200 → 750, both shotguns 100 → 500), `s.4` (Uzi 300 → 500, minigun 300 → 750, sniper 400 → 1,500, shotguns 200 → 500) and `s.6` (Uzi 2 → 15, minigun 1 → 22.5, auto shotgun 0.5 → 3, double shotgun 1 → 4.5): reading, the scatter spread and range grow, so deathmatch bullets spread more and reach farther (**unverified**).
- **Bullet damage type and impulse:** `bswp_fBulletImpulse` and `…UpRatio` are −1 in all (the default of the game rules); the damage type of a bullet is not stored in the weapon file (`bp_dtDirectDamageType` exists only for projectiles).

## Projectile weapons

`b.23` names a `SGenericProjectileParams` file (`CFlamerProjectileParams` for the flamer): the 12 files are `CannonBall`, `CannonBallDM`, `Blades`, `Flamer`, `Grenade`, `GrenadeDM`, `Klodovik`, `PlasmaRay`, `PlasmaRay_DM`, `Rocket`, `RocketDM`, `ZapGunProjectile`. Deathmatch swaps the file for the cannon, grenade launcher, plasma rifle and rocket launcher; the Klodovik gun, flamer, zap gun and blades share one file. Per file (`weapons-tables.md`):

| File | Speed `p.1` | Direct damage `p.2` / `p.11` | Range damage `p.3` / `p.12` | Radii `p.7` `p.8` / `p.13` `p.14` | Flight life `p.5` |
|---|---|---|---|---|---|
| `Rocket` | 60 | 50 / 0 | 80 / 0 | 4, 10 / 8, 16 | 5 |
| `RocketDM` | 45 | 50 / 0 | 50 / 0 | 4, 8 / 4, 8 | 5 |
| `Grenade` | 30 | 35 / 0 | 100 / 0 | 4, 8 / 8, 16 | 2.5 |
| `GrenadeDM` | 30 | 75 / 75 | 100 / 100 | 4, 8 / 4, 8 | 3 |
| `CannonBall` | 0 | 600 / 700 | 100 / 100 | 2, 8 / 2, 8 | 3.5 |
| `CannonBallDM` | 0 | 0 / 750 | 25 / 25 | 2, 16 / 2, 16 | 5 |
| `PlasmaRay` | 100 | 35 / 35 | 0 / 0 | 0 | 2.5 |
| `PlasmaRay_DM` | 100 | 15 / 15 | 0 / 0 | 0 | 2.5 |
| `Klodovik` | 25 | 600 / 0 | 100 / 0 | 8, 16 / 8, 16 | 6 |
| `ZapGunProjectile` | 75 | 14 / 80 | 0 / 0 | 8, 16 / 8, 16 | 5 |
| `Flamer` | 30 | 6 / 6 | 0 | 8, 16 / 8, 16 | 1 |
| `Blades` | 100 | 50 / 0 | 0 | 0, 0 / 8, 16 | 5 |

- **Readings.** `p.1` is the launch speed (0 for the cannon ball: it is thrown with a launch velocity: `CLaunchVelocities`, `rsc-ctl.md`); `p.2` and `p.3` are the **direct hit damage** and the **explosion (range) damage**, with `p.7` and `p.8` the **damage radii** (a hot spot and a fall-off radius: the same two-radius model as `audio.md`); the members `p.11` to `p.14` are a **second set** of the same four values (an override of the first set for another victim kind: equal to the first set in the grenade deathmatch, plasma ray and flamer files, different in the cannon ball (600 / 700; 0 / 750 in deathmatch) and the zap gun (14 / 80), and **0 = not overridden** in the rocket, the single-player grenade, the Klodovik gun and the blades); `p.5` is probably the life time in seconds. Which victim kind each set serves (puppets versus environment or players) is **not stated by the data**. Common members: `bp_fImpactForceImpulse` (impact impulse) and `bp_fRangeForceImpulse` with up ratios (rocket 200 / 2,500, 0.5), piercing power (`bp_slPiercingPowerMin/Max`), seeking (`bp_f…Seek…`, the rocket has `bp_fSeekMovedCheckPeriod` 0.25), the owner damage multipliers (`bp_fDirectDamageOwnerMultiplier` 1 and `bp_fRangeDamageOwnerMultiplier` 0.5 on the rocket: **the shooter takes half damage from their own explosion**).
- **Special.** The plasma ray has no explosion (range damage 0; speed 100); the flamer's projectile is a `CFlamerProjectileParams` with flame effects (`CFlamerFlameParams`: `rsc-ctl.md`); the zap gun hits with 14 direct and 80 in the second set; the flamer's deathmatch file adds a fire effect (`Flamethower_Fire.pfx`).

## Special weapons

- **Circular saw** (`SCircularSawWeaponParams`): `cswp_fCuttingRange` 3, `cswp_fCuttingWide` 2, `cswp_fCuttingForceImpulse` 1 and six sounds (engine start, loop and stop; cutting start, loop and stop): by the names a **cutting volume 3 long and 2 wide in front of the player** that damages and pushes what it touches, with a continuous fire (`b.30` is 0). Its blades are a second weapon class (`SCircularSawBladesWeaponParams`) that **launches the blades projectile** (`Blades.ep`, speed 100, direct damage 50): the saw can throw blades.
- **Serious Bomb** (`SSeriousBombWeaponParams`): base members only (`b.30` 0, `b.34` 0.2, no projectile); its effect is in game code and the placed or thrown entity (not in the weapon file).
- **Flamer** (`CFlamerWeaponParams`, the only class with a `C` prefix) fires 0.1 s apart (`b.34` 0.05).

## Base members (all weapons)

`b.7` and `b.9` (0.005 or 0.01), `b.8` (5 or 10), `b.10` (6 or 8), `b.11` (0.1 or 0.4) and `b.12` (2 or 3) take two values each across the 17 weapons and look like **recoil, sway and kick** settings of the first-person view; `b.13` 0 to 1.25, `b.15` 0 to 40, `b.16` 0 to 140, `b.20` 0 to 0.6 (the beam gun: 0.25 → 0.5 in deathmatch), `b.21` 0 to 3 (0.75 → 3 for the beam gun in deathmatch), `b.22` (0, 500 or 550), `b.30` the **time between shots** (0.035 to 1.3 s; 0 for the sniper in single player and continuous weapons), `b.31` 0 to 0.75 (equal to `b.30` for the plasma rifle: a charge or second period), `b.32` 0 to 1, `b.33` 0 or 0.3, `b.34` 0.05 to 0.75 (a second period: probably an animation or reload gap). Names are numbers, so these are **readings from the value ranges only**; the camera and head-bob members that `camera-view.md` could not find are not named here either.

## Reader invariants

1. Load the weapon by class from the file; a missing deathmatch file means the single-player values apply (all 17 have one).
2. Use the **deathmatch** file in deathmatch modes and the single-player file otherwise: `game-modes.md` documents the same split for the player puppet file (`PlayerPuppet.ep` versus `PlayerPuppet_DM.ep`); that the weapons follow the same switch is **assumed, not verified**.
3. A weapon fires the projectile named by `.23` when it is not null; otherwise it fires **hitscan bullets** (`ctBullets` pellets of the named damage, scattered by `bswp_vPatternStretch` and the scatter members).
4. Treat unnamed numbers as opaque until the oracle capture names them; do not drop them (round-trip).
5. Fail on a weapon class outside the 17 listed; the combo-weapons pickup has no parameter file.

## Test vector

`EntityParams/ColtWeapon.ep` (`SColtWeaponParams`): model `Models/Weapons/Colt/Colt_FP.mdl`, tools `PlayerWeapon_Colt_L.ctl` and `_R.ctl`, `b.30` 0.5, `b.34` 0.25, `b.5` 6, bullet damage 10, impulse −1, pattern stretch (1, 1), scatter `s.3` 130, `s.4` 100; `ColtWeapon_DM.ep`: damage 15 (nothing else differs). `EntityParams/RocketLauncherWeapon.ep`: `b.30` 0.6 (0.665 DM), `b.23` `Rocket.ep` (`RocketDM.ep`); `Rocket.ep`: speed 60, direct damage 50, range damage 80, radii 4 and 10 (second set 8 and 16), impulses 200 / 2,500 with up ratio 0.5, owner multipliers 1 and 0.5, seeking period 0.25, flight life 5. `EntityParams/SniperWeapon.ep`: damage 75, zoomed 300, zoom 2 to 4 in 0.8 s; `SniperWeapon_DM.ep`: 25, 150, 2 to 6 in 1.0 s.

## Open questions

- **Everything behavioural** (task T01-075): the unit of the fire period and what `b.34` is, reload and magazine rules, ammunition use per shot (not in the data), the scatter model (`bswp_vPatternStretch` and `s.1` to `s.6`), spread growth in deathmatch, damage types and the victim kinds of the two damage sets, the damage falloff formulas, the combo weapons (dual wielding) rules and the Serious Bomb: oracle capture (T01-038) and the game module.
- The weapon switching order and the weapon bits of the cheat set (`player-rules.md`: `0xBFFF`) are not in the weapon files (they are in the game's player parameters).
- Sounds other than the minigun's and the saw's are requested by the hand animations (`.ctl`, `ans.md`) and the model's sound events, not named in the weapon file.
