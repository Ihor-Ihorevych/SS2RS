# `.tex` flag bits, wrap codes and format codes: what the data says (T01-054)

- **Roadmap task:** T01-054 (the leftovers of `tex.md`)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the numbers and for bit `0x10` (baked lightmap) and `0x02` (cube map, from `tex.md`); **medium** for wrap `0x50` = repeat and `0x51` = clamp, and for the grouping of the format codes by what the pixels look like; **low** for bits `0x01`, `0x04`, `0x40` and for wrap `0x52`. Nothing here is a definition from the engine: these are readings of 2,610 standalone textures and 1,246 embedded lightmaps.

Prerequisites: `tex.md` (layout, formats, the observed values), `wld-lighting.md` and `lighting.md` (lightmaps, the oven and its baker record).

## Summary

| Question | Answer | Confidence |
|---|---|---|
| Bit `0x10` | the texture is **the output of the texture oven's light-map baker**: set on all 1,246 embedded world lightmaps read (10 worlds) and on 2 standalone `shadow.tex` files, on none of the other 2,608 standalone textures | high |
| Bit `0x02` | cube map (as in `tex.md`) | high |
| Bit `0x20` | always set (2,610 of 2,610); no information | high |
| Bit `0x04` | **interface texture**: 187 textures, 149 of them (80 %) under `textures/interface` or `textures/menu`; they have no mip chain (6 DXT1, 2 DXT3 and 10 DXT5 of 187) | low |
| Bit `0x01` | set in 72 % of textures; rare in interface textures and absent from the 63 loading screens; no mip-chain or size explanation found | low |
| Bit `0x40` | set on 1,031 of the 1,246 embedded lightmaps, on both `shadow.tex` files and on one terrain blend mask; **tracks the baker preset** (see below); meaning unknown | low |
| Wrap `0x50` | the default: **repeat** | medium |
| Wrap `0x51` | **clamp to the edge**: interface, menu and loading-screen textures and 958 of the 1,031 `0x71` lightmaps | medium |
| Wrap `0x52` | only 17 textures on at least one axis (flares, waterfalls, windows); **not determined** (mirror and clamp-to-border are the candidates) | low |
| Format codes `0`, `2`, `6` and `1`, `3`, `4`, `5`, `11` | same layout, different **content class**: see below | medium |

## Evidence

`tools/re/texflags_verify.py` (read-only, `SS2_DATA_DIR`; about 10 minutes); its output is `tex-flags-tables.md`. The seam test and the format survey decode a sample of at most 80 textures per class (those up to 256 px on the long side) with the decoder of `tools/re/texpng.py`.

## Flag bits

Values seen (`tex.md`): `0x21` (1,837), `0x20` (551), `0x24` (164), `0x23` (32), `0x25` (23), `0x71` (2), `0x60` (1) in standalone files; **`0x71` (1,031), `0x30` (159), `0x31` (56)** in the 1,246 lightmaps embedded in the 10 smallest worlds.

- **`0x10` is the lightmap flag.** Every texture that a texture oven modifier writes (all with the **light-map baker**, class `CTextureBakerLightMap`, 1,246 of 1,246) has it; so have the two standalone `building_16_top.../shadow.tex` files of the Sirius level (flags `0x71`, 1,024 and 512 px DXT1, clamp). No other standalone texture has it. A reader can use it to tell "baked light" from "authored colour".
- **`0x40` follows the baker preset, not the format.** The baker record (`wld-lighting.md`) holds the bake settings; the names of its members are not in the data (the editor help lists, among others, the occlusion radius, blur radius and sample count, which are the likely candidates). Among the lightmaps **with** `0x40`, the members `CTextureBakerLightMap.10`, `.11` and `.12` are 10.0, 2 and 20 in 1,019 of 1,031; among those **without** it they are 25.0, 12 and 36 in 160 of 215 (a different, stronger preset), while 55 of the 215 have the first preset anyway. `0x40` also appears on one terrain blend mask (`terrain_06/blend.tex`, flags `0x60`, 32-bit, 256 px). The best reading is a **baker-version or quality marker**; a reader should ignore it.
- **Lightmap format and wrap by flag:** `0x71` is always DXT1 (958 clamp, 73 repeat); `0x30` is DXT1 in 155 of 159 and repeat; `0x31` is mostly 24-bit (53 of 56) and repeat in 48 of 56. So the flag is also a rough proxy for "how the baker stored it".
- **`0x04` is for interface textures.** Of 187, 126 are in `interface` and 23 in `menu`; the others are mostly in unlisted folders (38). 150 of the 187 are 32-bit with alpha and 19 are 24-bit; only 18 are DXT, so **no mip chain** is stored for them in nearly all cases. A reimplementation does not need the flag: a texture without mips and with clamp is a UI texture anyway.
- **`0x01`** is set in 1,894 of 2,610 standalone textures. It is rare where `0x04` is (23 of the 187) and the 63 loading screens are among the textures without it, and it is common in `levels`, `generic` and `effects`. By format code, textures with and without the bit: DXT1 1,109 and 131, DXT3 71 and 172, DXT5 272 and 17; 24-bit code 0 113 and 159, code 2 108 and 3, code 6 49 and 2; 32-bit code 1 113 and 181, code 4 16 and 23, code 5 35 and 21. No size or mip-chain rule explains it. **Unknown**; the loader of `tex.md` does not need it.

