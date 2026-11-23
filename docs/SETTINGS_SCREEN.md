# The settings screen model

T10-012 asks for the settings screens. What exists is the **model** of one: the tabs, the rows, the pending edits, Apply, Revert, Reset, the "keep these settings?" countdown, and the adapters that fill the rows from the settings the game really has. **Nothing draws it yet** (see "Not covered").

It is two modules, split by the dependency direction of the crates:

| Module | Crate | What it knows |
|---|---|---|
| `ss2_ui::settings_model` | `ss2-ui` | Nothing about graphics or sound. A row is a name, a label, a kind (`Choice`, `Range`, `Count`, `Toggle`), a default and a text value. Tabs, pending and applied values, the countdown. |
| `ss2_game::settings` | `ss2-game` | `QualitySettings`, `AccessibilitySettings`, the bus volumes, the window mode and resolution: which rows there are, how current settings fill them, and how applied rows become settings again. |

**Why the adapters are in `ss2-game`.** `ss2-render-api` (which owns `QualitySettings`) depends on `ss2-ui`, so `ss2-ui` cannot name it; the generic model has to be in `ss2-ui` (the screens will be drawn there) and the adapters have to be in a crate that may see `ss2-ui`, `ss2-render-api`, `ss2-audio-api` and `ss2-platform-api`. `ss2-app` was the other candidate and does not work: it is a binary crate (no library target, so `cargo test -p ss2-app --lib` has nothing to run), and public functions nobody calls in a binary are dead code to the compiler. A new crate would have needed an architecture change and an RFC. `ss2-game` already depends on the render API and sits at the top of the graph in `docs/architecture.md`; its "gameplay" description is a poor fit for this module, and moving it to a crate of its own when the screens are drawn is part of follow-up T10-012b.

## The rules of the model

All values are **text**, in the spelling the existing option sets already use (`on`/`off`, `0.85`, `msaa2`), so a row, a console command and a settings file agree.

