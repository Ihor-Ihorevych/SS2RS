//! Classes and properties: what an entity is made of, described as data (roadmap T00-017, RFC 0009).
//!
//! The simulation, the replication framework, the save system and the console all need to talk about
//! "the `health` of this entity" without knowing the Rust type that holds it. A [`ClassDesc`] lists
//! a class's **properties**: a name, a [`PropertyType`], a default, and **replication attributes**
//! ([`ReplicationAttr`]: whether, how precisely and how urgently it goes to clients). The
//! [`ClassRegistry`] holds every class, resolves inheritance (a class has all the properties of its
//! parents, parents first) and computes a **schema hash** of the parts that decide the wire layout, so
//! two builds that disagree about a class can be told apart at the handshake (the number goes in
//! the replay header and next to `ss2_net::protocol::Schema`).
//!
//! The registry is built once at start-up, in the same order on every machine; ids are assigned in
//! registration order. Everything here is deterministic: no hash maps, no clocks.

use std::collections::BTreeMap;
use std::fmt;

use crate::entity::{ClassId, EntityId};

/// The longest class or property name, in bytes.
pub const MAX_NAME_LEN: usize = 64;
/// The most properties one class may have, counting those it inherits.
pub const MAX_PROPERTIES: usize = 1024;

/// The kinds of value a property holds. More are added as the game needs them, so match on it with
/// a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum PropertyType {
    /// `true` or `false`.
    Bool,
    /// A signed 32-bit integer.
    I32,
    /// An unsigned 32-bit integer.
    U32,
    /// A 32-bit float.
    F32,
    /// Three floats: a position, a direction, a scale.
    Vec3,
    /// Text (a name, a target, a sound path).
    Text,
    /// A reference to another entity.
    Entity,
}

impl PropertyType {
    /// A stable number for the type, used in the schema hash.
    fn tag(self) -> u8 {
        match self {
            Self::Bool => 1,
            Self::I32 => 2,
            Self::U32 => 3,
            Self::F32 => 4,
            Self::Vec3 => 5,
            Self::Text => 6,
            Self::Entity => 7,
        }
    }

    /// Whether [`Quantize`] makes sense for the type.
    pub fn is_float_like(self) -> bool {
        matches!(self, Self::F32 | Self::Vec3)
    }
}

/// A property's value.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PropertyValue {
    /// See [`PropertyType::Bool`].
    Bool(bool),
    /// See [`PropertyType::I32`].
    I32(i32),
    /// See [`PropertyType::U32`].
    U32(u32),
    /// See [`PropertyType::F32`].
    F32(f32),
    /// See [`PropertyType::Vec3`].
    Vec3([f32; 3]),
    /// See [`PropertyType::Text`].
    Text(String),
    /// See [`PropertyType::Entity`].
    Entity(EntityId),
}

impl PropertyValue {
    /// The type of the value.
    pub fn ty(&self) -> PropertyType {
        match self {
            Self::Bool(_) => PropertyType::Bool,
            Self::I32(_) => PropertyType::I32,
            Self::U32(_) => PropertyType::U32,
            Self::F32(_) => PropertyType::F32,
            Self::Vec3(_) => PropertyType::Vec3,
            Self::Text(_) => PropertyType::Text,
            Self::Entity(_) => PropertyType::Entity,
        }
    }

    /// The default of a type: false, 0, 0.0, the zero vector, empty text, the null entity.
    pub fn default_for(ty: PropertyType) -> Self {
        match ty {
            PropertyType::Bool => Self::Bool(false),
            PropertyType::I32 => Self::I32(0),
            PropertyType::U32 => Self::U32(0),
            PropertyType::F32 => Self::F32(0.0),
            PropertyType::Vec3 => Self::Vec3([0.0; 3]),
            PropertyType::Text => Self::Text(String::new()),
            PropertyType::Entity => Self::Entity(EntityId::NULL),
        }
    }

