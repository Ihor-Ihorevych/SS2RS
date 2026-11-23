# RFC 0009: The simulation contract (v0): entities, classes, properties, the world

- **Number:** 0009 (`docs/rfcs/0009-sim-contract.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-sim` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T00-017 (this contract); unblocks T08-002 (class model and spawn-from-record), T08-004 (server and client split), T08-007 (replication), T07-023 (the `World` instance), T09 (gameplay code)

## Summary

Add to `ss2-sim` the types the rest of the engine needs to talk about entities **before any of them is stored in a particular ECS**: an `EntityId` (index plus generation), a **class registry** with
**property reflection and replication attributes**, and a **`World` trait** with a plain reference implementation (`SimpleWorld`) and a **conformance suite** that any other implementation (RFC 0005's `hecs`
behind this trait) must pass. Together with the tick loop and schedule (T08-003, `docs/SIMLOOP.md`) and the event bus (T08-028, `docs/EVENTBUS.md`), which already exist, this is the "Sim" contract: tick API,
entity model, reflection, replication attributes, events.

It is a **v0**: the smallest surface that lets replication, the server, the save system and gameplay code be written against it, with the choices that are expensive to change later made explicitly and the rest marked open.

## What exists, and what this adds

| Part of the contract | Where | State |
|---|---|---|
| Tick API: fixed step, accumulator, pause, step, time scale, stall guard | `ss2_sim::TickLoop` | built (T08-003) |
| Phases and systems | `ss2_sim::Schedule`, `Simulation`, `TickContext` | built (T08-003) |
| Event bus with replication-aware audiences | `ss2_sim::EventBus` | built (T08-028) |
| **Entity handles** | `ss2_sim::EntityId`, `ClassId` | **new** |
| **Classes, properties, inheritance, replication attributes, schema hash** | `ss2_sim::reflect::{ClassRegistry, ClassDesc, PropertyDesc, ...}` | **new** |
| **The world: spawn, despawn, parent, get and set properties, changes, lifecycle** | `ss2_sim::World` (trait), `SimpleWorld` (reference), `world::conformance` (tests) | **new** |

## Design

### Entities

An `EntityId` is `(index: u32, generation: u32)`, packed into a `u64` for the wire and replays (`to_u64`). Despawning bumps the slot's generation, so **a handle to a dead entity is an error
(`WorldError::Dead`), never a different entity**. `EntityId::NULL` (both halves `u32::MAX`) is the default of an entity-reference property and is never alive. Free slots are reused **lowest index first**, so
the same calls make the same handles on every machine (a test replays 200 spawns and despawns twice).

### Classes and properties

A class is data: a name, an optional parent, a list of properties. A property has a **name, a type, a default, replication attributes and a save flag**. The types are `Bool`, `I32`, `U32`, `F32`, `Vec3`, `Text`
and `Entity` (non-exhaustive: more are added as the game needs them). A class has its parents' properties **first**, so a property's **slot** (its index in `ClassRegistry::properties`) is stable under inheritance:
a `Boss` has the slots of an `Enemy`, then its own. Names are 1 to 64 characters of letters, digits, `_`, `.`, `-`; a property cannot shadow an inherited one; classes get ids in registration order, so **every machine must
register the same classes in the same order** (the schema hash checks that).

### Replication attributes

Each property says whether and how it is replicated, because the replication framework (T08-007) is generic and must not know what a `health` is:

- **Mode:** `None` (simulation only), `Always` (to everyone, when changed), `SpawnOnly` (once, at creation), `Owner` (only the owning player).
- **Quantization:** `Fixed { min, max, bits }` for floats and vectors, 1 to 32 bits per component (checked at registration).
- **Priority:** which property goes first when bandwidth is short (it does not change the layout).

`ClassRegistry::schema_hash()` is an FNV-1a hash of **what decides the wire layout**: the classes in id order, their names and parents, and each property's name, type, mode and quantization, in order.
Defaults, priority and the save flag are deliberately **not** in it (changing a default does not change the bytes). It is meant for the connection handshake (next to `ss2_net::protocol::Schema`)
and the replay header, so two builds that disagree about a class refuse each other instead of corrupting each other's state.

### The world

`World` is a trait because RFC 0005 proposes `hecs` behind it and the choice should stay cheap to change. Its rules (also in the module documentation, and enforced by the conformance suite):

1. **Properties are type-checked** against the class; an unknown property or a wrong type is an error and changes nothing. Access is by name (convenient: console, tools, spawn-from-record) or by slot
   (fast: replication, save).
2. **Changes are recorded**: `set` records `(entity, slot)` when the value differs **bit for bit** (so `-0.0` differs from `0.0`, a NaN equals itself) and the property is replicated;
   `drain_changes` returns them sorted, without repeats, and forgets them; a despawned entity's changes are dropped.
3. **Lifecycle is recorded** in order (`Spawned`, `Destroyed`) and drained the same way: that is what the replication layer and the event bus consume.
4. **Despawning a parent despawns its descendants**, deepest first, and reports how many entities went. `set_parent` refuses a cycle.
5. **Order is deterministic**: `entities()` and `children_of()` are by slot index; nothing reads a clock or uses a hash map (the crate's `clippy.toml` enforces it).

`SimpleWorld` is the reference implementation (a vector of slots, a `BTreeSet` of free indices): plain, deterministic, **not fast**, with a configurable entity limit (default about a million, refused with `WorldError::Full`).
`world::conformance::run(&make)` is a test suite (spawn and defaults, stale handles, slot reuse, property checks, change recording, lifecycle order, parenting, cascading despawn, ordering) that **any `World` must pass**;
T08-002's implementation calls it from its own tests.

## How it was tested

Entity, registry and world together have 19 new tests (the conformance suite is one of them and runs nine checks) and a doctest. The registry tests cover id assignment, inheritance and slots, defaults, every refusal (names, duplicates, shadowing, unknown parent, quantization bounds, the property
limit), a refused class leaving nothing behind, and a schema hash that is stable, sensitive to every wire-layout detail (name, type, mode, each quantization field, order, an added property, names that would run
together) and **not** sensitive to defaults, priority or the save flag. The world tests are the conformance suite run against `SimpleWorld`, plus the entity limit, a thousand generations in one slot, determinism of handles across two runs and references to dead entities.
**Twenty deliberate breakages each fail a test**: no generation bump, the highest free slot first, handles that ignore the generation, a same-value write counted as a change, unreplicated properties recorded, a dead entity's changes kept, a parent
destroyed before its children, cycles allowed, the type not checked, the limit off by one, a destroyed or moved child left in its parent, the name and property limits off by one, an empty quantization range allowed, 32 bits refused, floats
compared by value rather than by bits, the mode left out of the hash, a duplicate class allowed, `is_a` broken. The tests passed at the first run, so these breakages are the evidence that they test something; the suite was written with the code, not before it.

## What this does not settle

- **The real implementation.** `SimpleWorld` is a double. The `hecs` facade (RFC 0005, still in review) and its speed are T08-002; the contract says nothing about performance except that slot access exists for the hot paths.
- **Behaviour.** There is no lifecycle callback (`init`, `tick`, `destroy` on a class), no system that runs per entity, no "spawn from a record" and no mapping from the original's records to classes: that is T08-002 and
  gameplay (T09), and the contract keeps to **data**. Whether behaviour is a trait object per class, a registered function table or systems over components is open.
- **Component storage versus properties.** Reflection here describes **properties**; whether a hot component such as a position is stored as an ECS component and exposed as a property, or stored as the property, is the
  implementation's choice, but a property read must return what the system sees.
- **Value types.** Seven types are enough to write the contract and its tests; the original's records, the save format and the UI will want more (enums, flags, colours, fixed arrays, entity lists). The enums are non-exhaustive for that reason.
- **Owner.** `ReplicationMode::Owner` needs a notion of which player owns an entity; the contract has no `owner` field (it can be an ordinary property today). Where it lives is T08-007's to propose.
- **Delta and interest management.** What to send to whom is T08-007; this contract only records what changed and what was created or destroyed.
- **Saving.** The `persistent` flag exists; the save format (T08-022) does not.
- **Threading.** `SimpleWorld` is single-threaded; the trait says nothing about `Sync`.

## Alternatives considered

- **Start with the `hecs` facade and let the trait fall out of it.** Rejected: it ties the contract to an RFC that is not approved and makes every consumer wait for the slowest piece. The reference world lets replication, the server and
  gameplay start now and checks the eventual real one against the same suite.
- **No reflection: typed Rust structs and generated replication code.** Fast and type-safe, but the class model must follow the original's data-driven classes and mods (T13) need to add classes at run time; a registry is needed anyway.
- **Properties as `Box<dyn Any>`.** Flexible, but the schema hash, quantization and save flags need a closed description of types.

## Follow-ups

T08-002 (class model, lifecycle callbacks, spawn-from-record) implements `World` for the real store and runs `world::conformance::run`; T08-007 consumes `Change` and `LifecycleEvent` and `schema_hash`; the handshake adds the
hash next to the protocol schema (T08-006 follow-up). I have added no new task lines: those exist.
