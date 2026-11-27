//! Game data for the app (T03-036): find and mount the player's install, list its levels, load one and put it on
//! the GPU, and the offscreen `--level NAME --screenshot OUT` mode.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_physics::controller::WalkConfig;
use ss2_render::{Renderer, SkyBatch, TextureHandle, WorldBatch};
use ss2_render_api::Camera;
use ss2_render_api::scene_data::{AlphaMode, TextureData};
use ss2_ui::Color;
use ss2_vfs::levels::{LevelInfo, discover_levels};
use ss2_vfs::{SystemEnvironment, Vfs};
use ss2_world::load::{LoadOptions, load_level};
use ss2_world::scene::{LevelScene, LoadProgress};

/// What the world is cleared to behind the level (no sky yet).
pub(crate) const SKY: Color = Color([0.4, 0.6, 0.8, 1.0]);

/// The install folder to use: `explicit` (`--data`), then `SS2_DATA_DIR`, then the first install the search finds
/// (Steam, the registry, common folders).
///
/// # Errors
/// When none of them gives a folder.
pub(crate) fn data_dir(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(dir) = explicit {
        return Ok(dir.to_path_buf());
    }
    if let Some(dir) = std::env::var_os(ss2_vfs::DATA_DIR_ENV).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    ss2_vfs::discover(&SystemEnvironment, None)
        .candidates
        .into_iter()
        .next()
        .map(|c| c.dir)
        .ok_or_else(|| {
            format!(
                "no Serious Sam 2 install found: pass --data <folder> or set {}",
                ss2_vfs::DATA_DIR_ENV
            )
        })
}

/// A mounted install and its levels.
pub(crate) struct GameData {
    vfs: Box<dyn Vfs + Send + Sync>,
    levels: Vec<LevelInfo>,
}

impl GameData {
    /// Mount the install in `dir` and list its levels.
    ///
    /// # Errors
    /// When the folder cannot be mounted or its levels cannot be listed.
    pub(crate) fn open(dir: &Path) -> Result<Self, String> {
        let vfs = ss2_vfs::mount_install(dir)
            .map_err(|e| format!("cannot mount the game data in {}: {e}", dir.display()))?;
        Self::from_vfs(Box::new(vfs))
    }

    /// Use an already mounted file system.
    pub(crate) fn from_vfs(vfs: Box<dyn Vfs + Send + Sync>) -> Result<Self, String> {
        let levels =
            discover_levels(vfs.as_ref()).map_err(|e| format!("cannot list the levels: {e}"))?;
        Ok(Self { vfs, levels })
    }

    /// Every level of the install, sorted by group, then id.
    pub(crate) fn levels(&self) -> &[LevelInfo] {
        &self.levels
    }

    /// The level whose id (its folder name) or world file name is `name`, ignoring case and a `.wld` ending.
    pub(crate) fn find(&self, name: &str) -> Option<&LevelInfo> {
        let name = name.trim().to_ascii_lowercase();
        let name = name.strip_suffix(".wld").unwrap_or(&name);
        self.levels.iter().find(|l| {
            l.id == name
                || l.wld
                    .file_name()
                    .is_some_and(|f| f.strip_suffix(".wld") == Some(name))
        })
    }

    /// Load `level` with the default options, reporting progress.
    ///
    /// # Errors
    /// When the world file itself cannot be read (anything else is listed in [`LevelScene::skipped`]).
    pub(crate) fn load(
        &self,
        level: &LevelInfo,
        progress: &mut dyn FnMut(LoadProgress),
    ) -> Result<Arc<LevelScene>, String> {
        let scene = load_level(
            self.vfs.as_ref(),
            &level.wld,
            &LoadOptions::default(),
            progress,
        )
        .map_err(|e| format!("cannot load {}: {e}", level.id))?;
        for line in &scene.skipped {
            tracing::debug!(level = %level.id, "skipped: {line}");
        }
        if !scene.skipped.is_empty() {
            tracing::info!(
                level = %level.id,
                skipped = scene.skipped.len(),
                "the level loaded without some of its parts"
            );
        }
        Ok(Arc::new(scene))
    }
}

