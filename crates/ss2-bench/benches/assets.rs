//! VFS and asset-manager hot paths: path normalization, archive reads, cold and cached loads.

use std::cell::LazyCell;
use std::fs::File;
use std::hint::black_box;
use std::io::Write;
use std::sync::Arc;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use ss2_assets::AssetManager;
use ss2_bench::{LenLoader, sample_path, sample_vfs};
use ss2_vfs::{ArchiveVfs, Vfs, VfsPath};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const FILES_PER_BATCH: usize = 100;
const SAMPLE_BYTES: usize = 4096;
const ARCHIVE_BYTES: usize = 4 * 1024 * 1024;

fn vfs_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("vfs");
    group.bench_function("path_normalize", |b| {
        b.iter(|| VfsPath::new(black_box(r"Models\Enemies\.\Gnaar\Gnaar_Body.MDL")).unwrap());
    });
    let vfs = LazyCell::new(|| sample_vfs(1_000, SAMPLE_BYTES));
    let path = VfsPath::new(&sample_path(537)).unwrap();
    group.bench_function("stat", |b| {
        let vfs = &*vfs;
        b.iter(|| vfs.stat(black_box(&path)).unwrap());
    });
    group.bench_function("read_4k", |b| {
        let vfs = &*vfs;
        b.iter(|| vfs.read(black_box(&path)).unwrap());
    });
    let dir = VfsPath::new("models/d5").unwrap();
    group.bench_function("list_100", |b| {
        let vfs = &*vfs;
        b.iter(|| vfs.list(black_box(&dir)).unwrap());
    });
    group.finish();
}

fn asset_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("assets");
    group.bench_function("load_cached_100", |b| {
        let warm = AssetManager::new(Arc::new(sample_vfs(1_000, SAMPLE_BYTES)));
        warm.register_loader(LenLoader).unwrap();
        for i in 0..FILES_PER_BATCH {
            warm.load::<usize>(&sample_path(i)).unwrap();
        }
        b.iter(|| {
            for i in 0..FILES_PER_BATCH {
                black_box(warm.load::<usize>(&sample_path(i)).unwrap());
            }
        });
    });

    group.throughput(Throughput::BytesDecimal(
        (FILES_PER_BATCH * SAMPLE_BYTES) as u64,
    ));
    group.bench_function("load_cold_100", |b| {
        let vfs = Arc::new(sample_vfs(1_000, SAMPLE_BYTES));
        let new_manager = || {
            let assets = AssetManager::new(vfs.clone());
            assets.register_loader(LenLoader).unwrap();
            assets
        };
        let preflight = new_manager();
        for i in 0..FILES_PER_BATCH {
            let handle = preflight.load::<usize>(&sample_path(i)).unwrap();
            assert_eq!(*handle.get().unwrap(), SAMPLE_BYTES);
        }
        drop(preflight);

        b.iter_batched(
            new_manager,
            |assets| {
                for i in 0..FILES_PER_BATCH {
                    black_box(assets.load::<usize>(&sample_path(i)).unwrap());
                }
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn archive_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("asset_archive");
    group.throughput(Throughput::BytesDecimal(ARCHIVE_BYTES as u64));
    for (name, method) in [
        ("stored_4mib", CompressionMethod::Stored),
        ("deflated_4mib", CompressionMethod::Deflated),
    ] {
        group.bench_function(name, |b| {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("sample.gro");
            let mut writer = ZipWriter::new(File::create(&path).unwrap());
            writer
                .start_file(
                    "payload.bin",
                    SimpleFileOptions::default().compression_method(method),
                )
                .unwrap();
            let mut block = [0; SAMPLE_BYTES];
            let mut seed = 42_u32;
            for _ in 0..ARCHIVE_BYTES / block.len() {
                // Alternate fixed and pseudorandom bytes for a reproducible deflate workload.
                for byte in block.iter_mut().step_by(2) {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    *byte = seed.to_be_bytes()[0];
                }
                writer.write_all(&block).unwrap();
            }
            drop(writer.finish().unwrap());

            let archive = ArchiveVfs::from_file(&path).unwrap();
            let key = VfsPath::new("payload.bin").unwrap();
            assert_eq!(archive.stat(&key).unwrap().size, ARCHIVE_BYTES as u64);
            b.iter(|| black_box(archive.read(black_box(&key)).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, vfs_path, asset_manager, archive_reads);
criterion_main!(benches);
