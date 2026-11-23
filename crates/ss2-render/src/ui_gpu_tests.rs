//! GPU tests of the 2D renderer: images, 9-slice, scissor, text clipping, render targets and recovery.
//!
//! Every picture is made in code (no game content). They need a GPU adapter (a software one is fine) and skip with
//! a message when there is none. Every test that makes or destroys a device takes `gpu_test_guard`.

use ss2_ui::{Canvas, Color, DrawList, Quad, Rect, TextAlign, TextRun, Vec2};

use crate::gpu::{self, GpuConfig, gpu_test_guard};
use crate::{
    Fill, Filter, ImageHandle, ImageRgba8, Insets, RenderError, Renderer, UiFrame, UvRect,
};

const BLACK: Color = Color([0.0, 0.0, 0.0, 1.0]);
const BLACK_PX: [u8; 4] = [0, 0, 0, 255];
const WHITE: Color = Color([1.0; 4]);

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn offscreen(size: (u32, u32)) -> Option<Renderer> {
    match Renderer::new_offscreen(size) {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            None
        }
        Err(e) => panic!("renderer creation failed: {e}"),
    }
}

/// A 1920x1080 renderer: the canvas scale is exactly 1 and there is no offset, so canvas units are pixels.
fn unit_renderer() -> Option<Renderer> {
    offscreen((1920, 1080))
}

fn px(img: &ImageRgba8, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.pixels[i..i + 4].try_into().unwrap()
}

fn solid(w: u32, h: u32, rgba: [u8; 4]) -> ImageRgba8 {
    ImageRgba8 {
        width: w,
        height: h,
        pixels: rgba.repeat((w * h) as usize),
    }
}

/// A checkerboard of `cell`-pixel squares whose every texel also differs from its neighbours in the green and blue
/// channels, so a one-texel shift or a wrong row is seen.
fn pattern(w: u32, h: u32, cell: u32) -> ImageRgba8 {
    let mut pixels = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let odd = ((x / cell) + (y / cell)) % 2 == 1;
            pixels.extend([
                if odd { 240 } else { 20 },
                (x * 255 / (w - 1).max(1)) as u8,
                (y * 255 / (h - 1).max(1)) as u8,
                255,
            ]);
        }
    }
    ImageRgba8 {
        width: w,
        height: h,
        pixels,
    }
}

const EDGE_TOP: [u8; 4] = [255, 128, 0, 255];
const EDGE_BOTTOM: [u8; 4] = [0, 255, 255, 255];
const EDGE_LEFT: [u8; 4] = [128, 0, 255, 255];
const EDGE_RIGHT: [u8; 4] = [255, 0, 128, 255];
const CENTRE: [u8; 4] = [200, 200, 200, 255];

/// A 24x24 image with 8-texel borders: every corner texel is different, each edge is one colour, the centre another.
fn bordered() -> ImageRgba8 {
    let mut img = solid(24, 24, CENTRE);
    for y in 0..24u32 {
        for x in 0..24u32 {
            let (left, right) = (x < 8, x >= 16);
            let (top, bottom) = (y < 8, y >= 16);
            let c = match (left || right, top || bottom) {
                (true, true) => {
                    let id = u8::from(right) + 2 * u8::from(bottom);
                    [
                        (x % 8 * 30 + 10) as u8,
                        (y % 8 * 30 + 10) as u8,
                        id * 60 + 30,
                        255,
                    ]
                }
                (false, true) => {
                    if top {
                        EDGE_TOP
                    } else {
                        EDGE_BOTTOM
                    }
                }
                (true, false) => {
                    if left {
                        EDGE_LEFT
                    } else {
                        EDGE_RIGHT
                    }
                }
                (false, false) => CENTRE,
            };
            let i = ((y * 24 + x) * 4) as usize;
            img.pixels[i..i + 4].copy_from_slice(&c);
        }
    }
    img
}

fn upload(r: &mut Renderer, img: &ImageRgba8) -> ImageHandle {
    r.create_image(img).expect("the image is uploaded")
}

fn render(r: &mut Renderer, frame: &UiFrame) -> ImageRgba8 {
    r.render_frame_to_image(frame, BLACK).unwrap()
}

fn count_where(img: &ImageRgba8, f: impl Fn(u32, u32, [u8; 4]) -> bool) -> usize {
    let mut n = 0;
    for y in 0..img.height {
        for x in 0..img.width {
            if f(x, y, px(img, x, y)) {
                n += 1;
            }
        }
    }
    n
}

fn close(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) <= tolerance)
}

// ---- images ----------------------------------------------------------------------------------------------

#[test]
fn an_image_drawn_one_to_one_is_pixel_exact_with_nearest_sampling() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = pattern(32, 32, 4);
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.image(h, rect(100.0, 50.0, 32.0, 32.0));
    let out = render(&mut r, &f);
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(
                px(&out, 100 + x, 50 + y),
                px(&src, x, y),
                "texel {x},{y} drawn 1:1"
            );
        }
    }
    // Nothing outside the rectangle, including the pixel just outside on every side.
    assert_eq!(px(&out, 99, 60), BLACK_PX);
    assert_eq!(px(&out, 132, 60), BLACK_PX);
    assert_eq!(px(&out, 110, 49), BLACK_PX);
    assert_eq!(px(&out, 110, 82), BLACK_PX);
    assert_eq!(count_where(&out, |_, _, p| p != BLACK_PX), 32 * 32);
}

#[test]
fn nearest_sampling_replicates_texels_when_the_image_is_enlarged() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = pattern(8, 8, 1);
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.image(h, rect(200.0, 200.0, 40.0, 40.0)); // 5x
    let out = render(&mut r, &f);
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(
                px(&out, 200 + x, 200 + y),
                px(&src, x / 5, y / 5),
                "pixel {x},{y}"
            );
        }
    }
}

