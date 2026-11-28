//! Loading a level: a `.wld` and every model, material and texture it uses, turned into a
//! format-neutral [`LevelScene`] (RFC 0013, roadmap T07-028).
//!
//! [`load_level`] reads the world (`ss2-format-wld`), then each distinct model once (`.mdl`, and
//! the external `.bmf` mesh of the few models that have one), the materials their surfaces name
//! (embedded presets or external `.mtr` files) and finally each distinct texture once (`.tex`).
//! A model **embedded in the world** (`wld-geometry.md`: a static model may name a
//! `CModelConfiguration` stored in the world instead of an external `.mdl`, 109 placements of 82
//! models in 24 retail worlds; the object has the shape of `mdl.md`) is read by the world reader
//! ([`ss2_format_wld::EmbeddedModel`]) and built exactly like an external one, collision and
//! lightmap included; its [`ModelData::name`] is the world's path and the object id
//! (`<world>#<id>`).
//! Only the world itself can fail the load: anything else that cannot be read is left out or
//! replaced and recorded as one line in [`LevelScene::skipped`]:
//!
//! - a model that is missing, broken, has no mesh or no level of detail for the PC: its
//!   placements are dropped (of the embedded models, the 9 whose `CMesh` has neither a render
//!   nor a collision mesh are refused by `ss2-format-bmf` and get such a line);
//! - a material that cannot be read: [`MaterialData::FALLBACK`] (a surface without material
//!   gets it too, without a line);
//! - a texture that cannot be read or converted (a cube map, a block texture whose largest kept
//!   level is not a multiple of 4): a white [`TextureData::solid`], the material is kept.
//!
//! ## What is drawn
//!
//! - **Meshes** (`bmf.md`): the first level of detail whose platform mask includes the PC
//!   ([`ss2_format_bmf::Lod::is_for_pc`]) is drawn. Models that some placement gives a hull
//!   collide with their own **collision mesh** ([`ss2_format_bmf::CollisionMesh`], `bmf.md`;
//!   `wld-geometry.md`: a `CModelHull` uses the model's collision mesh), in model space like the
//!   render meshes; a model without one (or with an empty one) collides with its last PC level of
//!   detail (the coarsest) instead ([`CollisionData::from_collision_mesh`] tells which). Only a
//!   model's first mesh instance is used (`mdl.md`: how the engine uses the second instance of 7
//!   models is not specified). Surfaces sharing a material are merged into
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
//! ## Lightmaps
//!
//! A static model whose model instance carries an oven modifier (`wld-lighting.md`, "Baked
//! lightmaps", read by [`ss2_format_wld::Lightmap`]) is lit by its own baked texture
//! (T07-035). The modifier's UV map id names none of the mesh's streams, so the lightmap's UVs
//! are chosen from the mesh data by the spec's rule (reader invariant 3, T01-407 and T01-420):
//! of the UV streams of the drawn level of detail, the one whose coordinates all lie in [0, 1]
//! and whose triangles, over all surfaces together, do not overlap (at most 2 % of the rasterized
//! area covered twice). With no such stream, the least-overlapping stream that still lies in
//! [0, 1] is taken; equal overlaps go to a stream no surface's material uses (the UV map of a
//! non-empty texture group of any config), then to the lowest position. Every part of the model
//! gets [`MeshData::lightmap_uvs`] from the chosen stream. With several such streams, the one
//! with the least overlap is taken (T01-420 stage 1); when several tie on it (usually at 0 %),
//! **each lit placement chooses for itself** (T07-045): of the tied streams, the one whose
//! footprint is the most lit in the placement's own lightmap (decoded at mip 0; T01-420 stage 2
//! with T01-421's lit-texel test), and on a further tie a stream no material uses, then the
//! lowest position (stage 3). The model as built carries the stage-3 stream; a placement that
//! chooses another one is pointed at a **variant** of the model: a copy of its [`ModelData`]
//! (same name, parts, materials and collision) whose lightmap UVs come from that stream, added
//! after the other models once per (model, stream). So [`MeshData::lightmap_uvs`] stays one
//! stream per model (RFC 0013), and only the lightmaps of these placements are decoded. A model
//! with no stream in [0, 1] at all gets no lightmap UVs and its lit placements stay unlit with
//! one skipped line (**a departure from the spec**, whose slot-0 fallback samples the lightmap
//! through a tiling channel there: dark bands). This is done only for models that some
//! lit placement uses. A placement then gets [`Instance::lightmap`]: its texture,
//! converted by the same texture path as the others (the size limit, BC kept, RGBA8 from
//! BGR/BGRA, appended after the file textures); a lightmap that cannot be read or converted
//! leaves the placement unlit with a skipped line. Of several oven modifiers only the first is
//! used (the spec does not say how they combine). The UVs are kept as authored, like the base UVs.
//! [`load_level_with_report`] also returns how often each case happened ([`LightmapReport`]).
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
//! most skies), the red, green and blue bytes taken as sRGB and made linear.
//!
//! The cube's placement (T07-039; `environment.md`, "Placement: the cube is rotated, never
//! translated", T01-409, and reader invariant 7) is the sky entity's `en_qvPlacement` **rotation**,
//! applied in full and normalised (44 of the 58 retail skies are turned, all about +Y); its
//! position is never used, the cube stays centred on the viewer. The size is the record's `.2`
//! (`en_fSize`, 64 in 56 skies) and the viewer height the record's `.3` (`en_fViewerY`, -6.66 to
//! 4.52), both as stored: the spec reads `en_fViewerY` as the viewer's vertical offset in the cube,
//! which is what [`SkyData::viewer_height`] holds (positive: the viewer above the centre; the spec
//! leaves the exact combination with the camera open). A rotation that cannot be normalised, a
//! size that is not positive or a height that is not finite is replaced (no turn, 64, 0) with a
//! skipped line.
//!
//! ## Haze
//!
//! Each haze entity of the world (`environment.md`, "Haze", read by [`ss2_format_wld::Haze`])
//! becomes one [`HazeVolume`] of [`LevelScene::hazes`], in the world's entity-list order
//! (T07-042; RFC 0013 amendment 4). The numbers come from the property record (reader invariant
//! 1): the box is the record's size (`.2`, `.3`, `.4`) centred on the haze renderable's world
//! placement, which is how the renderable stores it (its `Box3f` is the size centred on its
//! origin in all 65 retail hazes). The spec measures the camera "in the box's own local, rotated
//! frame" (T01-070), so the volume keeps the renderable's world **rotation** (13 retail hazes are
//! turned, 11 of them without a parent; the world placement of an attached haze already includes
//! its parent's, `wld-geometry.md`), normalised, with the box's corners expressed in that turned
//! frame: `min = R⁻¹·t − size/2`, `max = R⁻¹·t + size/2`. A rotation that cannot be normalised is
//! replaced by none with a skipped line. The colour is the stored `0xAARRGGBB`: red, green and blue
//! taken as sRGB and made linear like the sky's tint, the alpha byte carried as it is (`/ 255`; the
//! engine reads it, what for is open). `fade_margin` is `.5` raised to
//! [`HazeVolume::MIN_FADE_MARGIN`], `near` is `.6` as stored (it may be negative) and `far` is the
//! engine's clamp `max(.6, 0.01, .7)` (reader invariant 8). `.10` is not carried (its role is not
//! established). A haze whose size is not positive or whose numbers are not finite is left out
//! with a skipped line. A haze attached to a moving parent is placed where the world file has it;
//! following the parent is the game's job.
//!
//! ## Bloom
//!
//! Each bloom entity of the world (`environment.md`, "Bloom", T01-417/T01-423/T01-424, read by
//! [`ss2_format_wld::Bloom`]) becomes one [`BloomVolume`] of [`LevelScene::blooms`], in the
//! world's entity-list order (T07-049; RFC 0013 amendment 6). The box and its turn are made as for
//! a haze: the record's size (`.2`, `.3`, `.4`) centred on the bloom renderable's world placement,
//! in the renderable's normalised world rotation (a rotation that cannot be normalised is replaced
//! by none with a skipped line). The colour is the stored `0xAARRGGBB` made linear like a haze's
//! (red, green and blue as sRGB, the alpha byte `/ 255`). The numbers are the record's, through
//! the clamps the engine applies when it copies the record into the renderable (T01-417):
//! `fade_margin` is `.5` raised to 0 and then to [`HazeVolume::MIN_FADE_MARGIN`] (the render-time
//! floor), `strength` `.6` in `[0, 2]`, `ramp` the record's **second** `.21` in `[0, 2]`, `blur`
//! `.8` in `[0, 8]`, `balance` `.13` in `[0.1, 10]` and `high_pass` `.16` in `[0, 1]`. No retail
//! bloom is changed by them. The HDR-only threshold (the record's first `.21`) and the mask
//! texture are not carried. A bloom whose size is not positive or whose numbers are not finite is
//! left out with a skipped line.
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
//! ## Animated models
//!
//! An external `.mdl` (T04-044, RFC 0013 amendment 8) whose drawn level of detail has a
//! **skinned surface** (bone weights, bone indices and a non-empty palette, `bmf.md`
//! "Skinning"), which has a **skeleton** (embedded, or an external `.skl`; `mdl.md` "Skeleton")
//! and whose animation host has an **animation that binds** to that skeleton becomes a
//! [`SkinnedModelData`] instead of a [`ModelData`], and all its placements become
//! [`SkinnedInstance`]s (not [`Instance`]s); every other model is built exactly as before. In
//! detail:
//!
//! - The skeleton is its first level of detail (`skl.md`; `mdl.md`: palette entries are bone
//!   ids of LOD 0), bones in file order with their rest pose, stored inverse bind and name. A
//!   skeleton that cannot be read or used (a `.rsc` skeleton, more than 256 bones) leaves the
//!   model static with a skipped line.
//! - Each part (surfaces merged by material, as for static parts, with the same UV stream rule)
//!   lists the whole skeleton as its bones. A vertex's bone index addresses its surface's palette,
//!   whose entry is a bone id of the skeleton (`mdl.md`, "Bone palettes in models"); its weight is
//!   the stored byte over 255. A palette entry that is no bone of the skeleton is "no bone"
//!   (`mdl.md` reader invariant 4): that weight is dropped (one skipped line per model says how
//!   many), so the vertex follows its other bones or stays at its bind position. Surfaces without
//!   skinning stay at their bind position.
//! - The animations are those of every set of the host (embedded, or external `.ans` files read
//!   once per load), in host order, bound **by bone name** (`ans.md` reader invariant 3): a
//!   track whose name is no bone is skipped, an animation with no bound track is left out. Key
//!   times are seconds since the first frame and one pass lasts `last - first + 1` frames (the
//!   frame count `ans.md` loops by), so the last key is held for one frame before the loop.
//!   Scalar tracks and events are not carried.
//! - The animation a placement plays is the first whose name contains `idle` (ignoring case),
//!   else the first named `default`, else the first. The specs do not say what the original
//!   plays on a placed model; mechanisms (doors, lifts) whose animations are all actions loop
//!   their first one until game logic drives them.
//! - The collision is the static build's, in the bind pose; a placement collides as a static
//!   one would. A lit placement of an animated model is drawn without its lightmap
//!   ([`LightmapReport::unlit_animated`]): the skinned path has none.
//! - Models stored in the world are never animated: their bone and track names would come from
//!   the world's identifier table, which does not hold them at the bones' ids.
//!
//! ## Placements and starts
//!
//! An [`Instance`] takes the world rotation, translation and per-axis scale of its static model
//! (`wld-geometry.md`); a negative scale is kept as it is (the renderer draws both faces). It is
//! collidable when its entity has a hull and the model has collision.
//!
//! The spawn points (T07-040; `chapter-restore-players.md` sections 2.8 and 2.9, T01-210) follow
//! the original's start rule:
//!
//! - **deathmatch**: the world's spawn markers (`CSpawnMarkerEntity`), in entity-list order. Only
//!   deathmatch worlds have any (16 worlds, all `dm_`), so a world with spawn markers is taken as
//!   a deathmatch world; which marker a player gets is the game's choice
//!   (`helper-classes-iterators.md` section 2.5), not the loader's.
//! - **single player and co-op** (every other world): the chapters of the chapter chain, in chain
//!   order, starting at the world's first chapter ([`World::chapter_chain`]: the world info's
//!   `CWorldInfoEntity.2`, then each chapter's next chapter). The first spawn, and so
//!   [`LevelScene::start`], is the chain's first chapter: never the first chapter of the entity
//!   list (they differ in 31 of the 51 worlds with a chain) and never a spawn marker. Chapters off
//!   the chain are left out. Co-op slot offsets are the game's, not the loader's.
//! - a world without spawn markers and without a chain (no first chapter) falls back to its
//!   chapters in entity-list order; without those, [`LevelScene::start`] uses the level's bounds.
//!
//! A spawn's position is the marker's or chapter's world translation and its yaw the heading of
//! its rotated -Z.
//!
//! [`load_level`] keeps no state between calls (the caches live in the call) and may run on any
//! thread.

