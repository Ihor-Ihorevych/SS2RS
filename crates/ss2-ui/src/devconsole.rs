//! The developer console and the performance overlays (roadmap T10-007): what is on screen when a
//! developer presses the console key, and the small text blocks that show frame time and network
//! figures.
//!
//! [`ConsoleView`] is the model of the drop-down console: the lines printed so far (bounded), the
//! input line with a cursor, the history and Tab completion of the backend (`ss2_core::console`),
//! scrolling, and drawing as a [`DrawList`] in canvas units. It owns no console: the caller passes
//! the [`Console`] to [`ConsoleView::handle_key`], so the same view works for a client and a server.
//! Keys arrive as [`ConsoleKey`] (the platform layer maps its key events) and typed characters as text.
//!
//! [`FrameStats`] keeps a window of frame times and answers "how many frames per second" and "how
//! long did frames take", and [`draw_overlay`] puts lines of text in a corner of the screen.

use std::collections::VecDeque;

use ss2_core::console::Console;

use crate::draw::{Color, DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{CANVAS_H, CANVAS_W, Rect, Vec2};

/// The longest input line, in characters.
pub const MAX_INPUT_CHARS: usize = 256;
/// The most lines an overlay shows.
pub const MAX_OVERLAY_LINES: usize = 32;

const PANEL_HEIGHT: f32 = CANVAS_H * 0.45;
const PADDING: f32 = 8.0;
const LINE_HEIGHT: f32 = 24.0;
const TEXT_SIZE: f32 = 20.0;
const PROMPT_HEIGHT: f32 = 34.0;
/// How many lines of scrollback fit above the prompt.
const VISIBLE_LINES: usize = ((PANEL_HEIGHT - PADDING - PROMPT_HEIGHT) / LINE_HEIGHT) as usize;

/// A key the console reacts to; the platform layer maps its key events onto these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleKey {
    /// Delete the character before the cursor.
    Backspace,
    /// Delete the character at the cursor.
    Delete,
    /// Move the cursor left.
    Left,
    /// Move the cursor right.
    Right,
    /// Cursor to the start of the line.
    Home,
    /// Cursor to the end of the line.
    End,
    /// An older line of the history.
    Up,
    /// A newer line of the history.
    Down,
    /// Complete the name being typed.
    Tab,
    /// Run the line.
    Enter,
    /// Scroll back.
    PageUp,
    /// Scroll forward.
    PageDown,
    /// Close the console.
    Escape,
}

/// What kind of line this is, for its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// A line the player typed.
    Input,
    /// What a command said.
    Output,
    /// An error.
    Error,
}

/// One line of the scrollback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsoleLine {
    /// The text.
    pub text: String,
    /// What kind it is.
    pub kind: LineKind,
}

/// The drop-down console; see the module documentation.
///
/// ```
/// use ss2_core::console::Console;
/// use ss2_ui::devconsole::{ConsoleKey, ConsoleView};
///
/// let mut console = Console::new();
/// console.register_command("echo", "print the arguments", |_, args| Ok(args.join(" "))).unwrap();
/// let mut view = ConsoleView::new(100);
/// assert!(!view.handle_key(ConsoleKey::Enter, &mut console), "closed: the game gets the key");
/// view.toggle();
/// view.handle_text("echo hello");
/// assert!(view.handle_key(ConsoleKey::Enter, &mut console));
/// assert_eq!(view.lines().last().unwrap().text, "hello");
/// assert!(!view.draw().texts.is_empty());
/// ```
pub struct ConsoleView {
    open: bool,
    lines: Vec<ConsoleLine>,
    max_lines: usize,
    input: String,
    /// The cursor, in characters from the start of `input`.
    cursor: usize,
    /// How many lines the view is scrolled back from the newest.
    scroll: usize,
}

impl ConsoleView {
    /// A closed console that remembers at most `max_lines` lines (at least 1).
    pub fn new(max_lines: usize) -> Self {
        Self {
            open: false,
            lines: Vec::new(),
            max_lines: max_lines.max(1),
            input: String::new(),
            cursor: 0,
            scroll: 0,
        }
    }

