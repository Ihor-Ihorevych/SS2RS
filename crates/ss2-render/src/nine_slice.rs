//! 9-slice (nine-patch) geometry: the cells a bordered image is cut into when it is drawn at some size.
//!
//! Pure CPU code, no GPU: [`nine_slice`] returns the destination rectangle and the texture rectangle of every
//! cell, in the order the cells are drawn (row by row, top to bottom, left to right).
//!
//! The rules (also in `docs/UI_RENDERER.md`):
//!
//! * The four **corners keep their size**: an inset of `n` texels is drawn `n * scale` canvas units wide or high, and
//!   a corner is never stretched.
//! * The four **edges** keep the border's thickness and fill the space between the corners; the **centre** fills
//!   what is left. With [`Fill::Stretch`] each is one cell; with [`Fill::Tile`] the edges and the centre are repeated
//!   at the size of their texels (times `scale`) and the last repeat is cut off, not squeezed.
//! * A destination **smaller than the borders** shrinks the borders on that axis by the same factor (the left and
//!   right borders by one factor, the top and bottom by another), so no size is negative and nothing overlaps; the
//!   centre then has no width (or height) and its cells are left out.
//! * Insets that are negative or not finite count as zero. Insets larger than the texture are scaled down by the
//!   same factor on that axis so that they fit.
//! * A texture whose insets use up the whole texture on an axis has **no centre** on that axis: nothing is drawn
//!   there (a gap, not a stretched edge texel).
//! * Nothing is drawn (the result is empty) for a destination without area, a texture without texels, or a `scale`
//!   that is not a positive finite number.
//! * Tiling that would take more than [`MAX_CELLS`] cells falls back to stretching, so a tiny texture drawn into
//!   a large rectangle cannot turn into a million quads.
//!
//! Cells are not snapped to whole pixels: with an integer canvas scale, integer positions and an integer `scale`
//! the borders are exactly whole pixels; otherwise the rasteriser decides which pixel an edge falls on.

use ss2_ui::Rect;

/// The most cells one 9-slice may produce (nine when stretched; the rest is tiling).
pub const MAX_CELLS: usize = 4096;

/// Border sizes, in **texels** of the source image.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    /// Width of the left border.
    pub left: f32,
    /// Height of the top border.
    pub top: f32,
    /// Width of the right border.
    pub right: f32,
    /// Height of the bottom border.
    pub bottom: f32,
}

impl Insets {
    /// No borders: the whole image is the centre.
    pub const ZERO: Insets = Insets {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// The same border on all four sides.
    pub const fn uniform(texels: f32) -> Insets {
        Insets {
            left: texels,
            top: texels,
            right: texels,
            bottom: texels,
        }
    }
}

/// A rectangle of the texture, in 0..=1 coordinates (`v` runs down the image).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    /// Left.
    pub u0: f32,
    /// Top.
    pub v0: f32,
    /// Right.
    pub u1: f32,
    /// Bottom.
    pub v1: f32,
}

impl UvRect {
    /// The whole image.
    pub const FULL: UvRect = UvRect {
        u0: 0.0,
        v0: 0.0,
        u1: 1.0,
        v1: 1.0,
    };
}

/// What the edges and the centre of a 9-slice do when the destination is larger than the image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fill {
    /// One cell each, stretched.
    #[default]
    Stretch,
    /// Repeated at texel size (times the border scale), the last repeat cut off.
    Tile,
}

/// One rectangle of a 9-slice: where it goes and what part of the texture it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// Destination, in the units of the `dest` that was given.
    pub dest: Rect,
    /// Source, in 0..=1 texture coordinates.
    pub uv: UvRect,
}

/// One piece of one axis: a span of the destination and the span of the texture it shows.
#[derive(Clone, Copy, Debug)]
struct Span {
    start: f32,
    len: f32,
    uv0: f32,
    uv1: f32,
}

/// Cut `dest` into cells for an image of `texture_size` texels with `insets`, drawing a texel `scale` units wide.
/// See the module documentation for the rules.
pub fn nine_slice(
    dest: Rect,
    insets: Insets,
    texture_size: (u32, u32),
    scale: f32,
    fill: Fill,
) -> Vec<Cell> {
    let (tw, th) = texture_size;
    let usable = tw > 0
        && th > 0
        && scale.is_finite()
        && scale > 0.0
        && [dest.x, dest.y, dest.w, dest.h]
            .iter()
            .all(|v| v.is_finite())
        && dest.w > 0.0
        && dest.h > 0.0;
    if !usable {
        return Vec::new();
    }
    let mut cells = build(dest, insets, (tw as f32, th as f32), scale, fill);
    if cells.len() > MAX_CELLS {
        cells = build(dest, insets, (tw as f32, th as f32), scale, Fill::Stretch);
    }
    cells
}

