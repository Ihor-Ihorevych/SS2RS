//! Clipping: a stack of clip rectangles and their conversion to pixel rectangles.
//!
//! Pure CPU code, no GPU. The rules (also in `docs/UI_RENDERER.md`):
//!
//! * A [`ScissorStack`] starts **unclipped**. [`push`](ScissorStack::push) clips to the intersection of the
//!   rectangle with whatever clip is current; [`pop`](ScissorStack::pop) restores the clip from before the matching
//!   push. A rectangle without area (zero or negative size, or one that is not finite) or an intersection
//!   without area makes the clip **empty**: everything drawn under it is skipped, and it stays empty under further
//!   pushes until it is popped.
//! * Clip rectangles are in **canvas units** (the units of the quads they clip) and are intersected in those units.
//! * [`to_pixels`] converts one to a pixel rectangle of the target: the corners go through the [`Canvas`], and a
//!   pixel is inside when its **centre** is inside the rectangle, which is the rule the rasteriser uses for the
//!   quads themselves (a pixel centre exactly on a left or top edge is inside, on a right or bottom edge it is
//!   not). So `x0 = ceil(left - 0.5)` and `x1 = ceil(right - 0.5)`, clamped to the target, and a clipped quad ends
//!   on exactly the pixel where the same unclipped quad would have ended. A rectangle that holds no pixel centre
//!   is empty. Rounding each edge separately is monotone, so clipping with the intersection of two rectangles
//!   gives the same pixels as intersecting their pixel rectangles.
//! * The clip is **not** limited to the canvas: in a letterboxed window it can reach into the bars (the quads
//!   themselves can too).

use ss2_ui::{Canvas, Rect, Vec2};

/// A rectangle of whole pixels inside a target (so never negative, and never empty where it is returned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    /// Left column.
    pub x: u32,
    /// Top row.
    pub y: u32,
    /// Width in pixels (at least 1).
    pub w: u32,
    /// Height in pixels (at least 1).
    pub h: u32,
}

/// What clipping is in force.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Clip {
    /// Nothing is clipped.
    None,
    /// Clipped to this rectangle (canvas units, with area).
    Rect(Rect),
    /// Nothing can be seen: draws are skipped.
    Empty,
}

/// What a [`Clip`] means on a target of some size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipPixels {
    /// Draw without a scissor rectangle.
    Unclipped,
    /// Draw inside this pixel rectangle.
    Rect(PixelRect),
    /// No pixel is inside: skip the draw.
    Empty,
}

/// A stack of clip rectangles. See the module documentation.
#[derive(Clone, Debug)]
pub struct ScissorStack {
    saved: Vec<Clip>,
    current: Clip,
}

impl Default for ScissorStack {
    fn default() -> Self {
        Self::new()
    }
}

fn has_area(r: Rect) -> bool {
    [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite()) && r.w > 0.0 && r.h > 0.0
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.w).min(b.x + b.w);
    let bottom = (a.y + a.h).min(b.y + b.h);
    let r = Rect {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    };
    has_area(r).then_some(r)
}

impl ScissorStack {
    /// An unclipped stack.
    pub fn new() -> Self {
        Self {
            saved: Vec::new(),
            current: Clip::None,
        }
    }

    /// Clip to `rect` (canvas units) as well as to everything pushed before.
    pub fn push(&mut self, rect: Rect) {
        self.saved.push(self.current);
        self.current = match self.current {
            Clip::Empty => Clip::Empty,
            _ if !has_area(rect) => Clip::Empty,
            Clip::None => Clip::Rect(rect),
            Clip::Rect(now) => intersect(now, rect).map_or(Clip::Empty, Clip::Rect),
        };
    }

    /// Undo the last push. `false` (and nothing changes) if nothing was pushed.
    pub fn pop(&mut self) -> bool {
        match self.saved.pop() {
            Some(before) => {
                self.current = before;
                true
            }
            None => false,
        }
    }

    /// The clip in force now.
    pub fn current(&self) -> Clip {
        self.current
    }

