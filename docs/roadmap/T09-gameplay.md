# T09 — Gameplay (`ss2-game`)

Gameplay logic lives in `Sam2Game.dll` in the original; we **re-implement behavior from specs
(T01-026…030)**, using `.ep` entity params as data. Weapons and enemies parallelize extremely well:
**one task per weapon, one per enemy, one per level-logic feature**; each owner needs only the
framework (T09-001/005/010) plus their spec. Everything here runs in the server sim and is
replication-aware (guide: T08-032). Mark which fields are predicted on the local client.

## Framework (blocks everything else here)
- [ ] **T09-001** · Gameplay framework: class registry, spawn from `.wld` records + `.ep` params, component set (Transform, Health, Team, Inventory, Weapon, AI…), lifecycle hooks · `L` · `P0` · Deps: T08-002
- [ ] **T09-002** · Game rules/mode manager: campaign (SP+co-op), difficulty, spawn rules, win/lose, level flow; mode trait for later survival/DM · `M` · `P1` · Deps: T01-037
- [ ] **T09-003** · Damage system: damage types, resistances, armor absorption, hit zones, knockback, kill credit, events · `L` · `P0` · Deps: T07-010
- [ ] **T09-004** · Hit reactions & death: pain/death animation selection, gibs, ragdoll handoff, corpse cleanup budgets · `M` · `P1`

## Player
- [ ] **T09-006** · Player entity: spawn/respawn, health/armor, inventory & ammo caps, powerups, scoring/stats · `L` · `P0` · Deps: T01-027
- [ ] **T09-007** · Player movement integration: input → controller (T07-005), crouch/sprint/jump, fall damage, footstep events · `M` · `P0`
- [ ] **T09-008** · Player view: camera, FOV, head bob/sway, viewmodel animation hookup (T01-042) · `M` · `P0`
- [ ] **T09-009** · Player models/skins & character select (`Models/Player/*` incl. `.nfo` display names) · `M` · `P2`
- [ ] **T09-011** · Netricsa (in-game info device): data, entries, unlock logic (UI in T10-008) · `M` · `P2`
- [ ] **T09-012** · Co-op player rules in game layer: revive/respawn, shared pickups, ping/markers, ghosts (with T08-023) · `L` · `P1`

## Weapon framework & weapons
- [ ] **T09-005** · Weapon framework: state machine (raise/idle/fire/reload/lower/switch), anim-event-driven timing, ammo, fire modes, dual-wield support, client-prediction hooks · `XL` → split · `P0`
- [ ] **T09-013** · Projectile framework: kinematic/physics projectiles, lifetimes, impact → explosion/area damage, homing, bounce; replication-friendly (spawn event + deterministic flight) · `L` · `P0`
- [ ] **T09-014** · Effects hooks: muzzle flash, tracers, shell eject, impact FX via `.pfx`/`.mat`, sounds via `.scs` · `M` · `P1`
- [ ] **T09-015** · **Weapon tasks — one per weapon** from T01-028 (`T09-015a…`, each `L` · `P1`): start with a pistol-class and a shotgun-class weapon (`AutoShotgun` exists in the data), then automatic, explosive, energy, melee, heavy. Each: behavior reproduced, numbers validated vs oracle, animations/sounds/FX wired, replication + prediction done, scenario test.
- [ ] **T09-016** · Pickups: ammo/health/armor/powerups/keys, respawn rules, magnetism, replication · `M` · `P0`

## AI & enemies
- [ ] **T09-010** · AI framework: perception (sight/hearing), blackboard, behavior/state machines, steering, target selection across multiple players, aggro/leash · `XL` → split · `P0` · Deps: T07-013, T07-024, T01-025
- [x] **T09-017** · AI performance: rate-scaling of far enemies, budgeted path requests, horde stress test (hundreds): `ss2_game::ai_budget` (`docs/AI_BUDGET.md`): distance and visibility bands for the think rate, a staggered and capped scheduler that serves the most overdue first and tells each thought the real elapsed ticks, an ageing path-request budget, and an 800-agent test of the scheduler; **no AI, enemy or path finder exists yet to use it, so the horde test is of toy agents, not of real enemies, and nothing was measured** (T09-017b) · `L` · `P1`
- [ ] **T09-017b** · Use the AI budget: call the scheduler and the path budget from the AI framework (T09-010) and the path finder (T07-013), compute the distance and visibility inputs, a real horde stress test with enemies on the scenario harness (T09-029b), measure the cost of a thought and choose the caps and bands on PC and a phone · `M` · `P1` · Deps: T09-017, T09-010, T07-013
- [ ] **T09-018** · Spawners & waves: trigger spawners, teleport-in effects, wave scripts, count/difficulty scaling by player count · `L` · `P1` · Deps: T01-030
- [ ] **T09-019** · **Enemy tasks — one per enemy** from T01-029 (`T09-019a…`, each `L` · `P1`): movement kit, attacks (melee/ranged/special), pain/death, animations (`.ans`), sounds, FX, difficulty tables, replication, scenario tests. Start with 2 simple enemies for M5.
- [ ] **T09-020** · **Boss tasks — one per boss** (`XL` · `P2`): phases, scripted arenas, special mechanics (from T01-048)

