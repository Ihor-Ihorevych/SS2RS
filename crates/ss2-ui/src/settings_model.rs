//! The model behind a **settings screen** (roadmap T10-012, `docs/SETTINGS_SCREEN.md`): tabs of rows, edits that are **pending** until Apply, Revert, Reset to
//! defaults, a list of what changed, stepping a row left or right, the text to show for every row, and a "keep these settings?" countdown for changes that can
//! leave the player with a screen they cannot read.
//!
//! No drawing, no window and no clock: time is the `now` argument (seconds, from any steady source) and a screen draws what [`SettingsScreen::view`] returns. The model
//! knows nothing about *which* settings exist. A row is a name, a label, a [`RowKind`] and a default; the crate that owns the real settings (`ss2_game::settings`)
//! describes them as rows and reads the applied values back. That is why this lives in `ss2-ui` while the quality and audio adapters do not: `ss2-render-api` depends
//! on `ss2-ui`, so `ss2-ui` cannot name `QualitySettings`.
//!
//! Every value is **text**, the same form the existing option sets read and write (`QualitySettings::set`, `AccessibilitySettings::set`), kept in a canonical spelling
//! (`ON` and `true` both become `on`; `0.50` becomes `0.5`). A row **refuses** text that is not a valid value with a message in the same words
//! (`render_scale takes a number from 0.5 to 1`) and the state is left exactly as it was.
//!
//! * `set_text`, `step`, `reset_row` change only the **pending** value. Nothing is applied until [`SettingsScreen::apply`]; [`SettingsScreen::revert`] throws the pending
//!   edits away.
//! * [`SettingsScreen::apply`] makes the pending values the applied ones and returns what changed. If any changed row is **risky** (a resolution, a window mode: something
//!   that can leave a black screen) it also starts a **confirmation**: the caller shows "keep these settings? reverting in N seconds"; [`SettingsScreen::confirm`] keeps
//!   them, and [`SettingsScreen::tick`] (or [`SettingsScreen::revert_applied`]) puts the previous values back and says what to undo. While a confirmation is open the rows
//!   cannot be edited.
//! * Pure and deterministic (no clocks, no hash maps).

use ss2_core::input::NavInput;

/// What a row takes.
#[derive(Clone, Debug, PartialEq)]
pub enum RowKind {
    /// One of these words, lowest first; stepping cycles around.
    Choice(Vec<String>),
    /// A number from `min` to `max` (inclusive); stepping moves by `step` on the grid that starts at `min` and stops at the ends.
    Range {
        /// The smallest value.
        min: f32,
        /// The largest value.
        max: f32,
        /// How far one step moves.
        step: f32,
    },
    /// A whole number from `min` to `max` (inclusive); stepping moves by `step` and stops at the ends.
    Count {
        /// The smallest value.
        min: u32,
        /// The largest value.
        max: u32,
        /// How far one step moves.
        step: u32,
    },
    /// `on` or `off`; stepping either way flips it.
    Toggle,
}

impl RowKind {
    /// What the row takes, in the words of a refusal: `one of off, low, high`, `hold or toggle`, `a number from 0.5 to 1`.
    pub fn expected(&self) -> String {
        match self {
            Self::Choice(words) if words.len() == 2 => format!("{} or {}", words[0], words[1]),
            Self::Choice(words) => format!("one of {}", words.join(", ")),
            Self::Range { min, max, .. } => format!("a number from {min} to {max}"),
            Self::Count { min, max, .. } => format!("a whole number from {min} to {max}"),
            Self::Toggle => "on or off".to_owned(),
        }
    }

    /// The canonical spelling of `text` if it is a valid value (any case, spaces around; `on/true/yes/1` and `off/false/no/0` for a toggle), else `None`.
    pub fn canonical(&self, text: &str) -> Option<String> {
        let t = text.trim();
        match self {
            Self::Choice(words) => words.iter().find(|w| w.eq_ignore_ascii_case(t)).cloned(),
            Self::Range { min, max, .. } => {
                let v: f32 = t.parse().ok()?;
                if !v.is_finite() || v < *min || v > *max {
                    return None;
                }
                // `-0` reads as `0`.
                Some(if v == 0.0 { 0.0 } else { v }.to_string())
            }
            Self::Count { min, max, .. } => {
                let v: u32 = t.parse().ok()?;
                (*min..=*max).contains(&v).then(|| v.to_string())
            }
            Self::Toggle => match t.to_ascii_lowercase().as_str() {
                "on" | "true" | "yes" | "1" => Some("on".to_owned()),
                "off" | "false" | "no" | "0" => Some("off".to_owned()),
                _ => None,
            },
        }
    }