    /// Whether the console is showing.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open it if closed, close it if open.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// The input line.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The cursor, in characters from the start of the input line.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The scrollback, oldest first.
    pub fn lines(&self) -> &[ConsoleLine] {
        &self.lines
    }

    /// Add `text` to the scrollback (one line per line of text) and show the newest line.
    pub fn print(&mut self, text: String, kind: LineKind) {
        for line in text.lines() {
            self.lines.push(ConsoleLine {
                text: line.to_owned(),
                kind,
            });
        }
        if self.lines.len() > self.max_lines {
            let extra = self.lines.len() - self.max_lines;
            self.lines.drain(..extra);
        }
        self.scroll = 0;
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.input
            .char_indices()
            .nth(chars)
            .map_or(self.input.len(), |(i, _)| i)
    }

    fn set_input(&mut self, text: &str) {
        self.input = text.chars().take(MAX_INPUT_CHARS).collect();
        self.cursor = self.input.chars().count();
    }

    /// Typed text (what the keyboard produced, not key presses). Control characters and newlines
    /// are left out; the line is limited to [`MAX_INPUT_CHARS`]. Ignored while the console is closed.
    pub fn handle_text(&mut self, text: &str) {
        if !self.open {
            return;
        }
        for ch in text.chars().filter(|c| !c.is_control()) {
            if self.input.chars().count() >= MAX_INPUT_CHARS {
                break;
            }
            let at = self.byte_at(self.cursor);
            self.input.insert(at, ch);
            self.cursor += 1;
        }
    }

    /// A key. Returns whether the console used it (a closed console uses none, so the game gets them).
    pub fn handle_key(&mut self, key: ConsoleKey, console: &mut Console) -> bool {
        if !self.open {
            return false;
        }
        let length = self.input.chars().count();
        match key {
            ConsoleKey::Escape => self.open = false,
            ConsoleKey::Left => self.cursor = self.cursor.saturating_sub(1),
            ConsoleKey::Right => self.cursor = (self.cursor + 1).min(length),
            ConsoleKey::Home => self.cursor = 0,
            ConsoleKey::End => self.cursor = length,
            ConsoleKey::Backspace => {
                if self.cursor > 0 {
                    let at = self.byte_at(self.cursor - 1);
                    self.input.remove(at);
                    self.cursor -= 1;
                }
            }
            ConsoleKey::Delete => {
                if self.cursor < length {
                    let at = self.byte_at(self.cursor);
                    self.input.remove(at);
                }
            }
            ConsoleKey::Up => {
                if let Some(line) = console.history_prev().map(str::to_owned) {
                    self.set_input(&line);
                }
            }
            ConsoleKey::Down => match console.history_next().map(str::to_owned) {
                Some(line) => self.set_input(&line),
                None => self.set_input(""),
            },
            ConsoleKey::PageUp => {
                let most = self.lines.len().saturating_sub(VISIBLE_LINES);
                self.scroll = (self.scroll + VISIBLE_LINES - 1).min(most);
            }
            ConsoleKey::PageDown => self.scroll = self.scroll.saturating_sub(VISIBLE_LINES - 1),
            ConsoleKey::Tab => self.complete(console),
            ConsoleKey::Enter => self.submit(console),
        }
        true
    }

    fn submit(&mut self, console: &mut Console) {
        let line = self.input.trim().to_owned();
        self.set_input("");
        if line.is_empty() {
            return;
        }
        self.print(format!("> {line}"), LineKind::Input);
        match console.execute(&line) {
            Ok(text) if text.is_empty() => {}
            Ok(text) => self.print(text, LineKind::Output),
            Err(error) => self.print(error.to_string(), LineKind::Error),
        }
    }

    /// Complete the first word of the line, up to the cursor.
    fn complete(&mut self, console: &Console) {
        let before: String = self.input.chars().take(self.cursor).collect();
        if before.chars().any(char::is_whitespace) {
            return;
        }
        let completion = console.complete(&before);
        let replacement = match completion.candidates.as_slice() {
            [] => return,
            [only] => format!("{only} "),
            many => {
                if completion.common.chars().count() > before.chars().count() {
                    completion.common.clone()
                } else {
                    self.print(many.join("  "), LineKind::Output);
                    return;
                }
            }
        };
        let rest: String = self.input.chars().skip(self.cursor).collect();
        self.input = format!("{replacement}{rest}");
        self.cursor = replacement.chars().count();
    }

