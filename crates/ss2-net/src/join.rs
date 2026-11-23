//! **Joining a game in progress** (roadmap T08-013, `docs/JOIN.md`): streaming a full state to a late joiner, keeping track of where each joiner is, and making sure no event is
//! lost or played twice across the hand-over from snapshot to live play.
//!
//! Pure logic like the rest of `ss2-net`: time is a number of microseconds passed in, there is no randomness and no `HashMap`.
//!
//! * [`ChunkSender`] and [`ChunkReceiver`] move one big blob (the encoded full state) over an unreliable channel: it is cut into numbered chunks, the receiver says which it has
//!   ([`ChunkAck`]), the sender repeats only the missing ones, and the receiver checks the length and a checksum before handing the blob over.
//! * [`JoinManager`] is the server's record of every joiner: queued, loading the level, receiving the snapshot, catching up, in the game. It limits how many joiners are
//!   served at once, times out a joiner that stalls, and refuses steps out of order.
//! * The **event baseline**: when the snapshot is taken the server notes the last event sequence number it contains. The joiner's [`EventCursor`] starts exactly there, so events that
//!   happened **before** the snapshot are not replayed (their effects are in it) and events **after** it are delivered once the snapshot is applied.

use std::collections::BTreeMap;

use crate::events::EventCursor;
use crate::session::PeerId;

// ---------------------------------------------------------------------------------------------------------------------
// Chunked transfer
// ---------------------------------------------------------------------------------------------------------------------

/// The limits of a transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkConfig {
    /// Bytes of data in each chunk but the last (below the transport's packet size).
    pub chunk_size: usize,
    /// The largest blob accepted, in bytes.
    pub max_bytes: usize,
}

impl Default for ChunkConfig {
    /// 1,024-byte chunks and at most 8 MiB.
    fn default() -> Self {
        Self {
            chunk_size: 1024,
            max_bytes: 8 << 20,
        }
    }
}

/// Why a transfer or a chunk was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkError {
    /// The blob is empty or bigger than the limit, or the chunk size is zero.
    BadSize,
    /// A chunk names a transfer other than the one in progress.
    WrongTransfer,
    /// A chunk's header disagrees with the first chunk's (total, length, checksum), or its index is out of range, or its data has the wrong length.
    Inconsistent,
    /// Not every chunk has arrived yet.
    Incomplete,
    /// All chunks arrived but the whole does not match its checksum.
    Corrupt,
}

impl std::fmt::Display for ChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BadSize => "the blob is empty, too large or the chunk size is zero",
            Self::WrongTransfer => "the chunk belongs to another transfer",
            Self::Inconsistent => "the chunk disagrees with the transfer it claims to be part of",
            Self::Incomplete => "chunks are still missing",
            Self::Corrupt => "the reassembled data does not match its checksum",
        })
    }
}

impl std::error::Error for ChunkError {}

/// FNV-1a over a byte string: an integrity check against damage and mix-ups, **not** a defence against tampering (the transport authenticates).
pub fn checksum(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// One piece of a blob.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// Which transfer.
    pub transfer: u32,
    /// This chunk's number, from 0.
    pub index: u32,
    /// How many chunks the blob has.
    pub total: u32,
    /// The length of the whole blob in bytes.
    pub total_len: u32,
    /// The checksum of the whole blob.
    pub checksum: u64,
    /// This chunk's bytes.
    pub data: Vec<u8>,
}

/// Which chunks a receiver has: one bit per chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkAck {
    /// Which transfer.
    pub transfer: u32,
    /// How many chunks there are.
    pub total: u32,
    /// Bit `i` of the words (least significant first) is set if chunk `i` has arrived.
    pub received: Vec<u64>,
}

impl ChunkAck {
    /// Whether chunk `index` is acknowledged.
    pub fn has(&self, index: u32) -> bool {
        index < self.total
            && self
                .received
                .get(index as usize / 64)
                .is_some_and(|w| w >> (index % 64) & 1 == 1)
    }

    /// How many chunks are acknowledged.
    pub fn count(&self) -> u32 {
        (0..self.total).filter(|i| self.has(*i)).count() as u32
    }
}

/// The sending end of a transfer.
///
/// ```
/// use ss2_net::join::{ChunkConfig, ChunkReceiver, ChunkSender};
///
/// let blob: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
/// let mut tx = ChunkSender::new(1, &blob, ChunkConfig::default()).unwrap();
/// let mut rx = ChunkReceiver::new(ChunkConfig::default());
/// while !tx.is_complete() {
///     for chunk in tx.next_chunks(2) {          // two chunks per tick
///         rx.receive(&chunk).unwrap();
///     }
///     tx.on_ack(&rx.ack().unwrap());
/// }
/// assert_eq!(rx.finish().unwrap(), blob);
/// ```
#[derive(Clone, Debug)]
pub struct ChunkSender {
    transfer: u32,
    chunks: Vec<Vec<u8>>,
    total_len: u32,
    checksum: u64,
    acked: Vec<bool>,
    /// Where the next batch starts looking, so repeats cycle through the missing chunks instead of resending the first ones forever.
    cursor: usize,
}

impl ChunkSender {
    /// Prepare to send `blob`.
    ///
    /// # Errors
    /// [`ChunkError::BadSize`] for an empty blob, one over the limit, or a chunk size of zero.
    pub fn new(transfer: u32, blob: &[u8], cfg: ChunkConfig) -> Result<Self, ChunkError> {
        if blob.is_empty()
            || blob.len() > cfg.max_bytes
            || cfg.chunk_size == 0
            || u32::try_from(blob.len()).is_err()
        {
            return Err(ChunkError::BadSize);
        }
        let chunks: Vec<Vec<u8>> = blob.chunks(cfg.chunk_size).map(<[u8]>::to_vec).collect();
        let n = chunks.len();
        Ok(Self {
            transfer,
            total_len: blob.len() as u32,
            checksum: checksum(blob),
            acked: vec![false; n],
            chunks,
            cursor: 0,
        })
    }

    /// How many chunks the blob has.
    pub fn total(&self) -> usize {
        self.chunks.len()
    }

    /// How many have been acknowledged.
    pub fn acked(&self) -> usize {
        self.acked.iter().filter(|a| **a).count()
    }

    /// Whether every chunk is acknowledged.
    pub fn is_complete(&self) -> bool {
        self.acked.iter().all(|a| *a)
    }

    /// The next `max` chunks that have not been acknowledged, **cycling**: a call continues after the chunks the last call gave, wrapping, so a lost chunk is repeated within one
    /// round of the missing ones rather than after every earlier one has been resent.
    pub fn next_chunks(&mut self, max: usize) -> Vec<Chunk> {
        let n = self.chunks.len();
        let start = self.cursor;
        let mut out = Vec::new();
        let mut scanned = 0;
        while out.len() < max && scanned < n {
            let i = (start + scanned) % n;
            scanned += 1;
            if !self.acked[i] {
                out.push(Chunk {
                    transfer: self.transfer,
                    index: i as u32,
                    total: n as u32,
                    total_len: self.total_len,
                    checksum: self.checksum,
                    data: self.chunks[i].clone(),
                });
                self.cursor = (i + 1) % n;
            }
        }
        out
    }

