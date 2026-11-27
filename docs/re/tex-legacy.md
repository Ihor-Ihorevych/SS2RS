# Legacy `.tex` pixel storage

- **Roadmap task:** T01-083
- **Author:** @lxstyo/agent · **Reviewer:** _independent spec re-read pending (T01-049)_
- **Confidence:** **high** for the observed layout variants, sizes, format mappings and zlib framing. The individual controls of `CColorTexture` and unobserved animation and palette layouts remain open.

Prerequisites: [CTSEMETA container](ctsemeta-container.md), [type tables](ctsemeta-types.md) and
the version 4–7 frame differences in `ctsemeta-leftovers.md`. The [version 8 texture
spec](tex.md) describes the common pixel and block layouts.

## Summary

The CTSEMETA version selects the container frame. The `CBaseTexture` class-layout version
and its declared member types select the texture encoding. Container version 6 alone contains
three different class layouts. All the retail files expose one baked frame through
`CBaseTexture.18`, including the derived `CColorTexture` that references a source texture.

## Evidence

`python -I tools/re/tex_legacy_verify.py` reads the retail install from `SS2_DATA_DIR` or an explicit
directory. It walks all 155 physical version 4–7 entries in the 2.090 and 2.100 patches, checks
the decompressed or raw pixel size using the rules below, and separately counts the last
case-insensitive copy of each texture path across the installed archives.

| CTSEMETA version | Archive entries | Effective paths |
|---:|---:|---:|
| 4 | 20 | 20 |
| 5 | 4 | 4 |
| 6 | 113 | 87 |
| 7 | 18 | 14 |
| Total | 155 | 125 |

All 155 passed. The corpus has 154 `CStaticTexture` instances and one `CColorTexture`.
Each stands alongside a `CResourceFile`, and every object walk ends at the zero-record EDOB.

For same-path version 8 copies with the same dimensions, pixel layout and face count,
2 BGR, 14 BGRA, 15 DXT1, 3 DXT3 and 12 DXT5 payloads are byte-identical after unwrapping.
The DXT1 matches include the entire six-face cube payload. One further BGRA texture has an
identical base level and carries extra mip levels in its older copy.

## Members and layout selection

The type table gives member order. Version 4 uses the named aliases below in its type table.
Versions 5–7 use the numbered names. The class-layout version is the `kind` word of the
`CBaseTexture` struct block.

| Member | Version 4 name | Type and value |
|---|---|---|
| `.1`, `.2` | `tx_pixWidth`, `tx_pixHeight` | `PIX`, 4-byte width and height |
| `.4` | `tx_ulFlags` | `TexFlags`. Bit `0x02` identifies the one cube map. Bit `0x20` selects zlib in class layout 9 |
| `.5` | `tx_ctFrames` | `INDEX` frame count, always 1 in class layouts 5, 6 and 7. Absent in layout 9 |
| `.6` | `tx_txqDefault` | `TexQuality` in layout 5. `TexFormat` in layouts 6, 7 and 9 |
| `.17` | `tx_txfStored` | `TexStoredFormat` in layout 5. Absent in the other observed layouts |
| `.18` | `tx_aubFrames` | `CStaticArray<UBYTE>`: `STAR`, u32 byte count, then the payload |
| `.8` | `tx_acolFrames` | `CStaticArray<COLOR>` in layout 5. The count is zero in every one of its 72 files. Absent in layouts 6, 7 and 9 |
| `.9`, `.10`, `.14` | `tx_eWrapU`, `tx_eWrapV`, `tx_colBorder` | U/V `GfxTexWrap` codes and the packed border color |

| Class layout | Container versions | How to read `.18` | Entries |
|---:|---|---|---:|
| 5 | 4, 5, 6 | Raw pixel bytes. Select the layout using `.17` | 72 |
| 6 | 6 | u32 compressed byte count followed by one zlib stream. The inner count equals the outer `STAR` count minus four | 63 |
| 7 | 6, 7 | Raw pixel bytes | 3 |
| 9 | 7 | With bit `0x20` set, one zlib stream beginning at byte zero. With the bit clear, raw pixel bytes | 10 zlib, 7 raw |