#[test]
fn linear_sampling_blends_where_nearest_does_not() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = ImageRgba8 {
        width: 2,
        height: 1,
        pixels: vec![0, 0, 0, 255, 255, 255, 255, 255],
    };
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.image_ex(
        h,
        rect(0.0, 0.0, 16.0, 4.0),
        UvRect::FULL,
        WHITE,
        Filter::Linear,
    );
    f.image_ex(
        h,
        rect(0.0, 10.0, 16.0, 4.0),
        UvRect::FULL,
        WHITE,
        Filter::Nearest,
    );
    let out = render(&mut r, &f);
    let blended = (0..16).filter(|&x| px(&out, x, 1)[0] > 10 && px(&out, x, 1)[0] < 245);
    assert!(blended.count() >= 4, "a ramp between the two texels");
    assert!(px(&out, 0, 1)[0] < 20 && px(&out, 15, 1)[0] > 235);
    assert!(
        (0..16).all(|x| matches!(px(&out, x, 11)[0], 0 | 255)),
        "nearest has only the two texel values"
    );
    assert_eq!(px(&out, 3, 11)[0], 0);
    assert_eq!(px(&out, 12, 11)[0], 255);
}

#[test]
fn the_texture_rectangle_selects_part_of_the_image() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let mut src = solid(8, 8, [255, 0, 0, 255]);
    for y in 0..8 {
        for x in 4..8 {
            let i = ((y * 8 + x) * 4) as usize;
            src.pixels[i..i + 4].copy_from_slice(&[0, 255, 0, 255]);
        }
    }
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    // The right half only, stretched over 16x16.
    f.image_ex(
        h,
        rect(50.0, 50.0, 16.0, 16.0),
        UvRect {
            u0: 0.5,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        },
        WHITE,
        Filter::Nearest,
    );
    let out = render(&mut r, &f);
    assert_eq!(px(&out, 50, 50), [0, 255, 0, 255]);
    assert_eq!(px(&out, 65, 65), [0, 255, 0, 255]);
}

#[test]
fn the_tint_multiplies_the_texels_and_alpha_blends_over_what_is_behind() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let white = upload(&mut r, &solid(4, 4, [255, 255, 255, 255]));
    let translucent = upload(&mut r, &solid(4, 4, [255, 0, 0, 128]));
    let mut f = UiFrame::new();
    f.quad(rect(0.0, 0.0, 100.0, 100.0), Color([0.0, 0.0, 1.0, 1.0]));
    f.image_ex(
        white,
        rect(0.0, 0.0, 8.0, 8.0),
        UvRect::FULL,
        Color([1.0, 0.5, 0.0, 1.0]),
        Filter::Nearest,
    );
    f.image(translucent, rect(20.0, 0.0, 8.0, 8.0));
    f.image_ex(
        white,
        rect(40.0, 0.0, 8.0, 8.0),
        UvRect::FULL,
        Color([1.0, 1.0, 1.0, 0.5]),
        Filter::Nearest,
    );
    let out = render(&mut r, &f);
    assert!(
        close(px(&out, 4, 4), [255, 128, 0, 255], 1),
        "{:?}",
        px(&out, 4, 4)
    );
    // 50% red over blue (straight alpha): (128, 0, 127).
    assert!(
        close(px(&out, 24, 4), [128, 0, 127, 255], 2),
        "{:?}",
        px(&out, 24, 4)
    );
    // 50% white over blue: (128, 128, 255).
    assert!(
        close(px(&out, 44, 4), [128, 128, 255, 255], 2),
        "{:?}",
        px(&out, 44, 4)
    );
    // Behind everything, the blue square.
    assert_eq!(px(&out, 60, 60), [0, 0, 255, 255]);
}

#[test]
fn flat_quads_are_unchanged_by_the_white_texel_they_sample() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let mut f = UiFrame::new();
    f.quad(rect(10.0, 10.0, 20.0, 20.0), Color([0.2, 0.4, 0.6, 1.0]));
    let out = render(&mut r, &f);
    assert_eq!(px(&out, 15, 15), [51, 102, 153, 255]);
    // The same through the DrawList path, and through the new one for a DrawList.
    let mut list = DrawList::default();
    list.quads.push(Quad {
        rect: rect(10.0, 10.0, 20.0, 20.0),
        color: Color([0.2, 0.4, 0.6, 1.0]),
    });
    list.texts.push(TextRun {
        pos: Vec2 { x: 100.0, y: 100.0 },
        text: "Hello".into(),
        size: 40.0,
        color: WHITE,
        align: TextAlign::Left,
    });
    let a = r.render_to_image(&list, BLACK).unwrap();
    let b = r
        .render_frame_to_image(&UiFrame::from(&list), BLACK)
        .unwrap();
    assert_eq!(a, b);
    assert!(
        count_where(&a, |_, _, p| p != BLACK_PX) > 400 + 100,
        "text and quad are there"
    );
}

// ---- 9-slice ---------------------------------------------------------------------------------------------

