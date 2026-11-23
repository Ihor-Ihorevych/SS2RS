# T07 — World, Collision, Movement & Physics (`ss2-physics`, `ss2-world`)

Sim-safe crates: no wall-clock, no hash-order iteration, no thread-order effects. The **character
controller must be a pure function of (state, input, world, dt)** so the client can predict and
the server can reconcile (T08-009).

## Foundations
- [x] **T07-001** · Math/geometry primitives: AABB/OBB/capsule/sphere/plane/ray/swept shapes on `glam`; consistent coordinate system doc (handedness, up-axis, units); fixed-point-free but platform-stable ops (by construction; not measured across architectures): `ss2_physics::geometry` (`docs/GEOMETRY.md`) · `M` · `P0` · 🟢
- [ ] **T07-001b** · Geometry follow-ups: oriented-box overlap (separating axis) and capsule / oriented-box sweeps, sphere-vs-box sweeps of a box, triangle primitives; one shared set of `Aabb`/`Sphere`/`Plane` for `ss2-render-api` and `ss2-physics` (cross-crate: needs an RFC) · `M` · `P1` · Deps: T07-001
- [ ] **T07-006** · **SPIKE**: collision/rigid-body engine — `rapier3d` vs custom for world queries + character; compare features, mobile perf, determinism, ragdolls; RFC · `M` · `P0`
- [x] **T07-003** · Spatial acceleration: static BVH from world mesh, dynamic broadphase, query API (ray, sweep, overlap) w/ filters/layers: `ss2_physics::spatial` (`docs/SPATIAL.md`): `Bvh`, `Layers`, `Triangle`, `TriMesh`; the sweep is candidate-level (exact sweeps against triangles are T07-003b) · `L` · `P0`
- [ ] **T07-003b** · Spatial follow-ups: exact sphere/capsule sweeps against triangles (face, edge, vertex), box-triangle overlap, SAH build and incremental refit for large dynamic sets, benchmarks (T07-017) · `M` · `P1` · Deps: T07-003
- [x] **T07-004** · Query API frozen as contract: `raycast`, `shapecast`, `overlap`, hit result (point/normal/material/entity): RFC 0007 (in review, like RFCs 0001-0006) and `ss2_physics::query` with the reference `QueryWorld` (`docs/PHYSICS_QUERIES.md`); unsupported shape combinations are refused and listed in T07-004b · `M` · `P0`
- [ ] **T07-004b** · Query follow-ups: capsule casts (against the static world and moving colliders; the character controller needs them), box casts against the static world, box/capsule overlaps with the static world, rotated boxes and capsules, `raycast_all`, the triangle index in a hit · `M` · `P0` · Deps: T07-004
- [ ] **T07-002** · Collision geometry extraction from `.wld` (static) and `.mdl` (dynamic/hit volumes) · `L` · `P0` · Deps: T02-020, T02-017

## Movement (M3/M4)
- [ ] **T07-005** · Character controller matching the original: accel/friction, air control, jump, gravity, step-up, slope sliding, crouch, swim/fly volumes, ladders, moving platforms · `XL` → split by mode · `P0` · Deps: T01-024
- [ ] **T07-019** · Movement validation harness: replay recorded inputs/oracle CSV (T01-039) through controller, report positional error per tick · `M` · `P0` · Deps: T01-039
- [x] **T07-020** · Free-fly/noclip debug camera (needed before the controller is ready): `ss2_game::freefly` (`docs/FREEFLY.md`); the app's input wiring is part of T03-033/T10 · `S` · `P0` · 🟢
- [ ] **T07-018** · Water/liquids: swimming, buoyancy, damage/slime volumes, splash events · `M` · `P1`

## Physics objects & world interaction
- [ ] **T07-007** · Rigid bodies for props/debris/gibs; sleeping; configurable budget caps (so a low tier is possible later) · `L` · `P1`
- [ ] **T07-008** · Ragdolls from skeleton + death impulse, settle → freeze/cleanup · `L` · `P2`
- [ ] **T07-009** · Physical materials (`.mat`): friction/restitution/sound/effect tables hooked to hits · `S` · `P1` · Deps: T02-019
- [ ] **T07-010** · Hitscan & projectile ballistics: ray/sweep hits, penetration, ricochet (if present), hit zones (head/limbs), splash damage queries · `L` · `P1`
- [ ] **T07-011** · Volumes: triggers, damage zones, water, kill, push/jump pads, music/reverb zones · `M` · `P1` · Deps: T02-020d
- [ ] **T07-012** · Mechanisms: doors, elevators, rotators, movers w/ riders & crush rules, event I/O · `L` · `P1` · Deps: T01-030
- [ ] **T07-021** · Destructibles (`EditKit_Destruction` hint): fracture/replace-with-debris, state sync hooks · `L` · `P2` · Deps: T01-046
- [ ] **T07-022** · Vehicles/turrets, if the campaign has them · `L` · `P3` · Deps: T01-047

## World instance & navigation
- [ ] **T07-023** · `World` instance: load `.wld` → entities+static geometry+nav; spawn by class via registry; level restart/unload; streaming hooks · `L` · `P0` · Deps: T02-020, T08-002
- [ ] **T07-013** · Navigation: use authored nav data if present else generate navmesh from collision; A* + funnel, dynamic obstacles, off-mesh links (jumps/doors) · `XL` → split · `P1` · Deps: T01-025, T02-020e
- [ ] **T07-024** · Visibility queries for AI & gameplay (line of sight, hearing propagation through sectors) · `M` · `P1`
- [ ] **T07-015** · Sector/portal data shared with renderer culling (`ss2-world` owns, renderer reads) · `M` · `P1`

## Tooling & quality
- [x] **T07-016** · Physics/collision debug draw (contacts, normals, sweeps, BVH) via T04-018: `ss2_game::physics_debug` (`docs/PHYSICS_DEBUG.md`): colliders, capsules, oriented boxes, BVH nodes, ray and shape casts, meshes; contacts of the controller and rigid bodies wait for them · `S` · `P1` · 🟢
- [x] **T07-017** · Performance benchmarks: 300 AI + 500 debris + projectiles: `crates/ss2-bench/benches/physics.rs` (BVH build and rays, mesh rays and sphere sweeps, the named workload as one tick of queries), measured once on a laptop (`docs/PERF_BUDGETS.md` section 5.7); **no per-system budget is set** (the tick rate and share are T01-022) and the mobile re-run is Phase 2 (T07-017b) · `M` · `P1`
- [ ] **T07-017b** · Physics performance follow-ups: profile and cut the per-query overhead (a query costs 4 to 9 us whatever the scene size), a selection instead of a sort in the BVH build, set the per-system budget once T01-022 gives the tick rate, re-run on a reference mid-range PC and on mobile CPUs (Phase 2), add the real controller and AI to the workload · `M` · `P1` · Deps: T07-017, T01-022
- [ ] **T07-025** · Property tests (proptest): no tunneling at max speed, no stuck in corners, determinism across runs/threads · `M` · `P1` · 🟢
- [ ] **T07-026** · Cross-arch float consistency test (x86_64 vs aarch64) of the controller; document tolerances for reconciliation · `M` · `P1`
