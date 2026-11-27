//! Loading a level: a `.wld` and every model, material and texture it uses, turned into a
//! format-neutral [`LevelScene`] (RFC 0013, roadmap T07-028).
//!
//! [`load_level`] reads the world (`ss2-format-wld`), then each distinct model once (`.mdl`, and
//! the external `.bmf` mesh of the few models that have one), the materials their surfaces name
//! (embedded presets or external `.mtr` files) and finally each distinct texture once (`.tex`).
//! Only the world itself can fail the load: anything else that cannot be read is left out or
//! replaced and recorded as one line in [`LevelScene::skipped`]:
//!
//! - a model that is missing, broken, has no mesh or no level of detail for the PC, or is
//!   embedded in the world (`wld-geometry.md` leaves the 109 embedded models unanalysed): its
//!   placements are dropped;
//! - a material that cannot be read: [`MaterialData::FALLBACK`] (a surface without material
//!   gets it too, without a line);
//! - a texture that cannot be read or converted (a cube map, a block texture whose largest kept
//!   level is not a multiple of 4): a white [`TextureData::solid`], the material is kept.
//!
//! ## What is drawn
//!
//! - **Meshes** (`bmf.md`): the first level of detail whose platform mask includes the PC
//!   ([`ss2_format_bmf::Lod::is_for_pc`]) is drawn; the last one (the coarsest) is the collision
//!   of models that some placement gives a hull, until the real collision mesh is specified
//!   (T01-055). Only a model's first mesh instance is used (`mdl.md`: how the engine uses the
//!   second instance of 7 models is not specified). Surfaces sharing a material are merged into
//!   one [`ModelPart`].
//! - **UVs**: the stream whose UV map id is the one the material's base colour slot names
//!   (`mtr.md`, reader rule 4), else the first stream. Coordinates are kept as authored: `bmf.md`
//!   leaves the orientation of `v` untested, and `tex.md` stores rows top to bottom, which is the
//!   convention of the renderer too, so no flip is applied.
//! - **Materials** (`mtr.md`): the base colour texture ([`ss2_format_mtr::Material::base_color_texture`]);
//!   the alpha mode from the common parameters of the config that supplies it: blend type above
//!   `0x1F4` blends, `0x1F4` with the alpha test switch (`.5`) set masks at `.6` (0.5 by
//!   default), otherwise opaque. Both readings are "medium" in the spec. The tint colours are
//!   not used (their byte order is not confirmed), a material without base colour texture is
//!   mid grey. Every material is double-sided: the front-face winding is not verified
//!   (`bmf.md`) and mirrored placements flip it.
//! - **Textures** (`tex.md`): DXT1/3/5 become BC1/2/3 with their mip chain, the 24- and 32-bit
//!   layouts become RGBA8 (from BGR/BGRA). Top levels are dropped (block formats) or the image
//!   is halved (RGBA8) until both sides fit [`LoadOptions::max_texture_size`]. Wrap code `0x51`
//!   clamps, every other code repeats.
//!
//! ## Sky
//!
//! The world's sky entity (`environment.md`, "Sky", read by [`ss2_format_wld::Sky`]) becomes
//! [`LevelScene::sky`] (T07-033). Each face texture goes through the same texture path as the
//! materials' textures (read once, the size limit, BC kept or RGBA8, a white stand-in and a skipped
//! line when it cannot be read) and lands on its cube face: North `.10` on -Z, West `.11` on -X,
//! South `.12` on +Z, East `.13` on +X, Floor `.14` on -Y, Ceiling `.15` on +Y. The spec's seams
//! put every game face image already in the RFC 0013 orientation convention
//! ([`CubeFace::basis`]), so none is turned; the derivation and the machinery that would turn one
//! are in the private `sky` module. A face the world does not name is `None` and shows the tint.
//! The tint is the stored colour read as `0xAARRGGBB` with the alpha byte ignored (it is 0 in
//! most skies), the red, green and blue bytes taken as sRGB and made linear. The cube's size, the
//! viewer height and the entity's placement are not part of the boundary and are not used.
//!
//! ## Coordinates
//!
//! The game's data is already in the engine's convention (right-handed, Y up, -Z forward), so
//! nothing is converted: `wld-root.md` reads the gravity `(0, -30, 0)` as "y up",
//! `wld-geometry.md` lays the collision grid in the x-z plane "y is up", and the gameplay specs
//! state the forward axis and turning sense outright: "the engine's forward is -z"
//! (`puppet-dummy-foe.md`), "local x to the right, y up, forward is -z" (`spawner-spawn-side.md`),
//! and a heading of 90 degrees turns forward `(0, 0, -1)` into `(-1, 0, 0)`
//! (`puppet-attacks.md`, `puppet-legged-sequences.md`), which is a right-handed rotation about
//! +Y. Units are taken as metres (gravity 30 m/s^2).
//!
//! ## Placements and starts
//!
//! An [`Instance`] takes the world rotation, translation and per-axis scale of its static model
//! (`wld-geometry.md`); a negative scale is kept as it is (the renderer draws both faces). It is
//! collidable when its entity has a hull and the model has collision. The spawn points are the
//! world's spawn markers; a world without any uses its chapter markers instead. How single
//! player and co-op actually choose a start is not specified yet (T01-210). A spawn's yaw is the
//! heading of the marker's rotated -Z.
//!
//! [`load_level`] keeps no state between calls (the caches live in the call) and may run on any
//! thread.

