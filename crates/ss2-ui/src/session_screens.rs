//! The screens that come between playing and the menu (roadmap T10-020): the **death and respawn** screen and the **end-of-level summary**.
//! (The pause menu is [`crate::Menu::open_pause`].)
//!
//! Both are plain state machines like the menu: they take [`UiInput`] and the passage of ticks, give back commands for the app, and draw into a
//! [`DrawList`] in canvas units with the placeholder theme. They know nothing of the game: the summary is given a [`LevelStats`] to show, and the
//! death screen is told how long until a co-op respawn. See `docs/SESSION_SCREENS.md`.

use std::borrow::Cow;

use ss2_core::input::NavInput;

use crate::draw::{DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{CANVAS_W, Rect, Vec2};
use crate::menu::UiInput;
use crate::theme;

const BUTTON_W: f32 = 640.0;
const BUTTON_H: f32 = 72.0;
const BUTTON_GAP: f32 = 16.0;
const LABEL_SIZE: f32 = 40.0;
const TITLE_SIZE: f32 = 96.0;
const LINE_SIZE: f32 = 36.0;

/// A column of centred buttons: focus, the press that has not been released yet, and which one was activated. Shared by both screens.
#[derive(Clone, Debug)]
struct Buttons {
    top: f32,
    count: usize,
    focus: usize,
    pressed: Option<usize>,
}

impl Buttons {
    fn new(top: f32, count: usize) -> Self {
        Self {
            top,
            count,
            focus: 0,
            pressed: None,
        }
    }

    fn rect(&self, i: usize) -> Rect {
        Rect {
            x: (CANVAS_W - BUTTON_W) / 2.0,
            y: self.top + i as f32 * (BUTTON_H + BUTTON_GAP),
            w: BUTTON_W,
            h: BUTTON_H,
        }
    }

    fn hit(&self, p: Vec2) -> Option<usize> {
        (0..self.count).find(|&i| self.rect(i).contains(p))
    }

    /// Feed an input; `Some(i)` when button `i` was activated. The rules are the menu's: up and down wrap, hovering focuses, and a pointer
    /// activates only when released over the button it was pressed on.
    fn handle(&mut self, input: UiInput) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        match input {
            UiInput::Nav(NavInput::Up) => {
                self.focus = (self.focus + self.count - 1) % self.count;
                None
            }
            UiInput::Nav(NavInput::Down) => {
                self.focus = (self.focus + 1) % self.count;
                None
            }
            UiInput::Nav(NavInput::Activate) => Some(self.focus),
            UiInput::Nav(_) => None,
            UiInput::PointerMoved(p) => {
                if let Some(i) = self.hit(p) {
                    self.focus = i;
                }
                None
            }
            UiInput::PointerPressed(p) => {
                self.pressed = self.hit(p);
                if let Some(i) = self.pressed {
                    self.focus = i;
                }
                None
            }
            UiInput::PointerReleased(p) => match (self.pressed.take(), self.hit(p)) {
                (Some(a), Some(b)) if a == b => Some(a),
                _ => None,
            },
        }
    }

    fn draw(&self, labels: &[&'static str], list: &mut DrawList) {
        for (i, label) in labels.iter().enumerate() {
            let r = self.rect(i);
            list.quads.push(Quad {
                rect: r,
                color: if i == self.focus {
                    theme::FOCUSED
                } else {
                    theme::IDLE
                },
            });
            list.texts.push(TextRun {
                pos: Vec2 {
                    x: r.x + r.w / 2.0,
                    y: r.y + 14.0,
                },
                text: Cow::Borrowed(label),
                size: LABEL_SIZE,
                color: theme::TEXT,
                align: TextAlign::Center,
            });
        }
    }
}

fn title(text: &'static str, list: &mut DrawList) {
    list.texts.push(TextRun {
        pos: Vec2 { x: 960.0, y: 160.0 },
        text: Cow::Borrowed(text),
        size: TITLE_SIZE,
        color: theme::TEXT,
        align: TextAlign::Center,
    });
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Death

/// How the player gets back into the game after dying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathMode {
    /// Alone: the player chooses to retry (from the last checkpoint) or to leave.
    Solo,
    /// In a co-op game: the team respawns the player after a countdown, or when a respawn point is available.
    Coop {
        /// Ticks until the automatic respawn; `None` while it is not known (waiting for the others).
        respawn_in_ticks: Option<u32>,
        /// The simulation's tick rate, to show the countdown in seconds.
        ticks_per_second: u32,
    },
}

/// What the app must do after a [`DeathScreen`] event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCommand {
    /// Load the last checkpoint and play on (solo).
    Retry,
    /// The countdown ended: respawn the player (co-op).
    Respawn,
    /// Watch the others instead of waiting (co-op).
    Spectate,
    /// Leave for the main menu.
    QuitToMenu,
}

