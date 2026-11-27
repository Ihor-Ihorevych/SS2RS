//! ss2-format-mdl — a minimal `.mdl` model reader (roadmap T02-036; spec `docs/re/mdl.md`).
//!
//! A `.mdl` is a CTSEMETA file with one `CModelConfiguration`: a bounding box, three distances,
//! a list of **mesh instances** (each an embedded `CMesh`, an external `.bmf`, or null), and
//! optional skeleton, animation host, attachment children, physics, destruction and custom data.
//!
//! This reader takes only what a static renderer needs: the bounding box, the distances, the
//! meshes (embedded ones read by [`ss2_format_bmf::Mesh::from_object`]) and the **material
//! presets** the embedded meshes' surfaces use (read by [`ss2_format_mtr::Material::from_preset`]).
//! Skeleton, animation, attachment children, physics, destruction, custom data and the editor
//! records after `EDOB` are not read.
//!
//! ```ignore
//! let model = ss2_format_mdl::Model::from_bytes(&bytes)?;
//! match model.mesh() {
//!     MeshSource::Embedded(mesh) => {
//!         for surface in &mesh.lods[0].surfaces {
//!             // surface.indices, surface.positions, surface.normals, surface.uv_sets ...
//!             let texture = model.material_of(surface).and_then(|m| m.base_color_texture());
//!         }
//!     }
//!     MeshSource::External(bmf_path) => { /* load the .bmf; its materials are in that file */ }
//!     MeshSource::None => {}
//! }
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_format_bmf::{Aabb, MaterialRef, Mesh, Surface};
use ss2_format_mtr::Material;
use ss2_meta::{Container, Decoded, MetaDeserialize, ObjectRef};

const WHAT: &str = ".mdl model";

/// The class of the model's root object.
const CONFIGURATION: &str = "CModelConfiguration";

/// Where the mesh of one mesh instance comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum MeshSource {
    /// A `CMesh` stored inside the model (3,517 instances in the retail data, `mdl.md`). Its
    /// surfaces' [`MaterialRef::Embedded`] ids address [`Model::materials`].
    Embedded(Mesh),
    /// An external `.bmf` file, the path as listed in the model's `RFIL` (127 instances). Its
    /// surfaces' materials live in that file: read it with [`Mesh::from_bytes`].
    External(String),
    /// A null mesh reference (1 instance in the retail data).
    None,
}

/// A decoded `.mdl` file, reduced to its static mesh and materials.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// The model's bounding box (`CModelConfiguration.1`).
    pub bounds: Aabb,
    /// `CModelConfiguration.6`, `.7`, `.12`: three distances. One of them scales the bounding
    /// box's half-extent into the default cull distance; which one, and the roles of the other
    /// two, are open (`mdl.md`, "The three distances").
    pub distances: [f32; 3],
    /// One entry per mesh instance (`CModelConfiguration.3`), in file order. Most models have
    /// one, 69 have none and 7 have two; how the engine uses a second instance is not specified.
    pub meshes: Vec<MeshSource>,
    /// The `CShaderPreset`s that the surfaces of the embedded meshes refer to, each once, sorted
    /// by object id ([`Material::id`], the id in [`MaterialRef::Embedded`]). Look one up with
    /// [`Model::material`] or [`Model::material_of`].
    pub materials: Vec<Material>,
}

#[derive(MetaDeserialize)]
#[meta(class = "CModelConfiguration")]
struct RawConfiguration {
    #[meta(name = "1")]
    bounds: RawBox,
    #[meta(name = "3", default)]
    instances: Vec<RawInstance>,
    #[meta(name = "6", default)]
    d6: f32,
    #[meta(name = "7", default)]
    d7: f32,
    #[meta(name = "12", default)]
    d12: f32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CMeshInstance")]
struct RawInstance {
    #[meta(name = "1")]
    mesh: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "Box3f")]
struct RawBox {
    #[meta(name = "1")]
    x0: f32,
    #[meta(name = "2")]
    y0: f32,
    #[meta(name = "3")]
    z0: f32,
    #[meta(name = "4")]
    x1: f32,
    #[meta(name = "5")]
    y1: f32,
    #[meta(name = "6")]
    z1: f32,
}

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

/// The path of external object `id`, if it is one.
fn external_path(container: &Container<'_>, id: u32) -> Option<String> {
    let e = container
        .external_objects
        .iter()
        .find(|e| e.object_id == id)?;
    Some(container.files.get(e.file_ref as usize)?.path.to_string())
}

