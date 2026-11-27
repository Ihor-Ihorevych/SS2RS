//! The level picker: a scrolling list of levels grouped under headers, driven by [`UiInput`]
//! and drawn into a [`DrawList`] on the placeholder skin (slice of T10-004 for the playable MVP).
//!
//! The app fills it from the install's levels and acts on the [`PickerCommand`] it returns.
//!
//! ```
//! use ss2_core::input::NavInput;
//! use ss2_ui::level_select::{LevelPicker, PickerCommand, PickerEntry};
//! use ss2_ui::UiInput;
//!
//! let mut picker = LevelPicker::new(vec![
//!     PickerEntry::new("1_1_jungle", "01_mdigbo", "Jungle"),
//!     PickerEntry::new("1_2_village", "01_mdigbo", "Village"),
//! ]);
//! picker.handle(UiInput::Nav(NavInput::Down));
//! assert_eq!(
//!     picker.handle(UiInput::Nav(NavInput::Activate)),
//!     Some(PickerCommand::Start("1_2_village".to_string()))
//! );
//! ```

use std::borrow::Cow;

use ss2_core::input::NavInput;

use crate::draw::{Color, DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{CANVAS_W, Rect, Vec2};
use crate::menu::UiInput;
use crate::theme;

/// Width of the list.
pub const LIST_W: f32 = 1100.0;
/// Top of the first visible row.
pub const LIST_TOP: f32 = 200.0;
/// Height of one row.
pub const ROW_H: f32 = 48.0;
/// Gap between rows.
pub const ROW_GAP: f32 = 8.0;
/// How many rows fit on screen at once.
pub const VISIBLE_ROWS: usize = 13;

const HEADER: Color = Color([0.65, 0.65, 0.70, 1.0]);
const HINT: Color = Color([0.55, 0.55, 0.60, 1.0]);

/// One level the player can pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerEntry {
    /// What [`PickerCommand::Start`] returns.
    pub id: String,
    /// Header the level is listed under; consecutive entries with the same group share one.
    pub group: String,
    /// What the row shows; the id when empty.
    pub label: String,
}

impl PickerEntry {
    /// An entry.
    pub fn new(id: &str, group: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            group: group.to_string(),
            label: label.to_string(),
        }
    }
}

/// What the app must do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerCommand {
    /// Load the level with this id.
    Start(String),
    /// Close the picker.
    Back,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Row {
    Header(String),
    Level(usize),
}

/// The picker's state.
#[derive(Clone, Debug)]
pub struct LevelPicker {
    entries: Vec<PickerEntry>,
    rows: Vec<Row>,
    /// Index into `rows` of the focused level row; `None` without levels.
    focus: Option<usize>,
    /// First visible row.
    scroll: usize,
    /// Row under a pointer press, if any.
    pressed: Option<usize>,
}

impl LevelPicker {
    /// A picker over `entries`, in the given order, with the first level focused.
    pub fn new(entries: Vec<PickerEntry>) -> Self {
        let mut rows = Vec::with_capacity(entries.len() + 8);
        let mut group: Option<&str> = None;
        for (i, e) in entries.iter().enumerate() {
            if group != Some(e.group.as_str()) {
                rows.push(Row::Header(e.group.clone()));
                group = Some(e.group.as_str());
            }
            rows.push(Row::Level(i));
        }
        let focus = rows.iter().position(|r| matches!(r, Row::Level(_)));
        let mut picker = Self {
            entries,
            rows,
            focus,
            scroll: 0,
            pressed: None,
        };
        picker.keep_focus_visible();
        picker
    }

    /// The focused level's id.
    pub fn focused(&self) -> Option<&str> {
        match self.rows.get(self.focus?)? {
            Row::Level(i) => Some(self.entries[*i].id.as_str()),
            Row::Header(_) => None,
        }
    }

    /// Focus the level `id` (for example the one played last) and scroll to it; `false` if
    /// there is no such level.
    pub fn select(&mut self, id: &str) -> bool {
        let found = self
            .rows
            .iter()
            .position(|r| matches!(r, Row::Level(i) if self.entries[*i].id == id));
        if found.is_some() {
            self.focus = found;
            self.keep_focus_visible();
        }
        found.is_some()
    }