    /// Take what the receiver says it has. An ack for another transfer or with another total is ignored; bits for chunks beyond the total are ignored.
    pub fn on_ack(&mut self, ack: &ChunkAck) {
        if ack.transfer != self.transfer || ack.total as usize != self.chunks.len() {
            return;
        }
        for (i, a) in self.acked.iter_mut().enumerate() {
            if ack.has(i as u32) {
                *a = true;
            }
        }
    }
}

/// The receiving end of a transfer.
#[derive(Clone, Debug)]
pub struct ChunkReceiver {
    cfg: ChunkConfig,
    header: Option<(u32, u32, u32, u64)>, // transfer, total, total_len, checksum
    parts: Vec<Option<Vec<u8>>>,
    got: usize,
    duplicates: u64,
}

impl ChunkReceiver {
    /// A receiver that has seen nothing.
    pub fn new(cfg: ChunkConfig) -> Self {
        Self {
            cfg,
            header: None,
            parts: Vec::new(),
            got: 0,
            duplicates: 0,
        }
    }

    /// How many distinct chunks have arrived.
    pub fn received(&self) -> usize {
        self.got
    }

    /// How many chunks arrived again.
    pub fn duplicates(&self) -> u64 {
        self.duplicates
    }

    /// Whether every chunk has arrived.
    pub fn is_complete(&self) -> bool {
        self.header.is_some() && self.got == self.parts.len()
    }

    /// Take a chunk. The first one fixes the transfer, the total, the length and the checksum; every later one must agree. Out of order and repeated chunks are fine (a repeat
    /// is counted). Returns whether the chunk was new.
    ///
    /// # Errors
    /// [`ChunkError::BadSize`] if the first chunk's header is impossible (zero chunks, a length over the limit, a total that does not match the length);
    /// [`ChunkError::WrongTransfer`] for another transfer id than the one in progress; [`ChunkError::Inconsistent`] for an index out of range, a header that
    /// differs from the first chunk's, or data of the wrong length. A refused chunk changes nothing.
    pub fn receive(&mut self, chunk: &Chunk) -> Result<bool, ChunkError> {
        let cs = self.cfg.chunk_size;
        let n = chunk.total as usize;
        let len = chunk.total_len as usize;
        match self.header {
            None => {
                if n == 0
                    || len == 0
                    || len > self.cfg.max_bytes
                    || cs == 0
                    || len.div_ceil(cs) != n
                {
                    return Err(ChunkError::BadSize);
                }
                self.header = Some((chunk.transfer, chunk.total, chunk.total_len, chunk.checksum));
                self.parts = vec![None; n];
            }
            Some((transfer, total, total_len, sum)) => {
                if chunk.transfer != transfer {
                    return Err(ChunkError::WrongTransfer);
                }
                if (chunk.total, chunk.total_len, chunk.checksum) != (total, total_len, sum) {
                    return Err(ChunkError::Inconsistent);
                }
            }
        }
        let index = chunk.index as usize;
        let expected = if index + 1 == n {
            len - cs * (n - 1)
        } else {
            cs
        };
        if index >= n || chunk.data.len() != expected {
            // The header may have been set by this very chunk: if nothing has arrived and the chunk is bad, forget it, so a bad first chunk does not fix a bad header.
            if self.got == 0 {
                self.header = None;
                self.parts.clear();
            }
            return Err(ChunkError::Inconsistent);
        }
        if self.parts[index].is_some() {
            self.duplicates += 1;
            return Ok(false);
        }
        self.parts[index] = Some(chunk.data.clone());
        self.got += 1;
        Ok(true)
    }

    /// What has arrived, to send back; `None` before the first chunk.
    pub fn ack(&self) -> Option<ChunkAck> {
        let (transfer, total, _, _) = self.header?;
        let mut received = vec![0u64; (total as usize).div_ceil(64)];
        for (i, p) in self.parts.iter().enumerate() {
            if p.is_some() {
                received[i / 64] |= 1 << (i % 64);
            }
        }
        Some(ChunkAck {
            transfer,
            total,
            received,
        })
    }

    /// The reassembled blob once every chunk is in.
    ///
    /// # Errors
    /// [`ChunkError::Incomplete`] while chunks are missing; [`ChunkError::Corrupt`] if the length or checksum is wrong (the receiver is then reset, so the sender can start over).
    pub fn finish(&mut self) -> Result<Vec<u8>, ChunkError> {
        let Some((_, _, total_len, sum)) = self.header else {
            return Err(ChunkError::Incomplete);
        };
        if self.got != self.parts.len() {
            return Err(ChunkError::Incomplete);
        }
        let blob: Vec<u8> = self.parts.iter().flatten().flatten().copied().collect();
        if blob.len() != total_len as usize || checksum(&blob) != sum {
            *self = Self::new(self.cfg);
            return Err(ChunkError::Corrupt);
        }
        Ok(blob)
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Join tracking
// ---------------------------------------------------------------------------------------------------------------------

/// Where a joiner is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinPhase {
    /// Waiting for a place among the joiners being served.
    Queued,
    /// Told to load the level; the server waits for `level_ready`.
    LoadingLevel,
    /// The level is loaded; the server will take a snapshot and start sending it.
    AwaitingBaseline,
    /// The snapshot is being sent.
    Transferring,
    /// The snapshot is applied; the joiner is catching up with ticks since the baseline.
    CatchingUp,
    /// In the game.
    InGame,
}

/// Why a join ended without the joiner getting in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinFailure {
    /// The joiner's level is not the server's (a different build of the level, or the wrong level): it must reload.
    LevelMismatch,
    /// The joiner took too long to load the level.
    LoadTimedOut,
    /// The snapshot transfer made no progress for too long.
    TransferStalled,
    /// The joiner did not catch up in time.
    CatchUpTimedOut,
    /// The snapshot became too old: the events since it were pruned from the log before the joiner applied it. The joiner must start the transfer again.
    SnapshotTooOld,
    /// The joiner or the host cancelled.
    Cancelled,
}

/// What happened to a joiner, from [`JoinManager::tick`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinEvent {
    /// A queued joiner has a place now and should load the level.
    Admitted(PeerId),
    /// A joiner is in the game.
    Completed(PeerId),
    /// A joiner failed; its place is free.
    Failed(PeerId, JoinFailure),
}

/// Why a step was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinError {
    /// There is no join in progress for that peer.
    Unknown,
    /// The step is not allowed in the joiner's current phase.
    WrongPhase(JoinPhase),
    /// Too many joiners are tracked (queued or active).
    TooMany,
    /// That peer is already joining.
    AlreadyJoining,
    /// The transfer could not start.
    Transfer(ChunkError),
}

