//! The life of a connection above the transport: joining a session, being refused or kicked, staying
//! alive, leaving (roadmap T08-006).
//!
//! A client says **hello** (protocol version and schema hash, the hash of its mods, a name, the
//! session password, an optional auth token). The server checks, in a fixed order, that the peer is
//! not banned or locked out, that the builds can talk (`ss2_net::protocol`), that the mods match, that
//! the name is acceptable and unique, that the password is right, that there is room and that the
//! auth token is good, and then answers **welcome** with a slot or **refused** with a reason a person
//! can read. Both sides send **keepalives**, drop a peer that has been silent too long, and say
//! **disconnect** when they leave.
//!
//! It is pure logic over messages, like the rest of this crate: no sockets, no clock (time is a
//! number of microseconds the caller passes in), no randomness. The transport (RFC 0006) carries the
//! messages and is expected to encrypt them, so the password travels inside that protection; this
//! module never stores or logs it and compares it in constant time.
//!
//! Everything here is built for hostile peers: the decoder bounds every length before allocating,
//! a refused peer costs the server no memory beyond a bounded failure counter, a wrong password
//! counts toward a lockout, and a duplicate hello (the first welcome was lost) is answered again
//! without taking a second slot.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::protocol::{Compat, Handshake, ProtocolVersion, check_handshake};

/// The longest player name, in bytes.
pub const MAX_NAME: usize = 32;
/// The longest session password, in bytes.
pub const MAX_PASSWORD: usize = 128;
/// The longest auth token, in bytes.
pub const MAX_TOKEN: usize = 256;
/// The longest kick reason, in bytes.
pub const MAX_REASON: usize = 64;
/// How many peers' recent password failures the server remembers at most.
pub const MAX_TRACKED_PEERS: usize = 1024;

/// Who is on the other end, as far as the session layer is concerned: an opaque id the transport
/// gives to a connection (or an address), and the key of bans and lockouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub u64);

/// Why a join was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefuseReason {
    /// The protocol versions cannot talk.
    VersionMismatch {
        /// The server's version.
        server: ProtocolVersion,
    },
    /// The versions agree but the message layouts do not.
    SchemaMismatch,
    /// The mods differ.
    ModMismatch,
    /// Wrong session password.
    BadPassword,
    /// No free slot.
    ServerFull,
    /// The peer is banned.
    Banned,
    /// The name is empty, too long or has control characters.
    InvalidName,
    /// Someone connected already has the name.
    NameTaken,
    /// The auth token was not accepted.
    AuthFailed,
    /// Too many wrong passwords: try again later.
    RateLimited {
        /// How long to wait, in milliseconds.
        retry_after_ms: u32,
    },
}

/// Why a connection ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The sender left.
    Quit,
    /// The host removed the player, with a reason.
    Kicked(String),
    /// The host banned the player.
    Banned,
    /// The server is stopping.
    ServerShutdown,
}

/// A message of the session layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// Client to server: ask to join.
    Hello {
        /// The client's protocol version and schema hash.
        handshake: Handshake,
        /// The hash of the client's mods.
        mod_hash: u64,
        /// The player's name.
        name: String,
        /// The session password (empty if none).
        password: Vec<u8>,
        /// An auth token (empty if none).
        token: Vec<u8>,
    },
    /// Server to client: you are in.
    Welcome {
        /// The server's protocol version and schema hash (the client checks them too).
        handshake: Handshake,
        /// The hash of the server's mods.
        mod_hash: u64,
        /// The client's player slot.
        slot: u8,
        /// The simulation tick rate.
        tick_hz: u32,
        /// The server's tick when the client was admitted.
        server_tick: u64,
    },
    /// Server to client: you are not coming in.
    Refused(RefuseReason),
    /// Either way: still here.
    Keepalive(u32),
    /// Either way: leaving.
    Disconnect(DisconnectReason),
}

/// Why bytes are not a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// No bytes.
    Empty,
    /// The first byte is not a message kind.
    UnknownTag(u8),
    /// The packet ends in the middle of a field.
    Truncated,
    /// Bytes follow the message.
    TrailingBytes,
    /// A text or blob claims more than its limit.
    TooLong(&'static str),
    /// A name or reason is not valid UTF-8.
    BadText,
    /// A reason code that does not exist.
    BadCode(u8),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "an empty packet"),
            Self::UnknownTag(t) => write!(f, "unknown message kind {t}"),
            Self::Truncated => write!(f, "the packet ends in the middle of a field"),
            Self::TrailingBytes => write!(f, "bytes after the end of the message"),
            Self::TooLong(what) => write!(f, "{what} is longer than allowed"),
            Self::BadText => write!(f, "text that is not UTF-8"),
            Self::BadCode(c) => write!(f, "unknown code {c}"),
        }
    }
}

impl std::error::Error for DecodeError {}

const TAG_HELLO: u8 = 1;
const TAG_WELCOME: u8 = 2;
const TAG_REFUSED: u8 = 3;
const TAG_KEEPALIVE: u8 = 4;
const TAG_DISCONNECT: u8 = 5;

fn put_handshake(out: &mut Vec<u8>, h: &Handshake) {
    out.extend_from_slice(&h.version.major.to_le_bytes());
    out.extend_from_slice(&h.version.minor.to_le_bytes());
    out.extend_from_slice(&h.schema_hash.to_le_bytes());
}

fn put_blob(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Write a message in the wire format of `docs/SESSION.md`.
pub fn encode(message: &Message) -> Vec<u8> {
    let mut out = Vec::new();
    match message {
        Message::Hello {
            handshake,
            mod_hash,
            name,
            password,
            token,
        } => {
            out.push(TAG_HELLO);
            put_handshake(&mut out, handshake);
            out.extend_from_slice(&mod_hash.to_le_bytes());
            put_blob(&mut out, name.as_bytes());
            put_blob(&mut out, password);
            put_blob(&mut out, token);
        }
        Message::Welcome {
            handshake,
            mod_hash,
            slot,
            tick_hz,
            server_tick,
        } => {
            out.push(TAG_WELCOME);
            put_handshake(&mut out, handshake);
            out.extend_from_slice(&mod_hash.to_le_bytes());
            out.push(*slot);
            out.extend_from_slice(&tick_hz.to_le_bytes());
            out.extend_from_slice(&server_tick.to_le_bytes());
        }
        Message::Refused(reason) => {
            out.push(TAG_REFUSED);
            match reason {
                RefuseReason::VersionMismatch { server } => {
                    out.push(0);
                    out.extend_from_slice(&server.major.to_le_bytes());
                    out.extend_from_slice(&server.minor.to_le_bytes());
                }
                RefuseReason::SchemaMismatch => out.push(1),
                RefuseReason::ModMismatch => out.push(2),
                RefuseReason::BadPassword => out.push(3),
                RefuseReason::ServerFull => out.push(4),
                RefuseReason::Banned => out.push(5),
                RefuseReason::InvalidName => out.push(6),
                RefuseReason::NameTaken => out.push(7),
                RefuseReason::AuthFailed => out.push(8),
                RefuseReason::RateLimited { retry_after_ms } => {
                    out.push(9);
                    out.extend_from_slice(&retry_after_ms.to_le_bytes());
                }
            }
        }
        Message::Keepalive(seq) => {
            out.push(TAG_KEEPALIVE);
            out.extend_from_slice(&seq.to_le_bytes());
        }
        Message::Disconnect(reason) => {
            out.push(TAG_DISCONNECT);
            match reason {
                DisconnectReason::Quit => out.push(0),
                DisconnectReason::Kicked(text) => {
                    out.push(1);
                    put_blob(&mut out, text.as_bytes());
                }
                DisconnectReason::Banned => out.push(2),
                DisconnectReason::ServerShutdown => out.push(3),
            }
        }
    }
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.bytes.len())
            .ok_or(DecodeError::Truncated)?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ))
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }
    fn handshake(&mut self) -> Result<Handshake, DecodeError> {
        let (major, minor) = (self.u16()?, self.u16()?);
        Ok(Handshake {
            version: ProtocolVersion::new(major, minor),
            schema_hash: self.u64()?,
        })
    }
    /// A blob with a 16-bit length, refused **before** it is read when it is over `limit`.
    fn blob(&mut self, limit: usize, what: &'static str) -> Result<&'a [u8], DecodeError> {
        let len = usize::from(self.u16()?);
        if len > limit {
            return Err(DecodeError::TooLong(what));
        }
        self.take(len)
    }
    fn text(&mut self, limit: usize, what: &'static str) -> Result<String, DecodeError> {
        String::from_utf8(self.blob(limit, what)?.to_vec()).map_err(|_| DecodeError::BadText)
    }
}