use std::collections::BTreeMap;

use ss2_core::math::{Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_format_bmf::{Lod, MaterialRef, Mesh, Surface};
use ss2_format_mdl::{MeshSource, Model};
use ss2_format_mtr::{Material, MaterialFile, Param, ShaderConfig};
use ss2_format_tex::{PixelFormat, Texture, WRAP_CLAMP};
use ss2_format_wld::{Marker, ModelRef, SkyFace, World};
use ss2_render_api::Transform;
use ss2_render_api::scene_data::{
    AlphaMode, CubeFace, MaterialData, MeshData, MeshVertex, TexelFormat, TextureData,
};
use ss2_vfs::{Vfs, VfsPath};

use crate::scene::{
    CollisionData, Instance, LevelScene, LoadProgress, LoadStage, ModelData, ModelPart, SkyData,
    SpawnPoint,
};

/// How to load a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadOptions {
    /// The largest texture side kept: larger textures drop their top mip levels (or are halved,
    /// for uncompressed ones) until both sides fit. 0 counts as 1.
    pub max_texture_size: u32,
}

impl Default for LoadOptions {
    /// 2048 texels, the largest side of most of the game's textures.
    fn default() -> Self {
        Self {
            max_texture_size: 2048,
        }
    }
}

/// Blend type that disables blending (`mtr.md`, "The blend mechanism").
const BLEND_NONE: u32 = 0x1F4;

