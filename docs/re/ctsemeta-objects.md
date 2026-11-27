# CTSEMETA object data (`OBJS`)

- **Roadmap tasks:** T01-050a (framing, primitives, class instances), T01-050b (containers and arrays), T01-050c (references, strings), T01-050d (independent object walk)
- **Author:** @Ihor-Ihorevych/agent · **Independent walker:** @lxstyo/agent, 2026-11-25 · **Reviewer:** _independent spec re-read pending (T01-049)_
- **Confidence:** **high** for everything below except where marked. Evidence: a decoder built from exactly these rules reads the object data of **all 9,269** format-8 file entries (every `.wld` world included) and ends on the `EDOB` marker **to the byte**, and the extra records after `EDOB` end on `METAEND ` (see "Evidence").

Prerequisites: `ctsemeta-container.md` (file frame) and `ctsemeta-types.md` (type table). Everything here is little-endian. There is **no padding or alignment anywhere**: values follow each other byte for byte.

## Summary

The type table says what a class looks like; `OBJS` contains the instances. Each instance is serialized by writing its members **flat, base class first, in the order the type table lists them**, with no field names, no sizes and no padding. Because the type table travels in the file, a generic reader needs no knowledge of the game classes.

## `OBJS` framing

```
"OBJS"  u32 count                      (equal to the OBTY count)
count × object record                  (in the same order as the OBTY entries)
"EDOB"  u32 count  count × extra record  "METAEND "     (count is 0 in 9,090 files; see "The EDOB section")
```

An **object record** is:

```
u32 objectId
u32 typeId                             (the same pair as the matching OBTY entry; check it)
member data of type typeId             (see below)
```

- Object ids are the ids used in `OBTY` and `EXOB`; both tables share one id space. Objects listed in `EXOB` are **external**: they live in another file (`RFIL`) and have **no record** in this file. Everything else in the id space has a record here.
- There is no per-object length: you must decode member by member to find where the next record starts. After the last record the cursor must be exactly at the `EDOB` tag.

## Encoding of a value of type `T`

Look `T` up in the shared type-id space (`DTTY`/`EXTY`) and choose by its shape (see `ctsemeta-types.md`):

### Struct or class (`DTTY` with an `STMB` member list)

1. Build the **chain**: follow `parent` until `-1`; write the **oldest ancestor's members first**, then each descendant's, ending with `T`'s own members.
2. Encode each member in that order as a value of the member's type, one after another.

Classes with an empty member list (such as `CObject`, `CSmartObject`, `CEntityParams`) occupy 0 bytes. A struct stored **by value** (for example `Vector2f` inside another class) is encoded exactly the same way, inline.

Worked example (a real 68-byte object region from `…/Items/Ammo/AmmoPackItem.ep`, hand-annotated):

| Word | Value | Meaning |
|---:|---|---|
| 0–1 | `0, 0` | record header: objectId 0, typeId 0 (`CResourceFile`) |
| 2–4 | `1, 2, 0xFFFFFFFF` | `CResourceFile` members `"2":INDEX`, `"1":INDEX`, `"5":INDEX` (−1 = none) |
| 5–7 | `DCON, 1, 1` | `rf_crsShared`: container with 1 element: object id 1 |
| 8–9 | `DCON, 0` | `rf_crsExposed`: empty container |
| 10–11 | `1, 5` | record header: objectId 1, typeId 5 (`CBaseItemParams`) |
| 12–15 | `1, 0xFFFFFFFF, 0, 0` | base `CResource` members: `INDEX`, `INDEX`, `IDENT` (0 = empty identifier), `CResourceFile*` = object 0 |
| 16 | `0x41F00000` | `CBaseItemParams` member `"1":FLOAT` = 30.0 |

## Primitive and short types

| Type shape (from `DTTY`) | Encoding |
|---|---|
| kind 1, three words `[0, size, align]` (e.g. `FLOAT`, `SLONG`, `ULONG`, `IDENT`, `UBYTE`, `UWORD`, `SQUAD`) | `size` raw bytes (1, 2, 4 or 8), little-endian; `FLOAT` is IEEE-754 binary32 |
| kind 1, two words `[13, targetTypeId]` (aliases: `INDEX`, `BOOL`, `COLOR`, `PIX`) | encode as the **target** type (resolve per file: the target id differs between files) |
| kind 0, category 1 `[1, size]` (enumerations) | `size` raw bytes (1, 2 or 4) |
| kind 0, category 2 / 11 / 12 `[cat, X]` — raw pointer `X*`, `Ptr<X>`, `Handle<X>` | **one `u32` object id**; `0xFFFFFFFF` = null |
| `CMetaHandle` (a class **declared without members**) | **one `u32` object id**; `0xFFFFFFFF` = null. The type table gives no hint of these 4 bytes: a generic reader that follows the empty member list reads 0 bytes and loses alignment. See below |
| `IDENT` | a plain 4-byte value: an index into the file's `IDNT` table (0 = the empty identifier) |
| `INDEX` | a 4-byte signed value (−1 = none) |
| external primitive used by id only (`EXTY` entries `UBYTE`, `UWORD`, `ULONG`, `SLONG`, `FLOAT`, …) | sized as the primitive of that name (1/2/4 bytes) |