/// Read a message. Every length is checked against its limit before anything is copied.
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    let mut r = Reader { bytes, pos: 0 };
    let tag = *bytes.first().ok_or(DecodeError::Empty)?;
    r.pos = 1;
    let message = match tag {
        TAG_HELLO => Message::Hello {
            handshake: r.handshake()?,
            mod_hash: r.u64()?,
            name: r.text(MAX_NAME, "the name")?,
            password: r.blob(MAX_PASSWORD, "the password")?.to_vec(),
            token: r.blob(MAX_TOKEN, "the token")?.to_vec(),
        },
        TAG_WELCOME => Message::Welcome {
            handshake: r.handshake()?,
            mod_hash: r.u64()?,
            slot: r.u8()?,
            tick_hz: r.u32()?,
            server_tick: r.u64()?,
        },
        TAG_REFUSED => Message::Refused(match r.u8()? {
            0 => RefuseReason::VersionMismatch {
                server: ProtocolVersion::new(r.u16()?, r.u16()?),
            },
            1 => RefuseReason::SchemaMismatch,
            2 => RefuseReason::ModMismatch,
            3 => RefuseReason::BadPassword,
            4 => RefuseReason::ServerFull,
            5 => RefuseReason::Banned,
            6 => RefuseReason::InvalidName,
            7 => RefuseReason::NameTaken,
            8 => RefuseReason::AuthFailed,
            9 => RefuseReason::RateLimited {
                retry_after_ms: r.u32()?,
            },
            code => return Err(DecodeError::BadCode(code)),
        }),
        TAG_KEEPALIVE => Message::Keepalive(r.u32()?),
        TAG_DISCONNECT => Message::Disconnect(match r.u8()? {
            0 => DisconnectReason::Quit,
            1 => DisconnectReason::Kicked(r.text(MAX_REASON, "the reason")?),
            2 => DisconnectReason::Banned,
            3 => DisconnectReason::ServerShutdown,
            code => return Err(DecodeError::BadCode(code)),
        }),
        other => return Err(DecodeError::UnknownTag(other)),
    };
    if r.pos != bytes.len() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(message)
}

/// Compare two byte strings without stopping at the first difference (so the time does not tell
/// how much of a password was right). Strings of different length are different.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() ^ b.len()) as u32;
    for i in 0..a.len().max(b.len()) {
        diff |= u32::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

/// A message to send to a peer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outgoing {
    /// Who to send it to.
    pub to: PeerId,
    /// What to send.
    pub message: Message,
}

/// Why a player left, as the server saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveReason {
    /// They said goodbye.
    Quit,
    /// The host kicked them.
    Kicked,
    /// The host banned them.
    Banned,
    /// They went silent.
    TimedOut,
    /// The server stopped.
    ServerShutdown,
}

/// What happened on the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerEvent {
    /// A player joined.
    Joined {
        /// Who.
        peer: PeerId,
        /// Their slot.
        slot: u8,
        /// Their name.
        name: String,
    },
    /// A player left.
    Left {
        /// Who.
        peer: PeerId,
        /// The slot that is free again.
        slot: u8,
        /// Why.
        reason: LeaveReason,
    },
}

/// What a server needs to know.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// The server's protocol version and schema hash.
    pub handshake: Handshake,
    /// The hash of the server's mods; a client must match it.
    pub mod_hash: u64,
    /// How many players at most.
    pub max_players: u8,
    /// The session password; `None` for an open session.
    pub password: Option<Vec<u8>>,
    /// The simulation tick rate, told to clients.
    pub tick_hz: u32,
    /// A connected peer silent for longer than this (microseconds) is dropped.
    pub idle_timeout_us: u64,
    /// Send a keepalive to a peer we have not sent anything to for this long.
    pub keepalive_us: u64,
    /// Wrong passwords within the window that cause a lockout.
    pub max_failed_attempts: u32,
    /// The window in which failures are counted.
    pub attempt_window_us: u64,
    /// How long a locked-out peer is refused outright.
    pub lockout_us: u64,
}

impl ServerConfig {
    /// A configuration with sensible limits: 8 players, no password, 30 Hz, a 10 s idle timeout,
    /// a keepalive every 2 s, and five wrong passwords in a minute earning a five minute lockout.
    pub fn new(handshake: Handshake) -> Self {
        Self {
            handshake,
            mod_hash: 0,
            max_players: 8,
            password: None,
            tick_hz: 30,
            idle_timeout_us: 10_000_000,
            keepalive_us: 2_000_000,
            max_failed_attempts: 5,
            attempt_window_us: 60_000_000,
            lockout_us: 300_000_000,
        }
    }
}

struct Peer {
    slot: u8,
    name: String,
    last_heard: u64,
    last_sent: u64,
    welcome: Message,
}

#[derive(Clone, Copy)]
struct Failures {
    count: u32,
    first_at: u64,
    locked_until: u64,
}

type Authenticator = Box<dyn FnMut(&str, &[u8]) -> bool>;

/// The server side of the session layer: who is in, who may join, who is kept out.
///
/// ```
/// use ss2_net::protocol::{Handshake, ProtocolVersion};
/// use ss2_net::session::{ClientConfig, ClientState, Message, PeerId, ServerConfig, SessionClient, SessionServer};
///
/// let handshake = Handshake { version: ProtocolVersion::new(1, 0), schema_hash: 7 };
/// let mut server = SessionServer::new(ServerConfig::new(handshake));
/// let mut client = SessionClient::new(ClientConfig::new(handshake, "Sam"));
/// let hello = client.start(0).remove(0);
/// let reply = server.handle(PeerId(1), hello, 0).remove(0).message;
/// client.handle(reply, 1_000);
/// assert!(matches!(client.state(), ClientState::Connected { slot: 0, .. }));
/// assert_eq!(server.connected(), 1);
/// ```
pub struct SessionServer {
    cfg: ServerConfig,
    peers: BTreeMap<PeerId, Peer>,
    banned: BTreeSet<PeerId>,
    failures: BTreeMap<PeerId, Failures>,
    authenticator: Option<Authenticator>,
    events: Vec<ServerEvent>,
    server_tick: u64,
    keepalive_seq: u32,
}

