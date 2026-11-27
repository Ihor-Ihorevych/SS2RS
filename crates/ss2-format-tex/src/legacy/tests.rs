//! The legacy layouts of `tex-legacy.md`, on synthetic files (no game data), and the retail
//! corpus (data-gated).

use crate::testing::{Legacy, zlib};
use crate::{PixelFormat, Texture};

fn pattern(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 13 % 251) as u8).collect()
}

fn read(l: &Legacy) -> Texture {
    Texture::from_bytes(&l.build()).unwrap_or_else(|e| panic!("{l:?}: {e}"))
}

fn err(l: &Legacy) -> String {
    match Texture::from_bytes(&l.build()) {
        Ok(t) => panic!("read although broken: {t:?}"),
        Err(e) => e.to_string(),
    }
}

/// Bytes of a whole mip chain of `format` from `w x h` down to 1 x 1.
fn chain(format: PixelFormat, w: u32, h: u32) -> usize {
    let levels = u32::BITS - w.max(h).leading_zeros();
    (0..levels)
        .map(|l| format.level_bytes((w >> l).max(1), (h >> l).max(1)) as usize)
        .sum()
}

#[test]
fn the_spec_vector_of_layout_5_is_two_bgr_pixels() {
    let t = read(&Legacy::layout5(2, 1, 0, &[3, 2, 1, 6, 5, 4]));
    assert_eq!(
        (t.width, t.height, t.format, t.mip_levels, t.faces),
        (2, 1, PixelFormat::Bgr8, 1, 1)
    );
    assert_eq!((t.format_version, t.frames), (6, 1));
    // BGR: the pixels (1, 2, 3) and (4, 5, 6) as RGB.
    let rgb: Vec<[u8; 3]> = t.data.chunks(3).map(|c| [c[2], c[1], c[0]]).collect();
    assert_eq!(rgb, [[1, 2, 3], [4, 5, 6]]);
    assert_eq!(
        (t.wrap_u, t.wrap_v, t.border_color),
        (0x50, 0x51, 0x0080_8080)
    );
    assert!(t.palette.is_empty());
}

#[test]
fn the_spec_vector_of_layout_6_is_one_zlib_bgra_pixel() {
    let blob = Legacy::layout6_blob(&[3, 2, 1, 0xFF]);
    let t = read(&Legacy::with_format(6, 1, 1, 1, &blob));
    assert_eq!(
        (t.format, t.data.as_slice()),
        (PixelFormat::Bgra8, &[3, 2, 1, 255][..])
    );
    assert_eq!((t.format_code, t.flags, t.frames), (1, 0x01, 1));
}

#[test]
fn layout_5_selects_the_pixels_by_the_stored_format() {
    for (stored, format) in [
        (0, PixelFormat::Bgr8),
        (1, PixelFormat::Bgra8),
        (2, PixelFormat::Dxt1),
        (3, PixelFormat::Dxt3),
        (4, PixelFormat::Dxt5),
    ] {
        let px = pattern(if format.is_compressed() {
            chain(format, 8, 4)
        } else {
            format.level_bytes(8, 4) as usize
        });
        // The quality code `.6` (here 9, a DXT5 `TexFormat` code) says nothing about the bytes.
        let mut l = Legacy::layout5(8, 4, stored, &px);
        l.code = 9;
        let t = read(&l);
        assert_eq!((t.format, t.data.as_slice()), (format, px.as_slice()));
        let levels = if format.is_compressed() { 4 } else { 1 };
        assert_eq!(t.mip_levels, levels, "{format:?}");
        assert_eq!(t.format_code, 9, "the quality code is kept as stored");
    }
    assert!(err(&Legacy::layout5(4, 4, 5, &pattern(64))).contains("stored format 5"));
}

