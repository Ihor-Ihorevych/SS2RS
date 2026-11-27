# `.skl` skeletons

- **Roadmap tasks:** T01-009 (format), T01-056 (bone lengths, IK and skinning)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object layout, hierarchy, length-field names and serialized IK bounds. The quaternion and matrix conventions are checked on 563 of 564 bones. **Low** for the IK solver and skinning behavior inferred from a static trace, pending runtime confirmation. Data scope is all **31** distinct format-8 `.skl` files of the retail install with patches (42 LODs, 564 bones).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`. Related: `bmf.md` (bone weights and bone palettes).

## Summary

A skeleton is a CTSEMETA file with exactly two objects, a `CResourceFile` (object 0) and a `CSkeleton` (object 1). The skeleton is a **list of LODs**; each LOD is a self-contained **list of bones** with a parent, a parent-relative **local transform** (quaternion + translation) and the **inverse bind matrix**. There is no animation data here (that is in `.ans`, T01-010).

## Evidence

`tools/re/skl_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches:

- All 31 files decode; each has the classes `[CResourceFile, CSkeleton]`.
- For **563 of 564 bones**, composing the local transforms (member 4) down the parent chain and inverting the result gives member 5 to within 1e-4. Conjugating each stored quaternion before composition increases the maximum error to about 22.
- Member 6 equals the length of the local translation for 537 bones; for the other 27 the translation is zero and member 6 is exactly 1.0.
- The quaternions have unit length (error below 4e-7). The parent of every bone has a smaller id (42 of 42 LODs) and is present in the same LOD.

T01-056 cross-checked the serialized fields with the type registration, bone accessors,
IK and model-render paths in the installed 32-bit `Engine.dll`, plus the editor's IK flag
operations in `EditKit_Skeleton.dll`. The retail data checks used the 2.100 patch set.
The analyzed binary identities are SHA-256
`da6efc9f72637eb3b6f48eadca2107be89b09c00618b6e72d5d3632938a7d851` (Engine) and
`6208c40fc02e5d6790d10b86e08d9c08ecf6fc57dc15804c8aaaf86221a2da1f` (editor).
The verifier reads only archive data.

## Objects and members

```
CSkeleton
  CResource.{1,5,3,2}   as for the other resources (1, -1, 0, -> CResourceFile)
  CSkeleton.1           list of CSkeletonLOD
CSkeletonLOD
  .1  float             switch distance (ascending with the LOD number in all 31 files)
  .4  EPlatform         platform mask: 0xFFFFFFFF in all 42 LODs
  .2  list of SkeletonBone
SkeletonBone
  .1  int               bone id
  .2  int               parent bone id (0 = no parent)
  .3  float             authored bone length (sb_fLength)
  .4  QuatVect          local transform: .1 Quaternion4f (members 1..4 = x, y, z, w), .2 Vector3f (translation)
  .5  Matrix34f         inverse bind matrix: members 1..12 = 3 rows of 4, row-major
  .6  float             parent-offset length (sb_fOffsetLength)
  .7  Vector3f          lower Euler angle limits in radians (x, y, z)
  .8  Vector3f          upper Euler angle limits in radians (x, y, z)
  .9  SkeletonBoneIKFlags  (4-byte enum, one bit per enabled IK component)
```

### Ids and parents

- Bone ids start at 1; 0 means "no parent". Within a LOD every parent has a smaller id than its child, so one pass in file order builds the hierarchy.
- LOD 0 has the ids `1..n` in order. The lower LODs keep the **ids of LOD 0** but have fewer bones (10 of 11 lower LODs; one skeleton goes from 40 bones to 13), so their ids have gaps; in all 11 lower LODs the ids are a subset of LOD 0's. Parents and local transforms of the shared bones are **not** always identical to LOD 0 (identical in only 4 of 11): treat each LOD as a self-contained skeleton.
- 40 of the 42 LODs have a single root (`parent = 0`); 2 have two roots.

### Local transform (member 4)

The quaternion is stored as **x, y, z, w** (the identity is `0, 0, 0, 1`). The local matrix is `[R(q) | t]` with `R(q)` the usual rotation matrix of the quaternion and `t` the translation, applied to **column vectors**, and
`world(bone) = world(parent) × local(bone)`, with `world(0)` = identity.

### Inverse bind matrix (member 5)

`Matrix34f.1`..`.12` are the **rows** of a 3×4 matrix: `.1–.4` the first row, `.5–.8` the second, `.9–.12` the third; the last column (`.4`, `.8`, `.12`) is the translation. It is the **inverse of `world(bone)`**: it takes a bind-pose position into the bone's space. For the example below, bone 1 has the world position (0, 1.55, 0) and the stored matrix translation is (0, −1.55, 0).

Bone 6 of `circularsaw_l.skl`, referenced by `circularsaw_fp_l.mdl`, is the exception.
Its local rotation is within about 3e-7 of identity as a matrix. The stored inverse bind
contains a quarter turn about Y.

