# Modding: start here

This section is for someone who owns the game, has a text editor and wants to make or check a mod for SS2RS. You do not
need to know Rust. It has five pages:

| Page | What it is for |
|---|---|
| **Start here** (this page) | what a mod is, what works today, how a mod is laid out and mounted |
| [File formats](formats.md) | the game's file types, how to look inside them, what is known |
| [Mod API reference](api.md) | the descriptor keys, the lint codes, the settings; and what does **not** exist yet |
| [Tutorial: the example mod](tutorial.md) | make, check, inspect and load a tiny mod, step by step |
| [Troubleshooting](troubleshooting.md) | every message of `ss2-doctor --mod`, its cause and its fix |

> **Status: pre-alpha.** SS2RS does not play the game yet ([install guide](../user/install.md)), so **no mod can be played
> yet**. What exists is the part a mod author needs first: the code that reads a mod's descriptors, stacks its files over the
> game's, and the checker `ss2-doctor --mod` that tells you what is wrong. Everything in these pages was checked against the
> code or by running the tools; where something is a guess, it says so.

## What is supported today, and what is not

| | Today |
|---|---|
| Reading a mod's `.mod` and `.nfo` descriptors | **Yes** (`ss2_vfs::ModManifest`, `Nfo`, `discover_mods`) |
| Stacking a mod's `.gro` archives, or a loose folder, over the game's files | **Yes, as library code** (`ss2_vfs::mount_mod`, `DirectoryVfs`, `LayeredVfs`), tested; the game window does not call it yet, so there is no way to *choose* a mod in the program |
| Checking a mod for mistakes | **Yes**: `ss2-doctor --mod NAME`, `--mod-dir DIR`, `--all-mods` ([lint list](api.md#the-lint-codes)) |
| Replacing the game's textures, sounds, parameters by path | **Yes in principle** (a mod file with the same path wins); nothing plays them yet |
| Looking inside the game's files | **Yes**: `gro-tool`, `meta-dump`, `asset-export` ([formats](formats.md)) |
| Making new `.tex` textures | Only by hand-building the file; there is **no PNG-to-`.tex` converter** in the repository |
| Making models (`.mdl`, `.bmf`), levels (`.wld`), skeletons, animations | **No authoring tool.** The repository can read these formats; the only writer is the container writer of the test fixtures (`ss2_meta::write`) |
| A scripting language for mods | **No.** The runtime strategy is undecided ([T13-002](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/roadmap/T13-mods.md)); the `ss2-script` crate is an empty placeholder |
| A Rust plugin interface, a "Mod API" with events, spawning, UI hooks | **No** (T13-005 is planned for after the entity API is stable) |
| Native game modules (`.dll`) like the one `InSamnity! 2` ships | **Never**: a Rust engine cannot load them; the checker warns (`MOD023`) |
| Mods that must be the same on every player of a co-op game | **Not decided** (T13-006) |
| A mod list screen to enable or disable mods | **No** (T13-007) |

If you want a mod that only changes **data** (textures, sounds, parameter files, level descriptions) you are on firm ground:
that is what the descriptor and the overlay rules below describe. If you want new behaviour, there is nothing to build on yet.

Read the [modding policy](../MODDING_POLICY.md) before you publish anything: a mod must never contain Croteam's files, not even
converted ones.

## The tools you can use

All are built from this repository (`cargo run -p <package> -- ...`); they are not in a download yet.

| Tool | Package | Does |
|---|---|---|
| `ss2-doctor` | `ss2-vfs` (`--bin ss2-doctor`) | checks the install and, with `--mod`, your mod ([Mod validation](../MOD_VALIDATION.md)) |
| `gro-tool` | `gro-tool` | `list`, `find`, `where`, `cat`, `extract`, `stats` over the install's `.gro` archives as the game sees them (newest patch wins) |
| `meta-dump` | `meta-dump` | prints any CTSEMETA file (nearly every game file) as annotated text, JSON or RON ([meta-dump](../META_DUMP.md)) |
| `asset-export` | `asset-export` | `.tex` to PNG, `.bmf` to glTF ([asset-export](../ASSET_EXPORT.md)) |

`gro-tool` finds the install through `--data DIR` or the `SS2_DATA_DIR` environment variable. `ss2-doctor` takes the install
folder as its first argument, or `SS2_DATA_DIR`. Anything these tools write from the game's own files is the game's content
in another format: keep it outside any repository and never share it.

## The folder layout

The install folder is the one that holds `All_PC_01.gro`. A mod adds files next to the archives and below `Content/`:

```
<install folder>/
  All_PC_01.gro, All_PC_02.gro, All_PC_03.gro       the base game
  Movies_PC_01.gro, Movies_PC_02.gro                its movies
  Patch_02_*.gro                                     the patches (newest wins)
  Bin/                                               the game's modules (a mod must not add to it)
  Content/
    HelloMod.mod                                     the mod's module list
    HelloMod.nfo                                     name, author, thumbnail
    HelloMod/
      HelloMod.gro                                   the mod's files (a zip archive)
      z_HelloModUpdate_01.gro                        optional later packs
```

Inside `HelloMod.gro` every path starts with `Content/`. A mod puts its **own** files under `Content/HelloMod/` and replaces
a game file by repeating the game file's path (usually below `Content/SeriousSam2/`):

```
Content/HelloMod/Textures/Checker.tex                a new file of the mod
Content/SeriousSam2/Databases/Something.ep           would replace the game's file of this path
```

A mod being made is simpler: a **loose folder** with the same `Content/` tree and no archive (this is what `--mod-dir` checks
and what the [example mod](tutorial.md) is):

```
HelloMod/
  Content/
    HelloMod.mod
    HelloMod.nfo
    HelloMod/Textures/Checker.tex
```

A `.gro` file is an ordinary **zip archive** (files stored or deflated; the reader and its tests cover those two methods) whose
paths start with `Content/`. Any zip program makes one. Do not ship it with the game's files in it
([policy](../MODDING_POLICY.md)).

## The descriptor files

### `Content/<Mod>.mod`

```
MODULES = Sam2Game <TOOLS>Sam2GameGui ;
```

What the code reads (`ss2_vfs::ModManifest::parse`, verified by its tests and by the examples in these pages):

- the word `MODULES` (any letter case), `=`, then names separated by white space, then a closing `;`;
- a name with the prefix `<TOOLS>` belongs to the editor tools only; the others are the **game modules**;
- a UTF-8 byte-order mark is skipped, line ends may be CRLF or LF;
- it is an error (`MOD002`) if `MODULES`, `=` or `;` is missing, a name is empty or the list is empty.

For a **data-only mod write `Sam2Game`** (the example above is exactly what the base game's own descriptor says). Any other
game module name is native code of your own: the checker warns (`MOD023`) and only your content would be used.

The mod's **id** is the file name without `.mod`; the engine lower-cases it (`HelloMod.mod` is the mod `hellomod`), and the
mod's own folder is `Content/<id>/` whatever the case on disk.

### `Content/<Mod>.nfo`

```
NAME = "Hello Mod" ;
AUTHOR = "SS2RS example mod" ;
THUMBNAIL = "Content/HelloMod/Textures/Checker.tex" ;
```

The syntax is the general `.nfo` syntax the game uses for levels, episodes and player models too (`ss2_vfs::Nfo::parse`):

- an optional **kind word** alone on the first line (`LEVEL`, `EPISODE`); a descriptor has none;
- assignments `KEY = "value"`, the closing `;` optional; a value may start on the next line; a value that is **not quoted**
  runs to the end of its line or to a `;`;
- **there are no escapes in strings**: a value cannot contain a `"`. A quote that is never closed swallows the rest of the file
  (see [troubleshooting](troubleshooting.md#a-quote-that-is-never-closed-mod004));
- keys are compared without regard to case; if a key appears twice, the **first** one is used; unknown keys are kept and
  ignored;
- the file is UTF-8 (a BOM is skipped), or Latin-1 when it is not valid UTF-8;
- a descriptor or `.nfo` over 1 MiB is not read (`MOD043`).

The keys of a mod descriptor ([full table](api.md#nfo-keys)): `NAME`, `AUTHOR`, `THUMBNAIL`, and the SS2RS-only
`REQUIRES_PATCH`. Without a `NAME` the mod list shows the id; without an `.nfo` it shows only the id.

### Which mod is active

`Content/CurrentMod.txt`, or `Content/DefaultMod.txt` when there is no `CurrentMod.txt`, holds `CURRENTMOD = <file>.mod ;`
(lines that start with `#` are comments). The install's `DefaultMod.txt` names `SeriousSam2.mod`, the game itself: "the game"
is just the default mod. `ss2_vfs::selected_mod` reads these two files. Whether and how the program lets you change it is
**not decided** (T13-001).

## Load order and overrides, as the code does them

1. **Base archives first, then patches.** Every `.gro` directly inside the install folder is mounted. Names that do not start
   with `patch` come first, the `Patch*` ones after them; inside each group names are ordered "naturally" (numbers by value:
   `Patch_02_100` is after `Patch_02_068`, `All_PC_02` after `All_PC_01`). **A later archive wins.**
   (`ss2_vfs::archive_load_order`, `mount_install`.)
2. **Then the mod's archives.** Every `.gro` in `Content/<Mod>/` is mounted on top, in the same natural order, so the `z_`
   prefix of update packs puts them last. Archives that contain only a file `dummy.txt` are placeholders: skipped
   (`MOD042`). (`ss2_vfs::mount_mod`.)
3. **A file with the same path replaces the one below it.** Paths are compared **without regard to case** and `/` equals `\`:
   `Content\HelloMod\TEXTURES\checker.TEX` is the file `content/hellomod/textures/checker.tex`. Paths may not contain `..`, a
   drive letter or `:`, or control characters. Folder listings are the union of all layers.
4. **A loose folder is one more layer** (`ss2_vfs::DirectoryVfs`), mounted the same way. Links and special files inside it are
   not seen, and two names that differ only by case are an error (`MOD045`).
5. **Several mods:** `ss2-doctor --mod A --mod B` takes the order you give as the load order and reports files both provide
   (`MOD060`, `MOD061`). That is a question the checker can answer; **how the game will load several mods together is not
   settled** (the original data does not say, and one active mod is assumed: [spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/mods.md)).

One consequence to know about: a mod that replaces a game file replaces it for everyone who plays with the mod. The checker
singles out files that change how the game *plays* (`.wld` levels, `.ep` parameters, `.rsc` databases and menus): `MOD051`.

## What real mods look like

Two mods ship in the retail install (counted with `ss2-doctor --all-mods`; no content is quoted):

| Mod | Archives | Files | Replace game files | Native module |
|---|---|---|---|---|
| Renovation | 1 | 3,194 | 1,963 | none (uses `Sam2Game`) |
| InSamnity! 2 | 8 real + 8 placeholders | 3,169 | 399 | its own `.dll`: `MOD023` |

The doctor finds no error in either. Renovation is a pure content mod and is the model for what SS2RS aims to load first
(T13-003). The descriptor of Renovation uses exactly three keys, `NAME`, `AUTHOR` and `THUMBNAIL`.

## Next

[File formats](formats.md) to see what is inside the game's files, or straight to the [tutorial](tutorial.md).
