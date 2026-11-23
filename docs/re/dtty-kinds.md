# `DTTY` kind codes and flags (T01-052)

- **Roadmap task:** T01-052
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the short forms (a closed list, checked on 235,000 blocks), for `flags` (one value everywhere) and for the statement that the struct `kind` **grows with the writer build** (checked on every class that occurs with two kinds); **medium** for the interpretation "class version number" (the data fits it and no other reading, but the engine's own name for the field is unknown); **open** for why a few classes have unusual values (99, 246, 252, 1000).

Prerequisites: `ctsemeta-types.md` (the block frames and short forms), `ctsemeta-container.md`, `ctsemeta-leftovers.md` (older format versions). Reproduce with `tools/re/dtty_verify.py` (read-only, `SS2_DATA_DIR`).

## Summary

The `kind` word of a `DTTY` block has **two different roles**, told apart by the block shape:

| Block shape | Role of `kind` | Values |
|---|---|---|
| **Short** (`u32 a, u32 b` or `u32 a, u32 b, u32 c`) | a **category** of derived or primitive type | `0` = derived type (enumeration, pointer, array, container, handle), `1` = primitive or alias |
| **Struct / class** (`flags`, `parent`, `STMB`, members) | the **version number of the class layout** | `0` to `1000`, 46 distinct values; small for most classes (`1` for 354 classes) and growing with development |

`flags` is **5 in every one of the 235,824 struct blocks** (and absent in short blocks): a constant that a reader can ignore.

**There is no relation to entity or class categories.** The categories of entity classes are given by the `EXOB` codes (`ctsemeta-leftovers.md`, `ctsemeta-entity-codes.md`), not by `kind`.

## Evidence

`tools/re/dtty_verify.py` over the 8,063 effective format-8 files (every type table of every file, 1,043 distinct classes), plus the older formats (versions 4–7) of `ctsemeta-leftovers.md`.

## Short blocks: the complete list of forms

Counted over all type tables. **Exactly ten forms occur**; none other exists in the install. `a` is the first word after `kind` (`b`, `c` the next words).

| Kind | Words | Form | Blocks |
|---:|---|---|---:|
| 0 | `1, size` | enumeration stored in `size` bytes | 44,782 |
| 0 | `2, T` | raw pointer to type `T` | 50,232 |
| 0 | `4, T` | fixed array of `T`; the length is in the type **name** (`n[4]`), not in the words | 7,860 |
| 0 | `6, T` | `CStaticArray<T>` | 66,726 |
| 0 | `7, T` | `CStaticStackArray<T>` | 16,588 |
| 0 | `8, T` | `CDynamicContainer<T>` | 27,593 |
| 0 | `11, T` | `Ptr<T>` (smart pointer) | 49,789 |
| 0 | `12, T` | `Handle<T>` | 8,009 |
| 1 | `0, size, align` | primitive of `size` bytes with alignment `align` | 39,563 |
| 1 | `13, T` | alias of type `T` (`INDEX`, `BOOL`, `COLOR`, `PIX`, …; the target id is per file) | 22,986 |

(The forms and their meaning are those of `ctsemeta-types.md`; this table adds the counts and the statement that the list is closed.) A reader that meets another `(kind, a, length)` combination should treat the file as unsupported: the two-word and three-word shapes above are the only ones in the install.

**Reading of the category.** Kind **0** blocks are *constructed* types, always two words (the constructor code and one argument); kind **1** blocks are *leaf* types (a primitive with a size and an alignment, or a pure alias). Types with `kind 1` and members (the struct role) are a different thing: the struct blocks below also use the value 1.

## Struct blocks: `kind` is a class version

**Observations (all measured):**

1. **`flags` = 5** in 235,824 of 235,824 struct blocks.
2. **`kind` is a property of the class as written by one build**: within one file, every class has one kind; across files, **1,008 of 1,043 classes have the same kind everywhere**, and the other **35 classes have two to four kinds** (`CWorldInfoEntity` and `CMGArrangerLayout` four each, `SSpawnerEntity` and `SPlayerPuppetParams` three).
3. **For the 35 classes with several kinds, the kind never decreases as the writer build number grows** (34 of 35; the one exception, `CFlamerWeaponParams`, has kinds 5 and 4 in two files of the *same* build). Example: `CCharacterTool` has kind 14 in files of build 57284 and kind 17 in files of builds 269486 to 591220 with the same 20 members; `CChapterInfoProperties` 19 then 20; `CEditMesh` 8 then 9; `CEntity` 6 then 7; `CBaseProperties` 3 then 4 (0 members then 1).
4. **The development files of `ctsemeta-leftovers.md` agree**: in version-4 files (builds 2004-05) `CResourceFile` has kind 2 and 3 members, in gold-era files kind 3 and 5 members; `CBaseTexture` has kind 5 with 11 members in version 4 and kind 9 with 8 members in gold: the number rose while members were removed, so it is **not a member count**.
5. **The kind is not derived from the member list**: `CAnimHost` and `CDestructionHolder` keep kind 1 while gaining a member (1 to 2), `CCharacterTool` changes from 14 to 17 with the same member count.
6. **Big classes have big values**: `SPuppetEntity` 246 with 234 members and 252 with 242 members, `SLeggedPuppetEntity` 114, `CNetricsaHudElementParams` 41 (96 members), `CGlobalGameParams` 63 (80 members), `CWorldInfoEntity` 73 (77 members): consistent with a counter that is bumped by the authors whenever the saved layout changes (a class edited often reaches a high number).

**Conclusion.** The struct `kind` is the **version number of the class's serialization layout**: it starts at 1, is raised by the authors when the layout of the class changes, and is stored with the type table so that a file written by an older build stays readable. The reader needs only the file's own member list to decode objects (which is what `ctsemeta-objects.md` does); `kind` is useful for diagnostics ("this file was written with an older `CCharacterTool` layout") and for a writer, which must write a version that matches its member list.

Distribution (blocks and classes per value; a class is counted once for each kind it has):

| `kind` | Blocks | Classes | `kind` | Blocks | Classes |
|---:|---:|---:|---:|---:|---:|
| 1 | 112,014 | 354 | 10 | 847 | 10 |
| 2 | 37,822 | 291 | 11 | 253 | 6 |
| 3 | 29,214 | 114 | 12 | 3,994 | 6 |
| 4 | 15,678 | 70 | 13 | 498 | 12 |
| 5 | 4,602 | 43 | 14 | 1,357 | 8 |
| 6 | 11,715 | 42 | 15–19 | 4,299 | 20 |
| 7 | 6,257 | 33 | 20–34 | 1,123 | 21 |
| 8 | 2,493 | 24 | 41–252 | 226 | 12 |
| 9 | 3,309 | 15 | 0, 99, 1000 | 123 | 3 |

### Unusual values

| Class | `kind` | Where | Note |
|---|---:|---|---|
| `CMoveCharacter` | **1000** | 120 blocks in 73 `.cb`, 9 `.ep`, 38 `.wld` | no members; a reserved large number |
| `SCircularSawBladesWeaponParams` | **99** | 2 `.ep` | no members |
| `CFlamerProjectileParams` | **0** | 1 `.ep` | no members; the only struct block with kind 0 |
| `SPuppetEntity` | **246** and **252** | 14 and 31 `.wld` | two layouts of the biggest class (234 and 242 members) |
| `SLeggedPuppetEntity` | **114** | 44 `.wld` | derived from `SPuppetEntity` |

## What a reader does

1. Decode objects by the file's own member lists; ignore `kind` and `flags` for decoding (as `ctsemeta-objects.md` already does).
2. Accept any `kind` value for a struct block; keep it when rewriting a file.
3. For short blocks accept only the ten forms above; any other combination means an unsupported file.
4. A class that has several layouts (different member lists) in different files is normal: the table of each file is authoritative (`ctsemeta-types.md`).

## What is not in the data

1. The engine's name and the exact bump rules of the version field, and why a few classes carry round numbers (99, 1000).
2. The meaning of the bits of `flags = 5` (bit 0 and bit 2 are set; no file has any other value).
3. Whether the version numbers carry any compatibility logic in the engine (a loader migrating old layouts): all 9,269 files decode from their own tables alone, so no such logic is needed to read the install.

## Test vectors

- `CResourceFile` in a gold-era file: `DTTY 0 "CResourceFile" kind=3 flags=5 parent=-1 STMB 5 …`; in the version-4 file `Textures/Levels/01a_Aisha/Trim_Horizontal_01.tex`: `kind=2 flags=5 parent=-1`, 3 members (`rf_iNextFreeID`, `rf_iDefaultID`, `rf_crsShared`).
- `INDEX` (alias): `kind=1`, words `13, 2` (alias of the `SLONG` of the file's table); `SLONG`: `kind=1`, words `0, 4, 4`; `CDynamicContainer<CResource>`: `kind=0`, words `8, 4`.
- Counts: 235,824 struct blocks all with `flags` 5; ten short forms (counts above); 1,043 classes, 35 with more than one kind, 34 of them never decreasing with the writer build.
