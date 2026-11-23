//! The world: the entities of a simulation, as the rest of the engine sees them (roadmap T00-017, RFC 0009).
//!
//! [`World`] is the **contract**: what spawning, despawning, parenting, reading and writing a
//! property, and finding out what changed look like, whatever stores the entities (RFC 0005 proposes
//! `hecs` behind this trait). [`SimpleWorld`] is the **reference implementation**: slow and plain,
//! deterministic, and the double that replication, the server and the tests can use today.
//! [`conformance`] is a test suite any implementation must pass.
//!
//! The rules, which an implementation must follow:
//!
//! * **Handles are checked.** Despawning bumps the slot's generation; a handle to a dead entity is an
//!   error ([`WorldError::Dead`]), never a different entity. Free slots are reused **lowest index first**, so the
//!   same calls give the same handles on every machine.
//! * **Properties are type-checked** against the class: a value of the wrong type is refused.
//! * **Changes are recorded for replication.** [`World::set`] records `(entity, slot)` when the value
//!   differs (bit for bit) and the property is replicated; [`World::drain_changes`] returns them sorted and without repeats and
//!   forgets them. A despawned entity's changes are dropped.
//! * **Lifecycle is recorded** in order ([`LifecycleEvent`]), for the replication layer and the event bus;
//!   [`World::drain_lifecycle`] returns and forgets them.
//! * **Despawning a parent despawns its children**, deepest first, and reports how many entities went.
//! * **Order is deterministic**: [`World::entities`] is by slot index; children are by slot index.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::entity::{ClassId, EntityId};
use crate::reflect::{ClassRegistry, PropertyType, PropertyValue, ReplicationMode};

/// Why a world operation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    /// The class id is not in the registry.
    UnknownClass(ClassId),
    /// The handle is stale, null or was never issued.
    Dead(EntityId),
    /// The class has no such property.
    NoSuchProperty {
        /// The class of the entity.
        class: String,
        /// The name asked for.
        property: String,
    },
    /// The value is not of the property's type.
    TypeMismatch {
        /// The property's type.
        expected: PropertyType,
        /// The type of the value given.
        got: PropertyType,
    },
    /// The new parent would be the entity itself or one of its descendants.
    Cycle,
    /// The world holds as many entities as it may.
    Full,
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownClass(c) => write!(f, "unknown class {}", c.0),
            Self::Dead(e) => write!(f, "entity {e} does not exist"),
            Self::NoSuchProperty { class, property } => {
                write!(f, "class {class:?} has no property {property:?}")
            }
            Self::TypeMismatch { expected, got } => {
                write!(f, "expected a {expected:?}, got a {got:?}")
            }
            Self::Cycle => write!(f, "an entity cannot be its own ancestor"),
            Self::Full => write!(f, "the world is full"),
        }
    }
}

impl std::error::Error for WorldError {}

/// Something that happened to the set of entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// An entity was created.
    Spawned(EntityId),
    /// An entity was destroyed (children before their parent).
    Destroyed(EntityId),
}

/// A replicated property that changed: the entity and the property's slot in its class
/// ([`ClassRegistry::properties`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Change {
    /// The entity.
    pub entity: EntityId,
    /// The slot of the property.
    pub slot: u16,
}

/// What the simulation, the replication layer, the save system and the console may do with entities.
pub trait World {
    /// The classes.
    fn registry(&self) -> &ClassRegistry;

    /// Create an entity of `class` with the default values, optionally under `parent`.
    fn spawn(&mut self, class: ClassId, parent: Option<EntityId>) -> Result<EntityId, WorldError>;

    /// Destroy the entity and its descendants; returns how many entities were destroyed.
    fn despawn(&mut self, id: EntityId) -> Result<usize, WorldError>;

    /// Whether the handle refers to a living entity.
    fn is_alive(&self, id: EntityId) -> bool;

    /// How many entities are alive.
    fn len(&self) -> usize;

    /// Whether there are none.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The class of an entity.
    fn class_of(&self, id: EntityId) -> Result<ClassId, WorldError>;

    /// The parent of an entity.
    fn parent_of(&self, id: EntityId) -> Result<Option<EntityId>, WorldError>;

