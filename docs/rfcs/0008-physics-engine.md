# RFC 0008: The physics engine: our own query layer, `rapier3d` for rigid bodies

- **Number:** 0008 (`docs/rfcs/0008-physics-engine.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-physics` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T07-006 (this spike); informs T07-005 (character controller), T07-007 (rigid bodies), T07-008 (ragdolls), T07-004b (capsule casts), T07-017b (query speed), T07-026 (cross-architecture test), T08-015 (authority for physics objects)

## Summary

**Keep the query layer we have** (`ss2-physics`: geometry, BVH, `SpatialQuery`, `QueryWorld`; RFC 0007) as the world-query API and as the foundation of the **character controller and
hitscan**, and **use `rapier3d` for rigid bodies, debris and ragdolls** (T07-007, T07-008), **behind a facade of ours** so that the rest of the game never names a rapier type.
Do **not** adopt rapier's `KinematicCharacterController` for the player: the roadmap asks for a controller "matching the original" (acceleration, friction, air control), which is our code on top of shape casts whichever
engine answers them.

This is the output of a time-boxed spike (`spikes/physics/`, outside the main workspace): our code and `rapier3d` 0.36.1 on the same synthetic scenes. The numbers are from **one machine, one architecture,
a synthetic workload and no mobile device**; section "What this does not settle" lists what must be checked before rapier ships.

## Motivation

The simulation needs, at a mid-range PC's budget: (1) queries against the static world and moving colliders (shots, line of sight, the controller's casts); (2) a character controller that reproduces
the original's movement; (3) props, debris and gibs that move; (4) ragdolls. Roadmap T07-006 asks whether one engine (`rapier3d`) should do all of it or whether the pieces should be ours.
The deciding facts are the ones the game cannot recover from later: **how much code we would have to write and keep correct**, **whether the answers are the same on every machine** (prediction and reconciliation, T08-009), and
**what it costs per tick**.

## What each candidate gives

| Need | ours (`ss2-physics`, built: T07-001, 003, 004) | `rapier3d` 0.36.1 (with `parry3d` 0.31.1) |
|---|---|---|
| Ray against a triangle mesh and moving colliders | built, tested, mutation-checked | built in |
| Sphere cast | built | built in |
| **Capsule and box casts against the static mesh** | **not built** (T07-004b, T07-001b, T07-003b) | **built in** (`cast_shape`) |
| Rigid bodies (contacts, friction, stacking, sleeping) | **none; a large piece of work** | built in |
| Joints and ragdolls | none | impulse and multibody joints built in |
| Character controller | none yet (T07-005); must match the original | a generic "move and slide" (`KinematicCharacterController`: slopes, auto-step, snap to ground) |
| Sim-safe rules (no `HashMap`, no wall clock; `clippy.toml`) | enforced by lint | the library's own business; see determinism below |
| Dependencies | none beyond `ss2-core` | **59 crates** in its tree (including `nalgebra`, `glam`, proc macros) |
| Licence | Apache-2.0 | Apache-2.0 (MSRV 1.86; the workspace is 1.99) |

## Measurements

Release build, Windows 11, Rust 1.99, 12th Gen Intel i5-12450HX, one thread. Terrain meshes of 20,000 and 180,000 triangles; the same 100,000 seeded rays for both engines.
Three full runs; the figures are from the last and the others were within about 10 % of them.

| | ours | `rapier3d` |
|---|---|---|
| Build a 20,000-triangle static mesh | 10.6 ms | 4.7 ms |
| Build a 180,000-triangle static mesh | 155 ms | 44 ms |
| Ray against 20,000 triangles | 1,030 ns | 315 ns |
| Ray against 180,000 triangles | 1,600 ns | 660 ns |
| Capsule cast against 20,000 triangles (radius 0.4, 1.8 high; most hit) | **not available** | 10,900 ns |
| 2,000 moving spheres: update every tick + 1,000 rays | 1,300 us | 670 us |
| Character controller `move_shape`, flat floor | n/a | 11 us per tick |
| Rigid bodies: 64 / 216 / 1,000 boxes on a terrain, one step (mean, p99) | **none** | 0.11 ms, 0.39 ms / 0.32 ms, 1.3 ms / 1.2 ms, 2.6 ms |
| Ragdoll, a chain of 12 capsules with impulse joints, one step | none | 29 us |

- **Our answers are right.** Over 200,000 rays (two meshes) the two engines **agreed on hit or miss for every ray** (98,024 and 99,334 hits, no ray hit by one and missed by the other) and on distance to
  **4e-6** at most. That is independent evidence that the BVH, the triangle test and the tie rules of `ss2-physics` are correct, and it says nothing against rapier either.
- **Our queries are two to three times slower**, and building the tree is three times slower. That is a known gap (T07-017b: "a selection instead of a sort in the BVH build", the per-query overhead); it is not a reason to
  change engines, because a ray costs 1 to 1.6 us and a controller or a weapon asks for a handful per tick. It does mean a tick that moves 2,000 colliders and fires 1,000 rays costs 1.3 ms of ours against 0.7 ms.
