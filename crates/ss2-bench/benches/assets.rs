//! VFS and asset-manager hot paths: path normalization, reads, cold and cached asset loads.

use std::hint::black_box;
use std::sync::Arc;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ss2_assets::AssetManager;
use ss2_bench::{LenLoader, sample_path, sample_vfs};
use ss2_vfs::{Vfs, VfsPath};

fn vfs_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("vfs");
    group.bench_function("path_normalize", |b| {
        b.iter(|| VfsPath::new(black_box(r"Models\Enemies\.\Gnaar\Gnaar_Body.MDL")).unwrap());
    });
    let vfs = sample_vfs(1_000, 4096);
    let path = VfsPath::new(&sample_path(537)).unwrap();
    group.bench_function("stat", |b| b.iter(|| vfs.stat(black_box(&path)).unwrap()));
    group.bench_function("read_4k", |b| {
        b.iter(|| vfs.read(black_box(&path)).unwrap())
    });
    let dir = VfsPath::new("models/d5").unwrap();
    group.bench_function("list_100", |b| {
        b.iter(|| vfs.list(black_box(&dir)).unwrap())
    });
    group.finish();
}

fn asset_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("assets");
    let vfs = Arc::new(sample_vfs(1_000, 4096));

    group.bench_function("load_cold_100", |b| {
        b.iter_batched(
            || {
                let assets = AssetManager::new(vfs.clone());
                assets.register_loader(LenLoader).unwrap();
                assets
            },
            |assets| {
                for i in 0..100 {
                    black_box(assets.load::<usize>(&sample_path(i)).unwrap());
                }
            },
            BatchSize::SmallInput,
        );
    });

    let warm = AssetManager::new(vfs.clone());
    warm.register_loader(LenLoader).unwrap();
    for i in 0..100 {
        warm.load::<usize>(&sample_path(i)).unwrap();
    }
    group.bench_function("load_cached_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(warm.load::<usize>(&sample_path(i)).unwrap());
            }
        });
    });
    group.finish();
}

criterion_group!(benches, vfs_path, asset_manager);
criterion_main!(benches);
