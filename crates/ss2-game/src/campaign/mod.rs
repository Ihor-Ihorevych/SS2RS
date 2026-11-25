//! **Campaign flow** (roadmap T09-025, `docs/CAMPAIGN.md`): the list of levels and how one leads to the next, the level transition (outro movie, load,
//! episode movie, intro movie, spawn), what the players carry from one level into the next, the difficulty that carries with them, what finishing
//! levels unlocks, and the campaign's save note.
//!
//! Everything is pure logic that never touches a device: time is a count of ticks, there is no randomness, no `HashMap`, no clock, so the same events
//! always give the same state, byte for byte. **Nothing in the game calls it yet** (no level is loaded, no movie is played, no inventory is read
//! from a player): it has only been exercised by its own tests and by one data-gated test that builds the list of the real install's levels.
//!
//! * [`list`]: [`LevelList`], the validated, ordered list of levels with their exits (one main exit, optional alternative exits for secret levels).
//! * [`gate`]: [`ExitGate`], the co-op "everyone at the exit" rule with a timeout and the ghosting of late players.
//! * [`carry`]: [`CarryOver`], the per-player snapshot that crosses a level boundary, with a checksummed byte format.
//! * [`unlocks`]: [`UnlockSet`], [`UnlockRules`] and the pure function [`unlocks_after`].
//! * [`state`]: [`Campaign`], the state machine that ties these together, and [`CampaignNote`], what a save keeps.
//!
//! Every rule is either **Spec** (written down in `docs/re/`) or **Chosen** (not in the data, ours): [`Rule`] lists them with their provenance, and a
//! test checks that `docs/CAMPAIGN.md` says the same.

pub mod carry;
mod codec;
pub mod gate;
pub mod list;
pub mod state;
pub mod unlocks;

pub use carry::{CarryError, CarryOver, CarryPolicy, PlayerCarry, Slot};
pub use gate::{ExitGate, GateDecision, GateError, PlayerStatus};
pub use list::{Destination, LevelEntry, LevelKind, LevelList, ListError, Next};
pub use state::{
    Campaign, CampaignNote, Departure, LeaveRequest, Phase, Progress, RestoreError, Stage,
    TransitionError,
};
pub use unlocks::{
    LevelResult, Trigger, Unlock, UnlockError, UnlockKind, UnlockRule, UnlockRules, UnlockSet,
    unlocks_after,
};

pub use crate::difficulty::Provenance;

/// Why bytes written by one of this module's `to_bytes` were refused. The state of the caller is never changed by a refusal, and no input, however
/// damaged, panics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BytesError {
    /// Fewer bytes than the smallest possible value (magic, version and checksum).
    TooShort,
    /// The checksum at the end does not match the bytes before it.
    BadChecksum,
    /// The four-byte tag at the start is not the expected one.
    WrongMagic,
    /// A format version this code does not read (there is no migration).
    UnknownVersion {
        /// The version found.
        found: u16,
    },
    /// The bytes end before the value does.
    Truncated,
    /// Bytes are left after the value.
    Trailing,
    /// A field is outside its limits or inconsistent (the text says which).
    Invalid(&'static str),
}

impl std::fmt::Display for BytesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => f.write_str("too short"),
            Self::BadChecksum => f.write_str("the checksum does not match"),
            Self::WrongMagic => f.write_str("not the expected kind of data"),
            Self::UnknownVersion { found } => write!(f, "unknown format version {found}"),
            Self::Truncated => f.write_str("the bytes end too early"),
            Self::Trailing => f.write_str("bytes left over"),
            Self::Invalid(why) => write!(f, "invalid: {why}"),
        }
    }
}

impl std::error::Error for BytesError {}