    /// The first visible row.
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// React to one input.
    pub fn handle(&mut self, input: UiInput) -> Option<PickerCommand> {
        match input {
            UiInput::Nav(NavInput::Back) => return Some(PickerCommand::Back),
            UiInput::Nav(NavInput::Activate) => return self.start_focused(),
            UiInput::Nav(NavInput::Down) => self.step(1, true),
            UiInput::Nav(NavInput::Up) => self.step(-1, true),
            UiInput::Nav(NavInput::Right) => self.page(1),
            UiInput::Nav(NavInput::Left) => self.page(-1),
            UiInput::PointerMoved(p) => {
                if let Some(row) = self.level_row_at(p) {
                    self.focus = Some(row);
                }
            }
            UiInput::PointerPressed(p) => {
                self.pressed = self.level_row_at(p);
                if self.pressed.is_some() {
                    self.focus = self.pressed;
                }
            }
            UiInput::PointerReleased(p) => {
                let pressed = self.pressed.take();
                if pressed.is_some() && pressed == self.level_row_at(p) {
                    return self.start_focused();
                }
            }
        }
        None
    }

    /// Append this frame's quads and texts to `out`.
    pub fn draw(&self, out: &mut DrawList) {
        let x = (CANVAS_W - LIST_W) / 2.0;
        out.texts.push(TextRun {
            pos: Vec2 {
                x: CANVAS_W / 2.0,
                y: 100.0,
            },
            text: Cow::Borrowed("Choose a level"),
            size: 56.0,
            color: theme::TEXT,
            align: TextAlign::Center,
        });
        if self.focus.is_none() {
            out.texts.push(TextRun {
                pos: Vec2 {
                    x: CANVAS_W / 2.0,
                    y: LIST_TOP + ROW_H,
                },
                text: Cow::Borrowed("No levels found in the game data"),
                size: 36.0,
                color: HINT,
                align: TextAlign::Center,
            });
        }
        for (slot, row) in self
            .rows
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(VISIBLE_ROWS)
        {
            let rect = row_rect(slot - self.scroll);
            match row {
                Row::Header(name) => out.texts.push(TextRun {
                    pos: Vec2 {
                        x: rect.x,
                        y: rect.y + 10.0,
                    },
                    text: Cow::Owned(name.clone()),
                    size: 30.0,
                    color: HEADER,
                    align: TextAlign::Left,
                }),
                Row::Level(i) => {
                    let focused = self.focus == Some(slot);
                    out.quads.push(Quad {
                        rect,
                        color: if focused { theme::FOCUSED } else { theme::IDLE },
                    });
                    let e = &self.entries[*i];
                    let label = if e.label.is_empty() { &e.id } else { &e.label };
                    out.texts.push(TextRun {
                        pos: Vec2 {
                            x: rect.x + 24.0,
                            y: rect.y + 8.0,
                        },
                        text: Cow::Owned(label.clone()),
                        size: 32.0,
                        color: theme::TEXT,
                        align: TextAlign::Left,
                    });
                }
            }
        }
        // A scroll bar when the list does not fit.
        if self.rows.len() > VISIBLE_ROWS {
            let track = Rect {
                x: x + LIST_W + 16.0,
                y: LIST_TOP,
                w: 8.0,
                h: list_height(),
            };
            let n = self.rows.len() as f32;
            out.quads.push(Quad {
                rect: track,
                color: theme::TRACK,
            });
            out.quads.push(Quad {
                rect: Rect {
                    y: track.y + track.h * self.scroll as f32 / n,
                    h: track.h * VISIBLE_ROWS as f32 / n,
                    ..track
                },
                color: theme::TRACK_FILL,
            });
        }
        out.texts.push(TextRun {
            pos: Vec2 {
                x: CANVAS_W / 2.0,
                y: LIST_TOP + list_height() + 24.0,
            },
            text: Cow::Borrowed("Enter: play    Left/Right: page    Esc: back"),
            size: 28.0,
            color: HINT,
            align: TextAlign::Center,
        });
    }

    fn start_focused(&self) -> Option<PickerCommand> {
        self.focused()
            .map(|id| PickerCommand::Start(id.to_string()))
    }

    /// Move to the next level row in direction `dir`, wrapping around if `wrap`.
    fn step(&mut self, dir: isize, wrap: bool) {
        let Some(start) = self.focus else { return };
        let n = self.rows.len() as isize;
        let mut i = start as isize;
        loop {
            i += dir;
            if !(0..n).contains(&i) {
                if !wrap {
                    return;
                }
                i = i.rem_euclid(n);
            }
            if i as usize == start {
                return;
            }
            if matches!(self.rows[i as usize], Row::Level(_)) {
                self.focus = Some(i as usize);
                self.keep_focus_visible();
                return;
            }
        }
    }

