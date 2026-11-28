//! Software decoding of every pixel layout of [`PixelFormat`] to RGBA8 (roadmap T02-043).
//!
//! The block formats follow their public description (Microsoft, "Block Compression
//! (Direct3D 10)" and "Compressed texture formats"; Khronos Data Format Specification, "S3TC
//! compressed texture image formats"):
//!
//! - A block covers 4 x 4 texels; blocks are stored row of blocks after row of blocks, left to
//!   right. Inside a block texel `i` is at column `i % 4`, row `i / 4`.
//! - The colour half (all of DXT1, the last 8 bytes of DXT3 and DXT5) is two little-endian
//!   RGB 5:6:5 words `color_0` and `color_1` (red in the top 5 bits) and a little-endian 32-bit
//!   word of 2-bit indices, texel 0 in the lowest bits. The 5- and 6-bit channels widen to 8 bits
//!   by repeating their top bits (`v << 3 | v >> 2`, `v << 2 | v >> 4`).
//! - DXT1: if `color_0 > color_1` (as unsigned words) the palette is `c0, c1, (2 c0 + c1) / 3,
//!   (c0 + 2 c1) / 3`, all opaque; otherwise `c0, c1, (c0 + c1) / 2` and index 3 is transparent
//!   black `(0, 0, 0, 0)`.
//! - DXT3 and DXT5 always use the four-colour palette, whatever the order of the two colours.
//! - DXT3 alpha: the first 8 bytes are a little-endian 64-bit word of 4-bit alphas, texel 0 in
//!   the lowest bits; a 4-bit alpha `a` becomes `a * 17`.
//! - DXT5 alpha: bytes `alpha_0`, `alpha_1`, then a little-endian 48-bit word of 3-bit indices,
//!   texel 0 in the lowest bits. If `alpha_0 > alpha_1` the palette is the two values and six
//!   interpolated ones (`((7 - k) a0 + k a1) / 7` for `k` 1 to 6); otherwise four interpolated
//!   ones (`((5 - k) a0 + k a1) / 5` for `k` 1 to 4), then 0 and 255.
//!
//! The interpolations work on the widened 8-bit values and round to the nearest integer.
//! Texels of a block that fall outside a level whose side is not a multiple of 4 (the small mip
//! levels: 2 x 2, 1 x 1, …) are dropped.

use ss2_core::Result;

use crate::{MAX_SIDE, MipLevel, PixelFormat, Texture, bad};

/// A level decoded to 8-bit RGBA.
///
/// [`RgbaImage::pixels`] holds `width * height` texels of 4 bytes each, in the byte order
/// **red, green, blue, alpha** (whatever the stored order), rows top to bottom, each row left to
/// right, without padding. Formats without alpha (24-bit BGR, opaque DXT1 texels) have alpha 255.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA, rows top to bottom.
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// The texel at column `x`, row `y` as `[r, g, b, a]`; `None` outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        let p = self.pixels.get(i..i + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }
}

