# Mod interface: `.mod`, `.nfo`, overlays, game module replacement (T01-040)

- **Roadmap task:** T01-040
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the file formats and layout (every mod file read) and for the overlay facts (the archives were compared entry by entry); **medium** for how the launcher and the engine apply them (inferred from names and from the install, not from run-time observation); **open** for the exact load-order and conflict rules.

Prerequisites: `misc-types.md` (`.nfo`), `inventory.md`, `player-rules.md`. Feeds T13-001 (mod loading semantics).

## Summary

A **mod** is a *module description* plus *content archives*, selected at start-up. It is made of:

1. `Content/<Mod>.mod`: names the **game module(s)** (DLLs) to load;
2. `Content/<Mod>.nfo`: the display name, author and thumbnail for the mod list;
3. `Content/<Mod>/*.gro`: **overlay archives**: ordinary archives whose entries are added to, or replace, the base content;
4. optionally a **replacement game module** in `Bin/` (`InSamnity2Game.dll` for InSamnity! 2) and mod-specific data files.

The base game itself is described the same way (`Content/SeriousSam2.mod`/`.nfo`), so "the game" is just the default mod. Two mods ship in the install: **InSamnity! 2** (a content and game-logic mod with its own game module) and **Renovation** (a high-detail content replacement using the stock game module).

## Evidence

The `.mod` and `.nfo` files were read; all `.gro` of `Content/<Mod>/` were opened and compared entry by entry (case-insensitive paths) with the base archives (10,933 distinct entries); the text files were read. The file names `DefaultMod.txt` and `.mod` also occur as strings in `Core.dll` and `Sam2.exe`. Nothing else from binaries.

## `.mod` files

```
MODULES = <CRLF>
  Sam2Game<CRLF>
  <TOOLS>Sam2GameGui<CRLF>
;<CRLF>
```

| File | Modules |
|---|---|
| `SeriousSam2.mod` (49 bytes, in `Patch_02_068`) | `Sam2Game`, `<TOOLS>Sam2GameGui` |
| `Sam2Renovation.mod` (49 bytes) | `Sam2Game`, `<TOOLS>Sam2GameGui` (the same as the base game: Renovation changes content only) |
| `InSamnity2.mod` (55 bytes) | `InSamnity2Game`, `<TOOLS>Sam2GameGui` |

