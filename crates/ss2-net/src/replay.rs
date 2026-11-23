//! Replay files: a recorded session that can be played back for a bug report (roadmap T08-025).
//!
//! A replay is the **header** (build, protocol, schema hash, tick rate, random seed, map, players),
//! then **records** in tick order: the players' inputs, **snapshots** (keyframes to start playback
//! from), gameplay events and per-tick **state hashes** (to see where a playback stops matching the
//! recording), then an **end** record. Payloads are opaque bytes: this module neither knows nor
//! cares what an input or a snapshot contains.
//!
//! Everything is built for files that are **cut short or damaged** (a crash is when a bug report is
//! needed most) and for files that are **hostile** (someone attaches one to an issue): every record
//! carries a CRC-32, a length is checked before anything is read, memory is bounded by [`Limits`],
//! and a bad record ends the replay at the last good one instead of failing the whole file.
//!
//! The format is documented in `docs/REPLAY.md`. Nothing here reads a clock or uses a hash map.

use std::fmt;

/// The four bytes every replay file starts with.
pub const MAGIC: [u8; 4] = *b"SS2R";
/// The format version this code reads and writes.
pub const FORMAT_VERSION: u16 = 1;

const MAX_STRING: usize = 256;
const MAX_META: usize = 16;
const MAX_HEADER_BYTES: usize = 64 * 1024;

const KIND_INPUT: u8 = 1;
const KIND_SNAPSHOT: u8 = 2;
const KIND_EVENT: u8 = 3;
const KIND_STATE_HASH: u8 = 4;
const KIND_END: u8 = 0xFF;

const fn crc_table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut bit = 0;
        while bit < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            bit += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

/// The CRC-32 (IEEE 802.3, the one of zip and PNG) of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in data {
        crc = CRC_TABLE[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

/// What a replay says about the session it recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayHeader {
    /// The build that recorded it (`ss2rs 0.2.0`).
    pub build: String,
    /// The network protocol version (`ProtocolVersion` packed by the caller).
    pub protocol: u32,
    /// The schema hash of `ss2_net::protocol`: replays of another schema cannot be played.
    pub schema_hash: u64,
    /// Simulation ticks per second, at least 1.
    pub tick_hz: u32,
    /// The random seed of the session.
    pub seed: u64,
    /// The level.
    pub map: String,
    /// How many players there were.
    pub players: u8,
    /// A few notes (host name, game mode, difficulty), at most 16 pairs.
    pub meta: Vec<(String, String)>,
}

impl ReplayHeader {
    fn validate(&self) -> Result<(), &'static str> {
        if self.tick_hz == 0 {
            return Err("tick_hz is 0");
        }
        if self.build.len() > MAX_STRING || self.map.len() > MAX_STRING {
            return Err("a text is longer than 256 bytes");
        }
        if self.meta.len() > MAX_META {
            return Err("more than 16 meta entries");
        }
        if self
            .meta
            .iter()
            .any(|(k, v)| k.len() > MAX_STRING || v.len() > MAX_STRING)
        {
            return Err("a meta text is longer than 256 bytes");
        }
        Ok(())
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_str(&mut out, &self.build);
        out.extend_from_slice(&self.protocol.to_le_bytes());
        out.extend_from_slice(&self.schema_hash.to_le_bytes());
        out.extend_from_slice(&self.tick_hz.to_le_bytes());
        out.extend_from_slice(&self.seed.to_le_bytes());
        put_str(&mut out, &self.map);
        out.push(self.players);
        out.push(self.meta.len() as u8);
        for (k, v) in &self.meta {
            put_str(&mut out, k);
            put_str(&mut out, v);
        }
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self, ReplayError> {
        let mut r = Cursor { bytes, pos: 0 };
        let header = Self {
            build: r.string()?,
            protocol: r.u32()?,
            schema_hash: r.u64()?,
            tick_hz: r.u32()?,
            seed: r.u64()?,
            map: r.string()?,
            players: r.u8()?,
            meta: {
                let count = usize::from(r.u8()?);
                if count > MAX_META {
                    return Err(ReplayError::HeaderInvalid("more than 16 meta entries"));
                }
                let mut meta = Vec::with_capacity(count);
                for _ in 0..count {
                    meta.push((r.string()?, r.string()?));
                }
                meta
            },
        };
        if r.pos != bytes.len() {
            return Err(ReplayError::HeaderInvalid("bytes left over in the header"));
        }
        header.validate().map_err(ReplayError::HeaderInvalid)?;
        Ok(header)
    }
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u16).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], ReplayError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len());
        let end = end.ok_or(ReplayError::HeaderInvalid(
            "the header ends in the middle of a field",
        ))?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, ReplayError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, ReplayError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    fn u64(&mut self) -> Result<u64, ReplayError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }
    fn string(&mut self) -> Result<String, ReplayError> {
        let len = usize::from(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ));
        if len > MAX_STRING {
            return Err(ReplayError::HeaderInvalid(
                "a text is longer than 256 bytes",
            ));
        }
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| ReplayError::HeaderInvalid("a text is not UTF-8"))
    }
}

