//! The renderer-independent output of the UI: quads and text runs in canvas units.

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
    /// The text.
    pub text: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_list_default_is_empty() {
        let l = DrawList::default();
        assert!(l.quads.is_empty() && l.texts.is_empty());
    }
}