    /// Move an entity under another (or to the top with `None`).
    fn set_parent(&mut self, id: EntityId, parent: Option<EntityId>) -> Result<(), WorldError>;

    /// The children of an entity, by slot index.
    fn children_of(&self, id: EntityId) -> Result<Vec<EntityId>, WorldError>;

    /// The value of a property by name.
    fn get(&self, id: EntityId, property: &str) -> Result<PropertyValue, WorldError>;

    /// Set a property by name; the value must have the property's type.
    fn set(&mut self, id: EntityId, property: &str, value: PropertyValue)
    -> Result<(), WorldError>;

    /// The value of a property by slot (faster than by name; `None` for a bad slot).
    fn get_slot(&self, id: EntityId, slot: u16) -> Result<Option<PropertyValue>, WorldError>;

    /// Every living entity, by slot index.
    fn entities(&self) -> Vec<EntityId>;

    /// The replicated properties changed since the last call, sorted, without repeats.
    fn drain_changes(&mut self) -> Vec<Change>;

    /// The spawns and destroys since the last call, in order.
    fn drain_lifecycle(&mut self) -> Vec<LifecycleEvent>;
}

struct Entity {
    class: ClassId,
    parent: Option<EntityId>,
    children: BTreeSet<u32>,
    values: Vec<PropertyValue>,
}

struct Slot {
    generation: u32,
    entity: Option<Entity>,
}

/// The default limit of entities in a [`SimpleWorld`].
pub const DEFAULT_MAX_ENTITIES: usize = 1 << 20;

/// The reference [`World`]: a vector of slots. Plain and deterministic, not fast.
pub struct SimpleWorld {
    registry: ClassRegistry,
    slots: Vec<Slot>,
    free: BTreeSet<u32>,
    alive: usize,
    max_entities: usize,
    changes: BTreeSet<Change>,
    lifecycle: Vec<LifecycleEvent>,
}

impl SimpleWorld {
    /// A world of the classes in `registry`, with the default entity limit.
    pub fn new(registry: ClassRegistry) -> Self {
        Self::with_limit(registry, DEFAULT_MAX_ENTITIES)
    }

    /// A world that holds at most `max_entities` entities.
    pub fn with_limit(registry: ClassRegistry, max_entities: usize) -> Self {
        Self {
            registry,
            slots: Vec::new(),
            free: BTreeSet::new(),
            alive: 0,
            max_entities,
            changes: BTreeSet::new(),
            lifecycle: Vec::new(),
        }
    }

    fn entity(&self, id: EntityId) -> Result<&Entity, WorldError> {
        self.slots
            .get(id.index() as usize)
            .filter(|s| s.generation == id.generation())
            .and_then(|s| s.entity.as_ref())
            .ok_or(WorldError::Dead(id))
    }

    fn entity_mut(&mut self, id: EntityId) -> Result<&mut Entity, WorldError> {
        self.slots
            .get_mut(id.index() as usize)
            .filter(|s| s.generation == id.generation())
            .and_then(|s| s.entity.as_mut())
            .ok_or(WorldError::Dead(id))
    }

    fn handle_of(&self, index: u32) -> EntityId {
        EntityId::new(index, self.slots[index as usize].generation)
    }

    /// Whether `ancestor` is `id` or above it.
    fn is_ancestor_or_self(&self, ancestor: EntityId, id: EntityId) -> bool {
        let mut current = Some(id);
        while let Some(c) = current {
            if c == ancestor {
                return true;
            }
            current = self.entity(c).ok().and_then(|e| e.parent);
        }
        false
    }

    /// Destroy one entity (its children already gone).
    fn destroy_one(&mut self, id: EntityId) {
        let Some(slot) = self.slots.get_mut(id.index() as usize) else {
            return;
        };
        let Some(entity) = slot.entity.take() else {
            return;
        };
        slot.generation = slot.generation.wrapping_add(1);
        self.free.insert(id.index());
        self.alive -= 1;
        if let Some(parent) = entity.parent {
            if let Ok(p) = self.entity_mut(parent) {
                p.children.remove(&id.index());
            }
        }
        self.changes.retain(|c| c.entity != id);
        self.lifecycle.push(LifecycleEvent::Destroyed(id));
    }
}

