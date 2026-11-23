//! The resolution and scale policy of the UI (roadmap T10-017): how the 1920 by 1080 design
//! canvas meets a window of any size, any aspect ratio and any display scaling.
//!
//! The rule: **everything is laid out in canvas units, and a 1920 by 1080 reference area is always
//! fully visible**. [`AspectMode::Letterbox`] keeps exactly that area and fills the rest with bars (what
//! [`Canvas::fit`] does). [`AspectMode::Expand`] uses the whole window instead: on a wider or
//! taller screen the canvas grows past the reference area, so a screen background can fill the window
//! while a HUD stays in a **safe area** (a 16:9 to 21:9 region in the middle) or anchors itself to the
//! screen's edges. The canvas scale comes from the physical size of the window, so a high-DPI display
//! already gets crisp text; the operating system's own scale factor only matters for the smallest
//! text, below which the UI will not go ([`ScreenMetrics::text_px`]).

use crate::geometry::{CANVAS_H, CANVAS_W, Canvas, Rect, Vec2};

/// The smallest text the UI draws, in physical pixels at an operating-system scale of 100 %.
pub const MIN_TEXT_PX: f32 = 12.0;

/// The widest region the HUD is laid out in, as a ratio of width to height (21:9).
const SAFE_ASPECT: f32 = 21.0 / 9.0;

/// What to do with the part of the window the 16:9 reference area does not cover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AspectMode {
    /// Keep the reference area and fill the rest with bars.
    Letterbox,
    /// Grow the canvas to cover the window.
    #[default]
    Expand,
}

/// The player's choices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleSettings {
    /// What to do with the extra room.
    pub aspect: AspectMode,
    /// Make the whole UI smaller, from 0.5 to 1.0 (1.0 is the design size: the reference area
    /// fills the window's limiting side). Values outside that range are limited; a bigger UI would
    /// cut the reference area off.
    pub user_scale: f32,
}

impl Default for ScaleSettings {
    fn default() -> Self {
        Self {
            aspect: AspectMode::Expand,
            user_scale: 1.0,
        }
    }
}

/// Where the canvas lies in a window; see the module documentation.
///
/// ```
/// use ss2_ui::geometry::Vec2;
/// use ss2_ui::scale::{Anchor, ScaleSettings, ScreenMetrics, anchor};
///
/// // A 21:9 window: the canvas grows to 2580 by 1080 units and the HUD stays in the 21:9 safe area.
/// let m = ScreenMetrics::new(3440, 1440, 1.0, &ScaleSettings::default()).unwrap();
/// assert!((m.size.x - 2580.0).abs() < 0.01 && (m.size.y - 1080.0).abs() < 0.01);
/// let health = anchor(Anchor::BottomLeft, m.safe_area, Vec2 { x: 300.0, y: 120.0 }, 24.0);
/// assert!(health.x >= m.safe_area.x && health.y + health.h <= m.size.y);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenMetrics {
    /// Canvas units to window pixels (the type the renderer and the pointer code already use).
    pub canvas: Canvas,
    /// The size of the canvas in units: 1920 by 1080 when letterboxed, larger when expanded.
    pub size: Vec2,
    /// The region a HUD should stay inside: at most 21:9, centred.
    pub safe_area: Rect,
    /// The window's physical size.
    pub window_px: (u32, u32),
    /// The operating system's display scale factor (1.0 for 100 %).
    pub dpi_scale: f32,
}

