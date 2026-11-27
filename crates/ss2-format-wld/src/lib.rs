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
//!   (`CChapterInfoEntity`) with their placements and editor names, and the chapter chain: the
//!   first chapter the world info entity names (`CWorldInfoEntity.2`) and each chapter's next
//!   chapter (`CChapterInfoEntity.2`; `mechanism-classes-world-info.md`, the single-player start
//!   of `chapter-restore-players.md` section 2.8),
//! - the sky (`CCubeBackgroundEntity`, T02-039; spec `docs/re/environment.md`, section "Sky"):
//!   its face textures by face member, colour, size and viewer height ([`Sky`]),
//! - each static model's **baked lightmap** (T02-040; spec `docs/re/wld-lighting.md`, section
//!   "Baked lightmaps"): the `CTextureOvenShaderModifier` in its model instance's modifier list
//!   and the `CStaticTexture` stored in the world that it names, decoded with `ss2-format-tex`
//!   ([`Lightmap`]); and, over every model instance of the world, how many are lit
//!   ([`LightmapCounts`]).
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
use ss2_format_tex::Texture;
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
    /// The baked lightmap of this placement, `None` when its model instance has no oven
    /// modifier, or when the chain to it is broken (no modifier list, a reference to an object
    /// that is not in the file, a modifier of the wrong shape).
    pub lightmap: Option<Lightmap>,
}

/// An identifier as stored (`IDENT`): its index in the file's identifier table and its text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ident {
    /// The index in the identifier table (0 is the empty identifier).
    pub id: u32,
    /// The text, empty when the index is not in the table.
    pub name: String,
}

/// A placement's baked lightmap: the first `CTextureOvenShaderModifier` of its model instance's
/// modifier list (`CModelInstance.4` → `CShaderModifierList.1`) and the texture it names
/// (`wld-lighting.md`, "Baked lightmaps").
///
/// Which of the mesh's UV streams the lightmap is sampled through is **not** stored here: the
/// modifier's UV map id is not one of the mesh's ids (reader invariant 3); the spec's rule picks
/// the mesh's own stream that lies in [0, 1] without overlap, which is the job of the code that
/// has the mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct Lightmap {
    /// The object id of the oven modifier.
    pub modifier: u32,
    /// The shader parameter overridden with the texture (`.1`; "3 texture" in the test vector).
    pub texture_param: Ident,
    /// The shader parameter overridden with the UV map (`.2`; "3 uvmap").
    pub uv_param: Ident,
    /// The UV map id (`.3` → `CShaderArgValUVMap.1`; "Shadow"): a per-world constant that names
    /// none of the mesh's streams. For diagnostics only.
    pub uv_map: Ident,
    /// The object id of the lightmap texture (`.4` → `CShaderArgValTexture.1`) when it is a
    /// `CStaticTexture` object of the file, `None` otherwise.
    pub texture_object: Option<u32>,
    /// The decoded lightmap ([`Texture::from_object`]), or why it cannot be read (the texture
    /// is null, not in the file, not a `CStaticTexture`, or breaks a rule of `tex.md`).
    pub texture: std::result::Result<Texture, String>,
    /// How many more oven modifiers the list has after this one. The spec counts instances
    /// with two or more but does not say how the engine combines them; only the first is read.
    pub other_oven_modifiers: usize,
}

/// Lightmap counts over **every** `CModelInstance` of the main section, whichever entity uses
/// it (static models, but also doors and props): the numbers `wld-lighting.md` gives per world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LightmapCounts {
    /// Model instances with at least one oven modifier in their list.
    pub lit_instances: usize,
    /// Oven modifiers in the lists of all model instances.
    pub oven_modifiers: usize,
    /// Model instances with more than one oven modifier.
    pub several: usize,
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
    /// For a chapter info entity, the next chapter of the chain (`CChapterInfoEntity.2`) as an
    /// index into [`World::chapters`]; `None` at the end of the chain, when the handle names no
    /// chapter of the entity list, and always for a spawn marker.
    pub next: Option<usize>,
}

/// One face of the sky cube: the `CCubeRenderable` face members `.10` to `.15`, in that order
/// (`docs/re/environment.md`, "Which face member is which direction").
///
/// The engine axis of each face is the spec's convention (**medium** confidence: the compass names
/// come from the engine, the axes follow the right-handed, y-up, -Z-forward frame): North -Z,
/// West -X, South +Z, East +X, Floor -Y, Ceiling +Y. How each face image is turned is the spec's
/// orientation rule and is left to the code that draws the sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkyFace {
    /// `.10`, towards -Z.
    North,
    /// `.11`, towards -X.
    West,
    /// `.12`, towards +Z.
    South,
    /// `.13`, towards +X.
    East,
    /// `.14`, towards -Y (below); present in only 11 retail skies.
    Floor,
    /// `.15`, towards +Y (above).
    Ceiling,
}