/// One thing that happened, stamped with the simulation tick it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// A player's input for `tick` (the bytes are whatever the game encodes).
    Input {
        /// The tick.
        tick: u64,
        /// Which player.
        player: u8,
        /// The encoded input.
        data: Vec<u8>,
    },
    /// The whole game state **before** the inputs of `tick` are applied: a place to start from.
    Snapshot {
        /// The tick.
        tick: u64,
        /// The encoded state.
        data: Vec<u8>,
    },
    /// A gameplay event (door opened, boss killed), for readers of the replay.
    Event {
        /// The tick.
        tick: u64,
        /// The encoded event.
        data: Vec<u8>,
    },
    /// A hash of the game state after `tick`, to find where a playback stops matching.
    StateHash {
        /// The tick.
        tick: u64,
        /// The hash.
        hash: u64,
    },
}

impl Record {
    /// The tick of the record.
    pub fn tick(&self) -> u64 {
        match self {
            Self::Input { tick, .. }
            | Self::Snapshot { tick, .. }
            | Self::Event { tick, .. }
            | Self::StateHash { tick, .. } => *tick,
        }
    }

    fn kind_and_payload(&self) -> (u8, Vec<u8>) {
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.tick().to_le_bytes());
        match self {
            Self::Input { player, data, .. } => {
                payload.push(*player);
                payload.extend_from_slice(data);
                (KIND_INPUT, payload)
            }
            Self::Snapshot { data, .. } => {
                payload.extend_from_slice(data);
                (KIND_SNAPSHOT, payload)
            }
            Self::Event { data, .. } => {
                payload.extend_from_slice(data);
                (KIND_EVENT, payload)
            }
            Self::StateHash { hash, .. } => {
                payload.extend_from_slice(&hash.to_le_bytes());
                (KIND_STATE_HASH, payload)
            }
        }
    }

    fn decode(kind: u8, payload: &[u8]) -> Option<Self> {
        let tick = u64::from_le_bytes(payload.get(..8)?.try_into().ok()?);
        let rest = &payload[8..];
        match kind {
            KIND_INPUT => {
                let (player, data) = rest.split_first()?;
                Some(Self::Input {
                    tick,
                    player: *player,
                    data: data.to_vec(),
                })
            }
            KIND_SNAPSHOT => Some(Self::Snapshot {
                tick,
                data: rest.to_vec(),
            }),
            KIND_EVENT => Some(Self::Event {
                tick,
                data: rest.to_vec(),
            }),
            KIND_STATE_HASH => Some(Self::StateHash {
                tick,
                hash: u64::from_le_bytes(rest.try_into().ok()?),
            }),
            _ => None,
        }
    }
}

/// Why a record could not be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// The header breaks a limit.
    HeaderInvalid(&'static str),
    /// A record is for an earlier tick than the one before it.
    TickWentBackwards {
        /// The tick of the previous record.
        last: u64,
        /// The tick of the refused record.
        got: u64,
    },
    /// The record is bigger than [`Limits::max_record_bytes`] allows by default.
    RecordTooLarge {
        /// Its size.
        len: usize,
        /// The most allowed.
        max: usize,
    },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderInvalid(why) => write!(f, "invalid replay header: {why}"),
            Self::TickWentBackwards { last, got } => {
                write!(f, "a record for tick {got} after one for tick {last}")
            }
            Self::RecordTooLarge { len, max } => {
                write!(f, "a record of {len} bytes (the most is {max})")
            }
        }
    }
}

impl std::error::Error for WriteError {}

/// Builds a replay in memory; see [`ReplayWriter::drain`] to stream a long session to disk.
#[derive(Debug)]
pub struct ReplayWriter {
    pending: Vec<u8>,
    last_tick: Option<u64>,
    count: u64,
}

