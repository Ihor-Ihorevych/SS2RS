# Mod validation (`ss2-doctor --mod`)

- **Roadmap task:** T13-009 (follow-up T13-009b)
- **Where:** `crates/ss2-vfs/src/modcheck.rs` (the checks), `crates/ss2-vfs/src/bin/ss2-doctor.rs` (the command)
- **For mod authors:** [modder docs](modding/start-here.md): a tutorial with a checked example mod, and the cause and fix of every code ([troubleshooting](modding/troubleshooting.md))

`ss2-doctor` checks an install ([user guide](user/install.md)). With `--mod` it also checks the mods of that
install, or a mod folder you are working on, and tells you what is wrong **before** a player finds out: a
descriptor that is missing or malformed, a version the install does not meet, a file that is named but
exists nowhere, a path that is not allowed, a file that replaces a base file, two mods that both provide
the same file.

The checks never stop with an error of their own and never panic on damaged input: every problem is a
finding. The output is deterministic (sorted, no hash order, no clock), so two runs on the same input give
the same text.

## Usage

```
ss2-doctor [FOLDER] [--mod NAME]... [--mod-dir DIR]... [--all-mods]
```

| Option | Meaning |
|---|---|
| `FOLDER` | the install folder (the one with `All_PC_01.gro`); without it `SS2_DATA_DIR` is used |
| `--mod NAME` | check the mod in `Content/NAME/` of the install. Repeat it for several mods: **the order given is the load order** (later wins), and files that two mods provide are reported |
| `--mod-dir DIR` | check a loose mod folder: `DIR/Content/<Mod>.mod` and the mod's files below `DIR/Content/` (a mod being made, not yet packed into a `.gro`) |
| `--all-mods` | check every folder `Content/<Name>/` of the install that holds a `.gro` archive |

From a source checkout: `cargo run --release -p ss2-vfs --bin ss2-doctor -- <install> --all-mods`.
The install check is printed first, then the mod report. **Exit code:** 0 when there is no error (warnings
and notes do not change it), 1 when there is at least one error, 2 for a wrong command line.

A report looks like this (ASCII only; the first line of a finding is the code, then what is wrong, then `->` what to do):

```
Mod testmod ("Test Mod"): 1 layer, 4 files, 1 override base files
  ERROR   MOD030 content/testmod/p.nfo: MODEL = "Content/TestMod/gone.mdl" (model) exists neither in the mod nor in the install
          -> Add the model to the mod, or point MODEL at a model of the base game.
```

From Rust: `ss2_vfs::check_mods(&ModCheckEnv, &[ModUnderTest])` for layers you built yourself (archives, loose
folders or a `MemoryVfs` in a test) and `ss2_vfs::check_install_mods(folder, &[ModSpec])` for an install.

## What is looked at

A **mod** is the id (the file stem of `Content/<Mod>.mod`) and its **layers**: the `.gro` archives of
`Content/<Mod>/` in mount order (the last wins), or a loose folder. Placeholder archives (only `dummy.txt`) are
not layers. The *base* is the install's archives and patches; the install's loose files (the mod's own
`.mod`/`.nfo` next to the archives, `Bin/`, a loose thumbnail) are also used for descriptors and to resolve references.

**References** are the values of these `.nfo` keys, in every `.nfo` file of the mod and in its descriptor:
`MODEL` (`.mdl`), `LOADING_MODEL` (`.mdl`), `THUMBNAIL` (`.tex`), `LOADING_SCREEN` (`.tex`), `INTRO_MOVIE`,
`OUTRO_MOVIE`, `EPISODE_MOVIE` (`.ogv`). The key set and the file types come from `docs/re/misc-types.md` and from
the files of the install. A reference resolves when the file exists in the mod's layers, in the base or among the install's loose files.
References from the mod's own files into **another mod** do not resolve (one mod is assumed active at a time,
`docs/re/mods.md`).

## The lints

Codes are stable: a lint keeps its code and its severity, and a new lint gets a new code. **Error** = the mod
cannot work as it is. **Warning** = probably not what the author wants. **Info** = a note.

