//! `.tex` to PNG: level and face selection, channel handling and the PNG encoder.

use ss2_format_tex::{PixelFormat, Texture};

use crate::error::{ExportError, Result};
use crate::image::{Rgba, decode_level};

/// Which face of a cube map to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceSel {
    /// One face (0 for a texture that is not a cube map).
    One(u32),
    /// All faces side by side in one wide image, in file order.
    All,
}

/// What the PNG holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channels {
    /// RGB for 24-bit textures and for DXT1 without transparent pixels, RGBA for the rest.
    Auto,
    /// Always RGBA.
    Rgba,
    /// Colour only: the alpha channel is dropped (the colour is not changed).
    RgbOnly,
    /// The alpha channel only, as a grayscale image.
    AlphaOnly,
}

/// Options of a texture export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexOptions {
    /// Mip level to write (0 is the largest).
    pub mip: u32,
    /// Face to write.
    pub face: FaceSel,
    /// Channel handling.
    pub channels: Channels,
}

impl Default for TexOptions {
    fn default() -> Self {
        Self {
            mip: 0,
            face: FaceSel::One(0),
            channels: Channels::Auto,
        }
    }
}

/// What was written, for reports and manifests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TexInfo {
    /// Width of the written image.
    pub width: u32,
    /// Height of the written image.
    pub height: u32,
    /// The pixel format of the `.tex` file.
    pub format: PixelFormat,
    /// Width of the file's largest level.
    pub source_width: u32,
    /// Height of the file's largest level.
    pub source_height: u32,
    /// Mip levels in the file.
    pub mip_levels: u32,
    /// Faces in the file (6 for a cube map).
    pub faces: u32,
    /// `true` if the PNG has an alpha channel (or is the alpha channel).
    pub png_has_alpha: bool,
}

/// A PNG and what it contains.
#[derive(Clone, Debug)]
pub struct TexExport {
    /// The PNG file.
    pub png: Vec<u8>,
    /// The facts about it.
    pub info: TexInfo,
}

/// Decode the selected level (and face) of `tex` to RGBA8.
///
/// # Errors
/// [`ExportError::Option`] if the level or face does not exist; [`ExportError::Format`] if the
/// level cannot be decoded.
pub fn decode_selection(tex: &Texture, mip: u32, face: FaceSel) -> Result<Rgba> {
    if mip >= tex.mip_levels {
        return Err(ExportError::Option(format!(
            "mip level {mip} does not exist: the texture has {} level{}",
            tex.mip_levels,
            if tex.mip_levels == 1 { "" } else { "s" }
        )));
    }
    let faces: Vec<u32> = match face {
        FaceSel::One(f) if f < tex.faces => vec![f],
        FaceSel::One(f) => {
            return Err(ExportError::Option(format!(
                "face {f} does not exist: the texture has {} face{}",
                tex.faces,
                if tex.faces == 1 { "" } else { "s" }
            )));
        }
        FaceSel::All => (0..tex.faces).collect(),
    };
    let mut images = Vec::with_capacity(faces.len());
    for f in faces {
        let level = tex
            .level(f, mip)
            .ok_or_else(|| ExportError::Format(format!("face {f} level {mip} is missing")))?;
        images.push(decode_level(tex.format, &level)?);
    }
    let (w, h) = (images[0].width, images[0].height);
    let mut strip = Rgba::new(w * images.len() as u32, h);
    let row = w as usize * 4;
    for (n, img) in images.iter().enumerate() {
        for y in 0..h as usize {
            let dst = (y * strip.width as usize + n * w as usize) * 4;
            strip.pixels[dst..dst + row].copy_from_slice(&img.pixels[y * row..(y + 1) * row]);
        }
    }
    Ok(strip)
}