/// What [`upload`] put on the GPU.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct UploadReport {
    /// Meshes uploaded (one per model part that has instances).
    pub meshes: usize,
    /// Textures uploaded.
    pub textures: usize,
    /// Textures drawn white because the GPU cannot use their format or the upload failed.
    pub textures_replaced: usize,
    /// Instances placed, summed over all batches.
    pub instances: usize,
    /// Whether the level's sky was set.
    pub sky: bool,
    /// Placements drawn with their own lightmap (one batch each per model part).
    pub lightmapped: usize,
}

/// Put `scene` on the GPU as the renderer's world, replacing what was there: one mesh and one batch per model part,
/// with every placement of its model.
///
/// # Errors
/// When a mesh cannot be uploaded or the batches are refused (the scene was validated by the loader, so this means a
/// GPU problem).
pub(crate) fn upload(renderer: &mut Renderer, scene: &LevelScene) -> Result<UploadReport, String> {
    renderer.clear_world();
    let mut report = UploadReport::default();
    let white = renderer
        .create_texture(&TextureData::solid([255, 255, 255, 255]))
        .map_err(|e| e.to_string())?;
    let mut textures: Vec<TextureHandle> = Vec::with_capacity(scene.textures.len());
    for t in &scene.textures {
        let handle = if renderer.supports(t.format) {
            renderer.create_texture(t).ok()
        } else {
            None
        };
        match handle {
            Some(h) => {
                report.textures += 1;
                textures.push(h);
            }
            None => {
                report.textures_replaced += 1;
                textures.push(white);
            }
        }
    }
    // Placements without a lightmap share one batch per model part; each lightmapped placement gets its own, with its
    // lightmap (RFC 0013 amendment 2).
    let mut shared: Vec<Vec<Mat4>> = vec![Vec::new(); scene.models.len()];
    let mut lit: Vec<Vec<(Mat4, TextureHandle)>> = vec![Vec::new(); scene.models.len()];
    for inst in &scene.instances {
        let m = inst.model as usize;
        if m >= scene.models.len() {
            continue;
        }
        match inst.lightmap {
            Some(l) => lit[m].push((inst.transform.matrix(), textures[l as usize])),
            None => shared[m].push(inst.transform.matrix()),
        }
    }
    report.lightmapped = lit.iter().map(Vec::len).sum();
    let mut batches = Vec::new();
    for ((model, shared), lit) in scene.models.iter().zip(&shared).zip(&lit) {
        if shared.is_empty() && lit.is_empty() {
            continue;
        }
        for part in &model.parts {
            let material = scene.materials[part.material as usize];
            let mesh = renderer
                .create_mesh(&part.mesh)
                .map_err(|e| e.to_string())?;
            report.meshes += 1;
            let batch = |instances: Vec<Mat4>, lightmap: Option<TextureHandle>| WorldBatch {
                mesh,
                texture: material.base_texture.map(|t| textures[t as usize]),
                tint: material.base_color,
                // Blending is drawn as a cutout until the renderer sorts transparent surfaces.
                alpha_cutoff: match material.alpha {
                    AlphaMode::Opaque => None,
                    AlphaMode::Mask { cutoff } => Some(cutoff),
                    AlphaMode::Blend => Some(0.5),
                },
                instances,
                lightmap,
            };
            if !shared.is_empty() {
                report.instances += shared.len();
                batches.push(batch(shared.clone(), None));
            }
            // A part without lightmap UVs cannot use a lightmap: it is drawn by the flat light.
            let lightmap_uvs = part.mesh.lightmap_uvs.is_some();
            for (matrix, lightmap) in lit {
                report.instances += 1;
                batches.push(batch(vec![*matrix], lightmap_uvs.then_some(*lightmap)));
            }
        }
    }
    renderer.set_world(batches).map_err(|e| e.to_string())?;
    // The sky's faces are textures of the scene like any other (stand-ins included).
    let sky = scene.sky.map(|sky| SkyBatch {
        faces: sky.faces.map(|f| f.map(|t| textures[t as usize])),
        tint: sky.tint,
    });
    report.sky = sky.is_some();
    renderer.set_sky(sky).map_err(|e| e.to_string())?;
    Ok(report)
}

