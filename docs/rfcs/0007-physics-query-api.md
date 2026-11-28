# RFC 0007: Physics query API (contract v0)

- **Number:** 0007 (`docs/rfcs/0007-physics-query-api.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-physics` **plus one** other maintainer (the consumers are `ss2-game` and `ss2-sim`)
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T07-004 (this contract); unblocks T07-005 (character controller), T07-010 (hit scanning), T07-011/012/018 (volumes, mechanisms, water), T07-024 (visibility), T08-010 (lag compensation), T09 weapons and AI

## Summary

`ss2_physics::query` defines the three questions the rest of the engine asks of the physical world, and the answers: **`raycast`**, **`shapecast`** (move a sphere, box or capsule along a line and find where it first touches something) and **`overlap`** (what does a shape at a place touch). They are the trait `SpatialQuery`. An answer says where (`distance`, `point`), how the surface faces (`normal`), what it is made of (`PhysMaterialId`) and what it belongs to (`ColliderId`, with `None` meaning the static world). A `QueryFilter` picks layers, one collider to ignore (the shooter's own body) and static, moving or both. `QueryWorld` is the reference implementation (a static triangle mesh plus the moving colliders of this tick), so callers can be written and tested now.

## Motivation

Almost everything in the game is a query: a rocket flies (shapecast), a minigun fires (raycast), a stomp hits whoever is near (overlap), the player stands on a floor (shapecast down), an enemy sees the player (raycast). If each caller invents its own call, the physics crate cannot change without touching them all. Freezing the contract first lets the controller, the weapons and the netcode's lag compensation be written against it while the implementation grows.

## Detailed design

```rust
pub trait SpatialQuery {
    fn raycast(&self, ray: &Ray, max_distance: f32, filter: &QueryFilter) -> Option<Hit>;
    fn shapecast(&self, shape: &Shape, from: Vec3, delta: Vec3, filter: &QueryFilter) -> Result<Option<Hit>, QueryError>;
    fn overlap(&self, shape: &Shape, at: Vec3, filter: &QueryFilter, out: &mut Vec<Overlap>) -> Result<(), QueryError>;
}
```

- **`Shape`**: `Sphere { radius }`, `Capsule { half_height, radius }` (always standing along Y: the character's shape) and `Box { half_extents }` (axis-aligned). Rotated boxes and capsules are not part of v0.
- **`Hit`**: `distance` (the **travel**, not a fraction), `point` (the contact on the thing hit; for a box cast the box's centre at the hit), `normal` (unit, away from the thing hit), `material`, `collider` and `started_overlapping` (the ray began inside, or the shape already overlapped: `distance` is 0 and `normal` is the way out; a caller that resolves collisions must be told).
- **`QueryFilter`**: `mask: Layers`, `exclude: Option<ColliderId>`, `statics: bool`, `dynamics: bool`. The default looks at everything.
- **`Overlap`** results are **appended** to `out`, deterministically: the static world first (one entry per distinct material, sorted), then the moving colliders by id.
- **Errors**: `QueryError::Unsupported` for a combination of shapes the implementation cannot answer **correctly yet** (it refuses rather than guess), and `BadInput` for a negative or non-finite size, position or movement. A ray with a bad length simply hits nothing.
- **Determinism** (the physics crate is sim-safe): the same world and query give the same answer on every machine; moving colliders are kept sorted by id, so the order they are given in never changes an answer, and a tie in distance goes to the lower id.
- **Cost model:** the moving colliders are rebuilt into the BVH each tick with `QueryWorld::set_dynamic`; the static mesh is built once. Queries take `&self` and do not allocate for a ray cast.
- **Threading:** queries are `&self` and the world is read-only during a tick, so many queries may run at once (not exercised yet).

**What the reference implementation supported at first** (the rest returned `Unsupported`; since Amendment 1 everything is supported, and the full matrix is in `docs/PHYSICS_QUERIES.md`):

| Query | Static world | Moving colliders |
|---|---|---|
| `raycast` | yes | sphere, capsule, box |
| `shapecast` sphere | yes (exact: faces, edges, vertices) | sphere, capsule, box |
| `shapecast` box | **no** | box only |
| `shapecast` capsule | **no** | **no** |
| `overlap` sphere | yes (materials) | sphere, capsule, box |
| `overlap` box or capsule | **no** | sphere, capsule, box |

The missing capsule cast is the largest gap: the character controller needs it. It is the first thing in T07-004b.

## Amendment 1 (T07-004b): capsule and box casts, `raycast_all`, the triangle index

Additive, with one exception for code that builds a `Hit` by hand:

- **`Hit::triangle: Option<u32>`**: the index of the static triangle hit (in the mesh given to `QueryWorld::set_static`), `None` for a
  moving collider. This answers the first open question. A struct literal of `Hit` must now name the field (`triangle: None`); the only
  ones in the workspace were two tests in `ss2-game`, updated in the same change.
- **`SpatialQuery::raycast_all(&self, ray, max_distance, filter, out: &mut Vec<Hit>)`**: appends every hit along the ray, nearest first;
  at the same distance the static world comes first, by triangle index, then the colliders by id. A new required trait method; the only
  implementation is `QueryWorld`. This answers the third open question (the piercing weapons of T07-010 can use it).
- **The support matrix is complete** for v0 shapes: capsule casts against the static world and the moving colliders, box casts against
  the static world and against sphere and capsule colliders, box and capsule overlaps with the static world. No combination returns
  `Unsupported` any more; the variant stays, for future shapes.
- **Determinism:** between static triangles at the same distance the lower index wins, for every cast (the sphere cast used to follow
  the tree's visiting order there; the answer was deterministic before too, but now does not depend on the tree's shape).

Rotated boxes and capsules (the second open question) are left to T07-046, when ragdolls and vehicles need them.

## Alternatives considered

- **Return a list of all hits** instead of the nearest: more general (penetration, piercing bullets) but every caller then sorts and filters. A `raycast_all` can be added without changing these calls.
- **Closures or callbacks** for results: no allocation, but hard to use from scripts and from tests. `overlap` fills a caller-owned `Vec` instead.
- **Entity ids in the hit** (`Option<EntityId>`): the physics crate would depend on the simulation's entity model, which does not exist yet (RFC 0005 proposes it). `ColliderId` is an opaque id the simulation maps to an entity.
- **A separate crate** (as `ss2-render-api` is): the renderer's implementation is heavy and GPU-bound, so its contract had to be GPU-free. The physics crate has no heavy dependencies, so the contract lives in it as a module and consumers depend on the crate.
- **A rapier-based implementation:** the engine choice is a separate spike (T07-006). This contract is deliberately small enough to be implemented over any engine.

## Compatibility and migration

New code only: `ss2_physics::query` (and `spatial`, `geometry`, which it builds on). Nothing else changes. Consumers depend on `ss2-physics`. Adding shapes or methods later is additive; removing support for a combination is a breaking change and needs an RFC.

## Open questions

- ~~Should a hit carry the **triangle index**?~~ Yes: Amendment 1.
- Rotated boxes and capsules: needed for ragdolls and vehicles (T07-008, T07-022); decided when those start.
- ~~`raycast_all` (piercing)~~: Amendment 1.
- Whether queries need a **time** argument for lag compensation (T08-010 rewinds colliders to a past tick: probably a second `QueryWorld` built from the history, not a parameter).
