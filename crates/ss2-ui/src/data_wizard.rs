//! The first-run **data-location wizard** (roadmap T10-019, `docs/FIRST_RUN.md`): where is Serious Sam 2?
//!
//! A state machine like the menu: it takes [`WizardInput`], gives back [`WizardCommand`]s for the app, and draws into a
//! [`DrawList`] in canvas units with the placeholder theme. It knows nothing of the disk: the app does the searching and the
//! checking (`ss2_vfs::discover`, `ss2_vfs::check_install`) and hands the answers back with [`DataWizard::set_candidates`] and
//! [`DataWizard::set_report`].
//!
//! ```text
//!   Searching --found--> Candidates --pick--> Validating --report--> Result --Use this folder--> Done
//!                           |  ^                                       |
//!                  "Enter a path"  Back                              Back (to where it came from)
//!                           v  |
//!                         Manual --Check--> Validating
//! ```

use std::borrow::Cow;
use std::path::PathBuf;

use ss2_core::input::NavInput;

use crate::draw::{Color, DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{CANVAS_W, Rect, Vec2};
use crate::menu::UiInput;
use crate::theme;

/// The longest folder path the text field takes, in characters.
pub const MAX_PATH_CHARS: usize = 1024;
/// How many characters of the text field are drawn at once; a longer path scrolls with the cursor.
pub const FIELD_VISIBLE_CHARS: usize = 60;
/// How many candidate rows are drawn at once; more are reached by scrolling.
pub const VISIBLE_CANDIDATES: usize = 5;
/// The width, in characters, at which the lines of a report are wrapped.
pub const WRAP_CHARS: usize = 88;
/// How many wrapped lines of a report are drawn.
pub const MAX_REPORT_LINES: usize = 9;
/// The longest path shown on one row, in characters; a longer one loses its middle.
pub const MAX_SHOWN_PATH_CHARS: usize = 70;

const GOOD: Color = Color([0.55, 0.90, 0.55, 1.0]);
const WARN: Color = Color([0.95, 0.80, 0.35, 1.0]);
const BAD: Color = Color([1.00, 0.45, 0.40, 1.0]);
const DIM: Color = Color([0.75, 0.77, 0.82, 1.0]);

const ROW_W: f32 = 1500.0;
const ROW_H: f32 = 76.0;
const ROW_GAP: f32 = 10.0;
const LIST_TOP: f32 = 250.0;
const BUTTON_W: f32 = 640.0;
const BUTTON_H: f32 = 64.0;
const BUTTON_GAP: f32 = 12.0;
const FIELD_W: f32 = 1500.0;
const FIELD_H: f32 = 72.0;
const FIELD_TOP: f32 = 300.0;

// ---------------------------------------------------------------------------------------------------------------------------------
// Text field

/// A single-line text field: the characters, a cursor between them, and the rules of editing. The cursor counts characters, not bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextField {
    text: String,
    cursor: usize,
    max_chars: usize,
}

impl TextField {
    /// An empty field that takes at most `max_chars` characters.
    pub fn new(max_chars: usize) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            max_chars,
        }
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The cursor, in characters from the start (0 to the length).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The number of characters.
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Whether the field is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Type or paste text at the cursor. Control characters (new lines and tabs included) are left out, and what does not fit is cut off.
    pub fn insert_str(&mut self, text: &str) {
        for ch in text.chars().filter(|c| !c.is_control()) {
            if self.len() >= self.max_chars {
                break;
            }
            let at = self.byte_at(self.cursor);
            self.text.insert(at, ch);
            self.cursor += 1;
        }
    }

    /// Remove the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let at = self.byte_at(self.cursor - 1);
            self.text.remove(at);
            self.cursor -= 1;
        }
    }

    /// Remove the character at the cursor.
    pub fn delete(&mut self) {
        if self.cursor < self.len() {
            let at = self.byte_at(self.cursor);
            self.text.remove(at);
        }
    }

    /// Move the cursor one character left.
    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the cursor one character right.
    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    /// Move the cursor to the start.
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// Move the cursor to the end.
    pub fn end(&mut self) {
        self.cursor = self.len();
    }

    /// Replace the text (cut to the limit, control characters left out) with the cursor at the end.
    pub fn set(&mut self, text: &str) {
        self.text.clear();
        self.cursor = 0;
        self.insert_str(text);
    }
}