| Code | Severity | What it says | What to do |
|---|---|---|---|
| MOD001 | error | no `Content/<Mod>.mod` in the layers or the install | add `MODULES = Sam2Game ;` |
| MOD002 | error | the `.mod` file cannot be read or parsed, or is far too large | `MODULES = <name> ... ;` |
| MOD003 | warning | no `Content/<Mod>.nfo`; the mod list shows only the id | add NAME, AUTHOR, THUMBNAIL |
| MOD004 | warning | an `.nfo` cannot be read or parsed (missing `=`, unclosed quote); the message gives the file | fix the line |
| MOD005 | warning | the descriptor has no non-empty `NAME` | add NAME |
| MOD006 | info | the descriptor names no `AUTHOR` | add AUTHOR |
| MOD007 | warning | the layers carry the `.mod`/`.nfo` of another mod id | keep only this mod's descriptors |
| MOD008 | warning | the descriptor inside the mod's archive differs from the copy in the install folder | make them equal |
| MOD009 | warning | the mod has no files in any layer | check the archives |
| MOD020 | warning | `REQUIRES_PATCH` is not a version (`2.100` or `2.100.800000`) | fix the value |
| MOD021 | error | the install's newest patch is older than `REQUIRES_PATCH`, or there is no patch | install the newer patches |
| MOD022 | warning | an update archive is named for a newer patch than the install has (inferred from the name, see below) | install that patch, or check with the author |
| MOD023 | warning | the `.mod` names a game module other than `Sam2Game`: native code this engine cannot load | only the mod's content is used |
| MOD024 | warning | that game module's DLL is not in `Bin/` (only judged when `Bin/` exists) | copy the DLL |
| MOD030 | error | a `MODEL` file exists neither in the mod nor in the install | add the model or fix the path |
| MOD031 | warning | another referenced file (thumbnail, loading screen or model, movie) exists nowhere | add the file or fix the path |
| MOD032 | error | a reference with `..`, an absolute path, a drive letter or URL (`:`), or control characters | use a relative path below `Content/` |
| MOD033 | warning | a reference that mixes `/` and `\` | use `/` |
| MOD034 | warning | a reference key with an empty value | give a path or remove the key |
| MOD035 | warning | a `LEVEL` `.nfo` without the `.wld` of the same name | add the `.wld` |
| MOD036 | warning | a reference to a file of an unexpected type (a `.png` as `THUMBNAIL`) | check the path |
| MOD038 | error | archive entries whose names are not valid paths (`..`, drive letter, control characters); they are not loaded | repack with plain names |
| MOD039 | warning | entries of one archive with the same path (same name twice, or only case/slashes differ); the last wins | keep one |
| MOD040 | warning | file names not valid on Windows (`<>"\|?*`, trailing space or dot, `CON`, `NUL`, `COM1`...) | rename |
| MOD041 | info | files outside `Content/` (where the loaders do not look) | move them |
| MOD042 | info | placeholder archives (only `dummy.txt`); ignored | nothing |
| MOD043 | warning | a descriptor or `.nfo` larger than 1 MiB; not read | it is not a descriptor |
| MOD044 | warning | a file or folder of the mod cannot be read or inspected | check the archive |
| MOD045 | error | a folder with names that differ only by case (a loose folder on a case-sensitive system) | rename one |
| MOD046 | warning | native code or archives (`.dll`, `.exe`, `.module`, `.so`, `.gro`) inside the mod | remove; see the [modding policy](MODDING_POLICY.md) |
| MOD050 | info | how many files replace base files, and the biggest types | nothing |
| MOD051 | warning | the mod replaces critical files: `.wld`, `.ep`, `.rsc`, or the game's own descriptor or mod-selection files (one finding per type) | intended? every player needs the same files |
| MOD060 | warning | two mods in the load order provide the same files and at least one differs (or could not be compared) | enable one at a time, or order them |
| MOD061 | info | two mods provide the same files with identical content | one copy is redundant |
| MOD090 | error | the install itself cannot be read | run `ss2-doctor` without `--mod` |
| MOD091 | error | the named mod is not in the install, or a mod folder cannot be read | check the name |
| MOD092 | error | an archive of the mod cannot be opened; the rest of the mod is still checked | copy the file again |

Findings are listed per mod in load order, most serious first, then by code and path. Long lists are folded into one
finding that gives the count and the first three paths.

### Version rules

