//! **Level progress** (roadmap T09-023, `docs/LEVEL_PROGRESS.md`): what a level remembers about how it is going, and what the end-of-level
//! summary says. Pure logic: time is a count of simulation ticks (never a clock), there is no randomness and no `HashMap`, so the same
//! sequence of events always gives the same state, byte for byte.
//!
//! * **Kills.** Enemies are *registered* as the level creates them (spawners create enemies while the level runs, so the total is not
//!   known at load), and each kill is credited to a player slot or to nobody (a trap, another enemy). The team count is the sum.
//! * **Secrets.** A level has a known number of secrets, numbered `0..total`. Each is found **once**; finding it again changes nothing.
//!   The finder is kept (the original's secret entity has a finder handle).
//! * **Time.** [`LevelProgress::advance`] adds ticks; [`LevelProgress::summary`] turns them into milliseconds at a tick rate.
//! * **Checkpoints.** Reached in order, each once. Reaching one stores a snapshot of the counters and an opaque, versioned payload
//!   the caller supplies (the game world itself is not this module's business). [`LevelProgress::restore`] puts the counters back to
//!   that snapshot and hands the payload back.
//! * **Score** is optional, per player, with an uncredited share (the world info's `macAddScore` has no player).
//! * [`LevelProgress::to_bytes`] and [`LevelProgress::from_bytes`] write and read the whole state with a checksum; damaged input is
//!   refused, never a panic.

use std::collections::BTreeMap;

/// The most players one level tracks (the same as the network's player slots).
pub const MAX_PLAYERS: u8 = 64;
/// The most secrets one level may declare.
pub const MAX_SECRETS: u32 = 4096;
/// The most checkpoints one level may declare (retail worlds have 1 to 12 chapters).
pub const MAX_CHECKPOINTS: u32 = 256;
/// The default and the largest allowed checkpoint payload, in bytes.
pub const DEFAULT_MAX_PAYLOAD: usize = 1 << 20;
/// The largest value of [`LevelSetup::max_payload`].
pub const MAX_PAYLOAD_LIMIT: usize = 16 << 20;

const MAGIC: [u8; 4] = *b"S2LP";
const FORMAT_VERSION: u16 = 1;
const NO_FINDER: u8 = 0xFF;

/// Why a call was refused. The state is unchanged after any error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressError {
    /// The setup is not valid (the text says which part).
    BadSetup(&'static str),
    /// The level is finished; nothing changes any more.
    Finished,
    /// A player slot at or past the player count.
    NoSuchPlayer {
        /// The slot asked for.
        slot: u8,
    },
    /// A secret number at or past the level's secret count.
    NoSuchSecret {
        /// The number asked for.
        id: u32,
    },
    /// A checkpoint number at or past the level's checkpoint count.
    NoSuchCheckpoint {
        /// The number asked for.
        index: u32,
    },
    /// A checkpoint was reached before the one in front of it.
    OutOfOrder {
        /// The checkpoint that has to come next.
        expected: u32,
        /// The one that was reached.
        got: u32,
    },
    /// The payload is larger than the level allows.
    PayloadTooLarge {
        /// The payload's size.
        len: usize,
        /// The limit.
        max: usize,
    },
    /// A tick rate of zero.
    BadTickRate,
    /// Saved bytes that cannot be read (the text says why).
    BadBytes(&'static str),
}

impl std::fmt::Display for ProgressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSetup(why) => write!(f, "bad level setup: {why}"),
            Self::Finished => f.write_str("the level is finished"),
            Self::NoSuchPlayer { slot } => write!(f, "no player slot {slot}"),
            Self::NoSuchSecret { id } => write!(f, "no secret {id}"),
            Self::NoSuchCheckpoint { index } => write!(f, "no checkpoint {index}"),
            Self::OutOfOrder { expected, got } => {
                write!(f, "checkpoint {got} reached, but {expected} comes next")
            }
            Self::PayloadTooLarge { len, max } => write!(
                f,
                "checkpoint payload of {len} bytes is over the limit of {max}"
            ),
            Self::BadTickRate => f.write_str("the tick rate must be above zero"),
            Self::BadBytes(why) => write!(f, "cannot read saved level progress: {why}"),
        }
    }
}

impl std::error::Error for ProgressError {}

