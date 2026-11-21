# Inventory of a Serious Sam 2 install (retail, patched to 1.0)

- **Roadmap task:** T01-003 · **Confidence:** high (mechanically generated) · **Author:** @Ihor-Ihorevych/agent
- **Method:** every `.gro` archive was opened as a ZIP and its entry table (names, sizes) and the first 8 bytes of each file were read. Only **paths, sizes and counts** are recorded here — no file contents. The full path list is generated locally by `gro-tool list` (T11-001) and is intentionally not committed.

## Archives and mount order

`.gro` files are standard ZIP archives. Order below is the proposed mount order (base archives first, then `Patch_*` by natural numeric order; **later archives override earlier ones** for the same case-insensitive path — see T02-002).

| # | Archive | Entries | Packed (MB) | Unpacked (MB) | On disk (MB) |
|---|---|---:|---:|---:|---:|
| 1 | `All_PC_01.gro` | 1,194 | 58.7 | 66.6 | 59.0 |
| 2 | `All_PC_02.gro` | 5,213 | 647.3 | 831.2 | 648.3 |
| 3 | `All_PC_03.gro` | 3,274 | 497.6 | 664.7 | 498.3 |
| 4 | `Movies_PC_01.gro` | 52 | 644.4 | 646.1 | 644.4 |
| 5 | `Movies_PC_02.gro` | 44 | 589.5 | 591.4 | 589.5 |
| 6 | `Patch_02_068.gro` | 360 | 27.4 | 46.0 | 27.5 |
| 7 | `Patch_02_080_210639.gro` | 100 | 0.6 | 1.7 | 0.7 |
| 8 | `Patch_02_080_243026.gro` | 14 | 0.2 | 0.4 | 0.2 |
| 9 | `Patch_02_080_243893.gro` | 4 | 0.0 | 0.0 | 0.0 |
| 10 | `Patch_02_080_244138.gro` | 10 | 0.0 | 0.1 | 0.1 |
| 11 | `Patch_02_080_244697.gro` | 1 | 0.0 | 0.0 | 0.0 |
| 12 | `Patch_02_080_252822.gro` | 2 | 0.0 | 0.0 | 0.0 |
| 13 | `Patch_02_090_593613.gro` | 1,332 | 342.3 | 550.9 | 342.5 |
| 14 | `Patch_02_100_800000.gro` | 688 | 201.6 | 322.1 | 201.7 |

**Totals:** 12,288 file entries across 14 archives; **10,933 distinct paths** (case-insensitive); **847 paths exist in more than one archive** (patch overrides).

## File types

`CTSEMETA` = file starts with the 8 bytes `CTSEMETA` (see `ctsemeta-container.md`).

| Extension | Files | Size (MB) | CTSEMETA | other |
|---|---:|---:|---:|---:|
| `.ogv` | 99 | 1,310.8 | 0 | 99 |
| `.wld` | 107 | 1,059.0 | 107 | 0 |
| `.tex` | 3,281 | 543.7 | 3,281 | 0 |
| `.wav` | 1,874 | 330.1 | 0 | 1,874 |
| `.mdl` | 3,827 | 229.6 | 3,827 | 0 |
| `.ogg` | 392 | 163.5 | 0 | 392 |
| `.bmf` | 123 | 27.4 | 123 | 0 |
| `.xmv` | 1 | 26.1 | 0 | 1 |
| `.ans` | 101 | 10.3 | 101 | 0 |
| `.fnt` | 33 | 4.6 | 33 | 0 |
| `.preload` | 46 | 3.9 | 0 | 46 |
| `.pfx` | 612 | 3.8 | 612 | 0 |
| `.ep` | 394 | 2.0 | 394 | 0 |
| `.rsc` | 391 | 2.0 | 391 | 0 |
| `.tga` | 14 | 2.0 | 0 | 14 |
| `.cb` | 82 | 0.5 | 82 | 0 |
| `.mtr` | 102 | 0.3 | 102 | 0 |
| `.ctl` | 88 | 0.2 | 88 | 0 |
| `.mat` | 65 | 0.2 | 65 | 0 |
| `.asm` | 2 | 0.2 | 0 | 2 |
| `.scs` | 46 | 0.2 | 46 | 0 |
| `.skl` | 33 | 0.1 | 33 | 0 |
| `.dsc` | 209 | 0.1 | 0 | 209 |
| `.srt` | 72 | 0.1 | 0 | 72 |
| `.tfx` | 28 | 0.1 | 28 | 0 |
| `.mac` | 8 | 0.1 | 8 | 0 |
| `.lfx` | 37 | 0.1 | 37 | 0 |
| `.nfo` | 122 | 0.1 | 0 | 122 |
| `.txt` | 15 | 0.1 | 0 | 15 |
| `.mch` | 8 | 0.1 | 8 | 0 |
| `.mas` | 21 | 0.0 | 21 | 0 |
| `.crt` | 10 | 0.0 | 0 | 10 |
| `.bin` | 1 | 0.0 | 0 | 1 |
| `.sfc` | 11 | 0.0 | 11 | 0 |
| `.ctr` | 3 | 0.0 | 3 | 0 |
| `.scp` | 7 | 0.0 | 7 | 0 |
| `.pss` | 7 | 0.0 | 7 | 0 |
| `.vfx` | 5 | 0.0 | 5 | 0 |
| `.sca` | 4 | 0.0 | 4 | 0 |
| `.cfg` | 4 | 0.0 | 0 | 4 |

(+1 rarer extensions, 3 files.)

## Directory layout (depth 3, entries)

| Folder | Entries |
|---|---:|
| `Content/SeriousSam2/Models` | 5,316 |
| `Content/SeriousSam2/Sounds` | 2,231 |
| `Content/SeriousSam2/Textures` | 2,018 |
| `Content/SeriousSam2/Databases` | 656 |
| `Content/SeriousSam2/Effects` | 636 |
| `Content/SeriousSam2/Levels` | 522 |
| `Content/SeriousSam2/Menus` | 361 |
| `Content/SeriousSam2/Movies` | 180 |
| `Content/SeriousSam2/Locales` | 135 |
| `Content/SeriousSam2/Music` | 78 |
| `Content/SeriousSam2/Misc` | 57 |
| `Content/SeriousSam2/Fonts` | 27 |
| `Content/SeriousEditor2/Gui` | 21 |
| `Content/SeriousSam2` | 8 |
| `Content` | 7 |
| `Content/SeriousSam2/Vibro` | 6 |
| `Content/SeriousSam2/Materials` | 5 |
| `Content/SeriousSam2/Config` | 4 |
| `Content/Fonts` | 4 |
| `Content/SeriousEditor2/CustomTrees` | 3 |
| `` | 2 |
| `Sources/Shaders` | 2 |
| `Content/SeriousEditor2/Shared` | 2 |
| `Content/SeriousSam2/Demos` | 2 |
| `Content/Engine` | 1 |
