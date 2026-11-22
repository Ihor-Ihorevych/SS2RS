# Parity with the original game: checklists and sign-off

This page says how SS2RS is checked against the original Serious Sam 2, what is compared, and who
signs off. It covers gameplay numbers and behaviour. Rendering differences use the visual parity
template and [bug triage](TRIAGE.md). Automated tests are described in [testing strategy](TESTING.md).

Status: **process defined (T12-003)**. No level, weapon or enemy checklist has been completed yet.

## 1. What is compared

Each gameplay task that makes a level, a weapon or an enemy work in SS2RS (for example M5 "2+ weapons
and 2+ enemy types", M6 "first level playable") has one checklist. The checklist lists the items to
compare. Each item has:

- a **source of truth**: the `docs/re/` spec that defines the value or rule, or
  `measured on the original`, with the spec that says where the value is looked at;
- a **result**: `match`, `differs` or `not checked`;
- **evidence**: the measurements, as rows in a CSV file or as values pasted in the PR;
- **notes**: the parity issue for a difference, or the accepted-difference reason (section 4).

Numbers printed in the 2006 editor help are not evidence. `docs/re/player-rules.md` records that the
help disagrees with the shipped files in several places. Measure the value on the original.

Templates (one copy per level, weapon or enemy; the same text is in `docs/parity/templates/`):

### 1.1 Level template

```markdown
# Parity checklist: LEVEL <level id>

- Level file (file name only): `<name>.wld`
- Install (store, version, `Patch_*` file names): <...>
- SS2RS commit: `<sha>`
- Task completed by this PR: T<nn-nnn>, PR #<n>; priority P<0-3>
- Evidence file: `docs/parity/level/<level id>.csv`

| ID | Item | What to compare | Source of truth | Result | Evidence rows | Notes |
|---|---|---|---|---|---|---|
| L1 | Geometry and collision | Walk the route: walls, ledges, stairs, falls; where the player stops | `docs/re/wld-geometry.md`, `docs/re/wld-physics.md` | not checked | | |
| L2 | Lighting | Light positions, colours and falloff at fixed camera spots | `docs/re/wld-lighting.md` | not checked | | |
| L3 | Triggers and scripted events | Each trigger fires once, at the same place, with the same result | `docs/re/wld-macro.md`, `docs/re/wld-entities.md` | not checked | | |
| L4 | Spawns and counts | Enemies and items created per spawner and per trigger | `docs/re/wld-entities.md` (Spawners), `docs/re/ai.md` | not checked | | |
| L5 | Secrets | Number and location of secrets; how each is found | `docs/re/wld-entities.md` (`CSecretEntity`) | not checked | | |
| L6 | Pickups | Pickup kinds and placement; amount given (measured) | `docs/re/player-rules.md` (Pickups) | not checked | | |
| L7 | Music and sound | Track changes and event sounds at the same triggers | `docs/re/audio.md` | not checked | | |
| L8 | Performance | Frame time and load time on the reference PC for this level | `docs/PERF_BUDGETS.md` | not checked | | |

Sign-off
- Author: @<handle>, <YYYY-MM-DD>
- Second person (required when any item is P0): @<handle>, <YYYY-MM-DD>
```

### 1.2 Weapon template

```markdown
# Parity checklist: WEAPON <weapon id>

- Weapon parameter file (file name only): `<name>.ep`
- Install (store, version, `Patch_*` file names): <...>
- SS2RS commit: `<sha>`
- Task completed by this PR: T<nn-nnn>, PR #<n>; priority P<0-3>
- Evidence file: `docs/parity/weapon/<weapon id>.csv`
- Test setup: difficulty <...>, single player or co-op, number of repeats <n> (at least 3)

| ID | Item | What to compare | Source of truth | Result | Evidence rows | Notes |
|---|---|---|---|---|---|---|
| W1 | Damage per hit | Health removed from a target at a known distance | `docs/re/ep-schema.md` (`SBaseWeaponParams`, projectile files); value measured | not checked | | |
| W2 | Fire rate | Seconds between shots, held trigger | measured on the original; members in `docs/re/ep.md` | not checked | | |
| W3 | Ammo use | Ammunition removed per shot and per burst; cap | measured on the original; caps are not in the data, see `docs/re/player-rules.md` | not checked | | |
| W4 | Spread and accuracy | Hit pattern on a wall at a fixed distance (width, shots) | measured on the original | not checked | | |
| W5 | Projectile speed and range | Speed and distance before the projectile stops | `docs/re/ep.md` (projectile files); measured | not checked | | |
| W6 | Effects and sounds | Muzzle, impact and trail effects; fire and impact sounds | `docs/re/pfx.md`, `docs/re/fx.md`, `docs/re/audio.md` | not checked | | |
| W7 | Switching times | Seconds to draw, holster and reload | measured on the original | not checked | | |

Sign-off
- Author: @<handle>, <YYYY-MM-DD>
- Second person (required when any item is P0): @<handle>, <YYYY-MM-DD>
```

### 1.3 Enemy template

```markdown
# Parity checklist: ENEMY <character id>

- Puppet parameter file (file name only): `<name>.ep`
- Install (store, version, `Patch_*` file names): <...>
- SS2RS commit: `<sha>`
- Task completed by this PR: T<nn-nnn>, PR #<n>; priority P<0-3>
- Evidence file: `docs/parity/enemy/<character id>.csv`
- Test setup: difficulty <...>, number of repeats <n> (at least 3)

| ID | Item | What to compare | Source of truth | Result | Evidence rows | Notes |
|---|---|---|---|---|---|---|
| E1 | Health | Shots or damage to death, per difficulty | `docs/re/puppet-stats.md`; difficulty factor in `docs/re/player-rules.md` | not checked | | |
| E2 | Speed | Walk and run speed over a measured distance | `docs/re/ep.md` (puppet); measured | not checked | | |
| E3 | Damage dealt | Health removed from the player per attack, per difficulty | `docs/re/ep.md` (attacks); measured | not checked | | |
| E4 | Perception and aggro | Distance and angle at which it notices the player; time to react | `docs/re/ai.md`; the evaluation rules are measured | not checked | | |
| E5 | Attack timing | Wind-up, attack and recovery times | `docs/re/ai.md`, `docs/re/ep.md`; measured | not checked | | |
| E6 | Animations | Clip used for each state, and that it plays to the end | `docs/re/skl.md`, `docs/re/ans.md` | not checked | | |
| E7 | Drops | What it drops, and how often over N deaths | measured on the original; open in `docs/re/player-rules.md` | not checked | | |
| E8 | Death effects | Death animation length, effects and sounds | `docs/re/pfx.md`, `docs/re/audio.md` | not checked | | |

Sign-off
- Author: @<handle>, <YYYY-MM-DD>
- Second person (required when any item is P0): @<handle>, <YYYY-MM-DD>
```

## 2. Evidence

- **Format:** plain text or CSV, UTF-8, LF line endings. One CSV per checklist, at
  `docs/parity/<kind>/<name>.csv`, where `<kind>` is `level`, `weapon` or `enemy` and `<name>` is the
  file name stem used in the checklist.
- **Header:** `row,measure,original,ss2rs,unit,repeats,method,tolerance`. `row` is the checklist ID
  (for example `W2`) with a suffix when one item needs several rows (`W2a`).
- **Content:** only numbers and short labels that you measured yourself on the original and on SS2RS,
  and the method. Labels must not reproduce game text (names of items or dialogue lines).
- **Tolerance:** write it in the row before measuring, and do not change it afterwards. Use `0` for
  counts.
- **Or:** values pasted into the PR description, one line per row.
- **Never committed:** game assets, game text, `*.gro`, `*.wld`, `*.ep` or other game files;
  screenshots or video of game content; decompiler output (see `CLEANROOM.md` and `AGENTS.md`
  section 2). Parameter file names are allowed.

Automated checks (sim-scenario, golden-image and fixture tests, see `docs/TESTING.md` section 2)
can be cited as evidence instead of a CSV row, when the row names the test and the CI run that
passed. Until those layers exist, the CSV is the evidence.

## 3. When the checklist is filled in and signed off

- The checklist is **filled in and signed off in the same PR** that completes the gameplay task (the
  task line is changed to `[x]` in that PR, see `docs/roadmap/README.md` section 4). A later PR does
  not complete it.
- **Every item has a result.** `not checked` is allowed for an item the task does not touch. The PR
  says why.
- **Sign-off.** The author signs. For a task with any P0 item, a second person who did not write the
  evidence signs as well. The second person re-checks at least one `match` row by measuring it
  again.

## 4. Differences

When a row says `differs`:

1. Open a parity issue with the template that matches the difference: `.github/ISSUE_TEMPLATE/parity.md`
   for numbers, timing and behaviour, or `.github/ISSUE_TEMPLATE/visual-parity.md` for rendering. Put
   the measured values in it, as text or CSV.
2. Put the issue number in the row's Notes column, for example `#<n>`.
3. Decide the row's status:
   - **Fix it.** Keep the row `differs` until a PR that changes the value passes the re-measurement.
     That PR updates the row to `match`.
   - **Accepted difference.** Write `accepted difference: <reason>` in the Notes column, with the
     owners who accepted it. Use this only when the reason is one of: the original behaviour cannot
     be reproduced on purpose (for example a random seed the data does not provide); the difference
     is needed for a documented change of the engine (an RFC or spec says so); or the difference is
     within the tolerance written in the row. A difference the owners do not accept stays `differs`.
4. Severity and priority follow [bug triage](TRIAGE.md) section 3. A parity break that blocks a
   level, weapon or enemy from working as in the original is S2 and P1. If the original behaves the
   same way in the same scenario, the difference is not a parity bug (TRIAGE section 3); close the
   issue as `wont-fix` with a note.

## 5. Relation to other documents

- [Bug triage](TRIAGE.md): intake templates, labels (`parity`, `gameplay`, `visual`), severity and
  priority.
- [Testing strategy](TESTING.md): which automated layers exist. Parity checklists are manual records
  until a layer covers the item.
- [Performance budgets](PERF_BUDGETS.md): the budgets for item L8.
- `docs/re/` specs: the source of truth column. A spec that is wrong or missing is an issue with
  label `spec`, not a reason to read the original binary (`docs/re/README.md`).

## 6. Worked examples

All values in these examples are **made up**. They are marked EXAMPLE. They are not measurements, and
no file with these names exists in the repository.

### 6.1 Level (example)

Checklist `EXAMPLE-LEVEL`, row L4 only:

| ID | Item | What to compare | Source of truth | Result | Evidence rows | Notes |
|---|---|---|---|---|---|---|
| L4 | Spawns and counts | EXAMPLE: enemies created after the first trigger | `docs/re/wld-entities.md` (Spawners) | differs | EX4 | EXAMPLE: #101 (parity). Fix it. |

`docs/parity/level/EXAMPLE-LEVEL.csv`:

```csv
row,measure,original,ss2rs,unit,repeats,method,tolerance
EX4,EXAMPLE enemies after first trigger,12,11,count,3,counted by hand after the trigger,0
```

### 6.2 Weapon (example)

Checklist `EXAMPLE-WEAPON`, rows W2 and W4:

| ID | Item | What to compare | Source of truth | Result | Evidence rows | Notes |
|---|---|---|---|---|---|---|
| W2 | Fire rate | EXAMPLE: seconds between shots | measured on the original | match | EXW2 | |
| W4 | Spread and accuracy | EXAMPLE: hit width at 10 m | measured on the original | differs | EXW4 | EXAMPLE: #102 (parity). accepted difference: EXAMPLE, the original's spread uses a seed the data does not provide, so only the average width is compared, and it is within 5 % of the original. |

`docs/parity/weapon/EXAMPLE-WEAPON.csv`:

```csv
row,measure,original,ss2rs,unit,repeats,method,tolerance
EXW2,EXAMPLE seconds between shots,0.40,0.40,s,3,stopwatch over 10 shots,0.02
EXW4,EXAMPLE mean hit width at 10 m,1.80,1.62,m,5,tape on a wall,0.09
```

### 6.3 Enemy (example)

Checklist `EXAMPLE-ENEMY`, rows E1 and E4:

| ID | Item | What to compare | Source of truth | Result | Evidence rows | Notes |
|---|---|---|---|---|---|---|
| E1 | Health | EXAMPLE: shots to death on normal | `docs/re/puppet-stats.md` | match | EXE1 | |
| E4 | Perception and aggro | EXAMPLE: distance at which it turns to the player | `docs/re/ai.md` | differs | EXE4 | EXAMPLE: #103 (parity). Fix it. Re-measure after the perception change. |

`docs/parity/enemy/EXAMPLE-ENEMY.csv`:

```csv
row,measure,original,ss2rs,unit,repeats,method,tolerance
EXE1,EXAMPLE shots to death (normal),8,8,count,3,counted by hand,0
EXE4,EXAMPLE distance at which it turns,14.0,11.5,m,3,walked toward it from 20 m,0.5
```