static NO_MESH: MeshSource = MeshSource::None;

impl Model {
    /// Read a `.mdl` file.
    ///
    /// Follows the reader invariants of `mdl.md`: the file must hold exactly one
    /// `CModelConfiguration` whose `CResource.1` is 1 (helper configurations with −1 are
    /// ignored); every mesh reference is null, external (`EXOB`/`RFIL`) or an object of the file,
    /// and an unknown id is an error; an embedded mesh must pass the checks of `bmf.md`; every
    /// embedded material a surface names must be a readable `CShaderPreset`.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(bad(format!(
                "format version {}",
                container.header.format_version
            )));
        }
        let decoded = container.decode_objects()?;
        let config_id = Self::configuration(&decoded)?;
        let raw: RawConfiguration = decoded.typed(config_id)?;

        let mut meshes = Vec::with_capacity(raw.instances.len());
        for instance in &raw.instances {
            meshes.push(match instance.mesh.0 {
                None => MeshSource::None,
                Some(id) if decoded.object(id).is_some() => {
                    MeshSource::Embedded(Mesh::from_object(&container, &decoded, id)?)
                }
                Some(id) => MeshSource::External(external_path(&container, id).ok_or_else(
                    || {
                        bad(format!(
                            "a mesh instance refers to object {id}, which is neither an object of the file nor an external reference"
                        ))
                    },
                )?),
            });
        }

        let mut ids: Vec<u32> = meshes
            .iter()
            .filter_map(|m| match m {
                MeshSource::Embedded(mesh) => Some(mesh),
                _ => None,
            })
            .flat_map(|mesh| &mesh.lods)
            .flat_map(|lod| &lod.surfaces)
            .filter_map(|s| match s.material {
                MaterialRef::Embedded(id) => Some(id),
                _ => None,
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let materials = ids
            .into_iter()
            .map(|id| Material::from_preset(&container, &decoded, id))
            .collect::<Result<Vec<_>>>()?;

        let b = &raw.bounds;
        Ok(Self {
            bounds: Aabb {
                min: [b.x0, b.y0, b.z0],
                max: [b.x1, b.y1, b.z1],
            },
            distances: [raw.d6, raw.d7, raw.d12],
            meshes,
            materials,
        })
    }

    /// The object id of the one main configuration (`CResource.1` = 1).
    fn configuration(decoded: &Decoded) -> Result<u32> {
        let mut found = None;
        for o in &decoded.objects {
            if decoded.types.type_name(o.type_id) != Some(CONFIGURATION) {
                continue;
            }
            let role = o
                .value
                .as_struct()
                .and_then(|s| decoded.types.member(s, "CResource", "1"))
                .and_then(ss2_meta::Value::as_i64)
                .ok_or_else(|| bad("a model configuration without `CResource.1`"))?;
            if role != 1 {
                continue; // a helper configuration (−1 in the retail data)
            }
            if found.replace(o.id).is_some() {
                return Err(bad("the file has more than one main CModelConfiguration"));
            }
        }
        found.ok_or_else(|| bad("the file has no CModelConfiguration with CResource.1 = 1"))
    }

    /// The first mesh instance's mesh ([`MeshSource::None`] if the model has no instance). Use
    /// [`Model::meshes`] for the rare models with two.
    pub fn mesh(&self) -> &MeshSource {
        self.meshes.first().unwrap_or(&NO_MESH)
    }

    /// The embedded material preset with object id `id`.
    pub fn material(&self, id: u32) -> Option<&Material> {
        self.materials
            .binary_search_by_key(&id, |m| m.id)
            .ok()
            .map(|i| &self.materials[i])
    }

    /// The material of a surface of one of this model's embedded meshes: `Some` for an embedded
    /// preset, `None` for a surface without a material or with an external `.mtr`
    /// ([`MaterialRef::External`]: read that file with `ss2_format_mtr::MaterialFile`).
    pub fn material_of(&self, surface: &Surface) -> Option<&Material> {
        match surface.material {
            MaterialRef::Embedded(id) => self.material(id),
            _ => None,
        }
    }
}

/// The `.mdl` loader: register it with `AssetManager::register_loader` to load [`Model`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct MdlLoader;

impl AssetLoader for MdlLoader {
    type Asset = Model;

    fn extensions(&self) -> &[&str] {
        &["mdl"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Model> {
        Model::from_bytes(&ctx.read()?)
    }
}

#[cfg(test)]
mod tests;