    /// Move about one screen of levels in direction `dir`, stopping at the ends.
    fn page(&mut self, dir: isize) {
        for _ in 0..VISIBLE_ROWS - 1 {
            self.step(dir, false);
        }
    }

    /// Scroll so the focused row, and the header right above it, are on screen.
    fn keep_focus_visible(&mut self) {
        let Some(f) = self.focus else { return };
        let top = if f > 0 && matches!(self.rows[f - 1], Row::Header(_)) {
            f - 1
        } else {
            f
        };
        if top < self.scroll {
            self.scroll = top;
        } else if f >= self.scroll + VISIBLE_ROWS {
            self.scroll = f + 1 - VISIBLE_ROWS;
        }
    }

    /// The level row drawn under canvas point `p`.
    fn level_row_at(&self, p: Vec2) -> Option<usize> {
        (self.scroll..self.rows.len().min(self.scroll + VISIBLE_ROWS)).find(|&i| {
            matches!(self.rows[i], Row::Level(_)) && row_rect(i - self.scroll).contains(p)
        })
    }
}

/// Where visible slot `slot` is drawn.
fn row_rect(slot: usize) -> Rect {
    Rect {
        x: (CANVAS_W - LIST_W) / 2.0,
        y: LIST_TOP + slot as f32 * (ROW_H + ROW_GAP),
        w: LIST_W,
        h: ROW_H,
    }
}

