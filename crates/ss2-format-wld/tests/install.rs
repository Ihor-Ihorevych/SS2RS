//! Data-gated tests on the worlds of the retail install (skip without `SS2_DATA_DIR`).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::f32::consts::FRAC_1_SQRT_2;

use ss2_core::math::{Quat, Vec3};
use ss2_format_tex::PixelFormat;
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
    let (mut chain_worlds, mut chain_not_first_in_list, mut next_links) = (0, 0, 0);
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
        next_links += world.chapters.iter().filter(|c| c.next.is_some()).count();
        let chain = world.chapter_chain();
        if !chain.is_empty() {
            chain_worlds += 1;
            chain_not_first_in_list += usize::from(chain[0] != 0);
        }
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
    // mechanism-classes-world-info.md / wld-entity-schema.md: 51 worlds name a first chapter,
    // 245 chapters a next chapter; chapter-restore-players.md section 2.9: in 31 of the 51 the
    // chain's first chapter is not the first chapter of the entity list.
    eprintln!(
        "{chain_worlds} worlds with a chain, {chain_not_first_in_list} not starting at the first chapter of the list; {next_links} next-chapter links"
    );
    assert_eq!(
        (chain_worlds, chain_not_first_in_list, next_links),
        (51, 31, 245)
    );
}

/// The two single-player starts of `chapter-restore-players.md` section 2.9: the chain's first
/// chapter, its placement and heading.
#[test]
fn the_chain_starts_of_the_start_rule_examples() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let paths = files_with_extension(&vfs, "wld");
    // (file, first chapter serial, its rank in the list, chain length, position, heading)
    let cases = [
        ("1_1_jungle.wld", 3619, 3, 5, [263.2, -0.6, -242.0], 104.5),
        ("7_1_city_part_i.wld", 3250, 7, 8, [-54.7, 3.4, 39.6], 0.0),
    ];
    for (file, serial, rank, length, position, heading) in cases {
        let path = paths
            .iter()
            .find(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(file)))
            .unwrap_or_else(|| panic!("no {file}"));
        let world = World::from_bytes(&vfs.read(path).unwrap()).unwrap();
        let chain = world.chapter_chain();
        assert_eq!(chain.len(), length, "{file}");
        assert_eq!(chain[0], rank, "{file}");
        let first = &world.chapters[chain[0]];
        assert_eq!((first.serial, first.name.as_str()), (serial, "Chapter001"));
        assert!(
            first.translation.abs_diff_eq(Vec3::from(position), 0.06),
            "{file}: {}",
            first.translation
        );
        let forward = first.rotation.normalize() * Vec3::NEG_Z;
        let yaw = (-forward.x).atan2(-forward.z).to_degrees();
        assert!((yaw - heading).abs() < 0.1, "{file}: heading {yaw}");
        assert!(world.spawn_markers.is_empty(), "{file}");
    }
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

/// The lightmap test vector of `wld-lighting.md`: `dm_stadium.wld`, instance #2435 (`Fence.mdl`)
/// with oven modifier #3553: parameter ids 12 and 13, UV map id 10, texture #11 (128 x 128 DXT1,
/// flags 0x71). The spec's "all 266 oven modifiers use UV map id 10" counts the main section's
/// 133 and their 133 editor copies in the extra section; the reader sees the main ones.
#[test]
fn the_stadium_lightmap() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let path =
        VfsPath::new("Content/SeriousSam2/Levels/Multiplayer/DM_Stadium/DM_Stadium.wld").unwrap();
    let world = World::from_bytes(&vfs.read(&path).unwrap()).unwrap();
    let lm = world
        .static_models
        .iter()
        .filter_map(|m| m.lightmap.as_ref())
        .find(|l| l.modifier == 3553)
        .expect("the oven modifier #3553 belongs to a static model");
    assert_eq!(
        (lm.texture_param.id, lm.uv_param.id, lm.uv_map.id),
        (12, 13, 10)
    );
    assert_eq!(lm.uv_map.name, "Shadow");
    assert_eq!(lm.texture_object, Some(11));
    let t = lm.texture.as_ref().unwrap();
    assert_eq!(
        (t.width, t.height, t.format, t.format_code, t.flags),
        (128, 128, PixelFormat::Dxt1, 7, 0x71)
    );
    assert_eq!(world.lightmap_counts.oven_modifiers, 133);
    assert!(
        world
            .static_models
            .iter()
            .filter_map(|m| m.lightmap.as_ref())
            .all(|l| l.uv_map.id == 10)
    );
}