    fn check(&self) -> Result<(), String> {
        let ok = |c: bool, m: &str| if c { Ok(()) } else { Err(m.to_owned()) };
        match self {
            Self::Choice(words) => {
                ok(!words.is_empty(), "a choice needs at least one word")?;
                for (i, w) in words.iter().enumerate() {
                    ok(
                        !w.trim().is_empty() && w.trim() == w,
                        "a choice word is empty or has spaces around it",
                    )?;
                    ok(
                        !words[..i].iter().any(|o| o.eq_ignore_ascii_case(w)),
                        "a choice word appears twice",
                    )?;
                }
                Ok(())
            }
            Self::Range { min, max, step } => {
                ok(
                    min.is_finite() && max.is_finite() && step.is_finite(),
                    "a range has a number that is not finite",
                )?;
                ok(min < max, "a range must have min below max")?;
                ok(
                    *step > 0.0 && *step <= max - min,
                    "a range step must be above 0 and within the range",
                )
            }
            Self::Count { min, max, step } => {
                ok(min < max, "a count must have min below max")?;
                ok(*step >= 1, "a count step must be at least 1")
            }
            Self::Toggle => Ok(()),
        }
    }
}

/// How a value is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ValueFormat {
    /// The value as it is (toggles read `On` and `Off`, words get a capital and spaces for underscores).
    #[default]
    Plain,
    /// A range as a whole percentage: `0.85` reads `85%`.
    Percent,
    /// The value followed by a unit: `90°`, `16x`.
    Suffix(&'static str),
}

/// One row of a settings screen, as the screen's owner describes it.
#[derive(Clone, Debug, PartialEq)]
pub struct RowSpec {
    /// The name the owner reads the value back by (lower case, unique on the screen); also the name in a refusal.
    pub id: String,
    /// What the player sees.
    pub label: String,
    /// What it takes.
    pub kind: RowKind,
    /// The value *Reset to defaults* returns to (any valid spelling).
    pub default: String,
    /// How the value is shown.
    pub format: ValueFormat,
    /// Whether changing it can leave the player unable to see the screen (a resolution, a window mode): it starts a confirmation on apply.
    pub risky: bool,
}

impl RowSpec {
    /// A plain row.
    pub fn new(id: &str, label: &str, kind: RowKind, default: &str) -> Self {
        Self {
            id: id.to_owned(),
            label: label.to_owned(),
            kind,
            default: default.to_owned(),
            format: ValueFormat::Plain,
            risky: false,
        }
    }

    /// Show the value in this way.
    pub fn format(mut self, format: ValueFormat) -> Self {
        self.format = format;
        self
    }

    /// Mark the row as risky.
    pub fn risky(mut self) -> Self {
        self.risky = true;
        self
    }
}

/// A tab: a name and its rows, in screen order.
#[derive(Clone, Debug, PartialEq)]
pub struct TabSpec {
    /// What the player sees on the tab.
    pub name: String,
    /// The rows.
    pub rows: Vec<RowSpec>,
}

impl TabSpec {
    /// A tab.
    pub fn new(name: &str, rows: Vec<RowSpec>) -> Self {
        Self {
            name: name.to_owned(),
            rows,
        }
    }
}

/// Why a screen could not be built or edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// The description is wrong (what is wrong).
    Build(String),
    /// There is no row of that name.
    UnknownRow(String),
    /// The text is not a valid value of the row. The message reads `<row> takes <what it takes>`.
    Refused {
        /// The row.
        row: String,
        /// What it takes.
        expected: String,
    },
    /// A confirmation is open: answer it first.
    Locked,
    /// The time is not a finite number.
    BadTime,
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build(m) => write!(f, "bad settings screen: {m}"),
            Self::UnknownRow(r) => write!(f, "there is no setting called {r:?}"),
            Self::Refused { row, expected } => write!(f, "{row} takes {expected}"),
            Self::Locked => f.write_str("answer the \"keep these settings?\" question first"),
            Self::BadTime => f.write_str("the time is not a finite number"),
        }
    }
}

