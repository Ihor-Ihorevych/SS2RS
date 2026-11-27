//! Data-gated tests on the worlds of the retail install (skip without `SS2_DATA_DIR`).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::f32::consts::FRAC_1_SQRT_2;

use ss2_core::math::{Quat, Vec3};
use ss2_format_wld::{ModelRef, SkyFace, World};
use ss2_vfs::{EntryKind, Vfs, VfsPath};

/// Every file of the install with extension `ext`, sorted by path.
fn files_with_extension(vfs: &dyn Vfs, ext: &str) -> Vec<VfsPath> {
    let mut out = Vec::new();
    let mut stack = vec![VfsPath::root()];
    while let Some(d) = stack.pop() {
        for e in vfs.list(&d).unwrap() {
            let path = d.join(&e.name).unwrap();
            if e.kind == EntryKind::Dir {
                stack.push(path);
            } else if path.extension() == Some(ext) {
                out.push(path);
            }
        }
    }
    out.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    out
}

/// Relative tolerance when sorting scales into classes.
const EPS: f32 = 1e-5;

fn close(a: Vec3, b: [f32; 3]) -> bool {
    a.abs_diff_eq(Vec3::from(b), 1e-3)
}

/// The worked example of `wld-root.md` and `wld-geometry.md`: the deathmatch world DockingBay.
#[test]
fn the_docking_bay_example() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let path = VfsPath::new("Content/SeriousSam2/Levels/Multiplayer/DM_DockingBay/DockingBay.wld")
        .unwrap();
    let bytes = vfs.read(&path).unwrap();
    assert_eq!(bytes.len(), 33_217);
    let world = World::from_bytes(&bytes).unwrap();
    assert!(close(world.gravity, [0.0, -30.0, 0.0]));
    // CWorldInfoEntity, CStaticModelEntity, CChapterInfoEntity.
    assert_eq!(world.entity_count, 3);
    assert_eq!(world.static_models.len(), 1);
    let m = &world.static_models[0];
    assert_eq!(
        (m.serial, m.name.as_str(), m.model.path(), m.has_hull),
        (
            1,
            "StaticModel001",
            Some("Content/SeriousSam2/Levels/Multiplayer/DM_DockingBay/DockingBay.mdl"),
            true
        )
    );
    // Every placement of the subtree is the identity, the scale (1, 1, 1).
    assert!(m.rotation.abs_diff_eq(Quat::IDENTITY, 1e-6));
    assert!(close(m.translation, [0.0, 0.0, 0.0]));
    assert!(close(m.scale, [1.0, 1.0, 1.0]));
    assert!(world.spawn_markers.is_empty());
    // The chapter info entity #6 is placed by its editor renderable #68 (the top; #69 is its
    // child at world (24.9866, 0, 30.4261)).
    assert_eq!(world.chapters.len(), 1);
    let c = &world.chapters[0];
    assert_eq!(c.serial, 2);
    assert!(close(c.translation, [22.4866, 0.0, 30.4261]), "{c:?}");
    assert!(
        c.rotation.abs_diff_eq(
            Quat::from_xyzw(0.0, -FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2),
            1e-3
        ),
        "{c:?}"
    );
}