impl SkyFace {
    /// Every face, in member order (`.10` to `.15`): the order of [`Sky::faces`].
    pub const ALL: [Self; 6] = [
        Self::North,
        Self::West,
        Self::South,
        Self::East,
        Self::Floor,
        Self::Ceiling,
    ];

    /// The position of this face in [`SkyFace::ALL`] and [`Sky::faces`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// The name of its `CCubeRenderable` member (`"10"` to `"15"`).
    pub fn member(self) -> &'static str {
        ["10", "11", "12", "13", "14", "15"][self.index()]
    }
}

/// The sky of a world: its `CCubeBackgroundEntity`, the entity's property record
/// (`CCubeBackgroundProperties`) and its renderables (`CCubeRenderable`,
/// `CSimpleBackgroundViewerRenderable`).
///
/// The values the record and the renderable share (colour, size, the `.4` flag) and the viewer
/// height are taken from the record, as the spec's reader invariant 1 asks; a sky without a record
/// takes them from the renderable and the entity instead (in the retail data they agree, 58 of 58).
#[derive(Clone, Debug, PartialEq)]
pub struct Sky {
    /// The entity's serial (`CEntity.1`).
    pub serial: u32,
    /// The editor name from the property record, empty without a record.
    pub name: String,
    /// The face textures in [`SkyFace::ALL`] order (members `.10` to `.15`): the path as the
    /// world's file table stores it (an external `.tex`; two retail skies name pre-format-8
    /// "legacy" textures), `None` where the member is null. Retail skies have five faces (no
    /// floor), six, or none at all (3 worlds). A member that points at an object stored in the
    /// world itself rather than an external file is also `None` (no retail sky does that).
    pub faces: [Option<String>; 6],
    /// The record's own texture (`CCubeBackgroundProperties.1`): in the retail data always the
    /// same file as one of the faces, so drawing the sky does not need it. `None` when null, when
    /// the sky has no record, or when it is not an external file.
    pub texture: Option<String>,
    /// The colour tint (`CCubeBackgroundProperties.6`, else `CCubeRenderable.3`) as the stored
    /// 32-bit value, which the spec reads as ARGB (`0xFFFFFF` in most skies, where the top byte
    /// is 0).
    pub color: u32,
    /// The cube's half-extent in world units (`.2`, else `CCubeRenderable.1`; 64 in 56 skies).
    pub size: f32,
    /// The viewer's vertical offset in the cube (`.3`, else `en_fViewerY`; -6.66 to 4.52).
    pub viewer_y: f32,
    /// The record's `.4` (else `CCubeRenderable.2`): 0 or 1, meaning not established.
    pub flag_4: f32,
    /// The record's `.5` (`None` without a record): meaning not established.
    pub flag_5: Option<bool>,
    /// The viewer renderable's `.1` (5.22 or 55.68 in the retail data; meaning not
    /// established), `None` when the entity has no viewer renderable.
    pub viewer_value: Option<f32>,
    /// The entity's rotation (`en_qvPlacement`): the renderer turns the whole cube by it, in
    /// full (`environment.md`, T01-409; 44 of the 58 retail skies are turned, all about +y).
    pub rotation: Quat,
    /// The entity's position (`en_qvPlacement`); never used by the renderer: the cube is drawn
    /// around the viewer, not here (T01-409).
    pub translation: Vec3,
}

impl Sky {
    /// The texture path of one face, `None` when the face is absent.
    pub fn face(&self, face: SkyFace) -> Option<&str> {
        self.faces[face.index()].as_deref()
    }

