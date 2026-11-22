# RFC 0005: The entity store (ECS) for the simulation

- **Number:** 0005 (`docs/rfcs/0005-ecs-choice.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-sim` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T08-001 (this spike); unblocks T00-017 (Sim contract), T08-002 (class model), T08-003 (sim loop), T08-007 (replication)

## Summary

Use **`hecs`** as the storage of the simulation's entities and components, **behind a small facade owned by `ss2-sim`**, and
build the parts the netcode needs on top of it ourselves: a **component registry** (names, replication flags, writers and
readers, quantization) and **stable entity handles** exposed to the rest of the code as our own type. The facade is what makes
the choice cheap to change. The fallback, if the registry-driven access dominates a profile, is a small custom sparse-set store
(measured below); `bevy_ecs` is not recommended.

This is the output of a time-boxed spike: a throwaway benchmark (`spikes/ecs/`, outside the main workspace) of three candidates on
the operations the netcode uses. The numbers are from one machine and a synthetic workload; the recommendation rests as much on
footprint and fit as on them, and it says where the evidence is thin.

## Motivation

The simulation (`ss2-sim`), the replication framework (T08-007), the class model that spawns entities from `.ep`/`.wld` records
(T08-002) and the sim loop (T08-003) all need an entity store. Nothing can be written against an entity API until the choice is
made, and a wrong choice is costly to undo once components are scattered across the code. The roadmap asks for: **stable entity
IDs with generations, per-component replication flags, reflection hooks, cheap snapshot of state**.

## Requirements and what each candidate gives

| Requirement | `hecs` 0.11.2 | `bevy_ecs` 0.20.1 | custom sparse set (spike code) |
|---|---|---|---|
| Stable IDs with generations | yes (`Entity`) | yes (`Entity`) | yes (index + generation) |
| A stale handle is detected after despawn and slot reuse | yes (tested) | yes (tested) | yes (tested) |
| Per-component replication flags | no: **our registry** | no (with `bevy_reflect` it has type data; not used here) | **our registry** |
| Reflection hooks (named properties for the class model) | no: **our registry** | `bevy_reflect` (an optional, large dependency) | **our registry** |
| Cheap snapshot of replicated state | typed query: fast; by registry: the slowest of the three | fast both ways | typed: slowest; registry: middle (a naive version) |
| Deterministic iteration order | yes in the spike (below) | yes in the spike | yes |
| Dependencies | **4 crates** in the tree | **65 crates** (`default-features = false, features = ["std"]`) | none |
| MSRV | 1.81 | 1.95 (the workspace is 1.99) | n/a |
| Licence | MIT OR Apache-2.0 | MIT OR Apache-2.0 | ours (Apache-2.0) |
| Scheduler, change detection, events | none (we have our own phases, T08-003) | yes, which we would not use but must carry and follow | none |

**Everything the netcode needs beyond the store (replication flags, reflection, quantization, relevancy) has to be written by us
whichever store is chosen.** That is the main finding: none of the candidates removes the registry.

## Measurements

Workload: 50,000 entities, 70 % mobile (position, velocity, health, team) and 30 % static (position, team); that is more than
a level needs. Median times in microseconds over many runs, three runs of the whole program (the range is the spread), release
build, Windows 11, Rust 1.99.1, 12th Gen Intel i5-12450HX, single thread.

| | spawn 50,000 | movement pass (35,000) | snapshot, typed query | snapshot, through a registry | churn (despawn and respawn 5,000) |
|---|---|---|---|---|---|
| `hecs` | 3,350 to 3,390 | 39 to 40 | 190 to 194 | **1,420 to 1,520** | 590 to 760 |
| `bevy_ecs` | 4,300 to 4,540 | 61 to 72 | 200 to 216 | **470 to 580** | 860 to 910 |
| custom sparse set | 2,640 to 2,770 | 50 to 55 | 260 to 263 | **760 to 790** | 416 to 422 |

- **Snapshot** writes the id, the position (12 bytes) and, where present, the health (4 bytes) into a byte vector: 940,000 bytes
  for all three (`bevy_ecs` walks one entity of its own in the registry variant: 940,004). The *typed* variant is one query; the
  *registry* variant visits every entity and asks a table of `(name, writer)` entries for each replicated component by type, which
  is what a data-driven class system with reflection needs.
- **Churn** also checks that every handle of a despawned entity is recognised as dead and that no new entity got an old handle
  back (generations work): true for all three.
- **Determinism:** the order in which entities are iterated after the churn was fingerprinted; it was **identical in four separate
  process runs** for all three (a store whose iteration depended on a randomly seeded hasher would differ). This checks one
  operation sequence; it is not a proof.

What the numbers say: nothing here is slow for the sizes that matter. A client replicates the entities near it, say 1,000 to
3,000, so even the slowest registry walk (`hecs`, 1.5 ms per 50,000) costs about 30 to 90 microseconds per snapshot. The ranking
between the three changes with the operation, and **the custom store is a naive implementation** (a map from type to column
looked up per access) that a careful one would beat in the registry case. The spread is too small to choose on speed alone.

## Detailed design

### The facade

`ss2-sim` exposes its own types and keeps `hecs` private:

```rust
pub struct EntityId { /* index + generation, Copy, Ord, Hash-free for the sim */ }

pub struct Sim { world: hecs::World, registry: ComponentRegistry, /* tick, event bus (T08-028) ... */ }

impl Sim {
    pub fn spawn(&mut self, components: impl Bundle) -> EntityId;
    pub fn despawn(&mut self, id: EntityId) -> bool;          // false if the handle is stale
    pub fn contains(&self, id: EntityId) -> bool;
    pub fn get<C: Component>(&self, id: EntityId) -> Option<&C>;
    pub fn query<Q: Query>(&mut self) -> impl Iterator<Item = (EntityId, Q::Item)>;
}

/// Declared once per component type, at startup:
registry.register::<Position>(ComponentInfo { name: "position", replicated: Replicate::Snapshot, quantize: Quantize::Position, .. });
registry.register::<Health>(ComponentInfo { name: "health", replicated: Replicate::OnChange, .. });
registry.register::<Team>(ComponentInfo { name: "team", replicated: Replicate::No, .. });
```

The registry holds, per component: a stable **numeric id and name** (the wire format uses the id, the class records and the console
use the name), the replication mode, the quantization, and **type-erased reader and writer functions** (like the `(name, writer)`
table of the benchmark). `ComponentInfo` is also where the reflection hook lives: the class model (T08-002) builds an entity
from a record by looking components up **by name** and filling their fields.

### What stays open for the Sim contract (T00-017)

- Whether a data-driven class is one Rust struct per class or a generic property bag with typed slots. The registry supports both;
  the choice belongs to the class model (T08-002) and affects how much of the entity is addressable by name.
- Parallel iteration: not measured and not needed by this design: the simulation is single-threaded and deterministic by design
  (`ss2-sim` forbids unordered collections and wall-clock time).
- The event bus (T08-028) and the system phases (T08-003) are ours and sit beside the facade, not inside the store.

### Determinism rules for the facade

Iteration must follow an order that depends only on the operations performed, never on a hasher seed, address or thread. The
spike's check (same fingerprint in four processes) should become a test of the facade (iteration order after a fixed script of
spawns, despawns and component changes) so that a library upgrade that changes it is caught.

