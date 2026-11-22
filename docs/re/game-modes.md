# Game modes, scoring, multiplayer differences (T01-037)

- **Roadmap task:** T01-037
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the mode list, the maps per mode, the single-player versus multiplayer parameter differences, the score and health tables and the network-smoothing constants (read from the data); **medium** for what the modes do at run time; **open** for the rules (win conditions, frag limits, respawn timing, CTF flag rules, co-op scaling): they are game logic.

Prerequisites: `player-rules.md` (difficulty tables, power-ups, global parameters), `misc-types.md` (`.nfo` keys), `ai.md`, `puppet-stats.md` (generated per-character table).

## Summary

Levels declare the game types they support in their `.nfo` (`MAP_TYPE`): **`SINGLE; COOP;`** for the 42 campaign levels (the same level runs single player and co-op), **`DM;`** for 15 deathmatch arenas and **`DM; CTF;`** for one arena (`dm_littletrouble`), so the shipped modes are **single player, cooperative, deathmatch and capture-the-flag**. Multiplayer uses its own set of parameter files (`*_dm.ep`) for the player and for most weapons, a shorter set of power-up durations, and a network prediction/smoothing parameter block. Difficulty applies to the campaign (five levels, `player-rules.md`).

## Evidence

`.nfo` files (79) decoded for `MAP_TYPE` and `PLAYERS`; the `_dm` parameter files compared member by member with their single-player counterparts; `GlobalGameParameters.rsc` read for the network block and the power-up durations; `tools/re/puppet_stats.py` for the score and health tables. Nothing from binaries.

## Modes and maps

| `MAP_TYPE` | Levels | `PLAYERS` |
|---|---|---|
| `SINGLE; COOP;` | 42 (chapter levels such as `1_1_jungle` … `7_7_…`, in `Content/SeriousSam2/Levels/<episode>/`) | not given |
| `DM;` | 15 (`dm_brkeenchevap`, `dm_chanolin`, `dm_metropolis`, `dm_branchester`, `dm_catacombs`, `dm_cliff`, …) | the recommended player count: `4` (6 maps), `6` (5), `2-4`, `2-6`, `5-10`, `6-12`, `10` |
| `DM; CTF;` | 1 (`dm_littletrouble`) | one of the values above |

A level `.nfo` also names its loading screen, loading model, ETA and movies (`misc-types.md`, `cutscenes.md`). The **co-op** mode reuses the campaign levels: a chapter info has a "cooperative portal marker" and a limit of spawnees per alignment (`help-digest.md`), so enemy counts are adjusted by the number of players, but the adjustment rule is not in the data.

## Multiplayer parameter set

| Item | Single player | Multiplayer |
|---|---|---|
| Player puppet file | `PlayerPuppet.ep` | `PlayerPuppet_DM.ep`: the first envelope speeds **75 → 110**, **45 → 80**, **7.5 → 12**; jump-up velocity **10 → 11**; maximum jump slope **60° → 70°**; an "exploding damage" threshold **0 → 90** (corpses gib from 90 damage); one extra flag set |
| Weapon files | `*Weapon.ep` | `*Weapon_DM.ep` for every weapon (17 files); 6 differ in no member (the two circular saw files, grenade launcher, klodovik gun, serious bomb, zap gun) and cannon and flamer in one |
| Power-up durations | 60 s each | strength 40, speed 20, jump 40, damage 30, score 40, invulnerability 30, invisibility 30 |
| Auto-jump | on | off |

### Weapon parameters that differ (single player → multiplayer)

