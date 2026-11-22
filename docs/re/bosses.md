# Boss fights: the seven bosses, their states and the arena scripts (T01-048)

- **Roadmap task:** T01-048 (feeds the AI, level-logic and boss tasks; run-time timing needs the oracle capture T01-038)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what is stored: the seven boss characters (health, healing members, attacks), their behavior files (states, areas, action counts) and which arena world places each boss and what its scripts do to it (all seven arenas, `tools/re/bosses_verify.py`; scripts resolved through the macro entity's variable bindings, `level-logic.md`); **medium** for what the scripts do in order (the tool lists the events, actions and calls addressed to each boss, **not their sequence**); **low** for phase timing, thresholds in script conditions and the recharge rule. Scope: `Puppets/Bosses/*.ep`, the seven `.cb` behavior files and the seven arena worlds.

Prerequisites: `enemies.md` (attacks, projectiles, kicking), `ai.md` (behavior files: states, areas, moments), `level-logic.md` and `level-logic-api.md` (events and actions by class), `wld-macro.md` (macro programs), `effects-pipeline.md`.

## Summary

- **A boss fight has three layers of data.** (1) The **boss character**: an ordinary puppet parameter file in `Puppets/Bosses/` (health 5,000 to 1,000,000, named shooting processes, huge melee blasts, **healing members**, a boss name and a **boss icon**: exactly 7 of the 126 puppet files have an icon, presumably the seven bosses). (2) The **behavior file** (`.cb`): one to four **states** (`Default`, `Agressive`, `Outside`, `Flying`, `Static`, `UpClose`, `ShootingCannonballs`, …), each with distance bands and actions (shoot, melee, local move, run over, animation, change of moving abilities). (3) The **arena script**: a macro program in the boss arena world that **changes the state**, makes the boss invulnerable, lets it recharge and reports health thresholds. **Phases are driven by scripts, not by the behavior file**: no boss behavior has a health-based validity term and none has a state-change action; scripts call the character's `ChangeState` (1 to 4 times in five arenas; not at all for the Dragon and ZumZum).
- **One arena per boss** (7 worlds): `1_8_bossarena` (Kwongo), `2_6_bossarena` (ZumZum), `3_5_bossarena` (Prince Chan), `4_4_bossarena` (Kleerofski), `5_7_bossarena` (Dragon), `6_5_bossarena` (Transformer) and `7_7_mental` (the Pyramid); each places one boss entity. The world info's `RegisterBoss` action occurs 7 times (`level-logic-api.md`: 6 + 1 asynchronous): one per fight (reading; the registered entity is the boss: the health bar and the boss name use it).

## Evidence

`tools/re/bosses_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes); the per-arena script tables are `bosses-tables.md` (generated): all seven boss files and behaviors read, **7 of 59 worlds place a boss** (one entity each), and in each the scripts address the boss through macro variables bound to that entity.

## The bosses (data of the character and its behavior)

| Boss | Health | Healing members (`php_slHealingStart` / `End` / `PerSec`) | Speeds | Attacks (`enemies-tables.md`) | Behavior states (areas; actions) | Ranges (active / sense / threat) |
|---|---|---|---|---|---|---|
| **Kwongo** | 100,000 | 0 / 0 / 0 | 15 / 15 / 15 | `Shoot` the rock `KwongoRock` (range damage 300, speed 500); a throw; kick 1 | `Default` (1; 1 shoot, 1 animation), `UpClose` (1) | 2,500 / 5,000 / 5,000 |
| **ZumZum** | 5,000 | 5,999 / 5,000 / 100 | 12 / 12 / 12 | `FlyByFire` and `FlyFire` (`ZumZumSlime`), `FireNeedle` (`ZumZumNeedle`); melee 30 (r10); run over ×2; kick 50 | `Default` (1; 2 shoot, 1 animation) | 2,500 / 10 / 10 |
| **Prince Chan** | 50,000 | 0 / 0 / 0 | 8 / 8 / 15 | `Breath` from 300 (`PrincChan_Breath`); melee 40 (r30) twice; kick 10 | `Default` (1; animation), `Agressive` (2; 1 shoot, 2 melee) | 2,500 / 50 / 50 |
| **Kleerofski** | 5,000 | 4,999 / 5,000 / 200 | 30 / 30 / 30 | `Fireball` (`KleerofskiFireball`, direct 20, range 10); melee 40 (r12); run over ×1.6; kick 30 | `Default` (4; 1 shoot, 1 melee, 5 local moves, 1 run over), `Agressive` (3; 2 shoot, 1 melee, 1 run over) | 2,500 / 5,000 / 5,000 |
| **Dragon** | 50,000 | 200 / 500 / 1 | 12 / 12 / 12 | `FlyByFireBreath` and `FlyByFireBall` (both from 150; fireball direct 50, range 50); melee 30 (r10); run over ×2; kick 50 | `Default` (2; 3 shoot) | 2,500 / 50 / 50 |
| **Transformer** | 100,000 | 0 / 0 / 0 | 9 / 9 / 15 | `Cannon`, `Rockets` (3 blasts), `Laser` (3 blasts); five melee blasts: 30 (r15), **10,000 (r50)**, 10,000 (r20), 10,000 (r20), 30 (r15) | `Default` (3; 2 shoot, 2 melee), `Outside` (3; 3 shoot, 2 melee), `Flying` (2; 4 shoot, 2 changes of moving abilities), `Static` (3; 5 shoot) | 2,500 / 50 / 50 |
| **Pyramid** (`pyramidslow`) | **1,000,000** | 0 / 0 / 0 | 2.35 / 2.35 / 2.5 | `FireCannons` (3 × `PyramidCannonBall`, direct 30), `FireFireballs` (3 × `PyramidFireball`), `FireRockets` (`PyramidRocket`) | `Default` (0), `ShootingCannonballs`, `ShootingFireballs`, `ShootingRockets` (1 area, 1 shoot each) | 25,000 / 10 / 10 |

- **Healing members.** The three healing members (`SPuppetHealthParams`) are set for **ZumZum, Kleerofski and the Dragon** and 0 for the others: Kleerofski 4,999 → 5,000 at 200 per second (health 5,000: it **regenerates fast after the first hit**), ZumZum 5,999 / 5,000 at 100 per second and the Dragon 200 → 500 at 1 per second (health 50,000: slow). By their names (`Healing Start`, `End`, `PerSec`) these are **the health range in which healing runs and its rate**; the exact rule (whether healing starts below `Start` or above) is not stated (reading: a boss with a healing rate recovers while its health is inside the range).
- **State names show the fight's phases:** the **Pyramid** has one state per weapon (cannonballs, fireballs, rockets: the script switches between them); the **Transformer** has four modes (`Default`, `Outside`, `Flying`, `Static`); `Flying` contains two `CBAChangeMovingAbilities` actions (adding and removing movement abilities: the boss takes off and lands, reading); **Kwongo** has `Default` and `UpClose` (a close-range mode: the script event `UpClose`); **Prince Chan** and **Kleerofski** have `Default` and `Agressive`.

## The arena scripts (what the scripts do to each boss)

`bosses-tables.md` lists, per arena, every event, action, call and sent event addressed to the boss (with the number of uses). Common building blocks across the arenas:

- **Invulnerability and recharging** (ZumZum, Kleerofski): `BeInvulnerable` (5 and 6 uses) with `macAllowRecharging` (3 and 3), the character events `Invulnerable`, `Vulnerable`, `Recharging`, `RechargingOver`, `HealingStarted`, `HealingStopped`, and the **same names sent back** as events by the script's `Recharge`, `RechargeAnimation` and `Recharge_LoopAnimation` functions (the world also has the model events `BossStartedRecharging` and `BossStoppedRecharging`, `level-logic-api.md`; how they connect was not decoded). The boss **becomes invulnerable, plays a recharge animation and regains health, then becomes vulnerable again**.
- **Health thresholds** (Kwongo): the script reads `macGetHealth` and `macGetNominalHealth` (3 each) and **sends `HealthUnder75`, `HealthUnder50` and `HealthUnder10`**, then `FirstPartOver`, `SecondPartOver` and `ThirdPartOver`: the fight has **three parts** divided at 75 %, 50 % and 10 % of health (reading from the names). Other arenas read health 1 to 5 times (`macGetHealth`, `macGetNominalHealth`).
- **State changes:** `ChangeState` is called on the boss in Kwongo (1, asynchronous), Prince Chan (1, asynchronous), Kleerofski (1), the **Transformer (4)** and the **Pyramid (4)**, with `macSetEmotionState` (4) on the Pyramid (its face changes with the phase).
- **Control of the AI:** `macDisableAI` (6 for Kwongo, 3 for the Transformer) and `macEnableAI` (6, 3 and 3 for Kleerofski), `ExecuteAIMoment` (6 for Kwongo, 2 for Prince Chan), `SetTacticEntity` (the boss follows tactic markers: 7 for the Transformer, `LastTacticMarkerReached` 8 times), `PatrolMarkers`, `macTeleport` (8 for the Transformer), `macSetLookTarget` (12 for the Transformer), `macSetThreatSensitivity`.
- **Damage and death:** `ReportDamage` (the script is told when the boss is hurt), `macInflictDamage` (the script itself hurts the boss, 1 to 2 uses in five arenas: scripted damage, for example a gong, `GongDamage` in Prince Chan's arena), `Died` (1 to 4 uses per arena: the end of the fight and the level), `Damaged`.
- **Animations:** `PlayAnimNet` (Kwongo 8, ZumZum 3, Prince Chan 1; a network-synchronised animation), `StopAnimNet`; Kwongo's fists send `LeftFistDown` and `RightFistDown`.
- **Boss sound:** `PlaySchemeSound` (Dragon).

## Reader invariants

1. A boss is an ordinary character (`enemies.md`) plus a **behavior file with named states** plus an **arena script**; load all three.
2. **Phases are changed by the arena script** (`ChangeState` on the boss, `BeInvulnerable`, `macAllowRecharging`), not by the behavior file; a boss without its script stays in its first state.
3. Health values are **base values**; the health bar uses the boss's health and name (`mpp_strBossName` key, `mpp_ptxBossIcon`), registered by the world info (`RegisterBoss`).
4. Treat the healing members as a **regeneration rule** of the character (health range and rate), the recharge as a **scripted** state of the fight.
5. Fail on a boss entity whose parameter file has no behavior, or an arena without its macro entity; each of the seven arenas has exactly one boss entity.

## Test vector

`4_4_bossarena.wld` (Kleerofski): one boss entity using `Puppets/Bosses/Kleerofski.ep` (health 5,000, healing 4,999 / 5,000 / 200, speeds 30, `Fireball` shooting process, melee 40 r12, run over ×1.6); behavior `Kleerofski.cb` with states `Default` (4 areas: 1 shoot, 1 melee, 5 local moves, 1 run over) and `Agressive` (3 areas); the script functions that address it: `Boss`, `Chapter001`, `Chapter002`, `Main`, `PatrolingSuccesful`, `Quote`, `Recharge`, `RechargeAnimation`; its events and actions: `Died` ×2, `Invulnerable` ×2, `Damaged`, `HealingStarted`, `Recharging`, `RechargingOver`, `Vulnerable`; `BeInvulnerable` ×6, `macDisableMovingAbility` ×3, `macEnableAI` ×3, `StopPatrolingMarkers` ×3, `ChangeState` ×1, `macAllowRecharging` ×1 + 2 asynchronous, and the calls `macGetNominalHealth` ×4 and `macGetHealth` ×3.

## Open questions

- **Sequence and timing** (task T01-077): the order in which each script changes states, thresholds that are inside script conditions (only Kwongo's `HealthUnder…` names are visible), recharge timing and durations, what triggers the Transformer's modes, the healing rule (which side of `Start` heals), the use of `macSetEmotionState`, and the fights' reward and level-end logic (`macTheEndSequence`, chapters): oracle capture (T01-038) or a statement-level reading of the macro programs (`wld-macro.md`).
- Whether each arena's `RegisterBoss` names exactly the boss entity (the action argument was not decoded here).
- Boss intros and quotes (`Quote` functions, `SayPuppet`) belong to the cutscene and dialogue specs (`cutscenes.md`, `localization.md`).
