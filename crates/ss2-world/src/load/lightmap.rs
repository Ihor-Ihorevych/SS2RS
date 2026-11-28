//! Which UV stream of a mesh is its lightmap's (`wld-lighting.md`, "Baked lightmaps", reader
//! invariant 3: T01-407's candidate test, T01-420's ranking and T01-421's lit-texel test).
//!
//! The world's oven modifier names a UV map id that is none of the mesh's own ids (reader
//! invariant 3), so the stream is found from the mesh data: a lightmap unwrap lies entirely
//! inside [0, 1] and its triangles do not overlap (each texel lights one spot). A stream is
//! **in range** when, on every surface of the level of detail that has triangles, it exists and
//! its coordinates are inside [0, 1] (within [`RANGE_EPS`]); its **overlap** is the share of the
//! rasterized area of all its triangles (all surfaces together, since they share one lightmap)
//! covered twice or more. A **candidate** is in range with at most [`MAX_OVERLAP`] overlap.
//! Streams are identified by their position in each surface's [`Surface::uv_sets`] (slot order,
//! `bmf.md` member 12/14).
//!
//! - Exactly one candidate is the lightmap's stream.
//! - No candidate: the in-range stream with the least overlap, the 2 % cutoff relaxed but never
//!   the [0, 1] requirement (T01-420, stage 1). Equal overlaps go to a stream the material does
//!   not use (no surface's material has the id that surface declares at that position among its
//!   UV map ids: the UV map of each non-empty texture group of every config, `mtr.md`), then to
//!   the lowest position (T01-420, stage 3).
//! - Several candidates: the one with the least overlap (stage 1). True candidates usually tie
//!   at 0 % overlap; the tied ones are then ranked **per placement** by how much of each one's
//!   footprint is lit in the placement's own baked lightmap ([`lit_fraction`], T01-420 stage 2
//!   as pinned down by T01-421), and what still ties goes to a stream the material does not use,
//!   then to the lowest position (stage 3, [`choose_tied`]). Placements of one model can so end up
//!   with different streams.
//! - No stream in range at all: nothing is chosen. **This departs from the spec**, which keeps
//!   T01-414's slot-0 fallback there: such a slot 0 is a tiling channel, and sampling a lightmap
//!   through it tiles the texture into dark bands, so the model is drawn unlit instead.

use ss2_format_bmf::Surface;
use ss2_format_tex::RgbaImage;

/// How far outside [0, 1] a coordinate may lie and still count as inside.
pub(crate) const RANGE_EPS: f32 = 1e-3;
/// The largest share of a stream's rasterized area that may be covered more than once (the
/// spec's "at most 2 percent").
pub(crate) const MAX_OVERLAP: f64 = 0.02;
/// Cells per side of the grid the overlap is measured on.
const GRID: usize = 256;

/// The outcome of the choice for one mesh: the rule that applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StreamChoice {
    /// Exactly one candidate: the stream at this position.
    One(usize),
    /// Several candidates (their positions, ascending). `tied` holds those with the least
    /// overlap in stage-3 order (a stream no material uses first, then by position): a single
    /// one is the stage-1 pick for every placement; with more, each placement chooses among
    /// them ([`choose_tied`]).
    Several {
        candidates: Vec<usize>,
        tied: Vec<usize>,
    },
    /// No candidate: `stream` is the in-range stream with the least overlap.
    NearMiss { stream: usize },
    /// No stream is in range: none is taken (a departure from the spec's slot-0 fallback).
    NoneInRange,
}

impl StreamChoice {
    /// The position of the stream the whole model gets, if any: for several candidates tied
    /// on overlap, the stage-3 default that a placement keeps unless its own lightmap decides.
    pub(crate) fn stream(&self) -> Option<usize> {
        match self {
            Self::One(k) | Self::NearMiss { stream: k } => Some(*k),
            Self::Several { tied, .. } => tied.first().copied(),
            Self::NoneInRange => None,
        }
    }

