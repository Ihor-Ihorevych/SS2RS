# CTSEMETA container (file frame)

- **Roadmap task:** T01-004
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet — needs an independent re-read (T01-049)_
- **Confidence:** **high** for the frame (header → type table → object-table headers → footer); **medium** for the meaning of individual fields (stated per field); **low / open** for the object *data* (see "Open questions").

## Summary

Almost every asset in a Serious Sam 2 install is stored in one self-describing binary container that starts with the 8 bytes `CTSEMETA`: **9,424 files** (textures `.tex`, models `.mdl`, meshes `.bmf`, worlds `.wld`, animations `.ans`, skeletons `.skl`, entity parameters `.ep`, databases and menus `.rsc`, particle effects `.pfx`, fonts `.fnt`, materials `.mtr`/`.mat`, and a dozen rarer types — see `inventory.md`). The container holds a table of referenced files, a table of identifiers, the **definitions of the data types** used by the file, and then **objects** serialized according to those types.

Because the type definitions travel inside each file, **one generic reader can parse every CTSEMETA file**; per-format code only has to interpret the resulting objects. This is the main finding for milestone M1.

All integers are **little-endian**. All counts and lengths are `u32`. Strings are raw bytes (Latin-1/ASCII in practice), **not** NUL-terminated, always preceded by a `u32` byte length.

## Evidence

Everything below was measured on the retail install (14 `.gro` archives, 12,288 file entries). A throw-away parser written from these rules (not committed) walked **9,269 of 9,269** format-8 files from the first byte to the final `METAEND ` marker with no mismatch, and cross-checks listed per field held as stated. No binaries or decompiler output were used (see `CLEANROOM.md`).

## File layout (format version 8)

```
header
INFO   5 × u32
RFIL   u32 count, count × file-reference entry
IDNT   u32 count, count × identifier entry
EXTY   u32 count, count × external-type entry
INTY   u32 N                      (number of DTTY blocks in the type table)
type table: DTTY blocks, with ADIM blocks interleaved   (N DTTY blocks in total)
EXOB   u32 count, count × 16-byte entry
OBTY   u32 count, count × 8-byte entry
EDTY   u32 count, count × 8-byte entry
OBJS   u32 count, … object data …
EDOB   u32 count, then count extra object records   (count 0 in most files)
"METAEND "                         (8 bytes, the last bytes of every file)
```

The section **order is identical in every one of the 9,269 format-8 files**; sections are never reordered or omitted (some are empty, e.g. `RFIL` with count 0). Tags are 4 ASCII bytes with no length prefix and no padding.

### Header

| Offset | Size | Field | Notes |
|---:|---:|---|---|
| 0 | 8 | magic `CTSEMETA` | |
| 8 | 4 | `0x1234ABCD` | byte-order marker; the same value in all files |
| 12 | 4 | format version | **8** in 9,269 files; 4, 5, 6, 7 only in 155 old `.tex` files (specified in `ctsemeta-leftovers.md`: a four-word `INFO`, and in version 4 a member number before every member name) |
| 16 | 4 | `L`, version-string length | |
| 20 | `L` | version string | `$Version: Serious Sam 2; <maj>.<min>.<rev>:<build>; <date-tag>$` |

157 distinct version strings occur, from the gold build `2.062.00:57284; 2005-Gold-RC5` (most files) through patch builds `2.066` (`2005-PATCH-2-068-inc`), `2.090`, `2.092`, `2.100` (`2021-PATCH-2-090-inc`). A file keeps the version of the build that last wrote it, so **a loader must accept any version string** and should not use it for anything but diagnostics.

### INFO — 5 × `u32`

| Index | Meaning | Confidence |
|---:|---|---|
| 0 | unknown: 1 in 7,294 version-8 files (never in a file with extra records), 0 in 769 (all 119 files with extra records, plus 650 others); a reader ignores it (`ctsemeta-leftovers.md`); absent in versions 4–7 | open |
| 1 | number of `RFIL` entries | **high** (9,269/9,269) |
| 2 | number of `IDNT` entries | **high** (9,269/9,269) |
| 3 | total number of types = `EXTY` count + `INTY` value | medium (holds in every sampled row; exact in only part of a first, incomplete check) |
| 4 | total number of objects = `EXOB` count + `OBTY` count (= `OBJS` count in the files checked) | **high** (9,090/9,175) |

### RFIL — referenced files

`u32 count`, then per entry: `u32 index` · `u32 flags` · `u32 len` · `len` bytes path.

- `index` is always `0..count-1` in order (9,269/9,269).
- `flags` is `0` (49,376 entries) or `0x00040000` (3,743 entries); meaning unknown. Observed (`ctsemeta-leftovers.md`): only worlds, `.ep`, `.rsc` and `.ctl` files carry the flag, and it sits on references made by the parameter layer (weapon, player, HUD and character databases); a reader can ignore it.
- The path is an engine-relative resource path with `/` separators, for example `Content/SeriousSam2/Models/Weapons/AutoShotgun/AutoShotgun_FP.mdl`. It names another file that the VFS must resolve (case-insensitively; see T02-002).

### IDNT — identifiers

`u32 count`, then per entry: `u32 id` · `u32 len` · `len` bytes. Ids are always `0..count-1` in order and **id 0 is the empty string** (9,269/9,269). The table is a string pool referenced by id from object data (references to it from objects are not yet specified).