/// The limits and timeouts of joining.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinConfig {
    /// How many joiners are served at once (snapshots are expensive); the rest wait in the queue.
    pub max_active: usize,
    /// How many joiners are tracked in all, queued or active.
    pub max_tracked: usize,
    /// Time allowed to load the level, in microseconds.
    pub load_timeout_us: u64,
    /// A transfer that makes no new progress for this long fails.
    pub transfer_stall_us: u64,
    /// Time allowed from applying the snapshot to being caught up.
    pub catch_up_timeout_us: u64,
    /// The joiner is caught up when it is within this many ticks of the server.
    pub catch_up_ticks: u64,
    /// Chunking limits.
    pub chunks: ChunkConfig,
}

impl Default for JoinConfig {
    /// Two at a time, 16 tracked, 2 minutes to load, 10 s without transfer progress, 15 s to catch up, within 3 ticks.
    fn default() -> Self {
        Self {
            max_active: 2,
            max_tracked: 16,
            load_timeout_us: 120_000_000,
            transfer_stall_us: 10_000_000,
            catch_up_timeout_us: 15_000_000,
            catch_up_ticks: 3,
            chunks: ChunkConfig::default(),
        }
    }
}

struct Join {
    phase: JoinPhase,
    phase_since: u64,
    last_progress: u64,
    baseline_tick: u64,
    event_seq: u64,
    sender: Option<ChunkSender>,
    transfers_started: u32,
}

/// The server's record of everybody joining.
///
/// ```
/// use ss2_net::join::{JoinConfig, JoinEvent, JoinManager, JoinPhase};
/// use ss2_net::session::PeerId;
///
/// let mut joins = JoinManager::new(JoinConfig::default(), 0xABCD); // 0xABCD is the hash of the level being played
/// let sam = PeerId(1);
/// joins.begin(sam, 0).unwrap();
/// assert_eq!(joins.tick(0), [JoinEvent::Admitted(sam)]);
/// assert_eq!(joins.phase(sam), Some(JoinPhase::LoadingLevel));
/// ```
pub struct JoinManager {
    cfg: JoinConfig,
    level_hash: u64,
    joins: BTreeMap<PeerId, Join>,
    /// Peers in the order they asked, for the queue.
    order: Vec<PeerId>,
    next_transfer: u32,
}

impl JoinManager {
    /// A manager for a server playing the level with this hash.
    pub fn new(cfg: JoinConfig, level_hash: u64) -> Self {
        Self {
            cfg,
            level_hash,
            joins: BTreeMap::new(),
            order: Vec::new(),
            next_transfer: 1,
        }
    }

    /// The server moved on to another level: joiners in progress must start again (they are cancelled), and new ones are checked against the new hash.
    pub fn set_level(&mut self, level_hash: u64) -> Vec<JoinEvent> {
        self.level_hash = level_hash;
        let peers: Vec<PeerId> = self.order.clone();
        let mut events = Vec::new();
        for p in peers {
            self.joins.remove(&p);
            events.push(JoinEvent::Failed(p, JoinFailure::Cancelled));
        }
        self.order.clear();
        events
    }

    /// A peer wants to join a game in progress. It is queued; [`tick`](Self::tick) admits it when there is room.
    ///
    /// # Errors
    /// [`JoinError::AlreadyJoining`] or [`JoinError::TooMany`].
    pub fn begin(&mut self, peer: PeerId, now_us: u64) -> Result<(), JoinError> {
        if self.joins.contains_key(&peer) {
            return Err(JoinError::AlreadyJoining);
        }
        if self.joins.len() >= self.cfg.max_tracked {
            return Err(JoinError::TooMany);
        }
        self.joins.insert(
            peer,
            Join {
                phase: JoinPhase::Queued,
                phase_since: now_us,
                last_progress: now_us,
                baseline_tick: 0,
                event_seq: 0,
                sender: None,
                transfers_started: 0,
            },
        );
        self.order.push(peer);
        Ok(())
    }

    /// The phase of a joiner.
    pub fn phase(&self, peer: PeerId) -> Option<JoinPhase> {
        self.joins.get(&peer).map(|j| j.phase)
    }

    /// Where a queued joiner stands: 0 is next. `None` if not queued.
    pub fn queue_position(&self, peer: PeerId) -> Option<usize> {
        if self.phase(peer) != Some(JoinPhase::Queued) {
            return None;
        }
        self.order
            .iter()
            .filter(|p| self.phase(**p) == Some(JoinPhase::Queued))
            .position(|p| *p == peer)
    }

    /// How many joiners hold a place (loading, transferring or catching up).
    pub fn active(&self) -> usize {
        self.joins
            .values()
            .filter(|j| !matches!(j.phase, JoinPhase::Queued | JoinPhase::InGame))
            .count()
    }

    /// How many are tracked.
    pub fn tracked(&self) -> usize {
        self.joins.len()
    }

    fn step(&mut self, peer: PeerId, expected: JoinPhase) -> Result<&mut Join, JoinError> {
        let j = self.joins.get_mut(&peer).ok_or(JoinError::Unknown)?;
        if j.phase == expected {
            Ok(j)
        } else {
            Err(JoinError::WrongPhase(j.phase))
        }
    }

    /// The joiner reports that it has loaded a level with `level_hash`. A match moves it on; a mismatch **fails** the join ([`JoinFailure::LevelMismatch`], returned and the place freed).
    ///
    /// # Errors
    /// [`JoinError::Unknown`], or [`JoinError::WrongPhase`] unless it was loading the level.
    pub fn level_ready(
        &mut self,
        peer: PeerId,
        level_hash: u64,
        now_us: u64,
    ) -> Result<Option<JoinFailure>, JoinError> {
        let ok = level_hash == self.level_hash;
        let j = self.step(peer, JoinPhase::LoadingLevel)?;
        if !ok {
            self.remove(peer);
            return Ok(Some(JoinFailure::LevelMismatch));
        }
        j.phase = JoinPhase::AwaitingBaseline;
        j.phase_since = now_us;
        j.last_progress = now_us;
        Ok(None)
    }

    /// Start sending the snapshot `blob`, taken at `baseline_tick` and containing every event up to sequence number `event_seq` (the log's `latest_seq()` when the snapshot was taken).
    /// Returns the sender to pull chunks from.
    ///
    /// # Errors
    /// [`JoinError::Unknown`], [`JoinError::WrongPhase`] (unless the joiner awaits a baseline), or [`JoinError::Transfer`] for a blob that cannot be sent.
    pub fn start_transfer(
        &mut self,
        peer: PeerId,
        baseline_tick: u64,
        event_seq: u64,
        blob: &[u8],
        now_us: u64,
    ) -> Result<(), JoinError> {
        let cfg = self.cfg.chunks;
        let id = self.next_transfer;
        let j = self.step(peer, JoinPhase::AwaitingBaseline)?;
        let sender = ChunkSender::new(id, blob, cfg).map_err(JoinError::Transfer)?;
        j.sender = Some(sender);
        j.baseline_tick = baseline_tick;
        j.event_seq = event_seq;
        j.phase = JoinPhase::Transferring;
        j.phase_since = now_us;
        j.last_progress = now_us;
        j.transfers_started += 1;
        self.next_transfer += 1;
        Ok(())
    }

