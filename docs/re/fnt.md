# `.fnt` bitmap fonts

- **Roadmap task:** T01-018
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the structure, the glyph rectangles, the code table and the way text is laid out (checked on all 21 fonts, and by rendering test strings with the decoded data); **medium** for the glyph members `.5` (x offset) and `.6`/`.8` (always 0 here), and for the header members `.1`/`.2`. Scope: all **21** distinct format-8 `.fnt` files of the retail install with patches (13 of them are game fonts, 8 belong to the editor GUI).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`, `tex.md` (the atlas is a `CStaticTexture`).

## Summary

A font is a **bitmap atlas** plus a table of glyph rectangles. The file holds three objects: `CResourceFile`, one `CFont` and one `CStaticTexture` (the atlas, stored inside the font). Glyphs are not rendered from outlines: the engine copies rectangles of the atlas. A **code table** maps a character code to a glyph index; glyph 0 is the fallback for unmapped codes. Eight of the fonts carry editor records after `EDOB` (the source description from which the atlas was baked).

## Evidence

`tools/re/fnt_verify.py` (read-only, `SS2_DATA_DIR`; `--png DIR` renders a test string with every font) over the retail install with all patches:

- **All 21 files** have exactly the classes `CResourceFile`, `CFont` and `CStaticTexture`; every glyph record has 8 members.
- In **every** font: all glyph rectangles lie inside the atlas; all code-table values are glyph indices below the glyph count; the non-zero table values are **distinct and cover exactly glyphs 1…n−1** (glyph 0 is never mapped); `CFont.4` equals the atlas width.
- Rendering the string `Hello, Serious Sam 2! 0123` with the console font and `Hello, Serious Sam! gjpqy AVA` with the proportional `tahoma8b` font, using only the members described below (table lookup, rectangle, x offset, advance), gives correct, correctly spaced text.
- The 8 fonts with extra records decode them exactly to `METAEND ` (classes `CEditFont` 8, `CStaticTexture` 8, `CFontFilterOutline` 3).

## The `CFont` object

| Member | Type | Meaning |
|---|---|---|
| `.5` | → `CStaticTexture` | the **atlas** (an object of the same file) |
| `.6` | list of `CFontCharacter` | the **glyph records** (96 to 271 per font); index 0 is the fallback glyph |
| `.7` | list of `i32` | the **code table**: entry `c` is the glyph index for character code `c`; 0 = no glyph (use glyph 0). Length 127 (ASCII only, 5 fonts), 8,231 (3) or 8,483 (13) |
| `.4` | int | the atlas **width** in pixels (equal in all 21 fonts) |
| `.3` | int | the **line height** (equal to every glyph height in 18 of 21 fonts) |
| `.2` | int | the advance of a **monospaced** font (equal to every glyph advance in 9 fonts; in proportional fonts it is a different, larger number) |
| `.1` | int | a size value just below `.3` in most fonts (9, 17, 12, 8, 51, …): meaning not determined |

### `CFontCharacter` (8 ints)

| Member | Meaning |
|---|---|
| `.1`, `.2` | **x, y** of the glyph rectangle in the atlas (pixels, origin top-left) |
| `.3`, `.4` | **width, height** of the rectangle |
| `.5` | **x offset**: draw the rectangle this many pixels to the right of the pen position (−7 to 17; negative in proportional fonts, for example for `A`, `V`) |
| `.6` | y offset: 0 in every glyph of every font |
| `.7` | **advance**: move the pen by this many pixels after the glyph |
| `.8` | 0 in every glyph of every font |

**Layout.** Start the pen at the origin of the text; for each character, take `g = glyphs[table[code]]` (glyph 0 if the code is outside the table or the entry is 0), copy the rectangle `(x, y, w, h)` of the atlas to `pen.x + g.xOffset`, then add `g.advance` to `pen.x`. A new line is `CFont.3` pixels lower. No kerning data exists in the file. (The first glyph of the console font, the fallback, is a visible box; the space is glyph 1 with a 1-pixel-wide blank rectangle and an advance of 7.)

**Atlas.** The glyphs are packed in rows with a 1-pixel margin. The atlas is a normal `CStaticTexture` of `tex.md`: format DXT5 (10 fonts), 32-bit (7) or DXT3 (4); `TexFlags 0x24` (18) or `0x25` (3), wrap `0x51`/`0x51` (all 21). The glyph shapes are in the **alpha channel** in every font; the colour channels are one constant colour in 17 fonts and carry shading (an outline or gradient) in 4. Sizes from 128 × 64 to 2,048 × 2,048.

## The fonts

| Folder | Fonts | Code table |
|---|---|---|
| `Content/Fonts/` (4) | `gameconsolefont`, `profilerfont` (monospaced 7 × 12, ASCII, 96 glyphs, DXT5), `menufont`, `samprojectfont` (proportional, ASCII, DXT3, 256 × 128) | 127 |
| `Content/SeriousSam2/Fonts/` (9) | `seriouschatconsole`, `seriousconsole`, `seriousconsole_hd`, `seriousverdana`, `seriousverdana_hd`, `seriousverdana_menu`, `seriousverdana_server`, `seriouszurich`, `seriouszurich_hd` (atlases from 256 × 128 to 2,048 × 2,048; the `_hd` variants have the larger atlases) | 127, 8,231 or 8,483 |
| `Content/SeriousEditor2/GUI/Default/Fonts/` (8) | `couriernew10`, `lucidaconsole8`, `tahoma8`, `tahoma8b` in two sizes (editor GUI; not needed for the game) | 8,483 |

The large code tables (8,231 and 8,483 entries) cover **Latin and more**: for example `0x20`–`0x7E` (without `0x60` in some), Latin-1 (`0xA1`–`0xFF`), Latin Extended-A (from `0x106`), and further ranges up to `0x2122` (the highest mapped code of the 8,483-entry tables; `0x2026` for the 8,231-entry ones); the exact coverage per font is printed by `fnt_verify.py`. The game's text is therefore addressed by Unicode-like code points (UTF-16 or similar; the string encoding is not part of this file's format). `seriousverdana` also maps `0x0A` and `0x1F`.

## Reader invariants

1. Require exactly one `CFont` and its atlas object; fail on a glyph rectangle outside the atlas or a table value ≥ the glyph count.
2. Treat table entries outside the table as 0 (glyph 0).
3. Decode the atlas with the texture rules (`tex.md`) and use its **alpha** as the glyph coverage.
4. The extra records after `EDOB` are editor data and can be ignored.
5. Cap the glyph and table counts by the bytes remaining.

## Test vector

`Content/Fonts/GameConsoleFont.fnt` (7,349 bytes): `CFont.1..4` = 9, 7, 12, 128; the atlas is a 128 × 64 `CStaticTexture` (DXT5 = format 9, `TexFlags 0x24`, wrap `0x51`/`0x51`, a 2,366-byte compressed blob); 96 glyphs; a 127-entry table whose entries 32 to 126 are 1 to 95 (entries 0 to 31 are 0). Glyph 0 = (1, 1, 5, 12, offset 1, 0, advance 7, 0); the space (code 32) is glyph 1 = (7, 1, 1, 12, 0, 0, 7, 0); `!` (33) is glyph 2 = (9, 1, 1, 12, **3**, 0, 7, 0); `A` (65) is glyph 34 = (63, 13, 7, 12, 0, 0, 7, 0); `~` (126) is glyph 95 = (77, 49, 7, 12, 0, 0, 7, 0).

## Open questions

- The meaning of `CFont.1` and of the always-zero glyph members `.6` and `.8` (possibly a y offset and a kerning class reserved by the format).
- Which text encoding the game uses to index the table (see T01-041, localization).
- How fonts are selected for a given GUI element (the GUI system, T01-036) and whether a font has fallback fonts.