impl World for SimpleWorld {
    fn registry(&self) -> &ClassRegistry {
        &self.registry
    }

    fn spawn(&mut self, class: ClassId, parent: Option<EntityId>) -> Result<EntityId, WorldError> {
        let values: Vec<PropertyValue> = self
            .registry
            .properties(class)
            .ok_or(WorldError::UnknownClass(class))?
            .iter()
            .map(|p| p.default.clone())
            .collect();
        if let Some(p) = parent {
            self.entity(p)?;
        }
        if self.alive >= self.max_entities {
            return Err(WorldError::Full);
        }
        let index = match self.free.pop_first() {
            Some(i) => i,
            None => {
                self.slots.push(Slot {
                    generation: 0,
                    entity: None,
                });
                (self.slots.len() - 1) as u32
            }
        };
        self.slots[index as usize].entity = Some(Entity {
            class,
            parent,
            children: BTreeSet::new(),
            values,
        });
        self.alive += 1;
        let id = self.handle_of(index);
        if let Some(p) = parent {
            self.entity_mut(p)?.children.insert(index);
        }
        self.lifecycle.push(LifecycleEvent::Spawned(id));
        Ok(id)
    }

    fn despawn(&mut self, id: EntityId) -> Result<usize, WorldError> {
        self.entity(id)?;
        // Children first, deepest first: collect in pre-order and destroy in reverse.
        let mut order = vec![id];
        let mut next = 0;
        while next < order.len() {
            let children: Vec<EntityId> = self
                .entity(order[next])?
                .children
                .iter()
                .map(|&i| self.handle_of(i))
                .collect();
            order.extend(children);
            next += 1;
        }
        let count = order.len();
        for e in order.into_iter().rev() {
            self.destroy_one(e);
        }
        Ok(count)
    }

    fn is_alive(&self, id: EntityId) -> bool {
        self.entity(id).is_ok()
    }

    fn len(&self) -> usize {
        self.alive
    }

    fn class_of(&self, id: EntityId) -> Result<ClassId, WorldError> {
        Ok(self.entity(id)?.class)
    }

    fn parent_of(&self, id: EntityId) -> Result<Option<EntityId>, WorldError> {
        Ok(self.entity(id)?.parent)
    }

    fn set_parent(&mut self, id: EntityId, parent: Option<EntityId>) -> Result<(), WorldError> {
        self.entity(id)?;
        if let Some(p) = parent {
            self.entity(p)?;
            if self.is_ancestor_or_self(id, p) {
                return Err(WorldError::Cycle);
            }
        }
        let old = self.entity(id)?.parent;
        if let Some(old) = old {
            self.entity_mut(old)?.children.remove(&id.index());
        }
        if let Some(p) = parent {
            self.entity_mut(p)?.children.insert(id.index());
        }
        self.entity_mut(id)?.parent = parent;
        Ok(())
    }

    fn children_of(&self, id: EntityId) -> Result<Vec<EntityId>, WorldError> {
        Ok(self
            .entity(id)?
            .children
            .iter()
            .map(|&i| self.handle_of(i))
            .collect())
    }

    fn get(&self, id: EntityId, property: &str) -> Result<PropertyValue, WorldError> {
        let entity = self.entity(id)?;
        let slot = self
            .registry
            .property_slot(entity.class, property)
            .ok_or_else(|| WorldError::NoSuchProperty {
                class: self
                    .registry
                    .class_name(entity.class)
                    .unwrap_or_default()
                    .to_string(),
                property: property.to_string(),
            })?;
        Ok(entity.values[slot].clone())
    }

