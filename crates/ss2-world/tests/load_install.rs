//! Data-gated tests of `load_level` on the retail install (skip without `SS2_DATA_DIR`).
//!
//! `SS2_LOAD_ALL=1` additionally loads every discovered level (use a release build) and prints,
//! per level, the instances, models, textures, skipped lines and load time, and the approximate
//! memory of `1_1_jungle`.

use std::collections::BTreeMap;
use std::time::Instant;

use ss2_core::math::{Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_format_wld::World;
use ss2_physics::geometry::Ray;
use ss2_physics::query::{QueryFilter, SpatialQuery};
use ss2_vfs::levels::{LevelInfo, discover_levels};
use ss2_vfs::{DirEntry, ReadSeek, Stat, Vfs, VfsPath};
use ss2_world::collision::build_static;
use ss2_world::load::{LightmapReport, LoadOptions, load_level, load_level_with_report};
use ss2_world::scene::{CubeFace, LevelScene, LoadStage, PlayMode, is_idle_or_default};

fn level<'a>(levels: &'a [LevelInfo], id: &str) -> &'a LevelInfo {
    levels
        .iter()
        .find(|l| l.id == id)
        .unwrap_or_else(|| panic!("no level {id}"))
}

/// Bytes of vertices, indices and texels: roughly what the scene occupies.
fn scene_bytes(scene: &LevelScene) -> usize {
    let meshes: usize = scene
        .models
        .iter()
        .flat_map(|m| &m.parts)
        .map(|p| p.mesh.vertices.len() * 32 + p.mesh.indices.len() * 4)
        .sum();
    let collision: usize = scene
        .models
        .iter()
        .filter_map(|m| m.collision.as_ref())
        .map(|c| c.positions.len() * 12 + c.indices.len() * 4)
        .sum();
    let texels: usize = scene
        .textures
        .iter()
        .flat_map(|t| &t.mips)
        .map(Vec::len)
        .sum();
    meshes + collision + texels
}

#[test]
fn dm_dockingbay_loads() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let info = level(&levels, "dm_dockingbay");
    let mut stages = Vec::new();
    let scene = load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |p| {
        stages.push(p);
    })
    .unwrap();
    scene.validate().unwrap();
    eprintln!(
        "{}: {} instances, {} models, {} materials, {} textures, {} spawns, skipped {:?}",
        info.wld.as_str(),
        scene.instances.len(),
        scene.models.len(),
        scene.materials.len(),
        scene.textures.len(),
        scene.spawns.len(),
        scene.skipped
    );
    assert!(!scene.instances.is_empty());
    assert!(scene.instances.iter().any(|i| i.collidable));
    assert!(!scene.textures.is_empty());
    assert!(scene.materials.iter().any(|m| m.base_texture.is_some()));
    assert!(!scene.spawns.is_empty());
    let b = scene.bounds().unwrap();
    assert!(b.max.x - b.min.x > 10.0, "{b:?}");
    for stage in [LoadStage::World, LoadStage::Models, LoadStage::Textures] {
        let last = stages.iter().rev().find(|p| p.stage == stage).unwrap();
        assert_eq!(last.done, last.total, "{stage:?}");
    }
}

/// The two deathmatch levels whose skies are legacy (version 4-7) textures (`tex-legacy.md`):
/// every sky face is a real image and no texture is replaced for being a legacy file. Before
/// T02-041 their legacy textures were drawn white, each with a skipped line.
#[test]
fn legacy_textures_of_two_levels_load() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    for id in ["dm_dockingbay", "dm_deserttemple"] {
        let info = level(&levels, id);
        let scene = load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        scene.validate().unwrap();
        let textures: Vec<&String> = scene
            .skipped
            .iter()
            .filter(|l| l.starts_with("texture"))
            .collect();
        eprintln!(
            "{id}: {} skipped lines, {} of them textures: {:#?}",
            scene.skipped.len(),
            textures.len(),
            scene.skipped
        );
        // Every texture of both levels reads (before T02-041 the legacy textures were drawn
        // white: 15 skipped texture lines in `dm_dockingbay`, 31 in `dm_deserttemple`).
        assert!(textures.is_empty(), "{textures:#?}");
        let sky = scene.sky.as_ref().expect("a sky");
        let faces: Vec<_> = sky
            .faces
            .iter()
            .flatten()
            .map(|&t| &scene.textures[t as usize])
            .collect();
        assert!(faces.len() >= 5, "{id}: {} faces", faces.len());
        for f in faces {
            assert!(
                f.width >= 64 && f.height >= 64,
                "{id}: a {}x{} face",
                f.width,
                f.height
            );
        }
    }
}

