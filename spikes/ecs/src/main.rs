//! Throwaway benchmark behind RFC 0005 (the ECS choice, T08-001). It measures the things the
//! netcode needs from an entity store, on the same workload for three candidates:
//!
//!  * `hecs` 0.11 (archetypes),
//!  * `bevy_ecs` 0.20 (archetypes plus tables, change detection),
//!  * a minimal sparse-set store written for this spike (what "custom" would start from).
//!
//! The workload: 50,000 entities, 70 % mobile (position, velocity, health, team) and 30 % static
//! (position, team). Operations: spawn everything; one movement pass (position += velocity);
//! a snapshot of the replicated components (position and health) written to bytes **by a typed
//! query** and **through a registry of (name, replicated flag, writer)**, which is what a
//! data-driven class system with reflection needs; and churn (despawn 10 %, spawn as many, check
//! that old handles are recognised as dead).
//!
//! Run: `cargo run --release` in this folder. Numbers vary by machine; see the RFC for the ones
//! quoted and their caveats. The timer reports the median of many runs.

use std::time::Instant;

const N: usize = 50_000;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vel {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Health(i32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Team(u8);

#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BPos(Pos);
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BVel(Vel);
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BHealth(Health);
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BTeam(Team);

fn fold(hash: u64, value: u32) -> u64 {
    hash.wrapping_mul(1_099_511_628_211).wrapping_add(u64::from(value))
}

fn is_mobile(i: usize) -> bool {
    i % 10 < 7
}

fn pos_of(i: usize) -> Pos {
    Pos { x: i as f32, y: 0.5, z: -(i as f32) }
}

fn put_pos(out: &mut Vec<u8>, p: &Pos) {
    out.extend_from_slice(&p.x.to_le_bytes());
    out.extend_from_slice(&p.y.to_le_bytes());
    out.extend_from_slice(&p.z.to_le_bytes());
}

fn put_health(out: &mut Vec<u8>, h: &Health) {
    out.extend_from_slice(&h.0.to_le_bytes());
}

/// Run `f` `reps` times and return the median in microseconds.
fn median_us(reps: usize, mut f: impl FnMut()) -> f64 {
    let mut times: Vec<f64> = (0..reps)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    times[times.len() / 2]
}

struct Row {
    name: &'static str,
    spawn_us: f64,
    step_us: f64,
    snapshot_typed_us: f64,
    snapshot_registry_us: f64,
    churn_us: f64,
    typed_bytes: usize,
    registry_bytes: usize,
    order_fingerprint: u64,
    stale_handles_detected: bool,
}

// ---------------------------------------------------------------------------------------- hecs

fn run_hecs() -> Row {
    use hecs::{Entity, EntityRef, World};
    let spawn = |world: &mut World, ids: &mut Vec<Entity>| {
        for i in 0..N {
            let e = if is_mobile(i) {
                world.spawn((pos_of(i), Vel { x: 1.0, y: 0.0, z: -1.0 }, Health(100), Team((i % 4) as u8)))
            } else {
                world.spawn((pos_of(i), Team((i % 4) as u8)))
            };
            ids.push(e);
        }
    };
    let spawn_us = median_us(15, || {
        let mut w = World::new();
        let mut ids = Vec::with_capacity(N);
        spawn(&mut w, &mut ids);
        std::hint::black_box(&w);
    });
    let mut world = World::new();
    let mut ids = Vec::with_capacity(N);
    spawn(&mut world, &mut ids);
    let step_us = median_us(60, || {
        for (p, v) in world.query_mut::<(&mut Pos, &Vel)>() {
            p.x += v.x * 0.016;
            p.y += v.y * 0.016;
            p.z += v.z * 0.016;
        }
    });
    let mut typed = Vec::new();
    let snapshot_typed_us = median_us(40, || {
        typed.clear();
        for (e, p, h) in world.query::<(Entity, &Pos, Option<&Health>)>().iter() {
            typed.extend_from_slice(&e.id().to_le_bytes());
            put_pos(&mut typed, p);
            if let Some(h) = h {
                put_health(&mut typed, h);
            }
        }
    });
    // A registry of replicated components: (name, writer). The writer looks the component up
    // by type on one entity, as a reflection layer would.
    type Writer = fn(EntityRef<'_>, &mut Vec<u8>);
    let registry: Vec<(&str, Writer)> = vec![
        ("Pos", |e, out| {
            if let Some(p) = e.get::<&Pos>() {
                put_pos(out, &p);
            }
        }),
        ("Health", |e, out| {
            if let Some(h) = e.get::<&Health>() {
                put_health(out, &h);
            }
        }),
    ];
    let mut generic = Vec::new();
    let snapshot_registry_us = median_us(20, || {
        generic.clear();
        for e in world.iter() {
            generic.extend_from_slice(&e.entity().id().to_le_bytes());
            for (_, write) in &registry {
                write(e, &mut generic);
            }
        }
    });
    let mut stale_ok = true;
    let churn_us = median_us(1, || {
        let victims: Vec<Entity> = ids.iter().step_by(10).copied().collect();
        let victim_set: std::collections::HashSet<Entity> = victims.iter().copied().collect();
        for e in &victims {
            world.despawn(*e).unwrap();
        }
        stale_ok = victims.iter().all(|e| !world.contains(*e));
        for i in 0..victims.len() {
            let e = world.spawn((pos_of(i), Team(0)));
            stale_ok &= !victim_set.contains(&e);
        }
    });
    Row {
        name: "hecs 0.11",
        spawn_us,
        step_us,
        snapshot_typed_us,
        snapshot_registry_us,
        churn_us,
        typed_bytes: typed.len(),
        registry_bytes: generic.len(),
        order_fingerprint: world.iter().fold(0, |h, e| fold(h, e.entity().id())),
        stale_handles_detected: stale_ok,
    }
}

// ------------------------------------------------------------------------------------- bevy_ecs

fn run_bevy() -> Row {
    use bevy_ecs::prelude::*;
    let spawn = |world: &mut World, ids: &mut Vec<Entity>| {
        for i in 0..N {
            let e = if is_mobile(i) {
                world.spawn((BPos(pos_of(i)), BVel(Vel { x: 1.0, y: 0.0, z: -1.0 }), BHealth(Health(100)), BTeam(Team((i % 4) as u8)))).id()
            } else {
                world.spawn((BPos(pos_of(i)), BTeam(Team((i % 4) as u8)))).id()
            };
            ids.push(e);
        }
    };
    let spawn_us = median_us(15, || {
        let mut w = World::new();
        let mut ids = Vec::with_capacity(N);
        spawn(&mut w, &mut ids);
        std::hint::black_box(&w);
    });
    let mut world = World::new();
    let mut ids = Vec::with_capacity(N);
    spawn(&mut world, &mut ids);
    let mut step_query = world.query::<(&mut BPos, &BVel)>();
    let step_us = median_us(60, || {
        for (mut p, v) in step_query.iter_mut(&mut world) {
            p.0.x += v.0.x * 0.016;
            p.0.y += v.0.y * 0.016;
            p.0.z += v.0.z * 0.016;
        }
    });
    let mut typed = Vec::new();
    let mut snap_query = world.query::<(Entity, &BPos, Option<&BHealth>)>();
    let snapshot_typed_us = median_us(40, || {
        typed.clear();
        for (e, p, h) in snap_query.iter(&world) {
            typed.extend_from_slice(&e.index_u32().to_le_bytes());
            put_pos(&mut typed, &p.0);
            if let Some(h) = h {
                put_health(&mut typed, &h.0);
            }
        }
    });
    type Writer = fn(EntityRef<'_>, &mut Vec<u8>);
    let registry: Vec<(&str, Writer)> = vec![
        ("Pos", |e, out| {
            if let Some(p) = e.get::<BPos>() {
                put_pos(out, &p.0);
            }
        }),
        ("Health", |e, out| {
            if let Some(h) = e.get::<BHealth>() {
                put_health(out, &h.0);
            }
        }),
    ];
    let mut generic = Vec::new();
    let mut all_entities = world.query::<EntityRef>();
    let snapshot_registry_us = median_us(20, || {
        generic.clear();
        for e in all_entities.iter(&world) {
            generic.extend_from_slice(&e.id().index_u32().to_le_bytes());
            for (_, write) in &registry {
                write(e, &mut generic);
            }
        }
    });
    let mut stale_ok = true;
    let churn_us = median_us(1, || {
        let victims: Vec<Entity> = ids.iter().step_by(10).copied().collect();
        let victim_set: std::collections::HashSet<Entity> = victims.iter().copied().collect();
        for e in &victims {
            world.despawn(*e);
        }
        stale_ok = victims.iter().all(|e| world.get_entity(*e).is_err());
        for i in 0..victims.len() {
            let e = world.spawn((BPos(pos_of(i)), BTeam(Team(0)))).id();
            stale_ok &= !victim_set.contains(&e);
        }
    });
    Row {
        name: "bevy_ecs 0.20",
        spawn_us,
        step_us,
        snapshot_typed_us,
        snapshot_registry_us,
        churn_us,
        typed_bytes: typed.len(),
        registry_bytes: generic.len(),
        order_fingerprint: all_entities.iter(&world).fold(0, |h, e| fold(h, e.id().index_u32())),
        stale_handles_detected: stale_ok,
    }
}

// ----------------------------------------------------------------------------- custom sparse set

mod custom {
    use std::any::{Any, TypeId};
    use std::collections::BTreeMap;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Id {
        pub index: u32,
        pub generation: u32,
    }

    /// One column per component type: a dense array plus the owner of each element and a sparse
    /// index from entity slot to dense position.
    struct Column<T> {
        dense: Vec<T>,
        owner: Vec<u32>,
        sparse: Vec<u32>,
    }

    const NONE: u32 = u32::MAX;

    impl<T> Column<T> {
        fn new() -> Self {
            Self { dense: Vec::new(), owner: Vec::new(), sparse: Vec::new() }
        }
        fn insert(&mut self, slot: u32, value: T) {
            if self.sparse.len() <= slot as usize {
                self.sparse.resize(slot as usize + 1, NONE);
            }
            self.sparse[slot as usize] = self.dense.len() as u32;
            self.dense.push(value);
            self.owner.push(slot);
        }
        fn get(&self, slot: u32) -> Option<&T> {
            let at = *self.sparse.get(slot as usize)?;
            (at != NONE).then(|| &self.dense[at as usize])
        }
        fn remove(&mut self, slot: u32) {
            let Some(&at) = self.sparse.get(slot as usize) else { return };
            if at == NONE {
                return;
            }
            let last = self.dense.len() - 1;
            self.dense.swap_remove(at as usize);
            self.owner.swap_remove(at as usize);
            if (at as usize) < last {
                self.sparse[self.owner[at as usize] as usize] = at;
            }
            self.sparse[slot as usize] = NONE;
        }
    }

    trait AnyColumn {
        fn as_any(&self) -> &dyn Any;
        fn as_any_mut(&mut self) -> &mut dyn Any;
        fn remove_slot(&mut self, slot: u32);
    }

    impl<T: 'static> AnyColumn for Column<T> {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
        fn remove_slot(&mut self, slot: u32) {
            self.remove(slot);
        }
    }

    pub struct World {
        generations: Vec<u32>,
        free: Vec<u32>,
        columns: BTreeMap<TypeId, Box<dyn AnyColumn>>,
    }

    impl World {
        pub fn new() -> Self {
            Self { generations: Vec::new(), free: Vec::new(), columns: BTreeMap::new() }
        }
        pub fn create(&mut self) -> Id {
            if let Some(index) = self.free.pop() {
                Id { index, generation: self.generations[index as usize] }
            } else {
                self.generations.push(0);
                Id { index: self.generations.len() as u32 - 1, generation: 0 }
            }
        }
        pub fn alive(&self, id: Id) -> bool {
            self.generations.get(id.index as usize) == Some(&id.generation)
        }
        pub fn add<T: 'static>(&mut self, id: Id, value: T) {
            self.columns
                .entry(TypeId::of::<T>())
                .or_insert_with(|| Box::new(Column::<T>::new()))
                .as_any_mut()
                .downcast_mut::<Column<T>>()
                .unwrap()
                .insert(id.index, value);
        }
        pub fn get<T: 'static>(&self, id: Id) -> Option<&T> {
            if !self.alive(id) {
                return None;
            }
            self.columns.get(&TypeId::of::<T>())?.as_any().downcast_ref::<Column<T>>()?.get(id.index)
        }
        pub fn despawn(&mut self, id: Id) -> bool {
            if !self.alive(id) {
                return false;
            }
            for column in self.columns.values_mut() {
                column.remove_slot(id.index);
            }
            self.generations[id.index as usize] += 1;
            self.free.push(id.index);
            true
        }
        fn column<T: 'static>(&self) -> Option<&Column<T>> {
            self.columns.get(&TypeId::of::<T>())?.as_any().downcast_ref::<Column<T>>()
        }
        /// Position += velocity for everything with both: walk the smaller column, look the other up.
        pub fn step<P: 'static, V: 'static>(&mut self, apply: impl Fn(&mut P, &V)) {
            let (Some(pc), Some(vc)) = (self.columns.get(&TypeId::of::<P>()), self.columns.get(&TypeId::of::<V>())) else {
                return;
            };
            let vc = vc.as_any().downcast_ref::<Column<V>>().unwrap();
            let owners: Vec<(u32, u32)> = vc.owner.iter().enumerate().map(|(i, o)| (i as u32, *o)).collect();
            let _ = pc;
            let mut pcol = self.columns.remove(&TypeId::of::<P>()).unwrap();
            {
                let pcol = pcol.as_any_mut().downcast_mut::<Column<P>>().unwrap();
                let vc = self.column::<V>().unwrap();
                for (di, slot) in owners {
                    let at = pcol.sparse[slot as usize];
                    apply(&mut pcol.dense[at as usize], &vc.dense[di as usize]);
                }
            }
            self.columns.insert(TypeId::of::<P>(), pcol);
        }
        /// Visit every entity that has `P`, with its optional `O`.
        pub fn each_with_optional<P: 'static, O: 'static>(&self, mut visit: impl FnMut(Id, &P, Option<&O>)) {
            let Some(pc) = self.column::<P>() else { return };
            let oc = self.column::<O>();
            for (value, slot) in pc.dense.iter().zip(&pc.owner) {
                let id = Id { index: *slot, generation: self.generations[*slot as usize] };
                visit(id, value, oc.and_then(|c| c.get(*slot)));
            }
        }
        pub fn slots(&self) -> impl Iterator<Item = Id> + '_ {
            self.generations.iter().enumerate().filter_map(|(i, g)| {
                (!self.free.contains(&(i as u32))).then_some(Id { index: i as u32, generation: *g })
            })
        }
    }
}

