# Instancing and batching for hordes

`ss2_render_api::batching` (T04-023) turns a frame's list of `Renderable`s into **as few draw calls as possible**, and packs the per-instance data for one instance buffer. It is device-free: it plans, it does not draw. A scene with a few hundred enemies of three kinds should cost three draw calls, not three hundred.

```rust
let plan = plan_batches(&renderables, &BatchConfig::default())?;   // groups, limits, order
let data = pack_instances(&renderables, &plan);                    // 24 floats per instance, in batch order
for b in &plan.batches { /* draw b.model, b.instance_count instances from b.first_instance */ }
```

## Grouping

Renderables share a draw call when they have the **same model, the same material override and the same flags** (a viewmodel is drawn in its own pass, and shadow casters differ from non-casters), and either neither is animated or animated instancing is allowed.

- **`animated_instancing: true`** (the default): skinned renderables with different clips and times share a draw call. Each instance carries its clip and time, and the renderer reads the pose from them (an animation texture, T04-006/T04-023b). Animated and static renderables of one model are still separate batches because they use different shaders.
- **`animated_instancing: false`**: every animated renderable is drawn alone (a batch of one), and the static ones still group.
- **`max_instances_per_batch`** (1 to 65,536, default 1,024): a group bigger than the device's limit for an instance slice is split into several batches. The limit is the caller's: it should come from the device (T04-001).

## Order, and why it is deterministic

Batches are ordered by model, then material override, then flags, then animated. The instances of a batch go **by `RenderId`**, and a split group keeps that order across its batches. The result **does not depend on the order of the input** (tested on 400 renderables shuffled ten times: identical batches and identical instance order), so a frame draws the same way on every run and machine, and a flicker between two equal draws cannot come from the batcher. `BatchPlan::order` maps each instance slot back to the index in the list that was planned, and every renderable appears **exactly once** (checked, with every batch uniform and within the limit).

Sorting for **transparency** (back to front) is a different concern and a different module (`transparency`, T04-016): batch the opaque draws with this; blended draws that must be in depth order cannot be freely grouped.

## The instance data

`pack_instances` writes `INSTANCE_FLOATS = 24` floats per instance: the **world matrix** (16, column-major, so the translation is at 12 to 14), the **tint** (4), the **animation time** (1), the **clip id** and the **flag bits** (each stored as the bit pattern of the integer in a float, to be read with `floatBitsToUint`; a numeric conversion would lose them), and one pad. A clip id of 0 means "not animated" (real ids are never 0). Each `Batch` knows `first_instance` and `instance_count`, so its slice of the buffer is `[first_instance x 24, (first_instance + instance_count) x 24)`.

`BatchStats` reports renderables, batches, **draw calls saved** (`renderables - batches`), the largest batch and the number of batches of one.

## How it was tested

Ten unit tests and a doctest: grouping by model, material override, flags and animation; animated instancing on and off; a 2,500-instance group split at 1,024 into 1,024, 1,024 and 452 with contiguous slices and id order across them; limits of 1, exactly the group size and one below; bad limits and an empty frame; order independence, completeness and uniformity on 400 random renderables; a horde of 300 animated renderables of three models costing three draw calls; and the packed floats checked field by field (matrix columns against `Transform::matrix`, tint, time, bit patterns of the clip and flags, padding) and slice by slice against the plan. **Twenty-two deliberate breakages: all twenty-two failed a test at once.** I expected a survivor or two; I did not find one, and I do not claim that means the tests are complete, only that these twenty-two edits were all seen.

## Not covered (follow-up T04-023b)

- **No GPU side**: nothing creates the instance buffer, uploads the floats (through the T04-003 staging ring), or issues the instanced draws; the vertex shader that reads the layout does not exist, and neither does the **animation texture** that animated instancing needs (baked bone matrices per clip and frame, read by clip and time).
- **No culling or LOD in the plan**: feed it only what is visible and already LOD-chosen (`frustum`, `lod`); an instance whose LOD differs from its neighbours is a different model and so a different batch, which is the intended behaviour.
- **Interpolation between ticks** is the renderer's job before packing; this module packs the transforms it is given.
- **No measurement**: how many instances per draw a real GPU wants, and the cost of the sort for thousands of renderables, are untested guesses (1,024 is a common but arbitrary limit).
- **Material-then-model ordering** for fewer pipeline switches is not offered; the order is by model.