In layout 5, `.6` records a default quality setting and `.17` describes the stored bytes. For
example, the same stored BGR format occurs with quality codes 0, 2 and 6.
Use `.6` to select the pixel layout in the three layouts that declare it as `TexFormat`.

| Pixel layout | Layout 5 `TexStoredFormat` | Layout 6/7/9 `TexFormat` | Level storage |
|---|---:|---:|---|
| BGR | 0 | 2, 6 | 3 bytes per pixel, base level |
| BGRA | 1 | 1, 4, 5 | 4 bytes per pixel, base level except the layout-7 case below |
| DXT1 / BC1 | 2 | 7 | 8 bytes per 4×4 block, full mip chain |
| DXT3 / BC2 | 3 | 8 | 16 bytes per 4×4 block, full mip chain |
| DXT5 / BC3 | 4 | 9 | 16 bytes per 4×4 block, full mip chain |

Pixels and blocks are packed as described in [version 8](tex.md). Halve both dimensions for
each level, rounding down and clamping to one. For block layouts round each level's block
dimensions up to multiples of four pixels, with at least one block per dimension.
The levels run largest first, with six successive face chains when the cube bit is set.

Layout 7 also has one raw `TexFormat=1` BGRA texture with an entire mip chain. Its 256×128
base level is 131,072 bytes, and all nine levels total 174,764 bytes. The next level is on
average 0.183 byte values per channel away from the corresponding 2×2 averages of the base
level. Three DXT1 thumbnail samples make the same comparison at 2.903–5.909 per channel after
block decoding. For DXT3 and DXT5 the block sizes coincide. The distinction is confirmed by
byte-identical version 8 overlaps and the top-level alpha signatures: 18,637 opaque DXT3
alpha blocks across both code families, and 22,889 opaque DXT5 blocks. Each group showed zero
instances of the other format's opaque signature.

## `CColorTexture`

The sole instance, `Noise_02.tex`, is a version 7, layout 9 texture with flags `0x21` and
`TexFormat=6`. Its `.18` stream is 2,192 bytes. It inflates to 49,152 bytes, a complete 128×128
BGR base level. `CColorTexture.1` is a `Ptr<CTexture>` that uses external object ID 2 to refer to
`Noise_01.tex`. At every corresponding pixel, source gray 0 becomes 115 (7,747 pixels) and
source gray 255 becomes 140 (8,637 pixels). A loader for the stored image can use the baked
base level directly.

The class also stores `.2` (INDEX 0), `.3`–`.6` (FLOAT 1, 0, about 0.1, 1), `.7`
(INDEX 256) and `.8`–`.10` (FLOAT 1 in each field). A single instance gives insufficient
evidence to assign names or a general transform to those controls.

## Invariants

1. Resolve the declared format and class layout from the type table. Read exactly the payload
   bytes declared by `STAR`. Layout 6 also requires a matching inner compressed byte count.
2. Compute the frame size from positive dimensions, the layout, mip count and face count
   before allocating or inflating data. A compressed payload must consume exactly one complete
   stream and yield the exact frame size.
3. The observed frame count is one, and every observed palette array is empty. Preserve the
   distinction between an omitted and an empty field when inspecting the type table.

## Test vectors

- Synthetic raw BGR 2×1, layout 5 with stored format 0 and frame count 1. The payload
  `03 02 01 06 05 04` is two RGB pixels `(1, 2, 3)` and `(4, 5, 6)`.
- Synthetic layout 6 BGRA 1×1. Compress four bytes `03 02 01 FF` with zlib. Store the
  compressed length as a little-endian u32, then the stream. The `STAR` byte count is four
  plus the compressed length. Expected RGBA is `(1, 2, 3, 255)`.
- Size check for the observed layout 7 BGRA mip case. For 256×128, the second level starts
  at 131,072 and the final 1×1 starts at 174,760. The payload ends at 174,764.

## Open questions

The retail corpus has no multi-frame or nonempty palette example. The ordering and sizes
of such frames and the interpretation of a palette-bearing payload remain unverified.
The numbered `CColorTexture` controls need additional evidence if regeneration is required.