/// Every world of the install reads, with the totals of `wld-root.md`, `wld-geometry.md` and
/// `wld-entities.md` (59 distinct worlds; identical copies under another path are counted once).
#[test]
fn every_world_of_the_install_reads() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let paths = files_with_extension(&vfs, "wld");
    let mut seen = HashSet::new();
    let mut failures = Vec::new();
    let (mut worlds, mut entities, mut models, mut hulls, mut embedded) = (0, 0, 0, 0, 0);
    let (mut spawn_markers, mut spawn_worlds, mut chapters) = (0, 0, 0);
    let mut scales: BTreeMap<&str, usize> = BTreeMap::new();
    let mut distinct_models = BTreeSet::new();
    let mut odd = Vec::new();
    for path in &paths {
        let bytes = vfs.read(path).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        let world = match World::from_bytes(&bytes) {
            Ok(w) => w,
            Err(e) => {
                failures.push(format!("{}: {e}", path.as_str()));
                continue;
            }
        };
        worlds += 1;
        entities += world.entity_count;
        if !close(world.gravity, [0.0, -30.0, 0.0]) {
            odd.push(format!("{}: gravity {}", path.as_str(), world.gravity));
        }
        for m in &world.static_models {
            models += 1;
            hulls += usize::from(m.has_hull);
            match &m.model {
                ModelRef::External(p) => {
                    distinct_models.insert(p.to_ascii_lowercase());
                }
                ModelRef::Embedded(_) => embedded += 1,
            }
            let s = m.scale;
            let kind = if s.abs_diff_eq(Vec3::ONE, EPS) {
                "one"
            } else if s.min_element() < 0.0 {
                "negative"
            } else if (s.x - s.y).abs() <= EPS * s.x.abs() && (s.y - s.z).abs() <= EPS * s.y.abs() {
                "uniform"
            } else {
                "non-uniform"
            };
            *scales.entry(kind).or_default() += 1;
            if !m.rotation.is_finite()
                || (m.rotation.length() - 1.0).abs() > 1e-3
                || !m.translation.is_finite()
            {
                odd.push(format!("{}: {m:?}", path.as_str()));
            }
        }
        spawn_markers += world.spawn_markers.len();
        spawn_worlds += usize::from(!world.spawn_markers.is_empty());
        chapters += world.chapters.len();
    }
    eprintln!(
        "{} .wld paths, {worlds} distinct worlds read, {entities} entities; {models} static models \
         ({hulls} with hull, {embedded} embedded, {} distinct .mdl), scales {scales:?}; \
         {spawn_markers} spawn markers in {spawn_worlds} worlds; {chapters} chapters; odd: {odd:?}",
        paths.len(),
        distinct_models.len()
    );
    assert!(
        failures.is_empty(),
        "first failures: {:?}",
        &failures[..failures.len().min(20)]
    );
    assert!(odd.is_empty(), "{odd:?}");
    // wld-root.md: 59 distinct worlds, 73,098 entities.
    assert_eq!((worlds, entities), (59, 73_098));
    // wld-geometry.md: 25,383 static models, 21,420 with a mechanism and hulls, 109 embedded
    // models, 2,258 distinct external models, all of which exist.
    assert_eq!((models, hulls, embedded), (25_383, 21_420, 109));
    assert_eq!(distinct_models.len(), 2_258);
    let missing: Vec<_> = distinct_models
        .iter()
        .filter(|p| !VfsPath::new(p.as_str()).is_ok_and(|p| vfs.exists(&p)))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
    // Scales: wld-geometry.md counts 2,683 (1,1,1), 16,377 uniform, 5,057 non-uniform and 1,266
    // with a negative axis; within a relative tolerance one model more counts as uniform here.
    assert_eq!(scales["one"], 2_683);
    assert_eq!(scales["negative"], 1_266);
    assert_eq!(scales["uniform"] + scales["non-uniform"], 16_377 + 5_057);
    // wld-entities.md: 177 spawn markers in 16 worlds, 307 chapter info entities.
    assert_eq!((spawn_markers, spawn_worlds, chapters), (177, 16, 307));
}

/// The sky test vector of `environment.md`: `1_3_Village`, a five-face sky.
#[test]
fn the_village_sky() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let path =
        VfsPath::new("Content/SeriousSam2/Levels/01_MDigBo/1_3_Village/1_3_Village.wld").unwrap();
    let world = World::from_bytes(&vfs.read(&path).unwrap()).unwrap();
    let sky = world.sky.expect("the village has a sky");
    let file = |f: SkyFace| {
        sky.face(f)
            .map(|p| p.rsplit('/').next().unwrap().to_ascii_lowercase())
    };
    assert_eq!(
        file(SkyFace::North).as_deref(),
        Some("mdigbo_village_bckg_0000.tex")
    );
    assert_eq!(
        file(SkyFace::West).as_deref(),
        Some("mdigbo_village_bckg_0003.tex")
    );
    assert_eq!(
        file(SkyFace::South).as_deref(),
        Some("mdigbo_village_bckg_0002.tex")
    );
    assert_eq!(
        file(SkyFace::East).as_deref(),
        Some("mdigbo_village_bckg_0001.tex")
    );
    assert_eq!(file(SkyFace::Floor), None);
    assert_eq!(
        file(SkyFace::Ceiling).as_deref(),
        Some("mdigbo_village_bckg_0004.tex")
    );
    // The record's own texture is the south face's file.
    assert_eq!(sky.texture.as_deref(), sky.face(SkyFace::South));
    assert_eq!(
        (sky.size, sky.viewer_y, sky.color),
        (64.0, 0.0, 0x00FF_FFFF)
    );
}

