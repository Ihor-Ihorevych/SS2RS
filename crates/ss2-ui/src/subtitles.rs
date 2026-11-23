//! The timing model behind **subtitles and captions**: a queue of cues, each shown for at least the time it takes to read it, with fades, a limit on
//! simultaneous lines, priorities, late and out-of-order arrival, pause, seek and skip (roadmap T06-012, `docs/SUBTITLES.md`).
//!
//! No drawing, no audio clock and no subtitle file reader: the caller owns the timeline. It adds cues ([`Subtitles::add`]) with start times on that timeline,
//! reports the time that passed ([`Subtitles::advance`]; nothing moves while paused) or jumps ([`Subtitles::seek`], [`Subtitles::skip_all`]), and reads what to
//! show ([`Subtitles::visible`]). How a line looks (size, box, contrast) is the player's choice in
//! [`AccessibilitySettings::subtitle_style`](crate::accessibility::AccessibilitySettings::subtitle_style); [`SubtitleLine::box_alpha`] combines it with the fade.
//!
//! The rules, in short:
//! - A cue is shown for `max(its duration, its reading time)`, and the reading time is at least `min_duration`. Nobody is on screen for less, unless the cue is
//!   dropped (see [`DropReason`]) or a seek lands inside it (then only the rest of it is shown).
//! - At most `max_lines` cues are shown at once. A cue that starts when every line is taken **evicts** the lowest-priority one (the oldest of equals) if its
//!   own priority is *strictly* higher; otherwise it **waits** for a line, at most `max_wait` seconds, and is dropped after that. A waiting cue is shown for its
//!   full time from the moment it gets a line.
//! - A cue added after its start time is shifted to now if it is at most `late_grace` late, and dropped if it is later. Cues may be added in any order.
//! - Time is processed event by event, so the result does not depend on how the caller splits time into steps.

use std::cmp::Ordering;

use crate::accessibility::SubtitleStyle;

/// The most lines a configuration may allow.
pub const MAX_LINES_LIMIT: usize = 8;

/// The most events kept until [`Subtitles::take_events`]; the oldest are lost beyond this.
pub const MAX_EVENTS: usize = 256;

/// A word is counted as this many characters when the text has few spaces (languages written without them), so that its reading time is not one word.
pub const CHARS_PER_WORD: usize = 6;

/// How subtitles are timed. Odd values (NaN, negative, huge) are replaced or clamped by [`SubtitleConfig::sanitized`], which [`Subtitles::new`] applies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubtitleConfig {
    /// The most cues shown at once, 1 to [`MAX_LINES_LIMIT`].
    pub max_lines: usize,
    /// How fast people are assumed to read, in words a second (0.5 to 10).
    pub words_per_second: f64,
    /// No cue is shown for less than this many seconds (0 to 30).
    pub min_duration: f64,
    /// Seconds the alpha takes to rise from 0 to 1 (0 to 5; shortened to half the cue for a very short one).
    pub fade_in: f64,
    /// Seconds the alpha takes to fall from 1 to 0 (0 to 5; shortened likewise).
    pub fade_out: f64,
    /// A cue added this late (seconds, 0 to 10) or less is shifted to now; later than that it is dropped.
    pub late_grace: f64,
    /// A cue that cannot get a line waits this long (seconds, 0 to 30) before it is dropped.
    pub max_wait: f64,
    /// The most cues waiting to start, 1 to 256.
    pub max_pending: usize,
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            max_lines: 3,
            words_per_second: 3.0,
            min_duration: 1.0,
            fade_in: 0.15,
            fade_out: 0.25,
            late_grace: 0.5,
            max_wait: 1.5,
            max_pending: 64,
        }
    }
}

fn sane(value: f64, default: f64, lo: f64, hi: f64) -> f64 {
    if value.is_finite() {
        value.clamp(lo, hi)
    } else {
        default
    }
}