- **Pending, not applied.** `set_text`, `step`, `reset_row` and `reset_all` change only the *pending* value. The game's settings (`applied_settings(&screen)`) change only at `apply(now)`. `revert()` throws the pending edits away; `pending_settings(&screen)` gives the settings with the edits for a live preview.
- **Refusals in the existing words.** A value that is not valid is refused with `<row> takes <what it takes>` (`render_scale takes a number from 0.5 to 1`, `crouch takes hold or toggle`) and the state is left exactly as it was. A test sets each of the 11 graphics rows to 52 probe texts and each of the 10 accessibility rows to 38 (952 comparisons) and requires the screen and `QualitySettings::set` / `AccessibilitySettings::set` to agree on accepted or refused, on the canonical text of every accepted value and on the message of every refusal, word for word.
- **Canonical spelling.** `ON`, `true` and `1` become `on`; `0.50` becomes `0.5`; `-0` becomes `0`; a choice keeps the case of its own list. The same text is what the game's `set` and `get` use.
- **Changed list.** `changed()` lists `(row, from, to)` for every row whose pending value differs from the applied one, in screen order; `changed_in_tab(tab)` is the count for a mark on a tab. Setting a value back to what is applied is not a change.
- **Stepping.** A **choice** cycles and wraps either way. A **range** moves along the grid that starts at its minimum (so a value between two grid points moves to the next one in the direction asked) and **stops at the ends**; the result is rounded to four decimals, so repeated steps read `0.9`, never `0.90000004`, and a step through zero never reads `-0`. A **count** moves by its step and stops at the ends. A **toggle** flips either way. `press(NavInput)` maps up/down to the focus (wrapping in the tab), left/right to a step, activate to a flip or a step up, and reports `Nav::Limit` when a row was already at its end.
- **Display text.** `display_value` and `view(tab)` give what to draw: `On`/`Off`, `85%` for a percentage row, `80°` or `8x` for a suffix, a capital letter and spaces for underscores in a word (`Extra large`), plus per row whether it changed, is at its default, has the focus, can go down or up, and is risky.
- **The countdown.** Rows marked **risky** (here: the window mode and the resolution, the two that can leave a player with a screen they cannot read) start a question when applied: `apply(now)` returns `confirm_until = now + 15 s` (the time is the caller's argument; there is no clock in the model). The new values are in force while the question is open. `confirm()` keeps them; `revert_applied()` or `tick(now)` once `now >= deadline` puts the previous values back and returns the list of changes to undo, with pending equal to applied again. While the question is open rows cannot be edited or applied (`Locked`), but the focus can move. `confirm_prompt(now)` reads `Keep these settings? Reverting in 12 s` and counts whole seconds upward, so it never shows `0 s` before the time is out. A time that is not a finite number is ignored (`tick`) or refused (`apply`: nothing is applied). Fifteen seconds is my guess, not a researched number.
- **Reset to defaults** is an edit: it sets every pending value to its default and still needs Apply (and, for the resolution, confirmation).

## The tabs

| Tab | Rows |
|---|---|
| **Video** | `window_mode` (windowed, borderless, exclusive: risky), `resolution` (risky), `vsync`, `fov` (30 to 120 degrees in steps of 5, default 80: the range and default of the `r_fov` console variable), `aspect` (letterbox or expand: `ss2_ui::scale`), `ui_scale` (50 % to 100 %) |
| **Graphics** | every option of `QualitySettings::options()` (11), with steps 0.25 for `lod_bias`, 0.1 for `view_distance`, 0.05 for `particle_density` and `render_scale`, 50 for `max_decals`; `set_quality_preset(screen, tier)` sets the pending values of all of them and `quality_label_of(screen)` names the preset they equal or `Custom` |
| **Audio** | `master_volume`, `music_volume`, `sfx_volume`, `voice_volume`, `ui_volume`: one per `ss2_audio_api::Bus` in the order of `Bus::ALL`, 0 to 100 % in steps of 10 %; `AudioSettings::apply(&mut dyn Audio)` and `read(&dyn Audio)` talk to the mixer |
| **Accessibility** | every option of `AccessibilitySettings` (10): text size, colour-blind mode, subtitle size and background, the three reductions, hold or toggle for crouch, sprint and aim |

The resolution row lists what the caller gives `build_screen` (use `ss2_platform_api::resolution_list` of the monitor), plus the current resolution if it is missing, smallest first (so "increase" goes to a larger resolution). Its default is 1920x1080 if the list has it, else the largest.

`GameSettings::from_core` and `write_core` map the five values the existing settings file has (`vsync`, `fullscreen` as borderless, master, music and effects volume) to and from the screen's settings; the rest of the screen's settings have no file.

`section_of(row)` and `affected_sections(&changes)` say which part of the game a change touches (video, graphics, audio, accessibility), so the code that applies a change knows what to redo.

## How it was tested

`ss2_ui::settings_model`: 30 unit tests and a doctest. They cover: bad descriptions (26 distinct refusals at build time), every kind's accepted and refused spellings and its exact refusal text, pending versus applied, the changed list, stepping every kind including grid and noise cases, the ends, a fine grid of four decimals and a 32-bit overflow, revert and reset, apply and the round trip, the countdown (exact deadline, `now` before, at and after it, not-finite times, locked operations, undo lists), display text, the view flags, tabs and focus with wrapping, navigation inputs, and a 4000-step deterministic random walk that checks after every step that **every value is in canonical valid spelling, nothing reaches the applied values except Apply and the confirmation, and a refused edit changes nothing**.

`ss2_game::settings`: 24 unit tests and a doctest, including the two differential tests above, a round trip of five different `GameSettings` through the screen and back, a walk of 45 steps up, down and up again over **every row** that requires the game's own validation to accept each intermediate value, a walk of every range row over its whole grid (every text at most three decimals, and the number of distinct values as predicted), the risky rows being exactly two, the undo of a rejected resolution, the preset names, the mixer round trip on `NullAudio`, and the settings-file mapping.

**Mutation checks.** 142 deliberate one-line breakages, applied one at a time to the source file and run against the module's tests (flipped comparisons, off-by-one ends, a dropped clamp, swapped constants and rows, removed assignments, `all` for `any`, a floor for a ceiling).

- `settings_model.rs`: 85 breakages. 78 failed a test at once. **Seven survived my first tests:** (1) and (2) rounding to two or three decimals instead of four (no test had a step finer than 0.05: a grid of 0.0125 now does); (3) dropping the `-0` guard of a step (a test now walks 100 grids through zero and requires the text never to be `-0`; it fails without the guard, so the guard is needed); (4) `is_default` compared with the applied value instead of the default; (5) the badge count `changed_in_tab` skipping the last row of a tab. These five are real gaps; each got a test, and all five now fail a test. (6) `focused` without the `tab == self.tab` check is **equivalent** (a row index is unique across tabs and the focus is always inside the current tab), so I removed the redundant check. (7) was a mistake of mine: my "breakage" appended an empty string, which changes nothing; it is not a mutation.
- `settings.rs` (the adapters): 57 breakages. 54 failed a test at once. **Three survived:** removing the `sort` in `affected_sections` (the tests only passed lists that were already in order), and the two fallbacks for an option this build does not know (the default of a range or a choice). Tests added; all three now fail a test. Before the run I also removed two pieces of code that no test could reach (a lock check made redundant by `set_text` reporting the lock first, and an unreachable "Custom" branch).
- After the fixes only the survivors were re-run (8 breakages, all killed), not the whole list again; the full suites were run again for the gate.

## Not covered (follow-up T10-012b)

- **Nothing draws the screen** and nothing in `ss2-app` creates one: no layout, no mouse, touch or gamepad hookup (it should use `ss2_ui::focus`, T10-005b, for pointer and a slider that takes left and right), no sliders, no scrolling for a long tab (Graphics has 11 rows), no "Preset" selector. Labels are English literals; localization is T10-011.
- **Nothing applies the result.** No code re-creates the window for a new mode or resolution, sets vsync, gives the camera the field of view, hands `QualitySettings` to the renderer, or sets the `r_fov` console variable; the screen returns the list of what changed and the settings to apply, and that is all.
- **No persistence except what already exists.** Only `vsync`, fullscreen and three volumes are in the settings file (`ss2_core::settings`); quality, field of view, aspect, UI scale, the voice and interface volumes and the accessibility options are lost at exit. `from_core` and `write_core` exist but nothing calls them.
- **No monitor list from the window system**: the resolution list is whatever the caller passes. `ss2_platform_api::resolution_list` can compute it from a `MonitorInfo`, but nothing in the app gathers monitors for the screen, and the refresh rate and the monitor itself are not rows.
- **No controls tab (T03-005 rebinding), no gameplay tab and no network tab**: the task title names them; they are not done. Controls need the binding model of T03-005 and a "press a key" capture state; gameplay and network have no settings yet to list.
- **Choice words are shown capitalised, not translated or prettified**: `anti_aliasing` reads `Msaa2`, not `MSAA 2x`.
- **The 15-second countdown is a guess**, and is not tested against a real display change (no device was involved).
- **No "apply on close" or "unsaved changes?" flow**: `Nav::Back` is reported and the screen decides.