#[test]
fn every_level_loads() {
    let dir = ss2_testkit::require_game_data!();
    if std::env::var_os("SS2_LOAD_ALL").is_none_or(|v| v.is_empty() || v == "0") {
        eprintln!("SKIPPED: set SS2_LOAD_ALL=1 (release build) to load every level");
        return;
    }
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let mut times = Vec::new();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped_total = 0;
    let mut failures = Vec::new();
    for info in &levels {
        let start = Instant::now();
        let scene = match load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{}: {e}", info.id));
                continue;
            }
        };
        let elapsed = start.elapsed();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: invalid: {e}", info.id));
        }
        times.push((elapsed, info.id.clone()));
        skipped_total += scene.skipped.len();
        for line in &scene.skipped {
            *reasons.entry(reason(line)).or_default() += 1;
        }
        eprintln!(
            "{:<28} {:>6} instances {:>5} models {:>5} textures {:>4} skipped {:>8.1} ms",
            format!("{}/{}", info.group, info.id),
            scene.instances.len(),
            scene.models.len(),
            scene.textures.len(),
            scene.skipped.len(),
            elapsed.as_secs_f64() * 1000.0
        );
        if info.id == "1_1_jungle" {
            eprintln!(
                "1_1_jungle: about {:.1} MiB of vertices, indices, collision and texels",
                scene_bytes(&scene) as f64 / (1024.0 * 1024.0)
            );
        }
    }
    times.sort();
    if let (Some(min), Some(max)) = (times.first(), times.last()) {
        let median = &times[times.len() / 2];
        eprintln!(
            "{} levels; load time min {:.0} ms ({}), median {:.0} ms ({}), max {:.0} ms ({}); {skipped_total} skipped lines",
            times.len(),
            min.0.as_secs_f64() * 1000.0,
            min.1,
            median.0.as_secs_f64() * 1000.0,
            median.1,
            max.0.as_secs_f64() * 1000.0,
            max.1
        );
    }
    let mut common: Vec<_> = reasons.into_iter().collect();
    common.sort_by_key(|c| std::cmp::Reverse(c.1));
    for (r, n) in common.iter().take(15) {
        eprintln!("{n:>6} x {r}");
    }
    assert!(failures.is_empty(), "{failures:?}");
}

/// The install without its models: loading a level then reads the world and the sky's textures
/// only, so every level loads in a moment.
struct WithoutModels<'a>(&'a dyn Vfs);

impl WithoutModels<'_> {
    fn check(path: &VfsPath) -> Result<()> {
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mdl"))
        {
            return Err(Error::NotFound(path.as_str().to_string()));
        }
        Ok(())
    }
}

impl Vfs for WithoutModels<'_> {
    fn stat(&self, path: &VfsPath) -> Result<Stat> {
        Self::check(path)?;
        self.0.stat(path)
    }

    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>> {
        Self::check(path)?;
        self.0.open(path)
    }

    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        self.0.list(dir)
    }

    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        Self::check(path)?;
        self.0.read(path)
    }
}

/// Every level whose world has a sky loads one, with a face for each face texture the world
/// names (`environment.md`: 55 skies with five or six faces, 3 without any), and the scene
/// validates. Every face texture reads: the 11 faces of `dm_deserttemple` and `dm_dockingbay`
/// are legacy (version 4-7) textures, read since T02-041.
#[test]
fn every_sky_loads() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let without_models = WithoutModels(&vfs);
    let mut seen = std::collections::BTreeSet::new();
    let (mut worlds, mut skies, mut faces) = (0, 0, 0);
    let mut per_face = [0usize; 6];
    let mut tints = std::collections::BTreeSet::new();
    let mut unreadable = Vec::new();
    let mut white_faces = 0;
    let mut failures = Vec::new();
    let mut turned = 0;
    for info in &levels {
        let bytes = vfs.read(&info.wld).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        worlds += 1;
        let world = World::from_bytes(&bytes).unwrap();
        let scene = load_level(
            &without_models,
            &info.wld,
            &LoadOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: {e}", info.id));
        }
        let (Some(want), Some(got)) = (&world.sky, &scene.sky) else {
            if world.sky.is_some() != scene.sky.is_some() {
                failures.push(format!("{}: sky {:?}", info.id, scene.sky));
            }
            continue;
        };
        skies += 1;
        // The placement (`environment.md`, T01-409): the entity's rotation, normalised, never
        // its position; the size and the viewer height as stored.
        if !got.rotation.abs_diff_eq(want.rotation.normalize(), 1e-6)
            || (got.size, got.viewer_height) != (want.size, want.viewer_y)
        {
            failures.push(format!(
                "{}: placement {:?} {} {} of {want:?}",
                info.id, got.rotation, got.size, got.viewer_height
            ));
        }
        if !got.rotation.abs_diff_eq(Quat::IDENTITY, 1e-6) {
            turned += 1;
            // A pure heading: the up axis is never tilted.
            let up = got.rotation * Vec3::Y;
            if !up.abs_diff_eq(Vec3::Y, 1e-3) {
                failures.push(format!(
                    "{}: sky turned off +Y: {:?}",
                    info.id, got.rotation
                ));
            }
        }
        let loaded = got.faces.iter().flatten().count();
        if loaded != want.face_count() {
            failures.push(format!(
                "{}: {loaded} faces loaded of {}",
                info.id,
                want.face_count()
            ));
        }
        faces += loaded;
        for f in CubeFace::ALL {
            per_face[f.index()] += usize::from(got.faces[f.index()].is_some());
        }
        white_faces += got
            .faces
            .iter()
            .flatten()
            .filter(|&&t| scene.textures[t as usize].width == 1)
            .count();
        tints.insert(got.tint.map(f32::to_bits));
        unreadable.extend(
            scene
                .skipped
                .iter()
                .filter(|l| l.starts_with("texture"))
                .map(|l| format!("{}: {l}", info.id)),
        );
    }
    eprintln!(
        "{worlds} worlds, {skies} skies, {faces} faces, per cube face {per_face:?}          (+X -X +Y -Y +Z -Z), {} tints; unreadable faces: {unreadable:#?}",
        tints.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
    // 44 of the 58 skies are turned, all about +Y (`environment.md`, T01-409).
    eprintln!("{turned} turned skies");
    assert_eq!(turned, 44);
    // `discover_levels` lists the 58 playable worlds, all with a sky (the 59th world of the
    // install, `dockingbay`, has none and is not a level).
    assert_eq!((worlds, skies), (58, 58));
    // Every face reads, including the 11 faces of the two skies whose textures are legacy
    // version 4-7 files (`environment.md`, `tex-legacy.md`; drawn white before T02-041).
    assert!(unreadable.is_empty(), "{unreadable:#?}");
    assert_eq!(white_faces, 0, "white stand-in faces");
    // East +X, West -X, Ceiling +Y, Floor -Y, South +Z, North -Z.
    assert_eq!(per_face, [55, 55, 55, 11, 55, 55]);
    assert_eq!(faces, 55 * 5 + 11);
}