Grammar (inferred from the three files): `MODULES =` followed by a whitespace-separated list of module names and a closing `;`. A name is a DLL in `Bin/` without extension (`Sam2Game` is `Bin/Sam2Game.dll`; the editor's "shader module" and "game module" names in data files are `Bin/<Name>.module`: `mtr.md`). The prefix **`<TOOLS>`** marks a module that is loaded only by the tools (the editor), not by the game (`Sam2GameGui` is a separate small DLL: `Sam2GameGui.dll`, 8 KB). A reimplementation has no DLLs; it needs only the **mod's identity** and the list of overlays.

## `.nfo` files

`NAME = "…" ; AUTHOR = "…" ; THUMBNAIL = "Content/<Mod>/<…>.tex" ;` (the key set of `misc-types.md`): `Serious Sam 2` by Croteam, `InSamnity! 2` and `Renovation` by their authors. The thumbnail is a `.tex` (Renovation ships it loose: `Content/Sam2Renovation/Sam2Renovation_thumbnail.tex`).

## Folder layout

```
Content/SeriousSam2.mod, .nfo                 (inside Patch_02_068 and later)
Content/<Mod>.mod, <Mod>.nfo                  (loose files in the install, and copies inside the mod's .gro)
Content/<Mod>/<Mod>.gro                       the main overlay archive
Content/<Mod>/z_<Mod>Update_NN.gro            update and option archives (InSamnity! 2 has fifteen: eleven `z_InSamnity2Update_*` and four others)
Content/<Mod>/*.txt, Updater/CurrentVersion.DAT   mod data and updater state
Bin/<Mod>Game.dll                             the replacement game module (InSamnity! 2 only)
```

## Overlay archives

Overlays are **ordinary `.gro` archives** (zip files, `Content/` at the root). Measured against the base archives:

| Archive | Entries | Replace a base entry | New |
|---|---|---|---|
| `Sam2Renovation.gro` (459 MB) | 3,194 | **1,963** | 1,231 (527 of them are parameter databases under `Content/Sam2Renovation/Databases`) |
| `InSamnity2.gro` (187 MB) | 3,152 | 399 | 2,753 (own folder `Content/InSamnity2/…` with 925 models, plus 613 models and 443 effects under `Content/SeriousSam2/…`) |
| `z_InSamnity2Update_08` / `_09` / `_10` / `_11` / `_12_100` | 25 / 3 / 1 / 15 / 29 | 0 | all (menus, parameters, macros, a font, textures under `Content/InSamnity2/…`) |
| `z_InSamnity2Update_02_090`, `_07` | 3, 3 | 0 | menus and databases |
| `InSamnity2Players`, `z_InSamnity2Effects`, `…Update_00/_01/_02/_03`, `z_InSamnity2_Greendale`, `z_InSamnity2_ShieldGen` | 1 each | 0 | a single `dummy.txt`: empty **placeholder** archives (zero payload) that mark that an optional pack is installed |

Facts that matter for a loader:

- **Replacement is by path, case-insensitively**: a mod entry with the same path as a base entry replaces it (1,963 of Renovation's entries); the rest are additions. This is the same rule as the patch archives of the base game (`ctsemeta-container.md` and the archive reader: later archives override earlier ones).
- **Order**: the engine evidently loads the base archives, then the patches in numeric order, then the mod's archives; the **`z_` prefix on update packs makes them sort last** in a directory listing, so "alphabetical" and "later wins" agree (the actual rule, and whether `Content/<Mod>/` archives sort among the base ones, is not in the data).
- A mod's own data lives in `Content/<Mod>/…` (separate namespace), and it overrides base content only where it deliberately repeats a path.
- Paths inside a mod `.gro` begin with `Content/` and the archive also carries the mod's `.mod`/`.nfo` (so the descriptor can be read from the archive as well as from disk).

## Game module replacement and the replace history

`InSamnity2Game.dll` replaces `Sam2Game.dll` as the game module (6.1 MB against 5.6 MB; the same imports: `Core`, `Engine`, `Shaders`, `ProcRender`, `steam_api`, runtime). `InSamnity2/ReplaceHistory.txt` is a **rename and replace log** kept by the mod's author (397 lines, quote-delimited pairs) with two kinds of lines: a **chain of source module renames** (`Content/SeriousSam2/Modules/Entities.scm` → `Sources/Sam2Scripts/Entities.scm` → `Sam2Scripts.scm` → `Bin/Sam2Game.module` → `Bin/InSamnity2Game.module`: the game module was originally a script module, `.scm`, compiled to a `.module`), and **menu screen class renames** (`Menus/S…MenuScreen.rsc` → `Menus/C…MenuScreen.rsc`; the mod renamed its menu classes from the game's `S…` prefix to `C…`). For a reimplementation this only documents that mods may **rename classes and resources**; the log has no run-time role (it is not read by the game).

`InSamnity2/ChapterMaxes.txt` is a mod **configuration file** in a brace syntax: a quoted level folder name, then `Chapter001 = "40";` lines: **the maximum number of simultaneous enemies per chapter**, 35 to 40 in the part read (`07_Dunes`, `10_Metropolis`, …; the editor's "Max evil spawnees" setting of chapter info, `help-digest.md`). The mod's game module reads it; the base game has no such file. `Updater/CurrentVersion.DAT` holds the number `110` (apparently the mod's installed version, for its updater).

## The default mod

The strings `DefaultMod.txt` appear in `Core.dll` and `Sam2.exe` and no such file ships: it is apparently the file the launcher **writes** to remember which mod was last chosen (`Sam2.exe` is a small 28 KB executable that imports `Core.dll` and `Engine.dll`). The mod list is presumably built from the `.nfo` files in `Content/` (their keys, `NAME`, `AUTHOR` and `THUMBNAIL`, are what a list needs).

## What is not in the data

1. The exact **load order** of base archives, patches and mods and how a user's mod selection changes it.
2. How conflicts between two mods are handled (one active mod at a time is assumed; only one `DefaultMod` exists).
3. What happens when a module named in `.mod` is missing.
4. Whether `.mod` can list more than the two module kinds seen (`<TOOLS>` is the only prefix).
5. The mods' game logic (it is in their DLLs); T13 decides what a Rust reimplementation can load (only data mods).

## Reader invariants

1. `.mod`: tokens after `MODULES =` up to `;`; an optional `<TOOLS>` prefix; CRLF or LF.
2. `.nfo`: `misc-types.md`.
3. Treat every `.gro` below `Content/<Mod>/` as an overlay; ignore archives whose only entry is `dummy.txt` (markers).
4. Overlays replace by case-insensitive path; mod-local content is namespaced under `Content/<Mod>/`.

## Test vectors

`SeriousSam2.mod` bytes: `MODULES = \r\n  Sam2Game\r\n  <TOOLS>Sam2GameGui\r\n;\r\n` (49 bytes). `Sam2Renovation.gro`: 3,194 entries of which 1,963 replace base entries. `InSamnity2/` contains 19 files: `InSamnity2.gro`, 15 further archives (8 of them empty placeholders holding only `dummy.txt`), `ChapterMaxes.txt`, `ReplaceHistory.txt` and `Updater/CurrentVersion.DAT`.