/// What the field shows: at most [`FIELD_VISIBLE_CHARS`] characters around the cursor, `...` where the text goes on, and `|` at the cursor when `show_cursor`.
pub fn field_view(text: &str, cursor: usize, show_cursor: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let cursor = cursor.min(len);
    let start = cursor
        .saturating_sub(FIELD_VISIBLE_CHARS * 2 / 3)
        .min(len.saturating_sub(FIELD_VISIBLE_CHARS));
    let end = (start + FIELD_VISIBLE_CHARS).min(len);
    let mut out = String::new();
    if start > 0 {
        out.push_str("...");
    }
    for (i, c) in chars[start..end].iter().enumerate() {
        if show_cursor && start + i == cursor {
            out.push('|');
        }
        out.push(*c);
    }
    if show_cursor && cursor == end {
        out.push('|');
    }
    if end < len {
        out.push_str("...");
    }
    out
}

/// `text` cut to `max` characters by taking out the middle (`C:\Program Files...\Serious Sam 2`).
pub fn shorten_middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    let keep = max.saturating_sub(3);
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push_str("...");
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// `text` broken into lines of at most `width` characters at spaces; a word longer than a line is split. Continuation lines are
/// indented like the first line (and by three more after an `->` arrow).
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let lead = text.chars().take_while(|c| *c == ' ').count();
    let body = &text[lead..];
    let hang = lead + if body.starts_with("->") { 3 } else { 0 };
    let mut lines: Vec<String> = Vec::new();
    let mut line = " ".repeat(lead);
    let mut line_has_word = false;
    for word in body.split(' ').filter(|w| !w.is_empty()) {
        let mut word: Vec<char> = word.chars().collect();
        loop {
            let used = line.chars().count();
            let gap = usize::from(line_has_word);
            if used + gap + word.len() <= width {
                if line_has_word {
                    line.push(' ');
                }
                line.extend(&word);
                line_has_word = true;
                break;
            }
            if line_has_word {
                lines.push(std::mem::replace(&mut line, " ".repeat(hang)));
                line_has_word = false;
                continue;
            }
            // A word that does not fit even on an empty line: take what fits.
            let room = width.saturating_sub(used).max(1);
            let take = room.min(word.len());
            line.extend(&word[..take]);
            word.drain(..take);
            lines.push(std::mem::replace(&mut line, " ".repeat(hang)));
            if word.is_empty() {
                break;
            }
        }
    }
    if line_has_word || lines.is_empty() {
        lines.push(line);
    }
    lines
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The wizard

/// One folder that looks like an install, as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardCandidate {
    /// The folder.
    pub dir: PathBuf,
    /// A line under it: where it came from and its patch level.
    pub detail: String,
}

/// The result of checking a folder, as the screen shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardReport {
    /// The folder that was checked.
    pub dir: PathBuf,
    /// The install can be used (it has no error).
    pub usable: bool,
    /// How many warnings there are.
    pub warnings: usize,
    /// The verdict in one line.
    pub headline: String,
    /// The details, one per line (long lines are wrapped when drawn).
    pub lines: Vec<String>,
}

/// An input event. The navigation and the pointer are the menu's [`UiInput`]; typing needs these as well.
#[derive(Clone, Debug, PartialEq)]
pub enum WizardInput {
    /// Navigation or the pointer.
    Ui(UiInput),
    /// What the keyboard typed, or a paste.
    Text(String),
    /// The backspace key.
    Backspace,
    /// The delete key.
    Delete,
    /// The home key.
    Home,
    /// The end key.
    End,
}

/// What the app must do after an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WizardCommand {
    /// Look for installs and answer with [`DataWizard::set_candidates`].
    Search,
    /// Check this folder and answer with [`DataWizard::set_report`].
    Validate(PathBuf),
    /// The player chose this folder and it was checked: remember it and go on.
    Accept(PathBuf),
    /// Leave the application.
    Quit,
}