use std::collections::BTreeMap;

use ss2_core::math::{Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_format_ans::AnimSet;
use ss2_format_bmf::{CollisionMesh, Lod, MaterialRef, Mesh, Surface};
use ss2_format_mdl::{AnimSetSource, MeshSource, Model, SkeletonSource};
use ss2_format_mtr::{Material, MaterialFile, Param, ShaderConfig, TextureRef};
use ss2_format_skl::Skeleton;
use ss2_format_tex::{PixelFormat, Texture, WRAP_CLAMP};
use ss2_format_wld::{Bloom, Haze, Marker, ModelRef, Sky, SkyFace, World};
use ss2_render_api::Transform;
use ss2_render_api::scene_data::{
    AlphaMode, CubeFace, MaterialData, MeshData, MeshVertex, TexelFormat, TextureData,
};
use ss2_vfs::{Vfs, VfsPath};

use crate::scene::{
    BloomVolume, CollisionData, HazeVolume, Instance, LevelScene, LoadProgress, LoadStage,
    ModelData, ModelPart, SkinnedInstance, SkinnedModelData, SkyData, SpawnPoint,
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

/// How the lightmaps of one load went: how many placements and models were lit, and why the
/// others were not (module docs, "Lightmaps").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LightmapReport {
    /// Static models of the world with an oven modifier (lit placements).
    pub lit_placements: usize,
    /// Placements that got a lightmap ([`Instance::lightmap`]).
    pub lightmapped: usize,
    /// Lit placements left unlit because their model has no lightmap UVs (no UV stream in
    /// [0, 1]).
    pub unlit_no_uvs: usize,
    /// Lit placements left unlit because their lightmap could not be read or converted.
    pub unlit_texture: usize,
    /// Lit placements drawn animated, without their lightmap (the skinned path has none; module
    /// docs, "Animated models").
    pub unlit_animated: usize,
    /// Lit placements that are not in the scene at all (their model is missing or broken, or
    /// the placement is unusable).
    pub dropped: usize,
    /// Distinct loaded models that some lit placement uses.
    pub lit_models: usize,
    /// Of those, models with exactly one candidate stream, by the stream's position in the
    /// surfaces' UV streams (slot order).
    pub one_candidate: BTreeMap<usize, usize>,
    /// Lit models with several candidate streams.
    pub several: usize,
    /// Of those, models whose candidates tie on the least overlap: each placement chooses.
    pub several_tied: usize,
    /// Lightmapped placements of models with several candidate streams whose stream the
    /// overlap decided (T01-420 stage 1, the same for the whole model).
    pub several_by_overlap: usize,
    /// Lightmapped placements whose stream their own lightmap's lit fraction decided (stage 2).
    pub several_by_lit_fraction: usize,
    /// Lightmapped placements whose tied streams tie on the lit fraction too: the first in
    /// stage-3 order (a stream no material uses, then the lowest position).
    pub several_by_order: usize,
    /// Model variants added for placements that chose another stream than their model's.
    pub variant_models: usize,
    /// Lit models without a candidate, given the least-overlapping stream in [0, 1].
    pub near_miss: usize,
    /// Of those, models whose stream is not slot 0.
    pub near_miss_not_slot_zero: usize,
    /// Lit models without any UV stream in [0, 1]: no lightmap UVs.
    pub none_in_range: usize,
}

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
    load_level_with_report(vfs, wld, options, progress).map(|(scene, _)| scene)
}

