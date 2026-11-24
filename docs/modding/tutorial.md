# Tutorial: the example mod

You will check, inspect, break, fix and pack a tiny mod. Everything it contains is invented for this tutorial (no game
content) and lives in `examples/mods/HelloMod/` of the repository, licensed CC0-1.0.

A test (`crates/ss2-vfs/tests/example_mods.rs`) runs `ss2-doctor` on every folder of `examples/mods/` and fails unless the
exit code is 0 with no finding, and it fails if a report printed on this page differs from what the tool prints. So what you
read here is what you get, **for an install whose newest patch is `Patch_02_100_800000.gro`**: the line `Install patch level`
shows your install's own level, and the install check that `ss2-doctor` prints first (a list of your archives) is left out of the
reports below because it depends on your copy of the game.

## 1. What is in the example

```
examples/mods/HelloMod/
  Content/
    HelloMod.mod                                  the module list
    HelloMod.nfo                                  name, author, thumbnail
    HelloMod/
      Levels/HelloEpisode.nfo                     an episode description
      Textures/Checker.tex                        an 8 x 8 texture, 981 bytes
```

`Content/HelloMod.mod` (one line):

```
MODULES = Sam2Game <TOOLS>Sam2GameGui ;
```

`Content/HelloMod.nfo`:

```
NAME = "Hello Mod" ;
AUTHOR = "SS2RS example mod" ;
THUMBNAIL = "Content/HelloMod/Textures/Checker.tex" ;
LICENSE = "CC0-1.0" ;
```

`Content/HelloMod/Levels/HelloEpisode.nfo` (an episode: kind word, then keys; no `;` needed):

```
EPISODE
NAME = "Hello Episode"
THUMBNAIL = "Content/HelloMod/Textures/Checker.tex"
```

`Checker.tex` is a real `.tex` file (format version 8, 24-bit BGR, no mips): a checkerboard of orange and dark blue squares
of 2 by 2 pixels. It is **made by a test** (`tools/asset-export/tests/example_mod.rs`) from the public format spec, byte for
byte, so nobody has to wonder where it came from; the test also decodes it with `asset-export` and checks the pixels.
`UPDATE_FIXTURES=1 cargo test -p asset-export --test example_mod` rewrites it.

## 2. Build the tools

You need a Rust toolchain (see [developer tools](../DEV_TOOLS.md)) and a clone of the repository. Then, from its root:

```
cargo build -p ss2-vfs --bin ss2-doctor -p meta-dump -p asset-export -p gro-tool
```

The programs are in `target/debug/`. The commands below call them by name; from a checkout you can also write
`cargo run -p ss2-vfs --bin ss2-doctor -- ...`.

## 3. Check it

`ss2-doctor` needs your install (the folder with `All_PC_01.gro`) to resolve references to the game's files and to know the
patch level. It reads it and never changes it.

```
ss2-doctor "C:/Games/Serious Sam 2" --mod-dir examples/mods/HelloMod
```

The part of the output that belongs to the mod (it follows the install check):

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  no findings

No problems found (0 notes).
```

The exit code is `0`. The first line of the mod says: the id (`hellomod`, lower-case), the display name from the `.nfo`, how
many layers (a loose folder is one), how many files, and how many of them replace a game file (none: everything is under
`Content/HelloMod/`). If you leave the install out, `ss2-doctor` uses `SS2_DATA_DIR`; with neither it prints the usage and
exits with `2`.

## 4. Break it on purpose

Each experiment below was run exactly like this (the report is the tool's real output).

**Delete the thumbnail** (`Checker.tex`) while the descriptor still names it. The mod still works, so these are warnings and the
exit code stays `0`:

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 3 files, 0 override base files
  WARNING MOD031 content/hellomod.nfo: THUMBNAIL = "Content/HelloMod/Textures/Checker.tex" (thumbnail) exists neither in the mod nor in the install
          -> Add the file to the mod, or point the key at a file of the base game (or another mod you require).
  WARNING MOD031 content/hellomod/levels/helloepisode.nfo: THUMBNAIL = "Content/HelloMod/Textures/Checker.tex" (thumbnail) exists neither in the mod nor in the install
          -> Add the file to the mod, or point the key at a file of the base game (or another mod you require).

No errors, 2 warnings, 0 notes.
```

Put the file back. **Name a model that is nowhere**: add the line `MODEL = "Content/HelloMod/Models/Box.mdl"` to
`HelloEpisode.nfo`. A missing `MODEL` is an **error**, so the exit code becomes `1`:

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  ERROR   MOD030 content/hellomod/levels/helloepisode.nfo: MODEL = "Content/HelloMod/Models/Box.mdl" (model) exists neither in the mod nor in the install
          -> Add the model to the mod, or point MODEL at a model of the base game.

