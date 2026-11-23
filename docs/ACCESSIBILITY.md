# Accessibility review and checklist

Status: **draft (T12-012)**. This page lists what the original game offers according to the specs in
`docs/re/`, what SS2RS should provide, and a checklist with an owner task and a status for each item.
Nothing on this page is implemented yet unless a status says so.

## 1. Scope and sources

- **Source column.** `spec: <file>` means the fact is stated in that file under `docs/re/`. `new requirement`
  means this review proposes it; it needs maintainer agreement before it becomes a roadmap change.
- **Owner column.** A roadmap task ID from `docs/roadmap/*.md`. `no task yet` means no task covers the item
  today; the item is then listed in section 6.
- **Limits.** The specs say what the install contains. They do not describe the original's menus or
  controls in full, so this page does not guess at them. Where a spec leaves a question open, the
  checklist says so.

## 2. What the original game offers (from the specs)

- **Options that exist as data.** An "invert look" option label exists in the menu data
  ([localization.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/localization.md), `opslInvertLook`). The default horizontal field of view is 80 degrees
  ([player-rules.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/player-rules.md)).
- **Difficulty.** Five levels (Tourist, Easy, Normal, Hard, Serious) change enemy health, speed, think
  rate, player damage, auto-aim and health recharge. The table applies to single player and co-op
  ([player-rules.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/player-rules.md), [game-modes.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/game-modes.md)). Tourist alone has health recharge.
- **Auto-aim and auto-jump.** Auto-aim strength is set per difficulty and an auto-aim angle graph exists
  ([player-rules.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/player-rules.md), [rsc-ctl-schema.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/rsc-ctl-schema.md)). Auto-jump is on in
  single player and off in multiplayer ([player-rules.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/player-rules.md)).
- **Crosshairs.** The player crosshair set has 20 textures; crosshair size is a graph
  ([rsc-ctl-schema.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/rsc-ctl-schema.md)).
- **Subtitles and credits.** 641 subtitle cues in 72 `.srt` files, one per movie. Each cue can name a voice
  `.wav`, so the subtitle file schedules the voice ([misc-types.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/misc-types.md),
  [cutscenes.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/cutscenes.md)). Subtitles, voices and movies are English only in the install. A German
  overlay covers level and model descriptions and two credits files ([localization.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/localization.md)).
- **Text style for conversations.** A `.tfx` style sets font, placement and timing for on-screen text and
  conversation subtitles. The three conversation files appear to carry a background box colour
  ([fx.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/fx.md), not confirmed).
- **Vibration.** Five controller vibration patterns (Xbox) exist as data ([fx.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/fx.md)).
- **Camera shake, bloom and haze.** Twelve shaker entities in seven worlds; bloom and haze volumes in most
  (haze in 45 worlds, bloom in 33). Underwater overlays, fades and damage flashes are not stored in the worlds
  ([environment.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/environment.md)).
- **Sound.** Positional sources with a hot spot and a fall-off radius; two-source music crossfade
  ([audio.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/audio.md), [wld-sound-effects.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/wld-sound-effects.md)).
- **Not in the specs read for this review:** the default key and button bindings, the pause rules, the
  rule for skipping a movie or camera cutscene ([cutscenes.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/cutscenes.md), open question 1), and any
  screen-reader or input-hint support.

## 3. What SS2RS should provide

1. Every action is rebindable on keyboard, mouse and gamepad. No action is hard-coded to a key.
2. Subtitles for every spoken line, and text for objective and Netricsa messages, sized and placed by the player.
3. Text and HUD that do not rely on colour alone, with a scale setting and contrast goals.
4. Options that reduce or disable camera shake, head bob and sway, screen flashes and bloom; a field-of-view setting.
5. Separate volume buses, and visual cues for sounds that matter to play (goal).
6. Difficulty levels and assist settings that are visible and documented on the settings screen.
7. Menus usable with keyboard or gamepad only; screen-reader support as a goal, not a 1.0 requirement.

## 4. Review checklist

Status values: **Open** (not started), **In progress**, **Done** (evidence recorded, see section 5), **N/A**.
Every row starts as **Open**.

