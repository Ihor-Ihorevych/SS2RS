//! Saving and loading the simulation (roadmap T08-022): the **entities of a [`World`] written as bytes** and
//! read back, in a format of our own.
//!
//! What goes in a save: the **persistent properties** (`PropertyDesc::persistent`) of every entity, the
//! **parent links**, and a small header (the tick, the random seed, the number of players, a few notes). What
//! does not: properties that are not marked persistent (they come back as their defaults), the event queue,
//! the changes and lifecycle records of the world, and **anything the entities' behaviour keeps outside their
//! properties** (a system's private state must be put in properties to survive a save: the contract says so).
//!
//! The file is checked before anything is built from it: magic, format version, a **checksum of every
//! byte**, the **schema hash** of the class registry (a save from a build with other classes or other property
//! layouts is refused rather than misread), every class id and every value, parent links that must point at
//! saved entities and must not form a cycle, and limits on sizes. [`parse`] returns a [`SaveFile`] or an
//! error and builds nothing; [`restore`] then fills an **empty** world. Entities get new handles when
//! loaded (the saved ones are only used to rebuild references); an entity-valued property that pointed at an
//! entity that was not saved comes back as the null entity and is counted ([`Restored::dangling_refs`]).
//!
//! Layout (all integers little endian): `"SS2S"`, format version `u16`, schema hash `u64`, tick `u64`,
//! seed `u64`, players `u8`, notes (`u8` count, then pairs of `u32` length and UTF-8), entity count `u32`,
//! each entity (index `u32`, generation `u32`, class `u16`, parent `u64` or all ones, then its persistent
//! property values in slot order), and the CRC-32 of everything before it.
//!
//! Deterministic: entities are written by slot index, so two worlds built by the same calls give the same bytes.

use std::collections::BTreeMap;
use std::fmt;

use crate::entity::{ClassId, EntityId};
use crate::reflect::{ClassRegistry, PropertyType, PropertyValue};
use crate::world::{World, WorldError};

/// The version of the file format this code writes and reads.
pub const FORMAT_VERSION: u16 = 1;
const MAGIC: [u8; 4] = *b"SS2S";
/// The most notes a header may carry.
pub const MAX_NOTES: usize = 16;
/// The longest note key or value, in bytes.
pub const MAX_NOTE_LEN: usize = 256;

/// What a save says about the game it was made in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveHeader {
    /// The simulation tick the save was made at.
    pub tick: u64,
    /// The random seed of the session.
    pub seed: u64,
    /// How many players were in the session (a co-op save has several).
    pub players: u8,
    /// A few notes: the level, the difficulty, a title for the save list.
    pub notes: Vec<(String, String)>,
}

/// The limits [`parse`] applies, so that a damaged or hostile file cannot make it allocate without bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most entities a file may hold.
    pub max_entities: usize,
    /// The longest text value, in bytes.
    pub max_text_bytes: usize,
    /// The largest file, in bytes.
    pub max_file_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entities: 1 << 20,
            max_text_bytes: 64 * 1024,
            max_file_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Why a save could not be written, read or restored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    /// The file does not start with the save magic.
    BadMagic,
    /// A format version this code does not read.
    UnsupportedVersion {
        /// The version in the file.
        found: u16,
    },
    /// The file is too short to be a save.
    Truncated,
    /// The checksum does not match: the file is damaged.
    BadChecksum,
    /// The save was made with different classes or property layouts.
    SchemaMismatch {
        /// The hash in the file.
        saved: u64,
        /// The hash of the registry it is being loaded into.
        current: u64,
    },
    /// A limit was exceeded (what).
    TooLarge(&'static str),
    /// The file is malformed (what).
    Invalid(&'static str),
    /// The world already holds entities.
    WorldNotEmpty,
    /// The world refused something while restoring (the world may be partly filled).
    World(WorldError),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a save file"),
            Self::UnsupportedVersion { found } => write!(
                f,
                "save format version {found} (this build reads {FORMAT_VERSION})"
            ),
            Self::Truncated => write!(f, "the save file is too short"),
            Self::BadChecksum => write!(f, "the save file is damaged (checksum mismatch)"),
            Self::SchemaMismatch { saved, current } => {
                write!(
                    f,
                    "the save was made with other classes ({saved:#018x}, this build {current:#018x})"
                )
            }
            Self::TooLarge(what) => write!(f, "the save exceeds a limit: {what}"),
            Self::Invalid(what) => write!(f, "invalid save: {what}"),
            Self::WorldNotEmpty => write!(f, "the world already holds entities"),
            Self::World(e) => write!(f, "the world refused the save: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

/// CRC-32 (IEEE, the one in zip and PNG).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// One entity as saved.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedEntity {
    /// The handle it had when saved.
    pub id: EntityId,
    /// Its class.
    pub class: ClassId,
    /// Its parent, as saved.
    pub parent: Option<EntityId>,
    /// Its persistent properties as (slot, value), in slot order.
    pub values: Vec<(u16, PropertyValue)>,
}

/// A save that has been read and checked, not yet built into a world.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveFile {
    /// The header.
    pub header: SaveHeader,
    /// The schema hash the save was made with.
    pub schema_hash: u64,
    /// The entities, by saved slot index.
    pub entities: Vec<SavedEntity>,
}

/// What [`restore`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restored {
    /// Saved handle to the new one.
    pub id_map: BTreeMap<EntityId, EntityId>,
    /// Entity-valued properties that pointed at an entity that was not saved, now null.
    pub dangling_refs: usize,
}

