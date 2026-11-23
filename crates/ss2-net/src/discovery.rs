//! Finding a game to join (roadmap T08-019): the **LAN scan**, **direct addresses** and **invite codes**, as pure logic over
//! datagrams, like the rest of this crate (no sockets, no clock, no randomness: the caller passes time, a
//! nonce and the source address).
//!
//! * **LAN scan.** A client broadcasts a **query** and collects **info** replies. A server's [`Announcer`] answers
//!   a query with its [`ServerInfo`] (name, map, mode, players, protocol version and schema hash, mods hash, whether a password is
//!   needed, the game port). A [`Scanner`] accepts only replies that echo its **nonce**, during the scan, and keeps a bounded
//!   [`ServerEntry`] list with the round trip time and whether the protocol versions are compatible
//!   (`ss2_net::protocol::check_handshake`).
//! * **Not a reflector.** A UDP service that answers a spoofed address with more bytes than it received is a weapon
//!   against a third party (`docs/THREAT_MODEL_NET.md`). Queries must be **padded to [`MIN_QUERY_LEN`] bytes**,
//!   which is more than the largest possible reply, so **a reply is never larger than the query that caused it**, and an
//!   [`Announcer`] limits answers per source address and overall (bounded tables).
//! * **Direct addresses and invite codes.** [`parse_direct`] reads what a player types (`192.168.1.5`, `host:27015`, `[::1]:27015`).
//!   An [`encode_invite`] code is a short, typeable, checksummed form of an IP address and port
//!   (`XXXX-XXXX-XXXX-XXX` for IPv4), tolerant of case, dashes and the letters people confuse; [`decode_invite`] reads it.
//!
//! What it does **not** do: the relay or rendezvous server for internet play, NAT traversal and hole punching (the
//! transport's job, RFC 0006), and **authenticating who answered**: a LAN scan can be answered by anyone on the network, so the
//! list is a convenience and the **connection pins the host's key** (RFC 0006), it does not trust the list.
//! An invite code carries no key either.

use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::protocol::{Compat, Handshake, ProtocolVersion, check_handshake};

const MAGIC: [u8; 4] = *b"SS2D";
const WIRE_VERSION: u8 = 1;
const KIND_QUERY: u8 = 1;
const KIND_INFO: u8 = 2;

/// A query is padded to at least this many bytes, and a reply is never longer than the query it answers.
pub const MIN_QUERY_LEN: usize = 256;
/// Datagrams longer than this are not discovery messages.
pub const MAX_DATAGRAM: usize = 1200;
/// The longest server name, in bytes.
pub const MAX_NAME: usize = 32;
/// The longest map name, in bytes.
pub const MAX_MAP: usize = 48;
/// The longest game mode name, in bytes.
pub const MAX_MODE: usize = 24;
/// The most player slots a server may announce.
pub const MAX_PLAYER_SLOTS: u8 = 64;
/// The most servers a [`ServerList`] holds.
pub const MAX_SERVERS: usize = 256;
/// The most source addresses an [`Announcer`] remembers for rate limiting.
pub const MAX_TRACKED_SOURCES: usize = 1024;

/// What a server says about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerInfo {
    /// The name in the server list.
    pub name: String,
    /// The level.
    pub map: String,
    /// The game mode (`coop`, ...).
    pub mode: String,
    /// Players now.
    pub players: u8,
    /// Player slots (at most [`MAX_PLAYER_SLOTS`]).
    pub max_players: u8,
    /// The port to connect to (the discovery port may be another).
    pub port: u16,
    /// The protocol version of the server.
    pub protocol: ProtocolVersion,
    /// The schema hash of the server's message layouts.
    pub schema_hash: u64,
    /// The hash of the server's mods (0 for none).
    pub mods_hash: u64,
    /// Whether joining needs a password.
    pub password: bool,
}

fn clean(text: &str, max: usize) -> bool {
    text.len() <= max && !text.chars().any(char::is_control)
}

impl ServerInfo {
    /// Whether the fields are within their limits: a non-empty name, strings without control characters,
    /// `players <= max_players <= 64`, a non-zero port.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.name.is_empty() || !clean(&self.name, MAX_NAME) {
            return Err("the name is empty, too long or has control characters");
        }
        if !clean(&self.map, MAX_MAP) || !clean(&self.mode, MAX_MODE) {
            return Err("the map or mode is too long or has control characters");
        }
        if self.max_players > MAX_PLAYER_SLOTS || self.players > self.max_players {
            return Err("the player counts are out of range");
        }
        if self.port == 0 {
            return Err("the port is 0");
        }
        Ok(())
    }

    /// Whether every slot is taken.
    pub fn is_full(&self) -> bool {
        self.players >= self.max_players
    }
}

/// A discovery message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// "Who is there?", with a nonce the replies must echo.
    Query {
        /// The scan's nonce.
        nonce: u64,
        /// The protocol version of the asker (informational).
        protocol: ProtocolVersion,
    },
    /// A server's answer.
    Info {
        /// The nonce of the query it answers.
        nonce: u64,
        /// What the server says about itself.
        info: ServerInfo,
    },
}

/// Why a datagram is not a discovery message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Does not start with the discovery magic.
    NotDiscovery,
    /// Longer than [`MAX_DATAGRAM`].
    TooLong,
    /// A wire version this build does not read.
    BadVersion(u8),
    /// A message kind that does not exist.
    UnknownKind(u8),
    /// Ends in the middle of a field.
    Truncated,
    /// A query shorter than [`MIN_QUERY_LEN`]: it would let a reply be longer than the request.
    Unpadded,
    /// A text that is not UTF-8.
    BadText,
    /// The info fails [`ServerInfo::validate`].
    InvalidInfo(&'static str),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotDiscovery => write!(f, "not a discovery message"),
            Self::TooLong => write!(f, "longer than a discovery datagram may be"),
            Self::BadVersion(v) => write!(f, "discovery wire version {v} is not understood"),
            Self::UnknownKind(k) => write!(f, "unknown discovery message kind {k}"),
            Self::Truncated => write!(f, "the discovery message ends early"),
            Self::Unpadded => write!(f, "the query is not padded to {MIN_QUERY_LEN} bytes"),
            Self::BadText => write!(f, "a text is not valid UTF-8"),
            Self::InvalidInfo(why) => write!(f, "invalid server info: {why}"),
        }
    }
}

impl std::error::Error for DecodeError {}

fn put_text(out: &mut Vec<u8>, text: &str) {
    out.push(text.len() as u8);
    out.extend_from_slice(text.as_bytes());
}

/// The bytes of a query for `nonce`, padded to [`MIN_QUERY_LEN`].
pub fn encode_query(nonce: u64, protocol: ProtocolVersion) -> Vec<u8> {
    let mut out = Vec::with_capacity(MIN_QUERY_LEN);
    out.extend_from_slice(&MAGIC);
    out.push(WIRE_VERSION);
    out.push(KIND_QUERY);
    out.extend_from_slice(&nonce.to_le_bytes());
    out.extend_from_slice(&protocol.major.to_le_bytes());
    out.extend_from_slice(&protocol.minor.to_le_bytes());
    out.resize(MIN_QUERY_LEN, 0);
    out
}