impl ReplayWriter {
    /// Start a replay with `header`.
    pub fn new(header: ReplayHeader) -> Result<Self, WriteError> {
        header.validate().map_err(WriteError::HeaderInvalid)?;
        let body = header.encode();
        let mut pending = Vec::new();
        pending.extend_from_slice(&MAGIC);
        pending.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        pending.extend_from_slice(&(body.len() as u32).to_le_bytes());
        pending.extend_from_slice(&body);
        pending.extend_from_slice(&crc32(&body).to_le_bytes());
        Ok(Self {
            pending,
            last_tick: None,
            count: 0,
        })
    }

    fn encode(&mut self, record: &Record) {
        let (kind, payload) = record.kind_and_payload();
        let mut framed = Vec::with_capacity(payload.len() + 9);
        framed.push(kind);
        framed.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        framed.extend_from_slice(&payload);
        let crc = crc32(&framed);
        framed.extend_from_slice(&crc.to_le_bytes());
        self.pending.extend_from_slice(&framed);
        self.last_tick = Some(record.tick());
        self.count += 1;
    }

    /// Add a record. Ticks must not go backwards, and a record must fit the default limit.
    pub fn push(&mut self, record: Record) -> Result<(), WriteError> {
        if let Some(last) = self.last_tick
            && record.tick() < last
        {
            return Err(WriteError::TickWentBackwards {
                last,
                got: record.tick(),
            });
        }
        let (_, payload) = record.kind_and_payload();
        let max = Limits::default().max_record_bytes;
        if payload.len() > max {
            return Err(WriteError::RecordTooLarge {
                len: payload.len(),
                max,
            });
        }
        self.encode(&record);
        Ok(())
    }

    #[cfg(test)]
    fn push_unchecked_for_test(&mut self, record: Record) {
        self.encode(&record);
    }

    /// Take the bytes written so far (the header first), to append them to a file; what comes
    /// out is never repeated.
    pub fn drain(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }

    /// End the replay: the bytes not yet drained, plus the end record that says it is complete.
    pub fn finish(mut self) -> Vec<u8> {
        let payload = self.count.to_le_bytes();
        let mut framed = vec![KIND_END];
        framed.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        framed.extend_from_slice(&payload);
        let crc = crc32(&framed);
        framed.extend_from_slice(&crc.to_le_bytes());
        self.pending.extend_from_slice(&framed);
        self.pending
    }
}

/// What [`Replay::parse`] may use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The largest record payload accepted.
    pub max_record_bytes: usize,
    /// The most records kept.
    pub max_records: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_record_bytes: 4 * 1024 * 1024,
            max_records: 5_000_000,
        }
    }
}

/// Why a replay cannot be used at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// The file does not start with the replay magic.
    BadMagic,
    /// A format version this code does not read.
    UnsupportedVersion {
        /// The version in the file.
        found: u16,
    },
    /// The file ends inside the header.
    HeaderTruncated,
    /// The header fails its checksum.
    HeaderCorrupt,
    /// The header breaks a limit or is malformed.
    HeaderInvalid(&'static str),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a replay file"),
            Self::UnsupportedVersion { found } => write!(
                f,
                "replay format version {found} (this build reads {FORMAT_VERSION})"
            ),
            Self::HeaderTruncated => write!(f, "the replay ends inside its header"),
            Self::HeaderCorrupt => write!(f, "the replay header is damaged"),
            Self::HeaderInvalid(why) => write!(f, "invalid replay header: {why}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Why reading stopped before a complete end record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TruncationReason {
    /// The records ended cleanly but there is no end record (the recorder never finished).
    MissingEnd,
    /// The file ends in the middle of a record.
    CutShort,
    /// A record fails its checksum.
    BadChecksum,
    /// A record of a kind this version does not know.
    UnknownKind(u8),
    /// A record claims more than [`Limits::max_record_bytes`].
    TooLarge,
    /// A record is for an earlier tick than the one before.
    OutOfOrder,
    /// More records than [`Limits::max_records`].
    TooManyRecords,
    /// A record's payload is malformed.
    BadPayload,
    /// The end record's count differs from the records read.
    CountMismatch,
    /// Bytes follow the end record.
    TrailingData,
}

/// How a replay ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayEnd {
    /// An end record with the right count closes the file.
    Complete,
    /// Reading stopped at `offset` (a byte of the file); the records before it are good.
    Truncated {
        /// Where reading stopped.
        offset: usize,
        /// Why.
        reason: TruncationReason,
    },
}