### 4.1 Input

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| IN-1 | All game, menu and console actions can be rebound on keyboard, mouse and gamepad | new requirement | T03-005, T10-012 | must | Open |
| IN-2 | No hard-coded keys in gameplay, menu or console code; defaults come from the action map | new requirement | T03-005 | must | Open |
| IN-3 | Bindings are saved in the settings file and survive upgrades (migration) | new requirement | T03-014 | must | Open |
| IN-4 | Hold or toggle option for actions such as crouch, zoom and run | new requirement (listed in T10-010) | T10-010 | should | In progress |
| IN-5 | Gamepad dead zone and sensitivity settings; rumble can be turned off or reduced | fx.md (vibration patterns exist); new requirement | T03-003 | should | Open |
| IN-6 | Invert look and mouse sensitivity options | spec: localization.md (`opslInvertLook`) | T10-012 | must | Open |
| IN-7 | Menus are fully usable with keyboard or gamepad only, with a visible focus marker | new requirement | T10-005 | must | Open |
| IN-8 | Input hints show the button or key currently bound to an action, by name, and update after rebinding | new requirement | no task yet | should | Open |
| IN-9 | Movie and cutscene skip and pause are rebindable and shown in the on-screen prompt | spec: cutscenes.md (skip rule open); new requirement | T06-011, T10-021 | must | Open |
| IN-10 | Single-player pause is available where the design allows; multiplayer pause rule stated | spec: none (pause semantics open); new requirement | T10-020, T08-003 | should | Open |
| IN-11 | One-handed and alternative schemes (sticky aim, auto-fire, single-hand layouts) as goals | new requirement | no task yet | could | Open |

### 4.2 Subtitles and text

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| SU-1 | Subtitles for every spoken line in movies and in-game conversations, timed to the voice cue | spec: misc-types.md, cutscenes.md | T06-012, T10-021 | must | Open |
| SU-2 | Text for objective, Netricsa and pickup messages, readable without audio | spec: player-rules.md (texts from localization tables); new requirement | T10-003 (partial) | must | Open |
| SU-3 | Subtitle size, font and background box options | spec: fx.md (`.tfx` background box); new requirement | T10-010 | should | In progress |
| SU-4 | Shown text comes from a string table (key and default text), never from baked textures | spec: localization.md (`CTransString`) | T10-011, T10-002 | must | Open |
| SU-5 | Language choice from a documented table format, with fallback to the default text | spec: localization.md (tables absent from install) | T10-011 | should | Open |
| SU-6 | Localized subtitle and credit files replace base files by path (overlay) | spec: localization.md, mods.md | T10-011 | should | Open |
| SU-7 | Menus and HUD text scale without overflow or clipping, checked for longer translations | new requirement | T10-017, T12-013 | should | Open |
| SU-8 | Credits (`.crt`) positions scale with the screen; the 640 x 480 layout is not pinned in pixels | spec: cutscenes.md (`pos` in 640 x 480) | T06-011 | should | Open |

### 4.3 Visual contrast and colour

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| VC-1 | Health, armor, ammo and crosshair state readable without colour alone (shape, number or icon as well) | new requirement | T10-003 | must | Open |
| VC-2 | Colour-blind-safe HUD palette option | new requirement (listed in T10-010) | T10-010 | should | In progress |
| VC-3 | Keyboard focus shown by more than colour (the menu test checks an orange focus highlight; add a shape or outline cue) | new requirement; spec-level test: TESTING.md 3.4 | T10-005 | must | Open |
| VC-4 | Contrast goals: body text at least 4.5:1 and HUD icons and large text at least 3:1 against their background | new requirement | no task yet | should | Open |
| VC-5 | Global UI scale setting, tested at three resolutions and two aspect ratios | new requirement | T10-017, T10-015 | should | Open |
| VC-6 | Crosshair style, colour and size options (20 original crosshair textures; size graph) | spec: rsc-ctl-schema.md | no task yet | should | Open |

### 4.4 Motion and photosensitivity

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| MO-1 | Option to reduce or disable camera shake (shaker entities are data) | spec: environment.md (`SShakerEffectEntity`) | T09-008 (no toggle task) | should | Open |
| MO-2 | Option to reduce or disable head bob and sway | new requirement (named in T09-008) | T09-008 (no toggle task) | should | Open |
| MO-3 | Option to disable screen flashes: damage flashes, fades and effect flashes | spec: environment.md (not stored in data); new requirement | T10-010, T04-031 | must | In progress |
| MO-4 | Bloom and haze strength can be lowered or turned off | spec: environment.md (bloom and haze entities) | T04-020 | should | Open |
| MO-5 | Field-of-view setting; default 80 degrees horizontal | spec: player-rules.md (`gpp_fFOV`) | T10-012 | must | Open |
| MO-6 | Weapon viewmodel keeps its own FOV so a wide view does not distort the weapon | new requirement (T04-030 has a separate FOV pass) | T04-030 | could | Open |
| MO-7 | Warning and review for content with rapid flashing light | new requirement | no task yet | should | Open |

