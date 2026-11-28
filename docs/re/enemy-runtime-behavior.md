# Enemy run-time behaviour: attack choice, speeds, difficulty and kick/knock-out (T01-076)

- **Roadmap task:** T01-076 (unblocks the oracle capture T01-434; the decision formula itself is `ai.md` open question 2, closed by T01-126)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for every number quoted from the data (the join script, section Evidence) and for the cross-references to pages that already read the binary (`puppet-record-members.md`, `projectile-classes.md`, `puppet-classes.md`, `player-rules.md`, `puppet-damage.md`); **medium** where this page's own reading adds something (the fire-power argument of a puppet's shot, section 5) or where it carries forward a cross-referenced page's own medium rating (the difficulty damage multiplier, section 3); **low/open** for the two items no page has read yet (the difficulty health/speed multipliers' application point, section 3; the kick's own code path, section 4). **Nothing was run on the original game**: every timing number below is a data value or a formula from a decompiled function, not a measurement, and the oracle capture T01-434 is the way to check them.

Prerequisites: [`enemies.md`](enemies.md) (the parameter-file side: roster, attacks, kicking, knock-out, speeds — this page corrects two of its open questions), [`ai.md`](ai.md) (the `.cb` structure: states, bands, movers, actions, moments), [`puppet-attacks.md`](puppet-attacks.md) (the generic shooting/melee/leap/run-over chains and their timing formulas), [`puppet-record-members.md`](puppet-record-members.md) section 8 (the environment record members), [`puppet-legged-classes.md`](puppet-legged-classes.md) and [`puppet-legged-sequences.md`](puppet-legged-sequences.md) (knock-out), [`puppet-classes.md`](puppet-classes.md) (the `EnemySize` enumeration), [`projectile-classes.md`](projectile-classes.md) (the projectile members, including the fire-power pairs), [`player-rules.md`](player-rules.md) (the difficulty table), [`puppet-damage.md`](puppet-damage.md) (where `dif_fPlayerDamageMultiplier` is actually read and applied), [`behavior-action-classes.md`](behavior-action-classes.md) (the action member names used to join `.cb` to `.ep`).

## Summary

