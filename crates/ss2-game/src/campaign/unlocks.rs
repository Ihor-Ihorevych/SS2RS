//! **Unlocks**: what finishing a level (or finding its secrets) makes available: levels for a level-select list, cheats, game modes.
//!
//! **Spec**: the data has a cheats screen that lists the chapters (`menus-tables.md`: `SCheatsMenuScreen` links to `SChapterListMenuScreen`) and an
//! `SMSEnterUnlockKey` screen, but **no rule** for what unlocks what, and `cutscenes.md` says the roles of the five `medallion_N` clips are unknown. Every
//! rule here is therefore **Chosen**:
//!
//! * Finishing a level unlocks the level its exit leads to (`UnlockExitTarget`); the first level is unlocked from the start.
//! * A table of rules, supplied by the caller, may unlock any level, cheat or mode when a level is finished, when enough of its secrets were found, or
//!   when the campaign is complete (`UnlockTriggers`).
//!
//! [`unlocks_after`] is a pure function of the rules, the level list and what happened in the level (the counters of
//! [`crate::level_progress::LevelSummary`]); [`UnlockSet`] is the persisted set.

use std::collections::BTreeSet;

use super::BytesError;
use super::codec::{Reader, Writer};
use super::list::{Destination, LevelKind, LevelList, ListError, valid_id};
use crate::level_progress::LevelSummary;

/// The most things that can be unlocked at once.
pub const MAX_UNLOCKS: usize = 4096;
/// The most rules in a table.
pub const MAX_RULES: usize = 256;
/// The most things one rule may unlock.
pub const MAX_GRANTS: usize = 16;

const MAGIC: [u8; 4] = *b"S2UL";
const VERSION: u16 = 1;

/// What kind of thing is unlocked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnlockKind {
    /// A level (its id is a level id).
    Level,
    /// A cheat (its id is the cheat's name).
    Cheat,
    /// A game mode (its id is the mode's name).
    Mode,
}

impl UnlockKind {
    const fn tag(self) -> u8 {
        match self {
            Self::Level => 0,
            Self::Cheat => 1,
            Self::Mode => 2,
        }
    }

    const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Level),
            1 => Some(Self::Cheat),
            2 => Some(Self::Mode),
            _ => None,
        }
    }
}

/// One unlockable thing. Ordered by kind, then by id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unlock {
    /// What kind.
    pub kind: UnlockKind,
    /// Its id: 1 to 64 characters of letters, digits, `_`, `-` and `.`.
    pub id: String,
}

impl Unlock {
    /// A level.
    #[must_use]
    pub fn level(id: &str) -> Self {
        Self {
            kind: UnlockKind::Level,
            id: id.to_string(),
        }
    }

    /// A cheat.
    #[must_use]
    pub fn cheat(id: &str) -> Self {
        Self {
            kind: UnlockKind::Cheat,
            id: id.to_string(),
        }
    }

    /// A game mode.
    #[must_use]
    pub fn mode(id: &str) -> Self {
        Self {
            kind: UnlockKind::Mode,
            id: id.to_string(),
        }
    }
}

/// Why a rule table, a result or a set was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnlockError {
    /// A trigger or a granted level names a level that is not in the list.
    UnknownLevel {
        /// The id.
        id: String,
    },
    /// An id that is empty, too long or has a character outside letters, digits, `_`, `-` and `.`.
    BadId {
        /// The id.
        id: String,
    },
    /// A rule that grants nothing.
    NoGrants,
    /// A rule that grants more than [`MAX_GRANTS`] things.
    TooManyGrants,
    /// More than [`MAX_RULES`] rules.
    TooManyRules,
    /// A `SecretsFound` threshold of zero (it would fire on every level).
    ZeroThreshold,
    /// More than [`MAX_UNLOCKS`] things would be unlocked.
    TooManyUnlocks,
    /// More secrets found than the level has.
    BadResult,
    /// The level or its exit is not in the list.
    List(ListError),
}