#[test]
fn version_4_names_its_members_and_reads_the_same() {
    for version in [4, 5, 6] {
        let mut l = Legacy::layout5(4, 2, 1, &pattern(32));
        l.version = version;
        let t = read(&l);
        assert_eq!(
            (t.format_version, t.width, t.height, t.format),
            (version, 4, 2, PixelFormat::Bgra8)
        );
        assert_eq!((t.wrap_u, t.wrap_v, t.flags), (0x50, 0x51, 0x21));
        assert_eq!(t.data, pattern(32));
    }
}

#[test]
fn layouts_6_7_and_9_select_the_pixels_by_the_tex_format() {
    for (code, format) in [
        (2, PixelFormat::Bgr8),
        (6, PixelFormat::Bgr8),
        (1, PixelFormat::Bgra8),
        (4, PixelFormat::Bgra8),
        (5, PixelFormat::Bgra8),
        (7, PixelFormat::Dxt1),
        (8, PixelFormat::Dxt3),
        (9, PixelFormat::Dxt5),
    ] {
        let px = pattern(if format.is_compressed() {
            chain(format, 4, 8)
        } else {
            format.level_bytes(4, 8) as usize
        });
        let px = px.as_slice();
        for (layout, blob) in [
            (6, Legacy::layout6_blob(px)),
            (7, px.to_vec()),
            (9, zlib(px)),
        ] {
            let t = read(&Legacy::with_format(layout, 4, 8, code, &blob));
            assert_eq!((t.format, t.format_code), (format, code), "layout {layout}");
            assert_eq!(t.data, px, "layout {layout} code {code}");
        }
    }
}

#[test]
fn layout_9_is_raw_without_flag_0x20() {
    let px = pattern(4 * 4 * 3);
    let mut l = Legacy::with_format(9, 4, 4, 2, &px);
    l.flags = 0x01;
    let t = read(&l);
    assert_eq!(
        (t.format, t.data.as_slice()),
        (PixelFormat::Bgr8, px.as_slice())
    );
    // With the bit set the same raw bytes are not a zlib stream.
    l.flags = 0x21;
    assert!(err(&l).contains("zlib"));
}

#[test]
fn a_raw_bgra_texture_may_carry_its_whole_mip_chain() {
    // The observed layout-7 case: 256 x 128, base 131,072 bytes, nine levels 174,764 bytes.
    let px = pattern(174_764);
    let t = read(&Legacy::with_format(7, 256, 128, 1, &px));
    assert_eq!(
        (t.format, t.mip_levels, t.data.len()),
        (PixelFormat::Bgra8, 9, 174_764)
    );
    let second = t.level(0, 1).unwrap();
    assert_eq!((second.width, second.height), (128, 64));
    assert_eq!(second.data, &px[131_072..131_072 + 128 * 64 * 4]);
    let last = t.level(0, 8).unwrap();
    assert_eq!((last.width, last.height, last.data), (1, 1, &px[174_760..]));
    // The base level alone is the ordinary case.
    let t = read(&Legacy::with_format(7, 256, 128, 1, &px[..131_072]));
    assert_eq!((t.mip_levels, t.data.len()), (1, 131_072));
}

#[test]
fn the_color_texture_reads_its_baked_base_level() {
    // `Noise_02.tex`: version 7, layout 9, flags 0x21, `TexFormat` 6, zlib BGR.
    let px = pattern(16 * 16 * 3);
    let mut l = Legacy::with_format(9, 16, 16, 6, &zlib(&px));
    l.color_texture = true;
    let t = read(&l);
    assert_eq!(
        (t.format, t.flags, t.format_version, t.data.as_slice()),
        (PixelFormat::Bgr8, 0x21, 7, px.as_slice())
    );
}