/// A replay read into memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replay {
    header: ReplayHeader,
    records: Vec<Record>,
    end: ReplayEnd,
    keyframes: Vec<usize>,
}

impl Replay {
    /// Read a replay. Only a bad header is an error; a bad or missing end leaves the good records
    /// and says so in [`Replay::end`].
    pub fn parse(bytes: &[u8], limits: &Limits) -> Result<Self, ReplayError> {
        if bytes.len() < 4 || bytes[..4] != MAGIC {
            return Err(ReplayError::BadMagic);
        }
        let version = u16::from_le_bytes(
            bytes
                .get(4..6)
                .ok_or(ReplayError::HeaderTruncated)?
                .try_into()
                .expect("2 bytes"),
        );
        if version != FORMAT_VERSION {
            return Err(ReplayError::UnsupportedVersion { found: version });
        }
        let header_len = u32::from_le_bytes(
            bytes
                .get(6..10)
                .ok_or(ReplayError::HeaderTruncated)?
                .try_into()
                .expect("4 bytes"),
        ) as usize;
        if header_len > MAX_HEADER_BYTES {
            return Err(ReplayError::HeaderInvalid(
                "the header is larger than 64 KiB",
            ));
        }
        let body_end = 10 + header_len;
        let body = bytes
            .get(10..body_end)
            .ok_or(ReplayError::HeaderTruncated)?;
        let crc = u32::from_le_bytes(
            bytes
                .get(body_end..body_end + 4)
                .ok_or(ReplayError::HeaderTruncated)?
                .try_into()
                .expect("4 bytes"),
        );
        if crc32(body) != crc {
            return Err(ReplayError::HeaderCorrupt);
        }
        let header = ReplayHeader::decode(body)?;

        let mut records: Vec<Record> = Vec::new();
        let mut pos = body_end + 4;
        let end = loop {
            let stop = |reason| ReplayEnd::Truncated {
                offset: pos,
                reason,
            };
            if pos == bytes.len() {
                break stop(TruncationReason::MissingEnd);
            }
            let Some(head) = bytes.get(pos..pos + 5) else {
                break stop(TruncationReason::CutShort);
            };
            let kind = head[0];
            if !matches!(
                kind,
                KIND_INPUT | KIND_SNAPSHOT | KIND_EVENT | KIND_STATE_HASH | KIND_END
            ) {
                break stop(TruncationReason::UnknownKind(kind));
            }
            let len = u32::from_le_bytes(head[1..5].try_into().expect("4 bytes")) as usize;
            if len > limits.max_record_bytes {
                break stop(TruncationReason::TooLarge);
            }
            let total = 5 + len + 4;
            let Some(frame) = bytes.get(pos..pos + total) else {
                break stop(TruncationReason::CutShort);
            };
            let (signed, crc_bytes) = frame.split_at(total - 4);
            if crc32(signed) != u32::from_le_bytes(crc_bytes.try_into().expect("4 bytes")) {
                break stop(TruncationReason::BadChecksum);
            }
            let payload = &signed[5..];
            if kind == KIND_END {
                let Ok(count) = <[u8; 8]>::try_from(payload) else {
                    break stop(TruncationReason::BadPayload);
                };
                if u64::from_le_bytes(count) != records.len() as u64 {
                    break stop(TruncationReason::CountMismatch);
                }
                if pos + total != bytes.len() {
                    break ReplayEnd::Truncated {
                        offset: pos + total,
                        reason: TruncationReason::TrailingData,
                    };
                }
                break ReplayEnd::Complete;
            }
            let Some(record) = Record::decode(kind, payload) else {
                break stop(TruncationReason::BadPayload);
            };
            if records
                .last()
                .is_some_and(|last| record.tick() < last.tick())
            {
                break stop(TruncationReason::OutOfOrder);
            }
            if records.len() >= limits.max_records {
                break stop(TruncationReason::TooManyRecords);
            }
            records.push(record);
            pos += total;
        };

        let keyframes = records
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, Record::Snapshot { .. }))
            .map(|(i, _)| i)
            .collect();
        Ok(Self {
            header,
            records,
            end,
            keyframes,
        })
    }

    /// The header.
    pub fn header(&self) -> &ReplayHeader {
        &self.header
    }

    /// The records, in tick order.
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// How the file ended.
    pub fn end(&self) -> &ReplayEnd {
        &self.end
    }

    /// The tick of the last record.
    pub fn last_tick(&self) -> Option<u64> {
        self.records.last().map(Record::tick)
    }

    /// The ticks that have a snapshot.
    pub fn keyframes(&self) -> Vec<u64> {
        self.keyframes
            .iter()
            .map(|i| self.records[*i].tick())
            .collect()
    }

    /// The latest snapshot at or before `tick`: its tick and its bytes.
    pub fn snapshot_at_or_before(&self, tick: u64) -> Option<(u64, &[u8])> {
        let after = self
            .keyframes
            .partition_point(|i| self.records[*i].tick() <= tick);
        let index = *self.keyframes.get(after.checked_sub(1)?)?;
        match &self.records[index] {
            Record::Snapshot { tick, data } => Some((*tick, data)),
            _ => None,
        }
    }
}

