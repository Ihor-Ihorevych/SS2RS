//! Which UV stream of a mesh is its lightmap's (`wld-lighting.md`, "Baked lightmaps", reader
//! invariant 3: T01-407 and the T01-414 tie-breaks).
//!
//! The world's oven modifier names a UV map id that is none of the mesh's own ids (reader
//! invariant 3), so the stream is found from the mesh data: a lightmap unwrap lies entirely
//! inside [0, 1] and its triangles do not overlap (each texel lights one spot). A stream is a
//! **candidate** when, over every surface of the level of detail that has triangles, its
//! coordinates are inside [0, 1] (within [`RANGE_EPS`]) and at most [`MAX_OVERLAP`] of the
//! rasterized area of all its triangles (all surfaces together, since they share one lightmap)
//! is covered twice or more. Streams are identified by their position in each surface's
//! [`Surface::uv_sets`] (slot order, `bmf.md` member 12/14). Exactly one candidate is the
//! lightmap's stream; of several the lowest-indexed one is taken; with none the mesh's slot 0
//! is taken regardless of its range or overlap. Only a mesh where some surface with triangles
//! has no stream at all (no slot 0) is left without one.

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
    /// Several streams qualify (their positions, ascending): the first is taken.
    Lowest(Vec<usize>),
    /// No stream qualifies: slot 0 is taken.
    SlotZero,
    /// No stream qualifies and some surface with triangles has no slot 0: nothing is taken.
    Nothing,
}

impl StreamChoice {
    /// The position of the chosen stream, if any.
    pub(crate) fn stream(&self) -> Option<usize> {
        match self {
            Self::One(k) => Some(*k),
            Self::Lowest(ks) => ks.first().copied(),
            Self::SlotZero => Some(0),
            Self::Nothing => None,
        }
    }
}

/// Choose the lightmap stream of the surfaces of one level of detail.
pub(crate) fn choose(surfaces: &[Surface]) -> StreamChoice {
    let drawn = || surfaces.iter().filter(|s| s.indices.len() >= 3);
    let streams = drawn().map(|s| s.uv_sets.len()).max().unwrap_or(0);
    let candidates: Vec<usize> = (0..streams)
        .filter(|&k| is_candidate(surfaces, k))
        .collect();
    match candidates.as_slice() {
        [] if streams > 0 && drawn().all(|s| !s.uv_sets.is_empty()) => StreamChoice::SlotZero,
        [] => StreamChoice::Nothing,
        [k] => StreamChoice::One(*k),
        _ => StreamChoice::Lowest(candidates),
    }
}

/// Whether stream `k` is in [0, 1] and does not overlap, over every surface with triangles.
fn is_candidate(surfaces: &[Surface], k: usize) -> bool {
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
    any && overlap(surfaces, k) <= MAX_OVERLAP
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

    #[test]
    fn the_unwrap_is_chosen_over_a_tiling_stream() {
        // Slot 0 tiles (outside [0, 1]), slot 1 is the unwrap.
        let s = surface(
            4,
            vec![stream(4, |_| tile(3.0)), stream(4, |q| strip(q, 4))],
        );
        assert_eq!(choose(&[s]), StreamChoice::One(1));
    }

    #[test]
    fn a_stream_inside_the_unit_square_that_overlaps_is_not_a_candidate() {
        // Slot 0 maps every quad onto the whole square (inside [0, 1] but stacked).
        let s = surface(
            4,
            vec![stream(4, |_| tile(1.0)), stream(4, |q| strip(q, 4))],
        );
        assert_eq!(choose(&[s]), StreamChoice::One(1));
    }

    #[test]
    fn the_surfaces_of_a_mesh_share_the_lightmap() {
        // Each surface alone is a fine unwrap, but in slot 0 both use the same half: they
        // overlap. In slot 1 they use different halves.
        let left = |q| strip(q, 4);
        let right = |q| strip(q + 2, 4);
        let a = surface(2, vec![stream(2, left), stream(2, left)]);
        let b = surface(2, vec![stream(2, left), stream(2, right)]);
        assert_eq!(choose(&[a, b]), StreamChoice::One(1));
    }

    #[test]
    fn several_candidates_take_the_lowest_indexed_one() {
        let twice = surface(
            2,
            vec![stream(2, |q| strip(q, 2)), stream(2, |q| strip(q, 4))],
        );
        let choice = choose(&[twice]);
        assert_eq!(choice, StreamChoice::Lowest(vec![0, 1]));
        assert_eq!(choice.stream(), Some(0));
        // Slot 0 tiles; slots 1 and 2 are both unwraps: slot 1 wins.
        let later = surface(
            2,
            vec![
                stream(2, |_| tile(3.0)),
                stream(2, |q| strip(q, 2)),
                stream(2, |q| strip(q, 4)),
            ],
        );
        let choice = choose(&[later]);
        assert_eq!(choice, StreamChoice::Lowest(vec![1, 2]));
        assert_eq!(choice.stream(), Some(1));
    }

    #[test]
    fn no_candidate_falls_back_to_slot_zero() {
        // Slot 0 tiles outside [0, 1], slot 1 is stacked inside it: neither qualifies.
        let tiling = surface(2, vec![stream(2, |_| tile(2.0)), stream(2, |_| tile(1.0))]);
        let choice = choose(&[tiling]);
        assert_eq!(choice, StreamChoice::SlotZero);
        assert_eq!(choice.stream(), Some(0));
    }

    #[test]
    fn without_a_slot_zero_nothing_is_chosen() {
        assert_eq!(choose(&[]), StreamChoice::Nothing);
        assert_eq!(choose(&[surface(2, vec![])]), StreamChoice::Nothing);
        // One surface has streams (none qualifies), the other none at all.
        let tiling = surface(2, vec![stream(2, |_| tile(2.0))]);
        let bare = surface(1, vec![]);
        let choice = choose(&[tiling, bare]);
        assert_eq!(choice, StreamChoice::Nothing);
        assert_eq!(choice.stream(), None);
    }

    #[test]
    fn a_single_candidate_is_still_taken_as_it_is() {
        let s = surface(
            4,
            vec![stream(4, |_| tile(3.0)), stream(4, |q| strip(q, 4))],
        );
        let choice = choose(&[s]);
        assert_eq!(choice, StreamChoice::One(1));
        assert_eq!(choice.stream(), Some(1));
    }

    #[test]
    fn a_stream_missing_from_one_surface_or_not_finite_is_not_a_candidate() {
        let a = surface(
            1,
            vec![stream(1, |q| strip(q, 2)), stream(1, |q| strip(q, 2))],
        );
        let b = surface(1, vec![stream(1, |q| strip(q + 1, 2))]);
        assert_eq!(choose(&[a, b]), StreamChoice::One(0));
        let mut nan = stream(2, |q| strip(q, 2));
        nan[1] = [f32::NAN, 0.0];
        assert_eq!(choose(&[surface(2, vec![nan])]), StreamChoice::SlotZero);
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
        assert_eq!(choose(&[s]), StreamChoice::One(0));
    }

    #[test]
    fn coordinates_a_hair_outside_the_square_still_count() {
        let mut s = stream(2, |q| strip(q, 2));
        s[0] = [-0.0005, 1.0005];
        assert_eq!(choose(&[surface(2, vec![s])]), StreamChoice::One(0));
    }
}
