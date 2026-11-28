# Oracle harness: measuring the running original game

`tools/oracle` (T11-008) reads numbers out of a **running copy of the original game** (the tick or time, the player's position, velocity and health, ...) at a fixed rate and writes them to a CSV file. Its purpose is to turn the `medium`-confidence specs in `docs/re/`, which were read from code, into numbers measured on the real game (T01-038 and T01-039, and the `run-time checks` tasks that depend on them).

```
python -I tools/oracle/oracle.py --help
```

- **Python 3.9+, standard library only** (`ctypes` against `kernel32`). No `unsafe` Rust, no Frida, no packages to install. Windows only (the game is a Windows program).
- **Read-only on the game.** The process is opened with `PROCESS_VM_READ | PROCESS_QUERY_INFORMATION` and nothing else: no memory writes, no injection, no patching, no breakpoints, no suspending threads, no input events. It cannot change what it measures, and a unit test (`test_readonly.py`) fails if the sources ever name an API that could.
- **No game data and no addresses in this repository.** The tool needs a *target map* (which memory cell is which field). That map comes from your own reverse-engineering work, lives on your machine only and is git-ignored. See "The target map" below and `CLEANROOM.md`.

## Quick start

1. Make your own target map (below) at `%LOCALAPPDATA%\ss2rs-oracle\targets.json`, or anywhere with `--config PATH`.
2. Start the game yourself. The harness never starts, drives or closes it.
3. In the repository:

```
python -I tools/oracle/oracle.py --validate-config                     # parse and check the map; needs no process
python -I tools/oracle/oracle.py --list-modules                        # the game's modules with their run-time base addresses
python -I tools/oracle/oracle.py --dump-fields                         # read every field once and print it
python -I tools/oracle/oracle.py --scenario menu-idle --out out/menu-idle.csv --duration 10
python -I tools/oracle/oracle.py --wait-for-process Sam2.exe --scenario level1-start --out out/l1.csv --duration 30 --rate 60
```

Defaults: the process is `Sam2.exe`; the rate is 30 samples per second; the capture ends at `--duration` or `--samples`, when the process exits, or on Ctrl-C (the file is complete and the sidecar is written in every case). An existing output file is not overwritten without `--force`. Keep the output outside the repository tree (`/oracle-out/` is git-ignored as a convenience).

| Option | Meaning |
|---|---|
| `--config PATH` | the target map (default `%LOCALAPPDATA%\ss2rs-oracle\targets.json`) |
| `--pid N`, `--process NAME`, `--wait-for-process NAME` | which process: by id, by exe name now, or wait for it to appear (`--wait-timeout`, default 300 s). Two processes with the same name are refused: pass `--pid` |
| `--validate-config` | check the map and exit (needs no process) |
| `--list-modules` | print every module of the process: name, base, size (the base moves under ASLR, which is why the map stores module-relative offsets) |
| `--dump-fields` | read each field once; `(invalid)` marks a cell that could not be read; exit status 4 if any is invalid |
| `--scenario NAME`, `--out FILE.csv` | a capture; `--out` gets the CSV, `--out` plus `.json` gets the sidecar |
| `--rate HZ`, `--duration S`, `--samples N` | sampling rate (default 30, at most 1000), stop conditions |
| `--install DIR` | the install folder for the build id (default: derived from the process's exe path) |

Exit status: `0` ok, `2` usage or config error, `3` cannot find or open the process, `4` no sample, or `--dump-fields` found an invalid cell.

## Output

**The CSV contains numbers only.** One header row, then one row per sample:

| Column | Meaning |
|---|---|
| `sample` | 0, 1, 2, ... |
| `t_wall_s` | when the sample *started*, in seconds from the start of the capture (monotonic clock) |
| `jitter_s` | how late the sample started compared with its own slot `k / rate` (never negative); the true spacing of any two rows is `t_wall_s` of the later minus the earlier |
| `read_s` | how long reading all fields of this sample took |
| `valid` | 1 if every field was read, else 0 |
| `n_invalid` | how many cells of this row are empty |
| one column per field | the configured field names, in the map's order; `name[i]` for repeated fields |

An **empty cell** means "not readable right now" (a null pointer on a chain, for example while a level loads, or a module that has not loaded yet); the capture never stops for it. Integers are exact; floats use the shortest text that reads back to the same value at the field's own width (`f32` is not printed with 17 digits); `nan` and `inf` are written as such.

The loop does not burst to catch up: when a sample takes longer than a slot, the skipped slots are counted as `missed_slots` and the next sample waits for its own slot. **Use a game-time column (the engine's time or tick) as the time base of any analysis**, not `t_wall_s`: the two clocks are only loosely related (the game's frame rate and its step rule are not those of this tool, `docs/re/main-loop.md`). Reads are not atomic with the game's steps (the game is neither stopped nor slowed): the cells of one row are read one after another within `read_s` (normally well under a millisecond) and can come from two different steps, so a row is a best-effort snapshot. Put the time column first in the map: it is then the oldest value of the row.

The **sidecar** (`FILE.csv.json`) holds: the scenario name, start time (UTC), rate, tool version, process name and id and its pointer width, the **install's build id** (read from the files on disk, never from the process: size, SHA-256 and PE time stamp of `Sam2.exe`, `Engine.dll`, `Core.dll` and `Sam2Game.dll`, the names and sizes of the `Patch_*.gro` archives, and one 16-hex-digit id over all of it), the SHA-256 of the target map file, the list of columns with their types, and the capture statistics (samples, missed slots, jitter and interval and read-time statistics, why it ended). It contains **no offsets or addresses**.

## The target map

A JSON file mapping field names to a place in the process: a module, a chain of numeric offsets and a type.

```json
{
  "format": 1,
  "fields": {
    "example.time": { "module": "Example.dll", "offsets": ["0x1000"], "type": "i64", "scale": 1e-6 },
    "example.player.x": { "module": "Example.dll", "offsets": ["0x2000", "0x10", "0x40"], "type": "f32" },
    "example.slots": { "module": "Example.dll", "offsets": ["0x3000", "0x20"], "type": "f32",
                       "repeat": { "count": 4, "stride": 4, "at": 0 } }
  }
}
```

**Every value above is an invented example.** Real values come from your own work with a disassembler on your own copy of the game, and they **must never be committed, pasted into an issue, a pull request or a spec** (`CLEANROOM.md`, `docs/re/conventions.md` section 5: no raw addresses, offsets or pointer chains in the repository). `targets.json` and `*.local.json` are in `.gitignore`; `cargo xtask guard` still applies. Share *results* (names of fields and measured numbers), not the map.

How a chain is read: the module's base is looked up **by name at run time** (modules are not always loaded at their preferred base). The first offset is added to the base; for every further offset the 4-byte (or, for a 64-bit process, 8-byte) pointer at the current address is read, and the next offset is added to it. A pointer of 0 or a failed read ends the chain with an empty cell. The last address is read as `type`.

A handle is `(generation << index_bits) | index`. The table is an array of fixed-size records; the record for `index` must hold the same generation (otherwise the handle is stale) and a non-null object pointer, and `index` must be below the record count. A handle of 0, an index out of range, a generation mismatch, an empty table or an unreadable table all end the chain with an empty cell. The table is read without the game's lock, so one row can occasionally mix two frames: such a row comes out empty or stale, never as a crash.

| Key | Meaning |
|---|---|
| `module` | a module file name (`Engine.dll`, `Sam2.exe`), not a path; case-insensitive |
| `offsets` | 1 to 16 steps; integers or strings like `"0x10"`; **never negative**, at most 32 bits |
| `type` | `f32` `f64` `i8` `u8` `i16` `u16` `i32` `u32` `i64` `u64` (little-endian) |
| `scale` | optional: the value is multiplied by this and written as a float (turn a tick count into seconds) |
| `repeat` | optional: make `count` columns `name[0..count-1]`; element `i` adds `i * stride` to step `at` of the chain (arrays of pointers or of structs) |
| `pointer_size` (top level) | optional safety check: 4 or 8; the tool refuses to run if the process is not that |
| step `{"offset": N, "handle": true}` | in `offsets` (never the first step): the value at the current address is a 32-bit **handle**, not a pointer; it is resolved through `handle_table` and `N` is added to the object it names |
| `handle_table` (top level) | needed when a handle step is used: `module`, `table` and `count` (offsets from the module base of the variables holding the table's address and the number of records), `record_size`, `generation_at`, `object_at` (inside a record) and optionally `index_bits` (default 24) |

The tool refuses a bad map with a message that names the field and the rule (unknown type, negative offset, missing module, duplicate or reserved name, unknown key, chain too long).

The game is a **32-bit process** (`IsWow64Process` is used to find out, not assumed), so pointers are 4 bytes and a 64-bit Python reads it through `ReadProcessMemory`.

## Rules for running the original

- It is a normal Windows program on your desktop: it opens a window and plays sound. Start it yourself; the harness does not.
- **Never send synthetic input** to it (no `SendInput`, no window messages): the harness has no such code. A scenario that needs input is a manual step performed by a person.
- The game writes settings, profiles and logs when it runs. Back up its user-data folders before a session and restore them afterwards if you want your own profile unchanged.
- Run one copy at a time, keep runs short, and stop the game yourself when done.

## Running the original under the harness

### What the game needs (checked on 2026-11-26, install build 2.224.00:747079)

- **The Steam client must be running and logged in.** Started without it, `Sam2.exe` crashes about one second after the line `Steam initialize` in its log (three starts, same result: a null-pointer access in the code that clears the Steam statistics at start-up; the crash report names that function). The harness cannot work around this and does not try: it never starts or touches Steam. Start Steam yourself first. Note that a running game talks to your Steam account (the log shows `Stored Achievement for Steam` in a normal session), so a measurement session is a normal play session from Steam's point of view.
- **Which mod runs** is chosen by `Content/CurrentMod.txt` (then `Content/DefaultMod.txt`); `+mod` on the command line is read too late. For measurements of the original game the file must say `CURRENTMOD = SeriousSam2.mod ;` (or be absent). If you play another mod normally, back the file up and put it back afterwards.

### The command line

The command line is read after the configuration scripts and `Sam2.ini`, so it wins. Its only syntax is `+name value`: every argument that starts with `+` is a console variable (`docs/re/console-variables.md`) and the **next argument** is its value (two separate arguments, no `=`); other arguments are ignored; there is no `-option` form. The game log echoes the line as `Command: ...`. Verified on the running game: the log echoed the line and the window came up at the requested size (`Window: 800 x 450`).

| Variable | Use |
|---|---|
| `sam_bBootSequence 0`, `sam_bSkipMovies 1` | skip the intro movies |
| `sam_bStartDemoPlayback 1`, `sam_strDemoFile Content/SeriousSam2/Demos/Demo_0001.dem` | play a recorded demo: **the simulation replays by itself, no input needed**; the install ships `Demo_0001` to `Demo_0003`; every demo replays the same inputs, which makes a demo the most repeatable "fixed scenario" there is; `sam_bLoopDemo` repeats it |
| `sam_strLevelToStart <world path>`, `sam_bStartMainMenu 0` | start a single-player level directly (needs an existing player profile; whether the last used profile is picked without input was not checked) |
| `gfx_bFullScreen 0`, `gfx_bSimulatedFullScreen 0`, `gfx_pixWinWidth 800`, `gfx_pixWinHeight 450`, `gfx_pixResWidth 800`, `gfx_pixResHeight 450` | a small window; set both pairs |

The grammar is read from the code (medium to high confidence); only the sizes and the echo were observed.

### What the game writes

Everything the game writes is **inside the install folder** (no registry, `AppData` or `Documents` use was found for settings): the logs `Sam2.log` and `All.log` (and `<Mod>.log`, `<Mod>_All.log` for another mod) in the root, `Temp/ConsoleHistory.txt`, `Content/SeriousSam2/Sam2.ini` (**every `+` override is saved back into it at a normal shutdown**), `Content/CurrentMod.txt`, `Content/PlayerProfiles/<id>/` (profile and `SavedGames/`), `Content/SeriousSam2/Demos/` (recorded demos), and crash reports in `Bin/`. If you want your settings unchanged, copy those paths before a session and put them back after it (a small script that lists every file with size and time before and after, copies the non-`.gro` files, and restores what changed is about 50 lines of Python; it is not part of the repository because it is specific to one machine).

### Which fields the harness reads

Field names are free (they are yours), but these are the ones the first measurement tasks need; every one is a number the engine keeps and that the specs already describe. **None of them has been read on a running game yet** (see "Status" below).

| Field name used here | What it is | Type / unit |
|---|---|---|
| `sim.now_raw`, `sim.now_s` | the simulation clock `Now` (`docs/re/main-loop.md`: a 64-bit count of 2^-32 s; `sim.now_s` is the same with `scale` 2^-32) | `i64` |
| `sim.last_step` | length of the last simulation step in seconds | `f32` |
| `player.health`, `player.health_max`, `player.armour`, `player.alive` | the local player puppet's integer health, its maximum, armour and the alive flag | `i32` |
| `player.pos.x/y/z` | the puppet's absolute position (world axes) | `f32` |
| `player.vel.x/y/z` | the puppet's body velocity | `f32` |
| `player.quat[0..3]` | the placement's orientation quaternion (component order unverified) | `f32` |

To build the map you need your own reverse-engineering notes: the simulation object's `Now` member, and the chain from the current world through the world-info and game-info entities and the player brain to the player puppet (`docs/re/players-session-classes.md`, `puppet-entity-classes.md`), and from the puppet through its mechanism and placement to the position. Object links inside the game are 32-bit **handles**, not pointers, which is what the `handle_table` and the `{"offset": ..., "handle": true}` steps of the map are for.

### Status of the first live check

What has been checked against the running original (2026-11-26): the harness attaches to `Sam2.exe`, finds that it is a 32-bit process, lists its 100 or so modules and reads their run-time bases (the engine module really is relocated: its base differs between starts and from the preferred base, so offsets must be module-relative); the command line grammar as far as stated above. What could **not** be checked: every field. Without a running Steam client the game dies before any world exists, so no tick, position, velocity or health was read. These are the first things to check once Steam is up (the expected values are a clock that advances by about 4.29e9 raw counts per second of game time and stands still in menus, a starting health of 100 in a fresh level, positions inside the level's bounds, and a position at level start equal to the start marker of the world file).

**Update, 2026-11-28 (T01-038):** with Steam started and logged in and a demo played back (`+sam_bStartDemoPlayback 1`, see "Steps" above), the harness read real numbers from the running original for the first time:

- `sim.now_raw` advances steadily while a level runs and stands still at none observed (menus were not sampled this run): over a 500-sample, 25-second capture at 20 Hz it advanced by 107,152,798,422 raw counts in 24.950317 s of wall time, **4.2946×10⁹ counts/s**, against the predicted 2³² = 4.29497×10⁹ (`docs/re/main-loop.md` §1) — within 0.01%, the gap being ordinary wall-clock/read-latency noise, not a wrong unit. **Confidence: high** (one long capture, consistent throughout, matches the static prediction).
- The target map's chain to the player puppet (`sim`'s world pointer → a handle → the world-info entity → a handle → the game-info entity → a handle → the player brain → a handle → the player puppet) had **one wrong hop**: the local, git-ignored map's second-to-last handle offset (from the game-info entity) needed the third hop's offset corrected (it pointed at the wrong member). Every other hop and the trailing field offsets were already right. `player.health`, `player.health_max`, `player.armour` and `player.alive` were then read and checked against the HUD by eye, repeatedly, over several minutes of demo playback and several level/weapon changes (values seen: health 125, 135, 150, 165, 200 and a momentary 2; armour 0; health_max 200; alive 1) — every reading matched the HUD at the moment it was taken. **Confidence: high** (repeated, consistent, cross-checked against an independent source — the rendered HUD — not just internally consistent).
- `player.pos.*`, `player.vel.*` and `player.quat[]` are **still not mapped**: the map's existing guess (a placement object reached through a handle at the puppet's `+0x114`-equivalent member, then a further offset for x/y/z) resolves to a *different* object almost every time it is read, which reads as plausible position/orientation floats on some samples and obvious garbage (values like 10¹⁸) a fraction of a second later with the player not moving — i.e. that handle slot is reused very rapidly for something short-lived (a pooled temporary, not a stable placement reference), unlike the four-hop chain above which is stable for as long as the puppet exists. Finding the real placement/velocity needs more work than this run's time box (see the follow-up task appended to `docs/roadmap/T01-reverse-engineering.md`). **No position, velocity or orientation number is reported from this run.**
- Practical notes for the next person: `Content/CurrentMod.txt` has to say `SeriousSam2.mod` (there is no such file in this install; an absent/non-matching file falls back to the original, which is what running under the original game's own build id needs) — this install's default was the `Sam2Renovation` mod and had to be switched back after; starting `Sam2.exe` with a demo on the command line, once its settings have been saved back into `Sam2.ini` by a normal shutdown, **persists**: a later start with no arguments at all replayed the same demo automatically, including restarting the whole process partway through this run (`Sam2.exe`'s PID changed) — always check the actual command line (`Get-CimInstance Win32_Process` on Windows) before assuming a capture reflects the scenario you asked for.

### Steps for the first live check

1. Start Steam and log in. Make sure `Content/CurrentMod.txt` selects the original game (see above).
2. In a terminal, in the install folder, start a demo (this opens a game window; do not press anything in it):

   ```
   Bin\Sam2.exe +sam_bBootSequence 0 +sam_bSkipMovies 1 +sam_bStartDemoPlayback 1 +sam_strDemoFile Content/SeriousSam2/Demos/Demo_0001.dem +gfx_bFullScreen 0 +gfx_bSimulatedFullScreen 0 +gfx_pixWinWidth 800 +gfx_pixWinHeight 450 +gfx_pixResWidth 800 +gfx_pixResHeight 450
   ```

3. In a second terminal in the repository, once the level is visible:

   ```
   python -I tools/oracle/oracle.py --dump-fields
   python -I tools/oracle/oracle.py --scenario demo1 --out C:\temp\demo1.csv --duration 60 --rate 30
   ```

4. Look at the CSV: `sim.now_s` should grow steadily, the player's health should be an integer such as 100, the position should change smoothly. A column that is always empty means the map for that field is wrong or the object does not exist yet; report which.


## Tests

```
python -I -m unittest discover -s tools/oracle -v                      # fake-process tests; no game, no Windows needed
set SS2_ORACLE_SELF_TEST=1 && python -I -m unittest discover -s tools/oracle -v   # plus the real-process tests (Windows)
python -I tools/oracle/mutate.py                                       # hand-written mutation check of the core logic
```

The unit tests use a **fake process** (`fakes.py`: bytes served from a dict, a clock the test moves) and cover pointer chains, null and wild pointers, 4- and 8-byte pointers, every value type, number formatting, module lookup (ASLR-style relocation, late-loading modules), CSV shape, jitter and missed-slot accounting, process exit mid-capture, config validation, the build id and the command line. The tests that run only with `SS2_ORACLE_SELF_TEST=1` attach to **the Python test process itself** and read known buffers through the real `ReadProcessMemory` path (module list, pointer widths, a pointer chain into memory that the test owns, a full capture, a child process that exits), so the Windows code is verified without the game.

**Mutation check.** `tools/oracle/mutate.py` applies 64 hand-written, exact-text edits to the core logic (pointer-chain arithmetic, null and width checks, handle resolution, type decoding, number formatting, config validation, the jitter and missed-slot accounting, process-exit handling, the build id and the Windows backend), runs the whole suite after each, and restores the file. Result of the last run: **64 of 64 killed**. The first run had 8 survivors out of 51; five were missing tests (added), two were dead or redundant code (a duplicate-column check that cannot fire, an `abs()` of a value that cannot be negative: both removed) and one was a mutant that weakened a number format that the tests did not pin (now pinned). The mutants are my own selection, not an exhaustive operator set, so 64 of 64 is evidence about those places only. The Windows-backend mutants are killed only by the `SS2_ORACLE_SELF_TEST=1` tests.

## What this does not do

- It cannot start a level, load a save or press a key; those are manual steps (above and in the scenario list of T01-039).
- It does not stop or slow the game: it samples. It cannot see a value that changes and changes back between two samples; for a quantity that is only defined at the instant of an event (a shot's launch velocity) read the game's own time column and the quantity in the same row and look at what changed between rows.
- A 64-bit or different build of the game needs a different map (`install_build` in the sidecar says which build a capture came from).