    /// How many faces name a texture.
    pub fn face_count(&self) -> usize {
        self.faces.iter().filter(|f| f.is_some()).count()
    }
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
    /// Every `CSpawnMarkerEntity`, in entity-list order. Only deathmatch worlds have them, and
    /// only deathmatch places players at them (`helper-classes-iterators.md` section 2.5,
    /// `chapter-restore-players.md` section 2.9).
    pub spawn_markers: Vec<Marker>,
    /// Every `CChapterInfoEntity` (what the player starts a chapter with), in entity-list order.
    /// That order is not the order of the chain: follow [`World::first_chapter`] and
    /// [`Marker::next`], or use [`World::chapter_chain`].
    pub chapters: Vec<Marker>,
    /// The world's first chapter (`CWorldInfoEntity.2`, the handle a new game starts at) as an
    /// index into [`World::chapters`]; `None` when the world has no world info entity, its
    /// handle is null (8 retail worlds) or names no chapter of the entity list. With several
    /// world info entities (none in the retail data) the first in entity-list order counts.
    pub first_chapter: Option<usize>,
    /// The sky (`CCubeBackgroundEntity`), `None` when the world has none (one retail world,
    /// `dockingbay`). Retail worlds have at most one; if a world had several, this is the first
    /// in entity-list order.
    pub sky: Option<Sky>,
    /// How many model instances of the world are lit, and by how many oven modifiers.
    pub lightmap_counts: LightmapCounts,
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

/// Read on its own: a chapter that lacks the member just has no next chapter.
#[derive(MetaDeserialize)]
struct ChapterLink {
    #[meta(owner = "CChapterInfoEntity", name = "2")]
    next: ObjectRef,
}

/// Read on its own: a world info that lacks the member just names no first chapter.
#[derive(MetaDeserialize)]
struct WorldInfoLink {
    #[meta(owner = "CWorldInfoEntity", name = "2")]
    first_chapter: ObjectRef,
}

#[derive(MetaDeserialize)]
struct StaticModelEntity {
    #[meta(owner = "CStaticModelEntity", name = "1")]
    renderable: ObjectRef,
    #[meta(owner = "CStaticModelEntity", name = "10")]
    mechanism: ObjectRef,
}

#[derive(MetaDeserialize)]
struct SkyEntity {
    #[meta(owner = "CCubeBackgroundEntity", name = "1")]
    cube: ObjectRef,
    #[meta(owner = "CCubeBackgroundEntity", name = "3")]
    viewer: ObjectRef,
    #[meta(owner = "CCubeBackgroundEntity", name = "en_qvPlacement")]
    placement: QuatVect,
    #[meta(owner = "CCubeBackgroundEntity", name = "en_fViewerY")]
    viewer_y: f32,
}

#[derive(MetaDeserialize)]
struct CubeRenderable {
    #[meta(owner = "CCubeRenderable", name = "1")]
    size: f32,
    #[meta(owner = "CCubeRenderable", name = "2")]
    flag: f32,
    #[meta(owner = "CCubeRenderable", name = "3")]
    color: u32,
    #[meta(owner = "CCubeRenderable", name = "10")]
    north: ObjectRef,
    #[meta(owner = "CCubeRenderable", name = "11")]
    west: ObjectRef,
    #[meta(owner = "CCubeRenderable", name = "12")]
    south: ObjectRef,
    #[meta(owner = "CCubeRenderable", name = "13")]
    east: ObjectRef,
    #[meta(owner = "CCubeRenderable", name = "14")]
    floor: ObjectRef,
    #[meta(owner = "CCubeRenderable", name = "15")]
    ceiling: ObjectRef,
}

#[derive(MetaDeserialize)]
struct ViewerRenderable {
    #[meta(owner = "CSimpleBackgroundViewerRenderable", name = "1")]
    value: f32,
}

#[derive(MetaDeserialize)]
struct SkyRecord {
    #[meta(owner = "CCubeBackgroundProperties", name = "1")]
    texture: ObjectRef,
    #[meta(owner = "CCubeBackgroundProperties", name = "2")]
    size: f32,
    #[meta(owner = "CCubeBackgroundProperties", name = "3")]
    viewer_y: f32,
    #[meta(owner = "CCubeBackgroundProperties", name = "4")]
    flag_4: f32,
    #[meta(owner = "CCubeBackgroundProperties", name = "5")]
    flag_5: bool,
    #[meta(owner = "CCubeBackgroundProperties", name = "6")]
    color: u32,
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

/// Read apart from [`ModelInstance`]: a broken modifier list costs the lightmap, not the model.
#[derive(MetaDeserialize)]
struct InstanceModifiers {
    #[meta(owner = "CModelInstance", name = "4")]
    list: ObjectRef,
}

#[derive(MetaDeserialize)]
struct ModifierList {
    #[meta(owner = "CShaderModifierList", name = "1")]
    modifiers: Vec<ObjectRef>,
}

#[derive(MetaDeserialize)]
struct OvenModifier {
    #[meta(owner = "CTextureOvenShaderModifier", name = "1")]
    texture_param: u32,
    #[meta(owner = "CTextureOvenShaderModifier", name = "2")]
    uv_param: u32,
    #[meta(owner = "CTextureOvenShaderModifier", name = "3")]
    uv_map: UvMapArg,
    #[meta(owner = "CTextureOvenShaderModifier", name = "4")]
    texture: TextureArg,
}

#[derive(MetaDeserialize)]
struct UvMapArg {
    #[meta(owner = "CShaderArgValUVMap", name = "1")]
    id: u32,
}

#[derive(MetaDeserialize)]
struct TextureArg {
    #[meta(owner = "CShaderArgValTexture", name = "1")]
    texture: ObjectRef,
}

const OVEN_MODIFIER: &str = "CTextureOvenShaderModifier";

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