/// Which screen of the wizard shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WizardView {
    /// Looking for installs (waiting for the app).
    Searching,
    /// The list of what was found.
    Candidates,
    /// Typing a folder path.
    Manual,
    /// Checking a folder (waiting for the app).
    Validating,
    /// The result of the check.
    Result,
    /// A folder was accepted.
    Done,
}

const MANUAL_FIELD: usize = 0;
const MANUAL_CHECK: usize = 1;
const MANUAL_BACK: usize = 2;
const ACTION_ROWS: [&str; 3] = ["Enter a folder path", "Search again", "Quit"];

/// The first-run wizard. See the module documentation.
#[derive(Clone, Debug)]
pub struct DataWizard {
    view: WizardView,
    message: Option<String>,
    notice: Option<String>,
    candidates: Vec<WizardCandidate>,
    scroll: usize,
    field: TextField,
    report: Option<WizardReport>,
    pending_dir: Option<PathBuf>,
    return_to: WizardView,
    focus: usize,
    pressed: Option<usize>,
}

impl DataWizard {
    /// A wizard about to search. `message` is a sentence for the top of the screen (why the wizard is shown).
    pub fn new(message: Option<String>) -> Self {
        Self {
            view: WizardView::Searching,
            message,
            notice: None,
            candidates: Vec::new(),
            scroll: 0,
            field: TextField::new(MAX_PATH_CHARS),
            report: None,
            pending_dir: None,
            return_to: WizardView::Candidates,
            focus: 0,
            pressed: None,
        }
    }

    /// Begin (or begin again): shows the searching screen and asks the app to search.
    pub fn start(&mut self) -> WizardCommand {
        self.view = WizardView::Searching;
        self.notice = None;
        self.pressed = None;
        WizardCommand::Search
    }

    /// The screen that shows.
    pub fn view(&self) -> WizardView {
        self.view
    }

    /// Whether a folder was accepted.
    pub fn is_finished(&self) -> bool {
        self.view == WizardView::Done
    }

    /// The folders found.
    pub fn candidates(&self) -> &[WizardCandidate] {
        &self.candidates
    }

    /// The text field of the manual screen.
    pub fn field(&self) -> &TextField {
        &self.field
    }

    /// The focused row (of the list, of the manual screen or of the result buttons).
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// The report being shown.
    pub fn report(&self) -> Option<&WizardReport> {
        self.report.as_ref()
    }

    /// The notice under the message, if there is one.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Show a notice (for example that the choice could not be saved). It goes away with the next input that changes something.
    pub fn set_notice(&mut self, text: impl Into<String>) {
        self.notice = Some(text.into());
    }

    /// The answer to [`WizardCommand::Search`]. Ignored unless the wizard is searching. An empty list is a valid answer: the screen then offers to type the path.
    pub fn set_candidates(&mut self, candidates: Vec<WizardCandidate>) {
        if self.view != WizardView::Searching {
            return;
        }
        self.candidates = candidates;
        self.view = WizardView::Candidates;
        self.scroll = 0;
        self.focus = 0;
        self.pressed = None;
    }

    /// The answer to [`WizardCommand::Validate`]. Ignored unless the wizard is validating.
    pub fn set_report(&mut self, report: WizardReport) {
        if self.view != WizardView::Validating {
            return;
        }
        self.focus = 0;
        self.pressed = None;
        self.report = Some(report);
        self.view = WizardView::Result;
    }

    fn rows(&self) -> usize {
        match self.view {
            WizardView::Candidates => self.candidates.len() + ACTION_ROWS.len(),
            WizardView::Manual => 3,
            WizardView::Result => self.result_buttons().len(),
            _ => 0,
        }
    }