/// The camera at the level's start: at eye height above the start point, facing its heading.
pub(crate) fn start_camera(scene: &LevelScene) -> Camera {
    let start = scene.start();
    Camera {
        position: start.position + Vec3::Y * WalkConfig::PROVISIONAL.eye_height,
        rotation: Quat::from_rotation_y(start.yaw),
        ..Camera::default()
    }
}

/// `--level NAME --screenshot OUT`: load the level and render it offscreen from its start to a PNG.
///
/// # Errors
/// When the data, the level, the GPU or the file fails; the message says which.
pub(crate) fn take_level_screenshot(
    level: &str,
    out: &Path,
    size: (u32, u32),
    data: Option<&Path>,
) -> Result<(), String> {
    let dir = data_dir(data)?;
    let game = GameData::open(&dir)?;
    let info = game.find(level).ok_or_else(|| {
        let names: Vec<&str> = game.levels().iter().map(|l| l.id.as_str()).collect();
        format!(
            "no level named '{level}'; the install has: {}",
            names.join(", ")
        )
    })?;
    let started = std::time::Instant::now();
    let scene = game.load(info, &mut |_| {})?;
    let loaded = started.elapsed();
    let mut renderer = Renderer::new_offscreen(size).map_err(|e| e.to_string())?;
    let report = upload(&mut renderer, &scene)?;
    let image = renderer
        .render_world_to_image(&start_camera(&scene), None, SKY)
        .map_err(|e| e.to_string())?;
    crate::screenshot::save_png(out, &image)?;
    tracing::info!(
        level = %info.id,
        load_ms = loaded.as_millis(),
        instances = report.instances,
        meshes = report.meshes,
        textures = report.textures,
        textures_replaced = report.textures_replaced,
        skipped = scene.skipped.len(),
        out = %out.display(),
        "level screenshot saved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_render::RenderError;
    use ss2_vfs::MemoryVfs;

    fn game() -> GameData {
        let nfo =
            |name: &str| format!("LEVEL\nNAME = \"{name}\"\nMAP_TYPE = \"SINGLE\"\n").into_bytes();
        let mut v = MemoryVfs::new();
        let l = "content/serioussam2/levels";
        v.insert(&format!("{l}/01_mdigbo/1_1_jungle/1_1_jungle.wld"), vec![])
            .unwrap();
        v.insert(
            &format!("{l}/01_mdigbo/1_1_jungle/1_1_jungle.nfo"),
            nfo("Jungle"),
        )
        .unwrap();
        v.insert(&format!("{l}/multiplayer/dm_yard/yard.wld"), vec![])
            .unwrap();
        v.insert(&format!("{l}/multiplayer/dm_yard/yard.nfo"), nfo("Yard"))
            .unwrap();
        GameData::from_vfs(Box::new(v)).unwrap()
    }

    #[test]
    fn levels_are_found_by_id_or_world_name_without_case() {
        let g = game();
        assert_eq!(g.levels().len(), 2);
        assert_eq!(
            g.find("1_1_JUNGLE").map(|l| l.id.as_str()),
            Some("1_1_jungle")
        );
        assert_eq!(g.find("dm_yard").map(|l| l.id.as_str()), Some("dm_yard"));
        assert_eq!(
            g.find("Yard.wld").map(|l| l.id.as_str()),
            Some("dm_yard"),
            "the world file name"
        );
        assert!(g.find("nope").is_none());
    }

    #[test]
    fn an_explicit_data_folder_wins() {
        let dir = Path::new("C:/somewhere/ss2");
        assert_eq!(data_dir(Some(dir)).unwrap(), dir);
    }

    #[test]
    fn the_start_camera_stands_at_eye_height_facing_the_start() {
        let room = LevelScene::test_room();
        let cam = start_camera(&room);
        let start = room.start();
        assert_eq!(
            cam.position,
            start.position + Vec3::Y * WalkConfig::PROVISIONAL.eye_height
        );
        assert!(
            (cam.forward() - Vec3::NEG_Z).length() < 1e-5,
            "yaw 0 looks down -Z"
        );
    }

    #[test]
    fn the_test_room_uploads_one_batch_per_part_and_draws() {
        let mut renderer = match Renderer::new_offscreen((160, 90)) {
            Ok(r) => r,
            Err(RenderError::NoAdapter(reason)) => {
                eprintln!("SKIPPED (no GPU adapter): {reason}");
                return;
            }
            Err(e) => panic!("{e}"),
        };
        let room = LevelScene::test_room();
        let report = upload(&mut renderer, &room).unwrap();
        assert_eq!(report.meshes, 1);
        assert_eq!(
            report.textures,
            room.textures.len(),
            "every texture, sky faces included"
        );
        assert_eq!(report.instances, room.instances.len());
        let image = renderer
            .render_world_to_image(&start_camera(&room), None, SKY)
            .unwrap();
        let sky = [102, 153, 204, 255];
        let not_sky = image.pixels.chunks(4).filter(|p| *p != sky).count();
        assert!(not_sky > 160 * 90 / 2, "the room fills the view: {not_sky}");
        // Uploading again replaces the world.
        let again = upload(&mut renderer, &room).unwrap();
        assert_eq!(again, report);
        // Straight up from the start there is no roof: the sky's top face shows, unlit.
        assert!(report.sky);
        let up = Camera {
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ..start_camera(&room)
        };
        let image = renderer.render_world_to_image(&up, None, SKY).unwrap();
        let centre = &image.pixels[((45 * 160 + 80) * 4)..][..4];
        let top = ss2_world::scene::test_sky_colour(ss2_world::scene::CubeFace::PosY);
        assert!(
            centre.iter().zip(top).all(|(a, b)| a.abs_diff(b) <= 3),
            "{centre:?} is the sky's top face {top:?}"
        );
        // The floor is the room's one lightmapped placement: from above, its middle is in the lightmap's shadow.
        assert_eq!(report.lightmapped, 1);
        let above = Camera {
            position: Vec3::new(0.0, 12.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ..start_camera(&room)
        };
        let image = renderer.render_world_to_image(&above, None, SKY).unwrap();
        let at = |x: usize, y: usize| f32::from(image.pixels[(y * 160 + x) * 4]);
        let ratio = at(80, 45) / at(80, 3);
        assert!(
            (ratio - 0.3).abs() < 0.05,
            "shadow/light ratio on the floor: {ratio}"
        );
        // A level without a sky clears to the sky colour.
        let mut bare = LevelScene::test_room();
        bare.sky = None;
        assert!(!upload(&mut renderer, &bare).unwrap().sky);
        let image = renderer.render_world_to_image(&up, None, SKY).unwrap();
        assert_eq!(
            &image.pixels[((45 * 160 + 80) * 4)..][..4],
            &[102, 153, 204, 255]
        );
    }

    #[test]
    fn a_real_level_renders_from_its_start() {
        let dir = ss2_testkit::require_game_data!();
        let out = tempfile::tempdir().unwrap();
        let path = out.path().join("dock.png");
        match take_level_screenshot("dm_dockingbay", &path, (320, 180), Some(&dir)) {
            Ok(()) => {}
            Err(e) if e.contains("no suitable GPU adapter") => {
                eprintln!("SKIPPED (no GPU adapter): {e}");
                return;
            }
            Err(e) => panic!("{e}"),
        }
        let png = std::fs::read(&path).unwrap();
        let image = ss2_render::golden::decode_png(&png).unwrap();
        assert_eq!((image.width, image.height), (320, 180));
        let sky = [102, 153, 204, 255];
        let not_sky = image.pixels.chunks(4).filter(|p| *p != sky).count();
        assert!(
            not_sky * 10 > 320 * 180 * 3,
            "more than 30 % of the picture is the level: {not_sky}"
        );
        // The picture itself is game content: it stays in the temporary folder.
    }
}