## Level logic
- [ ] **T09-021** · Trigger/event I/O system: outputs → inputs, delays, once/multi, activators, conditions · `L` · `P0` · Deps: T01-030
- [ ] **T09-022** · Mechanism gameplay (doors, lifts, switches, locks/keys) on top of T07-012 · `M` · `P1`
- [x] **T09-023** · Checkpoints, secrets, level stats, kill/secret counters, end-of-level summary: `ss2_game::level_progress` (`docs/LEVEL_PROGRESS.md`): kill counters with per-player credit and a registered total, secrets found once, a tick clock, in-order checkpoints with an opaque size-limited versioned payload, restore semantics (counters revert to the checkpoint, the clock keeps running), the `LevelSummary` (kills, secrets, time, par, score) and a checksummed byte format; **it is a model only: no world snapshot is taken or restored, nothing in the game calls it, no save file, no UI, and the retry rules are my choices because the data does not state them** (T09-023b) · `M` · `P1`
- [ ] **T09-023b** · Use the level progress model: take and restore a world snapshot as the checkpoint payload (T08), register enemies from spawners (T09-018), record kills and secrets from the entities (`CSecretEntity` `MarkFulfilled`, T09-021), reach checkpoints from chapter entities, advance the clock from the sim loop, write the summary into the UI's `LevelStats` (T10-020), embed it in saves (T08-022b) and check the retry rules against the original · `M` · `P1` · Deps: T09-023, T09-021, T09-018
- [ ] **T09-024** · Scripted sequences & in-engine cutscenes: camera paths, actor scripting, letterbox, skip, co-op sync · `L` · `P2` · Deps: T01-035
- [ ] **T09-025** · Level transitions & campaign progression: level list, carry-over inventory, difficulty carry, unlocks · `M` · `P1`
- [ ] **T09-026** · Environmental hazards & destructibles gameplay · `M` · `P2`
- [ ] **T09-027** · **Per-level tasks — one per campaign level** (`T09-027a…`, each `L` · `P2`): all entity classes it uses exist, scripts behave, soft-locks checked, co-op tested, parity checklist signed. Driven by T11-016's "unimplemented classes per level" report.

## Dev & test
- [x] **T09-028** · Cheats/dev commands (god, noclip, notarget, give, spawn, kill-all, level skip) via console: the commands, the `sv_cheats` gate and the request queue (`docs/CHEATS.md`); applying the requests needs the simulation (T08, T09) · `S` · `P1` · 🟢
- [x] **T09-029** · Scenario test harness: headless sim, scripted player/enemy setups, assertions (damage dealt, time-to-kill, path reached): `ss2_sim::scenario` (`docs/SCENARIOS.md`): exact tick stepping, probes with series statistics (total drop, first tick where), `run_until`, scripted actions, a lifecycle log and a determinism check; **it has only run against toy systems: no player, weapon or enemy exists yet to build real scenarios from** (T09-029b) · `M` · `P0`
- [ ] **T09-029b** · The first real scenarios on the harness once the gameplay exists: a weapon's time-to-kill table, an enemy reaching a point, damage dealt per weapon (T09-005, T09-019); recorded-input playback (T07-019), a failure report that prints the series, and multi-client scenarios over the simulated network (T08-029) · `M` · `P0` · Deps: T09-029, T09-005
- [ ] **T09-030** · Balance/parity validation vs oracle numbers (T01-038): time-to-kill tables, rate-of-fire, projectile speeds, enemy speeds · `L` · `P1`
- [x] **T09-031** · Difficulty system tables & modifiers: `ss2_game::difficulty` (`docs/DIFFICULTY.md`): the five levels, the PC and Xbox tables with the seven members the data has per level (enemy health, speed, think frequency, player damage taken, health recharging, initial delay, auto-aim; checked equal to the real `GlobalGameParameters.rsc`), integer/saturating apply helpers with a named rounding, validation with typed errors, our own co-op rule (off by default), and difficulty carry-over/locking with a save note; **nothing calls it yet, it does not read the table from the file itself, and the data has no per-level pickup, score or enemy-count modifiers (not invented)** (T09-031b) · `M` · `P1`
- [ ] **T09-031b** · Use the difficulty model: read the table from `GlobalGameParameters.rsc` (and a mod override), apply the modifiers in enemy health, speed, think rate, initial delay, player damage and auto-aim (T09-019, T09-005), health recharging for Tourist (rate not in the data: measure, T01-038), the difficulty chooser and the single-player settings (T10), who may change it in co-op, the spawn-count/enemy-multiplier settings (T09-018), and how the multipliers combine with damage types and power-ups (T01-076, T01-038) · `M` · `P1` · Deps: T09-031
- [ ] **T09-032** · Gameplay documentation: how to add an entity class / weapon / enemy (template + checklist) · `M` · `P0`
