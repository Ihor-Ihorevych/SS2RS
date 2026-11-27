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
use ss2_world::scene::{CubeFace, LevelScene, LoadStage};

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
/// validates. Faces whose texture cannot be read become white with a skipped line: only the two
/// skies whose faces are pre-format-8 textures.
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
    // The two skies with pre-format-8 face textures (`environment.md`) are drawn white.
    let legacy: std::collections::BTreeSet<_> = unreadable
        .iter()
        .map(|l| l.split(':').next().unwrap_or_default())
        .collect();
    assert_eq!(
        legacy.into_iter().collect::<Vec<_>>(),
        ["dm_deserttemple", "dm_dockingbay"]
    );
    assert_eq!(unreadable.len(), 11);
    // East +X, West -X, Ceiling +Y, Floor -Y, South +Z, North -Z.
    assert_eq!(per_face, [55, 55, 55, 11, 55, 55]);
    assert_eq!(faces, 55 * 5 + 11);
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
/// `SS2_LOAD_ALL=1` checks every single-player level (use a release build). Two starts have no
/// static ground within reach in the retail data: `5_1_village` (53 m above the static collision)
/// and `5_2_seasidetownapproaching` (none below at all). The start rule itself is satisfied there
/// (the previous test); what the player stands on is presumably not a static model (a chapter can
/// follow a parent entity, `mechanism-classes-world-info.md`), which this loader does not read.
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
    let expected: &[&str] = if all {
        &["5_1_village", "5_2_seasidetownapproaching"]
    } else {
        &[]
    };
    assert_eq!(no_ground, expected);
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
    assert!(
        report.lightmapped * 10 >= report.lit_placements * 8,
        "{report:?}"
    );
    assert_eq!(report.unlit_texture, 0, "{report:?}");
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
        let lit = scene
            .instances
            .iter()
            .filter(|i| i.lightmap.is_some())
            .count();
        if lit != r.lightmapped {
            failures.push(format!("{}: {lit} lit instances, report {r:?}", info.id));
        }
        lit_instances += lit;
        eprintln!(
            "{:<24} lit {:>4}/{:<4} placements ({:>3} no UVs, {} bad texture, {:>3} dropped); \
             models {:>3}: one {:?}, none {}, several {}",
            info.id,
            r.lightmapped,
            r.lit_placements,
            r.unlit_no_uvs,
            r.unlit_texture,
            r.dropped,
            r.lit_models,
            r.one_candidate,
            r.no_candidate,
            r.several_candidates
        );
        all.lit_placements += r.lit_placements;
        all.lightmapped += r.lightmapped;
        all.unlit_no_uvs += r.unlit_no_uvs;
        all.unlit_texture += r.unlit_texture;
        all.dropped += r.dropped;
        all.lit_models += r.lit_models;
        all.no_candidate += r.no_candidate;
        all.several_candidates += r.several_candidates;
        for (k, n) in r.one_candidate {
            *all.one_candidate.entry(k).or_default() += n;
        }
    }
    eprintln!("{} worlds, in all: {all:?}", seen.len());
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(lit_instances, all.lightmapped);
    // Most lit placements are drawn with their lightmap (the spec: 86.6 % of lit instances
    // have exactly one candidate stream).
    assert!(all.lightmapped * 10 >= all.lit_placements * 8, "{all:?}");
    assert_eq!(all.unlit_texture, 0, "{all:?}");
    // As measured: the 18,195 lit static models of `ss2-format-wld` (58 levels), 15,976 drawn
    // with their lightmap, 2,182 whose model has no single candidate, 37 not in the scene.
    assert_eq!(
        (
            all.lit_placements,
            all.lightmapped,
            all.unlit_no_uvs,
            all.dropped
        ),
        (18_195, 15_976, 2_182, 37)
    );
}
