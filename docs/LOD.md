# Level of detail and distance culling

`ss2_render_api::lod` (T04-024) decides **which version of a model to draw, or none**. It is pure logic over a `Camera` and a bounding `Sphere`: no GPU, no model data. A renderer asks it once per thing per frame; what the levels *are* (the meshes in a `.bmf`, the impostor texture) comes from the mesh path (T04-005, T04-023).

## The measure: screen coverage

*Coverage* is the sphere's radius as a share of **half the screen height** at its distance: `radius / (distance x tan(fov_y / 2))` for a perspective camera, `radius / (height / 2)` for an orthographic one. 1.0 means the sphere is as tall as the screen; 0.05 means one twentieth.

It does not depend on the window size, so one set of thresholds serves every resolution. What makes a phone or a weak PC draw coarser is the **quality tier**, not its pixel count. It does depend on the field of view (a wider view makes things smaller, as it should).

## A model's `LodSet`

- `switch`: a strictly descending list: level `k + 1` takes over from level `k` when the coverage drops below `switch[k]` (n thresholds give n + 1 levels, at most 255);
- an optional `impostor_below`: below it a flat stand-in is drawn (for far enemies in a horde);
- `cull_below`: below it nothing is drawn (0 never culls by size);
- an optional `max_distance`: nothing beyond it.

`LodSet::new` refuses anything that is not strictly descending in that order (a level that could never be chosen is a mistake in the data), non-finite or non-positive values, and a bad distance. `LodSet::single(cull)` is a model with one level.

A value **exactly on** a threshold is the finer side (tested to the bit with an orthographic view, where coverage is exactly the radius).

## The policy: quality and hysteresis

`LodPolicy { lod_bias, view_distance, hysteresis }`:

- `lod_bias` (-1 to 3, from `QualitySettings`): each +1 judges the thing at **half** its coverage, which is the same as being twice as far away; -1 is finer. `LodPolicy::from_quality(&settings)` takes `lod_bias` and `view_distance` from the quality settings (T04-020).
- `view_distance` (0.5 to 2): multiplies every set's `max_distance`.
- `hysteresis` (0 to 0.5, default 0.1): a thing that was drawn as one level **keeps it until the coverage is 10 % past the threshold**, so a thing standing on a threshold does not flicker as the camera shakes. Implementation: the finest and coarsest choice allowed are computed with every threshold scaled by 1 - h and 1 + h, and last frame's choice is clamped between them; a thing that moved a lot jumps straight to the right choice rather than one step per frame. A thing culled by distance comes back inside the limit minus the band.

`policy.select(&set, &camera, sphere, previous)` returns `Level(n)`, `Impostor` or `Culled`. `LodTracker` keeps the previous choice per `RenderId` (a `BTreeMap`, so iteration is ordered), counts the choices per frame (`LodStats`) and **forgets things not selected in a frame**, so the memory follows the world instead of growing.

## What the tests establish

Nineteen unit tests and a doctest: the coverage formula against hand-computed values (the distance at which a radius-1 sphere is exactly screen-high, twice that), orthographic views ignoring distance, the order of levels, impostor and culling on both sets with and without an impostor, **monotonic in distance** (a sweep of about 9,000 distances: the choice never gets finer as the thing moves away, and every choice is reached), bigger things keeping detail farther, the bias in both directions, the view distance at three scales, **a lower tier never choosing a finer level than ultra at any distance**, the hysteresis (the same 4 % wobble around a threshold produces **no** changes with it and more than ten without it; a 15 % move switches in both directions; 5 % holds), a jump skipping levels, a remembered choice that the (changed) set does not have, the tracker (counts, memory, forgetting) and 22 bad inputs refused.

**Twenty-four deliberate breakages: sixteen failed a test at once; eight survived my first tests.** Five were real gaps (three exact-threshold comparisons: the impostor edge, the cull edge, the maximum distance; the rank of a remembered "culled"; the 255-level bound) and now each fails a test. Three were equivalent and I **removed the code** instead (a branch for non-finite coverage that every comparison already handled, a clamp of a remembered level, and a special case for a remembered impostor that the set lacks: the final clamp covers them).

## Not covered (follow-up T04-024b)

- **Nothing feeds it yet**: there is no mesh path that has levels (T04-005), no impostor generation or drawing, and no renderer that calls it. The thresholds in a real `LodSet` have to come from the model data or a tuning pass; I have not seen the original's LODs.
- **No cross-fade or dithering** between levels (a pop is what you get), no **impostor atlas**, no LOD for **shadow** casters, and no **per-class distance** policy (enemies, props, particles) beyond each set's own `max_distance`.
- **Not measured**: the cost of selecting for thousands of things per frame (it is a handful of float operations each; a benchmark belongs with the renderer's culling pass T04-004).
- The bounding **sphere is the caller's**: the policy uses the centre and radius as given, with no frustum test (`Frustum` does that).
