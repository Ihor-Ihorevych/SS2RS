//! Data-gated (skips without `SS2_DATA_DIR`): every level of the retail install loads and
//! validates, its collision comes from the models' own collision meshes where they have one
//! (T07-036), and a walker dropped at the level's start lands on ground within a few seconds of
//! simulated time (except where the ground is a model embedded in the world, which the loader
//! does not read yet).
//!
//! Slow in a debug build (it loads all 58 levels); `cargo test --release -p ss2-game --test
//! levels_land -- --nocapture` prints the per-level table in a minute or two.

use std::collections::BTreeMap;
use std::sync::Arc;

use ss2_game::play::{PlayInput, PlaySession};
use ss2_vfs::levels::discover_levels;
use ss2_world::collision::build_static;
use ss2_world::load::{LoadOptions, load_level};

const DT: f32 = 1.0 / 60.0;
/// How long a walker may fall before it must stand on ground, seconds.
const LAND_WITHIN: f32 = 5.0;

#[test]
fn every_level_collides_with_its_collision_meshes_and_a_walker_lands_at_its_start() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let levels = discover_levels(&vfs).unwrap();
    // Textures do not matter here: keep them tiny.
    let options = LoadOptions {
        max_texture_size: 4,
    };
    let (mut total_real, mut total_fallback) = (0usize, 0usize);
    let (mut inst_real, mut inst_fallback) = (0usize, 0usize);
    let mut failures = Vec::new();
    let mut landed = 0usize;
    let mut on_embedded = Vec::new();
    // Distinct colliding models over all levels, by path: from a collision mesh or not.
    let mut distinct: BTreeMap<String, bool> = BTreeMap::new();
    for info in &levels {
        let scene = match load_level(&vfs, &info.wld, &options, &mut |_| {}) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{}: {e}", info.id));
                continue;
            }
        };
        if let Err(e) = scene.validate() {
            failures.push(format!("{}: invalid: {e}", info.id));
            continue;
        }
        for m in &scene.models {
            if let Some(c) = &m.collision {
                distinct.insert(m.name.clone(), c.from_collision_mesh);
            }
        }
        let colliding: Vec<bool> = scene
            .models
            .iter()
            .filter_map(|m| m.collision.as_ref())
            .map(|c| c.from_collision_mesh)
            .collect();
        let real = colliding.iter().filter(|r| **r).count();
        let fallback = colliding.len() - real;
        total_real += real;
        total_fallback += fallback;
        for inst in scene.instances.iter().filter(|i| i.collidable) {
            let from_mesh = scene.models[inst.model as usize]
                .collision
                .as_ref()
                .is_some_and(|c| c.from_collision_mesh);
            if from_mesh {
                inst_real += 1;
            } else {
                inst_fallback += 1;
            }
        }

        let physics = match build_static(&scene) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("{}: collision: {e}", info.id));
                continue;
            }
        };
        let start = scene.start().position;
        // The ground of a few levels is a model embedded in the world, which the loader does not
        // read yet (`load_level`'s docs): a walker there falls through, whatever the collision.
        let embedded_unread = scene.skipped.iter().any(|l| l.contains("embedded model"));
        let mut session = PlaySession::new(Arc::new(scene), physics);
        let idle = PlayInput::default();
        let mut time = 0.0f32;
        let mut on_ground = None;
        while time < LAND_WITHIN {
            session.update(&idle, DT);
            time += DT;
            if session.grounded() && session.respawns() == 0 {
                on_ground = Some(time);
                break;
            }
        }
        let fell = start.y - session.feet().y;
        match on_ground {
            Some(t) => {
                landed += 1;
                eprintln!(
                    "{:<28} models: {real:>4} collision mesh, {fallback:>3} fallback; landed after {t:>4.2} s, {fell:>6.2} m below the start",
                    format!("{}/{}", info.group, info.id)
                );
            }
            None => {
                let line = format!(
                    "{}: the walker did not land within {LAND_WITHIN} s (start {start}, feet {}, {} respawns)",
                    info.id,
                    session.feet(),
                    session.respawns()
                );
                if embedded_unread {
                    eprintln!("{line}; the level has unread embedded models");
                    on_embedded.push(info.id.clone());
                } else {
                    failures.push(line);
                }
            }
        }
    }
    eprintln!(
        "{} levels; models (per level, summed): {total_real} with a collision mesh, {total_fallback} on the render-LOD fallback; collidable instances: {inst_real} on a collision mesh, {inst_fallback} on the fallback; {landed} walkers landed",
        levels.len()
    );
    let distinct_real = distinct.values().filter(|r| **r).count();
    let fallback_names: Vec<&String> = distinct
        .iter()
        .filter(|(_, r)| !**r)
        .map(|(n, _)| n)
        .collect();
    eprintln!(
        "distinct colliding models: {distinct_real} with a collision mesh, {} on the fallback: {fallback_names:#?}",
        fallback_names.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
    // Measured: the two levels whose start stands on an embedded world model (a `Terrain` or
    // `StaticModel096` entity that `load_level` skips); the fallback render LOD fails there
    // the same way. Every other level's walker lands.
    on_embedded.sort();
    assert_eq!(on_embedded, ["dm_cliff", "dm_stadium"]);
    assert_eq!(landed + on_embedded.len(), levels.len());
    assert!(
        total_real > total_fallback,
        "most colliding models have a collision mesh"
    );
}