impl ScreenMetrics {
    /// The metrics for a window of `width_px` by `height_px` physical pixels on a display with the
    /// operating system scale factor `dpi_scale`. `None` for a window with no area (minimised).
    pub fn new(
        width_px: u32,
        height_px: u32,
        dpi_scale: f32,
        settings: &ScaleSettings,
    ) -> Option<Self> {
        if width_px == 0 || height_px == 0 {
            return None;
        }
        let dpi_scale = if dpi_scale.is_finite() && dpi_scale > 0.0 {
            dpi_scale
        } else {
            1.0
        };
        let user = if settings.user_scale.is_finite() {
            settings.user_scale.clamp(0.5, 1.0)
        } else {
            1.0
        };
        let (w, h) = (width_px as f32, height_px as f32);
        let scale = (w / CANVAS_W).min(h / CANVAS_H) * user;
        let (canvas, size, safe_area) = match settings.aspect {
            AspectMode::Letterbox => {
                let offset = Vec2 {
                    x: (w - CANVAS_W * scale) / 2.0,
                    y: (h - CANVAS_H * scale) / 2.0,
                };
                (
                    Canvas { scale, offset },
                    Vec2 {
                        x: CANVAS_W,
                        y: CANVAS_H,
                    },
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        w: CANVAS_W,
                        h: CANVAS_H,
                    },
                )
            }
            AspectMode::Expand => {
                let size = Vec2 {
                    x: w / scale,
                    y: h / scale,
                };
                let safe_w = size.x.min(size.y * SAFE_ASPECT);
                let safe = Rect {
                    x: (size.x - safe_w) / 2.0,
                    y: 0.0,
                    w: safe_w,
                    h: size.y,
                };
                (
                    Canvas {
                        scale,
                        offset: Vec2::default(),
                    },
                    size,
                    safe,
                )
            }
        };
        Some(Self {
            canvas,
            size,
            safe_area,
            window_px: (width_px, height_px),
            dpi_scale,
        })
    }

    /// The 1920 by 1080 area that is always fully visible, centred in the canvas.
    pub fn reference_area(&self) -> Rect {
        Rect {
            x: (self.size.x - CANVAS_W) / 2.0,
            y: (self.size.y - CANVAS_H) / 2.0,
            w: CANVAS_W,
            h: CANVAS_H,
        }
    }

    /// The physical pixel size to draw text of `size_units` at: never below [`MIN_TEXT_PX`] (scaled up
    /// with the operating system's scale factor); 0 for no text or a size that is not a number.
    pub fn text_px(&self, size_units: f32) -> f32 {
        if !size_units.is_finite() || size_units <= 0.0 {
            return 0.0;
        }
        (size_units * self.canvas.scale).max(MIN_TEXT_PX * self.dpi_scale)
    }
}

/// Where in an area a box sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// Top left corner.
    TopLeft,
    /// Top edge, centred.
    TopCenter,
    /// Top right corner.
    TopRight,
    /// The middle.
    Center,
    /// Bottom left corner.
    BottomLeft,
    /// Bottom edge, centred.
    BottomCenter,
    /// Bottom right corner.
    BottomRight,
}

