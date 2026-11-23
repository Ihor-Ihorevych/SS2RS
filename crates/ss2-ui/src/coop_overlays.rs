//! The overlays a co-op game draws over the world (roadmap T10-014): the **scoreboard**, the **chat**, the **nameplates** over the
//! heads of teammates, the **ping markers** and the **voice indicators**.
//!
//! They know nothing of the game. Each takes plain data in (a list of players, text that was typed, positions that the caller has
//! already projected to the screen) and appends quads and text runs to a [`DrawList`] in canvas units (1920x1080) with a placeholder
//! look. They *append* instead of clearing, so that several overlays (and the HUD) can share one list; the caller clears it once a
//! frame. [`CoopOverlays`] holds all five and draws them in the right order.
//!
//! * [`Scoreboard`]: rows of name, kills, deaths, score and ping, sorted, the local player highlighted, shown while a key is held.
//! * [`ChatBox`]: a bounded history whose lines fade out, and an input line with a cursor that gives [`ChatCommand`]s.
//! * [`Nameplates`]: a name and a health and armor bar per teammate, smaller and fainter with distance, hidden when off screen or
//!   behind the camera, kept inside the safe area, at most a few at a time.
//! * [`PingMarkers`]: a marker with a label and a lifetime; one that is off screen is pinned to the edge with an arrow.
//! * [`VoiceIndicators`]: who is talking, with a level meter, kept on screen a moment after the voice stops so it does not flicker.
//!
//! **No font metrics.** `ss2-ui` does not know how wide a string is drawn, so text is placed by anchor (left or centred) in fixed
//! columns, and lengths are limited in *characters* with a conservative width estimate ([`estimate_text_width`]). Names and messages
//! are cleaned of control characters and cut with an ellipsis. **Strings that vary** (names, chat rows, scores, distances) are owned
//! `String`s, so drawing allocates a short string per number or name; none of this is part of the menu frame budget of
//! `docs/UI_BUDGET.md`. See `docs/COOP_OVERLAYS.md`.

use std::borrow::Cow;
use std::collections::VecDeque;

use crate::draw::{Color, DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{CANVAS_H, CANVAS_W, Rect, Vec2};
use crate::theme;

/// The part of the canvas that overlays keep inside (5 % margins), where a screen edge can be hidden by a television's overscan.
pub const SAFE_AREA: Rect = Rect {
    x: 96.0,
    y: 54.0,
    w: 1728.0,
    h: 972.0,
};

/// The row of the local player on the scoreboard.
pub const LOCAL_ROW: Color = Color([0.85, 0.45, 0.10, 0.75]);
/// The green of a voice that is on (the dot on the scoreboard, the strip on a nameplate, the meter).
pub const VOICE_ON: Color = Color([0.35, 0.9, 0.4, 1.0]);
/// Health bar above half.
pub const HEALTH_OK: Color = Color([0.3, 0.85, 0.35, 1.0]);
/// Health bar between a quarter and a half.
pub const HEALTH_LOW: Color = Color([0.95, 0.8, 0.2, 1.0]);
/// Health bar at a quarter or less.
pub const HEALTH_CRITICAL: Color = Color([0.95, 0.25, 0.2, 1.0]);
/// The armor bar.
pub const ARMOR_BAR: Color = Color([0.35, 0.6, 1.0, 1.0]);

/// The longest name drawn, in characters (longer ones end in an ellipsis).
pub const MAX_NAME_CHARS: usize = 20;

/// The width a string of `chars` characters is assumed to take at font size `size`. Inter averages about half the size per
/// character; this takes 0.55 to err on the wide side. A guess for layout, not a measurement.
pub fn estimate_text_width(chars: usize, size: f32) -> f32 {
    chars as f32 * size * 0.55
}

/// Remove control characters, trim, and cut to at most `max_chars` characters, ending in an ellipsis (`…`, one character) when cut.
pub fn clip_text(text: &str, max_chars: usize) -> String {
    let cleaned: String = text.chars().filter(|c| !c.is_control()).collect();
    let cleaned = cleaned.trim();
    if cleaned.chars().count() <= max_chars {
        return cleaned.to_owned();
    }
    if max_chars == 0 {
        return String::new();
    }
    let mut out: String = cleaned.chars().take(max_chars - 1).collect();
    out.push('\u{2026}');
    out
}

/// `color` with its alpha multiplied by `alpha` (clamped to 0..=1; NaN counts as 0).
fn faded(color: Color, alpha: f32) -> Color {
    let a = if alpha.is_nan() {
        0.0
    } else {
        alpha.clamp(0.0, 1.0)
    };
    let Color([r, g, b, base]) = color;
    Color([r, g, b, base * a])
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// `v` limited to `lo..=hi` without the panics of `f32::clamp` (a NaN bound or `hi < lo` gives `lo`-ish values instead of a crash).
fn pin(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi.max(lo))
}

fn unit(x: f32) -> f32 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

fn push_quad(list: &mut DrawList, x: f32, y: f32, w: f32, h: f32, color: Color) {
    list.quads.push(Quad {
        rect: Rect { x, y, w, h },
        color,
    });
}

fn push_text(
    list: &mut DrawList,
    pos: Vec2,
    text: impl Into<Cow<'static, str>>,
    size: f32,
    color: Color,
    align: TextAlign,
) {
    list.texts.push(TextRun {
        pos,
        text: text.into(),
        size,
        color,
        align,
    });
}

fn finite(v: Vec2) -> bool {
    v.x.is_finite() && v.y.is_finite()
}

/// A point already projected to the screen by the caller (the camera and the world are not this crate's business).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projected {
    /// Where it lands, in canvas units (y down). Meaningless when `in_front` is false.
    pub screen: Vec2,
    /// How far it is from the camera, in the game's units (metres).
    pub distance: f32,
    /// False when the point is behind the camera.
    pub in_front: bool,
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Scoreboard

/// The most rows the scoreboard shows; further players are summarised as "+N more".
pub const MAX_SCOREBOARD_ROWS: usize = 16;

/// One player on the scoreboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreEntry {
    /// The player's id (breaks ties, so the order is the same on every machine).
    pub id: u32,
    /// The name as the player chose it (cleaned and cut when stored).
    pub name: String,
    /// Kills.
    pub kills: u32,
    /// Deaths.
    pub deaths: u32,
    /// The score (may be negative).
    pub score: i64,
    /// Round-trip time in milliseconds, or `None` when unknown (or a bot).
    pub ping_ms: Option<u32>,
    /// This is the player at this machine: the row is highlighted and is never left out.
    pub local: bool,
    /// The player's voice is on at this moment.
    pub talking: bool,
}

