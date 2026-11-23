//! Contact sheets: many textures as thumbnails in one PNG, over a checkerboard so alpha shows.
//!
//! Thumbnails keep their aspect ratio and are never enlarged by a non-integer factor: an image
//! larger than a cell is shrunk with an alpha-weighted box filter, a smaller one is enlarged by
//! the largest whole factor that fits (nearest neighbour), so small icons stay crisp. Cells are
//! filled row by row; the sidecar list says which input is in which cell.

use serde_json::{Value, json};
use ss2_format_tex::Texture;

use crate::error::{ExportError, Result};
use crate::image::Rgba;
use crate::source::{Entry, Source};
use crate::tex::{TexOptions, decode_selection, encode_png};

const GAP: u32 = 4;
const BACKGROUND: [u8; 3] = [0x20, 0x20, 0x20];
const CHECKER: [[u8; 3]; 2] = [[0x99, 0x99, 0x99], [0xBB, 0xBB, 0xBB]];
const CHECKER_SIZE: u32 = 8;

/// Where one input went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Row of the cell (0 at the top).
    pub row: u32,
    /// Column of the cell (0 at the left).
    pub column: u32,
    /// Left edge of the cell in the sheet, in pixels.
    pub x: u32,
    /// Top edge of the cell in the sheet, in pixels.
    pub y: u32,
    /// Width of the thumbnail inside the cell.
    pub thumb_width: u32,
    /// Height of the thumbnail inside the cell.
    pub thumb_height: u32,
}

/// A finished sheet (opaque RGBA) and the cell of every input.
#[derive(Clone, Debug)]
pub struct Sheet {
    /// The image.
    pub image: Rgba,
    /// One entry per input, in input order.
    pub placements: Vec<Placement>,
}

