//! ss2-format-wld — a minimal `.wld` world reader (roadmap T02-035, a slice of T02-020; specs
//! `docs/re/wld-root.md`, `wld-geometry.md`, `wld-entities.md`, `wld-entity-schema.md`).
//!
//! A world is a CTSEMETA object graph with one `CWorld` root. Its entity list (`CWorld.1`) holds
//! every entity; the level architecture is not a brush or terrain format but **placed models**:
//! `CStaticModelEntity` objects, each owning a model renderable whose instance names an `.mdl`
//! file and a per-axis scale. [`World::from_bytes`] reads just enough to put a level on screen and
//! walk in it:
//!
//! - the gravity vector (`CWorld.10`; `(0, -30, 0)` in every retail world),
//! - every static model: the model, its **world** placement (taken as stored from its renderable,
//!   `CAspect.7`, so child entities need no recomputation), the instance scale (which may be
//!   negative on an axis: a mirrored model) and whether it has a collision hull,
//! - the spawn markers (`CSpawnMarkerEntity`) and the chapter info entities
//!   (`CChapterInfoEntity`) with their placements and editor names.
//!
//! Everything else (lights, the grids, the physics bodies, scripts, the other entity classes) is
//! left out; [`World::entity_count`] says how many entities the file has in all.
//!
//! Coordinates are the file's: right-handed, y up. A rotation is stored as a quaternion
//! `(x, y, z, w)`; the matrix of a placement is the rotation matrix with the translation as a
//! column, and a model's matrix is `translation × rotation × scale` ([`StaticModel::transform`]).
//!
//! ```ignore
//! let world = ss2_format_wld::World::from_bytes(&bytes)?;
//! for m in &world.static_models {
//!     if let Some(path) = m.model.path() { /* load the .mdl, draw it with m.transform() */ }
//! }
//! ```

use std::collections::HashMap;

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, FromValue, MetaDeserialize, Object, ObjectRef, TypeSystem};

const WHAT: &str = ".wld world";

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

/// Where the model of a static model comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelRef {
    /// An external `.mdl` file: the path as the world's file table stores it (for example
    /// `Content/SeriousSam2/Levels/Multiplayer/DM_DockingBay/DockingBay.mdl`).
    External(String),
    /// A `CModelConfiguration` object stored in the world itself (the object id). 109 of the
    /// 25,383 retail static models do this; reading such a model is not part of this crate.
    Embedded(u32),
}

impl ModelRef {
    /// The path of an external model, `None` for an embedded one.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::External(p) => Some(p),
            Self::Embedded(_) => None,
        }
    }
}

/// One `CStaticModelEntity`: a model placed in the world.
#[derive(Clone, Debug, PartialEq)]
pub struct StaticModel {
    /// The entity's serial (`CEntity.1`), unique in the world.
    pub serial: u32,
    /// The editor name from the property record (`CEntityProperties.4`), empty without a record.
    pub name: String,
    /// The model (`CModelInstance.3` of the renderable's instance).
    pub model: ModelRef,
    /// World rotation of the model renderable (`CAspect.7`).
    pub rotation: Quat,
    /// World translation of the model renderable (`CAspect.7`).
    pub translation: Vec3,
    /// Per-axis scale of the instance (`CModelInstance.1`); a negative axis mirrors the model,
    /// which flips its triangle winding.
    pub scale: Vec3,
    /// Whether the entity has a mechanism and so at least one collision hull (`.10` not null).
    /// The hull is the model's own collision mesh for static level geometry.
    pub has_hull: bool,
}