/// How good a connection is, for the colour of the ping figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingQuality {
    /// Under 80 ms.
    Good,
    /// 80 to 149 ms.
    Fair,
    /// 150 ms and more.
    Poor,
}

/// Classify a round-trip time.
pub fn ping_quality(ms: u32) -> PingQuality {
    if ms < 80 {
        PingQuality::Good
    } else if ms < 150 {
        PingQuality::Fair
    } else {
        PingQuality::Poor
    }
}

/// The scoreboard: set the players, hold a key.
#[derive(Clone, Debug, Default)]
pub struct Scoreboard {
    entries: Vec<ScoreEntry>,
    hidden: usize,
    held: bool,
}

const SB_W: f32 = 1000.0;
const SB_ROW_H: f32 = 44.0;
const SB_HEAD_H: f32 = 96.0;
const SB_FOOT_H: f32 = 44.0;
const SB_SIZE: f32 = 30.0;

impl Scoreboard {
    /// An empty scoreboard, not shown.
    pub fn new() -> Self {
        Self::default()
    }

    /// The scoreboard key went down (`true`) or up (`false`).
    pub fn set_held(&mut self, held: bool) {
        self.held = held;
    }

    /// Whether the scoreboard is on screen.
    pub fn is_visible(&self) -> bool {
        self.held
    }

    /// Replace the players. They are sorted by score (high first), then kills (high first), then deaths (low first), then id; names are
    /// cleaned and cut to [`MAX_NAME_CHARS`] (an empty one becomes "Player N"). At most [`MAX_SCOREBOARD_ROWS`] are kept, **and the
    /// local player is always among them**: if the sort puts it beyond the last row, it takes the last row's place.
    pub fn set_entries(&mut self, entries: &[ScoreEntry]) {
        let mut all: Vec<ScoreEntry> = entries.to_vec();
        for e in &mut all {
            let name = clip_text(&e.name, MAX_NAME_CHARS);
            e.name = if name.is_empty() {
                format!("Player {}", e.id)
            } else {
                name
            };
        }
        all.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.kills.cmp(&a.kills))
                .then(a.deaths.cmp(&b.deaths))
                .then(a.id.cmp(&b.id))
        });
        let total = all.len();
        if total > MAX_SCOREBOARD_ROWS {
            let local = all
                .iter()
                .skip(MAX_SCOREBOARD_ROWS)
                .position(|e| e.local)
                .map(|i| i + MAX_SCOREBOARD_ROWS);
            if let Some(i) = local {
                all.swap(MAX_SCOREBOARD_ROWS - 1, i);
            }
            all.truncate(MAX_SCOREBOARD_ROWS);
        }
        self.hidden = total - all.len();
        self.entries = all;
    }

    /// The rows in the order they are drawn.
    pub fn rows(&self) -> &[ScoreEntry] {
        &self.entries
    }

    /// How many players did not fit.
    pub fn hidden(&self) -> usize {
        self.hidden
    }

    /// The panel's rectangle (empty scoreboard included), centred on the canvas.
    fn panel(&self) -> Rect {
        let foot = if self.hidden > 0 { SB_FOOT_H } else { 0.0 };
        let h = SB_HEAD_H + self.entries.len() as f32 * SB_ROW_H + foot + 16.0;
        Rect {
            x: (CANVAS_W - SB_W) / 2.0,
            y: (CANVAS_H - h) / 2.0,
            w: SB_W,
            h,
        }
    }

    /// Append the scoreboard to `list` (nothing unless the key is held).
    pub fn draw_into(&self, list: &mut DrawList) {
        if !self.held {
            return;
        }
        let p = self.panel();
        push_quad(list, p.x, p.y, p.w, p.h, Color([0.02, 0.02, 0.05, 0.88]));
        push_text(
            list,
            Vec2 {
                x: p.x + p.w / 2.0,
                y: p.y + 10.0,
            },
            format!("Scoreboard ({})", self.entries.len() + self.hidden),
            40.0,
            theme::TEXT,
            TextAlign::Center,
        );
        let muted = Color([0.65, 0.68, 0.75, 1.0]);
        let head_y = p.y + 62.0;
        for (label, x, align) in [
            ("Player", p.x + 56.0, TextAlign::Left),
            ("Kills", p.x + 520.0, TextAlign::Center),
            ("Deaths", p.x + 630.0, TextAlign::Center),
            ("Score", p.x + 770.0, TextAlign::Center),
            ("Ping", p.x + 910.0, TextAlign::Center),
        ] {
            push_text(list, Vec2 { x, y: head_y }, label, 24.0, muted, align);
        }
        for (i, e) in self.entries.iter().enumerate() {
            let y = p.y + SB_HEAD_H + i as f32 * SB_ROW_H;
            let stripe = if e.local {
                LOCAL_ROW
            } else if i % 2 == 0 {
                Color([0.16, 0.17, 0.20, 0.6])
            } else {
                Color([0.10, 0.11, 0.14, 0.6])
            };
            push_quad(list, p.x + 8.0, y, p.w - 16.0, SB_ROW_H - 4.0, stripe);
            if e.talking {
                push_quad(list, p.x + 20.0, y + 12.0, 16.0, 16.0, VOICE_ON);
            }
            let ty = y + 2.0;
            push_text(
                list,
                Vec2 {
                    x: p.x + 56.0,
                    y: ty,
                },
                e.name.clone(),
                SB_SIZE,
                theme::TEXT,
                TextAlign::Left,
            );
            for (value, x) in [
                (e.kills.to_string(), 520.0),
                (e.deaths.to_string(), 630.0),
                (e.score.to_string(), 770.0),
            ] {
                push_text(
                    list,
                    Vec2 { x: p.x + x, y: ty },
                    value,
                    SB_SIZE,
                    theme::TEXT,
                    TextAlign::Center,
                );
            }
            let (ping, color) = match e.ping_ms {
                Some(ms) => (
                    ms.min(999).to_string(),
                    match ping_quality(ms) {
                        PingQuality::Good => Color([0.45, 0.95, 0.5, 1.0]),
                        PingQuality::Fair => Color([0.98, 0.85, 0.35, 1.0]),
                        PingQuality::Poor => Color([1.0, 0.4, 0.4, 1.0]),
                    },
                ),
                None => ("-".to_owned(), muted),
            };
            push_text(
                list,
                Vec2 {
                    x: p.x + 910.0,
                    y: ty,
                },
                ping,
                SB_SIZE,
                color,
                TextAlign::Center,
            );
        }
        if self.hidden > 0 {
            let y = p.y + SB_HEAD_H + self.entries.len() as f32 * SB_ROW_H;
            push_text(
                list,
                Vec2 {
                    x: p.x + p.w / 2.0,
                    y: y + 4.0,
                },
                format!("+{} more", self.hidden),
                SB_SIZE - 4.0,
                muted,
                TextAlign::Center,
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Chat

/// What kind of line the chat shows (it decides the colour and the prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatKind {
    /// A message to everybody: `Name: text`.
    Player,
    /// A message to the team: `[Team] Name: text`.
    Team,
    /// A line from the game ("Mia joined"): the text alone.
    System,
}

/// The limits of the chat. All counts are in characters or ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatConfig {
    /// Ticks a line stays fully visible after it arrived (while the input is closed).
    pub visible_ticks: u32,
    /// Ticks it then takes to fade out.
    pub fade_ticks: u32,
    /// Lines kept in the history; older ones are dropped.
    pub max_stored: usize,
    /// Rows drawn at once (a long line takes several).
    pub max_rows: usize,
    /// Characters per row before a line is wrapped.
    pub wrap_chars: usize,
    /// The longest message, typed or received.
    pub max_message_chars: usize,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            visible_ticks: 600,
            fade_ticks: 60,
            max_stored: 100,
            max_rows: 8,
            wrap_chars: 56,
            max_message_chars: 200,
        }
    }
}