/// Every lightmap of the install, with the counts of `wld-lighting.md` ("Baked lightmaps"):
/// 18,679 oven modifiers on 18,392 lit model instances (18,105 with exactly one), each texture
/// used once. The texture formats and sizes are counted over the static models' lightmaps, the
/// part of the spec's 18,676 textures this reader decodes.
#[test]
fn every_lightmap_of_the_install() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let mut seen = HashSet::new();
    let (mut worlds, mut totals) = (0, ss2_format_wld::LightmapCounts::default());
    let (mut lit_models, mut models, mut several_models) = (0, 0, 0);
    let mut formats: BTreeMap<u32, usize> = BTreeMap::new();
    let mut sizes: BTreeMap<u32, usize> = BTreeMap::new();
    let mut flags: BTreeMap<u32, usize> = BTreeMap::new();
    let (mut square, mut decoded) = (0, 0);
    let mut unreadable: BTreeMap<String, usize> = BTreeMap::new();
    let mut reused = Vec::new();
    for path in files_with_extension(&vfs, "wld") {
        let bytes = vfs.read(&path).unwrap();
        if !seen.insert(bytes.clone()) {
            continue;
        }
        worlds += 1;
        let world = World::from_bytes(&bytes).unwrap();
        let c = world.lightmap_counts;
        totals.lit_instances += c.lit_instances;
        totals.oven_modifiers += c.oven_modifiers;
        totals.several += c.several;
        let mut textures = HashSet::new();
        for m in &world.static_models {
            models += 1;
            let Some(lm) = &m.lightmap else { continue };
            lit_models += 1;
            several_models += usize::from(lm.other_oven_modifiers > 0);
            if let Some(id) = lm.texture_object
                && !textures.insert(id)
            {
                reused.push(format!("{}: texture {id}", path.as_str()));
            }
            match &lm.texture {
                Ok(t) => {
                    decoded += 1;
                    *formats.entry(t.format_code).or_default() += 1;
                    *sizes.entry(t.width.max(t.height)).or_default() += 1;
                    *flags.entry(t.flags).or_default() += 1;
                    square += usize::from(t.width == t.height);
                }
                Err(e) => *unreadable.entry(e.clone()).or_default() += 1,
            }
        }
    }
    eprintln!(
        "{worlds} worlds; {totals:?}; {lit_models} of {models} static models lit \
         ({several_models} with several oven modifiers); {decoded} lightmaps decoded: \
         format codes {formats:?}, largest side {sizes:?}, flags {flags:x?}, {square} square; \
         unreadable {unreadable:?}"
    );
    assert_eq!(worlds, 59);
    // wld-lighting.md: 18,679 oven modifiers on 18,392 lit instances, 18,105 with exactly one.
    assert_eq!(totals.oven_modifiers, 18_679);
    assert_eq!(totals.lit_instances, 18_392);
    assert_eq!(totals.lit_instances - totals.several, 18_105);
    // Each lightmap texture is used by exactly one instance.
    assert!(reused.is_empty(), "{reused:?}");
    // The static models carry 18,195 of them (the rest are doors' and props'), 286 with a
    // second oven modifier, and every one of their lightmaps decodes.
    assert_eq!((lit_models, several_models), (18_195, 286));
    assert!(unreadable.is_empty(), "{unreadable:?}");
    assert_eq!(decoded, lit_models);
    // Formats and sizes are a part of the spec's (DXT1 17,299, 24-bit 1,123, 32-bit 251; sides
    // 16 to 2,048): as measured on the static models.
    let formats: Vec<_> = formats.into_iter().collect();
    assert_eq!(formats, [(0, 1_089), (1, 250), (2, 2), (6, 1), (7, 16_853)]);
    let sizes: Vec<_> = sizes.into_iter().collect();
    assert_eq!(
        sizes,
        [
            (16, 111),
            (32, 787),
            (64, 7_466),
            (128, 7_008),
            (256, 2_243),
            (512, 444),
            (1_024, 130),
            (2_048, 6)
        ]
    );
    assert_eq!(square, 17_791);
    // Bit 0x10 marks a baked texture: set on every one of them (the spec's one exception,
    // flags 0x20, is not a static model's).
    assert!(flags.keys().all(|f| f & 0x10 != 0), "{flags:x?}");
}