impl SubtitleConfig {
    /// The same configuration with every value inside its range (not-a-number and infinities become the default).
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        Self {
            max_lines: self.max_lines.clamp(1, MAX_LINES_LIMIT),
            words_per_second: sane(self.words_per_second, d.words_per_second, 0.5, 10.0),
            min_duration: sane(self.min_duration, d.min_duration, 0.0, 30.0),
            fade_in: sane(self.fade_in, d.fade_in, 0.0, 5.0),
            fade_out: sane(self.fade_out, d.fade_out, 0.0, 5.0),
            late_grace: sane(self.late_grace, d.late_grace, 0.0, 10.0),
            max_wait: sane(self.max_wait, d.max_wait, 0.0, 30.0),
            max_pending: self.max_pending.clamp(1, 256),
        }
    }
}

/// One caption.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    /// What to show. Empty (or only spaces) text is refused.
    pub text: String,
    /// Who speaks, if it should be shown.
    pub speaker: Option<String>,
    /// When it should appear, on the caller's timeline (seconds).
    pub start: f64,
    /// How long it should stay (seconds); 0 means "as long as it takes to read". Never shorter than the reading time.
    pub duration: f64,
    /// Higher wins a line when all are taken. Default 0.
    pub priority: i32,
}

impl Cue {
    /// A cue with no speaker, the default priority and a duration of "as long as it takes to read".
    pub fn new(text: impl Into<String>, start: f64) -> Self {
        Self {
            text: text.into(),
            speaker: None,
            start,
            duration: 0.0,
            priority: 0,
        }
    }

    /// Sets the speaker.
    pub fn speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = Some(speaker.into());
        self
    }

    /// Sets the duration.
    pub fn duration(mut self, seconds: f64) -> Self {
        self.duration = seconds;
        self
    }

    /// Sets the priority.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// A cue that was added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CueId(pub u64);

/// Why a cue was not (or no longer) going to be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReason {
    /// Empty text, or a start or duration that is not a finite number (or a negative duration).
    Invalid,
    /// Added more than `late_grace` seconds after its start.
    Late,
    /// The queue already held `max_pending` cues.
    QueueFull,
    /// Waited `max_wait` seconds for a line and none became free.
    Overflow,
    /// Pushed off its line by a cue of strictly higher priority.
    Evicted,
    /// Passed over by a seek or a skip.
    Skipped,
}

impl std::fmt::Display for DropReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "the cue is empty or has a bad time",
            Self::Late => "the cue arrived too late",
            Self::QueueFull => "too many cues are waiting",
            Self::Overflow => "no line became free in time",
            Self::Evicted => "a cue of higher priority took its line",
            Self::Skipped => "the cue was skipped",
        })
    }
}

impl std::error::Error for DropReason {}

/// What happened to a cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// It got a line.
    Started,
    /// It was shown for its whole time.
    Finished,
    /// It was dropped (before or while being shown).
    Dropped(DropReason),
    /// A backward seek moved it before its start: it is waiting again and will start again.
    Rewound,
}

/// A change in what is shown, read with [`Subtitles::take_events`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubtitleEvent {
    /// The cue.
    pub id: CueId,
    /// The time on the timeline when it happened.
    pub at: f64,
    /// What happened.
    pub kind: EventKind,
}

/// A cue on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubtitleLine<'a> {
    /// The cue.
    pub id: CueId,
    /// The text.
    pub text: &'a str,
    /// The speaker.
    pub speaker: Option<&'a str>,
    /// Opacity from 0 to 1 (the fades).
    pub alpha: f32,
    /// The cue's priority.
    pub priority: i32,
    /// 0 for the newest line (the bottom), 1 for the one before it pushed up, and so on.
    pub row: usize,
    /// Seconds since it got its line.
    pub shown_for: f64,
}

impl SubtitleLine<'_> {
    /// The alpha of the box behind this line: the player's box opacity times the fade, so the box fades with the text.
    pub fn box_alpha(&self, style: &SubtitleStyle) -> f32 {
        (self.alpha * style.background_alpha).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug)]