#[test]
fn a_nine_slice_keeps_its_corner_pixels_and_stretches_the_rest_at_several_sizes() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = bordered();
    let h = upload(&mut r, &src);
    let sizes = [(24u32, 24u32), (40, 24), (24, 40), (97, 63), (300, 200)];
    for (w, hh) in sizes {
        let (x0, y0) = (500u32, 300u32);
        let mut f = UiFrame::new();
        f.nine_slice(
            h,
            rect(x0 as f32, y0 as f32, w as f32, hh as f32),
            Insets::uniform(8.0),
            1.0,
        );
        let out = render(&mut r, &f);
        // Corners: every pixel is the source texel, at the four corners of the rectangle.
        for cy in 0..8 {
            for cx in 0..8 {
                assert_eq!(px(&out, x0 + cx, y0 + cy), px(&src, cx, cy), "{w}x{hh} TL");
                assert_eq!(
                    px(&out, x0 + w - 8 + cx, y0 + cy),
                    px(&src, 16 + cx, cy),
                    "{w}x{hh} TR"
                );
                assert_eq!(
                    px(&out, x0 + cx, y0 + hh - 8 + cy),
                    px(&src, cx, 16 + cy),
                    "{w}x{hh} BL"
                );
                assert_eq!(
                    px(&out, x0 + w - 8 + cx, y0 + hh - 8 + cy),
                    px(&src, 16 + cx, 16 + cy),
                    "{w}x{hh} BR"
                );
            }
        }
        // Edges and centre stretch: one colour each over their whole span.
        for x in 8..w - 8 {
            assert_eq!(
                px(&out, x0 + x, y0 + 3),
                EDGE_TOP,
                "{w}x{hh} top edge at {x}"
            );
            assert_eq!(
                px(&out, x0 + x, y0 + hh - 3),
                EDGE_BOTTOM,
                "{w}x{hh} bottom at {x}"
            );
        }
        for y in 8..hh - 8 {
            assert_eq!(px(&out, x0 + 3, y0 + y), EDGE_LEFT, "{w}x{hh} left at {y}");
            assert_eq!(
                px(&out, x0 + w - 3, y0 + y),
                EDGE_RIGHT,
                "{w}x{hh} right at {y}"
            );
        }
        if w > 16 && hh > 16 {
            let centre = count_where(&out, |_, _, p| p == CENTRE);
            assert_eq!(centre, ((w - 16) * (hh - 16)) as usize, "{w}x{hh} centre");
        }
        // Nothing outside, including the pixel just outside on every side.
        assert_eq!(px(&out, x0 - 1, y0 + 4), BLACK_PX);
        assert_eq!(px(&out, x0 + w, y0 + 4), BLACK_PX);
        assert_eq!(px(&out, x0 + 4, y0 - 1), BLACK_PX);
        assert_eq!(px(&out, x0 + 4, y0 + hh), BLACK_PX);
        assert_eq!(
            count_where(&out, |_, _, p| p != BLACK_PX),
            (w * hh) as usize,
            "{w}x{hh}: exactly the rectangle is covered"
        );
    }
}

#[test]
fn a_nine_slice_smaller_than_its_borders_shrinks_them_and_still_draws() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = bordered();
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.nine_slice(h, rect(100.0, 100.0, 10.0, 10.0), Insets::uniform(8.0), 1.0);
    let out = render(&mut r, &f);
    // Four 5x5 corners, no centre and no edge.
    assert_eq!(count_where(&out, |_, _, p| p != BLACK_PX), 100);
    assert!(close(px(&out, 101, 101), px(&src, 1, 1), 40));
    assert!(close(px(&out, 108, 101), px(&src, 22, 1), 40));
}

#[test]
fn a_nine_slice_border_scale_doubles_the_borders() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = bordered();
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.nine_slice(
        h,
        rect(100.0, 100.0, 120.0, 80.0),
        Insets::uniform(8.0),
        2.0,
    );
    let out = render(&mut r, &f);
    // A corner texel becomes a 2x2 block.
    for cy in 0..8 {
        for cx in 0..8 {
            for (dx, dy) in [(0, 0), (1, 1)] {
                assert_eq!(
                    px(&out, 100 + cx * 2 + dx, 100 + cy * 2 + dy),
                    px(&src, cx, cy)
                );
            }
        }
    }
    assert_eq!(px(&out, 160, 107), EDGE_TOP);
    assert_eq!(px(&out, 160, 140), CENTRE);
}

#[test]
fn a_tiled_nine_slice_repeats_the_centre() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    // A 6x6 image with 1-texel borders and a 4x4 centre of two colours in a checker (2x2 cells).
    let mut src = solid(6, 6, [9, 9, 9, 255]);
    for y in 1..5u32 {
        for x in 1..5u32 {
            let c = if (x / 3 + y / 3) % 2 == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            };
            let i = ((y * 6 + x) * 4) as usize;
            src.pixels[i..i + 4].copy_from_slice(&c);
        }
    }
    // Centre texels x,y in 1..5: x/3 is 0 for x in 1..3 and 1 for x in 3..5, so cells of 2 texels.
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.push(crate::UiCommand::NineSlice(crate::NineSliceDraw {
        image: h,
        rect: rect(100.0, 100.0, 18.0, 18.0), // centre 16x16 = four repeats each way
        insets: Insets::uniform(1.0),
        scale: 1.0,
        fill: Fill::Tile,
        tint: WHITE,
        filter: Filter::Nearest,
    }));
    let out = render(&mut r, &f);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let tx = 1 + x % 4;
            let ty = 1 + y % 4;
            assert_eq!(
                px(&out, 101 + x, 101 + y),
                px(&src, tx, ty),
                "repeat at {x},{y}"
            );
        }
    }
}

// ---- scissor ---------------------------------------------------------------------------------------------

fn red_pixels(img: &ImageRgba8) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for y in 0..img.height {
        for x in 0..img.width {
            if px(img, x, y) == [255, 0, 0, 255] {
                v.push((x, y));
            }
        }
    }
    v
}

#[test]
fn a_scissored_quad_is_clipped_to_exactly_the_rectangle() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let mut f = UiFrame::new();
    f.push_clip(rect(100.0, 50.0, 40.0, 30.0))
        .quad(rect(0.0, 0.0, 1920.0, 1080.0), Color([1.0, 0.0, 0.0, 1.0]))
        .pop_clip();
    let out = render(&mut r, &f);
    assert_eq!(red_pixels(&out).len(), 40 * 30);
    for (x, y) in [(100, 50), (139, 79), (100, 79), (139, 50)] {
        assert_eq!(px(&out, x, y), [255, 0, 0, 255], "inside corner {x},{y}");
    }
    // The pixel just outside, on every side and at the corners.
    for (x, y) in [
        (99, 60),
        (140, 60),
        (120, 49),
        (120, 80),
        (99, 49),
        (140, 80),
        (99, 80),
        (140, 49),
    ] {
        assert_eq!(px(&out, x, y), BLACK_PX, "just outside at {x},{y}");
    }
}