/// Encode an RGBA image as a PNG according to `channels`. Returns the PNG and whether it carries
/// alpha.
///
/// # Errors
/// [`ExportError::Encode`] if the encoder refuses the image.
pub fn encode_png(img: &Rgba, channels: Channels, format: PixelFormat) -> Result<(Vec<u8>, bool)> {
    let (color, alpha, data): (png::ColorType, bool, Vec<u8>) = match channels {
        Channels::AlphaOnly => (
            png::ColorType::Grayscale,
            true,
            img.pixels.as_chunks::<4>().0.iter().map(|p| p[3]).collect(),
        ),
        Channels::RgbOnly => (png::ColorType::Rgb, false, strip_alpha(img)),
        Channels::Rgba => (png::ColorType::Rgba, true, img.pixels.clone()),
        Channels::Auto => {
            let rgb = format == PixelFormat::Bgr8
                || (format == PixelFormat::Dxt1 && !img.has_transparency());
            if rgb {
                (png::ColorType::Rgb, false, strip_alpha(img))
            } else {
                (png::ColorType::Rgba, true, img.pixels.clone())
            }
        }
    };
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(color);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc
        .write_header()
        .map_err(|e| ExportError::Encode(format!("PNG header: {e}")))?;
    writer
        .write_image_data(&data)
        .map_err(|e| ExportError::Encode(format!("PNG data: {e}")))?;
    writer
        .finish()
        .map_err(|e| ExportError::Encode(format!("PNG end: {e}")))?;
    Ok((out, alpha))
}

fn strip_alpha(img: &Rgba) -> Vec<u8> {
    img.pixels
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect()
}

