# Baked lightmaps for the playable MVP

- **Date:** 2026-11-27
- **Roadmap:** T00-037 (this note and RFC 0013 amendment 2), T07-034, T04-040, T02-040, T07-035, T03-039
- **Builds on:** [Sky and lightmaps](2026-11-27-sky-and-lightmaps-design.md), [RFC 0013](../../rfcs/0013-level-scene-boundary.md),
  `docs/re/wld-lighting.md` (T01-406, T01-407)

## Intent

Levels are drawn with a flat light from the camera. The original lights static geometry with **baked lightmaps**: one
texture per lit model instance (about 800 in a level), stored in the world file. T01-407 found which UVs they use: the
mesh's UV stream that lies inside 0..1 without overlapping (the unwrap), usually the first declared slot and not used by
the material (86.6 % of lit instances have exactly one such stream). Drawing them is the largest visual step left.

## Goals and non-goals

- Lit instances are drawn **full bright**: base colour × lightmap, no run-time light (`docs/re/lighting.md`).
- Instances without a lightmap, or whose mesh has no single unwrap-like stream, keep today's flat light.
- Non-goals: dynamic lights, shadows, the ambient-occlusion masks, specular, lightmap atlases.

## Approaches (owner's choice: 1)

1. **One draw per lit instance with its own lightmap bound** (chosen): simple, about 1,000 draws per level, fine on a
   desktop GPU; lit instances no longer share a batch. Can grow into 2 without changing the boundary.
2. Pack each level's lightmaps into a few atlases and remap the lightmap UVs: batching kept, faster on weak GPUs, costs
   a packer and padding against bleeding.

## Design

**Boundary (RFC 0013 amendment 2):**
- `MeshData::lightmap_uvs: Option<Vec<[f32; 2]>>`: the lightmap's UV per vertex (same length as `vertices`), present
  only on meshes that have one. `MeshVertex` stays 32 bytes.
- `Instance::lightmap: Option<u32>`: index into `LevelScene::textures` of the instance's lightmap; only meaningful
  when the instance's model parts carry `lightmap_uvs`.

**Renderer (T04-040):** `WorldBatch::lightmap: Option<TextureHandle>`; a mesh uploaded with lightmap UVs gets a second
vertex stream (zeros otherwise). A batch with a lightmap draws `base × lightmap` with no light term; without one, as
today. The renderer does not care how many instances a batch has; the app makes one batch per lit instance.

**Readers (clean room):** T02-040: `ss2-format-wld` returns each static model's lightmap (the embedded
`CStaticTexture` of its `CTextureOvenShaderModifier`, decoded like any texture) and the modifier's ids. T07-035:
`load_level` picks each mesh's lightmap stream by the rule of `wld-lighting.md` (the single stream inside [0, 1]
without overlap; none or several: no lightmap UVs for that mesh, a `skipped` line), fills `lightmap_uvs` and
`Instance::lightmap`.

**App (T03-039):** `upload` makes one batch per lit instance (with its lightmap) and keeps grouping the others.

## Open questions (checked by eye)

- **Brightness:** whether the original multiplies by the lightmap once or with an overbright factor (×2 is common in
  engines of the time). Start with ×1; compare by eye and record the result.
- **UV orientation** of the lightmap (flipped or not) and the 13 % of instances without a single unwrap stream.

## Verification

Unit and golden tests on `test_room()` with a procedural lightmap on the floor; data-gated: every world loads with
lightmaps on most lit instances (counts against `wld-lighting.md`); real levels by eye (`1_1_jungle`, `dm_stadium`,
a city level): shadows under objects and along walls where the original has them.