    fn ident(&self, id: u32) -> Ident {
        Ident {
            id,
            name: self.container.ident(id).unwrap_or_default().to_string(),
        }
    }

    /// The oven modifiers of a model instance's modifier list, in list order; empty when the
    /// instance has no list or the list cannot be read.
    fn oven_modifiers(&self, instance: &Object) -> Vec<&'a Object> {
        let Ok(InstanceModifiers {
            list: ObjectRef(Some(list)),
        }) = self.typed(instance, "")
        else {
            return Vec::new();
        };
        let Some(list) = self.by_id.get(&list) else {
            return Vec::new();
        };
        let Ok(list) = self.typed::<ModifierList>(list, "") else {
            return Vec::new();
        };
        list.modifiers
            .iter()
            .filter_map(|r| r.0.and_then(|id| self.by_id.get(&id)).copied())
            .filter(|o| self.class_of(o) == OVEN_MODIFIER)
            .collect()
    }

    /// The lightmap of a model instance: its first oven modifier and the texture it names.
    fn lightmap(&self, instance: &Object) -> Option<Lightmap> {
        let ovens = self.oven_modifiers(instance);
        let first = *ovens.first()?;
        let oven: OvenModifier = self.typed(first, "").ok()?;
        let (texture_object, texture) = match oven.texture.texture.0 {
            None => (None, Err("the lightmap texture is null".to_string())),
            Some(id) => match self.by_id.get(&id) {
                None => (
                    None,
                    Err(format!(
                        "the lightmap texture refers to object {id}, which is not stored in the file{}",
                        self.external_path(id)
                            .map(|p| format!(" (the external file `{p}`)"))
                            .unwrap_or_default()
                    )),
                ),
                Some(o) if self.class_of(o) != "CStaticTexture" => (
                    None,
                    Err(format!(
                        "the lightmap texture is object {id}, a `{}`, not a CStaticTexture",
                        self.class_of(o)
                    )),
                ),
                Some(o) => (
                    Some(id),
                    Texture::from_object(&o.value, self.types()).map_err(|e| e.to_string()),
                ),
            },
        };
        Some(Lightmap {
            modifier: first.id,
            texture_param: self.ident(oven.texture_param),
            uv_param: self.ident(oven.uv_param),
            uv_map: self.ident(oven.uv_map.id),
            texture_object,
            texture,
            other_oven_modifiers: ovens.len() - 1,
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

        // Editor names and the records themselves, by entity id, from the property records
        // (`CWorld.5`).
        let mut names: HashMap<u32, String> = HashMap::with_capacity(root.records.len());
        let mut records: HashMap<u32, &Object> = HashMap::with_capacity(root.records.len());
        for r in &root.records {
            let o = g.object(r.0, "a property record of CWorld.5")?;
            let record: Record = g.typed(o, "a property record")?;
            if let Some(entity) = record.entity.0 {
                names.insert(entity, record.name);
                records.insert(entity, o);
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
            first_chapter: None,
            sky: None,
            lightmap_counts: LightmapCounts::default(),
        };
        for o in &decoded.objects {
            if g.class_of(o) != "CModelInstance" {
                continue;
            }
            let n = g.oven_modifiers(o).len();
            let c = &mut world.lightmap_counts;
            c.oven_modifiers += n;
            c.lit_instances += usize::from(n > 0);
            c.several += usize::from(n > 1);
        }
        // The chain's handles (entity object ids), resolved to chapter indices at the end.
        let mut first_chapter: Option<Option<u32>> = None;
        let mut chapter_ids: Vec<u32> = Vec::new();
        let mut next_ids: Vec<Option<u32>> = Vec::new();
        for e in &root.entities {
            let o = g.object(e.0, "an entity of CWorld.1")?;
            let class = g.class_of(o);
            if class == "CWorldInfoEntity" {
                if first_chapter.is_none() {
                    first_chapter = Some(
                        g.typed::<WorldInfoLink>(o, "")
                            .ok()
                            .and_then(|w| w.first_chapter.0),
                    );
                }
                continue;
            }
            if !matches!(
                class,
                "CStaticModelEntity"
                    | "CSpawnMarkerEntity"
                    | "CChapterInfoEntity"
                    | "CCubeBackgroundEntity"
            ) {
                continue;
            }
            if class == "CCubeBackgroundEntity" && world.sky.is_some() {
                continue;
            }
            let serial = g.typed::<Entity>(o, "an entity")?.serial;
            let name = names.get(&o.id).cloned().unwrap_or_default();
            match class {
                "CStaticModelEntity" => {
                    world.static_models.push(static_model(&g, o, serial, name)?);
                }
                "CCubeBackgroundEntity" => {
                    let record = records.get(&o.id).copied();
                    world.sky = Some(sky(&g, o, record, serial, name)?);
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
                        next: None,
                    };
                    if class == "CSpawnMarkerEntity" {
                        world.spawn_markers.push(marker);
                    } else {
                        world.chapters.push(marker);
                        chapter_ids.push(o.id);
                        next_ids.push(g.typed::<ChapterLink>(o, "").ok().and_then(|c| c.next.0));
                    }
                }
            }
        }
        let chapter_index =
            |id: Option<u32>| id.and_then(|id| chapter_ids.iter().position(|c| *c == id));
        for (chapter, next) in world.chapters.iter_mut().zip(next_ids) {
            chapter.next = chapter_index(next);
        }
        world.first_chapter = chapter_index(first_chapter.flatten());
        Ok(world)
    }

    /// The chapter chain as indices into [`World::chapters`]: [`World::first_chapter`], then each
    /// chapter's [`Marker::next`], until a chapter has none or the chain would come back to a
    /// chapter it already holds (the engine forbids such a link). Empty without a first chapter.
    pub fn chapter_chain(&self) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut at = self.first_chapter;
        while let Some(i) = at {
            if i >= self.chapters.len() || chain.contains(&i) {
                break;
            }
            chain.push(i);
            at = self.chapters[i].next;
        }
        chain
    }
}

fn static_model(g: &Graph<'_>, o: &Object, serial: u32, name: String) -> Result<StaticModel> {
    let entity: StaticModelEntity = g.typed(o, "a static model entity")?;
    let what = format!("the renderable of static model {}", o.id);
    let renderable = g.object(entity.renderable.0, &what)?;
    let aspect: Aspect = g.typed(renderable, &what)?;
    let model: ModelRenderable = g.typed(renderable, &what)?;
    let what = format!("the model instance of static model {}", o.id);
    let instance_object = g.object(model.instance.0, &what)?;
    let instance: ModelInstance = g.typed(instance_object, &what)?;
    let (rotation, translation) = aspect.world.split();
    Ok(StaticModel {
        serial,
        name,
        model: g.model_ref(instance.model.0)?,
        rotation,
        translation,
        scale: instance.scale.into(),
        has_hull: entity.mechanism.0.is_some(),
        lightmap: g.lightmap(instance_object),
    })
}

fn sky(
    g: &Graph<'_>,
    o: &Object,
    record: Option<&Object>,
    serial: u32,
    name: String,
) -> Result<Sky> {
    let entity: SkyEntity = g.typed(o, "a sky entity")?;
    let what = format!("the cube renderable of sky {}", o.id);
    let cube: CubeRenderable = g.typed(g.object(entity.cube.0, &what)?, &what)?;
    let viewer_value = match entity.viewer.0 {
        Some(id) => {
            let what = format!("the viewer renderable of sky {}", o.id);
            let viewer: ViewerRenderable = g.typed(g.object(Some(id), &what)?, &what)?;
            Some(viewer.value)
        }
        None => None,
    };
    let record: Option<SkyRecord> = record
        .map(|r| g.typed(r, &format!("the property record of sky {}", o.id)))
        .transpose()?;
    let texture = |r: &ObjectRef| r.0.and_then(|id| g.external_path(id)).map(str::to_string);
    let faces = [
        &cube.north,
        &cube.west,
        &cube.south,
        &cube.east,
        &cube.floor,
        &cube.ceiling,
    ]
    .map(texture);
    let (rotation, translation) = entity.placement.split();
    Ok(match record {
        Some(r) => Sky {
            serial,
            name,
            faces,
            texture: texture(&r.texture),
            color: r.color,
            size: r.size,
            viewer_y: r.viewer_y,
            flag_4: r.flag_4,
            flag_5: Some(r.flag_5),
            viewer_value,
            rotation,
            translation,
        },
        None => Sky {
            serial,
            name,
            faces,
            texture: None,
            color: cube.color,
            size: cube.size,
            viewer_y: entity.viewer_y,
            flag_4: cube.flag,
            flag_5: None,
            viewer_value,
            rotation,
            translation,
        },
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