The default local pose uses member 4. The render evaluation composes the pose down the
hierarchy and right-multiplies each evaluated bone matrix by the **stored member 5**. The
render palette copies those results for CPU vertex transforms and shader skinning. The
absolute-placement query removes the stored inverse bind from the cached result to recover
the unskinned bone placement. This is a static trace of the installed engine.

With column vectors, the skin transform is the evaluated bone-to-model transform times the stored inverse bind.
Keep member 5 even where it disagrees with an inverse reconstructed from member 4. For the
odd bone, combining the serialized local pose hierarchy with member 5 gives the rounded
rows below. Reconstructing an inverse bind from the hierarchy would erase this residual.

```text
[ 0  0  1   0.716610 ]
[ 0  1  0   0        ]
[-1  0  0  -0.392502 ]
```

### Bone length and parent-offset length (members 3 and 6)

The registration names member 3 `sb_fLength`. `sklGetBoneLength` returns it, and
`CIKMotionHandler::SetupIKMotionHandler` uses that getter when building the chain. The
editor exposes this setting through `edtSetBoneLength`. Values
range from about 0.011 to 10.8 in the retail skeletons. Member 6 is `sb_fOffsetLength`,
the length of the local translation in 537 bones and **1.0** when the translation is
zero in the other 27. Preserve the two values independently. For example, the first
bone in the Levitator skeleton has an authored length of about 0.71 and an offset length
of 1.55.

### Angle limits and IK flags

The constructor starts all three lower bounds at −π and upper bounds at +π. The editor's
enable/disable functions set or clear the flag bit for the supplied component index.
The engine's IK setup tests masks `0x1`, `0x2` and `0x4` separately, in the same order as the
`Vector3f` components in members 7 and 8. The solver retains the current angle for a clear
bit and uses that component's bounds when the bit is set. These flags are used by the IK
path traced here.

All 1,692 angle pairs in the standalone retail corpus are symmetric and approximately
±π. The script compares the signed bounds independently.

| Bound magnitudes in radians | Enabled component | Disabled component |
|---:|---:|---:|
| 3.141590118 | 283 | 1,166 |
| 3.141592741 | 0 | 243 |

Member 9 masks are `0x0` (437 bones), `0x3` (86), `0x7` (33), and `0x1`, `0x2`,
`0x5`, `0x6` (2 each). The nearly full-turn ranges leave the visual effect of tighter
bounds unmeasured. Preserve the signed stored values.

## Relation to meshes

Skinned `.bmf` surfaces refer to bones through a per-surface **bone palette** (`bmf.md`): small positive ints, never 0. For the 44 skinned meshes that have a skeleton in the same model folder (heuristic pairing by the folder above `Sources/`; 15 more meshes have none), the palette entries are directly bone ids of LOD 0 in 15 cases; in 14 more they are shifted by a constant (by 1 in 6 meshes, by 2 in 1, by 3 in 7); in 15 they fit no shift in −6..9 (mostly weapon models). So the palette is **not** simply "bone id" everywhere; a model (`.mdl`, T01-008) probably supplies the mapping. Open.

## Invariants for a reader

1. Require the classes `CResourceFile` and `CSkeleton`; fail on anything else.
2. Per LOD: bone ids unique, each parent 0 or a bone earlier in the list; fail on a missing parent or a cycle.
3. Quaternions: normalise or reject if the length is far from 1.
4. Do not rely on ids being contiguous (lower LODs have gaps); look bones up by id.
5. Do not allocate from list counts before checking them against the bytes remaining.

## Test vector

`Content/SeriousSam2/Models/Enemies/Levitator/Sources/Skeletons/Skeleton.skl` (2,102 bytes): two LODs (distances 50.0 and 150.0, platform mask `0xFFFFFFFF`), each with the same two bones:

| Bone | id | parent | bone length (`.3`) | local rotation (x,y,z,w) | local translation | offset length (`.6`) | inverse bind matrix |
|---|---|---|---|---|---|---|---|
| 1 | 1 | 0 | 0.71 | 0, 0, 0, 1 | (0, 1.55, 0) | 1.55 | rows `[1 0 0 0]`, `[0 1 0 -1.55]`, `[0 0 1 0]` |
| 2 | 2 | 1 | 0.49 | 0, 0, 0, 1 | (0, −0.7, −1.01) | 1.2289 | rows `[1 0 0 0]`, `[0 1 0 -0.85]`, `[0 0 1 1.01]` |

(For bone 2 the world position is (0, 0.85, −1.01), so the stored translation is (0, −0.85, 1.01). Both bones have limits near ±π and `SkeletonBoneIKFlags` 0.)

## Open questions

- Verify the skin palette and tighter IK limits at runtime using the oracle and instrumentation work in T01-038 and T01-039.
- The model spec (`mdl.md`) resolves most embedded mesh palettes. The remaining palette IDs
  and standalone mesh pairings need the model follow-up T01-057.
- Which LOD the engine selects, and whether the skeleton LOD distances relate to the mesh LOD distances (T01-008).
- The one skeleton with extra records after `EDOB` carries editor data (`CEditSkeleton`, `CEditSkeletonLOD`; see `ctsemeta-objects.md`), which a loader can ignore.
