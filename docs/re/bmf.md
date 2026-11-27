# `.bmf` binary meshes

- **Roadmap tasks:** T01-007 (mesh layout), T01-055 (platforms, winding, morphs, collision and materials)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for object and stream layout, triangle indices, the observed morph streams, collision index widths, and collision-tree framing. Engine field names agree with the serialized shapes. **Low** for runtime behavior inferred only from the static trace, including the material fallback. Data scope is all **76** effective format-8 `.bmf` files of the retail install with patches.

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`.

## Summary

A `.bmf` is a CTSEMETA file holding **one `CMesh`**. The mesh has a render part (`CRenderMesh`: a list of LODs, each with surfaces) and, in 34 of 76 files, a `CCollisionMesh`. The vertex and index data are not parsed into structures: they sit in raw byte blobs inside `GFXHANDLE` objects, and every surface says where inside those blobs its streams start (an **element path**: element code, buffer, byte offset). Vertex data is **planar**: each attribute is its own contiguous array. Indices are `u16` triangle lists, local to the surface.

The files hold 125 LODs, 353 surfaces, 213,294 triangles and 216,784 vertices in total.

## Evidence

`tools/re/bmf_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches. For **every** one of the 353 surfaces: all streams and the index list fit inside their buffers; positions lie inside the LOD's bounding box; normals have unit length (within 0.02, in at least 99 % of the vertices); every index is smaller than the surface's vertex count; the number of UV streams equals surface member 4. For **every** buffer (125 index buffers, 125 vertex buffers) the streams of its surfaces **tile the buffer exactly**, with no gap and no byte left over, using the element sizes given below. 120 surfaces have a tangent stream whose fourth component is exactly ±1. For all 333 skinned surfaces the largest bone index is smaller than the surface's palette. `tools/re/edob_verify.py` checks the data after `EDOB` (below).

`python -I tools/re/bmf_details_verify.py` with `SS2_DATA_DIR` set reproduces the
T01-055 data measurements. Field names and runtime paths were checked against the
installed 32-bit `Engine.dll` (SHA-256
`da6efc9f72637eb3b6f48eadca2107be89b09c00618b6e72d5d3632938a7d851`).
The script reads archives only. An offscreen Direct3D 9 render tested winding and UV
row selection with synthetic geometry and a two-color texture.

## Object graph

Every file has the objects (ids as in `OBTY`):

```
CResourceFile                         (object 0)
CMesh                                 (object 1)
  CMesh.1  -> CRenderMesh
  CMesh.2  -> CCollisionMesh          (34 files; null in the other 42)
CRenderMesh
  .12      list of VBufferHolder      (index buffers; .1 = reference to a GFXHANDLE, or null)
  .13      list of VBufferHolder      (vertex buffers; same shape)
  .5       list of CRenderMeshLOD
GFXHANDLE x 2 per LOD                 (one index buffer, one vertex buffer)
CShaderPreset (+ its CShaderArgs objects)   (materials, see below)
```

`CResource` members (`.1` = 1 for the mesh, −1 for the helper objects; `.5` = −1; `.3` = 0; `.2` = `CResourceFile` or null) are the same as for textures.

### `GFXHANDLE`

| Member | Type | Meaning |
|---|---|---|
| `GFXHANDLE.1` | `GfxBufferType` (4-byte enum) | `0x5F` = **index buffer** (referenced from `CRenderMesh.12`), `0x1F` = **vertex buffer** (from `.13`); no other value occurs (125 of each) |
| `GFXHANDLE.2` | `CStaticArray<UBYTE>` | the raw bytes (`STAR`, count, bytes). **Not compressed** |

Buffer numbers used by element paths are positions in the `.12` / `.13` lists. A list entry may be null (8 files: an unused slot, for example in `Helicopter.bmf`); no surface refers to a null slot.

### `CRenderMesh`

| Member | Meaning |
|---|---|
| `.5` | LODs, 1 to 4 per file (1 LOD: 48 files, 2: 11, 3: 13, 4: 4) |
| `.12`, `.13` | index / vertex buffer holders (above) |
| `.1`, `.2` | always 0 |
| `.3`, `.4` | always empty lists |
| `.6` | always null |
| `.7`, `.8`, `.9` | Float fields `rm_fFadeOutStart`, `rm_fFadeOutEnd`, and `rm_fCullDistance`. All 76 files have `0 ≤ .7 < .8` and `.8 = .9`. In 69 files `.8` also equals the last LOD's max distance |
| `.11` | `rm_fFadeLength`, always 10.0 |