/// Every haze volume of the install reaches the scene (`environment.md`, "Haze": 65 in 45
/// worlds, T07-042), in the world's order, and `1_3_village`'s worked example comes out as the
/// spec works it: colour `0x2a383d`, a 300 x 100 x 400 box, a hard edge (`.5` = 0 → 0.001),
/// near -150 and far 1,860.
#[test]
fn every_haze_loads() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let without_models = WithoutModels(&vfs);
    let mut seen = std::collections::BTreeSet::new();
    let (mut hazes, mut worlds) = (0, 0);
    let mut failures = Vec::new();
    for info in &levels {
        let bytes = vfs.read(&info.wld).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        let world = World::from_bytes(&bytes).unwrap();
        let scene = load_level(
            &without_models,
            &info.wld,
            &LoadOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: {e}", info.id));
        }
        if scene.skipped.iter().any(|l| l.starts_with("haze"))
            || scene.hazes.len() != world.hazes.len()
        {
            failures.push(format!(
                "{}: {} of {} hazes",
                info.id,
                scene.hazes.len(),
                world.hazes.len()
            ));
            continue;
        }
        hazes += scene.hazes.len();
        worlds += usize::from(!scene.hazes.is_empty());
        for (got, want) in scene.hazes.iter().zip(&world.hazes) {
            // The box is the record's size about the renderable's world position, in its turned
            // frame.
            let centre = got.rotation * ((got.min + got.max) / 2.0);
            if !centre.abs_diff_eq(want.translation, 1e-2)
                || !(got.max - got.min).abs_diff_eq(want.size, 1e-2)
                || got.fade(want.translation) != 1.0
            {
                failures.push(format!("{}: haze {} as {got:?}", info.id, want.serial));
            }
        }
        if info.id == "1_3_village" {
            let [h] = scene.hazes.as_slice() else {
                panic!("1_3_village: {:?}", scene.hazes);
            };
            assert!(
                (h.max - h.min).abs_diff_eq(Vec3::new(300.0, 100.0, 400.0), 1e-3),
                "{h:?}"
            );
            assert_eq!((h.fade_margin, h.near, h.far), (0.001, -150.0, 1860.0));
            let lin = |b: f32| ((b / 255.0 + 0.055) / 1.055).powf(2.4);
            let want = [lin(42.0), lin(56.0), lin(61.0), 0.0];
            assert!(
                h.colour.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6),
                "{h:?}"
            );
        }
    }
    eprintln!("{hazes} hazes in {worlds} worlds");
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!((hazes, worlds), (65, 45));
}

