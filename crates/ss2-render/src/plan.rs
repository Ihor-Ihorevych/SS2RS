//! Turning a list of commands into batches: the CPU half of drawing a frame, testable without a GPU.
//!
//! A [`Plan`] holds the vertices of every quad, image and 9-slice cell and the **batches** that draw them. Rules:
//!
//! * **Order is kept.** Batches are drawn in the order of the commands that made them; nothing is moved across
//!   anything else.
//! * **Merging.** A command joins the previous batch when that batch is of the same kind and has the same image,
//!   sampling and scissor rectangle. Only *consecutive* commands merge, so an image between two flat quads splits
//!   them. A flat quad has no image (a white texel is bound) and its sampling does not matter, so flat quads share
//!   one. A different tint or texture rectangle is vertex data and never splits a batch.
//! * **Text** is batched like geometry: consecutive text runs with the same scissor are one batch (one GPU draw
//!   call); text and geometry never merge, because they are drawn by different pipelines.
//! * **Skipped commands** (under an empty clip, or naming an image that is not known) draw nothing and do not
//!   split the batches around them: skipping a command cannot change the order of the others.
//! * An unbalanced [`PopClip`](crate::UiCommand::PopClip) is ignored and counted; clips still open at the end are
//!   closed by the end of the frame.

use std::ops::Range;

use ss2_ui::{Canvas, TextRun};

use crate::batch::{Vertex, push_quad};
use crate::nine_slice::{UvRect, nine_slice};
use crate::scissor::{ClipPixels, PixelRect, ScissorStack};
use crate::ui_frame::{CommandRef, Filter, ImageHandle, UiScene};

/// What one batch draws.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BatchKind {
    /// Triangles: `vertices` of the plan's vertex list, with `image` (or the white texel) and `filter` bound.
    Geometry {
        image: Option<ImageHandle>,
        filter: Filter,
        vertices: Range<u32>,
    },
    /// The text runs `runs` of the plan's list.
    Text { runs: Range<usize> },
}

/// One draw call.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Batch {
    /// The scissor rectangle, or `None` to draw unclipped.
    pub scissor: Option<PixelRect>,
    pub kind: BatchKind,
}

/// The result of planning a frame.
pub(crate) struct Plan<'a> {
    pub vertices: Vec<Vertex>,
    pub batches: Vec<Batch>,
    pub texts: Vec<&'a TextRun>,
    /// Images the commands name that the registry does not know (those commands were skipped), without repeats.
    pub missing: Vec<ImageHandle>,
    /// `PopClip` commands that had no push.
    pub unbalanced_pops: u32,
}

struct Builder<'a, 's> {
    plan: Plan<'a>,
    canvas: &'s Canvas,
    target: (u32, u32),
    size_of: &'s dyn Fn(ImageHandle) -> Option<(u32, u32)>,
    clip: ScissorStack,
    clip_px: ClipPixels,
}

impl<'a> Plan<'a> {
    /// Plan `scene` for a `target`-sized image. `size_of` gives an image's size in texels (`None`: unknown).
    pub(crate) fn build(
        scene: &'a dyn UiScene,
        canvas: &Canvas,
        target: (u32, u32),
        size_of: &dyn Fn(ImageHandle) -> Option<(u32, u32)>,
    ) -> Plan<'a> {
        let mut b = Builder {
            plan: Plan {
                vertices: Vec::new(),
                batches: Vec::new(),
                texts: Vec::new(),
                missing: Vec::new(),
                unbalanced_pops: 0,
            },
            canvas,
            target,
            size_of,
            clip: ScissorStack::new(),
            clip_px: ClipPixels::Unclipped,
        };
        scene.visit(&mut |c| b.command(c));
        b.plan
    }
}