    fn set(
        &mut self,
        id: EntityId,
        property: &str,
        value: PropertyValue,
    ) -> Result<(), WorldError> {
        let class = self.entity(id)?.class;
        let slot = self
            .registry
            .property_slot(class, property)
            .ok_or_else(|| WorldError::NoSuchProperty {
                class: self
                    .registry
                    .class_name(class)
                    .unwrap_or_default()
                    .to_string(),
                property: property.to_string(),
            })?;
        let desc = &self
            .registry
            .properties(class)
            .ok_or(WorldError::UnknownClass(class))?[slot];
        if desc.ty != value.ty() {
            return Err(WorldError::TypeMismatch {
                expected: desc.ty,
                got: value.ty(),
            });
        }
        let replicated = desc.replication.mode != ReplicationMode::None;
        let entity = self.entity_mut(id)?;
        let changed = !entity.values[slot].same_bits(&value);
        entity.values[slot] = value;
        if changed && replicated {
            self.changes.insert(Change {
                entity: id,
                slot: slot as u16,
            });
        }
        Ok(())
    }

    fn get_slot(&self, id: EntityId, slot: u16) -> Result<Option<PropertyValue>, WorldError> {
        Ok(self.entity(id)?.values.get(usize::from(slot)).cloned())
    }

    fn entities(&self) -> Vec<EntityId> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.entity.is_some())
            .map(|(i, s)| EntityId::new(i as u32, s.generation))
            .collect()
    }

    fn drain_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes).into_iter().collect()
    }

    fn drain_lifecycle(&mut self) -> Vec<LifecycleEvent> {
        std::mem::take(&mut self.lifecycle)
    }
}