An object id is valid if it appears in `OBTY` (record present here) or in `EXOB` (external); a `CMetaHandle` may also name an object of the extra records after `EDOB`.

### `CMetaHandle` (the world rule)

`CMetaHandle` is a class whose type-table entry has **no members** (and no base class), but a by-value instance occupies **4 bytes**: an object id, exactly like a `Handle<X>`. It occurs in worlds in three places: member `.1` of `CVariantImpHANDLE`, the elements of a `CStaticStackArray<CMetaHandle>` (inline `SSAR` of 4-byte values), and member `.1` of `CSMEntry`. This one rule is what stopped the 107 worlds from decoding: one missing word shifted every later object header, and the other suspects of the first analysis (`CEntryList`, `Time`, the state manager) were only downstream effects. In `dm_dockingbay.wld` 56 of 57 values are ids of objects of the file and one is null.

Other classes with an empty member list (`CStaticSoundFeeder`, `CSLODCache`, 2,864 by-value instances in all) **do** occupy 0 bytes: alignment holds in every file when they are read as empty. So the empty-member-list shortcut is correct for them and wrong only for `CMetaHandle`; a reader must carry this one exception.

## Strings

- `CString`: `u32 length`, then `length` raw bytes. No terminator, no padding. Length 0 is the empty string.
- `CTransString` (a translatable text): the 4 ASCII bytes `DTRS`, then two `CString`s (a lookup key such as `ChapterList.lblCaption.Name`, then the text, usually empty in the key-only form). `DTRS` is a marker written **instead of** any length; do not read it as a string.

## Containers and arrays

A **container** starts with a 4-byte ASCII marker, then a `u32` element count, then the elements. The marker is determined by the container type's category (`kind 0`, first word):

| Category | Type family | Marker | Elements |
|---:|---|---|---|
| 8 | `CDynamicContainer<T>` | `DCON` | `count` × **`u32` object id** (the container owns/refers to objects; the objects have their own records) |
| 6 | `CStaticArray<T>` | `STAR` | `count` × **value of `T` inline** |
| 7 | `CStaticStackArray<T>` | `SSAR` | `count` × **value of `T` inline** |

A **fixed array** (category 4, name `X[N]`, words `[4, elementTypeId]`) has **no marker and no count**: it is `N` elements of the element type written inline, where `N` is the number in square brackets at the end of the **type name** (the word after `4` is the element type id, not the count).

Marker usage measured in the 9,162 files that decode: `STAR` 165,678 · `DCON` 45,373 · `SSAR` 32,121 · `DTRS` 8,283. The marker always matched the category above where the data was aligned.

## Invariants for a reader

1. For each object record: `(objectId, typeId)` equals the `OBTY` entry at that position.
2. A container marker must be one of `STAR`/`SSAR`/`DCON` and must match the type's category; a mismatch means the reader lost alignment.
3. Decoding all records must end exactly at the `EDOB` tag; anything else is a reader bug or a corrupt file.
4. Never trust counts or lengths for allocation: cap by the bytes remaining in the file.
5. Unknown type shapes must make the file fail loudly, not be guessed.

## Evidence

`tools/re/walk_objects.py` implements these rules (read-only; `SS2_DATA_DIR`). On the retail install with all patches:

- **9,090** file entries decode exactly to `EDOB`, `0`, `METAEND `.
- **179** file entries (107 `.wld` worlds, 31 `.mdl`, 20 `.bmf`, 16 `.fnt`, 4 `.ans`, 1 `.skl`) have a non-zero count after `EDOB`; their extra records decode exactly to `METAEND ` (see "The EDOB section").
- That is **9,269 of 9,269**: no file fails. (The walk takes about 11 minutes: it reads every archive entry, also the ones a later patch overrides.) The distinct paths are 59 worlds and 60 other files with extra records; `tools/re/wld_verify.py` checks the worlds one by one.
- Markers met while decoding: 420,586 `DCON`, 548,328 `SSAR`, 283,556 `STAR`, 11,000 `DTRS`.

