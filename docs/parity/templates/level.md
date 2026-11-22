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
