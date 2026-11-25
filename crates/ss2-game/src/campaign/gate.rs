//! The **exit gate**: the co-op rule for leaving a level. All **Chosen**: the data says nothing about how a co-op game waits at the exit
//! (`game-modes.md`: the co-op rules are not in the data).
//!
//! * The gate waits until every player who is connected and alive is at the exit (`GateEveryone`). Dead players do not block it; disconnected
//!   players do not count.
//! * The first arrival starts a timer of `timeout_ticks`; when it runs out, the level is left without those who are still away (`GateTimeout`).
//!   The timer only runs while at least one player is at the exit, and restarts from zero when nobody is.
//! * A living player who is not at the exit when the timeout ends is **ghosted** (`GhostKeepsState`): they are carried to the next level with the
//!   state they had, and the list of them is handed to the game (so it can show that they were left behind).
//! * With nobody connected and alive the gate never opens (`NobodyAlive`).

use crate::level_progress::MAX_PLAYERS;

/// Where a player is, for the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerStatus {
    /// Connected, alive, not at the exit.
    Away,
    /// Connected, alive, at the exit.
    AtExit,
    /// Connected and dead (waiting to respawn); does not block the gate.
    Dead,
    /// Not connected; does not count.
    Gone,
}

/// What the gate says now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateDecision {
    /// Nobody may leave yet. `remaining` is the ticks left of the timeout, or `None` while nobody is at the exit (no timer is running).
    Wait {
        /// Ticks until the timeout ends.
        remaining: Option<u64>,
    },
    /// Leave now.
    Go {
        /// Living players who are not at the exit (slots, ascending).
        ghosted: Vec<u8>,
        /// Dead players (slots, ascending).
        dead: Vec<u8>,
    },
    /// Nobody who is connected is alive.
    NobodyAlive,
}

/// Why a gate could not be made or changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateError {
    /// Zero players, or more than [`MAX_PLAYERS`].
    BadPlayerCount,
    /// A slot at or past the player count.
    NoSuchPlayer {
        /// The slot.
        slot: u8,
    },
}

impl std::fmt::Display for GateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadPlayerCount => write!(f, "the player count is not 1 to {MAX_PLAYERS}"),
            Self::NoSuchPlayer { slot } => write!(f, "no player in slot {slot}"),
        }
    }
}

impl std::error::Error for GateError {}

/// The gate of one level exit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitGate {
    status: Vec<PlayerStatus>,
    timeout_ticks: u64,
    waited: u64,
}

impl ExitGate {
    /// A gate for `players` slots (1 to 64), all connected, alive and away, with a timeout of `timeout_ticks` (0 = go as soon as one player is at
    /// the exit).
    ///
    /// # Errors
    /// [`GateError::BadPlayerCount`].
    pub fn new(players: u8, timeout_ticks: u64) -> Result<Self, GateError> {
        if players == 0 || players > MAX_PLAYERS {
            return Err(GateError::BadPlayerCount);
        }
        Ok(Self {
            status: vec![PlayerStatus::Away; usize::from(players)],
            timeout_ticks,
            waited: 0,
        })
    }

    /// How many slots there are.
    #[must_use]
    pub fn players(&self) -> u8 {
        u8::try_from(self.status.len()).unwrap_or(u8::MAX)
    }

    /// The status of a slot.
    #[must_use]
    pub fn status(&self, slot: u8) -> Option<PlayerStatus> {
        self.status.get(usize::from(slot)).copied()
    }

    /// The timeout, in ticks.
    #[must_use]
    pub fn timeout_ticks(&self) -> u64 {
        self.timeout_ticks
    }

    /// Set where a player is. When nobody is at the exit afterwards the timer restarts.
    ///
    /// # Errors
    /// [`GateError::NoSuchPlayer`].
    pub fn set_status(&mut self, slot: u8, status: PlayerStatus) -> Result<(), GateError> {
        let s = self
            .status
            .get_mut(usize::from(slot))
            .ok_or(GateError::NoSuchPlayer { slot })?;
        *s = status;
        if !self.status.contains(&PlayerStatus::AtExit) {
            self.waited = 0;
        }
        Ok(())
    }

    /// Time passes. The timer only runs while somebody is at the exit; it saturates.
    pub fn advance(&mut self, ticks: u64) {
        if self.status.contains(&PlayerStatus::AtExit) {
            self.waited = self.waited.saturating_add(ticks);
        }
    }

    /// What the gate says now.
    #[must_use]
    pub fn decision(&self) -> GateDecision {
        let slots = |wanted: PlayerStatus| -> Vec<u8> {
            self.status
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == wanted)
                .map(|(i, _)| u8::try_from(i).unwrap_or(u8::MAX))
                .collect()
        };
        let away = slots(PlayerStatus::Away);
        let at_exit = slots(PlayerStatus::AtExit);
        let dead = slots(PlayerStatus::Dead);
        if away.is_empty() && at_exit.is_empty() {
            return GateDecision::NobodyAlive;
        }
        if at_exit.is_empty() {
            return GateDecision::Wait { remaining: None };
        }
        if away.is_empty() {
            return GateDecision::Go {
                ghosted: Vec::new(),
                dead,
            };
        }
        if self.waited >= self.timeout_ticks {
            return GateDecision::Go {
                ghosted: away,
                dead,
            };
        }
        GateDecision::Wait {
            remaining: Some(self.timeout_ticks - self.waited),
        }
    }
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