/// Whether a state hash from a playback matches the recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashCheck {
    /// Same hash.
    Match,
    /// Different: the playback has drifted from the recording.
    Mismatch {
        /// The recorded hash.
        expected: u64,
    },
    /// The recording has no hash for that tick.
    NotRecorded,
}

/// A cursor over a [`Replay`] for playing it back.
#[derive(Clone, Debug)]
pub struct Playback<'a> {
    replay: &'a Replay,
    cursor: usize,
}

impl<'a> Playback<'a> {
    /// Start at the first record.
    pub fn new(replay: &'a Replay) -> Self {
        Self { replay, cursor: 0 }
    }

    /// Jump to the latest snapshot at or before `tick`: returns its tick and bytes (restore them,
    /// then call [`Playback::step`] to get that tick's inputs). `None` when there is no snapshot.
    pub fn seek(&mut self, tick: u64) -> Option<(u64, &'a [u8])> {
        let (snapshot_tick, data) = self.replay.snapshot_at_or_before(tick)?;
        self.cursor = self
            .replay
            .records
            .partition_point(|r| r.tick() < snapshot_tick);
        Some((snapshot_tick, data))
    }

    /// The next tick that has records, with its inputs, events and hashes in recorded order
    /// (snapshots are left out).
    pub fn step(&mut self) -> Option<(u64, Vec<&'a Record>)> {
        let records = &self.replay.records;
        while let Some(record) = records.get(self.cursor) {
            if matches!(record, Record::Snapshot { .. }) {
                self.cursor += 1;
                continue;
            }
            let tick = record.tick();
            let mut group = Vec::new();
            while let Some(r) = records.get(self.cursor).filter(|r| r.tick() == tick) {
                if !matches!(r, Record::Snapshot { .. }) {
                    group.push(r);
                }
                self.cursor += 1;
            }
            return Some((tick, group));
        }
        None
    }