impl StaticModel {
    /// The model-to-world matrix: scale first, then rotate, then translate.
    pub fn transform(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

/// A placed marker entity (a spawn marker or a chapter info entity).
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    /// The entity's serial (`CEntity.1`).
    pub serial: u32,
    /// The editor name from the property record, empty without a record.
    pub name: String,
    /// World rotation.
    pub rotation: Quat,
    /// World position.
    pub translation: Vec3,
}

/// The parts of a `.wld` world this crate reads.
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    /// Gravity in m/s², y up (`CWorld.10`).
    pub gravity: Vec3,
    /// The number of entities of the world (`CWorld.1`), of every class.
    pub entity_count: usize,
    /// Every `CStaticModelEntity`, in entity-list order.
    pub static_models: Vec<StaticModel>,
    /// Every `CSpawnMarkerEntity`, in entity-list order. The specs only describe how deathmatch
    /// picks among them (`helper-classes-iterators.md`); single player and co-op starts are open
    /// (T01-210).
    pub spawn_markers: Vec<Marker>,
    /// Every `CChapterInfoEntity` (what the player starts a chapter with), in entity-list order.
    /// Whether a player is placed at a chapter's placement is not specified yet (T01-210); it is
    /// read because it is cheap and a level without spawn markers usually has chapters.
    pub chapters: Vec<Marker>,
}

// ---------------------------------------------------------------------------------------------
// The typed views of the classes read (member names from `wld-entity-schema.md`).
// ---------------------------------------------------------------------------------------------

#[derive(MetaDeserialize)]
struct Vector3f {
    #[meta(owner = "Vector3f", name = "1")]
    x: f32,
    #[meta(owner = "Vector3f", name = "2")]
    y: f32,
    #[meta(owner = "Vector3f", name = "3")]
    z: f32,
}

impl From<Vector3f> for Vec3 {
    fn from(v: Vector3f) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

#[derive(MetaDeserialize)]
struct Quaternion4f {
    #[meta(owner = "Quaternion4f", name = "1")]
    x: f32,
    #[meta(owner = "Quaternion4f", name = "2")]
    y: f32,
    #[meta(owner = "Quaternion4f", name = "3")]
    z: f32,
    #[meta(owner = "Quaternion4f", name = "4")]
    w: f32,
}

#[derive(MetaDeserialize)]
struct QuatVect {
    #[meta(owner = "QuatVect", name = "1")]
    rotation: Quaternion4f,
    #[meta(owner = "QuatVect", name = "2")]
    translation: Vector3f,
}

impl QuatVect {
    fn split(self) -> (Quat, Vec3) {
        let q = self.rotation;
        (Quat::from_xyzw(q.x, q.y, q.z, q.w), self.translation.into())
    }
}

#[derive(MetaDeserialize)]
struct WorldRoot {
    #[meta(owner = "CWorld", name = "1")]
    entities: Vec<ObjectRef>,
    #[meta(owner = "CWorld", name = "5")]
    records: Vec<ObjectRef>,
    #[meta(owner = "CWorld", name = "10")]
    gravity: Vector3f,
}

#[derive(MetaDeserialize)]
struct Record {
    #[meta(owner = "CEntityProperties", name = "2")]
    entity: ObjectRef,
    #[meta(owner = "CEntityProperties", name = "4")]
    name: String,
}

#[derive(MetaDeserialize)]
struct Entity {
    #[meta(owner = "CEntity", name = "1")]
    serial: u32,
}

#[derive(MetaDeserialize)]
struct StaticModelEntity {
    #[meta(owner = "CStaticModelEntity", name = "1")]
    renderable: ObjectRef,
    #[meta(owner = "CStaticModelEntity", name = "10")]
    mechanism: ObjectRef,
}

#[derive(MetaDeserialize)]
struct Aspect {
    #[meta(owner = "CAspect", name = "1")]
    parent: ObjectRef,
    #[meta(owner = "CAspect", name = "5")]
    owner: ObjectRef,
    #[meta(owner = "CAspect", name = "7")]
    world: QuatVect,
}

#[derive(MetaDeserialize)]
struct ModelRenderable {
    #[meta(owner = "CModelRenderable", name = "1")]
    instance: ObjectRef,
}

#[derive(MetaDeserialize)]
struct ModelInstance {
    #[meta(owner = "CModelInstance", name = "1")]
    scale: Vector3f,
    #[meta(owner = "CModelInstance", name = "3")]
    model: ObjectRef,
}

/// The decoded file with an id index over the main objects and the extra records.
struct Graph<'a> {
    container: &'a Container<'a>,
    decoded: &'a Decoded,
    by_id: HashMap<u32, &'a Object>,
}