#[test]
fn a_clip_with_fractional_edges_covers_the_pixels_whose_centres_are_inside() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((1280, 720)) else {
        return;
    };
    let canvas = Canvas::fit(1280, 720).unwrap(); // scale 2/3
    let clip = rect(100.4, 50.6, 399.2, 129.9);
    let mut f = UiFrame::new();
    f.push_clip(clip)
        .quad(rect(0.0, 0.0, 1920.0, 1080.0), Color([1.0, 0.0, 0.0, 1.0]))
        .pop_clip();
    let out = render(&mut r, &f);
    // Worked out independently, pixel by pixel: the centre of the pixel is inside the clip rectangle.
    let tl = canvas.to_screen(Vec2 {
        x: clip.x,
        y: clip.y,
    });
    let br = canvas.to_screen(Vec2 {
        x: clip.x + clip.w,
        y: clip.y + clip.h,
    });
    let mut expected = 0;
    for y in 0..720u32 {
        for x in 0..1280u32 {
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = cx >= tl.x && cx < br.x && cy >= tl.y && cy < br.y;
            expected += usize::from(inside);
            assert_eq!(
                px(&out, x, y) == [255, 0, 0, 255],
                inside,
                "pixel {x},{y} against clip {tl:?}..{br:?}"
            );
        }
    }
    assert!(expected > 20_000);
}

#[test]
fn nested_clips_draw_in_the_intersection_and_a_pop_restores_the_outer_clip() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let red = Color([1.0, 0.0, 0.0, 1.0]);
    let green = Color([0.0, 1.0, 0.0, 1.0]);
    let mut f = UiFrame::new();
    f.push_clip(rect(100.0, 100.0, 100.0, 100.0))
        .push_clip(rect(150.0, 150.0, 100.0, 100.0))
        .quad(rect(0.0, 0.0, 1920.0, 1080.0), red)
        .pop_clip()
        // Back in the outer clip: a green quad over all of it.
        .quad(rect(0.0, 0.0, 1920.0, 1080.0), green)
        .pop_clip()
        // Unclipped again: a small quad outside every clip.
        .quad(rect(400.0, 400.0, 10.0, 10.0), red);
    let out = render(&mut r, &f);
    // The red intersection (150..200) is covered by the green quad: no red remains but the outside quad.
    assert_eq!(red_pixels(&out).len(), 100);
    assert_eq!(px(&out, 120, 120), [0, 255, 0, 255]);
    assert_eq!(px(&out, 199, 199), [0, 255, 0, 255]);
    assert_eq!(px(&out, 200, 150), BLACK_PX);
    assert_eq!(px(&out, 99, 150), BLACK_PX);
    assert_eq!(px(&out, 405, 405), [255, 0, 0, 255]);
    let greens = count_where(&out, |_, _, p| p == [0, 255, 0, 255]);
    assert_eq!(greens, 100 * 100);
}

#[test]
fn an_empty_clip_draws_nothing_and_the_draws_after_it_still_work() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = pattern(8, 8, 2);
    let h = upload(&mut r, &src);
    let mut f = UiFrame::new();
    f.push_clip(rect(0.0, 0.0, 10.0, 10.0))
        .push_clip(rect(500.0, 500.0, 10.0, 10.0)) // misses the outer one
        .quad(rect(0.0, 0.0, 1920.0, 1080.0), Color([1.0, 0.0, 0.0, 1.0]))
        .image(h, rect(0.0, 0.0, 8.0, 8.0))
        .nine_slice(h, rect(0.0, 0.0, 600.0, 600.0), Insets::uniform(2.0), 1.0)
        .text(
            Vec2 { x: 0.0, y: 0.0 },
            "hidden",
            50.0,
            WHITE,
            TextAlign::Left,
        )
        .pop_clip()
        .pop_clip()
        .quad(rect(300.0, 300.0, 10.0, 10.0), Color([1.0, 0.0, 0.0, 1.0]));
    let out = render(&mut r, &f);
    assert_eq!(count_where(&out, |_, _, p| p != BLACK_PX), 100);
    assert_eq!(px(&out, 305, 305), [255, 0, 0, 255]);
}

#[test]
fn images_and_nine_slices_are_clipped_too() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let src = pattern(32, 32, 4);
    let b = bordered();
    let (img, nine) = (upload(&mut r, &src), upload(&mut r, &b));
    let mut f = UiFrame::new();
    f.push_clip(rect(110.0, 60.0, 10.0, 8.0))
        .image(img, rect(100.0, 50.0, 32.0, 32.0))
        .pop_clip()
        .push_clip(rect(520.0, 320.0, 30.0, 12.0))
        .nine_slice(
            nine,
            rect(500.0, 300.0, 100.0, 60.0),
            Insets::uniform(8.0),
            1.0,
        )
        .pop_clip();
    let out = render(&mut r, &f);
    assert_eq!(count_where(&out, |_, _, p| p != BLACK_PX), 10 * 8 + 30 * 12);
    // The clipped image shows the texels it would show unclipped.
    assert_eq!(px(&out, 110, 60), px(&src, 10, 10));
    assert_eq!(px(&out, 119, 67), px(&src, 19, 17));
    assert_eq!(px(&out, 109, 60), BLACK_PX);
    assert_eq!(px(&out, 120, 60), BLACK_PX);
}

// ---- text ------------------------------------------------------------------------------------------------

fn long_text(f: &mut UiFrame) {
    f.text(
        Vec2 { x: 100.0, y: 100.0 },
        "WWWW WWWW WWWW WWWW",
        64.0,
        WHITE,
        TextAlign::Left,
    );
}

