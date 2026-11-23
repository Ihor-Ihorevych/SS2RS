# asset-export: textures to PNG, models to glTF

`tools/asset-export` (T11-014) turns the game's own textures (`.tex`) into **PNG** files and its models (`.bmf`) into **glTF 2.0** (`.glb` or `.gltf`), so that modders and spec reviewers can look at
them in any image viewer, in Blender, in the glTF viewers of the browsers and in the Khronos tools. It reads through the existing readers (`ss2-format-tex`, `ss2-format-bmf`, `ss2-format-mtr`) and the
VFS; their public API is unchanged. It needs no game data to run on a file you give it; with `--install DIR` it reads from the install through the VFS (the newest patch wins, as in the game).

> **Never commit what it writes from real game data.** The output of a real install is the game's content in another format. Write it to a folder outside the repository. The examples below use only
> the hand-made fixtures in `testdata/asset-export/` (see [Fixtures](#fixtures)).

```
asset-export tex   FILE -o out.png                      one texture file
asset-export tex   --install DIR content/x/y.tex -o out.png   one texture of the install (a VFS path)
asset-export tex   --all FOLDER -o OUTDIR [--install DIR]     every .tex below FOLDER, mirrored under OUTDIR, + manifest-texture.json
asset-export sheet FILES... -o sheet.png                contact sheet (+ sheet.json, the list of cells)
asset-export mesh  FILE -o out.glb | out.gltf           one model
asset-export mesh  --all FOLDER -o OUTDIR [--install DIR]     every .bmf below FOLDER, mirrored, + manifest-mesh.json
```

With `--install`, `--all` takes a VFS folder (`/` or `""` for the whole install, or for example `content/serioussam2/models`); without it, a directory on disk. Output files mirror the **lower-case
install path** with the extension replaced (`.png`, `.glb`/`.gltf`), which is also what the texture URIs inside the glTF files assume (see [Textures of a model](#textures-of-a-model)).

## Textures: `tex`

```
asset-export tex testdata/asset-export/bgr_8x4.tex -o out/ramp.png
asset-export tex testdata/asset-export/dxt1_4x4.tex -o out/level1.png --mip 1
asset-export tex testdata/asset-export/cube_4x4.tex -o out/face2.png --face 2
asset-export tex testdata/asset-export/cube_4x4.tex -o out/all.png --face all        # six faces side by side, in file order
asset-export tex testdata/asset-export/bgra_4x4.tex -o out/colour.png --rgb-only      # drop alpha, colours unchanged
asset-export tex testdata/asset-export/bgra_4x4.tex -o out/alpha.png --alpha-only     # the alpha channel as a grayscale image
```

| Option | Meaning |
|---|---|
| `--mip N` | mip level (0, the default, is the largest). Only DXT textures have levels (the whole chain down to 1 x 1); asking for a level the file does not have is an error that says how many it has. |
| `--face N\|all` | face of a cube map (0 to 5, file order; the order's meaning is **medium** confidence in `docs/re/tex.md`); `all` writes the six faces side by side in one wide image. A texture that is not a cube map has face 0 only. The format has **no texture arrays**, so there is no layer option. |
| `--rgb-only` / `--alpha-only` / `--rgba` | channel handling. Default: RGB for 24-bit textures and for DXT1 without transparent pixels, RGBA otherwise (32-bit, DXT3, DXT5, DXT1 with at least one transparent pixel). |

Pixel formats (`docs/re/tex.md`): 24-bit BGR and 32-bit BGRA are converted **exactly** (channel swap only). DXT1, DXT3 and DXT5 are decoded in software (BC1, BC2, BC3 block layouts): the two RGB565
end points are expanded by bit replication and interpolated with integer division by 3; DXT1 with the end points in the "three colour" order (`c0 <= c1`) makes colour 3 transparent black; DXT2-5
always use four opaque colours; DXT3 alpha is 4 bits (stored `a` comes back as `a * 17`); DXT5 alpha uses the 8-value or 6-value table. A GPU may differ from this by one level per channel. Rows are
written top to bottom as stored (the spec confirms the orientation visually). Normal-map handling (what the channels of a `_nm` texture mean) is **not** touched: the channels are written as
stored. PNG output is 8-bit and deterministic (the same input gives the same bytes); no gamma, colour-space or text chunks are written.

### Contact sheets: `sheet`

```
asset-export sheet testdata/asset-export/bgr_8x4.tex testdata/asset-export/bgra_4x4.tex testdata/asset-export/dxt1_4x4.tex -o out/sheet.png --cell 64
asset-export sheet --folder content/serioussam2/textures --install DIR --filter "*menu*" --limit 200 -o out/menu.png
```

Many textures as thumbnails in one PNG, row by row (`--columns N`, default a square-ish grid; `--cell PX`, default 128), over a checkerboard so that alpha shows. A thumbnail keeps its aspect
ratio: larger images shrink with an alpha-weighted box filter, smaller ones are enlarged by the largest whole factor that fits (nearest neighbour). `--mip` and `--face` choose the level and face.
`out/sheet.json` lists, for every cell, the source, row, column, pixel position and sizes, and lists the textures that could not be included (with the reason); the sheet itself has no labels (there is
no font), the list is the key. Exit code 1 if some textures were left out.

## Models: `mesh`

```
asset-export mesh testdata/asset-export/quad.bmf -o out/quad.glb
asset-export mesh testdata/asset-export/rich.bmf -o out/rich.gltf --all-lods
asset-export mesh testdata/asset-export/rich.bmf -o out/rich_far.glb --lod 1 --flip-winding
```

| Option | Meaning |
|---|---|
| output extension | `.glb` (binary, one file) or `.gltf` (JSON, the buffer embedded as a base64 data URI, one file). With `--all`, `--format glb\|gltf` (default `glb`). |
| `--lod N` / `--all-lods` | LOD 0 (the nearest) by default; `--all-lods` writes every LOD as its own node (`name_lodN`) of one scene (they overlap in space; hide all but one in a viewer). |
| `--flip-winding` | swap two indices of every triangle (see the conventions below). |

What is written, per surface (one glTF primitive, triangle list): `POSITION` (with the required min/max), `NORMAL`, `TANGENT` where the file has them, one `TEXCOORD_n` per UV stream (in slot order),
`UNSIGNED_SHORT` indices, and a material. Per LOD node, `extras.ss2` has the LOD number, the switch distance, the platform mask and the **bounding box** of the LOD as the file states it; the scene's
`extras.ss2` has the three mesh distances, whether the file has a collision mesh, the notes and the warnings. Buffer views are 4-byte aligned and each attribute has its own view. Output is
deterministic (keys sorted, no time stamps).

### Materials

A surface's material is an embedded `CShaderPreset` or an external `.mtr` file or none (`bmf.md`). The exporter reads it with `ss2-format-mtr` (an external `.mtr` needs `--install`; without it the
material is written with a warning). From the config for the PC (the first with platform bit 0, else the first) it takes the shader class and **all external texture references**
(`extras.ss2.textures`: member number and install path). One of them becomes the material's `baseColorTexture`, chosen **by the shader class** from `docs/re/mtr.md`: member `.10` for the shade,
reflection and pixel-light shaders, `.21` for the specular shader; the other shaders (terrain, water, refraction, blend) get no base colour because the spec does not name one. The texture coordinate
set is the UV stream whose map id the material names. Colour tints, blend types, modifiers and normal maps are **not** translated; materials are written double sided, with `metallicFactor` 0.

### Textures of a model

Textures are **external references**, never embedded: `images[].uri` is the relative URI of the PNG that `tex --all` writes for the same `.tex` path (`../../textures/body.png`, percent-encoded),
computed from the folder of the model in the install. Export the textures and the models with `--all` into the same output folder and the links work; a single model exported with `--install` assumes
the same layout. For a model read from a plain file (no install) the folder is unknown and the URIs are relative to the root of the install tree.

### Conventions that are not confirmed

The spec leaves **handedness, winding and the orientation of the UV v axis open** (`docs/re/bmf.md`, "Open questions"). Coordinates and UVs are written exactly as stored, with no axis change and no
v flip; if a model looks inside out or mirrored in a viewer, that is the open question, not a bug of the exporter: try `--flip-winding`. Materials are double sided so that a wrong winding does not hide
the surface.

### Skeletons and animation

**Not exported.** The mesh reader exposes bone weights, bone indices and the per-surface bone palette, but there is no skeleton (`.skl`) reader yet, so no glTF skin can be built. A model with
skinned surfaces is exported as a static mesh in its bind pose and the export says so (`warning: N skinned surfaces: bone weights, ...`, also in the scene's `extras`). Morph maps and the collision mesh
are not read by the mesh reader and are not exported. Models (`.mdl`, which point to `.bmf` meshes) are not exported either: no reader exists yet.

## Batch export and the manifest

`--all FOLDER -o OUTDIR` converts every file in sorted order and writes `OUTDIR/manifest-texture.json` or `manifest-mesh.json`. One bad file never stops the run. Options: `--filter GLOB` (`*` any run
of characters including `/`, `?` one character; case-insensitive; matched against the install path), `--limit N` (the first N after sorting and filtering), `--dry-run` (convert, write nothing, print the
manifest). The manifest has no time stamps and no absolute paths, so two runs give the same bytes:

```json
{
  "tool": "asset-export 0.0.0", "kind": "texture", "folder": ".", "filter": null,
  "options": { "channels": "Auto", "face": 0, "mip": 0 },
  "summary": { "found": 4, "exported": 3, "unsupported": 0, "failed": 1, "bytes_in": 3000, "bytes_out": 300 },
  "reasons": { "failed: invalid CTSEMETA container: ...": 1 },
  "entries": [
    { "source": "textures/a_bgr.tex", "status": "exported", "output": "textures/a_bgr.png", "bytes_in": 985, "bytes_out": 80,
      "details": { "format": "Bgr8", "mip_levels": 1, "faces": 1, "size": [8, 4], "source_size": [8, 4], "png_alpha": false } },
    { "source": "textures/broken.tex", "status": "failed", "bytes_in": 13, "error_kind": "format", "error": "..." }
  ]
}
```

A status is `exported`, `unsupported` (a known gap: the development-build textures of format versions 4 to 7, 125 of them in the install as the VFS shows it, which the texture reader refuses; counted separately and **not** a failure) or `failed`
(`error_kind`: `read`, `format`, `option`, `encode`, `write`).

**Exit codes:** `0` success (unsupported files are not failures) · `1` a batch or sheet finished but some files failed or were left out · `2` error: usage, an unreadable or damaged single input, an
unwritable output, a mesh LOD or texture level that does not exist · `3` the single input is of a kind the readers do not handle yet (an old development texture). Nothing panics on damaged input
(tests below); a damaged file is a typed error.

## How it was tested

* **Unit and fixture tests** (`cargo test -p asset-export`, no game data): PNG round trips of every pixel format the reader supports (24-bit codes 0, 2, 6 and 32-bit codes 1, 3, 4, 5, 11 compared
  pixel for pixel; DXT1/3/5 against a test encoder with the tolerances written in the tests: DXT1 gray ramps within 8 levels, DXT3 within 16, DXT5 within 8, two-colour DXT1 blocks whose colours fit
  RGB565 exactly equal); the block decoders against hand-computed blocks (interpolated colours and alphas, the three-colour mode, partial blocks, block order); mip and face selection, the
  cube strip, the channel modes; the glTF structure of hand-made meshes (accessor counts, types, min/max, index range, alignment, buffer length, the GLB chunk rules, the data URI) read **back by
  the `gltf` crate** (a dev-dependency accepted by `cargo deny check`) and compared with the engine's reader; LOD selection, winding flip, non-finite values refused; materials from hand-built
  `.mtr` structures; relative URIs; the batch manifest, filter, limit, dry run; the command line end to end; **every truncation and single-byte corruption** of the fixtures (no panic).
* **The same outputs through two reference validators** (run by hand, scripts in `tools/asset-export/scripts/`): the Khronos **glTF-Validator** (`gltf-validator` from npm, a Node script
  `validate_gltf.mjs`) and **Pillow** (`validate_png.py`).
* **Real install** (`tests/real_install.rs`, data-gated by `SS2_DATA_DIR`): see below.

### Results on the real install

Measured on the owner's retail install with all patches mounted (the VFS view; `SS2_DATA_DIR`), on the tree of this pull request:

| | Found | Exported | Unsupported | Failed |
|---|---|---|---|---|
| Textures (`.tex`) | 2,688 | **2,563** | 125 (format versions 4 to 7, the old development textures the reader refuses) | **0** |
| Models (`.bmf`) | 76 | **76** | 0 | **0** |

* **Textures:** 535,184,566 bytes of `.tex` in, 850,204,474 bytes of PNG out, 820,923,420 pixels; formats Bgr8 432, Bgra8 389, Dxt1 1,225, Dxt3 240, Dxt5 277 (PNG: 1,657 RGB, 906 RGBA); 31 cube maps.
  Every PNG was decoded by the `png` crate inside the test (dimensions equal the manifest's) **and** by **Pillow 13.0.0**: 2,563 of 2,563 decode, 0 failures. A roughness check (mean difference of
  neighbouring pixels, about 85 for noise) flags 2 textures, `effects/screen_01.tex` and `interface/hud/weaponnumberbarcornerglow.tex`, which are noise and glow images by nature. The author looked at
  contact sheets and single exports (the HUD sheet, the enemy body and normal maps, a 1024 x 1024 boss atlas): upright, colours right (a blue-and-yellow Netricsa icon, a red heart, blue-violet
  normal maps), alpha shown as a checkerboard.
* **Models:** 17,758,812 bytes of `.bmf` in, 11,445,072 bytes of `.glb` out; 353 primitives (the spec's 353 surfaces), 216,784 vertices, 213,294 triangles, 265 materials (258 embedded presets
  and 7 external `.mtr` files, all resolved). 66 files have skinned surfaces (bone data not exported, warned). Every `.glb` was read back by the `gltf` crate (positions, normals, UVs, tangents and
  indices equal the engine reader's) and validated by the **Khronos glTF-Validator 2.0.0-dev.3.10: 76 of 76 files with 0 errors and 0 warnings** (558 infos: unused UV sets, unused tangents,
  the GLB note). The first version of the exporter failed 4 files with 12,649 errors: some retail meshes have **zero-length normals and tangents** (the spec says normals are unit length). They are
  now repaired by default (normalised, or +Y / +X when zero) and counted in the warnings; `--raw-vectors` keeps them as stored.
* **Textures referenced by the models:** 222 references in 66 files, 221 exist in the install; the one that does not (`player/sources/meshes/ss2loggy.bmf` names `.../meshes/metal_nm.tex`) is a
  dangling reference in the game data.
* **Time:** the whole walk (all textures, all models, PNG and glTF encoding, in memory) took 462 s in a **release** build; the command line, `tex --all content/serioussam2/models`, 1,041 textures in
  116 s. A debug build is much slower (the data-gated test samples every 16th texture unless `ASSET_EXPORT_FULL=1`).
* The test `every_texture_of_the_install_is_exported_and_decodes` and `every_model_of_the_install_is_exported_and_read_back` print these numbers with `--nocapture`.

### Mutation check

91 one-line breakages of the exporter's own code (the DXT decoders, the alpha tables, face and mip selection, channel handling, the glTF writer, alignment and padding, material choice, URIs, vector
repair, the sheet layout and blending, the glob matcher, the batch counters), each applied alone and followed by the library and CLI tests: **89 killed** (one of them by a test timeout: a glob
breakage that loops forever), **2 equivalent**, 0 survivors left. 15 survived the first round of 78 applicable ones and led to tests for the DXT5 alpha modes, URI edge cases, view alignment,
the union of LOD boxes, the checkerboard, blend rounding, the box filter, glob case and stars, and the manifest counters; one of the equivalent ones was a single-image shortcut that was removed, the
other is `w <= cell` in the sheet thumbnail (a texture exactly one cell wide takes the shrink path at scale 1, which gives the same pixels).


## Fixtures

`testdata/asset-export/` holds six tiny files **made by the test builders** (`tools/asset-export/src/fixtures.rs`, built from the public specs with the testkit's CTSEMETA builder; no game content):
`bgr_8x4.tex`, `bgra_4x4.tex`, `dxt1_4x4.tex`, `cube_4x4.tex`, `quad.bmf`, `rich.bmf` (two LODs, three surfaces with tangents, an embedded empty material and a skinned surface). A test checks that they
equal what the builders make; `UPDATE_FIXTURES=1 cargo test -p asset-export --test cli` rewrites them.

## Validating by hand

```
cd $TEMP/gltfval && npm install gltf-validator          # once, outside the repository
GLTF_VALIDATOR_DIR=$TEMP/gltfval node tools/asset-export/scripts/validate_gltf.mjs OUTDIR
python -I tools/asset-export/scripts/validate_png.py OUTDIR
```

## Not covered

* **Skeletons and animation** (T11-014b): bone weights, bone indices and the palette are read but not exported (no `.skl` reader, so no glTF skin or joints); `.ans` animations are not exported.
* **`.mdl` models** (the files that name a `.bmf` mesh, LODs and materials): no reader exists; only `.bmf` meshes are exported. Morph maps and the collision mesh are not read by the mesh reader.
* **Conventions:** handedness, triangle winding and the UV v axis are written as stored and **not verified** against the engine (`bmf.md` leaves them open); `--flip-winding` is a workaround, not a fix.
* **Materials** are only partly translated: one base colour texture chosen by shader class (shade, reflection, pixel-light, specular only), the texture list in `extras`; tints, blend modes, normal
  maps, modifiers and animated parameters are not. Embedded textures (8 models) are not handled; only external `.tex` references become images.
* **Textures:** the 125 development-build textures (format versions 4 to 7) are not exported (the reader refuses them, T01-083); no normal-map channel handling; no HDR, 16-bit or compressed (BC) output
  (PNG is always 8-bit RGB/RGBA/gray); DXT decoding follows the usual software convention and may differ from a GPU by one level; contact sheets have no text labels (the JSON list is the key); `--face
  all` writes a strip, not a cross, and the cube face order has only medium confidence in the spec; there is no mip-chain sheet (all levels of one texture in one image).
* **Validation by hand:** the Khronos validator and Pillow were run by hand for this pull request (the Rust test for the validator runs only when `GLTF_VALIDATOR_DIR` is set); CI does not have them.
  Linux tests and `gitleaks` were not run for this pull request (see the pull request description).
* **Speed:** conversion is sequential (the whole install in about 8 minutes in release); no incremental export (it rewrites every file).