## Wrap codes

The pairs (U, V): `0x50/0x50` 2,092 · `0x51/0x51` 467 · `0x50/0x51` 25 · `0x51/0x50` 9 · `0x52/0x52` 8 · `0x50/0x52` 6 · `0x52/0x50` 3.

- **`0x50`** is used by the world, model and effect textures (`levels` 461, `generic` 337, `effects` 138, unlisted folders 1,079) and **`0x51`** by `interface` (124), `menu` (94), `loadingscreens` (55) and 103 `levels` textures (plus the clamp lightmaps embedded in the worlds: 958 of the 1,031 `0x71` ones). That matches repeat and clamp.
- **Seam test** (does the left edge continue into the right edge, and the top into the bottom, as smoothly as two neighbouring columns do?). Among sampled textures whose edges are **not** mostly transparent or black (so the test means something): `0x50` has a smooth seam in **54 %** (U) and **53 %** (V) of 26 and 30 textures, median ratio 1.72 and 1.75; `0x51` in **33 %** and **30 %** of 15 and 20, median ratio 3.48 and 4.24. The difference is in the expected direction but **small**: many world textures are not seamless, and the test says only that clamp textures are less tileable. It is supporting evidence, not proof.
- **Edges that are transparent or black**: 64 % of the `0x50` and 76 % of the `0x51` U samples. This is the common case for sprites and effect textures whatever the wrap, which is why those textures give no seam information.
- **`0x52`**: only the small textures could be sampled, 4 on U and 6 on V; the data are too few to tell mirror from clamp-to-border. The one non-zero `COLOR` (the border colour member `CBaseTexture.11`, `tex.md`) is on a `0x52` texture, so clamp-to-border is possible.
- Mixed pairs (`0x50/0x51`, `0x51/0x50`) are 34 textures: tiling on one axis only (horizon strips, interface bars).

**For a reimplementation:** map `0x50` to repeat, `0x51` to clamp-to-edge; treat `0x52` as clamp-to-edge until a code analysis says otherwise (the 17 textures are effects whose edges are mostly transparent, where clamp and border look the same).

## Format codes that share a layout

Sample of up to 80 textures per code (only those up to 256 px on the long side are decoded): share of grayscale images (R, G and B within 6 levels), of images whose alpha varies and of images that are partly transparent.

| Layout | Code | Textures up to 256 px | Looks like | Where |
|---|---|---|---|---|
| 24-bit BGR | `0` | 114 | colour, **0 % grayscale** | general model textures |
| | `2` | 62 | colour, 3 % grayscale | effects 35, general 18, generic 6 |
| | `6` | 47 | **85 % grayscale** (masks, gradients) | effects 17, generic 15 |
| 32-bit BGRA | `1` | 216 | colour with alpha (22 % grayscale; 98 % alpha varies) | interface 47 of 80 |
| | `3` | 1 | colour with alpha | one effect |
| | `4` | 38 | colour with alpha (18 % grayscale) | unlisted 14, interface 13, effects 8 |
| | `5` | 48 | **96 % grayscale with alpha** (white sprites and glyphs tinted at run time) | interface 24, menu 14 |
| | `11` | 10 (14 in all) | **grayscale alpha masks** (100 % grayscale) | `Alpha_01`, `Mask` and similar |

The codes are therefore **content classes**: for 24-bit colour data `0` is ordinary colour, `2` effect colour, `6` single-channel-like data stored as RGB; for 32-bit data `1`, `3`, `4` are coloured images with alpha and `5`, `11` are grayscale with alpha. **The pixel layout is the same inside each group**, so a loader needs no change; the classes may select a texture filter or a compression preference in the original, which the data cannot show.

## Reader invariants

1. Ignore `0x01`, `0x04`, `0x20` and `0x40` when decoding; they do not change the layout.
2. `0x02` = cube map; `0x10` = baked lightmap (use it to choose a lightmap sampler or to skip colour management).
3. Wrap: `0x50` repeat, `0x51` clamp to edge, anything else clamp to edge.
4. The format-code groups are not a decoding input; keep the code if the original distinction is wanted later.

## Test vectors

- `content/serioussam2/models/levels/08_sirius/architecture/building_16_top/shadow.tex` (the path as stored in the archive, lower case): 1,024 x 1,024, format 7 (DXT1), flags `0x71`, wrap `0x51`/`0x51`.
- `content/serioussam2/models/levels/01_mdigbo/architecture/terrain_06/blend.tex`: 256 x 256, format 4, flags `0x60`, wrap `0x50`/`0x50`.
- The embedded lightmaps of the ten smallest worlds (the verifier prints the counts): flags `0x71`, `0x30` or `0x31` and nothing else.

## Open questions (a follow-up task, T01-080: code analysis)

- What bits `0x01` and `0x40` switch in the original loader or renderer.
- Whether wrap `0x52` is mirror or clamp-to-border, and how the border colour is used.
- Whether the format-code classes select anything (filtering, compression, gamma).
- Whether textures with bit `0x04` or without a mip chain are sampled differently (point or bilinear).
- Gamma and sRGB handling (still no flag found, `tex.md`).
