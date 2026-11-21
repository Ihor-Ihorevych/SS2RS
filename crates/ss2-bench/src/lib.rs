//! ss2-bench: the benchmark harness (roadmap T00-019).
//!
//! The benchmarks live in `benches/` (criterion). This library holds the **fixtures they share**,
//! so every crate's benchmark measures the same synthetic, license-clean scenes and results stay
//! comparable over time. Nothing here uses game data.
//!
//! How to run, compare against a baseline and read the budgets: `docs/PERF_BUDGETS.md`.

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::Result;
use ss2_core::math::{Quat, Vec3};
use ss2_render_api::{ModelId, RenderId, RenderWorld, Renderable, Transform};
use ss2_vfs::MemoryVfs;

/// A [`MemoryVfs`] with `files` small files spread over directories of 100
/// (`models/d0/f0.bin`, ... ), each `size` bytes.
pub fn sample_vfs(files: usize, size: usize) -> MemoryVfs {
    let mut vfs = MemoryVfs::new();
    let payload = vec![0xA5_u8; size];
    for i in 0..files {
        let path = format!("Models/D{}/F{i}.bin", i / 100);
        vfs.insert(&path, payload.clone())
            .expect("generated paths are valid and conflict-free");
    }
    vfs
}

/// The path of the `i`-th file of [`sample_vfs`], spelled the way game code might (mixed case).
pub fn sample_path(i: usize) -> String {
    format!("Models/D{}/F{i}.bin", i / 100)
}

/// Loads `.bin` files; the asset is the file length. Stands in for a real format loader.
pub struct LenLoader;

impl AssetLoader for LenLoader {
    type Asset = usize;

    fn extensions(&self) -> &[&str] {
        &["bin"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<usize> {
        Ok(ctx.read()?.len())
    }
}

/// A [`RenderWorld`] with `n` renderables laid out on a grid, all valid.
pub fn sample_world(tick: u64, n: u64) -> RenderWorld {
    let mut world = RenderWorld::new(tick);
    fill_world(&mut world, n);
    world
}

/// Refill `world` (already [`reset`](RenderWorld::reset)) with `n` grid renderables.
pub fn fill_world(world: &mut RenderWorld, n: u64) {
    let model = ModelId::from_raw(1).expect("1 is a valid id");
    let side = ((n as f64).sqrt().ceil() as u64).max(1);
    for i in 0..n {
        let (x, z) = ((i % side) as f32, (i / side) as f32);
        world.renderables.push(Renderable::new(
            RenderId(i),
            model,
            Transform {
                translation: Vec3::new(x * 2.0, 0.0, -z * 2.0),
                rotation: Quat::from_rotation_y(i as f32 * 0.01),
                scale: Vec3::ONE,
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::AssetManager;
    use ss2_vfs::{Vfs, VfsPath};

    use super::*;

    #[test]
    fn sample_vfs_has_the_requested_files() {
        let vfs = sample_vfs(250, 16);
        let path = VfsPath::new(&sample_path(249)).unwrap();
        assert_eq!(vfs.stat(&path).unwrap().size, 16);
        let dir = VfsPath::new("models/d2").unwrap();
        assert_eq!(vfs.list(&dir).unwrap().len(), 50);
    }

    #[test]
    fn len_loader_loads_through_the_manager() {
        let assets = AssetManager::new(Arc::new(sample_vfs(3, 7)));
        assets.register_loader(LenLoader).unwrap();
        let h = assets.load::<usize>(&sample_path(2)).unwrap();
        assert_eq!(*h.get().unwrap(), 7);
    }

    #[test]
    fn sample_world_is_valid_and_sized() {
        for n in [0, 1, 7, 1000] {
            let w = sample_world(1, n);
            assert_eq!(w.renderables.len() as u64, n);
            assert!(w.validate().is_ok(), "n = {n}");
        }
    }

    #[test]
    fn fill_world_after_reset_reuses_the_allocation() {
        let mut w = sample_world(1, 500);
        let cap = w.renderables.capacity();
        w.reset(2);
        fill_world(&mut w, 500);
        assert_eq!(w.renderables.capacity(), cap);
        assert!(w.validate().is_ok());
    }
}
