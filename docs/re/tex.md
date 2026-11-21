# `.tex` textures

- **Roadmap task:** T01-006
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the class layout, the pixel-data encoding (zlib + raw layouts), mip/cube arrangement, channel order and orientation; **medium** for the cube face order and for the meaning of `TexFlags`/`GfxTexWrap` bits; **low** for the meaning of the format *codes* that share a layout. Scope: format-8 files (2,610 textures); the 155 older-format files (4–7) are not covered (T01-051).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`.

## Summary

A texture is a CTSEMETA file with exactly two objects: a `CResourceFile` (object 0) and a `CStaticTexture` (object 1). The texture holds its width, height, a format code, flags, wrap modes and **one `zlib`-compressed byte array with all the pixel data**. Inflating it yields raw pixels in one of three layouts: 24-bit BGR, 32-bit BGRA or DXT1/DXT3/DXT5 blocks (with a complete mip chain). Cube maps are six DXT1 faces back to back.

## Evidence

`tools/re/tex_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches:

- **2,610 / 2,610** format-8 textures decode; every one has the object classes `[CResourceFile, CStaticTexture]` and no `RFIL` entries.
- For **every** texture the inflated size equals the size predicted by the rules below (0 mismatches): 434 24-bit, 404 32-bit, 1,208 DXT1, 32 DXT1 cube maps, 243 DXT3, 289 DXT5.
- Channel order and orientation were confirmed visually: `Textures/Menu/Background_Blue_PC.tex` is a blue picture only when read as BGR; `Textures/Interface/Hud/netricsa.tex` (the blue HUD avatar) only as BGRA; both upright with rows stored top to bottom.
- DXT3 vs DXT5 was settled by alpha-block statistics (below). Mip order was checked numerically (below).

## The `CStaticTexture` object

Member order, flat, base class first (the `type table` of each file is authoritative; a reader must use it, see `ctsemeta-objects.md`):

| Class.member | Type | Meaning / observed values |
|---|---|---|
| `CResource.1` | `INDEX` | always 1 |
| `CResource.5` | `INDEX` | −1 (absent from the class definition in 31 files) |
| `CResource.3` | `IDENT` | 0 (empty identifier) |
| `CResource.2` | `CResourceFile*` | object 0 |
| `CBaseTexture.1` | `PIX` (4-byte signed) | **width** in pixels, 1…4096 |
| `CBaseTexture.2` | `PIX` | **height** in pixels, 1…2048 |
| `CBaseTexture.4` | `TexFlags` (4-byte enum) | flags, see below |
| `CBaseTexture.6` | `TexFormat` (4-byte enum) | **format code** 0–9, 11 |
| `CBaseTexture.18` | `CStaticArray<UBYTE>` | **the pixel blob**: `STAR`, byte count, then a zlib stream |
| `CBaseTexture.9` | `GfxTexWrap` (4-byte enum) | wrap mode, U |
| `CBaseTexture.10` | `GfxTexWrap` | wrap mode, V |
| `CBaseTexture.14` | `COLOR` (4 bytes) | 0 in all but one file (`0x808080`); probably the border colour |

All widths and heights are powers of two; 758 textures are non-square, with aspect ratios from 2:1 up to 256:1 (thin strips such as `netricsa_menu_tile.tex`).

## Pixel data

The blob is a standard **zlib stream** (starts `78 9C`; all 2,610 inflate with plain zlib). The inflated bytes are tightly packed: **no row padding, no header**. Rows run **top to bottom**; pixels left to right.

| Format code | Layout | Inflated size |
|---|---|---|
| 0, 2, 6 | **24-bit BGR** (byte order B, G, R), one level | `w·h·3` |
| 1, 3, 4, 5, 11 | **32-bit BGRA** (B, G, R, A), one level | `w·h·4` |
| 7 | **DXT1** (BC1, 8 bytes per 4×4 block) with full mip chain | `Σ levels` |
| 8 | **DXT3** (BC2, 16 bytes per block, explicit 4-bit alpha) with full mip chain | `Σ levels` |
| 9 | **DXT5** (BC3, 16 bytes per block, interpolated alpha) with full mip chain | `Σ levels` |

