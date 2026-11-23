//! Debug drawing: lines, boxes, spheres, axes and text labels in world space (roadmap T04-018).
//!
//! Gameplay, physics and net code record shapes into a [`DebugDraw`] with a [`DebugCategory`]; a
//! shape of a category that is switched off in [`DebugOverlays`] is **dropped at the call**, so
//! leaving debug calls in the code costs a branch. The renderer reads the recorded
//! [`DebugLine`]s and [`DebugLabel`]s and draws them (the GPU pass is a later task, see
//! `docs/DEBUG_DRAW.md`). Shapes can live for several ticks ([`DebugDraw::advance_tick`]), and the
//! number of lines and labels is capped, so a runaway loop cannot eat the memory.

use crate::{Aabb, Sphere};
use ss2_core::math::{Mat4, Vec2, Vec3, Vec4};

/// An RGBA colour, 8 bits per channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DebugColor(pub [u8; 4]);

impl DebugColor {
    /// Opaque red.
    pub const RED: Self = Self([255, 40, 40, 255]);
    /// Opaque green.
    pub const GREEN: Self = Self([60, 255, 60, 255]);
    /// Opaque blue.
    pub const BLUE: Self = Self([70, 110, 255, 255]);
    /// Opaque yellow.
    pub const YELLOW: Self = Self([255, 230, 60, 255]);
    /// Opaque white.
    pub const WHITE: Self = Self([255, 255, 255, 255]);
}

/// The overlay a debug shape belongs to; each can be switched on and off on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugCategory {
    /// Colliders, contacts, raycasts.
    Physics,
    /// Navigation meshes and paths.
    Navigation,
    /// Bounding volumes and frusta used by culling.
    Culling,
    /// Network: server positions, prediction error, lag compensation.
    Net,
    /// Audio sources and their ranges.
    Audio,
    /// Gameplay: triggers, spawn points, AI state.
    Gameplay,
    /// The renderer's own helpers (light volumes, probes).
    Render,
    /// Anything else.
    Misc,
}

impl DebugCategory {
    /// Every category, in a fixed order.
    pub const ALL: [Self; 8] = [
        Self::Physics,
        Self::Navigation,
        Self::Culling,
        Self::Net,
        Self::Audio,
        Self::Gameplay,
        Self::Render,
        Self::Misc,
    ];

    /// The name used on the console (`r_debug physics`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Physics => "physics",
            Self::Navigation => "navigation",
            Self::Culling => "culling",
            Self::Net => "net",
            Self::Audio => "audio",
            Self::Gameplay => "gameplay",
            Self::Render => "render",
            Self::Misc => "misc",
        }
    }

    /// The category called `name` (ignoring case and surrounding spaces).
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        Self::ALL
            .into_iter()
            .find(|c| c.name().eq_ignore_ascii_case(name))
    }

    fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

/// Which categories are drawn. Everything is off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DebugOverlays {
    bits: u32,
}

impl DebugOverlays {
    /// Whether `category` is drawn.
    pub fn is_enabled(&self, category: DebugCategory) -> bool {
        self.bits & category.bit() != 0
    }

    /// Switch `category` on or off.
    pub fn set(&mut self, category: DebugCategory, on: bool) {
        if on {
            self.bits |= category.bit();
        } else {
            self.bits &= !category.bit();
        }
    }

    /// Flip `category`; returns the new state.
    pub fn toggle(&mut self, category: DebugCategory) -> bool {
        let now = !self.is_enabled(category);
        self.set(category, now);
        now
    }

    /// Switch every category on.
    pub fn enable_all(&mut self) {
        for c in DebugCategory::ALL {
            self.set(c, true);
        }
    }

    /// Switch every category off.
    pub fn disable_all(&mut self) {
        self.bits = 0;
    }

    /// The categories that are on.
    pub fn enabled(&self) -> impl Iterator<Item = DebugCategory> + '_ {
        DebugCategory::ALL
            .into_iter()
            .filter(|c| self.is_enabled(*c))
    }
}

