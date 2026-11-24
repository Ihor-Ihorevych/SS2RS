# Mod API reference

This page lists the **whole surface a mod can touch today**. It is short, because today a mod is **data plus descriptors**:
there is no scripting language, no plugin interface and no event or entity API. The [last section](#not-available-yet) lists
what is missing so that nobody builds on something that is not there.

Every statement here was checked against the code named next to it, or by running the tool; the tests in
`crates/ss2-vfs/tests/example_mods.rs` fail if a code, a flag or a key listed here stops matching the code.

## Nfo keys

An `.nfo` is `KEY = "value"` pairs ([syntax](start-here.md#contentmodnfo)). Unknown keys are kept and ignored, so you may add
your own (the example mod has `LICENSE = "CC0-1.0"`, which no tool reads).

### The mod descriptor `Content/<Mod>.nfo`

| Key | Default when absent | What reads it | Rule |
|---|---|---|---|
| `NAME` | the mod id (lower-case file stem) | mod list (`ModInfo::name`); `MOD005` when missing or blank | the text is trimmed |
| `AUTHOR` | none | `ModInfo::author`; `MOD006` when missing or blank | |
| `THUMBNAIL` | none | `ModInfo::thumbnail`; `MOD031`/`MOD036` check it | a `.tex` path; a value that is not a valid path gives no thumbnail |
| `REQUIRES_PATCH` | no requirement | `MOD020`, `MOD021` | **SS2RS's own key**; the original game ignores it. `major.minor` or `major.minor.build`, digits only, for example `2.100` or `2.100.800000` (a bare `2` is read as `2.0`) |

`REQUIRES_PATCH` is compared with the newest `Patch_02_<minor>[_<build>].gro` of the install (`2.100.800000` is
`Patch_02_100_800000.gro`). A missing patch, or an older one, is `MOD021` (an error). The original `.nfo` format has **no
version field**, so a mod's own version cannot be expressed yet.

### Keys that name a file (checked in every `.nfo` of the mod)

| Key | Must be | Missing file is | Where the original game uses it |
|---|---|---|---|
| `MODEL` | `.mdl` | `MOD030` error | player-model `.nfo` |
| `LOADING_MODEL` | `.mdl` | `MOD031` warning | level `.nfo` (the model shown while loading) |
| `THUMBNAIL` | `.tex` | `MOD031` warning | mod, level, episode, player model |
| `LOADING_SCREEN` | `.tex` | `MOD031` warning | level `.nfo` |
| `INTRO_MOVIE`, `OUTRO_MOVIE`, `EPISODE_MOVIE` | `.ogv` | `MOD031` warning | level and episode `.nfo` |

The value is a path below the install, written `Content/<Mod>/Models/x.mdl` (a path is found without regard to case, `/` or
`\`). A reference **resolves** if the file exists in the mod, in the game's archives and patches, or among the install's loose
files. It is an error (`MOD032`) if it has `..`, starts with `/` or `\`, has a `:` (drive letter, URL) or control characters;
a warning if it mixes `/` and `\` (`MOD033`), is empty (`MOD034`) or is the wrong type (`MOD036`). A reference into
*another mod* does not resolve. Only `.nfo` files are searched for references: the references **inside** `.ep`, `.rsc`, `.wld`,
`.mdl` and the other binary files are not checked yet (T13-009b).

### Level and episode descriptors

An `.nfo` whose first line is `LEVEL` describes a level and must sit next to a `.wld` of the same name (`MOD035` otherwise);
`EPISODE` describes a folder of levels. The keys the retail install uses are listed here with what is *known*; the doctor does
**not** validate any of them except the file keys above.

| Kind | Keys seen in the install (counted over all `.nfo` of the base game, the patches and the two mods) | Known meaning |
|---|---|---|
| `LEVEL` | `NAME`, `MAP_TYPE`, `THUMBNAIL`, `LOADING_MODEL`, `MODEL_NAME`, `MODEL_ROTATE`, `MODEL_DISTANCE`, `MODEL_ANIMATION`, `LOADING_SCREEN`, `ETA`, `PLAYERS`, `LOCKED`, `INTRO_MOVIE`, `OUTRO_MOVIE`, `EPISODE_MOVIE` | `MAP_TYPE` is a list such as `SINGLE; COOP;` or `DEATHMATCH;` (42 levels say `SINGLE; COOP;`); `NAME` may be a text key `TTRS:...`; `ETA` looks like `00:10:00`. `LOCKED`: not covered by any spec page |
| `EPISODE` | `NAME`, `THUMBNAIL` | |
| none (player model) | `NAME`, `MODEL`, `THUMBNAIL`, `DESCRIPTION`, `MODEL_STRETCH`, `EXPLODES` | `DESCRIPTION` refers to the text in the `.dsc` of the same name; `MODEL_STRETCH`, `EXPLODES`: not covered by any spec page |
| none (recorded demo) | `NAME`, `TIMESTAMP` | |

The counts come from a read of all `.nfo` files of the install on the author's machine; they say which keys exist, not what
the game does with them ([spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/misc-types.md)). The campaign model
reads `MAP_TYPE` and the three movie keys ([campaign flow](../CAMPAIGN.md)), nothing else.

### `.mod` file

`MODULES = <name> ... ;`, names with an optional `<TOOLS>` prefix; the only name a data mod may use is `Sam2Game`.

## The `ss2-doctor` command

```
ss2-doctor [FOLDER] [--mod NAME]... [--mod-dir DIR]... [--all-mods]
```

| Flag | Meaning |
|---|---|
| `FOLDER` | the install (holds `All_PC_01.gro`); without it the environment variable `SS2_DATA_DIR` |
| `--mod NAME` | check `Content/NAME/` of the install; repeat it for several mods, **in load order** |
| `--mod-dir DIR` | check a loose folder `DIR/Content/<Mod>.mod` ... (a mod being made) |
| `--all-mods` | check every `Content/<Name>/` folder that holds a `.gro` |
| `--help`, `-h` / `--version`, `-V` | usage / version, exit 0 |

**Exit codes:** `0` no error (warnings and notes allowed), `1` at least one error, `2` wrong command line (unknown option, an
option without a value, two folders, no folder and no `SS2_DATA_DIR`). The install is checked first and the mod report follows;
the report is plain ASCII and is the same on every run. `--all-mods` on an install without a mod folder prints
`No mod folders (Content/<Mod>/ with .gro archives) in the install.` and exits 0 if the install is fine.

The `ss2-vfs` library offers the same as functions (`ss2_vfs::check_install_mods`, `check_mods`, `Lint::ALL`, `Lint::code`,
`Lint::severity`, `ModReport::render`, `exit_code`) for tools of your own.

## The lint codes

Codes are stable: a lint keeps its code and its severity for good, and a new lint gets a new code. The cause and the fix of
each one are in [Troubleshooting](troubleshooting.md); the design and the limits of the checker are in
[Mod validation](../MOD_VALIDATION.md).

| Group | Errors | Warnings | Notes |
|---|---|---|---|
| Descriptors | `MOD001`, `MOD002` | `MOD003`, `MOD004`, `MOD005`, `MOD007`, `MOD008`, `MOD009`, `MOD043` | `MOD006` |
| Version and game module | `MOD021` | `MOD020`, `MOD022`, `MOD023`, `MOD024` | |
| References | `MOD030`, `MOD032` | `MOD031`, `MOD033`, `MOD034`, `MOD035`, `MOD036` | |
| Files and archives | `MOD038`, `MOD045` | `MOD039`, `MOD040`, `MOD044`, `MOD046` | `MOD041`, `MOD042` |
| Overrides and conflicts | | `MOD051`, `MOD060` | `MOD050`, `MOD061` |
| The check could not run | `MOD090`, `MOD091`, `MOD092` | | |

An **error** means the mod cannot work as it is, a **warning** that it is probably not what you want, a **note** is
information. There is no way to silence a code and no machine-readable (JSON) output yet (T13-009b).

## Settings and console variables

A mod **cannot define settings or console variables yet**. What exists belongs to the player and the developer:

| What | Where | Detail |
|---|---|---|
| Settings file | `<config dir>/SS2RS/settings.toml` (on Windows `%APPDATA%\SS2RS\settings.toml`) | keys `vsync` (default `true`), `fullscreen` (`false`), `master_volume` (`0.8`), `music_volume` (`0.6`), `sfx_volume` (`0.8`); volumes are clamped to 0..1 |
| Command line | `ss2-app [--connect HOST[:PORT] \| --listen] [--level NAME] [--data DIR] [--portable] [--set KEY=VALUE]...` | `--set` accepts the five settings above; booleans accept `on/off`, `1/0`, `true/false` |
| Environment | `SS2_DATA_DIR` (the install), `SS2_LOG` (log filter, for example `SS2_LOG=ss2_vfs=debug`) | |
| Console variable | `sv_cheats` (bool, default `0`) | must be `1` before `god`, `noclip`, `notarget`, `give <item> [amount]`, `spawn <class> [count]`, `killall`, `skip` work ([cheats](../CHEATS.md)) |

A mod may name an item or class in a cheat command only if the game has it. Whether a mod can *add* items, classes or
variables is the open question of the Mod API (below).

## Not available yet

None of these exists; do not write a mod that expects one.

- **Scripting or code in a mod.** The strategy (an embedded language, Rust plugins, or porting each mod by hand) is undecided
  (T13-002). `ss2-script` is an empty crate.
- **A Mod API**: registering entity classes, subscribing to events, spawning or damaging things, UI hooks, replication
  attributes, a stability policy (T13-005, after the entity API is stable).
- **Loading native game modules** (`.dll`): never. A mod like `InSamnity! 2` can only have its content used.
- **Choosing a mod in the program** (no mod list, no profile, no enable/disable: T13-007); how several mods combine is open
  (T13-001). Whether the game window mounts a mod at all is not wired yet.
- **Mod versions and dependencies.** There is no version field, and no way to say "needs mod B". `REQUIRES_PATCH` only
  speaks about the game's patch level.
- **Mods in multiplayer**: mod list and hash in the handshake, mismatch handling (T13-006).
- **Checks of references inside binary files**, JSON output, a way to silence a lint (T13-009b).
- **Authoring tools**: converters from PNG, glTF or WAV into the game's formats, a level editor, a `.gro` packer.
- **Hot reload of a mod**, and a published stability promise for any of the file or key names on this page.