The original `.mod` and `.nfo` formats carry **no version field** (`docs/re/mods.md`: `NAME`, `AUTHOR`,
`THUMBNAIL`; `.mod` has only `MODULES`). So there are two sources, and both are weak:

- **`REQUIRES_PATCH`** (MOD020, MOD021) is an **optional key that SS2RS defines**; the original game ignores unknown keys. It is
  `major.minor[.build]`, written like the patch archives (`Patch_02_100_800000.gro` is `2.100.800000`). A mod author who knows the
  mod needs a patch level can write `REQUIRES_PATCH = "2.100" ;` in the descriptor. Nothing else defines it: it is not part of
  the original format.
- **Update archive names** (MOD022) are an **inference**: the InSamnity! 2 archives `z_InSamnity2Update_02_090.gro` and
  `z_InSamnity2Update_12_100.gro` end in a three-digit number that equals the minor number of the patches `Patch_02_090_*` and
  `Patch_02_100_*`. The check reads a name that ends in `_<digits>_<three digits>` as "this pack is for patch 2.`<minor>`" and
  warns when the install's newest patch has a lower minor. This is a guess from two names, not a documented rule, which is why
  it is only a warning.

### What "critical" means (MOD051)

The set is a project choice (`CRITICAL_EXTENSIONS`, `CRITICAL_PATHS` in `modcheck.rs`): levels (`.wld`), entity
parameters (`.ep`, the balance of weapons, enemies, projectiles) and resource databases/menus (`.rsc`) change how the game plays,
and in a game with several players every player needs the same ones (T13-006 will compare mod hashes); the game's own
descriptor `Content/SeriousSam2.mod`/`.nfo` and the mod-selection files `Content/DefaultMod.txt`/`CurrentMod.txt` choose the game itself.
Replacing a model or a texture is not critical. A warning is not a ban: Renovation replaces 31 levels on purpose.

### Load order and conflicts

`docs/re/mods.md` records that **how two mods are loaded together is not in the data** (one active mod is assumed). MOD060/MOD061
therefore answer "if both were mounted in this order, which files would be provided twice?", not "this
combination is supported". Two files are *identical* only when both are at most 8 MiB, have the same size and the same bytes; a
larger or unreadable file is reported as differing.

## How it was tested

Everything below was run on the commit this page ships with.

- **58 unit tests** in `modcheck.rs` (one of them covers the new `ArchiveVfs::duplicate_entries`, one checks that this page lists every code), **8 new command-line tests** in
  `crates/ss2-vfs/tests/doctor_cli.rs`. Every lint has a test on hand-made fixture mods built in memory (`MemoryVfs`) or in a
  temp folder with tiny `.gro` files made by `ZipBuilder`; nothing from a game install is in the repository. Every severity,
  the exit code, plural forms, the 3-example folding, and the rendering (a golden report, plus ASCII-only output for hostile names)
  are covered.
- **Damage tests:** a file system whose `list`, `stat` or `read` fails on chosen paths; descriptors and `.nfo` files of
  0xFF bytes, a lone BOM, unclosed quotes, a folder where a descriptor should be, files of exactly 1 MiB and 1 MiB + 1 byte,
  archive entries named `../evil.txt` or `C:/evil.txt`, case-only duplicates inside an archive. None panics; each becomes a finding.
- **Determinism:** the same input in reversed insertion order gives an equal report and equal text. (This found one real
  bug: the newest patch of an install was picked by directory read order when two patch names had the same numbers, so
  `ss2-doctor --mod` could print `2.100` or `2.100 (build 0)` on different runs. Ties are now broken by name.)
- **The real mods** (data-gated, `SS2_DATA_DIR`, release build, the whole check of both mods takes 0.8 s). The test fails if
  any finding other than the six expected codes appears, so a false positive of a lint shows up as a test failure:

  | Mod | Layers | Files | Replace base files | Errors | Warnings | Notes |
  |---|---|---|---|---|---|---|
  | `Sam2Renovation` | 1 | 3194 | 1963 | 0 | 1 | 1 |
  | `InSamnity2` | 8 (+8 placeholder archives) | 3169 | 399 | 0 | 4 | 2 |

  The override counts equal the ones measured by hand for `docs/re/mods.md` (1,963 and 399). Renovation's warning is MOD051 (31 levels replaced).
  InSamnity! 2's warnings are MOD023 (its own native game module `InSamnity2Game`), two MOD051 (7 `.ep`, 2 `.wld` replaced) and one
  **MOD031 that is a real defect of the mod**: `Content/SeriousSam2/Levels/FELevels.nfo` has `THUMBNAIL="Content/SeriousSam2/Levels/FELevels/01/01_thumbnail.tex"`,
  which is in no archive of the install (checked with a script over every `.gro`). Both mods together give one MOD060 (168 files that differ).
  **False positives found: none**; no lint had to be loosened for these mods.