    /// Whether two values are the same **bit for bit** (so `0.0` and `-0.0` differ and a NaN equals
    /// itself): what "changed" means for replication.
    pub fn same_bits(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::F32(a), Self::F32(b)) => a.to_bits() == b.to_bits(),
            (Self::Vec3(a), Self::Vec3(b)) => {
                a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
            }
            _ => self == other,
        }
    }
}

/// Whether and to whom a property is sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplicationMode {
    /// Never: simulation-only state.
    #[default]
    None,
    /// To everyone, whenever it changes.
    Always,
    /// To everyone, once, when the entity is created (a class, a model name).
    SpawnOnly,
    /// Only to the player that owns the entity (ammunition, the private inventory).
    Owner,
}

impl ReplicationMode {
    fn tag(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Always => 1,
            Self::SpawnOnly => 2,
            Self::Owner => 3,
        }
    }
}

/// How precisely a float or vector goes on the wire.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Quantize {
    /// Full precision.
    #[default]
    None,
    /// `bits` bits per component over `min..=max`.
    Fixed {
        /// The smallest value.
        min: f32,
        /// The largest value.
        max: f32,
        /// Bits per component, 1 to 32.
        bits: u8,
    },
}

/// The replication attributes of a property.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReplicationAttr {
    /// Whether and to whom.
    pub mode: ReplicationMode,
    /// How precisely.
    pub quantize: Quantize,
    /// How urgently it is sent when bandwidth is short: higher goes first. It does **not** change
    /// the wire layout, so it is not in the schema hash.
    pub priority: u8,
}

/// One property of a class.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyDesc {
    /// The name, unique within the class including inherited properties.
    pub name: String,
    /// The type.
    pub ty: PropertyType,
    /// The value of a new entity.
    pub default: PropertyValue,
    /// How it is replicated.
    pub replication: ReplicationAttr,
    /// Whether it is written to a save game.
    pub persistent: bool,
}

impl PropertyDesc {
    /// A property with the default value of its type, not replicated, not saved.
    pub fn new(name: &str, ty: PropertyType) -> Self {
        Self {
            name: name.to_string(),
            ty,
            default: PropertyValue::default_for(ty),
            replication: ReplicationAttr::default(),
            persistent: false,
        }
    }

    /// Set the default value.
    #[must_use]
    pub fn default_value(mut self, value: PropertyValue) -> Self {
        self.default = value;
        self
    }

    /// Replicate it in `mode`.
    #[must_use]
    pub fn replicated(mut self, mode: ReplicationMode) -> Self {
        self.replication.mode = mode;
        self
    }

    /// Quantize it to `bits` bits per component over `min..=max`.
    #[must_use]
    pub fn quantized(mut self, min: f32, max: f32, bits: u8) -> Self {
        self.replication.quantize = Quantize::Fixed { min, max, bits };
        self
    }

    /// Set the send priority.
    #[must_use]
    pub fn priority(mut self, priority: u8) -> Self {
        self.replication.priority = priority;
        self
    }

    /// Save it in a save game.
    #[must_use]
    pub fn persistent(mut self) -> Self {
        self.persistent = true;
        self
    }
}

/// A class to register.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassDesc {
    /// The name, unique in the registry.
    pub name: String,
    /// The class this one extends (it gets all of that class's properties first).
    pub parent: Option<ClassId>,
    /// The properties this class adds.
    pub properties: Vec<PropertyDesc>,
}

impl ClassDesc {
    /// A class with no parent and no properties yet.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: None,
            properties: Vec::new(),
        }
    }

    /// Extend `parent`.
    #[must_use]
    pub fn extends(mut self, parent: ClassId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Add a property.
    #[must_use]
    pub fn property(mut self, property: PropertyDesc) -> Self {
        self.properties.push(property);
        self
    }
}

