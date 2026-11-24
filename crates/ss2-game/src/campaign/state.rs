//! The **campaign state machine**: which level is being played, how one level hands over to the next, and what a save keeps.
//!
//! A campaign is always in one phase:
//!
//! * **`Playing`**: a level is being played. [`Campaign::leave`] may start a transition (rule `WhenToLeave`), [`Campaign::restart`] may restart the
//!   level, [`Campaign::advance`] counts the time.
//! * **`Transition`**: the stages `Outro` (the leaving level's outro movie), `Load`, `EpisodeMovie`, `Intro` (the entered level's intro movie) and
//!   `Spawn` in that order, each ended by a [`Progress`] event from the game; a stage with no movie is skipped (`StageOrder`). A restart and a loaded
//!   save go through `Load` and `Spawn` only (`NoMoviesOnRestart`).
//! * **`Ending`** and **`Finished`**: the last level's outro, then no more levels.
//!
//! **Spec**: a chapter with a next level ends the level, the outro movie plays and the next level loads (`OutroThenLoad`); the movies come from the
//! level's `.nfo`. **Chosen**: the rest (the order of the stages after the load, the rules for leaving, the restart, the save).

use super::BytesError;
use super::carry::{CarryOver, CarryPolicy, PlayerCarry, Slot};
use super::codec::{Reader, Writer};
use super::gate::{ExitGate, GateDecision, PlayerStatus};
use super::list::{LevelEntry, LevelKind, LevelList, MAX_ID_LEN};
use super::unlocks::{LevelResult, Unlock, UnlockError, UnlockRules, UnlockSet, unlocks_after};
use crate::difficulty::{CampaignDifficulty, Difficulty, DifficultyError};

const MAGIC: [u8; 4] = *b"S2CN";
const VERSION: u16 = 1;

/// A stage of a transition between levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The leaving level's outro movie plays.
    Outro,
    /// The next level loads.
    Load,
    /// The movie of the episode the next level starts plays.
    EpisodeMovie,
    /// The next level's intro movie plays.
    Intro,
    /// The players are placed in the next level.
    Spawn,
}

/// What the game tells the campaign when a stage is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// A movie ended or was skipped (the data does not say whether the player may skip one).
    MovieDone,
    /// The level finished loading.
    Loaded,
    /// The players have been placed.
    Spawned,
}

/// The phase of a campaign. Level numbers are positions in the [`LevelList`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// A level is being played.
    Playing {
        /// The level.
        level: usize,
    },
    /// Between two levels (or before the first, or after a restart).
    Transition {
        /// The level being left (its outro plays); `None` for a new game, a restart and a loaded save.
        from: Option<usize>,
        /// The level being entered.
        to: usize,
        /// The stage in progress.
        stage: Stage,
        /// Whether the movies play (false after a restart or a load).
        movies: bool,
    },
    /// The last level's outro movie is playing; then the campaign is finished.
    Ending {
        /// The last level.
        last: usize,
    },
    /// There is no next level.
    Finished {
        /// The last level played.
        last: usize,
    },
}