    /// The next chunks to send to a joiner (up to `max`); empty when it is not transferring or everything is acknowledged.
    pub fn next_chunks(&mut self, peer: PeerId, max: usize) -> Vec<Chunk> {
        self.joins
            .get_mut(&peer)
            .and_then(|j| j.sender.as_mut())
            .map(|s| s.next_chunks(max))
            .unwrap_or_default()
    }

    /// The joiner's acknowledgement of chunks. New progress resets the stall timer.
    ///
    /// # Errors
    /// [`JoinError::Unknown`] or [`JoinError::WrongPhase`] unless transferring.
    pub fn on_ack(&mut self, peer: PeerId, ack: &ChunkAck, now_us: u64) -> Result<(), JoinError> {
        let j = self.step(peer, JoinPhase::Transferring)?;
        let sender = j
            .sender
            .as_mut()
            .expect("a transferring joiner has a sender");
        let before = sender.acked();
        sender.on_ack(ack);
        if sender.acked() > before {
            j.last_progress = now_us;
        }
        Ok(())
    }

    /// The joiner has the whole snapshot **and applied it**. `oldest_event_seq` is the oldest sequence number still in the event log: if the events the joiner needs (those after the
    /// snapshot's `event_seq`) have been pruned, the snapshot is too old and the join **fails** ([`JoinFailure::SnapshotTooOld`]).
    ///
    /// # Errors
    /// [`JoinError::Unknown`], or [`JoinError::WrongPhase`] unless transferring with every chunk acknowledged ([`JoinError::Transfer`]`(Incomplete)` while some are missing).
    pub fn snapshot_applied(
        &mut self,
        peer: PeerId,
        oldest_event_seq: u64,
        now_us: u64,
    ) -> Result<Option<JoinFailure>, JoinError> {
        let j = self.step(peer, JoinPhase::Transferring)?;
        if !j.sender.as_ref().is_some_and(ChunkSender::is_complete) {
            return Err(JoinError::Transfer(ChunkError::Incomplete));
        }
        // The first event the joiner needs is `event_seq + 1`; the log starts at `oldest_event_seq`.
        if j.event_seq.saturating_add(1) < oldest_event_seq {
            self.remove(peer);
            return Ok(Some(JoinFailure::SnapshotTooOld));
        }
        j.phase = JoinPhase::CatchingUp;
        j.phase_since = now_us;
        j.last_progress = now_us;
        j.sender = None;
        Ok(None)
    }

    /// The joiner reports the tick it has simulated up to; within `catch_up_ticks` of `server_tick` it is in the game.
    ///
    /// # Errors
    /// [`JoinError::Unknown`] or [`JoinError::WrongPhase`] unless catching up.
    pub fn progress(
        &mut self,
        peer: PeerId,
        joiner_tick: u64,
        server_tick: u64,
        now_us: u64,
    ) -> Result<bool, JoinError> {
        let slack = self.cfg.catch_up_ticks;
        let j = self.step(peer, JoinPhase::CatchingUp)?;
        if server_tick.saturating_sub(joiner_tick) <= slack {
            j.phase = JoinPhase::InGame;
            j.phase_since = now_us;
            return Ok(true);
        }
        Ok(false)
    }

    /// A joiner that is in the game (or any joiner) leaves the tracker: its place is free.
    pub fn finish(&mut self, peer: PeerId) {
        self.remove(peer);
    }

    /// Cancel a join (the peer disconnected).
    pub fn cancel(&mut self, peer: PeerId) -> bool {
        self.remove(peer)
    }

    fn remove(&mut self, peer: PeerId) -> bool {
        self.order.retain(|p| *p != peer);
        self.joins.remove(&peer).is_some()
    }

    /// The tick number the joiner's snapshot was taken at (for the deltas that follow it).
    pub fn baseline_tick(&self, peer: PeerId) -> Option<u64> {
        self.joins
            .get(&peer)
            .filter(|j| j.transfers_started > 0)
            .map(|j| j.baseline_tick)
    }

    /// The joiner's event cursor: it has "acknowledged" every event the snapshot contains, so the event log sends it only the later ones, and **none of them are skipped or repeated**.
    pub fn event_cursor(&self, peer: PeerId) -> Option<EventCursor> {
        let j = self.joins.get(&peer).filter(|j| j.transfers_started > 0)?;
        let mut cursor = EventCursor::new();
        cursor.ack(j.event_seq);
        Some(cursor)
    }

    /// Advance: admit queued joiners while there is room (oldest first), and fail the ones that have stalled or timed out. Returns what happened, ordered by peer within a kind
    /// (failures first, then admissions).
    pub fn tick(&mut self, now_us: u64) -> Vec<JoinEvent> {
        let mut events = Vec::new();
        let mut failed: Vec<(PeerId, JoinFailure)> = Vec::new();
        for (peer, j) in &self.joins {
            let waited = now_us.saturating_sub(j.phase_since);
            let idle = now_us.saturating_sub(j.last_progress);
            let fail = match j.phase {
                JoinPhase::LoadingLevel if waited >= self.cfg.load_timeout_us => {
                    Some(JoinFailure::LoadTimedOut)
                }
                JoinPhase::Transferring if idle >= self.cfg.transfer_stall_us => {
                    Some(JoinFailure::TransferStalled)
                }
                JoinPhase::CatchingUp if waited >= self.cfg.catch_up_timeout_us => {
                    Some(JoinFailure::CatchUpTimedOut)
                }
                _ => None,
            };
            if let Some(f) = fail {
                failed.push((*peer, f));
            }
        }
        for (peer, f) in failed {
            self.remove(peer);
            events.push(JoinEvent::Failed(peer, f));
        }
        let mut room = self.cfg.max_active.saturating_sub(self.active());
        let queued: Vec<PeerId> = self
            .order
            .iter()
            .copied()
            .filter(|p| self.phase(*p) == Some(JoinPhase::Queued))
            .collect();
        for peer in queued {
            if room == 0 {
                break;
            }
            if let Some(j) = self.joins.get_mut(&peer) {
                j.phase = JoinPhase::LoadingLevel;
                j.phase_since = now_us;
                j.last_progress = now_us;
                room -= 1;
                events.push(JoinEvent::Admitted(peer));
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i.wrapping_mul(31) % 251) as u8).collect()
    }

    // ---- chunks ---------------------------------------------------------------------------------------