    /// The streams each placement chooses among (stage-3 order), when several candidates tie on
    /// overlap; `None` when the stream is the same for every placement.
    pub(crate) fn ties(&self) -> Option<&[usize]> {
        match self {
            Self::Several { tied, .. } if tied.len() > 1 => Some(tied),
            _ => None,
        }
    }
}

/// Choose the lightmap stream of the surfaces of one level of detail. `material_uv_ids[i]` holds
/// the UV map ids that surface `i`'s material uses (empty without a material); a surface past
/// its end counts as using none.
pub(crate) fn choose(surfaces: &[Surface], material_uv_ids: &[Vec<u32>]) -> StreamChoice {
    let drawn = || {
        surfaces
            .iter()
            .enumerate()
            .filter(|(_, s)| s.indices.len() >= 3)
    };
    let streams = drawn().map(|(_, s)| s.uv_sets.len()).max().unwrap_or(0);
    let used = |k: usize| {
        drawn().any(|(i, s)| {
            let ids = material_uv_ids.get(i).map_or(&[][..], Vec::as_slice);
            s.uv_map_ids.get(k).is_some_and(|id| ids.contains(id))
        })
    };
    // (overlap, position) of every stream in range, by position.
    let in_range: Vec<(f64, usize)> = (0..streams)
        .filter(|&k| is_in_range(surfaces, k))
        .map(|k| (overlap(surfaces, k), k))
        .collect();
    let candidates: Vec<usize> = in_range
        .iter()
        .filter(|(o, _)| *o <= MAX_OVERLAP)
        .map(|(_, k)| *k)
        .collect();
    // Stage 3's order: a stream the material does not use, then the lowest position.
    let stage3 = |a: usize, b: usize| used(a).cmp(&used(b)).then(a.cmp(&b));
    match candidates.as_slice() {
        [k] => StreamChoice::One(*k),
        // The least overlap, then stage 3.
        [] => match in_range
            .iter()
            .min_by(|a, b| a.0.total_cmp(&b.0).then(stage3(a.1, b.1)))
        {
            Some(&(_, stream)) => StreamChoice::NearMiss { stream },
            None => StreamChoice::NoneInRange,
        },
        _ => {
            let least = in_range
                .iter()
                .filter(|(o, _)| *o <= MAX_OVERLAP)
                .map(|(o, _)| *o)
                .fold(f64::INFINITY, f64::min);
            let mut tied: Vec<usize> = in_range
                .iter()
                .filter(|(o, _)| *o == least)
                .map(|(_, k)| *k)
                .collect();
            tied.sort_by(|&a, &b| stage3(a, b));
            StreamChoice::Several { candidates, tied }
        }
    }
}

/// The reference grey of the lit-texel test: the colour of a lightmap's unlit gutters (T01-421).
const GREY: u8 = 127;
/// A texel is lit when one of its R, G, B channels departs from [`GREY`] by more than this.
const GREY_TOL: u8 = 10;

/// How much of a stream's footprint is lit in a lightmap: of the `covered` texels (whose centre
/// lies in one of the stream's triangles), `lit` are lit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LitFraction {
    pub(crate) lit: u64,
    pub(crate) covered: u64,
}

impl LitFraction {
    /// Compare the fractions exactly (no rounding); a footprint that covers no texel counts as
    /// none lit.
    fn cmp(self, other: Self) -> std::cmp::Ordering {
        let a = u128::from(self.lit) * u128::from(other.covered.max(1));
        let b = u128::from(other.lit) * u128::from(self.covered.max(1));
        a.cmp(&b)
    }
}

/// Whether a texel `[r, g, b, a]` is lit: the largest of its R, G, B departures from (127, 127,
/// 127) is above 10 (alpha is ignored).
fn is_lit(texel: &[u8]) -> bool {
    texel[..3].iter().any(|&c| c.abs_diff(GREY) > GREY_TOL)
}

