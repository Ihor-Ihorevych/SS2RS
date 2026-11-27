//! Data-gated tests of `load_level` on the retail install (skip without `SS2_DATA_DIR`).
//!
//! `SS2_LOAD_ALL=1` additionally loads every discovered level (use a release build) and prints,
//! per level, the instances, models, textures, skipped lines and load time, and the approximate
//! memory of `1_1_jungle`.

use std::collections::BTreeMap;
use std::time::Instant;

use ss2_vfs::levels::{LevelInfo, discover_levels};
use ss2_world::load::{LoadOptions, load_level};
use ss2_world::scene::{LevelScene, LoadStage};

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