impl std::fmt::Display for UnlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLevel { id } => write!(f, "no level `{id}`"),
            Self::BadId { id } => write!(f, "`{id}` is not a valid id"),
            Self::NoGrants => f.write_str("a rule grants nothing"),
            Self::TooManyGrants => write!(f, "a rule grants more than {MAX_GRANTS} things"),
            Self::TooManyRules => write!(f, "more than {MAX_RULES} rules"),
            Self::ZeroThreshold => f.write_str("a secrets threshold of zero"),
            Self::TooManyUnlocks => write!(f, "more than {MAX_UNLOCKS} unlocks"),
            Self::BadResult => f.write_str("more secrets found than the level has"),
            Self::List(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UnlockError {}

impl From<ListError> for UnlockError {
    fn from(e: ListError) -> Self {
        Self::List(e)
    }
}

/// When a rule fires. **Chosen.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// The level is finished (through any exit).
    LevelFinished {
        /// The level's id.
        level: String,
    },
    /// The level is finished with at least this many of its secrets found.
    SecretsFound {
        /// The level's id.
        level: String,
        /// At least this many (1 or more).
        at_least: u32,
    },
    /// The level is finished with every secret found (never fires for a level without secrets).
    AllSecrets {
        /// The level's id.
        level: String,
    },
    /// A `Main` level is finished through an exit that ends the campaign.
    CampaignComplete,
}

/// One rule: when it fires, what it unlocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnlockRule {
    /// The trigger.
    pub when: Trigger,
    /// What it unlocks (1 to 16 things).
    pub grants: Vec<Unlock>,
}

/// A validated table of rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnlockRules {
    rules: Vec<UnlockRule>,
}

impl UnlockRules {
    /// No rules besides the built-in one (finishing a level unlocks where its exit leads).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Validate a table against the list.
    ///
    /// # Errors
    /// [`UnlockError`]: too many rules or grants, a rule with no grants, a bad id, a level that is not in the list, a threshold of zero.
    pub fn new(rules: Vec<UnlockRule>, list: &LevelList) -> Result<Self, UnlockError> {
        if rules.len() > MAX_RULES {
            return Err(UnlockError::TooManyRules);
        }
        for r in &rules {
            match &r.when {
                Trigger::LevelFinished { level } | Trigger::AllSecrets { level } => {
                    known_level(list, level)?;
                }
                Trigger::SecretsFound { level, at_least } => {
                    known_level(list, level)?;
                    if *at_least == 0 {
                        return Err(UnlockError::ZeroThreshold);
                    }
                }
                Trigger::CampaignComplete => {}
            }
            if r.grants.is_empty() {
                return Err(UnlockError::NoGrants);
            }
            if r.grants.len() > MAX_GRANTS {
                return Err(UnlockError::TooManyGrants);
            }
            for g in &r.grants {
                if !valid_id(&g.id) {
                    return Err(UnlockError::BadId { id: g.id.clone() });
                }
                if g.kind == UnlockKind::Level {
                    known_level(list, &g.id)?;
                }
            }
        }
        Ok(Self { rules })
    }

    /// The rules.
    #[must_use]
    pub fn rules(&self) -> &[UnlockRule] {
        &self.rules
    }
}

fn known_level(list: &LevelList, id: &str) -> Result<(), UnlockError> {
    if !valid_id(id) {
        return Err(UnlockError::BadId { id: id.to_string() });
    }
    if list.index_of(id).is_none() {
        return Err(UnlockError::UnknownLevel { id: id.to_string() });
    }
    Ok(())
}

/// What happened in a level that was just finished: the facts the unlock rules look at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelResult {
    /// The level's id.
    pub level: String,
    /// The exit that was taken (0 is the main exit).
    pub exit: u8,
    /// Secrets found.
    pub secrets_found: u32,
    /// Secrets the level has.
    pub secrets_total: u32,
}

impl LevelResult {
    /// Take the counters from the level's summary (`level_progress`).
    #[must_use]
    pub fn from_summary(level: &str, exit: u8, summary: &LevelSummary) -> Self {
        Self {
            level: level.to_string(),
            exit,
            secrets_found: summary.secrets,
            secrets_total: summary.secrets_total,
        }
    }

    /// Whether the numbers can be true.
    ///
    /// # Errors
    /// [`UnlockError::BadResult`].
    pub fn validate(&self) -> Result<(), UnlockError> {
        if self.secrets_found > self.secrets_total {
            return Err(UnlockError::BadResult);
        }
        Ok(())
    }
}

