# The player's stored movement constants (T01-410)

- **Roadmap task:** T01-410 (feeds T07-037, the retuning of the provisional walker of T07-030; the
  measured version stays T01-024 — T01-038 (2026-11-28) got the oracle harness attached to the
  running original for the first time, but could not yet read the player's position or velocity; see
  "Live-check attempt, 2026-11-28" below)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for every number read directly from `EntityParams/PlayerPuppet.ep` and
  `PlayerPuppet_DM.ep` and reproduced by `tools/re/player_movement_verify.py`; **medium** for the
  roles that rest on one reading of a small function (air control, the jump-slope test, the
  pose-to-environment choice — all already read for `puppet-legged-classes.md` and
  `puppet-record-members.md`, plus two getters and one reset site read in full for this page); **low
  or open** where stated, in particular the player's collision shape and size, crouch movement
  speed, and whether the player's gravity is scaled. Nothing was run on the original game.

Prerequisites: `player-rules.md` (the global and puppet parameters),
[`puppet-legged-classes.md`](puppet-legged-classes.md) (the three environment records, jump and
air-control members, pose rules), [`puppet-record-members.md`](puppet-record-members.md) section 8
(the declared names and roles of the environment record, read in full for T01-214),
`game-modes.md` (the single-player/deathmatch parameter diff), `ep.md` /
`ep-schema.md` (the file format), `wld-root.md` (the world's gravity
vector), `misc-types.md` (`.mch` mechanism templates, still open as T01-057).

## Summary

The player's movement numbers are **data**, read straight from the two live player puppet parameter
files (`Content/SeriousSam2/Databases/EntityParams/PlayerPuppet.ep` for single player and co-op,
`PlayerPuppet_DM.ep` for multiplayer — the third file the install carries,
`Databases/Puppets/Player/Player.ep`, is a different, unused-by-this-class file and is **not** the
one `CPlayerPuppetEntity` reads: see "A third file" below). They sit in the same three records
every legged puppet has (`puppet-legged-classes.md`, `puppet-record-members.md` section 8):