impl std::error::Error for SettingsError {}

/// A direction to step a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Toward the lower value, the previous word.
    Decrease,
    /// Toward the higher value, the next word.
    Increase,
}

/// One value that differs between two states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    /// The row.
    pub id: String,
    /// The value before.
    pub from: String,
    /// The value after.
    pub to: String,
}

/// What [`SettingsScreen::apply`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplyOutcome {
    /// What was applied, in screen order; empty if nothing was pending.
    pub changes: Vec<Change>,
    /// If a risky row changed: the time at which the new values are put back unless [`SettingsScreen::confirm`] was called.
    pub confirm_until: Option<f32>,
}

/// What [`SettingsScreen::press`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    /// The focus moved to another row.
    Moved,
    /// The focused row's pending value changed.
    Changed,
    /// The focused row was already at the end of its range: nothing changed (a screen may play a "bump").
    Limit,
    /// The player asked to go back; the screen decides what that means (leave, or ask about unsaved edits).
    Back,
}

/// A row as a screen draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct RowView {
    /// The row's name.
    pub id: String,
    /// The label.
    pub label: String,
    /// The pending value, formatted.
    pub value: String,
    /// Whether the pending value differs from the applied one.
    pub changed: bool,
    /// Whether the pending value is the default.
    pub is_default: bool,
    /// Whether the row has the focus.
    pub focused: bool,
    /// Whether stepping down changes the value (false at the lower end of a range).
    pub can_decrease: bool,
    /// Whether stepping up changes the value (false at the upper end of a range).
    pub can_increase: bool,
    /// Whether changing it starts a confirmation.
    pub risky: bool,
}

#[derive(Clone, Debug)]
struct TabInfo {
    name: String,
    first: usize,
    len: usize,
}

#[derive(Clone, Debug)]
struct Confirming {
    before: Vec<String>,
    deadline: f32,
}

/// The settings screen: tabs, rows, pending and applied values. See the module documentation.
///
/// ```
/// use ss2_ui::settings_model::{RowKind, RowSpec, SettingsScreen, Step, TabSpec};
///
/// let rows = vec![
///     RowSpec::new("volume", "Volume", RowKind::Range { min: 0.0, max: 1.0, step: 0.1 }, "0.8"),
///     RowSpec::new("mode", "Mode", RowKind::Choice(vec!["window".into(), "full".into()]), "window").risky(),
/// ];
/// let mut screen = SettingsScreen::new(vec![TabSpec::new("Main", rows)], 15.0).unwrap();
/// screen.step("volume", Step::Increase).unwrap();
/// assert_eq!(screen.pending("volume"), Some("0.9"));
/// assert_eq!(screen.applied("volume"), Some("0.8")); // nothing is applied yet
/// let outcome = screen.apply(100.0).unwrap();
/// assert_eq!(outcome.changes.len(), 1);
/// assert!(outcome.confirm_until.is_none()); // volume is not risky
/// ```
#[derive(Clone, Debug)]
pub struct SettingsScreen {
    tabs: Vec<TabInfo>,
    rows: Vec<RowSpec>,
    applied: Vec<String>,
    pending: Vec<String>,
    defaults: Vec<String>,
    tab: usize,
    focus: usize,
    confirm_secs: f32,
    confirming: Option<Confirming>,
}

