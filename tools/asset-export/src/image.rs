//! Decoded pixels: BGR/BGRA and the block-compressed DXT1/3/5 (BC1/BC2/BC3) levels to RGBA8.
//!
//! The DXT decoders follow the public block layouts (a 4 x 4 block: two RGB565 end points and
//! 2-bit indices; BC2 adds 4-bit explicit alpha, BC3 two alpha end points with 3-bit indices).
//! Colours are expanded by bit replication and interpolated with integer division by 3, the common
//! software-decoder convention; a GPU may differ by one level per channel. Tests therefore state
//! the tolerance they use for block-compressed round trips.

use ss2_format_tex::{MipLevel, PixelFormat};

use crate::error::{ExportError, Result};

/// An 8-bit RGBA image, rows top to bottom (the order the `.tex` files store).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes: R, G, B, A.
    pub pixels: Vec<u8>,
}

impl Rgba {
    /// A fully transparent black image.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    /// `true` if any pixel has an alpha below 255.
    pub fn has_transparency(&self) -> bool {
        self.pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 255)
    }
}

/// Expand an RGB565 value to 8-bit channels by bit replication.
fn expand565(c: u16) -> [u8; 3] {
    let (r, g, b) = ((c >> 11) & 0x1F, (c >> 5) & 0x3F, c & 0x1F);
    [
        ((r << 3) | (r >> 2)) as u8,
        ((g << 2) | (g >> 4)) as u8,
        ((b << 3) | (b >> 2)) as u8,
    ]
}

/// The four colours of a BC1 colour block. `three_colour_mode` is the BC1 rule (`c0 <= c1` makes
/// colour 3 transparent black); BC2 and BC3 always use four opaque colours.
fn colour_table(block: &[u8], bc1: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (a, b) = (expand565(c0), expand565(c1));
    let mix = |wa: u16, wb: u16, div: u16| -> [u8; 4] {
        let ch = |i: usize| ((wa * u16::from(a[i]) + wb * u16::from(b[i])) / div) as u8;
        [ch(0), ch(1), ch(2), 255]
    };
    if bc1 && c0 <= c1 {
        [
            [a[0], a[1], a[2], 255],
            [b[0], b[1], b[2], 255],
            mix(1, 1, 2),
            [0, 0, 0, 0],
        ]
    } else {
        [
            [a[0], a[1], a[2], 255],
            [b[0], b[1], b[2], 255],
            mix(2, 1, 3),
            mix(1, 2, 3),
        ]
    }
}

/// The eight alpha values of a BC3 alpha block.
fn alpha_table(a0: u8, a1: u8) -> [u8; 8] {
    let (x, y) = (u16::from(a0), u16::from(a1));
    let mut t = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for i in 1..=6u16 {
            t[1 + i as usize] = (((7 - i) * x + i * y) / 7) as u8;
        }
    } else {
        for i in 1..=4u16 {
            t[1 + i as usize] = (((5 - i) * x + i * y) / 5) as u8;
        }
        t[6] = 0;
        t[7] = 255;
    }
    t
}

/// Decode one DXT block into `out` (a 4 x 4 pixel RGBA tile, row-major).
fn decode_block(format: PixelFormat, block: &[u8], out: &mut [[u8; 4]; 16]) {
    let (colour_block, alpha_block) = match format {
        PixelFormat::Dxt1 => (block, None),
        _ => (&block[8..16], Some(&block[0..8])),
    };
    let table = colour_table(colour_block, format == PixelFormat::Dxt1);
    let bits = u32::from_le_bytes([
        colour_block[4],
        colour_block[5],
        colour_block[6],
        colour_block[7],
    ]);
    for (i, px) in out.iter_mut().enumerate() {
        *px = table[((bits >> (2 * i)) & 3) as usize];
    }
    match (format, alpha_block) {
        (PixelFormat::Dxt3, Some(a)) => {
            for (i, px) in out.iter_mut().enumerate() {
                let nibble = (a[i / 2] >> (4 * (i % 2))) & 0xF;
                px[3] = nibble * 17;
            }
        }
        (PixelFormat::Dxt5, Some(a)) => {
            let table = alpha_table(a[0], a[1]);
            let mut bits = 0u64;
            for (i, byte) in a[2..8].iter().enumerate() {
                bits |= u64::from(*byte) << (8 * i);
            }
            for (i, px) in out.iter_mut().enumerate() {
                px[3] = table[((bits >> (3 * i)) & 7) as usize];
            }
        }
        _ => {}
    }
}