fn put_text(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

fn put_value(out: &mut Vec<u8>, value: &PropertyValue) {
    match value {
        PropertyValue::Bool(b) => out.push(u8::from(*b)),
        PropertyValue::I32(v) => out.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::U32(v) => out.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::F32(v) => out.extend_from_slice(&v.to_bits().to_le_bytes()),
        PropertyValue::Vec3(v) => {
            for c in v {
                out.extend_from_slice(&c.to_bits().to_le_bytes());
            }
        }
        PropertyValue::Text(t) => put_text(out, t),
        PropertyValue::Entity(e) => out.extend_from_slice(&e.to_u64().to_le_bytes()),
    }
}

/// Write the persistent state of `world` and the header as a save file.
pub fn save<W: World>(world: &W, header: &SaveHeader) -> Result<Vec<u8>, SaveError> {
    if header.notes.len() > MAX_NOTES {
        return Err(SaveError::TooLarge("more notes than the limit"));
    }
    if header
        .notes
        .iter()
        .any(|(k, v)| k.len() > MAX_NOTE_LEN || v.len() > MAX_NOTE_LEN)
    {
        return Err(SaveError::TooLarge("a note is longer than the limit"));
    }
    let registry = world.registry();
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&registry.schema_hash().to_le_bytes());
    out.extend_from_slice(&header.tick.to_le_bytes());
    out.extend_from_slice(&header.seed.to_le_bytes());
    out.push(header.players);
    out.push(header.notes.len() as u8);
    for (k, v) in &header.notes {
        put_text(&mut out, k);
        put_text(&mut out, v);
    }
    let entities = world.entities();
    out.extend_from_slice(&(entities.len() as u32).to_le_bytes());
    for id in entities {
        let class = world.class_of(id).map_err(SaveError::World)?;
        let parent = world.parent_of(id).map_err(SaveError::World)?;
        out.extend_from_slice(&id.index().to_le_bytes());
        out.extend_from_slice(&id.generation().to_le_bytes());
        out.extend_from_slice(&class.0.to_le_bytes());
        out.extend_from_slice(&parent.unwrap_or(EntityId::NULL).to_u64().to_le_bytes());
        let properties = registry
            .properties(class)
            .ok_or(SaveError::Invalid("an entity has an unknown class"))?;
        for (slot, desc) in properties.iter().enumerate() {
            if desc.persistent {
                let value = world
                    .get_slot(id, slot as u16)
                    .map_err(SaveError::World)?
                    .ok_or(SaveError::Invalid("a property slot is missing"))?;
                put_value(&mut out, &value);
            }
        }
    }
    let crc = crc32(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    Ok(out)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], SaveError> {
        let end = self.pos.checked_add(n).ok_or(SaveError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(SaveError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, SaveError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, SaveError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().unwrap_or([0; 2]),
        ))
    }

    fn u32(&mut self) -> Result<u32, SaveError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().unwrap_or([0; 4]),
        ))
    }

    fn u64(&mut self) -> Result<u64, SaveError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().unwrap_or([0; 8]),
        ))
    }

    fn text(&mut self, max: usize) -> Result<String, SaveError> {
        let len = self.u32()? as usize;
        if len > max {
            return Err(SaveError::TooLarge("a text is longer than the limit"));
        }
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| SaveError::Invalid("a text is not valid UTF-8"))
    }

    fn value(&mut self, ty: PropertyType, max_text: usize) -> Result<PropertyValue, SaveError> {
        Ok(match ty {
            PropertyType::Bool => match self.u8()? {
                0 => PropertyValue::Bool(false),
                1 => PropertyValue::Bool(true),
                _ => return Err(SaveError::Invalid("a boolean is neither 0 nor 1")),
            },
            PropertyType::I32 => PropertyValue::I32(self.u32()? as i32),
            PropertyType::U32 => PropertyValue::U32(self.u32()?),
            PropertyType::F32 => PropertyValue::F32(f32::from_bits(self.u32()?)),
            PropertyType::Vec3 => PropertyValue::Vec3([
                f32::from_bits(self.u32()?),
                f32::from_bits(self.u32()?),
                f32::from_bits(self.u32()?),
            ]),
            PropertyType::Text => PropertyValue::Text(self.text(max_text)?),
            PropertyType::Entity => PropertyValue::Entity(EntityId::from_u64(self.u64()?)),
        })
    }
}