- **Mutation check of the checker itself.** 164 one-line breakages of the non-test code (operators, comparison edges, constants,
  every lint code, every severity arm, deleted `continue`s) were applied one at a time and the lib tests run:
  **136 killed, 18 survived, 10 did not compile (not counted).** The 18 survivors were: 2 size constants (the tests derived the
  boundary from the constant: now literal sizes), 9 missing tests (edges of the "three examples" folding, the eight-type cut, a build number in a
  patch requirement, a patch minor with no patch installed, the two hints of MOD044/MOD045, an extension-less file in a mod folder: all added), 1 real bug (the
  patch tie above: fixed), and 6 **equivalent** mutants, which meant redundant code that was simplified instead of tested (an `is_empty` check that
  `parse` makes unnecessary, an `if` in front of a function that already returns on an empty list, `parent()` that is never `None` for a file,
  an `rsplit_once` that cannot fail after `ends_with`, an `is_dir` check that `read_dir` already implies). The 17 mutants of the lines that changed or
  survived were rerun after the fixes: **17 of 17 killed**. The 136 that were killed in the first pass were not rerun after the new tests and
  simplifications (the tests only grew); the full lib suite (131 tests) passes.
- Not run in this PR: the lints on Linux/macOS, and mutation of `ss2-doctor.rs` and of `check_install_mods`' folder/archive loaders beyond what the CLI and install tests exercise.

## Not covered

These are **not** done. Follow-up: T13-009b.

- **References inside binary files.** Only `.nfo` text is read. The references inside `.ep`, `.rsc`, `.ctl`, `.wld`, `.mdl`, `.mat` and the rest
  (about 600 in the `.ep` files of the base game alone, `docs/re/ep.md`) are not checked, so a mod that points a model at a missing texture is not caught.
  `ss2-format-ep` already has `referenced_files()`; wiring the formats in is a cross-crate change (RFC).
- **Case against the real file.** The VFS lowercases every path, so a reference written `Content/Foo.tex` for a file stored as `content/foo.tex` is accepted, and
  a loose mod unpacked on a case-sensitive file system is not tested for that. Only two names in one archive or folder that differ **only** by case are found (MOD039, MOD045).
- **Version information that does not exist.** The original formats have no version field. `REQUIRES_PATCH` is our own optional key, and the update-name rule (MOD022) is a guess from two names; nothing here
  is a statement about which patch a mod really needs. There is no check against the **mod's own** version (`Updater/CurrentVersion.DAT`).
- **Loose files in `Content/<Mod>/`** (`ChapterMaxes.txt`, `ReplaceHistory.txt`, a loose thumbnail) are used to resolve references but are not counted as mod files, not checked for names, and not part of the override and conflict counts.
- **Several mods.** References between mods are not resolved (MOD030/MOD031 fire if mod B names a file only mod A has). Conflicts compare the final file of each mod, not each layer, and compare bytes only up to 8 MiB.
  How the engine will really load mods together is not settled (T13-001, T13-006); MOD060 is a hypothetical.
- **Game modules.** MOD023/MOD024 look at the names in `.mod`; they do not inspect a DLL, and `Bin/<Name>.dll` is looked up only when the install has a `Bin` folder.
- **Other `.nfo` keys** (`MAP_TYPE`, `PLAYERS`, `ETA`, `LOCKED`, `MODEL_*`) are not validated; strings of the form `TTRS:...` are not looked up in a localization table.
- **What the engine does with a bad reference is not known** (`docs/re/mods.md`, "What is not in the data"). The severity of MOD030 (error) and MOD031 (warning) is our judgement.
- **Not tested** on Linux or macOS here, and not tested on a case-sensitive file system (MOD045 is tested with a mock).
- No machine-readable output (JSON), no `--fix`, no way to silence a code.