/// Load the level whose world is `wld`, reporting progress through `progress`.
///
/// # Errors
/// Only when the `.wld` itself cannot be read or parsed; everything else that fails is recorded
/// in [`LevelScene::skipped`]. The result passes [`LevelScene::validate`].
pub fn load_level(
    vfs: &dyn Vfs,
    wld: &VfsPath,
    options: &LoadOptions,
    progress: &mut dyn FnMut(LoadProgress),
) -> Result<LevelScene> {
    let report = |progress: &mut dyn FnMut(LoadProgress), stage, done: usize, total: usize| {
        let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
        progress(LoadProgress {
            stage,
            done: n(done),
            total: n(total),
        });
    };
    report(progress, LoadStage::World, 0, 1);
    let world = World::from_bytes(&vfs.read(wld)?)?;
    report(progress, LoadStage::World, 1, 1);

    let mut loader = Loader {
        vfs,
        skipped: Vec::new(),
        materials: Vec::new(),
        material_ids: BTreeMap::new(),
        texture_paths: Vec::new(),
        texture_ids: BTreeMap::new(),
        mtr_cache: BTreeMap::new(),
    };

    // The distinct external models in order of first use, with how many placements use each
    // and whether any of them has a hull.
    let mut model_order: Vec<VfsPath> = Vec::new();
    let mut model_use: BTreeMap<VfsPath, (usize, bool)> = BTreeMap::new();
    let mut placement_model: Vec<Option<VfsPath>> = Vec::with_capacity(world.static_models.len());
    for m in &world.static_models {
        let path = match &m.model {
            ModelRef::External(raw) => match VfsPath::new(raw) {
                Ok(p) => Some(p),
                Err(e) => {
                    loader.skip(format!(
                        "static model {} ({}): model path `{raw}`: {e}",
                        m.serial, m.name
                    ));
                    None
                }
            },
            ModelRef::Embedded(id) => {
                loader.skip(format!(
                    "static model {} ({}): embedded model (object {id}) is not read",
                    m.serial, m.name
                ));
                None
            }
        };
        if let Some(p) = &path {
            let entry = model_use.entry(p.clone()).or_insert_with(|| {
                model_order.push(p.clone());
                (0, false)
            });
            entry.0 += 1;
            entry.1 |= m.has_hull;
        }
        placement_model.push(path);
    }

    let mut models: Vec<ModelData> = Vec::new();
    let mut model_index: BTreeMap<VfsPath, u32> = BTreeMap::new();
    report(progress, LoadStage::Models, 0, model_order.len());
    for (i, path) in model_order.iter().enumerate() {
        let (uses, collide) = model_use[path];
        match loader.model(path, collide) {
            Ok(model) => {
                model_index.insert(path.clone(), index_u32(models.len()));
                models.push(model);
            }
            Err(why) => loader.skip(format!(
                "model `{}`: {why} ({uses} placement{} left out)",
                path.as_str(),
                if uses == 1 { "" } else { "s" }
            )),
        }
        report(progress, LoadStage::Models, i + 1, model_order.len());
    }

    let mut instances = Vec::new();
    for (m, path) in world.static_models.iter().zip(&placement_model) {
        let Some(&model) = path.as_ref().and_then(|p| model_index.get(p)) else {
            continue;
        };
        let rotation = m.rotation;
        let length = rotation.length();
        if !(length.is_finite() && length > 1e-6 && m.translation.is_finite())
            || !m.scale.is_finite()
            || m.scale.abs().min_element() == 0.0
        {
            loader.skip(format!(
                "static model {} ({}): unusable placement {:?} {} scale {}",
                m.serial, m.name, m.rotation, m.translation, m.scale
            ));
            continue;
        }
        instances.push(Instance {
            model,
            transform: Transform {
                translation: m.translation,
                rotation: rotation / length,
                scale: m.scale,
            },
            collidable: m.has_hull && models[model as usize].collision.is_some(),
            // Lightmaps are read by T07-035.
            lightmap: None,
        });
    }

    // The sky's faces are read with the other textures; a face whose image needs turning gets
    // its own turned copy after them.
    let mut sky_faces: Vec<(CubeFace, u32, sky::Turn)> = Vec::new();
    if let Some(s) = &world.sky {
        for face in SkyFace::ALL {
            let Some(raw) = s.face(face) else { continue };
            let path = match VfsPath::new(raw) {
                Ok(p) => p,
                Err(e) => {
                    loader.skip(format!("sky {face:?} face: texture path `{raw}`: {e}"));
                    continue;
                }
            };
            let (cube, right, down) = sky::game_face(face);
            let Some(turn) = sky::turn(right, down, cube.basis()) else {
                loader.skip(format!("sky {face:?} face: no turn into {cube:?}"));
                continue;
            };
            sky_faces.push((cube, loader.texture_id(path), turn));
        }
    }

    let total = loader.texture_paths.len();
    report(progress, LoadStage::Textures, 0, total);
    let paths = std::mem::take(&mut loader.texture_paths);
    let mut textures = Vec::with_capacity(total);
    for (i, path) in paths.iter().enumerate() {
        textures.push(loader.texture(path, options.max_texture_size));
        report(progress, LoadStage::Textures, i + 1, total);
    }

    let sky = world.sky.as_ref().map(|s| {
        let mut faces = [None; 6];
        for (cube, id, turn) in sky_faces {
            faces[cube.index()] = Some(if turn == sky::Turn::NONE {
                id
            } else {
                textures.push(sky::orient(textures[id as usize].clone(), turn));
                index_u32(textures.len() - 1)
            });
        }
        SkyData {
            faces,
            tint: sky::tint(s.color),
        }
    });

    let markers = if world.spawn_markers.is_empty() {
        &world.chapters
    } else {
        &world.spawn_markers
    };
    let spawns = markers
        .iter()
        .filter_map(|marker| loader.spawn(marker))
        .collect();

    let name = wld
        .file_name()
        .map(|n| n.rsplit_once('.').map_or(n, |(stem, _)| stem))
        .unwrap_or_default()
        .to_string();
    let scene = LevelScene {
        name,
        gravity: if world.gravity.is_finite() {
            world.gravity
        } else {
            loader.skip(format!(
                "gravity {} is not finite, using (0, -30, 0)",
                world.gravity
            ));
            Vec3::new(0.0, -30.0, 0.0)
        },
        models,
        materials: loader.materials,
        textures,
        instances,
        spawns,
        skipped: loader.skipped,
        sky,
    };
    debug_assert!(scene.validate().is_ok(), "{:?}", scene.validate());
    Ok(scene)
}

