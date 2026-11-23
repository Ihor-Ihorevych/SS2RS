//! Transparency: **blend modes** and the **order things are drawn in** (roadmap T04-016, `docs/TRANSPARENCY.md`).
//!
//! Device-free: the descriptions of what each mode does to the framebuffer ([`BlendMode::factors`]) for the wgpu backend to map one to one, the sort that
//! orders a frame's draws ([`sort_draws`]), the choice of alpha-to-coverage ([`uses_alpha_to_coverage`]) and the soft-particle fade
//! ([`soft_particle_fade`]) with the depth conversion it needs ([`linear_depth_from_reverse_z`]). Nothing here draws.
//!
//! * **Opaque** and **alpha-tested** draws write depth and do not depend on the order (the depth buffer sorts them), so they are grouped by material to
//!   cut state changes, and nearest first within a material so early depth rejection works.
//! * **Blended** draws (alpha, premultiplied alpha, additive) do not write depth and are drawn **after** all of those, **farthest first**, so what is
//!   nearer covers what is behind it. An artist can push a draw earlier or later with a **layer** (water before glass before particles) and a **bias**
//!   (a distance added to its depth).
//! * The order is **total and deterministic**: depth ties are broken by material, then by [`RenderId`], so the same frame draws in the same order on every
//!   machine and every run (a flickering order between equal depths is a bug that only shows up sometimes).

use std::cmp::Ordering;

use ss2_core::math::Vec3;

use crate::{Camera, RenderId};

/// How a draw combines with what is already in the framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Replaces the pixel; writes depth.
    Opaque,
    /// Replaces the pixel where the texture's alpha passes a cut-off, discards the rest; writes depth. (The cut-off is a material value.)
    AlphaTest,
    /// `src x alpha + dst x (1 - alpha)`: glass, smoke, fog cards. Drawn in order.
    Alpha,
    /// `src x alpha + dst`: fire, glows, sparks. The result does not depend on the order.
    Additive,
    /// `src + dst x (1 - alpha)` with a colour already multiplied by its alpha: correct filtering and both glow and shade in one texture. Drawn in order.
    Premultiplied,
}

/// A factor of the blend equation (the names of the usual graphics APIs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Factor {
    /// 0.
    Zero,
    /// 1.
    One,
    /// The source alpha.
    SrcAlpha,
    /// 1 minus the source alpha.
    OneMinusSrcAlpha,
}

/// What the blend unit does: `result = src x src_factor + dst x dst_factor`, the same for colour and alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlendFactors {
    /// Applied to the fragment's colour.
    pub src: Factor,
    /// Applied to what is in the framebuffer.
    pub dst: Factor,
}

impl BlendMode {
    /// Every mode.
    pub const ALL: [BlendMode; 5] = [
        Self::Opaque,
        Self::AlphaTest,
        Self::Alpha,
        Self::Additive,
        Self::Premultiplied,
    ];

    /// The blend equation, or `None` for the modes that do not blend.
    pub fn factors(self) -> Option<BlendFactors> {
        match self {
            Self::Opaque | Self::AlphaTest => None,
            Self::Alpha => Some(BlendFactors {
                src: Factor::SrcAlpha,
                dst: Factor::OneMinusSrcAlpha,
            }),
            Self::Additive => Some(BlendFactors {
                src: Factor::SrcAlpha,
                dst: Factor::One,
            }),
            Self::Premultiplied => Some(BlendFactors {
                src: Factor::One,
                dst: Factor::OneMinusSrcAlpha,
            }),
        }
    }

    /// Whether the draw blends with the framebuffer.
    pub fn is_blended(self) -> bool {
        self.factors().is_some()
    }

    /// Whether the draw writes depth (the blended ones do not: a glass pane must not hide what is behind it from the next draw).
    pub fn writes_depth(self) -> bool {
        !self.is_blended()
    }

    /// Whether the result depends on the order of draws: alpha and premultiplied do; additive does not (a sum), nor do the unblended modes.
    pub fn order_dependent(self) -> bool {
        matches!(self, Self::Alpha | Self::Premultiplied)
    }

    /// The name used in material files and consoles.
    pub fn name(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::AlphaTest => "alpha_test",
            Self::Alpha => "alpha",
            Self::Additive => "additive",
            Self::Premultiplied => "premultiplied",
        }
    }

    /// The mode called `name` (any case).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|m| m.name().eq_ignore_ascii_case(name.trim()))
    }
}

