# Physics queries

`ss2_physics::query` (T07-004, `docs/rfcs/0007-physics-query-api.md`) is how gameplay asks the physical world what is there. This page
is the usage guide; the RFC has the reasons and the alternatives.

```rust
use ss2_physics::query::*;

let mut world = QueryWorld::new();
world.set_static(level_mesh, material_per_triangle)?;       // once per level
// every tick:
world.set_dynamic(colliders_this_tick);                    // rebuilt, so there is no state to keep in step

// A bullet: the nearest thing along a ray, ignoring the shooter.
let filter = QueryFilter { exclude: Some(shooter), ..QueryFilter::default() };
if let Some(hit) = world.raycast(&ray, 200.0, &filter) {
    spawn_impact(hit.point, hit.normal, hit.material);       // footsteps and impacts pick sounds by material
    if let Some(who) = hit.collider { damage(who); }         // None: the static world
}

// A rocket: move a sphere along its path.
let hit = world.shapecast(&Shape::Sphere { radius: 0.2 }, from, delta, &filter)?;
// Standing on something: cast a sphere down and look at the normal.
// Triggers and splash: what does a shape here touch?
let mut touching = Vec::new();
world.overlap(&Shape::Sphere { radius: 5.0 }, centre, &filter, &mut touching)?;
```

## The answers

`Hit { distance, point, normal, material, collider, started_overlapping }`: `distance` is the **travel** along the ray or of the shape, not a
fraction; `normal` is a unit vector away from the thing hit; `collider` is `None` for the static world. **`started_overlapping`** means the
ray began inside or the shape already overlapped: then `distance` is 0 and `normal` is the way out, so a caller that resolves collisions
can push apart. `Overlap { collider, material }` entries are appended to your vector: the static world first (one per distinct material,
sorted), then moving colliders by id.

## What is supported

| Query | Static world | Moving colliders |
|---|---|---|
| `raycast` | yes | sphere, capsule, box |
| `shapecast` sphere | yes, exact: faces, edges and vertices of the triangles | sphere, capsule, box |
| `shapecast` box | **`Unsupported`** | box only (`Unsupported` if a sphere or capsule on the mask is in play) |
| `shapecast` capsule | **`Unsupported`** | **`Unsupported`** |
| `overlap` sphere | yes (the materials of the triangles in range) | sphere, capsule, box |
| `overlap` box, capsule | **`Unsupported`** (ask with `statics: false`) | sphere, capsule, box |

Capsules stand along Y; boxes are axis aligned. What is not supported is **refused with `QueryError::Unsupported`**, never answered
wrongly. Bad sizes, NaN and infinities give `QueryError::BadInput` (a ray cast with a bad length just hits nothing).

## Filters

`mask` (layers: a thing is considered if its layers and the mask share a bit; the static mesh is on all layers unless set), `exclude`
(one collider to ignore, usually the shooter) and the `statics` and `dynamics` switches. A box cast and a box overlap against the static
world are unsupported, so a trigger volume that only cares about moving things passes `statics: false`.

## Determinism

The same world and query give the same answer on every machine: moving colliders are sorted by id when set (so the order given never
matters), overlap results are ordered, and a tie in distance goes to the lower id. A test gives two identical colliders in both orders;
it caught a real bug, below.

## How it was tested

Fourteen tests for the contract and nine more for the sphere-against-triangle sweep it needed (`Triangle::sweep_sphere`,
`TriMesh::sweep_sphere` in `docs/SPATIAL.md`): hits on a floor with the material of the triangle that was hit, moving colliders with their
ids and the nearest winning, every filter, sphere casts stopping a radius above a floor and sliding past an edge, the cast distance being the
travel, **800 random sphere casts against 60 random moving colliders checked against sampled signed distances** (touching at the hit, not
overlapping before it, a miss never overlapping; hits and misses both required to be common), overlaps against the predicates, the static
materials sorted and distinct, the refusals, the input order not changing answers, use through `&dyn SpatialQuery`. The triangle sweep is
checked against sampled distance to the triangle (3,000 random aimed sweeps).

**A real bug was found by the tie test.** The closures that record the winning hit overwrote it when a second hit had exactly the same
distance, while the tree kept the first; the recorded details then belonged to the wrong collider. Fixed in `QueryWorld::raycast` and in
the two `TriMesh` queries (the rule now mirrors the tree's: take a hit only if it is the first or nearer).

Mutation checks (each fails a test): the exclusion in the ray cast and in the shape cast, the box-cast refusal, the static cast distance
as a fraction, the contact point, the sorting of the colliders and of the overlaps, the skipped static overlap, and in the triangle sweep:
only one face side, a missing edge, no overlap check. Two of those (the exclusion in the shape cast, the box-cast refusal) **survived at first**
and I added a test for each. One more is equivalent: in `QueryWorld::raycast` the moving colliders are queried only up to the static hit's
distance, so "a moving hit always replaces the static one" cannot be told apart from the real rule.

## Not covered (follow-up T07-004b)

- **Capsule casts** (the character controller needs them; first in line), box casts against the static world, box and capsule overlaps with
  the static world, rotated boxes and capsules.
- **Several hits** (`raycast_all`) for piercing weapons; **the triangle index** in a hit.
- **The material table** that says what a `PhysMaterialId` means (T07-009), the **entity mapping** for `ColliderId` (the simulation's job,
  T08-002) and **lag compensation** (T08-010).
- **Threads:** queries are `&self`, but running them in parallel is not tested.