#[test]
fn text_is_clipped_by_the_scissor_rectangle() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let mut plain = UiFrame::new();
    long_text(&mut plain);
    let whole = render(&mut r, &plain);
    let clip = (150u32, 110u32, 200u32, 50u32);
    let mut f = UiFrame::new();
    f.push_clip(rect(
        clip.0 as f32,
        clip.1 as f32,
        clip.2 as f32,
        clip.3 as f32,
    ));
    long_text(&mut f);
    f.pop_clip();
    let clipped = render(&mut r, &f);

    let inside =
        |x: u32, y: u32| x >= clip.0 && x < clip.0 + clip.2 && y >= clip.1 && y < clip.1 + clip.3;
    let outside_whole = count_where(&whole, |x, y, p| !inside(x, y) && p != BLACK_PX);
    let outside_clipped = count_where(&clipped, |x, y, p| !inside(x, y) && p != BLACK_PX);
    assert!(
        outside_whole > 2000,
        "the text reaches outside the rectangle without a clip"
    );
    assert_eq!(
        outside_clipped, 0,
        "no text pixel outside the clip, not even just outside it"
    );
    let inside_clipped = count_where(&clipped, |x, y, p| inside(x, y) && p != BLACK_PX);
    assert!(inside_clipped > 200, "the text is drawn inside the clip");
    // Inside the rectangle it is the same picture as the unclipped text.
    for y in clip.1..clip.1 + clip.3 {
        for x in clip.0..clip.0 + clip.2 {
            assert_eq!(px(&clipped, x, y), px(&whole, x, y), "pixel {x},{y}");
        }
    }
}

#[test]
fn text_between_quads_is_drawn_in_order() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let cover = Color([0.0, 0.0, 1.0, 1.0]);
    let mut under = UiFrame::new();
    under.quad(rect(90.0, 90.0, 600.0, 120.0), cover);
    long_text(&mut under);
    let mut over = UiFrame::new();
    long_text(&mut over);
    over.quad(rect(90.0, 90.0, 600.0, 120.0), cover);

    let under = render(&mut r, &under);
    let over = render(&mut r, &over);
    let in_quad = |x: u32, y: u32| (90..690).contains(&x) && (90..210).contains(&y);
    let white_under = count_where(&under, |x, y, p| in_quad(x, y) && p[0] > 200 && p[1] > 200);
    let white_over = count_where(&over, |x, y, p| in_quad(x, y) && p[0] > 200 && p[1] > 200);
    assert!(white_under > 1000, "text drawn after the quad is visible");
    assert_eq!(white_over, 0, "text drawn before the quad is covered by it");
    assert_eq!(
        count_where(&over, |_, _, p| p == [0, 0, 255, 255]),
        600 * 120
    );
}

#[test]
fn text_in_two_clips_is_clipped_to_each_and_both_are_drawn() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let mut f = UiFrame::new();
    f.push_clip(rect(100.0, 100.0, 120.0, 80.0));
    long_text(&mut f);
    f.pop_clip().push_clip(rect(100.0, 400.0, 120.0, 80.0));
    f.text(
        Vec2 { x: 100.0, y: 400.0 },
        "WWWW WWWW WWWW",
        64.0,
        WHITE,
        TextAlign::Left,
    );
    f.pop_clip();
    let out = render(&mut r, &f);
    let in_a = |x: u32, y: u32| (100..220).contains(&x) && (100..180).contains(&y);
    let in_b = |x: u32, y: u32| (100..220).contains(&x) && (400..480).contains(&y);
    assert_eq!(
        count_where(&out, |x, y, p| !in_a(x, y) && !in_b(x, y) && p != BLACK_PX),
        0
    );
    assert!(count_where(&out, |x, y, p| in_a(x, y) && p != BLACK_PX) > 100);
    assert!(count_where(&out, |x, y, p| in_b(x, y) && p != BLACK_PX) > 100);
}

// ---- render to texture -----------------------------------------------------------------------------------

/// What the render-target tests draw, at `(ox, oy)`: a background, a 9-slice panel, an image, a clipped label.
fn scene(frame: &mut UiFrame, panel: ImageHandle, icon: ImageHandle, (ox, oy): (f32, f32)) {
    frame.quad(rect(ox, oy, 160.0, 96.0), Color([0.1, 0.2, 0.3, 1.0]));
    frame.nine_slice(
        panel,
        rect(ox + 8.0, oy + 8.0, 100.0, 60.0),
        Insets::uniform(8.0),
        1.0,
    );
    frame.image(icon, rect(ox + 120.0, oy + 10.0, 32.0, 32.0));
    frame.push_clip(rect(ox + 10.0, oy + 70.0, 90.0, 20.0));
    frame.text(
        Vec2 {
            x: ox + 12.0,
            y: oy + 68.0,
        },
        "Target text",
        24.0,
        WHITE,
        TextAlign::Left,
    );
    frame.pop_clip();
}

#[test]
fn a_render_target_drawn_back_gives_the_same_pixels_as_drawing_directly() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let panel = upload(&mut r, &bordered());
    let icon = upload(&mut r, &pattern(16, 16, 2));
    let target = r.create_render_target(160, 96).unwrap();
    assert_eq!(r.image_size(target), Some((160, 96)));

    let mut inner = UiFrame::new();
    scene(&mut inner, panel, icon, (0.0, 0.0));
    r.render_to_target(target, &inner, Color([0.0, 0.0, 0.0, 0.0]))
        .unwrap();

    let mut via_target = UiFrame::new();
    via_target.image(target, rect(300.0, 200.0, 160.0, 96.0));
    let mut direct = UiFrame::new();
    scene(&mut direct, panel, icon, (300.0, 200.0));

    let a = render(&mut r, &via_target);
    let b = render(&mut r, &direct);
    assert_eq!(a, b, "the target drawn 1:1 is the scene drawn in place");
    assert!(count_where(&a, |_, _, p| p != BLACK_PX) >= 160 * 96);
    // And the target itself holds the scene at its own origin.
    let own = r.read_image(target).unwrap();
    assert_eq!((own.width, own.height), (160, 96));
    assert!(
        close(px(&own, 5, 5), [26, 51, 77, 255], 1),
        "{:?}",
        px(&own, 5, 5)
    );
}