/// Convert one mip level to RGBA8.
///
/// # Errors
/// [`ExportError::Format`] if the level's byte count does not match its format and size (the
/// texture reader guarantees it; this keeps a hand-built [`MipLevel`] from panicking).
pub fn decode_level(format: PixelFormat, level: &MipLevel<'_>) -> Result<Rgba> {
    let (w, h) = (level.width as usize, level.height as usize);
    let expected = format.level_bytes(level.width, level.height);
    if level.data.len() as u64 != expected {
        return Err(ExportError::Format(format!(
            "a {w} x {h} {format:?} level needs {expected} bytes, it has {}",
            level.data.len()
        )));
    }
    let mut out = Rgba::new(level.width, level.height);
    match format {
        PixelFormat::Bgr8 => {
            let (src_px, _) = level.data.as_chunks::<3>();
            let (dst_px, _) = out.pixels.as_chunks_mut::<4>();
            for (src, dst) in src_px.iter().zip(dst_px.iter_mut()) {
                *dst = [src[2], src[1], src[0], 255];
            }
        }
        PixelFormat::Bgra8 => {
            let (src_px, _) = level.data.as_chunks::<4>();
            let (dst_px, _) = out.pixels.as_chunks_mut::<4>();
            for (src, dst) in src_px.iter().zip(dst_px.iter_mut()) {
                *dst = [src[2], src[1], src[0], src[3]];
            }
        }
        PixelFormat::Dxt1 | PixelFormat::Dxt3 | PixelFormat::Dxt5 => {
            let block_bytes = if format == PixelFormat::Dxt1 { 8 } else { 16 };
            let blocks_x = w.div_ceil(4).max(1);
            let mut tile = [[0u8; 4]; 16];
            for (n, block) in level.data.chunks_exact(block_bytes).enumerate() {
                let (bx, by) = ((n % blocks_x) * 4, (n / blocks_x) * 4);
                decode_block(format, block, &mut tile);
                for (i, px) in tile.iter().enumerate() {
                    let (x, y) = (bx + i % 4, by + i / 4);
                    if x < w && y < h {
                        let at = (y * w + x) * 4;
                        out.pixels[at..at + 4].copy_from_slice(px);
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(img: &Rgba, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * img.width + x) * 4) as usize;
        img.pixels[at..at + 4].try_into().unwrap()
    }

    fn level(w: u32, h: u32, data: &[u8]) -> MipLevel<'_> {
        MipLevel {
            width: w,
            height: h,
            data,
        }
    }

    #[test]
    fn bgr_and_bgra_swap_to_rgb_order() {
        let img = decode_level(PixelFormat::Bgr8, &level(2, 1, &[1, 2, 3, 4, 5, 6])).unwrap();
        assert_eq!(img.pixels, [3, 2, 1, 255, 6, 5, 4, 255]);
        let img =
            decode_level(PixelFormat::Bgra8, &level(1, 2, &[1, 2, 3, 9, 4, 5, 6, 8])).unwrap();
        assert_eq!(img.pixels, [3, 2, 1, 9, 6, 5, 4, 8]);
    }

    #[test]
    fn a_wrong_sized_level_is_an_error_not_a_panic() {
        assert!(decode_level(PixelFormat::Bgr8, &level(2, 2, &[0; 11])).is_err());
        assert!(decode_level(PixelFormat::Dxt5, &level(4, 4, &[0; 8])).is_err());
    }

    #[test]
    fn expand565_hits_the_extremes_and_replicates_bits() {
        assert_eq!(expand565(0xFFFF), [255, 255, 255]);
        assert_eq!(expand565(0), [0, 0, 0]);
        // r5 = 16 -> 0b10000 -> 0b10000100 = 132; g6 = 32 -> 0b100000 -> 0b10000010 = 130.
        assert_eq!(expand565(0x8410), [132, 130, 132]);
        assert_eq!(expand565(0xF800), [255, 0, 0]);
        assert_eq!(expand565(0x07E0), [0, 255, 0]);
        assert_eq!(expand565(0x001F), [0, 0, 255]);
    }

    /// Red and blue end points; one index per pixel, row by row.
    fn bc1_block(c0: u16, c1: u16, rows: [[u8; 4]; 4]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(c0.to_le_bytes());
        b.extend(c1.to_le_bytes());
        for r in rows {
            b.push(r[0] | (r[1] << 2) | (r[2] << 4) | (r[3] << 6));
        }
        b
    }

    #[test]
    fn dxt1_four_colour_block_by_hand() {
        let block = bc1_block(0xF800, 0x001F, [[0, 1, 2, 3], [3, 2, 1, 0], [0; 4], [3; 4]]);
        let img = decode_level(PixelFormat::Dxt1, &level(4, 4, &block)).unwrap();
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        let two_thirds_red = [170, 0, 85, 255];
        let one_third_red = [85, 0, 170, 255];
        assert_eq!(
            [0, 1, 2, 3].map(|x| px(&img, x, 0)),
            [red, blue, two_thirds_red, one_third_red]
        );
        assert_eq!(
            [0, 1, 2, 3].map(|x| px(&img, x, 1)),
            [one_third_red, two_thirds_red, blue, red]
        );
        assert!((0..4).all(|x| px(&img, x, 2) == red && px(&img, x, 3) == one_third_red));
    }

    #[test]
    fn dxt1_three_colour_block_has_a_transparent_fourth_colour() {
        let block = bc1_block(0x001F, 0xF800, [[0, 1, 2, 3]; 4]);
        let img = decode_level(PixelFormat::Dxt1, &level(4, 4, &block)).unwrap();
        assert_eq!(px(&img, 0, 0), [0, 0, 255, 255]);
        assert_eq!(px(&img, 1, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 2, 0), [127, 0, 127, 255]);
        assert_eq!(px(&img, 3, 0), [0, 0, 0, 0]);
        assert!(img.has_transparency());
    }

    #[test]
    fn equal_end_points_are_three_colour_mode_in_dxt1_only() {
        let block = bc1_block(0x07E0, 0x07E0, [[3; 4]; 4]);
        let dxt1 = decode_level(PixelFormat::Dxt1, &level(4, 4, &block)).unwrap();
        assert_eq!(px(&dxt1, 0, 0), [0, 0, 0, 0]);
        let mut dxt5 = vec![0xFF; 8];
        dxt5.extend(&block);
        let dxt5 = decode_level(PixelFormat::Dxt5, &level(4, 4, &dxt5)).unwrap();
        assert_eq!(px(&dxt5, 0, 0), [0, 255, 0, 255]);
    }

    #[test]
    fn dxt3_alpha_is_four_bits_per_pixel_low_nibble_first() {
        let colour = bc1_block(0xFFFF, 0xFFFF, [[0; 4]; 4]);
        let mut block: Vec<u8> = (0..8).map(|i| (2 * i) | ((2 * i + 1) << 4)).collect();
        block.extend(&colour);
        let img = decode_level(PixelFormat::Dxt3, &level(4, 4, &block)).unwrap();
        for i in 0..16u32 {
            assert_eq!(
                px(&img, i % 4, i / 4),
                [255, 255, 255, (i * 17) as u8],
                "{i}"
            );
        }
    }

    fn pack3(indices: [u8; 16]) -> [u8; 6] {
        let mut bits = 0u64;
        for (i, v) in indices.iter().enumerate() {
            bits |= u64::from(*v) << (3 * i);
        }
        bits.to_le_bytes()[..6].try_into().unwrap()
    }

    #[test]
    fn dxt5_eight_value_alpha_by_hand() {
        let mut block = vec![255, 0];
        block.extend(pack3([0, 1, 2, 3, 4, 5, 6, 7, 7, 6, 5, 4, 3, 2, 1, 0]));
        block.extend(bc1_block(0xFFFF, 0xFFFF, [[0; 4]; 4]));
        let img = decode_level(PixelFormat::Dxt5, &level(4, 4, &block)).unwrap();
        let alphas: Vec<u8> = (0..16).map(|i| px(&img, i % 4, i / 4)[3]).collect();
        assert_eq!(
            alphas,
            [
                255, 0, 218, 182, 145, 109, 72, 36, 36, 72, 109, 145, 182, 218, 0, 255
            ]
        );
    }

    #[test]
    fn dxt5_six_value_alpha_has_explicit_zero_and_opaque() {
        let mut block = vec![0, 255];
        block.extend(pack3([0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 0]));
        block.extend(bc1_block(0xFFFF, 0xFFFF, [[0; 4]; 4]));
        let img = decode_level(PixelFormat::Dxt5, &level(4, 4, &block)).unwrap();
        let alphas: Vec<u8> = (0..8).map(|i| px(&img, i % 4, i / 4)[3]).collect();
        assert_eq!(alphas, [0, 255, 51, 102, 153, 204, 0, 255]);
    }

    #[test]
    fn levels_smaller_than_a_block_are_cropped() {
        // 2 x 1 DXT1: one block, only the top-left 2 pixels are kept.
        let block = bc1_block(0xF800, 0x001F, [[0, 1, 2, 3]; 4]);
        let img = decode_level(PixelFormat::Dxt1, &level(2, 1, &block)).unwrap();
        assert_eq!(img.pixels, [255, 0, 0, 255, 0, 0, 255, 255]);
    }

    #[test]
    fn blocks_are_placed_row_major() {
        // 8 x 4: two blocks, the left red, the right blue (both all index 0).
        let mut data = bc1_block(0xF800, 0x0000, [[0; 4]; 4]);
        data.extend(bc1_block(0x001F, 0x0000, [[0; 4]; 4]));
        let img = decode_level(PixelFormat::Dxt1, &level(8, 4, &data)).unwrap();
        assert_eq!(px(&img, 3, 3), [255, 0, 0, 255]);
        assert_eq!(px(&img, 4, 0), [0, 0, 255, 255]);
        // 4 x 8: two blocks stacked.
        let img = decode_level(PixelFormat::Dxt1, &level(4, 8, &data)).unwrap();
        assert_eq!(px(&img, 0, 3), [255, 0, 0, 255]);
        assert_eq!(px(&img, 0, 4), [0, 0, 255, 255]);
    }

    #[test]
    fn dxt5_equal_alpha_end_points_use_the_six_value_table() {
        // a0 == a1 is not "a0 > a1": the six-value mode, whose indices 6 and 7 are 0 and 255.
        let mut block = vec![100, 100];
        block.extend(pack3([6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 6, 7, 6, 7, 6, 7]));
        block.extend(bc1_block(0xFFFF, 0xFFFF, [[0; 4]; 4]));
        let img = decode_level(PixelFormat::Dxt5, &level(4, 4, &block)).unwrap();
        let alphas: Vec<u8> = (0..8).map(|i| px(&img, i % 4, i / 4)[3]).collect();
        assert_eq!(alphas, [0, 255, 100, 100, 100, 100, 100, 100]);
    }

    #[test]
    fn dxt5_six_value_alpha_interpolates_between_unequal_end_points() {
        let mut block = vec![10, 210];
        block.extend(pack3([2, 3, 4, 5, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1]));
        block.extend(bc1_block(0xFFFF, 0xFFFF, [[0; 4]; 4]));
        let img = decode_level(PixelFormat::Dxt5, &level(4, 4, &block)).unwrap();
        let alphas: Vec<u8> = (0..4).map(|i| px(&img, i, 0)[3]).collect();
        // (4*10+210)/5, (3*10+2*210)/5, (2*10+3*210)/5, (10+4*210)/5
        assert_eq!(alphas, [50, 90, 130, 170]);
    }
}
