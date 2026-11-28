# Spatial acceleration: the BVH, layers and triangle meshes

`ss2_physics::spatial` (T07-003) answers "what is near this ray, sweep or region?" without testing everything. One structure, a bounding
volume hierarchy, serves two jobs:

- the **static world**: build once from the world's triangles (`TriMesh`);
- the **dynamic broadphase**: build again every tick from the bounds of the moving things (`Bvh::build`). A median-split build is
  `O(n log n)` and has no state to keep in step with the simulation, so there is nothing to get out of sync. (An incremental refit is a
  possible later optimisation; it is not needed until profiling says so.)

It is sim-safe and deterministic: ties in the build are broken by the order of the input, queries visit in tree order, nothing uses a hash
map, so the same input gives the same tree and the same answers on every machine.

## `Layers`

A set of up to 32 layers as bits. A thing is on some layers and a query carries a `mask`; the thing is considered when they share a bit.
`Layers::ALL`, `Layers::NONE`, `Layers::bit(n)` (a number past 31 is no layer, not a wrap-around), `intersects`, `contains`, `union`. What
the layers mean (world, player, enemy, projectile, trigger) is the game's decision; this module only filters.

## `Bvh<T>`

Items are `(Aabb, Layers, T)`. The BVH finds **candidates**; the caller's closure makes the exact test.

| Call | What it does |
|---|---|
| `Bvh::build(items)` | median split on the longest axis of the centres; leaves of up to 4; items with non-finite bounds are left out and counted (`dropped()`) |
| `query_aabb(region, mask, visit)` | every item whose box overlaps the region and whose layers match, in tree order |
| `query_ray(ray, max_t, mask, inflate, hit)` | the nearest hit: nodes are visited **nearest first**, items whose box (grown by `inflate`) the ray reaches before the best hit so far are offered to `hit(value, best_t)`, which does the exact test and returns the distance of its hit |
| `len`, `bounds`, `depth`, `check` | the size, the box around everything, the number of levels, and a structure check for tests |

**Sweeps:** pass the radius of a swept sphere as `inflate`; the ray is then fattened by the radius, so every item the sphere could touch is
offered, and the closure measures the exact sphere test (for boxes, `Sphere::sweep_aabb` of `docs/GEOMETRY.md`).

Boxes are padded by 0.0001 in a ray query: a box with no thickness (an axis-aligned floor or wall triangle) would otherwise be missed by the
slab test, since the ray enters and leaves it at the same instant. This was found by a test and is the reason the padding exists.

## `Triangle` and `TriMesh`

`Triangle`: `raycast` (Möller-Trumbore, two-sided; the returned normal always faces the ray), `closest_point`, `aabb`, `is_degenerate`.

`TriMesh::new(vertices, indices)` refuses an index list that is not made of triples, an index past the vertices and a non-finite vertex
(each with a named error), drops triangles with no area and counts them (`dropped_degenerate`), puts every triangle on all layers
(`set_all_layers` changes that), and builds the BVH. `raycast(ray, max_t, mask)` returns the nearest triangle hit (`MeshHit`: distance,
point, normal, triangle index); `query_sphere(center, radius, mask, visit)` visits every triangle within the radius, exactly.

## How it was tested

Seventeen tests for this module, written first and seen to fail on the missing API, against **brute force**:

- the overlap query returns exactly the brute-force set for 300 random regions and masks over 600 boxes;
- the nearest-hit ray query agrees with brute force for 600 random rays over 500 boxes (hits and misses both required to be common);
- the tree is well formed (every node's box contains its children, every item is in exactly one leaf, depth near `log2 n`) for 1 to 1000
  items, and 200 identical boxes do not break the build;
- a **box made of 12 triangles agrees with the box** for 3,000 random rays; random triangle soup agrees with brute force for rays and
  for sphere overlap; the closest point of a triangle is checked against a sampling of the triangle;
- bad meshes, degenerate triangles, layers and masks, empty trees, determinism of the visiting order;
- **efficiency:** over 3,000 boxes a ray query runs the exact test on **1.2 items on average** (5.8 when the farther child is visited
  first, which the test rejects at a threshold of 3).

Mutation checks (each fails a test): the padding of flat boxes, the layer check on items, the median split, the sphere distance test, the
normal turned to the ray, the child visiting order. **Survivors, stated plainly:**

- the layer check on **nodes** and the "stop when a node is farther than the best hit" test are pure optimisations; removing them leaves
  every result the same, so no result test can see them (the efficiency test sees only the child order);
- the tie-break by input order is redundant with the stability of the sort;
- the **tolerance at the seam between two triangles** (`1e-6` in the barycentric test) is a defence that no test needs: 400 rays through
  the exact diagonal seam of a box face hit with and without it. I kept it because a miss on a seam is a hole in the world, but it is not
  demonstrated to matter.

## Not covered (follow-up T07-003b)

- ~~**Exact sweeps against triangles**~~: done. The sphere (T07-004), the capsule and the axis-aligned box (T07-004b:
  `Triangle::sweep_capsule`, `Triangle::sweep_aabb`, `TriMesh::sweep_capsule`, `TriMesh::sweep_aabb`, ties to the lower triangle index),
  the capsule and box overlaps (`TriMesh::query_capsule`, `TriMesh::query_box`) and `TriMesh::raycast_all`; see `docs/PHYSICS_QUERIES.md`.
- **A better build** (surface area heuristic) and an **incremental refit** for large dynamic sets, and **benchmarks** (T07-017); the median
  split is simple and balanced, not optimal.
- Loading the **world's triangles** from a `.wld` (T07-002) and the **frozen query contract** with materials and entities (T07-004).
- ~~Box-against-triangle overlap~~: done (`Triangle::overlaps_aabb`, T07-004b).