/// Read and check a save against `registry`. Builds nothing.
pub fn parse(
    bytes: &[u8],
    registry: &ClassRegistry,
    limits: &Limits,
) -> Result<SaveFile, SaveError> {
    if bytes.len() > limits.max_file_bytes {
        return Err(SaveError::TooLarge("the file"));
    }
    if bytes.len() < 4 || bytes[..4] != MAGIC {
        return Err(if bytes.len() < 4 {
            SaveError::Truncated
        } else {
            SaveError::BadMagic
        });
    }
    let mut r = Reader { bytes, pos: 4 };
    let version = r.u16()?;
    if version != FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion { found: version });
    }
    if bytes.len() < 4 + 2 + 4 {
        return Err(SaveError::Truncated);
    }
    let (body, trailer) = bytes.split_at(bytes.len() - 4);
    if crc32(body) != u32::from_le_bytes(trailer.try_into().unwrap_or([0; 4])) {
        return Err(SaveError::BadChecksum);
    }
    let mut r = Reader {
        bytes: body,
        pos: 6,
    };
    let saved_hash = r.u64()?;
    let current = registry.schema_hash();
    if saved_hash != current {
        return Err(SaveError::SchemaMismatch {
            saved: saved_hash,
            current,
        });
    }
    let mut header = SaveHeader {
        tick: r.u64()?,
        seed: r.u64()?,
        players: r.u8()?,
        notes: Vec::new(),
    };
    let notes = usize::from(r.u8()?);
    if notes > MAX_NOTES {
        return Err(SaveError::TooLarge("more notes than the limit"));
    }
    for _ in 0..notes {
        header
            .notes
            .push((r.text(MAX_NOTE_LEN)?, r.text(MAX_NOTE_LEN)?));
    }
    let count = r.u32()? as usize;
    if count > limits.max_entities {
        return Err(SaveError::TooLarge("more entities than the limit"));
    }
    // An entity takes at least 18 bytes: a count the file cannot hold is refused before anything is reserved.
    if count > (body.len() - r.pos) / 18 {
        return Err(SaveError::Truncated);
    }
    let mut entities: Vec<SavedEntity> = Vec::with_capacity(count);
    for _ in 0..count {
        let index = r.u32()?;
        let generation = r.u32()?;
        let class = ClassId(r.u16()?);
        let parent_bits = r.u64()?;
        let properties = registry
            .properties(class)
            .ok_or(SaveError::Invalid("an entity has an unknown class"))?;
        let id = EntityId::new(index, generation);
        if id.is_null() {
            return Err(SaveError::Invalid("an entity handle is null"));
        }
        if entities.last().is_some_and(|e| e.id.index() >= index) {
            return Err(SaveError::Invalid("entities are not in index order"));
        }
        let parent = EntityId::from_u64(parent_bits);
        let mut values = Vec::new();
        for (slot, desc) in properties.iter().enumerate() {
            if desc.persistent {
                values.push((slot as u16, r.value(desc.ty, limits.max_text_bytes)?));
            }
        }
        entities.push(SavedEntity {
            id,
            class,
            parent: if parent.is_null() { None } else { Some(parent) },
            values,
        });
    }
    if r.pos != body.len() {
        return Err(SaveError::Invalid("bytes follow the last entity"));
    }
    // Parents must be saved entities (same index and generation) and must not form a cycle.
    let by_index: BTreeMap<u32, &SavedEntity> =
        entities.iter().map(|e| (e.id.index(), e)).collect();
    for e in &entities {
        if e.parent
            .is_some_and(|p| by_index.get(&p.index()).is_none_or(|saved| saved.id != p))
        {
            return Err(SaveError::Invalid("a parent is not a saved entity"));
        }
    }
    for e in &entities {
        let (mut current, mut steps) = (e, 0usize);
        while let Some(p) = current.parent {
            steps += 1;
            if steps > entities.len() {
                return Err(SaveError::Invalid("parent links form a cycle"));
            }
            current = by_index[&p.index()];
        }
    }
    Ok(SaveFile {
        header,
        schema_hash: saved_hash,
        entities,
    })
}