This page answers the six items of its roadmap line by **joining two files that were each already specified on their own** (a character's `.cb` behavior and its `.ep` parameters) and by **resolving two open questions that later pages had already answered without the cross-reference being made**:

1. **Attack choice and timing per enemy.** The structure is `ai.md`'s (bands, movers, actions) and the attack timing formulas are `puppet-attacks.md`'s; this page's own contribution is the **join**: for every character whose `.cb` file matches an `.ep` file (53 of them; `tools/re/enemy_runtime_verify.py`), which band runs which attack at which distance, and that attack's own timing numbers (start distance, wait-before, aim time, wait-after). The **formula that picks among several candidates** (weights, validity terms, frequency graphs) is still not run: that is `ai.md` open question 2, closed by T01-126, and needs the oracle capture T01-434.
2. **The land speed members are not forward/backward/side speed.** `puppet-record-members.md` (T01-151, a later page) already read the declared names: `.3` = `pep_fSpeedMin`, `.4` = `pep_fSpeedMax` (a **range** the puppet's nominal speed is drawn from once, not three directions) and `.7` = `pep_fWalkVelocity` (the **walk/run threshold of the animation chooser**, unrelated to the actual speed cap). `enemies.md`'s "probably the maximum forward, backward and side speeds" guess is **superseded**; this page carries the correction back (section 2).
3. **Difficulty scaling** of enemy health and speed, and of the damage the player takes from an enemy, is a **data table** already recorded in `player-rules.md` (`ggp_gdGameDifficulties`). The **damage** column is now confirmed in code by `puppet-damage.md` (T01-140, merged after this page's data pass started): the player puppet's override of `GetReceiveDamageMultiplier` reads exactly `dif_fPlayerDamageMultiplier`, with no player-count term. The **health and speed** columns are still not confirmed: no function that reads `dif_fEnemyHealthMultiplier` or an enemy-speed difficulty multiplier and applies it to a puppet's health or `SPuppetEnvironmentParams` was found in any page read so far, `puppet-damage.md` included (`GetNominalHealth`'s separate "unresolved game-info formula", `puppet-record-members.md` section 3.1).
4. **Knock-out** is fully specified elsewhere (`puppet-legged-sequences.md`, T01-147); **kicking** is not: `SKickingAbilities` has 15 members (`ep-schema.md`) but only one caller chain was read (`OnKicked` → `CheckImpulseForKnockOut` → `StartKO`, for the **victim's** knock-out gate); nothing here traces **who delivers a kick** (presumably the player's melee weapon or a puppet's own melee blast with a launch heading/pitch, by analogy with `OnAnimEventMeleeDamage`) or reads `ka_fKickedDamageMultiplier`, `ka_fMinKickVelocityRatio`, `ka_fMaxKickVelocityRatio`, `ka_fMinKickPeriod`, `ka_bAllowTrampling`, `ka_bKickAllPuppets` or `ka_bExplodeWhenCantKick`. **Open** (section 4).
5. **The first and second damage sets of a projectile file are fire power 0 and fire power 1**, already read by `projectile-classes.md` (`bp_slDirectDamage`/`Max`, `bp_slAreaDamage`/`Max`, `bp_fDamageHotspot`/`Max`, `bp_fDamageFalloff`/`Max`: "pair = value at fire power 0 and 1"). This page's own reading (`puppet-attacks.md` section 5.2 step 5: `GenericOwnerShootOntoFoe` calls `FireProjectile(parameters, fire power 0, ...)` for every blast's projectile) is that **the puppet attack chain always launches at fire power 0**, so **the second set is dead data on that path, for every enemy, NPC, turret and boss**: it only matters for the player's own chargeable weapons (`weapons.md`, T01-075; `weapon-runtime-behaviour.md`, T01-075, separately found that a projectile's direct and area damage always share one `EDamageType`, a different open question of the same two sets). This closes `enemies.md`'s open question for the enemy side.
6. **The `EnemySize` names are in the data**: `puppet-classes.md` already lists the declared enumeration (0 `ES_EX_SMALL`, 1 `ES_SMALL`, 2 `ES_MEDIUM`, 3 `ES_LARGE`, 4 `ES_X_LARGE`, 5 `ES_BOSS`); `enemies.md`'s "names are not in the data" is **stale** (section 6).
7. **NPC behaviours.** Of the 22 `.ep` files under `Databases/Puppets/NPCs`, only **two** (`simba_chief`, `simba_shaman`) have a `.cb` file of their own name; the rest share **three generic behaviors** (`NPC_Civilian` alignment 3, `NPC_Civilian_Good` alignment 0, `NPC_Civilian_Static` alignment 3) with states for greeting, ignoring and **fleeing** the player and for **approaching and throwing an item** at them. Which placed NPC entity uses which generic file is **not resolved here** (needs the world data, section 7).

## Evidence

- **Data.** `tools/re/enemy_runtime_verify.py` (a few seconds; `SS2_DATA_DIR`): loads all 82 `.cb` files (`ai.md`'s decoder) and the 86 `.ep` files of `enemies/bosses/animals/npcs/enemies_cutsequences/npcs_cutsequences` (`enemies.md`'s decoder), matches them by **file stem** (53 of 82/86 match; verified against `cb_idName`, e.g. `Kleer` ↔ `kleer.cb`/`kleer.ep`), and for every land-medium band and every behavior-wide entry of the `Action` moment, resolves the action's own id member (`as_idShoot`, `mel_idMeleeAttack`, `lof_idLeapAttack`, `rof_idRunOverFoe`; the data's saved class names are the `S` twins `SBAShoot`/`SBAMelee`/`SBALeapOntoFoe`/`SBARunOverFoe`, `behavior-action-classes.md`) against the matching `.ep` file's own attack records (by their own id, `sp_idName`/`ma_idName`/`la_idName`/`rof_idName`). The per-character table is `enemy-runtime-tables.md` (47 characters with at least one resolved action; 132 band/behavior-wide actions resolved, 5 not — section "Notable data findings"). Also applies the difficulty multipliers of `player-rules.md` to a few common enemies' health and speed as a worked table (section 3).
- **Binary.** No new function was decompiled for this page; sections 5 and 4 reuse decisions already read for `puppet-attacks.md` (`SpawnProjectile`'s fire power argument) and `puppet-legged-classes.md`/`puppet-legged-sequences.md` (`OnKicked`, `CheckImpulseForKnockOut`).
- **Not checked:** nothing was run on the original game; the action-selection formula, the difficulty multipliers' application point, and the kick's own delivery code are open (sections 1, 3, 4).

## 1. Attack choice and timing per enemy

Per `ai.md`: a character's behavior has **states**, each with **land/air/water bands** ordered by outer distance, each band naming a **mover** and an **action list** for the `Action` moment (evaluated periodically; the period and the selection rule among several weighted actions are not in the data). A band's action is resolved here against the character's own `.ep` attack records by the id the action names.

**Worked examples** (full tables in `enemy-runtime-tables.md`; timing members are `puppet-attacks.md` section 1's `CShootingProcess` fields, in seconds, -1 = disabled/no gate, 0 = immediate):

- **Kleer** (state `Default`, land bands nearest first): `Slash` at 3 (no mover: stand and act) → melee `Slash` (two blasts, 15 damage, radius 3 each); `Jump` at 6 (`CMCTowardsFoe`) → leap `Jump` (damage 20); `Close`/`Middle`/`Far` at 25/70/500 (`CMCTowardsFoe`) → no attack (close the distance; `Middle` also rolls `CBADefaultAction`, name only). Behavior-wide (any band): shoot `Chainballs` — `start_dist` -1 (always available), `wait_before` 0, `aim_time` 0, `wait_after` 0 (2 blasts): nothing paces this shot beyond the generic chain's per-shot animation length (not in the data; `puppet-attacks.md` section 4 step 4), so Kleer's ranged attack is "as fast as the animation allows," unlike enemies with an explicit pause.
- **Albino** (states `Default`/`Chaser`/`Ridee`, all the same): `Melee` at 8 (`CMCTowardsFoe`) → both `Slap_LeftHand` and `Slap_RightHand` (25 damage, radius 8 each; `ep-schema.md`'s `ma_idName` distinguishes the two melee attacks the parameter file in `enemies.md` counts as "melee 25 r8 twice"). Behavior-wide: shoot `Shoot` — start -1, wait-before 0.5, aim 0, wait-after 0.5.
- **SciFi_Grunt** (state `Default`): `Melee` at 2 (`CMCTowardsFoe`) → melee `Kick` (20 damage, radius 3). Behavior-wide: shoot `Shoot` — start -1, wait-before 0, **aim 0.5**, wait-after 0.1: this one pauses half a second to aim before every shot, matching `ai.md`'s pick of SciFi_Grunt as "a shooter with bands".
- **Rhino**: no shooting process (it has none, `enemies.md`); `.cb` gives only a `Default` state whose single band is a melee attack at its own radius; its speed stands out in section 2's table.

**Notable data findings** (5 of the 137 resolved attempts; the per-character table marks each blank):

- **`levitator.cb`** names a melee attack **`Kick`** and **`spawner.cb`** a melee attack **`HandKick`**, but `levitator.ep` and `spawner.ep` have **no melee attack at all** (`enemies-tables.md`). By `puppet-attacks.md` section 7 step 2, a request for a missing attack id prints a console error and does nothing: these two bands' melee action is **dead** on the shipped data.
- **`zumzum.cb`** (the boss) has an empty `Default` state (no bands at all) and its only `Action`-moment entries are three **behavior-wide** shots (`ShootSpit`, `ShootHands`, `ShootButt`, weights 10/1/1) — but **`zumzum.ep`'s shooting processes are `FlyByFire`, `FlyFire` and `FireNeedle`** (`enemies-tables.md`); none of the three cb names match. `cb_pcbBasedOnCharacter` (behavior inheritance) is null (`ai.md`), so this is not inheritance; by the same rule as above these three actions would always fail to find their shot and do nothing. Either the boss's live behavior comes from elsewhere (a script, a different per-level `.cb`) or this is leftover data from an earlier version of the file. **Not resolved**; worth a one-line check in the oracle capture (does ZumZum shoot at all?).

**Open (needs T01-434 or T01-126):** the action-selection formula itself (weight, validity term, frequency-over-position graph, randomization of the period); whether bands are tried nearest-first as `ai.md` infers; the two zumzum/levitator/spawner findings above.

## 2. Units and order of the land speed members (`SPuppetEnvironmentParams.3/.4/.7`)

**This was already answered by a later page and the correction had not been carried back to `enemies.md`.** `puppet-record-members.md` section 8 (T01-151, reading the DLL's own declared member table of `SPuppetEnvironmentParams`, checked against all 258 environment records):

| Member | Name | Role |
|---|---|---|
| `.3` | `pep_fSpeedMin` | the low end of the puppet's **nominal speed range** |
| `.4` | `pep_fSpeedMax` | the high end |
| `.7` | `pep_fWalkVelocity` | the **walk/run threshold** of the moving-animation chooser (`puppet-moving-anim.md`), not a speed cap |

**Not** "maximum forward, backward and side speed" as `enemies.md` guessed. The actual nominal speed is drawn **once per puppet, for its life**: `GetDeclaredNominalVelocity = SpeedMin + (SpeedMax - SpeedMin) × rnd`, where `rnd` is the puppet's own `en_fRnd` (uniform in [0, 1), 0 for the player). So two placed Kleer (`8.5`/`9.5`/`5`) differ from each other by up to 1 unit/second (the formula's test vector: `rnd` 0.25 gives 8.75), and the "5" is only the speed below which the walk animation plays instead of the run animation — unrelated to how fast the Kleer actually moves. `.1`/`.2` (`pep_fAcceleration`/`Decceleration`) and `.5`/`.6` (`pep_fMovingRotationMin`/`Max`, turn rate) keep their already-correct roles.

**Action required on `enemies.md`:** its "Speeds" column and the sentence "probably the maximum forward, backward and side speeds ... the order and units are not named" should be read as superseded by this page and `puppet-record-members.md` (both already merged); a follow-up editing pass should update the prose (not done by this page, to keep the diff of this task to its own files).

## 3. Difficulty scaling of health, damage and speed

The **data** is `player-rules.md`'s difficulty table (`ggp_gdGameDifficulties`, PC), restated for an enemy:

| Level | Enemy health × | Enemy speed × | Player damage taken × | Think frequency × |
|---|---|---|---|---|
| Tourist | **0.5** | 0.75 | 0.2 | 0.75 |
| Easy | 1 | 0.75 | 0.5 | 0.75 |
| Normal | 1 | 1 | 1 | 1 |
| Hard | 1 | 1.125 | 1.5 | 1.125 |
| Serious | 1 | 1.25 | 2 | 1.75 |

Read as the game's own documentation names them: **enemy health only changes on Tourist** (half); **enemy speed** rises steadily from Easy to Serious (0.75× to 1.25×); the number the file calls "player damage" is the multiplier on **damage the player receives**, i.e. it scales what an enemy's stored attack damage effectively does to the player without changing the stored number itself; **think frequency** (the AI's decision rate, `ai.md`) is the fourth lever, separate from health/speed/damage.

**The damage column is now confirmed by a direct code read, merged after this page's own reading started: `puppet-damage.md` (T01-140) reads `CPuppetEntity::OnReceiveDamage`'s damage-scaling call** (`GetReceiveDamageMultiplier`, dispatched virtually) and finds that **the player puppet overrides it to return `dif_fPlayerDamageMultiplier` of the active difficulty in a non-deathmatch session** (1.0 in deathmatch), applied to the amount a hit deals **before** the armour/health split, with **no player-count term** — so `player-rules.md`'s "player damage" column is exactly this multiplier, confirmed in code, not only read as data. The same function's **co-op/extra-enemy-strength divisor** (`GetReceiveDamageMultiplier`'s other override, for an evil puppet) reduces the damage an **enemy** receives as more players join or the session raises enemy strength — a *second*, independent lever from the difficulty table, and the two are never combined for the same puppet (a player's incoming damage ignores player count; an enemy's incoming damage ignores difficulty, per `puppet-damage.md`'s own reading, which found no difficulty term in the evil-puppet branch).

**The health and speed columns are still not confirmed in code.** No page, including `puppet-damage.md` (whose scope is the damage path, not a puppet's health pool or its movement speed), has found a reader of `dif_fEnemyHealthMultiplier` or a per-difficulty enemy-speed multiplier applied to `pep_fSpeedMin`/`Max` or to `GetNominalHealth`/`php_slMaxHealth`. **Open** (unchanged from this page's first reading, below).

Applied to the worked examples of section 1 (`tools/re/enemy_runtime_verify.py`'s difficulty block):

| Character | Base health | Tourist | Normal | Serious | Base speed (min/max) | Normal | Serious |
|---|---|---|---|---|---|---|---|
| Kleer | 96 | 48 | 96 | 96 | 8.5 / 9.5 | 8.5 / 9.5 | 10.625 / 11.875 |
| Albino | 700 | 350 | 700 | 700 | 6 / 6 | 6 / 6 | 7.5 / 7.5 |
| SciFi_Grunt | 19 | 9.5 | 19 | 19 | 6 / 6 | 6 / 6 | 7.5 / 7.5 |
| Rhino | 225 | 112.5 | 225 | 225 | 18 / 22 | 18 / 22 | 22.5 / 27.5 |

**What is confirmed:** the table's own numbers (**high**, read straight from `GlobalGameParameters.rsc`, `player-rules.md`); that `dif_fPlayerDamageMultiplier` is read and applied, in code, to damage a **player** puppet receives (**medium**, `puppet-damage.md`/T01-140, merged after this page's own data pass).
**What is not confirmed (open):** no function that reads `dif_fEnemyHealthMultiplier` or an enemy-speed difficulty multiplier and applies it to a puppet's health or its `SPuppetEnvironmentParams` was found by any page so far (including `puppet-damage.md`, whose scope stops at the damage path); `GetNominalHealth`'s separate co-op rescale is likewise an "unresolved game-info formula" (`puppet-record-members.md` 3.1, `T01-140`/`T01-162`). It is possible the speed multiplier is never applied to `pep_fSpeedMin/Max` at all and the apparent difficulty difference is **entirely** the think-frequency and reaction-delay columns (`player-rules.md`) acting on how **often** and how **soon** an enemy attacks, not how fast it walks. **The oracle capture T01-434 is the direct way to settle this**: spawn or observe the same enemy on two difficulties and compare its measured health, speed and time-to-first-attack.

## 4. Knock-out and kick mechanics

**Knock-out** (victim side) is specified in full by `puppet-legged-sequences.md` (T01-147): a hit with an impulse vector longer than the file's `kop_fKOKickImpulse` (`SKnockOutParams`, `enemies.md`: 500 to 10,000,000, 4 enemies allow it) starts it, gated at 0.25 s since the last one (`StartKO`), and it lasts at least `kop_fKOSleep` (1 or 2 s) before the get-up sequence. This page adds nothing new here; see that page for the numbers.

**Kicking** (`SKickingAbilities`, `ep-schema.md`, 15 members, 126 files) is only **partly** traced:

| Member | Values in the data | Traced? |
|---|---|---|
| `ka_bIsKicker` | 0/1 | yes — `enemies.md`: 26 of 59 enemies/bosses/animals |
| `ka_slMinKickDamage`/`MaxKickDamage` | 1 to 1e7 | yes — the damage range a kick deals (`enemies.md`) |
| `ka_fKickImpulse` | 0 to 4000 | yes — named only |
| `ka_fMinKickPeriod` | 0 to 60 s | yes — named only ("a minimum period 1 s" in `enemies.md`'s examples) |
| `ka_fLaunchAngleH`/`V` | 0–75° / 15–20° | yes — named only |
| `ka_bAllowTrampling` | 0/1 | **no reader found** |
| `ka_fKickedDamageMultiplier` | 0, 0.05, 0.1, 1 | **no reader found** — by name, how much a **kicked** puppet's kick-damage is itself scaled (a victim that is a kicker too?) |
| `ka_bKickAllPuppets` | 0/1 | **no reader found** |
| `ka_bExplodeWhenCantKick` | 0/1 | **no reader found** |
| `ka_fMinKickVelocityRatio`/`MaxKickVelocityRatio` | -1..0.5 / 0,1,2 | **no reader found** — by name, a speed window the kicker must be moving within to deliver a kick |
| `ka_fMinKickPeriodNonPuppets` | 0..5 s | **no reader found** |
| `ka_fEnvDamageMultiplier` | 0.5..10,000 | **no reader found** |

**Caution: "kicker" is overloaded in this codebase.** `puppet-damage.md` (T01-140) reads the debug text `"Kicker launch impulse"` inside `OnHullTouched`, but that is a **world kicker** — a launcher material/flag on level geometry (a fan, a bounce pad) that gives any puppet touching it a physics impulse plus a `DMT_KICKING` damage record — unrelated to `SKickingAbilities` or to one puppet kicking another. The only function that touches **this page's** kicking side (`ka_bIsKicker` and its ten relatives) read by any page so far is the hook `OnKicked` (`puppet-legged-classes.md`: "the puppet was kicked (can start a knock-out)") — i.e. what is read is the **victim's** reaction (it may enter knock-out), not **how a kicker decides to kick, how its velocity and the ten untraced members above combine into the delivered impulse, or whether the player's own run-over/trampling counts**. **Open**, a new small follow-up (section Open questions).

## 5. The first and second damage sets of a projectile file

`projectile-classes.md` section 4 already names them: every `SGenericProjectileParams` carries a value **at fire power 0** (`bp_slDirectDamage`, `bp_slAreaDamage`, `bp_fDamageHotspot`, `bp_fDamageFalloff`, and `bp_fStretch`) and the matching value **at fire power 1** (`bp_slDirectDamageMax`, `bp_slAreaDamageMax`, `bp_fDamageHotspotMax`, `bp_fDamageFalloffMax`, `bp_fStretchMax`) — the two sets `enemies.md`/`weapons.md` call "p.2/p.3" and "p.11/p.12" (plus the radii p.7/p.8 and p.13/p.14). Fire power is the player's charge level for a chargeable weapon (`weapons.md`); **the puppet attack chain that fires a blast's projectile always launches it at fire power 0**: `puppet-attacks.md` section 5.2 step 5 reads the call made from `OnAnimEventShootingBlast`'s shooter (`GenericOwnerShootOntoFoe`) as `FireProjectile(parameters, fire power 0, launch speed = |v|)`, literally — the only path a `CShootingProcess` blast (the generic shooting chain of section 1) can reach a projectile through. (`FireProjectileFrom`/`ThrowProjectileFrom`/`SpawnProjectile`, `puppet-attacks.md` section 6, are a **different** path used by weapon `OnFire` functions and the player puppet's own `DoAttack`, not by a puppet's generic shooting chain; they are out of scope here, T01-119/T01-264.) **So, for every character this page covers, the second set never applies to a blast's own projectile**: a puppet's `TankRocket_Big` always deals its fire-power-0 numbers (direct 50, range 50), and the fire-power-1 numbers stored alongside it (`enemies-tables.md` shows them as the value after the slash) are dead data on the enemy side. This closes `enemies.md` open question 1's "roles of the first and second damage sets" and the matching line of `weapons.md`'s open questions for every projectile a puppet's `CShootingProcess` fires.

## 6. The `EnemySize` names

`puppet-classes.md` (reading the DLL's declared enumeration table) already gives the names `enemies.md` said were missing:

| Value | Name | Characters (from `enemies.md`) |
|---|---|---|
| 0 | `ES_EX_SMALL` | fido, lizard, cerberus, zumzumfly |
| 1 | `ES_SMALL` | witch, and three others (of the 23 files that explicitly store the member; it is also the class default kept by the other 103 files) |
| 2 | `ES_MEDIUM` | barbarian, bull soldier, centaur, rhino, tank, zombie soldier |
| 3 | `ES_LARGE` | albino, gigant neanderthal |
| 4 | `ES_X_LARGE` | tank general, tmech |
| 5 | `ES_BOSS` | big spider, tank monster, big tank general |

`enemies.md`'s "names are not in the data" (its open question 2) is now **stale**; it should cite `puppet-classes.md` instead. Whether this member is also the spawner's own `EnemySize` choice (`CSpawnerTweakEnemySize`, `mechanism-classes-spawner-tweaks.md`) is a separate, still-open question (T01-166/T01-380), unrelated to the name table itself.

## 7. NPC behaviours

The 22 `.ep` files under `Databases/Puppets/NPCs` are alignment-0/2/3 by `ai.md`'s `.cb` reading (good/misc, never evil): most have **no attack of their own** beyond `throws` (`enemies-tables.md`: chiche, elvian, zixie, simba variants mostly throw 2–3 items; a few — `elvian_gunner`, `simba_gunner`, `zixie_gunner`, `simba_child/doll/female/male/warrior` — do carry a shoot or melee record identical in shape to an enemy's).

**Only two NPCs have a `.cb` file matching their own name**: `simba_chief.cb` and `simba_shaman.cb` (alignment 0), each with one state and bands that do resolve to their `.ep`'s own `ChiefKlodovik` shooting process (`enemy-runtime-tables.md`). **The other 20 NPC `.ep` files have no matching `.cb` file name.** Three generic behaviors cover civilians instead (`ai.md`'s alignment-0/3 group; read here for their land bands):

| `.cb` | Alignment | States (bands, read) |
|---|---|---|
| `NPC_Civilian` | 3 (misc) | `Default`/`GreetPlayer`/`IgnorePlayer` (one band each, 1000 m, `CMCDontMove`, no attack action); `Frightened` (`Close` at 30 m, `CMCAwayFromFoe` — flee; `Far` at 100 m, `CMCDummy`, action `CBACrouch`); `ThrowItem` (one band, 1000 m, `CMCTowardsFoe` — approach the player, presumably to then throw something, `rsc-ctl.md`'s `CThrowDescription` `ThrowItem`); `Ameba` (100 m, no mover) |
| `NPC_Civilian_Good` | 0 (good) | the same six states |
| `NPC_Civilian_Static` | 3 (misc) | the same plus two more (8 states, `ai.md`'s table) |

So an ordinary civilian's behavioural repertoire is: stand, greet or ignore the player at rest, **run away** when frightened (and crouch once far enough), or **approach and (by the state's name) throw an item** at them — never fight. **Which placed `.wld` entity is given which of the three `.cb` files is not resolved by this page**: it requires reading the world's placement data for each of the 20 unmatched NPC `.ep` files, which `ai.md`'s world sample (6 smallest worlds) does not cover for NPC placements specifically. **Open** (section Open questions).

## Reader invariants

1. A `.cb`/`.ep` pair is matched here by **file stem**, verified against `cb_idName` for the enemies/bosses/animals group (`enemies_verify.py`'s and `ai_verify.py`'s own counts agree with the names they print); a reader that needs the true run-time binding (which placed entity's `pp_` parameters pick which behavior) must use the puppet entity's own behavior reference, not file names — the zumzum and civilian findings above show file-stem matching can be wrong or incomplete.
2. The land speed members are `pep_fSpeedMin`/`Max`/`WalkVelocity` at positions `.3`/`.4`/`.7` in every one of the 258 environment records of the install (`puppet-record-members.md`, re-confirmed by this page's script using the same positions); a reader must not treat them as three directions.
3. A projectile's "second set" (`p.11`–`p.14`) is only read by the engine when the firer's fire power is above 0; a puppet's `CShootingProcess` blast always fires at 0 (`puppet-attacks.md` section 5.2), so a reader implementing enemy attacks may ignore the second set entirely for that path (but must keep it for the player's chargeable weapons, T01-075, and for any other caller not covered here).
4. `EnemySize` is a 6-value enumeration (`ES_EX_SMALL` 0 .. `ES_BOSS` 5), class default 1 (`ES_SMALL`); 103 of 126 puppet files do not store the member and keep the default.

## Open questions

1. **The action-selection formula** (`ai.md` open question 2, closed by **T01-126**): weights, validity terms, the frequency-over-position graph, period randomization, whether bands are tried nearest first; needs the oracle capture **T01-434**.
2. **Whether the difficulty health/speed multipliers are applied to a puppet at all, and where** (the damage column is now confirmed, `puppet-damage.md`/T01-140; no reader found for the health or speed columns; ties to `T01-162`'s still-unresolved `GetNominalHealth` co-op rescale): needs either a decompiled read of the difficulty-selection code or, more directly, **T01-434**'s measurement on two difficulties.
3. **The kick's own delivery code**: who calls `OnKicked` and how the ten untraced `SKickingAbilities` members (`ka_bAllowTrampling`, `ka_fKickedDamageMultiplier`, `ka_bKickAllPuppets`, `ka_bExplodeWhenCantKick`, the two kick-velocity-ratio members, `ka_fMinKickPeriodNonPuppets`, `ka_fEnvDamageMultiplier`) combine into a delivered kick. A small follow-up task is appended to the roadmap for this.
4. **The zumzum/levitator/spawner data findings** (section 1): whether these are genuinely dead actions or the behavior comes from elsewhere; a one-line check in the oracle capture would settle it.
5. **Which placed NPC entity uses which of the three generic civilian behaviors** (section 7): needs the world data for the levels that place the 20 unmatched NPCs, not the 6-world sample `ai.md` already read.
6. **Attack cadence end to end** still needs the model's animation lengths (`.mdl`, not read here) to turn the wait/aim/after numbers of section 1 into a true shots-per-second figure; `puppet-attacks.md` open question 1 (**T01-263**) already covers the run-time half of this.

## Test vectors

All from `tools/re/enemy_runtime_verify.py` (the install's own data; `enemy-runtime-tables.md` has the full per-character output).

- **Kleer** (`Default` state): `Slash` band at 3 m → melee `Slash` (15×3 damage×radius, two blasts); `Jump` band at 6 m → leap `Jump` (damage 20); behavior-wide shoot `Chainballs` (start -1, wait-before 0, aim 0, wait-after 0, 2 blasts).
- **Albino** (`Default`/`Chaser`/`Ridee`, identical): `Melee` band at 8 m → `Slap_LeftHand` and `Slap_RightHand` (25×8 each); behavior-wide shoot `Shoot` (start -1, wait-before 0.5, aim 0, wait-after 0.5).
- **SciFi_Grunt** (`Default`): `Melee` band at 2 m → melee `Kick` (20×3); behavior-wide shoot `Shoot` (start -1, wait-before 0, aim **0.5**, wait-after 0.1).
- **Difficulty** (section 3 table): Kleer health 96 at Normal, 48 at Tourist, unchanged at Hard/Serious; speed 8.5/9.5 at Normal, 10.625/11.875 at Serious.
- **`EnemySize`**: `fido.ep`, `lizard.ep`, `cerberus.ep`, `zumzumfly.ep` → 0 `ES_EX_SMALL`; `tank_monster.ep` → 5 `ES_BOSS`.
- **Fire power**: `TankRocket_Big.ep` (direct 50 at fire power 0, 0 at fire power 1, `enemies-tables.md`) is fired by `tank_general`/`tank_general_big`'s shooting-process blasts through `GenericOwnerShootOntoFoe`, always at fire power 0, so it always deals 50, never the fire-power-1 value, whatever that file stores there.
- **Join counts**: 82 `.cb` files, 86 enemy/boss/animal/NPC `.ep` files, 53 matched by file stem, 132 band/behavior-wide actions resolved to the character's own attack record, 5 not (section 1).