/// The screen shown when the local player dies.
#[derive(Clone, Debug)]
pub struct DeathScreen {
    mode: DeathMode,
    buttons: Buttons,
    done: bool,
}

impl DeathScreen {
    /// A death screen for `mode` with the first button focused.
    pub fn new(mode: DeathMode) -> Self {
        let count = 2;
        let top = match mode {
            DeathMode::Solo => 380.0,
            DeathMode::Coop { .. } => 460.0,
        };
        Self {
            mode,
            buttons: Buttons::new(top, count),
            done: false,
        }
    }

    /// The mode (with the countdown as it stands).
    pub fn mode(&self) -> DeathMode {
        self.mode
    }

    /// Tell the screen how long until the respawn (co-op), from the server's latest word. Does nothing in a solo game.
    pub fn set_respawn_in(&mut self, ticks: Option<u32>) {
        if let DeathMode::Coop {
            respawn_in_ticks, ..
        } = &mut self.mode
        {
            *respawn_in_ticks = ticks;
            self.done = false;
        }
    }

    /// The whole seconds left in the countdown, rounded up (so that it shows 1 until it reaches 0), if there is one.
    pub fn seconds_left(&self) -> Option<u32> {
        match self.mode {
            DeathMode::Coop {
                respawn_in_ticks: Some(t),
                ticks_per_second,
            } => Some(t.div_ceil(ticks_per_second.max(1))),
            _ => None,
        }
    }

    fn labels(&self) -> [&'static str; 2] {
        match self.mode {
            DeathMode::Solo => ["Retry", "Quit to Menu"],
            DeathMode::Coop { .. } => ["Spectate", "Quit to Menu"],
        }
    }

    /// One simulation tick: counts the co-op countdown down and gives [`DeathCommand::Respawn`], once, when it reaches 0.
    pub fn update(&mut self) -> Option<DeathCommand> {
        let DeathMode::Coop {
            respawn_in_ticks: Some(t),
            ..
        } = &mut self.mode
        else {
            return None;
        };
        *t = t.saturating_sub(1);
        if *t == 0 && !self.done {
            self.done = true;
            return Some(DeathCommand::Respawn);
        }
        None
    }

    /// Feed one input event.
    pub fn handle(&mut self, input: UiInput) -> Option<DeathCommand> {
        let activated = self.buttons.handle(input)?;
        Some(match (self.mode, activated) {
            (DeathMode::Solo, 0) => DeathCommand::Retry,
            (DeathMode::Coop { .. }, 0) => DeathCommand::Spectate,
            _ => DeathCommand::QuitToMenu,
        })
    }

    /// Replace the contents of `list` with the screen. (The countdown line is built from a number, so it allocates a short string.)
    pub fn draw_into(&self, list: &mut DrawList) {
        list.clear();
        title("You Died", list);
        let line = match self.mode {
            DeathMode::Solo => Some(Cow::Borrowed("Retry from the last checkpoint")),
            DeathMode::Coop { .. } => match self.seconds_left() {
                Some(s) => Some(Cow::Owned(format!("Respawning in {s}"))),
                None => Some(Cow::Borrowed("Waiting for your team")),
            },
        };
        if let Some(text) = line {
            list.texts.push(TextRun {
                pos: Vec2 { x: 960.0, y: 290.0 },
                text,
                size: LINE_SIZE,
                color: theme::TEXT,
                align: TextAlign::Center,
            });
        }
        self.buttons.draw(&self.labels(), list);
    }

    /// The screen as a new list.
    pub fn draw(&self) -> DrawList {
        let mut list = DrawList::default();
        self.draw_into(&mut list);
        list
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Summary

/// What the end-of-level screen shows (the game fills it in; T09-023 keeps the counters).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelStats {
    /// The level's name, as the player knows it.
    pub level: String,
    /// Enemies killed.
    pub kills: u32,
    /// Enemies in the level.
    pub kills_total: u32,
    /// Secrets found.
    pub secrets: u32,
    /// Secrets in the level.
    pub secrets_total: u32,
    /// Time played in the level, in seconds.
    pub elapsed_secs: u32,
    /// The time to beat, in seconds, if the level has one.
    pub par_secs: Option<u32>,
    /// The score, if the game keeps one.
    pub score: Option<i64>,
}

/// What the app must do after a [`SummaryScreen`] event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryCommand {
    /// Go on to the next level (or the credits).
    Continue,
}

/// Ticks each row takes to count up, and the delay between the start of one row and the next.
pub const COUNT_TICKS: u32 = 45;
/// Ticks between the start of one row and the next.
pub const ROW_DELAY_TICKS: u32 = 30;

/// The end-of-level screen: the rows count up one after the other, a press skips to the end, and a press after that continues.
#[derive(Clone, Debug)]
pub struct SummaryScreen {
    stats: LevelStats,
    tick: u32,
    buttons: Buttons,
}

