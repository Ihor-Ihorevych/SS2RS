# Geometry primitives and the coordinate system

`ss2_physics::geometry` (T07-001) holds the shapes and the exact queries between them that collision, the character controller,
hit scanning and the world queries are built on. It uses `glam` vectors through `ss2_core::math`.

## The coordinate system

One convention for the whole engine (`ss2_core::math`, `docs/architecture.md`):

- **Right-handed, Y up, -Z forward**, angles in **radians**.
- A **unit is one metre** by convention. The scale and axes of the original game's files are the business of the format loaders, which
  convert on the way in; nothing downstream converts. (What the original's unit is will come from the format specs.)
- Quaternions are unit quaternions; `Obb::new` normalises its rotation.

## What there is

| Shape | Queries |
|---|---|
| `Aabb` (axis-aligned box) | ray cast, signed distance, closest point, contains, overlaps, grow, union, **sweep against another box** |
| `Sphere` | ray cast, signed distance, overlap with a sphere or a box, **sweep against a sphere, a plane and a box** |
| `Capsule` (a segment with a radius) | ray cast, signed distance, overlap with a sphere, a capsule or a box |
| `Obb` (oriented box) | ray cast, signed distance, closest point, contains, bounding `Aabb` |
| `Plane` (the half-space behind it is solid) | ray cast, signed distance |
| `Ray` (origin and **unit** direction), `RayHit`, `SweepHit` | results: distance, point, normal, and whether the shape was already overlapped |

Conventions every query follows:

- **Solids are solid.** A ray that starts inside a shape hits it at `t = 0` with `inside = true` and the normal pointing back along the ray.
  A ray on the surface going inward hits at 0; going outward or along the surface, it misses.
- **A sweep** moves a sphere or a box by `delta` and returns the fraction `t` of the motion at which it first touches, the centre's
  `position` then, and the contact `normal` (unit, from the thing hit toward the moving shape). Shapes that **already interpenetrate**
  are always reported (`overlapping_at_start`, `t = 0`, the normal is the way to push them apart), even when moving away, because a caller
  that resolves collisions has to hear about it. Shapes that only **touch** are not overlapping: moving away or sliding along a touching
  face is no hit, moving into it is a hit at 0.
- **Bad input is no hit**, not NaN: a zero or non-finite direction gives no `Ray`, a non-finite `delta` gives no sweep.
- A sweep of a sphere against a box is exact, including its rounded edges and corners: the sphere's centre is a ray against the box grown
  by the radius, which is three slabs plus twelve capsules along the edges.

## Platform-stable

The queries use only `+ - * /`, `sqrt`, `min`, `max`, `abs` and comparisons on `f32`, no trigonometry (whose results differ between
math libraries) and no fused multiply-add, so the same inputs should give the same bits on every machine. A test checks that repeating a
sweep gives identical bits. **That the bits agree between x86_64 and aarch64 has not been tested** (it is T07-026); the claim is by
construction, not by measurement.

## How it was tested

Sixteen unit tests, written first and seen to fail on the missing API, and then **property tests against a reference**: the signed
distance of each shape.

- For 4,000 random rays per shape (box, sphere, capsule, oriented box, plane) aimed near the shape: a hit lies on the surface, its normal
  is unit and faces the ray, and **no sampled point of the ray before the hit is inside**; a miss means **no sampled point of the ray is
  inside**. Hits and misses are both required to be common, or the test fails.
- For 3,000 random sphere sweeps against a box, a sphere and a plane: at the hit the sphere is exactly touching, it was **not
  overlapping at any sampled earlier moment**, and a miss means it never overlapped along the path.
- Box sweeps: the boxes touch at the hit and not clearly before; a miss means they never overlap along the path.
- Capsule against box overlap against a sampling of the capsule axis; closest points against signed distances.
- Exact cases: faces, the diagonal approach to a corner (travelled distance and normal), a sphere passing a corner without touching, a
  diamond-shaped oriented box seen corner-on, resting contact and sliding.

Mutation checks (each fails at least one test): the slab tie rule, the inside check of the sphere and of the plane, the capsule's side
not limited to its segment, the capsule edges of the box sweep missing, the oriented box normal not rotated, the segment's closest point
unclamped, the capsule-box search switched off. **A mutant survived**: removing the eight corner spheres from the sphere-box sweep changed
nothing, because the round ends of the edge capsules *are* the corners; the spheres were redundant and I removed them. One more is
equivalent (the default direction when no overlap axis is found cannot be reached).

Two of my tests were wrong, not the code, and I fixed them: random rays missed a small sphere too often for the hit counts to be
meaningful (the rays are now aimed near the shape), and a slide that exactly grazed a box edge is a coin flip of rounding (now it stops
short of the edge; an exact tangent is not tested).

## Not covered (follow-up T07-001b)

- **Oriented box overlap** (separating axis test) and **capsule and oriented-box sweeps**; sweeps of a box against a sphere.
- **Triangles and meshes** (the static world: T07-002, T07-003).
- **A single set of primitives for the whole engine.** `ss2-render-api` has its own `Aabb`, `Sphere` and `Plane` (T04-004, for frustum
  culling) because the render contract may not depend on the physics crate. They should become one set in `ss2-core`; that is a
  cross-crate change that needs an RFC.
- **The cross-architecture bit-for-bit check** (T07-026) and benchmarks of the queries (T07-017).