/// One draw to be ordered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawItem {
    /// The thing drawn.
    pub id: RenderId,
    /// How it blends.
    pub mode: BlendMode,
    /// The material (any value that is equal for draws that can share state).
    pub material: u32,
    /// Distance in front of the camera along its view direction; see [`view_depth`].
    pub depth: f32,
    /// Among blended draws, lower layers are drawn first whatever their depth (0 for most things).
    pub layer: i16,
    /// Added to the depth of a blended draw before sorting: positive draws it earlier (as if farther), negative later.
    pub bias: f32,
}

impl DrawItem {
    /// A draw in layer 0 with no bias.
    pub fn new(id: RenderId, mode: BlendMode, material: u32, depth: f32) -> Self {
        Self {
            id,
            mode,
            material,
            depth,
            layer: 0,
            bias: 0.0,
        }
    }
}

/// How far in front of the camera a point is, along the view direction (negative behind it). This, not the straight-line distance, is what the depth buffer
/// and a sort of flat layers want: two cards side by side at the same depth do not change places as the camera turns.
pub fn view_depth(camera: &Camera, point: Vec3) -> f32 {
    (point - camera.position).dot(camera.forward())
}

/// A depth that sorts: a non-finite one (a NaN position) is treated as the farthest possible, so the draw goes first among the blended and cannot poison the order.
fn sane(depth: f32) -> f32 {
    if depth.is_finite() { depth } else { f32::MAX }
}

fn phase(mode: BlendMode) -> u8 {
    match mode {
        BlendMode::Opaque => 0,
        BlendMode::AlphaTest => 1,
        _ => 2,
    }
}

fn compare(a: &DrawItem, b: &DrawItem) -> Ordering {
    let (pa, pb) = (phase(a.mode), phase(b.mode));
    pa.cmp(&pb).then_with(|| {
        if pa < 2 {
            // Depth-written draws: by material, then nearest first, then by id.
            a.material
                .cmp(&b.material)
                .then(sane(a.depth).total_cmp(&sane(b.depth)))
                .then(a.id.cmp(&b.id))
        } else {
            // Blended draws: by layer, then farthest first (with the bias), then by material and id.
            let da = sane(a.depth) + a.bias;
            let db = sane(b.depth) + b.bias;
            a.layer
                .cmp(&b.layer)
                .then(db.total_cmp(&da))
                .then(a.material.cmp(&b.material))
                .then(a.id.cmp(&b.id))
        }
    })
}

/// How many draws of each kind a sort found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SortCounts {
    /// Opaque draws (the first `opaque` items).
    pub opaque: usize,
    /// Alpha-tested draws (the next `alpha_test`).
    pub alpha_test: usize,
    /// Blended draws (the rest).
    pub blended: usize,
}

/// Put a frame's draws in drawing order (see the module documentation) and say where each kind starts and ends.
///
/// ```
/// use ss2_render_api::transparency::{BlendMode, DrawItem, sort_draws};
/// use ss2_render_api::RenderId;
///
/// let mut draws = vec![
///     DrawItem::new(RenderId(1), BlendMode::Alpha, 7, 5.0),   // glass, near
///     DrawItem::new(RenderId(2), BlendMode::Opaque, 3, 9.0),  // a wall
///     DrawItem::new(RenderId(3), BlendMode::Alpha, 7, 20.0),  // glass, far
/// ];
/// let counts = sort_draws(&mut draws);
/// let order: Vec<u64> = draws.iter().map(|d| d.id.0).collect();
/// assert_eq!(order, [2, 3, 1]); // the wall, then the far glass, then the near glass
/// assert_eq!((counts.opaque, counts.alpha_test, counts.blended), (1, 0, 2));
/// ```
pub fn sort_draws(draws: &mut [DrawItem]) -> SortCounts {
    draws.sort_unstable_by(compare);
    let mut counts = SortCounts::default();
    for d in draws.iter() {
        match phase(d.mode) {
            0 => counts.opaque += 1,
            1 => counts.alpha_test += 1,
            _ => counts.blended += 1,
        }
    }
    counts
}

/// Whether a draw should use alpha-to-coverage: only an **alpha-tested** draw, only with **multisampling** (more than one sample per pixel) and only when
/// the option is on. It turns the hard cut-off into smooth edges on foliage and fences; with one sample per pixel it would do nothing useful.
pub fn uses_alpha_to_coverage(mode: BlendMode, msaa_samples: u32, enabled: bool) -> bool {
    enabled && mode == BlendMode::AlphaTest && msaa_samples > 1
}