/// The lit fraction of stream `k`'s footprint in `image` (mip 0 of the placement's lightmap,
/// T01-421): the footprint is every texel of the image's own grid whose centre lies inside or on
/// the edge of at least one triangle of the stream, all surfaces together; texel `(x, y)` has
/// its centre at `u = (x + 0.5) / width`, `v = (y + 0.5) / height` (rows top to bottom, no V
/// flip). A triangle of zero area covers nothing; a surface without the stream adds nothing.
pub(crate) fn lit_fraction(surfaces: &[Surface], k: usize, image: &RgbaImage) -> LitFraction {
    let (w, h) = (image.width as usize, image.height as usize);
    let mut covered = vec![false; w * h];
    for s in surfaces.iter().filter(|s| s.indices.len() >= 3) {
        let Some(uvs) = s.uv_sets.get(k) else {
            continue;
        };
        for t in s.indices.as_chunks::<3>().0 {
            let p = t.map(|i| {
                let [u, v] = uvs.get(usize::from(i)).copied().unwrap_or([f32::NAN; 2]);
                [f64::from(u) * w as f64, f64::from(v) * h as f64]
            });
            cover(&mut covered, w, h, p);
        }
    }
    let mut f = LitFraction::default();
    for (texel, _) in image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(&covered)
        .filter(|(_, c)| **c)
    {
        f.covered += 1;
        f.lit += u64::from(is_lit(texel));
    }
    f
}

/// Mark every texel of the `w` x `h` grid whose centre lies inside the triangle or on one of its
/// edges (an inclusive test: nothing is counted, so shared edges need no tie-break).
fn cover(covered: &mut [bool], w: usize, h: usize, p: [[f64; 2]; 3]) {
    if p.iter().flatten().any(|c| !c.is_finite()) {
        return;
    }
    let area = cross(p[0], p[1], p[2]);
    if area == 0.0 {
        return;
    }
    // Counter-clockwise order.
    let p = if area < 0.0 { [p[0], p[2], p[1]] } else { p };
    let min = |a: usize| p.iter().map(|q| q[a]).fold(f64::MAX, f64::min);
    let max = |a: usize| p.iter().map(|q| q[a]).fold(f64::MIN, f64::max);
    // The first and last texel whose centre (`i + 0.5`) can lie in [lo, hi].
    let span = |lo: f64, hi: f64, n: usize| {
        let first = (lo - 0.5).ceil().max(0.0);
        let last = (hi - 0.5).floor().min(n as f64 - 1.0);
        (first <= last).then_some((first as usize, last as usize))
    };
    let (Some((x0, x1)), Some((y0, y1))) = (span(min(0), max(0), w), span(min(1), max(1), h))
    else {
        return;
    };
    let edges = [(p[1], p[2]), (p[2], p[0]), (p[0], p[1])];
    for y in y0..=y1 {
        for x in x0..=x1 {
            let c = [x as f64 + 0.5, y as f64 + 0.5];
            if edges.iter().all(|(a, b)| cross(*a, *b, c) >= 0.0) {
                covered[y * w + x] = true;
            }
        }
    }
}

/// Which rule chose a placement's stream among streams tied on overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// One stream has the highest lit fraction (T01-420 stage 2).
    LitFraction,
    /// The highest lit fraction ties too: the first of those in stage-3 order (T01-420 stage 3).
    Order,
}

/// Choose among the streams tied on overlap (stage-3 order, [`StreamChoice::ties`]) by their lit
/// fractions (`fractions[i]` is the `i`-th stream's): the highest wins; equal highest fractions
/// go to the first in stage-3 order. Returns the index of the stream and the rule that decided.
pub(crate) fn choose_tied(fractions: &[LitFraction]) -> (usize, Stage) {
    let Some(best) = fractions.iter().copied().max_by(|a, b| a.cmp(*b)) else {
        return (0, Stage::Order);
    };
    let mut top = fractions
        .iter()
        .enumerate()
        .filter(|(_, f)| f.cmp(best).is_eq())
        .map(|(i, _)| i);
    let first = top.next().unwrap_or(0);
    let stage = if top.next().is_some() {
        Stage::Order
    } else {
        Stage::LitFraction
    };
    (first, stage)
}

