use super::*;
use crate::testing::texture_file;

const RED: u16 = 0xF800;
const GREEN: u16 = 0x07E0;
const BLUE: u16 = 0x001F;
const WHITE: u16 = 0xFFFF;
const BLACK: u16 = 0x0000;

/// A colour half: the two 5:6:5 words, then the index word, all little-endian.
fn color_half(c0: u16, c1: u16, indices: u32) -> [u8; 8] {
    let mut b = [0u8; 8];
    b[0..2].copy_from_slice(&c0.to_le_bytes());
    b[2..4].copy_from_slice(&c1.to_le_bytes());
    b[4..8].copy_from_slice(&indices.to_le_bytes());
    b
}

fn block16(alpha: [u8; 8], color: [u8; 8]) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&alpha);
    b[8..].copy_from_slice(&color);
    b
}

/// Index byte `0b11_10_01_00`: the texels of a row take the indices 0, 1, 2, 3.
const ROW_0123: u32 = 0xE4E4_E4E4;

#[test]
fn bc1_four_colour_block() {
    // color_0 > color_1: red, blue and the two thirds between them, all opaque.
    let t = decode_bc1_block(&color_half(RED, BLUE, ROW_0123));
    for row in 0..4 {
        assert_eq!(t[row * 4], [255, 0, 0, 255]);
        assert_eq!(t[row * 4 + 1], [0, 0, 255, 255]);
        assert_eq!(t[row * 4 + 2], [170, 0, 85, 255]);
        assert_eq!(t[row * 4 + 3], [85, 0, 170, 255]);
    }
}

#[test]
fn bc1_three_colour_punch_through_block() {
    // color_0 < color_1: blue, red, their half, and index 3 is transparent black.
    let t = decode_bc1_block(&color_half(BLUE, RED, ROW_0123));
    assert_eq!(t[0], [0, 0, 255, 255]);
    assert_eq!(t[1], [255, 0, 0, 255]);
    assert_eq!(t[2], [128, 0, 128, 255]);
    assert_eq!(t[3], [0, 0, 0, 0]);
    // Equal colours are the three-colour mode too.
    let t = decode_bc1_block(&color_half(GREEN, GREEN, 0xFFFF_FFFF));
    assert!(t.iter().all(|&p| p == [0, 0, 0, 0]));
    let t = decode_bc1_block(&color_half(GREEN, GREEN, 0xAAAA_AAAA));
    assert!(t.iter().all(|&p| p == [0, 255, 0, 255]));
}

#[test]
fn colours_widen_by_repeating_their_top_bits() {
    // 0x8410: red 16, green 32, blue 16 of 31 / 63 / 31.
    let t = decode_bc1_block(&color_half(0x8410, BLACK, 0));
    assert_eq!(t[0], [132, 130, 132, 255]);
    let t = decode_bc1_block(&color_half(WHITE, BLACK, 0));
    assert_eq!(t[0], [255, 255, 255, 255]);
}

#[test]
fn texels_are_stored_row_by_row_lowest_bits_first() {
    // Index bytes per row: row 0 all 0, row 1 all 1, row 2 all 2, row 3 all 3.
    let t = decode_bc1_block(&color_half(WHITE, BLACK, 0xFFAA_5500));
    let rows: Vec<_> = (0..4).map(|r| t[r * 4][0]).collect();
    assert_eq!(rows, [255, 0, 170, 85]);
    for r in 0..4 {
        assert!(t[r * 4..r * 4 + 4].iter().all(|&p| p == t[r * 4]));
    }
    // A single texel: texel 5 (column 1, row 1) has index 1, the rest 0.
    let t = decode_bc1_block(&color_half(WHITE, BLACK, 1 << 10));
    for (i, p) in t.iter().enumerate() {
        assert_eq!(p[0], if i == 5 { 0 } else { 255 }, "texel {i}");
    }
}