/// One line segment in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugLine {
    /// Start.
    pub from: Vec3,
    /// End.
    pub to: Vec3,
    /// Colour.
    pub color: DebugColor,
}

/// A piece of text anchored at a point in world space.
#[derive(Clone, Debug, PartialEq)]
pub struct DebugLabel {
    /// The anchor.
    pub position: Vec3,
    /// The text, at most [`DebugDraw::MAX_LABEL_CHARS`] characters.
    pub text: String,
    /// Colour.
    pub color: DebugColor,
}

/// A label placed on the screen, ready for the 2D/UI renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenLabel {
    /// Pixels from the top left of the viewport.
    pub position: Vec2,
    /// The text.
    pub text: String,
    /// Colour.
    pub color: DebugColor,
}

/// The recorder for debug shapes; see the module documentation.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_render_api::{DebugCategory, DebugColor, DebugDraw};
///
/// let mut debug = DebugDraw::new(10_000, 256);
/// debug.line(DebugCategory::Physics, Vec3::ZERO, Vec3::X, DebugColor::RED);
/// assert_eq!(debug.line_count(), 0, "the physics overlay is off: nothing is recorded");
///
/// debug.overlays_mut().set(DebugCategory::Physics, true); // `r_debug physics`
/// debug.line(DebugCategory::Physics, Vec3::ZERO, Vec3::X, DebugColor::RED);
/// assert_eq!(debug.line_count(), 1);
/// debug.advance_tick(); // end of the tick: one-tick shapes are gone
/// assert_eq!(debug.line_count(), 0);
/// ```
#[derive(Clone, Debug)]
pub struct DebugDraw {
    overlays: DebugOverlays,
    max_lines: usize,
    max_labels: usize,
    lines: Vec<DebugLine>,
    line_ticks: Vec<u32>,
    labels: Vec<DebugLabel>,
    label_ticks: Vec<u32>,
    dropped_lines: u64,
    dropped_labels: u64,
}

impl DebugDraw {
    /// Segments in a circle (a sphere is three circles).
    pub const CIRCLE_SEGMENTS: u32 = 24;
    /// The longest label text kept, in characters.
    pub const MAX_LABEL_CHARS: usize = 64;

    /// A recorder that keeps at most `max_lines` lines and `max_labels` labels at a time.
    pub fn new(max_lines: usize, max_labels: usize) -> Self {
        Self {
            overlays: DebugOverlays::default(),
            max_lines,
            max_labels,
            lines: Vec::new(),
            line_ticks: Vec::new(),
            labels: Vec::new(),
            label_ticks: Vec::new(),
            dropped_lines: 0,
            dropped_labels: 0,
        }
    }

    /// Which categories are recorded.
    pub fn overlays(&self) -> &DebugOverlays {
        &self.overlays
    }

    /// Change which categories are recorded.
    pub fn overlays_mut(&mut self) -> &mut DebugOverlays {
        &mut self.overlays
    }

    /// The recorded lines.
    pub fn lines(&self) -> &[DebugLine] {
        &self.lines
    }

    /// The recorded labels.
    pub fn labels(&self) -> &[DebugLabel] {
        &self.labels
    }

    /// How many lines are recorded.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// How many labels are recorded.
    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    /// Lines refused since the last tick because the cap was reached.
    pub fn dropped_lines(&self) -> u64 {
        self.dropped_lines
    }

    /// Labels refused since the last tick because the cap was reached.
    pub fn dropped_labels(&self) -> u64 {
        self.dropped_labels
    }

