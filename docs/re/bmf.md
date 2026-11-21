# `.bmf` binary meshes

- **Roadmap task:** T01-007
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object layout, the buffer and stream layout, the index format, the bounds, the skinning data and the LOD list (all checked on every mesh, see "Evidence"); **medium** for what the stream slots and the bone palette mean; **low** for the morph maps, the collision mesh and several scalar fields (listed under "Open questions"). Scope: all **76** distinct format-8 `.bmf` files of the retail install with patches.

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`.

## Summary

A `.bmf` is a CTSEMETA file holding **one `CMesh`**. The mesh has a render part (`CRenderMesh`: a list of LODs, each with surfaces) and, in 34 of 76 files, a `CCollisionMesh`. The vertex and index data are not parsed into structures: they sit in raw byte blobs inside `GFXHANDLE` objects, and every surface says where inside those blobs its streams start (an **element path**: element code, buffer, byte offset). Vertex data is **planar**: each attribute is its own contiguous array. Indices are `u16` triangle lists, local to the surface.

The files hold 125 LODs, 353 surfaces, 213,294 triangles and 216,784 vertices in total.

## Evidence

`tools/re/bmf_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches. For **every** one of the 353 surfaces: all streams and the index list fit inside their buffers; positions lie inside the LOD's bounding box; normals have unit length (within 0.02, in at least 99 % of the vertices); every index is smaller than the surface's vertex count; the number of UV streams equals surface member 4. For **every** buffer (125 index buffers, 125 vertex buffers) the streams of its surfaces **tile the buffer exactly**, with no gap and no byte left over, using the element sizes given below. 120 surfaces have a tangent stream whose fourth component is exactly ±1. For all 333 skinned surfaces the largest bone index is smaller than the surface's palette. `tools/re/edob_verify.py` checks the data after `EDOB` (below).

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
| `.7`, `.8`, `.9` | three floats; in all 76 files `0 ≤ .7 < .8` and `.8 = .9` (values such as 0/990/190/90 and 1000/200/100/150/160). In 69 of 76 files `.8` equals the last LOD's distance. Probably fade/view distances: **meaning unconfirmed** |
| `.11` | always 10.0 |

### `CRenderMeshLOD`

| Member | Type | Meaning |
|---|---|---|
| `.1` | `MeshLODFlags` | 0 in all 125 |
| `.3` | `Box3f` (6 floats: min x,y,z then max x,y,z) | bounding box of the LOD (`Box3f.1`..`.6`) |
| `.4` | reference | always null |
| `.5` | list of `CRenderMeshSection` | exactly **one** section per LOD |
| `.6` | float | switch distance; ascending with the LOD number in all 76 files (for example 150, 300, 500) |
| `.7` | `EPlatform` | platform mask: `0xFFFFFFFF` (83 LODs), `0xFFFFFFFD` (32), `0x3` (4), `0xFFFFFFFE` (4), `0x1` (2). Bit meanings **unknown** |
| `.8` | int | 1 in 74 LODs; 31 distinct values between 0 and 64 in all: **unknown** |

`CRenderMeshSection.1` is the list of `CRenderMeshSurface`.

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
| `.13` | 8 words, all 0 in all 353 surfaces |
| `.14` | 8 element paths: the **UV streams** (slot order as in `.12`); unused slots have buffer 255 |
| `.15` | 8 element paths: always all absent |
| `.16` | reference: the surface's **material** (a `CShaderPreset`), or null (88 surfaces), see below |
| `.17` | reference, always null |
| `.18` | list of ints: the **bone palette** (empty for the 20 static surfaces, 1 to 20 entries for skinned ones) |
| `.20` | int: −1 in 319 surfaces, otherwise 0 to 5 (a material or slot index?): unknown |
| `.22` | list of `CSurfaceMorphMap` (empty in 339 surfaces, 1 to 10 maps in 14) |
| `.23` | int, 32 distinct values from 1 to 55: unknown. In `Grass_14.bmf` it is 2 |

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

## Skinning

A skinned surface has bone weights (`.10`) and bone indices (`.11`): 4 bytes per vertex each.

- **Weights** are bytes that add up to about 255 per vertex: 174,602 vertices sum to 255, 3,697 to 256, 11 to 254 (rounding), and **1,421 vertices have all four weights 0** (unweighted vertices; a reader must not divide by the sum). Usually one weight is non-zero (169,621 vertices), two for 7,723, three for 894, four for 72.
- **Indices** refer to the **bone palette** (`.18`) of the same surface, not to skeleton bones directly: the largest index is below the palette length in all 333 surfaces. The palette entries are presumably skeleton bone numbers (example from `Kleer.bmf`: `[4, 5, 6, 7]`); the skeleton format (`.skl`, T01-009) must confirm it.

## Materials

`CRenderMeshSurface.16` is one of: an **embedded `CShaderPreset`** object in the main object section (258 surfaces); an **external reference** to a `.mtr` file listed in `RFIL`/`EXOB` (7 surfaces in 3 files, for example `Minigun/Sources/Display.mtr`); or **null** (88 surfaces; what the engine draws then is unknown). A preset holds a list of `CShaderConfig` (shader arguments object + `EPlatform` mask); the argument classes (`CShadeShaderArgs`, `CPixelightShaderArgs`, `CReflectionShaderArgs`, `CSpecularShaderArgs`) carry texture references and parameters. Their meaning belongs to the material and shader specs (T01-011, T01-020).

## Morph maps (low confidence)

`CSurfaceMorphMap` has six members: `.1` (an id), `.2` (a small number), `.3` (a list of vertex indices), `.4` (a list of `Vector3f`, one entry per listed vertex, apparently position offsets), `.5` (another list of the same length, not decoded further), `.6` (a list, empty in the sample). It is a **sparse** morph target: only the listed vertices move. 14 surfaces carry morph maps (character faces such as `Female_Ingame.bmf`). The exact semantics are open.

## Collision mesh (low confidence)

34 meshes have a `CCollisionMesh` object. It carries an index list (`.13`, 3 per triangle), a list `.17` (the entries 0 to 7 in the sample: one per triangle), a bounding box (`.18`), a list of `CCollisionMeshNode` (`.16`; the one node in the sample has the fields 65535, 65535, 8, 0) and two counts (`.22`, `.23`); in `Grass_14.bmf` they are 16 and 192 = 16 × 12, equal to the render surface's vertex count and position bytes. The collision mesh therefore seems to **reuse the render positions**. The full layout is left to a follow-up (T01-055).

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

- Winding order of the triangles, handedness and units of the coordinate system, and the orientation of the UV `v` axis: **not tested** (they need a rendered comparison or the engine's conventions).
- Meaning of `EPlatform` bits in `CRenderMeshLOD.7` (which LODs a PC uses), of `CRenderMeshLOD.8`, of `CRenderMesh.7`/`.8`/`.9`, and of surface `.13`, `.20`, `.23` → **T01-055**.
- What the engine uses for the 88 surfaces without a preset.
- Morph map semantics and the collision mesh layout → **T01-055**.
- Whether the bone palette holds skeleton bone indices (needs T01-009).
- The `.mdl` model files point to `.bmf` meshes; how a model selects LODs and materials is T01-008.