struct Pending {
    id: CueId,
    cue: Cue,
    /// The time it wants a line (may have been moved from the cue's own).
    start: f64,
    /// How long it stays once it has one.
    eff: f64,
}

#[derive(Clone, Debug)]
struct Active {
    id: CueId,
    cue: Cue,
    shown_from: f64,
    eff: f64,
}

impl Active {
    fn end(&self) -> f64 {
        self.shown_from + self.eff
    }
}

/// The order cues start in: earliest, then highest priority, then first added.
fn start_order(a: &Pending, b: &Pending) -> Ordering {
    a.start
        .total_cmp(&b.start)
        .then(b.cue.priority.cmp(&a.cue.priority))
        .then(a.id.cmp(&b.id))
}

/// The subtitle queue. See the [module](self) for the rules.
///
/// ```
/// use ss2_ui::subtitles::{Cue, SubtitleConfig, Subtitles};
///
/// let mut subs = Subtitles::new(SubtitleConfig::default());
/// subs.add(Cue::new("Here they come!", 1.0).speaker("Sam")).unwrap();
/// subs.advance(0.5);
/// assert!(subs.visible().is_empty());      // not yet
/// subs.advance(1.0);                        // t = 1.5: half-way through the fade-in
/// let line = subs.visible()[0];
/// assert_eq!(line.text, "Here they come!");
/// assert!(line.alpha > 0.0 && line.alpha <= 1.0);
/// subs.advance(5.0);
/// assert!(subs.visible().is_empty());      // read and gone
/// ```
#[derive(Clone, Debug)]
pub struct Subtitles {
    config: SubtitleConfig,
    now: f64,
    paused: bool,
    next_id: u64,
    /// Sorted by [`start_order`].
    pending: Vec<Pending>,
    active: Vec<Active>,
    events: Vec<SubtitleEvent>,
}

impl Subtitles {
    /// An empty queue at time 0, with the configuration sanitized.
    pub fn new(config: SubtitleConfig) -> Self {
        Self {
            config: config.sanitized(),
            now: 0.0,
            paused: false,
            next_id: 0,
            pending: Vec::new(),
            active: Vec::new(),
            events: Vec::new(),
        }
    }

    /// The configuration in force (sanitized).
    pub fn config(&self) -> &SubtitleConfig {
        &self.config
    }

    /// The current time on the timeline.
    pub fn now(&self) -> f64 {
        self.now
    }

    /// Whether time is stopped.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Stops or restarts time. While paused [`advance`](Self::advance) does nothing and the alphas stay where they are; adding, seeking and skipping still work.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// How many cues are on screen.
    pub fn active_len(&self) -> usize {
        self.active.len()
    }

    /// How many cues have not started (including those waiting for a line).
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Nothing is shown and nothing is waiting.
    pub fn is_idle(&self) -> bool {
        self.active.is_empty() && self.pending.is_empty()
    }

    /// How long this text takes to read: its words (or [`CHARS_PER_WORD`]-character groups, whichever are more) at the configured speed, but not less than
    /// `min_duration`.
    pub fn reading_time(&self, text: &str) -> f64 {
        let words = text.split_whitespace().count();
        let chars = text.chars().filter(|c| !c.is_whitespace()).count();
        let units = words.max(chars.div_ceil(CHARS_PER_WORD));
        (units as f64 / self.config.words_per_second).max(self.config.min_duration)
    }

    /// How long a cue is shown once it has a line: its own duration, or the reading time if that is longer.
    pub fn effective_duration(&self, cue: &Cue) -> f64 {
        cue.duration.max(self.reading_time(&cue.text))
    }