/// Why a call was refused. Nothing changes after a refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError {
    /// The call needs a level being played.
    NotPlaying,
    /// The call needs a transition in progress.
    NotInTransition,
    /// The event does not end the stage that is in progress.
    WrongStage {
        /// The stage in progress.
        stage: Stage,
        /// The event received.
        got: Progress,
    },
    /// The result is for another level than the one being played.
    WrongLevel {
        /// The level being played.
        expected: String,
        /// The level the result names.
        got: String,
    },
    /// The level has no such exit.
    NoSuchExit {
        /// The level.
        id: String,
        /// The exit.
        exit: u8,
    },
    /// The exit gate says wait.
    Waiting {
        /// Ticks until the timeout ends, or `None` while nobody is at the exit.
        remaining: Option<u64>,
    },
    /// Nobody who is connected is alive.
    NobodyAlive,
    /// A living player has no carried state in the request.
    MissingCarry {
        /// The slot.
        slot: u8,
    },
    /// The request carries a slot the gate does not have.
    UnknownSlot {
        /// The slot.
        slot: u8,
    },
    /// The result is impossible (more secrets found than the level has).
    BadResult,
    /// The unlock rules could not be applied.
    Unlock(UnlockError),
    /// A save can only be written while a level is being played or when the campaign is finished.
    NotSaveable,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPlaying => f.write_str("no level is being played"),
            Self::NotInTransition => f.write_str("no transition is in progress"),
            Self::WrongStage { stage, got } => {
                write!(f, "{got:?} does not end the stage {stage:?}")
            }
            Self::WrongLevel { expected, got } => {
                write!(
                    f,
                    "the result is for `{got}`, but `{expected}` is being played"
                )
            }
            Self::NoSuchExit { id, exit } => write!(f, "level `{id}` has no exit {exit}"),
            Self::Waiting { remaining } => match remaining {
                Some(t) => write!(f, "waiting for the other players, {t} ticks left"),
                None => f.write_str("nobody is at the exit"),
            },
            Self::NobodyAlive => f.write_str("nobody is alive"),
            Self::MissingCarry { slot } => {
                write!(f, "no carried state for the living player {slot}")
            }
            Self::UnknownSlot { slot } => write!(f, "slot {slot} is not in the gate"),
            Self::BadResult => f.write_str("more secrets found than the level has"),
            Self::Unlock(e) => write!(f, "{e}"),
            Self::NotSaveable => f.write_str("a save is not written during a transition"),
        }
    }
}

impl std::error::Error for TransitionError {}

/// Why a note could not be turned back into a campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreError {
    /// The note was saved with another level list.
    WrongList {
        /// The fingerprint in the note.
        saved: u32,
        /// The fingerprint of the list given.
        current: u32,
    },
    /// The note's level is not in the list.
    UnknownLevel {
        /// The id.
        id: String,
    },
    /// An unlocked level is not in the list.
    UnknownUnlock {
        /// The id.
        id: String,
    },
    /// The bytes are damaged.
    Bytes(BytesError),
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongList { saved, current } => write!(
                f,
                "saved with another level list (fingerprint {saved:08x}, this one is {current:08x})"
            ),
            Self::UnknownLevel { id } => write!(f, "the saved level `{id}` is not in the list"),
            Self::UnknownUnlock { id } => write!(f, "the unlocked level `{id}` is not in the list"),
            Self::Bytes(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RestoreError {}

impl From<BytesError> for RestoreError {
    fn from(e: BytesError) -> Self {
        Self::Bytes(e)
    }
}

/// What a request to leave a level needs.
#[derive(Debug)]
pub struct LeaveRequest<'a> {
    /// The gate of the exit; must say go.
    pub gate: &'a ExitGate,
    /// The state of each living player as they leave (dead players are taken from the gate, disconnected ones are dropped).
    pub carry: CarryOver,
    /// What happened in the level, and which exit is taken.
    pub result: LevelResult,
    /// The unlock rules to apply.
    pub rules: &'a UnlockRules,
}

/// What a successful [`Campaign::leave`] decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    /// The level entered, or `None` when the campaign ends.
    pub to: Option<String>,
    /// Living players who were not at the exit and are carried along (ghosted).
    pub ghosted: Vec<u8>,
    /// Players who were dead.
    pub dead: Vec<u8>,
    /// What the level newly unlocked.
    pub newly_unlocked: Vec<Unlock>,
    /// True when a `Main` level was left through an exit that ends the campaign.
    pub campaign_complete: bool,
}