/// What happens to the level clock when the player goes back to a checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RestoreTime {
    /// The clock keeps running: the time spent on the failed attempt counts. (Default.)
    #[default]
    Continue,
    /// The clock goes back to what it was when the checkpoint was reached (to zero for the level start).
    Rewind,
}

/// What the level declares up front.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelSetup {
    /// Player slots, 1 to [`MAX_PLAYERS`].
    pub players: u8,
    /// How many secrets the level has, at most [`MAX_SECRETS`].
    pub secrets: u32,
    /// How many checkpoints the level has, at most [`MAX_CHECKPOINTS`].
    pub checkpoints: u32,
    /// The par time in milliseconds, if the level has one.
    pub par_ms: Option<u64>,
    /// Whether the mode keeps score. When it does not, scores are ignored and the summary has none.
    pub scoring: bool,
    /// The largest checkpoint payload in bytes, 1 to [`MAX_PAYLOAD_LIMIT`].
    pub max_payload: usize,
    /// What a restore does to the clock.
    pub restore_time: RestoreTime,
}

impl Default for LevelSetup {
    fn default() -> Self {
        Self {
            players: 1,
            secrets: 0,
            checkpoints: 0,
            par_ms: None,
            scoring: true,
            max_payload: DEFAULT_MAX_PAYLOAD,
            restore_time: RestoreTime::Continue,
        }
    }
}

impl LevelSetup {
    /// Check every limit.
    pub fn validate(&self) -> Result<(), ProgressError> {
        if self.players == 0 || self.players > MAX_PLAYERS {
            return Err(ProgressError::BadSetup("players must be from 1 to 64"));
        }
        if self.secrets > MAX_SECRETS {
            return Err(ProgressError::BadSetup("too many secrets"));
        }
        if self.checkpoints > MAX_CHECKPOINTS {
            return Err(ProgressError::BadSetup("too many checkpoints"));
        }
        if self.max_payload == 0 || self.max_payload > MAX_PAYLOAD_LIMIT {
            return Err(ProgressError::BadSetup(
                "max_payload must be from 1 to 16 MiB",
            ));
        }
        Ok(())
    }
}

/// The opaque, versioned state of the game world that goes with a checkpoint. This module stores, size-limits and returns it; it
/// never looks inside.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointPayload {
    /// The caller's format number for `bytes`; the caller checks it on restore.
    pub version: u32,
    /// The caller's data.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PlayerCounters {
    kills: u32,
    score: u32,
}

/// The counters a checkpoint restores.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Counters {
    enemies_total: u32,
    kills_uncredited: u32,
    score_uncredited: u32,
    players: Vec<PlayerCounters>,
    /// Secret number to the slot that found it (`None`: found by nobody in particular).
    secrets: BTreeMap<u32, Option<u8>>,
}

impl Counters {
    fn new(players: u8) -> Self {
        Self {
            players: vec![PlayerCounters::default(); usize::from(players)],
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Checkpoint {
    tick: u64,
    counters: Counters,
    payload: CheckpointPayload,
}

/// What [`LevelProgress::find_secret`] found out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretOutcome {
    /// The secret had not been found before.
    New,
    /// It had; nothing changed.
    AlreadyFound,
}

/// What [`LevelProgress::reach_checkpoint`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointOutcome {
    /// The checkpoint is new: the snapshot and payload were stored.
    New,
    /// It was reached before (a retry walking over it again): nothing changed.
    AlreadyReached,
}

/// Where [`LevelProgress::restore`] went back to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Restored<'a> {
    /// No checkpoint had been reached: everything is as at the start of the level.
    LevelStart,
    /// The last checkpoint reached.
    Checkpoint {
        /// Its number.
        index: u32,
        /// The payload stored with it.
        payload: &'a CheckpointPayload,
    },
}

/// The state of one level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelProgress {
    setup: LevelSetup,
    elapsed: u64,
    finished: bool,
    restores: u32,
    live: Counters,
    reached: u32,
    last: Option<Checkpoint>,
}

/// One player's share of the summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSummary {
    /// The player slot.
    pub slot: u8,
    /// Kills credited to this player.
    pub kills: u32,
    /// Secrets this player found.
    pub secrets: u32,
    /// This player's score, if the mode keeps score.
    pub score: Option<u32>,
}

