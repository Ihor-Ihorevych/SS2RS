# Enemies, bosses and animals: what a character is in the data (T01-029)

- **Roadmap task:** T01-029 (feeds the AI, spawner and balance tasks; run-time behaviour needs the oracle capture T01-038)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what is stored: the roster, health, score, size class, speed members, every attack (shooting processes with their single blasts, melee blasts, leaps, run-over attacks, throws), kicking, knock-out and death parameters and the projectile files with their damage (all 124 puppet parameter files and the 73 projectile files they fire, `tools/re/enemies_verify.py`; members from the files' own type tables); **medium** for the roles of the numbered members (the three speed members, the second set of projectile numbers); **low** for run-time behaviour: when and how an enemy chooses an attack, how speeds scale with difficulty, how a knock-out works. Scope: `Databases/Puppets/**/*.ep`.

Prerequisites: `ep.md` and `ep-schema.md` (the parameter file), `puppet-stats.md` (health and score by group), `ai.md` (the behavior files `.cb` that choose actions), `weapons.md` (the same projectile and damage members), `vehicles.md`, `effects-pipeline.md` (death parameters, corpse sink), `player-rules.md` (difficulty multipliers).

## Summary

- **A character is one puppet parameter file** (`Databases/Puppets/<group>/<name>.ep`, one root object). **124 files:** enemies 48, NPCs 22, vehicles 21, turrets 11, bosses 7, animals 4, boss turrets 3, cut-scene variants 5, boss vehicle 1, object 1, player 1. **Root classes:** `SLeggedPuppetParams` 81, `SAircraftPuppetParams` 19, `STurretPuppetParams` 14, `SHovercraftPuppetParams` 5, `SPlayerPuppetParams` 2, `SRollingBallPuppetParams` 2, `SGenericLeggedPuppetParams` 1 (`vehicles.md` covers the non-walking classes).
- **Every character has the same named parts** (`SPuppetParams`): health (`SPuppetHealthParams`: `.1` health, healing, health bar), body (`SPuppetBodyParams`: the model, aim points, aim-ability flags), **three environments** (`SPuppetEnvironmentParams`: movement numbers, probably for land, air and water like the three content areas of the behavior file in `ai.md`; not named), `SMovingAbilities` (steering, step heights, allowed fall height), kicking (`SKickingAbilities`), knock-out (`SKnockOutParams`), death (`SPuppetDeathParams`), effects, cameras, `CMiscPupeptParams` (score, boss name, enemy size) and **`CPuppetAttack`**, the list of attacks.
- **Attacks are data:** 123 `CShootingProcess` (one per ranged attack: a name such as `Shoot` or `ThrowAxe`, a start distance, animations, a list of **`CSingleBlast`** each with a bullet-firing record or a **projectile file**), 71 `CMeleeAttack` (each with up to three **`CMeleeBlast`**: damage, radius, impulse), 9 `CLeapAttack`, 13 `CRunOverFoeAttack`, and throw descriptions. **When an attack is chosen** is the behavior file (`.cb`, `ai.md`); the parameter file only says what each attack does.

## Evidence

`tools/re/enemies_verify.py` (read-only, `SS2_DATA_DIR`; about 1 minute); the per-character table is `enemies-tables.md` (generated: class, health, score, size, speeds, attacks, kick damage, exploding damage; plus the 73 projectile files with speed, damage and radii):

- Of the **59 enemies, bosses and animals**: 46 have a ranged attack, 43 melee, 8 leaps, 11 run-over attacks, 1 a throw (`kwongo`), 26 are **kickers**, 4 can be **knocked out**, 20 have a non-zero **exploding damage** (the corpse gibs: `effects-pipeline.md`). Health is 10 to 1,000,000 with a **median of 125**.
- **Ranged attacks:** 22 bullet blasts (bullet count 1 to 6, damage 2 to 50 per bullet) and **73 distinct projectile files** (all 73 found and read). **Melee blast damage:** 15 (29 blasts), 20 (18), 25 (9), 10 (5), 40 (5), 30 (4), 100 (1: `tank_monster`, radius 20) and **10,000** (3: the `transformer` boss, radii 50, 20 and 20: its stomps kill anything in reach).
- **Size class** (`CMiscPupeptParams.8`, an `EnemySize` enumeration with values 0 to 5 set in 23 files): 0 for fido, lizard, cerberus and zumzumfly; 1 for the witch and three others; 2 for the barbarian, bull soldier, centaur, rhino, tank and zombie soldier; 3 for the albino and the gigant neanderthal; 4 for the tank general and tmech; 5 for the big spider, the tank monster and the big tank general: small to huge; names are not in the data.

## The roster (examples; every file is in `enemies-tables.md`)

| Character | Health | Score | Speeds (`.3` / `.4` / `.7`) | Attacks |
|---|---|---|---|---|
| `kleer` (worked example) | 96 | 100 | 8.5 / 9.5 / 5 | `Chainballs`: 2 × `ChainedBall` (speed 30, damage 10); melee 15 (radius 3) twice; leap 20 |
| `fido` | 20 | 50 | 10 / 10 / 4 | melee 15 (r2); leap 15; run over ×1.25; kicks 10 |
| `rhino` | 225 | 300 | 18 / 22 / 12 | melee 20 (r5); run over ×1; kicks 20: the fastest ordinary enemy |
| `neanderthal` | 96 | 100 | 5 / 5 / 99 | melee 20 and 10 (r3); leap 30; run over ×1.3 |
| `albino` | 700 | 500 | 6 / 6 / 3 | `Shoot`: `AlbinoMagicBall` (direct 5, range 15); melee 25 (r8) twice; kicks 100 |
| `barbarian` | 380 | 200 | 3 / 3 / 10 | `ThrowAxe`: `Axe` (direct 20); melee 20 (r6) |
| `tank` / `tank_general` / `tank_monster` | 252 / 2,000 / 5,000 | 200 / 1,000 / 5,000 | 5 / 5 / 3 | two-shot salvos: `TankPlasma` (speed 80, direct 10), `TankRocket_Big` (direct 50), `TankRocket_Monster` (direct 50); melee 25 (r4), 25, **100 (r20)** |
| `zombiesoldier` | 130 | 150 | 2.5 / 2.5 / 8 | `Minigun`: 1 bullet of 2; melee 20 |
| `zombieagent` | 55 | 50 | 2.5 / 2.5 / 4.5 | `Shotgun`: 6 bullets of 4; melee 20 |
| `scifi_grunt` | 19 | 10 | 6 / 6 / 4 | `ShortPlasmaBeam`; melee 20 |
| `spider_big` | 2,500 | 750 | 6 / 6 / 5 | `BigAcidNet` (direct 10, range 20); melee 20 (r18); kicks 50 |
| bosses: `dragon`, `kleerofski`, `zumzum`, `kwongo`, `princchan`, `transformer`, `pyramidslow` | 50,000 / 5,000 / 5,000 / 100,000 / 50,000 / 100,000 / **1,000,000** | 5,000 each | up to 30 (kleerofski) | several named shooting processes each (`FlyByFire`, `Breath`, `FireCannons`, `Rockets`, `Laser`); `pyramidslow` fires three cannon, three fireball and rocket salvos |

Details of the examples: `Kleer` has 2 melee blasts (15, radius 3) and 1 leap (damage 20), is not a kicker, and has an exploding damage of 0; the boss **health values are base values before the difficulty multiplier** (`player-rules.md`); `kwongo` throws a rock (`KwongoRock`: range damage 300, speed 500) and its catapult is a turret (`vehicles.md`).

- **Speeds.** The land environment (`SPuppetEnvironmentParams`, the first of three per file) has seven numbers; `.3`, `.4`, `.7` are the **speed members** (8.5 / 9.5 / 5 for the Kleer; 7.5 / 7.5 / 3 for the player), probably the maximum forward, backward and side speeds in metres per second (the order and units are **not named**); `.1` and `.2` are acceleration-like (40 / 40 for the Kleer), `.5` and `.6` rotation-like (1,000 to 1,100) and `pep_fStandingRotationMin/Max` the turning while standing. Flying and hovering characters take their numbers from `SAircraftPuppetParams` and `SHovercraftPuppetParams` (`vehicles.md`), and they appear blank in the speed column.
- **Enemy projectiles** (`enemies-tables.md`): speed 0 to 500 (30 to 100 for most; the pyramid cannon ball 0: it is thrown; the Kwongo rock 500), direct damage 0 to 6,000 with 5, 10, 20 as the common values (`BlueLaser` 10, `TankRocket` 20, `TankRocket_Big` 50, `DragonFireBall` 50 direct and 50 range, `KwongoRock` 300 range), radii as in `weapons.md`; the second set of damage members (`p.11` to `p.14`) is equal to the first in many files and 0 in others: the same open question as for the weapons.
- **Kicking** (`SKickingAbilities`): `ka_bIsKicker` and a damage range `ka_slMinKickDamage` to `ka_slMaxKickDamage` (the albino 100–100, the rhino 20–20, the bull soldier and tank 5–30 and 10–30, the tank general 20–60), a kick impulse (1,000 for the Kleer), a minimum period 1 s and launch angles 15°; **knock-out** (`SKnockOutParams`): `kop_bCanBeInKO`, a kick impulse threshold 500 to 10⁷ and a sleep time 1 or 2 s (4 characters).

## Reader invariants

1. The character is defined by **one `.ep` file**; load its root class first, then the named sub-structures; ignore members that are not in the type table of the file (compare names, not numbers).
2. Run each attack as given: a **shooting process** fires its blasts (bullets: count × damage within the reach; or a projectile file with its own damage), a **melee attack** applies each blast (damage in a radius with an impulse), a **leap** and a **run-over** are movement attacks with a damage.
3. Health, score and damage are **base values**; the difficulty multipliers and the deathmatch variants are applied by the game rules (`player-rules.md`, `game-modes.md`).
4. Attack choice and timing are **not** in this file: the behavior file (`ai.md`) names the attack by its process name.
5. Fail on an attack class outside the five listed (`CShootingProcess`, `CMeleeAttack`, `CLeapAttack`, `CRunOverFoeAttack`, throws) when it appears under `CPuppetAttack`.

## Test vector

`Databases/Puppets/Enemies/Kleer.ep` (`SLeggedPuppetParams`): health 96, score 100, no enemy size, model `Kleer.mdl`, land speeds 8.5 / 9.5 / 5, `ma_fAllowedFallHeight` 2.5, `ma_fStepUpHeight2` 2, step down 8, not a kicker (kick impulse 1,000, damage 5–30, period 1), not knock-out capable, exploding damage 0; attacks: one shooting process `Chainballs` with two single blasts firing `ChainedBall.ep` (speed 30, direct damage 10), two melee blasts (damage 15, radius 3) and one leap (damage 20). `Databases/Puppets/Bosses/Transformer.ep`: health 100,000, speeds 9 / 9 / 15, shooting processes `Cannon`, `Rockets` (three blasts), `Laser`, and five melee blasts: 30 (r15), **10,000 (r50)**, 10,000 (r20), 10,000 (r20), 30 (r15).

## Open questions

- **Everything behavioural** (task T01-076): which attack an enemy chooses and when (`ai.md` has the structure, the timings need the oracle capture), the units and order of the speed members, how difficulty scales health, damage and speed, the knock-out and kick mechanics, spawn effects, the behaviors of the 22 NPCs (they are not hostile) and the roles of the first and second damage sets of the projectile files.
- The names of the `EnemySize` values (0 to 5) and the damage type numbers (`mb_dtDamageType` 4, `bf_dtDamageType` 19, `bp_dtDirectDamageType` 8, 14, 18).
- Per-enemy sound and animation events (the `.mdl` and `.ans` files: `mdl.md`, `ans.md`) are not part of the parameter file.