/// A tally of how many entities of each class a world holds, by class name (for tools and tests).
pub fn census<W: World>(world: &W) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for id in world.entities() {
        if let Some(name) = world
            .class_of(id)
            .ok()
            .and_then(|c| world.registry().class_name(c))
        {
            *counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

/// The test suite every [`World`] implementation must pass: call it from a test with a function that
/// makes a world from a registry. It panics, naming the rule, when an implementation breaks one.
pub mod conformance {
    use super::*;
    use crate::reflect::{ClassDesc, PropertyDesc};

    /// The registry the suite uses: `Thing` (an `origin` that is replicated, a `label` that is not) and
    /// `Mob` extending it (`health`, replicated, default 100).
    pub fn registry() -> ClassRegistry {
        let mut r = ClassRegistry::new();
        let thing = r
            .register(
                ClassDesc::new("Thing")
                    .property(
                        PropertyDesc::new("origin", PropertyType::Vec3)
                            .replicated(ReplicationMode::Always),
                    )
                    .property(PropertyDesc::new("label", PropertyType::Text)),
            )
            .unwrap();
        r.register(
            ClassDesc::new("Mob")
                .extends(thing)
                .property(
                    PropertyDesc::new("health", PropertyType::F32)
                        .default_value(PropertyValue::F32(100.0))
                        .replicated(ReplicationMode::Always),
                )
                .property(
                    PropertyDesc::new("owner", PropertyType::Entity)
                        .replicated(ReplicationMode::SpawnOnly),
                ),
        )
        .unwrap();
        r
    }

    /// Run the whole suite against worlds made by `make`.
    pub fn run<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        spawn_and_defaults(make);
        stale_handles_are_dead(make);
        slots_are_reused_lowest_first(make);
        properties_are_checked(make);
        changes_are_recorded_for_replicated_properties_only(make);
        lifecycle_is_recorded_in_order(make);
        parenting(make);
        despawn_takes_the_children(make);
        order_is_by_index(make);
    }

    fn spawn_and_defaults<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let mob = w.registry().class_id("Mob").unwrap();
        let e = w.spawn(mob, None).unwrap();
        assert!(w.is_alive(e), "a new entity is alive");
        assert_eq!((w.len(), w.is_empty()), (1, false));
        assert_eq!(w.class_of(e).unwrap(), mob);
        assert_eq!(w.parent_of(e).unwrap(), None);
        assert_eq!(
            w.get(e, "health").unwrap(),
            PropertyValue::F32(100.0),
            "defaults come from the class"
        );
        assert_eq!(
            w.get(e, "origin").unwrap(),
            PropertyValue::Vec3([0.0; 3]),
            "inherited properties are there"
        );
        assert_eq!(
            w.get(e, "owner").unwrap(),
            PropertyValue::Entity(EntityId::NULL)
        );
        assert_eq!(
            w.spawn(ClassId(99), None),
            Err(WorldError::UnknownClass(ClassId(99))),
            "an unknown class is refused"
        );
        assert_eq!(w.len(), 1, "a refused spawn creates nothing");
        assert_eq!(
            w.get_slot(e, 2).unwrap(),
            Some(PropertyValue::F32(100.0)),
            "slot 2 of Mob is health"
        );
        assert_eq!(
            w.get_slot(e, 400).unwrap(),
            None,
            "a bad slot is None, not a panic"
        );
    }

    fn stale_handles_are_dead<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let a = w.spawn(thing, None).unwrap();
        assert_eq!(w.despawn(a).unwrap(), 1);
        assert!(!w.is_alive(a), "a destroyed entity is dead");
        assert_eq!(w.get(a, "origin"), Err(WorldError::Dead(a)));
        assert_eq!(
            w.despawn(a),
            Err(WorldError::Dead(a)),
            "destroying twice is an error"
        );
        let b = w.spawn(thing, None).unwrap();
        assert_eq!(b.index(), a.index(), "the slot is reused");
        assert_ne!(b.generation(), a.generation(), "with another generation");
        assert!(
            !w.is_alive(a),
            "so the old handle does not mean the new entity"
        );
        assert!(w.is_alive(b));
        assert!(
            !w.is_alive(EntityId::NULL),
            "the null handle is never alive"
        );
        assert!(
            !w.is_alive(EntityId::new(9999, 0)),
            "a handle never issued is not alive"
        );
        assert_eq!(
            w.set(a, "label", PropertyValue::Text("x".into())),
            Err(WorldError::Dead(a))
        );
    }

    fn slots_are_reused_lowest_first<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let ids: Vec<EntityId> = (0..5).map(|_| w.spawn(thing, None).unwrap()).collect();
        assert_eq!(
            ids.iter().map(|i| i.index()).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4],
            "slots are issued in order"
        );
        w.despawn(ids[3]).unwrap();
        w.despawn(ids[1]).unwrap();
        assert_eq!(
            w.spawn(thing, None).unwrap().index(),
            1,
            "lowest free slot first"
        );
        assert_eq!(w.spawn(thing, None).unwrap().index(), 3);
        assert_eq!(w.spawn(thing, None).unwrap().index(), 5, "then a new slot");
    }

    fn properties_are_checked<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let e = w.spawn(thing, None).unwrap();
        w.set(e, "label", PropertyValue::Text("door".into()))
            .unwrap();
        assert_eq!(
            w.get(e, "label").unwrap(),
            PropertyValue::Text("door".into())
        );
        assert_eq!(
            w.set(e, "label", PropertyValue::F32(1.0)),
            Err(WorldError::TypeMismatch {
                expected: PropertyType::Text,
                got: PropertyType::F32
            })
        );
        assert_eq!(
            w.get(e, "label").unwrap(),
            PropertyValue::Text("door".into()),
            "a refused write changes nothing"
        );
        assert!(
            matches!(w.get(e, "health"), Err(WorldError::NoSuchProperty { .. })),
            "a Thing has no health"
        );
        assert!(
            matches!(w.set(e, "health", PropertyValue::F32(1.0)), Err(WorldError::NoSuchProperty { class, .. }) if class == "Thing")
        );
    }

    fn changes_are_recorded_for_replicated_properties_only<W: World>(
        make: &dyn Fn(ClassRegistry) -> W,
    ) {
        let mut w = make(registry());
        let mob = w.registry().class_id("Mob").unwrap();
        let (a, b) = (w.spawn(mob, None).unwrap(), w.spawn(mob, None).unwrap());
        w.drain_changes();
        w.set(b, "health", PropertyValue::F32(50.0)).unwrap();
        w.set(a, "health", PropertyValue::F32(75.0)).unwrap();
        w.set(a, "health", PropertyValue::F32(60.0)).unwrap();
        w.set(a, "label", PropertyValue::Text("x".into())).unwrap();
        w.set(a, "origin", PropertyValue::Vec3([1.0, 2.0, 3.0]))
            .unwrap();
        let changes = w.drain_changes();
        assert_eq!(
            changes,
            vec![
                Change { entity: a, slot: 0 },
                Change { entity: a, slot: 2 },
                Change { entity: b, slot: 2 }
            ],
            "sorted, once each, and the unreplicated label is not in it"
        );
        assert!(w.drain_changes().is_empty(), "draining forgets");
        w.set(a, "health", PropertyValue::F32(60.0)).unwrap();
        assert!(
            w.drain_changes().is_empty(),
            "setting the same value is not a change"
        );
        w.set(a, "origin", PropertyValue::Vec3([0.0, 0.0, 0.0]))
            .unwrap();
        w.drain_changes();
        w.set(a, "origin", PropertyValue::Vec3([-0.0, 0.0, 0.0]))
            .unwrap();
        assert_eq!(
            w.drain_changes().len(),
            1,
            "-0.0 is not the same bits as 0.0"
        );
        w.set(b, "health", PropertyValue::F32(1.0)).unwrap();
        w.despawn(b).unwrap();
        assert!(
            w.drain_changes().is_empty(),
            "a destroyed entity's changes are dropped"
        );
    }

    fn lifecycle_is_recorded_in_order<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let a = w.spawn(thing, None).unwrap();
        let b = w.spawn(thing, None).unwrap();
        w.despawn(a).unwrap();
        let c = w.spawn(thing, None).unwrap();
        assert_eq!(
            w.drain_lifecycle(),
            vec![
                LifecycleEvent::Spawned(a),
                LifecycleEvent::Spawned(b),
                LifecycleEvent::Destroyed(a),
                LifecycleEvent::Spawned(c)
            ]
        );
        assert!(w.drain_lifecycle().is_empty());
        let failed = w.despawn(a);
        assert!(failed.is_err());
        assert!(
            w.drain_lifecycle().is_empty(),
            "a failed call records nothing"
        );
    }

    fn parenting<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let a = w.spawn(thing, None).unwrap();
        let b = w.spawn(thing, Some(a)).unwrap();
        let c = w.spawn(thing, Some(a)).unwrap();
        assert_eq!(w.parent_of(b).unwrap(), Some(a));
        assert_eq!(w.children_of(a).unwrap(), vec![b, c]);
        assert_eq!(
            w.spawn(thing, Some(EntityId::NULL)),
            Err(WorldError::Dead(EntityId::NULL))
        );
        assert_eq!(w.len(), 3, "a spawn under a dead parent creates nothing");
        w.set_parent(c, Some(b)).unwrap();
        assert_eq!(w.children_of(a).unwrap(), vec![b]);
        assert_eq!(w.children_of(b).unwrap(), vec![c]);
        assert_eq!(
            w.set_parent(a, Some(c)),
            Err(WorldError::Cycle),
            "no cycles"
        );
        assert_eq!(
            w.set_parent(a, Some(a)),
            Err(WorldError::Cycle),
            "not even by itself"
        );
        assert_eq!(
            w.parent_of(a).unwrap(),
            None,
            "a refused move changes nothing"
        );
        w.set_parent(c, None).unwrap();
        assert_eq!(
            (w.parent_of(c).unwrap(), w.children_of(b).unwrap()),
            (None, vec![])
        );
    }

    fn despawn_takes_the_children<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let a = w.spawn(thing, None).unwrap();
        let b = w.spawn(thing, Some(a)).unwrap();
        let c = w.spawn(thing, Some(b)).unwrap();
        let d = w.spawn(thing, Some(a)).unwrap();
        let other = w.spawn(thing, None).unwrap();
        w.drain_lifecycle();
        assert_eq!(w.despawn(a).unwrap(), 4);
        assert!(!w.is_alive(a) && !w.is_alive(b) && !w.is_alive(c) && !w.is_alive(d));
        assert!(w.is_alive(other));
        assert_eq!(w.len(), 1);
        let destroyed: Vec<EntityId> = w
            .drain_lifecycle()
            .into_iter()
            .map(|e| match e {
                LifecycleEvent::Destroyed(id) => id,
                other => panic!("only destroys expected, got {other:?}"),
            })
            .collect();
        let position = |id| destroyed.iter().position(|d| *d == id).unwrap();
        assert!(
            position(c) < position(b) && position(b) < position(a),
            "children are destroyed before their parent"
        );
        assert!(position(d) < position(a));
        // Destroying a child takes it out of its parent.
        let p = w.spawn(thing, None).unwrap();
        let k = w.spawn(thing, Some(p)).unwrap();
        w.despawn(k).unwrap();
        assert!(w.children_of(p).unwrap().is_empty());
    }

    fn order_is_by_index<W: World>(make: &dyn Fn(ClassRegistry) -> W) {
        let mut w = make(registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let ids: Vec<EntityId> = (0..6).map(|_| w.spawn(thing, None).unwrap()).collect();
        w.despawn(ids[2]).unwrap();
        w.despawn(ids[0]).unwrap();
        let again = w.spawn(thing, None).unwrap();
        assert_eq!(again.index(), 0);
        let order: Vec<u32> = w.entities().iter().map(|e| e.index()).collect();
        assert_eq!(order, [0, 1, 3, 4, 5], "entities are listed by slot index");
        assert!(w.entities().iter().all(|e| w.is_alive(*e)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_world_passes_the_conformance_suite() {
        conformance::run(&SimpleWorld::new);
    }

    #[test]
    fn the_entity_limit_is_enforced_and_freed_by_despawning() {
        let mut w = SimpleWorld::with_limit(conformance::registry(), 3);
        let thing = w.registry().class_id("Thing").unwrap();
        let ids: Vec<EntityId> = (0..3).map(|_| w.spawn(thing, None).unwrap()).collect();
        assert_eq!(w.spawn(thing, None), Err(WorldError::Full));
        assert_eq!(
            (w.len(), w.drain_lifecycle().len()),
            (3, 3),
            "a refused spawn records nothing"
        );
        w.despawn(ids[1]).unwrap();
        assert!(w.spawn(thing, None).is_ok());
    }

    #[test]
    fn a_generation_that_wraps_around_still_distinguishes_neighbours() {
        let mut w = SimpleWorld::new(conformance::registry());
        let thing = w.registry().class_id("Thing").unwrap();
        let mut last = w.spawn(thing, None).unwrap();
        for _ in 0..1000 {
            w.despawn(last).unwrap();
            let next = w.spawn(thing, None).unwrap();
            assert_ne!(next, last);
            assert!(!w.is_alive(last));
            last = next;
        }
        assert_eq!(last.generation(), 1000);
    }

    #[test]
    fn two_worlds_given_the_same_calls_give_the_same_handles() {
        let run = || {
            let mut w = SimpleWorld::new(conformance::registry());
            let mob = w.registry().class_id("Mob").unwrap();
            let mut out = Vec::new();
            let mut live = Vec::new();
            for i in 0..200u32 {
                if i % 3 == 2 && !live.is_empty() {
                    let victim = live.remove((i as usize * 7) % live.len());
                    w.despawn(victim).unwrap();
                }
                let e = w.spawn(mob, None).unwrap();
                live.push(e);
                out.push(e.to_u64());
            }
            out
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn the_census_counts_by_class_name() {
        let mut w = SimpleWorld::new(conformance::registry());
        let (thing, mob) = (
            w.registry().class_id("Thing").unwrap(),
            w.registry().class_id("Mob").unwrap(),
        );
        for _ in 0..2 {
            w.spawn(thing, None).unwrap();
        }
        w.spawn(mob, None).unwrap();
        let c = census(&w);
        assert_eq!(
            (c.get("Thing"), c.get("Mob"), c.len()),
            (Some(&2), Some(&1), 2)
        );
    }

    #[test]
    fn a_property_may_hold_a_reference_and_a_dead_target_does_not_make_the_holder_invalid() {
        let mut w = SimpleWorld::new(conformance::registry());
        let mob = w.registry().class_id("Mob").unwrap();
        let (a, b) = (w.spawn(mob, None).unwrap(), w.spawn(mob, None).unwrap());
        w.set(a, "owner", PropertyValue::Entity(b)).unwrap();
        w.despawn(b).unwrap();
        // The reference is just a handle; resolving it is the caller's job (is_alive says it is dead).
        match w.get(a, "owner").unwrap() {
            PropertyValue::Entity(e) => assert!(!w.is_alive(e)),
            other => panic!("{other:?}"),
        }
    }
}