    fn round_trip(len: usize, cfg: ChunkConfig, per_tick: usize) -> usize {
        let data = blob(len);
        let mut tx = ChunkSender::new(7, &data, cfg).unwrap();
        let mut rx = ChunkReceiver::new(cfg);
        let mut ticks = 0;
        while !tx.is_complete() {
            for c in tx.next_chunks(per_tick) {
                rx.receive(&c).unwrap();
            }
            tx.on_ack(&rx.ack().unwrap());
            ticks += 1;
            assert!(ticks < 100_000);
        }
        assert_eq!(rx.finish().unwrap(), data);
        ticks
    }

    #[test]
    fn blobs_of_every_awkward_size_arrive_intact() {
        let cfg = ChunkConfig {
            chunk_size: 100,
            max_bytes: 1 << 20,
        };
        for len in [1, 99, 100, 101, 199, 200, 201, 1_000, 6_400, 6_401, 12_345] {
            round_trip(len, cfg, 4);
        }
        assert_eq!(round_trip(1_000, cfg, 4), 3, "ten chunks, four per tick");
        assert_eq!(round_trip(1_000, cfg, 100), 1);
    }

    #[test]
    fn loss_and_reordering_and_duplicates_are_repaired_by_repeating_only_what_is_missing() {
        let cfg = ChunkConfig {
            chunk_size: 50,
            max_bytes: 1 << 20,
        };
        let data = blob(5_000); // 100 chunks
        let mut tx = ChunkSender::new(1, &data, cfg).unwrap();
        let mut rx = ChunkReceiver::new(cfg);
        let mut seed = 0x5EED_u64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut sent = 0;
        let mut rounds = 0;
        while !tx.is_complete() {
            let mut batch = tx.next_chunks(10);
            sent += batch.len();
            // Reverse them (reordering), drop a third (loss), and duplicate some.
            batch.reverse();
            for c in batch {
                if rnd() % 3 == 0 {
                    continue;
                }
                let _ = rx.receive(&c);
                if rnd() % 4 == 0 {
                    let _ = rx.receive(&c);
                }
            }
            // Some acks are lost too.
            if let Some(a) = rx.ack().filter(|_| rnd() % 5 != 0) {
                tx.on_ack(&a);
            }
            rounds += 1;
            assert!(rounds < 2_000, "it must finish");
        }
        assert_eq!(rx.finish().unwrap(), data);
        assert!(
            sent < 100 * 3,
            "about a third lost, so well under three copies of each chunk were sent: {sent}"
        );
        assert!(rx.duplicates() > 0);
    }

    #[test]
    fn repeats_cycle_through_the_missing_chunks_instead_of_resending_the_first_forever() {
        let cfg = ChunkConfig {
            chunk_size: 10,
            max_bytes: 1000,
        };
        let mut tx = ChunkSender::new(1, &blob(100), cfg).unwrap(); // 10 chunks
        let first: Vec<u32> = tx.next_chunks(3).iter().map(|c| c.index).collect();
        let second: Vec<u32> = tx.next_chunks(3).iter().map(|c| c.index).collect();
        assert_eq!((first, second), (vec![0, 1, 2], vec![3, 4, 5]));
        let rest: Vec<u32> = tx.next_chunks(10).iter().map(|c| c.index).collect();
        assert_eq!(
            rest,
            [6, 7, 8, 9, 0, 1, 2, 3, 4, 5],
            "wraps around to the ones not yet acknowledged"
        );
        let mut rx = ChunkReceiver::new(cfg);
        for c in tx.next_chunks(10) {
            if c.index != 4 {
                rx.receive(&c).unwrap();
            }
        }
        tx.on_ack(&rx.ack().unwrap());
        let again: Vec<u32> = tx.next_chunks(5).iter().map(|c| c.index).collect();
        assert_eq!(again, [4], "only the missing one is repeated");
    }

    #[test]
    fn the_receiver_refuses_bad_headers_and_data_and_a_refusal_changes_nothing() {
        let cfg = ChunkConfig {
            chunk_size: 10,
            max_bytes: 100,
        };
        let mut tx = ChunkSender::new(5, &blob(35), cfg).unwrap(); // 4 chunks: 10, 10, 10, 5
        let chunks = tx.next_chunks(4);
        let mut rx = ChunkReceiver::new(cfg);
        // A first chunk with an impossible header fixes nothing.
        let mut bad = chunks[0].clone();
        bad.total = 9;
        assert_eq!(rx.receive(&bad), Err(ChunkError::BadSize));
        assert!(rx.ack().is_none());
        // A first chunk of the wrong size is refused and does not fix the header.
        let mut short = chunks[0].clone();
        short.data.pop();
        assert_eq!(rx.receive(&short), Err(ChunkError::Inconsistent));
        assert!(rx.ack().is_none(), "the header was not kept");
        assert_eq!(rx.receive(&chunks[0]), Ok(true));
        // After that, every disagreement is refused.
        let mut other = chunks[1].clone();
        other.transfer = 6;
        assert_eq!(rx.receive(&other), Err(ChunkError::WrongTransfer));
        let mut total = chunks[1].clone();
        total.total = 5;
        assert_eq!(rx.receive(&total), Err(ChunkError::Inconsistent));
        let mut len = chunks[1].clone();
        len.total_len = 36;
        assert_eq!(rx.receive(&len), Err(ChunkError::Inconsistent));
        let mut sum = chunks[1].clone();
        sum.checksum ^= 1;
        assert_eq!(rx.receive(&sum), Err(ChunkError::Inconsistent));
        let mut far = chunks[1].clone();
        far.index = 4;
        assert_eq!(rx.receive(&far), Err(ChunkError::Inconsistent));
        let mut last_long = chunks[3].clone();
        last_long.data.push(0);
        assert_eq!(
            rx.receive(&last_long),
            Err(ChunkError::Inconsistent),
            "the last chunk is 5 bytes, not 6"
        );
        let mut mid_short = chunks[1].clone();
        mid_short.data.truncate(5);
        assert_eq!(rx.receive(&mid_short), Err(ChunkError::Inconsistent));
        assert_eq!(rx.received(), 1, "none of the refused chunks counted");
        assert_eq!(rx.finish(), Err(ChunkError::Incomplete));
    }

    #[test]
    fn an_oversized_header_is_refused_before_anything_is_allocated() {
        let cfg = ChunkConfig {
            chunk_size: 10,
            max_bytes: 100,
        };
        let mut rx = ChunkReceiver::new(cfg);
        let huge = Chunk {
            transfer: 1,
            index: 0,
            total: u32::MAX,
            total_len: u32::MAX,
            checksum: 0,
            data: vec![0; 10],
        };
        assert_eq!(rx.receive(&huge), Err(ChunkError::BadSize));
        let zero = Chunk {
            total: 0,
            total_len: 0,
            ..huge.clone()
        };
        assert_eq!(rx.receive(&zero), Err(ChunkError::BadSize));
        let mismatch = Chunk {
            total: 2,
            total_len: 100,
            ..huge
        };
        assert_eq!(
            rx.receive(&mismatch),
            Err(ChunkError::BadSize),
            "100 bytes in chunks of 10 is 10 chunks, not 2"
        );
    }