### EXTY — external types

`u32 count`, then per entry: `u32 typeId` · `u32 len` · `len` bytes name (e.g. `CModelConfiguration`, `CCharacterTool`, `SGenericProjectileParams`). External types are classes **defined by the engine/game and not described inside this file**.

### INTY and the type table

`INTY` is followed by one `u32` giving the number of internal-type definitions. The table that follows is a sequence of `DTTY` blocks (specified in `ctsemeta-types.md`) with **`ADIM` blocks interleaved** (`ADIM`, `u32 n`, then `n × u32`: array dimensions, e.g. `n = 1`, value `4`). In every file the number of `DTTY` blocks equals the `INTY` value, **counting the ones that appear after an `ADIM`** (3,951/3,951 files with `ADIM`). The table ends where the next tag is `EXOB`.

`typeId` values from `EXTY` and from `DTTY` live in **one shared id space**.

### EXOB — external objects

`u32 count`, then `count × 16 bytes`: four `u32`: `objectId`, `fileRef`, `flags`, `typeId`.

- `fileRef` is a valid index into `RFIL` in **all** 58,528 entries checked.
- `typeId` is listed in `EXTY` (30,971 entries) or in the `DTTY` table (2,756) in all entries checked.
- `flags` is `1` for 84 % of entries (ordinary resources); `1000` to `1070` identify the concrete shader class of an entry of `Bin/Shaders.module`; entries of type `CEntityClass` (file `Bin/Sam2Game.module`) carry a per-class code (94 codes, `ctsemeta-leftovers.md`, `ctsemeta-entity-codes.md`).
- Reading: "object `objectId` of type `typeId` lives in the referenced file `fileRef`". This is how a file points at other resources (its models, controllers, textures).

### OBTY — object types

`u32 count`, then `count × 8 bytes`: two `u32` (observed in the smallest files as `(0, 0)`, `(1, 5)` …: an object id and its type id — **medium-low**). `OBTY` count equals `OBJS` count in the files inspected.

### EDTY

`u32 count`, then `count × 8 bytes` (two `u32` each). Empty (count 0) in simple files; in all 107 worlds and 72 models/meshes/fonts/animations/skeletons it holds thousands of pairs (a world has 5,183 in one case). **Each pair is `(object id, type id)` of one extra record after `EDOB`, in the same order** (the count equals the `EDOB` count in all 8,063 effective files; for the 60 non-world files with extra records the pairs equal the record headers; `ctsemeta-leftovers.md`): `EDTY` is to the extra records what `OBTY` is to the main objects. The frame is **high** confidence (the entry size is exactly 8 bytes in every file).

### OBJS, EDOB, footer

`OBJS` is followed by a `u32` object count and then the object data (**open**, see below). `EDOB` is followed by `u32 0`. Every file ends with the 8 ASCII bytes `METAEND ` (including a trailing space). In **9,090 files** the last 16 bytes are exactly `EDOB 00 00 00 00 METAEND `; in **179 files** (all 107 worlds, 31 models, 20 meshes, 16 fonts, 4 animation sets, 1 skeleton) more data follows `EDOB` before `METAEND `. For the non-world files this is a list of extra object records, specified in `ctsemeta-objects.md` ("The EDOB section"); for the worlds it is still open.

## Invariants (what a reader must check / may rely on)

1. Magic `CTSEMETA` and marker `0x1234ABCD`, else reject.
2. Format version 8 is the main one; versions 4–7 (155 old `.tex` files) differ only as described in `ctsemeta-leftovers.md`; reject other values.
3. Sections appear in the order above; every count-prefixed section stays inside the file.
4. `RFIL` indices and `IDNT` ids are sequential from 0.
5. The number of `DTTY` blocks equals `INTY`.
6. Every `EXOB.fileRef < RFIL count`.
7. The file ends with `METAEND `.
8. Never trust counts for allocation: cap them by the remaining file size.

## Test vectors

Start of `Content/SeriousSam2/Databases/EntityParams/AutoShotgunWeapon.ep` (measured; header illustration only):

```
43 54 53 45 4d 45 54 41   cd ab 34 12   08 00 00 00   37 00 00 00     "CTSEMETA" marker format=8 L=55
24 56 65 72 73 69 6f 6e 3a 20 ... 52 43 35 24                         "$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5$"
49 4e 46 4f   01 00 00 00 03 00 00 00 01 00 00 00 16 00 00 00 05 00 00 00   "INFO" 1, 3, 1, 22, 5
52 46 49 4c   03 00 00 00 ...                                          "RFIL" count=3 (three referenced files)
```

Tail of the same file: `45 44 4f 42 00 00 00 00 4d 45 54 41 45 4e 44 20` = `EDOB`, 0, `METAEND `.

## Open questions

- Encoding of **object data** inside `OBJS`: **specified in `ctsemeta-objects.md`** (all 9,269 file entries decode to the byte, the worlds included).
- `EXOB.flags` and the `EDTY` pairs are explained, and the old versions 4–7 are specified, in `ctsemeta-leftovers.md` (T01-051). Still open: what `INFO[0]` and the `0x40000` bit of `RFIL.flags` mean to the engine (a reader needs neither); the data after `EDOB` is solved for all files.
- Formats 4–7 (old `.tex` only) → T01-051.
- Whether `IDNT` ids are referenced from object data and how.