/// Every bloom volume of the install reaches the scene (`environment.md`, "Bloom": 34 in 33
/// worlds, T07-049), in the world's order, with the record's numbers unchanged by the engine's
/// clamps, and the worked examples come out as the spec gives them: `dm_hole`'s two blooms and
/// `1_3_village`'s one (strength, ramp, blur, balance, high-pass, edge fade, box size, colour).
#[test]
fn every_bloom_loads() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let without_models = WithoutModels(&vfs);
    let mut seen = std::collections::BTreeSet::new();
    let (mut blooms, mut worlds, mut examples) = (0, 0, 0);
    let mut failures = Vec::new();
    let lin = |b: u32| {
        let c = b as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let linear = |argb: u32| {
        [
            lin((argb >> 16) & 0xFF),
            lin((argb >> 8) & 0xFF),
            lin(argb & 0xFF),
            (argb >> 24) as f32 / 255.0,
        ]
    };
    for info in &levels {
        let bytes = vfs.read(&info.wld).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        let world = World::from_bytes(&bytes).unwrap();
        let scene = load_level(
            &without_models,
            &info.wld,
            &LoadOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: {e}", info.id));
        }
        if scene.skipped.iter().any(|l| l.starts_with("bloom"))
            || scene.blooms.len() != world.blooms.len()
        {
            failures.push(format!(
                "{}: {} of {} blooms",
                info.id,
                scene.blooms.len(),
                world.blooms.len()
            ));
            continue;
        }
        blooms += scene.blooms.len();
        worlds += usize::from(!scene.blooms.is_empty());
        for (got, want) in scene.blooms.iter().zip(&world.blooms) {
            // The box is the record's size about the renderable's world position, in its turned
            // frame; the numbers are the record's (no retail value is clamped).
            let centre = got.rotation * ((got.min + got.max) / 2.0);
            let colour = linear(want.color);
            if !centre.abs_diff_eq(want.translation, 1e-2)
                || !(got.max - got.min).abs_diff_eq(want.size, 1e-2)
                || got.fade(want.translation) != 1.0
                || got.fade_margin != want.fade.max(0.001)
                || (got.strength, got.ramp, got.blur, got.balance, got.high_pass)
                    != (
                        want.strength,
                        want.ramp,
                        want.blur,
                        want.balance,
                        want.high_pass,
                    )
                || got
                    .colour
                    .iter()
                    .zip(colour)
                    .any(|(a, b)| (a - b).abs() > 1e-6)
            {
                failures.push(format!("{}: bloom {} as {got:?}", info.id, want.serial));
            }
        }
        let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
        let check =
            |b: &ss2_world::scene::BloomVolume, colour: u32, size: Vec3, numbers: [f32; 6]| {
                assert!((b.max - b.min).abs_diff_eq(size, 1e-2), "{b:?}");
                assert!(
                    b.colour
                        .iter()
                        .zip(linear(colour))
                        .all(|(a, w)| (a - w).abs() < 1e-6),
                    "{b:?}"
                );
                let got = [
                    b.fade_margin,
                    b.strength,
                    b.ramp,
                    b.blur,
                    b.balance,
                    b.high_pass,
                ];
                assert!(got.iter().zip(numbers).all(|(a, w)| near(*a, w)), "{b:?}");
            };
        if info.id == "dm_hole" {
            examples += 1;
            let [a, b] = scene.blooms.as_slice() else {
                panic!("dm_hole: {:?}", scene.blooms);
            };
            // "Bloom_PoolRoom", then "Bloom001", in the world's list order:
            // [fade margin, strength, ramp, blur, balance, high-pass].
            check(
                a,
                0xff15_0701,
                Vec3::new(96.2, 88.1, 123.3),
                [10.0, 1.0, 2.0, 1.205, 1.0, 0.5],
            );
            check(
                b,
                0xff27_2727,
                Vec3::splat(250.0),
                [10.0, 1.0, 1.5, 1.0, 1.0, 0.5],
            );
            // The spec's camera test: a point inside both fade-inset cores sees both at 1.
            let both = scene.blooms_at(Vec3::new(-40.0, 30.0, -20.0));
            assert_eq!(both.len(), 2, "{both:?}");
            assert!(both.iter().all(|b| b.fade == 1.0), "{both:?}");
        }
        if info.id == "1_3_village" {
            examples += 1;
            let [b] = scene.blooms.as_slice() else {
                panic!("1_3_village: {:?}", scene.blooms);
            };
            check(
                b,
                0xff46_3c26,
                Vec3::new(400.0, 148.0, 400.0),
                [25.0, 1.2422, 1.5, 1.0, 1.0, 0.15],
            );
        }
    }
    eprintln!("{blooms} blooms in {worlds} worlds");
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(examples, 2, "dm_hole and 1_3_village are levels");
    assert_eq!((blooms, worlds), (34, 33));
}

/// The models embedded in the worlds (`wld-geometry.md`, 109 placements, T07-043) are built like
/// external ones: no skipped line says an embedded model is not read, and of the 82 distinct
/// ones only the 10 with nothing to draw or collide with are left out, with a line each: 9 whose
/// mesh is an empty `CMesh` (no render and no collision mesh, which `ss2-format-bmf` refuses)
/// and one (`dm_cockaleekie`) whose render mesh has no level of detail and which has no
/// collision mesh.
#[test]
fn every_embedded_model_loads() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let without_models = WithoutModels(&vfs);
    let mut seen = std::collections::BTreeSet::new();
    let (mut built, mut placed, mut colliding) = (0, 0, 0);
    let mut left_out = Vec::new();
    let mut failures = Vec::new();
    let options = LoadOptions {
        max_texture_size: 4,
    };
    for info in &levels {
        let bytes = vfs.read(&info.wld).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        let scene = load_level(&without_models, &info.wld, &options, &mut |_| {}).unwrap();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: {e}", info.id));
        }
        for line in &scene.skipped {
            if line.contains("is not read") {
                failures.push(format!("{}: {line}", info.id));
            } else if line.contains("(embedded model") {
                left_out.push(format!("{}: {line}", info.id));
            }
        }
        let embedded: Vec<usize> = (0..scene.models.len())
            .filter(|&i| scene.models[i].name.contains(".wld#"))
            .collect();
        // Each model once: a lightmap variant (T07-045) has its model's name.
        let distinct: BTreeMap<&str, usize> = embedded
            .iter()
            .map(|&i| (scene.models[i].name.as_str(), i))
            .collect();
        built += distinct.len();
        colliding += distinct
            .values()
            .filter(|&&i| {
                scene.models[i]
                    .collision
                    .as_ref()
                    .is_some_and(|c| c.from_collision_mesh)
            })
            .count();
        placed += scene
            .instances
            .iter()
            .filter(|i| embedded.contains(&(i.model as usize)))
            .count();
    }
    eprintln!(
        "{built} embedded models built ({colliding} colliding with their collision mesh), {placed} placements; left out: {left_out:#?}"
    );
    assert!(failures.is_empty(), "{failures:#?}");
    let empty = left_out
        .iter()
        .filter(|l| l.contains("the mesh has no render mesh"))
        .count();
    let no_lod = left_out
        .iter()
        .filter(|l| l.contains("no level of detail is for the PC"))
        .count();
    assert_eq!((empty, no_lod, left_out.len()), (9, 1, 10), "{left_out:#?}");
    assert_eq!((built, placed), (72, 99));
}

/// The heading of a rotation in degrees: the angle about +Y of its rotated -Z.
fn heading(rotation: Quat) -> f32 {
    let forward = rotation.normalize() * Vec3::NEG_Z;
    (-forward.x).atan2(-forward.z).to_degrees()
}

