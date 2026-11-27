# Mod loading (`ss2_vfs`, T13-001)

How SS2RS finds the mods of an install, puts them in a load order, mounts them on the file system stack and reports which
mod overrides which file. It is a **library plus two `ss2-doctor` options**; the game window does not mount a mod yet
(that needs the mod browser, T13-007, and the data-only run of Renovation, T13-003).

The file formats (`.mod`, `.nfo`, `DefaultMod.txt`, overlay archives) are in the spec `docs/re/mods.md`; the modder's view is in
[Modding: start here](modding/start-here.md); the lints are in [Mod validation](MOD_VALIDATION.md).

## What comes from the original game and what is ours

The original game starts **one** mod (`Content/CurrentMod.txt`, else `Content/DefaultMod.txt`) and the data does not say
how two mods would combine (`docs/re/mods.md`, "What is not in the data"). So:

| Rule | Source |
|---|---|
| a mod is `Content/<id>.mod` + `.nfo` + overlay `.gro` archives in `Content/<id>/` | the original game |
| the base game is the default mod `serioussam2`; `CurrentMod.txt` beats `DefaultMod.txt` | the original game |
| archives replace by path, without regard to case; a later archive wins | the original game |
| placeholder archives (only `dummy.txt`) are not mounted | spec reader invariant 3 |
| **several mods at once**, their order, and `REQUIRES_MOD` | **SS2RS's own choice**, not a statement about the original |

A stack of one mod behaves exactly like the original's "current mod".

## Finding the mods: `scan_installed_mods`