#[test]
fn a_semi_transparent_target_blends_like_the_same_picture_drawn_directly() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let target = r.create_render_target(64, 64).unwrap();
    let mut inner = UiFrame::new();
    // Overlapping translucent quads on a transparent target.
    inner.quad(rect(0.0, 0.0, 40.0, 40.0), Color([1.0, 0.0, 0.0, 0.5]));
    inner.quad(rect(20.0, 20.0, 40.0, 40.0), Color([0.0, 1.0, 0.0, 0.5]));
    r.render_to_target(target, &inner, Color([0.0, 0.0, 0.0, 0.0]))
        .unwrap();

    let back = Color([0.0, 0.0, 1.0, 1.0]);
    let mut via_target = UiFrame::new();
    via_target.quad(rect(100.0, 100.0, 64.0, 64.0), back);
    via_target.image(target, rect(100.0, 100.0, 64.0, 64.0));
    let mut direct = UiFrame::new();
    direct.quad(rect(100.0, 100.0, 64.0, 64.0), back);
    direct.quad(rect(100.0, 100.0, 40.0, 40.0), Color([1.0, 0.0, 0.0, 0.5]));
    direct.quad(rect(120.0, 120.0, 40.0, 40.0), Color([0.0, 1.0, 0.0, 0.5]));
    let a = render(&mut r, &via_target);
    let b = render(&mut r, &direct);
    for y in 100..164 {
        for x in 100..164 {
            assert!(
                close(px(&a, x, y), px(&b, x, y), 2),
                "pixel {x},{y}: {:?} against {:?}",
                px(&a, x, y),
                px(&b, x, y)
            );
        }
    }
    // The overlap really mixes both.
    assert!(px(&a, 130, 130)[1] > 60 && px(&a, 130, 130)[0] > 20);

    // A tint with alpha fades the whole picture: the same as drawing it at half strength.
    let mut faded = UiFrame::new();
    faded.quad(rect(100.0, 100.0, 64.0, 64.0), back);
    faded.image_ex(
        target,
        rect(100.0, 100.0, 64.0, 64.0),
        UvRect::FULL,
        Color([1.0, 1.0, 1.0, 0.5]),
        Filter::Nearest,
    );
    let mut direct_faded = UiFrame::new();
    direct_faded.quad(rect(100.0, 100.0, 64.0, 64.0), back);
    direct_faded.quad(rect(100.0, 100.0, 40.0, 40.0), Color([1.0, 0.0, 0.0, 0.25]));
    direct_faded.quad(rect(120.0, 120.0, 40.0, 40.0), Color([0.0, 1.0, 0.0, 0.25]));
    let a = render(&mut r, &faded);
    let b = render(&mut r, &direct_faded);
    for y in 100..164 {
        for x in 100..164 {
            if (120..140).contains(&x) && (120..140).contains(&y) {
                continue; // the overlap: fading the group is not fading each member, see below
            }
            assert!(
                close(px(&a, x, y), px(&b, x, y), 3),
                "faded pixel {x},{y}: {:?} against {:?}",
                px(&a, x, y),
                px(&b, x, y)
            );
        }
    }
    // The overlap: red at 0.5 and green at 0.5 over it make (0.25, 0.5, 0) at alpha 0.75; faded to half that is
    // (0.125, 0.25, 0) at alpha 0.375, over blue: (0.125, 0.25, 0.625) = (32, 64, 159).
    assert!(
        close(px(&a, 130, 130), [32, 64, 159, 255], 2),
        "{:?}",
        px(&a, 130, 130)
    );
}

#[test]
fn a_target_can_be_drawn_into_another_target() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let inner = r.create_render_target(32, 32).unwrap();
    let outer = r.create_render_target(96, 64).unwrap();
    let red = Color([1.0, 0.0, 0.0, 1.0]);
    let mut f = UiFrame::new();
    f.quad(rect(0.0, 0.0, 32.0, 32.0), red);
    f.quad(rect(8.0, 8.0, 16.0, 16.0), Color([0.0, 1.0, 0.0, 1.0]));
    r.render_to_target(inner, &f, Color([0.0; 4])).unwrap();

    let mut g = UiFrame::new();
    g.quad(rect(0.0, 0.0, 96.0, 64.0), Color([0.0, 0.0, 1.0, 1.0]));
    g.image(inner, rect(4.0, 4.0, 32.0, 32.0));
    g.image(inner, rect(50.0, 20.0, 32.0, 32.0));
    r.render_to_target(outer, &g, Color([0.0; 4])).unwrap();

    let mut main = UiFrame::new();
    main.image(outer, rect(500.0, 500.0, 96.0, 64.0));
    let out = render(&mut r, &main);
    assert_eq!(px(&out, 500 + 5, 500 + 5), [255, 0, 0, 255]);
    assert_eq!(px(&out, 500 + 4 + 16, 500 + 4 + 16), [0, 255, 0, 255]);
    assert_eq!(px(&out, 500 + 50 + 3, 500 + 20 + 3), [255, 0, 0, 255]);
    assert_eq!(px(&out, 500 + 90, 500 + 60), [0, 0, 255, 255]);
    // Re-rendering the inner one changes what the outer shows the next time it is drawn into.
    let mut h = UiFrame::new();
    h.quad(rect(0.0, 0.0, 32.0, 32.0), Color([1.0, 1.0, 0.0, 1.0]));
    r.render_to_target(inner, &h, Color([0.0; 4])).unwrap();
    r.render_to_target(outer, &g, Color([0.0; 4])).unwrap();
    let out = render(&mut r, &main);
    assert_eq!(px(&out, 500 + 5, 500 + 5), [255, 255, 0, 255]);
}

