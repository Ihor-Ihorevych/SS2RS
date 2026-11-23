//! The renderer-independent output of the UI: quads and text runs in canvas units.

use std::borrow::Cow;

use crate::geometry::{Rect, Vec2};

/// Linear RGBA color, each channel 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color(pub [f32; 4]);

/// A filled rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    /// Where to draw, in canvas units.
    pub rect: Rect,
    /// Fill color.
    pub color: Color,
}

/// Horizontal anchoring of a [`TextRun`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// `pos.x` is the left edge.
    Left,
    /// `pos.x` is the horizontal center.
    Center,
}

/// A single line of text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// Anchor position (top edge; horizontal meaning depends on `align`).
    pub pos: Vec2,
    /// The text. Fixed labels are borrowed (`Cow::Borrowed`) so that drawing them allocates nothing; text built at run time is owned.
    pub text: Cow<'static, str>,
    /// Font size in canvas units.
    pub size: f32,
    /// Text color.
    pub color: Color,
    /// Horizontal anchoring.
    pub align: TextAlign,
}

/// Everything the UI wants drawn this frame, back to front.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawList {
    /// Filled rectangles.
    pub quads: Vec<Quad>,
    /// Text runs (drawn after all quads).
    pub texts: Vec<TextRun>,
}

impl DrawList {
    /// Empty the list **keeping its capacity**, so that filling it again each frame does not allocate (`docs/UI_BUDGET.md`).
    pub fn clear(&mut self) {
        self.quads.clear();
        self.texts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_empties_the_list_and_keeps_its_capacity() {
        let mut l = DrawList::default();
        for i in 0..10 {
            l.quads.push(Quad {
                rect: Rect {
                    x: i as f32,
                    y: 0.0,
                    w: 1.0,
                    h: 1.0,
                },
                color: Color([1.0; 4]),
            });
            l.texts.push(TextRun {
                pos: Vec2 { x: 0.0, y: 0.0 },
                text: "x".into(),
                size: 10.0,
                color: Color([1.0; 4]),
                align: TextAlign::Left,
            });
        }
        let (qc, tc) = (l.quads.capacity(), l.texts.capacity());
        l.clear();
        assert!(l.quads.is_empty() && l.texts.is_empty());
        assert_eq!((l.quads.capacity(), l.texts.capacity()), (qc, tc));
    }

    #[test]
    fn draw_list_default_is_empty() {
        let l = DrawList::default();
        assert!(l.quads.is_empty() && l.texts.is_empty());
    }
}
