# Projectile launch and charging (T01-119)

- **Roadmap task:** T01-119 (follow-up of `weapon-classes.md` open question 3; joins T01-132 for the damage pairs)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/projectile_launch_verify.py` reproduces from the data: the charging, recoil and launch-velocity members of the seven projectile weapons, the projectile files they name, the pairs at fire power 0 and 1, the values of the formulas of sections 4 and 5 for those numbers, the hand-grenade throw numbers, and which of the 94 projectile files a player weapon names; **medium** for everything read from `Sam2Game.dll` and `Engine.dll` (the two launch functions, the wall test, the fire-power sources, the discrete rule, the charge timing, the event replacement, the recoil rule, the hand-grenade throw), because each rests on **one reading** of the decompiled functions plus a read of the constants and **nothing was run**; **low** for the items marked low and for "Open questions".

Prerequisites: `weapon-classes.md` (the weapon entities and members, sections 4.1 and 9), `projectile-classes.md` (the projectile and its parameter file; section 5.3 asked this question), `hitscan-shot.md` (the placement of a shot, the wall test and the ray, sections 3 and 5.1), `players-session-classes.md` (the player puppet, the ammunition kinds), `weapons-tables.md`.

## Summary

A projectile weapon does not build the projectile itself. After the base class has paid the ammunition (`weapon-classes.md` rule 5) it asks the **owner puppet** to launch the projectile file named by `bwp_pgppProjectileParams`, through one of two virtual functions of `CPuppetEntity`, both of which end in `SpawnProjectile` (host only). The weapons differ only in **which function**, **which fire power**, **which launch velocity** and **which damage-inflictor weapon** they pass:

| Weapon | Owner function | Fire power passed | Launch velocity passed | Inflictor |
|---|---|---|---|---|
| Rocket launcher | `FireProjectileFrom` | 0 | -1 (the file's `bp_fFlyingSpeed`) | 6 |
| Grenade launcher | `ThrowProjectileFrom` | **0** | `lerp(10, 130, charge)` | 7 |
| Plasma rifle | `FireProjectileFrom` | 0 | -1 | 8 |
| Klodovik gun | `FireProjectileFrom` | 0 | -1 | 9 |
| Cannon | `ThrowProjectileFrom` | **the charge**, 0 to 1 | `lerp(40, 140, charge)` | 10 |
| Zap gun | `ThrowProjectileFrom` | **0 or 1** (discrete) | `lerp(40, 25, power)` | 1 |
| Flamer | `FireProjectileFrom` | 0 | -1 | 19 |
| Hand grenade (player puppet, not a weapon) | `SpawnProjectile` directly | 0 | `lerp(20, 40, pitch share)` | 14 |

What follows from it:

1. **Fire power is the charge of a charging weapon, and only two weapons ever pass more than 0 to the projectile**: the cannon (continuous, 0 to 1) and the zap gun (0 or 1, because its fire power is "discrete"). The grenade launcher scales its **speed** with the charge but gives the projectile fire power **0**. This answers `projectile-classes.md` section 5.3 and most of T01-132: the "maximum is 0 next to a positive minimum" members of the rocket, grenade, Klodovik and the other 36+ files are **never applied by a player weapon** (section 5).
2. **The charge** is `clamp((time since the charge began) / bwp_fChargingTime, 0, 1)` at the moment the fire button is released, or exactly 1 when the weapon fires itself `bwp_fChargingTime + bwp_fChargingAutoFireTime` seconds after the charge began (section 3).
3. **The launch velocity is `min + (max - min) × fire power`** of the weapon's own two members, which is **decreasing for the zap gun** (40 at power 0, 25 at power 1, section 4). A launch velocity of -1 means "use the projectile file's own speed".
4. **When the muzzle is behind a wall** the projectile is created half a metre in front of the wall (towards the player) and is told which entity it "hit immediately" (section 2.3).
5. The zap gun's `zgwp_ctMaxFirePowerWorthBullets` (6) only feeds a **fired-bullet counter that nothing reads** with the retail data (section 6).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs and, for the entity think scheduling, `Engine.dll`; no other build was compared. Exported functions read, by exported name: `CPuppetEntity::ThrowProjectileFrom`, `FireProjectileFrom`, `SpawnProjectile`, `GetProjectileProperties`; the `OnFire` of the seven projectile weapons; `CBaseWeaponEntity::OnFire`, `DoTheFiring`, `DoFiringsFromLastFireTime`, `CallFireStart`, `DoCharging`, `FireReleasedInternal`, `OnChargingEnd`, `OnFireReleased`, `OnStep`, `DoRecoil`, `OnRecoilEnd`, `GetRecoilDuration`, `SetState`, the default constructor; the default constructors of the 16 weapon entities (for the charging, discrete-power and recoil flags); `CGrenadeLauncherWeaponEntity::DoCharging`; `CZapGunWeaponEntity::ShouldReload`, `DoReload`; `CPlayerPuppetEntity::DoAttack`, `OnKlodovicSentFlying`, `CanFireFromWeaponNow`; `CEntity::SetNextThink` with the state manager function it calls. Which functions call the launch functions was found by searching **every** indirect call through the virtual table in `Sam2Game.dll` (no other module was searched): the two launch functions are called by exactly the seven weapons above (3 + 4), `SpawnProjectile` by the two launch functions, the player puppet's `DoAttack` and one function of the script virtual machine (section 8). The constants (the angle limits, the time scale, the multipliers) were read from the data section.
- **Data.** `tools/re/projectile_launch_verify.py [DATA_DIR]` (about 1 minute) prints the tables of sections 1 to 5 from the 14 weapon files, the projectile files and the player puppet files.
- **Not checked:** nothing was run on the original; the engine's collision handling after the spawn (`projectile-classes.md` section 5.1 reads it only for structure); the weapon state machine as a whole (T01-117); how the **enemies** launch (T01-143); the hand grenade's animation and the throw input; the time-scale constant is the engine's.

## Layout / rules

### 1. The seven projectile weapons (data; script section 1; **high**)

Member names are those of `weapon-classes.md` section 4.1. Single player / deathmatch where different. Time in seconds, speed in metres per second.

| Weapon | Projectile file (sp / dm) | `bwp_fChargingTime` | `bwp_fChargingAutoFireTime` | Recoil min | Recoil max | Launch min | Launch max | Own members |
|---|---|---|---|---|---|---|---|---|
| Rocket launcher | `Rocket.ep` / `RocketDM.ep` | 0 | 0 | 0.6 / 0.665 | 0 | 0 | 0 | |
| Grenade launcher | `Grenade.ep` / `GrenadeDM.ep` | 0.5 | 0 | 0.2 | 0.5 | 10 | 130 | |
| Plasma rifle | `PlasmaRay.ep` / `PlasmaRay_DM.ep` | 0 | 0 | 0.15 / 0.1 | 0.15 / 0.1 | 0 | 0 | ampules 3, 0.5, 0.8 |
| Klodovik gun | `Klodovik.ep` | 0 | 0 | 1.0 | 0 | 0 | 0 | |
| Cannon | `CannonBall.ep` / `CannonBallDM.ep` | 1.25 | 0 | 0.5 | 0.75 | 40 | 140 | |
| Zap gun | `ZapGunProjectile.ep` | 0.8 | 0.3 | 0.25 | 0.5 | 40 | 25 | `zgwp_ctMaxFirePowerWorthBullets` 6 |
| Flamer | `Flamer.ep` | 0 | 0 | 0.1 | 0 | 0 | 0 | deathmatch file adds a fire effect `Flamethower_Fire.pfx` (sic) |

`bwp_ctFiresToReload` is 0 for all seven. The data names a projectile file in **12** of the 94 (the 7 weapons' 11 files and `blades.ep`, which is the circular saw's, T01-120).

### 2. The two owner launch functions

#### 2.1 The calls (**medium**)

- **`ThrowProjectileFrom(params, placement, firePower, launchVelocity, inflictor)`** and **`FireProjectileFrom(params, placement, inflictor)`** are virtual functions of `CPuppetEntity` (the player puppet does not override them). `FireProjectileFrom` is the same as `ThrowProjectileFrom` with **fire power 0 and launch velocity -1**. Both first run the wall test (2.3), then call `SpawnProjectile(params, placement, firePower, launchVelocity, hitEntity, inflictor)`.
- **`SpawnProjectile`** does nothing on a **client** (`netIsRemote`). On the host it asks `GetProjectileProperties` for a properties object, stores the "hit entity" in it, spawns the entity from it and deletes the properties again. A debug check prints "SpawnProjectile executed on client!" if it is ever reached on a client after the properties were built.
- **`GetProjectileProperties(params, placement, firePower, launchVelocity, inflictor)`** returns **no properties (nothing is spawned) when `params` is null**; otherwise a `CFlamerProjectileProperties` when the parameter file's class derives from `CFlamerProjectileParams`, else a `CGenericProjectileProperties`. It stores in them: the parameter file, the fire power, the launch velocity, the placement, **the launching puppet as owner**, and the damage-inflictor weapon. The projectile's `OnCreate` reads them back (`projectile-classes.md` section 5.1); it takes a launch velocity of -1 as "use `bp_fFlyingSpeed`", reads the owner's damage multiplier (so the **serious-damage power-up of the owner multiplies projectile damage** as it does bullets, `hitscan-shot.md` 5.3), and adds the owner's velocity when the file has `bp_bAddShooterVelocity` (Klodovik gun and flamer: 1; all other projectile files of the seven weapons: 0, script section 2).
- Because the **weapon runs `OnFire` on every machine that holds it** but `SpawnProjectile` runs only on the host, ammunition is paid on the firing machine and the projectile exists on the host and reaches the clients through replication (`projectile-classes.md` section 9).
- **Ammunition is paid before the launch**: with no parameter file, or when the weapon's own parameter file is of the wrong class (the grenade launcher, cannon and zap gun test it; the flamer tests that its file is not null), the shot still costs ammunition and nothing is launched.

#### 2.2 The placement (**medium**)

The weapon passes the placement of `GetShootingPlacement` (`hitscan-shot.md` section 3): the owner's view orientation, **bent towards an auto-aimed target when the player has one**, from the world position of the muzzle attachment of the first-person model (the base class names it `Barrel01`; the flamer uses a fixed attachment of its own). A projectile therefore leaves the muzzle and flies towards the crosshair (or the auto-aimed enemy).

#### 2.3 The wall test and the "immediate hit" (**medium**)

Before spawning, both functions cast the engine's wall-test ray (the same `samIsShootingFromWall` as the hitscan shot: category `bullet`, radius 0, from the owner's eye to the muzzle position). **When it hits something**:

- the projectile's start position becomes **the hit point moved 0.5 m back along the ray direction** (towards the eye), the orientation is unchanged;
- the hit entity is passed to `SpawnProjectile` and stored in the projectile as its "immediately hit" entity, which the projectile's explosion uses as the entity that was hit (`projectile-classes.md` section 5.1: `en_henImmediatellyHit`). Whether the projectile explodes at once or only at its first normal collision was **not read** (follow-up T01-222).

The hand grenade (section 7) uses the wall test too but puts the start **on the hit point itself** (no 0.5 m shift).

### 3. Fire power and charging (**medium**)

`en_fLastFirePower` of the weapon holds the fire power of the last shot; it is set by `DoTheFiring` and read by the recoil duration (rule 5.3 of `weapon-classes.md`).

1. **Sources of the fire power** (the only callers of the shot function `DoFiringsFromLastFireTime(power)`):
   - `CallFireStart` (the press of the fire button, for a player owner whose `CanFireFromWeaponNow` agrees, with the weapon idle or "boring"): for a weapon **without** the charging flag it fires at once with **power 0**; with the **charging** flag it only starts charging (`DoCharging`); with the **spin-up** flag (minigun, circular saw) it only enters the spin-firing state;
   - **`FireReleasedInternal`** (the release): in the **charging** state the weapon fires with `power = clamp((now - charge start) / bwp_fChargingTime, 0, 1)`; in the spin-firing state it returns to idle; otherwise nothing;
   - **`OnChargingEnd`**, a scheduled event: fires with **power 1.0** exactly;
   - the weapon's step: while recoiling without the charging flag it fires again with **power 0** once `last shot + recoil duration` has passed and the owner still wants to fire (held fire button), and from idle it starts `CallFireStart` again when the owner wants to fire.
2. **`DoCharging`** enters the charging state, remembers the time as the charge start and schedules the event **`OnChargingEnd` after `bwp_fChargingTime + bwp_fChargingAutoFireTime` seconds**. Only **one** scheduled event exists per entity: scheduling another (the recoil's `OnRecoilEnd`, which every shot schedules, or a state's own event) **replaces** the pending one (the engine removes the entity's existing entry and inserts the new one; read in `Engine.dll`). So an early release, which fires and starts the recoil, **cancels** the pending `OnChargingEnd`.
3. **The discrete rule** (`DoTheFiring`): when the weapon's discrete-power flag is set and the power is **below 1.0**, the stored and passed power becomes **0**. Only the **zap gun** has the flag (default constructors of the 16 weapons). So the zap gun's power is **0** for a release before `bwp_fChargingTime` (0.8 s) and **1** from 0.8 s on, and it fires itself with power 1 at 0.8 + 0.3 = **1.1 s** if the button is held.
4. **The ammunition** is paid in `OnFire`, i.e. at the moment of the shot, not at the start of the charge. A shot that cannot be paid enters the can't-fire state and launches nothing.
5. **Flags per weapon** (default constructors): charging flag: grenade launcher, cannon, zap gun; spin-up flag: minigun, circular saw; discrete power: zap gun only; "scalable recoil duration": grenade launcher only (the others 0).
6. **Recoil duration** after a shot: `lerp(bwp_fMinRecoilDuration, bwp_fMaxRecoilDuration, en_fLastFirePower)`, except that **when the minimum is 0 (or less) and the weapon is not a charging weapon with scalable recoil, it is the length of the recoil animation** (`GetRecoilDuration`, as `weapon-classes.md` rule 2). With the data: rocket 0.6 (0.665 dm) whatever the power (the maximum is 0 but the power is 0); grenade launcher `0.2 + 0.3 × charge`; cannon `0.5 + 0.25 × charge`; zap gun 0.25 (power 0) or 0.5 (power 1); Klodovik 1.0; plasma rifle 0.15 (0.1 dm); flamer 0.1.
7. **The grenade launcher's charging animation** (local owner only, when the animation is longer than 0.1 s) plays at the speed `bwp_fChargingTime / animation length`, so that it ends when the charge is full. A weapon whose charging time is 0 and that has the charging flag would divide by zero; no file does.

**Charging, as a timeline (grenade launcher, held):** t = 0 press, charging starts; the charge is `t / 0.5`; at 0.5 s the weapon fires itself with charge 1 (speed 130), recoil 0.5 s; if the button is still held the next charge starts when the recoil ends. Released at 0.25 s: charge 0.5, speed 70, recoil 0.35 s.

### 4. The launch velocity (**high** for the formula's inputs, **medium** for the rule)

`speed = min + (max - min) × fire power` with the weapon's `bwp_fLaunchVelocityMin` and `bwp_fLaunchVelocityMax` (`weapon-classes.md` 4.1, whose two readings agree). It is the **initial speed along the projectile's forward direction**, replacing `bp_fFlyingSpeed`; the file's own value (cannon ball 0, zap bolt 75, grenade 30) is **not used** for these three weapons.

| Fire power | Grenade launcher (charge) | Cannon (charge) | Zap gun (0 or 1 only) |
|---|---|---|---|
| 0 | 10 | 40 | 40 |
| 0.25 | 40 | 65 | (not reachable) |
| 0.5 | 70 | 90 | (not reachable) |
| 0.75 | 100 | 115 | (not reachable) |
| 1 | 130 | 140 | 25 |

### 5. What the fire power does to the projectile (script section 2; **high** for the values, **medium** for the rule)

The projectile interpolates the pairs of its file with the fire power it was launched with (`projectile-classes.md` section 5.1, rule: value at power 0 plus `(maximum - minimum) × power`). With the powers of section 1:

| Weapon, file | Direct damage | Area damage | Hot spot / fall-off | Stretch | Speed |
|---|---|---|---|---|---|
| Rocket `Rocket.ep` | 50 | 80 | 4 / 10 | 0.75 | 60 (dm file 45) |
| Grenade `Grenade.ep` | 35 | 100 | 4 / 8 | 1 | 10 to 130 by charge |
| Plasma `PlasmaRay.ep` | 35 (dm 15) | 0 | 0 / 0 | 0.5 (dm 1) | 100 |
| Klodovik | 600 | 100 | 8 / 16 | **2.7** | 25 (+ owner velocity) |
| Flamer | 6 | 0 | 8 / 16 | 1 | 30 (+ owner velocity) |
| **Cannon** `CannonBall.ep` | **600 to 700** by charge | 100 | 2 / 8 | 1 | 40 to 140 |
| **Cannon** `CannonBallDM.ep` | **0 to 750** by charge | 25 | 2 / 16 | **0.4 to 1** by charge | 40 to 140 |
| **Zap gun** | **14 or 80** | 0 | 8 / 16 | **0.75 or 2** | 40 or 25 |

(The values at power 0 are the files' first members; the Klodovik's stretch pair is 2.7 then 1, so a power above 0 would **shrink** it; no weapon passes one.) All damage is then multiplied by the owner's serious-damage multiplier (4.0 in single player, 2.5 in a deathmatch-type session): a cannon ball at full charge in single player with the power-up is 700 × 4 = 2800. **Of the 94 projectile files, 12 are named by a player weapon; of the 41 files with a maximum of 0 next to a positive minimum in direct damage, area damage, hot spot or fall-off, 5 are among the 12** (`blades.ep`, `grenade.ep`, `klodovik.ep`, `rocket.ep`, `rocketdm.ep`), and **none of the five is ever launched with a power above 0 by a player weapon**. The enemies' attacks launch with power 0 as well (`projectile-classes.md` section 5.3, the AI path of the shooter); the script virtual machine's launch function (section 8) takes the power from the script. So the maxima of those files are, in practice, unused data.

### 6. The zap gun's bullet count (**medium** for the formula, **high** that it is unused)

Each shot adds `1 + (zgwp_ctMaxFirePowerWorthBullets - 1) × power` to the weapon's fired-bullet counter, truncated toward zero **after** the addition: `count = trunc((worth - 1) × power + count + 1)`. With the data (worth 6): power 0 adds 1, power 1 adds 6 (counts 1, 2, 3 and 6, 12, 18 after three shots). The counter is compared with `bwp_ctFiresToReload` for the zap gun's reload test, and **that member is 0 in all 34 weapon files**, so the weapon never reloads and the counter has no effect; the reload function would reset it. (The counter is a leftover of a magazine mechanic.)

### 7. The hand grenade (**medium**; `CPlayerPuppetEntity::DoAttack`, attack type 2)

Not a weapon: the player puppet itself throws `ppp_pgppHandGrenade` (`HandGrenade.ep` in `playerpuppet.ep`, `HandGrenadeDM.ep` in `playerpuppet_dm.ep`; the two other player-puppet files, `player.ep` and a cut-scene file, have none) when the attack of type 2 is requested. In this order:

1. **Interval.** If the time since the last throw is **less than `ppp_fHandGreandeRecoil`** (sic; 1.0 s in the data) nothing happens. If the player is **riding**, nothing happens. Otherwise the last-throw time is **set to now** (so a throw that then fails for lack of ammunition still starts the interval).
2. **Ammunition.** One unit of ammunition kind `AI_HAND_GRENADES` is drained, **unless** the game info's mode value is 2 and a second game-info member (not identified) is non-zero; if the drain fails, nothing is thrown.
3. **Direction.** The owner's view placement (eye position, view orientation) with the **pitch increased by `ppp_fAddedPitchDeg`** (10 degrees in the data) and wrapped into -180 to 180 degrees.
4. **Speed.** `t = clamp((pitch - Pmin) / (Pmax - Pmin), 0, 1)` using that adjusted pitch and the two limits `ppp_fMinBombThrowPitchDeg`, `ppp_fMaxBombThrowPitchDeg` (-20 and 20 degrees); `speed = Vmin + (Vmax - Vmin) × t` with `ppp_fMinBombThrowVelocity`, `ppp_fMaxBombThrowVelocity` (20 and 40). The names are by the order rule of `players-session-classes.md` section 7 (**medium**); looking up (positive pitch, by the engine's convention of `hitscan-shot.md` section 4) throws harder.
5. **Launch.** The wall test of 2.3 runs; on a hit the start is the hit point. `SpawnProjectile(params, placement, fire power 0, speed, hit entity, inflictor 14)`. The projectile's own fuse and damage come from its file (`HandGrenade.ep`: direct 20, area 120, hot spot 4, fall-off 10, life 5.0 s, speed member 30 unused; the deathmatch file area 80).
6. A counter of the achievement manager is increased; **the 200th throw unlocks achievement number 17** (the counter's internal id; the achievement's name was not read, **low**).

### 8. Other launches (**medium**)

- **Klodovik gun:** after the launch it calls the player puppet's `OnKlodovicSentFlying`, which increments a counter of the achievement manager; the **100th** shot unlocks achievement number 14 (**low** that it is "sent flying" and not "shots": the function increments on every call).
- **Flamer:** launches **one** `Flamer.ep` projectile per shot (period 0.1 s, life 1.0 s, speed 30 m/s plus the owner's velocity, direct damage 6); the projectile class and its burning rules are `projectile-classes.md` section 5.2 and 5.4. Its inflictor is 19 (the flamer), not a `WeaponIndex`.
- **Script launches.** One function of the script virtual machine reads a parameter file, a placement, a fire power, a launch velocity and a hit entity from the script stack and calls `SpawnProjectile` of the object it is given; level scripts can therefore launch any projectile at any power (which scripts do so is not examined; follow-up T01-223).
- **Enemies** launch through the shooter (`CShooterEntity::FireProjectile`, fire power 0, `projectile-classes.md`) or through the puppet functions above; T01-143.

## Invariants

1. Every projectile weapon passes fire power 0 **except the cannon (charge) and the zap gun (0 or 1)**; only the grenade launcher, cannon and zap gun have non-zero launch velocity members (3 of 7, script).
2. For the three scaling weapons `Launch min` and `Launch max` are both positive, and **`min` is above `max` for the zap gun only** (the speed falls with power).
3. A charging weapon has `bwp_fChargingTime` > 0 (3 of 3: 0.5, 1.25, 0.8); a non-charging weapon has 0 for it and for the auto-fire time (4 of 4); only the zap gun has an auto-fire time (0.3).
4. The fired-bullet count of the zap gun is never read (`bwp_ctFiresToReload` is 0 in 34 of 34 files).
5. The launch velocity passed is **-1 or `lerp(min, max, power)`** (never the file's speed for the three scaling weapons).
6. Nothing is spawned on a client; reject a launch without a parameter file (ammunition is still paid).

## Open questions

Ranked by what blocks an implementer. New task IDs are appended to `docs/roadmap/T01-reverse-engineering.md`.

1. **The weapon state machine around the shot** (the exact order of the shots owed after a long frame, the state changes of the held button, weapon switching while charging, whether a pending charge survives a weapon switch): `DoFiringsFromLastFireTime` and `OnStep` were read for the fire-power sources only. **T01-117.**
2. **Whether a projectile that was given an "immediately hit" entity explodes at once** and what its `OnCreate` does with it (`projectile-classes.md` 5.1 open question 1). **T01-222.**
3. **Which scripts launch projectiles and with which fire power** (the script native function; the 36+ files whose maxima no player weapon uses may be used there). **T01-223.**
4. **Run-time checks on the original** of the numbers of this page (charge times, speeds, recoil periods, the zap gun's two powers): **T01-124.**
5. The game info members of the hand grenade's free-ammunition rule (mode value 2 and a flag) and the sign of the engine's Euler pitch are not identified; the identity of achievements 14 and 17: part of **T01-114** (the player puppet).
6. **The grenade launcher's fire power 0**: it is certain that the code passes 0 (read in the call), but it is unusual next to the cannon; the data's maxima (direct 35 against 0) agree with a power that never exceeds 0 (the same for the rocket), so a defect of the original is possible. Left to T01-124.

## Test vectors

Derived from the data and the rules above; **not** run on the original.

1. **Grenade launcher released after 0.25 s** (single player): charge `0.25 / 0.5 = 0.5`; launch speed `10 + 120 × 0.5` = **70**; fire power passed **0**: `Grenade.ep` direct damage **35**, area damage **100**, stretch 1 (the maxima 0 are not applied); recoil `0.2 + 0.3 × 0.5` = **0.35 s**. Held: fires itself at **0.5 s** with charge 1: speed **130**, recoil 0.5 s.
2. **Cannon released after 0.625 s**: charge 0.5; speed `40 + 100 × 0.5` = **90**; fire power 0.5: direct damage `600 + 100 × 0.5` = **650** (deathmatch ball `0 + 750 × 0.5` = **375**, stretch `0.4 + 0.6 × 0.5` = 0.7); recoil `0.5 + 0.25 × 0.5` = **0.625 s**. Released after 2 s: the charge was already 1 at 1.25 s and the weapon fired itself then (charge 1, speed 140, direct 700).
3. **Zap gun tapped (0.3 s)**: charge 0.375 < 1, discrete rule: power **0**: speed **40**, direct damage **14**, stretch 0.75, fired-bullet count +1, recoil **0.25 s**. **Released at 0.8 s or later**: charge 1, power **1**: speed **25**, direct damage **80**, stretch 2, count +6, recoil **0.5 s**. **Held**: fires itself at **1.1 s** with power 1.
4. **Rocket launcher** (single player): fire power 0, velocity -1: speed 60 (file), direct 50, area 80, hot spot 4, fall-off 10, stretch 0.75; held button repeats every **0.6 s**; with the serious-damage power-up the damage is multiplied by 4.0.
5. **Hand grenade** (player.ep values): view pitch 0 degrees: adjusted pitch 10, `t = (10 + 20) / 40 = 0.75`, speed `20 + 20 × 0.75` = **35**; view pitch +10 or higher: speed **40**; view pitch -30 or lower: speed **20**. The launch pitch is the view pitch plus 10 degrees. A second throw within 1.0 s does nothing; the throw costs one hand grenade.
6. **Wall in front of the muzzle**: the wall-test ray from the eye to the muzzle hits a wall at (x, y, z) with ray direction d: a rocket is created at `(x, y, z) - 0.5 d` with the wall as its immediate-hit entity; a hand grenade at `(x, y, z)`.
7. **Zap counter**: three shots of power 1: count 6, 12, 18; with `bwp_ctFiresToReload` 0 no reload follows.
8. **Serious damage**: single-player cannon ball at charge 1 with the power-up: `700 × 4.0` = 2800 direct damage; deathmatch with the power-up: `750 × 2.5` = 1875.