- **Ground acceleration, deceleration and the nominal speed** are in the *land* environment record
  (`SLeggedPuppetParams.1`). The player's speed range is degenerate (`pep_fSpeedMin` =
  `pep_fSpeedMax`), so there is **one** nominal ground speed, not separate forward/backward/strafe
  values — the "probably forward, backward and side" guess in `enemies.md`/`player-rules.md` is
  **not what the data stores for the player** (it stores a per-instance random *range*, used so that
  otherwise-identical enemies are not all exactly as fast as each other; the player's `en_fRnd` is
  always 0, so it always gets the range's low end, which here equals the high end).
- **Step height, slope-for-jumping, fall height, jump velocity, view heights and the collision
  mechanism** are where this page had to look beyond `player-rules.md`: `SMovingAbilities`,
  `SLeggedPuppetBodyParams`, `SPuppetBodyParams`, and the physics mechanism file referenced from
  the player's model (`Player.mch`).
- **There is no separate "while airborne" acceleration record.** `GetEnvironmentParams` picks the
  *land* record for every pose except flying, diving and swimming — **including falling** — so the
  same ground accel/decel pair applies whether the player is standing on the floor or in the middle
  of a jump arc. The *second* environment record (misleadingly "air" by position) is only used in
  the `PP_FLY` pose, i.e. the **fly cheat's noclip-style flight**, not ordinary jumping; its very
  high numbers (accel/decel 1000) are "snap to the target speed almost immediately while flying
  freely", not a jump air-control constant.
- **Air control** (the brief steering a player gets right after leaving the ground) is a *different*,
  short-lived system (`en_bUsesAirControl`/`en_fAirControlFactor`/`...Period`/`...VelocityMultiplier`)
  that `WalkConfig::air_accel` has no equivalent shape for; see "Air control" below.
- **The player's collision radius and height were not found with confidence.** The physics
  mechanism file exists and decodes, but which of its seven mechanism templates is the walking
  collision (as opposed to crouch, ragdoll, ride or corpse) could not be confirmed in this task's
  time box; see "Collision shape" below.
- **The world unit is, with medium confidence, the metre** — no file states this directly, but the
  player's own numbers are only sensible together if it is (see "Units").

## Evidence

- **Data**, read-only, with `ctse_values.py` (the same CTSEMETA decoder every `tools/re/*_verify.py`
  script uses): `EntityParams/PlayerPuppet.ep`, `PlayerPuppet_DM.ep`, `Databases/Puppets/Player/Player.ep`,
  the player's model `Models/Player/SeriousSam/SeriousSam.mdl` and its externally-referenced physics
  mechanism file `Models/Player/SeriousSam/Mechanisms/Player.mch`. `tools/re/player_movement_verify.py`
  reproduces every number in the first table below from the two live `.ep` files and fails if the
  install disagrees with it.
- **Code**, `Sam2Game.dll`, read in the local decompile tree (never copied, quoted or committed;
  numbers only): `CLeggedPuppetEntity::GetAirControlFactor` and `::GetAirControlPeriod` (the same
  compiled function at the same address — two source-level one-line getters returning the same
  literal, folded together by the linker) and `::GetAirControlVelocityMultiplier` (reads a
  per-instance member), plus the four call sites that reset that member. This is in addition to the
  readings `puppet-legged-classes.md` and `puppet-record-members.md` already report (pose selection,
  the jump-slope getter, the environment record's roles).
- **Not done:** nothing was run on the original game; the player's collision mechanism's template
  names were not matched to roles (needs the `.mch` name table, T01-057, not yet decoded); the
  14-kilobyte `LerpToDesiredVelocityAndLook` (the actual per-step movement blend, including how
  accel/decel, air control and the cone limits combine) was not read here, only profiled by earlier
  pages (T01-141 is the open follow-up for that).

## A third file, not read by the live player

`puppet-legged-classes.md` already notes that the install carries **three** `SPlayerPuppetParams`
files: `EntityParams/PlayerPuppet.ep` and `PlayerPuppet_DM.ep` (used by `CPlayerPuppetEntity`,
confirmed by `game-modes.md`'s single-player/deathmatch diff, which matches this page's numbers
exactly) and `Databases/Puppets/Player/Player.ep` (which also carries a model reference,
`Models/Player/SeriousSam/SeriousSam.mdl`, that the two live files do not). Its numbers differ from
both live files (land accel/decel 200/100 vs. 75/45 and 110/80; `ma_maMovingAbilities` 0x29 vs.
0x2d for the live files) and its `ppp_fFreezeTime` is **30**, not the live files' 60. This page reads
**only** the two live files; `Databases/Puppets/Player/Player.ep` is treated as unused reference
data, not as a source of the live player's numbers (**medium**: inferred from the class inventory of
`puppet-legged-classes.md`/`players-session-classes.md`, not from a trace of which file the
constructor opens).

**Correction to `player-rules.md`:** that page's "freeze time 60 s (30 in multiplayer)" is wrong for
the live multiplayer file — `PlayerPuppet_DM.ep`'s `ppp_fFreezeTime` is **60**, the same as single
player (`tools/re/player_movement_verify.py` checks this). The "30" is `Databases/Puppets/Player/Player.ep`'s
value, not `PlayerPuppet_DM.ep`'s. Fixed in that page by this change.

## The constants

All values from `EntityParams/PlayerPuppet.ep` (single player/co-op) and `PlayerPuppet_DM.ep`
(multiplayer); unit is the game's length/time unit, believed to be the metre and the second
("Units" below).

| Constant | Single player | Multiplayer | Source | Confidence | Used |
|---|---|---|---|---|---|
| Ground acceleration (`pep_fAcceleration`, land record) | 75 | 110 | data | high | force per unit of puppet mass the ground motion joint is driven with, refreshed every step (`SetCurrentMechanism`/`GetCurrentAcceleration`, read in full for `puppet-record-members.md`); applies **on the ground and while falling** (land record covers every non-fly/dive/swim pose) |
| Ground deceleration/friction (`pep_fDecceleration`) | 45 | 80 | data | high | the joint's break force, same rule, same poses (`GetCurrentDeceleration`) |
| Nominal ground speed (`pep_fSpeedMin` = `pep_fSpeedMax`) | 7.5 | 12 | data | high | one speed for all directions (the player's `en_fRnd` is 0: `puppet-record-members.md` section 8); no separate forward/strafe/backward numbers exist in the data |
| Walk/run animation threshold (`pep_fWalkVelocity`) | 3 | 3 | data | high | **not a speed cap** — only which moving animation plays (`puppet-record-members.md`) |
| Sprint multiplier | ×1.5 | ×1.5 | code | medium | `GetSpeedMultiplier`, gated by the session option and the puppet not being in the crouch pose (already read for `players-session-classes.md` 7.2); derived sprint speed 11.25 / 18 |
| "Serious speed" power-up multiplier (`ppp_fSpeedPowerUpMultiplier`) | ×2 | ×2 | data | high | derived speed while active 15 / 24 |
| Jump-up velocity (`lpbp_fJumpUpVelocity`) | 10 | 11 | data | high | `DoJump`'s initial upward speed |
| Double jump | none found | none found | absence | medium | only `AutoJump` exists (an automatic hop over a small obstacle or gap, a different mechanic, not a second mid-air jump) |
| Gravity scale for the player | not found | not found | absence | open | no per-puppet gravity member in `SPuppetParams`/`SLeggedPuppetParams`/`SPlayerPuppetParams`; the player is presumed to use the world's own gravity unscaled (`CBody` has a gravity switch per `physics.md`, not a per-value scale, in what was read) |
| Maximum slope a **jump may start from** (`lpbp_fMaxValidSlopeForJump`) | 60° | 70° | data | high | `GetMaxJumpSlopeAngle`; **not** a general walkable-slope limit (see note below) |
| Step-up height (`ma_fStepUpHeight2`) | 2.0 | 2.0 | data | high | `SMovingAbilities`; shared with most enemies — likely an engine default, not individually tuned |
| Step-down height (`ma_fStepDownHeight`) | 8.0 | 8.0 | data | high | same caveat |
| Fall height without damage (`ma_fAllowedFallHeight`) | 2.5 | 2.5 | data | high | `CanReceiveFallingDamage` combines this with "serious jump" immunity (`players-session-classes.md` 7.2) |
| Eye height, standing (`pbp_vViewOrigin.y`) | 1.95 | 1.95 | data | high | `GetViewOrigin`/`GetRelativeViewOriginForPose` |
| Eye height, sitting/riding (`pbp_vViewOriginSitting.y`) | 1.2 | 1.2 | data | high | used while riding a seat |
| Eye height, crouching (`pbp_fViewHeightCrouch`) | 0.7 | 0.7 | data | high | `GetRelativeViewOriginForPose` |
| Eye height, swimming (`pbp_fViewHeightSwim`) | 1.5 | 1.5 | data | high | same |
| Eye height, diving (`pbp_fViewHeightDive`) | 1.1 | 1.1 | data | high | same |
| Model stretch (`pbp_fStretch`) | 0.9 | 0.9 | data | high | a scale factor on the player's model/mechanism; whether the view-origin numbers above are stored before or after it is **not determined** (medium-low); if before, the effective standing eye height would be ≈1.76 |
| Keep-breath period (`pbp_fKeepBreathPeriod`) | 60 s | 60 s | data | high | drowning starts after this long submerged |
| Crouch movement speed | not found | not found | absence | open | `GetSpeedMultiplier` (read in full elsewhere) covers the power-up, sprint and a deathmatch factor only; no crouch term was seen |
| Collision radius / height | not found with confidence | — | — | open | see "Collision shape" |

Water environment record (identical in both files; used only in the `PP_DIVE`/`PP_SWIM` poses):
acceleration 50, deceleration 10, nominal speed 6, walk/run threshold 3, moving turn rate 600–800°/s,
standing turn rate 180°/s (data, high).

"Air" environment record (identical in both files; used only in the `PP_FLY` pose, i.e. the fly
cheat): acceleration/deceleration 1000/1000 ("snap to speed"), nominal speed 7.5, standing turn rate
0 (data, high; role confirmed by `puppet-legged-classes.md`'s pose-to-record rule).

Turn rates of the land record are the engine's "no limit" sentinel (≈1×10³⁸) in both files, i.e. the
player's own heading turns as fast as the look input moves it; no turn-rate cap applies on land.

### Air control

The brief steering the player gets right after jumping is a separate system from the environment
records above, and does not fit `WalkConfig::air_accel`'s shape (a constant acceleration that
applies for as long as the player is airborne):

- A boolean (`en_bUsesAirControl`) gates whether it is active.
- Its **factor** and **period** default, in the base legged puppet class, to the **same** literal
  value (**0.5**), because the two one-line getters that return them (`GetAirControlFactor`,
  `GetAirControlPeriod`) compile to identical code and were folded into one function by the linker —
  read in full for this page (medium: a shared global float constant, confirmed by reading its raw
  value in the shipped binary; not a value `player-rules.md` or any `.ep` file states). This default
  is overridden, per `player-rules.md` 7.2, by the world info entity's own two members when they are
  not negative — a per-level, not per-player, override this page did not read.
- Its **velocity multiplier** (`en_fAirControlVelocityMultiplier`) defaults to **1.0** (read in full:
  four reset sites in `OnCreate`/`ResetDynamicMembers`-style functions all write the bit pattern of
  `1.0f`).
- While the "serious jump" power-up is active, four replicated player members
  (`en_fJumpPowerupJumpMultiplier`, `...AirControlPeriod`, `...AirControlFactor`,
  `...VelocityMultiplier`) replace all of this; their values are a power-up *effect*, not stored
  parameters (`players-session-classes.md` 7.2).
- How the factor, period and multiplier actually combine with the land acceleration inside a single
  step is in the unread `LerpToDesiredVelocityAndLook` (T01-141); this page only has the three
  default numbers above (0.5, 0.5 s, 1.0), not the formula.

**For T07-037:** the real game's air behaviour is "ground accel/decel, unchanged, the whole time
you're airborne, plus up to ~0.5 s of extra steering right after a jump" — not "ground accel/decel
while grounded, a separate `air_accel` the rest of the time" as `WalkConfig` currently models it.

### Slope: two different things

`player-rules.md` and this page both read a **slope-for-jumping** limit (`lpbp_fMaxValidSlopeForJump`,
60°/70°): it only gates whether `DoJump`/`AutoJump` may fire from the puppet's current ground slope.
No page has found a **general walkable-slope limit** (what makes a surface count as "ground" for
friction and footing at all, matching `WalkConfig::max_slope_cos`) anywhere in the player's or any
puppet's data; it may be an engine-global physics parameter (`CWorld`'s physics setup,
`wld-root.md`, not yet matched to a meaning) or simply not represented as a single constant. This
stays open for T01-024 or further code reading.

### Collision shape

Not resolved with confidence, despite finding the real source. The player's actual collision/physics
geometry is **not** embedded in `SeriousSam.mdl` (which has no mechanism-template objects at all) but
in a separately-referenced physics file, `Models/Player/SeriousSam/Mechanisms/Player.mch` — the same
`.mch` mechanism-template format `misc-types.md` already describes and leaves open as T01-057 ("`.mch`
units and the rest of the physics templates"). It decodes cleanly with the existing CTSEMETA tools:
**seven** mechanism templates, each with exactly one physics part and one primitive hull (hull types
seen: sphere and capsule). What this page could **not** establish in its time box:

1. which of the seven numbered templates (ids 1, 2, 3, 22, 23, 24, 25 in the file) is the walking
   collision, as opposed to crouch, ragdoll, ride or corpse — the template **names** (`Default`,
   `Ragdoll`, `RagdollSimple`, … per `misc-types.md`) are a separate string table this page did not
   decode;
2. the local axis convention the capsule hulls use (whether the "long axis" the primitive-hull rules
   of `wld-geometry.md` describe for world hulls applies unchanged here), without which the raw
   numbers (radius/half-length pairs of roughly 1.0/2.2, 1.0/1.4, 0.27/0, 0.43/2.2×2, 1.0/1.5 and
   1.0/3.0 across the seven templates) do not resolve to one clearly human-sized capsule.

Rather than assert an unproven radius/height, this page leaves the collision shape **open**, as a
natural continuation of T01-057 (same file format, same open status already recorded in
`misc-types.md`) rather than new work for T01-410. `WalkConfig::radius` (0.4) and `::height` (1.8)
therefore stay tuned-by-feel placeholders; nothing read here confirms or replaces them.

### Units

No page states outright that the world's length unit is the metre; `ai.md` says plainly that its own
range numbers are "probably meters; the unit is not stated". This page adds three mutually-consistent
data points rather than a proof:

1. `wld-root.md` already reads the world's gravity vector as **(0, −30, 0) in all 59 worlds**,
   itself only "almost certainly m/s²" (medium, that page's own words) — this is the same number the
   walker's `Walker::step` example and `T07-030`'s gravity already use.
2. With that gravity and this page's jump-up velocity (10 m/s, single player), the apex of an
   unmodified jump is `v² / (2g) = 100 / 60 ≈ 1.67` length units — a plausible jump height for a
   fast-moving human-scale action-game character if the unit is the metre; implausible (8 to 50×
   too high) for a smaller or larger assumed unit.
3. The player's own eye heights (1.95 standing, 0.7 crouching, 1.5 swimming) and the model stretch
   (0.9) are themselves metre-scale numbers for a tall, stylised human.

None of the three is independently conclusive, but they agree only if the unit is close to the
metre. **Confidence: medium** that the world unit is the metre; this is the same conclusion
`WalkConfig::PROVISIONAL`'s choice of gravity already assumed, now with supporting data rather than
only the walker's own feel.

## Mapping onto `WalkConfig`

| `WalkConfig` field | Provisional (T07-030) | What the data says | Fit |
|---|---|---|---|
| `radius`, `height` | 0.4, 1.8 | not found with confidence (see "Collision shape") | **no data to compare against**; stays as-is |
| `eye_height` | 1.65 | 1.95 standing (data, high) | real value is higher; T07-037 should probably raise it, modulo the stretch-factor ambiguity above |
| `step_height` | 0.5 | 2.0 up / 8.0 down (data, high), but shared with every enemy, so possibly an engine default rather than a human-scale tuning choice | real value is markedly higher; `WalkConfig` has no separate step-down number, so the real step-down (8.0) has no field to map onto |
| `max_slope_cos` | 0.707 (45°) | 60°/70° exists but is specifically the **jump-start** slope limit, not a walkable-ground limit (see "Slope") | **no matching constant found**; stays open for T01-024 |
| `speed` | 10.0 | 7.5 (SP) / 12.0 (DM) ground nominal speed (data, high); sprint ×1.5, speed power-up ×2 (code/data) on top | provisional is between the two; no sprint/power-up multiplier exists in the current model at all |
| `accel` | 80.0 | 75.0 (SP) / 110.0 (DM) ground acceleration (data, high) | provisional is close to the single-player value |
| `friction` | 60.0 | 45.0 (SP) / 80.0 (DM) ground deceleration (data, high) | provisional sits between the two |
| `air_accel` | 15.0 | **no equivalent constant**: the real game reuses the ground accel/decel while airborne and layers a short (~0.5 s) extra-steering window on top (see "Air control") | the field itself does not match the mechanism; T07-037 may need a different shape (ground accel/decel always, plus a timed post-jump nudge) rather than a single `air_accel` |
| `jump_speed` | 9.0 | 10.0 (SP) / 11.0 (DM) (data, high) | provisional is close, slightly low |

## What remains for T01-024 (measured movement)

1. The actual per-step formula of `LerpToDesiredVelocityAndLook` (how acceleration, deceleration, air
   control and the turn-rate cones combine every tick) — this page only has the declared constants,
   not the formula (T01-141).
2. A general walkable-slope limit, if one exists as a single constant at all (see "Slope").
3. The player's collision radius and height (see "Collision shape"; candidate follow-up to T01-057).
4. Crouch movement speed (no data or code reading found a term for it).
5. Whether the player's gravity is scaled from the world's own value.
6. Whether `pbp_vViewOrigin` is stored before or after the 0.9 model stretch is applied (changes the
   effective eye height by about 0.2).
7. Confirmation, from the running game, that the world unit really is the metre (this page's
   cross-check is medium confidence, not a measurement).

## Live-check attempt, 2026-11-28 (T01-038)

- **Roadmap task:** T01-038 (oracle capture, scoped to player movement this run) · **Author:**
  @Ihor-Ihorevych/agent

T01-038 set out to measure ground speed (including diagonal), acceleration/deceleration curves, jump
apex and air time, step-up height, gravity and eye height on the running original, to check against
the table above, using `tools/oracle` (T11-008; its live check, T11-008b, was still open). The
original **could** be started this run (Steam started and logged in locally, then
`Bin/Sam2.exe +sam_bStartDemoPlayback 1 +sam_strDemoFile Content/SeriousSam2/Demos/Demo_0001.dem …`,
per `docs/ORACLE_HARNESS.md`), and the harness attached and read real numbers for the first time —
but **no movement number below was obtained**, for the reasons below. Nothing was faked: where a
number could not be read, it is reported as not read, not guessed.

**What was confirmed live:**

- The simulation clock (`sim.now_raw`) advances at 4.2946×10⁹ raw counts per second of game time
  against the `docs/re/main-loop.md` §1 prediction of 2³² ≈ 4.29497×10⁹ — within 0.01%, over a
  500-sample/25-second capture. **Confidence: high.**
- The oracle target map's chain to the player puppet entity (world → a handle → world-info entity →
  a handle → game-info entity → a handle → player brain → a handle → player puppet) needed one hop's
  offset corrected; once fixed, `player.health`/`health_max`/`armour`/`alive` matched the on-screen
  HUD repeatedly across several minutes of demo playback, several level changes and a vehicle
  section (values cross-checked by eye against screenshots: 125, 135, 150, 165, 200 and a momentary
  2). **Confidence: high** (repeated, consistent, independently cross-checked against the HUD).

**What is still blocked — no movement number could be measured:**

The player's placement (position, orientation) and velocity are **not yet reachable** through the
oracle target map. The existing guess for the position/orientation chain (a handle on the puppet,
one more offset for x/y/z) resolves to a different object on almost every read — plausible-looking
position floats on one sample, obvious garbage a fraction of a second later with the player
visibly not moving — which reads as a pooled/reused temporary slot, not a stable reference to the
puppet's own placement. Without a stable position and velocity reading, **none** of this run's
target numbers could be produced:

- ground speed, steady-state or diagonal;
- acceleration/deceleration curves (time to full speed, time to stop);
- jump apex height and air time;
- step-up height;
- gravity (measured, as opposed to the data-only `wld-root.md` reading already in this page);
- eye height from the camera.

The table above (T01-410, data-only, read from the `.ep` files) is therefore **unchanged** by this
run: nothing here confirms or contradicts any of its numbers. Finding the real placement/velocity
object needs more targeted work than this run's time box — most likely reading `Sam2Game.dll`'s
placement/mechanism accessors in the local decompile tree against the now-corrected puppet chain,
rather than further blind memory scanning — and is split off as a follow-up task (see
`docs/roadmap/T01-reverse-engineering.md`, the task appended after T01-038).

## Test vectors

Reproduced by `tools/re/player_movement_verify.py` (exits 1 on a mismatch):

- Single player land record: acceleration 75, deceleration 45, nominal speed 7.5, walk/run threshold
  3; jump-up velocity 10; maximum jump slope 60°; fall height 2.5; step up 2.0, step down 8.0; eye
  height standing 1.95, sitting 1.2, crouching 0.7, swimming 1.5, diving 1.1; model stretch 0.9;
  speed power-up multiplier 2.0; freeze time 60.
- Multiplayer: the same, except land acceleration 110, deceleration 80, nominal speed 12, jump-up
  velocity 11, maximum jump slope 70° (all matching `game-modes.md`'s independently-stated diff) and
  freeze time **60**, not 30 (the correction above).
- Water record (both files): acceleration 50, deceleration 10, nominal speed 6, turn rate 600–800°/s.
- "Air" record / `PP_FLY` pose (both files): acceleration and deceleration 1000.