fn name_is_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && !name.chars().any(char::is_control)
        && !name.trim().is_empty()
}

fn cut_to(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

impl SessionServer {
    /// A server with nobody connected.
    pub fn new(cfg: ServerConfig) -> Self {
        Self {
            cfg,
            peers: BTreeMap::new(),
            banned: BTreeSet::new(),
            failures: BTreeMap::new(),
            authenticator: None,
            events: Vec::new(),
            server_tick: 0,
            keepalive_seq: 0,
        }
    }

    /// Check auth tokens with `check(name, token)`; without one, every token is accepted.
    pub fn set_authenticator(&mut self, check: Authenticator) {
        self.authenticator = Some(check);
    }

    /// How many players are connected.
    pub fn connected(&self) -> usize {
        self.peers.len()
    }

    /// The peer in `slot`, if any.
    pub fn peer_in_slot(&self, slot: u8) -> Option<PeerId> {
        self.peers
            .iter()
            .find(|(_, p)| p.slot == slot)
            .map(|(id, _)| *id)
    }

    /// How many peers' password failures are remembered (bounded by [`MAX_TRACKED_PEERS`]).
    pub fn tracked_failures(&self) -> usize {
        self.failures.len()
    }

    /// Take what happened since the last call.
    pub fn take_events(&mut self) -> Vec<ServerEvent> {
        std::mem::take(&mut self.events)
    }

    fn refuse(peer: PeerId, reason: RefuseReason) -> Vec<Outgoing> {
        vec![Outgoing {
            to: peer,
            message: Message::Refused(reason),
        }]
    }

    fn record_failure(&mut self, peer: PeerId, now: u64) {
        if !self.failures.contains_key(&peer) && self.failures.len() >= MAX_TRACKED_PEERS {
            // Make room: forget what has expired, else the oldest failure that is not a lockout, else the oldest.
            let window = self.cfg.attempt_window_us;
            self.failures
                .retain(|_, f| now < f.locked_until || now.saturating_sub(f.first_at) <= window);
            if self.failures.len() >= MAX_TRACKED_PEERS {
                let victim = self
                    .failures
                    .iter()
                    .min_by_key(|(_, f)| (now < f.locked_until, f.first_at))
                    .map(|(id, _)| *id);
                if let Some(victim) = victim {
                    self.failures.remove(&victim);
                }
            }
        }
        let window = self.cfg.attempt_window_us;
        let entry = self.failures.entry(peer).or_insert(Failures {
            count: 0,
            first_at: now,
            locked_until: 0,
        });
        if now.saturating_sub(entry.first_at) > window {
            *entry = Failures {
                count: 0,
                first_at: now,
                locked_until: 0,
            };
        }
        entry.count += 1;
        if entry.count >= self.cfg.max_failed_attempts {
            entry.locked_until = now + self.cfg.lockout_us;
            entry.count = 0;
            entry.first_at = now;
        }
    }

    /// Handle a message from `peer` at time `now_us`; returns what to send back.
    pub fn handle(&mut self, peer: PeerId, message: Message, now_us: u64) -> Vec<Outgoing> {
        match message {
            Message::Hello {
                handshake,
                mod_hash,
                name,
                password,
                token,
            } => self.on_hello(peer, handshake, mod_hash, name, &password, &token, now_us),
            Message::Keepalive(_) => {
                if let Some(p) = self.peers.get_mut(&peer) {
                    p.last_heard = now_us;
                }
                Vec::new()
            }
            Message::Disconnect(_) => {
                self.remove(peer, LeaveReason::Quit);
                Vec::new()
            }
            Message::Welcome { .. } | Message::Refused(_) => Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_hello(
        &mut self,
        peer: PeerId,
        handshake: Handshake,
        mod_hash: u64,
        name: String,
        password: &[u8],
        token: &[u8],
        now: u64,
    ) -> Vec<Outgoing> {
        if self.banned.contains(&peer) {
            return Self::refuse(peer, RefuseReason::Banned);
        }
        if let Some(existing) = self.peers.get_mut(&peer) {
            existing.last_heard = now;
            return vec![Outgoing {
                to: peer,
                message: existing.welcome.clone(),
            }];
        }
        if let Some(f) = self.failures.get(&peer)
            && now < f.locked_until
        {
            let retry_after_ms =
                u32::try_from((f.locked_until - now).div_ceil(1000)).unwrap_or(u32::MAX);
            return Self::refuse(peer, RefuseReason::RateLimited { retry_after_ms });
        }
        match check_handshake(&self.cfg.handshake, &handshake) {
            Compat::Compatible(_) => {}
            Compat::VersionMismatch { .. } => {
                return Self::refuse(
                    peer,
                    RefuseReason::VersionMismatch {
                        server: self.cfg.handshake.version,
                    },
                );
            }
            Compat::SchemaMismatch { .. } => {
                return Self::refuse(peer, RefuseReason::SchemaMismatch);
            }
        }
        if mod_hash != self.cfg.mod_hash {
            return Self::refuse(peer, RefuseReason::ModMismatch);
        }
        if !name_is_valid(&name) {
            return Self::refuse(peer, RefuseReason::InvalidName);
        }
        let lowered = name.to_lowercase();
        if self
            .peers
            .values()
            .any(|p| p.name.to_lowercase() == lowered)
        {
            return Self::refuse(peer, RefuseReason::NameTaken);
        }
        if let Some(expected) = &self.cfg.password
            && !constant_time_eq(expected, password)
        {
            self.record_failure(peer, now);
            return Self::refuse(peer, RefuseReason::BadPassword);
        }
        if self.peers.len() >= usize::from(self.cfg.max_players) {
            return Self::refuse(peer, RefuseReason::ServerFull);
        }
        if let Some(check) = &mut self.authenticator
            && !check(&name, token)
        {
            return Self::refuse(peer, RefuseReason::AuthFailed);
        }
        let slot = (0..self.cfg.max_players)
            .find(|s| !self.peers.values().any(|p| p.slot == *s))
            .expect("there is room: counted above");
        let welcome = Message::Welcome {
            handshake: self.cfg.handshake,
            mod_hash: self.cfg.mod_hash,
            slot,
            tick_hz: self.cfg.tick_hz,
            server_tick: self.server_tick,
        };
        self.peers.insert(
            peer,
            Peer {
                slot,
                name: name.clone(),
                last_heard: now,
                last_sent: now,
                welcome: welcome.clone(),
            },
        );
        self.failures.remove(&peer);
        self.events.push(ServerEvent::Joined { peer, slot, name });
        vec![Outgoing {
            to: peer,
            message: welcome,
        }]
    }

    fn remove(&mut self, peer: PeerId, reason: LeaveReason) {
        if let Some(p) = self.peers.remove(&peer) {
            self.events.push(ServerEvent::Left {
                peer,
                slot: p.slot,
                reason,
            });
        }
    }

    /// Advance time: drop peers that have been silent too long and send keepalives to quiet ones.
    /// `server_tick` is what new clients are told.
    pub fn tick(&mut self, now_us: u64, server_tick: u64) -> Vec<Outgoing> {
        self.server_tick = server_tick;
        let idle = self.cfg.idle_timeout_us;
        let silent: Vec<PeerId> = self
            .peers
            .iter()
            .filter(|(_, p)| now_us.saturating_sub(p.last_heard) > idle)
            .map(|(id, _)| *id)
            .collect();
        for peer in silent {
            self.remove(peer, LeaveReason::TimedOut);
        }
        let mut out = Vec::new();
        for (id, p) in &mut self.peers {
            if now_us.saturating_sub(p.last_sent) >= self.cfg.keepalive_us {
                p.last_sent = now_us;
                self.keepalive_seq = self.keepalive_seq.wrapping_add(1);
                out.push(Outgoing {
                    to: *id,
                    message: Message::Keepalive(self.keepalive_seq),
                });
            }
        }
        out
    }

    /// Remove the player in `slot`, telling them why (the reason is cut to [`MAX_REASON`] bytes).
    pub fn kick(&mut self, slot: u8, reason: &str) -> Vec<Outgoing> {
        let Some(peer) = self.peer_in_slot(slot) else {
            return Vec::new();
        };
        self.remove(peer, LeaveReason::Kicked);
        vec![Outgoing {
            to: peer,
            message: Message::Disconnect(DisconnectReason::Kicked(cut_to(reason, MAX_REASON))),
        }]
    }

    /// Keep `peer` out from now on, and disconnect it if it is connected.
    pub fn ban(&mut self, peer: PeerId) -> Vec<Outgoing> {
        self.banned.insert(peer);
        if self.peers.contains_key(&peer) {
            self.remove(peer, LeaveReason::Banned);
            return vec![Outgoing {
                to: peer,
                message: Message::Disconnect(DisconnectReason::Banned),
            }];
        }
        Vec::new()
    }

    /// Let `peer` in again.
    pub fn unban(&mut self, peer: PeerId) {
        self.banned.remove(&peer);
    }

    /// Tell everyone the server is stopping, and empty it.
    pub fn shutdown(&mut self) -> Vec<Outgoing> {
        let everyone: Vec<PeerId> = self.peers.keys().copied().collect();
        for peer in &everyone {
            self.remove(*peer, LeaveReason::ServerShutdown);
        }
        everyone
            .into_iter()
            .map(|to| Outgoing {
                to,
                message: Message::Disconnect(DisconnectReason::ServerShutdown),
            })
            .collect()
    }
}

/// Where a client is in joining and staying connected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientState {
    /// Not started.
    Idle,
    /// Hello sent, waiting for a welcome (and sending it again, with growing gaps).
    Connecting,
    /// In the session.
    Connected {
        /// The slot the server gave.
        slot: u8,
        /// The tick rate of the simulation.
        tick_hz: u32,
        /// The server's tick when this client was admitted.
        server_tick: u64,
    },
    /// The server said no.
    Refused(RefuseReason),
    /// The server's welcome shows builds that cannot talk (the client checks, whatever the server says).
    Incompatible(Compat),
    /// No answer in time, or the server went silent.
    TimedOut,
    /// The connection ended.
    Disconnected(DisconnectReason),
}

/// What happened on the client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientEvent {
    /// The welcome arrived.
    Connected {
        /// The slot.
        slot: u8,
        /// The tick rate.
        tick_hz: u32,
        /// The server's tick.
        server_tick: u64,
    },
    /// The server refused the join.
    Refused(RefuseReason),
    /// The server is not compatible.
    Incompatible(Compat),
    /// The connection ended.
    Disconnected(DisconnectReason),
    /// Nothing was heard in time.
    TimedOut,
}

/// What a client needs to know.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// This build's protocol version and schema hash.
    pub handshake: Handshake,
    /// The hash of this client's mods.
    pub mod_hash: u64,
    /// The player's name.
    pub name: String,
    /// The session password, if the player was given one.
    pub password: Vec<u8>,
    /// An auth token, if there is one.
    pub token: Vec<u8>,
    /// The first gap before sending hello again (microseconds); it doubles up to 4 seconds.
    pub hello_retry_us: u64,
    /// Give up joining after this long.
    pub connect_timeout_us: u64,
    /// The server silent for longer than this ends the connection.
    pub idle_timeout_us: u64,
    /// Send a keepalive when nothing was sent for this long.
    pub keepalive_us: u64,
}

