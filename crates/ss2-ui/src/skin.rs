//! Turns the menu state into a [`DrawList`] using the placeholder theme.

use crate::draw::{DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{Rect, Vec2};
use crate::layout::{SLIDER_TRACK_W, SLIDER_TRACK_X, item_rects};
use crate::menu::{Item, Menu, items, slider_value, title};
use crate::theme;

const LABEL_SIZE: f32 = 40.0;
/// Font size of the read-only lines of the About screen.
const TEXT_SIZE: f32 = 28.0;
const TEXT_TOP: f32 = 14.0;

impl Menu {
    /// One rectangle per item of the current screen (spacers included), in canvas units.
    pub fn item_rects(&self) -> Vec<Rect> {
        item_rects(items(self.screen()))
    }

    /// Everything to draw for the current screen, back to front.
    pub fn draw(&self) -> DrawList {
        let mut list = DrawList::default();
        list.texts.push(TextRun {
            pos: Vec2 { x: 960.0, y: 160.0 },
            text: title(self.screen()).to_string(),
            size: 96.0,
            color: theme::TEXT,
            align: TextAlign::Center,
        });
        for (i, (item, r)) in items(self.screen())
            .iter()
            .zip(self.item_rects())
            .enumerate()
        {
            match *item {
                Item::Spacer => continue,
                Item::Text { line } => {
                    list.texts.push(TextRun {
                        pos: Vec2 {
                            x: r.x,
                            y: r.y + 4.0,
                        },
                        text: line.to_string(),
                        size: TEXT_SIZE,
                        color: theme::TEXT,
                        align: TextAlign::Left,
                    });
                    continue;
                }
                _ => {}
            }
            let color = if i == self.focus {
                theme::FOCUSED
            } else {
                theme::IDLE
            };
            list.quads.push(Quad { rect: r, color });
            let text = |x: f32, text: String, align: TextAlign| TextRun {
                pos: Vec2 {
                    x,
                    y: r.y + TEXT_TOP,
                },
                text,
                size: LABEL_SIZE,
                color: theme::TEXT,
                align,
            };
            match *item {
                Item::Button { label, .. } => {
                    list.texts
                        .push(text(r.x + r.w / 2.0, label.to_string(), TextAlign::Center));
                }
                Item::Toggle { label, setting } => {
                    let on = match setting {
                        crate::menu::ToggleSetting::Vsync => self.settings.vsync,
                        crate::menu::ToggleSetting::Fullscreen => self.settings.fullscreen,
                    };
                    list.texts
                        .push(text(r.x + 24.0, label.to_string(), TextAlign::Left));
                    let value = if on { "On" } else { "Off" };
                    list.texts
                        .push(text(r.x + 560.0, value.to_string(), TextAlign::Center));
                }
                Item::Slider { label, setting } => {
                    let v = slider_value(&self.settings, setting).clamp(0.0, 1.0);
                    list.texts
                        .push(text(r.x + 24.0, label.to_string(), TextAlign::Left));
                    let pct = format!("{}%", (v * 100.0).round() as i32);
                    list.texts.push(text(r.x + 225.0, pct, TextAlign::Center));
                    let track = Rect {
                        x: r.x + SLIDER_TRACK_X,
                        y: r.y + 32.0,
                        w: SLIDER_TRACK_W,
                        h: 8.0,
                    };
                    list.quads.push(Quad {
                        rect: track,
                        color: theme::TRACK,
                    });
                    list.quads.push(Quad {
                        rect: Rect {
                            w: track.w * v,
                            ..track
                        },
                        color: theme::TRACK_FILL,
                    });
                }
                Item::Spacer | Item::Text { .. } => {}
            }
        }
        list
    }
}