### 4.5 Audio

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| AU-1 | Separate volume buses (master, music, effects, voice, UI), each adjustable and mutable | new requirement (bus list in T06-017) | T06-017 | must | Open |
| AU-2 | Voice volume independent of subtitles; subtitles stay on when voice is muted | spec: cutscenes.md (cue drives the voice) | T06-017, T06-012 | should | Open |
| AU-3 | Visual cue for important sounds that are off screen (enemy, pickup, boss) | spec: audio.md (positional sources exist as data); new requirement | no task yet | could | Open |
| AU-4 | Loud-sound limiting or ducking option | new requirement | T06-006 | could | Open |

### 4.6 Gameplay assists

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| GA-1 | Five difficulty levels as in the original table, for single player and co-op | spec: player-rules.md, game-modes.md | T09-031, T09-002 | must | Open |
| GA-2 | Tourist health recharge implemented with its rate and delay stated in the docs | spec: player-rules.md (rate and delay open) | T09-031 | must | Open |
| GA-3 | Auto-aim strength per difficulty as data; an on or off switch for auto-aim | spec: player-rules.md; on or off switch: new requirement | T09-031 (switch: no task yet) | should | Open |
| GA-4 | Auto-jump setting: on by default in single player, off in multiplayer, changeable | spec: player-rules.md (`gpp_bAutoJumpSP`); new requirement | no task yet | should | Open |
| GA-5 | Settings for assists and difficulty shown on the settings screen with a plain description | new requirement | T10-012 | should | Open |

### 4.7 Platform

| ID | Requirement | Source | Owner | Priority | Status |
|---|---|---|---|---|---|
| PL-1 | Settings file stores all accessibility options and can reset them to defaults | new requirement | T03-014 | must | Open |
| PL-2 | Menu items expose their labels and focus to a platform screen reader (goal only) | new requirement | no task yet | could | Open |
| PL-3 | Menu strings are available as key and text pairs so a reader can use them | spec: localization.md (`CTransString`) | T10-011 | could | Open |

## 5. How to review

- **Who tests.** A maintainer or contributor who owns a legal copy of the game runs the checks on their own
  machine. Checks that need game data follow the data-gated rule in [TESTING.md](TESTING.md) and skip
  without `SS2_DATA_DIR`. Players with access needs can take part through the playtest program (T12-011,
  `P2`); their reports go through the same record format.
- **How results are recorded.** One line per checklist ID, in the PR or issue that completes it:
  `ID | pass / fail / N/A | method (one line) | commit`. No screenshots, video or game text, following
  the evidence rules in [PARITY.md](PARITY.md), section 2. Set the status in section 4 in the same PR.
- **Defects.** Use the bug template (`.github/ISSUE_TEMPLATE/bug.md`), or the visual-parity template for a
  rendering difference, and put the checklist ID in the report. Labels and severity follow
  [TRIAGE.md](TRIAGE.md), sections 2 and 3.
- **Automated checks.** The menu golden-image test is the only one that exists today
  ([TESTING.md](TESTING.md), section 3.4). UI golden images for each screen are planned (T10-015).
  The logic behind IN-4, SU-3, VC-2 and MO-3 (contrast ratios, a palette checked to stay distinguishable for colour-blind
  players, a limit of three flashes a second, hold or toggle) is unit-tested in `ss2_ui::accessibility`; see
  [ACCESSIBILITY_OPTIONS.md](ACCESSIBILITY_OPTIONS.md). Nothing checks a drawn screen for contrast or flashing yet.

## 6. Gaps (items with no task yet)

- IN-8 input hints by name; IN-11 one-handed and alternative schemes (goal).
- SU-2 part: subtitles for objective and Netricsa messages (T10-003 covers the objective HUD only).
- VC-4 contrast ratio measurement; VC-6 crosshair options.
- MO-2 head bob toggle and MO-1 camera shake toggle (T09-008 implements the effects but names no option).
- MO-7 flashing-light review.
- GA-3 auto-aim switch; GA-4 auto-jump setting.
- AU-3 visual cues for sounds.
- PL-2 screen-reader support (goal only).

Suggested next step: a maintainer decides which of these become roadmap tasks (new IDs appended, per
`docs/roadmap/README.md`).
