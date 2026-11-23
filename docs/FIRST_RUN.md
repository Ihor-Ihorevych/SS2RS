# First run: where is Serious Sam 2?

Roadmap T10-019. The player's game data is never shipped with the engine, so at the first start the program has to find the player's install, check it, and remember the choice. This page describes the three layers that do that and what each rule is. **The wizard is not yet shown by the application window**: see "Not covered" at the end, and read it first if you are deciding whether this is finished.

The three layers need no window and no device, and each lives in the crate it belongs to:

| Layer | Where | What it does |
|---|---|---|
| Discovery, validation summary, memory, start-up decision | `ss2_vfs::locate` (`crates/ss2-vfs/src/locate.rs`) | which folders may hold an install, what the install check says in plain lines, the remembered choice, what to do at start-up |
| The wizard screen | `ss2_ui::data_wizard` (`crates/ss2-ui/src/data_wizard.rs`) | a state machine: input in, commands out, a `DrawList` in canvas units with the placeholder theme |
| The host | `ss2_app::data_location` (`crates/ss2-app/src/data_location.rs`) | answers the wizard's commands with the other two; no window needed |

`ss2-ui` does not depend on `ss2-vfs`: the wizard is handed plain lines and paths, and the host in the app converts.

## Discovery (`ss2_vfs::discover`)

`discover(env, remembered)` asks an injected `Environment` (variables, folders, small text files, a registry value; `SystemEnvironment` is the real machine, `FakeEnvironment` answers from tables for tests) and returns the folders that **list `All_PC_01.gro`**, most preferred first:

1. the **remembered** folder;
2. **`SS2_DATA_DIR`**;
3. the **current folder**;
4. the **program's folder**;
5. every **Steam library**: the Steam folder from the Windows registry (`HKCU\Software\Valve\Steam` `SteamPath`, `HKLM\...\Valve\Steam` `InstallPath`, read by running `reg query` without a window) and from the default places of each OS, plus every `path` of that Steam folder's `steamapps/libraryfolders.vdf` (both the current layout and the old one, parsed by `parse_library_folders`), looking for a `steamapps/common/Serious Sam 2` (or `SeriousSam2`) in each, any case;
6. the usual **GOG** places;
7. the usual **disc install** places.

Rules, each with a test:

- **A folder named twice appears once**, with every source that named it (`Candidate::sources`), whatever the spelling: case only, `/` or `\`, a trailing separator, `.` segments, the `\\?\` prefix (`dedupe_key`). The spelling of the first (most preferred) source is the one shown. Links and 8.3 short names are not resolved, and **on a case-sensitive file system two real folders that differ only in case would be merged** (nobody installs a game twice that way; it was chosen over showing a doubled list).
- **Only the cheap check decides candidacy**: the folder lists `All_PC_01.gro` (any case). Discovery does not open any archive; it reports the number of `.gro` files and the newest patch **from the file names**.
- A folder that the **remembered choice or `SS2_DATA_DIR` named and that fails** is listed in `Discovery::rejected` with the reason (so the screen can say why); a guess (a Steam library, a GOG path, the current folder) that does not exist is silently dropped.
- An empty `SS2_DATA_DIR`, a missing or damaged `libraryfolders.vdf` and a library without the game give nothing and no error. A machine with nothing gives an empty list and the wizard offers to type the path.

**The paths in items 5 to 7 are guesses.** Only the player's own copy has been seen (a folder named `Serious Sam 2` directly holding the `.gro` files). The GOG and disc locations (`GOG Galaxy\Games`, `GOG.com`, `GOG Games`, `Croteam`, `Devolver Digital`, the wine folders, and the folder name a Steam copy uses) are what the author expects, **not verified against a real GOG, disc or Steam install**; a wrong guess costs nothing but a missing suggestion, and the player can always type the path.

## Validation (`ss2_vfs::summarize`)

The folder the player picks is checked with `check_install` (the check behind `ss2-doctor`, T11-013: archives present and readable, patch level from the newest `Patch_*.gro` name, foreign archives) and `summarize` turns the report into:

- `usable`: there is **no error** (warnings do not stop it);
- a **headline**: `Serious Sam 2 found: the install looks complete`, `Usable, with N warning(s)`, or `This folder cannot be used: N error(s)`;
- **lines**: `Archives: 14 found, 14 readable`, `Patch level: 2.100 (build 800000)`, then each problem as `ERROR: ...` or `WARNING: ...` followed by `  -> what to do`.

**Patch level policy.** `NEWEST_KNOWN_PATCH` is 2.100 build 800000 (`Patch_02_100_800000.gro`), the level of the only install the project documents as working. An install **below it is usable but gets a warning** (`the patch level 2.068 is older than 2.100 ...`, with advice) and counts in the warning total; the newest level and anything above it do not. No patch at all is the warning the install check already gives. The version is read **from file names only**; nothing compares file contents.

## Memory (`save_choice`, `load_choice`, `resolve_startup`)

The choice lives in `<config dir>/data_location.toml` (next to `settings.toml`; `ss2_app::data_location::store_path`). **Exactly three keys are stored**: `version`, `path` and `validated_at` (seconds since 1970, passed in by the caller so tests need no clock). Nothing else about the machine or the player.

- **Atomic write**: the text goes to `data_location.toml.tmp` and is renamed over the old file; missing folders are created; a failed write leaves the old file alone. A path that is relative or is not valid UTF-8 is not stored (a clear error).
- **Reading never fails**: `load_choice` gives `Missing`, `Damaged(reason)` or `Found`. Damaged means: not TOML, no `path`, a `path` of the wrong type, empty or blank, relative, over 4096 characters, or holding a control character; a folder or non-text bytes where the file should be. Unknown keys (from a newer build) are ignored, and a missing or bad `validated_at` reads as 0.
- **Re-validation at every start** (`resolve_startup(env, explicit, store)`, no writes): first `--data` (explicit), then `SS2_DATA_DIR`, then the remembered folder. **The first that passes `check_install` is used.** A folder that was *named* and fails is **not skipped silently** (a typo in `--data` does not quietly switch to another install): the wizard is needed, with the reason (`WizardReason`: first run, damaged file, remembered folder no longer works, given folder does not work), which becomes the sentence on top of the screen.

## The wizard (`ss2_ui::data_wizard::DataWizard`)

```text
Searching --found--> Candidates --pick--> Validating --report--> Result --Use this folder--> Done
                        |  ^                                       |
               "Enter a folder path"  Back                    Back (to where it came from)
                        v  |
                      Manual --Check--> Validating
