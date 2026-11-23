# Physics debug drawing

`ss2_game::physics_debug` (T07-016) draws the physical world for a developer into a `DebugDraw` (`docs/DEBUG_DRAW.md`) under the
**`physics`** overlay. Nothing is recorded unless that overlay is on (`r_debug physics`), so the calls can stay in the code. The renderer's
line pass that puts the lines on the screen is T04-018b.

| Call | Draws |
|---|---|
| `draw_colliders(debug, colliders)` | each moving collider by its shape (box, sphere, capsule), yellow |
| `draw_capsule(debug, capsule, color)` | a sphere at each end and four lines along the sides, in the capsule's own axis |
| `draw_obb(debug, obb, color)` | the twelve edges of an oriented box, rotated |
| `draw_bvh(debug, tree, max_depth)` | the box of every BVH node down to a depth (the root is 0): inner nodes blue, leaves yellow |
| `draw_ray_cast(debug, ray, max, hit)` | green to the hit with an arrow for the surface normal; red to the full length for a miss |
| `draw_shape_cast(debug, shape, from, delta, hit)` | the shape at the start (white), the path, and where it stopped (red) with the contact normal, or the shape at the end of the path (green) |
| `draw_mesh(debug, mesh, max_triangles)` | the edges of the first triangles of a mesh, green; the cap keeps a big level from filling the buffer |

`Bvh::visit_nodes(depth, bounds, is_leaf)` was added to `ss2_physics::spatial` for the BVH overlay; it visits every node once, parents
first.

## Where it lives and why

It is in `ss2-game`, not in `ss2-physics`: drawing needs `ss2-render-api`, and the physics crate is sim-safe and should not depend on
the renderer's contract. `ss2-game` therefore gained a dependency on `ss2-physics` (and already had `ss2-render-api`).

## Tests

Eight tests here and one for `visit_nodes`, written first and seen to fail on the missing API: nothing is drawn with the overlay off (nor
with another overlay on), the line counts and positions of a capsule (vertical and along another axis), an oriented box (checked by how far
it **reaches along x once rotated**), the BVH down to a depth (node counts match `visit_nodes`, the root blue and the last leaf yellow), ray
hits and misses, shape casts (the shape stopping 6 along the path), colliders by shape, and the triangle cap. Seven deliberate breakages
each fail a test; **three survived at first** (the box rotation ignored, the leaf and inner colours swapped, the stop position shifted: all
were only checked by counting lines) and I strengthened the tests and re-ran them.

## Not covered

- **Contacts and contact normals of the character controller, rigid bodies and sweeps over time** (they do not exist yet: T07-005, T07-007).
- **Triangle normals, the heat of a BVH query** (which nodes a ray visited) and **labels** (the material name at a hit): small additions
  when something needs them.
- The **line pass** and the `r_debug` command (T04-018b).