#[test]
fn a_legacy_cube_map_has_six_dxt1_chains() {
    let px = pattern(6 * chain(PixelFormat::Dxt1, 8, 8));
    let mut l = Legacy::with_format(9, 8, 8, 7, &zlib(&px));
    l.flags = 0x23;
    let t = read(&l);
    assert!(t.is_cube_map());
    assert_eq!((t.faces, t.mip_levels, t.face_bytes()), (6, 4, 56));
    assert_eq!(t.level(4, 0).unwrap().data, &px[4 * 56..4 * 56 + 32]);
}

#[test]
fn frames_and_palette_are_exposed_and_the_first_frame_is_kept() {
    // Two frames back to back (unobserved: `tex-legacy.md` leaves their order open).
    let px = pattern(2 * 4 * 4 * 4);
    let mut l = Legacy::with_format(6, 4, 4, 1, &Legacy::layout6_blob(&px));
    l.frames = 2;
    let t = read(&l);
    assert_eq!((t.frames, t.data.as_slice()), (2, &px[..64]));
    let mut l = Legacy::with_format(7, 4, 4, 1, &px);
    l.frames = 2;
    assert_eq!(read(&l).frames, 2);
    // A layout-5 palette is kept as stored; the pixels still follow the stored format.
    let mut l = Legacy::layout5(2, 1, 0, &[3, 2, 1, 6, 5, 4]);
    l.palette = vec![0x1122_3344, 0x5566_7788];
    let t = read(&l);
    assert_eq!(t.palette, [0x1122_3344, 0x5566_7788]);
    assert_eq!(t.data, [3, 2, 1, 6, 5, 4]);
    // Format 8 has neither.
    let v8 = crate::testing::texture_file(1, 1, 0x20, 0, 0x50, 0x50, &[1, 2, 3]);
    let t = Texture::from_bytes(&v8).unwrap();
    assert_eq!((t.frames, t.palette.len(), t.format_version), (1, 0, 8));
}

#[test]
fn broken_legacy_files_are_refused_with_a_reason() {
    // Layout 6: the inner count must be the outer count minus four.
    let mut blob = Legacy::layout6_blob(&[3, 2, 1, 0xFF]);
    blob[0] = blob[0].wrapping_add(1);
    assert!(err(&Legacy::with_format(6, 1, 1, 1, &blob)).contains("compressed byte count"));
    assert!(err(&Legacy::with_format(6, 1, 1, 1, &[1, 0])).contains("compressed byte count"));
    // A stream with bytes after its end, a truncated stream, the wrong inflated size.
    let mut z = zlib(&[3, 2, 1, 0xFF]);
    z.push(0);
    assert!(err(&Legacy::with_format(9, 1, 1, 1, &z)).contains("after the end"));
    let z = zlib(&pattern(64));
    assert!(err(&Legacy::with_format(9, 4, 4, 1, &z[..z.len() - 3])).contains("zlib"));
    assert!(err(&Legacy::with_format(9, 4, 4, 1, &zlib(&pattern(60)))).contains("60 bytes"));
    assert!(err(&Legacy::with_format(9, 2, 2, 1, &zlib(&pattern(60)))).contains("more than"));
    // Raw pixels of the wrong size.
    assert!(err(&Legacy::with_format(7, 4, 4, 1, &pattern(63))).contains("63 bytes"));
    assert!(err(&Legacy::layout5(2, 1, 0, &[1, 2, 3])).contains("3 bytes"));
    // Unknown class layout, frame count, sizes, format code; a cube map that is not DXT.
    let mut l = Legacy::with_format(7, 1, 1, 1, &[0; 4]);
    l.layout = 8;
    assert!(err(&l).contains("class layout 8"));
    let mut l = Legacy::with_format(7, 1, 1, 1, &[0; 4]);
    l.frames = 0;
    assert!(err(&l).contains("frame count 0"));
    assert!(err(&Legacy::with_format(7, 0, 1, 1, &[])).contains("size 0 x 1"));
    assert!(err(&Legacy::with_format(7, 1, 1, 10, &[0; 4])).contains("format code 10"));
    let mut l = Legacy::with_format(9, 1, 1, 1, &zlib(&[0; 24]));
    l.flags = 0x23;
    assert!(err(&l).contains("cube map"));
    // A decompression bomb is cut off at the predicted size.
    let bomb = zlib(&vec![0u8; 10_000_000]);
    assert!(err(&Legacy::with_format(9, 4, 4, 1, &bomb)).contains("more than"));
}