impl SummaryScreen {
    /// A summary of `stats`, about to start counting.
    pub fn new(stats: LevelStats) -> Self {
        Self {
            stats,
            tick: 0,
            buttons: Buttons::new(860.0, 1),
        }
    }

    /// The statistics shown.
    pub fn stats(&self) -> &LevelStats {
        &self.stats
    }

    fn rows(&self) -> usize {
        2 + 1 + usize::from(self.stats.par_secs.is_some()) + usize::from(self.stats.score.is_some())
    }

    /// The tick at which every row has finished counting.
    pub fn counting_ticks(&self) -> u32 {
        (self.rows() as u32 - 1) * ROW_DELAY_TICKS + COUNT_TICKS
    }

    /// Whether the numbers are still counting up.
    pub fn counting(&self) -> bool {
        self.tick < self.counting_ticks()
    }

    /// One tick of the count-up.
    pub fn update(&mut self) {
        if self.counting() {
            self.tick += 1;
        }
    }

    /// How far row `row` has counted, 0 to 1.
    fn progress(&self, row: usize) -> f32 {
        let start = row as u32 * ROW_DELAY_TICKS;
        (self.tick.saturating_sub(start) as f32 / COUNT_TICKS as f32).min(1.0)
    }

    fn shown(target: u32, progress: f32) -> u32 {
        (target as f32 * progress).floor() as u32
    }

    fn skip(&mut self) {
        self.tick = self.counting_ticks();
    }

    /// Feed one input event. While the numbers count, a press (Enter or a click on the button) skips to the end instead of continuing.
    pub fn handle(&mut self, input: UiInput) -> Option<SummaryCommand> {
        let pressed_anywhere = matches!(
            input,
            UiInput::Nav(NavInput::Activate) | UiInput::PointerPressed(_)
        );
        if self.counting() {
            if pressed_anywhere {
                self.skip();
            }
            return None;
        }
        self.buttons.handle(input).map(|_| SummaryCommand::Continue)
    }

    /// Replace the contents of `list` with the screen as it is at the current tick. (The rows are built from numbers, so each allocates a short string.)
    pub fn draw_into(&self, list: &mut DrawList) {
        list.clear();
        title("Level Complete", list);
        list.texts.push(TextRun {
            pos: Vec2 { x: 960.0, y: 280.0 },
            text: Cow::Owned(self.stats.level.clone()),
            size: LINE_SIZE,
            color: theme::TEXT,
            align: TextAlign::Center,
        });
        let s = &self.stats;
        let mut rows: Vec<(&'static str, String)> = vec![
            (
                "Kills",
                counted_fraction(Self::shown(s.kills, self.progress(0)), s.kills_total),
            ),
            (
                "Secrets",
                counted_fraction(Self::shown(s.secrets, self.progress(1)), s.secrets_total),
            ),
            ("Time", clock(Self::shown(s.elapsed_secs, self.progress(2)))),
        ];
        let mut next = 3;
        if let Some(par) = s.par_secs {
            rows.push(("Par", clock(Self::shown(par, self.progress(next)))));
            next += 1;
        }
        if let Some(score) = s.score {
            let shown = (score.max(0) as f64 * f64::from(self.progress(next))).floor() as i64;
            rows.push(("Score", shown.to_string()));
        }
        for (i, (label, value)) in rows.into_iter().enumerate() {
            let y = 340.0 + i as f32 * 80.0;
            list.quads.push(Quad {
                rect: Rect {
                    x: 560.0,
                    y,
                    w: 800.0,
                    h: 64.0,
                },
                color: if i % 2 == 0 {
                    theme::IDLE
                } else {
                    theme::TRACK
                },
            });
            list.texts.push(TextRun {
                pos: Vec2 {
                    x: 584.0,
                    y: y + 8.0,
                },
                text: Cow::Borrowed(label),
                size: LABEL_SIZE,
                color: theme::TEXT,
                align: TextAlign::Left,
            });
            list.texts.push(TextRun {
                pos: Vec2 {
                    x: 1000.0,
                    y: y + 8.0,
                },
                text: Cow::Owned(value),
                size: LABEL_SIZE,
                color: theme::TEXT,
                align: TextAlign::Left,
            });
        }
        if !self.counting() {
            self.buttons.draw(&["Continue"], list);
        }
    }

    /// The screen as a new list.
    pub fn draw(&self) -> DrawList {
        let mut list = DrawList::default();
        self.draw_into(&mut list);
        list
    }
}

/// `"12 / 20  60%"` for a finished count; while counting, the percentage follows the number shown. A level with none of something shows `"0 / 0"`.
fn counted_fraction(shown: u32, total: u32) -> String {
    if total == 0 {
        return format!("{shown} / 0");
    }
    let percent = (u64::from(shown) * 100 / u64::from(total)).min(999);
    format!("{shown} / {total}  {percent}%")
}

/// Seconds as `m:ss` (`h:mm:ss` from an hour).
pub fn clock(secs: u32) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{}:{s:02}", secs / 60)
    }
}