/// The end-of-level numbers. (A UI maps `kills`, `kills_total`, `secrets`, `secrets_total`, `elapsed_ms` and `score` onto its own
/// statistics; see `docs/LEVEL_PROGRESS.md`.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelSummary {
    /// Kills by the whole team (credited or not).
    pub kills: u32,
    /// Enemies registered, never less than `kills`.
    pub kills_total: u32,
    /// Secrets found.
    pub secrets: u32,
    /// Secrets the level has.
    pub secrets_total: u32,
    /// Level time in ticks.
    pub elapsed_ticks: u64,
    /// Level time in whole milliseconds, rounded down, at the tick rate given.
    pub elapsed_ms: u64,
    /// The par time in milliseconds, if the level has one.
    pub par_ms: Option<u64>,
    /// Whether `elapsed_ms` is within `par_ms` (`None` without a par time).
    pub within_par: Option<bool>,
    /// The team score (credited and uncredited), if the mode keeps score.
    pub score: Option<u32>,
    /// How many times the player went back to a checkpoint or the start.
    pub restores: u32,
    /// Checkpoints reached.
    pub checkpoints: u32,
    /// Checkpoints the level has.
    pub checkpoints_total: u32,
    /// One entry per player slot.
    pub players: Vec<PlayerSummary>,
}

impl LevelSummary {
    /// Kills as a whole percentage of the total, or `None` when there is nothing to kill.
    pub fn kill_percent(&self) -> Option<u8> {
        percent(self.kills, self.kills_total)
    }

    /// Secrets as a whole percentage of the total, or `None` when the level has none.
    pub fn secret_percent(&self) -> Option<u8> {
        percent(self.secrets, self.secrets_total)
    }
}

fn percent(part: u32, total: u32) -> Option<u8> {
    if total == 0 {
        return None;
    }
    let p = u64::from(part.min(total)) * 100 / u64::from(total);
    u8::try_from(p).ok()
}

impl LevelProgress {
    /// A level at its start.
    pub fn new(setup: LevelSetup) -> Result<Self, ProgressError> {
        setup.validate()?;
        let live = Counters::new(setup.players);
        Ok(Self {
            setup,
            elapsed: 0,
            finished: false,
            restores: 0,
            live,
            reached: 0,
            last: None,
        })
    }

    /// The setup the level was made with.
    pub fn setup(&self) -> &LevelSetup {
        &self.setup
    }

    fn open(&self) -> Result<(), ProgressError> {
        if self.finished {
            Err(ProgressError::Finished)
        } else {
            Ok(())
        }
    }

    fn check_player(&self, by: Option<u8>) -> Result<(), ProgressError> {
        match by {
            Some(slot) if slot >= self.setup.players => Err(ProgressError::NoSuchPlayer { slot }),
            _ => Ok(()),
        }
    }

    /// Add `ticks` to the level clock. Ignored once the level is finished; saturates.
    pub fn advance(&mut self, ticks: u64) {
        if !self.finished {
            self.elapsed = self.elapsed.saturating_add(ticks);
        }
    }

    /// `count` more enemies exist (a spawner made them). Saturates.
    pub fn register_enemies(&mut self, count: u32) -> Result<(), ProgressError> {
        self.open()?;
        self.live.enemies_total = self.live.enemies_total.saturating_add(count);
        Ok(())
    }

    /// An enemy died. `by` is the player credited (`None`: nobody); `score` is what the kill is worth (ignored when the mode keeps
    /// no score). Counters saturate.
    pub fn record_kill(&mut self, by: Option<u8>, score: u32) -> Result<(), ProgressError> {
        self.open()?;
        self.check_player(by)?;
        match by {
            Some(slot) => {
                let p = &mut self.live.players[usize::from(slot)];
                p.kills = p.kills.saturating_add(1);
            }
            None => self.live.kills_uncredited = self.live.kills_uncredited.saturating_add(1),
        }
        self.add_score(by, score)
    }

    /// Points that do not come from a kill (treasure, the world's `macAddScore`). Ignored when the mode keeps no score.
    pub fn add_score(&mut self, by: Option<u8>, points: u32) -> Result<(), ProgressError> {
        self.open()?;
        self.check_player(by)?;
        if !self.setup.scoring {
            return Ok(());
        }
        match by {
            Some(slot) => {
                let p = &mut self.live.players[usize::from(slot)];
                p.score = p.score.saturating_add(points);
            }
            None => self.live.score_uncredited = self.live.score_uncredited.saturating_add(points),
        }
        Ok(())
    }

