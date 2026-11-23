//! Data-gated: every texture and every model of the player's own install is exported in memory
//! and read back by an independent reader (the `png` decoder, the `gltf` crate). Skips cleanly
//! without `SS2_DATA_DIR`.
//!
//! Walking the whole install is slow in a debug build (large DXT textures, PNG compression), so by
//! default every 16th texture is converted; `ASSET_EXPORT_FULL=1` converts all of them (run it
//! in release: `cargo test --release -p asset-export --test real_install -- --nocapture`). All
//! models are always converted. With `ASSET_EXPORT_KEEP_DIR=<folder outside the repository>` the
//! converted files are also written there (for the Khronos validator and Pillow:
//! `scripts/validate_gltf.mjs`, `scripts/validate_png.py`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use asset_export::batch::{Kind, Options, convert};
use asset_export::error::ExportError;
use asset_export::mesh::{GltfFormat, LodSel, MeshOptions};
use asset_export::source::Source;
use ss2_format_bmf::Mesh;
use ss2_vfs::{Vfs, VfsPath};

fn keep_dir() -> Option<PathBuf> {
    std::env::var_os("ASSET_EXPORT_KEEP_DIR").map(PathBuf::from)
}

fn keep(rel: &str, bytes: &[u8]) {
    if let Some(dir) = keep_dir() {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
}

/// Mean absolute difference between horizontally neighbouring pixels (RGB channels), 0 for a flat
/// image, about 85 for uniform noise.
fn roughness(color: png::ColorType, w: u32, px: &[u8]) -> f64 {
    let ch = match color {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => 1,
    };
    let (mut sum, mut n) = (0u64, 0u64);
    let row = w as usize * ch;
    for line in px.chunks(row) {
        for x in 1..w as usize {
            for c in 0..ch.min(3) {
                sum += u64::from(line[x * ch + c].abs_diff(line[(x - 1) * ch + c]));
                n += 1;
            }
        }
    }
    if n == 0 { 0.0 } else { sum as f64 / n as f64 }
}

#[test]
fn every_texture_of_the_install_is_exported_and_decodes() {
    let dir = ss2_testkit::require_game_data!();
    let source = Source::install(&dir).unwrap();
    let all = source.list("", &["tex"]).unwrap();
    let full = std::env::var_os("ASSET_EXPORT_FULL").is_some();
    let files: Vec<_> = all
        .iter()
        .enumerate()
        .filter(|(i, _)| full || i % 16 == 0)
        .map(|(_, e)| e)
        .collect();
    let opts = Options::default();
    let (mut exported, mut unsupported, mut failures) = (0usize, 0usize, Vec::new());
    let (mut bytes_in, mut bytes_out, mut pixels) = (0u64, 0u64, 0u64);
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    let mut colour_types: BTreeMap<String, usize> = BTreeMap::new();
    let (mut noisy, mut cubes) = (Vec::new(), 0usize);
    for file in &files {
        let bytes = source.read(file).unwrap();
        bytes_in += bytes.len() as u64;
        match convert(Kind::Texture, &bytes, file, &source, &opts) {
            Ok(c) => {
                exported += 1;
                bytes_out += c.bytes.len() as u64;
                // Independent decode.
                let mut reader = png::Decoder::new(std::io::Cursor::new(&c.bytes))
                    .read_info()
                    .unwrap_or_else(|e| panic!("{}: {e}", file.key));
                let mut buf = vec![0; reader.output_buffer_size().unwrap()];
                let info = reader
                    .next_frame(&mut buf)
                    .unwrap_or_else(|e| panic!("{}: {e}", file.key));
                let size = c.details["size"].as_array().unwrap();
                assert_eq!(
                    (u64::from(info.width), u64::from(info.height)),
                    (size[0].as_u64().unwrap(), size[1].as_u64().unwrap()),
                    "{}",
                    file.key
                );
                pixels += u64::from(info.width) * u64::from(info.height);
                *formats
                    .entry(c.details["format"].as_str().unwrap().to_string())
                    .or_default() += 1;
                *colour_types
                    .entry(format!("{:?}", info.color_type))
                    .or_default() += 1;
                cubes += usize::from(c.details["faces"] == 6);
                buf.truncate(info.buffer_size());
                if roughness(info.color_type, info.width, &buf) > 60.0 {
                    noisy.push(file.key.clone());
                }
                keep(&file.with_extension("png"), &c.bytes);
            }
            Err(ExportError::Unsupported(_)) => unsupported += 1,
            Err(e) => failures.push(format!("{}: {e}", file.key)),
        }
    }
    eprintln!(
        "textures: {} in the install, {} converted ({}): {exported} exported, {unsupported} unsupported, {} failed; \
         {bytes_in} bytes in, {bytes_out} PNG bytes out, {pixels} pixels; formats {formats:?}; PNG types {colour_types:?}; \
         {cubes} cube maps (face 0 only); {} noise-like (neighbour difference > 60): {:?}",
        all.len(),
        files.len(),
        if full { "all" } else { "every 16th" },
        failures.len(),
        noisy.len(),
        &noisy[..noisy.len().min(8)]
    );
    assert!(
        failures.is_empty(),
        "failures: {:?}",
        &failures[..failures.len().min(8)]
    );
    assert!(exported > 0 && formats.len() >= 4, "{formats:?}");
    if full {
        assert!(exported > 2400, "{exported}");
        assert!(unsupported >= 100, "{unsupported}");
    }
}

#[test]
fn every_model_of_the_install_is_exported_and_read_back() {
    let dir = ss2_testkit::require_game_data!();
    let source = Source::install(&dir).unwrap();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let files = source.list("", &["bmf"]).unwrap();
    let opts = Options {
        mesh: MeshOptions {
            lods: LodSel::All,
            format: GltfFormat::Glb,
            flip_winding: false,
            raw_vectors: false,
        },
        ..Options::default()
    };
    let (mut exported, mut unsupported, mut failures) = (0usize, 0usize, Vec::new());
    let (mut bytes_in, mut bytes_out) = (0u64, 0u64);
    let (mut prims, mut verts, mut tris, mut materials) = (0usize, 0usize, 0usize, 0usize);
    let (mut skinned_files, mut warned_files) = (0usize, 0usize);
    let mut warnings: BTreeMap<String, usize> = BTreeMap::new();
    let (mut tex_refs, mut tex_found) = (0usize, 0usize);
    let mut with_textures = 0usize;
    let mut missing = Vec::new();
    for file in &files {
        let bytes = source.read(file).unwrap();
        bytes_in += bytes.len() as u64;
        let converted = match convert(Kind::Mesh, &bytes, file, &source, &opts) {
            Ok(c) => c,
            Err(ExportError::Unsupported(_)) => {
                unsupported += 1;
                continue;
            }
            Err(e) => {
                failures.push(format!("{}: {e}", file.key));
                continue;
            }
        };
        exported += 1;
        bytes_out += converted.bytes.len() as u64;
        // Read back with the independent reader and compare with the engine's reader.
        let parsed = gltf::Gltf::from_slice(&converted.bytes)
            .unwrap_or_else(|e| panic!("{}: gltf crate: {e}", file.key));
        let blob = parsed.blob.clone().expect("BIN chunk");
        let mesh = Mesh::from_bytes(&bytes).unwrap();
        let mut want = mesh
            .lods
            .iter()
            .flat_map(|l| &l.surfaces)
            .filter(|s| !s.positions.is_empty() && !s.indices.is_empty());
        for prim in parsed.document.meshes().flat_map(|m| m.primitives()) {
            let s = want.next().expect("as many primitives as surfaces");
            let r = prim.reader(|_| Some(&blob[..]));
            let pos: Vec<[f32; 3]> = r.read_positions().unwrap().collect();
            assert_eq!(pos, s.positions, "{}", file.key);
            let idx: Vec<u32> = r.read_indices().unwrap().into_u32().collect();
            assert!(
                idx.iter().all(|&i| (i as usize) < pos.len()),
                "{}",
                file.key
            );
            assert_eq!(idx.len(), s.indices.len());
            assert_eq!(r.read_normals().unwrap().count(), pos.len());
            assert_eq!(
                (0..s.uv_sets.len())
                    .map(|n| r.read_tex_coords(n as u32).unwrap().into_f32().count())
                    .sum::<usize>(),
                s.uv_sets.iter().map(Vec::len).sum::<usize>()
            );
            assert_eq!(r.read_tangents().is_some(), s.tangents.is_some());
        }
        assert!(want.next().is_none(), "{}", file.key);
        let d = &converted.details;
        prims += d["primitives"].as_u64().unwrap() as usize;
        verts += d["vertices"].as_u64().unwrap() as usize;
        tris += d["triangles"].as_u64().unwrap() as usize;
        materials += d["materials"].as_u64().unwrap() as usize;
        skinned_files += usize::from(d["skinned_surfaces"].as_u64().unwrap() > 0);
        warned_files += usize::from(!converted.warnings.is_empty());
        for w in &converted.warnings {
            let kind = w
                .split(':')
                .next()
                .unwrap_or("")
                .chars()
                .filter(|c| !c.is_ascii_digit() && *c != '-')
                .collect::<String>();
            *warnings.entry(kind).or_default() += 1;
        }
        let textures = d["textures"].as_array().unwrap();
        with_textures += usize::from(!textures.is_empty());
        for t in textures {
            tex_refs += 1;
            let p = VfsPath::new(t.as_str().unwrap()).unwrap();
            let here = vfs.exists(&p);
            tex_found += usize::from(here);
            if !here {
                missing.push(format!("{} -> {p}", file.key));
            }
        }
        keep(&file.with_extension("glb"), &converted.bytes);
    }
    eprintln!(
        "models: {} in the install: {exported} exported, {unsupported} unsupported, {} failed; {bytes_in} bytes in, \
         {bytes_out} glb bytes out; {prims} primitives, {verts} vertices, {tris} triangles, {materials} materials; \
         {skinned_files} files with skinned surfaces; {warned_files} files with warnings {warnings:?}; \
         {with_textures} files reference textures, {tex_refs} references of which {tex_found} exist in the install; missing: {missing:?}",
        files.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "failures: {:?}",
        &failures[..failures.len().min(8)]
    );
    assert_eq!(exported, files.len());
    assert!(exported >= 70, "{exported}");
}