/// Read the `.tex` file in `bytes` and write the selected level as a PNG.
///
/// # Errors
/// The reader's refusal ([`ExportError::Format`], or [`ExportError::Unsupported`] for the old
/// development textures), an unusable option, or an encoder failure.
pub fn export_texture(bytes: &[u8], opts: &TexOptions) -> Result<TexExport> {
    let tex = Texture::from_bytes(bytes).map_err(|e| ExportError::from_reader(&e))?;
    let img = decode_selection(&tex, opts.mip, opts.face)?;
    let (png, png_has_alpha) = encode_png(&img, opts.channels, tex.format)?;
    Ok(TexExport {
        png,
        info: TexInfo {
            width: img.width,
            height: img.height,
            format: tex.format,
            source_width: tex.width,
            source_height: tex.height,
            mip_levels: tex.mip_levels,
            faces: tex.faces,
            png_has_alpha,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;

    /// Decode a PNG with the `png` crate: (width, height, colour type, bytes).
    fn decode_png(bytes: &[u8]) -> (u32, u32, png::ColorType, Vec<u8>) {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        buf.truncate(info.buffer_size());
        (info.width, info.height, info.color_type, buf)
    }

    fn export(file: &[u8], opts: &TexOptions) -> (u32, u32, png::ColorType, Vec<u8>) {
        decode_png(&export_texture(file, opts).unwrap().png)
    }

    fn with_channels(channels: Channels) -> TexOptions {
        TexOptions {
            channels,
            ..TexOptions::default()
        }
    }

    #[test]
    fn bgr_textures_round_trip_exactly_for_every_format_code() {
        // 5 x 3 so that nothing is a power of two or a multiple of four.
        let bgr: Vec<u8> = (0..5 * 3 * 3).map(|i| (i * 11 % 256) as u8).collect();
        let rgb: Vec<u8> = bgr.chunks(3).flat_map(|p| [p[2], p[1], p[0]]).collect();
        for code in [0, 2, 6] {
            let file = texture_file(5, 3, 0x20, code, (0x50, 0x50), &bgr);
            let (w, h, color, pixels) = export(&file, &TexOptions::default());
            assert_eq!((w, h, color), (5, 3, png::ColorType::Rgb), "code {code}");
            assert_eq!(pixels, rgb, "code {code}");
        }
    }

    #[test]
    fn bgra_textures_round_trip_exactly_for_every_format_code() {
        let bgra: Vec<u8> = (0..4 * 4 * 4).map(|i| (i * 7 + 3) as u8).collect();
        let rgba: Vec<u8> = bgra
            .chunks(4)
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect();
        for code in [1, 3, 4, 5, 11] {
            let file = texture_file(4, 4, 0x04, code, (0x50, 0x50), &bgra);
            let (w, h, color, pixels) = export(&file, &TexOptions::default());
            assert_eq!((w, h, color), (4, 4, png::ColorType::Rgba), "code {code}");
            assert_eq!(pixels, rgba, "code {code}");
        }
    }

    #[test]
    fn the_first_stored_pixel_is_the_top_left_one() {
        // B, G, R = (1, 2, 3) first, then pixels with other values.
        let mut bgr = vec![1u8, 2, 3];
        bgr.extend([9u8; 3 * 3]);
        let file = texture_file(2, 2, 0x20, 0, (0x50, 0x50), &bgr);
        let (_, _, _, px) = export(&file, &TexOptions::default());
        assert_eq!(&px[..3], &[3, 2, 1]);
        assert_eq!(&px[3..6], &[9, 9, 9]);
    }

    fn gradient(w: usize, h: usize, alpha: bool) -> Vec<u8> {
        let mut px = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let g = (x * 8 + y) as u8;
                px.extend([g, g, g, if alpha { (x * 8) as u8 } else { 255 }]);
            }
        }
        px
    }

    fn max_error(a: &[u8], b: &[u8]) -> u8 {
        assert_eq!(a.len(), b.len());
        a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap()
    }

    #[test]
    fn dxt1_round_trips_within_a_documented_tolerance() {
        let src = gradient(16, 16, false);
        let blocks = encode_dxt(&src, 16, 16, encode_bc1);
        let file = dxt_file(7, 16, 16, 0x21, &blocks, 0);
        let (w, h, color, px) = export(&file, &TexOptions::default());
        assert_eq!((w, h, color), (16, 16, png::ColorType::Rgb));
        let expect: Vec<u8> = src.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        // A gray ramp of at most 24 levels per block, RGB565 end points, 4 colours: within 8.
        let err = max_error(&px, &expect);
        assert!(err <= 8, "{err}");
    }

    #[test]
    fn two_colour_dxt1_blocks_are_exact_when_the_colours_fit_rgb565() {
        // Left half pure red, right half pure blue: both are exact in RGB565.
        let mut src = Vec::new();
        for _y in 0..4 {
            for x in 0..4 {
                src.extend(if x < 2 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                });
            }
        }
        let file = dxt_file(7, 4, 4, 0x21, &encode_dxt(&src, 4, 4, encode_bc1), 0);
        let (_, _, _, px) = export(&file, &TexOptions::default());
        let expect: Vec<u8> = src.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        assert_eq!(px, expect);
    }

    #[test]
    fn dxt3_and_dxt5_keep_colour_and_alpha_within_tolerance() {
        let src = gradient(8, 8, true);
        type Encoder = fn(&[[u8; 4]; 16]) -> Vec<u8>;
        for (code, encode, tolerance) in [(8, encode_bc2 as Encoder, 16u8), (9, encode_bc3, 8u8)] {
            let file = dxt_file(code, 8, 8, 0x21, &encode_dxt(&src, 8, 8, encode), 0);
            let (w, h, color, px) = export(&file, &TexOptions::default());
            assert_eq!((w, h, color), (8, 8, png::ColorType::Rgba), "code {code}");
            let err = max_error(&px, &src);
            assert!(err <= tolerance, "code {code}: error {err} > {tolerance}");
        }
        // DXT3 alpha has 4 bits: 0xA0 is stored as 0xA and comes back as 0xAA.
        let one = [[0u8, 0, 0, 0xA0]; 16];
        let file = dxt_file(8, 4, 4, 0x21, &encode_bc2(&one), 0);
        let (_, _, _, px) = export(&file, &TexOptions::default());
        assert_eq!(px[3], 0xAA);
    }

    #[test]
    fn mip_levels_are_selectable() {
        // 8 x 8 DXT1, levels 8, 4, 2, 1: each level one flat colour.
        let flat = |c: u16| {
            let mut b = c.to_le_bytes().to_vec();
            b.extend(c.to_le_bytes());
            b.extend([0u8; 4]);
            b
        };
        let mut data = Vec::new();
        for _ in 0..4 {
            data.extend(flat(0xF800)); // level 0: four red blocks
        }
        data.extend(flat(0x07E0)); // 4 x 4 green
        data.extend(flat(0x001F)); // 2 x 2 blue
        data.extend(flat(0xFFFF)); // 1 x 1 white
        let file = texture_file(8, 8, 0x21, 7, (0x50, 0x50), &data);
        let at = |mip| {
            let opts = TexOptions {
                mip,
                ..TexOptions::default()
            };
            let (w, h, _, px) = export(&file, &opts);
            (w, h, px[..3].to_vec())
        };
        assert_eq!(at(0), (8, 8, vec![255, 0, 0]));
        assert_eq!(at(1), (4, 4, vec![0, 255, 0]));
        assert_eq!(at(2), (2, 2, vec![0, 0, 255]));
        assert_eq!(at(3), (1, 1, vec![255, 255, 255]));
        let opts = TexOptions {
            mip: 4,
            ..TexOptions::default()
        };
        let err = export_texture(&file, &opts).unwrap_err();
        assert!(matches!(err, ExportError::Option(_)));
        assert!(err.to_string().contains("mip level 4") && err.to_string().contains("4 levels"));
        // A 24-bit texture has one level only.
        let bgr = texture_file(2, 2, 0x20, 0, (0x50, 0x50), &[0; 12]);
        let opts = TexOptions {
            mip: 1,
            ..TexOptions::default()
        };
        let err = export_texture(&bgr, &opts).unwrap_err();
        assert!(err.to_string().contains("has 1 level"), "{err}");
    }

    #[test]
    fn cube_faces_are_selectable_and_all_gives_a_strip_in_file_order() {
        let file = cube_file();
        let expect = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 0],
            [255, 0, 255],
            [0, 255, 255],
        ];
        for (n, rgb) in expect.iter().enumerate() {
            let opts = TexOptions {
                face: FaceSel::One(n as u32),
                ..TexOptions::default()
            };
            let (w, h, _, px) = export(&file, &opts);
            assert_eq!((w, h), (4, 4));
            assert_eq!(&px[..3], rgb, "face {n}");
        }
        let all = TexOptions {
            face: FaceSel::All,
            ..TexOptions::default()
        };
        let (w, h, _, px) = export(&file, &all);
        assert_eq!((w, h), (24, 4));
        for (n, rgb) in expect.iter().enumerate() {
            for y in 0..4usize {
                let at = (y * 24 + n * 4) * 3;
                assert_eq!(&px[at..at + 3], rgb, "face {n} row {y}");
                let end = (y * 24 + n * 4 + 3) * 3;
                assert_eq!(&px[end..end + 3], rgb, "face {n} row {y} last");
            }
        }
        // Mip 1 of face 2 is 2 x 2.
        let opts = TexOptions {
            mip: 1,
            face: FaceSel::One(2),
            ..TexOptions::default()
        };
        let (w, h, _, px) = export(&file, &opts);
        assert_eq!((w, h, &px[..3]), (2, 2, &[0u8, 0, 255][..]));
        // Face 6 does not exist; a plain texture has face 0 only, and `all` is then one image.
        let opts = TexOptions {
            face: FaceSel::One(6),
            ..TexOptions::default()
        };
        let err = export_texture(&file, &opts).unwrap_err();
        assert!(err.to_string().contains("face 6") && err.to_string().contains("6 faces"));
        let plain = bgr_file();
        let opts = TexOptions {
            face: FaceSel::One(1),
            ..TexOptions::default()
        };
        assert!(export_texture(&plain, &opts).is_err());
        let (w, h, _, _) = export(&plain, &all);
        assert_eq!((w, h), (8, 4));
    }

    #[test]
    fn channel_handling() {
        let opaque = dxt1_file();
        // DXT1 without transparent pixels: RGB by default.
        assert_eq!(
            export(&opaque, &TexOptions::default()).2,
            png::ColorType::Rgb
        );
        // Forced RGBA: alpha 255 everywhere.
        let (_, _, color, px) = export(&opaque, &with_channels(Channels::Rgba));
        assert_eq!(color, png::ColorType::Rgba);
        assert!(px.chunks(4).all(|p| p[3] == 255));
        // DXT1 with a transparent pixel (three-colour mode, index 3): RGBA by default.
        let mut block = 0x001Fu16.to_le_bytes().to_vec();
        block.extend(0xF800u16.to_le_bytes());
        block.extend([0xFF, 0, 0, 0]); // the four pixels of row 0 use index 3
        let file = dxt_file(7, 4, 4, 0x21, &block, 0);
        let (_, _, color, px) = export(&file, &TexOptions::default());
        assert_eq!(
            (color, px[3], px[4 * 4 + 3]),
            (png::ColorType::Rgba, 0, 255)
        );
        // 32-bit: RGB drops alpha but keeps the colour; alpha-only is a gray image of the alpha.
        let bgra = bgra_file();
        let (_, _, color, rgb) = export(&bgra, &with_channels(Channels::RgbOnly));
        assert_eq!(color, png::ColorType::Rgb);
        let (_, _, _, rgba) = export(&bgra, &TexOptions::default());
        let dropped: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        assert_eq!(rgb, dropped);
        let (_, _, color, a) = export(&bgra, &with_channels(Channels::AlphaOnly));
        assert_eq!(color, png::ColorType::Grayscale);
        let alphas: Vec<u8> = rgba.chunks(4).map(|p| p[3]).collect();
        assert_eq!(a, alphas);
        assert_eq!(alphas[5], 5 * 16);
        // 24-bit with alpha-only: opaque white.
        let (_, _, _, a) = export(&bgr_file(), &with_channels(Channels::AlphaOnly));
        assert!(a.iter().all(|&v| v == 255));
    }

    #[test]
    fn info_describes_the_file_and_the_png() {
        let out = export_texture(&cube_file(), &TexOptions::default()).unwrap();
        let i = out.info;
        assert_eq!(
            (i.width, i.height, i.source_width, i.source_height),
            (4, 4, 4, 4)
        );
        assert_eq!(
            (i.format, i.mip_levels, i.faces, i.png_has_alpha),
            (PixelFormat::Dxt1, 3, 6, false)
        );
    }

    #[test]
    fn output_is_deterministic() {
        let file = bgra_file();
        let a = export_texture(&file, &TexOptions::default()).unwrap().png;
        let b = export_texture(&file, &TexOptions::default()).unwrap().png;
        assert_eq!(a, b);
        assert!(a.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn reader_errors_are_typed() {
        assert!(matches!(
            export_texture(b"junk", &TexOptions::default()),
            Err(ExportError::Format(_))
        ));
        let old = ss2_core::Error::invalid(".tex texture", "format version 6: not supported");
        assert!(matches!(
            ExportError::from_reader(&old),
            ExportError::Unsupported(_)
        ));
        let other = ss2_core::Error::invalid(".tex texture", "unknown format code 10");
        assert!(matches!(
            ExportError::from_reader(&other),
            ExportError::Format(_)
        ));
        // A damaged texture (wrong pixel count) is a format error, not a panic.
        let bad = texture_file(4, 4, 0x20, 0, (0x50, 0x50), &[0; 47]);
        assert!(matches!(
            export_texture(&bad, &TexOptions::default()),
            Err(ExportError::Format(_))
        ));
    }

    #[test]
    fn every_truncation_and_single_byte_corruption_is_survived() {
        let all = TexOptions {
            face: FaceSel::All,
            channels: Channels::Rgba,
            mip: 0,
        };
        for good in [dxt1_file(), bgra_file(), cube_file()] {
            for len in 0..good.len() {
                let _ = export_texture(&good[..len], &all);
            }
            for i in 0..good.len() {
                for flip in [0xFFu8, 0x01, 0x80] {
                    let mut bad = good.clone();
                    bad[i] ^= flip;
                    let _ = export_texture(&bad, &all);
                }
            }
        }
    }
}