    /// Secret `id` was found by `by`. The first time counts; later times change nothing (the first finder keeps the credit).
    pub fn find_secret(&mut self, id: u32, by: Option<u8>) -> Result<SecretOutcome, ProgressError> {
        self.open()?;
        if id >= self.setup.secrets {
            return Err(ProgressError::NoSuchSecret { id });
        }
        self.check_player(by)?;
        if self.live.secrets.contains_key(&id) {
            return Ok(SecretOutcome::AlreadyFound);
        }
        self.live.secrets.insert(id, by);
        Ok(SecretOutcome::New)
    }

    /// Checkpoint `index` was reached with the world state `payload`. Only the next checkpoint in order is new; one already reached
    /// is `AlreadyReached` and its payload is ignored (the stored one is kept); a later one is refused.
    pub fn reach_checkpoint(
        &mut self,
        index: u32,
        payload: CheckpointPayload,
    ) -> Result<CheckpointOutcome, ProgressError> {
        self.open()?;
        if index >= self.setup.checkpoints {
            return Err(ProgressError::NoSuchCheckpoint { index });
        }
        if index < self.reached {
            return Ok(CheckpointOutcome::AlreadyReached);
        }
        if index > self.reached {
            return Err(ProgressError::OutOfOrder {
                expected: self.reached,
                got: index,
            });
        }
        if payload.bytes.len() > self.setup.max_payload {
            return Err(ProgressError::PayloadTooLarge {
                len: payload.bytes.len(),
                max: self.setup.max_payload,
            });
        }
        self.last = Some(Checkpoint {
            tick: self.elapsed,
            counters: self.live.clone(),
            payload,
        });
        self.reached += 1;
        Ok(CheckpointOutcome::New)
    }

