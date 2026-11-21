//! Plain 2D geometry and the virtual canvas the UI is laid out in.

/// Width of the virtual canvas in UI units.
pub const CANVAS_W: f32 = 1920.0;
/// Height of the virtual canvas in UI units.
pub const CANVAS_H: f32 = 1080.0;

/// A 2D point or vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

/// An axis-aligned rectangle (top-left origin, y down).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Half-open containment: left/top edges are inside, right/bottom edges are not.
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }
}

/// Maps the 1920x1080 virtual canvas onto a window of any size (uniform scale, letterboxed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Canvas {
    /// Pixels per canvas unit.
    pub scale: f32,
    /// Pixel position of the canvas origin.
    pub offset: Vec2,
}

impl Canvas {
    /// Fit the canvas into a `width_px` x `height_px` window. `None` if either side is zero
    /// (for example a minimized window).
    pub fn fit(width_px: u32, height_px: u32) -> Option<Canvas> {
        if width_px == 0 || height_px == 0 {
            return None;
        }
        let (w, h) = (width_px as f32, height_px as f32);
        let scale = (w / CANVAS_W).min(h / CANVAS_H);
        let offset = Vec2 {
            x: (w - CANVAS_W * scale) / 2.0,
            y: (h - CANVAS_H * scale) / 2.0,
        };
        Some(Canvas { scale, offset })
    }

    /// Canvas units to window pixels.
    pub fn to_screen(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: p.x * self.scale + self.offset.x,
            y: p.y * self.scale + self.offset.y,
        }
    }

    /// Window pixels to canvas units.
    pub fn to_canvas(&self, px: Vec2) -> Vec2 {
        Vec2 {
            x: (px.x - self.offset.x) / self.scale,
            y: (px.y - self.offset.y) / self.scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn canvas_fit_exact_16_9() {
        let c = Canvas::fit(1920, 1080).unwrap();
        assert!(approx(c.scale, 1.0));
        assert!(approx(c.offset.x, 0.0) && approx(c.offset.y, 0.0));
    }

    #[test]
    fn canvas_fit_ultrawide_centers() {
        let c = Canvas::fit(2560, 1080).unwrap();
        assert!(approx(c.scale, 1.0));
        assert!(approx(c.offset.x, 320.0) && approx(c.offset.y, 0.0));
    }

    #[test]
    fn canvas_fit_small() {
        let c = Canvas::fit(1280, 720).unwrap();
        assert!(approx(c.scale, 1280.0 / 1920.0));
        assert!(approx(c.offset.x, 0.0) && approx(c.offset.y, 0.0));
    }

    #[test]
    fn canvas_zero_size_is_none() {
        assert!(Canvas::fit(0, 720).is_none());
        assert!(Canvas::fit(1280, 0).is_none());
        assert!(Canvas::fit(0, 0).is_none());
    }

    #[test]
    fn canvas_roundtrip() {
        let c = Canvas::fit(2560, 1080).unwrap();
        for p in [
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 960.0, y: 540.0 },
            Vec2 {
                x: 1919.0,
                y: 1079.0,
            },
        ] {
            let back = c.to_canvas(c.to_screen(p));
            assert!(
                approx(back.x, p.x) && approx(back.y, p.y),
                "{p:?} -> {back:?}"
            );
        }
    }

    #[test]
    fn rect_contains_is_half_open() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
        };
        assert!(r.contains(Vec2 { x: 10.0, y: 20.0 }));
        assert!(r.contains(Vec2 { x: 109.9, y: 69.9 }));
        assert!(!r.contains(Vec2 { x: 110.0, y: 30.0 }));
        assert!(!r.contains(Vec2 { x: 50.0, y: 70.0 }));
        assert!(!r.contains(Vec2 { x: 9.9, y: 30.0 }));
    }
}