/// The heading of a rotation: the angle about +Y of its rotated -Z, 0 when it looks down -Z and
/// positive towards -X. A rotation looking straight up or down has heading 0.
fn yaw_of(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::NEG_Z;
    if forward.x.abs() < 1e-6 && forward.z.abs() < 1e-6 {
        return 0.0;
    }
    (-forward.x).atan2(-forward.z)
}

/// `n` as a `u32` index (saturating; the scene's own lists never come near the limit).
fn index_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A material's identity for de-duplication: texture, alpha mode and colour, as bits.
type MaterialKey = (Option<u32>, u8, u32, [u32; 4], bool);

fn material_key(m: &MaterialData) -> MaterialKey {
    let (kind, cutoff) = match m.alpha {
        AlphaMode::Opaque => (0, 0),
        AlphaMode::Mask { cutoff } => (1, cutoff.to_bits()),
        AlphaMode::Blend => (2, 0),
    };
    (
        m.base_texture,
        kind,
        cutoff,
        m.base_color.map(f32::to_bits),
        m.double_sided,
    )
}

/// The state of one load: what was skipped and the de-duplicated materials and textures.
struct Loader<'a> {
    vfs: &'a dyn Vfs,
    skipped: Vec<String>,
    materials: Vec<MaterialData>,
    material_ids: BTreeMap<MaterialKey, u32>,
    /// Distinct textures in order of first use; read after the models.
    texture_paths: Vec<VfsPath>,
    texture_ids: BTreeMap<VfsPath, u32>,
    /// External `.mtr` files by path (`None`: could not be read, already recorded).
    mtr_cache: BTreeMap<VfsPath, Option<Material>>,
}