/// Why a class could not be registered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// A class or property name is empty, longer than [`MAX_NAME_LEN`] or uses a character other than
    /// letters, digits, `_`, `.` and `-`.
    BadName(String),
    /// A class with this name exists.
    DuplicateClass(String),
    /// The parent id is not in the registry.
    UnknownParent(ClassId),
    /// A property name is used twice in the class or is already inherited.
    DuplicateProperty {
        /// The class being registered.
        class: String,
        /// The property.
        property: String,
    },
    /// A default value does not have the property's type.
    BadDefault {
        /// The property.
        property: String,
    },
    /// Quantization on a property that is not a float or vector, or with `min >= max`, a bound that
    /// is not finite or `bits` outside 1 to 32.
    BadQuantize {
        /// The property.
        property: String,
    },
    /// The class would have more than [`MAX_PROPERTIES`] properties.
    TooManyProperties(String),
    /// There are already 65,535 classes.
    TooManyClasses,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadName(n) => write!(f, "bad name {n:?}"),
            Self::DuplicateClass(n) => write!(f, "class {n:?} is already registered"),
            Self::UnknownParent(p) => write!(f, "unknown parent class {}", p.0),
            Self::DuplicateProperty { class, property } => {
                write!(
                    f,
                    "class {class:?} has the property {property:?} twice (or inherits it)"
                )
            }
            Self::BadDefault { property } => {
                write!(f, "the default of {property:?} has the wrong type")
            }
            Self::BadQuantize { property } => write!(f, "bad quantization on {property:?}"),
            Self::TooManyProperties(c) => {
                write!(f, "class {c:?} has more than {MAX_PROPERTIES} properties")
            }
            Self::TooManyClasses => write!(f, "too many classes"),
        }
    }
}

impl std::error::Error for RegistryError {}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

struct ClassEntry {
    name: String,
    parent: Option<ClassId>,
    /// Inherited properties first, then the class's own, in declaration order.
    properties: Vec<PropertyDesc>,
}

/// Every class of the simulation.
///
/// ```
/// use ss2_sim::reflect::{ClassDesc, ClassRegistry, PropertyDesc, PropertyType, ReplicationMode};
///
/// let mut registry = ClassRegistry::new();
/// let base = registry.register(
///     ClassDesc::new("Entity").property(PropertyDesc::new("origin", PropertyType::Vec3).replicated(ReplicationMode::Always).quantized(-4096.0, 4096.0, 20)),
/// ).unwrap();
/// let enemy = registry.register(
///     ClassDesc::new("Enemy").extends(base).property(PropertyDesc::new("health", PropertyType::F32).replicated(ReplicationMode::Always)),
/// ).unwrap();
/// // A class has its parents' properties first.
/// let names: Vec<&str> = registry.properties(enemy).unwrap().iter().map(|p| p.name.as_str()).collect();
/// assert_eq!(names, ["origin", "health"]);
/// assert!(registry.is_a(enemy, base));
/// ```
#[derive(Default)]
pub struct ClassRegistry {
    classes: Vec<ClassEntry>,
    by_name: BTreeMap<String, ClassId>,
}

