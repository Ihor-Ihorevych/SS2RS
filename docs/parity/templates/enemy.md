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