/// Build the entities of a checked save into `world`, which must be **empty** and use a registry with the save's
/// schema hash. Entities are spawned in saved order, then parented, then given their values; entity-valued
/// properties are mapped to the new handles. The world's change and lifecycle records hold the spawns afterwards:
/// drain them if they are not wanted.
pub fn restore<W: World>(file: &SaveFile, world: &mut W) -> Result<Restored, SaveError> {
    if !world.is_empty() {
        return Err(SaveError::WorldNotEmpty);
    }
    let current = world.registry().schema_hash();
    if file.schema_hash != current {
        return Err(SaveError::SchemaMismatch {
            saved: file.schema_hash,
            current,
        });
    }
    let mut id_map: BTreeMap<EntityId, EntityId> = BTreeMap::new();
    for e in &file.entities {
        let new = world.spawn(e.class, None).map_err(SaveError::World)?;
        id_map.insert(e.id, new);
    }
    for e in &file.entities {
        if let Some(parent) = e.parent {
            let (child, parent) = (
                id_map[&e.id],
                *id_map
                    .get(&parent)
                    .ok_or(SaveError::Invalid("a parent is not a saved entity"))?,
            );
            world
                .set_parent(child, Some(parent))
                .map_err(SaveError::World)?;
        }
    }
    let mut dangling = 0;
    for e in &file.entities {
        let new = id_map[&e.id];
        let class = world.class_of(new).map_err(SaveError::World)?;
        for (slot, value) in &e.values {
            let name = world
                .registry()
                .properties(class)
                .and_then(|p| p.get(usize::from(*slot)))
                .map(|d| d.name.clone())
                .ok_or(SaveError::Invalid("a property slot is out of range"))?;
            let value = match value {
                PropertyValue::Entity(old) if !old.is_null() => match id_map.get(old) {
                    Some(new) => PropertyValue::Entity(*new),
                    None => {
                        dangling += 1;
                        PropertyValue::Entity(EntityId::NULL)
                    }
                },
                other => other.clone(),
            };
            world.set(new, &name, value).map_err(SaveError::World)?;
        }
    }
    Ok(Restored {
        id_map,
        dangling_refs: dangling,
    })
}