/// A key the chat input reacts to (the platform layer maps its key events onto these; typed characters come as text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatKey {
    /// Send the message.
    Enter,
    /// Throw the message away and close.
    Escape,
    /// Delete the character before the cursor.
    Backspace,
    /// Delete the character after the cursor.
    Delete,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Cursor to the start.
    Home,
    /// Cursor to the end.
    End,
    /// Switch between "all" and "team".
    Tab,
}

/// What the app must do after a chat key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatCommand {
    /// Send `text` (trimmed, not empty, at most `max_message_chars`) to everybody or to the team.
    Send {
        /// The message.
        text: String,
        /// To the team only.
        team: bool,
    },
    /// The input was closed without sending anything.
    Cancel,
}

#[derive(Clone, Debug)]
struct ChatLine {
    kind: ChatKind,
    rows: Vec<String>,
    age: u32,
}

/// The chat: history and input.
#[derive(Clone, Debug)]
pub struct ChatBox {
    cfg: ChatConfig,
    lines: VecDeque<ChatLine>,
    open: bool,
    team: bool,
    input: String,
    cursor: usize,
}

const CHAT_X: f32 = 48.0;
const CHAT_W: f32 = 780.0;
const CHAT_ROW_H: f32 = 30.0;
const CHAT_SIZE: f32 = 24.0;
const CHAT_BOTTOM: f32 = 880.0;

/// Break `text` into rows of at most `width` characters: at the last space that fits, else in the middle of a word.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let chars: Vec<char> = text.chars().collect();
    let mut rows = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + width).min(chars.len());
        if end == chars.len() {
            rows.push(chars[start..end].iter().collect());
            break;
        }
        // Break after the last space inside the row, unless that would leave the row empty.
        let cut = if chars[end] == ' ' {
            end
        } else {
            chars[start..end]
                .iter()
                .rposition(|c| *c == ' ')
                .filter(|&i| i > 0)
                .map_or(end, |i| start + i)
        };
        rows.push(chars[start..cut].iter().collect::<String>());
        start = cut;
        while start < chars.len() && chars[start] == ' ' {
            start += 1;
        }
    }
    rows
}

impl ChatBox {
    /// An empty chat with the input closed.
    pub fn new(cfg: ChatConfig) -> Self {
        Self {
            cfg,
            lines: VecDeque::new(),
            open: false,
            team: false,
            input: String::new(),
            cursor: 0,
        }
    }

    /// Add a received line. The sender is cleaned and cut like a name, the text to `max_message_chars`; a line with no text is
    /// ignored. It is wrapped into rows now. The history keeps `max_stored` lines.
    pub fn push(&mut self, kind: ChatKind, sender: &str, text: &str) {
        let text = clip_text(text, self.cfg.max_message_chars);
        if text.is_empty() || self.cfg.max_stored == 0 {
            return;
        }
        let who = clip_text(sender, MAX_NAME_CHARS);
        let full = match kind {
            ChatKind::Player if who.is_empty() => text,
            ChatKind::Player => format!("{who}: {text}"),
            ChatKind::Team if who.is_empty() => format!("[Team] {text}"),
            ChatKind::Team => format!("[Team] {who}: {text}"),
            ChatKind::System => text,
        };
        self.lines.push_back(ChatLine {
            kind,
            rows: wrap(&full, self.cfg.wrap_chars),
            age: 0,
        });
        while self.lines.len() > self.cfg.max_stored {
            self.lines.pop_front();
        }
    }