`scan_installed_mods(dir)` lists every mod of the install folder (the one with `All_PC_01.gro`): the ids that have a
`Content/<id>.mod` in the base archives or loose in `Content/`, plus every `Content/<id>/` folder with `.gro` archives.
For each mod it reads the descriptor from the **first** of these places that has it: a loose file, the mod's own archives
(the last mounted first), the base archives. (Renovation's descriptor is loose; InSamnity! 2's exists only inside its own
archive; the game's own `SeriousSam2.mod` is in a patch.) A descriptor or `.nfo` over 1 MiB is not read.

A damaged mod does not stop the scan. It becomes a `ModProblem` (`NoDescriptor`, `BadDescriptor`, `BadNfo`,
`UnreadableArchive`) and the others are still listed. A mod with no usable `.mod` cannot be loaded and is unknown to the
load order; a mod with a bad `.nfo` is listed under its id; a mod with an archive that cannot be opened is listed but
cannot be mounted (**a mod is mounted whole or not at all**).

`InstalledMod` carries the id (lower case), name, author, thumbnail, the parsed `.mod`, the archives in mount order, the
placeholder archives, where the descriptor came from, and `native_modules()` (game modules other than `Sam2Game`: SS2RS cannot
load them, so such a mod can only have its content used; InSamnity! 2 is one).

`InstalledMods::selected` is the id named by `CurrentMod.txt`/`DefaultMod.txt`; `default_request()` is that mod, or nothing
when it is the base game.

## Load order: `resolve_load_order`

The request is a list of mod ids, **first loaded first**; **a later mod wins**. The base game is always at the bottom and
is never in the plan (naming it changes nothing). Ids are compared lower-case and may be written with `.mod`.

1. A mod loads after every mod it needs, and what it needs is pulled in automatically, transitively.
2. Among the mods that may load next, a pulled-in dependency goes before a requested mod; requested mods keep the order of
   the request.
3. What is still tied is ordered by id.

So a request that already respects the dependencies comes out unchanged, the same request gives the same plan whatever
the order mods were scanned in, and planning a plan again gives the same plan (property tests). Errors, all naming the mod:
a mod that is not installed (or cannot be loaded) and a circle of mods that need each other (`a -> b -> c -> a`, from its
smallest id). A circle that is not reachable from the request is not an error.

### `REQUIRES_MOD`

An optional key of the descriptor `.nfo` (the original game ignores unknown keys): ids or `.mod` names separated by `;`, `,`
or white space.

```
NAME = "My addon" ;
REQUIRES_MOD = "Sam2Renovation.mod; OtherMod" ;
```

It orders and pulls in; it is **not** a version check and not a promise that the combination works. A mod that needs itself or
the base game needs nothing. There is no version field in the original format, so there are no version ranges.

## Mounting: `mount_mods`

`mount_mods(dir, &installed, &plan)` mounts the base archives and patches (`mount_install`: base archives, then
`Patch_*` in numeric order), then each planned mod's archives in plan order, each mod's archives in the natural name order
(so `z_` update packs come last). Layers are named `<mod id>/<archive>`; `LayeredVfs::source_of(path)` says which one
serves a path. Whatever is later in that stack wins:

```
All_PC_01 < ... < Patch_02_100_800000 < modA/modA.gro < modA/z_modAUpdate_01.gro < modB/modB.gro < ...
```

## Conflict report: `analyze_overlays`, `overlay_report`

For a plan, `OverlayReport` says per mod how many files it provides, how many replace a base file or are new, how many a later
archive of the same mod replaces again, and how many files it loses to or wins from other mods. `conflicts` lists every file that two
or more mods provide: the **winner**, the **losers** in load order, whether the base game has the path, and whether the winning
copy is `Identical`, `Different` or `Unknown` (over 8 MiB with equal sizes, or unreadable). `pairs` sums that per pair of mods,
with up to three example paths. Only files count; a folder is never a conflict. The winner of each conflict is the layer
`LayeredVfs` really serves (tested). `analyze_overlays` takes any file systems, so the validator (T13-009) and the mod browser
(T13-007) can call it without an install; `overlay_report` is the same for an install and a plan.

The report is deterministic: sorted, no hash order, no clock, ASCII text from `render()`.

## On the command line

```
ss2-doctor [FOLDER] --list-mods
ss2-doctor [FOLDER] --load-order [--mod NAME]...
```

`--list-mods` prints the mods; `--load-order` prints the plan (the `--mod` names, or the selected mod), a note for mods with a
native module, and the overlay report. Exit code 1 if the plan cannot be made. Example on the install's Renovation and
InSamnity! 2 (numbers as in `docs/re/mods.md`):

```
Mods in load order (a later mod wins):
  1. sam2renovation: 3194 files; base game files replaced: 1963; new: 1231; ...
  2. insamnity2: 3169 files; base game files replaced: 399; new: 2770; ...
Files that more than one mod provides: 168
  insamnity2 wins over sam2renovation: 168 files (identical: 0, different: 168, not compared: 0): ...
```

## How it was verified

See the pull request for the commands run. Unit and property tests (`modload.rs`, `modinstall.rs`, `overlay.rs`, and
`crates/ss2-vfs/tests/doctor_cli.rs`) use hand-made `.gro` files; nothing from the game is in the repository. Data-gated tests
(skipped without `SS2_DATA_DIR`) run on the real install's mods: the scan finds `insamnity2`, `sam2renovation` and the base game
with the archive counts of the spec, every mod mounts, and the overlay counts equal the spec's measurements (Renovation 3,194 files,
1,963 replacing base files; InSamnity! 2 399 replacing; 168 files provided by both, all different).

## Not done (T13-001b)

- The game window and the engine do not call this yet: no mod is mounted at start-up, and there is no "write `CurrentMod.txt`"
  function (how the original launcher switches mods is not in the data).
- Per-profile or per-save mod lists, enabling and disabling in a UI (T13-007), and mod list/hash in the network handshake (T13-006).
- No version checks between mods; `REQUIRES_MOD` is only an order and a pull-in.
- Whether the original engine could load several mods together is unknown, so the multi-mod rules above are SS2RS's own and
  have not been compared with any original behaviour.
- References between mods in the validator (T13-009b) do not use the plan yet.
- Only run on Windows.