    /// Go back to the last checkpoint (or the start): the counters become what they were then (kills, secrets, score, who got
    /// credit), the clock follows [`LevelSetup::restore_time`], and the checkpoints already reached stay reached. The caller restores
    /// the world from the payload returned.
    pub fn restore(&mut self) -> Result<Restored<'_>, ProgressError> {
        self.open()?;
        self.restores = self.restores.saturating_add(1);
        match &self.last {
            None => {
                self.live = Counters::new(self.setup.players);
                if self.setup.restore_time == RestoreTime::Rewind {
                    self.elapsed = 0;
                }
                Ok(Restored::LevelStart)
            }
            Some(cp) => {
                self.live = cp.counters.clone();
                if self.setup.restore_time == RestoreTime::Rewind {
                    self.elapsed = cp.tick;
                }
                Ok(Restored::Checkpoint {
                    index: self.reached - 1,
                    payload: &cp.payload,
                })
            }
        }
    }

    /// The level ends: the clock and every counter stop. Calling it again does nothing.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// Whether [`LevelProgress::finish`] was called.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The level clock in ticks.
    pub fn elapsed_ticks(&self) -> u64 {
        self.elapsed
    }

    /// Kills by the whole team (credited and uncredited).
    pub fn kills(&self) -> u32 {
        self.live
            .players
            .iter()
            .fold(self.live.kills_uncredited, |t, p| t.saturating_add(p.kills))
    }

    /// Enemies registered, never less than [`LevelProgress::kills`].
    pub fn kills_total(&self) -> u32 {
        self.live.enemies_total.max(self.kills())
    }

    /// Secrets found.
    pub fn secrets_found(&self) -> u32 {
        // At most MAX_SECRETS entries, so this always fits.
        u32::try_from(self.live.secrets.len()).unwrap_or(u32::MAX)
    }

    /// Checkpoints reached so far (also the number of the next one).
    pub fn checkpoints_reached(&self) -> u32 {
        self.reached
    }

    /// How many times [`LevelProgress::restore`] went back.
    pub fn restores(&self) -> u32 {
        self.restores
    }

    /// The end-of-level numbers now (call it after [`LevelProgress::finish`] for the final ones). `tick_hz` is the simulation's tick
    /// rate, above zero.
    pub fn summary(&self, tick_hz: u32) -> Result<LevelSummary, ProgressError> {
        if tick_hz == 0 {
            return Err(ProgressError::BadTickRate);
        }
        let ms = u128::from(self.elapsed) * 1000 / u128::from(tick_hz);
        let elapsed_ms = u64::try_from(ms).unwrap_or(u64::MAX);
        let players = self
            .live
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                // `players` is at most 64, so the slot fits a byte.
                let slot = u8::try_from(i).unwrap_or(u8::MAX);
                let secrets = self
                    .live
                    .secrets
                    .values()
                    .filter(|f| **f == Some(slot))
                    .count();
                PlayerSummary {
                    slot,
                    kills: p.kills,
                    secrets: u32::try_from(secrets).unwrap_or(u32::MAX),
                    score: self.setup.scoring.then_some(p.score),
                }
            })
            .collect();
        let score = self.setup.scoring.then(|| {
            self.live
                .players
                .iter()
                .fold(self.live.score_uncredited, |t, p| t.saturating_add(p.score))
        });
        Ok(LevelSummary {
            kills: self.kills(),
            kills_total: self.kills_total(),
            secrets: self.secrets_found(),
            secrets_total: self.setup.secrets,
            elapsed_ticks: self.elapsed,
            elapsed_ms,
            par_ms: self.setup.par_ms,
            within_par: self.setup.par_ms.map(|par| elapsed_ms <= par),
            score,
            restores: self.restores,
            checkpoints: self.reached,
            checkpoints_total: self.setup.checkpoints,
            players,
        })
    }

    // ---- bytes -------------------------------------------------------------------------------------------

    /// The whole state as bytes: `"S2LP"`, format version `u16` (1), the body (little endian integers), the CRC-32 of everything before.
    /// The same state always gives the same bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(&MAGIC);
        w.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        let s = &self.setup;
        w.push(s.players);
        put_u32(&mut w, s.secrets);
        put_u32(&mut w, s.checkpoints);
        match s.par_ms {
            Some(p) => {
                w.push(1);
                w.extend_from_slice(&p.to_le_bytes());
            }
            None => w.push(0),
        }
        w.push(u8::from(s.scoring));
        put_u32(&mut w, u32::try_from(s.max_payload).unwrap_or(u32::MAX));
        w.push(match s.restore_time {
            RestoreTime::Continue => 0,
            RestoreTime::Rewind => 1,
        });
        w.extend_from_slice(&self.elapsed.to_le_bytes());
        w.push(u8::from(self.finished));
        put_u32(&mut w, self.restores);
        put_u32(&mut w, self.reached);
        put_counters(&mut w, &self.live);
        match &self.last {
            Some(cp) => {
                w.push(1);
                w.extend_from_slice(&cp.tick.to_le_bytes());
                put_counters(&mut w, &cp.counters);
                put_u32(&mut w, cp.payload.version);
                put_u32(
                    &mut w,
                    u32::try_from(cp.payload.bytes.len()).unwrap_or(u32::MAX),
                );
                w.extend_from_slice(&cp.payload.bytes);
            }
            None => w.push(0),
        }
        let crc = crc32(&w);
        put_u32(&mut w, crc);
        w
    }

    /// Read what [`LevelProgress::to_bytes`] wrote. Refuses anything damaged, truncated, of another version, over a limit or
    /// inconsistent (a secret that does not exist, a player slot out of range, checkpoint bookkeeping that does not add up).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgressError> {
        let bad = ProgressError::BadBytes;
        if bytes.len() < MAGIC.len() + 2 + 4 {
            return Err(bad("too short"));
        }
        let (body, tail) = bytes.split_at(bytes.len() - 4);
        let stored = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        if crc32(body) != stored {
            return Err(bad("checksum mismatch"));
        }
        let mut r = Reader { data: body, pos: 0 };
        if r.take(4)? != MAGIC {
            return Err(bad("not level progress"));
        }
        if r.u16()? != FORMAT_VERSION {
            return Err(bad("unknown format version"));
        }
        let players = r.u8()?;
        let secrets = r.u32()?;
        let checkpoints = r.u32()?;
        let par_ms = match r.u8()? {
            0 => None,
            1 => Some(r.u64()?),
            _ => return Err(bad("par flag is not 0 or 1")),
        };
        let scoring = r.bool()?;
        let max_payload =
            usize::try_from(r.u32()?).map_err(|_| bad("payload limit does not fit"))?;
        let restore_time = match r.u8()? {
            0 => RestoreTime::Continue,
            1 => RestoreTime::Rewind,
            _ => return Err(bad("unknown restore-time rule")),
        };
        let setup = LevelSetup {
            players,
            secrets,
            checkpoints,
            par_ms,
            scoring,
            max_payload,
            restore_time,
        };
        setup
            .validate()
            .map_err(|_| bad("the setup is out of range"))?;
        let elapsed = r.u64()?;
        let finished = r.bool()?;
        let restores = r.u32()?;
        let reached = r.u32()?;
        if reached > checkpoints {
            return Err(bad("more checkpoints reached than exist"));
        }
        let live = get_counters(&mut r, &setup)?;
        let last = match r.u8()? {
            0 => None,
            1 => {
                let tick = r.u64()?;
                if tick > elapsed {
                    return Err(bad("a checkpoint from the future"));
                }
                let counters = get_counters(&mut r, &setup)?;
                let version = r.u32()?;
                let len = usize::try_from(r.u32()?).map_err(|_| bad("payload does not fit"))?;
                if len > max_payload {
                    return Err(bad("payload over the limit"));
                }
                let payload = CheckpointPayload {
                    version,
                    bytes: r.take(len)?.to_vec(),
                };
                Some(Checkpoint {
                    tick,
                    counters,
                    payload,
                })
            }
            _ => return Err(bad("checkpoint flag is not 0 or 1")),
        };
        if last.is_some() != (reached > 0) {
            return Err(bad("checkpoint count and snapshot disagree"));
        }
        if r.pos != body.len() {
            return Err(bad("bytes left over"));
        }
        Ok(Self {
            setup,
            elapsed,
            finished,
            restores,
            live,
            reached,
            last,
        })
    }
}