    /// How many lines the history holds.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// One tick: every line gets older.
    pub fn update(&mut self) {
        for l in &mut self.lines {
            l.age = l.age.saturating_add(1);
        }
    }

    /// Open the input, for the team (`true`) or for everybody. A draft that was open is kept.
    pub fn open(&mut self, team: bool) {
        if !self.open {
            self.input.clear();
            self.cursor = 0;
        }
        self.open = true;
        self.team = team;
    }

    /// Whether the input is open (the game should not read movement keys then).
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Whether the message goes to the team.
    pub fn is_team(&self) -> bool {
        self.team
    }

    /// The draft message.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The cursor's place in the draft, in characters.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.input
            .char_indices()
            .nth(chars)
            .map_or(self.input.len(), |(i, _)| i)
    }

    fn close(&mut self) {
        self.open = false;
        self.input.clear();
        self.cursor = 0;
    }

    /// Typed text at the cursor. Control characters are left out and the draft stops at `max_message_chars`. Ignored while closed.
    pub fn handle_text(&mut self, text: &str) {
        if !self.open {
            return;
        }
        for ch in text.chars().filter(|c| !c.is_control()) {
            if self.input.chars().count() >= self.cfg.max_message_chars {
                break;
            }
            let at = self.byte_at(self.cursor);
            self.input.insert(at, ch);
            self.cursor += 1;
        }
    }

    /// A key. Gives the command it caused: Enter sends a non-empty message (and closes), Enter on an empty one and Escape cancel.
    /// A closed chat ignores every key.
    pub fn handle_key(&mut self, key: ChatKey) -> Option<ChatCommand> {
        if !self.open {
            return None;
        }
        let length = self.input.chars().count();
        match key {
            ChatKey::Enter => {
                let text = self.input.trim().to_owned();
                let team = self.team;
                self.close();
                return Some(if text.is_empty() {
                    ChatCommand::Cancel
                } else {
                    ChatCommand::Send { text, team }
                });
            }
            ChatKey::Escape => {
                self.close();
                return Some(ChatCommand::Cancel);
            }
            ChatKey::Tab => self.team = !self.team,
            ChatKey::Left => self.cursor = self.cursor.saturating_sub(1),
            ChatKey::Right => self.cursor = (self.cursor + 1).min(length),
            ChatKey::Home => self.cursor = 0,
            ChatKey::End => self.cursor = length,
            ChatKey::Backspace => {
                if self.cursor > 0 {
                    let at = self.byte_at(self.cursor - 1);
                    self.input.remove(at);
                    self.cursor -= 1;
                }
            }
            ChatKey::Delete => {
                if self.cursor < length {
                    let at = self.byte_at(self.cursor);
                    self.input.remove(at);
                }
            }
        }
        None
    }

    /// How opaque a line of this age is: 1 while the input is open or the line is young, then fading to 0.
    fn alpha(&self, age: u32) -> f32 {
        if self.open || age <= self.cfg.visible_ticks {
            return 1.0;
        }
        let over = age - self.cfg.visible_ticks;
        if over >= self.cfg.fade_ticks {
            0.0
        } else {
            1.0 - over as f32 / self.cfg.fade_ticks as f32
        }
    }

    /// The rows that are drawn, newest last, with their kind and opacity: at most `max_rows`, none that have faded out.
    pub fn visible_rows(&self) -> Vec<(&str, ChatKind, f32)> {
        let mut out = Vec::new();
        'lines: for line in self.lines.iter().rev() {
            let a = self.alpha(line.age);
            if a <= 0.0 {
                // Older lines are older still, so none of them shows.
                break;
            }
            for row in line.rows.iter().rev() {
                if out.len() >= self.cfg.max_rows {
                    break 'lines;
                }
                out.push((row.as_str(), line.kind, a));
            }
        }
        out.reverse();
        out
    }

    /// The input line as drawn: `[All] > ` or `[Team] > `, then the part of the draft around the cursor that fits one row, with `|`
    /// at the cursor.
    pub fn input_line(&self) -> String {
        let prefix = if self.team { "[Team] > " } else { "[All] > " };
        let room = self
            .cfg
            .wrap_chars
            .saturating_sub(prefix.chars().count())
            .max(2);
        let chars: Vec<char> = self.input.chars().collect();
        let cursor = self.cursor.min(chars.len());
        // Show at most `room - 1` characters and the cursor, scrolled so that the cursor is in view.
        let first = (cursor + 1).saturating_sub(room);
        let end = (first + room - 1).min(chars.len());
        let mut line = String::from(prefix);
        line.extend(&chars[first..cursor]);
        line.push('|');
        line.extend(&chars[cursor..end]);
        line
    }

    /// Append the chat to `list`: the visible rows above the input line (which is drawn only while the input is open).
    pub fn draw_into(&self, list: &mut DrawList) {
        let rows = self.visible_rows();
        let input_h = if self.open { CHAT_ROW_H + 8.0 } else { 0.0 };
        let bottom = CHAT_BOTTOM - input_h;
        for (i, (row, kind, alpha)) in rows.iter().enumerate() {
            let y = bottom - (rows.len() - i) as f32 * CHAT_ROW_H;
            push_quad(
                list,
                CHAT_X,
                y,
                CHAT_W,
                CHAT_ROW_H,
                Color([0.0, 0.0, 0.0, 0.45 * alpha]),
            );
            let base = match kind {
                ChatKind::Player => theme::TEXT,
                ChatKind::Team => Color([0.55, 1.0, 0.6, 1.0]),
                ChatKind::System => Color([1.0, 0.9, 0.5, 1.0]),
            };
            push_text(
                list,
                Vec2 {
                    x: CHAT_X + 8.0,
                    y: y + 1.0,
                },
                (*row).to_owned(),
                CHAT_SIZE,
                faded(base, *alpha),
                TextAlign::Left,
            );
        }
        if self.open {
            push_quad(
                list,
                CHAT_X,
                bottom + 4.0,
                CHAT_W,
                CHAT_ROW_H + 4.0,
                Color([0.05, 0.06, 0.10, 0.92]),
            );
            let color = if self.team {
                Color([0.55, 1.0, 0.6, 1.0])
            } else {
                theme::TEXT
            };
            push_text(
                list,
                Vec2 {
                    x: CHAT_X + 8.0,
                    y: bottom + 7.0,
                },
                self.input_line(),
                CHAT_SIZE,
                color,
                TextAlign::Left,
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Nameplates

/// A teammate to put a nameplate on.
#[derive(Clone, Debug, PartialEq)]
pub struct Nameplate {
    /// The player's id.
    pub id: u32,
    /// The name (cleaned and cut to 16 characters when drawn).
    pub name: String,
    /// Health, in the same unit as `max_health`.
    pub health: f32,
    /// Full health.
    pub max_health: f32,
    /// Armor.
    pub armor: f32,
    /// Full armor (0 for no armor bar).
    pub max_armor: f32,
    /// The player's voice is on.
    pub talking: bool,
    /// The point above the head, projected by the caller.
    pub at: Projected,
}

/// The rules for nameplates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NameplateConfig {
    /// The most plates drawn at once (the nearest ones win).
    pub max_count: usize,
    /// Up to this distance a plate is full size and fully opaque.
    pub near: f32,
    /// Beyond this distance there is no plate.
    pub far: f32,
    /// The scale at `far`.
    pub min_scale: f32,
    /// The opacity at `far`.
    pub min_alpha: f32,
    /// Plates are kept inside this rectangle.
    pub safe: Rect,
}

impl Default for NameplateConfig {
    fn default() -> Self {
        Self {
            max_count: 8,
            near: 15.0,
            far: 60.0,
            min_scale: 0.6,
            min_alpha: 0.35,
            safe: SAFE_AREA,
        }
    }
}

/// Where a nameplate ended up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateLayout {
    /// The index of the player in the list given.
    pub index: usize,
    /// The player's id.
    pub id: u32,
    /// The plate's rectangle (name, health and armor), inside the safe area.
    pub rect: Rect,
    /// Size factor, 1 up close, `min_scale` far away.
    pub scale: f32,
    /// Opacity, 1 up close, `min_alpha` far away.
    pub alpha: f32,
    /// Health as a fraction, 0 to 1.
    pub health: f32,
    /// Armor as a fraction, 0 to 1.
    pub armor: f32,
}

/// Nameplates over teammates' heads.
#[derive(Clone, Copy, Debug, Default)]
pub struct Nameplates {
    cfg: NameplateConfig,
}

const PLATE_W: f32 = 168.0;
const PLATE_H: f32 = 52.0;
/// The longest name on a plate.
const PLATE_NAME_CHARS: usize = 16;

fn fraction(value: f32, max: f32) -> f32 {
    if max > 0.0 { unit(value / max) } else { 0.0 }
}

impl Nameplates {
    /// Nameplates under the given rules.
    pub fn new(cfg: NameplateConfig) -> Self {
        Self { cfg }
    }

    /// Work out which plates are drawn and where. They come **nearest first**. A player is left out when the point is behind the camera,
    /// not finite, off the canvas, or farther than `far`; of the rest only the `max_count` nearest are kept. A plate that would stick
    /// out of the safe area is moved in (the anchor may be near an edge but must be on the canvas).
    pub fn layout_into(&self, players: &[Nameplate], out: &mut Vec<PlateLayout>) {
        out.clear();
        let c = &self.cfg;
        for (index, p) in players.iter().enumerate() {
            let s = p.at.screen;
            let d = p.at.distance;
            let on_canvas = s.x >= 0.0 && s.x <= CANVAS_W && s.y >= 0.0 && s.y <= CANVAS_H;
            if !p.at.in_front || !d.is_finite() || d < 0.0 || !on_canvas {
                continue;
            }
            if d > c.far {
                continue;
            }
            let t = if c.far > c.near {
                unit((d - c.near) / (c.far - c.near))
            } else {
                0.0
            };
            let scale = lerp(1.0, c.min_scale, t);
            let alpha = lerp(1.0, c.min_alpha, t);
            let (w, h) = (PLATE_W * scale, PLATE_H * scale);
            let x = pin(s.x - w / 2.0, c.safe.x, c.safe.x + c.safe.w - w);
            let y = pin(s.y - h, c.safe.y, c.safe.y + c.safe.h - h);
            out.push(PlateLayout {
                index,
                id: p.id,
                rect: Rect { x, y, w, h },
                scale,
                alpha,
                health: fraction(p.health, p.max_health),
                armor: fraction(p.armor, p.max_armor),
            });
        }
        // Nearest first; the sort is stable, so players at the same distance keep the order of the list.
        out.sort_by(|a, b| {
            players[a.index]
                .at
                .distance
                .total_cmp(&players[b.index].at.distance)
        });
        out.truncate(c.max_count);
    }

    /// Append the plates to `list`, farthest first so that the nearest end up on top.
    pub fn draw_into(&self, players: &[Nameplate], list: &mut DrawList) {
        let mut layout = Vec::new();
        self.layout_into(players, &mut layout);
        for l in layout.iter().rev() {
            let p = &players[l.index];
            let r = l.rect;
            let s = l.scale;
            push_quad(
                list,
                r.x,
                r.y,
                r.w,
                r.h,
                Color([0.0, 0.0, 0.0, 0.45 * l.alpha]),
            );
            if p.talking {
                push_quad(list, r.x, r.y, 4.0 * s, r.h, faded(VOICE_ON, l.alpha));
            }
            let dead = l.health <= 0.0;
            let name_color = if dead {
                Color([0.6, 0.6, 0.6, 1.0])
            } else {
                theme::TEXT
            };
            let name = clip_text(&p.name, PLATE_NAME_CHARS);
            push_text(
                list,
                Vec2 {
                    x: r.x + r.w / 2.0,
                    y: r.y,
                },
                name,
                24.0 * s,
                faded(name_color, l.alpha),
                TextAlign::Center,
            );
            let bar_x = r.x + 6.0 * s;
            let bar_w = r.w - 12.0 * s;
            let health_y = r.y + 34.0 * s;
            push_quad(
                list,
                bar_x,
                health_y,
                bar_w,
                10.0 * s,
                faded(Color([0.1, 0.1, 0.1, 1.0]), l.alpha),
            );
            let health_color = if l.health > 0.5 {
                HEALTH_OK
            } else if l.health > 0.25 {
                HEALTH_LOW
            } else {
                HEALTH_CRITICAL
            };
            if l.health > 0.0 {
                push_quad(
                    list,
                    bar_x,
                    health_y,
                    bar_w * l.health,
                    10.0 * s,
                    faded(health_color, l.alpha),
                );
            }
            if l.armor > 0.0 {
                push_quad(
                    list,
                    bar_x,
                    r.y + 46.0 * s,
                    bar_w * l.armor,
                    4.0 * s,
                    faded(ARMOR_BAR, l.alpha),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Ping markers

/// What a ping says (it decides the colour and the symbol).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingKind {
    /// "Look here."
    Look,
    /// "Danger."
    Danger,
    /// "An item."
    Item,
    /// "Go here."
    Go,
}

impl PingKind {
    fn color(self) -> Color {
        match self {
            PingKind::Look => Color([0.35, 0.7, 1.0, 1.0]),
            PingKind::Danger => Color([0.95, 0.25, 0.2, 1.0]),
            PingKind::Item => Color([0.95, 0.8, 0.2, 1.0]),
            PingKind::Go => Color([0.35, 0.85, 0.4, 1.0]),
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            PingKind::Look => "*",
            PingKind::Danger => "!",
            PingKind::Item => "+",
            PingKind::Go => ">",
        }
    }
}

/// One of eight directions, for the arrow of a marker that is off screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Right.
    East,
    /// Right and down.
    SouthEast,
    /// Down.
    South,
    /// Left and down.
    SouthWest,
    /// Left.
    West,
    /// Left and up.
    NorthWest,
    /// Up.
    North,
    /// Right and up.
    NorthEast,
}

impl Direction {
    /// The nearest of the eight directions to `(dx, dy)` (screen coordinates, y down). `(0, 0)` and non-finite vectors give `South`.
    pub fn from_vector(dx: f32, dy: f32) -> Direction {
        if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
            return Direction::South;
        }
        let sector = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() as i32;
        match sector.rem_euclid(8) {
            0 => Direction::East,
            1 => Direction::SouthEast,
            2 => Direction::South,
            3 => Direction::SouthWest,
            4 => Direction::West,
            5 => Direction::NorthWest,
            6 => Direction::North,
            _ => Direction::NorthEast,
        }
    }

    /// The arrow character for it.
    pub fn glyph(self) -> &'static str {
        match self {
            Direction::East => "\u{2192}",
            Direction::SouthEast => "\u{2198}",
            Direction::South => "\u{2193}",
            Direction::SouthWest => "\u{2199}",
            Direction::West => "\u{2190}",
            Direction::NorthWest => "\u{2196}",
            Direction::North => "\u{2191}",
            Direction::NorthEast => "\u{2197}",
        }
    }
}

/// The most pings alive at once; a new one beyond that drops the oldest.
pub const MAX_PINGS: usize = 16;
/// The longest ping label, in characters.
pub const MAX_PING_LABEL_CHARS: usize = 24;
/// Ticks a ping takes to fade out at the end of its life.
pub const PING_FADE_TICKS: u32 = 30;

/// A ping in the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ping {
    /// The marker's id, to match it with a [`PingProjection`].
    pub id: u32,
    /// The player who placed it (a player has one ping at a time).
    pub owner: u32,
    /// That player's name.
    pub owner_name: String,
    /// What it says.
    pub kind: PingKind,
    /// The label ("Ammo here"); empty for none.
    pub label: String,
    /// Ticks until it disappears.
    pub ticks_left: u32,
}

/// Where the caller found the world position of a ping on the screen this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PingProjection {
    /// The marker's id ([`Ping::id`]).
    pub id: u32,
    /// The projection.
    pub at: Projected,
}