impl<'a> Graph<'a> {
    fn new(container: &'a Container<'a>, decoded: &'a Decoded) -> Result<Self> {
        let mut by_id = HashMap::with_capacity(decoded.objects.len() + decoded.extra.len());
        for o in decoded.objects.iter().chain(&decoded.extra) {
            if by_id.insert(o.id, o).is_some() {
                return Err(bad(format!("object id {} is used twice", o.id)));
            }
        }
        Ok(Self {
            container,
            decoded,
            by_id,
        })
    }

    fn types(&self) -> &'a TypeSystem {
        &self.decoded.types
    }

    fn class_of(&self, o: &Object) -> &'a str {
        self.types().type_name(o.type_id).unwrap_or("?")
    }

    /// The object `id` (an object of the file, main or extra), or an error naming `what`.
    fn object(&self, id: Option<u32>, what: &str) -> Result<&'a Object> {
        let id = id.ok_or_else(|| bad(format!("{what} is null")))?;
        self.by_id.get(&id).copied().ok_or_else(|| {
            bad(format!(
                "{what} refers to object {id}, which is not in the file"
            ))
        })
    }

    fn typed<T: FromValue>(&self, o: &Object, what: &str) -> Result<T> {
        T::from_value(&o.value, self.types()).map_err(|e| {
            bad(format!(
                "{what} (object {}, `{}`): {e}",
                o.id,
                self.class_of(o)
            ))
        })
    }

    fn external_path(&self, id: u32) -> Option<&'a str> {
        let e = self
            .container
            .external_objects
            .iter()
            .find(|e| e.object_id == id)?;
        self.container
            .files
            .get(e.file_ref as usize)
            .map(|f| f.path.as_ref())
    }

    fn model_ref(&self, id: Option<u32>) -> Result<ModelRef> {
        let id = id.ok_or_else(|| bad("a model instance names no model"))?;
        if let Some(path) = self.external_path(id) {
            if !path.to_ascii_lowercase().ends_with(".mdl") {
                return Err(bad(format!("a model instance names `{path}`, not an .mdl")));
            }
            return Ok(ModelRef::External(path.to_string()));
        }
        let o = self.object(Some(id), "a model instance's model")?;
        match self.class_of(o) {
            "CModelConfiguration" => Ok(ModelRef::Embedded(id)),
            other => Err(bad(format!(
                "a model instance names object {id}, a `{other}`, not a model"
            ))),
        }
    }
}

impl World {
    /// Read a `.wld` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] if the file is not a format-8 CTSEMETA container, does not have
    /// exactly one `CWorld`, a reference the reader follows is null where it must not be or names
    /// an object that is not in the file, a member it reads is missing or has the wrong shape, or
    /// a static model names an external file that is not an `.mdl`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(bad(format!(
                "format version {}",
                container.header.format_version
            )));
        }
        let decoded = container.decode_objects()?;
        let g = Graph::new(&container, &decoded)?;

        let mut roots = decoded.objects.iter().filter(|o| g.class_of(o) == "CWorld");
        let root = match (roots.next(), roots.next()) {
            (Some(root), None) => root,
            (None, _) => return Err(bad("the file has no CWorld")),
            (Some(_), Some(_)) => return Err(bad("the file has more than one CWorld")),
        };
        let root: WorldRoot = g.typed(root, "the CWorld")?;

        // Editor names, by entity id, from the property records (`CWorld.5`).
        let mut names: HashMap<u32, String> = HashMap::with_capacity(root.records.len());
        for r in &root.records {
            let o = g.object(r.0, "a property record of CWorld.5")?;
            let record: Record = g.typed(o, "a property record")?;
            if let Some(entity) = record.entity.0 {
                names.insert(entity, record.name);
            }
        }

        let markers_wanted = root.entities.iter().any(|e| {
            e.0.and_then(|id| g.by_id.get(&id)).is_some_and(|o| {
                matches!(g.class_of(o), "CSpawnMarkerEntity" | "CChapterInfoEntity")
            })
        });
        let aspects = if markers_wanted {
            AspectIndex::new(&g)?
        } else {
            AspectIndex::default()
        };

        let mut world = Self {
            gravity: root.gravity.into(),
            entity_count: root.entities.len(),
            static_models: Vec::new(),
            spawn_markers: Vec::new(),
            chapters: Vec::new(),
        };
        for e in &root.entities {
            let o = g.object(e.0, "an entity of CWorld.1")?;
            let class = g.class_of(o);
            if !matches!(
                class,
                "CStaticModelEntity" | "CSpawnMarkerEntity" | "CChapterInfoEntity"
            ) {
                continue;
            }
            let serial = g.typed::<Entity>(o, "an entity")?.serial;
            let name = names.get(&o.id).cloned().unwrap_or_default();
            match class {
                "CStaticModelEntity" => {
                    world.static_models.push(static_model(&g, o, serial, name)?);
                }
                _ => {
                    let (rotation, translation) = aspects.placement(o.id).ok_or_else(|| {
                        bad(format!(
                            "the {class} object {} owns no aspect to place it",
                            o.id
                        ))
                    })?;
                    let marker = Marker {
                        serial,
                        name,
                        rotation,
                        translation,
                    };
                    if class == "CSpawnMarkerEntity" {
                        world.spawn_markers.push(marker);
                    } else {
                        world.chapters.push(marker);
                    }
                }
            }
        }
        Ok(world)
    }
}