/// Every rule of the module, with where it comes from. The doc comments of the code say `Spec` or `Chosen` too; `docs/CAMPAIGN.md` has one row per
/// rule with the same key and provenance, and a test compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// The campaign is the 42 levels whose `.nfo` says `SINGLE; COOP;`, in seven episodes.
    LevelSet,
    /// The order of the levels is the numbering of their ids (`1_1_...` to `7_7_...`): a level leads to the next number, the last of an episode to the
    /// first of the next.
    LevelOrder,
    /// A level may name an intro movie, an outro movie and (the first of an episode) an episode movie; any of them may be missing.
    MoviesPerLevel,
    /// A chapter with a next level ends the level: the outro movie plays and the next level loads.
    OutroThenLoad,
    /// A chapter holds the start inventory of the player.
    StartInventory,
    /// A level id is 1 to 64 characters of letters, digits, `_`, `-` and `.`; the list has 1 to 1,024 levels and ids are unique.
    ListShape,
    /// A list is refused when a next link dangles, the last level follows nothing, the links make a cycle, or a level cannot be reached from the first.
    ListValidation,
    /// A level has a main exit and up to seven alternative exits (secret levels); the list starts at its first entry.
    Exits,
    /// A level is `Main`, `Secret` or `Bonus`; finishing a `Main` level through an exit that ends the campaign completes it.
    LevelKinds,
    /// The stages of a transition are outro, load, episode movie, intro, spawn, and a stage with no movie is skipped.
    StageOrder,
    /// A restart and a loaded save go through load and spawn but replay no movie.
    NoMoviesOnRestart,
    /// A transition may only start from a playing level, through an exit that exists, for the level that is being played, once the gate says go.
    WhenToLeave,
    /// The exit gate waits for every living connected player; dead players do not block it.
    GateEveryone,
    /// The wait ends after a timeout counted from the first arrival, and the timer restarts when nobody is at the exit.
    GateTimeout,
    /// A living player who is not at the exit when the timeout ends is ghosted: carried along with the state they had.
    GhostKeepsState,
    /// A level cannot be left when nobody who is connected is alive.
    NobodyAlive,
    /// The difficulty is unchanged by a transition; it may be changed in the transition phase under its change policy.
    DifficultyCarries,
    /// A carried player keeps health, armor, weapons, ammo and items, each raised to the chapter's start inventory where it is lower.
    CarryFloor,
    /// A player who died, joined late or whose carry is missing starts the next level with the chapter's start inventory.
    DeadAndLateStart,
    /// A restart of the level begins again from what the level was entered with.
    RestartKeepsEntry,
    /// The elapsed time of the campaign only counts while a level is being played.
    ElapsedOnlyInPlay,
    /// Finishing a level unlocks the level its exit leads to.
    UnlockExitTarget,
    /// Rules may unlock levels, cheats and modes when a level is finished, enough secrets are found in it, or the campaign is complete.
    UnlockTriggers,
    /// A save is only written while a level is being played or when the campaign is finished.
    SaveOnlyWhenPlaying,
    /// A note carries a fingerprint of the level list and is refused with another list.
    Fingerprint,
}

impl Rule {
    /// Every rule, in the order of the table in `docs/CAMPAIGN.md`.
    pub const ALL: [Self; 25] = [
        Self::LevelSet,
        Self::LevelOrder,
        Self::MoviesPerLevel,
        Self::OutroThenLoad,
        Self::StartInventory,
        Self::ListShape,
        Self::ListValidation,
        Self::Exits,
        Self::LevelKinds,
        Self::StageOrder,
        Self::NoMoviesOnRestart,
        Self::WhenToLeave,
        Self::GateEveryone,
        Self::GateTimeout,
        Self::GhostKeepsState,
        Self::NobodyAlive,
        Self::DifficultyCarries,
        Self::CarryFloor,
        Self::DeadAndLateStart,
        Self::RestartKeepsEntry,
        Self::ElapsedOnlyInPlay,
        Self::UnlockExitTarget,
        Self::UnlockTriggers,
        Self::SaveOnlyWhenPlaying,
        Self::Fingerprint,
    ];

    /// The stable key used in the documentation table.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::LevelSet => "level-set",
            Self::LevelOrder => "level-order",
            Self::MoviesPerLevel => "movies-per-level",
            Self::OutroThenLoad => "outro-then-load",
            Self::StartInventory => "start-inventory",
            Self::ListShape => "list-shape",
            Self::ListValidation => "list-validation",
            Self::Exits => "exits",
            Self::LevelKinds => "level-kinds",
            Self::StageOrder => "stage-order",
            Self::NoMoviesOnRestart => "no-movies-on-restart",
            Self::WhenToLeave => "when-to-leave",
            Self::GateEveryone => "gate-everyone",
            Self::GateTimeout => "gate-timeout",
            Self::GhostKeepsState => "ghost-keeps-state",
            Self::NobodyAlive => "nobody-alive",
            Self::DifficultyCarries => "difficulty-carries",
            Self::CarryFloor => "carry-floor",
            Self::DeadAndLateStart => "dead-and-late-start",
            Self::RestartKeepsEntry => "restart-keeps-entry",
            Self::ElapsedOnlyInPlay => "elapsed-only-in-play",
            Self::UnlockExitTarget => "unlock-exit-target",
            Self::UnlockTriggers => "unlock-triggers",
            Self::SaveOnlyWhenPlaying => "save-only-when-playing",
            Self::Fingerprint => "fingerprint",
        }
    }

    /// Where the rule comes from. The five that read the data (`docs/re/`) are **Spec**; the rest are **Chosen**.
    #[must_use]
    pub const fn provenance(self) -> Provenance {
        match self {
            Self::LevelSet
            | Self::LevelOrder
            | Self::MoviesPerLevel
            | Self::OutroThenLoad
            | Self::StartInventory => Provenance::Spec,
            _ => Provenance::Chosen,
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
