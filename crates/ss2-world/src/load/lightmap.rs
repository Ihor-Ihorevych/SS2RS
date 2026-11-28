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
//! [`Surface::uv_sets`] (slot order, `bmf.md` member 12/14).
//!
//! Exactly one candidate is the lightmap's stream. With no candidate, the spec's data says the
//! lightmap stream is usually one the surface's own material does not use for any of its texture
//! groups (T01-407: 83 % of the single-candidate instances), so a stream is **material-unused**
//! when, on no surface with triangles, the material's UV map ids (the UV map of each non-empty
//! texture group of every config, `mtr.md`) include the id that surface declares at that
//! position; of the streams every surface with triangles has, the lowest-indexed material-unused
//! one is taken, else slot 0 (T01-414's fallback), regardless of range or overlap.
//!
//! With several candidates no stream is chosen yet: T01-414's lowest-index rule picked a stream
//! that renders visibly wrong on a real level, and preferring a material-unused one does not
//! separate the candidates there, so these meshes wait on T01-420. A mesh where some surface with
//! triangles has no stream at all gets none either.

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
    /// Several streams qualify (their positions, ascending): none is taken (T01-420).
    Several(Vec<usize>),
    /// No stream qualifies; `stream` is taken: the lowest material-unused one when `unused`,
    /// else slot 0.
    NoCandidate { stream: usize, unused: bool },
    /// No stream qualifies and some surface with triangles has no stream: nothing is taken.
    Nothing,
}

impl StreamChoice {
    /// The position of the chosen stream, if any.
    pub(crate) fn stream(&self) -> Option<usize> {
        match self {
            Self::One(k) => Some(*k),
            Self::NoCandidate { stream, .. } => Some(*stream),
            Self::Several(_) | Self::Nothing => None,
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
    let unused = |k: usize| {
        drawn().all(|(i, s)| {
            let used = material_uv_ids.get(i).map_or(&[][..], Vec::as_slice);
            s.uv_map_ids.get(k).is_none_or(|id| !used.contains(id))
        })
    };
    let candidates: Vec<usize> = (0..streams)
        .filter(|&k| is_candidate(surfaces, k))
        .collect();
    match candidates.as_slice() {
        [] => {
            // Only a stream every surface with triangles has.
            let common = drawn().map(|(_, s)| s.uv_sets.len()).min().unwrap_or(0);
            if common == 0 {
                return StreamChoice::Nothing;
            }
            match (0..common).find(|&k| unused(k)) {
                Some(stream) => StreamChoice::NoCandidate {
                    stream,
                    unused: true,
                },
                None => StreamChoice::NoCandidate {
                    stream: 0,
                    unused: false,
                },
            }
        }
        [k] => StreamChoice::One(*k),
        _ => StreamChoice::Several(candidates),
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

    /// Choose with no material: every stream is material-unused.
    fn pick(surfaces: &[Surface]) -> StreamChoice {
        choose(surfaces, &[])
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
    fn several_candidates_choose_nothing_even_if_the_material_uses_some() {
        // Slot 0 tiles; slots 1 and 2 (UV ids 4 and 5) are both unwraps.
        let s = surface(
            2,
            vec![
                stream(2, |_| tile(3.0)),
                stream(2, |q| strip(q, 2)),
                stream(2, |q| strip(q, 4)),
            ],
        );
        let s = std::slice::from_ref(&s);
        for used in [vec![], vec![4], vec![4, 5]] {
            let choice = choose(s, &[used]);
            assert_eq!(choice, StreamChoice::Several(vec![1, 2]));
            assert_eq!(choice.stream(), None);
        }
    }

    #[test]
    fn an_id_used_on_any_surface_makes_the_stream_used() {
        // No candidate: both slots tile outside [0, 1].
        let a = surface(1, vec![stream(1, |_| tile(2.0)), stream(1, |_| tile(2.0))]);
        let b = surface(1, vec![stream(1, |_| tile(2.0)), stream(1, |_| tile(2.0))]);
        // Surface 1's material uses id 3 (slot 0), surface 0 has none.
        assert_eq!(choose(&[a, b], &[vec![], vec![3]]).stream(), Some(1));
    }

    #[test]
    fn no_candidate_prefers_the_lowest_material_unused_stream_then_slot_zero() {
        // Slot 0 tiles outside [0, 1], slot 1 is stacked inside it: neither qualifies.
        let s = surface(2, vec![stream(2, |_| tile(2.0)), stream(2, |_| tile(1.0))]);
        let s = std::slice::from_ref(&s);
        let none = |stream, unused| StreamChoice::NoCandidate { stream, unused };
        assert_eq!(pick(s), none(0, true));
        // The material uses id 3 (slot 0): slot 1.
        let choice = choose(s, &[vec![3]]);
        assert_eq!(choice, none(1, true));
        assert_eq!(choice.stream(), Some(1));
        // It uses both: slot 0.
        let choice = choose(s, &[vec![4, 3]]);
        assert_eq!(choice, none(0, false));
        assert_eq!(choice.stream(), Some(0));
    }

    #[test]
    fn no_candidate_only_takes_a_stream_every_surface_has() {
        // Surface 1 has only slot 0, which surface 0's material uses (id 3); slot 1 is missing
        // from surface 1, so slot 0 is taken although it is used.
        let a = surface(2, vec![stream(2, |_| tile(2.0)), stream(2, |_| tile(2.0))]);
        let b = surface(2, vec![stream(2, |_| tile(2.0))]);
        assert_eq!(
            choose(&[a, b], &[vec![3]]),
            StreamChoice::NoCandidate {
                stream: 0,
                unused: false
            }
        );
    }

    #[test]
    fn without_a_stream_on_every_surface_nothing_is_chosen() {
        assert_eq!(pick(&[]), StreamChoice::Nothing);
        assert_eq!(pick(&[surface(2, vec![])]), StreamChoice::Nothing);
        // One surface has a stream (it does not qualify), the other none at all.
        let tiling = surface(2, vec![stream(2, |_| tile(2.0))]);
        let bare = surface(1, vec![]);
        let choice = pick(&[tiling, bare]);
        assert_eq!(choice, StreamChoice::Nothing);
        assert_eq!(choice.stream(), None);
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
    fn a_stream_missing_from_one_surface_or_not_finite_is_not_a_candidate() {
        let a = surface(
            1,
            vec![stream(1, |q| strip(q, 2)), stream(1, |q| strip(q, 2))],
        );
        let b = surface(1, vec![stream(1, |q| strip(q + 1, 2))]);
        assert_eq!(pick(&[a, b]), StreamChoice::One(0));
        let mut nan = stream(2, |q| strip(q, 2));
        nan[1] = [f32::NAN, 0.0];
        assert_eq!(
            pick(&[surface(2, vec![nan])]),
            StreamChoice::NoCandidate {
                stream: 0,
                unused: true
            }
        );
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
