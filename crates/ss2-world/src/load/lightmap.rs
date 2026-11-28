//! Which UV stream of a mesh is its lightmap's (`wld-lighting.md`, "Baked lightmaps", reader
//! invariant 3: T01-407's candidate test and T01-420's ranking).
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
//! - Several candidates: the one with the least overlap (T01-420).
//! - No candidate: the in-range stream with the least overlap, the 2 % cutoff relaxed but never
//!   the [0, 1] requirement (T01-420).
//! - Equal overlaps go to a stream the material does not use (no surface's material has the id
//!   that surface declares at that position among its UV map ids: the UV map of each non-empty
//!   texture group of every config, `mtr.md`), then to the lowest position.
//! - No stream in range at all: nothing is chosen. **This departs from the spec**, which keeps
//!   T01-414's slot-0 fallback there: such a slot 0 is a tiling channel, and sampling a lightmap
//!   through it tiles the texture into dark bands, so the model is drawn unlit instead.

use ss2_format_bmf::Surface;

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
    /// Several candidates (`candidates`, ascending): `stream` has the least overlap.
    Several {
        candidates: Vec<usize>,
        stream: usize,
    },
    /// No candidate: `stream` is the in-range stream with the least overlap.
    NearMiss { stream: usize },
    /// No stream is in range: none is taken (a departure from the spec's slot-0 fallback).
    NoneInRange,
}

impl StreamChoice {
    /// The position of the chosen stream, if any.
    pub(crate) fn stream(&self) -> Option<usize> {
        match self {
            Self::One(k) | Self::Several { stream: k, .. } | Self::NearMiss { stream: k } => {
                Some(*k)
            }
            Self::NoneInRange => None,
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
    // The least overlap; ties to a stream the material does not use, then the lowest position.
    let best = |pool: &[(f64, usize)]| {
        pool.iter()
            .min_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then(used(a.1).cmp(&used(b.1)))
                    .then(a.1.cmp(&b.1))
            })
            .map(|(_, k)| *k)
    };
    match candidates.as_slice() {
        [k] => StreamChoice::One(*k),
        [] => match best(&in_range) {
            Some(stream) => StreamChoice::NearMiss { stream },
            None => StreamChoice::NoneInRange,
        },
        _ => {
            let pool: Vec<(f64, usize)> = in_range
                .iter()
                .copied()
                .filter(|(o, _)| *o <= MAX_OVERLAP)
                .collect();
            let stream = best(&pool).unwrap_or(candidates[0]);
            StreamChoice::Several { candidates, stream }
        }
    }
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
    fn of_several_candidates_the_least_overlap_wins_over_the_lower_slot() {
        // Slot 0 overlaps a little (about 1 %, under the cutoff), slot 2 not at all.
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
        let choice = pick(s);
        assert_eq!(
            choice,
            StreamChoice::Several {
                candidates: vec![0, 2],
                stream: 2
            }
        );
        assert_eq!(choice.stream(), Some(2));
        // The overlap ranks before the material: slot 2 wins although the material uses it.
        assert_eq!(choose(s, &[vec![5]]).stream(), Some(2));
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
        // Several candidates with the same (zero) overlap: slots 0 and 1 (ids 3 and 4).
        let s = surface(
            2,
            vec![stream(2, |q| strip(q, 2)), stream(2, |q| strip(q, 2))],
        );
        let s = std::slice::from_ref(&s);
        assert_eq!(pick(s).stream(), Some(0));
        assert_eq!(choose(s, &[vec![3]]).stream(), Some(1));
        assert_eq!(choose(s, &[vec![3, 4]]).stream(), Some(0));
        // No candidate, two streams stacked alike: the same order.
        let t = surface(2, vec![stream(2, |_| tile(1.0)), stream(2, |_| tile(1.0))]);
        let t = std::slice::from_ref(&t);
        assert_eq!(pick(t), StreamChoice::NearMiss { stream: 0 });
        assert_eq!(choose(t, &[vec![3]]), StreamChoice::NearMiss { stream: 1 });
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