/// The distance in front of the camera that a **reverse-Z, infinite far plane** depth value stands for: `near / depth` (the engine's projection maps the near
/// plane to depth 1 and infinity to 0). A depth of 0 or below is infinitely far; a depth above 1 is nearer than the near plane and is clamped to it.
pub fn linear_depth_from_reverse_z(depth: f32, near: f32) -> f32 {
    if depth.is_nan() || depth <= 0.0 {
        return f32::INFINITY;
    }
    near / depth.min(1.0)
}

/// The opacity multiplier of a **soft particle**: 0 where the particle touches the geometry behind it and 1 once it is `fade_distance` in front of it, so a
/// smoke card does not cut a hard line where it meets the floor. Both depths are distances in front of the camera ([`linear_depth_from_reverse_z`] turns a
/// depth-buffer value into one). A scene behind the particle (a negative gap) gives 0; a non-positive fade distance turns the effect off (always 1).
pub fn soft_particle_fade(scene_depth: f32, particle_depth: f32, fade_distance: f32) -> f32 {
    if fade_distance.is_nan() || fade_distance <= 0.0 {
        return 1.0;
    }
    let gap = scene_depth - particle_depth;
    if gap.is_nan() {
        return 1.0;
    }
    (gap / fade_distance).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::math::{Quat, Vec4};

    fn item(id: u64, mode: BlendMode, material: u32, depth: f32) -> DrawItem {
        DrawItem::new(RenderId(id), mode, material, depth)
    }

    fn ids(draws: &[DrawItem]) -> Vec<u64> {
        draws.iter().map(|d| d.id.0).collect()
    }

    // ---- blend modes -------------------------------------------------------------------------------

    #[test]
    fn each_mode_has_the_blend_equation_of_its_name() {
        assert_eq!(BlendMode::Opaque.factors(), None);
        assert_eq!(BlendMode::AlphaTest.factors(), None);
        assert_eq!(
            BlendMode::Alpha.factors(),
            Some(BlendFactors {
                src: Factor::SrcAlpha,
                dst: Factor::OneMinusSrcAlpha
            })
        );
        assert_eq!(
            BlendMode::Additive.factors(),
            Some(BlendFactors {
                src: Factor::SrcAlpha,
                dst: Factor::One
            })
        );
        assert_eq!(
            BlendMode::Premultiplied.factors(),
            Some(BlendFactors {
                src: Factor::One,
                dst: Factor::OneMinusSrcAlpha
            })
        );
    }

    /// Evaluate the equation for one channel, to check the factors do what the doc says.
    fn blend(f: BlendFactors, src: f32, alpha: f32, dst: f32) -> f32 {
        let apply = |factor: Factor| match factor {
            Factor::Zero => 0.0,
            Factor::One => 1.0,
            Factor::SrcAlpha => alpha,
            Factor::OneMinusSrcAlpha => 1.0 - alpha,
        };
        src * apply(f.src) + dst * apply(f.dst)
    }

    #[test]
    fn the_equations_give_the_documented_results() {
        let alpha = BlendMode::Alpha.factors().unwrap();
        assert!((blend(alpha, 1.0, 0.25, 0.4) - (0.25 + 0.75 * 0.4)).abs() < 1e-6);
        assert_eq!(blend(alpha, 0.7, 1.0, 0.4), 0.7, "opaque alpha replaces");
        assert_eq!(
            blend(alpha, 0.7, 0.0, 0.4),
            0.4,
            "zero alpha leaves the framebuffer"
        );
        let add = BlendMode::Additive.factors().unwrap();
        assert!(
            (blend(add, 0.5, 0.5, 0.25) - 0.5).abs() < 1e-6,
            "0.5 x 0.5 + 0.25"
        );
        let pre = BlendMode::Premultiplied.factors().unwrap();
        assert!(
            (blend(pre, 0.2, 0.5, 0.4) - 0.4).abs() < 1e-6,
            "0.2 + 0.4 x 0.5"
        );
        // Order independence: additive commutes, alpha and premultiplied do not.
        let twice = |f: BlendFactors, a: (f32, f32), b: (f32, f32)| {
            let first = blend(f, a.0, a.1, 0.3);
            blend(f, b.0, b.1, first)
        };
        let (x, y) = ((0.9, 0.3), (0.2, 0.6));
        assert!((twice(add, x, y) - twice(add, y, x)).abs() < 1e-6);
        assert!((twice(alpha, x, y) - twice(alpha, y, x)).abs() > 1e-3);
        assert!((twice(pre, x, y) - twice(pre, y, x)).abs() > 1e-3);
    }

    #[test]
    fn mode_properties_agree_with_each_other() {
        for m in BlendMode::ALL {
            assert_eq!(m.is_blended(), m.factors().is_some());
            assert_eq!(m.writes_depth(), !m.is_blended());
            if m.order_dependent() {
                assert!(m.is_blended(), "{m:?}");
            }
        }
        assert!(BlendMode::Alpha.order_dependent() && BlendMode::Premultiplied.order_dependent());
        assert!(
            !BlendMode::Additive.order_dependent()
                && !BlendMode::Opaque.order_dependent()
                && !BlendMode::AlphaTest.order_dependent()
        );
        assert!(
            BlendMode::AlphaTest.writes_depth()
                && BlendMode::Opaque.writes_depth()
                && !BlendMode::Additive.writes_depth()
        );
    }

    #[test]
    fn modes_have_names_that_read_back() {
        for m in BlendMode::ALL {
            assert_eq!(BlendMode::from_name(m.name()), Some(m));
            assert_eq!(BlendMode::from_name(&m.name().to_uppercase()), Some(m));
        }
        assert_eq!(BlendMode::from_name(" alpha "), Some(BlendMode::Alpha));
        assert_eq!(BlendMode::from_name("multiply"), None);
    }

    // ---- sorting -----------------------------------------------------------------------------------

    #[test]
    fn opaque_then_alpha_tested_then_blended_each_in_its_own_order() {
        let mut d = vec![
            item(1, BlendMode::Additive, 1, 5.0),
            item(2, BlendMode::Opaque, 9, 2.0),
            item(3, BlendMode::AlphaTest, 4, 3.0),
            item(4, BlendMode::Alpha, 1, 8.0),
            item(5, BlendMode::Opaque, 2, 6.0),
            item(6, BlendMode::Premultiplied, 1, 1.0),
        ];
        let counts = sort_draws(&mut d);
        assert_eq!(
            counts,
            SortCounts {
                opaque: 2,
                alpha_test: 1,
                blended: 3
            }
        );
        // Opaque by material (2 before 9); alpha test; blended farthest first: 4 (8), 1 (5), 6 (1).
        assert_eq!(ids(&d), [5, 2, 3, 4, 1, 6]);
    }

    #[test]
    fn opaque_draws_group_by_material_and_go_nearest_first_within_one() {
        let mut d = vec![
            item(1, BlendMode::Opaque, 5, 10.0),
            item(2, BlendMode::Opaque, 3, 30.0),
            item(3, BlendMode::Opaque, 5, 2.0),
            item(4, BlendMode::Opaque, 3, 4.0),
        ];
        sort_draws(&mut d);
        assert_eq!(ids(&d), [4, 2, 3, 1]);
    }

    #[test]
    fn blended_draws_go_farthest_first_whatever_their_material() {
        let mut d = vec![
            item(1, BlendMode::Alpha, 1, 3.0),
            item(2, BlendMode::Alpha, 9, 30.0),
            item(3, BlendMode::Alpha, 5, 12.0),
        ];
        sort_draws(&mut d);
        assert_eq!(ids(&d), [2, 3, 1]);
    }

    #[test]
    fn layers_beat_depth_and_bias_moves_a_draw_within_its_layer() {
        let mut d = vec![
            DrawItem {
                layer: 1,
                ..item(1, BlendMode::Alpha, 1, 50.0)
            }, // far but in a later layer
            DrawItem {
                layer: 0,
                ..item(2, BlendMode::Alpha, 1, 5.0)
            }, // near, early layer
            DrawItem {
                layer: -1,
                ..item(3, BlendMode::Alpha, 1, 1.0)
            }, // nearest, earliest layer
        ];
        sort_draws(&mut d);
        assert_eq!(ids(&d), [3, 2, 1]);
        // A bias of +10 makes a draw at depth 5 sort as if at 15: before one at depth 12.
        let mut d = vec![
            item(1, BlendMode::Alpha, 1, 12.0),
            DrawItem {
                bias: 10.0,
                ..item(2, BlendMode::Alpha, 1, 5.0)
            },
        ];
        sort_draws(&mut d);
        assert_eq!(ids(&d), [2, 1]);
        // A negative one pushes it later.
        let mut d = vec![
            item(1, BlendMode::Alpha, 1, 12.0),
            DrawItem {
                bias: -10.0,
                ..item(2, BlendMode::Alpha, 1, 20.0)
            },
        ];
        sort_draws(&mut d);
        assert_eq!(
            ids(&d),
            [1, 2],
            "20 - 10 = 10 sorts as nearer than 12, so it is drawn later"
        );
    }

    #[test]
    fn the_bias_does_not_affect_opaque_draws() {
        let mut d = vec![
            DrawItem {
                bias: 100.0,
                ..item(1, BlendMode::Opaque, 1, 2.0)
            },
            item(2, BlendMode::Opaque, 1, 5.0),
        ];
        sort_draws(&mut d);
        assert_eq!(ids(&d), [1, 2]);
    }

    #[test]
    fn equal_depths_are_broken_by_material_then_id_so_the_order_is_total() {
        let mut d = vec![
            item(9, BlendMode::Alpha, 2, 7.0),
            item(3, BlendMode::Alpha, 2, 7.0),
            item(5, BlendMode::Alpha, 1, 7.0),
            item(4, BlendMode::Opaque, 1, 7.0),
            item(2, BlendMode::Opaque, 1, 7.0),
        ];
        sort_draws(&mut d);
        assert_eq!(ids(&d), [2, 4, 5, 3, 9]);
    }

    #[test]
    fn the_order_does_not_depend_on_the_input_order() {
        // A pile of draws with plenty of equal depths and materials, shuffled by a fixed generator, always sorts to the same list.
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let modes = BlendMode::ALL;
        let base: Vec<DrawItem> = (0..300)
            .map(|i| DrawItem {
                id: RenderId(i),
                mode: modes[(rnd() % 5) as usize],
                material: (rnd() % 4) as u32,
                depth: (rnd() % 6) as f32,
                layer: (rnd() % 3) as i16 - 1,
                bias: (rnd() % 3) as f32,
            })
            .collect();
        let mut reference = base.clone();
        sort_draws(&mut reference);
        for _ in 0..20 {
            let mut shuffled = base.clone();
            for i in (1..shuffled.len()).rev() {
                shuffled.swap(i, (rnd() % (i as u64 + 1)) as usize);
            }
            sort_draws(&mut shuffled);
            assert_eq!(shuffled, reference);
        }
    }

    #[test]
    fn a_draw_with_a_nan_or_infinite_depth_cannot_break_the_order() {
        let mut d = vec![
            item(1, BlendMode::Alpha, 1, 4.0),
            item(2, BlendMode::Alpha, 1, f32::NAN),
            item(3, BlendMode::Alpha, 1, 9.0),
            item(4, BlendMode::Alpha, 1, f32::NEG_INFINITY),
            item(5, BlendMode::Opaque, 1, f32::NAN),
            item(6, BlendMode::Opaque, 1, 1.0),
        ];
        sort_draws(&mut d);
        // Opaque: the finite one is nearer than the NaN one. Blended: the two bad depths count as farthest (ids 2 then 4 by id), then 9, then 4.
        assert_eq!(ids(&d), [6, 5, 2, 4, 3, 1]);
    }

    #[test]
    fn an_empty_list_and_a_single_draw_sort() {
        let mut none: Vec<DrawItem> = Vec::new();
        assert_eq!(sort_draws(&mut none), SortCounts::default());
        let mut one = vec![item(1, BlendMode::Additive, 0, 1.0)];
        assert_eq!(
            sort_draws(&mut one),
            SortCounts {
                opaque: 0,
                alpha_test: 0,
                blended: 1
            }
        );
    }

    // ---- view depth ----------------------------------------------------------------------------------

    #[test]
    fn view_depth_is_the_distance_along_the_view_direction() {
        let cam = Camera {
            position: Vec3::new(1.0, 2.0, 3.0),
            ..Camera::default()
        };
        assert!((view_depth(&cam, Vec3::new(1.0, 2.0, -7.0)) - 10.0).abs() < 1e-6);
        // Sideways does not change it; behind is negative.
        assert!((view_depth(&cam, Vec3::new(40.0, 2.0, -7.0)) - 10.0).abs() < 1e-6);
        assert!(view_depth(&cam, Vec3::new(1.0, 2.0, 8.0)) < 0.0);
        // A camera turned a quarter to the left looks down -X.
        let turned = Camera {
            rotation: Quat::from_rotation_y(90f32.to_radians()),
            ..Camera::default()
        };
        assert!((view_depth(&turned, Vec3::new(-5.0, 0.0, 0.0)) - 5.0).abs() < 1e-5);
    }

    // ---- alpha to coverage -----------------------------------------------------------------------------

    #[test]
    fn alpha_to_coverage_needs_the_option_alpha_test_and_multisampling() {
        assert!(uses_alpha_to_coverage(BlendMode::AlphaTest, 4, true));
        assert!(uses_alpha_to_coverage(BlendMode::AlphaTest, 2, true));
        assert!(
            !uses_alpha_to_coverage(BlendMode::AlphaTest, 1, true),
            "one sample per pixel"
        );
        assert!(!uses_alpha_to_coverage(BlendMode::AlphaTest, 0, true));
        assert!(
            !uses_alpha_to_coverage(BlendMode::AlphaTest, 4, false),
            "option off"
        );
        for m in [
            BlendMode::Opaque,
            BlendMode::Alpha,
            BlendMode::Additive,
            BlendMode::Premultiplied,
        ] {
            assert!(!uses_alpha_to_coverage(m, 4, true), "{m:?}");
        }
    }

    // ---- soft particles ---------------------------------------------------------------------------------

    #[test]
    fn reverse_z_depth_converts_back_to_the_distance_the_projection_started_from() {
        // Check against the engine's own projection matrix, not a restatement of the formula.
        let near = 0.1;
        let proj = ss2_core::math::projection::perspective_infinite_reverse(
            60f32.to_radians(),
            16.0 / 9.0,
            near,
        );
        for z in [0.1f32, 0.5, 3.0, 25.0, 400.0] {
            let clip = proj * Vec4::new(0.0, 0.0, -z, 1.0);
            let depth = clip.z / clip.w;
            let back = linear_depth_from_reverse_z(depth, near);
            assert!((back - z).abs() / z < 1e-3, "{z} -> {depth} -> {back}");
        }
        assert_eq!(
            linear_depth_from_reverse_z(0.0, 0.1),
            f32::INFINITY,
            "depth 0 is infinitely far"
        );
        assert_eq!(linear_depth_from_reverse_z(-1.0, 0.1), f32::INFINITY);
        assert_eq!(linear_depth_from_reverse_z(f32::NAN, 0.1), f32::INFINITY);
        assert!((linear_depth_from_reverse_z(1.0, 0.1) - 0.1).abs() < 1e-7);
        assert!(
            (linear_depth_from_reverse_z(3.0, 0.1) - 0.1).abs() < 1e-7,
            "nearer than the near plane is clamped to it"
        );
    }

    #[test]
    fn a_soft_particle_fades_linearly_over_the_fade_distance() {
        // The geometry is 10 m away, the particle at 9.5: half a metre of gap, fading over a metre: half.
        assert!((soft_particle_fade(10.0, 9.5, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(
            soft_particle_fade(10.0, 10.0, 1.0),
            0.0,
            "touching: invisible"
        );
        assert_eq!(
            soft_particle_fade(10.0, 8.0, 1.0),
            1.0,
            "well in front: fully visible"
        );
        assert_eq!(
            soft_particle_fade(10.0, 9.0, 1.0),
            1.0,
            "exactly one fade distance in front"
        );
        assert_eq!(
            soft_particle_fade(10.0, 11.0, 1.0),
            0.0,
            "geometry in front of the particle"
        );
        assert_eq!(
            soft_particle_fade(f32::INFINITY, 5.0, 1.0),
            1.0,
            "sky behind it"
        );
        // Off: a fade distance that is zero, negative or NaN.
        for off in [0.0, -1.0, f32::NAN] {
            assert_eq!(soft_particle_fade(10.0, 10.0, off), 1.0);
        }
        assert_eq!(
            soft_particle_fade(f32::NAN, 5.0, 1.0),
            1.0,
            "an unknown scene depth does not hide the particle"
        );
        // Monotonic as the gap grows.
        let mut last = 0.0;
        for i in 0..=40 {
            let f = soft_particle_fade(10.0, 10.0 - i as f32 * 0.05, 1.0);
            assert!(f >= last);
            last = f;
        }
    }
}
