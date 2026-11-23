# Death, respawn and end-of-level screens

`ss2_ui::session_screens` (T10-020) holds the two screens that come between playing and the menu. The third part of the task, the **pause menu**, already existed (`Menu::open_pause`, a "Paused" screen with Resume, Options and Quit to Menu, with golden images). Both new screens are state machines like the menu: they take `UiInput` and ticks, answer with a command for the app, and draw into a `DrawList` in canvas units with the placeholder theme. They know nothing of the game.

## `DeathScreen`

`DeathScreen::new(DeathMode)`:

- **`Solo`**: "You Died", "Retry from the last checkpoint", buttons **Retry** (`DeathCommand::Retry`) and **Quit to Menu**.
- **`Coop { respawn_in_ticks, ticks_per_second }`**: "You Died", **"Respawning in N"** (whole seconds, rounded up, so it shows 1 until it reaches 0; a tick rate of 0 does not divide by zero) or **"Waiting for your team"** while the wait is unknown, buttons **Spectate** (T08-024) and **Quit to Menu**. `update()` counts the countdown down one tick and gives `DeathCommand::Respawn` **once** when it reaches 0; `set_respawn_in(Some(n))` is how the server's latest word arrives (a new countdown can finish again, `None` means "waiting").

Back, Left and Right do nothing: the death screen cannot be dismissed by accident. Up and Down wrap; hovering focuses a button; a pointer activates only when released over the button it was pressed on (the menu's rules, tested the same way).

## `SummaryScreen`

`SummaryScreen::new(LevelStats)` with the level's name, kills and total, secrets and total, seconds played, an optional par time and an optional score (T09-023 will keep the counters; this takes the numbers it is given). The rows (**Kills** `12 / 20  60%`, **Secrets**, **Time** `3:25` or `1:02:05`, then **Par** and **Score** if given) **count up one after the other**: each takes 45 ticks and starts 30 after the one before, so three rows take 105 ticks and five take 165; a number is never shown above its final value and a percentage follows the number shown. **A press (Enter, or a click) while counting skips to the end** and does not continue; a press after that, on the **Continue** button (drawn only when the count is done), gives `SummaryCommand::Continue`. A click that skipped does not also continue when it is released. A level with none of something shows `0 / 0` (no division by zero), and more kills than the level said (a script spawned one) shows a percentage capped at 999 instead of wrapping.

## Not allocation-free

Unlike the menu (`docs/UI_BUDGET.md`), these screens build their number text with `format!` when drawn (the countdown line, each summary row), a handful of short strings a frame. They are on screen for seconds, not part of a frame budget; if one proves hot, a cache keyed on the shown numbers is a small change.

## How it was tested

Twenty-one unit tests: the buttons (wrap, hover, press and release on the same and on another button, outside every button), the solo and co-op screens (the commands, Back ignored, the countdown shown in seconds at 149, 120, 61, 60, 1 and 0 ticks left, a single respawn, a new countdown rearming it, an unknown wait, a solo screen ignoring a countdown, a zero tick rate), the summary (the exact rows and the lengths with and without par and score, the count at tick 0 and at 22 ticks, never above the final value, skip, continue, a click that skips, other keys, the button only when done, zero totals, a runaway percentage, the clock from 0 to 7325 s). The four screens are also in the golden matrix of `docs/GOLDEN_IMAGES.md`: the solo and co-op death screens, the finished summary and a summary part-way through its count (tick 70: the time still counting, the par and score not started), each at the five sizes (twenty goldens, which pass on Vulkan, DX12, GL and the software adapter; looking at the first render showed the summary's row text sitting too low in its stripes, which is fixed). **Twenty-five deliberate breakages** of the logic: twenty-three failed a test at once; **two survived**: a pointer press that does not focus the button (a touch screen presses without hovering first: now tested, and killed), and one **equivalent** (continuing while the numbers still count, which cannot differ because a press always skips first).

## Not covered (follow-up T10-020b)

- **Nothing shows or feeds them yet**: the app does not switch to the death screen when the player dies, nor to the summary at the end of a level; the stats are not collected (T09-023), the checkpoint reload and the respawn happen nowhere (T09-023, T08-023, T08-024b), and there is no input mapping (the menu's `UiInput` is reused).
- **Placeholder look**: the layout is the menu's, with no fade or screen-shake on death, no rank or medal on the summary, no per-player rows for co-op (names, kills, deaths), and no sound.
- **No text for other languages**: the strings are English literals until the localization pipeline (T10-011).
- **Death cause and killer's name** ("killed by …") are not shown: the game does not report them yet.
