# File formats

What is inside the game's files, in plain words, and how to look at each kind with the repository's tools. The engineering
specs behind every section (written by reading the files, never from the game's code) are linked at the end of each one; this
page summarises them and does not repeat their tables.

**How sure is it?** Each section says what is *verified* (checked against every file of the retail install by the spec, or
by a test in this repository) and what is *unknown*. "Read" means the repository has code that decodes the file; **none of
these formats can be written or edited by a tool in the repository yet** (only the test-fixture builders write them), so for
now this page is for understanding and checking, not for converting your own art into the game's formats.

## Looking inside: the four commands

```
gro-tool list "content/**/*.tex"            list files of the install, as the game sees them (newest patch wins)
gro-tool where content/serioussam2.mod      which archive provides a file (prints its name)
gro-tool cat <path> > somewhere-outside-the-repo     write one file's bytes to standard output
meta-dump FILE                               annotated text of a CTSEMETA file on disk
meta-dump --install DIR content/x.ep         the same for a file of the install
meta-dump --unknown FILE                     only how much of the file some decoder explains
asset-export tex FILE -o out.png             a texture as PNG
asset-export mesh FILE -o out.glb            a model mesh as glTF
ss2-doctor DIR --mod NAME | --mod-dir DIR    check a mod
```

(From a source checkout write `cargo run -p gro-tool --`, `cargo run -p meta-dump --`, `cargo run -p asset-export --`,
`cargo run -p ss2-vfs --bin ss2-doctor --`.) Exit codes: `gro-tool` 0 success, 1 nothing matched, 2 error; `meta-dump` 0
success, 1 only with `--unknown` when some bytes are undecoded, 2 unreadable or damaged file; `ss2-doctor` 0 usable, 1 errors
found, 2 wrong command line. More: [meta-dump](../META_DUMP.md), [asset-export](../ASSET_EXPORT.md),
[mod validation](../MOD_VALIDATION.md).

## Summary table

| Extension | What it is | Container | Read by | Written by a tool? |
|---|---|---|---|---|
| `.gro` | archive of the game's files | zip | `ss2-vfs` | any zip program |
| `.mod`, `.nfo`, `.txt` mod files | descriptors | text | `ss2-vfs` | a text editor |
| `.tex` | texture | CTSEMETA | `ss2-format-tex` | no |
| `.bmf` | mesh | CTSEMETA | `ss2-format-bmf` | no |
| `.mtr` | material (shader presets) | CTSEMETA | `ss2-format-mtr` | no |
| `.mat`, `.scs`, `.scp`, `.sca`, `.mas` | physical materials and sound, particle, animation schemes | CTSEMETA | `ss2-format-mat` (`.mat`, `.scs`, `.scp`) | no |
| `.ep` | entity parameters (damage, speed, models of a weapon, enemy, pickup) | CTSEMETA | `ss2-format-ep` | no |
| `.rsc`, `.ctl` | resource databases and menu screens; character tools | CTSEMETA | `ss2-format-rsc` | no |
| `.fnt` | bitmap font | CTSEMETA | `ss2-format-fnt` | no |
| `.ans` | animation set | CTSEMETA | `ss2-format-ans` | no |
| `.pfx`, `.lfx`, `.tfx`, `.vfx` | particle effect, lens flare, text effect, vibration | CTSEMETA | `ss2-format-fx` | no |
| `.mdl`, `.skl` | model, skeleton | CTSEMETA | not yet (specified only) | no |
| `.wld` | world (level) | CTSEMETA | `ss2-format-wld` (minimal: gravity, static model placements, spawn markers) | no |
| `.wav`, `.ogg` | sound | RIFF, Ogg Vorbis | `ss2-audio::decode` | any audio tool |
| `.ogv` | movie | Ogg Theora/Vorbis | not yet (T06-010) | any video tool |
| `.xmv` | Xbox movie | proprietary | **ignored by design** | no |
| `.dsc`, `.srt`, `.crt`, `.preload` | descriptions, subtitles, credits, preload lists | text | specified only | a text editor |

## CTSEMETA: the container of almost everything

**What.** Nearly every game file except sound, video and text is a *CTSEMETA* file: a self-describing container. It starts
with the text `CTSEMETA`, a byte-order marker (`0x1234ABCD`), a format version and a build string; then a list of the files it
refers to (`RFIL`), a string table of identifiers (`IDNT`), a table of its own data types (`INTY`), and the objects written with
those types (`OBJS`), ending in `METAEND`. Because the file carries its own type table, a reader needs no outside schema.

**Inspect.** `meta-dump FILE` prints all of this with byte offsets. This is the real beginning of the output for the example
mod's texture (`examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex`, made for these pages):

```
# meta-dump (annotated text; offsets are file offsets, +n is a size in bytes)
file: examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex
size: 981 bytes
status: ok

== header 0x00000000..0x0000004b (75 bytes)
magic: CTSEMETA   byte-order marker: 0x1234ABCD
format version: 8
version string: "$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5$"
build: product "Serious Sam 2", release 2.062.00, build 57284, tag 2005-Gold-RC5
```

and `meta-dump --unknown` on it ends with `explained 981 bytes (100.0000%)`.

**Verified.** The frame, the type table and the object data of all 9,269 format-8 file entries of the retail install decode
and end exactly on the end marker. Format versions 4 to 8 are read by `ss2-meta`; **version 8 is the one the game's files use**
(older versions occur in 155 development-build textures only).
**Unknown.** The meaning of some header words (`INFO` flag) and of fields with numbers for names: `meta-dump` marks them
"meaning unknown". The specs' confidence line says which parts are high, medium or low.
**Spec:** [container](../re/ctsemeta-container.md), [types](../re/ctsemeta-types.md),
[objects](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/ctsemeta-objects.md).

## `.tex`: textures

**What.** One image, with width, height, a format code, flags, wrap modes and **one zlib-compressed block** holding all the
pixels. After inflating, the pixels are one of five layouts: 24-bit BGR, 32-bit BGRA, or DXT1, DXT3, DXT5 block-compressed
(which carry a full chain of smaller copies, the *mips*). A cube map is six DXT1 faces. Rows are stored top to bottom, and the
colour bytes are in the order blue, green, red (alpha last).

**Inspect.** `asset-export tex FILE -o out.png` (options `--mip N`, `--face N|all`, `--rgb-only`, `--alpha-only`); `meta-dump
FILE` for the raw fields. Real run on the example texture:

```
asset-export tex examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex -o out/checker.png
examples/mods/hellomod/content/hellomod/textures/checker.tex -> out/checker.png (100 bytes) {"faces":1,"format":"Bgr8","mip_levels":1,"png_alpha":false,"size":[8,8],"source_size":[8,8]}
```

**Verified** (spec, all 2,610 format-8 textures of the install): the pixel size predicted from the format equals the inflated
size in every file; 24-bit, 32-bit, DXT1, DXT3 and DXT5 are converted by the exporter, DXT in software (a GPU may differ by one
level per channel). **Unknown.** What format codes that share a layout are *for* (the codes `0, 2, 6` are all 24-bit, `1, 3,
4, 5, 11` all 32-bit), the meaning of several flag bits (only `0x02` = cube map and `0x10` = baked lightmap are firm), the cube
face order (medium confidence), and the 155 old version 4 to 7 textures (the reader refuses them with a message that says so).
Normal-map channels are not interpreted.
**Spec:** [tex](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/tex.md),
[flags](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/tex-flags.md).

## `.bmf` meshes, `.mtr` materials, `.mdl` models, `.skl` skeletons

**`.bmf`** is one render mesh with one to four levels of detail (LOD), each with surfaces; vertex and index data sit in raw
buffers and every surface says where its streams start. The reader gives indices, positions, normals, optional tangents, UV sets
and bone weights. `asset-export mesh FILE -o out.glb` writes glTF (`--lod N`, `--all-lods`, `--flip-winding`); real fixtures
are in `testdata/asset-export/`. **Verified:** the layout on all 76 distinct format-8 meshes. **Unknown:** handedness, winding
and the direction of the UV v axis (the exporter writes the numbers as stored and says so); morph maps and the collision mesh
are not read; a skinned mesh is exported without its skeleton. [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/bmf.md).

**`.mtr`** is a material: one or more *shader presets*, each a list of configs that pair a shader class (shade, specular,
reflection, refraction, blended layers, terrain, ...) with numbered parameters and texture references. The same preset
objects are embedded in `.bmf` and `.mdl` files (7,391 of them). **Verified:** structure on all 91 files, shader class codes,
member layout. **Unknown:** what each shader computes (the shader programs are not in the data), blend types, the modifier
members; the role of a texture slot is inferred from the textures that fill it. [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/mtr.md).

**`.mdl`** is a model: a mesh reference, materials, attachments, animation host, physics and destruction parts (3,707 files).
**`.skl`** is a skeleton (31 files, 564 bones). **No reader exists in the repository yet**; `meta-dump` prints both.
[mdl](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/mdl.md), [skl](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/skl.md).

## `.mat`, `.scs`, `.scp`, `.sca`, `.mas`: what a surface is made of

A `.mat` file gives a physical material: density and friction-like numbers, a speed multiplier, damage over time, and two
schemes: a **sound scheme** (`.scs`) and a **particle scheme** (`.scp`) that map events such as `BulletHit` or `Step` to
weighted alternatives of sounds (`.wav` paths) or effects (`.pfx` paths). A scheme may name a base scheme that is searched when
an event is not found. `.sca` and `.mas` are the animation-side schemes. **Verified:** structure of all 35 `.mat`, 37 `.scs`, 7
`.scp`, 4 `.sca` and 20 `.mas`; the reader covers the first three. **Unknown:** the units of density and friction (medium),
`CMaterialAttributes.3/.4` and the flag bits (low). [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/mat.md).

## `.ep`: entity parameters

**What.** The numbers and references of one kind of game object (a projectile, an enemy, a pickup, a weapon): the class chain
from `CEntityParams` down, every member stored with its qualified name (`Class.member`), identifiers resolved to text,
references to other files (models, controls, textures, sounds, other `.ep`) as paths, owned sub-objects (attacks, blasts,
graphs) inlined. **Nothing is inherited from another file**: each file lists every member of its chain.

**Inspect.** `meta-dump --install DIR content/<path>.ep`. The reader exposes `referenced_files()`, which the checker does not use
yet (T13-009b).

**Verified:** structure and links on all 322 distinct `.ep` files and all 59 worlds. **Unknown:** what most numbers mean
(units, effects): names that are only numbers stay numbers, and gameplay meaning is specified elsewhere
(`docs/re/enemies.md`, `weapons.md`, ...). Because a replaced `.ep` changes balance for every player, the checker flags it
(`MOD051`). [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/ep.md),
[schema](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/ep-schema.md).

## `.rsc` and `.ctl`; `.fnt`; `.ans`; `.pfx` and friends

- **`.rsc`** (112 files) is a generic resource database: the class of its root object says what it is, one of 99 menu screen
  layouts, a skeleton, or one of ten global databases (cameras, game parameters, HUD, crosshairs, power-ups, menu fonts,
  spawning). **`.ctl`** (68 files) describes a held, picked-up or thrown object. `ss2-format-rsc`; replacing an `.rsc` is also a
  critical override (`MOD051`). [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/rsc-ctl.md).
- **`.fnt`** (21 files, 13 for the game) is a bitmap atlas plus glyph rectangles and a code table; glyph shapes are in the
  alpha channel; glyph 0 is the fallback. Verified by rendering test strings from the decoded data; two glyph members are
  medium confidence. [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/fnt.md).
- **`.ans`** (88 files; 354 more sets inside `.mdl`) holds animations: frame ranges, rates, sparse keyframe tracks per bone,
  events. **Unknown:** interpolation and looping (the reader chooses linear and shortest-arc and says so). [Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/ans.md).
- **`.pfx`** (604 files) particle effects, **`.lfx`** lens flares, **`.tfx`** text effects, **`.vfx`** controller vibration; almost
  every number is a constant that a graph may replace. **Unknown:** how graphs are evaluated and the exact simulation. [pfx](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/pfx.md), [fx](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/fx.md).

## `.wld`: levels

A world holds entities (73,098 in the 59 worlds), aspects, lighting, navigation, physics, visibility and the level's macro
program. `ss2-format-wld` reads the minimum needed to show and walk a level: the gravity, every placed static model (the
`.mdl` path, world placement, per-axis scale, whether it has a collision hull), the spawn markers and the chapter info
entities; the rest is not read yet, and there is **no level editor**. A
level in a mod is a `.wld` plus its description `.nfo` in the same folder (the checker warns about a level `.nfo` with no `.wld`,
`MOD035`). [Overview](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/wld-root.md).

## `.nfo` and the small text files

`.nfo` is described in [start here](start-here.md#contentmodnfo) and the [Mod API](api.md#nfo-keys). The other text types
(`.dsc` a description paragraph that goes with an `.nfo`; `.srt` subtitles that also schedule voice lines; `.crt` credits;
`.preload` lists of files to load first) are specified in [the spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/misc-types.md): UTF-8 (sometimes with a BOM, a few Latin-1), CRLF or LF. **No reader
for them exists in the repository yet.** Localised copies of text live under `Content/SeriousSam2/Locales/<lang>/` with the
original path repeated below it; the retail install has English plus a partial German overlay, no translation tables
([spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/localization.md)).

## Sound: `.wav` and `.ogg`

Sound effects are `.wav`, music and long voices are Ogg Vorbis (`.ogg`). **Verified:** all 1,868 of the game's `.wav` are 16-bit
PCM (1,861 mono, 7 stereo; mostly 44,100 or 22,050 Hz) and all 392 `.ogg` are 44,100 Hz Vorbis (314 mono, 78 stereo); the
decoder (`ss2_audio::decode`, [sound decoders](../AUDIO_DECODERS.md)) opens every one of them. **Also accepted** by the decoder, so a
mod may use them: 8, 24 and 32-bit PCM, 32 and 64-bit float, **IMA ADPCM**, each also in the "extensible" form; refused, with a
message that names the format: Microsoft ADPCM, A-law, mu-law, MP3 inside a WAV. Limits: at most 8 channels, 384 kHz, and half
an hour per sound. **Advice** (not verified by playing it, since nothing plays yet): keep effects **mono**, because positional sounds
need a single channel (nearly every game effect is mono), and music stereo at 44,100 Hz as the game's is. There is no repository
tool that prints a sound's header for you; `ffmpeg -i file` does (listed in [developer tools](../DEV_TOOLS.md)).
[Spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/audio.md).

## Video: `.ogv` plays, `.xmv` is ignored

Movies are Ogg containers with Theora video and Vorbis audio (`.ogv`). A level or episode `.nfo` names them with
`INTRO_MOVIE`, `OUTRO_MOVIE` and `EPISODE_MOVIE`; each movie has a same-named `.srt` subtitle file. **`.xmv`** is the Xbox
console format: the engine **skips** it with a warning (the cutscene is skipped, no error); the doctor accepts only `.ogv` for the
movie keys, so a mod that names an `.xmv` gets `MOD036` (or `MOD031` when the file is not there). The decoder and cutscene player do not exist yet (T06-010, T06-011).
The install has 98 `.ogv` and 1 `.xmv`. [Video formats](../VIDEO_FORMATS.md),
[cutscene spec](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/cutscenes.md).

## `.gro`: archives

A `.gro` is a standard zip file. The reader opens **stored** entries straight from the file (important for the big movie
files) and **deflated** entries into memory with a CRC check. Entry names are normalised like all paths (case-insensitive,
`/` equals `\`); an entry with `..`, a drive letter or control characters is **not loaded** (`MOD038`); two entries with the same
normalised path load as one: the **last** wins (`MOD039`). The install's base archives and patches are named in
[start here](start-here.md#the-folder-layout); [the install inventory](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/inventory.md) lists them.