fn static_model(g: &Graph<'_>, o: &Object, serial: u32, name: String) -> Result<StaticModel> {
    let entity: StaticModelEntity = g.typed(o, "a static model entity")?;
    let what = format!("the renderable of static model {}", o.id);
    let renderable = g.object(entity.renderable.0, &what)?;
    let aspect: Aspect = g.typed(renderable, &what)?;
    let model: ModelRenderable = g.typed(renderable, &what)?;
    let what = format!("the model instance of static model {}", o.id);
    let instance = g.object(model.instance.0, &what)?;
    let instance: ModelInstance = g.typed(instance, &what)?;
    let (rotation, translation) = aspect.world.split();
    Ok(StaticModel {
        serial,
        name,
        model: g.model_ref(instance.model.0)?,
        rotation,
        translation,
        scale: instance.scale.into(),
        has_hull: entity.mechanism.0.is_some(),
    })
}

/// Per entity, the aspects it owns (object order), and per aspect its parent and world placement.
#[derive(Default)]
struct AspectIndex {
    owned: HashMap<u32, Vec<u32>>,
    aspects: HashMap<u32, (Option<u32>, Option<u32>, Quat, Vec3)>,
}

impl AspectIndex {
    fn new(g: &Graph<'_>) -> Result<Self> {
        let mut index = Self::default();
        // Whether a class derives from `CAspect`, per type id.
        let mut is_aspect: HashMap<u32, bool> = HashMap::new();
        for o in &g.decoded.objects {
            let aspect_class = *is_aspect.entry(o.type_id).or_insert_with(|| {
                g.types()
                    .members(o.type_id)
                    .is_some_and(|m| m.iter().any(|m| m.owner_name == "CAspect"))
            });
            if !aspect_class {
                continue;
            }
            let a: Aspect = g.typed(o, "an aspect")?;
            let (rotation, translation) = a.world.split();
            if let Some(owner) = a.owner.0 {
                index.owned.entry(owner).or_default().push(o.id);
            }
            index
                .aspects
                .insert(o.id, (a.parent.0, a.owner.0, rotation, translation));
        }
        Ok(index)
    }

    /// The world placement of the top of the entity's aspects: the first owned aspect (in object
    /// order) whose parent is not owned by the same entity.
    fn placement(&self, entity: u32) -> Option<(Quat, Vec3)> {
        self.owned.get(&entity)?.iter().find_map(|id| {
            let (parent, _, rotation, translation) = self.aspects.get(id)?;
            let parent_owner = parent.and_then(|p| self.aspects.get(&p)).and_then(|a| a.1);
            (parent_owner != Some(entity)).then_some((*rotation, *translation))
        })
    }
}

/// The `.wld` loader: register it with `AssetManager::register_loader` to load [`World`]s. The
/// models are returned as paths; request them through the asset manager.
#[derive(Clone, Copy, Debug, Default)]
pub struct WldLoader;

impl AssetLoader for WldLoader {
    type Asset = World;

    fn extensions(&self) -> &[&str] {
        &["wld"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<World> {
        World::from_bytes(&ctx.read()?)
    }
}

#[cfg(test)]
mod tests;