impl ClassRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a class and return its id (the next free number).
    pub fn register(&mut self, desc: ClassDesc) -> Result<ClassId, RegistryError> {
        if !valid_name(&desc.name) {
            return Err(RegistryError::BadName(desc.name));
        }
        if self.by_name.contains_key(&desc.name) {
            return Err(RegistryError::DuplicateClass(desc.name));
        }
        if self.classes.len() >= usize::from(u16::MAX) {
            return Err(RegistryError::TooManyClasses);
        }
        let mut properties = match desc.parent {
            Some(parent) => self
                .classes
                .get(usize::from(parent.0))
                .ok_or(RegistryError::UnknownParent(parent))?
                .properties
                .clone(),
            None => Vec::new(),
        };
        for property in desc.properties {
            if !valid_name(&property.name) {
                return Err(RegistryError::BadName(property.name));
            }
            if properties.iter().any(|p| p.name == property.name) {
                return Err(RegistryError::DuplicateProperty {
                    class: desc.name,
                    property: property.name,
                });
            }
            if property.default.ty() != property.ty {
                return Err(RegistryError::BadDefault {
                    property: property.name,
                });
            }
            if let Quantize::Fixed { min, max, bits } = property.replication.quantize {
                let ok = property.ty.is_float_like()
                    && min.is_finite()
                    && max.is_finite()
                    && min < max
                    && (1..=32).contains(&bits);
                if !ok {
                    return Err(RegistryError::BadQuantize {
                        property: property.name,
                    });
                }
            }
            properties.push(property);
            if properties.len() > MAX_PROPERTIES {
                return Err(RegistryError::TooManyProperties(desc.name));
            }
        }
        let id = ClassId(self.classes.len() as u16);
        self.by_name.insert(desc.name.clone(), id);
        self.classes.push(ClassEntry {
            name: desc.name,
            parent: desc.parent,
            properties,
        });
        Ok(id)
    }

    /// How many classes there are.
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// The id of the class called `name`.
    pub fn class_id(&self, name: &str) -> Option<ClassId> {
        self.by_name.get(name).copied()
    }

    /// The name of a class.
    pub fn class_name(&self, id: ClassId) -> Option<&str> {
        self.classes.get(usize::from(id.0)).map(|c| c.name.as_str())
    }

    /// The class a class extends.
    pub fn parent(&self, id: ClassId) -> Option<ClassId> {
        self.classes.get(usize::from(id.0)).and_then(|c| c.parent)
    }

    /// Whether `class` is `ancestor` or extends it, however far up.
    pub fn is_a(&self, class: ClassId, ancestor: ClassId) -> bool {
        let mut current = Some(class);
        while let Some(c) = current {
            if c == ancestor {
                return true;
            }
            current = self.parent(c);
        }
        false
    }

    /// All the properties of a class, inherited ones first: the index in this slice is the property's
    /// **slot** for this class.
    pub fn properties(&self, id: ClassId) -> Option<&[PropertyDesc]> {
        self.classes
            .get(usize::from(id.0))
            .map(|c| c.properties.as_slice())
    }

    /// The slot of a property of a class.
    pub fn property_slot(&self, id: ClassId, name: &str) -> Option<usize> {
        self.properties(id)?.iter().position(|p| p.name == name)
    }

    /// A hash of everything that decides the wire layout: the classes in id order with their names,
    /// parents, and for each property its name, type, replication mode and quantization. Defaults, the
    /// send priority and the save flag are **not** in it. Two builds with different hashes must not talk.
    pub fn schema_hash(&self) -> u64 {
        let mut bytes: Vec<u8> = Vec::new();
        let text = |bytes: &mut Vec<u8>, s: &str| {
            bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
        };
        for class in &self.classes {
            text(&mut bytes, &class.name);
            bytes.extend_from_slice(
                &class
                    .parent
                    .map_or(u32::MAX, |p| u32::from(p.0))
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(&(class.properties.len() as u32).to_le_bytes());
            for p in &class.properties {
                text(&mut bytes, &p.name);
                bytes.push(p.ty.tag());
                bytes.push(p.replication.mode.tag());
                match p.replication.quantize {
                    Quantize::None => bytes.push(0),
                    Quantize::Fixed { min, max, bits } => {
                        bytes.push(1);
                        bytes.extend_from_slice(&min.to_bits().to_le_bytes());
                        bytes.extend_from_slice(&max.to_bits().to_le_bytes());
                        bytes.push(bits);
                    }
                }
            }
        }
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        bytes
            .iter()
            .fold(OFFSET, |h, b| (h ^ u64::from(*b)).wrapping_mul(PRIME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> ClassDesc {
        ClassDesc::new("Entity")
            .property(
                PropertyDesc::new("origin", PropertyType::Vec3)
                    .replicated(ReplicationMode::Always)
                    .quantized(-4096.0, 4096.0, 20)
                    .priority(5),
            )
            .property(PropertyDesc::new("name", PropertyType::Text))
    }

    fn enemy(parent: ClassId) -> ClassDesc {
        ClassDesc::new("Enemy")
            .extends(parent)
            .property(
                PropertyDesc::new("health", PropertyType::F32)
                    .default_value(PropertyValue::F32(100.0))
                    .replicated(ReplicationMode::Always)
                    .persistent(),
            )
            .property(PropertyDesc::new("target", PropertyType::Entity))
    }

    fn registry() -> (ClassRegistry, ClassId, ClassId) {
        let mut r = ClassRegistry::new();
        let b = r.register(base()).unwrap();
        let e = r.register(enemy(b)).unwrap();
        (r, b, e)
    }

    #[test]
    fn ids_are_assigned_in_registration_order_and_names_resolve_both_ways() {
        let (r, b, e) = registry();
        assert_eq!((b, e), (ClassId(0), ClassId(1)));
        assert_eq!(
            (
                r.class_id("Entity"),
                r.class_id("Enemy"),
                r.class_id("Nope")
            ),
            (Some(b), Some(e), None)
        );
        assert_eq!(
            (r.class_name(e), r.class_name(ClassId(9))),
            (Some("Enemy"), None)
        );
        assert_eq!((r.len(), r.is_empty()), (2, false));
        assert!(ClassRegistry::new().is_empty());
    }

    #[test]
    fn a_class_has_its_parents_properties_first_and_slots_follow_that_order() {
        let (r, b, e) = registry();
        let names = |c| {
            r.properties(c)
                .unwrap()
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(b), ["origin", "name"]);
        assert_eq!(names(e), ["origin", "name", "health", "target"]);
        assert_eq!(
            (
                r.property_slot(e, "origin"),
                r.property_slot(e, "health"),
                r.property_slot(e, "nope")
            ),
            (Some(0), Some(2), None)
        );
        assert_eq!(
            r.property_slot(b, "health"),
            None,
            "the parent does not have the child's property"
        );
        assert!(r.properties(ClassId(9)).is_none());
    }

    #[test]
    fn inheritance_is_followed_all_the_way_up() {
        let (mut r, b, e) = registry();
        let boss = r
            .register(
                ClassDesc::new("Boss")
                    .extends(e)
                    .property(PropertyDesc::new("phase", PropertyType::U32)),
            )
            .unwrap();
        assert!(r.is_a(boss, boss) && r.is_a(boss, e) && r.is_a(boss, b));
        assert!(!r.is_a(b, e) && !r.is_a(e, boss));
        assert_eq!((r.parent(boss), r.parent(b)), (Some(e), None));
        assert_eq!(r.properties(boss).unwrap().len(), 5);
    }

    #[test]
    fn defaults_have_the_type_and_a_wrong_one_is_refused() {
        let (r, _, e) = registry();
        assert_eq!(
            r.properties(e).unwrap()[2].default,
            PropertyValue::F32(100.0)
        );
        assert_eq!(
            r.properties(e).unwrap()[3].default,
            PropertyValue::Entity(EntityId::NULL)
        );
        for ty in [
            PropertyType::Bool,
            PropertyType::I32,
            PropertyType::U32,
            PropertyType::F32,
            PropertyType::Vec3,
            PropertyType::Text,
            PropertyType::Entity,
        ] {
            assert_eq!(PropertyValue::default_for(ty).ty(), ty);
        }
        let mut r2 = ClassRegistry::new();
        let bad = ClassDesc::new("X").property(
            PropertyDesc::new("a", PropertyType::F32).default_value(PropertyValue::Bool(true)),
        );
        assert_eq!(
            r2.register(bad),
            Err(RegistryError::BadDefault {
                property: "a".into()
            })
        );
        assert!(r2.is_empty(), "a refused class leaves nothing behind");
    }

    #[test]
    fn bad_registrations_are_refused_with_the_reason() {
        let mut r = ClassRegistry::new();
        let b = r.register(base()).unwrap();
        assert_eq!(
            r.register(base()),
            Err(RegistryError::DuplicateClass("Entity".into()))
        );
        assert_eq!(
            r.register(ClassDesc::new("A").extends(ClassId(7))),
            Err(RegistryError::UnknownParent(ClassId(7)))
        );
        for bad in ["", "a b", "a/b", "é", &"x".repeat(MAX_NAME_LEN + 1)] {
            assert_eq!(
                r.register(ClassDesc::new(bad)),
                Err(RegistryError::BadName(bad.to_string())),
                "{bad:?}"
            );
        }
        assert!(
            r.register(ClassDesc::new(&"x".repeat(MAX_NAME_LEN)))
                .is_ok()
        );
        assert!(r.register(ClassDesc::new("Fine.name-1_x")).is_ok());
        let dup = ClassDesc::new("D")
            .property(PropertyDesc::new("a", PropertyType::Bool))
            .property(PropertyDesc::new("a", PropertyType::I32));
        assert_eq!(
            r.register(dup),
            Err(RegistryError::DuplicateProperty {
                class: "D".into(),
                property: "a".into()
            })
        );
        let shadow = ClassDesc::new("S")
            .extends(b)
            .property(PropertyDesc::new("origin", PropertyType::Vec3));
        assert_eq!(
            r.register(shadow),
            Err(RegistryError::DuplicateProperty {
                class: "S".into(),
                property: "origin".into()
            })
        );
        assert!(matches!(
            r.register(
                ClassDesc::new("N").property(PropertyDesc::new("bad name", PropertyType::Bool))
            ),
            Err(RegistryError::BadName(_))
        ));
        let ids = r.len();
        assert_eq!(ids, 3, "only the three good ones were added");
    }

    #[test]
    fn quantization_is_checked() {
        let try_q = |ty, min, max, bits| {
            let mut r = ClassRegistry::new();
            r.register(
                ClassDesc::new("Q").property(PropertyDesc::new("p", ty).quantized(min, max, bits)),
            )
        };
        assert!(try_q(PropertyType::F32, 0.0, 1.0, 8).is_ok());
        assert!(try_q(PropertyType::Vec3, -10.0, 10.0, 32).is_ok());
        assert!(try_q(PropertyType::F32, 0.0, 1.0, 1).is_ok());
        for (ty, min, max, bits) in [
            (PropertyType::U32, 0.0, 1.0, 8),
            (PropertyType::F32, 1.0, 1.0, 8),
            (PropertyType::F32, 2.0, 1.0, 8),
            (PropertyType::F32, f32::NEG_INFINITY, 1.0, 8),
            (PropertyType::F32, 0.0, f32::NAN, 8),
            (PropertyType::F32, 0.0, 1.0, 0),
            (PropertyType::F32, 0.0, 1.0, 33),
        ] {
            assert_eq!(
                try_q(ty, min, max, bits),
                Err(RegistryError::BadQuantize {
                    property: "p".into()
                }),
                "{ty:?} {min} {max} {bits}"
            );
        }
    }

    #[test]
    fn a_class_cannot_have_more_than_the_limit_of_properties() {
        let mut r = ClassRegistry::new();
        let mut d = ClassDesc::new("Big");
        for i in 0..MAX_PROPERTIES {
            d = d.property(PropertyDesc::new(&format!("p{i}"), PropertyType::Bool));
        }
        let big = r.register(d.clone()).unwrap();
        let more = ClassDesc::new("Bigger")
            .extends(big)
            .property(PropertyDesc::new("one_more", PropertyType::Bool));
        assert_eq!(
            r.register(more),
            Err(RegistryError::TooManyProperties("Bigger".into()))
        );
        let over = d.property(PropertyDesc::new("extra", PropertyType::Bool));
        assert_eq!(
            r.register(ClassDesc {
                name: "Over".into(),
                ..over
            }),
            Err(RegistryError::TooManyProperties("Over".into()))
        );
    }

    #[test]
    fn the_schema_hash_is_stable_and_sensitive_to_the_wire_layout_only() {
        let (r, ..) = registry();
        let (again, ..) = registry();
        assert_eq!(r.schema_hash(), again.schema_hash());
        assert_ne!(r.schema_hash(), ClassRegistry::new().schema_hash());
        let hash_with = |change: &dyn Fn(&mut ClassDesc)| {
            let mut r = ClassRegistry::new();
            let mut b = base();
            change(&mut b);
            let id = r.register(b).unwrap();
            r.register(enemy(id)).unwrap();
            r.schema_hash()
        };
        let original = hash_with(&|_| {});
        // Things that change the layout.
        assert_ne!(original, hash_with(&|c| c.name = "Entity2".into()));
        assert_ne!(
            original,
            hash_with(&|c| c.properties[0].name = "position".into())
        );
        assert_ne!(
            original,
            hash_with(&|c| {
                c.properties[0].ty = PropertyType::F32;
                c.properties[0].default = PropertyValue::F32(0.0);
            })
        );
        assert_ne!(
            original,
            hash_with(&|c| c.properties[0].replication.mode = ReplicationMode::Owner)
        );
        assert_ne!(
            original,
            hash_with(
                &|c| c.properties[0].replication.quantize = Quantize::Fixed {
                    min: -4096.0,
                    max: 4096.0,
                    bits: 16
                }
            )
        );
        assert_ne!(
            original,
            hash_with(
                &|c| c.properties[0].replication.quantize = Quantize::Fixed {
                    min: -2048.0,
                    max: 4096.0,
                    bits: 20
                }
            )
        );
        assert_ne!(
            original,
            hash_with(
                &|c| c.properties[0].replication.quantize = Quantize::Fixed {
                    min: -4096.0,
                    max: 2048.0,
                    bits: 20
                }
            )
        );
        assert_ne!(
            original,
            hash_with(&|c| c.properties.swap(0, 1)),
            "the order of properties is part of the layout"
        );
        assert_ne!(
            original,
            hash_with(&|c| c
                .properties
                .push(PropertyDesc::new("extra", PropertyType::Bool)))
        );
        // Things that do not.
        assert_eq!(
            original,
            hash_with(&|c| c.properties[0].replication.priority = 200)
        );
        assert_eq!(original, hash_with(&|c| c.properties[1].persistent = true));
        assert_eq!(
            original,
            hash_with(&|c| c.properties[1].default = PropertyValue::Text("x".into()))
        );
    }

    #[test]
    fn the_hash_tells_apart_names_that_would_run_together() {
        let one = |a: &str, b: &str| {
            let mut r = ClassRegistry::new();
            r.register(ClassDesc::new(a)).unwrap();
            r.register(ClassDesc::new(b)).unwrap();
            r.schema_hash()
        };
        assert_ne!(one("ab", "c"), one("a", "bc"));
    }

    #[test]
    fn same_bits_distinguishes_what_replication_must_distinguish() {
        assert!(PropertyValue::F32(1.5).same_bits(&PropertyValue::F32(1.5)));
        assert!(!PropertyValue::F32(0.0).same_bits(&PropertyValue::F32(-0.0)));
        assert!(PropertyValue::F32(f32::NAN).same_bits(&PropertyValue::F32(f32::NAN)));
        assert!(
            !PropertyValue::Vec3([0.0, 1.0, 2.0]).same_bits(&PropertyValue::Vec3([0.0, 1.0, 2.5]))
        );
        assert!(
            PropertyValue::Vec3([0.0, 1.0, 2.0]).same_bits(&PropertyValue::Vec3([0.0, 1.0, 2.0]))
        );
        assert!(!PropertyValue::I32(1).same_bits(&PropertyValue::U32(1)));
        assert!(PropertyValue::Text("a".into()).same_bits(&PropertyValue::Text("a".into())));
    }
}