/// Height of all visible slots.
fn list_height() -> f32 {
    VISIBLE_ROWS as f32 * (ROW_H + ROW_GAP) - ROW_GAP
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(n: usize) -> Vec<PickerEntry> {
        (0..n)
            .map(|i| {
                let group = format!("{:02}_episode", i / 5 + 1);
                PickerEntry::new(&format!("level_{i:02}"), &group, &format!("Level {i}"))
            })
            .collect()
    }

    fn nav(p: &mut LevelPicker, n: NavInput) -> Option<PickerCommand> {
        p.handle(UiInput::Nav(n))
    }

    fn drawn(p: &LevelPicker) -> DrawList {
        let mut out = DrawList::default();
        p.draw(&mut out);
        out
    }

    fn texts(d: &DrawList) -> Vec<String> {
        d.texts.iter().map(|t| t.text.to_string()).collect()
    }

    #[test]
    fn the_first_level_is_focused_and_activate_starts_it() {
        let mut p = LevelPicker::new(entries(3));
        assert_eq!(p.focused(), Some("level_00"));
        assert_eq!(
            nav(&mut p, NavInput::Activate),
            Some(PickerCommand::Start("level_00".to_string()))
        );
        assert_eq!(nav(&mut p, NavInput::Back), Some(PickerCommand::Back));
    }

    #[test]
    fn focus_moves_over_levels_skipping_headers_and_wraps() {
        let mut p = LevelPicker::new(entries(7)); // groups of 5 and 2
        for i in 1..7 {
            assert_eq!(nav(&mut p, NavInput::Down), None);
            assert_eq!(p.focused(), Some(format!("level_{i:02}").as_str()));
        }
        nav(&mut p, NavInput::Down);
        assert_eq!(p.focused(), Some("level_00"), "wraps to the first");
        nav(&mut p, NavInput::Up);
        assert_eq!(p.focused(), Some("level_06"), "wraps to the last");
    }

    #[test]
    fn headers_show_once_per_group_and_labels_fall_back_to_the_id() {
        let mut list = entries(7);
        list[1].label.clear();
        let p = LevelPicker::new(list);
        let t = texts(&drawn(&p));
        assert_eq!(t.iter().filter(|s| *s == "01_episode").count(), 1);
        assert_eq!(t.iter().filter(|s| *s == "02_episode").count(), 1);
        assert!(t.iter().any(|s| s == "level_01"), "{t:?}");
        assert!(t.iter().any(|s| s == "Level 0"));
    }

    #[test]
    fn a_long_list_scrolls_to_keep_the_focus_visible_and_draws_only_what_fits() {
        let mut p = LevelPicker::new(entries(40));
        for _ in 0..30 {
            nav(&mut p, NavInput::Down);
        }
        assert_eq!(p.focused(), Some("level_30"));
        assert!(p.scroll() > 0);
        let d = drawn(&p);
        assert!(texts(&d).iter().any(|s| s == "Level 30"));
        assert!(!texts(&d).iter().any(|s| s == "Level 0"), "scrolled away");
        let row_quads = d
            .quads
            .iter()
            .filter(|q| q.color == theme::IDLE || q.color == theme::FOCUSED)
            .count();
        assert!(row_quads <= VISIBLE_ROWS, "{row_quads}");
        assert_eq!(
            d.quads.iter().filter(|q| q.color == theme::FOCUSED).count(),
            1
        );
        // Back to the top: the first header is visible again.
        nav(&mut p, NavInput::Down);
        for _ in 0..9 {
            nav(&mut p, NavInput::Down);
        }
        assert_eq!(p.focused(), Some("level_00"));
        assert_eq!(p.scroll(), 0, "the header above the first level shows too");
    }

    #[test]
    fn left_and_right_move_a_page() {
        let mut p = LevelPicker::new(entries(40));
        nav(&mut p, NavInput::Right);
        let after = p.focused().unwrap().to_string();
        assert!(after.as_str() > "level_05", "{after}");
        nav(&mut p, NavInput::Left);
        assert_eq!(p.focused(), Some("level_00"));
        nav(&mut p, NavInput::Left);
        assert_eq!(p.focused(), Some("level_00"), "a page does not wrap");
    }

    #[test]
    fn select_focuses_a_level_by_id() {
        let mut p = LevelPicker::new(entries(40));
        assert!(p.select("level_35"));
        assert_eq!(p.focused(), Some("level_35"));
        assert!(texts(&drawn(&p)).iter().any(|s| s == "Level 35"));
        assert!(!p.select("nope"));
        assert_eq!(p.focused(), Some("level_35"));
    }

    #[test]
    fn the_pointer_focuses_and_a_click_starts_the_row_under_it() {
        let mut p = LevelPicker::new(entries(7));
        let d = drawn(&p);
        let rows: Vec<Rect> = d
            .quads
            .iter()
            .filter(|q| q.color == theme::IDLE || q.color == theme::FOCUSED)
            .map(|q| q.rect)
            .collect();
        let centre = |r: Rect| Vec2 {
            x: r.x + r.w / 2.0,
            y: r.y + r.h / 2.0,
        };
        let third = centre(rows[2]);
        assert_eq!(p.handle(UiInput::PointerMoved(third)), None);
        assert_eq!(p.focused(), Some("level_02"));
        assert_eq!(p.handle(UiInput::PointerPressed(third)), None);
        assert_eq!(
            p.handle(UiInput::PointerReleased(third)),
            Some(PickerCommand::Start("level_02".to_string()))
        );
        // Press on one row, release on another: nothing.
        p.handle(UiInput::PointerPressed(centre(rows[0])));
        assert_eq!(p.handle(UiInput::PointerReleased(centre(rows[1]))), None);
        // Outside every row: nothing changes.
        let before = p.focused().map(str::to_string);
        p.handle(UiInput::PointerMoved(Vec2 { x: 5.0, y: 5.0 }));
        assert_eq!(p.focused().map(str::to_string), before);
    }

    #[test]
    fn without_levels_only_back_works_and_the_reason_shows() {
        let mut p = LevelPicker::new(Vec::new());
        assert_eq!(p.focused(), None);
        assert_eq!(nav(&mut p, NavInput::Activate), None);
        assert_eq!(nav(&mut p, NavInput::Down), None);
        assert_eq!(nav(&mut p, NavInput::Back), Some(PickerCommand::Back));
        let t = texts(&drawn(&p));
        assert!(t.iter().any(|s| s.contains("No levels")), "{t:?}");
    }

    #[test]
    fn every_drawn_rect_is_on_the_canvas() {
        let mut p = LevelPicker::new(entries(40));
        for _ in 0..25 {
            nav(&mut p, NavInput::Down);
            for q in drawn(&p).quads {
                let r = q.rect;
                assert!(r.x >= 0.0 && r.y >= 0.0, "{r:?}");
                assert!(r.x + r.w <= CANVAS_W && r.y + r.h <= crate::geometry::CANVAS_H);
            }
        }
    }
}