impl SettingsScreen {
    /// Build a screen from its tabs. Every row starts at its default (applied and pending); call [`load`](Self::load) to show the current settings.
    /// `confirm_secs` is how long a risky change is shown before it is put back.
    ///
    /// # Errors
    /// [`SettingsError::Build`] if there is no tab, a tab has no row, a name is empty or used twice, a row's kind or default is invalid, or `confirm_secs` is not above 0.
    pub fn new(tabs: Vec<TabSpec>, confirm_secs: f32) -> Result<Self, SettingsError> {
        let bad = |m: String| Err(SettingsError::Build(m));
        if !confirm_secs.is_finite() || confirm_secs <= 0.0 {
            return bad("the confirmation time must be above 0 seconds".to_owned());
        }
        if tabs.is_empty() {
            return bad("there is no tab".to_owned());
        }
        let mut infos = Vec::new();
        let mut rows: Vec<RowSpec> = Vec::new();
        let mut defaults = Vec::new();
        for tab in tabs {
            if tab.rows.is_empty() {
                return bad(format!("tab {:?} has no row", tab.name));
            }
            infos.push(TabInfo {
                name: tab.name,
                first: rows.len(),
                len: tab.rows.len(),
            });
            for mut row in tab.rows {
                if row.id.trim().is_empty() || row.id.trim() != row.id {
                    return bad(format!(
                        "row name {:?} is empty or has spaces around it",
                        row.id
                    ));
                }
                if rows.iter().any(|r| r.id == row.id) {
                    return bad(format!("row name {:?} is used twice", row.id));
                }
                if let Err(m) = row.kind.check() {
                    return bad(format!("row {}: {m}", row.id));
                }
                let Some(default) = row.kind.canonical(&row.default) else {
                    return bad(format!(
                        "row {}: the default {:?} is not valid ({})",
                        row.id,
                        row.default,
                        row.kind.expected()
                    ));
                };
                row.default = default.clone();
                defaults.push(default);
                rows.push(row);
            }
        }
        Ok(Self {
            tabs: infos,
            applied: defaults.clone(),
            pending: defaults.clone(),
            defaults,
            rows,
            tab: 0,
            focus: 0,
            confirm_secs,
            confirming: None,
        })
    }