#[test]
fn a_picture_cannot_be_drawn_into_itself_or_into_a_loop() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let a = r.create_render_target(16, 16).unwrap();
    let b = r.create_render_target(16, 16).unwrap();
    let c = r.create_render_target(16, 16).unwrap();
    let mut itself = UiFrame::new();
    itself.image(a, rect(0.0, 0.0, 8.0, 8.0));
    let e = r.render_to_target(a, &itself, Color([0.0; 4])).unwrap_err();
    assert!(matches!(e, RenderError::Image(_)), "{e}");
    assert!(e.to_string().contains("into itself"), "{e}");

    // b is drawn from a; c from b; drawing c into a would make a loop a -> c -> b -> a.
    let mut from_a = UiFrame::new();
    from_a.image(a, rect(0.0, 0.0, 8.0, 8.0));
    r.render_to_target(b, &from_a, Color([0.0; 4])).unwrap();
    let mut from_b = UiFrame::new();
    from_b.image(b, rect(0.0, 0.0, 8.0, 8.0));
    r.render_to_target(c, &from_b, Color([0.0; 4])).unwrap();
    let mut from_c = UiFrame::new();
    from_c.image(c, rect(0.0, 0.0, 8.0, 8.0));
    let e = r.render_to_target(a, &from_c, Color([0.0; 4])).unwrap_err();
    assert!(e.to_string().contains("into itself"), "{e}");
    // The refused render left no recipe behind: the other direction is still fine.
    r.render_to_target(a, &UiFrame::new(), Color([0.0; 4]))
        .unwrap();
}

#[test]
fn bad_image_requests_are_errors_and_change_nothing() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let before = r.image_count();
    let short = ImageRgba8 {
        width: 4,
        height: 4,
        pixels: vec![0; 10],
    };
    assert!(matches!(r.create_image(&short), Err(RenderError::Image(_))));
    assert!(matches!(
        r.create_image(&solid(0, 4, [0; 4])),
        Err(RenderError::Image(_))
    ));
    assert!(matches!(
        r.create_render_target(0, 5),
        Err(RenderError::Image(_))
    ));
    assert!(matches!(
        r.create_render_target(1 << 20, 5),
        Err(RenderError::Image(_))
    ));
    assert_eq!(r.image_count(), before);

    let img = upload(&mut r, &solid(2, 2, [1, 2, 3, 255]));
    let target = r.create_render_target(8, 8).unwrap();
    // An uploaded image is not a render target; an unknown one is not either.
    assert!(r.render_to_target(img, &UiFrame::new(), BLACK).is_err());
    let mut f = UiFrame::new();
    f.image(img, rect(0.0, 0.0, 4.0, 4.0));
    assert!(r.destroy_image(img));
    assert!(!r.destroy_image(img), "a second destroy finds nothing");
    assert_eq!(r.image_size(img), None);
    assert!(r.render_to_target(img, &UiFrame::new(), BLACK).is_err());
    // Drawing a destroyed image is an error, in a frame and in a target.
    assert!(matches!(
        r.render_frame_to_image(&f, BLACK),
        Err(RenderError::Image(_))
    ));
    assert!(matches!(
        r.render_to_target(target, &f, BLACK),
        Err(RenderError::Image(_))
    ));
    // And the renderer is still fine.
    let mut ok = UiFrame::new();
    ok.quad(rect(0.0, 0.0, 10.0, 10.0), Color([1.0, 0.0, 0.0, 1.0]));
    assert_eq!(px(&render(&mut r, &ok), 5, 5), [255, 0, 0, 255]);
}

// ---- recovery --------------------------------------------------------------------------------------------

/// A scene with everything: an image, a 9-slice, a clipped text, a target and a target drawn from a target.
fn everything(r: &mut Renderer) -> UiFrame {
    let panel = upload(r, &bordered());
    let icon = upload(r, &pattern(16, 16, 2));
    let inner = r.create_render_target(160, 96).unwrap();
    let mut f = UiFrame::new();
    scene(&mut f, panel, icon, (0.0, 0.0));
    r.render_to_target(inner, &f, Color([0.0, 0.0, 0.0, 0.0]))
        .unwrap();
    let outer = r.create_render_target(200, 130).unwrap();
    let mut g = UiFrame::new();
    g.quad(rect(0.0, 0.0, 200.0, 130.0), Color([0.3, 0.1, 0.1, 0.8]));
    g.image(inner, rect(20.0, 20.0, 160.0, 96.0));
    r.render_to_target(outer, &g, Color([0.0, 0.0, 0.0, 0.0]))
        .unwrap();

    let mut main = UiFrame::new();
    main.quad(rect(0.0, 0.0, 1920.0, 1080.0), Color([0.0, 0.0, 0.3, 1.0]));
    main.image(icon, rect(50.0, 50.0, 64.0, 64.0));
    main.nine_slice(
        panel,
        rect(150.0, 50.0, 220.0, 120.0),
        Insets::uniform(8.0),
        2.0,
    );
    main.image(outer, rect(400.0, 50.0, 200.0, 130.0));
    main.push_clip(rect(50.0, 300.0, 180.0, 40.0));
    main.text(
        Vec2 { x: 50.0, y: 300.0 },
        "Clipped after recovery",
        36.0,
        WHITE,
        TextAlign::Left,
    );
    main.pop_clip();
    main
}

