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

### `kind` codes observed (counts are blocks over the whole install)

`0` containers · `1` primitives/aliases and many classes · `2…21` classes (2: 20,658 · 3: 13,179 · 4: 12,754 · 6: 9,326 · 12: 3,813 · 18: 3,066 …) · `25, 27, 28, 29, 31` · `41, 63, 72, 73, 99` (single-digit counts) · `1000` (82). The code does **not** follow a simple "struct vs. primitive" split (kind 1 occurs for classes with member lists too) and its meaning is **not yet understood**; it may be an engine-specific class/category id.

## Worked example (measured, abridged)

`CResourceFile` (id 0): `DTTY 0 "CResourceFile" kind=3 flags=5 parent=-1 STMB 5 { "2"→INDEX(1), "1"→INDEX(1), "5"→INDEX(1), "rf_crsShared"→type 3, "rf_crsExposed"→type 3 }`.
`CResource` (id 4): `kind=4 flags=5 parent=6 STMB 5 { "1"→1, "5"→1, "3"→8, "2"→9, "4"→10 }`.

## Invariants

1. A `DTTY` block is: tag, `typeId`, length-prefixed name, `kind`, then either the short form (2 words), the primitive form (3 words) or `flags, parent, STMB, count, members`.
2. Member counts must be bounded by the remaining file size.
3. After the last block the next tag must be `EXOB`.

## Open questions

- Meaning of `kind`, of `flags` (observed 5 in all examples read) and of the primitive words.
- The second word of short forms for kind 0 (8 in the example) — likely a size/flags value.
- What `ADIM` dimensions attach to which array type (the dimension block sits right after the `n[4]`-style block).
- How object values are laid out and how member types select their encoding → **T01-050**.
- How `kind` and member order relate to the original game classes (`Sam2Game.dll` entity classes) → T01-021/T01-026.