impl Loader<'_> {
    fn skip(&mut self, line: String) {
        self.skipped.push(line);
    }

    /// Read one model and build its drawable parts (and collision when `collide`).
    fn model(&mut self, path: &VfsPath, collide: bool) -> std::result::Result<ModelData, String> {
        let bytes = self.vfs.read(path).map_err(|e| e.to_string())?;
        let model = Model::from_bytes(&bytes).map_err(|e| e.to_string())?;
        match model.mesh() {
            MeshSource::Embedded(mesh) => self.build(path, mesh, collide, &|id| model.material(id)),
            MeshSource::External(raw) => {
                let bmf = VfsPath::new(raw).map_err(|e| format!("mesh path `{raw}`: {e}"))?;
                let bytes = self
                    .vfs
                    .read(&bmf)
                    .map_err(|e| format!("mesh `{}`: {e}", bmf.as_str()))?;
                let (mesh, materials, problems) =
                    read_bmf(&bytes).map_err(|e| format!("mesh `{}`: {e}", bmf.as_str()))?;
                for p in problems {
                    self.skip(format!(
                        "mesh `{}`: {p} (drawn with the grey material)",
                        bmf.as_str()
                    ));
                }
                let lookup = |id: u32| {
                    materials
                        .binary_search_by_key(&id, |m| m.id)
                        .ok()
                        .map(|i| &materials[i])
                };
                self.build(path, &mesh, collide, &lookup)
            }
            MeshSource::None => Err("the model has no mesh".to_string()),
        }
    }

    /// The parts (and collision) of `mesh`; `embedded` finds the presets stored with the mesh.
    fn build<'m>(
        &mut self,
        path: &VfsPath,
        mesh: &Mesh,
        collide: bool,
        embedded: &dyn Fn(u32) -> Option<&'m Material>,
    ) -> std::result::Result<ModelData, String> {
        let lod = mesh
            .lods
            .iter()
            .find(|l| l.is_for_pc())
            .ok_or("no level of detail is for the PC")?;
        let mut parts: Vec<ModelPart> = Vec::new();
        let mut part_of: BTreeMap<u32, usize> = BTreeMap::new();
        for surface in &lod.surfaces {
            let material = match &surface.material {
                MaterialRef::None => None,
                MaterialRef::Embedded(id) => embedded(*id).cloned(),
                MaterialRef::External(raw) => self.mtr(raw, path),
            };
            let (material, uv_map) = self.material(material.as_ref());
            let i = *part_of.entry(material).or_insert_with(|| {
                parts.push(ModelPart {
                    mesh: MeshData::default(),
                    material,
                });
                parts.len() - 1
            });
            append_surface(&mut parts[i].mesh, surface, uv_map);
        }
        let mut kept = Vec::with_capacity(parts.len());
        for part in parts {
            match part.mesh.validate() {
                Ok(()) if !part.mesh.indices.is_empty() => kept.push(part),
                Ok(()) => {}
                Err(e) => self.skip(format!(
                    "model `{}`: a part is left out: {e}",
                    path.as_str()
                )),
            }
        }
        if kept.is_empty() {
            return Err("nothing to draw".to_string());
        }
        let collision = if collide {
            let coarsest = mesh
                .lods
                .iter()
                .rev()
                .find(|l| l.is_for_pc())
                .unwrap_or(lod);
            let c = collision_of(coarsest);
            if c.positions.iter().all(|p| p.is_finite()) {
                Some(c)
            } else {
                self.skip(format!(
                    "model `{}`: collision has non-finite positions, it does not collide",
                    path.as_str()
                ));
                None
            }
        } else {
            None
        };
        Ok(ModelData {
            name: path.as_str().to_string(),
            parts: kept,
            collision,
        })
    }

    /// The preset of an external `.mtr` file (read once per load).
    fn mtr(&mut self, raw: &str, user: &VfsPath) -> Option<Material> {
        let path = match VfsPath::new(raw) {
            Ok(p) => p,
            Err(e) => {
                self.skip(format!(
                    "material `{raw}` (used by `{}`): {e}",
                    user.as_str()
                ));
                return None;
            }
        };
        if let Some(m) = self.mtr_cache.get(&path) {
            return m.clone();
        }
        let read = self
            .vfs
            .read(&path)
            .and_then(|b| MaterialFile::from_bytes(&b))
            .and_then(|f| {
                // `mtr.md`: a `.mtr` file holds exactly one preset.
                f.presets
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::invalid(".mtr material", "the file has no preset"))
            });
        let material = match read {
            Ok(m) => Some(m),
            Err(e) => {
                self.skip(format!(
                    "material `{}` (used by `{}`): {e} (drawn with the grey material)",
                    path.as_str(),
                    user.as_str()
                ));
                None
            }
        };
        self.mtr_cache.insert(path, material.clone());
        material
    }

    /// The scene material for a preset (de-duplicated) and the UV map id of its base colour.
    fn material(&mut self, material: Option<&Material>) -> (u32, Option<u32>) {
        let Some(m) = material else {
            return (self.intern(MaterialData::FALLBACK), None);
        };
        let pc: Vec<&ShaderConfig> = m
            .configs
            .iter()
            .filter(|c| c.platform_mask & 0x1 != 0)
            .collect();
        let config = pc
            .iter()
            .find(|c| c.base_color_texture().is_some())
            .or(pc.first())
            .copied()
            .or(m.configs.first());
        let Some(config) = config else {
            return (self.intern(MaterialData::FALLBACK), None);
        };
        let uv_map = config
            .base_color_slot()
            .and_then(|slot| config.uv_map_of(slot));
        let texture = m
            .base_color_texture()
            .and_then(|raw| match VfsPath::new(raw) {
                Ok(p) => Some(self.texture_id(p)),
                Err(e) => {
                    self.skip(format!("texture `{raw}`: {e}"));
                    None
                }
            });
        let data = MaterialData {
            base_color: if texture.is_some() {
                [1.0; 4]
            } else {
                MaterialData::FALLBACK.base_color
            },
            base_texture: texture,
            alpha: alpha_of(config),
            double_sided: true,
        };
        (self.intern(data), uv_map)
    }

    fn intern(&mut self, data: MaterialData) -> u32 {
        let next = index_u32(self.materials.len());
        let id = *self.material_ids.entry(material_key(&data)).or_insert(next);
        if id == next {
            self.materials.push(data);
        }
        id
    }

    fn texture_id(&mut self, path: VfsPath) -> u32 {
        let next = index_u32(self.texture_paths.len());
        *self.texture_ids.entry(path.clone()).or_insert_with(|| {
            self.texture_paths.push(path);
            next
        })
    }

    /// Read and convert one texture; white on failure.
    fn texture(&mut self, path: &VfsPath, max: u32) -> TextureData {
        let read = self
            .vfs
            .read(path)
            .and_then(|b| Texture::from_bytes(&b))
            .map_err(|e| e.to_string())
            .and_then(|t| texture_data(&t, max));
        read.unwrap_or_else(|why| {
            self.skip(format!("texture `{}`: {why} (drawn white)", path.as_str()));
            TextureData::solid([255; 4])
        })
    }

    fn spawn(&mut self, marker: &Marker) -> Option<SpawnPoint> {
        let yaw = yaw_of(marker.rotation.normalize());
        if marker.translation.is_finite() && yaw.is_finite() {
            Some(SpawnPoint {
                position: marker.translation,
                yaw,
            })
        } else {
            self.skip(format!(
                "marker {} ({}): unusable placement {:?} {}",
                marker.serial, marker.name, marker.rotation, marker.translation
            ));
            None
        }
    }
}

