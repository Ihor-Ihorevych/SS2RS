# Difficulty: the five levels, the modifier table, co-op and what a campaign remembers

`ss2_game::difficulty` (T09-031) holds the **numbers of the five difficulty levels** and the **integer arithmetic** that applies them. It is pure logic: a multiplier is a count of thousandths (`Milli`), every calculation is integer and exact, results saturate, and the rounding is named at each call. There is no floating point in any calculation (a float is only used to *read* a number from data, `Milli::from_f64`), no `HashMap`, no clock and no randomness, so the same inputs give the same numbers on every machine. **Nothing in the game calls it yet** (see "Not covered").

Every value and rule is marked **Spec** (written down in `docs/re/`, read from the original's data) or **Chosen** (not in the data, so ours). The code marks the same thing: `Provenance`, `Field::provenance`, and `**Spec**` / `**Chosen**` in the doc comments.

## The levels (Spec)

Five, in this order: **Tourist, Easy, Normal, Hard, Serious** (`player-rules.md`; `console-config.md`: the single-player settings screen). `Difficulty` derives `Ord` in that order. The default is **Normal** (Chosen: the menu's initial choice is not in the data). The names `tourist` ... `serious` are the stable keys used in a save's `difficulty` note and a mod's table (Chosen; the key matches the one the save module's own test already uses).

## The table: what scales (Spec)

`GlobalGameParameters.rsc` holds two `CGameDifficulties` objects, `ggp_gdGameDifficulties` (PC) and `ggp_gdGameDifficultiesXBox`, each with one `CDifficultyParams` per level with **seven members**. That is all the data scales per level: **enemy health, enemy speed, think frequency, player damage, health recharging, initial delay and auto-aim**. The data has **no** per-level member for pickups, ammunition, score, enemy count or spawn selection, so none is invented here (see "Not covered").

PC table (`DifficultyTable::PC`), as written in the file:

| Level | Enemy health | Enemy speed | Think frequency | Player damage (taken) | Health recharges | Initial delay | Auto-aim | Provenance |
|---|---|---|---|---|---|---|---|---|
| Tourist | 0.5 | 0.75 | 0.75 | 0.2 | **yes** | 1.5 | 1.25 | Spec |
| Easy | 1 | 0.75 | 0.75 | 0.5 | no | 1.25 | 1.15 | Spec |
| Normal | 1 | 1 | 1 | 1 | no | 1 | 1 | Spec |
| Hard | 1 | 1.125 | 1.125 | 1.5 | no | 0.5 | 0.9 | Spec |
| Serious | 1 | 1.25 | 1.75 | 2 | no | 0.25 | 0.8 | Spec |

The Xbox table (`DifficultyTable::XBOX`, **reference only**: the PC game uses the PC table) is also Spec and is checked the same way:

| Level | Enemy health | Enemy speed | Think frequency | Player damage | Recharges | Initial delay | Auto-aim |
|---|---|---|---|---|---|---|---|
| Tourist | 0.5 | 0.75 | 0.75 | 0.2 | yes | 1.5 | 1.2 |
| Easy | 0.5 | 0.75 | 0.75 | 0.5 | no | 1.5 | 1.1 |
| Normal | 0.75 | 0.9 | 0.75 | 0.75 | no | 1.25 | 1 |
| Hard | 1 | 1 | 1 | 1.5 | no | 1 | 0.9 |
| Serious | 1 | 1.125 | 1.125 | 2 | no | 0.5 | 0.8 |

**Every value is a whole number of thousandths**, so nothing is rounded to build the constants. The file stores `f32`; for example 0.9 is 0.89999998... in the file and reads as 900 thousandths (`Milli::from_f64` rounds to the nearest thousandth, half away from zero).

**What a value means.** The names and values are Spec; the *roles* of four of them are the spec's reading, not something the data states (`Field::role_is_inferred`): `dif_fPlayerDamageMultiplier` is read as the damage the **player takes** (0.2 on Tourist, 2 on Serious), and what the AI does with the think frequency, the initial delay and the auto-aim factor is not in the data (`player-rules.md` open question 3 also says how the multipliers combine with the damage-type multipliers and power-ups is unknown: this module does not combine them). "Health recharges" has no rate or delay in the data and none is here.

## The arithmetic (Chosen)

| Helper | Rule |
|---|---|
| `Milli::apply_u32 / apply_u64 / apply_i64` | `value * thousandths / 1000` in 128-bit integers, then the `Rounding` you pass, then **saturate** at the type's limits (`u32::MAX`, `u64::MAX`, `i64::MIN`/`MAX`). |
| `Rounding` | `Down` (toward zero), `Nearest` (**half away from zero**), `Up` (away from zero). All act on the size of the result, so a negative number rounds like its positive twin. |
| `Milli::divide_u32` | `value * 1000 / thousandths`: turns a rate into a period. Division by a zero multiplier gives `u32::MAX` (0 for a value of 0). |
| `Milli::times` | two multipliers multiplied, nearest, saturating. |
| `Modifiers::scale_enemy_health` | nearest; **never below 1** for a positive base (a living enemy does not start dead). |
| `Modifiers::scale_damage_to_player` | nearest, **no floor**: on Tourist (x0.2) a hit of 2 or less does nothing. |
| `Modifiers::scale_enemy_speed`, `scale_auto_aim` | on a raw fixed-point `i64` (the point does not move), nearest, saturating. |
| `Modifiers::scale_think_period` | `base / think_frequency`, nearest, **never below 1 tick** for a positive base: x1.75 on Serious turns 20 ticks into 11. |
| `Modifiers::scale_initial_delay` | `base * initial_delay`, nearest; may reach 0 (Serious x0.25 of 1 tick). |

## Validation (Chosen limits)

`Modifiers::new` and `Modifiers::from_raw` (and `DifficultyTable::from_raw`, which names the level) refuse a table entry that has a **NaN** (`NotANumber`), an **infinite** (`Infinite`), a **negative** or an **absurd** value (above x100: `TooLarge`; the original's largest is x2), a **zero** health, speed or think frequency (`Zero`: a table that makes enemies unkillable-from-zero, immobile or brain-dead is broken; zero damage, delay and auto-aim are allowed), and a recharging flag that is not 0 or 1. The error is typed (`DifficultyError::BadValue { level, field, problem }`) and names the member of `CDifficultyParams`. A `Modifiers` can only be built through these checks (or is one of the two constant tables), so it is always valid. The table's entries are otherwise unconstrained (the Xbox table is not monotone in every column either).

## Co-op (Chosen)

The data has **no co-op scaling** and `game-modes.md` says the difficulty tables apply to single player and co-op unchanged. So the default, `CoopScaling::NONE`, changes nothing for any number of players. The rule, for a mod or a later decision, is a pure function: with `n` players (1 to 64) the factor is `1 + step * (n - 1)`, exactly 1 for one player, with **two independent steps**: one for each enemy's **health** and one for the **number of enemies** a spawner makes (`Effective::spawn_count`). The level's modifiers (damage taken, speed, think rate, delay, auto-aim) are **not** changed by the player count. `Effective::enemy_health` combines the level's multiplier and the co-op factor with **one** rounding at the end. Zero players and more than 64 are typed errors; a step above x100 is refused.

## Carry-over and locking (Chosen)

The original lets the player pick the level when starting a campaign (`SMSSelectDifficulty`) and says nothing about changing it later. `CampaignDifficulty` holds the level, a `ChangePolicy` and whether a level is being played:

- the difficulty **carries over** from level to level unchanged;
- `ChangePolicy::Locked` never changes (from construction), `BetweenLevels` (the **default**) only while no level is being played, up or down, `Anytime` always. Asking for the level already in force always succeeds with `Ok(false)`;
- a refused change changes nothing and says why (`ChangeRefusal`);
- a **save stores** the level as the note `difficulty` = the lower-case name (`CampaignDifficulty::note`, `from_note`; `docs/SAVES.md` notes are text), or four bytes (`to_bytes`: version, level, policy, in-level flag) that `from_bytes` refuses when the length, version, level, policy or flag is wrong. A loaded campaign is between levels.

Who may change it in a co-op game (the host only?) is a decision for the lobby, not made here.

## How it was tested

- **45 unit tests** in `crates/ss2-game/src/difficulty_tests.rs`: each level and the order; names (case, no trimming); the rounding boundaries (below, at and above the half, for 0.5 and 0.2) in all three modes; `u32`, `u64` and `i64` saturation at both limits; negative numbers; `divide` and zero; validation of every member for NaN, infinities, negative, over the limit, zero and a bad flag; the two tables value by value; the properties the spec states (recharging on Tourist only, Normal neutral, harder is not easier); each helper at each level with exact numbers (e.g. Tourist health 100 -> 50, Easy damage 15 -> 8, Serious think period 20 -> 11, Serious delay 1 -> 0); co-op with zero, one, 2, 4 and 64 players, the single-rounding check, determinism; the policies, refusals and the byte format with damaged input.
- **A data-gated test** (`the_constants_match_the_difficulty_tables_of_the_real_install`, runs with `SS2_DATA_DIR`, skips without) reads `GlobalGameParameters.rsc` from the player's install with `ss2-format-rsc`, builds a table from the raw floats with `DifficultyTable::from_raw` and asserts that both tables are **equal to the constants**: `ggp_gdGameDifficulties` == `PC` and `ggp_gdGameDifficultiesXBox` == `XBOX` (run on the real install, passes). Running it with `-- --nocapture` prints the PC table as read.
- **Mutation check** of `difficulty.rs`: 104 one-line breakages (a boundary `>=` for `>`, a swapped rounding mode, a wrong table digit, a dropped floor of 1, a swapped co-op step, a changed policy arm, a changed byte layout and so on), each applied alone to the source, then the 45 lib tests run. **102 were killed by a test, 2 do not compile** (removing `#[default]` from `Difficulty`, and changing the array length `CAMPAIGN_BYTES`: the compiler rejects them), **0 survived**. The first pass had 3 survivors: two were missing test cases (`Rounding::Up` with a remainder of one thousandth; the auto-aim helper's rounding), now tested, and one was an unreachable `other => other` arm in an error mapping, which was removed by restructuring (the private parsers now return the member and the problem). The 104-mutant pass above is the second one, run on the final source. Display texts, the byte layout and each policy arm are pinned by exact-value tests. One caveat: the tests were run **without** `SS2_DATA_DIR` during mutation (the data test skips), so the table constants were killed by the value-by-value unit tests, not by the data test.

## Not covered

- **Nothing calls this module.** No enemy, spawner, player, damage routine or menu uses it; it has only been exercised by its own tests. (T09-031b)
- **No reader.** The module has a plain constructor from raw numbers (`DifficultyTable::from_raw`) but does not read `GlobalGameParameters.rsc` itself: `ss2-game` has no run-time dependency on `ss2-format-rsc`. The data-gated test shows the 20 lines it takes. A loader (and a mod override of the table) is T09-031b.
- **No per-level pickups, ammo, score or enemy-count modifiers**: the data has none (`LEVEL_PROGRESS.md`: per-difficulty score is not in the data either). The single-player settings screen's **enemy multiplier** (none, 2x to 11x) and **enemy max multiplier** (50% to 1000%) and the spawning graphs of `SpawningParams.rsc` (`gsp_ghEnemyMultiplierOverEnemyHealth` and others) exist in the data but their meaning is not specified, so they are not modelled (T09-018).
- **How the multipliers combine** with the damage-type multipliers, armor and power-ups (open in `player-rules.md`), the health-recharge rate and delay, what the think frequency and the initial delay do in a behavior file, and how the auto-aim factor enters the auto-aim cone graph: unknown, so only the multipliers and the scaling helpers are here.
- **The co-op rule is ours** and off by default; no number for it is claimed to match the original. Nothing decides who may change the level in a co-op game, and the menus (`SMSSelectDifficulty`, T10) and the single-player settings screen are not connected.
- **Linux/WSL runs**: see the pull request.