impl ClientConfig {
    /// A configuration with sensible timings: retry after half a second, give up after 10 s,
    /// 10 s of silence ends a connection, a keepalive every 2 s.
    pub fn new(handshake: Handshake, name: &str) -> Self {
        Self {
            handshake,
            mod_hash: 0,
            name: name.to_owned(),
            password: Vec::new(),
            token: Vec::new(),
            hello_retry_us: 500_000,
            connect_timeout_us: 10_000_000,
            idle_timeout_us: 10_000_000,
            keepalive_us: 2_000_000,
        }
    }
}

const MAX_HELLO_GAP_US: u64 = 4_000_000;

/// The client side of the session layer.
pub struct SessionClient {
    cfg: ClientConfig,
    state: ClientState,
    started_at: u64,
    last_hello: u64,
    gap: u64,
    last_heard: u64,
    last_sent: u64,
    keepalive_seq: u32,
    events: Vec<ClientEvent>,
}

impl SessionClient {
    /// A client that has not started.
    pub fn new(cfg: ClientConfig) -> Self {
        let gap = cfg.hello_retry_us;
        Self {
            cfg,
            state: ClientState::Idle,
            started_at: 0,
            last_hello: 0,
            gap,
            last_heard: 0,
            last_sent: 0,
            keepalive_seq: 0,
            events: Vec::new(),
        }
    }

    /// Where the client is.
    pub fn state(&self) -> &ClientState {
        &self.state
    }

    /// Take what happened since the last call.
    pub fn take_events(&mut self) -> Vec<ClientEvent> {
        std::mem::take(&mut self.events)
    }

    fn hello(&self) -> Message {
        Message::Hello {
            handshake: self.cfg.handshake,
            mod_hash: self.cfg.mod_hash,
            name: self.cfg.name.clone(),
            password: self.cfg.password.clone(),
            token: self.cfg.token.clone(),
        }
    }

    /// Begin joining: returns the first hello.
    pub fn start(&mut self, now_us: u64) -> Vec<Message> {
        self.state = ClientState::Connecting;
        self.started_at = now_us;
        self.last_hello = now_us;
        self.gap = self.cfg.hello_retry_us;
        vec![self.hello()]
    }