/// Whether stream `k` exists and is in [0, 1] on every surface with triangles (and there is one).
fn is_in_range(surfaces: &[Surface], k: usize) -> bool {
    let mut any = false;
    for s in surfaces.iter().filter(|s| s.indices.len() >= 3) {
        let Some(uvs) = s.uv_sets.get(k) else {
            return false;
        };
        let used = s.indices.iter().map(|&i| uvs.get(usize::from(i)));
        for uv in used {
            let Some(&[u, v]) = uv else { return false };
            if !(inside(u) && inside(v)) {
                return false;
            }
        }
        any = true;
    }
    any
}

fn inside(c: f32) -> bool {
    c.is_finite() && (-RANGE_EPS..=1.0 + RANGE_EPS).contains(&c)
}

/// The share of the covered cells that are covered by two triangles or more.
fn overlap(surfaces: &[Surface], k: usize) -> f64 {
    let mut grid = vec![0u8; GRID * GRID];
    for s in surfaces.iter().filter(|s| s.indices.len() >= 3) {
        let uvs = &s.uv_sets[k];
        for t in s.indices.as_chunks::<3>().0 {
            let p = [0, 1, 2].map(|j| {
                let [u, v] = uvs[usize::from(t[j])];
                [f64::from(u) * GRID as f64, f64::from(v) * GRID as f64]
            });
            rasterize(&mut grid, p);
        }
    }
    let covered = grid.iter().filter(|&&c| c > 0).count();
    if covered == 0 {
        return 0.0;
    }
    let twice = grid.iter().filter(|&&c| c > 1).count();
    twice as f64 / covered as f64
}

/// Add one to every cell whose centre lies in the triangle (edges shared by two triangles count
/// for one of them only: the top-left rule).
fn rasterize(grid: &mut [u8], p: [[f64; 2]; 3]) {
    let area = cross(p[0], p[1], p[2]);
    if area.abs() < 1e-12 {
        return;
    }
    // Counter-clockwise order.
    let p = if area < 0.0 { [p[0], p[2], p[1]] } else { p };
    let min = |a: usize| p.iter().map(|q| q[a]).fold(f64::MAX, f64::min);
    let max = |a: usize| p.iter().map(|q| q[a]).fold(f64::MIN, f64::max);
    let cell = |x: f64| (x - 0.5).ceil().clamp(0.0, GRID as f64) as usize;
    let (x0, x1) = (cell(min(0)), cell(max(0)).min(GRID - 1));
    let (y0, y1) = (cell(min(1)), cell(max(1)).min(GRID - 1));
    let edges = [(p[1], p[2]), (p[2], p[0]), (p[0], p[1])];
    let top_left = edges.map(|(a, b)| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        // For a counter-clockwise triangle (y up): a "left" edge goes down, a "top" edge is
        // horizontal and goes left.
        dy < 0.0 || (dy == 0.0 && dx < 0.0)
    });
    for y in y0..=y1 {
        for x in x0..=x1 {
            let c = [x as f64 + 0.5, y as f64 + 0.5];
            let inside = edges.iter().zip(top_left).all(|((a, b), tl)| {
                let w = cross(*a, *b, c);
                w > 0.0 || (w == 0.0 && tl)
            });
            if inside {
                let g = &mut grid[y * GRID + x];
                *g = g.saturating_add(1);
            }
        }
    }
}

/// Twice the signed area of the triangle `a b c` (positive when counter-clockwise).
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

#[cfg(test)]
mod tests {
    use ss2_format_bmf::MaterialRef;

    use super::*;

