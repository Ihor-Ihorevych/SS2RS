# CTSEMETA type table (`DTTY` blocks)

- **Roadmap task:** T01-005 (partial — see "Open questions"; the rest is T01-050 and T01-052)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the block framing and the member list; **medium** for the names of fields; **low** for the semantics of the category codes.

## Summary

Each CTSEMETA file defines, inside the file, the data types its objects use. **The format is self-describing**: a reader can walk the type table without knowing any game classes, learn each type's name, its parent type and its members (name + type), and use that to decode objects. This document specifies the table's framing; how object *values* are encoded is T01-050.

Evidence: a throw-away parser built from the rules below read the type table of **9,269 of 9,269** format-8 files and landed exactly on the following `EXOB` tag every time (see `ctsemeta-container.md`).

## Where the table lives

After `INTY`: a sequence of `DTTY` blocks, with `ADIM` blocks (`ADIM`, `u32 n`, `n × u32`) interleaved; it ends at `EXOB`. The number of `DTTY` blocks equals the value after `INTY`.

## `DTTY` block

```
"DTTY"
u32 typeId
u32 nameLen, nameLen bytes name
u32 kind               (the "category" code, see below)
… payload — one of three shapes, chosen by what follows …
```

`typeId` values are unique in the file and **share one id space with the external types** listed in `EXTY` (type references from members, parents and `EXOB` entries may point at either table). Names are engine class names (`CResourceFile`, `CMenuScreenLayout`, `SHudBarConfig`), primitive names (`SLONG`, `INDEX`), template instances (`CDynamicContainer<CResource>`) and fixed arrays (`n[4]`).

### Payload shapes

After `kind`, the next bytes take one of three shapes (distinguished by whether the 4 bytes at *kind + 12* are `STMB`):

| Shape | Bytes after `kind` | Observed for |
|---|---|---|
| **Struct/class** | `u32 flags`, `i32 parent`, `"STMB"`, member list (below) | every kind ≥ 2 and about half of kind 1; roughly 40 % of all block occurrences in the install |
| **Short** | `u32 a`, `u32 b` | kind 0 (e.g. `CDynamicContainer<CResource>` = kind `0`, then `8, 4`: `b` is the element type id) and kind-1 aliases (`INDEX` = kind `1`, then `13, 2`: `b` is the underlying type id) |
| **Primitive** | `u32 a`, `u32 b`, `u32 c` | kind 1 primitives (e.g. `SLONG` = kind `1`, then `0, 4, 4`: the last two words look like size and alignment) |

Block occurrences counted across all files (each file repeats its own table): 141,532 short blocks of kind 0; 15,702 short alias blocks and 35,946 primitive blocks of kind 1; about 130,000 struct/class blocks. The shape is decided only by whether the four bytes 12 bytes after `kind` are `STMB`; if not, the block runs until the next `DTTY`, `ADIM` or `EXOB` tag.

`parent` is the type id of the base class, or `-1` (`0xFFFFFFFF`) for none. Example: `CResourceFile` has parent `-1`; `CMenuScreenLayout` has parent 4 = `CResource`.

### `STMB` — member list

```
"STMB"
u32 count
count × { u32 nameLen, nameLen bytes name, u32 typeId }
```

The member name is often a decimal number written as text (`"1"`, `"2"`, `"5"`): members are identified by number and only some carry a real name (`rf_crsShared`, `rf_crsExposed`). The `typeId` refers into the shared type-id space.

### Short forms (no member list)

Verified by the object-data decoder (`ctsemeta-objects.md`):

| `kind` | words after `kind` | Meaning |
|---|---|---|
| 1 | `0, size, align` | primitive of `size` bytes (`FLOAT`, `SLONG`, `ULONG`, `IDENT`, `UBYTE`, `UWORD`, `SQUAD`, `CString` …) |
| 1 | `13, targetTypeId` | alias of another type (`INDEX`, `BOOL`, `COLOR`, `PIX`); the target id is per file |
| 0 | `1, size` | enumeration stored in `size` bytes |
| 0 | `2, X` | raw pointer `X*` |
| 0 | `4, elementTypeId` | fixed array `X[N]`; **N is in the type name**, not in the words |
| 0 | `6, T` | `CStaticArray<T>` |
| 0 | `7, T` | `CStaticStackArray<T>` |
| 0 | `8, T` | `CDynamicContainer<T>` |
| 0 | `11, X` | `Ptr<X>` |
| 0 | `12, X` | `Handle<X>` |