## Alternatives considered

- **`bevy_ecs`.** Fastest of the three at the registry snapshot and the most complete. Against it: **65 crates** instead of 4, an
  MSRV of 1.95, a scheduler, change detection and a plugin style we would carry but not use, and an API that is pre-1.0 (0.20) and so may change in any release. Its reflection (`bevy_reflect`) is a further large dependency. Rejected for footprint and churn, not for speed.
- **A custom sparse-set store.** No dependency, full control of determinism and of the registry, and the **fastest to spawn and
  to churn** in the spike even though it is naive. Against it: we would write and maintain archetype-style multi-component
  queries, safe simultaneous access to several components, and the test suite that a mature crate already provides. It is the **fallback**: the facade
  makes switching a contained change if the registry-driven path proves to be the bottleneck.
- **`legion` 0.4.0 and `specs` 0.20.0** (both MIT or Apache): not measured. I did not check their maintenance status beyond the
  version numbers `cargo info` shows, and their APIs are older and more ceremonial; treat them as not evaluated, not as rejected on
  evidence. **`shipyard` 0.11.5**: not measured either (a sparse-set design like the custom store, with a scheduler).
- **No store: plain structs and vectors per class.** Simplest and fastest for fixed classes, but SS2's data defines hundreds of
  entity classes with named properties (`docs/re/wld-entities.md`) and the netcode wants one generic way to replicate and inspect
  them; that generic layer is a registry, which is the thing already needed.

## Compatibility and migration

New in `ss2-sim` only: no existing code uses an entity store. `ss2-sim` gains the `hecs` dependency (a four-crate tree, permissive
licence). It must follow `docs/DEPENDENCIES.md`: declared once in `[workspace.dependencies]` with default features off, `cargo deny check`
run when it is added, and, because `ss2-sim` is sim-safe, **no hash-order iteration in the code path it uses**: the spike fingerprinted the
iteration order only for one operation sequence, so the facade's determinism test (above) is a condition of accepting this choice. `spikes/ecs/` is not part of the workspace or CI and can be deleted once the
facade exists; its `Cargo.lock` is committed so the numbers can be reproduced. If the store is ever swapped, only the facade and
the registry's reader/writer glue change; the wire format uses the registry's numeric ids, not Rust types.

## Open questions

- **Does the workload resemble the real one?** 50,000 synthetic entities with four components. The real game has hundreds of
  classes and many more components per entity; the benchmark must be repeated with the class model (T08-002). *Decides:* the
  `ss2-sim` owner, before T08-007 starts.
- **Is the registry walk the right snapshot path?** If most replicated state turns out to be a few well-known components, typed
  loops are 7 times faster than the registry walk on `hecs`; the registry would then serve the rare and generic cases. *Decides:*
  the owner of replication (T08-007).
- **Maintenance of `hecs`.** I did not assess how active the project is (the version and licence are all I checked). *Decides:*
  the approving maintainer; the facade limits the damage of being wrong.
- **Pre-1.0 versions:** `hecs` 0.11 and `bevy_ecs` 0.20 are both pre-1.0; the facade also absorbs breaking changes.