    /// A surface of `quads` side-by-side unit quads (two triangles each), with the given UV
    /// streams (one `[u, v]` per vertex, four vertices per quad).
    fn surface(quads: usize, uv_sets: Vec<Vec<[f32; 2]>>) -> Surface {
        let n = quads * 4;
        let indices = (0..quads as u16)
            .flat_map(|q| [0, 1, 2, 2, 1, 3].map(|i| q * 4 + i))
            .collect();
        Surface {
            indices,
            positions: (0..n).map(|i| [i as f32, 0.0, 0.0]).collect(),
            normals: vec![[0.0, 1.0, 0.0]; n],
            tangents: None,
            uv_map_ids: (0..uv_sets.len() as u32).map(|i| i + 3).collect(),
            uv_sets,
            bone_weights: None,
            bone_indices: None,
            bone_palette: Vec::new(),
            material: MaterialRef::None,
        }
    }

    /// Quad `q` of `n` as a vertical strip of the unit square: an unwrap.
    fn strip(q: usize, n: usize) -> [[f32; 2]; 4] {
        let (a, b) = (q as f32 / n as f32, (q + 1) as f32 / n as f32);
        [[a, 0.0], [b, 0.0], [a, 1.0], [b, 1.0]]
    }

    /// Every quad covering the unit square times `scale`: a tiling stream.
    fn tile(scale: f32) -> [[f32; 2]; 4] {
        [[0.0, 0.0], [scale, 0.0], [0.0, scale], [scale, scale]]
    }

    fn stream(quads: usize, f: impl Fn(usize) -> [[f32; 2]; 4]) -> Vec<[f32; 2]> {
        (0..quads).flat_map(f).collect()
    }

    /// Choose with no material: no stream is used by the material.
    fn pick(surfaces: &[Surface]) -> StreamChoice {
        choose(surfaces, &[])
    }

    /// `strip` with quad 0 widened to `w` (it then overlaps quad 1).
    fn widened(n: usize, w: f32) -> impl Fn(usize) -> [[f32; 2]; 4] {
        move |q| {
            let mut s = strip(q, n);
            if q == 0 {
                s[1][0] = w;
                s[3][0] = w;
            }
            s
        }
    }

    #[test]
    fn the_unwrap_is_chosen_over_a_tiling_stream() {
        // Slot 0 tiles (outside [0, 1]), slot 1 is the unwrap.
        let s = surface(
            4,
            vec![stream(4, |_| tile(3.0)), stream(4, |q| strip(q, 4))],
        );
        assert_eq!(pick(&[s]), StreamChoice::One(1));
    }

    #[test]
    fn a_stream_inside_the_unit_square_that_overlaps_is_not_a_candidate() {
        // Slot 0 maps every quad onto the whole square (inside [0, 1] but stacked).
        let s = surface(
            4,
            vec![stream(4, |_| tile(1.0)), stream(4, |q| strip(q, 4))],
        );
        assert_eq!(pick(&[s]), StreamChoice::One(1));
    }

    #[test]
    fn the_surfaces_of_a_mesh_share_the_lightmap() {
        // Each surface alone is a fine unwrap, but in slot 0 both use the same half: they
        // overlap. In slot 1 they use different halves.
        let left = |q| strip(q, 4);
        let right = |q| strip(q + 2, 4);
        let a = surface(2, vec![stream(2, left), stream(2, left)]);
        let b = surface(2, vec![stream(2, left), stream(2, right)]);
        assert_eq!(pick(&[a, b]), StreamChoice::One(1));
    }

    #[test]
    fn a_single_candidate_is_taken_even_when_the_material_uses_it() {
        let s = surface(
            4,
            vec![stream(4, |_| tile(3.0)), stream(4, |q| strip(q, 4))],
        );
        // Id 4 is slot 1, the one candidate.
        let choice = choose(&[s], &[vec![4]]);
        assert_eq!(choice, StreamChoice::One(1));
        assert_eq!(choice.stream(), Some(1));
    }

