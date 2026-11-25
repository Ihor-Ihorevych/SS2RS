//! CPU-side conversion of quads, images and 9-slice cells into GPU vertices.

use ss2_ui::{Canvas, DrawList, Rect, Vec2};

use crate::nine_slice::UvRect;

/// One vertex of a textured, tinted triangle, positioned in **window pixels**.
///
/// A flat quad is drawn with a white 1x1 texture, so every vertex has a texture coordinate.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// Window-pixel position (origin top-left, y down).
    pub pos: [f32; 2],
    /// Texture coordinate (0..=1, `v` runs down the image).
    pub uv: [f32; 2],
    /// RGBA color (multiplied with the texel).
    pub color: [f32; 4],
}

/// Append the six vertices (two triangles) of `rect` (canvas units, converted to window pixels), showing `uv`.
pub(crate) fn push_quad(
    out: &mut Vec<Vertex>,
    canvas: &Canvas,
    rect: Rect,
    uv: UvRect,
    color: [f32; 4],
) {
    let tl = canvas.to_screen(Vec2 {
        x: rect.x,
        y: rect.y,
    });
    let br = canvas.to_screen(Vec2 {
        x: rect.x + rect.w,
        y: rect.y + rect.h,
    });
    let vertex = |x: f32, y: f32, u: f32, v: f32| Vertex {
        pos: [x, y],
        uv: [u, v],
        color,
    };
    out.extend([
        vertex(tl.x, tl.y, uv.u0, uv.v0),
        vertex(br.x, tl.y, uv.u1, uv.v0),
        vertex(tl.x, br.y, uv.u0, uv.v1),
        vertex(br.x, tl.y, uv.u1, uv.v0),
        vertex(br.x, br.y, uv.u1, uv.v1),
        vertex(tl.x, br.y, uv.u0, uv.v1),
    ]);
}

/// Six vertices (two triangles) per quad, with canvas units converted to window pixels.
pub fn quads_to_vertices(list: &DrawList, canvas: &Canvas) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(list.quads.len() * 6);
    for quad in &list.quads {
        push_quad(&mut out, canvas, quad.rect, UvRect::FULL, quad.color.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_ui::{Canvas, Color, DrawList, Quad, Rect};

    fn list_of(rects: &[Rect]) -> DrawList {
        DrawList {
            quads: rects
                .iter()
                .map(|&rect| Quad {
                    rect,
                    color: Color([0.1, 0.2, 0.3, 1.0]),
                })
                .collect(),
            texts: Vec::new(),
        }
    }

    #[test]
    fn one_quad_makes_six_vertices() {
        let c = Canvas::fit(1920, 1080).unwrap();
        let v = quads_to_vertices(
            &list_of(&[Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            }]),
            &c,
        );
        assert_eq!(v.len(), 6);
    }

    #[test]
    fn vertices_apply_scale_and_offset() {
        let c = Canvas::fit(2560, 1080).unwrap(); // scale 1.0, offset.x 320
        let v = quads_to_vertices(
            &list_of(&[Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            }]),
            &c,
        );
        let min_x = v.iter().map(|v| v.pos[0]).fold(f32::MAX, f32::min);
        let max_x = v.iter().map(|v| v.pos[0]).fold(f32::MIN, f32::max);
        let max_y = v.iter().map(|v| v.pos[1]).fold(f32::MIN, f32::max);
        assert_eq!((min_x, max_x, max_y), (320.0, 420.0, 50.0));
    }

    #[test]
    fn vertices_scale_sizes_with_canvas_scale() {
        let c = Canvas::fit(960, 540).unwrap(); // scale 0.5
        let v = quads_to_vertices(
            &list_of(&[Rect {
                x: 100.0,
                y: 100.0,
                w: 200.0,
                h: 80.0,
            }]),
            &c,
        );
        let max_x = v.iter().map(|v| v.pos[0]).fold(f32::MIN, f32::max);
        let max_y = v.iter().map(|v| v.pos[1]).fold(f32::MIN, f32::max);
        assert_eq!((max_x, max_y), (150.0, 90.0));
    }

    #[test]
    fn vertices_carry_color() {
        let c = Canvas::fit(1920, 1080).unwrap();
        let v = quads_to_vertices(
            &list_of(&[Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            }]),
            &c,
        );
        assert!(v.iter().all(|v| v.color == [0.1, 0.2, 0.3, 1.0]));
    }

    #[test]
    fn empty_list_makes_no_vertices() {
        let c = Canvas::fit(1920, 1080).unwrap();
        assert!(quads_to_vertices(&DrawList::default(), &c).is_empty());
    }

    #[test]
    fn zero_area_quad_is_still_six_vertices_without_nan() {
        let c = Canvas::fit(1280, 720).unwrap();
        let v = quads_to_vertices(
            &list_of(&[Rect {
                x: 5.0,
                y: 5.0,
                w: 0.0,
                h: 0.0,
            }]),
            &c,
        );
        assert_eq!(v.len(), 6);
        assert!(v.iter().all(|v| v.pos.iter().all(|p| p.is_finite())));
    }
}