### Independent OBJS verification (T01-050d)

The standard-library script `tools/re/ctsemeta_independent_verify.py` was written from this
specification and its frame and type-table prerequisites. It checks each record header against
`OBTY`, advances over each serialized value, and checks that the computed cursor lands exactly on
`EDOB`, zero, and `METAEND `. An empty inline array has no element bytes. Its element type can be
external even when the file supplies no inline layout for that type.

On the 14 retail archives, counting each entry including earlier copies replaced by patches:

| Result | Count |
|---|---:|
| Archive file entries | 12,288 |
| Version 8 CTSEMETA entries | 9,269 |
| Version 8 entries with nonempty `EDTY`, excluded from the walk | 179 |
| Version 8 entries walked exactly to zero-record `EDOB` | 9,090 |
| Main records walked in those 9,090 entries | 178,519 |
| Failures by format or object type | 0 |
| Older CTSEMETA entries, excluded | 155 |

Run `python -I tools/re/ctsemeta_independent_verify.py` with `SS2_DATA_DIR` set or pass the
installation directory as an argument. The script groups any failures by the current `OBTY`
type or by framing when no current object applies. The command succeeds only after matching
the retail archive, version 8 entry, and walked-entry counts above with zero failures.
This walk verifies byte boundaries and
record ordering. Extra records and the validity of individual reference targets were outside its scope.

## The `EDOB` section

After the last object record comes the tag `EDOB`, a `u32 count`, and `count` **extra records**, then the footer `METAEND `. An extra record has the same encoding as an `OBJS` record: `u32 object id`, `u32 type id`, then the value of that type by the rules above. The ids of extra records are **not** in `OBTY` and never clash with it (checked). After the last extra record the cursor is exactly on `METAEND `.

Evidence (`tools/re/edob_verify.py`): of **60 distinct non-world files** with a non-zero count (29 `.mdl`, 18 `.bmf`, 8 `.fnt`, 4 `.ans`, 1 `.skl`), **all 60** decode exactly to the footer. The records are mostly editor-side data (`CEditMesh`, `CPackedEditMesh`, `CEditFont`, `CEditSkeleton`, `CEditAnimation`, `CEditChannel`, `CEditEnvelope`, transfer maps, `CWeightVertexMap`, `CEdtDestructionEffect`), with some classes that look like game data (shader presets and graph modifiers, animation event holders, `CStaticTexture`, debris effects). Main-section objects of these files do refer to extra records, through nine edit-data pointer members (`CResource.4`, `CModelConfiguration.13`, `CMesh.3`, `CAnimHost.2`, `CDestructionHolder.2`, `CFont.10`, `CSkeleton.2`, `CMechanismTemplateHolder.2`, `CAnimSet.2`): the list, counts and the class table per container are in `ctsemeta-leftovers.md`; for `.bmf` surface materials none does (`bmf.md`).

**Worlds.** All 59 distinct worlds have extra records: 229,037 in total, which decode exactly to `METAEND ` with no id clashing with the 824,942 main objects (`tools/re/wld_verify.py`). They are editor and bake data: the most frequent classes are `CTextureBakerLightMap` (37,293), `CStaticModelProperties` (25,383), `CConstantShaderModifier` (25,056), `CAmbientMaskCache` (18,688), `CTextureOvenShaderModifier` (18,680), `CPointLightProperties` (11,349) and `SPathMarkerProperties` (10,639). Main-section objects of a world refer to them: of 23,761 `CMetaHandle` values, 19,727 are ids of objects of the file (main or extra), 4,034 are null and none is anything else, so **a world reader must keep the extra records** (unlike the model formats, where they are editor data only). Which of the handles point into the extra records is the business of the world spec (T01-012).

## Open questions (follow-ups)

- **Worlds:** the objects of all worlds now decode (the `CMetaHandle` rule above). What the world classes mean is the subject of the world spec (T01-012).
- **Data after `EDOB`:** solved for all files, worlds included (see "The EDOB section"). The meaning of the individual extra classes is left to the spec of the format they belong to.
- What the per-file `ADIM` blocks are for (fixed-array counts come from the type *name*; no other use was needed).
- Meaning of the `kind` codes and `flags` words: `dtty-kinds.md` (T01-052; the struct `kind` is a class version). `INFO[0]`, `RFIL.flags`, `EXOB.flags`, `EDTY` and the old versions 4–7: `ctsemeta-leftovers.md` (T01-051).
- Whether `UUUU` (seen in 43 `.tex` files) is a payload marker.
