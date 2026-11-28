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
// A character: move its upright capsule; the hit names the floor triangle it landed on.
let hit = world.shapecast(&Shape::Capsule { half_height: 0.6, radius: 0.4 }, from, delta, &filter)?;
// A piercing shot: everything along the ray, nearest first.
let mut pierced = Vec::new();
world.raycast_all(&ray, 200.0, &filter, &mut pierced);
// Triggers and splash: what does a shape here touch?
let mut touching = Vec::new();
world.overlap(&Shape::Sphere { radius: 5.0 }, centre, &filter, &mut touching)?;
```

## The answers

`Hit { distance, point, normal, material, collider, started_overlapping, triangle }`: `distance` is the **travel** along the ray or of the
shape, not a fraction; `normal` is a unit vector away from the thing hit; `collider` is `None` for the static world, and then `triangle`
is the index of the triangle hit in the mesh given to `set_static` (for decals and per-triangle effects; `None` for a moving collider).
`point` is the contact on the thing hit, except for a box cast, where it is the box's centre at the hit. **`started_overlapping`** means the
ray began inside or the shape already overlapped: then `distance` is 0 and `normal` is the way out, so a caller that resolves collisions
can push apart. `Overlap { collider, material }` entries are appended to your vector: the static world first (one per distinct material,
sorted), then moving colliders by id.

## What is supported

| Query | Static world | Moving colliders |
|---|---|---|
| `raycast`, `raycast_all` | yes | sphere, capsule, box |
| `shapecast` sphere | yes, exact: faces, edges and vertices of the triangles | sphere, capsule, box |
| `shapecast` capsule | yes, exact (T07-004b) | sphere, capsule, box (exact closed forms) |
| `shapecast` box | yes, exact (separating axes in motion, T07-004b) | sphere, capsule, box |
| `overlap` sphere | yes (the materials of the triangles in range) | sphere, capsule, box |
| `overlap` box, capsule | yes (T07-004b) | sphere, capsule, box |

Capsules stand along Y; boxes are axis aligned (rotated ones are T07-046). Every combination is answered; a future one an
implementation cannot answer correctly is **refused with `QueryError::Unsupported`**, never answered wrongly. Bad sizes, NaN and
infinities give `QueryError::BadInput` (a ray cast with a bad length just hits nothing).

**How the new casts work** (all standard geometry). A capsule is a segment grown by a radius, so against a triangle the first contact is
one of: an end of the segment against the triangle (a sphere sweep), a corner of the triangle against the segment (a ray from the corner,
backwards, against the capsule), or an edge against the inside of the segment (a ray against the parallelogram the edge and the segment
span, moved out by the radius); the earliest is exact (`Triangle::sweep_capsule`, any segment direction). A box against a triangle is
the separating axis test in motion over the 13 candidate axes (`Triangle::sweep_aabb`). Against moving colliders both shapes stand along
Y, so a capsule against a sphere or a capsule is a point against one bigger capsule, against a box a sphere against a taller box, and a
box against a sphere or a capsule is the same thing turned round. A shape that only **grazes** a surface it slides along (a capsule
resting on a floor and walking) is not a hit; one that starts inside is reported at 0 with the way out.

`raycast_all` appends every triangle the ray crosses and every moving collider it enters (once each), nearest first; at the same distance
the static world comes first, by triangle index, then the colliders by id.

## Filters

`mask` (layers: a thing is considered if its layers and the mask share a bit; the static mesh is on all layers unless set), `exclude`
(one collider to ignore, usually the shooter) and the `statics` and `dynamics` switches. A trigger volume that only cares about moving
things passes `statics: false`, which also saves the mesh query.

## Determinism

The same world and query give the same answer on every machine: moving colliders are sorted by id when set (so the order given never
matters), overlap results are ordered, a tie in distance goes to the lower id, and between triangles of the static world to the lower
triangle index (so the answer does not depend on the shape of the tree). A test gives two identical colliders in both orders;
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

**T07-004b tests** (thirty-four more: twenty-eight unit, six property): hand-made capsule cases that only one of the three contact kinds can find (an end on a
floor, a lying capsule's middle on a wall's top edge, an upright capsule's side on a triangle's corner), starting in contact, resting and
sliding (no hit) against resting and pressing down (a hit at 0), zero-length casts, degenerate triangles, a 20,000-unit cast, ties between
duplicate triangles going to the lower index; **3,000 random capsule sweeps** (any direction) checked against the distance from the segment
to the triangle found by a ternary search, and against sphere sweeps from five points of the axis (a capsule is never later); **3,000
random box sweeps** checked against clipping the triangle to the box (a reference that shares no code with the separating axes); the
capsule and box overlaps against the same references; the mesh queries against a loop over every triangle; `raycast_all` against a loop;
capsule and box casts against random moving colliders against sampled distances; property tests that capsule and box sweeps never tunnel,
that a capsule stops outside, that a capsule is never later than a sphere sweep from its axis, and that shape casts do not depend on the
order of the colliders. Mutation checks (each fails a test): each of the three capsule contact kinds removed, one end of the segment
skipped, the triangle normal or two edges dropped from the box axes, the triangle tie-break reversed, the taller box of the capsule-box
and box-capsule cases, the summed straight parts of two capsules, the `raycast_all` order. **One survivor**: in the box sweep, `enter >=
exit` against `enter > exit` differs only when the box touches an edge for a single instant, which no test produces.

## Not covered

- **Rotated boxes and capsules** in `Shape` (ragdolls, vehicles): T07-046. The triangle-level capsule sweep already takes any direction.
- **The material table** that says what a `PhysMaterialId` means (T07-009), the **entity mapping** for `ColliderId` (the simulation's job,
  T08-002) and **lag compensation** (built as a library: `docs/LAG_COMPENSATION.md`).
- **Threads:** queries are `&self`, but running them in parallel is not tested.