    fn index(&self, id: &str) -> Result<usize, SettingsError> {
        self.rows
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| SettingsError::UnknownRow(id.to_owned()))
    }

    fn editable(&self) -> Result<(), SettingsError> {
        if self.confirming.is_some() {
            Err(SettingsError::Locked)
        } else {
            Ok(())
        }
    }

    fn refuse(&self, i: usize) -> SettingsError {
        SettingsError::Refused {
            row: self.rows[i].id.clone(),
            expected: self.rows[i].kind.expected(),
        }
    }

    fn change_at(&self, i: usize, from: &[String], to: &[String]) -> Option<Change> {
        (from[i] != to[i]).then(|| Change {
            id: self.rows[i].id.clone(),
            from: from[i].clone(),
            to: to[i].clone(),
        })
    }

    fn diff(&self, from: &[String], to: &[String]) -> Vec<Change> {
        (0..self.rows.len())
            .filter_map(|i| self.change_at(i, from, to))
            .collect()
    }

    // ---------------------------------------------------------------- reading

    /// The tab names, in order.
    pub fn tab_names(&self) -> Vec<&str> {
        self.tabs.iter().map(|t| t.name.as_str()).collect()
    }

    /// The row descriptions of a tab (empty for a tab that does not exist).
    pub fn rows_in(&self, tab: usize) -> &[RowSpec] {
        self.tabs
            .get(tab)
            .map_or(&[][..], |t| &self.rows[t.first..t.first + t.len])
    }

    /// The description of a row.
    pub fn row(&self, id: &str) -> Option<&RowSpec> {
        self.rows.iter().find(|r| r.id == id)
    }

    /// The pending value of a row (what the screen shows), in canonical spelling.
    pub fn pending(&self, id: &str) -> Option<&str> {
        self.index(id).ok().map(|i| self.pending[i].as_str())
    }

    /// The applied value of a row (what the game is using).
    pub fn applied(&self, id: &str) -> Option<&str> {
        self.index(id).ok().map(|i| self.applied[i].as_str())
    }

    /// The default value of a row.
    pub fn default_of(&self, id: &str) -> Option<&str> {
        self.index(id).ok().map(|i| self.defaults[i].as_str())
    }

    /// Whether the pending value of a row differs from the applied one.
    pub fn is_changed(&self, id: &str) -> bool {
        self.index(id)
            .is_ok_and(|i| self.pending[i] != self.applied[i])
    }

    /// Whether any pending value differs from the applied one.
    pub fn has_changes(&self) -> bool {
        self.pending != self.applied
    }

    /// What Apply would change, in screen order.
    pub fn changed(&self) -> Vec<Change> {
        self.diff(&self.applied, &self.pending)
    }

    /// How many rows of a tab have unapplied edits (for a mark on the tab).
    pub fn changed_in_tab(&self, tab: usize) -> usize {
        self.tabs.get(tab).map_or(0, |t| {
            (t.first..t.first + t.len)
                .filter(|&i| self.pending[i] != self.applied[i])
                .count()
        })
    }

    /// The text to show for a row's pending value: `On`, `85%`, `90°`, `Extra large`.
    pub fn display_value(&self, id: &str) -> Option<String> {
        let i = self.index(id).ok()?;
        Some(format_value(&self.rows[i], &self.pending[i]))
    }

    /// Every row of a tab as a screen draws it (empty for a tab that does not exist).
    pub fn view(&self, tab: usize) -> Vec<RowView> {
        let Some(t) = self.tabs.get(tab) else {
            return Vec::new();
        };
        (t.first..t.first + t.len)
            .map(|i| {
                let row = &self.rows[i];
                let (can_decrease, can_increase) = self.can_step(i);
                RowView {
                    id: row.id.clone(),
                    label: row.label.clone(),
                    value: format_value(row, &self.pending[i]),
                    changed: self.pending[i] != self.applied[i],
                    is_default: self.pending[i] == self.defaults[i],
                    focused: i == self.focus, // `focus` is always in the current tab, and a row index is unique
                    can_decrease,
                    can_increase,
                    risky: row.risky,
                }
            })
            .collect()
    }

    fn can_step(&self, i: usize) -> (bool, bool) {
        let text = &self.pending[i];
        match &self.rows[i].kind {
            RowKind::Choice(_) | RowKind::Toggle => (true, true),
            RowKind::Range { min, max, .. } => {
                let v: f32 = text.parse().unwrap_or(*min);
                (v > *min, v < *max)
            }
            RowKind::Count { min, max, .. } => {
                let v: u32 = text.parse().unwrap_or(*min);
                (v > *min, v < *max)
            }
        }
    }

    // ---------------------------------------------------------------- editing

    /// Load the **current** value of a row, as both the applied and the pending value (use it to show the settings the game has).
    ///
    /// # Errors
    /// [`SettingsError::UnknownRow`], [`SettingsError::Refused`] (nothing changes), [`SettingsError::Locked`].
    pub fn load(&mut self, id: &str, text: &str) -> Result<(), SettingsError> {
        self.editable()?;
        let i = self.index(id)?;
        let value = self.rows[i]
            .kind
            .canonical(text)
            .ok_or_else(|| self.refuse(i))?;
        self.applied[i].clone_from(&value);
        self.pending[i] = value;
        Ok(())
    }

    /// Set the pending value of a row from text; returns the canonical spelling. Nothing is applied.
    ///
    /// # Errors
    /// [`SettingsError::UnknownRow`]; [`SettingsError::Refused`] with the row and what it takes (the pending value is left as it was);
    /// [`SettingsError::Locked`] while a confirmation is open.
    pub fn set_text(&mut self, id: &str, text: &str) -> Result<String, SettingsError> {
        self.editable()?;
        let i = self.index(id)?;
        let value = self.rows[i]
            .kind
            .canonical(text)
            .ok_or_else(|| self.refuse(i))?;
        self.pending[i].clone_from(&value);
        Ok(value)
    }

    /// Step the pending value of a row: a choice cycles (and wraps), a range or count moves by its step and **stops at the ends**, a toggle flips.
    /// Returns the new value.
    ///
    /// # Errors
    /// [`SettingsError::UnknownRow`], [`SettingsError::Locked`].
    pub fn step(&mut self, id: &str, step: Step) -> Result<String, SettingsError> {
        self.editable()?;
        let i = self.index(id)?;
        let value = step_value(&self.rows[i].kind, &self.pending[i], step);
        self.pending[i].clone_from(&value);
        Ok(value)
    }

    /// Put one row's pending value back to its default (it still needs Apply).
    ///
    /// # Errors
    /// [`SettingsError::UnknownRow`], [`SettingsError::Locked`].
    pub fn reset_row(&mut self, id: &str) -> Result<(), SettingsError> {
        self.editable()?;
        let i = self.index(id)?;
        self.pending[i].clone_from(&self.defaults[i]);
        Ok(())
    }

    /// Put every pending value back to its default (it still needs Apply).
    ///
    /// # Errors
    /// [`SettingsError::Locked`].
    pub fn reset_all(&mut self) -> Result<(), SettingsError> {
        self.editable()?;
        self.pending.clone_from(&self.defaults);
        Ok(())
    }

    /// Throw the pending edits away: every pending value becomes the applied one again.
    pub fn revert(&mut self) {
        self.pending.clone_from(&self.applied);
    }

    // ---------------------------------------------------------------- apply and confirm

    /// Make the pending values the applied ones and say what changed. If a changed row is risky, a confirmation opens and runs until `now + confirm_secs`.
    ///
    /// # Errors
    /// [`SettingsError::Locked`] while a confirmation is already open; [`SettingsError::BadTime`] if `now` is not finite (nothing is applied).
    pub fn apply(&mut self, now: f32) -> Result<ApplyOutcome, SettingsError> {
        self.editable()?;
        if !now.is_finite() {
            return Err(SettingsError::BadTime);
        }
        let changes = self.changed();
        if changes.is_empty() {
            return Ok(ApplyOutcome {
                changes,
                confirm_until: None,
            });
        }
        let risky = changes
            .iter()
            .any(|c| self.row(&c.id).is_some_and(|r| r.risky));
        let before = std::mem::replace(&mut self.applied, self.pending.clone());
        let confirm_until = risky.then_some(now + self.confirm_secs);
        self.confirming = confirm_until.map(|deadline| Confirming { before, deadline });
        Ok(ApplyOutcome {
            changes,
            confirm_until,
        })
    }

    /// Whether a "keep these settings?" question is open.
    pub fn is_confirming(&self) -> bool {
        self.confirming.is_some()
    }

    /// Seconds left to answer (never negative), or `None` if no question is open.
    pub fn confirm_remaining(&self, now: f32) -> Option<f32> {
        self.confirming
            .as_ref()
            .map(|c| (c.deadline - now).max(0.0))
    }

    /// The question to show: `Keep these settings? Reverting in 12 s`, counting whole seconds **up** (it reads 1 until the time is out), or `None`.
    pub fn confirm_prompt(&self, now: f32) -> Option<String> {
        let left = self.confirm_remaining(now)?;
        Some(format!(
            "Keep these settings? Reverting in {} s",
            left.ceil() as u32
        ))
    }

    /// The player kept the settings. Returns whether a question was open.
    pub fn confirm(&mut self) -> bool {
        self.confirming.take().is_some()
    }

    fn rollback(&mut self) -> Vec<Change> {
        let Some(c) = self.confirming.take() else {
            return Vec::new();
        };
        let undone = self.diff(&self.applied, &c.before);
        self.applied.clone_from(&c.before);
        self.pending = c.before;
        undone
    }

    /// The player chose to go back. The previous values are applied again and pending equals them; returns what to undo (applied to previous), empty if no question was open.
    pub fn revert_applied(&mut self) -> Vec<Change> {
        self.rollback()
    }

    /// Advance the clock. If a question is open and its time is up the previous values are put back: returns what to undo, otherwise `None`.
    /// A time that is not finite does nothing.
    pub fn tick(&mut self, now: f32) -> Option<Vec<Change>> {
        let due = now.is_finite() && self.confirming.as_ref().is_some_and(|c| now >= c.deadline);
        due.then(|| self.rollback())
    }

    // ---------------------------------------------------------------- navigation

    /// The index of the current tab.
    pub fn tab(&self) -> usize {
        self.tab
    }

    /// Show a tab and focus its first row. Out of range does nothing; returns whether the tab changed.
    pub fn select_tab(&mut self, tab: usize) -> bool {
        match self.tabs.get(tab) {
            Some(t) if tab != self.tab => {
                self.tab = tab;
                self.focus = t.first;
                true
            }
            _ => false,
        }
    }

    /// The next tab (wraps from the last to the first).
    pub fn next_tab(&mut self) {
        let n = (self.tab + 1) % self.tabs.len();
        self.select_tab(n);
    }

    /// The previous tab (wraps from the first to the last).
    pub fn prev_tab(&mut self) {
        let n = (self.tab + self.tabs.len() - 1) % self.tabs.len();
        self.select_tab(n);
    }

    /// The name of the focused row.
    pub fn focused(&self) -> &str {
        &self.rows[self.focus].id
    }

    /// Focus a row of the current tab. Returns whether it moved there (false for an unknown row or a row on another tab).
    pub fn focus_row(&mut self, id: &str) -> bool {
        let t = &self.tabs[self.tab];
        match self.index(id) {
            Ok(i) if (t.first..t.first + t.len).contains(&i) => {
                self.focus = i;
                true
            }
            _ => false,
        }
    }

    /// Handle a navigation input: up and down move the focus (wrapping within the tab), left and right step the focused row, activate flips a toggle and
    /// otherwise steps a row up, back is reported.
    ///
    /// # Errors
    /// [`SettingsError::Locked`] when the input would edit a row while a confirmation is open.
    pub fn press(&mut self, input: NavInput) -> Result<Nav, SettingsError> {
        let t = self.tabs[self.tab].clone();
        let at = self.focus - t.first;
        match input {
            NavInput::Up => {
                self.focus = t.first + (at + t.len - 1) % t.len;
                Ok(Nav::Moved)
            }
            NavInput::Down => {
                self.focus = t.first + (at + 1) % t.len;
                Ok(Nav::Moved)
            }
            NavInput::Left | NavInput::Right | NavInput::Activate => {
                let step = if input == NavInput::Left {
                    Step::Decrease
                } else {
                    Step::Increase
                };
                let id = self.rows[self.focus].id.clone();
                let before = self.pending[self.focus].clone();
                let after = self.step(&id, step)?;
                Ok(if after == before {
                    Nav::Limit
                } else {
                    Nav::Changed
                })
            }
            NavInput::Back => Ok(Nav::Back),
        }
    }
}