    /// Adds a cue. Cues may come in any order and at any time: a start in the past is shifted to now if within `late_grace`, otherwise refused with
    /// [`DropReason::Late`]. Returns its id, or why it was refused (nothing is queued then and no event is recorded).
    pub fn add(&mut self, cue: Cue) -> Result<CueId, DropReason> {
        if cue.text.trim().is_empty()
            || !cue.start.is_finite()
            || !cue.duration.is_finite()
            || cue.duration < 0.0
        {
            return Err(DropReason::Invalid);
        }
        if self.now - cue.start > self.config.late_grace {
            return Err(DropReason::Late);
        }
        let start = cue.start.max(self.now);
        if self.pending.len() >= self.config.max_pending {
            return Err(DropReason::QueueFull);
        }
        let id = CueId(self.next_id);
        self.next_id += 1;
        let eff = self.effective_duration(&cue);
        self.insert_pending(Pending {
            id,
            cue,
            start,
            eff,
        });
        self.settle();
        Ok(id)
    }

    fn insert_pending(&mut self, p: Pending) {
        let at = self.pending.partition_point(|q| start_order(q, &p).is_lt());
        self.pending.insert(at, p);
    }

    fn push_event(&mut self, id: CueId, kind: EventKind) {
        if self.events.len() >= MAX_EVENTS {
            self.events.remove(0);
        }
        self.events.push(SubtitleEvent {
            id,
            at: self.now,
            kind,
        });
    }

    /// The events since the last call, oldest first (at most [`MAX_EVENTS`]; older ones are lost).
    pub fn take_events(&mut self) -> Vec<SubtitleEvent> {
        std::mem::take(&mut self.events)
    }