#[test]
fn after_the_device_is_destroyed_images_scissor_and_targets_draw_the_same_picture() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let main = everything(&mut r);
    let before = r.render_frame_to_image(&main, BLACK).unwrap();
    assert!(count_where(&before, |_, _, p| p != [0, 0, 77, 255]) > 20_000);
    let handles = r.image_count();
    assert_eq!(r.stats().device_rebuilds, 0);

    r.destroy_device_for_tests();
    let after = r
        .render_frame_to_image(&main, BLACK)
        .expect("the frame rebuilds the device and draws");
    assert_eq!(r.stats().device_rebuilds, 1);
    assert_eq!(after, before, "the same picture on the new device");
    assert_eq!(
        r.image_count(),
        handles,
        "the same images, under the same handles"
    );

    // Again, and with a read of a target by itself: its content was drawn again, not lost.
    r.destroy_device_for_tests();
    let again = r.render_frame_to_image(&main, BLACK).unwrap();
    assert_eq!(again, before);
    assert_eq!(r.stats().device_rebuilds, 2);
    // New images made after a rebuild do not collide with the old handles.
    let fresh = upload(&mut r, &solid(1, 1, [9, 9, 9, 255]));
    let mut f = UiFrame::new();
    f.image(fresh, rect(0.0, 0.0, 4.0, 4.0));
    assert_eq!(px(&render(&mut r, &f), 1, 1), [9, 9, 9, 255]);
    assert_eq!(r.image_count(), handles + 1);
}

#[test]
fn a_target_is_drawn_again_after_what_it_was_drawn_from_whatever_the_creation_order() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    // The outer target has the smaller handle: replay must not simply go by handle.
    let outer = r.create_render_target(32, 32).unwrap();
    let inner = r.create_render_target(32, 32).unwrap();
    let mut f = UiFrame::new();
    f.quad(rect(0.0, 0.0, 32.0, 32.0), Color([1.0, 0.0, 0.0, 1.0]));
    r.render_to_target(inner, &f, Color([0.0; 4])).unwrap();
    let mut g = UiFrame::new();
    g.image(inner, rect(0.0, 0.0, 32.0, 32.0));
    r.render_to_target(outer, &g, Color([0.0; 4])).unwrap();
    let mut main = UiFrame::new();
    main.image(outer, rect(10.0, 10.0, 32.0, 32.0));
    let before = render(&mut r, &main);
    assert_eq!(px(&before, 20, 20), [255, 0, 0, 255]);
    r.destroy_device_for_tests();
    assert_eq!(render(&mut r, &main), before);
}

#[test]
fn a_target_drawn_from_an_image_destroyed_since_is_drawn_again_without_it() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = unit_renderer() else {
        return;
    };
    let img = upload(&mut r, &solid(8, 8, [0, 255, 0, 255]));
    let target = r.create_render_target(32, 32).unwrap();
    let mut f = UiFrame::new();
    f.quad(rect(0.0, 0.0, 32.0, 32.0), Color([1.0, 0.0, 0.0, 1.0]));
    f.image(img, rect(8.0, 8.0, 16.0, 16.0));
    r.render_to_target(target, &f, Color([0.0; 4])).unwrap();
    assert!(r.destroy_image(img));
    // Let the GPU finish before the device goes (a loss is noticed on the poll that follows completed work).
    r.read_image(target).unwrap();
    r.destroy_device_for_tests();
    let mut main = UiFrame::new();
    main.image(target, rect(0.0, 0.0, 32.0, 32.0));
    let out = render(&mut r, &main);
    assert_eq!(
        px(&out, 16, 16),
        [255, 0, 0, 255],
        "the red stays, the green square is gone"
    );
}

// ---- other target formats -----------------------------------------------------------------------------------

fn renderer_with_format(format: wgpu::TextureFormat, size: (u32, u32)) -> Option<Renderer> {
    let config = GpuConfig::from_env();
    let instance = gpu::make_instance(&config, None);
    let gpu = match pollster::block_on(gpu::request_gpu(&instance, None, Some(format), &config)) {
        Ok(gpu) => gpu,
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            return None;
        }
        Err(e) => panic!("renderer creation failed: {e}"),
    };
    let texture = crate::context::capture_texture(&gpu.device, format, size).unwrap();
    Some(Renderer::from_gpu_for_tests(gpu, texture, size))
}

#[test]
fn images_and_targets_work_on_a_bgra_target_and_come_out_in_rgba_order() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = renderer_with_format(wgpu::TextureFormat::Bgra8Unorm, (1920, 1080)) else {
        return;
    };
    let img = upload(&mut r, &solid(4, 4, [10, 20, 30, 255]));
    let target = r.create_render_target(8, 8).unwrap();
    let mut inner = UiFrame::new();
    inner.image(img, rect(0.0, 0.0, 8.0, 8.0));
    r.render_to_target(target, &inner, Color([0.0; 4])).unwrap();
    assert_eq!(px(&r.read_image(target).unwrap(), 3, 3), [10, 20, 30, 255]);
    let mut f = UiFrame::new();
    f.image(img, rect(0.0, 0.0, 4.0, 4.0));
    f.image(target, rect(10.0, 0.0, 8.0, 8.0));
    let out = r.capture_ui_frame(Some(&f), BLACK).unwrap();
    assert_eq!(px(&out, 1, 1), [10, 20, 30, 255]);
    assert_eq!(px(&out, 12, 2), [10, 20, 30, 255]);
}

#[test]
fn a_floating_point_target_captures_images_and_targets_through_the_temporary_renderer() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = renderer_with_format(wgpu::TextureFormat::Rgba16Float, (1920, 1080)) else {
        return;
    };
    let img = upload(&mut r, &pattern(8, 8, 2));
    let target = r.create_render_target(8, 8).unwrap();
    let mut inner = UiFrame::new();
    inner.image(img, rect(0.0, 0.0, 8.0, 8.0));
    r.render_to_target(target, &inner, Color([0.0; 4])).unwrap();
    let mut f = UiFrame::new();
    f.image(img, rect(0.0, 0.0, 8.0, 8.0));
    f.image(target, rect(20.0, 0.0, 8.0, 8.0));
    let out = r.capture_ui_frame(Some(&f), BLACK).unwrap();
    let src = pattern(8, 8, 2);
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(px(&out, x, y), px(&src, x, y), "image texel {x},{y}");
            assert_eq!(px(&out, 20 + x, y), px(&src, x, y), "target texel {x},{y}");
        }
    }
}