#[test]
fn bc2_explicit_alpha_and_four_colour_mode() {
    // Texel i has the 4-bit alpha i (low nibble first), widened to i * 17.
    let alpha = [0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE];
    // color_0 < color_1 would be punch-through in DXT1; DXT3 keeps four colours.
    let t = decode_bc2_block(&block16(alpha, color_half(BLACK, WHITE, ROW_0123)));
    for (i, p) in t.iter().enumerate() {
        assert_eq!(p[3], i as u8 * 17, "texel {i}");
    }
    assert_eq!(
        &t[..4].iter().map(|p| p[0]).collect::<Vec<_>>(),
        &[0, 255, 85, 170]
    );
}

/// 3-bit indices 0..7, 0..7 packed little-endian.
const ALPHA_01234567: [u8; 6] = [0x88, 0xC6, 0xFA, 0x88, 0xC6, 0xFA];

fn bc3_alpha(a0: u8, a1: u8, indices: [u8; 6]) -> [u8; 8] {
    let mut a = [a0, a1, 0, 0, 0, 0, 0, 0];
    a[2..].copy_from_slice(&indices);
    a
}

#[test]
fn bc3_eight_value_alpha() {
    let t = decode_bc3_block(&block16(
        bc3_alpha(255, 0, ALPHA_01234567),
        color_half(RED, BLUE, 0),
    ));
    let alphas: Vec<u8> = t.iter().map(|p| p[3]).collect();
    let palette = [255, 0, 219, 182, 146, 109, 73, 36];
    assert_eq!(&alphas[..8], &palette);
    assert_eq!(&alphas[8..], &palette);
    assert!(t.iter().all(|p| p[..3] == [255, 0, 0]));
}

#[test]
fn bc3_six_value_alpha() {
    let t = decode_bc3_block(&block16(
        bc3_alpha(40, 140, ALPHA_01234567),
        color_half(BLUE, RED, ROW_0123),
    ));
    let alphas: Vec<u8> = t.iter().map(|p| p[3]).collect();
    assert_eq!(&alphas[..8], &[40, 140, 60, 80, 100, 120, 0, 255]);
    // DXT5 keeps four colours even when color_0 < color_1: index 3 is not transparent.
    assert_eq!(t[3][..3], [170, 0, 85]);
    // Equal alphas are the six-value mode: indices 6 and 7 are 0 and 255.
    let t = decode_bc3_block(&block16(
        bc3_alpha(9, 9, ALPHA_01234567),
        color_half(0, 0, 0),
    ));
    let alphas: Vec<u8> = t.iter().map(|p| p[3]).collect();
    assert_eq!(&alphas[..8], &[9, 9, 9, 9, 9, 9, 0, 255]);
}

#[test]
fn raw_formats_swap_to_rgba() {
    let bgr = [1, 2, 3, 4, 5, 6];
    let img = decode_pixels(PixelFormat::Bgr8, 2, 1, &bgr).unwrap();
    assert_eq!(img.pixels, [3, 2, 1, 255, 6, 5, 4, 255]);
    let bgra = [1, 2, 3, 4, 5, 6, 7, 8];
    let img = decode_pixels(PixelFormat::Bgra8, 1, 2, &bgra).unwrap();
    assert_eq!(img.pixels, [3, 2, 1, 4, 7, 6, 5, 8]);
    assert_eq!(img.pixel(0, 1), Some([7, 6, 5, 8]));
    assert_eq!(img.pixel(1, 0), None);
}

#[test]
fn blocks_are_laid_out_row_of_blocks_after_row() {
    // 8 x 8 DXT1: four solid blocks red, green, blue, white.
    let mut data = Vec::new();
    for c in [RED, GREEN, BLUE, WHITE] {
        data.extend_from_slice(&color_half(c, BLACK, 0));
    }
    let img = decode_pixels(PixelFormat::Dxt1, 8, 8, &data).unwrap();
    assert_eq!(img.pixel(3, 3), Some([255, 0, 0, 255]));
    assert_eq!(img.pixel(4, 0), Some([0, 255, 0, 255]));
    assert_eq!(img.pixel(0, 4), Some([0, 0, 255, 255]));
    assert_eq!(img.pixel(7, 7), Some([255, 255, 255, 255]));
}

