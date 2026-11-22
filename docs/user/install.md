# Install and point SS2RS at your game data

> **Status: pre-alpha.** SS2RS does not play the game yet. Anything marked **planned** is a roadmap
> task that does not exist in the code today.

## What SS2RS is

SS2RS is an open-source engine, written in Rust, that is meant to play *Serious Sam 2* using the
data files of **your own copy** of the game. Its behaviour is re-created from observation and written
specifications under a [clean-room process](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md).

It is an **unofficial** project. It is not affiliated with, endorsed by or sponsored by Croteam or
Devolver Digital. "Serious Sam" and "Serious Engine" are trademarks of their owners
([NOTICE](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/NOTICE)).

## What SS2RS is not

- **Not a copy of the game.** It ships no game content: no archives, models, textures, sounds,
  movies, levels or game executables. The README and the NOTICE file say so.
- **Not a way to get the game.** You need your own legally obtained copy of Serious Sam 2. The
  project does not provide one.
- **Not playable yet.** Co-op, the campaign and the first playable level are planned in the
  [roadmap](../roadmap/README.md) (milestones M3 to M7).

## Supported game versions

The only install the project documents is the **retail PC copy patched to 1.0**, as read by the
project's tools. The [install inventory](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/inventory.md)
lists 14 `.gro` archives:

- three base archives, `All_PC_01.gro`, `All_PC_02.gro` and `All_PC_03.gro`;
- two movie archives, `Movies_PC_01.gro` and `Movies_PC_02.gro`;
- nine patch archives, named `Patch_02_*.gro`, the newest being `Patch_02_100_800000.gro`.

No document says that other editions, storefront copies or other patch levels work. Those are not
verified. The [crash report template](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/.github/ISSUE_TEMPLATE/crash.md)
asks for the store, version and patch level so the project can collect that information.

A checker for the folder exists: `ss2-doctor <folder>` (T11-013) lists the archives, tells you which are missing or damaged
and reports the patch level from the newest patch file name. It is not in the download archives yet; from a source checkout run
`cargo run -p ss2-vfs --bin ss2-doctor -- <folder>`. The first-run wizard (T10-019) is still planned.

## Platforms

- **PC first:** Windows, then Linux and macOS ([README](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/README.md)).
- **Windows version:** Windows 10 or later (64-bit). The program declares this and per-monitor high-DPI support in its
  manifest, so on a high-resolution screen with display scaling the window is sharp, not stretched.
- **Android and other platforms:** planned for later phases. The Android port starts only after
  milestone M8 ([roadmap section 5](../roadmap/README.md#5-milestones-and-exit-criteria)).

## Where to get a build

**There is no release yet.** The project has not published a build. The release process is written
down in [RELEASING.md](../RELEASING.md) and the release checklist in [PC_1_0_CHECKLIST.md](../PC_1_0_CHECKLIST.md).

Planned, not yet available:

- archives on **GitHub Releases** for Windows (zip), Linux and macOS, with a `SHA256SUMS` file
  (tasks T12-008 and T12-009);
- each archive holds `ss2-app`, `ss2-server`, `LICENSE`, `NOTICE` and `README.md`, and no game
  content;
- builds are **unsigned** until code signing exists, so Windows or macOS may show a warning.

Until a release exists, the only way to run the code is to build it from source. The tools and the
pinned Rust version are listed in [DEV_TOOLS.md](../DEV_TOOLS.md).

## What a valid install folder looks like

SS2RS reads the `.gro` archives **directly inside** the folder you give it. The folder must contain
the archives themselves, as the game installs them:

- `All_PC_01.gro`, `All_PC_02.gro`, `All_PC_03.gro` (base data);
- `Movies_PC_01.gro`, `Movies_PC_02.gro` (movies);
- `Patch_02_*.gro` (patches; keep all of them).

Things to check:

- **Point at the folder that contains the `.gro` files**, not at the `Serious Sam 2` folder above it.
  A parent folder without archives directly inside is rejected with a message that asks for the folder
  containing `All_PC_01.gro`.
- **Sub-folders are ignored.** The `Content` folder and mod folders inside the install are not read.
- **Do not add other `.gro` files** to this folder. Every `.gro` file directly inside is loaded.
- **Do not rename, re-zip or edit the archives.** Archives are loaded base first, then patches in
  numeric order, and a later archive replaces an earlier file with the same path. Renaming a patch can
  change which copy wins.

## Pointing SS2RS at the data (today)

Set the environment variable **`SS2_DATA_DIR`** to the folder with the `.gro` files. The tools and the
data-gated tests read this variable. You can also pass the folder to `gro-tool` with `--data`.

Windows (PowerShell, for the current session):

```
$env:SS2_DATA_DIR = "C:\path\to\folder-with-All_PC_01.gro"
```

Linux and macOS:

```
export SS2_DATA_DIR=/path/to/folder-with-All_PC_01.gro
```

Then, from a source checkout, check that the archives are read:

```
cargo run -p gro-tool -- stats
```

Today only two things read the data: `gro-tool`, an inspection tool, and the data-gated tests. The
placeholder window (`ss2-app`) uses no game data.

**Planned:** a first-run wizard in the window that finds the install, checks it and remembers the
choice (T10-019). Until then, `SS2_DATA_DIR` is the only way to point the project at the data.

## Checking that the program starts

Without game data, the placeholder window can still be checked:

```
cargo run -p ss2-app -- --smoke-frames 5
```

This opens the window, renders five frames and exits. If it fails, see
[troubleshooting](troubleshooting.md).