- **Capsule casts are the expensive query** (11 us, 35 times a ray) and **we do not have them**: exact capsule-versus-triangle sweeps (face, edge and vertex cases) are real work (T07-003b) and a source of bugs.
  `parry3d` has them, and the controller needs them first.
- **Rigid bodies are cheap enough**: a thousand boxes settling on a terrain cost 1.2 ms per step on average (p99 2.6 ms), which is far above what a level's debris needs; with sleeping it falls as bodies rest.
- **The character controller of rapier works** (it walked 600 ticks, stepped up a 0.3 unit step, stopped at a wall) and costs 11 us a tick, but it is not the original's movement model.
- **Ragdolls:** a chain with **impulse joints** collapsed and settled as expected. The chain with **multibody joints** stayed upright and asleep in my scene, which I did not expect; I did not investigate it, so **I have
  not shown that multibody joints work for a ragdoll**.

## Determinism

The game is server-authoritative with client prediction, not lockstep, so exact agreement across machines is a quality matter (small reconciliation errors) rather than a correctness one, **except for anything the server
simulates for gameplay**. What I could and could not check:

- **Same machine, same build: identical.** The same 216-box scene run twice in one process and in five separate processes gave the same hash of every body's final position (`c629fa84...`). Separate processes matter:
  a hidden hash map with a random seed would show up as a different result per process.
- **`enhanced-determinism`** (rapier's cross-platform mode: no SIMD, `libm` maths) gives a **different hash** (`82a0e05d...`, also identical across runs) at about the same speed on this machine
  (rays 324 ns, 1,000 bodies 1.17 ms, controller 10.5 us). Which mode a result came from matters for any recorded replay or test fixture.
- **Not checked: x86_64 against aarch64**, other compilers, or a thread count other than one (`parallel` was off). T07-026 exists for the cross-architecture test; until it runs, **no claim of cross-platform
  determinism is made**, for rapier or for our own code.

## Recommendation, and what it costs

1. **Queries and the controller stay ours** (T07-004b, T07-005). For the **capsule and box casts** we lack, the plan is to **try `parry3d` for the shapes** (`cast_shapes`, triangle meshes) behind our `SpatialQuery`
   trait before writing exact sweeps ourselves. That is a decision for T07-004b once the dependency question (below) is settled; this spike measured rapier's cast, not parry on its own.
2. **Rigid bodies, debris and ragdolls use `rapier3d`** (T07-007, T07-008) in a small crate or module that owns it, with our own handle types, a configuration (gravity, fixed step, solver iterations) and a **budget cap** (the roadmap asks for one).
   **Decide per object class whether it is authoritative**: debris, gibs and ragdolls are cosmetic and can run on each client with no replication and no determinism requirement; a pushable crate that
   affects gameplay is authoritative on the server and replicated (T08-015) and must use `enhanced-determinism` if prediction touches it.
3. **A `SpatialQuery` adapter for rapier** (rays, shape casts, overlaps against its world) keeps the choice reversible: if our own speed work stalls, the static world can be answered by rapier through the
   trait RFC 0007 defined, and nothing above it changes. It is not built.
4. **Before rapier is added to the workspace:** `cargo deny check` on its 59-crate tree (licences, advisories, duplicates; the spike is outside the workspace, so **this has not been run**), a build for
   `aarch64-linux-android`, and a look at the binary size and the compile time (a release build of the whole spike from scratch took about a minute and a half).

## Alternatives considered

- **`rapier3d` for everything, queries and controller included.** Least code, and faster queries today. Rejected for the player: the controller must reproduce the original's movement, so we would write it on top of
  rapier's casts anyway; and the sim-safe rules, the query contract (RFC 0007) and the tested query code we already have would be thrown away, with the determinism of the whole simulation resting on an external library.
  The adapter (item 3) keeps this option open for the static world.
- **Everything ours.** No new dependency and full control, but a rigid-body solver with stable stacking, sleeping and joints is a project of its own, and the roadmap lists ragdolls and debris as wanted
  (P1, P2). Rejected for rigid bodies; kept for queries.
- **`parry3d` only (no `rapier3d`).** Gives shapes and casts but no rigid bodies. It is where item 1 looks first.
- **Another engine** (e.g. `avian`, Bevy-only; PhysX bindings, a C++ build): not measured, and a C++ dependency fights the portability goal. Not recommended without a reason.

## What this does not settle

- **Mobile performance and memory.** No device was available; the 12th Gen laptop figures say nothing about a phone's CPU. T12-005 and T12-019 are Phase 2.
- **Cross-architecture determinism** (above) and **threading** (`parallel`).
- **Ragdoll quality**: one chain, no skeleton, no animation blend, and the multibody result above is unexplained.
- **A real level**: the scenes are synthetic (a sine terrain, boxes, one capsule chain), not the original's geometry, and the query mix is not the game's.
- **Our speed**: the gap above is measured with the code as it stands; the follow-up T07-017b may close part of it.

## How to re-run

```
cd spikes/physics
cargo run --release                      # the numbers above
cargo run --release --features enhanced  # rapier's cross-platform mode
```

The spike has its own `[workspace]` and `Cargo.lock`: it is **not** part of the main workspace, CI or `cargo deny`, and nothing in it ships. Delete it when the rigid-body module exists.