/// What a save keeps of the campaign. **Chosen**: it does not fit the save header's 256-byte notes, so it is a byte block; embedding it in a save
/// is T08-022b.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignNote {
    /// The fingerprint of the level list the note was saved with.
    pub fingerprint: u32,
    /// The level being played, or the last one when finished.
    pub level: String,
    /// The campaign's difficulty and its change policy.
    pub difficulty: CampaignDifficulty,
    /// What the level was entered with (what a restart returns to), or what the last level handed on when finished.
    pub carry: CarryOver,
    /// What is unlocked.
    pub unlocks: UnlockSet,
    /// The ticks spent playing levels.
    pub elapsed_ticks: u64,
    /// How many times a level was restarted.
    pub restarts: u32,
    /// True when there is no next level.
    pub finished: bool,
}

impl CampaignNote {
    /// The note as bytes: `"S2CN"`, version `u16` (1), the list fingerprint, the level id, the difficulty (its four bytes), the carry-over, the
    /// unlock set, the elapsed ticks, the restarts, the finished flag, the CRC-32. The same note gives the same bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new(MAGIC, VERSION);
        w.u32(self.fingerprint);
        w.str(&self.level);
        w.raw(&self.difficulty.to_bytes());
        self.carry.write(&mut w);
        self.unlocks.write(&mut w);
        w.u64(self.elapsed_ticks);
        w.u32(self.restarts);
        w.bool(self.finished);
        w.finish()
    }

    /// Read what [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    /// [`BytesError`] for anything damaged, truncated, over a limit or inconsistent.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BytesError> {
        let mut r = Reader::open(bytes, MAGIC, VERSION)?;
        let fingerprint = r.u32()?;
        let level = r.string(MAX_ID_LEN)?;
        let difficulty = CampaignDifficulty::from_bytes(r.take(crate::difficulty::CAMPAIGN_BYTES)?)
            .map_err(|_| BytesError::Invalid("the difficulty is not valid"))?;
        let carry = CarryOver::read(&mut r)?;
        let unlocks = UnlockSet::read(&mut r)?;
        let elapsed_ticks = r.u64()?;
        let restarts = r.u32()?;
        let finished = r.bool()?;
        r.finish()?;
        Ok(Self {
            fingerprint,
            level,
            difficulty,
            carry,
            unlocks,
            elapsed_ticks,
            restarts,
            finished,
        })
    }
}

/// The state of a campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Campaign {
    list: LevelList,
    difficulty: CampaignDifficulty,
    phase: Phase,
    /// What the current level was entered with.
    entry_carry: CarryOver,
    /// What the level being entered (or just left, when finished) is handed.
    pending_carry: CarryOver,
    unlocks: UnlockSet,
    elapsed_ticks: u64,
    restarts: u32,
}

impl Campaign {
    /// A new game at the first level of the list, in the `Load` stage (there is no outro to play), with the first level unlocked.
    #[must_use]
    pub fn new(list: LevelList, difficulty: CampaignDifficulty) -> Self {
        let mut difficulty = difficulty;
        difficulty.leave_level();
        let unlocks = UnlockSet::initial(&list);
        Self {
            list,
            difficulty,
            phase: Phase::Transition {
                from: None,
                to: 0,
                stage: Stage::Load,
                movies: true,
            },
            entry_carry: CarryOver::new(),
            pending_carry: CarryOver::new(),
            unlocks,
            elapsed_ticks: 0,
            restarts: 0,
        }
    }

    /// The level list.
    #[must_use]
    pub fn list(&self) -> &LevelList {
        &self.list
    }

    /// The phase.
    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The difficulty and its policy.
    #[must_use]
    pub fn difficulty(&self) -> &CampaignDifficulty {
        &self.difficulty
    }

    /// What is unlocked.
    #[must_use]
    pub fn unlocks(&self) -> &UnlockSet {
        &self.unlocks
    }