For the DXT formats a level of size `w × h` occupies `max(1, ⌈w/4⌉) · max(1, ⌈h/4⌉) · blockBytes` bytes. The **mip chain** has every level from `w × h` down to `1 × 1` (each dimension halved, minimum 1), **largest first, each level immediately after the previous one** (checked: decoding the data after level 0 as the half-size image matches the 2×2 average of level 0 to within about 1–4 of 255). Uncompressed formats (24/32-bit) store **only level 0**; the engine must generate the lower levels itself.

DXT3 vs DXT5 (measured in the top level of every texture of those formats): format 8 has 21.6 % of blocks whose alpha half is `FF FF FF FF FF FF FF FF` (opaque DXT3) and none of the DXT5 opaque pattern; format 9 has 42.3 % blocks starting `FF FF 00 00 00 00 00 00` (opaque DXT5) and none of the DXT3 pattern.

### Cube maps

Flag bit `0x02` (set in exactly the 32 textures with flags `0x23`, all format 7) marks a **cube map**: the inflated data is **six faces**, each a complete DXT1 mip chain, one after another (size `6 · Σ levels`; for 128×128: 6 × 10,936 = 65,616). Face order is consistent with the Direct3D order **+X, −X, +Y, −Y, +Z, −Z** (in `TempleGDC2.tex`, face 2 shows a bright light seen from above and face 3 is dark floor) — medium confidence.

## Flags, format codes and wrap modes (observed, not defined)

- `TexFlags` observed values: `0x21` (1,837), `0x20` (551), `0x24` (164), `0x23` (32 cube maps), `0x25` (23), `0x71` (2), `0x60` (1). Bit `0x20` is always set; **bit `0x02` = cube map** (verified by the sizes). `0x24` and `0x25` are mostly user-interface textures (79 % and 87 % of them live under `textures/interface` or `textures/menu`, against 5–6 % for `0x21` and `0x20`); `0x21` is the common world/effects value. Meanings of `0x01`, `0x04`, `0x10`, `0x40` are **not established**.
- `GfxTexWrap` (the pairs U/V observed: `0x50/0x50` 2,092 · `0x51/0x51` 467 · `0x50/0x51` 25 · `0x51/0x50` 9 · `0x52/0x52` 8 · `0x50/0x52` 6 · `0x52/0x50` 3): `0x50` is the default, `0x51` is common in Interface/Menu/HUD textures, `0x52` (17 textures use it on at least one axis: flares, waterfalls, windows). A plausible reading is repeat / clamp-to-edge / clamp-to-border (the only non-zero `COLOR` is on a `0x52` texture), **unverified**. Normal maps are always `0x50`.
- Format codes that share a layout differ in intended use, not in data: code 11 is used for alpha masks (`Alpha_01`, `Mask`), codes 2 and 6 for effect gradients and flares, codes 1/4/5 for interface graphics. Their exact meaning is **unknown**; a loader needs only the layout column above.

## Invariants for a reader

1. Read width, height, format code, flags from the object; reject sizes that are not positive.
2. Inflate the blob; the inflated length must equal the size predicted for the format code (and cube flag); otherwise fail.
3. Never allocate from the declared size without bounding it (4096 × 2048 × 4 × 6 bytes is the largest sane case).
4. Do not assume a `.tex` has mips: only DXT textures store them.

## Test vector

`Content/SeriousSam2/Textures/Background/PureWhite/WhiteBckg_001.tex` (1,245 bytes): width 16, height 16, flags `0x20`, format code 0, wrap `0x50`/`0x50`, `COLOR` 0, pixel blob (15 bytes) `78 9C FB FF 7F 14 8C 82 91 0B 00 44 D3 FD 1F`, which inflates to **768 bytes, all `0xFF`** (16 × 16 × 3: a white 24-bit image).

## Open questions

- Formats 4–7 (155 files) → T01-051.
- Exact meaning of the format codes within a layout group, of `TexFlags` bits other than `0x02`, and of the `GfxTexWrap` values → T01-054 (needs the material/shader specs and how the engine uses them).
- Gamma/sRGB treatment (no flag was identified for it).
- How the 31 files whose `CResource` lacks member 5 differ (probably a newer class version).