### `CRenderMeshLOD`

| Member | Type | Meaning |
|---|---|---|
| `.1` | `MeshLODFlags` | 0 in all 125 |
| `.3` | `Box3f` (6 floats: min x,y,z then max x,y,z) | bounding box of the LOD (`Box3f.1`..`.6`) |
| `.4` | reference | always null |
| `.5` | list of `CRenderMeshSection` | exactly **one** section per LOD |
| `.6` | float | switch distance; ascending with the LOD number in all 76 files (for example 150, 300, 500) |
| `.7` | `EPlatform` | Platform mask. Bit 0 is PC and bit 1 is Xbox (see `gui.md`). Counts are `0xFFFFFFFF` 83, `0xFFFFFFFD` 32, `0x3` 4, `0xFFFFFFFE` 4, and `0x1` 2 |
| `.8` | `IDENT` | `rml_idName`, the LOD's identifier in the file |

`CRenderMeshSection.1` is the list of `CRenderMeshSurface`.

The original `CMesh::Finalize` filters LODs by intersection with the target platform
mask. On PC 121 LODs are eligible for distance selection. Four have the PC bit clear.
Eligibility and the LOD selected for a particular draw are separate.

### `CRenderMeshSurface`

| Member | Meaning |
|---|---|
| `.1` | `MeshSurfaceFlags`, 0 in all 353 |
| `.2` | **triangle count** (`T`) |
| `.3` | **vertex count** (`V`) |
| `.4` | **number of UV streams** (1 to 5); equals the number of present entries in `.14` in all 353 |
| `.5` | always 0 |
| `.6` | element path of the **index list** |
| `.7` | element path of the **position** stream |
| `.8` | element path of the **normal** stream |
| `.9` | element path of the **tangent** stream (present in 120 surfaces, absent otherwise) |
| `.10` | element path of the **bone weights** (present in 333 surfaces, absent otherwise) |
| `.11` | element path of the **bone indices** (present exactly when `.10` is) |
| `.12` | 8 words: for each UV slot the UV map id (for example 3, 4, 5, 6, or 2); 0 for unused slots |
| `.13` | Eight color-map identifiers, one per `.15` slot. All are 0 in these meshes, which have zero color streams (`.5`) |
| `.14` | 8 element paths: the **UV streams** (slot order as in `.12`); unused slots have buffer 255 |
| `.15` | 8 element paths: always all absent |
| `.16` | reference: the surface's **material** (a `CShaderPreset`), or null (88 surfaces), see below |
| `.17` | reference, always null |
| `.18` | list of ints: the **bone palette** (empty for the 20 static surfaces, 1 to 20 entries for skinned ones) |
| `.20` | `rms_iMasterSurface`, a zero-based index into the same section's surface list. −1 means no master. The runtime link is `.21` |
| `.22` | list of `CSurfaceMorphMap` (empty in 339 surfaces, 1 to 10 maps in 14) |
| `.23` | `rms_idName`, the surface's file-local `IDENT`. In `Grass_14.bmf` it is 2 |

An **element path** (`CRenderMeshElementPath`) is three words:

| Word | Meaning |
|---|---|
| `.1` | **element code** (below); 0 when the stream is absent |
| `.2` | **buffer number** (index into `CRenderMesh.12` for `.6`, into `.13` for the others); **255** = stream absent |
| `.3` | **byte offset** of the stream inside that buffer (`0xFFFFFFFF` when absent) |

## Element codes and sizes

| Code | Size of one element | Use | Basis |
|---|---|---|---|
| 135 | 2 bytes (`u16`) | index | the index list of every surface |
| 133 | 12 bytes (3 × `f32`) | position (`.7`), normal (`.8`) | bounding box and unit-length tests |
| 132 | 8 bytes (2 × `f32`) | UV (`.14`) | exact tiling; the `Grass_14` values are 0.02 and 0.98 (the range of UVs was not measured) |
| 134 | 16 bytes (4 × `f32`) | tangent (`.9`): xyz and a sign (±1.0) | unit-length xyz in 97 % of the sampled vertices; `w` exactly ±1 |
| 128 | 4 bytes (4 × `u8`) | bone weights (`.10`), bone indices (`.11`) | see below |

The sizes are the only ones for which **all** buffers tile exactly. The names (float, byte) follow from the sizes and the value tests; the engine's own names for the codes are not known.