/// [`load_level`], also saying how the lightmaps went.
///
/// # Errors
/// As [`load_level`].
pub fn load_level_with_report(
    vfs: &dyn Vfs,
    wld: &VfsPath,
    options: &LoadOptions,
    progress: &mut dyn FnMut(LoadProgress),
) -> Result<(LevelScene, LightmapReport)> {
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
    Ok(scene_of_world(vfs, wld, &world, options, progress))
}

/// Where a model comes from: one key per distinct model of a load.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ModelKey {
    /// An external `.mdl` file.
    File(VfsPath),
    /// A `CModelConfiguration` stored in the world (its object id).
    Embedded(u32),
}

/// The rest of [`load_level_with_report`], once the world is read.
fn scene_of_world(
    vfs: &dyn Vfs,
    wld: &VfsPath,
    world: &World,
    options: &LoadOptions,
    progress: &mut dyn FnMut(LoadProgress),
) -> (LevelScene, LightmapReport) {
    let report = |progress: &mut dyn FnMut(LoadProgress), stage, done: usize, total: usize| {
        let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
        progress(LoadProgress {
            stage,
            done: n(done),
            total: n(total),
        });
    };
    let mut loader = Loader {
        vfs,
        skipped: Vec::new(),
        materials: Vec::new(),
        material_ids: BTreeMap::new(),
        texture_paths: Vec::new(),
        texture_ids: BTreeMap::new(),
        mtr_cache: BTreeMap::new(),
        skeleton_cache: BTreeMap::new(),
        anim_cache: BTreeMap::new(),
        report: LightmapReport::default(),
    };
    // Lit models with several candidate streams, by scene index: their streams tied on overlap
    // (`None`: overlap decided).
    let mut several: BTreeMap<u32, Option<Tie>> = BTreeMap::new();

    // The distinct models in order of first use, with how many placements use each, whether any
    // of them has a hull and how many of them are lit.
    let mut model_order: Vec<ModelKey> = Vec::new();
    let mut model_use: BTreeMap<ModelKey, (usize, bool, usize)> = BTreeMap::new();
    let mut placement_model: Vec<Option<ModelKey>> = Vec::with_capacity(world.static_models.len());
    for m in &world.static_models {
        let path = match &m.model {
            ModelRef::External(raw) => match VfsPath::new(raw) {
                Ok(p) => Some(ModelKey::File(p)),
                Err(e) => {
                    loader.skip(format!(
                        "static model {} ({}): model path `{raw}`: {e}",
                        m.serial, m.name
                    ));
                    None
                }
            },
            ModelRef::Embedded(id) => Some(ModelKey::Embedded(*id)),
        };
        if let Some(p) = &path {
            let entry = model_use.entry(p.clone()).or_insert_with(|| {
                model_order.push(p.clone());
                (0, false, 0)
            });
            entry.0 += 1;
            entry.1 |= m.has_hull;
            entry.2 += usize::from(m.lightmap.is_some());
        }
        placement_model.push(path);
    }

    let mut models: Vec<ModelData> = Vec::new();
    let mut model_index: BTreeMap<ModelKey, u32> = BTreeMap::new();
    let mut skinned_models: Vec<SkinnedModelData> = Vec::new();
    let mut skinned_index: BTreeMap<ModelKey, u32> = BTreeMap::new();
    report(progress, LoadStage::Models, 0, model_order.len());
    for (i, key) in model_order.iter().enumerate() {
        let (uses, collide, lit) = model_use[key];
        let (label, built) = match key {
            ModelKey::File(path) => (
                format!("`{}`", path.as_str()),
                loader.model(path, collide, lit),
            ),
            ModelKey::Embedded(id) => {
                let name = format!("{}#{id}", wld.as_str());
                let built = match world.embedded_model(*id).map(|e| &e.model) {
                    // A model stored in the world names its bones through the world's
                    // identifier table, so it is drawn static (module docs, "Animated models").
                    Some(Ok(model)) => loader.model_parts(&name, model, collide, lit, false),
                    Some(Err(why)) => Err(why.clone()),
                    None => Err("the world did not read it".to_string()),
                };
                (format!("`{name}` (embedded model {id})"), built)
            }
        };
        match built {
            Ok(Built {
                skinned: Some(skinned),
                ..
            }) => {
                skinned_index.insert(key.clone(), index_u32(skinned_models.len()));
                skinned_models.push(skinned);
            }
            Ok(Built {
                model, several: s, ..
            }) => {
                let index = index_u32(models.len());
                model_index.insert(key.clone(), index);
                models.push(model);
                if let Some(tie) = s {
                    several.insert(index, tie);
                }
            }
            Err(why) => loader.skip(format!(
                "model {label}: {why} ({uses} placement{} left out)",
                if uses == 1 { "" } else { "s" }
            )),
        }
        report(progress, LoadStage::Models, i + 1, model_order.len());
    }

    loader.report.lit_placements = world
        .static_models
        .iter()
        .filter(|m| m.lightmap.is_some())
        .count();
    let mut instances = Vec::new();
    let mut skinned_instances = Vec::new();
    // The static model of each instance, for its lightmap.
    let mut instance_source = Vec::new();
    for (m, path) in world.static_models.iter().zip(&placement_model) {
        let skinned = path.as_ref().and_then(|p| skinned_index.get(p)).copied();
        let Some(model) = path
            .as_ref()
            .and_then(|p| model_index.get(p))
            .copied()
            .or(skinned)
        else {
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
        let transform = Transform {
            translation: m.translation,
            rotation: rotation / length,
            scale: m.scale,
        };
        if skinned.is_some() {
            loader.report.unlit_animated += usize::from(m.lightmap.is_some());
            skinned_instances.push(SkinnedInstance {
                model,
                transform,
                collidable: m.has_hull && skinned_models[model as usize].collision.is_some(),
            });
            continue;
        }
        instances.push(Instance {
            model,
            transform,
            collidable: m.has_hull && models[model as usize].collision.is_some(),
            // Set after the textures, below.
            lightmap: None,
        });
        instance_source.push(m);
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

    // Each lit placement's own lightmap, after the file textures.
    for (instance, m) in instances.iter_mut().zip(&instance_source) {
        let Some(lm) = &m.lightmap else { continue };
        let model = &models[instance.model as usize];
        if model.parts.iter().any(|p| p.mesh.lightmap_uvs.is_none()) {
            loader.report.unlit_no_uvs += 1;
            continue;
        }
        match lm
            .texture
            .as_ref()
            .map_err(String::clone)
            .and_then(|t| texture_data(t, options.max_texture_size).map(|d| (t, d)))
        {
            Ok((t, data)) => {
                textures.push(data);
                instance.lightmap = Some(index_u32(textures.len() - 1));
                loader.report.lightmapped += 1;
                match several.get_mut(&instance.model) {
                    None => {}
                    Some(None) => loader.report.several_by_overlap += 1,
                    Some(Some(tie)) => {
                        let label = format!("static model {} ({})", m.serial, m.name);
                        let chosen = loader.choose_stream(tie, t, &label);
                        if let Some((parts, index)) =
                            chosen.checked_sub(1).map(|i| &mut tie.variants[i])
                        {
                            let base = instance.model as usize;
                            instance.model = *index.get_or_insert_with(|| {
                                models.push(ModelData {
                                    name: models[base].name.clone(),
                                    parts: std::mem::take(parts),
                                    collision: models[base].collision.clone(),
                                });
                                loader.report.variant_models += 1;
                                index_u32(models.len() - 1)
                            });
                        }
                    }
                }
            }
            Err(why) => {
                loader.report.unlit_texture += 1;
                loader.skip(format!(
                    "static model {} ({}): lightmap (oven modifier {}): {why} (drawn unlit)",
                    m.serial, m.name, lm.modifier
                ));
            }
        }
    }
    let report = &mut loader.report;
    report.dropped = report.lit_placements
        - report.lightmapped
        - report.unlit_no_uvs
        - report.unlit_texture
        - report.unlit_animated;

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
        let (rotation, size, viewer_height) = sky_placement(s, &mut loader.skipped);
        SkyData {
            faces,
            tint: sky::tint(s.color),
            rotation,
            size,
            viewer_height,
        }
    });

    let hazes = world
        .hazes
        .iter()
        .filter_map(|h| haze_volume(h, &mut loader.skipped))
        .collect();
    let blooms = world
        .blooms
        .iter()
        .filter_map(|b| bloom_volume(b, &mut loader.skipped))
        .collect();

    let spawns = start_markers(world)
        .into_iter()
        .filter_map(|marker| loader.spawn(marker))
        .collect();

    let name = wld
        .file_name()
        .map(|n| n.rsplit_once('.').map_or(n, |(stem, _)| stem))
        .unwrap_or_default()
        .to_string();
    let scene = LevelScene {
        hazes,
        blooms,
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
        skinned_models,
        skinned_instances,
    };
    debug_assert!(scene.validate().is_ok(), "{:?}", scene.validate());
    (scene, loader.report)
}

