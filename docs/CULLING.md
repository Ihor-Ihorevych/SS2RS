# Culling primitives and TAA jitter

`ss2_render_api` (T04-004) has the geometry the renderer needs to decide what to draw. It is plain math on the engine's types
(`ss2_core::math`), with no GPU, so the simulation-side extract and the tests can use it too.

## What was already there

`Camera` (`crates/ss2-render-api/src/camera.rs`): position, rotation and a `Projection`: perspective with an **infinite far plane**
or orthographic, both in **reverse-Z** (near is depth 1) for wgpu clip space; `view_matrix`, `projection_matrix`, `view_projection`,
`is_valid`.

## What this adds

| Item | Use |
|---|---|
| `Frustum::from_camera(&camera, aspect)` / `from_view_projection(matrix)` | the planes of what a camera sees, by Gribb and Hartmann extraction; **five planes for a perspective camera** (no far plane), six for an orthographic one; an **invalid camera or a matrix with NaN sees nothing** instead of everything |
| `Frustum::contains_point`, `test_sphere`, `test_aabb` | `Visibility::{Outside, Intersecting, Inside}`; **conservative**: never `Outside` for something a pixel could show (it can say `Intersecting` for a shape just outside a frustum corner, as the standard plane test does) |
| `Aabb` (`from_points`, `corners`, `center`, `half_extents`, `transformed(matrix)`), `Sphere` (`from_aabb`) | bounding volumes; a transformed box is the box around the transformed corners |
| `jitter_for_frame(frame, width, height)` and `Camera::projection_matrix_jittered(aspect, jitter)` | the sub-pixel offset of **temporal anti-aliasing**: a Halton (2, 3) sequence that repeats every 8 frames, within half a pixel, in normalized device coordinates; depth is untouched; zero jitter returns the plain matrix. **Cull with the unjittered frustum.** |

```rust
let frustum = Frustum::from_camera(&camera, aspect);
for renderable in &world.renderables {
    if frustum.test_aabb(bounds_of(renderable)) == Visibility::Outside { continue; }
    draw(renderable);
}
let jittered = camera.projection_matrix_jittered(aspect, jitter_for_frame(frame, width, height));
```

## How it was tested

Unit tests for each shape and plane, a frustum that follows a moved and rotated camera, the orthographic far plane, and **two
property tests over 20,000 random cases** (perspective and orthographic, rotated cameras): a point is visible to the frustum exactly
when its clip-space coordinates are inside the clip volume (up to float rounding at the boundary), and a sphere or box with a
visible point inside it is never classified `Outside`. Four deliberate breakages (a lost plane, a box treated as a point, spheres
culled too eagerly, the jitter applied after the projection) each fail the tests meant to catch them.

## Not covered

- A **hierarchy** (BVH, grid, portals) to avoid testing every object: that is the world renderer (T04-009) and the culling policy (T04-024).
- **Occlusion culling** (T04-025) and **LOD selection** (T04-024).
- Where the **extract** puts the bounding volumes (`Renderable` has none yet): the renderer extract (T04-026).
- Reconstructing the previous frame's jittered matrix for TAA resolve is part of the anti-aliasing task (T04-029).