/// The bytes of a reply to the query with `nonce`. `Err` if the info is invalid.
pub fn encode_info(nonce: u64, info: &ServerInfo) -> Result<Vec<u8>, &'static str> {
    info.validate()?;
    let mut out = Vec::with_capacity(160);
    out.extend_from_slice(&MAGIC);
    out.push(WIRE_VERSION);
    out.push(KIND_INFO);
    out.extend_from_slice(&nonce.to_le_bytes());
    out.extend_from_slice(&info.protocol.major.to_le_bytes());
    out.extend_from_slice(&info.protocol.minor.to_le_bytes());
    out.extend_from_slice(&info.schema_hash.to_le_bytes());
    out.extend_from_slice(&info.mods_hash.to_le_bytes());
    out.push(info.players);
    out.push(info.max_players);
    out.extend_from_slice(&info.port.to_le_bytes());
    out.push(u8::from(info.password));
    put_text(&mut out, &info.name);
    put_text(&mut out, &info.map);
    put_text(&mut out, &info.mode);
    debug_assert!(out.len() <= MIN_QUERY_LEN);
    Ok(out)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::Truncated)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(DecodeError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().unwrap_or([0; 2]),
        ))
    }

    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().unwrap_or([0; 8]),
        ))
    }

    fn text(&mut self) -> Result<String, DecodeError> {
        let len = usize::from(self.u8()?);
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| DecodeError::BadText)
    }
}

/// Read a discovery datagram.
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    if bytes.len() < 4 || bytes[..4] != MAGIC {
        return Err(DecodeError::NotDiscovery);
    }
    if bytes.len() > MAX_DATAGRAM {
        return Err(DecodeError::TooLong);
    }
    let mut r = Reader { bytes, pos: 4 };
    let version = r.u8()?;
    if version != WIRE_VERSION {
        return Err(DecodeError::BadVersion(version));
    }
    match r.u8()? {
        KIND_QUERY => {
            let nonce = r.u64()?;
            let protocol = ProtocolVersion::new(r.u16()?, r.u16()?);
            if bytes.len() < MIN_QUERY_LEN {
                return Err(DecodeError::Unpadded);
            }
            Ok(Message::Query { nonce, protocol })
        }
        KIND_INFO => {
            let nonce = r.u64()?;
            let protocol = ProtocolVersion::new(r.u16()?, r.u16()?);
            let schema_hash = r.u64()?;
            let mods_hash = r.u64()?;
            let (players, max_players) = (r.u8()?, r.u8()?);
            let port = r.u16()?;
            let password = r.u8()? != 0;
            let (name, map, mode) = (r.text()?, r.text()?, r.text()?);
            let info = ServerInfo {
                name,
                map,
                mode,
                players,
                max_players,
                port,
                protocol,
                schema_hash,
                mods_hash,
                password,
            };
            info.validate().map_err(DecodeError::InvalidInfo)?;
            Ok(Message::Info { nonce, info })
        }
        other => Err(DecodeError::UnknownKind(other)),
    }
}

// ---- answering queries -----------------------------------------------------------------------------

const MICRO: u64 = 1_000_000;

/// A token bucket in whole microtokens, so refills are exact integers.
#[derive(Clone, Copy, Debug)]
struct Bucket {
    micro_tokens: u64,
    last_us: u64,
}

impl Bucket {
    fn refill(&mut self, now_us: u64, per_second: u64, burst: u64) {
        let elapsed = now_us.saturating_sub(self.last_us);
        self.micro_tokens = self
            .micro_tokens
            .saturating_add(elapsed.saturating_mul(per_second))
            .min(burst.saturating_mul(MICRO));
        self.last_us = self.last_us.max(now_us);
    }
}

/// How an [`Announcer`] limits its answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimit {
    /// Answers per second to one source address.
    pub per_source_per_second: u64,
    /// How many answers one source may take at once.
    pub per_source_burst: u64,
    /// Answers per second to everyone.
    pub total_per_second: u64,
    /// How many answers everyone together may take at once.
    pub total_burst: u64,
}

impl Default for RateLimit {
    /// Two answers per second per source (a burst of 4) and 100 per second overall (a burst of 200).
    fn default() -> Self {
        Self {
            per_source_per_second: 2,
            per_source_burst: 4,
            total_per_second: 100,
            total_burst: 200,
        }
    }
}

/// Why a datagram got no answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ignored {
    /// Not a valid discovery message (or not a query).
    NotAQuery(DecodeError),
    /// An info message arrived at a server.
    WrongDirection,
    /// This source asked too often.
    SourceLimited,
    /// Everyone together asked too often.
    TotalLimited,
    /// The server's own info is invalid, so there is nothing to say.
    BadInfo,
}

/// What an [`Announcer`] did with a datagram.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handled {
    /// Send these bytes back to the source (never longer than the query).
    Reply(Vec<u8>),
    /// Send nothing.
    Ignored(Ignored),
}

/// A server's side of the scan: answers queries with its info, within limits.
///
/// ```
/// use std::net::SocketAddr;
/// use ss2_net::discovery::{Announcer, Handled, RateLimit, ServerInfo, encode_query};
/// use ss2_net::protocol::ProtocolVersion;
///
/// let info = ServerInfo { name: "Sam's game".into(), map: "Sirius".into(), mode: "coop".into(), players: 1, max_players: 4,
///     port: 27015, protocol: ProtocolVersion::new(0, 3), schema_hash: 7, mods_hash: 0, password: false };
/// let mut announcer = Announcer::new(info, RateLimit::default());
/// let query = encode_query(42, ProtocolVersion::new(0, 3));
/// let from: SocketAddr = "192.168.1.20:5000".parse().unwrap();
/// match announcer.handle(0, from, &query) {
///     Handled::Reply(bytes) => assert!(bytes.len() <= query.len()),
///     other => panic!("{other:?}"),
/// }
/// ```
pub struct Announcer {
    info: ServerInfo,
    limit: RateLimit,
    sources: BTreeMap<SocketAddr, Bucket>,
    total: Bucket,
    answered: u64,
    ignored: u64,
}

impl Announcer {
    /// An announcer for `info`.
    pub fn new(info: ServerInfo, limit: RateLimit) -> Self {
        Self {
            total: Bucket {
                micro_tokens: limit.total_burst.saturating_mul(MICRO),
                last_us: 0,
            },
            info,
            limit,
            sources: BTreeMap::new(),
            answered: 0,
            ignored: 0,
        }
    }

    /// Change what is announced (players come and go, the map changes).
    pub fn set_info(&mut self, info: ServerInfo) {
        self.info = info;
    }

    /// The current info.
    pub fn info(&self) -> &ServerInfo {
        &self.info
    }