/// Resize `src` to `dw x dh` with an alpha-weighted box filter (shrinking).
fn shrink(src: &Rgba, dw: u32, dh: u32) -> Rgba {
    let mut out = Rgba::new(dw, dh);
    let (sw, sh) = (u64::from(src.width), u64::from(src.height));
    for dy in 0..dh {
        let y0 = u64::from(dy) * sh / u64::from(dh);
        let y1 = ((u64::from(dy) + 1) * sh / u64::from(dh)).max(y0 + 1);
        for dx in 0..dw {
            let x0 = u64::from(dx) * sw / u64::from(dw);
            let x1 = ((u64::from(dx) + 1) * sw / u64::from(dw)).max(x0 + 1);
            let (mut r, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for y in y0..y1 {
                for x in x0..x1 {
                    let at = ((y * sw + x) * 4) as usize;
                    let p = &src.pixels[at..at + 4];
                    let w = u64::from(p[3]);
                    r += u64::from(p[0]) * w;
                    g += u64::from(p[1]) * w;
                    b += u64::from(p[2]) * w;
                    a += w;
                    n += 1;
                }
            }
            let o = ((u64::from(dy) * u64::from(dw) + u64::from(dx)) * 4) as usize;
            let avg = |sum: u64| (sum + a / 2).checked_div(a).unwrap_or(0) as u8;
            out.pixels[o] = avg(r);
            out.pixels[o + 1] = avg(g);
            out.pixels[o + 2] = avg(b);
            out.pixels[o + 3] = ((a + n / 2) / n) as u8;
        }
    }
    out
}

/// Enlarge `src` by a whole factor (nearest neighbour).
fn enlarge(src: &Rgba, factor: u32) -> Rgba {
    let mut out = Rgba::new(src.width * factor, src.height * factor);
    for y in 0..out.height {
        for x in 0..out.width {
            let s = (((y / factor) * src.width + x / factor) * 4) as usize;
            let d = ((y * out.width + x) * 4) as usize;
            out.pixels[d..d + 4].copy_from_slice(&src.pixels[s..s + 4]);
        }
    }
    out
}

/// The thumbnail of `src` for a square cell of `cell` pixels.
fn thumbnail(src: &Rgba, cell: u32) -> Rgba {
    let (w, h) = (src.width, src.height);
    if w <= cell && h <= cell {
        let factor = (cell / w).min(cell / h).max(1);
        return if factor == 1 {
            src.clone()
        } else {
            enlarge(src, factor)
        };
    }
    let (tw, th) = if w >= h {
        (
            cell,
            (u64::from(h) * u64::from(cell) / u64::from(w)).max(1) as u32,
        )
    } else {
        (
            (u64::from(w) * u64::from(cell) / u64::from(h)).max(1) as u32,
            cell,
        )
    };
    shrink(src, tw, th)
}

fn put(sheet: &mut Rgba, x: u32, y: u32, rgb: [u8; 3]) {
    let at = ((y * sheet.width + x) * 4) as usize;
    sheet.pixels[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
}

/// Lay `images` out in rows of `columns` cells of `cell x cell` pixels.
///
/// `columns` is clamped to `1..=images.len()`; an empty input gives a 1 x 1 background image.
pub fn build(images: &[Rgba], cell: u32, columns: u32) -> Sheet {
    let cell = cell.max(1);
    let n = images.len() as u32;
    let columns = columns.clamp(1, n.max(1));
    let rows = n.div_ceil(columns).max(1);
    let (width, height) = (GAP + columns * (cell + GAP), GAP + rows * (cell + GAP));
    let mut sheet = Rgba::new(width, height);
    for y in 0..height {
        for x in 0..width {
            put(&mut sheet, x, y, BACKGROUND);
        }
    }
    let mut placements = Vec::with_capacity(images.len());
    for (i, img) in images.iter().enumerate() {
        let (row, column) = (i as u32 / columns, i as u32 % columns);
        let (x, y) = (GAP + column * (cell + GAP), GAP + row * (cell + GAP));
        for cy in 0..cell {
            for cx in 0..cell {
                let c = ((cx / CHECKER_SIZE + cy / CHECKER_SIZE) % 2) as usize;
                put(&mut sheet, x + cx, y + cy, CHECKER[c]);
            }
        }
        let thumb = thumbnail(img, cell);
        let (ox, oy) = (x + (cell - thumb.width) / 2, y + (cell - thumb.height) / 2);
        for ty in 0..thumb.height {
            for tx in 0..thumb.width {
                let s = ((ty * thumb.width + tx) * 4) as usize;
                let p = &thumb.pixels[s..s + 4];
                let a = u32::from(p[3]);
                let d = (((oy + ty) * sheet.width + ox + tx) * 4) as usize;
                for (c, &src) in p[..3].iter().enumerate() {
                    let under = u32::from(sheet.pixels[d + c]);
                    sheet.pixels[d + c] =
                        ((u32::from(src) * a + under * (255 - a) + 127) / 255) as u8;
                }
            }
        }
        placements.push(Placement {
            row,
            column,
            x,
            y,
            thumb_width: thumb.width,
            thumb_height: thumb.height,
        });
    }
    Sheet {
        image: sheet,
        placements,
    }
}

/// A contact sheet of textures, as PNG bytes, with the list of what is in which cell.
#[derive(Clone, Debug)]
pub struct SheetExport {
    /// The PNG of the sheet.
    pub png: Vec<u8>,
    /// The cell list and the files that could not be included (JSON).
    pub index: Value,
    /// Textures placed on the sheet.
    pub placed: usize,
    /// Textures left out, each with its reason in [`SheetExport::index`].
    pub skipped: usize,
}

/// Read, decode and lay out `files` (textures, the level and face of `tex`). A texture that
/// cannot be read or decoded is left out and listed with its reason; the others still go on the
/// sheet.
///
/// # Errors
/// [`ExportError::Encode`] if the finished sheet cannot be encoded (for example if it is
/// larger than the PNG format allows).
pub fn export_sheet(
    source: &Source,
    files: &[Entry],
    tex: &TexOptions,
    cell: u32,
    columns: u32,
) -> Result<SheetExport> {
    let mut images = Vec::new();
    let mut placed_keys = Vec::new();
    let mut skipped = Vec::new();
    for file in files {
        let image = source.read(file).and_then(|bytes| {
            let t = Texture::from_bytes(&bytes).map_err(|e| ExportError::from_reader(&e))?;
            decode_selection(&t, tex.mip, tex.face)
        });
        match image {
            Ok(img) => {
                images.push(img);
                placed_keys.push(file.key.clone());
            }
            Err(e) => skipped.push(json!({
                "source": file.key, "error_kind": e.kind(), "error": e.to_string(),
            })),
        }
    }
    let sheet = build(&images, cell, columns);
    let (png, _) = encode_png(
        &sheet.image,
        crate::tex::Channels::RgbOnly,
        ss2_format_tex::PixelFormat::Bgr8,
    )?;
    let cells: Vec<Value> = placed_keys
        .iter()
        .zip(&sheet.placements)
        .zip(&images)
        .map(|((key, p), img)| {
            json!({
                "source": key, "row": p.row, "column": p.column, "x": p.x, "y": p.y,
                "image_size": [img.width, img.height],
                "thumbnail_size": [p.thumb_width, p.thumb_height],
            })
        })
        .collect();
    Ok(SheetExport {
        png,
        placed: cells.len(),
        skipped: skipped.len(),
        index: json!({
            "tool": concat!("asset-export ", env!("CARGO_PKG_VERSION")),
            "cell": cell.max(1),
            "columns": sheet.image.width.saturating_sub(GAP) / (cell.max(1) + GAP),
            "size": [sheet.image.width, sheet.image.height],
            "mip": tex.mip,
            "cells": cells,
            "skipped": skipped,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, p: [u8; 4]) -> Rgba {
        Rgba {
            width: w,
            height: h,
            pixels: p.repeat((w * h) as usize),
        }
    }

    fn at(img: &Rgba, x: u32, y: u32) -> [u8; 4] {
        let o = ((y * img.width + x) * 4) as usize;
        img.pixels[o..o + 4].try_into().unwrap()
    }

    #[test]
    fn the_sheet_size_follows_the_grid() {
        let imgs = vec![solid(8, 8, [1, 2, 3, 255]); 5];
        let s = build(&imgs, 16, 3);
        // 3 columns, 2 rows.
        assert_eq!((s.image.width, s.image.height), (4 + 3 * 20, 4 + 2 * 20));
        assert_eq!(s.placements.len(), 5);
        assert_eq!(
            (s.placements[4].row, s.placements[4].column),
            (1, 1),
            "row-major order"
        );
        assert_eq!((s.placements[4].x, s.placements[4].y), (24, 24));
        // Too many columns is clamped to the input count.
        let s = build(&imgs[..2], 16, 9);
        assert_eq!(s.image.width, 4 + 2 * 20);
        assert_eq!(s.image.height, 4 + 20);
    }

    #[test]
    fn an_empty_sheet_is_a_valid_image() {
        let s = build(&[], 16, 4);
        assert!(s.placements.is_empty());
        assert_eq!(
            s.image.pixels.len(),
            (s.image.width * s.image.height * 4) as usize
        );
    }

    #[test]
    fn small_images_are_enlarged_by_whole_factors_and_centred() {
        let s = build(&[solid(3, 2, [200, 0, 0, 255])], 16, 1);
        // factor = min(16/3, 16/2) = 5: 15 x 10, centred in a 16 x 16 cell at (4, 4).
        let p = &s.placements[0];
        assert_eq!((p.thumb_width, p.thumb_height), (15, 10));
        assert_eq!(at(&s.image, 4, 4 + 3), [200, 0, 0, 255]);
        assert_eq!(at(&s.image, 4 + 14, 4 + 3 + 9), [200, 0, 0, 255]);
        // Outside the thumbnail the checkerboard shows.
        assert_eq!(at(&s.image, 4 + 15, 4 + 3), [0xBB, 0xBB, 0xBB, 255]);
    }

    #[test]
    fn large_images_shrink_keeping_the_aspect_ratio() {
        let s = build(&[solid(64, 32, [0, 200, 0, 255])], 16, 1);
        let p = &s.placements[0];
        assert_eq!((p.thumb_width, p.thumb_height), (16, 8));
        assert_eq!(at(&s.image, 4 + 8, 4 + 8), [0, 200, 0, 255]);
        let s = build(&[solid(32, 64, [0, 200, 0, 255])], 16, 1);
        assert_eq!(
            (s.placements[0].thumb_width, s.placements[0].thumb_height),
            (8, 16)
        );
        // A very thin strip keeps at least one pixel.
        let s = build(&[solid(256, 1, [9, 9, 9, 255])], 16, 1);
        assert_eq!(
            (s.placements[0].thumb_width, s.placements[0].thumb_height),
            (16, 1)
        );
    }

    #[test]
    fn the_box_filter_averages_and_weights_by_alpha() {
        // 2 x 1 image shrunk to 1 x 1: opaque white and fully transparent black -> white at half alpha.
        let img = Rgba {
            width: 2,
            height: 1,
            pixels: vec![255, 255, 255, 255, 0, 0, 0, 0],
        };
        assert_eq!(shrink(&img, 1, 1).pixels, [255, 255, 255, 128]);
        // Opaque black and white average to mid grey.
        let img = Rgba {
            width: 2,
            height: 1,
            pixels: vec![0, 0, 0, 255, 255, 255, 255, 255],
        };
        assert_eq!(shrink(&img, 1, 1).pixels, [128, 128, 128, 255]);
    }

    #[test]
    fn transparent_pixels_show_the_checkerboard() {
        let s = build(&[solid(16, 16, [255, 0, 0, 0])], 16, 1);
        assert_eq!(at(&s.image, 4, 4), [0x99, 0x99, 0x99, 255]);
        assert_eq!(at(&s.image, 4 + 8, 4), [0xBB, 0xBB, 0xBB, 255]);
        // Half transparent red over the dark square.
        let s = build(&[solid(16, 16, [200, 0, 0, 128])], 16, 1);
        let px = at(&s.image, 4, 4);
        assert_eq!(px[3], 255);
        assert!(px[0] > 0x99 && px[1] < 0x99 && px[1] > 0x40, "{px:?}");
    }

    #[test]
    fn the_background_and_gaps_are_filled() {
        let s = build(&[solid(16, 16, [1, 1, 1, 255])], 16, 1);
        assert_eq!(at(&s.image, 0, 0), [0x20, 0x20, 0x20, 255]);
        assert_eq!(
            at(&s.image, s.image.width - 1, s.image.height - 1),
            [0x20, 0x20, 0x20, 255]
        );
    }

    #[test]
    fn the_checkerboard_alternates_in_both_directions() {
        let s = build(&[solid(16, 16, [0, 0, 0, 0])], 16, 1);
        assert_eq!(at(&s.image, 4, 4), [0x99, 0x99, 0x99, 255]);
        assert_eq!(at(&s.image, 4 + 8, 4), [0xBB, 0xBB, 0xBB, 255]);
        assert_eq!(at(&s.image, 4, 4 + 8), [0xBB, 0xBB, 0xBB, 255]);
        assert_eq!(at(&s.image, 4 + 8, 4 + 8), [0x99, 0x99, 0x99, 255]);
        assert_eq!(at(&s.image, 4 + 7, 4 + 7), [0x99, 0x99, 0x99, 255]);
    }

    #[test]
    fn half_transparent_pixels_blend_with_rounding() {
        // 200 at alpha 128 over the grey 0x99 = 153: (200 * 128 + 153 * 127 + 127) / 255 = 177.
        let s = build(&[solid(16, 16, [200, 200, 200, 128])], 16, 1);
        assert_eq!(at(&s.image, 4, 4), [177, 177, 177, 255]);
        // Opaque pixels are exact.
        let s = build(&[solid(16, 16, [7, 8, 9, 255])], 16, 1);
        assert_eq!(at(&s.image, 4, 4), [7, 8, 9, 255]);
    }

    #[test]
    fn the_box_filter_also_copes_with_a_target_larger_than_the_source() {
        let img = Rgba {
            width: 1,
            height: 1,
            pixels: vec![10, 20, 30, 255],
        };
        assert_eq!(shrink(&img, 2, 3).pixels, [10, 20, 30, 255].repeat(6));
    }

    #[test]
    fn the_cell_list_reports_the_columns_actually_used() {
        use crate::fixtures::bgr_file;
        let dir = tempfile::tempdir().unwrap();
        let src = Source::Disk;
        let mut files = Vec::new();
        for n in ["a.tex", "b.tex"] {
            let p = dir.path().join(n);
            std::fs::write(&p, bgr_file()).unwrap();
            files.push(src.entry(p.to_str().unwrap()).unwrap());
        }
        // More columns than textures: clamped to 2, and the sheet is two cells wide.
        let out = export_sheet(&src, &files, &TexOptions::default(), 16, 9).unwrap();
        assert_eq!(out.index["columns"], 2);
        assert_eq!(out.index["size"], json!([4 + 2 * 20, 4 + 20]));
        assert_eq!((out.placed, out.skipped), (2, 0));
        // A mip level the textures do not have leaves both out, with the reason.
        let tex = TexOptions {
            mip: 3,
            ..TexOptions::default()
        };
        let out = export_sheet(&src, &files, &tex, 16, 2).unwrap();
        assert_eq!((out.placed, out.skipped), (0, 2));
        assert_eq!(out.index["skipped"][0]["error_kind"], "option");
    }
}