/// How a ping is laid out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PingLayout {
    /// The marker's id.
    pub id: u32,
    /// The centre of the marker, inside the safe area.
    pub center: Vec2,
    /// False when the marker is pinned to the edge because its target is off screen or behind the camera.
    pub on_screen: bool,
    /// The direction to turn, when off screen.
    pub arrow: Option<Direction>,
    /// Opacity: 1 until the last [`PING_FADE_TICKS`] of its life.
    pub alpha: f32,
}

/// The pings that are alive.
#[derive(Clone, Debug, Default)]
pub struct PingMarkers {
    pings: Vec<Ping>,
    next_id: u32,
}

const PING_SIZE: f32 = 32.0;

impl PingMarkers {
    /// No pings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Place a ping for `owner`, replacing the one that player had. `label` is cleaned and cut to [`MAX_PING_LABEL_CHARS`]. Returns
    /// its id, or `None` when `ttl_ticks` is 0 (it would not live a tick). Beyond [`MAX_PINGS`] the oldest is dropped.
    pub fn add(
        &mut self,
        owner: u32,
        owner_name: &str,
        kind: PingKind,
        label: &str,
        ttl_ticks: u32,
    ) -> Option<u32> {
        if ttl_ticks == 0 {
            return None;
        }
        self.pings.retain(|p| p.owner != owner);
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.pings.push(Ping {
            id,
            owner,
            owner_name: clip_text(owner_name, MAX_NAME_CHARS),
            kind,
            label: clip_text(label, MAX_PING_LABEL_CHARS),
            ticks_left: ttl_ticks,
        });
        if self.pings.len() > MAX_PINGS {
            self.pings.remove(0);
        }
        Some(id)
    }