    /// Queries answered so far.
    pub fn answered(&self) -> u64 {
        self.answered
    }

    /// Datagrams that got no answer so far.
    pub fn ignored(&self) -> u64 {
        self.ignored
    }

    /// How many source addresses are being tracked.
    pub fn tracked_sources(&self) -> usize {
        self.sources.len()
    }

    /// Handle a datagram that arrived at `now_us` from `from`.
    pub fn handle(&mut self, now_us: u64, from: SocketAddr, datagram: &[u8]) -> Handled {
        let outcome = self.handle_inner(now_us, from, datagram);
        match &outcome {
            Handled::Reply(_) => self.answered += 1,
            Handled::Ignored(_) => self.ignored += 1,
        }
        outcome
    }

    fn handle_inner(&mut self, now_us: u64, from: SocketAddr, datagram: &[u8]) -> Handled {
        let nonce = match decode(datagram) {
            Ok(Message::Query { nonce, .. }) => nonce,
            Ok(Message::Info { .. }) => return Handled::Ignored(Ignored::WrongDirection),
            Err(e) => return Handled::Ignored(Ignored::NotAQuery(e)),
        };
        let Ok(reply) = encode_info(nonce, &self.info) else {
            return Handled::Ignored(Ignored::BadInfo);
        };
        // Both buckets must have a token; a token is taken from both only when the answer is sent.
        self.total
            .refill(now_us, self.limit.total_per_second, self.limit.total_burst);
        if self.sources.len() >= MAX_TRACKED_SOURCES && !self.sources.contains_key(&from) {
            // Make room: forget the source that was seen longest ago.
            if let Some(oldest) = self
                .sources
                .iter()
                .min_by_key(|(addr, b)| (b.last_us, **addr))
                .map(|(a, _)| *a)
            {
                self.sources.remove(&oldest);
            }
        }
        let bucket = self.sources.entry(from).or_insert(Bucket {
            micro_tokens: self.limit.per_source_burst.saturating_mul(MICRO),
            last_us: now_us,
        });
        bucket.refill(
            now_us,
            self.limit.per_source_per_second,
            self.limit.per_source_burst,
        );
        if bucket.micro_tokens < MICRO {
            return Handled::Ignored(Ignored::SourceLimited);
        }
        if self.total.micro_tokens < MICRO {
            return Handled::Ignored(Ignored::TotalLimited);
        }
        bucket.micro_tokens -= MICRO;
        self.total.micro_tokens -= MICRO;
        debug_assert!(reply.len() <= datagram.len());
        Handled::Reply(reply)
    }
}

// ---- scanning -------------------------------------------------------------------------------------

/// A server found by a scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerEntry {
    /// Where to connect: the address the reply came from, with the game port from the reply.
    pub addr: SocketAddr,
    /// What the server said.
    pub info: ServerInfo,
    /// Microseconds from the query to the reply.
    pub rtt_us: u64,
    /// When the reply arrived.
    pub last_seen_us: u64,
    /// Whether this build can talk to it.
    pub compat: Compat,
}

/// Why a datagram did not become a list entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    /// No scan has been started.
    NotScanning,
    /// It arrived after the scan's time ran out.
    TooLate,
    /// Not a valid discovery message, or not an info message.
    Malformed,
    /// The nonce is not this scan's (an old scan, or an unsolicited packet).
    WrongNonce,
    /// The list is full.
    ListFull,
}

/// What [`Scanner::on_datagram`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accepted {
    /// A new server.
    Added,
    /// A server already in the list answered again.
    Updated,
}

/// The servers found, bounded, by address.
#[derive(Clone, Debug, Default)]
pub struct ServerList {
    entries: BTreeMap<SocketAddr, ServerEntry>,
}

impl ServerList {
    /// How many servers.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry for an address.
    pub fn get(&self, addr: &SocketAddr) -> Option<&ServerEntry> {
        self.entries.get(addr)
    }

    /// Every entry, by address.
    pub fn entries(&self) -> impl Iterator<Item = &ServerEntry> {
        self.entries.values()
    }

    /// The entries in the order to show them: servers this build can join first, then those with a free slot,
    /// then the fastest, then by address.
    pub fn for_display(&self) -> Vec<&ServerEntry> {
        let mut v: Vec<&ServerEntry> = self.entries.values().collect();
        v.sort_by_key(|e| {
            (
                !matches!(e.compat, Compat::Compatible(_)),
                e.info.is_full(),
                e.rtt_us,
                e.addr,
            )
        });
        v
    }

    /// Drop the entries not heard from for more than `max_age_us`; returns how many.
    pub fn expire(&mut self, now_us: u64, max_age_us: u64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|_, e| now_us.saturating_sub(e.last_seen_us) <= max_age_us);
        before - self.entries.len()
    }
}

/// A client's side of the scan.
#[derive(Clone, Debug)]
pub struct Scanner {
    local: Handshake,
    scan: Option<(u64, u64, u64)>, // (nonce, started_us, timeout_us)
    list: ServerList,
    rejected: u64,
}

impl Scanner {
    /// A scanner for a build that speaks `local`.
    pub fn new(local: Handshake) -> Self {
        Self {
            local,
            scan: None,
            list: ServerList::default(),
            rejected: 0,
        }
    }

    /// Start a scan at `now_us` with `nonce`, accepting replies for `timeout_us`. Returns the query to broadcast.
    /// The servers of an earlier scan stay in the list until they expire or answer again.
    pub fn start(&mut self, now_us: u64, nonce: u64, timeout_us: u64) -> Vec<u8> {
        self.scan = Some((nonce, now_us, timeout_us));
        encode_query(nonce, self.local.version)
    }

    /// The servers found.
    pub fn list(&self) -> &ServerList {
        &self.list
    }

    /// The servers found, to change (expire entries).
    pub fn list_mut(&mut self) -> &mut ServerList {
        &mut self.list
    }

    /// Datagrams that were not accepted so far.
    pub fn rejected(&self) -> u64 {
        self.rejected
    }

    /// Handle a datagram that arrived at `now_us` from `from`.
    pub fn on_datagram(
        &mut self,
        now_us: u64,
        from: SocketAddr,
        datagram: &[u8],
    ) -> Result<Accepted, Rejected> {
        let result = self.accept(now_us, from, datagram);
        if result.is_err() {
            self.rejected += 1;
        }
        result
    }