/// **Chosen** (`UnlockExitTarget`, `UnlockTriggers`): what finishing a level unlocks, in a fixed order: first the level its exit leads to, then the
/// grants of every rule that fires, in table order, without repeats. It does not look at what is already unlocked ([`UnlockSet::grant`] does).
///
/// # Errors
/// [`UnlockError::BadResult`], or the level or the exit is not in the list.
pub fn unlocks_after(
    rules: &UnlockRules,
    list: &LevelList,
    result: &LevelResult,
) -> Result<Vec<Unlock>, UnlockError> {
    result.validate()?;
    let dest = list.destination(&result.level, result.exit)?;
    let kind = list.get(&result.level).map(|e| e.kind);
    let complete = matches!(dest, Destination::End) && kind == Some(LevelKind::Main);
    let mut out: Vec<Unlock> = Vec::new();
    if let Destination::Level(next) = dest {
        out.push(Unlock::level(&next.id));
    }
    for rule in &rules.rules {
        let fires = match &rule.when {
            Trigger::LevelFinished { level } => *level == result.level,
            Trigger::SecretsFound { level, at_least } => {
                *level == result.level && result.secrets_found >= *at_least
            }
            Trigger::AllSecrets { level } => {
                *level == result.level
                    && result.secrets_total > 0
                    && result.secrets_found == result.secrets_total
            }
            Trigger::CampaignComplete => complete,
        };
        if fires {
            for g in &rule.grants {
                if !out.contains(g) {
                    out.push(g.clone());
                }
            }
        }
    }
    Ok(out)
}

/// The things that are unlocked. Kept in a sorted set, so the same set always gives the same bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnlockSet {
    set: BTreeSet<Unlock>,
}

impl UnlockSet {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The set at the start of a campaign: the first level is unlocked.
    #[must_use]
    pub fn initial(list: &LevelList) -> Self {
        let mut s = Self::new();
        s.set.insert(Unlock::level(&list.first().id));
        s
    }

    /// Whether this thing is unlocked.
    #[must_use]
    pub fn contains(&self, u: &Unlock) -> bool {
        self.set.contains(u)
    }

    /// Whether this level is unlocked.
    #[must_use]
    pub fn level_unlocked(&self, id: &str) -> bool {
        self.set.contains(&Unlock::level(id))
    }

    /// How many things are unlocked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// True when nothing is unlocked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Everything unlocked, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Unlock> {
        self.set.iter()
    }

    /// Unlock things; returns the ones that were **new**, in the order given. Nothing changes when the result would pass [`MAX_UNLOCKS`] or an id
    /// is not valid.
    ///
    /// # Errors
    /// [`UnlockError::BadId`], [`UnlockError::TooManyUnlocks`].
    pub fn grant(
        &mut self,
        items: impl IntoIterator<Item = Unlock>,
    ) -> Result<Vec<Unlock>, UnlockError> {
        let mut fresh: Vec<Unlock> = Vec::new();
        for u in items {
            if !valid_id(&u.id) {
                return Err(UnlockError::BadId { id: u.id });
            }
            if !self.set.contains(&u) && !fresh.contains(&u) {
                fresh.push(u);
            }
        }
        if self.set.len() + fresh.len() > MAX_UNLOCKS {
            return Err(UnlockError::TooManyUnlocks);
        }
        self.set.extend(fresh.iter().cloned());
        Ok(fresh)
    }

    /// The set as bytes: `"S2UL"`, version `u16` (1), the count, then per unlock (ascending) a kind tag and the id, then the CRC-32.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new(MAGIC, VERSION);
        self.write(&mut w);
        w.finish()
    }

    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(u32::try_from(self.set.len()).unwrap_or(u32::MAX));
        for u in &self.set {
            w.u8(u.kind.tag());
            w.str(&u.id);
        }
    }

    /// Read what [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    /// [`BytesError`] for anything damaged, truncated, over a limit, out of order or repeated.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BytesError> {
        let mut r = Reader::open(bytes, MAGIC, VERSION)?;
        let s = Self::read(&mut r)?;
        r.finish()?;
        Ok(s)
    }

    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, BytesError> {
        let n = r.count(MAX_UNLOCKS, 3)?;
        let mut set = BTreeSet::new();
        let mut prev: Option<Unlock> = None;
        for _ in 0..n {
            let kind =
                UnlockKind::from_tag(r.u8()?).ok_or(BytesError::Invalid("unknown unlock kind"))?;
            let id = r.string(super::list::MAX_ID_LEN)?;
            if !valid_id(&id) {
                return Err(BytesError::Invalid("an unlock id is not valid"));
            }
            let u = Unlock { kind, id };
            if prev.as_ref().is_some_and(|p| *p >= u) {
                return Err(BytesError::Invalid("unlocks out of order or repeated"));
            }
            prev = Some(u.clone());
            set.insert(u);
        }
        Ok(Self { set })
    }
}

#[cfg(test)]
#[path = "unlocks_tests.rs"]
mod tests;
