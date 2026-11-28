//! Placeholder-skin layout in canvas units (1920x1080).

use crate::geometry::Rect;
use crate::menu::Item;

/// Width of a button/toggle/slider row.
pub const ITEM_W: f32 = 640.0;
/// Height of a button/toggle/slider row.
pub const ITEM_H: f32 = 72.0;
/// Vertical gap between rows.
pub const GAP: f32 = 16.0;
/// Height of a spacer row.
pub const SPACER_H: f32 = 36.0;
/// Width of a row of read-only text.
pub const TEXT_W: f32 = 1400.0;
/// Height of a row of read-only text.
pub const TEXT_H: f32 = 40.0;
/// Vertical gap after a row of read-only text.
pub const TEXT_GAP: f32 = 6.0;
/// Top of the first row.
pub const TOP: f32 = 320.0;
/// Slider track offset from the row's left edge.
pub const SLIDER_TRACK_X: f32 = 300.0;
/// Slider track width.
pub const SLIDER_TRACK_W: f32 = 300.0;

/// One rectangle per item (spacers included), centered horizontally on the canvas, **without allocating**: the layout of a frame.
pub(crate) fn item_rect_iter(items: &[Item]) -> impl Iterator<Item = Rect> + '_ {
    let mut y = TOP;
    items.iter().map(move |item| {
        let (w, h, gap) = match item {
            Item::Spacer => (ITEM_W, SPACER_H, GAP),
            Item::Text { .. } => (TEXT_W, TEXT_H, TEXT_GAP),
            _ => (ITEM_W, ITEM_H, GAP),
        };
        let x = (crate::geometry::CANVAS_W - w) / 2.0;
        let r = Rect { x, y, w, h };
        y += h + gap;
        r
    })
}

/// One rectangle per item (spacers included), centered horizontally on the canvas.
pub(crate) fn item_rects(items: &[Item]) -> Vec<Rect> {
    item_rect_iter(items).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{ScreenId, items};

    #[test]
    fn button_rects_are_centered_640_wide() {
        let r = item_rects(items(ScreenId::Main));
        assert_eq!(r.len(), 5);
        assert_eq!((r[0].x, r[0].w, r[0].h), (640.0, 640.0, 72.0));
        assert_eq!(r[0].y, 320.0);
    }

    #[test]
    fn rects_do_not_overlap_and_follow_gap() {
        let list = items(ScreenId::Options);
        let r = item_rects(list);
        assert_eq!(r.len(), list.len());
        for w in r.windows(2) {
            let gap = w[1].y - (w[0].y + w[0].h);
            assert!((gap - 16.0).abs() < 1e-3, "gap {gap}");
        }
        assert_eq!(r[5].h, 36.0, "spacer height");
    }
}
