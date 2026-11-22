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