### `kind` codes of struct/class blocks

Observed values (block counts over the install): `1` (60,854 with members, e.g. `CFont`) · `2` (20,658) · `3` (13,179) · `4` (12,754) · `6` (9,326) · `12` (3,813) · `18` (3,066) · further values up to `99`, and `1000`. The code does **not** separate structs from primitives. **Explained in `dtty-kinds.md` (T01-052): for struct blocks `kind` is the version number of the class layout** (it grows with the writer build; `flags` is always 5), and for short blocks it is a category (0 constructed type, 1 primitive or alias); object decoding does not need it.

## Worked example (measured, abridged)

`CResourceFile` (id 0): `DTTY 0 "CResourceFile" kind=3 flags=5 parent=-1 STMB 5 { "2"→INDEX(1), "1"→INDEX(1), "5"→INDEX(1), "rf_crsShared"→type 3, "rf_crsExposed"→type 3 }`.
`CResource` (id 4): `kind=4 flags=5 parent=6 STMB 5 { "1"→1, "5"→1, "3"→8, "2"→9, "4"→10 }`.

## Invariants

1. A `DTTY` block is: tag, `typeId`, length-prefixed name, `kind`, then either the short form (2 words), the primitive form (3 words) or `flags, parent, STMB, count, members`.
2. Member counts must be bounded by the remaining file size.
3. After the last block the next tag must be `EXOB`.
4. **An `STMB` member list is not guaranteed to have distinct names.** `CBloomProperties` declares a member literally named `"21"` twice (T01-424); T01-425 swept every class's own `STMB` list across the whole install (`tools/re/ctse_duplicate_members_verify.py`, data, high confidence, all extensions and patches merged) and found five more: `CAllMovingAnimation` (`"7"` twice), `CDirectionalLightProperties` (`"12"` twice), `CEditAnimation` (`"9"` twice), `SCameraEntity` (`"24"` twice) and `SScannerHudElementParams` (`"9"` twice) — six classes total, each with exactly one repeated name, out of 1,043 classes with members over 8,063 CTSEMETA files. A reader must always resolve a member by its **file position**, never by looking its name up in a dict keyed by that name alone, or two unrelated fields silently collapse into one.

## Open questions

- Meaning of the struct `kind` codes and of `flags` (always 5): answered in `dtty-kinds.md` (T01-052); the bits of `flags` remain unexplained.
- What the `ADIM` blocks are for: fixed-array counts are taken from the type name, and decoding needed nothing else.
- Object value layout: **specified in `ctsemeta-objects.md`** (T01-050a/b/c).
- How `kind` and member order relate to the original game classes (`Sam2Game.dll` entity classes) → T01-021/T01-026.
- **Five generated schema docs have the same by-name-merge bug `environment-schema.md` had for `CBloomProperties.21` (T01-424), now also confirmed for the five classes invariant 4 lists** (T01-425): `wld-entity-schema.md` (`CDirectionalLightProperties`, `SCameraEntity`), `misc-schema.md` (`CAllMovingAnimation`) and `rsc-ctl-schema.md` (`SScannerHudElementParams`) each already print **two separate rows** for the repeated member (correctly split by file position, with the right type per row), but the **observed value/range text is identical in both rows** — pooled from both positions rather than computed separately for each — because the two generator scripts behind all of this project's `*-schema.md` docs (`tools/re/wld_entity_schema.py`'s `collect()`/`agg[k]`, `tools/re/ep_schema.py`'s `collect()`/`stats[k]`) key their per-member statistics by the name string alone (`f"{cls}.{m}"`), the same shared root cause, not independently re-derived each time. `CEditAnimation` has no per-member value table in any generated doc yet, so nothing to flag for it. Fixing both scripts to key statistics by `(class, name, position)` and regenerating the ten docs they produce (`environment-schema.md`, `ep-schema.md`, `misc-schema.md`, `pfx-schema.md`, `rsc-ctl-schema.md`, `wld-entity-schema.md`, `wld-macro-schema.md`, `wld-navigation-schema.md`, `wld-physics-schema.md`, `wld-sound-effects-schema.md`) is a follow-up task, appended to `docs/roadmap/T01-reverse-engineering.md`, not done by this task (out of its own `S` budget).