/// The scene's haze volume of one haze entity (module docs, "Haze"); `None` with a line in
/// `skipped` when its numbers cannot be used.
fn haze_volume(h: &Haze, skipped: &mut Vec<String>) -> Option<HazeVolume> {
    let length = h.rotation.length();
    let rotation = if length.is_finite() && length > 1e-6 {
        h.rotation / length
    } else {
        skipped.push(format!(
            "haze {} ({}): rotation {:?} cannot be normalised, not turned",
            h.serial, h.name, h.rotation
        ));
        Quat::IDENTITY
    };
    if !(h.size.is_finite() && h.size.min_element() > 0.0) || !h.translation.is_finite() {
        skipped.push(format!(
            "haze {} ({}): unusable box, size {} at {}",
            h.serial, h.name, h.size, h.translation
        ));
        return None;
    }
    let centre = rotation.inverse() * h.translation;
    let channel = |shift: u32| ((h.color >> shift) & 0xFF) as f32 / 255.0;
    let volume = HazeVolume {
        min: centre - h.size / 2.0,
        max: centre + h.size / 2.0,
        rotation,
        colour: [
            sky::srgb_to_linear(channel(16)),
            sky::srgb_to_linear(channel(8)),
            sky::srgb_to_linear(channel(0)),
            channel(24),
        ],
        fade_margin: h.fade.max(HazeVolume::MIN_FADE_MARGIN),
        near: h.near,
        far: h.near.max(0.01).max(h.far),
    };
    if [h.fade, h.near, h.far].iter().all(|v| v.is_finite()) {
        Some(volume)
    } else {
        skipped.push(format!(
            "haze {} ({}): fade {}, near {}, far {} are not all finite",
            h.serial, h.name, h.fade, h.near, h.far
        ));
        None
    }
}