    /// The live pings, oldest first.
    pub fn pings(&self) -> &[Ping] {
        &self.pings
    }

    /// One tick: lifetimes run down and the ones that reach 0 go.
    pub fn update(&mut self) {
        for p in &mut self.pings {
            p.ticks_left = p.ticks_left.saturating_sub(1);
        }
        self.pings.retain(|p| p.ticks_left > 0);
    }

    /// Lay out the pings that have a projection (one the caller did not project is not drawn), in the order of the pings.
    ///
    /// A marker whose target is in front of the camera and inside the safe area sits on it. Any other is **pinned to the edge of the
    /// safe area** on the line from the screen centre toward the target, with an arrow along that line. For a target behind the camera
    /// the projected position is mirrored, so the line is turned round: the arrow points the way to turn. A position that is not finite
    /// is not drawn.
    pub fn layout_into(&self, projections: &[PingProjection], out: &mut Vec<PingLayout>) {
        out.clear();
        let pin = Rect {
            x: SAFE_AREA.x + PING_SIZE / 2.0,
            y: SAFE_AREA.y + PING_SIZE / 2.0,
            w: SAFE_AREA.w - PING_SIZE,
            h: SAFE_AREA.h - PING_SIZE,
        };
        let centre = Vec2 {
            x: CANVAS_W / 2.0,
            y: CANVAS_H / 2.0,
        };
        for ping in &self.pings {
            let Some(proj) = projections.iter().find(|p| p.id == ping.id) else {
                continue;
            };
            let s = proj.at.screen;
            if !finite(s) {
                continue;
            }
            let alpha = (ping.ticks_left as f32 / PING_FADE_TICKS as f32).min(1.0);
            let inside =
                s.x >= pin.x && s.x <= pin.x + pin.w && s.y >= pin.y && s.y <= pin.y + pin.h;
            if proj.at.in_front && inside {
                out.push(PingLayout {
                    id: ping.id,
                    center: s,
                    on_screen: true,
                    arrow: None,
                    alpha,
                });
                continue;
            }
            let (mut dx, mut dy) = (s.x - centre.x, s.y - centre.y);
            if !proj.at.in_front {
                (dx, dy) = (-dx, -dy);
            }
            // A target at the very centre, or so near it that the ray would overflow, is straight down.
            if dx.abs().max(dy.abs()) < 1e-6 {
                (dx, dy) = (0.0, 1.0);
            }
            // Where the ray from the centre leaves the pin rectangle.
            let tx = if dx == 0.0 {
                f32::INFINITY
            } else {
                (pin.w / 2.0) / dx.abs()
            };
            let ty = if dy == 0.0 {
                f32::INFINITY
            } else {
                (pin.h / 2.0) / dy.abs()
            };
            let t = tx.min(ty);
            out.push(PingLayout {
                id: ping.id,
                center: Vec2 {
                    x: centre.x + dx * t,
                    y: centre.y + dy * t,
                },
                on_screen: false,
                arrow: Some(Direction::from_vector(dx, dy)),
                alpha,
            });
        }
    }