#[test]
fn partial_blocks_at_the_edges_drop_the_outside_texels() {
    // 6 x 5 DXT5: 2 x 2 blocks; each texel's colour index says which block it came from.
    let mut data = Vec::new();
    for (c, a) in [(RED, 10), (GREEN, 20), (BLUE, 30), (WHITE, 40)] {
        data.extend_from_slice(&block16(bc3_alpha(a, a, [0; 6]), color_half(c, BLACK, 0)));
    }
    let img = decode_pixels(PixelFormat::Dxt5, 6, 5, &data).unwrap();
    assert_eq!(img.pixels.len(), 6 * 5 * 4);
    assert_eq!(img.pixel(5, 0), Some([0, 255, 0, 20]));
    assert_eq!(img.pixel(0, 4), Some([0, 0, 255, 30]));
    assert_eq!(img.pixel(5, 4), Some([255, 255, 255, 40]));
    // The 2 x 2 and 1 x 1 levels of a DXT1 chain each read the top-left corner of one block.
    let level = color_half(WHITE, BLACK, 0x0000_0504); // (1,0) index 1, (1,1) index 1
    let one = decode_pixels(PixelFormat::Dxt1, 2, 2, &level).unwrap();
    assert_eq!(one.pixels.len(), 16);
    assert_eq!(one.pixel(0, 0), Some([255, 255, 255, 255]));
    assert_eq!(one.pixel(1, 0), Some([0, 0, 0, 255]));
    assert_eq!(one.pixel(1, 1), Some([0, 0, 0, 255]));
    let dot = decode_pixels(PixelFormat::Dxt1, 1, 1, &level).unwrap();
    assert_eq!(dot.pixels, [255, 255, 255, 255]);
}

/// A tiny encoder for the round trips: each block uses only its first two distinct texels as
/// endpoints, which suits images made of colours 5:6:5 represents exactly.
fn encode_two_colour(format: PixelFormat, w: u32, h: u32, rgba: &[[u8; 4]]) -> Vec<u8> {
    let to565 = |p: [u8; 4]| {
        (u16::from(p[0] >> 3) << 11) | (u16::from(p[1] >> 2) << 5) | u16::from(p[2] >> 3)
    };
    let mut out = Vec::new();
    for by in (0..h).step_by(4) {
        for bx in (0..w).step_by(4) {
            let texel = |i: u32| {
                let (x, y) = ((bx + i % 4).min(w - 1), (by + i / 4).min(h - 1));
                rgba[(y * w + x) as usize]
            };
            let first = texel(0);
            let second = (0..16)
                .map(texel)
                .find(|&p| to565(p) != to565(first))
                .unwrap_or(first);
            // Order the endpoints so DXT1 stays in four-colour mode (color_0 > color_1).
            let (c0, c1) = if to565(first) >= to565(second) {
                (first, second)
            } else {
                (second, first)
            };
            let mut indices = 0u32;
            let mut alpha4 = 0u64;
            let mut alpha3 = 0u64;
            for i in 0..16 {
                let p = texel(i);
                let idx = u32::from(to565(p) != to565(c0));
                indices |= idx << (2 * i);
                alpha4 |= u64::from(p[3] / 17) << (4 * i);
                alpha3 |= u64::from(p[3] == 0) << (3 * i); // DXT5: alpha_0 = 255, alpha_1 = 0
            }
            match format {
                PixelFormat::Dxt1 => {}
                PixelFormat::Dxt3 => out.extend_from_slice(&alpha4.to_le_bytes()),
                _ => {
                    out.extend_from_slice(&[255, 0]);
                    out.extend_from_slice(&alpha3.to_le_bytes()[..6]);
                }
            }
            let (w0, w1) = (to565(c0), to565(c1));
            // Four-colour DXT1 needs w0 > w1; a solid block gets a darker partner.
            let (w0, w1) = if w0 == w1 && format == PixelFormat::Dxt1 {
                if w0 == 0 { (1, 0) } else { (w0, w0 - 1) }
            } else {
                (w0, w1)
            };
            out.extend_from_slice(&color_half(w0, w1, indices));
        }
    }
    out
}