fn build(dest: Rect, insets: Insets, tex: (f32, f32), scale: f32, fill: Fill) -> Vec<Cell> {
    let xs = axis(
        dest.x,
        dest.w,
        insets.left,
        insets.right,
        tex.0,
        scale,
        fill,
    );
    let ys = axis(
        dest.y,
        dest.h,
        insets.top,
        insets.bottom,
        tex.1,
        scale,
        fill,
    );
    let mut cells = Vec::with_capacity(xs.len() * ys.len());
    for y in &ys {
        for x in &xs {
            cells.push(Cell {
                dest: Rect {
                    x: x.start,
                    y: y.start,
                    w: x.len,
                    h: y.len,
                },
                uv: UvRect {
                    u0: x.uv0,
                    v0: y.uv0,
                    u1: x.uv1,
                    v1: y.uv1,
                },
            });
        }
    }
    cells
}

/// A border size that can be used: finite and not negative, else zero.
fn clean(inset: f32) -> f32 {
    if inset.is_finite() && inset > 0.0 {
        inset
    } else {
        0.0
    }
}

/// The spans of one axis: low border, centre (one span or many tiles), high border. `size` is the texture's
/// extent on this axis in texels.
fn axis(start: f32, len: f32, low: f32, high: f32, size: f32, scale: f32, fill: Fill) -> Vec<Span> {
    let (mut low, mut high) = (clean(low), clean(high));
    if low + high > size {
        let f = size / (low + high);
        low *= f;
        high *= f;
    }
    let centre_texels = size - low - high;
    let (mut low_len, mut high_len) = (low * scale, high * scale);
    let mut centre_len = len - low_len - high_len;
    if centre_len <= 0.0 {
        // The destination is smaller than the borders: shrink them by one factor, no centre.
        let f = len / (low_len + high_len);
        low_len *= f;
        high_len = len - low_len;
        centre_len = 0.0;
    }
    let (u_low, u_high) = (low / size, (size - high) / size);

    let mut spans = Vec::with_capacity(3);
    if low_len > 0.0 {
        spans.push(Span {
            start,
            len: low_len,
            uv0: 0.0,
            uv1: u_low,
        });
    }
    if centre_len > 0.0 && centre_texels > 0.0 {
        // Stretch: one piece that is the whole centre. Tile: pieces one texel-size long, the last one cut off.
        let (tile, pieces) = match fill {
            Fill::Stretch => (centre_len, 1),
            // A repeat that is only a rounding error long is not a repeat.
            Fill::Tile => {
                let tile = centre_texels * scale;
                (tile, ((centre_len / tile - 1e-4).ceil() as usize).max(1))
            }
        };
        for i in 0..pieces {
            let offset = i as f32 * tile;
            let piece = (centre_len - offset).min(tile);
            spans.push(Span {
                start: start + low_len + offset,
                len: piece,
                uv0: u_low,
                uv1: u_low + (u_high - u_low) * (piece / tile),
            });
        }
    }
    if high_len > 0.0 {
        spans.push(Span {
            start: start + len - high_len,
            len: high_len,
            uv0: u_high,
            uv1: 1.0,
        });
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn cells(dest: Rect, insets: Insets, tex: (u32, u32), scale: f32) -> Vec<Cell> {
        nine_slice(dest, insets, tex, scale, Fill::Stretch)
    }

    fn area(cells: &[Cell]) -> f32 {
        cells.iter().map(|c| c.dest.w * c.dest.h).sum()
    }

    fn all_sane(cells: &[Cell]) -> bool {
        cells.iter().all(|c| {
            let d = c.dest;
            let u = c.uv;
            [d.x, d.y, d.w, d.h, u.u0, u.v0, u.u1, u.v1]
                .iter()
                .all(|v| v.is_finite())
                && d.w >= 0.0
                && d.h >= 0.0
                && u.u0 <= u.u1
                && u.v0 <= u.v1
                && (0.0..=1.0).contains(&u.u0)
                && (0.0..=1.0).contains(&u.u1)
                && (0.0..=1.0).contains(&u.v0)
                && (0.0..=1.0).contains(&u.v1)
        })
    }

    #[test]
    fn a_large_destination_makes_nine_cells_with_corners_at_texel_size() {
        let c = cells(
            rect(100.0, 50.0, 300.0, 200.0),
            Insets::uniform(8.0),
            (32, 32),
            1.0,
        );
        assert_eq!(c.len(), 9);
        // Corners: 8x8, at the four corners.
        assert_eq!(c[0].dest, rect(100.0, 50.0, 8.0, 8.0));
        assert_eq!(c[2].dest, rect(392.0, 50.0, 8.0, 8.0));
        assert_eq!(c[6].dest, rect(100.0, 242.0, 8.0, 8.0));
        assert_eq!(c[8].dest, rect(392.0, 242.0, 8.0, 8.0));
        // The corners show exactly the corner texels.
        assert_eq!(
            c[0].uv,
            UvRect {
                u0: 0.0,
                v0: 0.0,
                u1: 0.25,
                v1: 0.25
            }
        );
        assert_eq!(
            c[8].uv,
            UvRect {
                u0: 0.75,
                v0: 0.75,
                u1: 1.0,
                v1: 1.0
            }
        );
        // The centre takes the rest and shows the middle of the texture.
        assert_eq!(c[4].dest, rect(108.0, 58.0, 284.0, 184.0));
        assert_eq!(
            c[4].uv,
            UvRect {
                u0: 0.25,
                v0: 0.25,
                u1: 0.75,
                v1: 0.75
            }
        );
        // Edges: between the corners.
        assert_eq!(c[1].dest, rect(108.0, 50.0, 284.0, 8.0));
        assert_eq!(c[3].dest, rect(100.0, 58.0, 8.0, 184.0));
        assert_eq!(c[5].dest, rect(392.0, 58.0, 8.0, 184.0));
        assert_eq!(c[7].dest, rect(108.0, 242.0, 284.0, 8.0));
        // Together they cover the destination exactly.
        assert_eq!(area(&c), 300.0 * 200.0);
        assert!(all_sane(&c));
    }

    #[test]
    fn the_border_scale_multiplies_the_border_not_the_centre() {
        let c = cells(
            rect(0.0, 0.0, 200.0, 100.0),
            Insets::uniform(4.0),
            (16, 16),
            3.0,
        );
        assert_eq!(c[0].dest, rect(0.0, 0.0, 12.0, 12.0));
        assert_eq!(c[8].dest, rect(188.0, 88.0, 12.0, 12.0));
        assert_eq!(c[4].dest, rect(12.0, 12.0, 176.0, 76.0));
        assert_eq!(area(&c), 200.0 * 100.0);
    }

    #[test]
    fn insets_may_differ_on_each_side() {
        let c = cells(
            rect(0.0, 0.0, 100.0, 100.0),
            Insets {
                left: 2.0,
                top: 4.0,
                right: 6.0,
                bottom: 8.0,
            },
            (20, 40),
            1.0,
        );
        assert_eq!(c[0].dest, rect(0.0, 0.0, 2.0, 4.0));
        assert_eq!(c[2].dest, rect(94.0, 0.0, 6.0, 4.0));
        assert_eq!(c[6].dest, rect(0.0, 92.0, 2.0, 8.0));
        assert_eq!(c[8].dest, rect(94.0, 92.0, 6.0, 8.0));
        assert_eq!(c[0].uv.u1, 0.1);
        assert_eq!(c[0].uv.v1, 0.1);
        assert_eq!(c[8].uv.u0, 0.7);
        assert_eq!(c[8].uv.v0, 0.8);
    }

    #[test]
    fn a_destination_smaller_than_the_borders_shrinks_them_in_proportion() {
        // Borders of 10 + 30 = 40 into a width of 20: both halve; the height is large and unaffected.
        let c = cells(
            rect(5.0, 7.0, 20.0, 200.0),
            Insets {
                left: 10.0,
                top: 10.0,
                right: 30.0,
                bottom: 10.0,
            },
            (100, 100),
            1.0,
        );
        assert!(all_sane(&c));
        // No centre column: two columns by three rows.
        assert_eq!(c.len(), 6);
        assert_eq!(c[0].dest, rect(5.0, 7.0, 5.0, 10.0));
        assert_eq!(c[1].dest, rect(10.0, 7.0, 15.0, 10.0));
        assert_eq!(area(&c), 20.0 * 200.0);
        // The texture parts are the full borders even though they are drawn smaller.
        assert_eq!(c[0].uv.u1, 0.1);
        assert_eq!(c[1].uv.u0, 0.7);
    }

    #[test]
    fn a_destination_smaller_in_both_axes_leaves_only_the_four_corners() {
        let c = cells(
            rect(0.0, 0.0, 6.0, 4.0),
            Insets::uniform(10.0),
            (64, 64),
            1.0,
        );
        assert_eq!(c.len(), 4);
        assert!(all_sane(&c));
        assert_eq!(c[0].dest, rect(0.0, 0.0, 3.0, 2.0));
        assert_eq!(c[3].dest, rect(3.0, 2.0, 3.0, 2.0));
        assert_eq!(area(&c), 24.0);
    }

    #[test]
    fn a_destination_exactly_the_size_of_the_borders_has_no_centre() {
        let c = cells(
            rect(0.0, 0.0, 16.0, 16.0),
            Insets::uniform(8.0),
            (32, 32),
            1.0,
        );
        assert_eq!(c.len(), 4);
        assert_eq!(area(&c), 256.0);
    }

    #[test]
    fn zero_insets_make_one_stretched_cell_with_the_whole_texture() {
        let c = cells(rect(1.0, 2.0, 30.0, 40.0), Insets::ZERO, (8, 8), 1.0);
        assert_eq!(
            c,
            vec![Cell {
                dest: rect(1.0, 2.0, 30.0, 40.0),
                uv: UvRect::FULL
            }]
        );
    }

    #[test]
    fn insets_that_are_negative_or_not_finite_count_as_zero() {
        let c = cells(
            rect(0.0, 0.0, 50.0, 50.0),
            Insets {
                left: -3.0,
                top: f32::NAN,
                right: f32::INFINITY,
                bottom: f32::NEG_INFINITY,
            },
            (16, 16),
            1.0,
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].uv, UvRect::FULL);
    }

    #[test]
    fn insets_larger_than_the_texture_are_scaled_down_to_fit_it() {
        // 12 + 12 on an 8-texel axis: each becomes 4 texels; nothing is left for a centre.
        let c = cells(
            rect(0.0, 0.0, 100.0, 100.0),
            Insets {
                left: 12.0,
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
            },
            (8, 8),
            1.0,
        );
        assert!(all_sane(&c));
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].dest, rect(0.0, 0.0, 4.0, 100.0));
        assert_eq!(c[1].dest, rect(96.0, 0.0, 4.0, 100.0));
        assert_eq!(c[0].uv.u1, 0.5);
        assert_eq!(c[1].uv.u0, 0.5);
    }

    #[test]
    fn degenerate_inputs_draw_nothing_and_never_produce_nan() {
        let ok = rect(0.0, 0.0, 10.0, 10.0);
        let ins = Insets::uniform(2.0);
        assert!(cells(ok, ins, (0, 8), 1.0).is_empty());
        assert!(cells(ok, ins, (8, 0), 1.0).is_empty());
        assert!(cells(rect(0.0, 0.0, 0.0, 10.0), ins, (8, 8), 1.0).is_empty());
        assert!(cells(rect(0.0, 0.0, 10.0, -1.0), ins, (8, 8), 1.0).is_empty());
        assert!(cells(rect(f32::NAN, 0.0, 10.0, 10.0), ins, (8, 8), 1.0).is_empty());
        assert!(cells(rect(0.0, 0.0, f32::INFINITY, 10.0), ins, (8, 8), 1.0).is_empty());
        assert!(cells(ok, ins, (8, 8), 0.0).is_empty());
        assert!(cells(ok, ins, (8, 8), -1.0).is_empty());
        assert!(cells(ok, ins, (8, 8), f32::NAN).is_empty());
        assert!(cells(ok, ins, (8, 8), f32::INFINITY).is_empty());
    }

    #[test]
    fn a_tiny_destination_stays_finite_and_non_negative() {
        for w in [1e-6_f32, 1e-3, 0.5, 1.0] {
            let c = cells(rect(0.0, 0.0, w, w), Insets::uniform(8.0), (32, 32), 1.0);
            assert!(all_sane(&c), "{w}: {c:?}");
            assert!(
                (area(&c) - w * w).abs() <= w * w * 1e-3,
                "{w}: {}",
                area(&c)
            );
        }
    }

    #[test]
    fn a_texture_with_no_centre_draws_a_gap_not_a_stretched_texel() {
        // 4 + 4 = the whole 8-texel width: the borders are drawn, the middle of the destination is not.
        let c = cells(
            rect(0.0, 0.0, 100.0, 8.0),
            Insets {
                left: 4.0,
                top: 0.0,
                right: 4.0,
                bottom: 0.0,
            },
            (8, 8),
            1.0,
        );
        assert_eq!(c.len(), 2);
        assert_eq!(area(&c), 8.0 * 8.0);
    }

    #[test]
    fn tiling_repeats_the_centre_at_texel_size_and_cuts_the_last_repeat() {
        // 32x32 texture with 8-texel borders: the centre is 16x16 texels. Destination centre: 40 wide, 16 high.
        let c = nine_slice(
            rect(0.0, 0.0, 56.0, 32.0),
            Insets::uniform(8.0),
            (32, 32),
            1.0,
            Fill::Tile,
        );
        assert!(all_sane(&c));
        // Columns: left, 3 centre tiles (16, 16, 8), right = 5; rows: top, 1 centre tile (16), bottom = 3.
        assert_eq!(c.len(), 15);
        let middle: Vec<&Cell> = c.iter().filter(|c| c.dest.y == 8.0).collect();
        assert_eq!(middle.len(), 5);
        assert_eq!(middle[1].dest, rect(8.0, 8.0, 16.0, 16.0));
        assert_eq!(middle[2].dest, rect(24.0, 8.0, 16.0, 16.0));
        // The last tile is half a tile: its texture rectangle is half of the centre, not the whole.
        assert_eq!(middle[3].dest, rect(40.0, 8.0, 8.0, 16.0));
        assert_eq!(middle[3].uv.u0, 0.25);
        assert_eq!(middle[3].uv.u1, 0.5);
        // The first tile shows the whole centre.
        assert_eq!((middle[1].uv.u0, middle[1].uv.u1), (0.25, 0.75));
        assert_eq!(area(&c), 56.0 * 32.0);
    }

    #[test]
    fn tiling_with_the_border_scale_uses_the_scaled_texel_size() {
        let c = nine_slice(
            rect(0.0, 0.0, 100.0, 16.0),
            Insets {
                left: 2.0,
                top: 0.0,
                right: 2.0,
                bottom: 0.0,
            },
            (8, 8),
            2.0,
            Fill::Tile,
        );
        // Borders 4 units; centre 92 wide, tile 4 texels * 2 = 8 units: 11.5 tiles -> 12 pieces + 2 borders.
        assert_eq!(c.len(), 14);
        let last = &c[12];
        assert_eq!(last.dest.w, 4.0);
        assert_eq!(last.uv.u1, 0.25 + 0.5 * 0.5);
    }

    #[test]
    fn a_centre_only_a_sliver_wide_is_still_one_cut_piece() {
        // Borders of 4 + 4 leave 0.001 for the centre, which is a 16-texel tile.
        let c = nine_slice(
            rect(0.0, 0.0, 8.001, 8.0),
            Insets {
                left: 4.0,
                top: 0.0,
                right: 4.0,
                bottom: 0.0,
            },
            (24, 8),
            1.0,
            Fill::Tile,
        );
        assert!(all_sane(&c));
        assert_eq!(c.len(), 3, "{}", c.len());
        assert!(c[1].dest.w < 0.01 && c[1].dest.w > 0.0, "{:?}", c[1]);
    }

    #[test]
    fn tiles_that_fit_exactly_are_not_followed_by_an_empty_tile() {
        // The tile length is inexact in binary for these scales: N whole tiles must stay N pieces.
        for scale in [0.1_f32, 0.3, 0.7, 1.1, 2.3] {
            for n in [1usize, 2, 5, 12] {
                let tile = 16.0_f32 * scale;
                let border = 4.0 * scale;
                let c = nine_slice(
                    rect(0.0, 0.0, n as f32 * tile + 2.0 * border, 8.0 * scale),
                    Insets {
                        left: 4.0,
                        top: 0.0,
                        right: 4.0,
                        bottom: 0.0,
                    },
                    (24, 8),
                    scale,
                    Fill::Tile,
                );
                assert_eq!(c.len(), n + 2, "scale {scale}, {n} tiles: {}", c.len());
            }
        }
    }

    #[test]
    fn tiling_exactly_at_the_cell_limit_still_tiles() {
        // 62 tiles each way and the borders: 64 x 64 = 4096 cells = MAX_CELLS.
        let c = nine_slice(
            rect(0.0, 0.0, 126.0, 126.0),
            Insets::uniform(1.0),
            (4, 4),
            1.0,
            Fill::Tile,
        );
        assert_eq!(c.len(), MAX_CELLS);
    }

    #[test]
    fn tiling_that_would_make_too_many_cells_falls_back_to_stretching() {
        let c = nine_slice(
            rect(0.0, 0.0, 10_000.0, 10_000.0),
            Insets::uniform(1.0),
            (4, 4),
            1.0,
            Fill::Tile,
        );
        assert_eq!(c.len(), 9);
        assert_eq!(area(&c), 1e8);
        // Just under the cap still tiles.
        let c = nine_slice(
            rect(0.0, 0.0, 60.0, 60.0),
            Insets::uniform(1.0),
            (4, 4),
            1.0,
            Fill::Tile,
        );
        assert_eq!(c.len(), 31 * 31);
    }
}