/// The alpha mode of a config (`mtr.md`, "Common parameters"): a blend type above `0x1F4`
/// blends; otherwise the alpha test switch `.5` masks at `.6`.
fn alpha_of(config: &ShaderConfig) -> AlphaMode {
    if let Some(Param::BlendType(b)) = config.common_param(1)
        && *b > BLEND_NONE
    {
        return AlphaMode::Blend;
    }
    if let Some(Param::Bool(true)) = config.common_param(5) {
        let cutoff = match config.common_param(6) {
            Some(Param::Float(f)) if f.is_finite() => f.clamp(0.0, 1.0),
            _ => 0.5,
        };
        return AlphaMode::Mask { cutoff };
    }
    AlphaMode::Opaque
}

/// Append one surface's triangles to `mesh`, taking the UV stream with map id `uv_map` (else the
/// first). Non-finite normals and UVs are replaced (up, zero); positions are left for
/// validation.
fn append_surface(mesh: &mut MeshData, surface: &Surface, uv_map: Option<u32>) {
    let base = index_u32(mesh.vertices.len());
    let stream = uv_map
        .and_then(|id| surface.uv_map_ids.iter().position(|m| *m == id))
        .filter(|i| *i < surface.uv_sets.len())
        .unwrap_or(0);
    let uvs = surface.uv_sets.get(stream);
    for (i, position) in surface.positions.iter().enumerate() {
        let normal = surface
            .normals
            .get(i)
            .copied()
            .filter(|n| n.iter().all(|c| c.is_finite()))
            .unwrap_or([0.0, 1.0, 0.0]);
        let uv = uvs
            .and_then(|u| u.get(i))
            .copied()
            .filter(|u| u.iter().all(|c| c.is_finite()))
            .unwrap_or([0.0, 0.0]);
        mesh.vertices.push(MeshVertex {
            position: *position,
            normal,
            uv,
        });
    }
    mesh.indices
        .extend(surface.indices.iter().map(|i| base + u32::from(*i)));
}

/// All triangles of one level of detail as collision.
fn collision_of(lod: &Lod) -> CollisionData {
    let mut c = CollisionData::default();
    for s in &lod.surfaces {
        let base = index_u32(c.positions.len());
        c.positions
            .extend(s.positions.iter().map(|p| Vec3::from(*p)));
        c.indices
            .extend(s.indices.iter().map(|i| base + u32::from(*i)));
    }
    c
}