/// Every single-player level starts at its chapter chain's first chapter
/// (`chapter-restore-players.md` sections 2.8 and 2.9), not at the first chapter of the entity
/// list, and the scene's start is that chapter's placement.
#[test]
fn every_single_player_level_starts_at_its_chains_first_chapter() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let without_models = WithoutModels(&vfs);
    let mut failures = Vec::new();
    let (mut single, mut not_first_in_list) = (0, 0);
    for info in levels.iter().filter(|l| l.has_map_type("SINGLE")) {
        single += 1;
        let world = World::from_bytes(&vfs.read(&info.wld).unwrap()).unwrap();
        let scene = load_level(
            &without_models,
            &info.wld,
            &LoadOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        let chain = world.chapter_chain();
        if !world.spawn_markers.is_empty() || chain.is_empty() {
            failures.push(format!(
                "{}: {} spawn markers, chain {chain:?}",
                info.id,
                world.spawn_markers.len()
            ));
            continue;
        }
        not_first_in_list += usize::from(chain[0] != 0);
        let first = &world.chapters[chain[0]];
        let start = scene.start();
        if scene.spawns.len() != chain.len()
            || start.position != first.translation
            || (start.yaw.to_degrees() - heading(first.rotation)).abs() > 1e-3
        {
            failures.push(format!(
                "{}: start {start:?} of {} spawns, chain's first chapter {first:?}",
                info.id,
                scene.spawns.len()
            ));
        }
    }
    eprintln!(
        "{single} single-player levels, {not_first_in_list} start off the list's first chapter"
    );
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(single > 30, "{single}");
}

/// The two worked starts of `chapter-restore-players.md` section 2.9, and the walker's ground:
/// straight below the start (raised 2 m, as the chapter's fall-down search does), within
/// [`GROUND_REACH`], there is static collision.
///
/// `SS2_LOAD_ALL=1` checks every single-player level (use a release build): all 42 starts have ground
/// within reach. `5_1_village` and `5_2_seasidetownapproaching` stand on models embedded in the world
/// (read since T07-043), 0 m below.
#[test]
fn the_worked_starts_stand_on_ground() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let all = std::env::var_os("SS2_LOAD_ALL").is_some_and(|v| !v.is_empty() && v != "0");
    let worked = [
        ("1_1_jungle", Vec3::new(263.2, -0.6, -242.0), 104.5),
        ("7_1_city_part_i", Vec3::new(-54.7, 3.4, 39.6), 0.0),
    ];
    let mut failures = Vec::new();
    let mut no_ground = Vec::new();
    let mut checked = 0;
    for info in levels.iter().filter(|l| l.has_map_type("SINGLE")) {
        let example = worked.iter().find(|w| w.0 == info.id);
        if example.is_none() && !all {
            continue;
        }
        let scene = load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        let start = scene.start();
        if let Some((_, position, yaw)) = example
            && (!start.position.abs_diff_eq(*position, 0.06)
                || (start.yaw.to_degrees() - yaw).abs() > 0.1)
        {
            failures.push(format!("{}: start {start:?}", info.id));
        }
        let world = build_static(&scene).unwrap();
        let down = Ray::new(start.position + Vec3::Y * 2.0, Vec3::NEG_Y).unwrap();
        let ground = world.raycast(&down, 2.0 + GROUND_REACH, &QueryFilter::default());
        checked += 1;
        match ground {
            Some(hit) => eprintln!(
                "{}: start {:?}, ground {:.2} m below",
                info.id,
                start.position,
                start.position.y - hit.point.y
            ),
            None => {
                eprintln!("{}: no ground below {start:?}", info.id);
                no_ground.push(info.id.as_str());
            }
        }
    }
    eprintln!("{checked} starts checked for ground (SS2_LOAD_ALL=1: every single-player level)");
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(no_ground, Vec::<&str>::new());
}

/// How far below a start the walker may fall to the ground, metres.
const GROUND_REACH: f32 = 10.0;