    fn accept(
        &mut self,
        now_us: u64,
        from: SocketAddr,
        datagram: &[u8],
    ) -> Result<Accepted, Rejected> {
        let (nonce, started_us, timeout_us) = self.scan.ok_or(Rejected::NotScanning)?;
        let Ok(Message::Info {
            nonce: echoed,
            info,
        }) = decode(datagram)
        else {
            return Err(Rejected::Malformed);
        };
        if echoed != nonce {
            return Err(Rejected::WrongNonce);
        }
        if now_us.saturating_sub(started_us) > timeout_us {
            return Err(Rejected::TooLate);
        }
        let addr = SocketAddr::new(from.ip(), info.port);
        let existing = self.list.entries.contains_key(&addr);
        if !existing && self.list.entries.len() >= MAX_SERVERS {
            return Err(Rejected::ListFull);
        }
        let compat = check_handshake(
            &self.local,
            &Handshake {
                version: info.protocol,
                schema_hash: info.schema_hash,
            },
        );
        let entry = ServerEntry {
            addr,
            info,
            rtt_us: now_us.saturating_sub(started_us),
            last_seen_us: now_us,
            compat,
        };
        self.list.entries.insert(addr, entry);
        Ok(if existing {
            Accepted::Updated
        } else {
            Accepted::Added
        })
    }
}

// ---- direct addresses -----------------------------------------------------------------------------

/// The port used when a player types an address without one.
pub const DEFAULT_PORT: u16 = 27015;

/// Where a player asked to connect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectTarget {
    /// An IP address and port.
    Addr(SocketAddr),
    /// A host name (to be resolved by the caller) and port.
    Host {
        /// The name, lower case.
        name: String,
        /// The port.
        port: u16,
    },
}

fn valid_host(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u16>().ok().filter(|p| *p != 0)
}

/// Read what a player typed: `1.2.3.4`, `1.2.3.4:5`, `[::1]`, `[::1]:5`, `::1`, `host`, `host:5`. A missing port is
/// [`DEFAULT_PORT`]. Surrounding spaces are ignored; anything else odd is refused.
pub fn parse_direct(input: &str) -> Option<DirectTarget> {
    let text = input.trim();
    if text.is_empty()
        || text.len() > 300
        || text.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    if let Some(rest) = text.strip_prefix('[') {
        let (ip, tail) = rest.split_once(']')?;
        let ip: Ipv6Addr = ip.parse().ok()?;
        let port = match tail {
            "" => DEFAULT_PORT,
            t => parse_port(t.strip_prefix(':')?)?,
        };
        return Some(DirectTarget::Addr(SocketAddr::new(IpAddr::V6(ip), port)));
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Some(DirectTarget::Addr(SocketAddr::new(ip, DEFAULT_PORT)));
    }
    let (host, port) = match text.rsplit_once(':') {
        Some((h, p)) => (h, parse_port(p)?),
        None => (text, DEFAULT_PORT),
    };
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Some(DirectTarget::Addr(SocketAddr::new(IpAddr::V4(ip), port)));
    }
    // A name that looks like an IPv4 address but is not one (999.1.1.1, 1.2.3) is not a host name.
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let name = host.to_ascii_lowercase();
    valid_host(&name).then_some(DirectTarget::Host { name, port })
}

// ---- invite codes ---------------------------------------------------------------------------------

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const INVITE_VERSION: u8 = 1;

fn checksum16(bytes: &[u8]) -> u16 {
    let mut h: u32 = 0x811C_9DC5;
    for b in bytes {
        h = (h ^ u32::from(*b)).wrapping_mul(0x0100_0193);
    }
    ((h >> 16) ^ h) as u16
}

/// A short code for an IP address and port: the player types it instead of the address. Not for port 0 or an
/// unspecified address.
pub fn encode_invite(addr: SocketAddr) -> Option<String> {
    if addr.port() == 0 || addr.ip().is_unspecified() {
        return None;
    }
    let mut bytes = Vec::with_capacity(21);
    match addr.ip() {
        IpAddr::V4(ip) => {
            bytes.push(INVITE_VERSION << 4 | 4);
            bytes.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            bytes.push(INVITE_VERSION << 4 | 6);
            bytes.extend_from_slice(&ip.octets());
        }
    }
    bytes.extend_from_slice(&addr.port().to_be_bytes());
    let sum = checksum16(&bytes);
    bytes.extend_from_slice(&sum.to_be_bytes());
    let (mut bits, mut have, mut chars) = (0u32, 0u32, String::new());
    for b in bytes {
        bits = bits << 8 | u32::from(b);
        have += 8;
        while have >= 5 {
            chars.push(ALPHABET[((bits >> (have - 5)) & 31) as usize] as char);
            have -= 5;
        }
        bits &= (1 << have) - 1;
    }
    if have > 0 {
        chars.push(ALPHABET[((bits << (5 - have)) & 31) as usize] as char);
    }
    let grouped: Vec<&str> = chars
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();
    Some(grouped.join("-"))
}