    /// Append the markers to `list`: a square in the kind's colour with its symbol (or the arrow when pinned), and under or over it
    /// (away from the nearer screen edge) the label and "owner, N m".
    pub fn draw_into(&self, projections: &[PingProjection], list: &mut DrawList) {
        let mut layout = Vec::new();
        self.layout_into(projections, &mut layout);
        for l in &layout {
            let Some(ping) = self.pings.iter().find(|p| p.id == l.id) else {
                continue;
            };
            let c = l.center;
            let half = PING_SIZE / 2.0;
            let color = faded(ping.kind.color(), l.alpha);
            push_quad(list, c.x - half, c.y - half, PING_SIZE, PING_SIZE, color);
            let glyph = match l.arrow {
                Some(d) => d.glyph(),
                None => ping.kind.symbol(),
            };
            push_text(
                list,
                Vec2 {
                    x: c.x,
                    y: c.y - 14.0,
                },
                glyph,
                26.0,
                faded(Color([0.02, 0.02, 0.05, 1.0]), l.alpha),
                TextAlign::Center,
            );
            let distance = projections
                .iter()
                .find(|p| p.id == l.id)
                .map(|p| p.at.distance)
                .filter(|d| d.is_finite() && *d >= 0.0);
            let mut detail = ping.owner_name.clone();
            if let Some(d) = distance {
                if !detail.is_empty() {
                    detail.push_str(", ");
                }
                detail.push_str(&format!("{} m", d.round() as u32));
            }
            // Text goes on the side of the marker that faces the middle of the screen, so that it stays inside.
            let below = c.y < CANVAS_H / 2.0;
            let top = if below {
                c.y + half + 4.0
            } else {
                c.y - half - 4.0 - 52.0
            };
            let (first, second) = if below { (0.0, 28.0) } else { (24.0, 0.0) };
            for (text, size, dy) in [(ping.label.clone(), 22.0, first), (detail, 18.0, second)] {
                if text.is_empty() {
                    continue;
                }
                let w = estimate_text_width(text.chars().count(), size);
                let x = pin(c.x, w / 2.0 + 8.0, CANVAS_W - w / 2.0 - 8.0);
                push_text(
                    list,
                    Vec2 { x, y: top + dy },
                    text,
                    size,
                    faded(theme::TEXT, l.alpha),
                    TextAlign::Center,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Voice indicators

/// The most speakers listed.
pub const MAX_SPEAKERS: usize = 6;
/// Ticks a speaker stays listed after the voice stops, so that a pause between words does not make the row blink.
pub const VOICE_HOLD_TICKS: u32 = 20;
/// A level below this is silence.
pub const VOICE_THRESHOLD: f32 = 0.02;

/// What the voice system reports for one player in one tick.
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceSample {
    /// The player's id.
    pub id: u32,
    /// The name.
    pub name: String,
    /// How loud, 0 to 1 (out of range is clamped, NaN is silence).
    pub level: f32,
    /// This is the local player's own microphone.
    pub local: bool,
}

/// A speaker in the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Speaker {
    /// The player's id.
    pub id: u32,
    /// The name (cleaned and cut).
    pub name: String,
    /// The latest level, 0 to 1 (0 during the hold).
    pub level: f32,
    /// The local player.
    pub local: bool,
    /// Ticks left before the row goes, if the voice has stopped.
    pub hold: u32,
}

/// Who is talking.
#[derive(Clone, Debug, Default)]
pub struct VoiceIndicators {
    speakers: Vec<Speaker>,
}

const VOICE_ROW_H: f32 = 40.0;
const VOICE_W: f32 = 300.0;

impl VoiceIndicators {
    /// Nobody talking.
    pub fn new() -> Self {
        Self::default()
    }

    /// One tick, with what the voice system heard. A player at or above [`VOICE_THRESHOLD`] is talking (the loudest sample counts if
    /// a player appears twice); one who was and is not gets [`VOICE_HOLD_TICKS`] more ticks in the list at level 0. At most
    /// [`MAX_SPEAKERS`] are kept: the quietest (the ones holding, then the lowest levels) go first. The list is ordered by id.
    pub fn update(&mut self, samples: &[VoiceSample]) {
        let mut heard: Vec<(u32, f32, &VoiceSample)> = Vec::new();
        for s in samples {
            let level = unit(s.level);
            if level < VOICE_THRESHOLD {
                continue;
            }
            match heard.iter_mut().find(|h| h.0 == s.id) {
                Some(h) if h.1 >= level => {}
                Some(h) => *h = (s.id, level, s),
                None => heard.push((s.id, level, s)),
            }
        }
        let mut next: Vec<Speaker> = Vec::new();
        for old in &self.speakers {
            if heard.iter().any(|h| h.0 == old.id) {
                continue;
            }
            if old.hold > 0 {
                next.push(Speaker {
                    level: 0.0,
                    hold: old.hold - 1,
                    ..old.clone()
                });
            }
        }
        for (id, level, s) in heard {
            next.push(Speaker {
                id,
                name: clip_text(&s.name, MAX_NAME_CHARS),
                level,
                local: s.local,
                hold: VOICE_HOLD_TICKS,
            });
        }
        while next.len() > MAX_SPEAKERS {
            let quietest = next
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.level.total_cmp(&b.level).then(b.id.cmp(&a.id)))
                .map(|(i, _)| i);
            match quietest {
                Some(i) => next.remove(i),
                None => break,
            };
        }
        next.sort_by_key(|s| s.id);
        self.speakers = next;
    }

    /// The speakers listed, ordered by id.
    pub fn speakers(&self) -> &[Speaker] {
        &self.speakers
    }

    /// Append the list to `list`: a row per speaker with a level meter and the name, at the right edge of the safe area.
    pub fn draw_into(&self, list: &mut DrawList) {
        let x = SAFE_AREA.x + SAFE_AREA.w - VOICE_W;
        for (i, s) in self.speakers.iter().enumerate() {
            let y = 220.0 + i as f32 * (VOICE_ROW_H + 6.0);
            let holding = s.level <= 0.0;
            let alpha = if holding { 0.55 } else { 1.0 };
            push_quad(
                list,
                x,
                y,
                VOICE_W,
                VOICE_ROW_H,
                Color([0.02, 0.02, 0.05, 0.75 * alpha]),
            );
            if s.local {
                push_quad(
                    list,
                    x,
                    y,
                    4.0,
                    VOICE_ROW_H,
                    Color([0.85, 0.45, 0.10, alpha]),
                );
            }
            push_quad(
                list,
                x + 12.0,
                y + 8.0,
                24.0,
                24.0,
                faded(Color([0.12, 0.14, 0.12, 1.0]), alpha),
            );
            push_quad(
                list,
                x + 12.0,
                y + 8.0 + 24.0 * (1.0 - s.level),
                24.0,
                24.0 * s.level,
                faded(VOICE_ON, alpha),
            );
            push_text(
                list,
                Vec2 {
                    x: x + 48.0,
                    y: y + 5.0,
                },
                s.name.clone(),
                26.0,
                faded(theme::TEXT, alpha),
                TextAlign::Left,
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// All of them

/// The five overlays together, drawn in one order: nameplates and pings under the voice list, the chat, and the scoreboard on top.
#[derive(Clone, Debug)]
pub struct CoopOverlays {
    /// The scoreboard.
    pub scoreboard: Scoreboard,
    /// The chat.
    pub chat: ChatBox,
    /// The nameplate rules.
    pub nameplates: Nameplates,
    /// The pings.
    pub pings: PingMarkers,
    /// The voice list.
    pub voice: VoiceIndicators,
}

impl CoopOverlays {
    /// All overlays empty, with the given chat limits and nameplate rules.
    pub fn new(chat: ChatConfig, nameplates: NameplateConfig) -> Self {
        Self {
            scoreboard: Scoreboard::new(),
            chat: ChatBox::new(chat),
            nameplates: Nameplates::new(nameplates),
            pings: PingMarkers::new(),
            voice: VoiceIndicators::new(),
        }
    }

    /// One simulation tick: the chat ages, the pings run down, the voice list follows `voice`.
    pub fn update(&mut self, voice: &[VoiceSample]) {
        self.chat.update();
        self.pings.update();
        self.voice.update(voice);
    }

    /// Append everything that is showing to `list`.
    pub fn draw_into(
        &self,
        players: &[Nameplate],
        projections: &[PingProjection],
        list: &mut DrawList,
    ) {
        self.nameplates.draw_into(players, list);
        self.pings.draw_into(projections, list);
        self.voice.draw_into(list);
        self.chat.draw_into(list);
        self.scoreboard.draw_into(list);
    }
}

impl Default for CoopOverlays {
    fn default() -> Self {
        Self::new(ChatConfig::default(), NameplateConfig::default())
    }
}