/// The scene's bloom volume of one bloom entity (module docs, "Bloom"); `None` with a line in
/// `skipped` when its numbers cannot be used.
fn bloom_volume(b: &Bloom, skipped: &mut Vec<String>) -> Option<BloomVolume> {
    let length = b.rotation.length();
    let rotation = if length.is_finite() && length > 1e-6 {
        b.rotation / length
    } else {
        skipped.push(format!(
            "bloom {} ({}): rotation {:?} cannot be normalised, not turned",
            b.serial, b.name, b.rotation
        ));
        Quat::IDENTITY
    };
    if !(b.size.is_finite() && b.size.min_element() > 0.0) || !b.translation.is_finite() {
        skipped.push(format!(
            "bloom {} ({}): unusable box, size {} at {}",
            b.serial, b.name, b.size, b.translation
        ));
        return None;
    }
    let numbers = [b.fade, b.strength, b.ramp, b.blur, b.balance, b.high_pass];
    if !numbers.iter().all(|v| v.is_finite()) {
        skipped.push(format!(
            "bloom {} ({}): fade, strength, ramp, blur, balance, high-pass {numbers:?} are not \
             all finite",
            b.serial, b.name
        ));
        return None;
    }
    let centre = rotation.inverse() * b.translation;
    let channel = |shift: u32| ((b.color >> shift) & 0xFF) as f32 / 255.0;
    Some(BloomVolume {
        min: centre - b.size / 2.0,
        max: centre + b.size / 2.0,
        rotation,
        colour: [
            sky::srgb_to_linear(channel(16)),
            sky::srgb_to_linear(channel(8)),
            sky::srgb_to_linear(channel(0)),
            channel(24),
        ],
        fade_margin: b.fade.max(0.0).max(HazeVolume::MIN_FADE_MARGIN),
        strength: b.strength.clamp(0.0, 2.0),
        blur: b.blur.clamp(0.0, 8.0),
        balance: b.balance.clamp(0.1, 10.0),
        high_pass: b.high_pass.clamp(0.0, 1.0),
        ramp: b.ramp.clamp(0.0, 2.0),
    })
}

/// The placement of the sky cube (`environment.md`, T01-409): the entity's rotation normalised
/// (its position is never used), the size and the viewer height as stored. A value that cannot be
/// used is replaced by the unplaced one with a line in `skipped`.
fn sky_placement(s: &Sky, skipped: &mut Vec<String>) -> (Quat, f32, f32) {
    let length = s.rotation.length();
    let rotation = if length.is_finite() && length > 1e-6 {
        s.rotation / length
    } else {
        skipped.push(format!(
            "sky: rotation {:?} cannot be normalised, not turned",
            s.rotation
        ));
        Quat::IDENTITY
    };
    let size = if s.size.is_finite() && s.size > 0.0 {
        s.size
    } else {
        skipped.push(format!(
            "sky: size {} is not positive, using {}",
            s.size,
            SkyData::UNPLACED_SIZE
        ));
        SkyData::UNPLACED_SIZE
    };
    let viewer_height = if s.viewer_y.is_finite() {
        s.viewer_y
    } else {
        skipped.push(format!(
            "sky: viewer height {} is not finite, using 0",
            s.viewer_y
        ));
        0.0
    };
    (rotation, size, viewer_height)
}

