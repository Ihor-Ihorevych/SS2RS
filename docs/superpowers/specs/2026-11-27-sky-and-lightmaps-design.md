# Sky and baked lightmaps for the playable MVP

- **Date:** 2026-11-27
- **Roadmap:** T00-036 (this document and the RFC 0013 amendment), T01-405, T01-406, T07-032, T04-039, T02-039, T07-033, T03-038
- **Builds on:** [Playable MVP](2026-11-27-playable-mvp-design.md), [RFC 0013](../../rfcs/0013-level-scene-boundary.md)

## Intent

The playable MVP draws real levels with a flat light from the camera under a plain blue clear colour. The owner
wants the levels to look like Serious Sam 2: **the level's own sky first**, then **its baked lighting**. Lightmaps
make the biggest difference but cannot be drawn yet: each lit instance's lightmap names a UV map id that none of the
mesh's UV sets carries, and the spec leaves open where those UVs come from (`docs/re/wld-lighting.md`).

## Goals

- Every level with a sky (58 of 59 worlds) draws it around the player: up to six face textures, tinted, at any
  distance behind everything else.
- A time-boxed investigation answers where the lightmap UVs come from, as a reader rule, or documents a dead end.
- Lightmaps are designed and built only after that answer (a later plan).

## Non-goals

Haze, bloom, fog, water surfaces, dynamic lights, shadows, cube-map (single-texture) skies, sky animation.

## Constraints

- **Clean room.** The planning agent wrote the environment and lighting specs, so readers of the sky and lightmap data
  are written by fresh agents; the boundary types and the renderer are format-neutral and written by the planning agent.
- **Face order is unknown.** `environment.md` lists the sky's face texture members (`CCubeRenderable.10`…`.15`) but
  not which direction each is or how each image is oriented. T01-405 settles that from the data.

## Approaches considered

1. **Sky first, lightmap UVs investigated in parallel** (chosen by the owner). The sky is fully readable after a small
   data-only spec; the lightmap question is research with an uncertain end, so it is time-boxed and does not block the sky.
2. **Approximate lighting from the level's placed lights** (11,438 lights with position, range and colour) drawn
   dynamically: available now, but it would not look like the baked original and the colour byte order is unconfirmed.
3. **Wait for lightmaps before anything else:** blocks all visible progress on an open research question.

## Design

### Boundary (RFC 0013 amendment)

`ss2_world::scene::LevelScene` gains `pub sky: Option<SkyData>`:

```rust
pub struct SkyData {
    /// Texture index (into `LevelScene::textures`) per cube face, in `CubeFace` order; `None` leaves that face to `tint`.
    pub faces: [Option<u32>; 6],
    /// Linear RGBA multiplier on every face (the faces' colour where a face has no texture).
    pub tint: [f32; 4],
}
pub enum CubeFace { PosX, NegX, PosY, NegY, PosZ, NegZ }   // index 0..6
```

**Orientation convention** (seen from inside the cube, engine space: right-handed, Y up): each face image's texel
(0, 0) is its top-left corner; "right" and "down" in the image point in these world directions:

| Face | Looking towards | Image right | Image down |
|---|---|---|---|
| `PosX` | +X | +Z | −Y |
| `NegX` | −X | −Z | −Y |
| `PosY` (sky above) | +Y | +X | −Z |
| `NegY` (below) | −Y | +X | +Z |
| `PosZ` | +Z | −X | −Y |
| `NegZ` | −Z | +X | −Y |

The side faces stand upright (image down = world down); the top face is the view after tilting the head back from
facing −Z, the bottom face the view after tilting forward. The loader rotates or mirrors the game's face images into
this convention using T01-405's table, so the renderer needs no knowledge of the game's face order.

### Pieces

| ID | What | Where | Writer |
|---|---|---|---|
| T01-405 | Spec: which sky face member is which direction and orientation; what size, viewer height and colour do | `docs/re/environment.md` | spec agent |
| T01-406 | Spec: where the lightmap UVs come from (time-boxed) | `docs/re/wld-lighting.md` | spec agent |
| T07-032 | `SkyData`, `CubeFace`, `LevelScene::sky`, validation, a procedural sky on `test_room()` | `ss2-world` | planning agent |
| T04-039 | Sky pass: six textured faces around the camera, no depth, drawn before the world, tinted, unlit | `ss2-render` | planning agent |
| T02-039 | `ss2-format-wld` reads the sky entity (face texture paths, colour, size, viewer height) | `ss2-format-wld` | fresh clean-room agent |
| T07-033 | `load_level` fills `LevelScene::sky`, images turned into the convention above | `ss2-world` | fresh clean-room agent |
| T03-038 | The app uploads and draws the sky; the screenshot mode too | `ss2-app` | planning agent |

### Renderer (T04-039)

`Renderer::set_sky(Option<SkyBatch { faces: [Option<TextureHandle>; 6], tint }>)`, dropped with the world. The pass
draws a cube of six quads centred on the camera (translation only from the camera, so it never comes closer),
with depth test and depth writes off, unlit (the world shader with its light term switched off), before the world in
the same render pass; the world then draws over it. Without a sky the clear colour shows as today.

## Verification

- Unit: `SkyData` validation (indices in range, finite tint).
- Golden image of `test_room()` with a procedural sky whose six faces are distinct colours with an arrow pointing to
  each face's top, so a wrong orientation or face order changes the picture; tests that the sky is behind the world
  and does not move with the camera's position.
- Data-gated: every world with a sky entity loads one with at least five face textures; real levels checked by eye
  (village, desert, snow, lava levels), with seams between faces looked at specifically.

## Left to the builders

Exact procedural test pattern, whether faces with different sizes are resized, how the five-face skies' missing face
is filled (tint or the nearest face's edge colour), and the sky cube's draw size.
