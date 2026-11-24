# Troubleshooting: every `ss2-doctor --mod` message

`ss2-doctor` never stops with an error of its own and never crashes on a damaged mod: each problem becomes a **finding**.
This page lists all 37 of them, with the cause and the fix. The design and the limits of the checker are in
[Mod validation](../MOD_VALIDATION.md); how to run it is in the [Mod API reference](api.md#the-ss2-doctor-command).

## How to read a report

```
Mod hellomod ("Hello Mod"): 1 layer, 3 files, 0 override base files
  WARNING MOD031 content/hellomod.nfo: THUMBNAIL = "..." (thumbnail) exists neither in the mod nor in the install
          -> Add the file to the mod, or point the key at a file of the base game (or another mod you require).
```

The first line names the mod (the id, and its `NAME` in brackets when it has one), how many layers (archives, or one for a
loose folder), how many files and how many replace a game file. Each finding has a severity (`ERROR`, `WARNING` or `INFO`),
the code, the file it is about (always lower-case, as the game compares names) and what is wrong; the line with `->` says
what to do. The last line counts them. Paths are shown lower-case, and everything is plain ASCII: an odd character in a file
name appears as `\u{e9}`.

The exit code is `0` unless there is at least one **error**, then `1` (`2` is only for a wrong command line). Findings are
sorted most serious first, then by code and path; a long list of files becomes one finding with the count and the first
three names.

## The table

| Code | Severity | Why you see it | What to do |
|---|---|---|---|
| MOD001 | error | There is no `Content/<Mod>.mod`, neither in the mod's files nor loose in the install. | Create it: `MODULES = Sam2Game ;` |
| MOD002 | error | The `.mod` file cannot be read or understood: no `MODULES`, no `=`, no closing `;`, an empty list, or it is over 1 MiB. The message quotes the parser's reason. | Write `MODULES = <name> <name> ... ;` ([syntax](start-here.md#contentmodmod)) |
| MOD003 | warning | There is no `Content/<Mod>.nfo`; the mod list could only show the id. | Add it with `NAME`, `AUTHOR`, `THUMBNAIL` |
| MOD004 | warning | An `.nfo` of the mod (the descriptor or a level, episode...) cannot be parsed: an assignment without `=` or a quote that is not closed. The message gives the file and the parser's reason. | Fix the line ([example](#a-quote-that-is-never-closed-mod004)) |
| MOD005 | warning | The descriptor has no `NAME`, or it is blank; the mod list shows the id. | Add `NAME = "My Mod" ;` |
| MOD006 | note | The descriptor has no `AUTHOR`. | Add `AUTHOR = "Your Name" ;` |
| MOD007 | warning | The mod's archives carry the `.mod` or `.nfo` of another mod id. | Keep only this mod's two descriptors in its archives |
| MOD008 | warning | The `.mod` or `.nfo` inside the archive differs from the loose copy in the install's `Content/`. | Make the two copies identical, or remove one |
| MOD009 | warning | The mod has no files at all in any layer. | Check that the archives are in `Content/<Mod>/` and are not empty |
| MOD020 | warning | `REQUIRES_PATCH` is not a version. | Write `"2.100"` or `"2.100.800000"` ([example](#a-patch-the-install-does-not-have-mod020-mod021)) |
| MOD021 | error | The install's newest patch is older than `REQUIRES_PATCH`, or the install has no patch. | Install the newer `Patch_02_*.gro` files, or lower the requirement if the mod does not really need it |
| MOD022 | warning | An update archive (a name such as `z_Mod_Update_12_100.gro`) is named for a newer patch than the install has. This is **inferred from the name** (a guess from two shipped names, not a documented rule). | Install that patch, or ask the author |
| MOD023 | warning | The `.mod` names a game module other than `Sam2Game`: native code this engine cannot load ([example](#a-game-module-of-your-own-mod023-mod024)). | For a data-only mod write `Sam2Game`; otherwise only the mod's content is used |
| MOD024 | warning | That module's `Bin/<Name>.dll` is not in the install (only judged when the install has a `Bin/` folder). | Copy the DLL, or name `Sam2Game` |
| MOD030 | error | A `MODEL` names a `.mdl` that is neither in the mod nor in the game ([example](#a-file-that-does-not-exist-mod030-mod031)). | Add the model, or point `MODEL` at a game model |
| MOD031 | warning | A `THUMBNAIL`, `LOADING_SCREEN`, `LOADING_MODEL` or movie key names a file that exists nowhere. | Add the file, or fix the path |
| MOD032 | error | A reference has `..`, an absolute path (`/x`, `\x`), a `:` (drive letter, URL) or control characters ([example](#a-path-that-climbs-out-mod032)). | Use a relative path below `Content/`: `Content/<Mod>/Models/x.mdl` |
| MOD033 | warning | A reference mixes `/` and `\`. | Use `/` only |
| MOD034 | warning | A reference key has an empty value. | Give the path, or remove the key |
| MOD035 | warning | A `LEVEL` `.nfo` has no `.wld` of the same name next to it ([example](#a-level-without-its-world-mod035)). | Add the `.wld`, or remove the `.nfo` |
| MOD036 | warning | A reference points to a file of the wrong type, for example a `.png` as `THUMBNAIL` ([example](#the-wrong-type-of-file-mod036)). | Use the type the key needs: `.mdl`, `.tex` or `.ogv` |
| MOD038 | error | Entries of an archive have names that are not valid paths (`..`, a drive letter, control characters); they are **not loaded**. | Repack with plain relative names |
| MOD039 | warning | Entries of one archive repeat a path (same name twice, or only the case or the slashes differ); the last one wins. | Keep one entry per path |
| MOD040 | warning | File names that Windows cannot hold: `<>"\|?*`, a trailing space or dot, or a device name such as `CON`, `NUL`, `COM1`. | Rename them |
| MOD041 | note | Files outside `Content/` (a README, for example), where the game does not look ([example](#files-outside-content-mod041)). | Move them below `Content/<Mod>/`, or ignore the note |
| MOD042 | note | Archives that hold only a file `dummy.txt` are placeholders that mark an optional pack; they are skipped. | Nothing |
| MOD043 | warning | A descriptor or `.nfo` is larger than 1 MiB and is not read. | It is not a descriptor: check the file |
| MOD044 | warning | A file or folder of the mod cannot be read or inspected. | Check the permissions and that the archive is intact |
| MOD045 | error | A loose folder holds two names that differ only by case. The game sees one file; a case-sensitive system sees two. | Rename one |
| MOD046 | warning | Native code or foreign archives inside the mod: `.dll`, `.exe`, `.module`, `.so`, `.gro` ([example](#native-code-inside-the-mod-mod046)). | Remove them; see the [modding policy](../MODDING_POLICY.md) |
| MOD050 | note | How many of the mod's files replace game files, and the biggest types ([example](#a-file-that-replaces-a-game-file-mod050-mod051)). | Nothing: this is what the mod changes |
| MOD051 | warning | The mod replaces files that change how the game *plays*: `.wld`, `.ep`, `.rsc`, or the game's own `SeriousSam2.mod`/`.nfo` or the mod-selection files `DefaultMod.txt`/`CurrentMod.txt` (one finding per kind). | Intended? Every player needs the same files |
| MOD060 | warning | Two mods of the load order provide the same files and at least one differs (or could not be compared). Only a hypothetical: how the game combines mods is not decided. | Enable one mod at a time, or order them |
| MOD061 | note | Two mods provide the same files with identical content. | One copy is redundant |
| MOD090 | error | The install itself cannot be read (for example no `.gro` there). | Run `ss2-doctor` on the folder without `--mod` to see what is wrong |
| MOD091 | error | The named mod is not in the install (`Content/<Mod>/` has no archive folder), or a `--mod-dir` folder cannot be read. | Check the spelling and the folder |
| MOD092 | error | An archive of the mod cannot be opened as a zip file; the rest of the mod is still checked. | Copy the file again from the download |

## Worked examples

Each report below is the tool's real output for the mod folder of the [tutorial](tutorial.md) changed in the way named. They run
against an invented install whose newest patch is `Patch_02_100_800000.gro`.

### A file that does not exist (MOD030, MOD031)

Delete the thumbnail, or name a model that is nowhere: the two reports are in [the tutorial](tutorial.md#4-break-it-on-purpose).
A missing model is an error (exit 1), a missing picture or movie a warning.

### A quote that is never closed (MOD004)

`NAME = "Hello Mod ;` has an opening quote and no closing one; the value runs to the **next** quote, over the line end, so the
parser then finds `Me"` where a key should be. The doctor warns (exit `0`) and, because no `NAME` could be read, shows only the id
in the mod line:

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod: 1 layer, 4 files, 0 override base files
  WARNING MOD004 content/hellomod.nfo is not a valid .nfo file (invalid .nfo file: expected `=` after `Me"`)
          -> Each line is `KEY = "value" ;`; check for a missing `=` or a quote that is not closed.

No errors, 1 warning, 0 notes.
```

Strings have no escapes: a value can never contain a `"`.

### The `.mod` file (MOD001, MOD002)

A closing `;` is missing (`MODULES = Sam2Game` alone):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  ERROR   MOD002 content/hellomod.mod is not a valid .mod file (invalid .mod file: the module list is not closed by `;`)
          -> The format is `MODULES = <name> <name> ... ;` (optional `<TOOLS>` prefix on a name).

1 error, 0 warnings, 0 notes: fix the errors first.
```

No `.mod` file at all:

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 3 files, 0 override base files
  ERROR   MOD001 the mod has no content/hellomod.mod
          -> Add Content/<Mod>.mod with `MODULES = Sam2Game ;` (inside the mod's archive or loose next to it).

1 error, 0 warnings, 0 notes: fix the errors first.
```

### No `.nfo` (MOD003)

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod: 1 layer, 3 files, 0 override base files
  WARNING MOD003 the mod has no content/hellomod.nfo, so the mod list shows only its id
          -> Add Content/<Mod>.nfo with NAME, AUTHOR and THUMBNAIL.

No errors, 1 warning, 0 notes.
```

### A patch the install does not have (MOD020, MOD021)

`REQUIRES_PATCH = "2.200" ;` on an install whose newest patch is 2.100 is an error (exit 1):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  ERROR   MOD021 content/hellomod.nfo: the mod needs patch level 2.200, the install has 2.100 (build 800000)
          -> Install the newer Patch_02_*.gro files, or ask the mod author for a version that works with the install.

1 error, 0 warnings, 0 notes: fix the errors first.
```

`REQUIRES_PATCH = "latest" ;` is not a version (exit 0):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  WARNING MOD020 content/hellomod.nfo: REQUIRES_PATCH = "latest" is not a version
          -> Write it like the patch archives: "2.100" or "2.100.800000" (major.minor[.build]).

No errors, 1 warning, 0 notes.
```

### A path that climbs out (MOD032)

`THUMBNAIL = "../Checker.tex" ;` (exit 1):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  ERROR   MOD032 content/hellomod.nfo: THUMBNAIL = "../Checker.tex" is not allowed: it climbs out of the game folder with `..`
          -> Use a relative path below Content/, written like Content/<Mod>/Models/x.mdl.

1 error, 0 warnings, 0 notes: fix the errors first.
```

### A level without its world (MOD035)

A `LEVEL` description `Content/HelloMod/Levels/HelloLevel.nfo` and no `HelloLevel.wld` beside it:

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 5 files, 0 override base files
  WARNING MOD035 the level content/hellomod/levels/hellolevel.nfo has no content/hellomod/levels/hellolevel.wld next to it
          -> A level is a .nfo and a .wld of the same name in one folder; add the .wld or remove the .nfo.

No errors, 1 warning, 0 notes.
```

### The wrong type of file (MOD036)

`THUMBNAIL = "Content/HelloMod/Textures/Checker.png"` with that file present:

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 5 files, 0 override base files
  WARNING MOD036 content/hellomod.nfo: THUMBNAIL = "Content/HelloMod/Textures/Checker.png" should be a .tex file
          -> Check the path: the game expects this type of file here.

No errors, 1 warning, 0 notes.
```

The engine reads `.tex`; there is no converter from PNG yet ([formats](formats.md#tex-textures)).

### A game module of your own (MOD023, MOD024)

A `.mod` file with `MODULES = HelloModGame ;` (exit 0):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  WARNING MOD023 content/hellomod.mod: the mod loads its own game module `HelloModGame`, native code that this engine cannot load; only its content can be used
          -> Its gameplay changes will not run. A data-only mod names `Sam2Game` (docs/MODDING_POLICY.md).

No errors, 1 warning, 0 notes.
```

### Native code inside the mod (MOD046)

A `Content/HelloMod/Helper.dll` in the mod (exit 0; a mod must never carry one, the game's or its own):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 5 files, 0 override base files
  WARNING MOD046 files of type .dll: native code or foreign archives that cannot be loaded and may not be redistributed (1): content/hellomod/helper.dll
          -> Remove them; see docs/MODDING_POLICY.md (what a mod may contain).

No errors, 1 warning, 0 notes.
```

### Files outside `Content/` (MOD041)

A `README.txt` at the top of the mod folder (exit 0, a note only):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 5 files, 0 override base files
  INFO    MOD041 files outside Content/, where the game's loaders do not look (1): readme.txt
          -> Move them below Content/<Mod>/ (a mod's own files) or Content/SeriousSam2/ (overrides).

No problems found (1 note).
```

### A file that replaces a game file (MOD050, MOD051)

A file at the path of a game `.ep` (the invented install has one at `Content/SeriousSam2/Databases/Sample.ep`):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 5 files, 1 override base files
  WARNING MOD051 files of type .ep (entity parameters) that replace the base game's: the mod changes how the game plays, not only how it looks (1): content/serioussam2/databases/sample.ep
          -> Intended? In a game with several players every player needs the same files.
  INFO    MOD050 1 of 5 files replace base game files, 4 are new (replaced by type: .ep 1)
          -> Nothing to fix: this is what the mod changes. See the critical overrides below, if any.

No errors, 1 warning, 1 note.
```

On the real mods of the install the doctor reports exactly this kind of finding: Renovation replaces 31 levels (`MOD051`),
`InSamnity! 2` replaces 7 `.ep` and 2 `.wld` files (`MOD051`) and has its own game module (`MOD023`); both together give one
`MOD060`. The doctor finds no error in either.

## Still stuck

- A message that is not here: the codes are stable, so search for it in [Mod validation](../MOD_VALIDATION.md); if it is
  missing there too, that is a bug of the documentation.
- A mod that passes but does nothing in the game: the game cannot load mods yet ([start here](start-here.md#what-is-supported-today-and-what-is-not)).
- Something you think the doctor should catch (references inside `.ep` or `.mdl`, for instance): it does not look yet
  (T13-009b).