/// A skipped line without its paths, names and numbers, to group similar lines.
fn reason(line: &str) -> String {
    let mut out = String::new();
    let mut in_quote = false;
    let mut depth = 0u32;
    for c in line.chars() {
        match c {
            '(' if !in_quote && out.starts_with("static model") && depth == 0 => {
                depth = 1;
                out.push_str("(...)");
            }
            ')' if depth == 1 => depth = 0,
            _ if depth == 1 => {}
            '`' => {
                in_quote = !in_quote;
                if in_quote {
                    out.push_str("`…`");
                }
            }
            _ if in_quote => {}
            '0'..='9' => {
                if !out.ends_with('#') {
                    out.push('#');
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// The lightmap test vector of `wld-lighting.md` is `dm_stadium`: its lit placements get their
/// lightmaps, `Fence.mdl` (whose lightmap stream is its first, UV id 3) among them.
#[test]
fn dm_stadium_is_lightmapped() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let info = level(&levels, "dm_stadium");
    let (scene, report) =
        load_level_with_report(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
    scene.validate().unwrap();
    eprintln!("dm_stadium: {report:?}");
    let fence = scene
        .models
        .iter()
        .position(|m| m.name.to_ascii_lowercase().ends_with("/fence.mdl"))
        .expect("Fence.mdl is loaded");
    assert!(
        scene.models[fence]
            .parts
            .iter()
            .all(|p| p.mesh.lightmap_uvs.is_some())
    );
    let lit_fences = scene
        .instances
        .iter()
        .filter(|i| i.model as usize == fence && i.lightmap.is_some())
        .count();
    assert!(lit_fences > 0);
    assert_eq!(report.lit_placements, 133);
    // Every lit placement draws its lightmap, the one of the model with several candidate
    // streams too (T07-045).
    assert_eq!(
        (report.lightmapped, report.unlit_no_uvs, report.several),
        (133, 0, 1),
        "{report:?}"
    );
}

/// The UV map id of the stream of `mesh_path`'s first PC level of detail that `model` uses as
/// its lightmap UVs (compared as sorted coordinates, all surfaces together).
fn lightmap_uv_id(vfs: &dyn Vfs, model: &ss2_world::scene::ModelData) -> Option<u32> {
    use ss2_format_mdl::{MeshSource, Model};
    let path = VfsPath::new(&model.name).ok()?;
    let mdl = Model::from_bytes(&vfs.read(&path).ok()?).ok()?;
    let mesh = match mdl.mesh() {
        MeshSource::Embedded(m) => m.clone(),
        MeshSource::External(raw) => {
            let bytes = vfs.read(&VfsPath::new(raw).ok()?).ok()?;
            ss2_format_bmf::Mesh::from_bytes(&bytes).ok()?
        }
        MeshSource::None => return None,
    };
    let lod = mesh.lods.iter().find(|l| l.is_for_pc())?;
    let sorted = |uvs: Vec<[f32; 2]>| {
        let mut bits: Vec<[u32; 2]> = uvs.iter().map(|u| u.map(f32::to_bits)).collect();
        bits.sort_unstable();
        bits
    };
    let used = sorted(
        model
            .parts
            .iter()
            .flat_map(|p| p.mesh.lightmap_uvs.clone().unwrap_or_default())
            .collect(),
    );
    let drawn: Vec<_> = lod
        .surfaces
        .iter()
        .filter(|s| s.indices.len() >= 3)
        .collect();
    drawn
        .first()?
        .uv_map_ids
        .iter()
        .enumerate()
        .find_map(|(k, &id)| {
            let stream = drawn
                .iter()
                .flat_map(|s| s.uv_sets.get(k).cloned().unwrap_or_default())
                .collect();
            (sorted(stream) == used).then_some(id)
        })
}

/// T04-047: every lit animated placement of the install gets its own lightmap (or a line saying why not, the same
/// reasons a static placement would have), counted across every distinct world (58 levels; `SS2_LOAD_ALL=1` is not
/// needed, unlike `every_level_is_lightmapped`, since this only sums the lightmap fields, not every other check).
/// As measured: 2,046 lit animated placements, every one lightmapped (`unlit_animated` stays 0).
#[test]
fn every_lit_animated_placement_gets_its_lightmap() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut animated_lightmapped = 0;
    let mut unlit_animated = 0;
    let mut failures = Vec::new();
    for info in &levels {
        let bytes = vfs.read(&info.wld).unwrap();
        if !seen.insert(bytes) {
            continue;
        }
        let (scene, r) =
            load_level_with_report(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: {e}", info.id));
        }
        animated_lightmapped += scene
            .skinned_instances
            .iter()
            .filter(|i| i.lightmap.is_some())
            .count();
        unlit_animated += r.unlit_animated;
    }
    eprintln!(
        "{animated_lightmapped} lit animated placements got their lightmap ({unlit_animated} did not for a reason \
         that was neither missing UVs nor a bad texture)"
    );
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(unlit_animated, 0);
    assert_eq!(animated_lightmapped, 2_046);
}

/// T01-420's test vectors of several candidates tied on overlap, decided by each placement's
/// own lightmap (stage 2): `platform_03` takes UV id 5 (slot 2) in `1_7_city_metropolis`, the two
/// `1_4_approachingcity` terrain pieces UV id 3 (slot 0).
#[test]
fn tied_streams_go_by_each_placements_own_lightmap() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    for (id, model, expected) in [
        (
            "1_7_city_metropolis",
            "/architecture/platform_03/platform.mdl",
            5,
        ),
        ("1_4_approachingcity", "/terrain_04/terrain.mdl", 3),
        (
            "1_4_approachingcity",
            "/terrain_04_b/terrain_part_02.mdl",
            3,
        ),
    ] {
        let info = level(&levels, id);
        let (scene, report) =
            load_level_with_report(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        scene.validate().unwrap();
        let ids: Vec<Option<u32>> = scene
            .instances
            .iter()
            .filter(|i| i.lightmap.is_some())
            .map(|i| &scene.models[i.model as usize])
            .filter(|m| m.name.to_ascii_lowercase().ends_with(model))
            .map(|m| lightmap_uv_id(&vfs, m))
            .collect();
        eprintln!("{id} {model}: lightmap UV ids {ids:?}; {report:?}");
        assert!(!ids.is_empty(), "{id}: no lit {model}");
        assert!(
            ids.iter().all(|i| *i == Some(expected)),
            "{id} {model}: {ids:?}"
        );
    }
}

/// With `SS2_LOAD_ALL=1` (release build): every level loads with its lightmaps; prints, per
/// level and in all, the lit placements, how many got a lightmap and why the others did not, and
/// which UV stream the lit models' lightmaps use.
#[test]
fn every_level_is_lightmapped() {
    let dir = ss2_testkit::require_game_data!();
    if std::env::var_os("SS2_LOAD_ALL").is_none_or(|v| v.is_empty() || v == "0") {
        eprintln!("SKIPPED: set SS2_LOAD_ALL=1 (release build) to load every level");
        return;
    }
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut all = LightmapReport::default();
    let mut lit_instances = 0;
    // Lit animated placements that got their own lightmap (T04-047).
    let mut animated_lit_instances = 0;
    let mut failures = Vec::new();
    for info in &levels {
        let bytes = vfs.read(&info.wld).unwrap();
        if !seen.insert(bytes) {
            continue;
        }
        let (scene, r) =
            load_level_with_report(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: {e}", info.id));
        }
        let animated_lit = scene
            .skinned_instances
            .iter()
            .filter(|i| i.lightmap.is_some())
            .count();
        let lit = scene
            .instances
            .iter()
            .filter(|i| i.lightmap.is_some())
            .count()
            + animated_lit;
        if lit != r.lightmapped {
            failures.push(format!("{}: {lit} lit instances, report {r:?}", info.id));
        }
        lit_instances += lit;
        animated_lit_instances += animated_lit;
        eprintln!(
            "{:<24} lit {:>4}/{:<4} placements ({:>3} no UVs, {} bad texture, {:>3} dropped); \
             models {:>3}: one {:?}, several {} ({} tied, {} variants; placements by overlap \
             {}, by lit fraction {}, by order {}), near miss {} ({} not slot 0), none in range {}",
            info.id,
            r.lightmapped,
            r.lit_placements,
            r.unlit_no_uvs,
            r.unlit_texture,
            r.dropped,
            r.lit_models,
            r.one_candidate,
            r.several,
            r.several_tied,
            r.variant_models,
            r.several_by_overlap,
            r.several_by_lit_fraction,
            r.several_by_order,
            r.near_miss,
            r.near_miss_not_slot_zero,
            r.none_in_range
        );
        all.lit_placements += r.lit_placements;
        all.lightmapped += r.lightmapped;
        all.unlit_no_uvs += r.unlit_no_uvs;
        all.unlit_texture += r.unlit_texture;
        all.unlit_animated += r.unlit_animated;
        all.dropped += r.dropped;
        all.lit_models += r.lit_models;
        all.several += r.several;
        all.several_tied += r.several_tied;
        all.variant_models += r.variant_models;
        all.several_by_overlap += r.several_by_overlap;
        all.several_by_lit_fraction += r.several_by_lit_fraction;
        all.several_by_order += r.several_by_order;
        all.near_miss += r.near_miss;
        all.near_miss_not_slot_zero += r.near_miss_not_slot_zero;
        all.none_in_range += r.none_in_range;
        for (k, n) in r.one_candidate {
            *all.one_candidate.entry(k).or_default() += n;
        }
    }
    eprintln!(
        "{} worlds, in all: {all:?} ({animated_lit_instances} of the lit placements animated, T04-047)",
        seen.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(lit_instances, all.lightmapped);
    assert_eq!(all.unlit_texture, 0, "{all:?}");
    // T04-047: the skinned pipeline can now draw a lightmap, so a lit animated placement is lightmapped or
    // counted by `unlit_no_uvs`/`unlit_texture` like a static one; `unlit_animated` is the residual count of
    // those left unlit for neither reason, which has stayed 0 for every level the loader has been run against.
    assert_eq!(all.unlit_animated, 0, "{all:?}");
    // As measured: the 18,195 lit placements of `ss2-format-wld` (58 levels), static and animated alike. Placements
    // of models without any UV stream in [0, 1] (a departure from the spec's slot-0 fallback) are drawn unlit, and
    // one placement of `dm_cockaleekie` is not in the scene. Before T07-044, 2,198 were drawn unlit for lack of a
    // single candidate stream; before T07-045, 1,002 (the 902 placements of models with several candidate streams
    // too); between T04-044 and T04-047, 2,046 of them (every lit animated placement) counted as `unlit_animated`
    // instead of here, since the skinned pipeline had no lightmap at all.
    assert_eq!(
        (
            all.lit_placements,
            all.lightmapped,
            all.unlit_no_uvs,
            all.dropped
        ),
        (18_195, 18_094, 100, 1)
    );
    // The models, by the rule that applied: one candidate (by its slot); several; no candidate
    // (least overlap in [0, 1], and how often that is not slot 0); no stream in [0, 1] (left
    // unlit).
    assert_eq!(
        all.one_candidate,
        BTreeMap::from([(0, 2_021), (1, 195), (2, 36), (3, 2), (4, 1)])
    );
    assert_eq!(
        (
            all.lit_models,
            all.several,
            all.near_miss,
            all.near_miss_not_slot_zero,
            all.none_in_range
        ),
        (2_645, 209, 156, 28, 25)
    );
    // Of the several-candidate models, those tied on overlap (each placement chooses); their
    // lightmapped placements by the stage that decided: overlap (stage 1), the placement's own
    // lit fraction (stage 2), the stage-3 order; and the model variants that took. As measured
    // against the retail install (58 levels): this had already drifted from an earlier number
    // before T04-047 (confirmed unrelated to it: the same numbers measure on the code before this
    // task), uncaught because this test needs `SS2_LOAD_ALL=1` (release) with real game data,
    // rarely run.
    assert_eq!(
        (
            all.several_tied,
            all.several_by_overlap,
            all.several_by_lit_fraction,
            all.several_by_order,
            all.variant_models
        ),
        (159, 178, 237, 431, 28)
    );
}

/// Animated placements (T04-044, RFC 0013 amendment 8): two levels with animated models load them as skinned
/// instances, and every animation of every animated model poses with finite, bounded palettes: at 24 times over a
/// pass, each bone matrix is finite and rigid (determinant near 1) and its origin, like every skinned vertex, stays
/// within the model's bind-pose box grown by twice its diagonal plus 10 m. At least one model visibly moves.
#[test]
fn animated_placements_pose_with_finite_bounded_palettes() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    for id in ["dm_dockingbay", "6_3_snow_base"] {
        let info = level(&levels, id);
        let scene = load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        scene.validate().unwrap();
        assert!(
            scene.skinned_instances.len() >= 10,
            "{id}: {} animated placements",
            scene.skinned_instances.len()
        );
        let mut moved = 0usize;
        for model in &scene.skinned_models {
            let bounds = model.bounds().expect("an animated model has vertices");
            let reach = (bounds.max - bounds.min).length() * 2.0 + 10.0;
            let inside = |p: Vec3| {
                p.is_finite()
                    && p.cmpge(bounds.min - Vec3::splat(reach)).all()
                    && p.cmple(bounds.max + Vec3::splat(reach)).all()
            };
            let rest = model.palette(usize::MAX, 0.0);
            for (a, animation) in model.animations.iter().enumerate() {
                let mut max_change = 0.0f32;
                for step in 0..24 {
                    let t = animation.duration * step as f32 / 24.0;
                    let palette = model.palette(a, t);
                    assert_eq!(palette.len(), model.skeleton.bones().len());
                    for (b, m) in palette.iter().enumerate() {
                        let what =
                            format!("{id} `{}` `{}` t {t} bone {b}", model.name, animation.name);
                        assert!(m.is_finite(), "{what}: {m:?}");
                        assert!(
                            (m.determinant() - 1.0).abs() < 1e-2,
                            "{what}: det {}",
                            m.determinant()
                        );
                        assert!(inside(m.w_axis.truncate()), "{what}: origin {}", m.w_axis);
                        max_change = max_change.max(
                            (*m - rest[b])
                                .abs()
                                .to_cols_array()
                                .iter()
                                .fold(0.0, |x, y| x.max(*y)),
                        );
                    }
                    for part in &model.parts {
                        for v in (0..part.mesh.vertices.len()).step_by(7) {
                            let p = part.mesh.skinned_position(v, &palette).unwrap();
                            assert!(inside(p), "{id} `{}` vertex {v} at {p}", model.name);
                        }
                    }
                }
                moved += usize::from(a == model.default_animation as usize && max_change > 1e-3);
            }
        }
        eprintln!(
            "{id}: {} animated placements of {} models, {moved} of them move in their default animation",
            scene.skinned_instances.len(),
            scene.skinned_models.len()
        );
        assert!(moved > 0, "{id}: nothing moves");
    }
}

/// T04-048: no placement loops an action (module docs, "Animated models"): a placement's player loops only when
/// its chosen animation reads as a resting loop ([`is_idle_or_default`]); an action instead holds its first frame
/// ([`PlayMode::Once`]) until the game gives it a real start.
#[test]
fn dm_dockingbay_holds_actions_instead_of_looping_them() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    let info = level(&levels, "dm_dockingbay");
    let scene = load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
    scene.validate().unwrap();
    assert!(!scene.skinned_instances.is_empty());
    let mut held = 0;
    for inst in &scene.skinned_instances {
        let model = &scene.skinned_models[inst.model as usize];
        let name = &model.animations[inst.animation.animation as usize].name;
        let loops = inst.animation.mode == PlayMode::Loop;
        if !is_idle_or_default(name) {
            assert!(
                !loops,
                "`{}` loops its action `{name}` instead of holding it",
                model.name
            );
            held += 1;
        }
    }
    eprintln!(
        "dm_dockingbay: {held} of {} placements hold an action's first frame instead of looping it",
        scene.skinned_instances.len()
    );
}

/// T04-048: the box the renderer culls a model's draws with ([`ss2_world::scene::SkinnedModelData::culling_bounds`])
/// stays big enough for the player's own data: every posed vertex of `dm_dockingbay` and `6_3_snow_base`'s animated
/// models, across every animation at 24 points over its duration, lands inside its own model's culling bounds. A
/// model whose motion needs more room than this would have a draw vanish early; none here does.
#[test]
fn animated_models_stay_within_their_own_culling_bounds() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    for id in ["dm_dockingbay", "6_3_snow_base"] {
        let info = level(&levels, id);
        let scene = load_level(&vfs, &info.wld, &LoadOptions::default(), &mut |_| {}).unwrap();
        scene.validate().unwrap();
        for model in &scene.skinned_models {
            let bounds = model
                .culling_bounds()
                .expect("an animated model has vertices");
            for (a, animation) in model.animations.iter().enumerate() {
                for step in 0..24 {
                    let t = animation.duration * step as f32 / 24.0;
                    let palette = model.palette(a, t);
                    for part in &model.parts {
                        for i in 0..part.mesh.vertices.len() {
                            let p = part.mesh.skinned_position(i, &palette).unwrap();
                            assert!(
                                p.cmpge(bounds.min).all() && p.cmple(bounds.max).all(),
                                "{id} `{}` animation `{}` t {t} vertex {i} at {p} escapes its \
                                 culling bounds {bounds:?}",
                                model.name,
                                animation.name
                            );
                        }
                    }
                }
            }
        }
    }
}