    fn result_buttons(&self) -> &'static [&'static str] {
        match &self.report {
            Some(r) if r.usable => &["Use this folder", "Choose another folder", "Quit"],
            _ => &["Choose another folder", "Quit"],
        }
    }

    fn quit_row(&self) -> usize {
        self.rows().saturating_sub(1)
    }

    fn move_focus(&mut self, delta: isize) {
        let rows = self.rows();
        if rows == 0 {
            return;
        }
        self.focus = (self.focus as isize + delta).rem_euclid(rows as isize) as usize;
        self.follow_focus();
    }

    fn follow_focus(&mut self) {
        if self.view != WizardView::Candidates || self.focus >= self.candidates.len() {
            return;
        }
        if self.focus < self.scroll {
            self.scroll = self.focus;
        } else if self.focus >= self.scroll + VISIBLE_CANDIDATES {
            self.scroll = self.focus + 1 - VISIBLE_CANDIDATES;
        }
    }

    /// The path in the text field the way it is checked: surrounding spaces and one pair of surrounding double quotes (what "Copy as path" adds) removed.
    pub fn typed_path(&self) -> Option<PathBuf> {
        let text = self.field.text().trim();
        let text = text
            .strip_prefix('"')
            .and_then(|t| t.strip_suffix('"'))
            .unwrap_or(text)
            .trim();
        (!text.is_empty()).then(|| PathBuf::from(text))
    }

    fn validate(&mut self, dir: PathBuf, from: WizardView) -> Option<WizardCommand> {
        self.view = WizardView::Validating;
        self.return_to = from;
        self.notice = None;
        self.pressed = None;
        self.report = None;
        self.pending_dir = Some(dir.clone());
        Some(WizardCommand::Validate(dir))
    }

    fn submit_typed(&mut self) -> Option<WizardCommand> {
        match self.typed_path() {
            Some(dir) => self.validate(dir, WizardView::Manual),
            None => {
                self.notice = Some("Type or paste the folder path first".to_owned());
                None
            }
        }
    }

    fn go(&mut self, view: WizardView) {
        self.view = view;
        self.focus = 0;
        self.pressed = None;
        self.notice = None;
        if view == WizardView::Manual {
            self.field.end();
        }
    }

    /// What activating row `row` does.
    fn activate(&mut self, row: usize) -> Option<WizardCommand> {
        match self.view {
            WizardView::Candidates => {
                let n = self.candidates.len();
                if row < n {
                    let dir = self.candidates[row].dir.clone();
                    return self.validate(dir, WizardView::Candidates);
                }
                match row - n {
                    0 => {
                        self.go(WizardView::Manual);
                        None
                    }
                    1 => Some(self.start()),
                    _ => Some(WizardCommand::Quit),
                }
            }
            WizardView::Manual => match row {
                MANUAL_FIELD | MANUAL_CHECK => self.submit_typed(),
                _ => {
                    self.go(WizardView::Candidates);
                    None
                }
            },
            WizardView::Result => {
                let usable = self.report.as_ref().is_some_and(|r| r.usable);
                let first = usize::from(usable);
                if usable && row == 0 {
                    let dir = self.report.as_ref().map(|r| r.dir.clone())?;
                    self.view = WizardView::Done;
                    return Some(WizardCommand::Accept(dir));
                }
                if row == first {
                    let back = self.return_to;
                    self.go(back);
                    None
                } else {
                    Some(WizardCommand::Quit)
                }
            }
            _ => None,
        }
    }

    fn back(&mut self) -> Option<WizardCommand> {
        match self.view {
            WizardView::Candidates => {
                // The first Back only points at Quit, so that Escape cannot close the wizard by accident.
                if self.focus == self.quit_row() {
                    Some(WizardCommand::Quit)
                } else {
                    self.focus = self.quit_row();
                    self.follow_focus();
                    None
                }
            }
            WizardView::Manual => {
                self.go(WizardView::Candidates);
                None
            }
            WizardView::Result => {
                let back = self.return_to;
                self.go(back);
                None
            }
            _ => None,
        }
    }

    /// Feed one event.
    pub fn handle(&mut self, input: WizardInput) -> Option<WizardCommand> {
        if matches!(
            self.view,
            WizardView::Searching | WizardView::Validating | WizardView::Done
        ) {
            return None;
        }
        match input {
            WizardInput::Ui(ui) => self.handle_ui(ui),
            WizardInput::Text(text) => {
                self.edit(|f| f.insert_str(&text), true);
                None
            }
            WizardInput::Backspace => {
                self.edit(TextField::backspace, false);
                None
            }
            WizardInput::Delete => {
                self.edit(TextField::delete, false);
                None
            }
            WizardInput::Home => {
                self.edit(TextField::home, false);
                None
            }
            WizardInput::End => {
                self.edit(TextField::end, false);
                None
            }
        }
    }

    /// Edit the field. Typing (`takes_focus`) moves the focus to the field first, the other keys only act when it already has it.
    fn edit(&mut self, change: impl FnOnce(&mut TextField), takes_focus: bool) {
        if self.view != WizardView::Manual {
            return;
        }
        if self.focus != MANUAL_FIELD {
            if !takes_focus {
                return;
            }
            self.focus = MANUAL_FIELD;
        }
        change(&mut self.field);
        self.notice = None;
    }

    fn handle_ui(&mut self, input: UiInput) -> Option<WizardCommand> {
        match input {
            UiInput::Nav(NavInput::Up) => {
                self.move_focus(-1);
                None
            }
            UiInput::Nav(NavInput::Down) => {
                self.move_focus(1);
                None
            }
            UiInput::Nav(NavInput::Left) => {
                if self.view == WizardView::Manual && self.focus == MANUAL_FIELD {
                    self.field.left();
                }
                None
            }
            UiInput::Nav(NavInput::Right) => {
                if self.view == WizardView::Manual && self.focus == MANUAL_FIELD {
                    self.field.right();
                }
                None
            }
            UiInput::Nav(NavInput::Activate) => self.activate(self.focus),
            UiInput::Nav(NavInput::Back) => self.back(),
            UiInput::PointerMoved(p) => {
                if let Some(row) = self.hit(p) {
                    self.focus = row;
                }
                None
            }
            UiInput::PointerPressed(p) => {
                self.pressed = self.hit(p);
                if let Some(row) = self.pressed {
                    self.focus = row;
                }
                None
            }
            UiInput::PointerReleased(p) => match (self.pressed.take(), self.hit(p)) {
                // Pressing on the field only focuses it; every other row is a button.
                (Some(a), Some(b))
                    if a == b && !(self.view == WizardView::Manual && a == MANUAL_FIELD) =>
                {
                    self.activate(a)
                }
                _ => None,
            },
        }
    }

    // -----------------------------------------------------------------------------------------------------------------------------
    // Layout

    /// The rows that can be pointed at on this screen, as (row, rectangle).
    pub fn row_rects(&self) -> Vec<(usize, Rect)> {
        let x = (CANVAS_W - ROW_W) / 2.0;
        match self.view {
            WizardView::Candidates => {
                let n = self.candidates.len();
                let shown = n.saturating_sub(self.scroll).min(VISIBLE_CANDIDATES);
                // With nothing found, one slot says so.
                let slots = shown.max(1);
                let mut out = Vec::new();
                for slot in 0..shown {
                    out.push((self.scroll + slot, Self::slot_rect(x, slot)));
                }
                for (k, _) in ACTION_ROWS.iter().enumerate() {
                    out.push((n + k, Self::slot_rect(x, slots + k)));
                }
                out
            }
            WizardView::Manual => {
                let bx = (CANVAS_W - BUTTON_W) / 2.0;
                vec![
                    (
                        MANUAL_FIELD,
                        Rect {
                            x: (CANVAS_W - FIELD_W) / 2.0,
                            y: FIELD_TOP,
                            w: FIELD_W,
                            h: FIELD_H,
                        },
                    ),
                    (MANUAL_CHECK, Self::button_rect(bx, 470.0, 0)),
                    (MANUAL_BACK, Self::button_rect(bx, 470.0, 1)),
                ]
            }
            WizardView::Result => {
                let bx = (CANVAS_W - BUTTON_W) / 2.0;
                (0..self.result_buttons().len())
                    .map(|i| (i, Self::button_rect(bx, 700.0, i)))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn slot_rect(x: f32, slot: usize) -> Rect {
        Rect {
            x,
            y: LIST_TOP + slot as f32 * (ROW_H + ROW_GAP),
            w: ROW_W,
            h: ROW_H,
        }
    }

    fn button_rect(x: f32, top: f32, i: usize) -> Rect {
        Rect {
            x,
            y: top + i as f32 * (BUTTON_H + BUTTON_GAP),
            w: BUTTON_W,
            h: BUTTON_H,
        }
    }

    fn hit(&self, p: Vec2) -> Option<usize> {
        self.row_rects()
            .into_iter()
            .find(|(_, r)| r.contains(p))
            .map(|(row, _)| row)
    }

    // -----------------------------------------------------------------------------------------------------------------------------
    // Drawing

    fn text(
        list: &mut DrawList,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        align: TextAlign,
        text: impl Into<Cow<'static, str>>,
    ) {
        list.texts.push(TextRun {
            pos: Vec2 { x, y },
            text: text.into(),
            size,
            color,
            align,
        });
    }

    fn centered(
        list: &mut DrawList,
        y: f32,
        size: f32,
        color: Color,
        text: impl Into<Cow<'static, str>>,
    ) {
        Self::text(
            list,
            CANVAS_W / 2.0,
            y,
            size,
            color,
            TextAlign::Center,
            text,
        );
    }

    fn button(&self, list: &mut DrawList, row: usize, rect: Rect, label: &'static str) {
        list.quads.push(Quad {
            rect,
            color: if row == self.focus {
                theme::FOCUSED
            } else {
                theme::IDLE
            },
        });
        Self::centered(list, rect.y + 13.0, 34.0, theme::TEXT, label);
    }

    /// Replace the contents of `list` with the screen. (The paths and lines are built at run time, so this allocates a few short strings.)
    pub fn draw_into(&self, list: &mut DrawList) {
        list.clear();
        Self::centered(list, 60.0, 72.0, theme::TEXT, "Find Serious Sam 2");
        let line = self.notice.as_deref().or(self.message.as_deref());
        if let Some(text) = line {
            let color = if self.notice.is_some() { WARN } else { DIM };
            Self::centered(list, 160.0, 28.0, color, shorten_middle(text, 120));
        }
        match self.view {
            WizardView::Searching => {
                Self::centered(
                    list,
                    420.0,
                    40.0,
                    theme::TEXT,
                    "Looking for your Serious Sam 2 install...",
                );
                Self::centered(list, 480.0, 28.0, DIM, "This takes a moment.");
            }
            WizardView::Candidates => self.draw_candidates(list),
            WizardView::Manual => self.draw_manual(list),
            WizardView::Validating => {
                if let Some(dir) = self.validating_dir() {
                    Self::centered(
                        list,
                        400.0,
                        36.0,
                        DIM,
                        shorten_middle(&dir, MAX_SHOWN_PATH_CHARS),
                    );
                }
                Self::centered(list, 460.0, 40.0, theme::TEXT, "Checking the folder...");
                Self::centered(
                    list,
                    520.0,
                    28.0,
                    DIM,
                    "It reads every archive, which takes a moment.",
                );
            }
            WizardView::Result => self.draw_result(list),
            WizardView::Done => {
                if let Some(r) = &self.report {
                    Self::centered(
                        list,
                        400.0,
                        36.0,
                        DIM,
                        shorten_middle(&r.dir.display().to_string(), MAX_SHOWN_PATH_CHARS),
                    );
                }
                Self::centered(list, 460.0, 44.0, GOOD, "Saved. Starting...");
            }
        }
    }

    /// The folder being checked, while the wizard waits for the answer.
    fn validating_dir(&self) -> Option<String> {
        self.pending_dir.as_ref().map(|d| d.display().to_string())
    }

    fn draw_candidates(&self, list: &mut DrawList) {
        let n = self.candidates.len();
        if n == 0 {
            Self::centered(list, 212.0, 28.0, DIM, "No install was found on its own.");
        } else {
            let last = (self.scroll + VISIBLE_CANDIDATES).min(n);
            let text = if n > VISIBLE_CANDIDATES {
                format!(
                    "Found {n} folders (showing {}-{last}). Choose one:",
                    self.scroll + 1
                )
            } else if n == 1 {
                "Found one folder. Choose it, or enter another:".to_owned()
            } else {
                format!("Found {n} folders. Choose one:")
            };
            Self::centered(list, 212.0, 28.0, DIM, text);
        }
        let x = (CANVAS_W - ROW_W) / 2.0;
        if n == 0 {
            let r = Self::slot_rect(x, 0);
            list.quads.push(Quad {
                rect: r,
                color: theme::TRACK,
            });
            Self::centered(
                list,
                r.y + 20.0,
                32.0,
                DIM,
                "Nothing found. Enter the folder that holds All_PC_01.gro.",
            );
        }
        for (row, rect) in self.row_rects() {
            let focused = row == self.focus;
            list.quads.push(Quad {
                rect,
                color: if focused { theme::FOCUSED } else { theme::IDLE },
            });
            if row < n {
                let c = &self.candidates[row];
                Self::text(
                    list,
                    rect.x + 24.0,
                    rect.y + 6.0,
                    30.0,
                    theme::TEXT,
                    TextAlign::Left,
                    shorten_middle(&c.dir.display().to_string(), MAX_SHOWN_PATH_CHARS),
                );
                Self::text(
                    list,
                    rect.x + 24.0,
                    rect.y + 42.0,
                    24.0,
                    if focused { theme::TEXT } else { DIM },
                    TextAlign::Left,
                    shorten_middle(&c.detail, 100),
                );
            } else {
                Self::centered(list, rect.y + 16.0, 34.0, theme::TEXT, ACTION_ROWS[row - n]);
            }
        }
    }

    fn draw_manual(&self, list: &mut DrawList) {
        Self::text(
            list,
            210.0,
            250.0,
            30.0,
            theme::TEXT,
            TextAlign::Left,
            "The folder that contains All_PC_01.gro",
        );
        let rects = self.row_rects();
        let field = rects[MANUAL_FIELD].1;
        let focused = self.focus == MANUAL_FIELD;
        if focused {
            list.quads.push(Quad {
                rect: Rect {
                    x: field.x - 4.0,
                    y: field.y - 4.0,
                    w: field.w + 8.0,
                    h: field.h + 8.0,
                },
                color: theme::FOCUSED,
            });
        }
        list.quads.push(Quad {
            rect: field,
            color: theme::TRACK,
        });
        Self::text(
            list,
            field.x + 20.0,
            field.y + 17.0,
            34.0,
            theme::TEXT,
            TextAlign::Left,
            field_view(self.field.text(), self.field.cursor(), focused),
        );
        Self::text(
            list,
            210.0,
            392.0,
            24.0,
            DIM,
            TextAlign::Left,
            "Paste a path or type it. Enter checks it, Escape goes back.",
        );
        self.button(
            list,
            MANUAL_CHECK,
            rects[MANUAL_CHECK].1,
            "Check this folder",
        );
        self.button(list, MANUAL_BACK, rects[MANUAL_BACK].1, "Back");
    }

    fn draw_result(&self, list: &mut DrawList) {
        let Some(report) = &self.report else {
            return;
        };
        Self::centered(
            list,
            170.0,
            28.0,
            DIM,
            shorten_middle(&report.dir.display().to_string(), MAX_SHOWN_PATH_CHARS),
        );
        let color = if !report.usable {
            BAD
        } else if report.warnings > 0 {
            WARN
        } else {
            GOOD
        };
        Self::centered(
            list,
            212.0,
            44.0,
            color,
            shorten_middle(&report.headline, 70),
        );
        let mut lines: Vec<String> = report
            .lines
            .iter()
            .flat_map(|l| wrap(l, WRAP_CHARS))
            .collect();
        if lines.len() > MAX_REPORT_LINES {
            let more = lines.len() - (MAX_REPORT_LINES - 1);
            lines.truncate(MAX_REPORT_LINES - 1);
            lines.push(format!(
                "... and {more} more lines (ss2-doctor prints all of them)"
            ));
        }
        for (i, line) in lines.into_iter().enumerate() {
            let tint = if line.trim_start().starts_with("ERROR") {
                BAD
            } else if line.trim_start().starts_with("WARNING") {
                WARN
            } else {
                theme::TEXT
            };
            Self::text(
                list,
                210.0,
                285.0 + i as f32 * 40.0,
                28.0,
                tint,
                TextAlign::Left,
                line,
            );
        }
        let labels = self.result_buttons();
        for (row, rect) in self.row_rects() {
            self.button(list, row, rect, labels[row]);
        }
    }

    /// The screen as a new list.
    pub fn draw(&self) -> DrawList {
        let mut list = DrawList::default();
        self.draw_into(&mut list);
        list
    }
}