    /// The console as a draw list: nothing when closed.
    pub fn draw(&self) -> DrawList {
        if !self.open {
            return DrawList::default();
        }
        let mut list = DrawList::default();
        list.quads.push(Quad {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: CANVAS_W,
                h: PANEL_HEIGHT,
            },
            color: Color([0.02, 0.02, 0.05, 0.88]),
        });
        // The newest lines that fit, bottom aligned above the prompt, scrolled back by `scroll`.
        let end = self.lines.len().saturating_sub(self.scroll);
        let start = end.saturating_sub(VISIBLE_LINES);
        let shown = &self.lines[start..end];
        let first_row = VISIBLE_LINES - shown.len();
        for (i, line) in shown.iter().enumerate() {
            let color = match line.kind {
                LineKind::Input => Color([0.6, 0.8, 1.0, 1.0]),
                LineKind::Output => Color([0.85, 0.85, 0.85, 1.0]),
                LineKind::Error => Color([1.0, 0.45, 0.45, 1.0]),
            };
            list.texts.push(TextRun {
                pos: Vec2 {
                    x: PADDING,
                    y: PADDING + (first_row + i) as f32 * LINE_HEIGHT,
                },
                text: line.text.clone(),
                size: TEXT_SIZE,
                color,
                align: TextAlign::Left,
            });
        }
        let mut prompt = String::from("> ");
        prompt.extend(self.input.chars().take(self.cursor));
        prompt.push('|');
        prompt.extend(self.input.chars().skip(self.cursor));
        list.texts.push(TextRun {
            pos: Vec2 {
                x: PADDING,
                y: PANEL_HEIGHT - PROMPT_HEIGHT,
            },
            text: prompt,
            size: TEXT_SIZE,
            color: Color([1.0, 1.0, 1.0, 1.0]),
            align: TextAlign::Left,
        });
        list
    }
}

/// A window of the latest frame times.
pub struct FrameStats {
    window: usize,
    micros: VecDeque<u64>,
}

/// No frame counts as longer than a minute: a stall must not make the numbers meaningless.
const MAX_FRAME_MICROS: u64 = 60_000_000;

impl FrameStats {
    /// A window of `window` frames (at least 1).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            micros: VecDeque::new(),
        }
    }

    /// Add the time one frame took, in microseconds.
    pub fn record_micros(&mut self, micros: u64) {
        self.micros.push_back(micros.min(MAX_FRAME_MICROS));
        while self.micros.len() > self.window {
            self.micros.pop_front();
        }
    }

    fn average_micros(&self) -> f64 {
        if self.micros.is_empty() {
            return 0.0;
        }
        self.micros.iter().sum::<u64>() as f64 / self.micros.len() as f64
    }

    /// Frames per second, from the average frame time (0 when there is nothing to average).
    pub fn fps(&self) -> f64 {
        let average = self.average_micros();
        if average > 0.0 {
            1_000_000.0 / average
        } else {
            0.0
        }
    }

    /// The average frame time in milliseconds.
    pub fn average_ms(&self) -> f64 {
        self.average_micros() / 1000.0
    }

    /// The longest frame in the window, in milliseconds.
    pub fn max_ms(&self) -> f64 {
        self.micros.iter().copied().max().unwrap_or(0) as f64 / 1000.0
    }

    /// The frame rate of the slowest 1 % of the frames in the window (at least one frame): what a
    /// stutter feels like when the average hides it.
    pub fn one_percent_low_fps(&self) -> f64 {
        if self.micros.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<u64> = self.micros.iter().copied().collect();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        let count = sorted.len().div_ceil(100).max(1);
        let average = sorted[..count].iter().sum::<u64>() as f64 / count as f64;
        if average > 0.0 {
            1_000_000.0 / average
        } else {
            0.0
        }
    }
}