fn humanize(word: &str) -> String {
    let spaced = word.replace('_', " ");
    let mut chars = spaced.chars();
    chars.next().map_or(String::new(), |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

fn format_value(row: &RowSpec, text: &str) -> String {
    let suffix = match row.format {
        ValueFormat::Suffix(s) => s,
        _ => "",
    };
    match &row.kind {
        RowKind::Toggle => {
            if text == "on" {
                "On".to_owned()
            } else {
                "Off".to_owned()
            }
        }
        RowKind::Choice(_) => format!("{}{suffix}", humanize(text)),
        RowKind::Range { .. } => match (row.format, text.parse::<f32>()) {
            (ValueFormat::Percent, Ok(v)) => format!("{}%", (v * 100.0).round()),
            _ => format!("{text}{suffix}"),
        },
        RowKind::Count { .. } => format!("{text}{suffix}"),
    }
}

fn step_value(kind: &RowKind, current: &str, step: Step) -> String {
    let up = step == Step::Increase;
    match kind {
        RowKind::Toggle => if current == "on" { "off" } else { "on" }.to_owned(),
        RowKind::Choice(words) => {
            let at = words.iter().position(|w| w == current).unwrap_or(0);
            let next = if up {
                (at + 1) % words.len()
            } else {
                (at + words.len() - 1) % words.len()
            };
            words[next].clone()
        }
        RowKind::Range { min, max, step } => {
            let v: f32 = current.parse().unwrap_or(*min);
            // The grid starts at `min`; a value between two grid points moves to the next one in the direction asked.
            let k = (v - min) / step;
            let k = if up {
                (k + 1e-3).floor() + 1.0
            } else {
                (k - 1e-3).ceil() - 1.0
            };
            let moved = ((min + k * step) * 1e4).round() / 1e4;
            let moved = moved.clamp(*min, *max);
            if moved == 0.0 { 0.0 } else { moved }.to_string()
        }
        RowKind::Count { min, max, step } => {
            let v: u32 = current.parse().unwrap_or(*min);
            if up {
                v.saturating_add(*step).min(*max)
            } else {
                v.saturating_sub(*step).max(*min)
            }
            .to_string()
        }
    }
}

#[cfg(test)]
mod tests;