    #[test]
    fn a_corrupt_blob_is_detected_and_the_receiver_starts_over() {
        let cfg = ChunkConfig {
            chunk_size: 10,
            max_bytes: 100,
        };
        let data = blob(30);
        let mut tx = ChunkSender::new(1, &data, cfg).unwrap();
        let mut chunks = tx.next_chunks(3);
        chunks[1].data[0] ^= 0xFF; // damaged in transit, header untouched
        let mut rx = ChunkReceiver::new(cfg);
        for c in &chunks {
            rx.receive(c).unwrap();
        }
        assert!(rx.is_complete());
        assert_eq!(rx.finish(), Err(ChunkError::Corrupt));
        assert!(rx.ack().is_none() && rx.received() == 0, "reset");
        for c in tx.next_chunks(3) {
            rx.receive(&c).unwrap();
        }
        assert_eq!(
            rx.finish().unwrap(),
            data,
            "the same receiver takes a clean copy"
        );
    }

    #[test]
    fn senders_refuse_impossible_blobs_and_ignore_foreign_acks() {
        let cfg = ChunkConfig {
            chunk_size: 10,
            max_bytes: 100,
        };
        assert_eq!(
            ChunkSender::new(1, &[], cfg).unwrap_err(),
            ChunkError::BadSize
        );
        assert_eq!(
            ChunkSender::new(1, &blob(101), cfg).unwrap_err(),
            ChunkError::BadSize
        );
        assert!(ChunkSender::new(1, &blob(100), cfg).is_ok());
        assert_eq!(
            ChunkSender::new(
                1,
                &blob(5),
                ChunkConfig {
                    chunk_size: 0,
                    ..cfg
                }
            )
            .unwrap_err(),
            ChunkError::BadSize
        );
        let mut tx = ChunkSender::new(1, &blob(30), cfg).unwrap();
        let all = ChunkAck {
            transfer: 1,
            total: 3,
            received: vec![0b111],
        };
        tx.on_ack(&ChunkAck {
            transfer: 2,
            ..all.clone()
        });
        tx.on_ack(&ChunkAck {
            total: 4,
            ..all.clone()
        });
        assert_eq!(
            tx.acked(),
            0,
            "another transfer or another total is ignored"
        );
        tx.on_ack(&ChunkAck {
            received: vec![0b1010],
            ..all.clone()
        }); // bit 3 is beyond the total
        assert_eq!(tx.acked(), 1, "only bit 1 counts");
        tx.on_ack(&all);
        assert!(tx.is_complete());
        assert!(tx.next_chunks(5).is_empty());
        assert_eq!((all.count(), all.has(0), all.has(3)), (3, true, false));
        assert_eq!(
            ChunkAck {
                transfer: 1,
                total: 100,
                received: vec![]
            }
            .count(),
            0,
            "a short bit vector is fine"
        );
    }

    #[test]
    fn an_ack_never_reports_a_chunk_beyond_its_total_even_with_stray_bits() {
        let ack = ChunkAck {
            transfer: 1,
            total: 3,
            received: vec![0b1111],
        }; // bit 3 is not a chunk
        assert!(ack.has(2) && !ack.has(3) && !ack.has(4));
        assert_eq!(ack.count(), 3);
        let on_a_word_edge = ChunkAck {
            transfer: 1,
            total: 64,
            received: vec![u64::MAX, u64::MAX],
        };
        assert!(on_a_word_edge.has(63) && !on_a_word_edge.has(64));
        assert_eq!(on_a_word_edge.count(), 64);
    }

    #[test]
    fn the_checksum_depends_on_every_byte_and_on_order() {
        assert_ne!(checksum(b"abc"), checksum(b"abd"));
        assert_ne!(checksum(b"abc"), checksum(b"cba"));
        assert_ne!(checksum(b""), checksum(b"\0"));
        assert_eq!(checksum(b""), 0xcbf2_9ce4_8422_2325, "the FNV offset basis");
        assert_eq!(
            checksum(b"a"),
            0xaf63_dc4c_8601_ec8c,
            "the published FNV-1a value for \"a\""
        );
    }

    // ---- joining -----------------------------------------------------------------------------------------

    const LEVEL: u64 = 0xAB;

    fn manager() -> JoinManager {
        JoinManager::new(JoinConfig::default(), LEVEL)
    }

    /// Drive one joiner all the way in, returning the manager.
    fn full_join(m: &mut JoinManager, peer: PeerId, now: u64) {
        m.begin(peer, now).unwrap();
        assert_eq!(m.tick(now), [JoinEvent::Admitted(peer)]);
        assert_eq!(m.level_ready(peer, LEVEL, now + 1).unwrap(), None);
        m.start_transfer(peer, 500, 40, &blob(5_000), now + 2)
            .unwrap();
        let mut rx = ChunkReceiver::new(ChunkConfig::default());
        while !rx.is_complete() {
            for c in m.next_chunks(peer, 3) {
                rx.receive(&c).unwrap();
            }
            m.on_ack(peer, &rx.ack().unwrap(), now + 3).unwrap();
        }
        assert_eq!(m.snapshot_applied(peer, 1, now + 4).unwrap(), None);
        assert!(
            !m.progress(peer, 500, 520, now + 5).unwrap(),
            "20 ticks behind"
        );
        assert!(
            m.progress(peer, 518, 520, now + 6).unwrap(),
            "2 ticks behind is caught up"
        );
    }

    #[test]
    fn a_joiner_goes_through_every_phase_in_order() {
        let mut m = manager();
        let sam = PeerId(1);
        m.begin(sam, 0).unwrap();
        assert_eq!(
            (m.phase(sam), m.queue_position(sam)),
            (Some(JoinPhase::Queued), Some(0))
        );
        assert_eq!(m.tick(0), [JoinEvent::Admitted(sam)]);
        assert_eq!(
            (m.phase(sam), m.queue_position(sam), m.active()),
            (Some(JoinPhase::LoadingLevel), None, 1)
        );
        m.level_ready(sam, LEVEL, 10).unwrap();
        assert_eq!(m.phase(sam), Some(JoinPhase::AwaitingBaseline));
        assert_eq!(
            (m.baseline_tick(sam), m.event_cursor(sam).map(|c| c.acked())),
            (None, None),
            "no baseline before the snapshot is taken"
        );
        m.start_transfer(sam, 500, 40, &blob(2_500), 20).unwrap();
        assert_eq!(m.phase(sam), Some(JoinPhase::Transferring));
        assert_eq!(m.baseline_tick(sam), Some(500));
        assert_eq!(
            m.event_cursor(sam).unwrap().acked(),
            40,
            "events up to 40 are in the snapshot, so 41 on is sent"
        );
        full_join(&mut manager(), PeerId(9), 0);
        let mut done = manager();
        full_join(&mut done, sam, 0);
        assert_eq!(done.phase(sam), Some(JoinPhase::InGame));
        assert_eq!(done.active(), 0, "being in the game holds no place");
        done.finish(sam);
        assert_eq!(done.tracked(), 0);
    }