impl<'a> Builder<'a, '_> {
    fn command(&mut self, command: CommandRef<'a>) {
        match command {
            CommandRef::PushClip(rect) => {
                self.clip.push(rect);
                self.clip_px = self.clip.pixels(self.canvas, self.target);
            }
            CommandRef::PopClip => {
                if self.clip.pop() {
                    self.clip_px = self.clip.pixels(self.canvas, self.target);
                } else {
                    self.plan.unbalanced_pops += 1;
                }
            }
            CommandRef::Quad(q) => {
                self.geometry(None, Filter::Nearest, |v, canvas| {
                    push_quad(v, canvas, q.rect, UvRect::FULL, q.color.0);
                });
            }
            CommandRef::Image(i) => {
                if self.known(i.image).is_some() {
                    self.geometry(Some(i.image), i.filter, |v, canvas| {
                        push_quad(v, canvas, i.rect, i.uv, i.tint.0);
                    });
                }
            }
            CommandRef::NineSlice(n) => {
                if let Some(size) = self.known(n.image) {
                    let cells = nine_slice(n.rect, n.insets, size, n.scale, n.fill);
                    // A slice that draws nothing adds no vertices, and so no batch.
                    self.geometry(Some(n.image), n.filter, |v, canvas| {
                        for cell in &cells {
                            push_quad(v, canvas, cell.dest, cell.uv, n.tint.0);
                        }
                    });
                }
            }
            CommandRef::Text(t) => self.text(t),
        }
    }

    /// The size of `image`, or `None` (and a note) if it is not known.
    fn known(&mut self, image: ImageHandle) -> Option<(u32, u32)> {
        let size = (self.size_of)(image);
        if size.is_none() && !self.plan.missing.contains(&image) {
            self.plan.missing.push(image);
        }
        size
    }

    /// The scissor to draw with now: `None` if the clip leaves nothing to draw.
    fn scissor(&self) -> Option<Option<PixelRect>> {
        match self.clip_px {
            ClipPixels::Unclipped => Some(None),
            ClipPixels::Rect(r) => Some(Some(r)),
            ClipPixels::Empty => None,
        }
    }

    fn geometry(
        &mut self,
        image: Option<ImageHandle>,
        filter: Filter,
        add: impl FnOnce(&mut Vec<Vertex>, &Canvas),
    ) {
        let Some(scissor) = self.scissor() else {
            return;
        };
        let first = self.plan.vertices.len() as u32;
        add(&mut self.plan.vertices, self.canvas);
        let end = self.plan.vertices.len() as u32;
        if end == first {
            return;
        }
        if let Some(Batch {
            scissor: s,
            kind:
                BatchKind::Geometry {
                    image: i,
                    filter: f,
                    vertices,
                },
        }) = self.plan.batches.last_mut()
            && *s == scissor
            && *i == image
            && *f == filter
        {
            vertices.end = end;
            return;
        }
        self.plan.batches.push(Batch {
            scissor,
            kind: BatchKind::Geometry {
                image,
                filter,
                vertices: first..end,
            },
        });
    }

    fn text(&mut self, run: &'a TextRun) {
        let Some(scissor) = self.scissor() else {
            return;
        };
        let index = self.plan.texts.len();
        self.plan.texts.push(run);
        if let Some(Batch {
            scissor: s,
            kind: BatchKind::Text { runs },
        }) = self.plan.batches.last_mut()
            && *s == scissor
        {
            runs.end = index + 1;
            return;
        }
        self.plan.batches.push(Batch {
            scissor,
            kind: BatchKind::Text {
                runs: index..index + 1,
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nine_slice::Insets;
    use crate::ui_frame::UiFrame;
    use ss2_ui::{Color, Rect, TextAlign, Vec2};

    fn h(id: u32) -> ImageHandle {
        ImageHandle::new(id)
    }

    fn sizes(i: ImageHandle) -> Option<(u32, u32)> {
        match i.id() {
            1 | 2 => Some((32, 32)),
            3 => Some((8, 8)),
            _ => None,
        }
    }

    fn r(x: f32, y: f32, w: f32, hh: f32) -> Rect {
        Rect { x, y, w, h: hh }
    }

    const WHITE: Color = Color([1.0; 4]);

    fn canvas() -> Canvas {
        Canvas::fit(1920, 1080).unwrap()
    }

    fn plan_of(f: &UiFrame) -> Plan<'_> {
        Plan::build(f, &canvas(), (1920, 1080), &sizes)
    }

    fn text(f: &mut UiFrame, s: &'static str) {
        f.text(Vec2 { x: 0.0, y: 0.0 }, s, 20.0, WHITE, TextAlign::Left);
    }

    #[test]
    fn an_empty_frame_has_no_batches() {
        let f = UiFrame::new();
        let p = plan_of(&f);
        assert!(p.batches.is_empty() && p.vertices.is_empty() && p.texts.is_empty());
        assert_eq!(f.draw_call_count(&canvas(), (1920, 1080), &sizes), 0);
    }

    #[test]
    fn consecutive_flat_quads_are_one_draw_call() {
        let mut f = UiFrame::new();
        for i in 0..10 {
            f.quad(r(i as f32 * 20.0, 0.0, 10.0, 10.0), WHITE);
        }
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 1);
        assert_eq!(p.vertices.len(), 60);
        assert_eq!(
            p.batches[0].kind,
            BatchKind::Geometry {
                image: None,
                filter: Filter::Nearest,
                vertices: 0..60
            }
        );
    }

    #[test]
    fn the_same_image_in_a_row_merges_and_a_different_one_does_not() {
        let mut f = UiFrame::new();
        f.image(h(1), r(0.0, 0.0, 8.0, 8.0))
            .image(h(1), r(10.0, 0.0, 8.0, 8.0))
            .image(h(1), r(20.0, 0.0, 8.0, 8.0))
            .image(h(2), r(30.0, 0.0, 8.0, 8.0))
            .image(h(1), r(40.0, 0.0, 8.0, 8.0));
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 3);
        let counts: Vec<u32> = p
            .batches
            .iter()
            .map(|b| match &b.kind {
                BatchKind::Geometry { vertices, .. } => vertices.end - vertices.start,
                BatchKind::Text { .. } => panic!(),
            })
            .collect();
        assert_eq!(counts, [18, 6, 6]);
    }

    #[test]
    fn a_different_sampling_splits_the_batch_but_only_for_images() {
        let mut f = UiFrame::new();
        f.image_ex(
            h(1),
            r(0.0, 0.0, 8.0, 8.0),
            UvRect::FULL,
            WHITE,
            Filter::Nearest,
        )
        .image_ex(
            h(1),
            r(9.0, 0.0, 8.0, 8.0),
            UvRect::FULL,
            WHITE,
            Filter::Linear,
        );
        assert_eq!(plan_of(&f).batches.len(), 2);
        // Different tint or uv do not split: they are vertex data.
        let mut f = UiFrame::new();
        f.image_ex(
            h(1),
            r(0.0, 0.0, 8.0, 8.0),
            UvRect::FULL,
            WHITE,
            Filter::Linear,
        )
        .image_ex(
            h(1),
            r(9.0, 0.0, 8.0, 8.0),
            UvRect {
                u0: 0.0,
                v0: 0.0,
                u1: 0.5,
                v1: 0.5,
            },
            Color([1.0, 0.0, 0.0, 0.5]),
            Filter::Linear,
        );
        assert_eq!(plan_of(&f).batches.len(), 1);
    }

    #[test]
    fn an_image_between_flat_quads_keeps_the_order_and_splits_them() {
        let mut f = UiFrame::new();
        f.quad(r(0.0, 0.0, 5.0, 5.0), WHITE)
            .image(h(1), r(0.0, 0.0, 5.0, 5.0))
            .quad(r(0.0, 0.0, 5.0, 5.0), WHITE);
        let p = plan_of(&f);
        let images: Vec<Option<ImageHandle>> = p
            .batches
            .iter()
            .map(|b| match &b.kind {
                BatchKind::Geometry { image, .. } => *image,
                BatchKind::Text { .. } => panic!(),
            })
            .collect();
        assert_eq!(images, [None, Some(h(1)), None]);
    }

    #[test]
    fn text_in_a_row_is_one_batch_and_text_between_geometry_keeps_the_order() {
        let mut f = UiFrame::new();
        text(&mut f, "a");
        text(&mut f, "b");
        f.quad(r(0.0, 0.0, 5.0, 5.0), WHITE);
        text(&mut f, "c");
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 3);
        assert_eq!(p.batches[0].kind, BatchKind::Text { runs: 0..2 });
        assert!(matches!(p.batches[1].kind, BatchKind::Geometry { .. }));
        assert_eq!(p.batches[2].kind, BatchKind::Text { runs: 2..3 });
        let words: Vec<&str> = p.texts.iter().map(|t| &*t.text).collect();
        assert_eq!(words, ["a", "b", "c"]);
    }

    #[test]
    fn a_draw_list_makes_one_batch_of_quads_and_one_of_text() {
        let mut list = ss2_ui::DrawList::default();
        for i in 0..5 {
            list.quads.push(ss2_ui::Quad {
                rect: r(i as f32 * 10.0, 0.0, 5.0, 5.0),
                color: WHITE,
            });
            list.texts.push(TextRun {
                pos: Vec2 { x: 0.0, y: 0.0 },
                text: "t".into(),
                size: 10.0,
                color: WHITE,
                align: TextAlign::Left,
            });
        }
        let p = Plan::build(&list, &canvas(), (1920, 1080), &sizes);
        assert_eq!(p.batches.len(), 2);
        assert_eq!(p.vertices.len(), 30);
        assert_eq!(p.texts.len(), 5);
    }

    #[test]
    fn a_clip_splits_batches_and_the_same_clip_merges_them() {
        let mut f = UiFrame::new();
        f.quad(r(0.0, 0.0, 5.0, 5.0), WHITE)
            .push_clip(r(0.0, 0.0, 100.0, 100.0))
            .quad(r(0.0, 0.0, 50.0, 50.0), WHITE)
            .quad(r(60.0, 0.0, 50.0, 50.0), WHITE)
            .pop_clip()
            .quad(r(0.0, 0.0, 5.0, 5.0), WHITE);
        let p = plan_of(&f);
        let scissors: Vec<Option<PixelRect>> = p.batches.iter().map(|b| b.scissor).collect();
        assert_eq!(
            scissors,
            [
                None,
                Some(PixelRect {
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 100
                }),
                None
            ]
        );
        // The same clip pushed twice in a row (pop, push) merges only if nothing differs in between.
        let mut f = UiFrame::new();
        let clip = r(0.0, 0.0, 100.0, 100.0);
        f.push_clip(clip)
            .quad(r(0.0, 0.0, 5.0, 5.0), WHITE)
            .pop_clip()
            .push_clip(clip)
            .quad(r(0.0, 0.0, 5.0, 5.0), WHITE)
            .pop_clip();
        assert_eq!(plan_of(&f).batches.len(), 1);
    }

    #[test]
    fn text_in_different_clips_is_not_one_batch() {
        let mut f = UiFrame::new();
        text(&mut f, "a");
        f.push_clip(r(0.0, 0.0, 100.0, 100.0));
        text(&mut f, "b");
        text(&mut f, "c");
        f.pop_clip();
        text(&mut f, "d");
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 3);
        assert_eq!(p.batches[0].kind, BatchKind::Text { runs: 0..1 });
        assert_eq!(p.batches[1].kind, BatchKind::Text { runs: 1..3 });
        assert_eq!(p.batches[2].kind, BatchKind::Text { runs: 3..4 });
    }

    #[test]
    fn nested_clips_use_the_intersection() {
        let mut f = UiFrame::new();
        f.push_clip(r(0.0, 0.0, 100.0, 100.0))
            .push_clip(r(50.0, 50.0, 100.0, 100.0))
            .quad(r(0.0, 0.0, 500.0, 500.0), WHITE);
        let p = plan_of(&f);
        assert_eq!(
            p.batches[0].scissor,
            Some(PixelRect {
                x: 50,
                y: 50,
                w: 50,
                h: 50
            })
        );
    }

    #[test]
    fn commands_under_an_empty_clip_are_skipped_and_do_not_split_the_batches_around_them() {
        let mut f = UiFrame::new();
        f.quad(r(0.0, 0.0, 5.0, 5.0), WHITE)
            .push_clip(r(0.0, 0.0, 10.0, 10.0))
            .push_clip(r(100.0, 100.0, 10.0, 10.0)) // empty
            .quad(r(0.0, 0.0, 500.0, 500.0), WHITE)
            .image(h(1), r(0.0, 0.0, 500.0, 500.0))
            .nine_slice(h(1), r(0.0, 0.0, 500.0, 500.0), Insets::uniform(4.0), 1.0);
        text(&mut f, "skipped");
        f.pop_clip().pop_clip().quad(r(10.0, 0.0, 5.0, 5.0), WHITE);
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 1, "{:?}", p.batches);
        assert_eq!(p.vertices.len(), 12);
        assert!(p.texts.is_empty());
    }

    #[test]
    fn a_clip_with_no_pixel_in_it_skips_too() {
        let mut f = UiFrame::new();
        f.push_clip(r(0.6, 0.6, 0.5, 0.5))
            .quad(r(0.0, 0.0, 50.0, 50.0), WHITE);
        assert_eq!(plan_of(&f).batches.len(), 0);
    }

    #[test]
    fn an_unbalanced_pop_is_ignored_and_counted() {
        let mut f = UiFrame::new();
        f.pop_clip()
            .quad(r(0.0, 0.0, 5.0, 5.0), WHITE)
            .pop_clip()
            .pop_clip();
        let p = plan_of(&f);
        assert_eq!(p.unbalanced_pops, 3);
        assert_eq!(p.batches.len(), 1);
        assert_eq!(p.batches[0].scissor, None);
    }

    #[test]
    fn a_clip_left_open_at_the_end_is_harmless() {
        let mut f = UiFrame::new();
        f.push_clip(r(0.0, 0.0, 10.0, 10.0))
            .quad(r(0.0, 0.0, 50.0, 50.0), WHITE);
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 1);
        assert_eq!(p.unbalanced_pops, 0);
    }

    #[test]
    fn unknown_images_are_skipped_and_reported_once() {
        let mut f = UiFrame::new();
        f.image(h(9), r(0.0, 0.0, 5.0, 5.0))
            .image(h(9), r(0.0, 0.0, 5.0, 5.0))
            .nine_slice(h(8), r(0.0, 0.0, 50.0, 50.0), Insets::uniform(2.0), 1.0)
            .quad(r(0.0, 0.0, 5.0, 5.0), WHITE);
        let p = plan_of(&f);
        assert_eq!(p.missing, [h(9), h(8)]);
        assert_eq!(p.batches.len(), 1);
    }

    #[test]
    fn a_nine_slice_is_one_draw_call_of_nine_quads() {
        let mut f = UiFrame::new();
        f.nine_slice(h(1), r(0.0, 0.0, 200.0, 100.0), Insets::uniform(8.0), 1.0)
            .nine_slice(h(1), r(300.0, 0.0, 200.0, 100.0), Insets::uniform(8.0), 2.0);
        let p = plan_of(&f);
        assert_eq!(p.batches.len(), 1);
        assert_eq!(p.vertices.len(), 2 * 9 * 6);
        assert_eq!(f.draw_call_count(&canvas(), (1920, 1080), &sizes), 1);
    }

    #[test]
    fn a_nine_slice_that_draws_nothing_adds_no_batch() {
        let mut f = UiFrame::new();
        f.nine_slice(h(1), r(0.0, 0.0, 0.0, 100.0), Insets::uniform(8.0), 1.0)
            .nine_slice(h(1), r(0.0, 0.0, 10.0, 10.0), Insets::uniform(8.0), 0.0);
        let p = plan_of(&f);
        assert!(p.batches.is_empty() && p.missing.is_empty());
    }

    #[test]
    fn image_vertices_carry_the_uv_and_the_tint_and_the_canvas_transform() {
        let mut f = UiFrame::new();
        f.image_ex(
            h(1),
            r(10.0, 20.0, 30.0, 40.0),
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.75,
                v1: 1.0,
            },
            Color([0.1, 0.2, 0.3, 0.4]),
            Filter::Linear,
        );
        let c = Canvas::fit(960, 540).unwrap(); // scale 0.5
        let p = Plan::build(&f, &c, (960, 540), &sizes);
        let v = &p.vertices;
        assert_eq!(v.len(), 6);
        assert_eq!((v[0].pos, v[0].uv), ([5.0, 10.0], [0.25, 0.5]));
        assert_eq!((v[1].pos, v[1].uv), ([20.0, 10.0], [0.75, 0.5]));
        assert_eq!((v[2].pos, v[2].uv), ([5.0, 30.0], [0.25, 1.0]));
        assert_eq!((v[4].pos, v[4].uv), ([20.0, 30.0], [0.75, 1.0]));
        assert!(v.iter().all(|v| v.color == [0.1, 0.2, 0.3, 0.4]));
        // The clip is converted with the same canvas.
        let mut f = UiFrame::new();
        f.push_clip(r(100.0, 100.0, 200.0, 80.0))
            .quad(r(0.0, 0.0, 999.0, 999.0), WHITE);
        let p = Plan::build(&f, &c, (960, 540), &sizes);
        assert_eq!(
            p.batches[0].scissor,
            Some(PixelRect {
                x: 50,
                y: 50,
                w: 100,
                h: 40
            })
        );
    }

    #[test]
    fn the_draw_call_count_for_a_typical_panel_scene() {
        // Panel (9-slice), two labels, a clipped list of three rows with an icon each, a footer label.
        let mut f = UiFrame::new();
        f.nine_slice(
            h(1),
            r(100.0, 100.0, 600.0, 500.0),
            Insets::uniform(8.0),
            2.0,
        );
        text(&mut f, "Title");
        text(&mut f, "Subtitle");
        f.push_clip(r(120.0, 200.0, 560.0, 300.0));
        for i in 0..3 {
            let y = 200.0 + i as f32 * 100.0;
            f.quad(r(120.0, y, 560.0, 90.0), Color([0.2, 0.2, 0.2, 1.0]));
            f.image(h(3), r(130.0, y + 10.0, 64.0, 64.0));
            text(&mut f, "row");
        }
        f.pop_clip();
        text(&mut f, "Footer");
        // 1 nine-slice, 1 text, then per row: quad, icon, text (the clip makes these one batch each time they alternate),
        // then the footer text unclipped.
        assert_eq!(
            f.draw_call_count(&canvas(), (1920, 1080), &sizes),
            1 + 1 + 3 * 3 + 1
        );
    }
}