/// Every sky of the install, with the counts of `environment.md` and `environment-schema.md`.
#[test]
fn every_sky_of_the_install() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let mut seen = HashSet::new();
    let (mut worlds, mut skies) = (0, 0);
    let mut without_sky = Vec::new();
    let mut without_faces = BTreeSet::new();
    let mut per_face = [0usize; 6];
    let (mut six, mut five, mut textured, mut texture_is_a_face) = (0, 0, 0, 0);
    let (mut flag_4_set, mut flag_5_set) = (0, 0);
    let mut sizes: BTreeMap<u32, usize> = BTreeMap::new();
    let mut colors = BTreeSet::new();
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    let mut missing = Vec::new();
    for path in files_with_extension(&vfs, "wld") {
        let bytes = vfs.read(&path).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        worlds += 1;
        let world = World::from_bytes(&bytes).unwrap();
        let name = path.as_str().rsplit('/').next().unwrap_or_default();
        let stem = name.trim_end_matches(".wld").to_ascii_lowercase();
        let Some(sky) = world.sky else {
            without_sky.push(stem);
            continue;
        };
        skies += 1;
        for f in SkyFace::ALL {
            if let Some(p) = sky.face(f) {
                per_face[f.index()] += 1;
                if !VfsPath::new(p).is_ok_and(|p| vfs.exists(&p)) {
                    missing.push(p.to_string());
                }
            }
        }
        match sky.face_count() {
            0 => {
                without_faces.insert(stem);
            }
            5 => five += 1,
            6 => six += 1,
            n => panic!("{}: {n} faces", path.as_str()),
        }
        if let Some(t) = &sky.texture {
            textured += 1;
            texture_is_a_face += usize::from(sky.faces.iter().flatten().any(|f| f == t));
        }
        flag_4_set += usize::from(sky.flag_4 == 1.0);
        flag_5_set += usize::from(sky.flag_5 == Some(true));
        *sizes.entry(sky.size.to_bits()).or_default() += 1;
        colors.insert(sky.color);
        min_y = min_y.min(sky.viewer_y);
        max_y = max_y.max(sky.viewer_y);
    }
    eprintln!(
        "{worlds} worlds, {skies} skies (none in {without_sky:?}); faces per member {per_face:?}; \
         {five} five-face, {six} six-face, no faces in {without_faces:?}; record texture {textured} \
         ({texture_is_a_face} a face); .4 set {flag_4_set}, .5 set {flag_5_set}; colours {colors:x?}; \
         viewer y {min_y}..{max_y}"
    );
    assert!(missing.is_empty(), "{missing:?}");
    // environment.md: 58 of 59 worlds have one sky; `dockingbay` has none.
    assert_eq!((worlds, skies), (59, 58));
    assert_eq!(without_sky, ["dockingbay"]);
    // environment-schema.md: `.10`-`.13` and `.15` are set in 55 skies, `.14` (the floor) in 11.
    assert_eq!(per_face, [55, 55, 55, 55, 11, 55]);
    assert_eq!((five, six), (44, 11));
    // The three skies without any face texture.
    assert_eq!(
        without_faces.into_iter().collect::<Vec<_>>(),
        ["1_4_approachingcity", "dm_cockaleekie", "dm_hole"]
    );
    // The record's texture is set in 55 and is always one of the faces.
    assert_eq!((textured, texture_is_a_face), (55, 55));
    // Size 64 in 56 skies, 32 and 6 in one each; `.4` is 1 in 54, `.5` set in 16; 6 colours.
    let sizes: Vec<_> = sizes
        .into_iter()
        .map(|(s, n)| (f32::from_bits(s), n))
        .collect();
    assert_eq!(sizes, [(6.0, 1), (32.0, 1), (64.0, 56)]);
    assert_eq!((flag_4_set, flag_5_set), (54, 16));
    assert_eq!(colors.len(), 6);
    assert!(
        (min_y + 6.66).abs() < 0.01 && (max_y - 4.52).abs() < 0.01,
        "{min_y} {max_y}"
    );
}