    #[test]
    fn steps_out_of_order_are_refused_and_change_nothing() {
        let mut m = manager();
        let p = PeerId(1);
        assert_eq!(m.level_ready(p, LEVEL, 0), Err(JoinError::Unknown));
        assert_eq!(m.start_transfer(p, 1, 1, &[1], 0), Err(JoinError::Unknown));
        m.begin(p, 0).unwrap();
        assert_eq!(m.begin(p, 0), Err(JoinError::AlreadyJoining));
        assert_eq!(
            m.level_ready(p, LEVEL, 0),
            Err(JoinError::WrongPhase(JoinPhase::Queued))
        );
        assert_eq!(
            m.start_transfer(p, 1, 1, &[1], 0),
            Err(JoinError::WrongPhase(JoinPhase::Queued))
        );
        m.tick(0);
        assert_eq!(
            m.start_transfer(p, 1, 1, &[1], 0),
            Err(JoinError::WrongPhase(JoinPhase::LoadingLevel))
        );
        assert_eq!(
            m.snapshot_applied(p, 0, 0),
            Err(JoinError::WrongPhase(JoinPhase::LoadingLevel))
        );
        assert_eq!(
            m.progress(p, 1, 1, 0),
            Err(JoinError::WrongPhase(JoinPhase::LoadingLevel))
        );
        let ack = ChunkAck {
            transfer: 1,
            total: 1,
            received: vec![1],
        };
        assert_eq!(
            m.on_ack(p, &ack, 0),
            Err(JoinError::WrongPhase(JoinPhase::LoadingLevel))
        );
        assert_eq!(m.next_chunks(p, 5), Vec::<Chunk>::new());
        m.level_ready(p, LEVEL, 1).unwrap();
        assert_eq!(
            m.level_ready(p, LEVEL, 2),
            Err(JoinError::WrongPhase(JoinPhase::AwaitingBaseline)),
            "not twice"
        );
        assert_eq!(
            m.start_transfer(p, 1, 1, &[], 2),
            Err(JoinError::Transfer(ChunkError::BadSize))
        );
        assert_eq!(
            m.phase(p),
            Some(JoinPhase::AwaitingBaseline),
            "a refused transfer leaves the phase alone"
        );
    }

    #[test]
    fn the_snapshot_cannot_be_applied_before_every_chunk_is_acknowledged() {
        let mut m = manager();
        let p = PeerId(1);
        m.begin(p, 0).unwrap();
        m.tick(0);
        m.level_ready(p, LEVEL, 0).unwrap();
        m.start_transfer(p, 10, 5, &blob(3_000), 0).unwrap();
        assert_eq!(
            m.snapshot_applied(p, 0, 1),
            Err(JoinError::Transfer(ChunkError::Incomplete))
        );
        assert_eq!(m.phase(p), Some(JoinPhase::Transferring));
    }

    #[test]
    fn a_level_mismatch_fails_the_join_at_once_and_frees_the_place() {
        let mut m = manager();
        let p = PeerId(1);
        m.begin(p, 0).unwrap();
        m.tick(0);
        assert_eq!(
            m.level_ready(p, LEVEL + 1, 5).unwrap(),
            Some(JoinFailure::LevelMismatch)
        );
        assert_eq!((m.phase(p), m.tracked(), m.active()), (None, 0, 0));
    }

    #[test]
    fn only_so_many_joiners_are_served_at_once_and_the_queue_is_first_come_first_served() {
        let mut m = JoinManager::new(
            JoinConfig {
                max_active: 2,
                ..JoinConfig::default()
            },
            LEVEL,
        );
        for i in 1..=5 {
            m.begin(PeerId(i), i).unwrap();
        }
        assert_eq!(
            m.tick(10),
            [
                JoinEvent::Admitted(PeerId(1)),
                JoinEvent::Admitted(PeerId(2))
            ]
        );
        assert_eq!(
            (m.queue_position(PeerId(3)), m.queue_position(PeerId(5))),
            (Some(0), Some(2))
        );
        assert!(m.tick(11).is_empty(), "no room");
        m.cancel(PeerId(1));
        assert_eq!(m.tick(12), [JoinEvent::Admitted(PeerId(3))]);
        assert_eq!(m.queue_position(PeerId(4)), Some(0));
        assert!(!m.cancel(PeerId(1)), "already gone");
    }

    #[test]
    fn the_number_of_tracked_joiners_is_limited() {
        let mut m = JoinManager::new(
            JoinConfig {
                max_tracked: 3,
                ..JoinConfig::default()
            },
            LEVEL,
        );
        for i in 0..3 {
            m.begin(PeerId(i), 0).unwrap();
        }
        assert_eq!(m.begin(PeerId(9), 0), Err(JoinError::TooMany));
        m.cancel(PeerId(0));
        assert!(m.begin(PeerId(9), 0).is_ok());
    }

    #[test]
    fn stalls_and_timeouts_fail_the_join_at_the_exact_time() {
        let cfg = JoinConfig::default();
        // Loading the level.
        let mut m = JoinManager::new(cfg, LEVEL);
        m.begin(PeerId(1), 0).unwrap();
        m.tick(0);
        assert!(m.tick(cfg.load_timeout_us - 1).is_empty());
        assert_eq!(
            m.tick(cfg.load_timeout_us),
            [JoinEvent::Failed(PeerId(1), JoinFailure::LoadTimedOut)]
        );
        assert_eq!(m.tracked(), 0);
        // A transfer that stops making progress; progress resets the timer, repeated acks do not.
        let mut t = JoinManager::new(cfg, LEVEL);
        t.begin(PeerId(1), 0).unwrap();
        t.tick(0);
        t.level_ready(PeerId(1), LEVEL, 0).unwrap();
        t.start_transfer(PeerId(1), 1, 1, &blob(5_000), 0).unwrap();
        let mut rx = ChunkReceiver::new(cfg.chunks);
        for c in t.next_chunks(PeerId(1), 1) {
            rx.receive(&c).unwrap();
        }
        t.on_ack(PeerId(1), &rx.ack().unwrap(), 5_000_000).unwrap(); // progress at 5 s
        t.on_ack(PeerId(1), &rx.ack().unwrap(), 9_000_000).unwrap(); // the same ack again: no progress
        assert!(
            t.tick(14_999_999).is_empty(),
            "10 s after the last new chunk is 15 s"
        );
        assert_eq!(
            t.tick(15_000_000),
            [JoinEvent::Failed(PeerId(1), JoinFailure::TransferStalled)]
        );
        // Catching up.
        let mut c = JoinManager::new(cfg, LEVEL);
        c.begin(PeerId(1), 0).unwrap();
        c.tick(0);
        c.level_ready(PeerId(1), LEVEL, 0).unwrap();
        c.start_transfer(PeerId(1), 1, 1, &blob(10), 0).unwrap();
        for ch in c.next_chunks(PeerId(1), 5) {
            let mut r = ChunkReceiver::new(cfg.chunks);
            r.receive(&ch).unwrap();
            c.on_ack(PeerId(1), &r.ack().unwrap(), 1).unwrap();
        }
        c.snapshot_applied(PeerId(1), 0, 100).unwrap();
        assert!(c.tick(100 + cfg.catch_up_timeout_us - 1).is_empty());
        assert_eq!(
            c.tick(100 + cfg.catch_up_timeout_us),
            [JoinEvent::Failed(PeerId(1), JoinFailure::CatchUpTimedOut)]
        );
    }