/// The markers the spawn points come from, first the start (`chapter-restore-players.md`
/// sections 2.8 and 2.9): a deathmatch world's spawn markers; otherwise the chapter chain from the
/// world's first chapter; without a chain, the chapters in entity-list order.
fn start_markers(world: &World) -> Vec<&Marker> {
    if !world.spawn_markers.is_empty() {
        return world.spawn_markers.iter().collect();
    }
    let chain = world.chapter_chain();
    if chain.is_empty() {
        world.chapters.iter().collect()
    } else {
        chain.into_iter().map(|i| &world.chapters[i]).collect()
    }
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

/// A built model and, when it is lit with several candidate streams, `Some` with the streams
/// tied on overlap (`Some(None)`: the overlap decided).
struct Built {
    model: ModelData,
    several: Option<Option<Tie>>,
    /// The model drawn animated, when it is one (module docs, "Animated models"); its placements
    /// then become skinned instances and `model` is not used.
    skinned: Option<SkinnedModelData>,
}

/// A lit model whose candidate streams tie on overlap: each lit placement chooses among them by
/// its own lightmap (module docs, "Lightmaps").
struct Tie {
    /// The tied streams' positions in stage-3 order; the model as built uses the first.
    streams: Vec<usize>,
    /// The surfaces of the drawn level of detail, for the lit fractions.
    surfaces: Vec<Surface>,
    /// For each later stream (`variants[i]` for `streams[i + 1]`), the model's parts with that
    /// stream's lightmap UVs, and the scene index of the variant once a placement chose it
    /// (the parts are then moved into it).
    variants: Vec<(Vec<ModelPart>, Option<u32>)>,
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
    /// External `.skl` files by path (`None`: could not be read, already recorded).
    skeleton_cache: BTreeMap<VfsPath, Option<Skeleton>>,
    /// External `.ans` files by path (`None`: could not be read, already recorded).
    anim_cache: BTreeMap<VfsPath, Option<AnimSet>>,
    report: LightmapReport,
}

impl Loader<'_> {
    fn skip(&mut self, line: String) {
        self.skipped.push(line);
    }

    /// Read one model and build its drawable parts (and collision when `collide`; lightmap UVs
    /// when `lit` placements use it).
    fn model(
        &mut self,
        path: &VfsPath,
        collide: bool,
        lit: usize,
    ) -> std::result::Result<Built, String> {
        let bytes = self.vfs.read(path).map_err(|e| e.to_string())?;
        let model = Model::from_bytes(&bytes).map_err(|e| e.to_string())?;
        self.model_parts(path.as_str(), &model, collide, lit, true)
    }

    /// Build a read model (from a `.mdl` file or embedded in the world); `name` names it in
    /// [`ModelData::name`] and in messages. With `animate`, a model with a skeleton, skinned
    /// surfaces and animations is also built animated ([`Built::skinned`]).
    fn model_parts(
        &mut self,
        name: &str,
        model: &Model,
        collide: bool,
        lit: usize,
        animate: bool,
    ) -> std::result::Result<Built, String> {
        match model.mesh() {
            MeshSource::Embedded(mesh) => {
                let lookup = |id| model.material(id);
                let mut built = self.build(name, mesh, collide, lit, &lookup)?;
                if animate {
                    built.skinned = self.skinned(name, model, mesh, &lookup, &built.model);
                }
                Ok(built)
            }
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
                let mut built = self.build(name, &mesh, collide, lit, &lookup)?;
                if animate {
                    built.skinned = self.skinned(name, model, &mesh, &lookup, &built.model);
                }
                Ok(built)
            }
            MeshSource::None => Err("the model has no mesh".to_string()),
        }
    }

    /// The parts (and collision) of `mesh`; `embedded` finds the presets stored with the mesh.
    /// With `lit` placements, the parts get lightmap UVs from the stream `lightmap::choose` picks.
    fn build<'m>(
        &mut self,
        name: &str,
        mesh: &Mesh,
        collide: bool,
        lit: usize,
        embedded: &dyn Fn(u32) -> Option<&'m Material>,
    ) -> std::result::Result<Built, String> {
        let lod = mesh
            .lods
            .iter()
            .find(|l| l.is_for_pc())
            .ok_or("no level of detail is for the PC")?;
        let materials: Vec<Option<Material>> = lod
            .surfaces
            .iter()
            .map(|surface| match &surface.material {
                MaterialRef::None => None,
                MaterialRef::Embedded(id) => embedded(*id).cloned(),
                MaterialRef::External(raw) => self.mtr(raw, name),
            })
            .collect();
        let lightmap_choice = (lit > 0).then(|| {
            let uv_ids: Vec<Vec<u32>> = materials
                .iter()
                .map(|m| m.as_ref().map(material_uv_ids).unwrap_or_default())
                .collect();
            lightmap::choose(&lod.surfaces, &uv_ids)
        });
        let lightmap_stream = lightmap_choice
            .as_ref()
            .and_then(lightmap::StreamChoice::stream);
        let (kept, left_out) = self.parts(&lod.surfaces, &materials, lightmap_stream);
        for e in left_out {
            self.skip(format!("model `{name}`: a part is left out: {e}"));
        }
        if kept.is_empty() {
            return Err("nothing to draw".to_string());
        }
        let mut several = None;
        if let Some(choice) = &lightmap_choice {
            self.count_lightmap_choice(name, choice, lit);
            if let lightmap::StreamChoice::Several { .. } = choice {
                several = Some(choice.ties().map(|tied| {
                    Tie {
                        streams: tied.to_vec(),
                        surfaces: lod.surfaces.clone(),
                        variants: tied[1..]
                            .iter()
                            .map(|&k| (self.parts(&lod.surfaces, &materials, Some(k)).0, None))
                            .collect(),
                    }
                }));
            }
        }
        let collision = if collide {
            let coarsest = mesh
                .lods
                .iter()
                .rev()
                .find(|l| l.is_for_pc())
                .unwrap_or(lod);
            let c = match mesh.collision.as_deref().filter(|c| !c.is_empty()) {
                Some(own) => collision_from_mesh(own),
                None => collision_of(coarsest),
            };
            if c.positions.iter().all(|p| p.is_finite()) {
                Some(c)
            } else {
                self.skip(format!(
                    "model `{name}`: collision has non-finite positions, it does not collide"
                ));
                None
            }
        } else {
            None
        };
        Ok(Built {
            model: ModelData {
                name: name.to_string(),
                parts: kept,
                collision,
            },
            several,
            skinned: None,
        })
    }

    /// `model` built animated (module docs, "Animated models"), or `None` to draw it static: it
    /// needs a skinned surface in its drawn level of detail, a readable skeleton and an animation
    /// that binds to it. `stat` is the static build (its collision is reused). A model that has
    /// what it takes but whose skeleton or animations cannot be used gets a skipped line.
    fn skinned<'m>(
        &mut self,
        name: &str,
        model: &Model,
        mesh: &Mesh,
        embedded: &dyn Fn(u32) -> Option<&'m Material>,
        stat: &ModelData,
    ) -> Option<SkinnedModelData> {
        let lod = mesh.lods.iter().find(|l| l.is_for_pc())?;
        if !lod.surfaces.iter().any(skin::is_skinned) || model.anim_sets.is_empty() {
            return None;
        }
        let skeleton = match &model.skeleton {
            SkeletonSource::None => return None,
            SkeletonSource::Embedded(s) => s.clone(),
            SkeletonSource::External(raw) => self.external_skeleton(raw, name)?,
            SkeletonSource::Unreadable(why) => {
                self.skip(format!("model `{name}`: skeleton: {why} (drawn static)"));
                return None;
            }
        };
        let skeleton = match skeleton.lods().first().map(skin::skeleton_data) {
            Some(Ok(s)) => s,
            Some(Err(why)) => {
                self.skip(format!("model `{name}`: skeleton: {why} (drawn static)"));
                return None;
            }
            None => {
                self.skip(format!(
                    "model `{name}`: the skeleton has no level of detail (drawn static)"
                ));
                return None;
            }
        };
        let mut sets: Vec<AnimSet> = Vec::new();
        for source in &model.anim_sets {
            match source {
                AnimSetSource::Embedded(set) => sets.push(set.clone()),
                AnimSetSource::External(raw) => {
                    if let Some(set) = self.external_anim_set(raw, name) {
                        sets.push(set);
                    }
                }
                AnimSetSource::Unreadable(why) => {
                    self.skip(format!("model `{name}`: animation set: {why}"));
                }
                AnimSetSource::None => {}
            }
        }
        let (animations, default_animation) = skin::bind_animations(&sets, &skeleton);
        if animations.is_empty() {
            self.skip(format!(
                "model `{name}`: no animation binds to its skeleton (drawn static)"
            ));
            return None;
        }
        let materials: Vec<Option<Material>> = lod
            .surfaces
            .iter()
            .map(|surface| match &surface.material {
                MaterialRef::None => None,
                MaterialRef::Embedded(id) => embedded(*id).cloned(),
                MaterialRef::External(raw) => self.mtr(raw, name),
            })
            .collect();
        let (parts, left_out, dropped) = skin::skinned_parts(&lod.surfaces, &skeleton, |i| {
            self.material(materials[i].as_ref())
        });
        for e in left_out {
            self.skip(format!("model `{name}`: an animated part is left out: {e}"));
        }
        if dropped > 0 {
            self.skip(format!(
                "model `{name}`: {dropped} bone weights name a palette entry that is no bone of \
                 the skeleton (those vertices follow their other bones or stay put)"
            ));
        }
        if parts.is_empty() {
            self.skip(format!(
                "model `{name}`: nothing to draw animated (drawn static)"
            ));
            return None;
        }
        Some(SkinnedModelData {
            name: name.to_string(),
            skeleton,
            parts,
            animations,
            default_animation,
            collision: stat.collision.clone(),
        })
    }

    /// An external skeleton file (read once per load); `None` with a skipped line when it cannot
    /// be read.
    fn external_skeleton(&mut self, raw: &str, user: &str) -> Option<Skeleton> {
        let path = match VfsPath::new(raw) {
            Ok(p) => p,
            Err(e) => {
                self.skip(format!(
                    "skeleton `{raw}` (used by `{user}`): {e} (drawn static)"
                ));
                return None;
            }
        };
        if let Some(s) = self.skeleton_cache.get(&path) {
            return s.clone();
        }
        let read = self.vfs.read(&path).and_then(|b| Skeleton::parse(&b));
        let skeleton = match read {
            Ok(s) => Some(s),
            Err(e) => {
                self.skip(format!(
                    "skeleton `{}` (used by `{user}`): {e} (drawn static)",
                    path.as_str()
                ));
                None
            }
        };
        self.skeleton_cache.insert(path, skeleton.clone());
        skeleton
    }

    /// An external `.ans` file (read once per load); `None` with a skipped line when it cannot be
    /// read.
    fn external_anim_set(&mut self, raw: &str, user: &str) -> Option<AnimSet> {
        let path = match VfsPath::new(raw) {
            Ok(p) => p,
            Err(e) => {
                self.skip(format!("animation set `{raw}` (used by `{user}`): {e}"));
                return None;
            }
        };
        if let Some(s) = self.anim_cache.get(&path) {
            return s.clone();
        }
        let read = self.vfs.read(&path).and_then(|b| AnimSet::from_bytes(&b));
        let set = match read {
            Ok(s) => Some(s),
            Err(e) => {
                self.skip(format!(
                    "animation set `{}` (used by `{user}`): {e}",
                    path.as_str()
                ));
                None
            }
        };
        self.anim_cache.insert(path, set.clone());
        set
    }

    /// The parts of the surfaces of one level of detail (their `materials`), with lightmap UVs
    /// from the stream at position `lightmap_stream`; and why parts were left out.
    fn parts(
        &mut self,
        surfaces: &[Surface],
        materials: &[Option<Material>],
        lightmap_stream: Option<usize>,
    ) -> (Vec<ModelPart>, Vec<String>) {
        let mut parts: Vec<ModelPart> = Vec::new();
        let mut part_of: BTreeMap<u32, usize> = BTreeMap::new();
        for (surface, material) in surfaces.iter().zip(materials) {
            let (material, uv_map) = self.material(material.as_ref());
            let i = *part_of.entry(material).or_insert_with(|| {
                parts.push(ModelPart {
                    mesh: MeshData::default(),
                    material,
                });
                parts.len() - 1
            });
            append_surface(&mut parts[i].mesh, surface, uv_map, lightmap_stream);
        }
        let mut kept = Vec::with_capacity(parts.len());
        let mut left_out = Vec::new();
        for part in parts {
            match part.mesh.validate() {
                Ok(()) if !part.mesh.indices.is_empty() => kept.push(part),
                Ok(()) => {}
                Err(e) => left_out.push(e.to_string()),
            }
        }
        (kept, left_out)
    }

    /// Which of `tie`'s streams a placement takes, by its own lightmap `texture` (module docs,
    /// "Lightmaps"): the index into [`Tie::streams`]. `label` names the placement in a skipped
    /// line when the lightmap cannot be decoded (it then keeps the model's stream).
    fn choose_stream(&mut self, tie: &Tie, texture: &Texture, label: &str) -> usize {
        let image = match texture.decode_level(0) {
            Ok(image) => image,
            Err(e) => {
                self.report.several_by_order += 1;
                self.skip(format!(
                    "{label}: lightmap cannot be decoded to choose its stream: {e} (the \
                     model's stream is kept)"
                ));
                return 0;
            }
        };
        let fractions: Vec<lightmap::LitFraction> = tie
            .streams
            .iter()
            .map(|&k| lightmap::lit_fraction(&tie.surfaces, k, &image))
            .collect();
        let (chosen, stage) = lightmap::choose_tied(&fractions);
        match stage {
            lightmap::Stage::LitFraction => self.report.several_by_lit_fraction += 1,
            lightmap::Stage::Order => self.report.several_by_order += 1,
        }
        chosen
    }

    /// Count which rule chose the lightmap stream of a lit model; a model left without one gets a
    /// skipped line.
    fn count_lightmap_choice(&mut self, name: &str, choice: &lightmap::StreamChoice, lit: usize) {
        let report = &mut self.report;
        report.lit_models += 1;
        match choice {
            lightmap::StreamChoice::One(k) => *report.one_candidate.entry(*k).or_default() += 1,
            lightmap::StreamChoice::Several { .. } => {
                report.several += 1;
                report.several_tied += usize::from(choice.ties().is_some());
            }
            lightmap::StreamChoice::NearMiss { stream } => {
                report.near_miss += 1;
                report.near_miss_not_slot_zero += usize::from(*stream != 0);
            }
            lightmap::StreamChoice::NoneInRange => {
                report.none_in_range += 1;
                let placements = if lit == 1 { "placement" } else { "placements" };
                self.skip(format!(
                    "model `{name}`: no UV stream lies in [0, 1], so no lightmap UVs \
                     ({lit} lit {placements} drawn unlit)"
                ));
            }
        }
    }

    /// The preset of an external `.mtr` file (read once per load).
    fn mtr(&mut self, raw: &str, user: &str) -> Option<Material> {
        let path = match VfsPath::new(raw) {
            Ok(p) => p,
            Err(e) => {
                self.skip(format!("material `{raw}` (used by `{user}`): {e}"));
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
                    "material `{}` (used by `{user}`): {e} (drawn with the grey material)",
                    path.as_str()
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

/// Every UV map id `material` uses: of all its configs, the UV map of each texture group whose
/// texture is not empty (`mtr.md`: the UV map id is the member after its texture, "UV map ids").
fn material_uv_ids(material: &Material) -> Vec<u32> {
    material
        .configs
        .iter()
        .flat_map(|c| {
            c.params.iter().filter_map(|(member, p)| match p {
                Param::UvMap(id)
                    if member.checked_sub(1).and_then(|t| c.param(t)).is_some_and(
                        |t| matches!(t, Param::Texture(r) if *r != TextureRef::None),
                    ) =>
                {
                    Some(*id)
                }
                _ => None,
            })
        })
        .collect()
}

/// Append one surface's triangles to `mesh`, taking the UV stream with map id `uv_map` (else the
/// first) and, with `lightmap_stream`, the lightmap UVs from the stream at that position.
/// Non-finite normals and UVs are replaced (up, zero); positions are left for validation.
fn append_surface(
    mesh: &mut MeshData,
    surface: &Surface,
    uv_map: Option<u32>,
    lightmap_stream: Option<usize>,
) {
    let base = index_u32(mesh.vertices.len());
    if let Some(k) = lightmap_stream {
        let lightmap = surface.uv_sets.get(k);
        let uvs = mesh
            .lightmap_uvs
            .get_or_insert_with(|| vec![[0.0, 0.0]; mesh.vertices.len()]);
        uvs.extend((0..surface.positions.len()).map(|i| {
            lightmap
                .and_then(|u| u.get(i))
                .copied()
                .filter(|u| u.iter().all(|c| c.is_finite()))
                .unwrap_or([0.0, 0.0])
        }));
    }
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

/// A model's own collision mesh as collision: its vertices and triangles as stored.
fn collision_from_mesh(mesh: &CollisionMesh) -> CollisionData {
    CollisionData {
        positions: mesh.positions.iter().map(|p| Vec3::from(*p)).collect(),
        indices: mesh.triangles.iter().flatten().copied().collect(),
        from_collision_mesh: true,
    }
}

/// All triangles of one level of detail as collision (the fallback).
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

mod lightmap;
mod skin;
mod sky;

#[cfg(test)]
mod tests;