    #[test]
    fn several_candidates_go_to_the_least_overlap_whatever_the_material() {
        // Slot 0 overlaps a little (about 1 %, under the cutoff), slot 2 not at all; slot 1
        // tiles. Stage 1 decides for the whole model: no placement chooses.
        let s = surface(
            4,
            vec![
                stream(4, widened(4, 0.26)),
                stream(4, |_| tile(3.0)),
                stream(4, |q| strip(q, 4)),
            ],
        );
        let s = std::slice::from_ref(&s);
        let o = overlap(s, 0);
        assert!(o > 0.0 && o <= MAX_OVERLAP, "{o}");
        for used in [vec![], vec![3], vec![5], vec![3, 5]] {
            let choice = choose(s, &[used]);
            assert_eq!(
                choice,
                StreamChoice::Several {
                    candidates: vec![0, 2],
                    tied: vec![2]
                }
            );
            assert_eq!(choice.stream(), Some(2));
            assert_eq!(choice.ties(), None);
        }
    }

    #[test]
    fn candidates_tied_on_overlap_are_left_to_each_placement_in_stage_three_order() {
        // Slots 0 and 2 are both perfect unwraps (0 % overlap), slot 1 tiles.
        let s = surface(
            4,
            vec![
                stream(4, |q| strip(q, 4)),
                stream(4, |_| tile(3.0)),
                stream(4, |q| strip(3 - q, 4)),
            ],
        );
        let s = std::slice::from_ref(&s);
        // Ids 3, 4, 5 by slot. Unused streams first, then by position.
        for (used, tied) in [
            (vec![], vec![0, 2]),
            (vec![4], vec![0, 2]),
            (vec![3], vec![2, 0]),
            (vec![5], vec![0, 2]),
            (vec![3, 5], vec![0, 2]),
        ] {
            let choice = choose(s, std::slice::from_ref(&used));
            assert_eq!(choice.ties(), Some(&tied[..]), "{used:?}");
            assert_eq!(choice.stream(), Some(tied[0]), "{used:?}");
        }
    }

    /// A `w` x `h` image filled with grey but for the given lit texels.
    fn image(w: u32, h: u32, lit: &[(u32, u32)]) -> RgbaImage {
        let mut pixels = [GREY, GREY, GREY, 255].repeat((w * h) as usize);
        for &(x, y) in lit {
            let i = ((y * w + x) * 4) as usize;
            pixels[i] = 200;
        }
        RgbaImage {
            width: w,
            height: h,
            pixels,
        }
    }

    /// A surface of one triangle per entry of `tris` (its UVs in stream 0).
    fn triangles(tris: &[[[f32; 2]; 3]]) -> Surface {
        let mut s = surface(0, vec![tris.iter().flatten().copied().collect()]);
        let n = tris.len() * 3;
        s.indices = (0..n as u16).collect();
        s.positions = vec![[0.0; 3]; n];
        s.normals = vec![[0.0, 1.0, 0.0]; n];
        s
    }

    #[test]
    fn the_lit_texel_test_is_the_largest_channel_departure_from_grey_above_ten() {
        assert!(!is_lit(&[127, 127, 127, 255]));
        assert!(!is_lit(&[137, 117, 137, 255]));
        assert!(is_lit(&[127, 138, 127, 255]));
        assert!(is_lit(&[127, 127, 116, 255]));
        // Alpha is ignored; black is lit (far from grey).
        assert!(!is_lit(&[127, 127, 127, 0]));
        assert!(is_lit(&[0, 0, 0, 255]));
    }

    #[test]
    fn the_footprint_is_rasterized_on_the_lightmap_grid_at_texel_centres() {
        // The upper-left half of a 4 x 4 grid (v down the rows): the triangle (0,0) (1,0) (0,1)
        // in UV. Texel centres on the diagonal count (inclusive edges): 4 + 3 + 2 + 1 = 10.
        let s = [triangles(&[[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]])];
        let none = lit_fraction(&s, 0, &image(4, 4, &[]));
        assert_eq!(
            none,
            LitFraction {
                lit: 0,
                covered: 10
            }
        );
        // Row 0 (v near 0) is the top row of the image: no V flip. (3, 3) is outside.
        let f = lit_fraction(&s, 0, &image(4, 4, &[(0, 0), (3, 0), (3, 3)]));
        assert_eq!(
            f,
            LitFraction {
                lit: 2,
                covered: 10
            }
        );
        // A non-square image: the grid is the image's own.
        let f = lit_fraction(&s, 0, &image(8, 2, &[]));
        assert_eq!(f.covered, 6 + 2);
    }