    /// Compare the state hash of a playback at `tick` with the recorded one.
    pub fn check_hash(&self, tick: u64, actual: u64) -> HashCheck {
        let records = &self.replay.records;
        let start = records.partition_point(|r| r.tick() < tick);
        records[start..]
            .iter()
            .take_while(|r| r.tick() == tick)
            .find_map(|r| match r {
                Record::StateHash { hash, .. } => Some(*hash),
                _ => None,
            })
            .map_or(HashCheck::NotRecorded, |expected| {
                if expected == actual {
                    HashCheck::Match
                } else {
                    HashCheck::Mismatch { expected }
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> ReplayHeader {
        ReplayHeader {
            build: "ss2rs 0.0.0".into(),
            protocol: 7,
            schema_hash: 0xDEAD_BEEF_0123_4567,
            tick_hz: 30,
            seed: 42,
            map: "Levels/Egypt/Sirius".into(),
            players: 2,
            meta: vec![("host".into(), "test".into())],
        }
    }

    fn records() -> Vec<Record> {
        vec![
            Record::Snapshot {
                tick: 0,
                data: vec![0xAA; 20],
            },
            Record::Input {
                tick: 0,
                player: 0,
                data: vec![1, 2, 3],
            },
            Record::Input {
                tick: 0,
                player: 1,
                data: vec![4],
            },
            Record::StateHash { tick: 0, hash: 11 },
            Record::Input {
                tick: 1,
                player: 0,
                data: vec![],
            },
            Record::Event {
                tick: 1,
                data: b"door opened".to_vec(),
            },
            Record::StateHash { tick: 1, hash: 12 },
            Record::Snapshot {
                tick: 2,
                data: vec![0xBB; 5],
            },
            Record::Input {
                tick: 2,
                player: 1,
                data: vec![9; 40],
            },
            Record::StateHash { tick: 2, hash: 13 },
            Record::Input {
                tick: 5,
                player: 0,
                data: vec![7],
            },
        ]
    }

    fn write_all(records: &[Record]) -> Vec<u8> {
        let mut w = ReplayWriter::new(header()).unwrap();
        for r in records {
            w.push(r.clone()).unwrap();
        }
        w.finish()
    }

    #[test]
    fn crc32_matches_the_standard_check_values() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"a"), 0xE8B7_BE43);
    }

    #[test]
    fn a_written_replay_reads_back_exactly() {
        let bytes = write_all(&records());
        let replay = Replay::parse(&bytes, &Limits::default()).unwrap();
        assert_eq!(replay.header(), &header());
        assert_eq!(replay.records(), records().as_slice());
        assert_eq!(replay.end(), &ReplayEnd::Complete);
        assert_eq!(replay.last_tick(), Some(5));
        assert_eq!(replay.keyframes(), vec![0, 2]);
    }

    #[test]
    fn an_empty_replay_is_complete_and_has_no_records() {
        let bytes = write_all(&[]);
        let replay = Replay::parse(&bytes, &Limits::default()).unwrap();
        assert!(replay.records().is_empty());
        assert_eq!(replay.end(), &ReplayEnd::Complete);
        assert_eq!(replay.last_tick(), None);
    }

    #[test]
    fn the_same_recording_gives_the_same_bytes() {
        assert_eq!(write_all(&records()), write_all(&records()));
    }

    #[test]
    fn the_writer_refuses_what_the_reader_could_not_take() {
        let mut w = ReplayWriter::new(header()).unwrap();
        w.push(Record::Input {
            tick: 5,
            player: 0,
            data: vec![],
        })
        .unwrap();
        assert_eq!(
            w.push(Record::Input {
                tick: 4,
                player: 0,
                data: vec![]
            }),
            Err(WriteError::TickWentBackwards { last: 5, got: 4 })
        );
        w.push(Record::Input {
            tick: 5,
            player: 1,
            data: vec![],
        })
        .expect("the same tick again is fine");
        let big = vec![0_u8; Limits::default().max_record_bytes + 1];
        assert!(matches!(
            w.push(Record::Event { tick: 9, data: big }),
            Err(WriteError::RecordTooLarge { .. })
        ));

        let mut bad = header();
        bad.map = "x".repeat(300);
        assert!(matches!(
            ReplayWriter::new(bad),
            Err(WriteError::HeaderInvalid(_))
        ));
        let mut bad = header();
        bad.meta = (0..40).map(|i| (format!("k{i}"), "v".into())).collect();
        assert!(matches!(
            ReplayWriter::new(bad),
            Err(WriteError::HeaderInvalid(_))
        ));
        let mut bad = header();
        bad.tick_hz = 0;
        assert!(matches!(
            ReplayWriter::new(bad),
            Err(WriteError::HeaderInvalid(_))
        ));
    }

    #[test]
    fn a_file_cut_anywhere_gives_a_prefix_of_the_records_never_a_panic() {
        let all = records();
        let bytes = write_all(&all);
        let header_end = Replay::parse(&write_all(&[]), &Limits::default())
            .map(|_| ())
            .map(|()| 0);
        assert!(header_end.is_ok());
        let mut saw_truncated = false;
        for cut in 0..bytes.len() {
            match Replay::parse(&bytes[..cut], &Limits::default()) {
                Err(_) => {} // inside the header: nothing can be used
                Ok(replay) => {
                    let got = replay.records();
                    assert!(
                        got.len() <= all.len() && got == &all[..got.len()],
                        "cut at {cut}: not a prefix"
                    );
                    assert!(
                        matches!(replay.end(), ReplayEnd::Truncated { .. }),
                        "cut at {cut} cannot be complete"
                    );
                    saw_truncated = true;
                }
            }
        }
        assert!(saw_truncated);
        // Cut exactly before the end record: every record is there, but the file never said it was complete.
        let end_len = 1 + 4 + 8 + 4; // kind, length, count, crc
        let replay = Replay::parse(&bytes[..bytes.len() - end_len], &Limits::default()).unwrap();
        assert_eq!(replay.records(), all.as_slice());
        assert!(matches!(
            replay.end(),
            ReplayEnd::Truncated {
                reason: TruncationReason::MissingEnd,
                ..
            }
        ));
    }

    #[test]
    fn any_single_flipped_bit_is_detected_or_harmless() {
        let all = records();
        let bytes = write_all(&all);
        for position in 0..bytes.len() {
            for bit in [0_u8, 3, 7] {
                let mut damaged = bytes.clone();
                damaged[position] ^= 1 << bit;
                match Replay::parse(&damaged, &Limits::default()) {
                    Err(_) => {}
                    Ok(replay) => {
                        let got = replay.records();
                        assert!(
                            got.len() <= all.len() && got == &all[..got.len()],
                            "bit {bit} of byte {position}: the records changed without notice"
                        );
                        assert!(
                            matches!(replay.end(), ReplayEnd::Truncated { .. }),
                            "bit {bit} of byte {position}: a damaged file was called complete"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_file_that_is_not_a_replay_or_is_from_the_future_is_refused_by_name() {
        assert_eq!(
            Replay::parse(b"", &Limits::default()),
            Err(ReplayError::BadMagic)
        );
        assert_eq!(
            Replay::parse(b"PK\x03\x04 not a replay at all", &Limits::default()),
            Err(ReplayError::BadMagic)
        );
        let mut bytes = write_all(&records());
        bytes[4] = 0xFF; // the format version
        bytes[5] = 0x00;
        assert_eq!(
            Replay::parse(&bytes, &Limits::default()),
            Err(ReplayError::UnsupportedVersion { found: 255 })
        );
    }

    #[test]
    fn a_huge_claimed_length_is_cut_short_without_reading_or_allocating_it() {
        let mut bytes = write_all(&[]);
        // Replace the end record by one that claims 4 GiB.
        let end_len = 1 + 4 + 8 + 4;
        bytes.truncate(bytes.len() - end_len);
        bytes.push(1); // an input record
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&[0; 10]);
        let replay = Replay::parse(&bytes, &Limits::default()).unwrap();
        assert!(replay.records().is_empty());
        assert!(matches!(
            replay.end(),
            ReplayEnd::Truncated {
                reason: TruncationReason::TooLarge,
                ..
            }
        ));
    }

    #[test]
    fn records_out_of_tick_order_end_the_replay_there() {
        let mut w = ReplayWriter::new(header()).unwrap();
        w.push(Record::Input {
            tick: 3,
            player: 0,
            data: vec![],
        })
        .unwrap();
        w.push_unchecked_for_test(Record::Input {
            tick: 2,
            player: 0,
            data: vec![],
        });
        w.push(Record::Input {
            tick: 9,
            player: 0,
            data: vec![],
        })
        .unwrap();
        let bytes = w.finish();
        let replay = Replay::parse(&bytes, &Limits::default()).unwrap();
        assert_eq!(replay.records().len(), 1);
        assert!(matches!(
            replay.end(),
            ReplayEnd::Truncated {
                reason: TruncationReason::OutOfOrder,
                ..
            }
        ));
    }

    #[test]
    fn the_number_of_records_is_bounded() {
        let many: Vec<Record> = (0..50)
            .map(|i| Record::StateHash { tick: i, hash: i })
            .collect();
        let bytes = write_all(&many);
        let limits = Limits {
            max_records: 10,
            ..Limits::default()
        };
        let replay = Replay::parse(&bytes, &limits).unwrap();
        assert_eq!(replay.records().len(), 10);
        assert!(matches!(
            replay.end(),
            ReplayEnd::Truncated {
                reason: TruncationReason::TooManyRecords,
                ..
            }
        ));
    }

    #[test]
    fn an_end_record_with_the_wrong_count_is_not_complete() {
        let all = records();
        let mut bytes = write_all(&all);
        let n = bytes.len();
        // The count is the 8 bytes before the final CRC; change it and fix the CRC so only the count is wrong.
        let count_at = n - 4 - 8;
        bytes[count_at] = bytes[count_at].wrapping_add(1);
        let record_start = n - (1 + 4 + 8 + 4);
        let crc = crc32(&bytes[record_start..n - 4]);
        bytes[n - 4..].copy_from_slice(&crc.to_le_bytes());
        let replay = Replay::parse(&bytes, &Limits::default()).unwrap();
        assert!(matches!(
            replay.end(),
            ReplayEnd::Truncated {
                reason: TruncationReason::CountMismatch,
                ..
            }
        ));
    }

    #[test]
    fn a_long_recording_can_be_drained_to_disk_in_pieces() {
        let all = records();
        let mut whole = ReplayWriter::new(header()).unwrap();
        let mut streamed = ReplayWriter::new(header()).unwrap();
        let mut pieces = Vec::new();
        for r in &all {
            whole.push(r.clone()).unwrap();
            streamed.push(r.clone()).unwrap();
            pieces.extend(streamed.drain());
        }
        pieces.extend(streamed.finish());
        assert_eq!(pieces, whole.finish());
        let mut empty = ReplayWriter::new(header()).unwrap();
        assert!(!empty.drain().is_empty(), "the header comes out first");
        assert!(empty.drain().is_empty(), "and only once");
    }

    #[test]
    fn playback_starts_from_the_latest_keyframe_at_or_before_the_tick() {
        let replay = Replay::parse(&write_all(&records()), &Limits::default()).unwrap();
        assert_eq!(
            replay.snapshot_at_or_before(0).map(|(t, d)| (t, d.len())),
            Some((0, 20))
        );
        assert_eq!(replay.snapshot_at_or_before(1).map(|(t, _)| t), Some(0));
        assert_eq!(
            replay.snapshot_at_or_before(2).map(|(t, d)| (t, d.len())),
            Some((2, 5))
        );
        assert_eq!(replay.snapshot_at_or_before(99).map(|(t, _)| t), Some(2));
        let no_keyframe = Replay::parse(
            &write_all(&[Record::Input {
                tick: 1,
                player: 0,
                data: vec![],
            }]),
            &Limits::default(),
        )
        .unwrap();
        assert!(no_keyframe.snapshot_at_or_before(5).is_none());
    }

    #[test]
    fn stepping_yields_the_records_of_each_tick_in_order_without_the_snapshots() {
        let replay = Replay::parse(&write_all(&records()), &Limits::default()).unwrap();
        let mut playback = Playback::new(&replay);
        let mut seen = Vec::new();
        while let Some((tick, group)) = playback.step() {
            assert!(group.iter().all(|r| r.tick() == tick));
            assert!(group.iter().all(|r| !matches!(r, Record::Snapshot { .. })));
            seen.push((tick, group.len()));
        }
        assert_eq!(seen, vec![(0, 3), (1, 3), (2, 2), (5, 1)]);
        assert!(playback.step().is_none());
    }

    #[test]
    fn seeking_restores_a_keyframe_and_continues_from_its_tick() {
        let replay = Replay::parse(&write_all(&records()), &Limits::default()).unwrap();
        let mut playback = Playback::new(&replay);
        let (tick, data) = playback.seek(4).expect("a keyframe exists");
        assert_eq!((tick, data), (2, &[0xBB_u8; 5][..]));
        let (next, group) = playback.step().unwrap();
        assert_eq!(
            next, 2,
            "the keyframe is the state before its own tick's inputs"
        );
        assert_eq!(group.len(), 2);
        // Seeking back to the start works the same way.
        let (tick, _) = playback.seek(0).unwrap();
        assert_eq!(tick, 0);
        assert_eq!(playback.step().unwrap().0, 0);
        // A replay with no keyframe cannot seek.
        let bare = Replay::parse(
            &write_all(&[Record::Input {
                tick: 1,
                player: 0,
                data: vec![],
            }]),
            &Limits::default(),
        )
        .unwrap();
        assert!(Playback::new(&bare).seek(1).is_none());
    }

    #[test]
    fn a_playback_that_drifts_is_caught_at_the_first_state_hash_that_differs() {
        let replay = Replay::parse(&write_all(&records()), &Limits::default()).unwrap();
        let playback = Playback::new(&replay);
        assert_eq!(playback.check_hash(0, 11), HashCheck::Match);
        assert_eq!(
            playback.check_hash(1, 999),
            HashCheck::Mismatch { expected: 12 }
        );
        assert_eq!(
            playback.check_hash(3, 1),
            HashCheck::NotRecorded,
            "no hash for tick 3"
        );
    }
}
