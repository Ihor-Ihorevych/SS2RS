# CTSEMETA object data (`OBJS`)

- **Roadmap tasks:** T01-050a (framing, primitives, class instances), T01-050b (containers and arrays), T01-050c (references, strings)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet — needs an independent re-read (T01-049) and an independent walker (T01-050d)_
- **Confidence:** **high** for everything below except where marked. Evidence: a decoder built from exactly these rules reads the object data of **9,162 of 9,269** format-8 files and ends on the `EDOB` marker **to the byte** (see "Evidence"). The 107 world files (`.wld`) are not yet decodable; see "Open questions".

Prerequisites: `ctsemeta-container.md` (file frame) and `ctsemeta-types.md` (type table). Everything here is little-endian. There is **no padding or alignment anywhere**: values follow each other byte for byte.

## Summary

The type table says what a class looks like; `OBJS` contains the instances. Each instance is serialized by writing its members **flat, base class first, in the order the type table lists them**, with no field names, no sizes and no padding. Because the type table travels in the file, a generic reader needs no knowledge of the game classes.

## `OBJS` framing

```
"OBJS"  u32 count                      (equal to the OBTY count)
count × object record                  (in the same order as the OBTY entries)
"EDOB"  u32 0  [extra data in 72 files] "METAEND "
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
| `IDENT` | a plain 4-byte value: an index into the file's `IDNT` table (0 = the empty identifier) |
| `INDEX` | a 4-byte signed value (−1 = none) |
| external primitive used by id only (`EXTY` entries `UBYTE`, `UWORD`, `ULONG`, `SLONG`, `FLOAT`, …) | sized as the primitive of that name (1/2/4 bytes) |

An object id is valid if it appears in `OBTY` (record present here) or in `EXOB` (external).

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

- **9,090** files decode exactly to `EDOB`, `0`, `METAEND `.
- **72** files (31 `.mdl`, 20 `.bmf`, 16 `.fnt`, 4 `.ans`, 1 `.skl`) decode exactly to an `EDOB` that is followed by a non-zero `u32` and extra data (see open questions).
- **107** files, all the `.wld` worlds, do not decode yet.
- In files under 2 MB, 7,995 of 7,996 decode.

## Open questions (follow-ups)

- **Worlds (`.wld`, 107 files):** objects of world-only classes fail: `CEntryList` (a `CStaticStackArray<CSMEntry>` that meets zeros where `SSAR` is expected, in 11 files), and in the large worlds the cursor drifts by one word before an object header. Suspects: `CMetaHandle` and `Time`/state-manager members whose by-value encoding is not what the type table implies. → **T01-053**.
- **Data after `EDOB` (72 files):** the `u32` following `EDOB` is non-zero; it looks like a count followed by entries or blobs (mesh/animation payload). → **T01-051**.
- What the per-file `ADIM` blocks are for (fixed-array counts come from the type *name*; no other use was needed).
- Meaning of the `kind` codes and `flags` words (T01-052) and of `INFO[0]`.
- Whether `UUUU` (seen in 43 `.tex` files) is a payload marker.
- An **independent walker** written only from this document (T01-050d) is still needed to prove the text is complete; the author's decoder is not independent evidence.