    /// The ticks spent playing levels.
    #[must_use]
    pub fn elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks
    }

    /// How many times a level was restarted.
    #[must_use]
    pub fn restarts(&self) -> u32 {
        self.restarts
    }

    /// What the level being played was entered with.
    #[must_use]
    pub fn entry_carry(&self) -> &CarryOver {
        &self.entry_carry
    }

    /// What the level being entered is handed (during a transition).
    #[must_use]
    pub fn pending_carry(&self) -> &CarryOver {
        &self.pending_carry
    }

    /// The level being played, being entered, or (when ending or finished) the last one.
    #[must_use]
    pub fn current_level(&self) -> &LevelEntry {
        let i = match self.phase {
            Phase::Playing { level } => level,
            Phase::Transition { to, .. } => to,
            Phase::Ending { last } | Phase::Finished { last } => last,
        };
        &self.list.entries()[i]
    }

    /// The stage in progress, if there is a transition (the last outro counts as `Outro`).
    #[must_use]
    pub fn stage(&self) -> Option<Stage> {
        match self.phase {
            Phase::Transition { stage, .. } => Some(stage),
            Phase::Ending { .. } => Some(Stage::Outro),
            _ => None,
        }
    }

    /// The movie to play now: the outro of the level being left in the `Outro` stage, the episode movie or the intro of the level being entered.
    #[must_use]
    pub fn current_movie(&self) -> Option<&str> {
        let entries = self.list.entries();
        match self.phase {
            Phase::Transition {
                from,
                to,
                stage,
                movies: true,
            } => match stage {
                Stage::Outro => from.and_then(|f| entries[f].outro_movie.as_deref()),
                Stage::EpisodeMovie => entries[to].episode_movie.as_deref(),
                Stage::Intro => entries[to].intro_movie.as_deref(),
                Stage::Load | Stage::Spawn => None,
            },
            Phase::Ending { last } => entries[last].outro_movie.as_deref(),
            _ => None,
        }
    }

    /// **Chosen** (`ElapsedOnlyInPlay`): time passes; it only counts while a level is being played, and saturates.
    pub fn advance(&mut self, ticks: u64) {
        if matches!(self.phase, Phase::Playing { .. }) {
            self.elapsed_ticks = self.elapsed_ticks.saturating_add(ticks);
        }
    }

    /// Ask for another difficulty (see [`CampaignDifficulty::request_change`]): refused while a level is being played under the default policy,
    /// accepted during a transition.
    ///
    /// # Errors
    /// [`DifficultyError::ChangeRefused`].
    pub fn request_difficulty(&mut self, level: Difficulty) -> Result<bool, DifficultyError> {
        self.difficulty.request_change(level)
    }

    /// What the player in `slot` starts the level being entered with (during a transition) or was given (while playing).
    #[must_use]
    pub fn start_inventory(
        &self,
        slot: u8,
        chapter_start: &PlayerCarry,
        policy: CarryPolicy,
    ) -> PlayerCarry {
        let carry = match self.phase {
            Phase::Transition { .. } => &self.pending_carry,
            _ => &self.entry_carry,
        };
        carry.resolve(slot, chapter_start, policy)
    }

    /// **Chosen** (`WhenToLeave`): leave the level being played through the exit in `request.result`.
    ///
    /// It is refused unless a level is being played, the result is for that level and the exit exists, the result is possible, the gate says go,
    /// and every living player has carried state in the request. Then the difficulty may change again (it is between levels), the unlocks are
    /// applied, the carry-over is kept for the next level (dead players as dead, disconnected ones dropped; `DeadAndLateStart`), and the phase is the
    /// first stage of the transition (or `Ending`/`Finished` for an exit that ends the campaign). Nothing changes when it is refused.
    ///
    /// # Errors
    /// [`TransitionError`].
    pub fn leave(&mut self, request: LeaveRequest<'_>) -> Result<Departure, TransitionError> {
        let Phase::Playing { level } = self.phase else {
            return Err(TransitionError::NotPlaying);
        };
        let entry = &self.list.entries()[level];
        let result = &request.result;
        if result.level != entry.id {
            return Err(TransitionError::WrongLevel {
                expected: entry.id.clone(),
                got: result.level.clone(),
            });
        }
        let Some(dest) = self.list.destination_index(level, result.exit) else {
            return Err(TransitionError::NoSuchExit {
                id: entry.id.clone(),
                exit: result.exit,
            });
        };
        let (ghosted, dead) = match request.gate.decision() {
            GateDecision::Wait { remaining } => return Err(TransitionError::Waiting { remaining }),
            GateDecision::NobodyAlive => return Err(TransitionError::NobodyAlive),
            GateDecision::Go { ghosted, dead } => (ghosted, dead),
        };
        result.validate().map_err(|_| TransitionError::BadResult)?;
        let gate_players = request.gate.players();
        if let Some((slot, _)) = request.carry.iter().find(|(s, _)| *s >= gate_players) {
            return Err(TransitionError::UnknownSlot { slot });
        }
        let mut carry = CarryOver::new();
        for slot in 0..gate_players {
            let value = match request.gate.status(slot) {
                Some(PlayerStatus::Away | PlayerStatus::AtExit) => match request.carry.get(slot) {
                    Some(Slot::Carried(c)) => Slot::Carried(c.clone()),
                    _ => return Err(TransitionError::MissingCarry { slot }),
                },
                Some(PlayerStatus::Dead) => Slot::Dead,
                Some(PlayerStatus::Gone) | None => continue,
            };
            // The values were checked when they were put in the request; a failure here cannot happen, and would leave the campaign unchanged.
            carry
                .set(slot, value)
                .map_err(|_| TransitionError::MissingCarry { slot })?;
        }
        let grants =
            unlocks_after(request.rules, &self.list, result).map_err(TransitionError::Unlock)?;
        let mut unlocks = self.unlocks.clone();
        let newly_unlocked = unlocks.grant(grants).map_err(TransitionError::Unlock)?;
        let campaign_complete = dest.is_none() && entry.kind == LevelKind::Main;
        let has_outro = entry.outro_movie.is_some();
        let to = dest.map(|t| self.list.entries()[t].id.clone());

        self.unlocks = unlocks;
        self.pending_carry = carry;
        self.difficulty.leave_level();
        self.phase = match dest {
            Some(t) => Phase::Transition {
                from: Some(level),
                to: t,
                stage: if has_outro { Stage::Outro } else { Stage::Load },
                movies: true,
            },
            None if has_outro => Phase::Ending { last: level },
            None => Phase::Finished { last: level },
        };
        Ok(Departure {
            to,
            ghosted,
            dead,
            newly_unlocked,
            campaign_complete,
        })
    }

    /// **Chosen** (`RestartKeepsEntry`, `NoMoviesOnRestart`): start the level being played again, with what it was entered with and no movies. The
    /// difficulty may change again.
    ///
    /// # Errors
    /// [`TransitionError::NotPlaying`].
    pub fn restart(&mut self) -> Result<(), TransitionError> {
        let Phase::Playing { level } = self.phase else {
            return Err(TransitionError::NotPlaying);
        };
        self.restarts = self.restarts.saturating_add(1);
        self.pending_carry = self.entry_carry.clone();
        self.difficulty.leave_level();
        self.phase = Phase::Transition {
            from: None,
            to: level,
            stage: Stage::Load,
            movies: false,
        };
        Ok(())
    }

    /// The game reports that the stage in progress is over; returns the new phase.
    ///
    /// # Errors
    /// [`TransitionError::NotInTransition`], or [`TransitionError::WrongStage`] when the event does not end the stage.
    pub fn progress(&mut self, event: Progress) -> Result<Phase, TransitionError> {
        let (from, to, stage, movies) = match self.phase {
            Phase::Transition {
                from,
                to,
                stage,
                movies,
            } => (from, to, stage, movies),
            Phase::Ending { last } => {
                if event != Progress::MovieDone {
                    return Err(TransitionError::WrongStage {
                        stage: Stage::Outro,
                        got: event,
                    });
                }
                self.phase = Phase::Finished { last };
                return Ok(self.phase);
            }
            _ => return Err(TransitionError::NotInTransition),
        };
        let expected = match stage {
            Stage::Outro | Stage::EpisodeMovie | Stage::Intro => Progress::MovieDone,
            Stage::Load => Progress::Loaded,
            Stage::Spawn => Progress::Spawned,
        };
        if event != expected {
            return Err(TransitionError::WrongStage { stage, got: event });
        }
        let entry = &self.list.entries()[to];
        let go = |stage| Phase::Transition {
            from,
            to,
            stage,
            movies,
        };
        let intro_or_spawn = if movies && entry.intro_movie.is_some() {
            go(Stage::Intro)
        } else {
            go(Stage::Spawn)
        };
        self.phase = match stage {
            Stage::Outro => go(Stage::Load),
            Stage::Load if movies && entry.episode_movie.is_some() => go(Stage::EpisodeMovie),
            Stage::Load | Stage::EpisodeMovie => intro_or_spawn,
            Stage::Intro => go(Stage::Spawn),
            Stage::Spawn => {
                self.entry_carry = self.pending_carry.clone();
                self.difficulty.enter_level();
                Phase::Playing { level: to }
            }
        };
        Ok(self.phase)
    }

    /// **Chosen** (`SaveOnlyWhenPlaying`): what a save keeps.
    ///
    /// # Errors
    /// [`TransitionError::NotSaveable`] during a transition or the last outro.
    pub fn note(&self) -> Result<CampaignNote, TransitionError> {
        let (finished, carry) = match self.phase {
            Phase::Playing { .. } => (false, self.entry_carry.clone()),
            Phase::Finished { .. } => (true, self.pending_carry.clone()),
            _ => return Err(TransitionError::NotSaveable),
        };
        Ok(CampaignNote {
            fingerprint: self.list.fingerprint(),
            level: self.current_level().id.clone(),
            difficulty: self.difficulty,
            carry,
            unlocks: self.unlocks.clone(),
            elapsed_ticks: self.elapsed_ticks,
            restarts: self.restarts,
            finished,
        })
    }

    /// Bring a campaign back from a note: it re-enters the saved level through `Load` and `Spawn` with no movies (`NoMoviesOnRestart`), between
    /// levels as far as the difficulty policy is concerned; a finished campaign comes back finished.
    ///
    /// # Errors
    /// [`RestoreError`]: another level list (`Fingerprint`), a level or an unlocked level that is not in the list.
    pub fn restore(list: LevelList, note: &CampaignNote) -> Result<Self, RestoreError> {
        let current = list.fingerprint();
        if note.fingerprint != current {
            return Err(RestoreError::WrongList {
                saved: note.fingerprint,
                current,
            });
        }
        let level = list
            .index_of(&note.level)
            .ok_or_else(|| RestoreError::UnknownLevel {
                id: note.level.clone(),
            })?;
        if let Some(u) = note
            .unlocks
            .iter()
            .find(|u| u.kind == super::unlocks::UnlockKind::Level && list.index_of(&u.id).is_none())
        {
            return Err(RestoreError::UnknownUnlock { id: u.id.clone() });
        }
        let mut difficulty = note.difficulty;
        difficulty.leave_level();
        let phase = if note.finished {
            Phase::Finished { last: level }
        } else {
            Phase::Transition {
                from: None,
                to: level,
                stage: Stage::Load,
                movies: false,
            }
        };
        Ok(Self {
            list,
            difficulty,
            phase,
            entry_carry: note.carry.clone(),
            pending_carry: note.carry.clone(),
            unlocks: note.unlocks.clone(),
            elapsed_ticks: note.elapsed_ticks,
            restarts: note.restarts,
        })
    }

    /// [`Self::restore`] from the bytes of a note.
    ///
    /// # Errors
    /// [`RestoreError::Bytes`] for damaged bytes, or any error of [`Self::restore`].
    pub fn restore_bytes(list: LevelList, bytes: &[u8]) -> Result<Self, RestoreError> {
        Self::restore(list, &CampaignNote::from_bytes(bytes)?)
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