/// [`parse`] and [`restore`] in one call, using the world's own registry.
pub fn load<W: World>(
    bytes: &[u8],
    world: &mut W,
    limits: &Limits,
) -> Result<(SaveHeader, Restored), SaveError> {
    let file = parse(bytes, world.registry(), limits)?;
    let restored = restore(&file, world)?;
    Ok((file.header, restored))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflect::{ClassDesc, PropertyDesc, ReplicationMode};
    use crate::world::SimpleWorld;

    fn registry() -> ClassRegistry {
        let mut r = ClassRegistry::new();
        let thing = r
            .register(
                ClassDesc::new("Thing")
                    .property(
                        PropertyDesc::new("origin", PropertyType::Vec3)
                            .replicated(ReplicationMode::Always)
                            .persistent(),
                    )
                    .property(PropertyDesc::new("label", PropertyType::Text))
                    .property(PropertyDesc::new("tag", PropertyType::Text).persistent()),
            )
            .unwrap();
        r.register(
            ClassDesc::new("Mob")
                .extends(thing)
                .property(
                    PropertyDesc::new("health", PropertyType::F32)
                        .default_value(PropertyValue::F32(100.0))
                        .persistent(),
                )
                .property(PropertyDesc::new("target", PropertyType::Entity).persistent())
                .property(PropertyDesc::new("kills", PropertyType::U32).persistent())
                .property(
                    PropertyDesc::new("alive", PropertyType::Bool)
                        .default_value(PropertyValue::Bool(true))
                        .persistent(),
                )
                .property(PropertyDesc::new("score", PropertyType::I32).persistent())
                .property(PropertyDesc::new("temp", PropertyType::F32)),
        )
        .unwrap();
        r
    }

    fn header() -> SaveHeader {
        SaveHeader {
            tick: 12_345,
            seed: 0xDEAD_BEEF,
            players: 2,
            notes: vec![
                ("map".into(), "Sirius".into()),
                ("difficulty".into(), "hard".into()),
            ],
        }
    }

    /// A world with gaps, a hierarchy, references, text and values that are not the defaults.
    fn world() -> SimpleWorld {
        let mut w = SimpleWorld::new(registry());
        let (thing, mob) = (
            w.registry().class_id("Thing").unwrap(),
            w.registry().class_id("Mob").unwrap(),
        );
        let a = w.spawn(mob, None).unwrap();
        let junk = w.spawn(thing, None).unwrap();
        let b = w.spawn(mob, Some(a)).unwrap();
        let c = w.spawn(thing, Some(b)).unwrap();
        let d = w.spawn(mob, None).unwrap();
        w.despawn(junk).unwrap();
        w.set(a, "origin", PropertyValue::Vec3([1.5, -2.0, 3.25]))
            .unwrap();
        w.set(a, "label", PropertyValue::Text("not saved".into()))
            .unwrap();
        w.set(a, "tag", PropertyValue::Text("Ünïcode ✓".into()))
            .unwrap();
        w.set(a, "health", PropertyValue::F32(37.5)).unwrap();
        w.set(a, "target", PropertyValue::Entity(d)).unwrap();
        w.set(a, "kills", PropertyValue::U32(4_000_000_000))
            .unwrap();
        w.set(a, "alive", PropertyValue::Bool(false)).unwrap();
        w.set(a, "score", PropertyValue::I32(-77)).unwrap();
        w.set(a, "temp", PropertyValue::F32(9.0)).unwrap();
        w.set(b, "target", PropertyValue::Entity(a)).unwrap();
        w.set(c, "tag", PropertyValue::Text(String::new())).unwrap();
        w.set(d, "health", PropertyValue::F32(f32::NAN)).unwrap();
        w.set(
            d,
            "origin",
            PropertyValue::Vec3([-0.0, f32::INFINITY, 1e-30]),
        )
        .unwrap();
        w
    }

    /// Append the checksum to a body, to make a file that is damaged in a way the checksum does not catch.
    fn seal(mut body: Vec<u8>) -> Vec<u8> {
        let crc = crc32(&body);
        body.extend_from_slice(&crc.to_le_bytes());
        body
    }

    fn unseal(bytes: &[u8]) -> Vec<u8> {
        bytes[..bytes.len() - 4].to_vec()
    }

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn a_world_survives_a_save_and_a_load_with_its_persistent_state() {
        let w = world();
        let bytes = save(&w, &header()).unwrap();
        let mut loaded = SimpleWorld::new(registry());
        let (h, restored) = load(&bytes, &mut loaded, &Limits::default()).unwrap();
        assert_eq!(h, header());
        assert_eq!(loaded.len(), w.len());
        assert_eq!(restored.dangling_refs, 0);
        assert_eq!(restored.id_map.len(), 4);
        for (old, new) in &restored.id_map {
            assert_eq!(w.class_of(*old).unwrap(), loaded.class_of(*new).unwrap());
            let class = w.class_of(*old).unwrap();
            for desc in w.registry().properties(class).unwrap() {
                let (before, after) = (
                    w.get(*old, &desc.name).unwrap(),
                    loaded.get(*new, &desc.name).unwrap(),
                );
                if !desc.persistent {
                    assert_eq!(
                        after, desc.default,
                        "{} comes back as its default",
                        desc.name
                    );
                    continue;
                }
                match before {
                    PropertyValue::Entity(target) if !target.is_null() => assert_eq!(
                        after,
                        PropertyValue::Entity(restored.id_map[&target]),
                        "{}",
                        desc.name
                    ),
                    ref other => assert!(
                        after.same_bits(other),
                        "{}: {before:?} vs {after:?}",
                        desc.name
                    ),
                }
            }
            let parent = w.parent_of(*old).unwrap().map(|p| restored.id_map[&p]);
            assert_eq!(loaded.parent_of(*new).unwrap(), parent);
        }
        // Spot checks that do not rely on the loop above.
        let a = restored.id_map[&w.entities()[0]];
        assert_eq!(loaded.get(a, "health").unwrap(), PropertyValue::F32(37.5));
        assert_eq!(
            loaded.get(a, "tag").unwrap(),
            PropertyValue::Text("Ünïcode ✓".into())
        );
        assert_eq!(
            loaded.get(a, "label").unwrap(),
            PropertyValue::Text(String::new()),
            "unsaved properties are defaults"
        );
        assert_eq!(
            loaded.get(a, "kills").unwrap(),
            PropertyValue::U32(4_000_000_000)
        );
        assert_eq!(loaded.get(a, "alive").unwrap(), PropertyValue::Bool(false));
    }

    #[test]
    fn saving_is_deterministic_and_loading_compacts_the_handles() {
        let (one, two) = (
            save(&world(), &header()).unwrap(),
            save(&world(), &header()).unwrap(),
        );
        assert_eq!(one, two, "the same calls give the same bytes");
        let mut b = SimpleWorld::new(registry());
        load(&one, &mut b, &Limits::default()).unwrap();
        let from_b = save(&b, &header()).unwrap();
        let mut c = SimpleWorld::new(registry());
        load(&from_b, &mut c, &Limits::default()).unwrap();
        assert_eq!(
            save(&c, &header()).unwrap(),
            from_b,
            "a loaded world saves and loads to the same bytes"
        );
        assert_eq!(
            b.entities().iter().map(|e| e.index()).collect::<Vec<_>>(),
            [0, 1, 2, 3],
            "slots are filled from 0 again"
        );
    }

    #[test]
    fn a_reference_to_an_entity_that_was_not_saved_comes_back_null_and_is_counted() {
        let mut w = SimpleWorld::new(registry());
        let mob = w.registry().class_id("Mob").unwrap();
        let (a, gone) = (w.spawn(mob, None).unwrap(), w.spawn(mob, None).unwrap());
        w.set(a, "target", PropertyValue::Entity(gone)).unwrap();
        w.despawn(gone).unwrap();
        let reused = w.spawn(mob, None).unwrap();
        assert_eq!(reused.index(), gone.index());
        let b = w.spawn(mob, None).unwrap();
        w.set(b, "target", PropertyValue::Entity(gone)).unwrap(); // the stale handle of the old entity in that slot
        let bytes = save(&w, &header()).unwrap();
        let mut loaded = SimpleWorld::new(registry());
        let (_, restored) = load(&bytes, &mut loaded, &Limits::default()).unwrap();
        assert_eq!(
            restored.dangling_refs, 2,
            "a stale handle does not become the entity that reuses its slot"
        );
        for new in restored.id_map.values() {
            assert_eq!(
                loaded.get(*new, "target").unwrap(),
                PropertyValue::Entity(EntityId::NULL)
            );
        }
    }

    #[test]
    fn a_null_reference_stays_null_and_is_not_dangling() {
        let mut w = SimpleWorld::new(registry());
        w.spawn(w.registry().class_id("Mob").unwrap(), None)
            .unwrap();
        let bytes = save(&w, &header()).unwrap();
        let (_, restored) = load(
            &bytes,
            &mut SimpleWorld::new(registry()),
            &Limits::default(),
        )
        .unwrap();
        assert_eq!(restored.dangling_refs, 0);
    }

    #[test]
    fn an_empty_world_saves_and_loads() {
        let w = SimpleWorld::new(registry());
        let bytes = save(&w, &SaveHeader::default()).unwrap();
        let mut loaded = SimpleWorld::new(registry());
        let (h, restored) = load(&bytes, &mut loaded, &Limits::default()).unwrap();
        assert_eq!(
            (h, loaded.len(), restored.id_map.len()),
            (SaveHeader::default(), 0, 0)
        );
    }

    #[test]
    fn a_big_world_round_trips() {
        let mut w = SimpleWorld::new(registry());
        let mob = w.registry().class_id("Mob").unwrap();
        let mut parent = None;
        for i in 0..5_000u32 {
            let e = w
                .spawn(mob, if i % 7 == 0 { None } else { parent })
                .unwrap();
            w.set(e, "kills", PropertyValue::U32(i)).unwrap();
            parent = Some(e);
        }
        let bytes = save(&w, &header()).unwrap();
        let mut loaded = SimpleWorld::new(registry());
        load(&bytes, &mut loaded, &Limits::default()).unwrap();
        assert_eq!(loaded.len(), 5_000);
        let last = *loaded.entities().last().unwrap();
        assert_eq!(
            loaded.get(last, "kills").unwrap(),
            PropertyValue::U32(4_999)
        );
    }

    // ---- refusing bad files ---------------------------------------------------------------

    #[test]
    fn a_file_that_is_not_a_save_or_is_cut_short_is_refused_without_a_panic() {
        let good = save(&world(), &header()).unwrap();
        let reg = registry();
        assert_eq!(
            parse(b"", &reg, &Limits::default()),
            Err(SaveError::Truncated)
        );
        assert_eq!(
            parse(b"nope, not a save at all", &reg, &Limits::default()),
            Err(SaveError::BadMagic)
        );
        for cut in 0..good.len() {
            assert!(
                parse(&good[..cut], &reg, &Limits::default()).is_err(),
                "a file cut to {cut} bytes"
            );
        }
        assert!(parse(&good, &reg, &Limits::default()).is_ok());
    }

    #[test]
    fn a_single_flipped_bit_anywhere_is_caught() {
        let good = save(&world(), &header()).unwrap();
        let reg = registry();
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0x10;
            assert!(
                parse(&bad, &reg, &Limits::default()).is_err(),
                "bit flipped in byte {i}"
            );
        }
    }

    #[test]
    fn another_format_version_is_named() {
        let mut bytes = save(&world(), &header()).unwrap();
        bytes[4] = 9;
        bytes[5] = 0;
        assert_eq!(
            parse(&bytes, &registry(), &Limits::default()),
            Err(SaveError::UnsupportedVersion { found: 9 })
        );
    }

    #[test]
    fn a_save_from_other_classes_is_refused_by_the_schema_hash() {
        let bytes = save(&world(), &header()).unwrap();
        let mut other = ClassRegistry::new();
        other.register(ClassDesc::new("Thing")).unwrap();
        match parse(&bytes, &other, &Limits::default()) {
            Err(SaveError::SchemaMismatch { saved, current }) => assert_eq!(
                (saved, current),
                (registry().schema_hash(), other.schema_hash())
            ),
            e => panic!("{e:?}"),
        }
        let parsed = parse(&bytes, &registry(), &Limits::default()).unwrap();
        let mut w = SimpleWorld::new(other);
        assert!(matches!(
            restore(&parsed, &mut w),
            Err(SaveError::SchemaMismatch { .. })
        ));
        assert!(w.is_empty());
    }

    #[test]
    fn limits_are_enforced_before_anything_large_is_reserved() {
        let bytes = save(&world(), &header()).unwrap();
        let reg = registry();
        assert_eq!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_entities: 3,
                    ..Limits::default()
                }
            ),
            Err(SaveError::TooLarge("more entities than the limit"))
        );
        assert!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_entities: 4,
                    ..Limits::default()
                }
            )
            .is_ok()
        );
        assert_eq!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_file_bytes: bytes.len() - 1,
                    ..Limits::default()
                }
            ),
            Err(SaveError::TooLarge("the file"))
        );
        assert!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_file_bytes: bytes.len(),
                    ..Limits::default()
                }
            )
            .is_ok()
        );
        assert_eq!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_text_bytes: 3,
                    ..Limits::default()
                }
            ),
            Err(SaveError::TooLarge("a text is longer than the limit"))
        );
        // A count that claims a billion entities in a tiny file.
        let mut body = unseal(&save(&SimpleWorld::new(registry()), &header()).unwrap());
        let count_at = body.len() - 4;
        body[count_at..].copy_from_slice(&1_000_000_000u32.to_le_bytes());
        let lying = seal(body);
        assert_eq!(
            parse(
                &lying,
                &reg,
                &Limits {
                    max_entities: usize::MAX,
                    ..Limits::default()
                }
            ),
            Err(SaveError::Truncated)
        );
    }

    #[test]
    fn damage_the_checksum_cannot_see_is_still_refused() {
        let reg = registry();
        let good_body = unseal(&save(&world(), &header()).unwrap());
        let mut r = Reader {
            bytes: &good_body,
            pos: 6 + 8 + 8 + 8 + 1,
        };
        let notes = r.u8().unwrap();
        for _ in 0..notes * 2 {
            let n = r.u32().unwrap() as usize;
            r.take(n).unwrap();
        }
        let count_at = r.pos;
        let first_entity = count_at + 4;
        // The first entity's class id, parent and index.
        let class_at = first_entity + 8;
        let parent_at = class_at + 2;
        let tweak = |at: usize, with: &[u8]| {
            let mut b = good_body.clone();
            b[at..at + with.len()].copy_from_slice(with);
            seal(b)
        };
        assert_eq!(
            parse(
                &tweak(class_at, &99u16.to_le_bytes()),
                &reg,
                &Limits::default()
            ),
            Err(SaveError::Invalid("an entity has an unknown class"))
        );
        assert_eq!(
            parse(
                &tweak(parent_at, &EntityId::new(4242, 0).to_u64().to_le_bytes()),
                &reg,
                &Limits::default()
            ),
            Err(SaveError::Invalid("a parent is not a saved entity"))
        );
        assert!(
            parse(
                &tweak(first_entity, &u32::MAX.to_le_bytes()),
                &reg,
                &Limits::default()
            )
            .is_err()
        );
        // Trailing bytes after the last entity.
        let mut longer = good_body.clone();
        longer.push(0);
        assert_eq!(
            parse(&seal(longer), &reg, &Limits::default()),
            Err(SaveError::Invalid("bytes follow the last entity"))
        );
        // A header note that is not UTF-8.
        let note_at = 6 + 8 + 8 + 8 + 1 + 1 + 4;
        assert_eq!(
            parse(&tweak(note_at, &[0xFF]), &reg, &Limits::default()),
            Err(SaveError::Invalid("a text is not valid UTF-8"))
        );
    }

    /// A file written by hand: `Thing` entities given as (index, generation, parent) with default values.
    fn hand_made(entities: &[(u32, u32, EntityId)]) -> Vec<u8> {
        let reg = registry();
        let mut body = Vec::new();
        body.extend_from_slice(&MAGIC);
        body.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        body.extend_from_slice(&reg.schema_hash().to_le_bytes());
        body.extend_from_slice(&[0; 8 + 8 + 1 + 1]);
        body.extend_from_slice(&(entities.len() as u32).to_le_bytes());
        let thing = reg.class_id("Thing").unwrap();
        for (index, generation, parent) in entities {
            body.extend_from_slice(&index.to_le_bytes());
            body.extend_from_slice(&generation.to_le_bytes());
            body.extend_from_slice(&thing.0.to_le_bytes());
            body.extend_from_slice(&parent.to_u64().to_le_bytes());
            body.extend_from_slice(&[0; 12]); // origin
            body.extend_from_slice(&0u32.to_le_bytes()); // tag
        }
        seal(body)
    }

    #[test]
    fn hand_made_files_load_and_the_ways_they_can_be_wrong_are_each_refused() {
        let reg = registry();
        let parse = |bytes: &[u8]| parse(bytes, &reg, &Limits::default());
        let none = EntityId::NULL;
        let good = parse(&hand_made(&[
            (0, 0, none),
            (3, 2, EntityId::new(0, 0)),
            (9, 0, EntityId::new(3, 2)),
        ]))
        .unwrap();
        assert_eq!(good.entities.len(), 3);
        assert_eq!(good.entities[2].parent, Some(EntityId::new(3, 2)));
        // Two entities that are each other's parent.
        assert_eq!(
            parse(&hand_made(&[
                (0, 0, EntityId::new(1, 0)),
                (1, 0, EntityId::new(0, 0))
            ])),
            Err(SaveError::Invalid("parent links form a cycle"))
        );
        // An entity that is its own parent.
        assert_eq!(
            parse(&hand_made(&[(0, 0, EntityId::new(0, 0))])),
            Err(SaveError::Invalid("parent links form a cycle"))
        );
        // The right index with the wrong generation is another entity (a stale handle).
        assert_eq!(
            parse(&hand_made(&[(0, 0, none), (1, 0, EntityId::new(0, 5))])),
            Err(SaveError::Invalid("a parent is not a saved entity"))
        );
        // Entities out of order, or repeated.
        assert_eq!(
            parse(&hand_made(&[(5, 0, none), (2, 0, none)])),
            Err(SaveError::Invalid("entities are not in index order"))
        );
        assert_eq!(
            parse(&hand_made(&[(2, 0, none), (2, 1, none)])),
            Err(SaveError::Invalid("entities are not in index order"))
        );
        // The null handle as an entity.
        assert_eq!(
            parse(&hand_made(&[(u32::MAX, u32::MAX, none)])),
            Err(SaveError::Invalid("an entity handle is null"))
        );
    }

    #[test]
    fn the_text_limit_is_exact() {
        let bytes = save(&world(), &header()).unwrap();
        let longest = "Ünïcode ✓".len();
        let reg = registry();
        assert!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_text_bytes: longest,
                    ..Limits::default()
                }
            )
            .is_ok()
        );
        assert_eq!(
            parse(
                &bytes,
                &reg,
                &Limits {
                    max_text_bytes: longest - 1,
                    ..Limits::default()
                }
            ),
            Err(SaveError::TooLarge("a text is longer than the limit"))
        );
    }

    #[test]
    fn restoring_into_a_world_that_has_entities_changes_nothing() {
        let bytes = save(&world(), &header()).unwrap();
        let file = parse(&bytes, &registry(), &Limits::default()).unwrap();
        let mut busy = SimpleWorld::new(registry());
        busy.spawn(busy.registry().class_id("Thing").unwrap(), None)
            .unwrap();
        assert_eq!(restore(&file, &mut busy), Err(SaveError::WorldNotEmpty));
        assert_eq!(busy.len(), 1);
    }

    #[test]
    fn a_header_that_cannot_be_written_is_refused_at_save_time() {
        let w = SimpleWorld::new(registry());
        let many = SaveHeader {
            notes: (0..=MAX_NOTES)
                .map(|i| (i.to_string(), String::new()))
                .collect(),
            ..SaveHeader::default()
        };
        assert!(matches!(save(&w, &many), Err(SaveError::TooLarge(_))));
        let long = SaveHeader {
            notes: vec![("k".into(), "v".repeat(MAX_NOTE_LEN + 1))],
            ..SaveHeader::default()
        };
        assert!(matches!(save(&w, &long), Err(SaveError::TooLarge(_))));
        let edge = SaveHeader {
            notes: (0..MAX_NOTES)
                .map(|_| ("k".repeat(MAX_NOTE_LEN), "v".repeat(MAX_NOTE_LEN)))
                .collect(),
            ..SaveHeader::default()
        };
        let bytes = save(&w, &edge).unwrap();
        assert_eq!(
            parse(&bytes, &registry(), &Limits::default())
                .unwrap()
                .header,
            edge
        );
    }

    #[test]
    fn the_spawns_of_a_load_are_left_in_the_worlds_records_for_the_caller_to_drain() {
        let bytes = save(&world(), &header()).unwrap();
        let mut loaded = SimpleWorld::new(registry());
        load(&bytes, &mut loaded, &Limits::default()).unwrap();
        assert_eq!(loaded.drain_lifecycle().len(), 4);
    }
}