#[test]
fn tiny_images_round_trip() {
    // Colours 5:6:5 holds exactly.
    let palette = [
        [255, 0, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
        [0, 255, 0, 255],
    ];
    for (w, h) in [(4, 4), (8, 4), (4, 8), (8, 8), (2, 2), (1, 1), (6, 3)] {
        // Two colours per block: a checker of palette[block % 4] and black.
        let image: Vec<[u8; 4]> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let block = (x / 4 + y / 4) as usize;
                if (x + y) % 2 == 0 {
                    palette[block % 4]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect();
        for format in [PixelFormat::Dxt1, PixelFormat::Dxt3, PixelFormat::Dxt5] {
            let data = encode_two_colour(format, w, h, &image);
            let img = decode_pixels(format, w, h, &data).unwrap();
            let got = img.pixels.as_chunks::<4>().0.to_vec();
            assert_eq!(got, image, "{format:?} {w} x {h}");
        }
        // The raw formats: BGR and BGRA bytes of the same image.
        let bgra: Vec<u8> = image
            .iter()
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect();
        let bgr: Vec<u8> = image.iter().flat_map(|p| [p[2], p[1], p[0]]).collect();
        let flat: Vec<u8> = image.iter().flatten().copied().collect();
        assert_eq!(
            decode_pixels(PixelFormat::Bgra8, w, h, &bgra)
                .unwrap()
                .pixels,
            flat
        );
        assert_eq!(
            decode_pixels(PixelFormat::Bgr8, w, h, &bgr).unwrap().pixels,
            flat
        );
    }
    // Alpha survives: DXT3 in steps of 17, DXT5 at 0 and 255.
    let image: Vec<[u8; 4]> = (0..16u8).map(|i| [255, 0, 0, i * 17]).collect();
    let data = encode_two_colour(PixelFormat::Dxt3, 4, 4, &image);
    let img = decode_pixels(PixelFormat::Dxt3, 4, 4, &data).unwrap();
    assert_eq!(img.pixels.as_chunks::<4>().0, image.as_slice());
    let image: Vec<[u8; 4]> = (0..16u8)
        .map(|i| [0, 0, 255, if i % 3 == 0 { 0 } else { 255 }])
        .collect();
    let data = encode_two_colour(PixelFormat::Dxt5, 4, 4, &image);
    let img = decode_pixels(PixelFormat::Dxt5, 4, 4, &data).unwrap();
    assert_eq!(img.pixels.as_chunks::<4>().0, image.as_slice());
}

#[test]
fn a_texture_decodes_every_level_and_face() {
    // DXT1 4 x 4 with its 2 x 2 and 1 x 1 levels: red, green, blue blocks.
    let mut px = Vec::new();
    for c in [RED, GREEN, BLUE] {
        px.extend_from_slice(&color_half(c, BLACK, 0));
    }
    let t = Texture::from_bytes(&texture_file(4, 4, 0x21, 7, 0x50, 0x50, &px)).unwrap();
    let sizes: Vec<_> = (0..3)
        .map(|l| t.decode_level(l).unwrap())
        .map(|i| (i.width, i.height, i.pixels.len(), i.pixel(0, 0).unwrap()))
        .collect();
    assert_eq!(
        sizes,
        [
            (4, 4, 64, [255, 0, 0, 255]),
            (2, 2, 16, [0, 255, 0, 255]),
            (1, 1, 4, [0, 0, 255, 255]),
        ]
    );
    // A cube map: face 4 is its own chain.
    let mut px = Vec::new();
    for face in 0..6u16 {
        for _ in 0..3 {
            px.extend_from_slice(&color_half(face * 0x0841 + 1, 0, 0));
        }
    }
    let t = Texture::from_bytes(&texture_file(4, 4, 0x23, 7, 0x51, 0x51, &px)).unwrap();
    let face4 = t.decode_face_level(4, 0).unwrap();
    // 0x2105: red 4, green 8, blue 5.
    assert_eq!(face4.pixel(0, 0), Some([33, 32, 41, 255]));
    assert_eq!(t.level(4, 0).unwrap().decode(t.format).unwrap(), face4);
    assert_ne!(face4, t.decode_level(0).unwrap());
    // A raw texture through the same call.
    let bgr: Vec<u8> = (0..8 * 4 * 3).map(|i| i as u8).collect();
    let t = Texture::from_bytes(&texture_file(8, 4, 0x20, 0, 0x50, 0x50, &bgr)).unwrap();
    let img = t.decode_level(0).unwrap();
    assert_eq!(img.pixel(1, 0), Some([5, 4, 3, 255]));
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    let msg = |r: Result<RgbaImage>| r.unwrap_err().to_string();
    // Wrong lengths for every format, both too short and too long.
    for format in [
        PixelFormat::Bgr8,
        PixelFormat::Bgra8,
        PixelFormat::Dxt1,
        PixelFormat::Dxt3,
        PixelFormat::Dxt5,
    ] {
        let n = format.level_bytes(4, 4) as usize;
        assert!(msg(decode_pixels(format, 4, 4, &vec![0; n - 1])).contains("given"));
        assert!(msg(decode_pixels(format, 4, 4, &vec![0; n + 1])).contains("given"));
        assert!(decode_pixels(format, 4, 4, &vec![0xA5; n]).is_ok());
        assert!(msg(decode_pixels(format, 0, 4, &[])).contains("between 1 and"));
        assert!(msg(decode_pixels(format, 4, MAX_SIDE + 1, &[])).contains("between 1 and"));
    }
    // Levels and faces that do not exist.
    let t = Texture::from_bytes(&texture_file(4, 4, 0x21, 7, 0x50, 0x50, &[0; 24])).unwrap();
    assert!(msg(t.decode_level(3)).contains("no level 3"));
    assert!(msg(t.decode_face_level(1, 0)).contains("face 1"));
    // Any bytes of the right length decode.
    for seed in 0..64u8 {
        let data: Vec<u8> = (0..48)
            .map(|i: u8| i.wrapping_mul(seed).wrapping_add(seed))
            .collect();
        decode_pixels(PixelFormat::Dxt5, 7, 5, &data[..32]).unwrap_err();
        assert_eq!(
            decode_pixels(PixelFormat::Dxt5, 7, 3, &data[..32])
                .unwrap()
                .pixels
                .len(),
            84
        );
        assert_eq!(
            decode_pixels(PixelFormat::Dxt1, 5, 7, &data[..32])
                .unwrap()
                .pixels
                .len(),
            140
        );
    }
}

/// Data-gated: the base level of every texture of one level's model folder decodes to an image
/// of the texture's size, and so does a sample of the rest of the install (every 20th texture,
/// plus the smallest level and the last cube face). Skips without `SS2_DATA_DIR`.
#[test]
fn real_textures_decode() {
    use ss2_vfs::{EntryKind, Vfs, VfsPath};
    use std::collections::BTreeMap;
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let folder = "content/seriousSam2/models/levels/01_mdigbo/".to_ascii_lowercase();
    let mut stack = vec![VfsPath::root()];
    let (mut seen, mut in_folder, mut failures) = (0usize, 0usize, Vec::new());
    let mut worst_mip = 0f64;
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    while let Some(d) = stack.pop() {
        for e in vfs.list(&d).unwrap() {
            let path = d.join(&e.name).unwrap();
            if e.kind == EntryKind::Dir {
                stack.push(path);
                continue;
            }
            if path.extension() != Some("tex") {
                continue;
            }
            seen += 1;
            let mine = path.as_str().to_ascii_lowercase().starts_with(&folder);
            if !mine && seen % 20 != 0 {
                continue;
            }
            in_folder += usize::from(mine);
            let t = Texture::from_bytes(&vfs.read(&path).unwrap()).unwrap();
            let mut check = |face: u32, level: u32| match t.decode_face_level(face, level) {
                Ok(img) => {
                    let (w, h) = ((t.width >> level).max(1), (t.height >> level).max(1));
                    if (img.width, img.height, img.pixels.len()) != (w, h, (w * h * 4) as usize) {
                        failures.push(format!(
                            "{}: size {} x {}",
                            path.as_str(),
                            img.width,
                            img.height
                        ));
                    }
                }
                Err(e) => failures.push(format!("{}: {e}", path.as_str())),
            };
            check(0, 0);
            check(t.faces - 1, t.mip_levels - 1);
            if mine && t.format.is_compressed() && t.width >= 8 && t.height >= 8 {
                let err = mip_error(&t.decode_level(0).unwrap(), &t.decode_level(1).unwrap());
                worst_mip = worst_mip.max(err);
                if err > 16.0 {
                    failures.push(format!(
                        "{}: level 1 is {err:.1} off level 0",
                        path.as_str()
                    ));
                }
            }
            *formats.entry(format!("{:?}", t.format)).or_default() += 1;
        }
    }
    // Channel order of the raw layout: this menu background is blue (`tex.md`).
    let blue = vfs
        .read(&VfsPath::new("Content/SeriousSam2/Textures/Menu/Background_Blue_PC.tex").unwrap())
        .unwrap();
    let blue = Texture::from_bytes(&blue).unwrap().decode_level(0).unwrap();
    let sum = |c: usize| {
        blue.pixels
            .iter()
            .skip(c)
            .step_by(4)
            .map(|&v| u64::from(v))
            .sum::<u64>()
    };
    assert!(
        sum(2) > 2 * sum(0),
        "the blue background decodes as r {} b {}",
        sum(0),
        sum(2)
    );
    eprintln!(
        "worst mean difference between a DXT level 1 and the 2 x 2 average of its level 0: {worst_mip:.2}"
    );
    eprintln!(
        "decoded {formats:?}; {in_folder} from {folder}; {} failures",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "first failures: {:?}",
        &failures[..failures.len().min(5)]
    );
    assert!(in_folder >= 10, "only {in_folder} textures under {folder}");
    assert_eq!(
        formats.len(),
        5,
        "not every pixel format was sampled: {formats:?}"
    );
}

/// Mean absolute difference per channel (alpha included) between `small` and the 2 x 2 box
/// average of `big`: a decoder that reads the blocks wrongly gives noise, far from the stored
/// mip level (`tex.md` measured 1 to 4 of 255 for the real files).
fn mip_error(big: &RgbaImage, small: &RgbaImage) -> f64 {
    let mut total = 0u64;
    for y in 0..small.height {
        for x in 0..small.width {
            let s = small.pixel(x, y).unwrap();
            for (c, &have) in s.iter().enumerate() {
                let sum: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .iter()
                    .map(|&(dx, dy)| u32::from(big.pixel(2 * x + dx, 2 * y + dy).unwrap()[c]))
                    .sum();
                total += u64::from((sum + 2) / 4).abs_diff(u64::from(have));
            }
        }
    }
    total as f64 / f64::from(small.width * small.height * 4)
}