    /// Gives `p` a line from `shown_from` if one is free or can be taken from a cue of strictly lower priority; otherwise hands it back.
    fn try_activate(&mut self, p: Pending, shown_from: f64) -> Result<(), Pending> {
        if self.active.len() >= self.config.max_lines {
            let victim = self
                .active
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.cue
                        .priority
                        .cmp(&b.cue.priority)
                        .then(a.shown_from.total_cmp(&b.shown_from))
                        .then(a.id.cmp(&b.id))
                })
                .map(|(i, _)| i);
            match victim {
                Some(i) if self.active[i].cue.priority < p.cue.priority => {
                    let gone = self.active.remove(i);
                    self.push_event(gone.id, EventKind::Dropped(DropReason::Evicted));
                }
                _ => return Err(p),
            }
        }
        self.push_event(p.id, EventKind::Started);
        self.active.push(Active {
            id: p.id,
            cue: p.cue,
            shown_from,
            eff: p.eff,
        });
        Ok(())
    }

    /// Brings the state up to date with `self.now`: retires finished cues, then starts, or gives up on, the cues whose time has come.
    fn settle(&mut self) {
        let now = self.now;
        let mut i = 0;
        while i < self.active.len() {
            if self.active[i].end() <= now {
                let done = self.active.remove(i);
                self.push_event(done.id, EventKind::Finished);
            } else {
                i += 1;
            }
        }
        let ready = self.pending.partition_point(|p| p.start <= now);
        let ready: Vec<Pending> = self.pending.drain(..ready).collect();
        let mut waiting = Vec::new();
        for p in ready {
            if let Err(p) = self.try_activate(p, now) {
                if now >= p.start + self.config.max_wait {
                    self.push_event(p.id, EventKind::Dropped(DropReason::Overflow));
                } else {
                    waiting.push(p);
                }
            }
        }
        waiting.append(&mut self.pending);
        self.pending = waiting;
    }

    /// The next time anything changes: a cue ending, starting, or giving up waiting.
    fn next_event(&self) -> Option<f64> {
        let ends = self.active.iter().map(Active::end);
        let starts = self.pending.iter().map(|p| {
            if p.start > self.now {
                p.start
            } else {
                p.start + self.config.max_wait
            }
        });
        ends.chain(starts).min_by(f64::total_cmp)
    }

    /// Lets `dt` seconds pass (nothing if paused, or if `dt` is not a positive finite number). Cues start and end at their exact times however large `dt` is.
    pub fn advance(&mut self, dt: f64) {
        if self.paused || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let target = self.now + dt;
        while let Some(t) = self.next_event() {
            if t > target {
                break;
            }
            self.now = t;
            self.settle();
        }
        self.now = target;
    }

    /// Moves the timeline to `to` (ignored if it is not finite); works while paused.
    ///
    /// Forwards (a cutscene skip, a chapter jump): cues that would have ended are dropped as [`DropReason::Skipped`], not shown; cues the jump lands inside are
    /// shown for the **rest** of their time (without fading in again); cues still to come stay queued. Backwards: cues that had started after `to` go back to
    /// waiting ([`EventKind::Rewound`]) and start again; cues already finished are **not** brought back.
    pub fn seek(&mut self, to: f64) {
        if !to.is_finite() {
            return;
        }
        if to >= self.now {
            self.now = to;
            let mut i = 0;
            while i < self.active.len() {
                if self.active[i].end() <= to {
                    let gone = self.active.remove(i);
                    self.push_event(gone.id, EventKind::Dropped(DropReason::Skipped));
                } else {
                    i += 1;
                }
            }
            let ready = self.pending.partition_point(|p| p.start <= to);
            let ready: Vec<Pending> = self.pending.drain(..ready).collect();
            for p in ready {
                let from = p.start;
                if from + p.eff <= to {
                    self.push_event(p.id, EventKind::Dropped(DropReason::Skipped));
                } else if let Err(p) = self.try_activate(p, from) {
                    self.push_event(p.id, EventKind::Dropped(DropReason::Skipped));
                }
            }
        } else {
            self.now = to;
            let mut i = 0;
            while i < self.active.len() {
                if self.active[i].shown_from > to {
                    let a = self.active.remove(i);
                    self.push_event(a.id, EventKind::Rewound);
                    self.insert_pending(Pending {
                        id: a.id,
                        start: a.shown_from,
                        eff: a.eff,
                        cue: a.cue,
                    });
                } else {
                    i += 1;
                }
            }
            self.settle();
        }
    }

    /// Drops everything, shown or waiting, as [`DropReason::Skipped`] (the player skipped the cutscene). Returns how many cues that was. Time is unchanged.
    pub fn skip_all(&mut self) -> usize {
        let active = std::mem::take(&mut self.active);
        let pending = std::mem::take(&mut self.pending);
        let n = active.len() + pending.len();
        for a in active {
            self.push_event(a.id, EventKind::Dropped(DropReason::Skipped));
        }
        for p in pending {
            self.push_event(p.id, EventKind::Dropped(DropReason::Skipped));
        }
        n
    }

    fn alpha_of(&self, a: &Active) -> f32 {
        let elapsed = self.now - a.shown_from; // never negative: a line has been on since `shown_from <= now`
        let remaining = a.end() - self.now; // always positive: a cue whose end has come is retired
        let fade_in = self.config.fade_in.min(a.eff / 2.0);
        let fade_out = self.config.fade_out.min(a.eff / 2.0);
        // A fade of 0 makes its ratio infinite (or NaN at the very start), and `f64::min` ignores NaN, so that side drops out and the other, or 1, decides.
        let up = elapsed / fade_in;
        let down = remaining / fade_out;
        up.min(down).min(1.0) as f32
    }

    /// What is on screen now, oldest first (the top line first); `row` 0 is the newest.
    pub fn visible(&self) -> Vec<SubtitleLine<'_>> {
        let mut order: Vec<&Active> = self.active.iter().collect();
        order.sort_by(|a, b| a.shown_from.total_cmp(&b.shown_from).then(a.id.cmp(&b.id)));
        let n = order.len();
        order
            .into_iter()
            .enumerate()
            .map(|(i, a)| SubtitleLine {
                id: a.id,
                text: &a.cue.text,
                speaker: a.cue.speaker.as_deref(),
                alpha: self.alpha_of(a),
                priority: a.cue.priority,
                row: n - 1 - i,
                shown_for: self.now - a.shown_from,
            })
            .collect()
    }
}
