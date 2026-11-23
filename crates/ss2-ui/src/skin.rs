//! Turns the menu state into a [`DrawList`] using the placeholder theme.

use crate::draw::{DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{Rect, Vec2};
use crate::layout::{SLIDER_TRACK_W, SLIDER_TRACK_X, item_rect_iter, item_rects};
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

    /// Everything to draw for the current screen, back to front. Allocates a new list: a frame loop should keep one and call [`Menu::draw_into`].
    pub fn draw(&self) -> DrawList {
        let mut list = DrawList::default();
        self.draw_into(&mut list);
        list
    }

    /// Replace the contents of `list` with everything to draw for the current screen, back to front. Once `list` has grown to the size of the
    /// largest screen this **allocates nothing** (`docs/UI_BUDGET.md`, tested with a counting allocator).
    pub fn draw_into(&self, list: &mut DrawList) {
        list.clear();
        list.texts.push(TextRun {
            pos: Vec2 { x: 960.0, y: 160.0 },
            text: title(self.screen()).into(),
            size: 96.0,
            color: theme::TEXT,
            align: TextAlign::Center,
        });
        for (i, (item, r)) in items(self.screen())
            .iter()
            .zip(item_rect_iter(items(self.screen())))
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
                        text: (*line).into(),
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
            let text = |x: f32, text: &'static str, align: TextAlign| TextRun {
                pos: Vec2 {
                    x,
                    y: r.y + TEXT_TOP,
                },
                text: text.into(),
                size: LABEL_SIZE,
                color: theme::TEXT,
                align,
            };
            match *item {
                Item::Button { label, .. } => {
                    list.texts
                        .push(text(r.x + r.w / 2.0, label, TextAlign::Center));
                }
                Item::Toggle { label, setting } => {
                    let on = match setting {
                        crate::menu::ToggleSetting::Vsync => self.settings.vsync,
                        crate::menu::ToggleSetting::Fullscreen => self.settings.fullscreen,
                    };
                    list.texts.push(text(r.x + 24.0, label, TextAlign::Left));
                    let value = if on { "On" } else { "Off" };
                    list.texts.push(text(r.x + 560.0, value, TextAlign::Center));
                }
                Item::Slider { label, setting } => {
                    let v = slider_value(&self.settings, setting).clamp(0.0, 1.0);
                    list.texts.push(text(r.x + 24.0, label, TextAlign::Left));
                    let pct = percent_text((v * 100.0).round() as i32);
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
    }
}

/// "0%" to "100%" as static text, so that a slider's value is drawn without formatting a string each frame (other values are clamped).
fn percent_text(percent: i32) -> &'static str {
    const PERCENT: [&str; 101] = [
        "0%", "1%", "2%", "3%", "4%", "5%", "6%", "7%", "8%", "9%", "10%", "11%", "12%", "13%",
        "14%", "15%", "16%", "17%", "18%", "19%", "20%", "21%", "22%", "23%", "24%", "25%", "26%",
        "27%", "28%", "29%", "30%", "31%", "32%", "33%", "34%", "35%", "36%", "37%", "38%", "39%",
        "40%", "41%", "42%", "43%", "44%", "45%", "46%", "47%", "48%", "49%", "50%", "51%", "52%",
        "53%", "54%", "55%", "56%", "57%", "58%", "59%", "60%", "61%", "62%", "63%", "64%", "65%",
        "66%", "67%", "68%", "69%", "70%", "71%", "72%", "73%", "74%", "75%", "76%", "77%", "78%",
        "79%", "80%", "81%", "82%", "83%", "84%", "85%", "86%", "87%", "88%", "89%", "90%", "91%",
        "92%", "93%", "94%", "95%", "96%", "97%", "98%", "99%", "100%",
    ];
    PERCENT[percent.clamp(0, 100) as usize]
}

#[cfg(test)]
mod percent_tests {
    use super::percent_text;

    #[test]
    fn the_static_table_matches_formatting_for_every_value_and_clamps_outside() {
        for i in 0..=100 {
            assert_eq!(percent_text(i), format!("{i}%"));
        }
        assert_eq!(percent_text(-5), "0%");
        assert_eq!(percent_text(101), "100%");
        assert_eq!(percent_text(i32::MAX), "100%");
        assert_eq!(percent_text(i32::MIN), "0%");
    }
}