fn run_custom() -> Row {
    use custom::{Id, World};
    let spawn = |world: &mut World, ids: &mut Vec<Id>| {
        for i in 0..N {
            let id = world.create();
            world.add(id, pos_of(i));
            if is_mobile(i) {
                world.add(id, Vel { x: 1.0, y: 0.0, z: -1.0 });
                world.add(id, Health(100));
            }
            world.add(id, Team((i % 4) as u8));
            ids.push(id);
        }
    };
    let spawn_us = median_us(15, || {
        let mut w = World::new();
        let mut ids = Vec::with_capacity(N);
        spawn(&mut w, &mut ids);
        std::hint::black_box(&w);
    });
    let mut world = World::new();
    let mut ids = Vec::with_capacity(N);
    spawn(&mut world, &mut ids);
    let step_us = median_us(60, || {
        world.step::<Pos, Vel>(|p, v| {
            p.x += v.x * 0.016;
            p.y += v.y * 0.016;
            p.z += v.z * 0.016;
        });
    });
    let mut typed = Vec::new();
    let snapshot_typed_us = median_us(40, || {
        typed.clear();
        world.each_with_optional::<Pos, Health>(|id, p, h| {
            typed.extend_from_slice(&id.index.to_le_bytes());
            put_pos(&mut typed, p);
            if let Some(h) = h {
                put_health(&mut typed, h);
            }
        });
    });
    // The registry walk visits every live entity and asks each registered writer.
    type Writer = fn(&World, Id, &mut Vec<u8>);
    let registry: Vec<(&str, Writer)> = vec![
        ("Pos", |w, id, out| {
            if let Some(p) = w.get::<Pos>(id) {
                put_pos(out, p);
            }
        }),
        ("Health", |w, id, out| {
            if let Some(h) = w.get::<Health>(id) {
                put_health(out, h);
            }
        }),
    ];
    let mut generic = Vec::new();
    let live: Vec<Id> = ids.clone();
    let snapshot_registry_us = median_us(20, || {
        generic.clear();
        for id in &live {
            generic.extend_from_slice(&id.index.to_le_bytes());
            for (_, write) in &registry {
                write(&world, *id, &mut generic);
            }
        }
    });
    let mut stale_ok = true;
    let churn_us = median_us(1, || {
        let victims: Vec<Id> = ids.iter().step_by(10).copied().collect();
        let victim_set: std::collections::HashSet<(u32, u32)> = victims.iter().map(|v| (v.index, v.generation)).collect();
        for v in &victims {
            world.despawn(*v);
        }
        stale_ok = victims.iter().all(|v| !world.alive(*v));
        for i in 0..victims.len() {
            let id = world.create();
            world.add(id, pos_of(i));
            world.add(id, Team(0));
            stale_ok &= !victim_set.contains(&(id.index, id.generation));
        }
    });
    let _ = world.slots().count();
    Row {
        name: "custom sparse set",
        spawn_us,
        step_us,
        snapshot_typed_us,
        snapshot_registry_us,
        churn_us,
        typed_bytes: typed.len(),
        registry_bytes: generic.len(),
        order_fingerprint: world.slots().fold(0, |h, id| fold(h, id.index)),
        stale_handles_detected: stale_ok,
    }
}

fn main() {
    println!("entities: {N} (70 % mobile); times are medians in microseconds (lower is better)");
    println!("{:<20}{:>10}{:>10}{:>14}{:>16}{:>10}{:>12}", "", "spawn", "step", "snap typed", "snap registry", "churn", "stale ok");
    let rows = [run_hecs(), run_bevy(), run_custom()];
    for r in &rows {
        println!(
            "{:<20}{:>10.0}{:>10.0}{:>14.0}{:>16.0}{:>10.0}{:>12}",
            r.name, r.spawn_us, r.step_us, r.snapshot_typed_us, r.snapshot_registry_us, r.churn_us, r.stale_handles_detected
        );
    }
    println!("expected snapshot bytes: {} (50,000 x (4-byte id + 12-byte position) + 35,000 x 4-byte health); bevy_ecs also walks one entity of its own in the registry snapshot", N * 16 + N * 7 / 10 * 4);
    for r in &rows {
        println!("{:<20} snapshot bytes: typed {}, registry {}; iteration order fingerprint {:#x}", r.name, r.typed_bytes, r.registry_bytes, r.order_fingerprint);
    }
}