    /// Handle a message from the server; returns what to send back.
    pub fn handle(&mut self, message: Message, now_us: u64) -> Vec<Message> {
        match (&self.state, message) {
            (
                ClientState::Connecting,
                Message::Welcome {
                    handshake,
                    slot,
                    tick_hz,
                    server_tick,
                    ..
                },
            ) => match check_handshake(&self.cfg.handshake, &handshake) {
                Compat::Compatible(_) => {
                    self.state = ClientState::Connected {
                        slot,
                        tick_hz,
                        server_tick,
                    };
                    self.last_heard = now_us;
                    self.last_sent = now_us;
                    self.events.push(ClientEvent::Connected {
                        slot,
                        tick_hz,
                        server_tick,
                    });
                    Vec::new()
                }
                other => {
                    self.state = ClientState::Incompatible(other);
                    self.events.push(ClientEvent::Incompatible(other));
                    vec![Message::Disconnect(DisconnectReason::Quit)]
                }
            },
            (ClientState::Connecting, Message::Refused(reason)) => {
                self.state = ClientState::Refused(reason.clone());
                self.events.push(ClientEvent::Refused(reason));
                Vec::new()
            }
            (
                ClientState::Connecting | ClientState::Connected { .. },
                Message::Disconnect(reason),
            ) => {
                self.state = ClientState::Disconnected(reason.clone());
                self.events.push(ClientEvent::Disconnected(reason));
                Vec::new()
            }
            (ClientState::Connected { .. }, _) => {
                self.last_heard = now_us;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Advance time: send hello again while joining, keepalives while connected, and give up on silence.
    pub fn tick(&mut self, now_us: u64) -> Vec<Message> {
        match self.state {
            ClientState::Connecting => {
                if now_us.saturating_sub(self.started_at) >= self.cfg.connect_timeout_us {
                    self.state = ClientState::TimedOut;
                    self.events.push(ClientEvent::TimedOut);
                    return Vec::new();
                }
                if now_us.saturating_sub(self.last_hello) >= self.gap {
                    self.last_hello = now_us;
                    self.gap = (self.gap * 2).min(MAX_HELLO_GAP_US);
                    return vec![self.hello()];
                }
                Vec::new()
            }
            ClientState::Connected { .. } => {
                if now_us.saturating_sub(self.last_heard) > self.cfg.idle_timeout_us {
                    self.state = ClientState::TimedOut;
                    self.events.push(ClientEvent::TimedOut);
                    return Vec::new();
                }
                if now_us.saturating_sub(self.last_sent) >= self.cfg.keepalive_us {
                    self.last_sent = now_us;
                    self.keepalive_seq = self.keepalive_seq.wrapping_add(1);
                    return vec![Message::Keepalive(self.keepalive_seq)];
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Leave: returns the goodbye to send.
    pub fn disconnect(&mut self) -> Vec<Message> {
        match self.state {
            ClientState::Connecting | ClientState::Connected { .. } => {
                self.state = ClientState::Disconnected(DisconnectReason::Quit);
                self.events
                    .push(ClientEvent::Disconnected(DisconnectReason::Quit));
                vec![Message::Disconnect(DisconnectReason::Quit)]
            }
            _ => Vec::new(),
        }
    }

    /// A sentence for the player when the join was refused or the server is incompatible; `None` otherwise.
    pub fn refusal_text(&self) -> Option<String> {
        match &self.state {
            ClientState::Refused(reason) => Some(match reason {
                RefuseReason::VersionMismatch { server } => {
                    let ours = self.cfg.handshake.version;
                    if *server > ours {
                        format!("the server is newer (protocol {server}, this build speaks {ours}): update this game")
                    } else {
                        format!("the server is older (protocol {server}, this build speaks {ours}): the host needs to update")
                    }
                }
                RefuseReason::SchemaMismatch => "the builds report the same protocol version but disagree about the message formats: they are not from the same source".to_owned(),
                RefuseReason::ModMismatch => "your mods do not match the server's".to_owned(),
                RefuseReason::BadPassword => "wrong password".to_owned(),
                RefuseReason::ServerFull => "the server is full".to_owned(),
                RefuseReason::Banned => "you are banned from this server".to_owned(),
                RefuseReason::InvalidName => "that name cannot be used".to_owned(),
                RefuseReason::NameTaken => "someone is already using that name".to_owned(),
                RefuseReason::AuthFailed => "the server did not accept your sign-in".to_owned(),
                RefuseReason::RateLimited { retry_after_ms } => format!("too many wrong passwords: try again in {} seconds", retry_after_ms.div_ceil(1000)),
            }),
            ClientState::Incompatible(compat) => compat.explanation(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::{Duplex, LinkConfig};
    use crate::protocol::{Handshake, ProtocolVersion};

    const SECOND: u64 = 1_000_000;

    fn hs() -> Handshake {
        Handshake {
            version: ProtocolVersion::new(1, 2),
            schema_hash: 0xABCD,
        }
    }

    fn server_config() -> ServerConfig {
        ServerConfig::new(hs())
    }

    fn client_config(name: &str) -> ClientConfig {
        ClientConfig::new(hs(), name)
    }

    // ---- the wire ---------------------------------------------------------------------------

    fn every_message() -> Vec<Message> {
        vec![
            Message::Hello {
                handshake: hs(),
                mod_hash: 7,
                name: "Sam".into(),
                password: b"secret".to_vec(),
                token: vec![1, 2, 3],
            },
            Message::Hello {
                handshake: hs(),
                mod_hash: 0,
                name: "x".into(),
                password: vec![],
                token: vec![],
            },
            Message::Welcome {
                handshake: hs(),
                mod_hash: 7,
                slot: 3,
                tick_hz: 30,
                server_tick: 123_456,
            },
            Message::Refused(RefuseReason::VersionMismatch {
                server: ProtocolVersion::new(2, 0),
            }),
            Message::Refused(RefuseReason::SchemaMismatch),
            Message::Refused(RefuseReason::ModMismatch),
            Message::Refused(RefuseReason::BadPassword),
            Message::Refused(RefuseReason::ServerFull),
            Message::Refused(RefuseReason::Banned),
            Message::Refused(RefuseReason::InvalidName),
            Message::Refused(RefuseReason::NameTaken),
            Message::Refused(RefuseReason::AuthFailed),
            Message::Refused(RefuseReason::RateLimited {
                retry_after_ms: 4_500,
            }),
            Message::Keepalive(77),
            Message::Disconnect(DisconnectReason::Quit),
            Message::Disconnect(DisconnectReason::Kicked("AFK for too long".into())),
            Message::Disconnect(DisconnectReason::Banned),
            Message::Disconnect(DisconnectReason::ServerShutdown),
        ]
    }

    #[test]
    fn every_message_survives_the_wire() {
        for m in every_message() {
            assert_eq!(decode(&encode(&m)).as_ref(), Ok(&m), "{m:?}");
        }
    }

    #[test]
    fn the_decoder_refuses_bad_input_by_name_and_never_panics() {
        assert_eq!(decode(&[]), Err(DecodeError::Empty));
        assert_eq!(decode(&[0xEE, 1, 2]), Err(DecodeError::UnknownTag(0xEE)));
        let hello = encode(&every_message()[0]);
        for cut in 1..hello.len() {
            assert_eq!(
                decode(&hello[..cut]),
                Err(DecodeError::Truncated),
                "cut at {cut}"
            );
        }
        let mut longer = hello.clone();
        longer.push(0);
        assert_eq!(decode(&longer), Err(DecodeError::TrailingBytes));
        // A name that claims more bytes than the limit, or than the packet holds, is refused before anything is allocated.
        let mut claim = hello.clone();
        let name_len_at = 1 + 2 + 2 + 8 + 8; // tag, version, schema, mod hash... see the layout in encode
        claim[name_len_at] = 0xFF;
        claim[name_len_at + 1] = 0xFF;
        assert!(matches!(
            decode(&claim),
            Err(DecodeError::TooLong(_) | DecodeError::Truncated)
        ));
        // Random bytes, including ones that start with each valid tag.
        let mut state = 12345_u64;
        for _ in 0..20_000 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let len = (state >> 60) as usize * 4;
            let bytes: Vec<u8> = (0..len)
                .map(|i| {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    if i == 0 {
                        (state >> 61) as u8
                    } else {
                        (state >> 56) as u8
                    }
                })
                .collect();
            let _ = decode(&bytes);
        }
    }

    // ---- a world to run the sessions in -----------------------------------------------------

    struct World {
        server: SessionServer,
        clients: Vec<(PeerId, SessionClient, Duplex<Vec<u8>>)>,
        now: u64,
        server_events: Vec<ServerEvent>,
    }

    impl World {
        fn new(server: ServerConfig) -> Self {
            Self {
                server: SessionServer::new(server),
                clients: Vec::new(),
                now: 0,
                server_events: Vec::new(),
            }
        }

        fn add(&mut self, id: u64, config: ClientConfig, link: LinkConfig) -> usize {
            let mut client = SessionClient::new(config);
            let mut duplex = Duplex::symmetric(link, id + 1000);
            for m in client.start(self.now) {
                let bytes = encode(&m);
                duplex.a_to_b.send(self.now, bytes.clone(), bytes.len());
            }
            self.clients.push((PeerId(id), client, duplex));
            self.clients.len() - 1
        }

        /// Run `micros` of virtual time in steps of 10 ms.
        fn run(&mut self, micros: u64) {
            let end = self.now + micros;
            while self.now < end {
                self.now += 10_000;
                let now = self.now;
                // Client to server.
                let mut to_server = Vec::new();
                for (id, _, duplex) in &mut self.clients {
                    for (_, bytes) in duplex.a_to_b.poll(now) {
                        to_server.push((*id, bytes));
                    }
                }
                for (id, bytes) in to_server {
                    if let Ok(m) = decode(&bytes) {
                        let out = self.server.handle(id, m, now);
                        self.deliver_from_server(out);
                    }
                }
                let out = self.server.tick(now, now / 33_333);
                self.deliver_from_server(out);
                // Server to client, then the clients' own timers.
                for (_, client, duplex) in &mut self.clients {
                    let mut replies = Vec::new();
                    for (_, bytes) in duplex.b_to_a.poll(now) {
                        if let Ok(m) = decode(&bytes) {
                            replies.extend(client.handle(m, now));
                        }
                    }
                    replies.extend(client.tick(now));
                    for m in replies {
                        let bytes = encode(&m);
                        duplex.a_to_b.send(now, bytes.clone(), bytes.len());
                    }
                }
                self.server_events.extend(self.server.take_events());
            }
        }

        fn deliver_from_server(&mut self, out: Vec<Outgoing>) {
            let now = self.now;
            for o in out {
                if let Some((_, _, duplex)) =
                    self.clients.iter_mut().rev().find(|(id, _, _)| *id == o.to)
                {
                    let bytes = encode(&o.message);
                    duplex.b_to_a.send(now, bytes.clone(), bytes.len());
                }
            }
        }

        fn state(&self, index: usize) -> &ClientState {
            self.clients[index].1.state()
        }
    }

    fn lossy() -> LinkConfig {
        LinkConfig {
            latency_us: 50_000,
            jitter_us: 20_000,
            loss_ppm: 200_000,
            duplicate_ppm: 100_000,
            ..LinkConfig::default()
        }
    }

    // ---- joining ---------------------------------------------------------------------------

    #[test]
    fn a_client_joins_on_a_perfect_link_and_gets_a_slot() {
        let mut w = World::new(server_config());
        let c = w.add(1, client_config("Sam"), LinkConfig::default());
        w.run(SECOND);
        assert!(
            matches!(
                w.state(c),
                ClientState::Connected {
                    slot: 0,
                    tick_hz: 30,
                    ..
                }
            ),
            "{:?}",
            w.state(c)
        );
        assert_eq!(w.server.connected(), 1);
        assert_eq!(
            w.server_events,
            vec![ServerEvent::Joined {
                peer: PeerId(1),
                slot: 0,
                name: "Sam".into()
            }]
        );
        assert_eq!(w.server.peer_in_slot(0), Some(PeerId(1)));
    }

    #[test]
    fn a_lossy_duplicating_link_still_joins_exactly_once() {
        for seed in 0..20_u64 {
            let mut w = World::new(server_config());
            let c = w.add(seed, client_config("Sam"), lossy());
            w.run(8 * SECOND);
            assert!(
                matches!(w.state(c), ClientState::Connected { .. }),
                "seed {seed}: {:?}",
                w.state(c)
            );
            assert_eq!(w.server.connected(), 1, "duplicate hellos take one slot");
            let joins = w
                .server_events
                .iter()
                .filter(|e| matches!(e, ServerEvent::Joined { .. }))
                .count();
            assert_eq!(joins, 1, "seed {seed}: one Joined event");
        }
    }

    #[test]
    fn a_repeated_hello_is_answered_with_the_same_welcome_and_no_second_slot() {
        let mut s = SessionServer::new(server_config());
        let hello = Message::Hello {
            handshake: hs(),
            mod_hash: 0,
            name: "Sam".into(),
            password: vec![],
            token: vec![],
        };
        let first = s.handle(PeerId(5), hello.clone(), 0);
        let again = s.handle(PeerId(5), hello, 1_000);
        assert_eq!(first, again);
        assert_eq!(s.connected(), 1);
        assert_eq!(s.take_events().len(), 1);
    }

    #[test]
    fn builds_that_cannot_talk_are_refused_with_a_reason_and_take_no_slot() {
        let mut w = World::new(server_config());
        let old = Handshake {
            version: ProtocolVersion::new(0, 9),
            schema_hash: 0xABCD,
        };
        let a = w.add(1, ClientConfig::new(old, "Old"), LinkConfig::default());
        let other_schema = Handshake {
            version: ProtocolVersion::new(1, 2),
            schema_hash: 0xDEAD,
        };
        let b = w.add(
            2,
            ClientConfig::new(other_schema, "Odd"),
            LinkConfig::default(),
        );
        let mut modded = client_config("Modder");
        modded.mod_hash = 99;
        let c = w.add(3, modded, LinkConfig::default());
        w.run(SECOND);
        assert!(
            matches!(
                w.state(a),
                ClientState::Refused(RefuseReason::VersionMismatch { .. })
            ),
            "{:?}",
            w.state(a)
        );
        assert!(
            matches!(
                w.state(b),
                ClientState::Refused(RefuseReason::SchemaMismatch)
            ),
            "{:?}",
            w.state(b)
        );
        assert!(
            matches!(w.state(c), ClientState::Refused(RefuseReason::ModMismatch)),
            "{:?}",
            w.state(c)
        );
        assert_eq!(w.server.connected(), 0);
        assert!(
            w.clients[a].1.refusal_text().unwrap().contains("update"),
            "a sentence a person can read"
        );
    }

    #[test]
    fn a_welcome_from_an_incompatible_server_is_not_trusted() {
        let mut c = SessionClient::new(client_config("Sam"));
        c.start(0);
        let liar = Message::Welcome {
            handshake: Handshake {
                version: ProtocolVersion::new(9, 0),
                schema_hash: 1,
            },
            mod_hash: 0,
            slot: 0,
            tick_hz: 30,
            server_tick: 0,
        };
        let out = c.handle(liar, 10);
        assert!(
            matches!(c.state(), ClientState::Incompatible(_)),
            "{:?}",
            c.state()
        );
        assert!(
            out.iter()
                .any(|m| matches!(m, Message::Disconnect(DisconnectReason::Quit))),
            "it leaves politely"
        );
    }

    #[test]
    fn names_must_be_sensible_and_unique() {
        let mut w = World::new(server_config());
        let long = "x".repeat(MAX_NAME + 1);
        let a = w.add(1, client_config(""), LinkConfig::default());
        let b = w.add(2, client_config(&long), LinkConfig::default());
        let c = w.add(3, client_config("bad\u{7}name"), LinkConfig::default());
        let d = w.add(4, client_config("Sam"), LinkConfig::default());
        let e = w.add(5, client_config("sam"), LinkConfig::default());
        w.run(SECOND);
        for i in [a, c] {
            assert!(
                matches!(w.state(i), ClientState::Refused(RefuseReason::InvalidName)),
                "{:?}",
                w.state(i)
            );
        }
        // A name over the limit cannot even be decoded: the server never sees a hello it could answer, and ignores it.
        assert!(
            matches!(w.state(b), ClientState::Connecting),
            "{:?}",
            w.state(b)
        );
        let too_long = encode(&Message::Hello {
            handshake: hs(),
            mod_hash: 0,
            name: long.clone(),
            password: vec![],
            token: vec![],
        });
        assert_eq!(decode(&too_long), Err(DecodeError::TooLong("the name")));
        assert!(matches!(w.state(d), ClientState::Connected { .. }));
        assert!(
            matches!(w.state(e), ClientState::Refused(RefuseReason::NameTaken)),
            "names are compared without case: {:?}",
            w.state(e)
        );
    }

    // ---- password, lockout, auth ------------------------------------------------------------

    fn passworded() -> ServerConfig {
        ServerConfig {
            password: Some(b"hunter2".to_vec()),
            max_failed_attempts: 3,
            attempt_window_us: 60 * SECOND,
            lockout_us: 300 * SECOND,
            ..server_config()
        }
    }

    fn hello_with(name: &str, password: &[u8]) -> Message {
        Message::Hello {
            handshake: hs(),
            mod_hash: 0,
            name: name.into(),
            password: password.to_vec(),
            token: vec![],
        }
    }

    fn refusal(out: &[Outgoing]) -> Option<RefuseReason> {
        out.iter().find_map(|o| {
            if let Message::Refused(r) = &o.message {
                Some(r.clone())
            } else {
                None
            }
        })
    }

    #[test]
    fn the_right_password_joins_and_the_wrong_one_is_refused() {
        let mut s = SessionServer::new(passworded());
        assert_eq!(
            refusal(&s.handle(PeerId(1), hello_with("A", b"wrong"), 0)),
            Some(RefuseReason::BadPassword)
        );
        assert_eq!(
            refusal(&s.handle(PeerId(1), hello_with("A", b""), 1)),
            Some(RefuseReason::BadPassword)
        );
        assert_eq!(
            refusal(&s.handle(PeerId(1), hello_with("A", b"hunter2"), 2)),
            None
        );
        assert_eq!(s.connected(), 1);
        // A prefix or an extension of the password is not the password.
        let mut s = SessionServer::new(passworded());
        assert_eq!(
            refusal(&s.handle(PeerId(2), hello_with("B", b"hunter"), 0)),
            Some(RefuseReason::BadPassword)
        );
        assert_eq!(
            refusal(&s.handle(PeerId(2), hello_with("B", b"hunter22"), 1)),
            Some(RefuseReason::BadPassword)
        );
    }

    #[test]
    fn guessing_the_password_locks_that_peer_out_even_for_the_right_one_until_the_lockout_ends() {
        let mut s = SessionServer::new(passworded());
        for i in 0..3 {
            assert_eq!(
                refusal(&s.handle(PeerId(9), hello_with("G", b"nope"), i * SECOND)),
                Some(RefuseReason::BadPassword)
            );
        }
        let locked = refusal(&s.handle(PeerId(9), hello_with("G", b"hunter2"), 4 * SECOND));
        let Some(RefuseReason::RateLimited { retry_after_ms }) = locked else {
            panic!("{locked:?}")
        };
        assert!(
            retry_after_ms > 290_000 && retry_after_ms <= 300_000,
            "{retry_after_ms}"
        );
        assert_eq!(
            s.connected(),
            0,
            "the right password during a lockout is not accepted"
        );
        // Another peer is unaffected.
        assert_eq!(
            refusal(&s.handle(PeerId(10), hello_with("H", b"hunter2"), 5 * SECOND)),
            None
        );
        // After the lockout the right password works.
        assert_eq!(
            refusal(&s.handle(
                PeerId(9),
                hello_with("G", b"hunter2"),
                2 * SECOND + 300 * SECOND + 1
            )),
            None
        );
    }

    #[test]
    fn failures_older_than_the_window_are_forgotten() {
        let mut s = SessionServer::new(passworded());
        for round in 0..5_u64 {
            for i in 0..2 {
                let at = round * 100 * SECOND + i * SECOND; // two failures per round, rounds 100 s apart (window is 60 s)
                assert_eq!(
                    refusal(&s.handle(PeerId(4), hello_with("S", b"x"), at)),
                    Some(RefuseReason::BadPassword)
                );
            }
        }
    }

    #[test]
    fn what_the_server_remembers_about_failures_is_bounded() {
        let mut s = SessionServer::new(passworded());
        for i in 0..50_000_u64 {
            s.handle(PeerId(i), hello_with("X", b"wrong"), i);
        }
        assert!(
            s.tracked_failures() <= MAX_TRACKED_PEERS,
            "{}",
            s.tracked_failures()
        );
        assert_eq!(
            s.connected(),
            0,
            "and a refused peer holds no slot or state"
        );
    }

    #[test]
    fn an_auth_token_is_checked_by_the_function_the_host_provides() {
        let mut s = SessionServer::new(server_config());
        s.set_authenticator(Box::new(|name: &str, token: &[u8]| {
            name == "Sam" && token == b"ok"
        }));
        let with = |name: &str, token: &[u8]| Message::Hello {
            handshake: hs(),
            mod_hash: 0,
            name: name.into(),
            password: vec![],
            token: token.to_vec(),
        };
        assert_eq!(
            refusal(&s.handle(PeerId(1), with("Sam", b"bad"), 0)),
            Some(RefuseReason::AuthFailed)
        );
        assert_eq!(refusal(&s.handle(PeerId(1), with("Sam", b"ok"), 1)), None);
    }

    // ---- capacity, kicking, banning ---------------------------------------------------------

    #[test]
    fn a_full_server_refuses_and_a_freed_slot_is_reused_lowest_first() {
        let mut w = World::new(ServerConfig {
            max_players: 2,
            ..server_config()
        });
        let a = w.add(1, client_config("A"), LinkConfig::default());
        let b = w.add(2, client_config("B"), LinkConfig::default());
        w.run(SECOND);
        let c = w.add(3, client_config("C"), LinkConfig::default());
        w.run(SECOND);
        assert!(
            matches!(w.state(c), ClientState::Refused(RefuseReason::ServerFull)),
            "{:?}",
            w.state(c)
        );
        let leave: Vec<Message> = w.clients[a].1.disconnect();
        for m in leave {
            let out = w.server.handle(PeerId(1), m, w.now);
            w.deliver_from_server(out);
        }
        assert_eq!(w.server.connected(), 1);
        let d = w.add(4, client_config("D"), LinkConfig::default());
        w.run(SECOND);
        assert!(
            matches!(w.state(d), ClientState::Connected { slot: 0, .. }),
            "slot 0 was freed: {:?}",
            w.state(d)
        );
        assert!(matches!(w.state(b), ClientState::Connected { slot: 1, .. }));
    }

    #[test]
    fn a_kicked_player_is_told_why_and_loses_the_slot_but_may_return() {
        let mut w = World::new(server_config());
        let a = w.add(1, client_config("A"), LinkConfig::default());
        w.run(SECOND);
        let out = w.server.kick(0, "AFK for too long");
        w.deliver_from_server(out);
        w.run(SECOND);
        assert_eq!(
            w.state(a),
            &ClientState::Disconnected(DisconnectReason::Kicked("AFK for too long".into()))
        );
        assert_eq!(w.server.connected(), 0);
        assert!(w.server.take_events().iter().all(|_| true));
        assert!(w.server_events.contains(&ServerEvent::Left {
            peer: PeerId(1),
            slot: 0,
            reason: LeaveReason::Kicked
        }));
        let again = w.add(1, client_config("A"), LinkConfig::default());
        w.run(SECOND);
        assert!(
            matches!(w.state(again), ClientState::Connected { .. }),
            "a kick is not a ban"
        );
    }

    #[test]
    fn a_banned_peer_is_disconnected_and_refused_until_unbanned() {
        let mut w = World::new(server_config());
        let a = w.add(1, client_config("A"), LinkConfig::default());
        w.run(SECOND);
        let out = w.server.ban(PeerId(1));
        w.deliver_from_server(out);
        w.run(SECOND);
        assert_eq!(
            w.state(a),
            &ClientState::Disconnected(DisconnectReason::Banned)
        );
        let b = w.add(1, client_config("A2"), LinkConfig::default());
        w.run(SECOND);
        assert!(
            matches!(w.state(b), ClientState::Refused(RefuseReason::Banned)),
            "{:?}",
            w.state(b)
        );
        w.server.unban(PeerId(1));
        let c = w.add(1, client_config("A3"), LinkConfig::default());
        w.run(SECOND);
        assert!(matches!(w.state(c), ClientState::Connected { .. }));
    }

    // ---- staying alive and leaving ----------------------------------------------------------

    #[test]
    fn keepalives_hold_a_quiet_connection_open_for_a_minute() {
        let mut w = World::new(ServerConfig {
            idle_timeout_us: 10 * SECOND,
            keepalive_us: SECOND,
            ..server_config()
        });
        let mut cfg = client_config("A");
        cfg.idle_timeout_us = 10 * SECOND;
        cfg.keepalive_us = SECOND;
        let a = w.add(1, cfg, lossy());
        w.run(60 * SECOND);
        assert!(
            matches!(w.state(a), ClientState::Connected { .. }),
            "{:?}",
            w.state(a)
        );
        assert_eq!(w.server.connected(), 1);
    }

    #[test]
    fn a_silent_client_is_dropped_by_the_server_and_a_silent_server_by_the_client() {
        let mut s = SessionServer::new(ServerConfig {
            idle_timeout_us: 5 * SECOND,
            ..server_config()
        });
        s.handle(PeerId(1), hello_with("A", b""), 0);
        assert!(
            s.tick(4 * SECOND, 0)
                .iter()
                .all(|o| !matches!(o.message, Message::Disconnect(_)))
        );
        s.tick(6 * SECOND, 0);
        assert_eq!(s.connected(), 0);
        assert!(s.take_events().contains(&ServerEvent::Left {
            peer: PeerId(1),
            slot: 0,
            reason: LeaveReason::TimedOut
        }));

        let mut c = SessionClient::new(client_config("A"));
        c.start(0);
        c.handle(
            Message::Welcome {
                handshake: hs(),
                mod_hash: 0,
                slot: 0,
                tick_hz: 30,
                server_tick: 0,
            },
            100,
        );
        c.tick(5 * SECOND);
        assert!(matches!(c.state(), ClientState::Connected { .. }));
        c.tick(11 * SECOND);
        assert_eq!(c.state(), &ClientState::TimedOut);
        assert!(c.take_events().contains(&ClientEvent::TimedOut));
    }

    #[test]
    fn connecting_retries_with_backoff_and_gives_up() {
        let mut c = SessionClient::new(client_config("A"));
        let mut sent = c.start(0).len();
        let mut times = vec![0_u64];
        let mut now = 0;
        while now < 12 * SECOND && !matches!(c.state(), ClientState::TimedOut) {
            now += 50_000;
            let out = c.tick(now);
            if !out.is_empty() {
                sent += out.len();
                times.push(now);
            }
        }
        assert_eq!(c.state(), &ClientState::TimedOut);
        assert!(sent >= 4 && sent <= 12, "{sent} hellos");
        let gaps: Vec<u64> = times.windows(2).map(|p| p[1] - p[0]).collect();
        assert!(
            gaps.windows(2).all(|g| g[1] >= g[0]),
            "the gaps never shrink: {gaps:?}"
        );
        assert!(
            now >= 10 * SECOND && now < 11 * SECOND,
            "gave up after the connect timeout: {now}"
        );
    }

    #[test]
    fn leaving_politely_frees_the_slot_and_a_shutdown_tells_everyone() {
        let mut w = World::new(server_config());
        let a = w.add(1, client_config("A"), LinkConfig::default());
        let b = w.add(2, client_config("B"), LinkConfig::default());
        w.run(SECOND);
        let bye = w.clients[a].1.disconnect();
        assert!(matches!(
            bye[0],
            Message::Disconnect(DisconnectReason::Quit)
        ));
        assert_eq!(
            w.state(a),
            &ClientState::Disconnected(DisconnectReason::Quit)
        );
        let out = w.server.handle(PeerId(1), bye[0].clone(), w.now);
        assert!(out.is_empty());
        assert!(w.server.take_events().contains(&ServerEvent::Left {
            peer: PeerId(1),
            slot: 0,
            reason: LeaveReason::Quit
        }));
        let out = w.server.shutdown();
        w.deliver_from_server(out);
        w.run(SECOND);
        assert_eq!(
            w.state(b),
            &ClientState::Disconnected(DisconnectReason::ServerShutdown)
        );
        assert_eq!(w.server.connected(), 0);
    }

    #[test]
    fn messages_from_peers_that_never_said_hello_are_ignored_and_create_nothing() {
        let mut s = SessionServer::new(server_config());
        for m in [
            Message::Keepalive(1),
            Message::Disconnect(DisconnectReason::Quit),
            Message::Welcome {
                handshake: hs(),
                mod_hash: 0,
                slot: 0,
                tick_hz: 1,
                server_tick: 0,
            },
            Message::Refused(RefuseReason::Banned),
        ] {
            assert!(s.handle(PeerId(77), m, 0).is_empty());
        }
        assert_eq!((s.connected(), s.tracked_failures()), (0, 0));
        assert!(s.take_events().is_empty());
    }

    #[test]
    fn a_kick_reason_is_cut_to_the_limit_at_a_character_boundary() {
        let mut s = SessionServer::new(server_config());
        s.handle(PeerId(1), hello_with("A", b""), 0);
        let long = "é".repeat(200);
        let out = s.kick(0, &long);
        let Message::Disconnect(DisconnectReason::Kicked(text)) = &out[0].message else {
            panic!("{out:?}")
        };
        assert!(
            text.len() <= MAX_REASON && text.chars().all(|c| c == 'é') && !text.is_empty(),
            "{}",
            text.len()
        );
        assert!(
            decode(&encode(&out[0].message)).is_ok(),
            "and it still fits the wire"
        );
    }

    #[test]
    fn the_constant_time_comparison_gets_the_answers_right() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(
            !constant_time_eq(b"a", b"a "),
            "a password with a zero byte added is not the password"
        );
        assert!(!constant_time_eq(b"", b" "));
        assert!(constant_time_eq(b"", b""));
    }
}