/// The lines of the frame-time overlay.
pub fn stats_lines(stats: &FrameStats) -> Vec<String> {
    vec![
        format!("{:.0} fps", stats.fps()),
        format!(
            "{:.1} ms avg, {:.1} ms max",
            stats.average_ms(),
            stats.max_ms()
        ),
        format!("1% low: {:.0} fps", stats.one_percent_low_fps()),
    ]
}

/// Which corner of the screen an overlay sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    /// Top left.
    TopLeft,
    /// Top right.
    TopRight,
    /// Bottom left.
    BottomLeft,
    /// Bottom right.
    BottomRight,
}

/// Lines of text on a translucent panel in a corner; nothing for no lines, and at most
/// [`MAX_OVERLAY_LINES`] lines.
pub fn draw_overlay(lines: &[String], corner: Corner) -> DrawList {
    const WIDTH: f32 = 360.0;
    const MARGIN: f32 = 12.0;
    const ROW: f32 = 22.0;
    if lines.is_empty() {
        return DrawList::default();
    }
    let lines = &lines[..lines.len().min(MAX_OVERLAY_LINES)];
    let height = lines.len() as f32 * ROW + 12.0;
    let x = match corner {
        Corner::TopLeft | Corner::BottomLeft => MARGIN,
        Corner::TopRight | Corner::BottomRight => CANVAS_W - MARGIN - WIDTH,
    };
    let y = match corner {
        Corner::TopLeft | Corner::TopRight => MARGIN,
        Corner::BottomLeft | Corner::BottomRight => CANVAS_H - MARGIN - height,
    };
    let mut list = DrawList::default();
    list.quads.push(Quad {
        rect: Rect {
            x,
            y,
            w: WIDTH,
            h: height,
        },
        color: Color([0.0, 0.0, 0.0, 0.6]),
    });
    for (i, line) in lines.iter().enumerate() {
        list.texts.push(TextRun {
            pos: Vec2 {
                x: x + 8.0,
                y: y + 6.0 + i as f32 * ROW,
            },
            text: line.clone(),
            size: 18.0,
            color: Color([0.9, 1.0, 0.9, 1.0]),
            align: TextAlign::Left,
        });
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::TextRun;
    use ss2_core::console::{CVarSpec, CVarValue, Console};

    fn console() -> Console {
        let mut c = Console::new();
        c.register_cvar(CVarSpec::new(
            "sv_gravity",
            CVarValue::Float(800.0),
            "gravity",
        ))
        .unwrap();
        c.register_cvar(CVarSpec::new("sv_speed", CVarValue::Float(320.0), "speed"))
            .unwrap();
        c.register_command("echo", "print the arguments", |_, args| Ok(args.join(" ")))
            .unwrap();
        c.register_command("fail", "always fails", |_, _| {
            Err(ss2_core::Error::invalid("fail", "on purpose"))
        })
        .unwrap();
        c
    }

    fn open_view() -> ConsoleView {
        let mut v = ConsoleView::new(100);
        v.toggle();
        v
    }

    fn type_text(v: &mut ConsoleView, text: &str) {
        v.handle_text(text);
    }

    fn press(v: &mut ConsoleView, c: &mut Console, key: ConsoleKey) -> bool {
        v.handle_key(key, c)
    }

    fn texts(list: &DrawList) -> Vec<String> {
        list.texts
            .iter()
            .map(|t: &TextRun| t.text.clone())
            .collect()
    }

    // ---- opening and closing ----------------------------------------------------------------

    #[test]
    fn a_closed_console_ignores_everything_and_draws_nothing() {
        let mut v = ConsoleView::new(10);
        let mut c = console();
        assert!(!v.is_open());
        assert!(
            !press(&mut v, &mut c, ConsoleKey::Enter),
            "keys are not consumed while closed"
        );
        type_text(&mut v, "abc");
        assert_eq!(v.input(), "", "typed text is not collected while closed");
        assert_eq!(v.draw(), DrawList::default());
    }

    #[test]
    fn the_toggle_and_escape_open_and_close_it() {
        let mut v = ConsoleView::new(10);
        let mut c = console();
        v.toggle();
        assert!(v.is_open());
        assert!(
            press(&mut v, &mut c, ConsoleKey::Escape),
            "escape closes it and is consumed"
        );
        assert!(!v.is_open());
        v.toggle();
        v.toggle();
        assert!(!v.is_open());
    }

    // ---- editing the line -------------------------------------------------------------------

    #[test]
    fn typed_text_goes_in_at_the_cursor_and_the_cursor_moves() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "helo");
        press(&mut v, &mut c, ConsoleKey::Left);
        type_text(&mut v, "l");
        assert_eq!((v.input(), v.cursor()), ("hello", 4));
        press(&mut v, &mut c, ConsoleKey::Home);
        assert_eq!(v.cursor(), 0);
        type_text(&mut v, ">");
        assert_eq!(v.input(), ">hello");
        press(&mut v, &mut c, ConsoleKey::End);
        assert_eq!(v.cursor(), 6);
        press(&mut v, &mut c, ConsoleKey::Right);
        assert_eq!(v.cursor(), 6, "the cursor stops at the end");
        press(&mut v, &mut c, ConsoleKey::Home);
        press(&mut v, &mut c, ConsoleKey::Left);
        assert_eq!(v.cursor(), 0, "and at the start");
    }

    #[test]
    fn backspace_and_delete_remove_around_the_cursor() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "abcd");
        press(&mut v, &mut c, ConsoleKey::Left);
        press(&mut v, &mut c, ConsoleKey::Backspace);
        assert_eq!((v.input(), v.cursor()), ("abd", 2));
        press(&mut v, &mut c, ConsoleKey::Delete);
        assert_eq!((v.input(), v.cursor()), ("ab", 2));
        press(&mut v, &mut c, ConsoleKey::Delete);
        assert_eq!(v.input(), "ab", "nothing after the cursor to delete");
        press(&mut v, &mut c, ConsoleKey::Home);
        press(&mut v, &mut c, ConsoleKey::Backspace);
        assert_eq!(v.input(), "ab", "nothing before the cursor to remove");
    }

    #[test]
    fn editing_is_by_character_not_by_byte() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "añb");
        press(&mut v, &mut c, ConsoleKey::Left);
        press(&mut v, &mut c, ConsoleKey::Backspace);
        assert_eq!(v.input(), "ab");
        type_text(&mut v, "é€");
        assert_eq!(v.input(), "aé€b");
        assert_eq!(v.cursor(), 3);
        // Several two-byte characters: a character index is not a byte index.
        let mut w = open_view();
        type_text(&mut w, "ññb");
        press(&mut w, &mut c, ConsoleKey::Backspace);
        assert_eq!(w.input(), "ññ");
        press(&mut w, &mut c, ConsoleKey::Left);
        press(&mut w, &mut c, ConsoleKey::Delete);
        assert_eq!(w.input(), "ñ");
    }

    #[test]
    fn control_characters_and_newlines_are_not_typed_and_the_line_is_limited() {
        let mut v = open_view();
        type_text(&mut v, "a\tb\nc\u{7}d");
        assert_eq!(v.input(), "abcd");
        type_text(&mut v, &"x".repeat(1000));
        assert_eq!(v.input().chars().count(), MAX_INPUT_CHARS);
    }

    // ---- running commands and history -------------------------------------------------------

    #[test]
    fn enter_runs_the_line_prints_what_it_said_and_clears_the_input() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "echo hello world");
        assert!(press(&mut v, &mut c, ConsoleKey::Enter));
        assert_eq!(v.input(), "");
        let lines = v.lines();
        assert_eq!(
            lines[lines.len() - 2],
            ConsoleLine {
                text: "> echo hello world".into(),
                kind: LineKind::Input
            }
        );
        assert_eq!(
            lines[lines.len() - 1],
            ConsoleLine {
                text: "hello world".into(),
                kind: LineKind::Output
            }
        );
    }

    #[test]
    fn a_failing_command_prints_its_error_in_the_error_kind_and_an_empty_line_prints_nothing() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "fail");
        press(&mut v, &mut c, ConsoleKey::Enter);
        let last = v.lines().last().unwrap().clone();
        assert_eq!(last.kind, LineKind::Error);
        assert!(last.text.contains("on purpose"), "{last:?}");
        let before = v.lines().len();
        press(&mut v, &mut c, ConsoleKey::Enter);
        press(&mut v, &mut c, ConsoleKey::Enter);
        assert_eq!(v.lines().len(), before, "an empty line adds nothing");
    }

    #[test]
    fn up_and_down_walk_the_history_of_the_console() {
        let mut v = open_view();
        let mut c = console();
        for line in ["echo one", "echo two", "echo three"] {
            type_text(&mut v, line);
            press(&mut v, &mut c, ConsoleKey::Enter);
        }
        press(&mut v, &mut c, ConsoleKey::Up);
        assert_eq!(v.input(), "echo three");
        press(&mut v, &mut c, ConsoleKey::Up);
        assert_eq!(v.input(), "echo two");
        press(&mut v, &mut c, ConsoleKey::Down);
        assert_eq!(v.input(), "echo three");
        press(&mut v, &mut c, ConsoleKey::Down);
        assert_eq!(
            v.input(),
            "",
            "past the newest entry is the empty line again"
        );
        assert_eq!(v.cursor(), 0);
        press(&mut v, &mut c, ConsoleKey::Up);
        assert_eq!(
            v.cursor(),
            "echo three".chars().count(),
            "the cursor goes to the end of a recalled line"
        );
    }

    // ---- completion -------------------------------------------------------------------------

    #[test]
    fn tab_completes_a_unique_name_and_extends_to_the_common_prefix_otherwise() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "ech");
        press(&mut v, &mut c, ConsoleKey::Tab);
        assert_eq!(
            v.input(),
            "echo ",
            "a single candidate is completed and followed by a space"
        );
        let mut v = open_view();
        type_text(&mut v, "sv_");
        let before = v.lines().len();
        press(&mut v, &mut c, ConsoleKey::Tab);
        assert_eq!(v.input(), "sv_", "the candidates share only what is typed");
        let shown: Vec<_> = v.lines()[before..].iter().map(|l| l.text.clone()).collect();
        assert!(
            shown
                .iter()
                .any(|t| t.contains("sv_gravity") && t.contains("sv_speed")),
            "the candidates are listed: {shown:?}"
        );
        type_text(&mut v, "g");
        press(&mut v, &mut c, ConsoleKey::Tab);
        assert_eq!(v.input(), "sv_gravity ");
    }

    #[test]
    fn tab_completes_only_the_first_word_and_does_nothing_for_an_unknown_prefix() {
        let mut v = open_view();
        let mut c = console();
        type_text(&mut v, "echo ec");
        press(&mut v, &mut c, ConsoleKey::Tab);
        assert_eq!(v.input(), "echo ec", "arguments are not completed");
        let mut v = open_view();
        type_text(&mut v, "zzz");
        press(&mut v, &mut c, ConsoleKey::Tab);
        assert_eq!(v.input(), "zzz");
    }

    // ---- scrollback -------------------------------------------------------------------------

    #[test]
    fn the_scrollback_is_bounded_and_drops_the_oldest_lines() {
        let mut v = ConsoleView::new(5);
        for i in 0..20 {
            v.print(format!("line {i}"), LineKind::Output);
        }
        let all: Vec<_> = v.lines().iter().map(|l| l.text.clone()).collect();
        assert_eq!(all, ["line 15", "line 16", "line 17", "line 18", "line 19"]);
    }

    #[test]
    fn page_up_and_down_scroll_back_and_new_output_snaps_to_the_bottom() {
        let mut v = open_view();
        let mut c = console();
        for i in 0..50 {
            v.print(format!("line {i}"), LineKind::Output);
        }
        let bottom = texts(&v.draw());
        assert!(bottom.iter().any(|t| t == "line 49"), "{bottom:?}");
        press(&mut v, &mut c, ConsoleKey::PageUp);
        let scrolled = texts(&v.draw());
        assert!(
            !scrolled.iter().any(|t| t == "line 49"),
            "scrolled away from the newest: {scrolled:?}"
        );
        assert!(
            scrolled.iter().any(|t| t == "line 40") || scrolled.iter().any(|t| t == "line 30"),
            "{scrolled:?}"
        );
        for _ in 0..100 {
            press(&mut v, &mut c, ConsoleKey::PageUp);
        }
        assert!(
            texts(&v.draw())
                .iter()
                .any(|t| t == "line 0" || t == "line 1" || t == "line 2"),
            "scrolling stops at the oldest line"
        );
        for _ in 0..100 {
            press(&mut v, &mut c, ConsoleKey::PageDown);
        }
        assert!(
            texts(&v.draw()).iter().any(|t| t == "line 49"),
            "and back at the bottom"
        );
        press(&mut v, &mut c, ConsoleKey::PageUp);
        v.print("new".into(), LineKind::Output);
        assert!(
            texts(&v.draw()).iter().any(|t| t == "new"),
            "new output shows even after scrolling"
        );
    }

    // ---- drawing ----------------------------------------------------------------------------

    #[test]
    fn an_open_console_draws_a_dark_panel_across_the_top_and_the_prompt_with_the_cursor() {
        let mut v = open_view();
        type_text(&mut v, "abc");
        let mut c = console();
        press(&mut v, &mut c, ConsoleKey::Left);
        let list = v.draw();
        let panel = list.quads[0];
        assert!(
            (panel.rect.w - crate::CANVAS_W).abs() < 1e-3
                && panel.rect.x == 0.0
                && panel.rect.y == 0.0,
            "{panel:?}"
        );
        assert!(
            panel.rect.h > 200.0 && panel.rect.h < crate::CANVAS_H * 0.7,
            "a drop-down, not the whole screen: {}",
            panel.rect.h
        );
        assert!(panel.color.0[3] > 0.5, "mostly opaque so text can be read");
        let prompt = list
            .texts
            .iter()
            .find(|t| t.text.starts_with("> "))
            .expect("the prompt");
        assert_eq!(
            prompt.text, "> ab|c",
            "the cursor is shown as a bar at its place"
        );
        assert!(
            prompt.pos.y > panel.rect.y && prompt.pos.y < panel.rect.y + panel.rect.h,
            "inside the panel"
        );
    }

    #[test]
    fn lines_are_drawn_oldest_at_the_top_with_a_colour_for_their_kind_and_fit_the_panel() {
        let mut v = open_view();
        v.print("first".into(), LineKind::Output);
        v.print("second".into(), LineKind::Error);
        v.print("third".into(), LineKind::Input);
        let list = v.draw();
        let by = |text: &str| list.texts.iter().find(|t| t.text == text).unwrap().clone();
        let (a, b, c) = (by("first"), by("second"), by("third"));
        assert!(
            a.pos.y < b.pos.y && b.pos.y < c.pos.y,
            "older lines are higher"
        );
        assert_ne!(a.color, b.color, "errors are coloured differently");
        let panel = list.quads[0].rect;
        for t in &list.texts {
            assert!(
                t.pos.y >= panel.y && t.pos.y + t.size <= panel.y + panel.h + 1e-3,
                "{t:?} is inside {panel:?}"
            );
        }
        let mut many = open_view();
        for i in 0..500 {
            many.print(format!("{i}"), LineKind::Output);
        }
        let shown = many.draw().texts.len();
        assert!(shown < 60, "only what fits is drawn: {shown}");
    }

    // ---- frame statistics and the overlay ---------------------------------------------------

    #[test]
    fn steady_frames_give_the_frame_rate_and_the_frame_time() {
        let mut s = FrameStats::new(120);
        assert_eq!(
            (s.fps(), s.average_ms(), s.max_ms()),
            (0.0, 0.0, 0.0),
            "nothing recorded"
        );
        for _ in 0..120 {
            s.record_micros(16_667);
        }
        assert!((s.fps() - 60.0).abs() < 0.1, "{}", s.fps());
        assert!((s.average_ms() - 16.667).abs() < 0.01);
        assert!((s.max_ms() - 16.667).abs() < 0.01);
    }

    #[test]
    fn the_statistics_cover_a_window_and_a_hitch_shows_in_the_maximum_and_the_slow_percentile() {
        let mut s = FrameStats::new(100);
        for _ in 0..100 {
            s.record_micros(10_000);
        }
        s.record_micros(80_000);
        assert!((s.max_ms() - 80.0).abs() < 0.01);
        assert!(
            s.fps() < 100.0 && s.fps() > 90.0,
            "one slow frame barely moves the average: {}",
            s.fps()
        );
        for _ in 0..100 {
            s.record_micros(10_000);
        }
        assert!(
            (s.max_ms() - 10.0).abs() < 0.01,
            "the hitch has left the window"
        );
        // The 1% low: the average of the slowest 1% of the frames in the window.
        let mut t = FrameStats::new(100);
        for i in 0..100 {
            t.record_micros(if i == 7 { 50_000 } else { 10_000 });
        }
        assert!(
            (t.one_percent_low_fps() - 20.0).abs() < 0.1,
            "{}",
            t.one_percent_low_fps()
        );
        assert_eq!(FrameStats::new(10).one_percent_low_fps(), 0.0);
        // With 200 frames the slowest 1 % is two frames: 50 ms and 30 ms average 40 ms, which is 25 fps.
        let mut u = FrameStats::new(200);
        for i in 0..200 {
            u.record_micros(match i {
                3 => 50_000,
                9 => 30_000,
                _ => 10_000,
            });
        }
        assert!(
            (u.one_percent_low_fps() - 25.0).abs() < 0.1,
            "{}",
            u.one_percent_low_fps()
        );
    }

    #[test]
    fn a_frame_time_of_zero_or_a_huge_one_does_not_break_the_numbers() {
        let mut s = FrameStats::new(10);
        s.record_micros(0);
        assert!(s.fps().is_finite() && s.average_ms() == 0.0);
        s.record_micros(u64::MAX);
        s.record_micros(u64::MAX);
        assert!(
            s.max_ms() <= 60_000.0,
            "a frame is counted as at most a minute: {}",
            s.max_ms()
        );
        assert!(s.fps().is_finite() && s.average_ms().is_finite() && s.max_ms().is_finite());
        assert_eq!(
            FrameStats::new(0).fps(),
            0.0,
            "a window of zero is a window of one"
        );
    }

    #[test]
    fn the_overlay_puts_lines_in_the_chosen_corner() {
        let lines = ["60 fps".to_owned(), "16.7 ms".to_owned()];
        let left = draw_overlay(&lines, Corner::TopLeft);
        let right = draw_overlay(&lines, Corner::TopRight);
        let bottom = draw_overlay(&lines, Corner::BottomLeft);
        assert_eq!(texts(&left), ["60 fps", "16.7 ms"]);
        assert!(left.texts[0].pos.y < left.texts[1].pos.y);
        assert!(left.texts[0].pos.x < 100.0 && left.texts[0].pos.y < 100.0);
        assert!(
            right.texts[0].pos.x > crate::CANVAS_W - 400.0,
            "{:?}",
            right.texts[0].pos
        );
        assert!(
            bottom.texts[0].pos.y > crate::CANVAS_H - 200.0,
            "{:?}",
            bottom.texts[0].pos
        );
        assert!(
            left.quads.len() == 1 && left.quads[0].color.0[3] < 1.0,
            "a translucent backing"
        );
        assert_eq!(
            draw_overlay(&[], Corner::TopLeft),
            DrawList::default(),
            "no lines, no panel"
        );
        let long: Vec<String> = (0..500).map(|i| i.to_string()).collect();
        assert!(
            draw_overlay(&long, Corner::TopLeft).texts.len() <= MAX_OVERLAY_LINES,
            "bounded"
        );
    }

    #[test]
    fn the_stats_lines_name_the_figures() {
        let mut s = FrameStats::new(60);
        for _ in 0..60 {
            s.record_micros(16_667);
        }
        let lines = stats_lines(&s);
        assert!(
            lines[0].contains("60") && lines[0].to_lowercase().contains("fps"),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("16.7") && l.contains("ms")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.to_lowercase().contains("1% low")),
            "{lines:?}"
        );
    }
}