/// Read a `.bmf` file: its mesh, the presets embedded in it that its surfaces name (sorted by
/// id), and a line for each preset that could not be read.
fn read_bmf(bytes: &[u8]) -> Result<(Mesh, Vec<Material>, Vec<String>)> {
    let container = ss2_meta::Container::parse(bytes)?;
    if container.header.format_version != 8 {
        return Err(Error::invalid(
            ".bmf mesh",
            format!("format version {}", container.header.format_version),
        ));
    }
    let decoded = container.decode_objects()?;
    let id = decoded
        .objects
        .iter()
        .find(|o| decoded.types.type_name(o.type_id) == Some("CMesh"))
        .map(|o| o.id)
        .ok_or_else(|| Error::invalid(".bmf mesh", "the file has no CMesh object"))?;
    let mesh = Mesh::from_object(&container, &decoded, id)?;
    let mut ids: Vec<u32> = mesh
        .lods
        .iter()
        .flat_map(|l| &l.surfaces)
        .filter_map(|s| match s.material {
            MaterialRef::Embedded(id) => Some(id),
            _ => None,
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let mut materials = Vec::with_capacity(ids.len());
    let mut problems = Vec::new();
    for id in ids {
        match Material::from_preset(&container, &decoded, id) {
            Ok(m) => materials.push(m),
            Err(e) => problems.push(format!("preset {id}: {e}")),
        }
    }
    Ok((mesh, materials, problems))
}

/// Convert a decoded `.tex` into [`TextureData`], keeping both sides at most `max` (0 counts as
/// 1). Errors say why it cannot be used.
fn texture_data(t: &Texture, max: u32) -> std::result::Result<TextureData, String> {
    if t.is_cube_map() {
        return Err("a cube map (not supported)".to_string());
    }
    let max = max.max(1);
    let size = |l: u32| ((t.width >> l).max(1), (t.height >> l).max(1));
    let level = |l: u32| {
        t.level(0, l)
            .map(|m| m.data)
            .ok_or_else(|| format!("mip level {l} is missing"))
    };
    let (format, mips, (width, height)) = match t.format {
        PixelFormat::Dxt1 | PixelFormat::Dxt3 | PixelFormat::Dxt5 => {
            let format = match t.format {
                PixelFormat::Dxt1 => TexelFormat::Bc1,
                PixelFormat::Dxt3 => TexelFormat::Bc2,
                _ => TexelFormat::Bc3,
            };
            let first = (0..t.mip_levels)
                .find(|l| {
                    let (w, h) = size(*l);
                    w <= max && h <= max
                })
                .unwrap_or(t.mip_levels.saturating_sub(1));
            let (w, h) = size(first);
            if !(w.is_multiple_of(4) && h.is_multiple_of(4)) {
                return Err(format!(
                    "a {w} x {h} block-compressed level is not a multiple of 4"
                ));
            }
            let mips = (first..t.mip_levels)
                .map(|l| level(l).map(<[u8]>::to_vec))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            (format, mips, (w, h))
        }
        PixelFormat::Bgr8 | PixelFormat::Bgra8 => {
            let data = level(0)?;
            let mut rgba: Vec<u8> = match t.format {
                PixelFormat::Bgr8 => data
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .flat_map(|[b, g, r]| [*r, *g, *b, 255])
                    .collect(),
                _ => data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|[b, g, r, a]| [*r, *g, *b, *a])
                    .collect(),
            };
            let (mut w, mut h) = (t.width, t.height);
            while w > max || h > max {
                (rgba, w, h) = halve(&rgba, w, h);
            }
            (TexelFormat::Rgba8, vec![rgba], (w, h))
        }
    };
    let out = TextureData {
        width,
        height,
        format,
        mips,
        repeat_u: t.wrap_u != WRAP_CLAMP,
        repeat_v: t.wrap_v != WRAP_CLAMP,
    };
    out.validate().map_err(|e| e.to_string())?;
    Ok(out)
}

/// Halve an RGBA8 image (a side of 1 stays 1), averaging each 2 x 2 (or 2 x 1) block.
fn halve(rgba: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let (sx, sy) = (if w > 1 { 2 } else { 1 }, if h > 1 { 2 } else { 1 });
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        for x in 0..nw {
            for c in 0..4 {
                let mut sum = 0u32;
                for dy in 0..sy {
                    for dx in 0..sx {
                        let (px, py) = (x * sx + dx, y * sy + dy);
                        sum += u32::from(rgba[((py * w + px) * 4 + c) as usize]);
                    }
                }
                let n = sx * sy;
                out.push(((sum + n / 2) / n) as u8);
            }
        }
    }
    (out, nw, nh)
}

mod sky;

#[cfg(test)]
mod tests;