```

Input is `WizardInput` (the menu's `UiInput` plus text, backspace, delete, home, end); output is `WizardCommand`: `Search`, `Validate(path)`, `Accept(path)`, `Quit`. The app answers with `set_candidates` and `set_report`; an answer nobody asked for is ignored.

- **Candidates**: up to five rows at a time (scrolling with the focus), then **Enter a folder path**, **Search again**, **Quit**; with nothing found, a line says so and the actions remain. Long paths lose their middle.
- **Manual**: a text field (cursor by character, not byte; control characters and new lines left out; 1024 characters; a long path scrolls with the cursor and shows `...` where it goes on), **Check this folder**, **Back**. Surrounding spaces and **one pair of surrounding quotes** (what "Copy as path" adds) are removed. An empty field shows a notice instead of checking. Typing while a button is focused moves the focus back to the field.
- **Validating**: nothing can be pressed and Back does nothing (the check cannot be cancelled).
- **Result**: the headline coloured green, amber or red, up to nine wrapped lines (more end with `... and N more lines`), and **Use this folder** (only when usable), **Choose another folder** (back to where it came from, **keeping what was typed**), **Quit**.
- **Back and Escape**: in the manual screen and in a result they go back; in the list **the first Back only focuses Quit and the second quits**, so Escape cannot close the wizard by accident.
- The pointer follows the menu's rules (hover focuses; a click counts only when released on the row it was pressed on); pressing the text field only focuses it.

The host (`Host::run`) turns commands into work: `Search` runs `discover` with the remembered folder, `Validate` runs `check_install` and `summarize`, `Accept` saves the choice with the time; **if saving fails the player is told (a notice) and the program goes on**, and is asked again next time. **The real install's check takes about 0.35 s** (see below), so a window must draw the wizard's "searching" or "checking" screen once before calling `Host::run`.

## How it was tested

Numbers are from the player's own install (`C:\Games\SS2Rewrite\Serious Sam 2`, 14 archives, patch 2.100 build 800000) on this Windows machine.

- **Discovery and memory** (`ss2_vfs`, 61 tests): the key rules (separators, case, `.`, the verbatim prefix, network paths, non-ASCII); the VDF parser (both layouts, the apps lists ignored, escapes, comments, damaged and unterminated text); the order of all seven sources; duplicates and case-only duplicates with merged sources; guesses that fail being silent and the remembered or variable folder being rejected once with a reason; an empty machine; 40 Steam libraries; Windows, Linux and macOS places; spaces, non-ASCII and 800-character paths; the real `SystemEnvironment` on a temporary folder (listing, a 1 MiB limit exactly, a missing file); the summaries (complete, old patch at, below and above the newest level, no patch, a missing base archive, damaged archives, foreign archives, no folder); the memory (round trips of awkward paths, only three keys, atomic write, a failed write keeping the old file, every damaged form, the 4096-character edge, unknown keys, `Missing` against `Damaged`); and the start-up order (first run, a vanished or damaged remembered folder, a remembered folder that lost an archive or got a damaged one, explicit over the variable over the remembered, a named folder that fails not being replaced, nothing written).
- **The wizard** (`ss2_ui`, 54 tests): the text field (insert, cursor, backspace and delete at the edges, characters not bytes, the limit, control characters, `set`), the visible window of a long path for every cursor place, shortening and wrapping, every flow and edge listed above, scrolling over 12 candidates, rows never overlapping or leaving the canvas, the pointer rules, the drawing (colours by severity, truncation with a count, nothing longer than a line even for 500 candidates with 3000-character paths).
- **The whole loop** (`ss2_app`, 11 tests, real wizard, real install check, real folders, no window): first run to a remembered choice and a second start that goes straight in; nothing found, a wrong folder refused, a pasted quoted path with spaces and non-ASCII accepted; an old patch level accepted with its warning; a missing base archive refused; a vanished remembered folder; a damaged file; a choice that cannot be saved; search again and the two-step Escape; `SS2_DATA_DIR` and `--data`.
- **Data-gated** (skip without `SS2_DATA_DIR`): discovery finds the real install from the variable (14 archives, patch 2.100 build 800000); the check passes with no warning (**0.24 s** by itself, **0.35 s** inside the wizard, **0.39 s** to re-validate the remembered choice at the next start: this is what every start would cost today, and it is why the placeholder window does not do it yet); the whole wizard flow with the real folder.
- **Golden images**: six screens at the five sizes of `docs/GOLDEN_IMAGES.md` (30 goldens, each under 190 KB): the list with three folders (one very long), nothing found, path entry with a long path and the cursor in the middle, the clean result, the result with two warnings, and the refusal with two errors. Looking at the first render showed the path line of a result printed over the reason line; it is fixed (the reason is not drawn on a result).
- **Mutation check**: 104 deliberate one-line breakages, applied one at a time to the source with the crate's tests run after each (a script restores the file every time): 52 in `locate.rs`, 45 in `data_wizard.rs`, 7 in the app host. **101 were killed, 3 were equivalent, none survives.** The first round left **21 alive** (and two that did not compile), which is what the tests added afterwards are for: the VDF key `Path` in another case, a numbered pair after a closed section (and the paths in those tests are now compared as text, because on Windows `PathBuf` equality ignores the kind of separator and hid a wrong escape), a failed rename leaving the temporary file, every Steam root and the program files variables of the store places (a table test over the four Linux roots and the five Windows ones), the exact window of the text field (forty characters before the cursor, twenty after, the dots from the second character on), a line exactly as wide as the limit, the Right key on a button, the reason not being drawn over a result, a single warning tinting the headline, the exact count in `... and N more lines`, the cursor going to the end when the manual screen is entered again (the old test never moved it), a notice cleared by `start`, and in the host the remembered folder being offered first together with the other sources that named it. **Equivalent:** the `Option::is_empty` check on a registry value (unreachable, because the value is trimmed and split before it: the code was changed to not carry it), and two bounds in the candidate list (`n.saturating_sub(scroll)` and `.min(n)` on the end of the showing range; the scroll never passes the last full window, so they never changed anything: removed). Those 21 breakages (with the two that did not compile, whose text had a wrong type, re-written, and the registry one re-written for the new code) were run again after the tests were added and **all 21 were killed**; the others were not re-run, since the new tests only add to the old ones. The golden shots were not part of this check, and the mutants ran only the crate's own tests.

## Not covered (follow-up T10-019b)

- **The window never shows the wizard.** `ss2-app` does not call the host or start-up decision, so **at first run nothing appears yet**, and the remembered choice is not read at start. The reasons: the window is being moved onto the platform layer (T03-033, claimed by someone else, same files), and the placeholder app uses no game data, so a mandatory wizard would only add 0.4 s and a blocking screen. T10-019b: add a wizard scene to the app, map `KeyboardInput` text and keys to `WizardInput`, run `Host::run` after drawing the searching and checking screens (or on a worker thread), show the wizard when `resolve_startup` says so, hand the folder to the loaders, and add a `--forget-data` or settings entry to choose again.
- **No folder-picker dialog**: the player types or pastes the path. A native picker (and on Linux the file-chooser portal, `docs/PACKAGING.md`) needs the platform layer; Android's import wizard is T10-024 (Phase 2).
- **The Windows registry is read by running `reg query`**, not through a registry API (the workspace forbids `unsafe` and has no registry crate): its output is in the OEM code page, so a Steam folder with non-ASCII letters in its name may not be found that way (the default places and the library list still are). It was not run against a real Steam install on this machine.
- **The GOG, disc and Steam folder names are unverified guesses** (see Discovery). Linux and macOS were tested only against a fake machine, not run on those systems, and macOS's own places (apart from Steam's) are not searched.
- **Only file names tell the version.** An install whose archives were renamed or replaced can pass; the contents are not compared.
- **A path that is not valid UTF-8 cannot be remembered** (TOML text); the wizard can still use it for the session.
- **No localization**: the strings are English literals (T10-011); the placeholder look has no art; the text field has no selection, clipboard shortcuts (a paste arrives as text from the window layer, which does not exist yet) or IME (T10-022).
- **Wide characters in a path may overflow a row**: widths are estimated in characters (a proportional font cannot be measured here), so a path of very wide letters can run past the edge of its row.
- Not run: Linux or macOS tests, a real GOG, disc or Steam install, the Windows registry path on a machine with Steam.
