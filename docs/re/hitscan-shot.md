# The hitscan shot (T01-118)

- **Roadmap task:** T01-118 (follow-up of `weapon-classes.md` open question 2; joins `projectile-classes.md` section 6, T01-107)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what `tools/re/hitscan_shot_verify.py` reproduces from the data: the six hitscan weapons' scatter, range, damage and pellet numbers (single player and deathmatch), the pattern stretch of each weapon, and which enemy attacks have a bullet description that fires at all and of which bullet type; **medium** for everything read from `Sam2Game.dll` (the shooter's set-up by each weapon, the pellet pattern tables, the scatter formula and its constants, the order of effects of a shot, the damage rounding, the ray set-up, the placement of the shot, the trails) and, as of T01-220 (`Engine.dll`'s `cldCheckRay`/`cldContinueRay` and their shared result handler, and `Sam2Game.dll`'s ray-query reset helper used by `FireBullets`, all read in full), for the global that holds the hit distance and for the query structure having two independent category slots; **low** for the items listed as such (the start point of the wall test for non-puppet owners, the exact avatar/avatar-mechanism fields, the "first of several candidates" rule) and for every item of "Open questions".

Prerequisites: `weapon-classes.md` (the weapons, their parameter members, section 4.2 for the scatter members and section 9 rule 6), `projectile-classes.md` (the shooter's members, sections 6.1 to 6.6), `players-session-classes.md` (the player puppet, the damage multiplier of the serious-damage power-up, the brain), `effects-pipeline.md` (the material events), `puppet-entity-classes.md` section 7 (what a puppet does with a damage record).

## Summary

A hitscan weapon (the Colt, Uzi, minigun, auto shotgun, double shotgun and sniper rifle) shoots by handing a **placement** (where the shot starts and which way it points) to its **owned shooter entity**, `CShooterEntity`, which was **configured once when the weapon was created**. The shooter then fires `en_ctBulletsToFire` **pellets**. For each pellet it (1) takes an offset from a fixed **pattern** (shotguns only), (2) adds a **random offset uniform over a ring** (the scatter), (3) casts a **ray** of a small radius, (4) on a hit applies an **impulse**, delivers a **damage record** (host only) and spawns the **`BulletHit` impact effect** of the surface material, and (5) spawns a cosmetic **bullet trail**.

What a reader needs to know first, with the corrections to earlier pages:

1. **The shooter is set up at creation, not at the first shot.** `weapon-classes.md` rule 6 said that the shotguns send their pellet count and damage "at the first shot". Each of the six weapons creates its shooter in `OnCreate` and fills in **everything** there (bullet type, pellet count, damage, damage type, scatter distance and radii, bullet velocity, range, impulse, ray radius, owner; the shotguns also the pattern). The "first shot" code of the shotguns is a **fallback** that sets only the pattern and is **not reached** for a normally created weapon (section 2.3).
2. **The pellet patterns are two static tables in the binary** (7 offsets for the auto shotgun, 14 for the double shotgun), answering "where the pellet pattern list comes from" (`projectile-classes.md` open question 3). Pellet *i* takes entry *i* modulo the table size, scaled by `bswp_vPatternStretch`. **Only the two shotguns have a pattern**; the Colt, Uzi, minigun and sniper have none, so their `bswp_vPatternStretch` (the minigun's is (1, 0.5)) is **never used** (`weapon-classes.md` section 4.2 listed (1, 1) only; the data has (1, 1), (1, 0.5), (0.5, 0.5) and (0.75, 0.75), table in section 1).
3. **The scatter** is uniform over a ring between two angles, `atan(Rmin / D)` and `atan(Rmax / D)`, centred on the pellet's pattern point (section 4). The random generator is seeded from the **processor's time-stamp counter at start-up**: the sequence of a run cannot be reproduced, only its distribution.
4. **The damage of a pellet** is the weapon's damage times the owner's **serious-damage multiplier** (4.0 in single player and co-op, 2.5 in a deathmatch-type session, else 1.0), **truncated toward zero** to a whole number (section 5.3). Damage types: Colt, Uzi and minigun `DMT_BULLET` (19), both shotguns `DMT_SLUG` (21), sniper `DMT_SNIPER_BULLET` (20).
5. **The ray radius is 0.5 m in single player** for every weapon except the sniper (0, a thin ray), and in deathmatch 0.1 (Colt), 0.5 (Uzi, minigun), 0.3 (both shotguns), 0 (sniper).
6. **The sniper's damage switches with the zoom**: 75 normal and 300 zoomed in single player (25 and 150 in deathmatch), and a zoomed shot starts at the player's eye, not at the muzzle.
7. **Only one enemy attack in the whole data fires a flying bullet** (`BT_PHYSICAL`): `helicopter.ep` (two bullets). Every other attack that has a bullet description with a positive count uses a ray type (script, section 3 of its output).

## Evidence

- **Binary.** `Sam2Game.dll` of the install used for the other specs (no other build was compared). The exported functions read, by their exported names: `CShooterEntity::FireBullets`, `ApplyScattering`, `SetParameters`, `SetPattern`, `SetBulletDamage`, `SetBulletImpulse`, `SetBulletRadius`, `SetBulletVelocity`, `SetShootDistance`, `SetOwner`, `SetDamageInflictorWeapon`, `UseThinBullet`, `FireBulletsFromPlacement`, `ShootPTP`, the constructor; `CBaseWeaponEntity::OnFire`, `DoTheFiring`, `GetShootingPlacement`, `GetShootingAttachment`, `AlterShootOrigin`, `AlterViewOrigin`, `SetOwner`, `FetchOwner`; the `OnCreate` and `OnFire` of the six weapon classes; `CSniperWeaponEntity::GetShootingPlacement`, `ActivateZoomMode`, `DeactivateZoomMode`; `CPlayerPuppetEntity::GetApplyDamageMultiplier`; `CBulletEntity::OnCreate`, `OnHullTouched`, `OnTargetHit`; `CBulletTrailEntity::OnCreate`; `CBulletTrailRenderable::GetDyingTime`; the engine function `samIsShootingFromWall`; the helpers that start and continue a ray; the engine's float-to-integer conversion; and the generator `mth_ulRandom1` / `mth_ulRandom2` with its initialisation in `Core.dll`. The constants (scatter, generator scale, thresholds, the two pellet tables, the multipliers) were read from the data section. The virtual-function slot numbers were matched against the vtables compared for `weapon-classes.md`.
- **T01-220** additionally read, in full, `Engine.dll`'s `SeriousEngine::cldCheckRay`, `cldContinueRay` and their shared internal result handler, and `Sam2Game.dll`'s reset helper for the ray-query object `FireBullets` passes to the cast.
- **Data.** `tools/re/hitscan_shot_verify.py [DATA_DIR]` (about 2 minutes) reads the 17 + 17 weapon parameter files and the 168 bullet descriptions (`CBulletFireing`) of the 90 enemy `.ep` files that have one, and prints the tables of section 1 (copied below) and the counts of section 8.
- **Not checked:** nothing was run on the original; the engine's ray cast was partially read (T01-220, see "The engine's ray cast" below; the exact candidate-sorting rule and the avatar fields' identity stay open); the damage a **target** does with the record (`OnReceiveDamage`, `puppet-entity-classes.md` section 7, T01-140); the **view origin** of the player (`GetViewOrigin`, its head-bob animation, `camera-view.md`); the weapon's **fire timing** (T01-117); trail rendering.

## Layout / rules

### 1. The numbers of the six hitscan weapons (data; script section 1)

All values are members of the weapon's `.ep` file (`weapons-tables.md` has the full files). `sp` = single player, `dm` = deathmatch file; one number where both are equal. **High** (script, all 12 files).

| Weapon | Bullet type | Pellets | Damage sp / dm | Scatter distance D | Radius min | Radius max | Range | Pattern stretch | Bullet velocity |
|---|---|---|---|---|---|---|---|---|---|
| Colt | 1 | 1 (fixed in code) | 10 / 15 | 10 | 0 | 0 | 100 | (1, 1) | 130 |
| Uzi | 1 | 1 | 10 / 13 | 100 / 500 | 0.1 / 0 | 2 / 15 | 300 / 500 | (1, 1) | 130 |
| Minigun | 2 | 1 | 10 / 12 | 200 / 750 | 0 | 1 / 22.5 | 300 / 750 | (1, 0.5) | 130 |
| Auto shotgun | 2 | 7 | 12 / 10 | 100 / 500 | 0 | 0.5 / 3 | 200 / 500 | (0.5, 0.5) | 100 |
| Double shotgun | 2 | 14 | 12 / 7 | 100 / 500 | 0 | 1 / 4.5 | 200 / 500 | (0.75, 0.75) | 100 |
| Sniper | 1 | 1 | 75 / 25 (zoomed 300 / 150) | 10 | 0 | 0 | 400 / 1500 | (1, 1) | 130 |

Bullet impulse is `-1` and up ratio `-1` in all 12 files (both mean "the global default": 250 and 0.5, section 5.2). Bullet velocity is used only by the flying bullet type 0 that no weapon uses.

Scatter angles that follow (`atan(radius / D)` in degrees; script section 2): Uzi sp 0.057 to 1.146, dm 0 to 1.718; minigun sp 0 to 0.287, dm 0 to 1.718; auto shotgun sp 0 to 0.287, dm 0 to 0.344; double shotgun sp 0 to 0.573, dm 0 to 0.516; Colt and sniper exactly 0. (The deathmatch radii grow with D so that the angles stay of the same size.)

### 2. The owned shooter

#### 2.1 Creation and ownership (**medium**)

- In its `OnCreate`, each of the six weapon classes first asks for its parameter file. **When the file is available and of the weapon's own parameter class**, the weapon spawns a `CShooterEntity` from a properties object with no member (so the shooter starts with its constructor defaults, section 2.2), stores its handle (the weapon's own member `en_henShooter`), and configures it as in section 2.3. **When the file is not available the weapon creates no shooter at all**, and `OnFire` of every one of them ends with "no shot" when the handle is empty (rule: no shooter, no shot).
- After the base class' `OnCreate`, the weapon tells the shooter its **owner** (the puppet that holds the weapon, `SetOwner`). The six classes' own `SetOwner` and `FetchOwner` repeat that call when the weapon's owner changes, so the shooter always has the weapon's owner.
- Each `OnFire` first runs the base `OnFire` (threat sound, owner notified, ammunition paid, `weapon-classes.md` rule 5); **only when it reports success** the weapon sets the shooter's damage-inflictor weapon to its own index (Colt 12, Uzi 4, minigun 5, auto shotgun 2, double shotgun 3, sniper 13: the `DamageInflictorWeapon` values of `weapon-classes.md` section 3), asks itself for the shot's placement (section 3) and calls the shooter's `FireBulletsFromPlacement(placement)` once. The minigun then also sets its spin state to "spinning" and its fire sound state to "firing". So **one `OnFire` is one `FireBullets` call**, however many pellets it has.

#### 2.2 Constructor defaults of the shooter (**medium**; they match the stored shooters of the worlds, `projectile-classes.md` 6.1)

Range 100; damage 0; damage type -1; pellets 0; bullet type 1 (`BT_RAY_STATIC_TRAIL`); bullet velocity 0; impulse -1 and up ratio 0 (the up ratio of the stored shooters is read as 0; **but** `FireBullets` replaces a ratio below 0 only, so a stored 0 means "no up component"); an **empty pattern** (no offsets); scatter distance 10, both radii 0; prediction ratio 0; damage-inflictor weapon -1; **ray radius = the global `bp_fBulletRayDiameter` (1.0) times 0.5 = 0.5**, or 0.5 when the global parameters are missing.

#### 2.3 What each weapon puts into the shooter (**medium**; one reading of each `OnCreate`; values from the weapon's own file)

In this order: (1) shotguns only: the pattern and the stretch; (2) `SetParameters(bullet type, pellets, damage, damage type, D, Rmin, Rmax)`; (3) bullet velocity; (4) range (the shooter's "shoot distance"); (5) impulse and up ratio; (6) the ray radius (deathmatch, or the sniper); (7) the owner, after the base `OnCreate`.

| Weapon | Pattern table | Stretch | Bullet type | Pellets | Damage | Damage type | Ray radius sp / dm |
|---|---|---|---|---|---|---|---|
| Colt | none | not applied | file | **1** (constant in code) | `cwp_slBulletDamage` | 19 `DMT_BULLET` | 0.5 / 0.1 |
| Uzi | none | not applied | file | 1 | `uwp_slBulletDamage` | 19 | 0.5 / 0.5 |
| Minigun | none | not applied | file | 1 | `mgwp_slBulletDamage` | 19 | 0.5 / 0.5 |
| Auto shotgun | 7 entries (section 2.4) | `bswp_vPatternStretch` | file | `aswp_ctBullets` | `aswp_slBulletDamage` | 21 `DMT_SLUG` | 0.5 / 0.3 |
| Double shotgun | 14 entries | `bswp_vPatternStretch` | file | `dswp_ctBullets` | `dswp_slBulletDamage` | 21 | 0.5 / 0.3 |
| Sniper | none | not applied | file | 1 | `swp_slBulletDamage` (and the zoom damage, section 2.5) | 20 `DMT_SNIPER_BULLET` | **0 / 0** (thin ray: the weapon calls the shooter's `UseThinBullet`) |

"File" = the value of the `SBaseScatterWeaponParams` member of section 1. "0.5" in single player is not set by the weapon: it is the constructor default (section 2.2). In deathmatch the five weapons override it (`samIsDeathmatch`), the sniper always.

**The first-shot code of the shotguns.** Both shotgun `OnFire` functions test a flag of the weapon that `OnCreate` sets right after configuring the shooter; when the flag is clear they would set the **pattern and stretch only** (no pellet count, no damage, no ranges) and set the flag. A shooter exists only when `OnCreate` configured it, and that same code sets the flag, so the branch is a **safety fallback that a normally created weapon never takes** (**medium**, one reading of both functions; the flag is not a named member). The auto shotgun's fallback passes the **double shotgun's** table (the same table address in both functions): a latent mix-up that cannot occur in normal play.

#### 2.4 The two pellet tables (**medium**; read from the data section, each table's size equals the pellet count of its weapon's file)

Entries are offsets (x right, y up) in **tangent units at one metre**, before the stretch; the offset of a pellet is `entry × stretch` per axis. An entry of 0.06 is an angle of `atan(0.06)` = 3.43 degrees.

| Index | Auto shotgun (7), x, y | Double shotgun (14), x, y |
|---|---|---|
| 0 | -0.06, 0.02 | -0.09, 0.045 |
| 1 | 0.00, 0.02 | 0.00, 0.045 |
| 2 | 0.06, 0.02 | 0.09, 0.045 |
| 3 | -0.08, -0.02 | -0.12, 0.015 |
| 4 | -0.02, -0.02 | -0.03, 0.015 |
| 5 | 0.02, -0.02 | 0.03, 0.015 |
| 6 | 0.08, -0.02 | 0.12, 0.015 |
| 7 | | -0.09, -0.015 |
| 8 | | 0.00, -0.015 |
| 9 | | 0.09, -0.015 |
| 10 | | -0.12, -0.045 |
| 11 | | -0.03, -0.045 |
| 12 | | 0.03, -0.045 |
| 13 | | 0.12, -0.045 |

(A table of 3 + 4 rows for the auto shotgun and 3 + 4 + 3 + 4 for the double shotgun: a staggered grid.) With the file's stretch the auto shotgun's fan spans x ±0.04, y ±0.01 and the double shotgun's x ±0.09, y ±0.034.

#### 2.5 The sniper's zoom damage (**medium**)

`ActivateZoomMode` sets the shooter's damage to `swp_slBulletDamageInZoomMode`; `DeactivateZoomMode` sets it back to `swp_slBulletDamage`. The creation value is the normal one. So the damage depends on whether the weapon is zoomed when the shot is fired.

### 3. The placement of the shot (**medium**; `CBaseWeaponEntity::GetShootingPlacement`)

The weapon computes a placement (a rotation and a position) for each shot:

1. **Start from the owner's view placement** (`GetViewOrigin` of the owner puppet; for a player it includes the view animation, `camera-view.md`).
2. **Origin.** When the weapon has a shooting attachment (the base class names the model attachment **`Barrel01`**; the flamer overrides it), the origin becomes the **world position of that attachment on the first-person weapon model**, found by combining the owner's view placement, the weapon-in-view matrix (the weapon's offset and sway in the view, T01-121) and the attachment's placement in the model. If the model has no such attachment the origin stays the owner's eye.
3. **Direction.** The orientation stays the owner's view orientation, **not** the muzzle's: the shot goes where the crosshair points, from the muzzle.
4. **Auto-aim.** When the owner's brain is a player brain and its **auto-aimed entity** (`en_henAutoAimed`) is set, the direction is replaced by the direction from the (muzzle) origin to that entity's **view target** point (`GetViewTarget`), when that distance is above a tiny threshold (1.2e-7). The new orientation has heading and pitch only (no roll).
5. **The sniper** overrides this: **while zoomed** the placement is the owner's view placement unchanged (the eye, no muzzle offset, no auto-aim); while not zoomed the base rule applies.

`AlterShootOrigin` and `AlterViewOrigin` of the weapon base class are empty functions that no weapon class overrides. The placement does not depend on the shooter.

### 4. Scatter and pattern (**medium**; `CShooterEntity::ApplyScattering` read in full, constants read from the data section)

Per pellet *i* (0-based, `0 <= i < en_ctBulletsToFire`), with `D = scatter distance`, `Rmin`, `Rmax` and the pattern offset `(px, py)` (zero when the shooter has no pattern, i.e. a pattern whose size is not above 0):

1. **Pattern offset.** `(px, py) = (e.x × stretchX, e.y × stretchY)` where `e` is the pattern entry number `i mod N` (`N` = table size, section 2.4). Pellets beyond `N` repeat the table (the retail data never has more pellets than entries).
2. **Ring radii.** If `D` is above 0.001: `r0 = Rmin / D` and `r1 = Rmax / D`; otherwise both are 0.
3. **Two uniform random numbers** `u1`, `u2`, each in [0, 1) (step 4 below). Radius **`r = r0 + (r1 - r0) × sqrt(u1)`** (uniform over the ring's area); angle **`a = 2π × u2`**.
4. **Local direction.** `x = px + r cos a`, `y = py + r sin a`; the offset direction in the shot's own frame is `(x, y, -1) / sqrt(x² + y² + 1)` (forward is -Z, up +Y).
5. **Turn.** The shot's orientation is turned by the heading and pitch of that direction (pitch `asin(y')` with the argument limited to ±1; heading `atan2(-x', -z')`, **0 when `|y'|` is above 0.99**), composed with the shot's own rotation: the final forward vector is the placement's rotation applied to the local direction.

Consequences: with `Rmin = Rmax = 0` and no pattern the shot is **exact** (Colt, sniper); the offsets are **tangent offsets at distance `D`**, hence the angle `atan(R / D)`; **every pellet of a pattern shot gets its own ring offset** on top of its table entry, so the shotguns' pellets are a fixed fan blurred by a ring of radius up to `Rmax / D` (0.005 for the auto shotgun in single player, 0.01 for the double shotgun).

**The random generator** is the engine-wide pair `mth_ulRandom1`, `mth_ulRandom2` (two 32-bit states, each call advances both: the first by multiplication with 1,220,703,125 modulo 2^32, the second by a shift-register step; the number is their exclusive-or read as an unsigned integer times 2^-32). It is **seeded once at engine start-up from the processor's time-stamp counter** (first state: its low half times that multiplier; second: its low half exclusive-or a fixed constant). So **the scatter sequence differs in every run**; only the uniform distribution is specified. A pellet takes two numbers, `u1` first, then `u2`.

### 5. `FireBullets`: the per-pellet sequence (**medium**)

Before the loop: the **impulse** is `en_fBulletImpulse` when it is 0 or above, else the global `bp_fDefaultBulletForceImpulse` (250); the **up ratio** is `en_fBulletImpulseUpRatio` when 0 or above, else `bp_fBulletUpImpulseRatio` (0.5); the **owner multiplier** is the owner's `GetApplyDamageMultiplier` (1.0 when there is no owner).

#### 5.1 One pellet

1. Direction and pattern as in section 4.
2. **The wall test** (the engine function `samIsShootingFromWall`): a ray with category `bullet` and radius 0 is cast **from the owner's eye to the shot's origin** (puppet owners: the start is the owner's view-origin position; for owners that are not puppets, a point on the owner's own forward axis nearest to the origin, **low**). It continues through the owner's own riders and ride up to 10 times, and is cleared again when it only hits a projectile of the same owner that has just been launched. If it **hits something**, the muzzle is behind geometry: that hit is **used as the pellet's hit** and no other ray is cast (the "shooting from a wall" case, there is **no shift of the origin**; the earlier guess of a shift in `projectile-classes.md` 6.2 is withdrawn).
3. **Bullet type 0 (`BT_PHYSICAL`) without a wall hit:** a `CBulletEntity` is spawned at the shooter's placement (position, orientation = the scattered orientation) with the damage, damage type, owner, range, velocity, impulse and ray settings; its own flight and hit are `projectile-classes.md` 6.4. **A sound flag is set for the first two pellets of the shot** (`en_bPlaySound` of the bullet is true for `i < 2`). Nothing else happens for this pellet (no trail).
4. **All other cases** (types 1, 2, 3, or type 0 with a wall hit): when there was no wall hit, a **ray is cast** from the origin along the scattered direction for **`en_fBulletShootDistance`** metres, with radius `en_fRayRadius`, the owner as the avatar (never hit by its own ray), category `bullet` for the ray and `bullet_no_solids` for its thick part (see below). When the first thing hit **belongs to the owner's own ride** (`GetRide`), the ray is **continued** from that hit, up to 10 times (a warning "Own ride hit again after continuing ray cast" is printed if it is hit again after the last continuation).
5. **On a hit** of an entity: if it is a rider of the owner, **nothing happens for this pellet** (no impulse, no damage, no effect, **no trail** either: the code leaves the pellet at this point). Otherwise, in this order:
   - the **trail length** is the distance from the ray's start to the hit point, at least 0.001; with no hit it is the full range;
   - the **impulse vector** = `normalize(forward + up × upRatio) × impulse` (`up` = (0, 1, 0), `forward` = the scattered direction); it is applied (the entity's `ReceiveForceImpulse`) to the **body** of the hull that was hit, at the hit point, **on every machine**, only if the hit entity and the hull's body exist;
   - **on the host only** (the machine for which `netIsRemote` is false): a **damage record** is built and delivered to the hit entity (its `OnReceiveDamage`), then deleted. Its fields: **amount** = `en_slBulletDamage × owner multiplier`, **truncated toward zero** to an integer; damage type `en_dtDamageType`; **attacker** = the owner puppet; the shot direction; the hit point; the impulse vector above; the **hull hit**; the **damage-inflictor weapon** `en_diwDamageInflictorWeapon`;
   - **on every machine**, when the hit surface has a material: the **`BulletHit` event of the material is spawned** at the hit point with the surface normal (`samSpawnEffectForMaterial`; `effects-pipeline.md`).
6. **Trail** (types 1 and 2 only, also when nothing was hit): a `CBulletTrailEntity` is spawned at the ray's start, oriented along the direction, with the trail type (**type 1: static, type 2: moving; type 3 and type 0: none**) and the trail length of step 5 (the range when nothing was hit).

Notes. (a) With **no hit** the pellet still produces its trail and nothing else. (b) The shooter has debug displays ("Bullet hit", "Bullet lines", "Bullet impact") that are only active with the engine's debug-draw switches; they have no game effect. (c) Everything in the loop uses the pellet's own random offsets; the **count of rays is the pellet count**: the double shotgun casts 14 rays per shot (sp: 14 × 12 damage = 168 maximum, before the multiplier).

**The thick ray.** The engine ray is set with a radius and two categories: the thin ray `bullet`, and the thick part `bullet_no_solids`. By the names the thick ray (radius `en_fRayRadius`) does **not** collide with solids and the thin centre ray does, so that a radius of 0.5 m makes the shot hit **entities** (enemies, props) that pass within 0.5 m of the line but **never** makes it pass a wall corner. **This is now confirmed to be a real, structural distinction, not just the two names** (T01-220, see "The engine's ray cast" below): the query object `FireBullets` fills in before casting has **two separate category fields** (reset to the engine's "invalid identifier" independently), one for the thin centre line and one for the thick part, matching the two names exactly; which literal string goes into which field was not pinned down further.

#### 5.2 Defaults

| Item | Value | Source |
|---|---|---|
| Impulse when the member is below 0 | **250** (`bp_fDefaultBulletForceImpulse`) | global parameters (`projectile-classes.md` 6.6, high) + code (medium) |
| Impulse up ratio when below 0 | **0.5** | same |
| Ray radius when not set (single player) | `bp_fBulletRayDiameter × 0.5 = 0.5` | same |
| Bullet velocity, reach for enemy descriptions with -1 | 80 and 160 (`GenericOwnerShootOntoFoe`, T01-143) | `projectile-classes.md` |

#### 5.3 The damage amount

`amount = trunc(en_slBulletDamage × multiplier)` where the multiplier is **4.0** (single player and co-op) or **2.5** (deathmatch-type session) while the owner has the serious-damage power-up (`PUI_SERIOUS_DAMAGE`), else **1.0** (`CPlayerPuppetEntity::GetApplyDamageMultiplier`; `players-session-classes.md` section 7). The conversion is the compiler's float-to-integer cast: **truncation toward zero**, so 15 × 2.5 = 37.5 gives **37** (**medium**: one reading of the conversion routine; the other multiplications by the difficulty are on the target side, T01-140).

### 6. Bullet types and trails (**medium**)

| `BulletType` | Name | Flight | Trail | Used by |
|---|---|---|---|---|
| 0 | `BT_PHYSICAL` | a `CBulletEntity` flies at `en_fBulletVelocity` to `en_fBulletShootDistance` | none | **one enemy attack** (`helicopter.ep`, 2 bullets) and nothing else in the data |
| 1 | `BT_RAY_STATIC_TRAIL` | ray, immediate | **static trail** | Colt, Uzi, sniper; 6 enemy files (`fighter01_mental`, `fighter02_mental`, `kwongo_machinegun`, `turret_machinegun`, `turret_machinegun_ctf`, `zombiesoldier`; 1 bullet) and `zombieagent`, `zombieagent_big` (6 bullets) |
| 2 | `BT_RAY_MOVING_TRAIL` | ray, immediate | **moving trail** | minigun, both shotguns; `playerhelicopter.ep` (2 bullets) |
| 3 | `BT_RAY_NO_TRAIL` | ray, immediate | none | `worm.ep` (1 bullet) |

**Trails** (from the global `CBulletParameters`, `projectile-classes.md` 6.6; **medium**): a **static** trail lives `sbt_fLifeTime` = 0.25 s; a **moving** trail lives **length / `mbt_fTrailVelocity`** = length / 30 s, where length is the trail length of 5.1 step 5 (the value for velocity 0 was not read). The trail is a cosmetic entity with the two textures `BulletTrail_01` (static) and `BulletTrail_02` (moving) of the effect textures; it is not replicated (every machine that runs the shot spawns its own). Width, fade and texture repeat are in `projectile-classes.md` 6.5.

### 7. The engine's ray cast (T01-220, partial; **medium** unless said)

`SeriousEngine::cldCheckRay` and `cldContinueRay` (`Engine.dll`, read in full) are both thin dispatchers: each updates a small amount of static state (a cleanup-list registration, or copying the previous hit's end point into the new ray's start for `cldContinueRay`) and then calls the same internal result handler, which:

- clears a per-step **"touched" flag** (one bit at a fixed offset) on every entity recorded as a touch candidate so far, then frees that candidate list — consistent with the engine tracking **several candidate entities per ray** and de-duplicating by entity, not just returning the single nearest hit blindly;
- reads back a global, **confirmed by name**, `SeriousEngine::ray_fHitDistance`: if it is negative the function clamps it to 0 before anything else reads it, and the function's own return value (hit / no hit) is this same "was something recorded" check. This is the field a caller like `FireBullets` ultimately uses for "how far along the ray" the hit was.

The ray-query object that `CShooterEntity::FireBullets` passes to the cast (its reset helper was read in full) has, among its fields: **two independent handle slots** reset to a null handle (plausibly the avatar and the avatar's mechanism, matching the open question's wording, but which field is which was not confirmed) and **two independent `IDENT` slots** reset to the engine's invalid-identifier sentinel — the two category fields section 5.1 describes. The object also carries three repeated default vectors and a few more handle/flag fields whose roles were not identified.

**Still open:** the exact geometric test (how a hull is intersected against the thick/thin combination), which of the handle fields is the avatar and which the avatar's mechanism, how the surviving multiple candidates are compared to pick the first (nearest) hit, and the precise contents and layout of "the hit record" at the engine's own level (the shooter-side reading in section 5.1, from `FireBullets`'s own use of the result, already gives entity, hull, hit point and — via the material event — a surface normal, but this was not cross-checked against the engine's own structure).

### 8. Enemy attacks that have a bullet description (script, section 3; **high** for the counts)

168 `CBulletFireing` objects in 90 `.ep` files; damage type 19 (`DMT_BULLET`) in all 168; bullet velocity -1 in all; reach -1 (160), 1000 (6), 800 (2); bullet type 0 in 124, 1 in 42, 2 in 1, 3 in 1. **Only 22 of the 168 have a pellet count above 0** and so can fire: 1 object of type 0 (`helicopter.ep`, 2), 19 of type 1 (17 with 1 bullet in 6 files, 2 with 6 bullets in `zombieagent.ep` and `zombieagent_big.ep`), 1 of type 2 (`playerhelicopter.ep`, 2), 1 of type 3 (`worm.ep`, 1). The 146 objects with 0 bullets are inert (their bullet type, the default 0, is not meaningful). How an enemy's attack uses them (aiming, the shooter's `GenericOwnerShootOntoFoe`) is T01-143.

### 9. Net behaviour (**medium**)

`CShooterEntity`, `CBulletEntity` and `CBulletTrailEntity` are not replicated (`projectile-classes.md` section 9). **The player's own weapon runs `FireBullets` on its own machine** (`weapon-classes.md` section 9), so on a client the shot's impulse, effect and trail are local; **only the host delivers damage**. A client therefore sees impacts and trails of its own shots at once, and a hit is real only when the host's own shot agrees (the host runs the same weapon from the replicated fire state).

## Invariants

1. Each of the six weapons has a bullet type of 1 or 2 (12 of 12 files); no weapon fires a flying bullet.
2. The pellet count of the file equals the size of the weapon's pattern table: 7 and 14 (4 of 4 files).
3. `bswp_fScatterRadiusMin` is not above `bswp_fScatterRadiusMax` in any of the 12 files, and `D` (scatter distance) is above 0.001 in all 12.
4. Impulse and up ratio are -1 in all 12 files; the damage of every hitscan weapon is a positive integer.
5. The 11 other weapons have no scatter member (script).
6. A bullet description with a pellet count of 0 never fires (146 of 168).
7. Reject a hitscan weapon without its shooter: the weapon does not shoot (section 2.1).

## Open questions

Ranked by what blocks an implementer. New task IDs are appended to `docs/roadmap/T01-reverse-engineering.md`.

1. **The engine's ray cast.** T01-220 confirms `cldCheckRay`/`cldContinueRay`'s shared result handler, the `ray_fHitDistance` global, and that the ray-query object has two independent category fields (matching `bullet`/`bullet_no_solids`) and tracks several touched candidates, not just one. **Still open**: which of two reset handle fields is the avatar and which the avatar's mechanism, the exact geometric test, and how several candidates are compared to pick the first (nearest) hit — the one number that decides which entities a 0.5 m ray hits. Left for a further pass.
2. **Run-time checks on the original** of the numbers of this page (the pellet fan, the scatter distribution, the damage rounding, the ray radius, the placement of the shot): the existing weapon run-time task **T01-124** takes them.
3. The **view origin** of the player used by section 3 (`GetViewOrigin`, the head-bob animation and the weapon-in-view matrix) and the owner's `GetViewTarget` of an auto-aimed entity: **T01-121** (weapon view) and `camera-view.md`; the auto-aim itself (`ProcessAutoAiming`) is not specified anywhere yet: **T01-221.**
4. The **start point of the wall test for owners that are not puppets** (turrets, aircraft, hovercraft with an embedded shooter) and the shooter's use by enemies (`GenericOwnerShootOntoFoe`, aiming, prediction, the clamp of the angle): **T01-143** (puppet attacks).
5. What a **target** does with the damage record and the impulse (`OnReceiveDamage`, `ReceiveForceImpulse`): **T01-140**.
6. Whether the first-shot fallback of the shotguns can ever run (for example after a saved game restores a weapon without its flag): not determined; the effect would be the double shotgun's pattern for the auto shotgun. Left to **T01-124**.

## Test vectors

Derived from the data and the formulas above; **not** run on the original.

1. **Damage with the serious-damage power-up.** Colt, single player: 10 × 4.0 = **40**; Colt, deathmatch (file 15, multiplier 2.5): 37.5 truncated = **37**; double shotgun, single player: 12 × 4.0 = **48** per pellet, 14 pellets; auto shotgun, deathmatch: 10 × 2.5 = **25**; sniper zoomed in single player: 300 × 4.0 = **1200**; the same sniper unzoomed: 75 × 4.0 = 300. Without the power-up the multiplier is 1.0.
2. **Scatter ring (Uzi, single player).** D = 100, Rmin = 0.1, Rmax = 2: `r0 = 0.001`, `r1 = 0.02`. With `u1 = 0.25` and `u2 = 0.25`: `r = 0.001 + 0.019 × 0.5 = 0.0105`, `a = 90 degrees`, so the offset is (0, 0.0105): the shot points **0.60 degrees above** the aim (`atan(0.0105)`). With `u1 = 0` the offset has radius 0.001 (0.057 degrees); with `u1 → 1` radius 0.02 (1.146 degrees).
3. **Exact shots.** Colt single player: D = 10, both radii 0: `r0 = r1 = 0` so the direction is the aim exactly for every `u1`, `u2`. The same for the sniper.
4. **Auto shotgun fan (single player).** Pellet 3 (entry (-0.08, -0.02), stretch (0.5, 0.5)): pattern point (-0.04, -0.01); the ring adds a radius of at most 0.5 / 100 = 0.005: the pellet's direction is `(-0.04 ± 0.005, -0.01 ± 0.005, -1)` normalised, i.e. about **2.3 degrees left and 0.6 degrees down**, blurred by at most 0.29 degrees. Pellet 7 (not in this table, 7 entries) would repeat entry 0.
5. **Double shotgun pellet 13.** Entry (0.12, -0.045) × stretch 0.75 = (0.09, -0.03375): 5.1 degrees right and 1.9 degrees down.
6. **Ray radius.** Single player Colt 0.5; deathmatch Colt 0.1; minigun 0.5 in both; auto and double shotgun 0.3 in deathmatch and 0.5 in single player; sniper 0 in both.
7. **Impulse.** A shot with member values (-1, -1) pushes with impulse 250 along `normalize(forward + (0, 0.5, 0))`; a hitscan weapon with an up ratio of 0 pushes along the forward direction only.
8. **Trail.** A minigun shot that hits a wall 15 m away makes a moving trail of length 15 that lives 15 / 30 = 0.5 s; the same shot into the open sky is a trail of length 300 (the range) living 10 s. A Colt shot (static) lives 0.25 s whatever the distance.
9. **Wall test.** A Colt fired while its muzzle is behind a thin wall (the line from the eye to the muzzle hits the wall): the wall is the hit, the damage record goes to the wall's entity, the `BulletHit` effect is spawned on the wall, and the Colt trail is as long as the distance from the **ray's start** to that hit.