/// Decode `data`, one level of `width x height` texels stored as `format`, to RGBA8.
///
/// `data` must be exactly [`PixelFormat::level_bytes`] long. See the module documentation for
/// the block rules and [`RgbaImage`] for the output.
///
/// # Errors
/// [`ss2_core::Error::Invalid`] if a side is 0 or above the largest the crate reads, or if the
/// length of `data` does not match the size and format.
pub fn decode_pixels(
    format: PixelFormat,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<RgbaImage> {
    if !(1..=MAX_SIDE).contains(&width) || !(1..=MAX_SIDE).contains(&height) {
        return Err(bad(format!(
            "cannot decode a level of {width} x {height}: each side must be between 1 and {MAX_SIDE}"
        )));
    }
    let expected = format.level_bytes(width, height);
    if data.len() as u64 != expected {
        return Err(bad(format!(
            "a {width} x {height} {format:?} level has {expected} bytes, {} given",
            data.len()
        )));
    }
    let (w, h) = (width as usize, height as usize);
    let out_len = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| bad("the decoded level does not fit in memory"))?;
    let mut pixels = vec![0u8; out_len];
    match format {
        PixelFormat::Bgr8 => {
            let src = data.as_chunks::<3>().0;
            for (dst, &[b, g, r]) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(src) {
                *dst = [r, g, b, 255];
            }
        }
        PixelFormat::Bgra8 => {
            let src = data.as_chunks::<4>().0;
            for (dst, &[b, g, r, a]) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(src) {
                *dst = [r, g, b, a];
            }
        }
        PixelFormat::Dxt1 => {
            for (b, block) in data.as_chunks::<8>().0.iter().enumerate() {
                place(&mut pixels, w, h, b, &decode_bc1_block(block));
            }
        }
        PixelFormat::Dxt3 => {
            for (b, block) in data.as_chunks::<16>().0.iter().enumerate() {
                place(&mut pixels, w, h, b, &decode_bc2_block(block));
            }
        }
        PixelFormat::Dxt5 => {
            for (b, block) in data.as_chunks::<16>().0.iter().enumerate() {
                place(&mut pixels, w, h, b, &decode_bc3_block(block));
            }
        }
    }
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

/// Write the texels of block `b` (blocks counted row by row) of a `w x h` RGBA image, dropping
/// the texels outside it.
fn place(pixels: &mut [u8], w: usize, h: usize, b: usize, texels: &[[u8; 4]; 16]) {
    let blocks_x = w.div_ceil(4);
    let (bx, by) = ((b % blocks_x) * 4, (b / blocks_x) * 4);
    for (i, texel) in texels.iter().enumerate() {
        let (x, y) = (bx + i % 4, by + i / 4);
        if x < w && y < h {
            let at = (y * w + x) * 4;
            if let Some(dst) = pixels.get_mut(at..at + 4) {
                dst.copy_from_slice(texel);
            }
        }
    }
}

/// Widen an RGB 5:6:5 word to 8-bit `[r, g, b]`.
fn rgb565(word: u16) -> [u8; 3] {
    let r = ((word >> 11) & 0x1F) as u8;
    let g = ((word >> 5) & 0x3F) as u8;
    let b = (word & 0x1F) as u8;
    [r << 3 | r >> 2, g << 2 | g >> 4, b << 3 | b >> 2]
}

/// `(wa * a + wb * b) / (wa + wb)` rounded to the nearest integer, per channel.
fn mix(a: [u8; 3], b: [u8; 3], wa: u32, wb: u32) -> [u8; 3] {
    let total = wa + wb;
    let one = |x: u8, y: u8| ((wa * u32::from(x) + wb * u32::from(y) + total / 2) / total) as u8;
    [one(a[0], b[0]), one(a[1], b[1]), one(a[2], b[2])]
}

/// The 16 texels of a colour half (8 bytes). `punch_through` allows the three-colour mode.
fn color_block(block: &[u8; 8], punch_through: bool) -> [[u8; 4]; 16] {
    let w0 = u16::from_le_bytes([block[0], block[1]]);
    let w1 = u16::from_le_bytes([block[2], block[3]]);
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    let (c0, c1) = (rgb565(w0), rgb565(w1));
    let opaque = |c: [u8; 3]| [c[0], c[1], c[2], 255];
    let palette = if w0 > w1 || !punch_through {
        [
            opaque(c0),
            opaque(c1),
            opaque(mix(c0, c1, 2, 1)),
            opaque(mix(c0, c1, 1, 2)),
        ]
    } else {
        [opaque(c0), opaque(c1), opaque(mix(c0, c1, 1, 1)), [0; 4]]
    };
    std::array::from_fn(|i| palette[(indices >> (2 * i)) as usize & 3])
}

/// Decode one DXT1 / BC1 block (8 bytes) to 16 RGBA texels, texel `i` at column `i % 4`, row
/// `i / 4`. Both the four-colour and the three-colour (punch-through) mode.
pub fn decode_bc1_block(block: &[u8; 8]) -> [[u8; 4]; 16] {
    color_block(block, true)
}

/// Decode one DXT3 / BC2 block (16 bytes: explicit 4-bit alpha, then a colour half).
pub fn decode_bc2_block(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let (alpha, color) = split(block);
    let alphas = u64::from_le_bytes(alpha);
    let mut texels = color_block(&color, false);
    for (i, t) in texels.iter_mut().enumerate() {
        t[3] = ((alphas >> (4 * i)) & 0xF) as u8 * 17;
    }
    texels
}

/// Decode one DXT5 / BC3 block (16 bytes: interpolated alpha, then a colour half). Both the
/// eight-value (`alpha_0 > alpha_1`) and the six-value mode.
pub fn decode_bc3_block(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let (alpha, color) = split(block);
    let (a0, a1) = (u32::from(alpha[0]), u32::from(alpha[1]));
    let lerp = |k: u32, n: u32| (((n - k) * a0 + k * a1 + n / 2) / n) as u8;
    let palette: [u8; 8] = if a0 > a1 {
        std::array::from_fn(|i| match i {
            0 => a0 as u8,
            1 => a1 as u8,
            _ => lerp(i as u32 - 1, 7),
        })
    } else {
        std::array::from_fn(|i| match i {
            0 => a0 as u8,
            1 => a1 as u8,
            6 => 0,
            7 => 255,
            _ => lerp(i as u32 - 1, 5),
        })
    };
    let mut bits = [0u8; 8];
    bits[..6].copy_from_slice(&alpha[2..8]);
    let indices = u64::from_le_bytes(bits);
    let mut texels = color_block(&color, false);
    for (i, t) in texels.iter_mut().enumerate() {
        t[3] = palette[(indices >> (3 * i)) as usize & 7];
    }
    texels
}

/// The alpha half and the colour half of a 16-byte block.
fn split(block: &[u8; 16]) -> ([u8; 8], [u8; 8]) {
    (
        std::array::from_fn(|i| block[i]),
        std::array::from_fn(|i| block[i + 8]),
    )
}

impl MipLevel<'_> {
    /// Decode this level, stored as `format`, to RGBA8 ([`decode_pixels`]).
    ///
    /// # Errors
    /// As [`decode_pixels`].
    pub fn decode(&self, format: PixelFormat) -> Result<RgbaImage> {
        decode_pixels(format, self.width, self.height, self.data)
    }
}

impl Texture {
    /// Decode mip level `level` (0 is the largest) of the first face to RGBA8. Use
    /// [`Texture::decode_face_level`] for the other faces of a cube map.
    ///
    /// # Errors
    /// [`ss2_core::Error::Invalid`] if the level does not exist or its bytes do not match the
    /// texture's size and format.
    pub fn decode_level(&self, level: u32) -> Result<RgbaImage> {
        self.decode_face_level(0, level)
    }

    /// Decode mip level `level` of face `face` to RGBA8 ([`decode_pixels`]).
    ///
    /// # Errors
    /// As [`Texture::decode_level`].
    pub fn decode_face_level(&self, face: u32, level: u32) -> Result<RgbaImage> {
        let mip = self.level(face, level).ok_or_else(|| {
            bad(format!(
                "no level {level} of face {face} (the texture has {} levels and {} faces)",
                self.mip_levels, self.faces
            ))
        })?;
        mip.decode(self.format)
    }
}

#[cfg(test)]
mod tests;