## Reading a surface

1. Index list: `T × 3` little-endian `u16` at the path's offset in the index buffer: a **triangle list** (no strips, no restart). Indices are **local**: smaller than `V`, and (352 of 353 surfaces) every vertex in `0..V` is used. The index lists of the surfaces of a LOD together fill the buffer exactly.
2. Each vertex stream is an array of `V` elements of the code's size, **starting at the path's offset** (planar layout: all positions, then all normals, and so on, never interleaved). The streams of all surfaces of a LOD together fill the vertex buffer exactly (no gap, no byte left over); the order of the streams inside the buffer was not analysed.
3. Positions are in the mesh's own object space (inside the LOD box). Normals are unit length.

### Winding and UV orientation

The stored index order usually gives a counterclockwise front face in a right-handed
mesh basis. The cross product of edges from the first vertex points with the average
stored normal in 212,719 of 213,294 triangles and against it in 567. The cross
product is zero in 8 triangles.
Preserve the index order. An importer that reflects an axis to change basis must
account for the reflection when selecting front-face winding. The data check is in
mesh space and does not establish the title's camera or projection handedness.

The upright grass card in the test vector has local Y=1 at V=0.02 and Y=0 at V=0.98.
In the offscreen Direct3D 9 comparison, a counterclockwise synthetic quad with this
UV assignment sampled the first texture row above the last row. Culling clockwise
faces retained the quad and culling counterclockwise faces removed it. Use the stored V
with the top-down texture rows in `tex.md`. Shader-specific UV transforms in the game
remain unmeasured. [Direct3D 9 texture coordinates](https://learn.microsoft.com/en-us/windows/desktop/direct3d9/directly-mapping-texels-to-pixels)

## Skinning

A skinned surface has bone weights (`.10`) and bone indices (`.11`): 4 bytes per vertex each.

- **Weights** are bytes that add up to about 255 per vertex: 174,602 vertices sum to 255, 3,697 to 256, 11 to 254 (rounding), and **1,421 vertices have all four weights 0** (unweighted vertices; a reader must not divide by the sum). Usually one weight is non-zero (169,621 vertices), two for 7,723, three for 894, four for 72.
- **Indices** refer to the **bone palette** (`.18`) of the same surface, not to skeleton bones directly: the largest index is below the palette length in all 333 surfaces. The palette entries are presumably skeleton bone numbers (example from `Kleer.bmf`: `[4, 5, 6, 7]`); the skeleton format (`.skl`, T01-009) must confirm it.

## Materials

`CRenderMeshSurface.16` refers to an embedded `CShaderPreset` (258 surfaces), to an external
`.mtr` through `RFIL`/`EXOB` (7 surfaces), or is null (88 surfaces). A preset holds a list of
`CShaderConfig` objects with shader arguments and an `EPlatform` mask (T01-011, T01-020).

The engine validates the section-local master index in `.20` and resolves it to `.21`.
`mdlGetSurfaceMaterial` reads the master's preset when that link exists. Of the 88 null
fields, 23 point to a master with a preset, 11 point to a master with no preset, and 54
have no master. A null result entering `shaExecutePreset` uses the default shader and
arguments in the ordinary shader path. A forced shader can override that selection.
These runtime rules come from a static trace.

## Morph maps

`CSurfaceMorphMap` is sparse. Member `.1` is the map `IDENT`, `.2` is the relative flag,
and `.3` is a list of unique surface-local `ULONG` vertex indices. Members `.4` and `.5`
have one `Vector3f` position and normal entry per index. Member `.6` is an optional
`Vector4f` tangent array. It is empty in 28 maps and has the same count as `.3` in 12.

All 40 maps across 14 surfaces have the relative flag set. The vertex-index order
matches the order of the entries. Adding the normal entry to its base normal produces a
result closer to unit length than treating the entry as a normal in 15,868 of 15,969
cases. Absolute morphs and tangent W behavior were outside the observed cases.

## Collision mesh

31 of 34 collision objects use render geometry (`.19=1`). Member `.20` selects a holder
from `CRenderMesh.13`. Members `.21` and `.23` are **byte offsets** of the positions
and normals, and `.22` is the vertex count. Each selected stream contains `.22` triples
of floats. Member `.12` is the own-position array. It is empty even in the three objects
with `.19=0`, which have no collision triangles or tree nodes.

Member `.13` stores a `u16` index list (30 populated objects). Member `.14` stores a
`u32` alternative (one populated object, `KlodovikLogo.bmf`). Three indices make a
triangle. Member `.9` has one collision surface ID per triangle, referencing the table
in `.11`. That table pairs a double-sided flag with a material-attributes pointer.
The IDs are in range in 30 meshes. `KlodovikLogo.bmf` has out-of-table entries, so a
reader needs a bounds check. Member `.26` has one byte of packed edge/vertex flags per
triangle.

### Collision tree

Member `.16` is a flat array of four-`u16` nodes, rooted at index 0. Each node has a
triangle count in `.3` and a starting index in `.4` into the triangle-ID list `.17`.
Internal nodes can hold triangles as well as children. Ranges cover the reference list
without overlap in all 31 populated objects. A triangle ID can occur in several ranges.

The 217 leaves have `.1` and `.2` set to `0xFFFF`. In the 186 internal nodes, the upper
two bits of `.1` give the axis (0=X, 1=Y, 2=Z). The lower 14 bits are the position in
units of 1/16,384 of the mesh box (`.18`) extent on that axis. Add that scaled offset to
the mesh box minimum to get the split. The below and above children are at `.2` and
`.2 + 1` respectively. The recursive traversal clips its current box at that split.
The decode uses the full mesh box as the quantization domain at every depth.

The verifier found 67,327 triangles and 77,378 references. In 77,336 references the
triangle's bounding box overlaps the reconstructed node box at a tolerance of 0.0001.
That comparison is evidence for split decoding and child order. It is unsuitable as
a file-validity condition.

## The data after `EDOB`

18 meshes carry extra records after `EDOB`. They follow the rule in `ctsemeta-objects.md` ("The EDOB section"): a count, then `(object id, type id, value)` records. In the meshes they are editor-side data (`CEditMesh`, `CPackedEditMesh`, `CCustomDataHolder`, surface/morph map transfer data, `CUVMapViewCustomData`, `CPlacementData`) plus extra `CShaderPreset`/shader-argument/graph-modifier objects (graphs such as `CSineGraph`). No surface's preset refers to them: all 258 embedded presets are in the main section. **A runtime reader can ignore the extra records** (after checking that they decode, or simply not reading further).

## Invariants for a reader

1. Decode the main object section by the rules; fail if the cursor is not on `EDOB`.
2. For each surface: reject unless `T × 3 × 2` bytes fit at the index offset, and every stream (`V × element size` bytes) fits at its offset in its buffer; reject unknown element codes.
3. Reject an index `≥ V`.
4. Treat a buffer number of 255 as "stream absent" and a null holder as "slot unused".
5. Treat all-zero bone weights as "no skinning for this vertex".
6. Never allocate from `T` or `V` before the bounds check (cap by buffer sizes).

## Test vector

`Content/SeriousSam2/Models/Crumbs/Grass/Grass_14/Grass_14.bmf` (6,892 bytes), one LOD, one surface:

| Item | Value |
|---|---|
| LOD distance (`CRenderMeshLOD.6`) / mesh `.8` | 1000.0 / 1000.0 |
| LOD box | min (−0.5, 0, −0.5), max (0.5, 1, 0.5) |
| surface `.2` / `.3` / `.4` | 8 triangles / 16 vertices / 1 UV stream |
| index path (`.6`) | code 135, buffer 0, offset 0 (index buffer: `GfxBufferType 0x5F`, **48 bytes** = 8 × 3 × 2) |
| position (`.7`), normal (`.8`) | code 133, buffer 0, offsets **0** and **192** (16 × 12 bytes each) |
| UV (`.14[0]`) | code 132, buffer 0, offset **384** (16 × 8 = 128 bytes) |
| vertex buffer | `GfxBufferType 0x1F`, **512 bytes** = 192 + 192 + 128 |
| first two UVs | (0.02, 0.02), (0.02, 0.98) |
| tangent, skin slots | absent (buffer 255) |
| material | embedded `CShaderPreset` |

## Open questions

- Confirm camera-space handedness, shader-specific UV transforms, and absolute morphs in the original runtime if a consumer needs those conventions.
- Remaining collision questions include the individual edge flag bits and the out-of-table collision surface IDs.
- Whether the bone palette holds skeleton bone indices (needs T01-009).
- The `.mdl` model files point to `.bmf` meshes; how a model selects LODs and materials is T01-008.