#[test]
fn nothing_panics_on_damaged_legacy_files() {
    let mut files = vec![
        Legacy::layout5(4, 2, 1, &pattern(32)).build(),
        Legacy::with_format(6, 4, 4, 9, &Legacy::layout6_blob(&pattern(48))).build(),
        Legacy::with_format(9, 4, 4, 7, &zlib(&pattern(24))).build(),
    ];
    let mut v4 = Legacy::layout5(2, 2, 0, &pattern(12));
    v4.version = 4;
    files.push(v4.build());
    for good in files {
        Texture::from_bytes(&good).unwrap();
        for len in 0..good.len() {
            let _ = Texture::from_bytes(&good[..len]);
        }
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            let _ = Texture::from_bytes(&bad);
        }
    }
}

/// Data-gated: every physical version 4–7 `.tex` entry of the two patches that hold them
/// (`tex-legacy.md`, "Evidence": 155 entries, versions 20/4/113/18, class layouts 72/63/3/17,
/// one `CColorTexture`) decodes to the size the spec predicts. Skips without `SS2_DATA_DIR`.
#[test]
fn every_legacy_texture_entry_of_the_install_decodes() {
    use ss2_vfs::{ArchiveVfs, EntryKind, Vfs, VfsPath};
    use std::collections::BTreeMap;
    let dir = ss2_testkit::require_game_data!();
    let mut versions: BTreeMap<u32, usize> = BTreeMap::new();
    let mut layouts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    let (mut cubes, mut chains, mut color, mut failures) = (0, 0, 0, Vec::new());
    for name in ["Patch_02_090_593613.gro", "Patch_02_100_800000.gro"] {
        let vfs = ArchiveVfs::from_file(dir.join(name)).unwrap();
        let mut stack = vec![VfsPath::root()];
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if !path
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("tex"))
                {
                    continue;
                }
                let bytes = vfs.read(&path).unwrap();
                let container = ss2_meta::Container::parse(&bytes).unwrap();
                if container.header.format_version == 8 {
                    continue;
                }
                let layout = container
                    .types
                    .iter()
                    .find(|t| t.name == "CBaseTexture")
                    .map(|t| t.kind)
                    .unwrap_or_default();
                *layouts.entry(layout).or_default() += 1;
                color += usize::from(container.types.iter().any(|t| t.name == "CColorTexture"));
                match Texture::from_bytes(&bytes) {
                    Ok(t) => {
                        *versions.entry(t.format_version).or_default() += 1;
                        *formats.entry(format!("{:?}", t.format)).or_default() += 1;
                        cubes += usize::from(t.is_cube_map());
                        chains += usize::from(!t.format.is_compressed() && t.mip_levels > 1);
                        assert_eq!((t.frames, t.palette.len()), (1, 0), "{}", path.as_str());
                    }
                    Err(e) => failures.push(format!("{name}/{}: {e}", path.as_str())),
                }
            }
        }
    }
    eprintln!(
        "versions {versions:?}; layouts {layouts:?}; formats {formats:?}; {cubes} cube maps; \
         {chains} uncompressed with a mip chain; {color} CColorTexture; failures {failures:#?}"
    );
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(
        versions.into_iter().collect::<Vec<_>>(),
        [(4, 20), (5, 4), (6, 113), (7, 18)]
    );
    assert_eq!(
        layouts.into_iter().collect::<Vec<_>>(),
        [(5, 72), (6, 63), (7, 3), (9, 17)]
    );
    assert_eq!((cubes, chains, color), (1, 1, 1));
}