    /// How many pushes have not been popped.
    pub fn depth(&self) -> usize {
        self.saved.len()
    }

    /// Back to unclipped, forgetting every push.
    pub fn reset(&mut self) {
        self.saved.clear();
        self.current = Clip::None;
    }

    /// The current clip on a target of `target` pixels.
    pub fn pixels(&self, canvas: &Canvas, target: (u32, u32)) -> ClipPixels {
        match self.current {
            Clip::None => ClipPixels::Unclipped,
            Clip::Empty => ClipPixels::Empty,
            Clip::Rect(r) => {
                to_pixels(r, canvas, target).map_or(ClipPixels::Empty, ClipPixels::Rect)
            }
        }
    }
}

/// The pixels of a `target`-sized image whose centres lie inside `rect` (canvas units). `None` if there are none.
pub fn to_pixels(rect: Rect, canvas: &Canvas, target: (u32, u32)) -> Option<PixelRect> {
    if !has_area(rect) {
        return None;
    }
    let tl = canvas.to_screen(Vec2 {
        x: rect.x,
        y: rect.y,
    });
    let br = canvas.to_screen(Vec2 {
        x: rect.x + rect.w,
        y: rect.y + rect.h,
    });
    let edge = |v: f32, limit: u32| -> Option<u32> {
        // `ceil(v - 0.5)`: the first pixel whose centre is not left of (or above) `v`.
        let e = (v - 0.5).ceil();
        e.is_finite().then(|| e.clamp(0.0, limit as f32) as u32)
    };
    let x0 = edge(tl.x, target.0)?;
    let y0 = edge(tl.y, target.1)?;
    let x1 = edge(br.x, target.0)?;
    let y1 = edge(br.y, target.1)?;
    (x1 > x0 && y1 > y0).then_some(PixelRect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn unit_canvas() -> Canvas {
        // 1920x1080: scale 1, no offset.
        Canvas::fit(1920, 1080).unwrap()
    }

    #[test]
    fn a_new_stack_is_unclipped() {
        let s = ScissorStack::new();
        assert_eq!(s.current(), Clip::None);
        assert_eq!(s.depth(), 0);
        assert_eq!(
            s.pixels(&unit_canvas(), (1920, 1080)),
            ClipPixels::Unclipped
        );
    }

    #[test]
    fn push_clips_and_pop_restores() {
        let mut s = ScissorStack::new();
        s.push(rect(10.0, 10.0, 100.0, 100.0));
        assert_eq!(s.current(), Clip::Rect(rect(10.0, 10.0, 100.0, 100.0)));
        assert_eq!(s.depth(), 1);
        assert!(s.pop());
        assert_eq!(s.current(), Clip::None);
        assert_eq!(s.depth(), 0);
    }

    #[test]
    fn a_push_intersects_with_the_current_clip_and_a_pop_restores_each_level() {
        let mut s = ScissorStack::new();
        s.push(rect(0.0, 0.0, 100.0, 100.0));
        s.push(rect(50.0, 20.0, 100.0, 30.0));
        assert_eq!(s.current(), Clip::Rect(rect(50.0, 20.0, 50.0, 30.0)));
        s.push(rect(60.0, 0.0, 10.0, 1000.0));
        assert_eq!(s.current(), Clip::Rect(rect(60.0, 20.0, 10.0, 30.0)));
        assert!(s.pop());
        assert_eq!(s.current(), Clip::Rect(rect(50.0, 20.0, 50.0, 30.0)));
        assert!(s.pop());
        assert_eq!(s.current(), Clip::Rect(rect(0.0, 0.0, 100.0, 100.0)));
        assert!(s.pop());
        assert_eq!(s.current(), Clip::None);
    }

    #[test]
    fn a_push_cannot_widen_the_clip() {
        let mut s = ScissorStack::new();
        s.push(rect(10.0, 10.0, 20.0, 20.0));
        s.push(rect(0.0, 0.0, 1000.0, 1000.0));
        assert_eq!(s.current(), Clip::Rect(rect(10.0, 10.0, 20.0, 20.0)));
    }

    #[test]
    fn rectangles_that_only_touch_or_do_not_meet_make_an_empty_clip() {
        let mut s = ScissorStack::new();
        s.push(rect(0.0, 0.0, 10.0, 10.0));
        s.push(rect(10.0, 0.0, 10.0, 10.0)); // shares an edge only
        assert_eq!(s.current(), Clip::Empty);
        s.pop();
        s.push(rect(50.0, 50.0, 5.0, 5.0)); // far away
        assert_eq!(s.current(), Clip::Empty);
    }

    #[test]
    fn an_empty_clip_stays_empty_under_pushes_until_popped_back_out() {
        let mut s = ScissorStack::new();
        s.push(rect(0.0, 0.0, 10.0, 10.0));
        s.push(rect(20.0, 20.0, 10.0, 10.0));
        assert_eq!(s.current(), Clip::Empty);
        s.push(rect(0.0, 0.0, 1000.0, 1000.0));
        assert_eq!(s.current(), Clip::Empty);
        s.pop();
        assert_eq!(s.current(), Clip::Empty);
        s.pop();
        assert_eq!(s.current(), Clip::Rect(rect(0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn a_rectangle_without_area_or_not_finite_is_empty() {
        for r in [
            rect(0.0, 0.0, 0.0, 10.0),
            rect(0.0, 0.0, 10.0, 0.0),
            rect(0.0, 0.0, -5.0, 10.0),
            rect(0.0, 0.0, 10.0, -5.0),
            rect(f32::NAN, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, f32::INFINITY, 10.0),
            rect(0.0, f32::NEG_INFINITY, 10.0, 10.0),
        ] {
            let mut s = ScissorStack::new();
            s.push(r);
            assert_eq!(s.current(), Clip::Empty, "{r:?}");
        }
    }

    #[test]
    fn pop_on_an_empty_stack_is_refused_and_changes_nothing() {
        let mut s = ScissorStack::new();
        assert!(!s.pop());
        assert_eq!(s.current(), Clip::None);
        s.push(rect(0.0, 0.0, 5.0, 5.0));
        assert!(s.pop());
        assert!(!s.pop());
    }

    #[test]
    fn reset_forgets_every_push() {
        let mut s = ScissorStack::new();
        s.push(rect(0.0, 0.0, 5.0, 5.0));
        s.push(rect(1.0, 1.0, 2.0, 2.0));
        s.reset();
        assert_eq!((s.current(), s.depth()), (Clip::None, 0));
    }

    #[test]
    fn whole_canvas_units_map_to_the_same_pixels_at_scale_one() {
        let c = unit_canvas();
        assert_eq!(
            to_pixels(rect(10.0, 20.0, 30.0, 40.0), &c, (1920, 1080)),
            Some(PixelRect {
                x: 10,
                y: 20,
                w: 30,
                h: 40
            })
        );
    }

    #[test]
    fn a_pixel_is_inside_when_its_centre_is() {
        let c = unit_canvas();
        let px = |r: Rect| to_pixels(r, &c, (100, 100));
        // 0.4..2.6: the centres 0.5, 1.5, 2.5 are inside (pixels 0, 1, 2).
        assert_eq!(
            px(rect(0.4, 0.4, 2.2, 2.2)),
            Some(PixelRect {
                x: 0,
                y: 0,
                w: 3,
                h: 3
            })
        );
        // 0.6..2.4: only the centres 1.5 is inside on each axis... and 0.5 is not (0.5 < 0.6), 2.5 is not (> 2.4).
        assert_eq!(
            px(rect(0.6, 0.6, 1.8, 1.8)),
            Some(PixelRect {
                x: 1,
                y: 1,
                w: 1,
                h: 1
            })
        );
        // A centre exactly on the left edge is inside, exactly on the right edge is not.
        assert_eq!(
            px(rect(0.5, 0.5, 2.0, 2.0)),
            Some(PixelRect {
                x: 0,
                y: 0,
                w: 2,
                h: 2
            })
        );
        // A sliver between two centres holds no pixel.
        assert_eq!(px(rect(0.6, 0.0, 0.8, 10.0)), None);
        assert_eq!(px(rect(0.0, 1.6, 10.0, 0.8)), None);
    }

    #[test]
    fn the_canvas_scale_and_offset_apply_before_rounding() {
        // 2560x1080: scale 1, offset.x 320.
        let wide = Canvas::fit(2560, 1080).unwrap();
        assert_eq!(
            to_pixels(rect(10.0, 5.0, 20.0, 8.0), &wide, (2560, 1080)),
            Some(PixelRect {
                x: 330,
                y: 5,
                w: 20,
                h: 8
            })
        );
        // 960x540: scale 0.5.
        let half = Canvas::fit(960, 540).unwrap();
        assert_eq!(
            to_pixels(rect(100.0, 100.0, 200.0, 80.0), &half, (960, 540)),
            Some(PixelRect {
                x: 50,
                y: 50,
                w: 100,
                h: 40
            })
        );
    }

    #[test]
    fn the_pixel_rectangle_is_clamped_to_the_target() {
        let c = unit_canvas();
        assert_eq!(
            to_pixels(rect(-50.0, -50.0, 100.0, 100.0), &c, (200, 200)),
            Some(PixelRect {
                x: 0,
                y: 0,
                w: 50,
                h: 50
            })
        );
        assert_eq!(
            to_pixels(rect(150.0, 190.0, 500.0, 500.0), &c, (200, 200)),
            Some(PixelRect {
                x: 150,
                y: 190,
                w: 50,
                h: 10
            })
        );
        // Entirely outside: empty, in every direction.
        for r in [
            rect(-100.0, 0.0, 50.0, 50.0),
            rect(0.0, -100.0, 50.0, 50.0),
            rect(250.0, 0.0, 50.0, 50.0),
            rect(0.0, 250.0, 50.0, 50.0),
        ] {
            assert_eq!(to_pixels(r, &c, (200, 200)), None, "{r:?}");
        }
    }

    #[test]
    fn huge_and_degenerate_rectangles_do_not_overflow() {
        let c = unit_canvas();
        assert_eq!(
            to_pixels(rect(-1e30, -1e30, 2e30, 2e30), &c, (64, 32)),
            Some(PixelRect {
                x: 0,
                y: 0,
                w: 64,
                h: 32
            })
        );
        assert_eq!(to_pixels(rect(0.0, 0.0, f32::NAN, 5.0), &c, (64, 32)), None);
        assert_eq!(to_pixels(rect(0.0, 0.0, 5.0, 5.0), &c, (0, 0)), None);
    }

    #[test]
    fn clipping_with_the_intersection_gives_the_intersection_of_the_pixel_rectangles() {
        let c = Canvas::fit(1280, 720).unwrap(); // scale 2/3: edges fall between pixels
        let a = rect(13.3, 7.7, 400.9, 300.1);
        let b = rect(200.2, 100.6, 500.5, 90.4);
        let pa = to_pixels(a, &c, (1280, 720)).unwrap();
        let pb = to_pixels(b, &c, (1280, 720)).unwrap();
        let mut s = ScissorStack::new();
        s.push(a);
        s.push(b);
        let both = s.pixels(&c, (1280, 720));
        let x0 = pa.x.max(pb.x);
        let y0 = pa.y.max(pb.y);
        let x1 = (pa.x + pa.w).min(pb.x + pb.w);
        let y1 = (pa.y + pa.h).min(pb.y + pb.h);
        assert_eq!(
            both,
            ClipPixels::Rect(PixelRect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0
            })
        );
    }

    #[test]
    fn the_stack_reports_empty_when_the_clip_holds_no_pixel() {
        let c = unit_canvas();
        let mut s = ScissorStack::new();
        s.push(rect(0.6, 0.6, 0.5, 0.5)); // has area, holds no centre
        assert_eq!(s.pixels(&c, (100, 100)), ClipPixels::Empty);
        let mut s = ScissorStack::new();
        s.push(rect(0.0, 0.0, 0.0, 0.0));
        assert_eq!(s.pixels(&c, (100, 100)), ClipPixels::Empty);
    }
}