    #[test]
    fn shared_edges_cover_once_and_degenerate_or_missing_triangles_cover_nothing() {
        // Two triangles splitting the square along the diagonal cover all 16 texels, the
        // diagonal centres counted once.
        let s = [triangles(&[
            [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            [[1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ])];
        assert_eq!(lit_fraction(&s, 0, &image(4, 4, &[])).covered, 16);
        // Zero area, not finite, or a stream the surface lacks: nothing.
        let flat = [triangles(&[[[0.0, 0.0], [1.0, 1.0], [0.5, 0.5]]])];
        assert_eq!(lit_fraction(&flat, 0, &image(4, 4, &[])).covered, 0);
        let nan = [triangles(&[[[0.0, 0.0], [f32::NAN, 0.0], [0.0, 1.0]]])];
        assert_eq!(lit_fraction(&nan, 0, &image(4, 4, &[])).covered, 0);
        assert_eq!(lit_fraction(&s, 1, &image(4, 4, &[])).covered, 0);
        // A triangle smaller than a texel that misses every centre covers nothing.
        let tiny = [triangles(&[[[0.0, 0.0], [0.1, 0.0], [0.0, 0.1]]])];
        assert_eq!(lit_fraction(&tiny, 0, &image(4, 4, &[])).covered, 0);
        // Coordinates past the image are clamped away, not wrapped.
        let past = [triangles(&[[[-1.0, -1.0], [2.0, -1.0], [-1.0, 2.0]]])];
        assert_eq!(lit_fraction(&past, 0, &image(2, 2, &[])).covered, 3);
    }

    fn frac(lit: u64, covered: u64) -> LitFraction {
        LitFraction { lit, covered }
    }

    #[test]
    fn the_highest_lit_fraction_wins_and_equal_ones_go_to_stage_three_order() {
        // Stage 2: the highest fraction, wherever it is in stage-3 order.
        assert_eq!(
            choose_tied(&[frac(99, 100), frac(10, 10)]),
            (1, Stage::LitFraction)
        );
        assert_eq!(
            choose_tied(&[frac(458, 1000), frac(998, 1000)]),
            (1, Stage::LitFraction)
        );
        assert_eq!(
            choose_tied(&[frac(9, 10), frac(8, 10)]),
            (0, Stage::LitFraction)
        );
        // Compared exactly: 1/3 and 2/6 are equal, so stage 3: the first.
        assert_eq!(choose_tied(&[frac(1, 3), frac(2, 6)]), (0, Stage::Order));
        assert_eq!(
            choose_tied(&[frac(1, 4), frac(1, 1), frac(64, 64)]),
            (1, Stage::Order)
        );
        // A footprint covering nothing counts as unlit.
        assert_eq!(
            choose_tied(&[frac(0, 0), frac(1, 9)]),
            (1, Stage::LitFraction)
        );
        assert_eq!(choose_tied(&[frac(0, 0), frac(0, 9)]), (0, Stage::Order));
        assert_eq!(choose_tied(&[]), (0, Stage::Order));
    }

    #[test]
    fn with_no_candidate_the_least_overlapping_stream_in_range_wins() {
        // Slot 0 tiles outside [0, 1], slot 1 is stacked inside it (all overlap), slot 2 is a
        // near-miss unwrap (a few percent of overlap, over the cutoff).
        let s = surface(
            4,
            vec![
                stream(4, |_| tile(3.0)),
                stream(4, |_| tile(1.0)),
                stream(4, widened(4, 0.3)),
            ],
        );
        let s = std::slice::from_ref(&s);
        assert!(overlap(s, 2) > MAX_OVERLAP);
        let choice = pick(s);
        assert_eq!(choice, StreamChoice::NearMiss { stream: 2 });
        assert_eq!(choice.stream(), Some(2));
    }

    #[test]
    fn a_stream_outside_the_unit_square_is_never_chosen() {
        // Neither stream is in [0, 1]; the material uses neither, slot 0 is lowest: still nothing.
        let s = surface(2, vec![stream(2, |_| tile(3.0)), stream(2, |_| tile(2.0))]);
        let choice = pick(&[s]);
        assert_eq!(choice, StreamChoice::NoneInRange);
        assert_eq!(choice.stream(), None);
        // A stream in range on one surface but missing from another is not in range either.
        let a = surface(1, vec![stream(1, |_| tile(2.0)), stream(1, |_| tile(1.0))]);
        let b = surface(1, vec![stream(1, |_| tile(2.0))]);
        assert_eq!(pick(&[a, b]), StreamChoice::NoneInRange);
        assert_eq!(pick(&[]), StreamChoice::NoneInRange);
        assert_eq!(pick(&[surface(2, vec![])]), StreamChoice::NoneInRange);
    }

    #[test]
    fn equal_overlaps_go_to_a_stream_the_material_does_not_use_then_the_lowest_slot() {
        // No candidate, two streams (ids 3 and 4) stacked alike: equal overlap.
        let t = surface(2, vec![stream(2, |_| tile(1.0)), stream(2, |_| tile(1.0))]);
        let t = std::slice::from_ref(&t);
        assert_eq!(pick(t), StreamChoice::NearMiss { stream: 0 });
        assert_eq!(choose(t, &[vec![3]]), StreamChoice::NearMiss { stream: 1 });
        assert_eq!(
            choose(t, &[vec![3, 4]]),
            StreamChoice::NearMiss { stream: 0 }
        );
        // An id used on any surface makes the stream used.
        let a = surface(1, vec![stream(1, |_| tile(1.0)), stream(1, |_| tile(1.0))]);
        let b = surface(1, vec![stream(1, |_| tile(1.0)), stream(1, |_| tile(1.0))]);
        assert_eq!(choose(&[a, b], &[vec![], vec![3]]).stream(), Some(1));
    }

    #[test]
    fn a_stream_missing_from_one_surface_or_not_finite_is_not_in_range() {
        let a = surface(
            1,
            vec![stream(1, |q| strip(q, 2)), stream(1, |q| strip(q, 2))],
        );
        let b = surface(1, vec![stream(1, |q| strip(q + 1, 2))]);
        assert_eq!(pick(&[a, b]), StreamChoice::One(0));
        let mut nan = stream(2, |q| strip(q, 2));
        nan[1] = [f32::NAN, 0.0];
        assert_eq!(pick(&[surface(2, vec![nan])]), StreamChoice::NoneInRange);
    }

    #[test]
    fn shared_edges_and_the_small_overlap_allowance() {
        // A grid of 16 x 16 quads tiling the square exactly: shared edges are not overlap.
        let n = 16;
        let grid = stream(n * n, |q| {
            let (x, y) = ((q % n) as f32 / n as f32, (q / n) as f32 / n as f32);
            let d = 1.0 / n as f32;
            [[x, y], [x + d, y], [x, y + d], [x + d, y + d]]
        });
        let s = surface(n * n, vec![grid]);
        assert_eq!(overlap(std::slice::from_ref(&s), 0), 0.0);
        assert_eq!(pick(&[s]), StreamChoice::One(0));
    }

    #[test]
    fn coordinates_a_hair_outside_the_square_still_count() {
        let mut s = stream(2, |q| strip(q, 2));
        s[0] = [-0.0005, 1.0005];
        assert_eq!(pick(&[surface(2, vec![s])]), StreamChoice::One(0));
    }
}