/// Read an invite code: case, dashes and spaces do not matter, and `O`, `I` and `L` are read as `0`, `1` and `1`.
/// `None` for anything that is not a code made by [`encode_invite`] (a wrong length, a character outside the
/// alphabet, a bad checksum, an unknown version, port 0).
pub fn decode_invite(code: &str) -> Option<SocketAddr> {
    let mut values = Vec::with_capacity(36);
    for c in code.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        };
        let index = ALPHABET.iter().position(|a| *a as char == c)?;
        values.push(index as u8);
        if values.len() > 40 {
            return None;
        }
    }
    let (mut bits, mut have, mut bytes) = (0u32, 0u32, Vec::new());
    for v in &values {
        bits = bits << 5 | u32::from(*v);
        have += 5;
        if have >= 8 {
            bytes.push((bits >> (have - 8)) as u8);
            have -= 8;
            bits &= (1 << have) - 1;
        }
    }
    if bits != 0 {
        return None; // the padding bits of a real code are zero
    }
    let header = *bytes.first()?;
    if header >> 4 != INVITE_VERSION {
        return None;
    }
    let ip_len = match header & 0xF {
        4 => 4,
        6 => 16,
        _ => return None,
    };
    if bytes.len() != 1 + ip_len + 2 + 2 {
        return None;
    }
    let (body, sum) = bytes.split_at(bytes.len() - 2);
    if checksum16(body) != u16::from_be_bytes([sum[0], sum[1]]) {
        return None;
    }
    let ip = if ip_len == 4 {
        IpAddr::V4(Ipv4Addr::new(body[1], body[2], body[3], body[4]))
    } else {
        let mut octets = [0u8; 16];
        octets.copy_from_slice(&body[1..17]);
        IpAddr::V6(Ipv6Addr::from(octets))
    };
    let port = u16::from_be_bytes([body[1 + ip_len], body[2 + ip_len]]);
    (port != 0 && !ip.is_unspecified()).then_some(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    fn info() -> ServerInfo {
        ServerInfo {
            name: "Sam's game".into(),
            map: "Sirius".into(),
            mode: "coop".into(),
            players: 2,
            max_players: 4,
            port: 27015,
            protocol: ProtocolVersion::new(0, 3),
            schema_hash: 0xABCD,
            mods_hash: 0,
            password: false,
        }
    }

    fn worst_info() -> ServerInfo {
        ServerInfo {
            name: "n".repeat(MAX_NAME),
            map: "m".repeat(MAX_MAP),
            mode: "o".repeat(MAX_MODE),
            players: 64,
            max_players: 64,
            port: 65535,
            protocol: ProtocolVersion::new(u16::MAX, u16::MAX),
            schema_hash: u64::MAX,
            mods_hash: u64::MAX,
            password: true,
        }
    }

    fn local() -> Handshake {
        Handshake {
            version: ProtocolVersion::new(0, 3),
            schema_hash: 0xABCD,
        }
    }

    // ---- messages ---------------------------------------------------------------------------------

    #[test]
    fn a_query_and_an_info_round_trip() {
        let q = encode_query(0xDEAD_BEEF_0123, ProtocolVersion::new(1, 7));
        assert_eq!(q.len(), MIN_QUERY_LEN);
        assert_eq!(
            decode(&q),
            Ok(Message::Query {
                nonce: 0xDEAD_BEEF_0123,
                protocol: ProtocolVersion::new(1, 7)
            })
        );
        let i = encode_info(99, &info()).unwrap();
        assert_eq!(
            decode(&i),
            Ok(Message::Info {
                nonce: 99,
                info: info()
            })
        );
        let w = encode_info(1, &worst_info()).unwrap();
        assert_eq!(
            decode(&w),
            Ok(Message::Info {
                nonce: 1,
                info: worst_info()
            })
        );
    }

    #[test]
    fn the_largest_possible_reply_is_smaller_than_the_smallest_possible_query() {
        let biggest = encode_info(u64::MAX, &worst_info()).unwrap();
        assert!(biggest.len() <= MIN_QUERY_LEN, "{} bytes", biggest.len());
        assert!(
            biggest.len() < encode_query(0, ProtocolVersion::new(0, 0)).len(),
            "so a reply never exceeds its query"
        );
    }

    #[test]
    fn invalid_info_is_not_encoded_and_is_refused_when_decoded() {
        let bad = |change: &dyn Fn(&mut ServerInfo)| {
            let mut i = info();
            change(&mut i);
            i
        };
        for i in [
            bad(&|i| i.name = String::new()),
            bad(&|i| i.name = "x".repeat(MAX_NAME + 1)),
            bad(&|i| i.map = "x".repeat(MAX_MAP + 1)),
            bad(&|i| i.mode = "x".repeat(MAX_MODE + 1)),
            bad(&|i| i.name = "a\nb".into()),
            bad(&|i| i.map = "a\u{7}".into()),
            bad(&|i| i.max_players = 65),
            bad(&|i| i.players = 5),
            bad(&|i| i.port = 0),
        ] {
            assert!(encode_info(1, &i).is_err(), "{i:?}");
        }
        // The same checks apply to what arrives: forge a datagram around a valid one.
        let mut forged = encode_info(1, &info()).unwrap();
        let players_at = 4 + 1 + 1 + 8 + 2 + 2 + 8 + 8;
        forged[players_at] = 9; // more players than slots
        assert_eq!(
            decode(&forged),
            Err(DecodeError::InvalidInfo(
                "the player counts are out of range"
            ))
        );
    }

    #[test]
    fn what_is_not_a_discovery_message_is_refused_with_the_reason() {
        let q = encode_query(1, ProtocolVersion::new(0, 3));
        assert_eq!(decode(b""), Err(DecodeError::NotDiscovery));
        assert_eq!(decode(b"SS2"), Err(DecodeError::NotDiscovery));
        assert_eq!(
            decode(b"HTTP/1.1 200 OK\r\n"),
            Err(DecodeError::NotDiscovery)
        );
        let mut v = q.clone();
        v[4] = 9;
        assert_eq!(decode(&v), Err(DecodeError::BadVersion(9)));
        let mut k = q.clone();
        k[5] = 7;
        assert_eq!(decode(&k), Err(DecodeError::UnknownKind(7)));
        assert_eq!(decode(&q[..10]), Err(DecodeError::Truncated));
        assert_eq!(
            decode(&q[..MIN_QUERY_LEN - 1]),
            Err(DecodeError::Unpadded),
            "one byte short of the padding"
        );
        let mut long = q.clone();
        long.resize(MAX_DATAGRAM + 1, 0);
        assert_eq!(decode(&long), Err(DecodeError::TooLong));
        long.truncate(MAX_DATAGRAM);
        assert!(decode(&long).is_ok(), "exactly the largest datagram");
        let info_bytes = encode_info(1, &info()).unwrap();
        for cut in 0..info_bytes.len() {
            assert!(decode(&info_bytes[..cut]).is_err(), "cut to {cut}");
        }
        let mut bad_text = encode_info(1, &info()).unwrap();
        let first_text = 4 + 1 + 1 + 8 + 2 + 2 + 8 + 8 + 1 + 1 + 2 + 1 + 1;
        bad_text[first_text] = 0xFF;
        assert_eq!(decode(&bad_text), Err(DecodeError::BadText));
    }

    // ---- answering --------------------------------------------------------------------------------

    fn announcer() -> Announcer {
        Announcer::new(info(), RateLimit::default())
    }

    #[test]
    fn a_query_is_answered_with_the_info_and_the_nonce_and_never_with_more_bytes() {
        let mut a = announcer();
        let query = encode_query(777, ProtocolVersion::new(0, 3));
        match a.handle(0, addr("192.168.1.20:5000"), &query) {
            Handled::Reply(bytes) => {
                assert!(bytes.len() <= query.len());
                assert_eq!(
                    decode(&bytes),
                    Ok(Message::Info {
                        nonce: 777,
                        info: info()
                    })
                );
            }
            other => panic!("{other:?}"),
        }
        assert_eq!((a.answered(), a.ignored()), (1, 0));
        // Even the biggest announcement stays within the query.
        a.set_info(worst_info());
        match a.handle(0, addr("192.168.1.21:5000"), &query) {
            Handled::Reply(bytes) => assert!(bytes.len() <= query.len()),
            other => panic!("{other:?}"),
        }
        assert_eq!(a.info(), &worst_info());
    }

    #[test]
    fn short_queries_other_traffic_and_info_messages_get_no_answer() {
        let mut a = announcer();
        let from = addr("10.0.0.1:1");
        let query = encode_query(1, ProtocolVersion::new(0, 3));
        assert_eq!(
            a.handle(0, from, &query[..200]),
            Handled::Ignored(Ignored::NotAQuery(DecodeError::Unpadded))
        );
        assert_eq!(
            a.handle(0, from, b"GET / HTTP/1.1\r\n\r\n"),
            Handled::Ignored(Ignored::NotAQuery(DecodeError::NotDiscovery))
        );
        let info_message = encode_info(1, &info()).unwrap();
        assert_eq!(
            a.handle(0, from, &info_message),
            Handled::Ignored(Ignored::WrongDirection)
        );
        let mut bad = Announcer::new(
            ServerInfo {
                name: String::new(),
                ..info()
            },
            RateLimit::default(),
        );
        assert_eq!(
            bad.handle(0, from, &query),
            Handled::Ignored(Ignored::BadInfo)
        );
        assert_eq!(a.answered(), 0);
        assert_eq!(a.ignored(), 3);
    }

    #[test]
    fn one_source_cannot_make_the_server_answer_without_limit() {
        let mut a = announcer(); // burst 4, 2 per second
        let (from, q) = (
            addr("10.0.0.1:1"),
            encode_query(1, ProtocolVersion::new(0, 3)),
        );
        let answers =
            |a: &mut Announcer, t: u64| matches!(a.handle(t, from, &q), Handled::Reply(_));
        assert!((0..4).all(|_| answers(&mut a, 0)), "the burst of four");
        assert_eq!(
            a.handle(0, from, &q),
            Handled::Ignored(Ignored::SourceLimited)
        );
        assert!(!answers(&mut a, 400_000), "0.4 s is 0.8 of a token");
        assert!(answers(&mut a, 500_000), "half a second brings one token");
        assert!(!answers(&mut a, 500_000));
        // Another source is not affected.
        assert!(matches!(
            a.handle(500_000, addr("10.0.0.2:1"), &q),
            Handled::Reply(_)
        ));
        // After a long quiet time the burst is back, and not more than the burst.
        let later = 1_000_000_000;
        let got = (0..10).filter(|_| answers(&mut a, later)).count();
        assert_eq!(got, 4);
    }

    #[test]
    fn everyone_together_is_limited_too() {
        let limit = RateLimit {
            per_source_per_second: 100,
            per_source_burst: 100,
            total_per_second: 1,
            total_burst: 3,
        };
        let mut a = Announcer::new(info(), limit);
        let q = encode_query(1, ProtocolVersion::new(0, 3));
        let mut replies = 0;
        for i in 0..20u8 {
            if matches!(
                a.handle(0, addr(&format!("10.0.0.{i}:1")), &q),
                Handled::Reply(_)
            ) {
                replies += 1;
            }
        }
        assert_eq!(replies, 3, "the total burst");
        assert_eq!(
            a.handle(0, addr("10.0.1.1:1"), &q),
            Handled::Ignored(Ignored::TotalLimited)
        );
        // A source that was refused for the total did not lose its own token.
        assert!(matches!(
            a.handle(2_000_000, addr("10.0.1.1:1"), &q),
            Handled::Reply(_)
        ));
    }

    #[test]
    fn the_table_of_sources_is_bounded_and_forgets_the_oldest() {
        let mut a = announcer();
        let q = encode_query(1, ProtocolVersion::new(0, 3));
        for i in 0..(MAX_TRACKED_SOURCES as u32 + 500) {
            let ip = Ipv4Addr::from(0x0A00_0000 + i);
            a.handle(u64::from(i), SocketAddr::new(IpAddr::V4(ip), 9), &q);
        }
        assert_eq!(a.tracked_sources(), MAX_TRACKED_SOURCES);
    }

    // ---- scanning ---------------------------------------------------------------------------------

    fn reply(nonce: u64, info: &ServerInfo) -> Vec<u8> {
        encode_info(nonce, info).unwrap()
    }

    #[test]
    fn a_scan_collects_the_replies_that_echo_its_nonce() {
        let mut s = Scanner::new(local());
        let q = s.start(1_000, 55, 2_000_000);
        assert_eq!(
            decode(&q),
            Ok(Message::Query {
                nonce: 55,
                protocol: ProtocolVersion::new(0, 3)
            })
        );
        let from = addr("192.168.1.20:5000");
        assert_eq!(
            s.on_datagram(21_000, from, &reply(55, &info())),
            Ok(Accepted::Added)
        );
        let entry = s
            .list()
            .get(&addr("192.168.1.20:27015"))
            .expect("keyed by the source address and the game port");
        assert_eq!((entry.rtt_us, entry.last_seen_us), (20_000, 21_000));
        assert_eq!(entry.compat, Compat::Compatible(ProtocolVersion::new(0, 3)));
        assert_eq!(entry.info, info());
        // The same server again is an update, not a second entry.
        assert_eq!(
            s.on_datagram(41_000, from, &reply(55, &info())),
            Ok(Accepted::Updated)
        );
        assert_eq!(s.list().len(), 1);
        assert_eq!(
            s.list()
                .get(&addr("192.168.1.20:27015"))
                .unwrap()
                .last_seen_us,
            41_000
        );
    }

    #[test]
    fn unsolicited_late_malformed_and_early_datagrams_are_rejected_and_counted() {
        let mut s = Scanner::new(local());
        let from = addr("192.168.1.20:5000");
        assert_eq!(
            s.on_datagram(0, from, &reply(1, &info())),
            Err(Rejected::NotScanning)
        );
        s.start(0, 55, 1_000_000);
        assert_eq!(
            s.on_datagram(10, from, &reply(56, &info())),
            Err(Rejected::WrongNonce),
            "a reply to another scan"
        );
        assert_eq!(s.on_datagram(10, from, b"junk"), Err(Rejected::Malformed));
        assert_eq!(
            s.on_datagram(10, from, &encode_query(55, ProtocolVersion::new(0, 3))),
            Err(Rejected::Malformed),
            "a query is not a reply"
        );
        assert_eq!(
            s.on_datagram(1_000_000, from, &reply(55, &info())),
            Ok(Accepted::Added),
            "exactly at the timeout"
        );
        assert_eq!(
            s.on_datagram(1_000_001, addr("192.168.1.21:5000"), &reply(55, &info())),
            Err(Rejected::TooLate)
        );
        assert_eq!(s.rejected(), 5);
        assert_eq!(s.list().len(), 1);
    }

    #[test]
    fn a_new_scan_ignores_replies_to_the_old_one() {
        let mut s = Scanner::new(local());
        s.start(0, 1, 1_000_000);
        s.start(5_000_000, 2, 1_000_000);
        assert_eq!(
            s.on_datagram(5_000_100, addr("1.2.3.4:5"), &reply(1, &info())),
            Err(Rejected::WrongNonce)
        );
        assert!(
            s.on_datagram(5_000_100, addr("1.2.3.4:5"), &reply(2, &info()))
                .is_ok()
        );
    }

    #[test]
    fn incompatible_servers_are_marked_and_listed_after_the_ones_that_can_be_joined() {
        let mut s = Scanner::new(local());
        s.start(0, 9, 10_000_000);
        let older = ServerInfo {
            protocol: ProtocolVersion::new(0, 2),
            ..info()
        };
        let mismatch = ServerInfo {
            schema_hash: 1,
            ..info()
        };
        let full = ServerInfo {
            players: 4,
            ..info()
        };
        s.on_datagram(5_000, addr("10.0.0.1:1"), &reply(9, &older))
            .unwrap();
        s.on_datagram(30_000, addr("10.0.0.2:1"), &reply(9, &info()))
            .unwrap();
        s.on_datagram(9_000, addr("10.0.0.3:1"), &reply(9, &full))
            .unwrap();
        s.on_datagram(20_000, addr("10.0.0.4:1"), &reply(9, &info()))
            .unwrap();
        s.on_datagram(8_000, addr("10.0.0.5:1"), &reply(9, &mismatch))
            .unwrap();
        let order: Vec<String> = s
            .list()
            .for_display()
            .iter()
            .map(|e| e.addr.ip().to_string())
            .collect();
        // Joinable and free by speed (.4 then .2), then joinable but full (.3), then the incompatible ones by speed (.1, .5).
        assert_eq!(
            order,
            ["10.0.0.4", "10.0.0.2", "10.0.0.3", "10.0.0.1", "10.0.0.5"]
        );
        assert!(matches!(
            s.list().get(&addr("10.0.0.1:27015")).unwrap().compat,
            Compat::VersionMismatch { .. }
        ));
        assert!(matches!(
            s.list().get(&addr("10.0.0.5:27015")).unwrap().compat,
            Compat::SchemaMismatch { .. }
        ));
    }

    #[test]
    fn the_list_is_bounded_and_old_entries_expire() {
        let mut s = Scanner::new(local());
        s.start(0, 1, u64::MAX);
        for i in 0..(MAX_SERVERS as u32 + 50) {
            let ip = Ipv4Addr::from(0x0A00_0000 + i);
            let _ = s.on_datagram(
                u64::from(i),
                SocketAddr::new(IpAddr::V4(ip), 9),
                &reply(1, &info()),
            );
        }
        assert_eq!(s.list().len(), MAX_SERVERS);
        assert_eq!(
            s.on_datagram(999, addr("10.0.0.0:9"), &reply(1, &info())),
            Ok(Accepted::Updated),
            "a known server may still update when the list is full"
        );
        assert_eq!(
            s.on_datagram(999, addr("11.0.0.0:9"), &reply(1, &info())),
            Err(Rejected::ListFull)
        );
        let removed = s.list_mut().expire(1_000, 900);
        assert!(removed > 0 && removed < MAX_SERVERS, "{removed}");
        assert!(s.list().entries().all(|e| 1_000 - e.last_seen_us <= 900));
        let left = s.list().len();
        assert_eq!(
            s.list_mut().expire(u64::MAX, 0),
            left,
            "everything is older than 0 microseconds when time has run out"
        );
        assert!(s.list().is_empty());
    }

    #[test]
    fn a_server_that_answers_from_a_spoofed_source_still_only_gets_its_own_port_applied_to_that_address()
     {
        // The reply says which port to connect to, and the address is where the reply came from: a stranger cannot make
        // the list point at somebody else's address.
        let mut s = Scanner::new(local());
        s.start(0, 1, 1_000);
        s.on_datagram(
            10,
            addr("203.0.113.9:6000"),
            &reply(
                1,
                &ServerInfo {
                    port: 1234,
                    ..info()
                },
            ),
        )
        .unwrap();
        let e = s.list().entries().next().unwrap();
        assert_eq!(e.addr, addr("203.0.113.9:1234"));
    }

    // ---- direct addresses -------------------------------------------------------------------------

    #[test]
    fn what_a_player_types_is_read_as_an_address_or_a_host() {
        let host = |n: &str, p| {
            Some(DirectTarget::Host {
                name: n.into(),
                port: p,
            })
        };
        let ip = |s: &str| Some(DirectTarget::Addr(addr(s)));
        assert_eq!(parse_direct("192.168.1.5"), ip("192.168.1.5:27015"));
        assert_eq!(parse_direct("  192.168.1.5:5000 "), ip("192.168.1.5:5000"));
        assert_eq!(parse_direct("[::1]"), ip("[::1]:27015"));
        assert_eq!(parse_direct("[2001:db8::7]:4000"), ip("[2001:db8::7]:4000"));
        assert_eq!(
            parse_direct("::1"),
            ip("[::1]:27015"),
            "a bare IPv6 address has no port"
        );
        assert_eq!(
            parse_direct("play.Example.com"),
            host("play.example.com", 27015)
        );
        assert_eq!(parse_direct("host:1"), host("host", 1));
        assert_eq!(parse_direct("host:65535"), host("host", 65535));
        assert_eq!(
            parse_direct("a-b.c1.example:8080"),
            host("a-b.c1.example", 8080)
        );
    }

    #[test]
    fn nonsense_is_refused() {
        for bad in [
            "",
            "   ",
            "host:0",
            "host:65536",
            "host:-1",
            "host:",
            ":27015",
            "1.2.3.4:",
            "[::1",
            "[::1]x",
            "[::1]:",
            "[::1]:0",
            "999.1.1.1",
            "1.2.3",
            "1.2.3.4.5",
            "a b",
            "ho st",
            "-host",
            "host-",
            "ho..st",
            ".host",
            "host.",
            "ho_st",
            "ho$t",
            "http://host",
            "host/path",
            "host:80:80",
            "a\nb",
            "\u{0}",
            "é.com",
            "[host]:5",
        ] {
            assert_eq!(parse_direct(bad), None, "{bad:?}");
        }
        assert_eq!(
            parse_direct(&"a".repeat(64)),
            None,
            "a label of 64 characters"
        );
        assert!(parse_direct(&"a".repeat(63)).is_some());
        let long = format!(
            "{}.{}.{}.{}",
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(63)
        );
        assert_eq!(
            parse_direct(&long),
            None,
            "a name longer than 253 characters"
        );
        assert_eq!(parse_direct(&"x".repeat(400)), None);
    }

    // ---- invite codes -----------------------------------------------------------------------------

    #[test]
    fn an_invite_code_round_trips_for_ipv4_and_ipv6() {
        for a in [
            "192.168.1.5:27015",
            "10.0.0.1:1",
            "255.255.255.255:65535",
            "1.2.3.4:5000",
            "[::1]:27015",
            "[2001:db8::dead:beef]:4000",
            "127.0.0.1:27015",
        ] {
            let a = addr(a);
            let code = encode_invite(a).unwrap();
            assert_eq!(decode_invite(&code), Some(a), "{code}");
        }
        let v4 = encode_invite(addr("192.168.1.5:27015")).unwrap();
        assert_eq!(v4.replace('-', "").len(), 15, "9 bytes are 15 characters");
        assert_eq!(v4.split('-').count(), 4, "grouped in fours");
        assert!(
            v4.chars()
                .all(|c| c == '-' || ALPHABET.contains(&(c as u8)))
        );
        let v6 = encode_invite(addr("[2001:db8::1]:1")).unwrap();
        assert_eq!(v6.replace('-', "").len(), 34, "21 bytes are 34 characters");
    }

    #[test]
    fn codes_are_read_loosely_but_not_wrongly() {
        let a = addr("192.168.1.5:27015");
        let code = encode_invite(a).unwrap();
        assert_eq!(decode_invite(&code.to_lowercase()), Some(a));
        assert_eq!(decode_invite(&code.replace('-', "")), Some(a));
        assert_eq!(
            decode_invite(&format!("  {}  ", code.replace('-', " "))),
            Some(a)
        );
        // O, I and L are read as 0, 1, 1: re-type a code the way a person confuses it.
        let confused: String = code
            .chars()
            .map(|c| match c {
                '0' => 'O',
                '1' => 'l',
                other => other,
            })
            .collect();
        assert_eq!(decode_invite(&confused), Some(a));
        // Characters that are not in the alphabet, wrong lengths, empty.
        for bad in [
            "",
            "-",
            "U123-4567-89AB-CDE",
            "12345",
            &format!("{code}0"),
            &code[..code.len() - 2],
            "!!!!-!!!!-!!!!-!!!",
        ] {
            assert_eq!(decode_invite(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_typo_is_caught_by_the_checksum() {
        let a = addr("192.168.1.5:27015");
        let code = encode_invite(a).unwrap();
        let digits: Vec<char> = code.chars().collect();
        let mut caught = 0;
        let mut tried = 0;
        for i in 0..digits.len() {
            if digits[i] == '-' {
                continue;
            }
            for replacement in ALPHABET
                .iter()
                .map(|b| *b as char)
                .filter(|c| *c != digits[i])
            {
                let mut changed = digits.clone();
                changed[i] = replacement;
                let text: String = changed.into_iter().collect();
                tried += 1;
                if decode_invite(&text) != Some(a) {
                    caught += 1;
                }
                // A changed code must never decode to the original address, and almost never to another one.
            }
        }
        assert_eq!(caught, tried, "no single typo gives the same address back");
        let wrong_address = (0..digits.len())
            .filter(|i| digits[*i] != '-')
            .flat_map(|i| ALPHABET.iter().map(move |b| (i, *b as char)))
            .filter(|(i, c)| *c != digits[*i])
            .filter(|(i, c)| {
                let mut changed = digits.clone();
                changed[*i] = *c;
                let text: String = changed.into_iter().collect();
                decode_invite(&text).is_some()
            })
            .count();
        assert_eq!(
            wrong_address, 0,
            "a single wrong character is never accepted as another address"
        );
    }

    #[test]
    fn codes_are_not_made_for_addresses_that_cannot_be_joined() {
        assert_eq!(encode_invite(addr("1.2.3.4:0")), None);
        assert_eq!(encode_invite(addr("0.0.0.0:27015")), None);
        assert_eq!(encode_invite(addr("[::]:27015")), None);
    }

    /// The bytes with the checksum of all of them appended.
    fn with_checksum(mut bytes: Vec<u8>) -> Vec<u8> {
        let sum = checksum16(&bytes);
        bytes.extend_from_slice(&sum.to_be_bytes());
        bytes
    }

    /// The code for raw bytes, grouped as `encode_invite` groups it.
    fn code_of(bytes: &[u8]) -> String {
        let mut text = String::new();
        let (mut bits, mut have) = (0u32, 0u32);
        for b in bytes {
            bits = bits << 8 | u32::from(*b);
            have += 8;
            while have >= 5 {
                text.push(ALPHABET[((bits >> (have - 5)) & 31) as usize] as char);
                have -= 5;
            }
            bits &= (1 << have) - 1;
        }
        if have > 0 {
            text.push(ALPHABET[((bits << (5 - have)) & 31) as usize] as char);
        }
        text
    }

    #[test]
    fn codes_that_are_well_formed_but_wrong_in_content_are_refused() {
        let v4 = |ip: [u8; 4], port: u16, extra: &[u8]| {
            let mut b = vec![1u8 << 4 | 4];
            b.extend_from_slice(&ip);
            b.extend_from_slice(&port.to_be_bytes());
            b.extend_from_slice(extra);
            code_of(&with_checksum(b))
        };
        assert_eq!(
            decode_invite(&v4([192, 168, 1, 5], 27015, &[])),
            Some(addr("192.168.1.5:27015")),
            "the helper makes real codes"
        );
        assert_eq!(decode_invite(&v4([192, 168, 1, 5], 0, &[])), None, "port 0");
        assert_eq!(
            decode_invite(&v4([0, 0, 0, 0], 27015, &[])),
            None,
            "the unspecified address"
        );
        assert_eq!(
            decode_invite(&v4([192, 168, 1, 5], 27015, &[9])),
            None,
            "an extra byte, even with a good checksum"
        );
        let mut v6_unspecified = vec![1u8 << 4 | 6];
        v6_unspecified.extend_from_slice(&[0u8; 16]);
        v6_unspecified.extend_from_slice(&27015u16.to_be_bytes());
        assert_eq!(
            decode_invite(&code_of(&with_checksum(v6_unspecified))),
            None,
            "the unspecified IPv6 address"
        );
        let mut unknown_kind = vec![1u8 << 4 | 5];
        unknown_kind.extend_from_slice(&[1, 2, 3, 4, 0x69, 0x87]);
        assert_eq!(
            decode_invite(&code_of(&with_checksum(unknown_kind))),
            None,
            "an address kind that is neither 4 nor 6"
        );
    }

    #[test]
    fn the_longest_host_name_and_the_longest_port_text_are_exact() {
        let name = |last: usize| {
            format!(
                "{}.{}.{}.{}",
                "a".repeat(63),
                "b".repeat(63),
                "c".repeat(63),
                "d".repeat(last)
            )
        };
        assert_eq!(name(61).len(), 253);
        assert!(parse_direct(&name(61)).is_some(), "253 characters");
        assert_eq!(parse_direct(&name(62)), None, "254 characters");
        assert_eq!(
            parse_direct("host:00080"),
            Some(DirectTarget::Host {
                name: "host".into(),
                port: 80
            }),
            "five digits"
        );
        assert_eq!(
            parse_direct("host:000080"),
            None,
            "six digits are never a port"
        );
    }

    #[test]
    fn entries_exactly_max_age_old_are_kept() {
        let mut s = Scanner::new(local());
        s.start(0, 1, u64::MAX);
        s.on_datagram(100, addr("10.0.0.1:1"), &reply(1, &info()))
            .unwrap();
        assert_eq!(
            s.list_mut().expire(1_000, 900),
            0,
            "exactly 900 microseconds old is not older than 900"
        );
        assert_eq!(s.list_mut().expire(1_001, 900), 1);
    }

    #[test]
    fn an_invite_from_a_newer_version_or_with_nonzero_padding_is_refused() {
        let a = addr("192.168.1.5:27015");
        // Make a code whose version nibble is 2 by building the bytes by hand, with a correct checksum.
        let mut bytes = vec![2u8 << 4 | 4];
        bytes.extend_from_slice(&[192, 168, 1, 5]);
        bytes.extend_from_slice(&27015u16.to_be_bytes());
        let text = code_of(&with_checksum(bytes));
        assert_eq!(decode_invite(&text), None, "version 2 is not understood");
        // The real code with its last character changed so that the padding bits are not zero.
        let code = encode_invite(a).unwrap();
        let last = code.chars().last().unwrap();
        let other = if last == '0' { '1' } else { '0' };
        let tampered = format!("{}{}", &code[..code.len() - 1], other);
        assert_eq!(decode_invite(&tampered), None);
    }
}