    #[test]
    fn a_failed_join_makes_room_for_the_next_in_the_same_tick() {
        let cfg = JoinConfig {
            max_active: 1,
            ..JoinConfig::default()
        };
        let mut m = JoinManager::new(cfg, LEVEL);
        m.begin(PeerId(1), 0).unwrap();
        m.begin(PeerId(2), 0).unwrap();
        assert_eq!(m.tick(0), [JoinEvent::Admitted(PeerId(1))]);
        let events = m.tick(cfg.load_timeout_us);
        assert_eq!(
            events,
            [
                JoinEvent::Failed(PeerId(1), JoinFailure::LoadTimedOut),
                JoinEvent::Admitted(PeerId(2))
            ]
        );
    }

    #[test]
    fn a_snapshot_whose_events_were_pruned_is_too_old_to_use() {
        let mut m = manager();
        let p = PeerId(1);
        m.begin(p, 0).unwrap();
        m.tick(0);
        m.level_ready(p, LEVEL, 0).unwrap();
        m.start_transfer(p, 100, 40, &blob(10), 0).unwrap();
        let mut rx = ChunkReceiver::new(ChunkConfig::default());
        for c in m.next_chunks(p, 5) {
            rx.receive(&c).unwrap();
        }
        m.on_ack(p, &rx.ack().unwrap(), 1).unwrap();
        // The log now starts at 43: events 41 and 42 are gone.
        assert_eq!(
            m.snapshot_applied(p, 43, 2).unwrap(),
            Some(JoinFailure::SnapshotTooOld)
        );
        assert_eq!(m.tracked(), 0);
        // One that starts exactly at the first needed event (41) is fine, and so is one that starts earlier.
        for oldest in [41, 1, 0] {
            let mut ok = manager();
            ok.begin(p, 0).unwrap();
            ok.tick(0);
            ok.level_ready(p, LEVEL, 0).unwrap();
            ok.start_transfer(p, 100, 40, &blob(10), 0).unwrap();
            let mut r = ChunkReceiver::new(ChunkConfig::default());
            for c in ok.next_chunks(p, 5) {
                r.receive(&c).unwrap();
            }
            ok.on_ack(p, &r.ack().unwrap(), 1).unwrap();
            assert_eq!(
                ok.snapshot_applied(p, oldest, 2).unwrap(),
                None,
                "oldest {oldest}"
            );
        }
    }

    #[test]
    fn the_event_cursor_skips_what_the_snapshot_holds_and_loses_nothing_after_it() {
        use crate::events::{EventConfig, EventLog, EventReceiver};
        let mut log: EventLog<&str> = EventLog::new(EventConfig::default());
        for i in 0..5 {
            log.emit(i, "before");
        }
        let at_snapshot = log.latest_seq();
        let mut m = manager();
        let p = PeerId(1);
        m.begin(p, 0).unwrap();
        m.tick(0);
        m.level_ready(p, LEVEL, 0).unwrap();
        m.start_transfer(p, 10, at_snapshot, &blob(10), 0).unwrap();
        // Events keep happening while the snapshot is in flight.
        let during = log.emit(11, "during");
        let after = log.emit(12, "after");
        let cursor = m.event_cursor(p).unwrap();
        let mut rx = EventReceiver::new(cursor.clone());
        let got = rx.receive(&log.packet(&cursor));
        assert_eq!(
            got.iter().map(|e| e.event).collect::<Vec<_>>(),
            ["during", "after"],
            "none of the five old ones, both newer ones"
        );
        assert_eq!((got[0].seq, got[1].seq), (during, after));
    }

    #[test]
    fn a_new_level_cancels_everyone_in_progress() {
        let mut m = manager();
        m.begin(PeerId(1), 0).unwrap();
        m.begin(PeerId(2), 0).unwrap();
        m.tick(0);
        let events = m.set_level(0xCD);
        assert_eq!(
            events,
            [
                JoinEvent::Failed(PeerId(1), JoinFailure::Cancelled),
                JoinEvent::Failed(PeerId(2), JoinFailure::Cancelled)
            ]
        );
        assert_eq!(m.tracked(), 0);
        m.begin(PeerId(3), 1).unwrap();
        m.tick(1);
        assert_eq!(
            m.level_ready(PeerId(3), 0xCD, 2).unwrap(),
            None,
            "checked against the new level"
        );
    }

    #[test]
    fn exactly_the_allowed_number_of_ticks_behind_counts_as_caught_up_and_one_more_does_not() {
        let slack = JoinConfig::default().catch_up_ticks;
        for (behind, expect) in [(0, true), (slack, true), (slack + 1, false)] {
            let mut m = manager();
            let p = PeerId(1);
            m.begin(p, 0).unwrap();
            m.tick(0);
            m.level_ready(p, LEVEL, 0).unwrap();
            m.start_transfer(p, 0, 0, &blob(10), 0).unwrap();
            let mut rx = ChunkReceiver::new(ChunkConfig::default());
            for c in m.next_chunks(p, 5) {
                rx.receive(&c).unwrap();
            }
            m.on_ack(p, &rx.ack().unwrap(), 0).unwrap();
            m.snapshot_applied(p, 0, 0).unwrap();
            assert_eq!(
                m.progress(p, 100, 100 + behind, 1).unwrap(),
                expect,
                "{behind} ticks behind"
            );
        }
    }

    #[test]
    fn catching_up_does_not_underflow_when_the_joiner_is_ahead_or_ticks_are_huge() {
        let mut m = manager();
        let p = PeerId(1);
        m.begin(p, 0).unwrap();
        m.tick(0);
        m.level_ready(p, LEVEL, 0).unwrap();
        m.start_transfer(p, 0, 0, &blob(10), 0).unwrap();
        let mut rx = ChunkReceiver::new(ChunkConfig::default());
        for c in m.next_chunks(p, 5) {
            rx.receive(&c).unwrap();
        }
        m.on_ack(p, &rx.ack().unwrap(), 0).unwrap();
        m.snapshot_applied(p, 0, 0).unwrap();
        assert!(
            m.progress(p, u64::MAX, 5, 1).unwrap(),
            "ahead of the server counts as caught up"
        );
    }
}