fn put_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&v.to_le_bytes());
}

fn put_counters(w: &mut Vec<u8>, c: &Counters) {
    put_u32(w, c.enemies_total);
    put_u32(w, c.kills_uncredited);
    put_u32(w, c.score_uncredited);
    for p in &c.players {
        put_u32(w, p.kills);
        put_u32(w, p.score);
    }
    put_u32(w, u32::try_from(c.secrets.len()).unwrap_or(u32::MAX));
    for (id, by) in &c.secrets {
        put_u32(w, *id);
        w.push(by.unwrap_or(NO_FINDER));
    }
}

fn get_counters(r: &mut Reader<'_>, setup: &LevelSetup) -> Result<Counters, ProgressError> {
    let bad = ProgressError::BadBytes;
    let mut c = Counters::new(setup.players);
    c.enemies_total = r.u32()?;
    c.kills_uncredited = r.u32()?;
    c.score_uncredited = r.u32()?;
    if !setup.scoring && c.score_uncredited != 0 {
        return Err(bad("a score in a mode without score"));
    }
    for p in &mut c.players {
        p.kills = r.u32()?;
        p.score = r.u32()?;
        if !setup.scoring && p.score != 0 {
            return Err(bad("a score in a mode without score"));
        }
    }
    let n = r.u32()?;
    if n > setup.secrets {
        return Err(bad("more secrets found than exist"));
    }
    let mut prev: Option<u32> = None;
    for _ in 0..n {
        let id = r.u32()?;
        let by = r.u8()?;
        if id >= setup.secrets || prev.is_some_and(|p| id <= p) {
            return Err(bad("secrets not in order or out of range"));
        }
        let by = if by == NO_FINDER {
            None
        } else if by < setup.players {
            Some(by)
        } else {
            return Err(bad("a finder that is not a player"));
        };
        c.secrets.insert(id, by);
        prev = Some(id);
    }
    Ok(c)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProgressError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.data.len())
            .ok_or(ProgressError::BadBytes("truncated"))?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, ProgressError> {
        Ok(self.take(1)?[0])
    }

    fn bool(&mut self) -> Result<bool, ProgressError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ProgressError::BadBytes("a flag is not 0 or 1")),
        }
    }

    fn u16(&mut self) -> Result<u16, ProgressError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, ProgressError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, ProgressError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
}

/// CRC-32 (the zlib/PNG polynomial, reflected), bit by bit: the data is small.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
#[path = "level_progress_tests.rs"]
mod tests;