1 error, 0 warnings, 0 notes: fix the errors first.
```

Remove the line again. Every message of the tool has a section in [Troubleshooting](troubleshooting.md).

## 5. Look inside the texture

```
meta-dump examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex
```

prints the header (`format version: 8`), the chunk list and the type table with byte offsets; `meta-dump --unknown` on it ends
with `explained 981 bytes (100.0000%)`. `asset-export` turns it into a picture you can open anywhere:

```
asset-export tex examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex -o out/checker.png
examples/mods/hellomod/content/hellomod/textures/checker.tex -> out/checker.png (100 bytes) {"faces":1,"format":"Bgr8","mip_levels":1,"png_alpha":false,"size":[8,8],"source_size":[8,8]}
```

Do the same with a game texture (`gro-tool list "content/**/*.tex"`, then `asset-export tex --install DIR <path> -o out.png`) to
see what a real one looks like; keep what you write out of any repository. See [File formats](formats.md#tex-textures).

## 6. Make it yours

1. Copy the folder `HelloMod` to a new place and **rename the three things that carry the id** together: `Content/HelloMod.mod`,
   `Content/HelloMod.nfo` and the folder `Content/HelloMod/`. The file stem is the id; the doctor finds a mod's descriptors by it.
2. Edit `NAME`, `AUTHOR` and the paths in `THUMBNAIL` (a new id means new paths).
3. Run the doctor after every change. Aim for no error and a warning you understand.
4. Replacing a game file means writing a file at the game's path (for example under `Content/SeriousSam2/`). The doctor then
   adds a note, `MOD050`, that counts the replaced files by type, and a warning, `MOD051`, if you replaced a level, a
   parameter file or a menu database, because those change how the game *plays*. Example: a mod folder with a file at the path of a
   game `.ep` gives

```
Mod hellomod ("Hello Mod"): 1 layer, 5 files, 1 override base files
  WARNING MOD051 files of type .ep (entity parameters) that replace the base game's: the mod changes how the game plays, not only how it looks (1): content/serioussam2/databases/sample.ep
          -> Intended? In a game with several players every player needs the same files.
  INFO    MOD050 1 of 5 files replace base game files, 4 are new (replaced by type: .ep 1)
          -> Nothing to fix: this is what the mod changes. See the critical overrides below, if any.
```

   (There the install's base game had a file `Content/SeriousSam2/Databases/Sample.ep`; the doctor tests use an invented
   install for this.) Remember the [modding policy](../MODDING_POLICY.md): the file you write must be **yours**, never a
   copy or conversion of a game file.

## 7. Pack and install it

A mod that other people install is a `.gro` archive (a plain zip file) in `Content/<Mod>/`, with the two descriptors next to it:

```
<install folder>/Content/HelloMod.mod
<install folder>/Content/HelloMod.nfo
<install folder>/Content/HelloMod/HelloMod.gro      zip of the folder's Content/ tree, paths starting with Content/
```

Zip the **contents** of `examples/mods/HelloMod/` so that the first path inside the archive is `Content/...` (any zip program
will do; stored and deflated entries both load; name the result `HelloMod.gro`), copy the
`.mod`, the `.nfo` and the archive as above, and check it by name instead of by folder:

```
ss2-doctor "C:/Games/Serious Sam 2" --mod HelloMod
```

```
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod hellomod ("Hello Mod"): 1 layer, 4 files, 0 override base files
  no findings

No problems found (0 notes).
```

The archive also holds the two descriptors, and the doctor compares them with the loose copies: if they differ, you get `MOD008`
(a test covers this). Keep them identical.

## 8. Loading it

**The game cannot load a mod yet**: it does not play, and the program has no way to choose a mod
([start here](start-here.md#what-is-supported-today-and-what-is-not)). What does exist is the library that the program will
use, and a test shows it working on this very mod. Mounting the loose folder over a base install in Rust:

```rust
let mut vfs = ss2_vfs::mount_install(install_dir)?;                      // base archives, then patches
vfs.mount("HelloMod", ss2_vfs::DirectoryVfs::new("examples/mods/HelloMod")?); // the mod on top
let mods = ss2_vfs::discover_mods(&vfs)?;                                 // [ModInfo { id: "hellomod", name: "Hello Mod", .. }]
let texture = vfs.read(&ss2_vfs::VfsPath::new(r"Content\HelloMod\TEXTURES\checker.TEX")?)?;
assert_eq!(&texture[..8], b"CTSEMETA");
```

For an archive use `ss2_vfs::mount_mod(&mut vfs, install_dir, "HelloMod")` instead of the second line: it mounts every `.gro`
in `Content/HelloMod/` on top, in natural order. The test `the_example_mod_mounts_over_the_base_and_is_listed_with_its_name`
runs the first form (on an invented install) and checks the name, the author, the module list, the thumbnail path and that the
texture comes from the mod's layer whatever the letter case of the path.

## Where to go next

- What keys exist: [Mod API reference](api.md).
- What a code means: [Troubleshooting](troubleshooting.md).
- What is inside the other files: [File formats](formats.md).
- What you may publish: [Modding policy](../MODDING_POLICY.md).