/// Place a box of `size` in `area` at `anchor`, `margin` units in from the edges it touches. A box
/// bigger than the area is pinned to the area's top left rather than put at negative offsets.
pub fn anchor(anchor: Anchor, area: Rect, size: Vec2, margin: f32) -> Rect {
    let (left, right, top, bottom) = (
        area.x + margin,
        area.x + area.w - size.x - margin,
        area.y + margin,
        area.y + area.h - size.y - margin,
    );
    let (center_x, center_y) = (
        area.x + (area.w - size.x) / 2.0,
        area.y + (area.h - size.y) / 2.0,
    );
    let (x, y) = match anchor {
        Anchor::TopLeft => (left, top),
        Anchor::TopCenter => (center_x, top),
        Anchor::TopRight => (right, top),
        Anchor::Center => (center_x, center_y),
        Anchor::BottomLeft => (left, bottom),
        Anchor::BottomCenter => (center_x, bottom),
        Anchor::BottomRight => (right, bottom),
    };
    Rect {
        x: x.max(area.x),
        y: y.max(area.y),
        w: size.x,
        h: size.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{CANVAS_H, CANVAS_W, Canvas, Rect, Vec2};

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-2
    }

    fn metrics(w: u32, h: u32, mode: AspectMode) -> ScreenMetrics {
        ScreenMetrics::new(
            w,
            h,
            1.0,
            &ScaleSettings {
                aspect: mode,
                ..ScaleSettings::default()
            },
        )
        .unwrap()
    }

    // ---- letterbox: the existing behaviour --------------------------------------------------

    #[test]
    fn letterbox_is_exactly_the_old_fit_with_the_reference_area_as_the_canvas() {
        for (w, h) in [
            (1920, 1080),
            (2560, 1080),
            (1280, 720),
            (1024, 768),
            (3840, 2160),
        ] {
            let m = metrics(w, h, AspectMode::Letterbox);
            assert_eq!(m.canvas, Canvas::fit(w, h).unwrap(), "{w}x{h}");
            assert_eq!((m.size.x, m.size.y), (CANVAS_W, CANVAS_H));
            assert_eq!(
                m.safe_area,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: CANVAS_W,
                    h: CANVAS_H
                }
            );
        }
    }

    // ---- expand ----------------------------------------------------------------------------

    #[test]
    fn at_16_9_both_modes_agree() {
        let m = metrics(1920, 1080, AspectMode::Expand);
        assert!(
            approx(m.canvas.scale, 1.0)
                && approx(m.canvas.offset.x, 0.0)
                && approx(m.canvas.offset.y, 0.0)
        );
        assert!(approx(m.size.x, 1920.0) && approx(m.size.y, 1080.0));
        assert_eq!(
            m.safe_area,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0
            }
        );
    }

    #[test]
    fn an_ultrawide_window_gets_a_wider_canvas_and_a_centred_safe_area() {
        // 21:9 = 3440 x 1440: the scale is 1440/1080, the canvas 2580 units wide.
        let m = metrics(3440, 1440, AspectMode::Expand);
        assert!(approx(m.canvas.scale, 1440.0 / 1080.0));
        assert!(approx(m.size.x, 3440.0 / (1440.0 / 1080.0)) && approx(m.size.y, 1080.0));
        assert!(
            approx(m.canvas.offset.x, 0.0),
            "no bars: the canvas covers the window"
        );
        assert!(
            approx(m.safe_area.w, 2520.0) && approx(m.safe_area.x, (m.size.x - 2520.0) / 2.0),
            "{:?}",
            m.safe_area
        );
        assert!(
            m.reference_area().w == CANVAS_W
                && approx(m.reference_area().x, (m.size.x - CANVAS_W) / 2.0)
        );
        // 32:9: the safe area stays at 21:9 and the screen's edges are far away.
        let wide = metrics(5120, 1440, AspectMode::Expand);
        assert!(
            approx(wide.size.x, 3840.0)
                && approx(wide.safe_area.w, 2520.0)
                && approx(wide.safe_area.x, 660.0),
            "{:?}",
            wide.safe_area
        );
    }

    #[test]
    fn a_narrow_or_square_window_grows_the_canvas_downwards_and_keeps_the_reference_area_visible() {
        let m = metrics(1024, 768, AspectMode::Expand); // 4:3
        assert!(approx(m.canvas.scale, 1024.0 / 1920.0));
        assert!(
            approx(m.size.x, 1920.0) && approx(m.size.y, 768.0 / (1024.0 / 1920.0)),
            "{:?}",
            m.size
        );
        let r = m.reference_area();
        assert!(
            r.x >= -1e-3
                && r.y >= -1e-3
                && r.x + r.w <= m.size.x + 1e-3
                && r.y + r.h <= m.size.y + 1e-3,
            "inside: {r:?} of {:?}",
            m.size
        );
        assert_eq!(
            m.safe_area,
            Rect {
                x: 0.0,
                y: 0.0,
                w: m.size.x,
                h: m.size.y
            },
            "a tall screen's whole area is safe"
        );
        let phone = metrics(800, 1280, AspectMode::Expand); // a portrait window
        assert!(phone.size.y > 1080.0 && approx(phone.size.x, 1920.0));
    }

    #[test]
    fn the_reference_area_is_always_fully_visible_whatever_the_window() {
        for (w, h) in [
            (640, 480),
            (800, 600),
            (1280, 720),
            (1366, 768),
            (1440, 900),
            (1920, 1200),
            (2560, 1080),
            (3440, 1440),
            (5120, 1440),
            (100, 2000),
            (2000, 100),
        ] {
            for mode in [AspectMode::Letterbox, AspectMode::Expand] {
                let m = metrics(w, h, mode);
                let r = m.reference_area();
                let (tl, br) = (
                    m.canvas.to_screen(Vec2 { x: r.x, y: r.y }),
                    m.canvas.to_screen(Vec2 {
                        x: r.x + r.w,
                        y: r.y + r.h,
                    }),
                );
                assert!(
                    tl.x >= -0.5
                        && tl.y >= -0.5
                        && br.x <= w as f32 + 0.5
                        && br.y <= h as f32 + 0.5,
                    "{w}x{h} {mode:?}: {tl:?} {br:?}"
                );
                assert!(m.size.x >= CANVAS_W - 1e-2 && m.size.y >= CANVAS_H - 1e-2);
            }
        }
    }

    #[test]
    fn expand_covers_the_whole_window() {
        for (w, h) in [(3440, 1440), (1024, 768), (1920, 1200)] {
            let m = metrics(w, h, AspectMode::Expand);
            let end = m.canvas.to_screen(Vec2 {
                x: m.size.x,
                y: m.size.y,
            });
            assert!(
                approx(end.x, w as f32) && approx(end.y, h as f32),
                "{w}x{h}: {end:?}"
            );
        }
    }

    // ---- the user's scale and the smallest text ---------------------------------------------

    #[test]
    fn the_user_can_make_the_ui_smaller_and_it_is_limited_to_a_sane_range() {
        let base = metrics(1920, 1080, AspectMode::Expand);
        let small = ScreenMetrics::new(
            1920,
            1080,
            1.0,
            &ScaleSettings {
                user_scale: 0.75,
                ..ScaleSettings::default()
            },
        )
        .unwrap();
        assert!(approx(small.canvas.scale, base.canvas.scale * 0.75));
        assert!(
            approx(small.size.x, 1920.0 / 0.75),
            "a smaller UI shows more canvas: {:?}",
            small.size
        );
        for silly in [0.0, -3.0, f32::NAN, f32::INFINITY, 100.0] {
            let m = ScreenMetrics::new(
                1920,
                1080,
                1.0,
                &ScaleSettings {
                    user_scale: silly,
                    ..ScaleSettings::default()
                },
            )
            .unwrap();
            assert!(
                m.canvas.scale.is_finite() && m.canvas.scale > 0.4 && m.canvas.scale <= 1.0 + 1e-3,
                "{silly}: {}",
                m.canvas.scale
            );
        }
        let letterbox_small = ScreenMetrics::new(
            1920,
            1080,
            1.0,
            &ScaleSettings {
                aspect: AspectMode::Letterbox,
                user_scale: 0.5,
                ..ScaleSettings::default()
            },
        )
        .unwrap();
        assert!(
            approx(letterbox_small.size.x, 1920.0),
            "letterbox keeps its canvas and shrinks the picture within the window"
        );
        assert!(
            approx(letterbox_small.canvas.scale, 0.5)
                && approx(letterbox_small.canvas.offset.x, 480.0),
            "{:?}",
            letterbox_small.canvas
        );
    }

    #[test]
    fn the_user_scale_never_makes_the_ui_bigger_than_the_window_allows() {
        let m = ScreenMetrics::new(
            1280,
            720,
            1.0,
            &ScaleSettings {
                user_scale: 1.0,
                ..ScaleSettings::default()
            },
        )
        .unwrap();
        let bigger_asked = ScreenMetrics::new(
            1280,
            720,
            1.0,
            &ScaleSettings {
                user_scale: 1.5,
                ..ScaleSettings::default()
            },
        )
        .unwrap();
        assert_eq!(
            m.canvas, bigger_asked.canvas,
            "above 1 would cut the reference area off, so it is limited to 1"
        );
    }

    #[test]
    fn text_never_goes_below_the_smallest_readable_size_in_physical_pixels() {
        let small_window = metrics(640, 360, AspectMode::Expand); // scale 0.333
        assert!(
            approx(small_window.text_px(18.0), MIN_TEXT_PX),
            "{}",
            small_window.text_px(18.0)
        );
        let normal = metrics(1920, 1080, AspectMode::Expand);
        assert!(approx(normal.text_px(18.0), 18.0));
        assert!(approx(normal.text_px(40.0), 40.0));
        // High DPI: the OS scale factor raises the floor so that the text stays as legible as on a 100 % screen.
        let hidpi = ScreenMetrics::new(640, 360, 2.0, &ScaleSettings::default()).unwrap();
        assert!(
            approx(hidpi.text_px(18.0), MIN_TEXT_PX * 2.0),
            "{}",
            hidpi.text_px(18.0)
        );
        assert_eq!(normal.text_px(0.0), 0.0, "no text stays no text");
        assert!(normal.text_px(f32::NAN) == 0.0);
    }

    // ---- bad input -------------------------------------------------------------------------

    #[test]
    fn a_zero_sized_or_nonsense_window_gives_none_and_a_bad_dpi_is_read_as_one() {
        assert!(ScreenMetrics::new(0, 720, 1.0, &ScaleSettings::default()).is_none());
        assert!(ScreenMetrics::new(1280, 0, 1.0, &ScaleSettings::default()).is_none());
        for dpi in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let m = ScreenMetrics::new(1280, 720, dpi, &ScaleSettings::default()).unwrap();
            assert_eq!(m.dpi_scale, 1.0, "{dpi}");
        }
        let huge = ScreenMetrics::new(u32::MAX, u32::MAX, 1.0, &ScaleSettings::default());
        assert!(huge.is_some_and(|m| m.canvas.scale.is_finite()));
    }

    // ---- anchoring --------------------------------------------------------------------------

    #[test]
    fn anchors_place_a_box_in_a_corner_or_the_middle_of_an_area_with_a_margin() {
        let area = Rect {
            x: 100.0,
            y: 50.0,
            w: 1000.0,
            h: 600.0,
        };
        let size = Vec2 { x: 200.0, y: 100.0 };
        let at = |a, m| anchor(a, area, size, m);
        assert_eq!(
            at(Anchor::TopLeft, 10.0),
            Rect {
                x: 110.0,
                y: 60.0,
                w: 200.0,
                h: 100.0
            }
        );
        assert_eq!(
            at(Anchor::TopRight, 10.0),
            Rect {
                x: 890.0,
                y: 60.0,
                w: 200.0,
                h: 100.0
            }
        );
        assert_eq!(
            at(Anchor::BottomLeft, 10.0),
            Rect {
                x: 110.0,
                y: 540.0,
                w: 200.0,
                h: 100.0
            }
        );
        assert_eq!(
            at(Anchor::BottomRight, 10.0),
            Rect {
                x: 890.0,
                y: 540.0,
                w: 200.0,
                h: 100.0
            }
        );
        assert_eq!(
            at(Anchor::Center, 10.0),
            Rect {
                x: 500.0,
                y: 300.0,
                w: 200.0,
                h: 100.0
            }
        );
        assert_eq!(
            at(Anchor::TopCenter, 0.0),
            Rect {
                x: 500.0,
                y: 50.0,
                w: 200.0,
                h: 100.0
            }
        );
        assert_eq!(
            at(Anchor::BottomCenter, 20.0),
            Rect {
                x: 500.0,
                y: 530.0,
                w: 200.0,
                h: 100.0
            }
        );
        // A box bigger than the area is pinned to its top left rather than put at negative offsets.
        let too_big = anchor(
            Anchor::Center,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
            Vec2 { x: 300.0, y: 300.0 },
            0.0,
        );
        assert_eq!((too_big.x, too_big.y), (0.0, 0.0));
    }

    #[test]
    fn a_hud_in_the_safe_area_moves_inward_on_an_ultrawide_and_a_screen_edge_anchor_stays_on_the_edge()
     {
        let m = metrics(5120, 1440, AspectMode::Expand);
        let health = Vec2 { x: 300.0, y: 120.0 };
        let in_safe = anchor(Anchor::BottomLeft, m.safe_area, health, 24.0);
        let on_edge = anchor(
            Anchor::BottomLeft,
            Rect {
                x: 0.0,
                y: 0.0,
                w: m.size.x,
                h: m.size.y,
            },
            health,
            24.0,
        );
        assert!(in_safe.x > on_edge.x + 500.0, "{in_safe:?} vs {on_edge:?}");
        assert!(approx(on_edge.x, 24.0));
        // Both are inside the canvas.
        for r in [in_safe, on_edge] {
            assert!(r.x >= 0.0 && r.x + r.w <= m.size.x && r.y + r.h <= m.size.y);
        }
    }

    #[test]
    fn the_pointer_maps_back_to_canvas_units_in_both_modes() {
        for mode in [AspectMode::Letterbox, AspectMode::Expand] {
            let m = metrics(3440, 1440, mode);
            for p in [
                Vec2 { x: 100.0, y: 100.0 },
                Vec2 { x: 960.0, y: 540.0 },
                Vec2 {
                    x: 1800.0,
                    y: 1000.0,
                },
            ] {
                let back = m.canvas.to_canvas(m.canvas.to_screen(p));
                assert!(approx(back.x, p.x) && approx(back.y, p.y), "{mode:?} {p:?}");
            }
        }
    }
}