    /// Remove everything.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.line_ticks.clear();
        self.labels.clear();
        self.label_ticks.clear();
    }

    /// End a tick: every shape has one tick less to live and those at zero go; the counters of
    /// dropped shapes restart.
    pub fn advance_tick(&mut self) {
        fn age<T>(items: &mut Vec<T>, ticks: &mut Vec<u32>) {
            let mut keep = ticks.iter().map(|t| *t > 1);
            items.retain(|_| keep.next().unwrap_or(false));
            ticks.retain(|t| *t > 1);
            for t in ticks.iter_mut() {
                *t -= 1;
            }
        }
        age(&mut self.lines, &mut self.line_ticks);
        age(&mut self.labels, &mut self.label_ticks);
        self.dropped_lines = 0;
        self.dropped_labels = 0;
    }

    /// Room for `count` more lines? Counts them as dropped if not (a shape is never kept in part).
    fn reserve_lines(&mut self, count: usize) -> bool {
        if self.lines.len() + count > self.max_lines {
            self.dropped_lines += count as u64;
            false
        } else {
            true
        }
    }

    fn push_line(&mut self, from: Vec3, to: Vec3, color: DebugColor, ticks: u32) {
        self.lines.push(DebugLine { from, to, color });
        self.line_ticks.push(ticks.max(1));
    }

    /// A line for this tick only.
    pub fn line(&mut self, category: DebugCategory, from: Vec3, to: Vec3, color: DebugColor) {
        self.line_for(category, from, to, color, 1);
    }

    /// A line that stays for `ticks` ticks (0 is read as 1).
    pub fn line_for(
        &mut self,
        category: DebugCategory,
        from: Vec3,
        to: Vec3,
        color: DebugColor,
        ticks: u32,
    ) {
        if !self.overlays.is_enabled(category) || !from.is_finite() || !to.is_finite() {
            return;
        }
        if self.reserve_lines(1) {
            self.push_line(from, to, color, ticks);
        }
    }

    fn edges(&mut self, category: DebugCategory, corners: [Vec3; 8], color: DebugColor) {
        if !self.overlays.is_enabled(category) || corners.iter().any(|c| !c.is_finite()) {
            return;
        }
        if !self.reserve_lines(12) {
            return;
        }
        for i in 0..8_usize {
            for bit in [1_usize, 2, 4] {
                if i & bit == 0 {
                    self.push_line(corners[i], corners[i | bit], color, 1);
                }
            }
        }
    }

    /// The twelve edges of an axis-aligned box.
    pub fn aabb(&mut self, category: DebugCategory, b: Aabb, color: DebugColor) {
        self.edges(category, b.corners(), color);
    }

    /// The twelve edges of `local` after it is transformed by `m` (an oriented box).
    pub fn obb(&mut self, category: DebugCategory, local: Aabb, m: Mat4, color: DebugColor) {
        let corners = local.corners().map(|c| m.transform_point3(c));
        self.edges(category, corners, color);
    }

    /// A circle of `radius` around `center`, in the plane with the given `normal`.
    pub fn circle(
        &mut self,
        category: DebugCategory,
        center: Vec3,
        normal: Vec3,
        radius: f32,
        color: DebugColor,
    ) {
        if !self.overlays.is_enabled(category)
            || !center.is_finite()
            || !normal.is_finite()
            || !radius.is_finite()
            || radius < 0.0
        {
            return;
        }
        let normal = normal.normalize_or_zero();
        if normal == Vec3::ZERO {
            return;
        }
        let n = Self::CIRCLE_SEGMENTS as usize;
        if !self.reserve_lines(n) {
            return;
        }
        let (u, v) = normal.any_orthonormal_pair();
        let point = |k: usize| {
            let angle = std::f32::consts::TAU * (k % n) as f32 / n as f32;
            center + (u * angle.cos() + v * angle.sin()) * radius
        };
        for k in 0..n {
            self.push_line(point(k), point(k + 1), color, 1);
        }
    }

    /// A sphere, drawn as three circles.
    pub fn sphere(&mut self, category: DebugCategory, s: Sphere, color: DebugColor) {
        if !self.overlays.is_enabled(category)
            || !s.center.is_finite()
            || !s.radius.is_finite()
            || s.radius < 0.0
        {
            return;
        }
        // All three or none: check the room once so a full buffer does not keep one circle.
        let need = 3 * Self::CIRCLE_SEGMENTS as usize;
        if self.lines.len() + need > self.max_lines {
            self.dropped_lines += need as u64;
            return;
        }
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            self.circle(category, s.center, axis, s.radius, color);
        }
    }

    /// The three axes of `m` from its origin, `length` long: x red, y green, z blue.
    pub fn axes(&mut self, category: DebugCategory, m: Mat4, length: f32) {
        if !self.overlays.is_enabled(category) || !m.is_finite() || !length.is_finite() {
            return;
        }
        let origin = m.transform_point3(Vec3::ZERO);
        for (axis, color) in [
            (m.x_axis, DebugColor::RED),
            (m.y_axis, DebugColor::GREEN),
            (m.z_axis, DebugColor::BLUE),
        ] {
            let direction = axis.truncate().normalize_or_zero();
            self.line(category, origin, origin + direction * length, color);
        }
    }

    /// An arrow from `from` to `to`, with four strokes for the head.
    pub fn arrow(&mut self, category: DebugCategory, from: Vec3, to: Vec3, color: DebugColor) {
        if !self.overlays.is_enabled(category) || !from.is_finite() || !to.is_finite() {
            return;
        }
        let shaft = to - from;
        let length = shaft.length();
        if length < 1e-6 {
            self.line(category, from, to, color);
            return;
        }
        if !self.reserve_lines(5) {
            return;
        }
        self.push_line(from, to, color, 1);
        let direction = shaft / length;
        let head = length * 0.2;
        let (u, v) = direction.any_orthonormal_pair();
        for side in [u, -u, v, -v] {
            self.push_line(to, to - direction * head + side * head * 0.5, color, 1);
        }
    }

    /// A text label at `position`, for this tick only.
    pub fn label(
        &mut self,
        category: DebugCategory,
        position: Vec3,
        text: &str,
        color: DebugColor,
    ) {
        if !self.overlays.is_enabled(category) || !position.is_finite() {
            return;
        }
        if self.labels.len() >= self.max_labels {
            self.dropped_labels += 1;
            return;
        }
        let text: String = text.chars().take(Self::MAX_LABEL_CHARS).collect();
        self.labels.push(DebugLabel {
            position,
            text,
            color,
        });
        self.label_ticks.push(1);
    }

    /// Place the labels on a viewport of `viewport` pixels, given the camera's world-to-clip
    /// matrix. Labels behind the camera or outside the view are left out.
    pub fn project_labels(&self, view_projection: Mat4, viewport: Vec2) -> Vec<ScreenLabel> {
        self.labels
            .iter()
            .filter_map(|label| {
                let clip = view_projection
                    * Vec4::new(label.position.x, label.position.y, label.position.z, 1.0);
                if clip.w <= 0.0 {
                    return None;
                }
                let ndc = Vec3::new(clip.x, clip.y, clip.z) / clip.w;
                if ndc.x.abs() > 1.0 || ndc.y.abs() > 1.0 || !(0.0..=1.0).contains(&ndc.z) {
                    return None;
                }
                Some(ScreenLabel {
                    position: Vec2::new(
                        (ndc.x + 1.0) * 0.5 * viewport.x,
                        (1.0 - ndc.y) * 0.5 * viewport.y,
                    ),
                    text: label.text.clone(),
                    color: label.color,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Aabb, Sphere};
    use ss2_core::math::{Mat4, Quat, Vec2, Vec3};

    fn all_on() -> DebugDraw {
        let mut d = DebugDraw::new(1000, 100);
        d.overlays_mut().enable_all();
        d
    }

    #[test]
    fn a_category_that_is_off_records_nothing() {
        let mut d = DebugDraw::new(100, 10);
        d.line(DebugCategory::Physics, Vec3::ZERO, Vec3::X, DebugColor::RED);
        d.label(DebugCategory::Physics, Vec3::ZERO, "x", DebugColor::WHITE);
        assert_eq!((d.line_count(), d.label_count()), (0, 0));
        d.overlays_mut().set(DebugCategory::Physics, true);
        d.line(DebugCategory::Physics, Vec3::ZERO, Vec3::X, DebugColor::RED);
        d.line(DebugCategory::Net, Vec3::ZERO, Vec3::Y, DebugColor::RED);
        assert_eq!(d.line_count(), 1, "only the enabled category");
    }

    #[test]
    fn overlays_toggle_and_parse_from_console_names() {
        let mut o = DebugOverlays::default();
        assert!(!o.is_enabled(DebugCategory::Net));
        assert!(o.toggle(DebugCategory::Net), "toggle returns the new state");
        assert!(o.is_enabled(DebugCategory::Net));
        assert!(!o.toggle(DebugCategory::Net));
        assert_eq!(
            DebugCategory::from_name("physics"),
            Some(DebugCategory::Physics)
        );
        assert_eq!(DebugCategory::from_name("  NET "), Some(DebugCategory::Net));
        assert_eq!(DebugCategory::from_name("nope"), None);
        for c in DebugCategory::ALL {
            assert_eq!(
                DebugCategory::from_name(c.name()),
                Some(c),
                "names round-trip"
            );
        }
        o.enable_all();
        assert!(DebugCategory::ALL.iter().all(|c| o.is_enabled(*c)));
        o.disable_all();
        assert!(DebugCategory::ALL.iter().all(|c| !o.is_enabled(*c)));
        assert_eq!(o.enabled().count(), 0);
    }

    #[test]
    fn a_box_is_twelve_edges_on_its_corners() {
        let mut d = all_on();
        let b = Aabb::new(Vec3::new(-1.0, 0.0, -2.0), Vec3::new(1.0, 2.0, 2.0));
        d.aabb(DebugCategory::Misc, b, DebugColor::GREEN);
        assert_eq!(d.line_count(), 12);
        let corners = b.corners();
        for line in d.lines() {
            assert!(
                corners.contains(&line.from) && corners.contains(&line.to),
                "{line:?}"
            );
            let differing = (line.from.x != line.to.x) as u8
                + (line.from.y != line.to.y) as u8
                + (line.from.z != line.to.z) as u8;
            assert_eq!(differing, 1, "an edge runs along one axis");
            assert_eq!(line.color, DebugColor::GREEN);
        }
    }

    #[test]
    fn an_oriented_box_follows_its_transform() {
        let mut d = all_on();
        let local = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let m = Mat4::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(10.0, 0.0, 0.0),
        );
        d.obb(DebugCategory::Misc, local, m, DebugColor::WHITE);
        assert_eq!(d.line_count(), 12);
        for line in d.lines() {
            for p in [line.from, line.to] {
                assert!(
                    (p.x - 10.0).abs() <= 1.0 + 1e-4
                        && p.y.abs() <= 1.0 + 1e-4
                        && p.z.abs() <= 1.0 + 1e-4,
                    "{p:?}"
                );
            }
        }
    }

    #[test]
    fn a_sphere_is_three_circles_on_its_surface() {
        let mut d = all_on();
        let s = Sphere::new(Vec3::new(1.0, 2.0, 3.0), 4.0);
        d.sphere(DebugCategory::Physics, s, DebugColor::BLUE);
        assert_eq!(d.line_count(), 3 * DebugDraw::CIRCLE_SEGMENTS as usize);
        for line in d.lines() {
            for p in [line.from, line.to] {
                assert!(
                    (p.distance(s.center) - 4.0).abs() < 1e-3,
                    "{p:?} is not on the sphere"
                );
            }
        }
    }

    #[test]
    fn a_circle_is_closed_and_flat() {
        let mut d = all_on();
        d.circle(
            DebugCategory::Misc,
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::Y,
            2.0,
            DebugColor::RED,
        );
        let lines: Vec<_> = d.lines().to_vec();
        assert_eq!(lines.len(), DebugDraw::CIRCLE_SEGMENTS as usize);
        for (i, l) in lines.iter().enumerate() {
            assert!((l.from.y - 5.0).abs() < 1e-4 && (l.to.y - 5.0).abs() < 1e-4);
            assert!((Vec3::new(l.from.x, 0.0, l.from.z).length() - 2.0).abs() < 1e-3);
            assert!(
                (l.to - lines[(i + 1) % lines.len()].from).length() < 1e-4,
                "segment {i} meets the next"
            );
        }
    }

    #[test]
    fn axes_are_three_coloured_lines_of_the_given_length() {
        let mut d = all_on();
        d.axes(
            DebugCategory::Misc,
            Mat4::from_translation(Vec3::new(1.0, 1.0, 1.0)),
            2.0,
        );
        assert_eq!(d.line_count(), 3);
        let l = d.lines();
        assert_eq!((l[0].to - l[0].from), Vec3::new(2.0, 0.0, 0.0));
        assert_eq!((l[1].to - l[1].from), Vec3::new(0.0, 2.0, 0.0));
        assert_eq!((l[2].to - l[2].from), Vec3::new(0.0, 0.0, 2.0));
        assert_eq!(
            (l[0].color, l[1].color, l[2].color),
            (DebugColor::RED, DebugColor::GREEN, DebugColor::BLUE)
        );
    }

    #[test]
    fn an_arrow_has_a_shaft_and_a_head_at_its_tip() {
        let mut d = all_on();
        d.arrow(
            DebugCategory::Misc,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -4.0),
            DebugColor::WHITE,
        );
        assert!(d.line_count() >= 3, "shaft plus at least two head strokes");
        assert!(
            d.lines()
                .iter()
                .skip(1)
                .all(|l| l.from == Vec3::new(0.0, 0.0, -4.0)),
            "the head strokes start at the tip"
        );
        // A zero-length arrow is only a point: it must not produce NaN.
        let mut z = all_on();
        z.arrow(DebugCategory::Misc, Vec3::X, Vec3::X, DebugColor::WHITE);
        assert!(
            z.lines()
                .iter()
                .all(|l| l.from.is_finite() && l.to.is_finite())
        );
    }

    #[test]
    fn non_finite_input_is_dropped_not_recorded() {
        let mut d = all_on();
        d.line(
            DebugCategory::Misc,
            Vec3::new(f32::NAN, 0.0, 0.0),
            Vec3::ONE,
            DebugColor::RED,
        );
        d.sphere(
            DebugCategory::Misc,
            Sphere::new(Vec3::ZERO, f32::INFINITY),
            DebugColor::RED,
        );
        d.sphere(
            DebugCategory::Misc,
            Sphere::new(Vec3::ZERO, -1.0),
            DebugColor::RED,
        );
        d.label(
            DebugCategory::Misc,
            Vec3::new(0.0, f32::NAN, 0.0),
            "x",
            DebugColor::RED,
        );
        assert_eq!((d.line_count(), d.label_count()), (0, 0));
    }

    #[test]
    fn shapes_live_for_the_number_of_ticks_asked() {
        let mut d = all_on();
        d.line_for(DebugCategory::Misc, Vec3::ZERO, Vec3::X, DebugColor::RED, 3);
        d.line(DebugCategory::Misc, Vec3::ZERO, Vec3::Y, DebugColor::RED); // one frame only
        assert_eq!(d.line_count(), 2);
        d.advance_tick();
        assert_eq!(
            d.line_count(),
            1,
            "the one-frame line is gone after the first tick"
        );
        d.advance_tick();
        assert_eq!(d.line_count(), 1);
        d.advance_tick();
        assert_eq!(
            d.line_count(),
            0,
            "the 3-tick line ends after its third tick"
        );
    }

    #[test]
    fn lifetimes_stay_attached_to_their_own_shapes_across_ticks() {
        let mut d = all_on();
        d.line_for(DebugCategory::Misc, Vec3::ZERO, Vec3::X, DebugColor::RED, 1); // a
        d.line_for(
            DebugCategory::Misc,
            Vec3::ZERO,
            Vec3::Y,
            DebugColor::GREEN,
            3,
        ); // b
        d.advance_tick(); // a goes
        d.line_for(
            DebugCategory::Misc,
            Vec3::ZERO,
            Vec3::Z,
            DebugColor::BLUE,
            1,
        ); // c
        d.advance_tick(); // c goes, b has one tick left
        assert_eq!(d.line_count(), 1);
        assert_eq!(d.lines()[0].color, DebugColor::GREEN, "b is still there");
        d.advance_tick();
        assert_eq!(d.line_count(), 0);
        // The same for labels.
        d.label(DebugCategory::Misc, Vec3::ZERO, "a", DebugColor::RED);
        d.advance_tick();
        d.label(DebugCategory::Misc, Vec3::ZERO, "b", DebugColor::RED);
        assert_eq!(d.label_count(), 1);
        assert_eq!(d.labels()[0].text, "b");
    }

    #[test]
    fn the_caps_bound_memory_and_count_what_was_dropped() {
        let mut d = DebugDraw::new(10, 2);
        d.overlays_mut().enable_all();
        for i in 0..25 {
            d.line(
                DebugCategory::Misc,
                Vec3::ZERO,
                Vec3::splat(i as f32),
                DebugColor::RED,
            );
        }
        assert_eq!(d.line_count(), 10);
        assert_eq!(d.dropped_lines(), 15);
        for _ in 0..5 {
            d.label(DebugCategory::Misc, Vec3::ZERO, "x", DebugColor::RED);
        }
        assert_eq!((d.label_count(), d.dropped_labels()), (2, 3));
        // A shape that does not fit entirely is not recorded partly: a sphere needs 3 circles.
        let mut e = DebugDraw::new(10, 1);
        e.overlays_mut().enable_all();
        e.sphere(
            DebugCategory::Misc,
            Sphere::new(Vec3::ZERO, 1.0),
            DebugColor::RED,
        );
        assert_eq!(e.line_count(), 0);
        assert_eq!(e.dropped_lines(), 3 * DebugDraw::CIRCLE_SEGMENTS as u64);
        d.advance_tick();
        assert_eq!(
            (d.line_count(), d.dropped_lines()),
            (0, 0),
            "the drop counters reset each tick"
        );
    }

    #[test]
    fn long_labels_are_cut_at_a_character_boundary() {
        let mut d = all_on();
        let text = "é".repeat(200);
        d.label(DebugCategory::Misc, Vec3::ZERO, &text, DebugColor::WHITE);
        let kept = &d.labels()[0].text;
        assert!(kept.chars().count() <= DebugDraw::MAX_LABEL_CHARS);
        assert!(kept.chars().all(|c| c == 'é'), "never a broken character");
    }

    #[test]
    fn labels_project_to_screen_pixels_and_skip_what_is_behind_or_outside() {
        let camera = crate::Camera::default();
        let vp = camera.view_projection(1.0);
        let mut d = all_on();
        d.label(
            DebugCategory::Misc,
            Vec3::new(0.0, 0.0, -5.0),
            "centre",
            DebugColor::WHITE,
        );
        d.label(
            DebugCategory::Misc,
            Vec3::new(0.0, 0.0, 5.0),
            "behind",
            DebugColor::WHITE,
        );
        d.label(
            DebugCategory::Misc,
            Vec3::new(500.0, 0.0, -5.0),
            "far right",
            DebugColor::WHITE,
        );
        let screen = d.project_labels(vp, Vec2::new(800.0, 600.0));
        assert_eq!(screen.len(), 1, "{screen:?}");
        assert_eq!(screen[0].text, "centre");
        assert!(
            (screen[0].position - Vec2::new(400.0, 300.0)).length() < 1e-2,
            "the screen centre: {:?}",
            screen[0].position
        );
        // y grows downwards on the screen: a point above the centre has a smaller y.
        let mut up = all_on();
        up.label(
            DebugCategory::Misc,
            Vec3::new(0.0, 1.0, -5.0),
            "up",
            DebugColor::WHITE,
        );
        assert!(up.project_labels(vp, Vec2::new(800.0, 600.0))[0].position.y < 300.0);
    }

    #[test]
    fn clearing_removes_everything() {
        let mut d = all_on();
        d.line_for(
            DebugCategory::Misc,
            Vec3::ZERO,
            Vec3::X,
            DebugColor::RED,
            100,
        );
        d.label(DebugCategory::Misc, Vec3::ZERO, "x", DebugColor::RED);
        d.clear();
        assert_eq!((d.line_count(), d.label_count()), (0, 0));
    }
}