| Weapon | Changes |
|---|---|
| Auto shotgun | bullet damage 12 → 10; unnamed members `.1`/`.4` 100/200 → 500/500, `.6` 0.5 → 3, `.30` 1 → 0.819 |
| Double shotgun | bullet damage 12 → 7 (14 pellets); the same `.1`/`.4` change; `.6` 1 → 4.5, `.30` 1.3 → 1.078 |
| Minigun | bullet damage 10 → 12; unnamed `.1`/`.4` 200/300 → 750/750, `.6` 1 → 22.5, `.22` 550 → 500, `.30` 0.035 → 0.1 |
| Uzi | bullet damage 10 → 13; unnamed `.1`/`.4` 100/300 → 500/500, `.6` 2 → 15, `.2` 0.1 → 0, `.30` 0.12 → 0.2, `.34` 0.08 → 0.1 |
| Colt | bullet damage 10 → 15 |
| Sniper | bullet damage 75 → 25; **zoom** damage 300 → 150; maximum zoom 4× → 6×; full zoom time 0.8 → 1.0 s; unnamed `.4` 400 → 1,500 |
| Beam gun | unnamed `.20`/`.21` 0.25/0.75 → 0.5/3, `.1` 14 → 7, `.4` 100 → 50 |
| Plasma rifle, rocket launcher | one or two timing members change slightly (0.15 → 0.1; 0.6 → 0.665) |
| Cannon | one member |

The named members (`*_slBulletDamage`) are the base single-player damage values: **auto shotgun 12 per pellet × 7 pellets, double shotgun 12 × 14, colt 10, minigun 10, uzi 10, sniper 75 (300 zoomed), circular saw cutting range 3 / width 2**; the other weapons use projectiles and unnamed members (`T01-028`).

## Scoring

- **Extra life every 10,000 points**; **serious score** multiplies by 5 (`player-rules.md`).
- **Score per kill** is `CMiscPupeptParams.mpp_slScore` of the victim: over the 126 character files **41 give 0** (civilians, objects, many NPCs and vehicles), the rest give (score: number of files) 10: 2, 20: 2, 40: 1, 50: 11, 100: 34, 150: 7, 200: 7, 300: 1, 400: 1, 500: 8, 750: 1, 1,000: 1, 5,000: 9: every boss gives **5,000** (dragon, kleerofski, kwongo, princchan, pyramid, transformer, zumzum) together with two big tanks; ordinary soldiers 100 to 200; large monsters 500. The full table with health is **`puppet-stats.md`**.
- Treasure items (S/M/L) are the score pickups; their values are not in the data (`player-rules.md`).
- Multiplayer scoring: the HUD has a **frags counter** (`CFragsCounterHudElementParams`, with a `.tfx` text effect: `cutscenes.md`/`fx.md`); frag limits and the CTF scoring are not in the data.

## Network parameter block (`ggp_snpNetworkParams`)

The client-side smoothing of replicated state uses these constants (the units are not stated): **ignored position error 0.05**, **teleported position error 5** (a larger error teleports instead of blending), **position-error melting factor 5**, **prediction tolerance for teleporting 0.2**, **Y tolerance multiplier 3**, **ignored animation-offset error 0.05**, **teleported animation-offset error 0.5**, **animation-offset melting factor 5**. They define how a client corrects the difference between its prediction and the server's state (the "error melting"): below the ignore threshold nothing is done, between the thresholds the error is reduced at a rate given by the melting factor, above the teleport threshold the object jumps. The exact formula is not in the data. This is useful vocabulary for the networking design (T08); the original replication protocol itself is out of scope (`CLEANROOM.md`: no protocol parity).

## What the data does not give

Win conditions and limits (frag, time, capture), respawn delays and spawn-point selection in deathmatch, the CTF flag rules, the co-op scaling, handicap and team rules, score sharing in co-op, and the lobby and match-making (the online classes `CMGOnline…` exist in the menu layouts; no run-time rule is in the data).

## Reader invariants

1. `MAP_TYPE` is a list separated by `;` of the tokens `SINGLE`, `COOP`, `DM`, `CTF`; ignore unknown tokens.
2. Pick the `_dm` parameter file for multiplayer, the plain file otherwise; a missing `_dm` file means "same as single player".
3. The difficulty tables apply to single player and co-op.

## Open questions

The "What the data does not give" list; how the `PLAYERS` range is used (a menu hint or a hard limit); the Xbox difficulty table is presumably not used on PC (the PC table is `ggp_gdGameDifficulties` by name).

## Test vectors

`MAP_TYPE` values over the 79 `.nfo` files: `SINGLE; COOP;` ×42, `DM;` ×15, `DM; CTF;` ×1. Power-up durations multiplayer 40, 20, 40, 30, 40, 30, 30. Network block: 0.05, 5, 5, 0.2, 3, 0.05, 0.5, 5. Boss score 5,000 in all seven boss files; the player puppet health is 100.
