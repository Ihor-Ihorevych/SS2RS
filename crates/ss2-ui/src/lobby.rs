//! The **co-op lobby and session screens** (roadmap T10-018, `docs/LOBBY_UI.md`): host or join a game, and wait for the others in the lobby.
//!
//! A state machine like the menu: it takes [`LobbyInput`] and data updates ([`Lobby::set_servers`], [`Lobby::set_players`],
//! [`Lobby::set_connection_state`], ...), gives back [`LobbyCommand`]s for the app, and draws into a [`DrawList`] in canvas units with the placeholder
//! theme. It knows nothing of the network: the app runs the discovery scan, the session and the join, and tells the screens what happened with the
//! plain data types of this module (`docs/LOBBY_UI.md` shows how to map the `ss2-net` types to them). `ss2-ui` does not depend on `ss2-net`.
//!
//! ```text
//!            Back <-- Start --Host--> HostSetup --Start--> Connecting --Connected--> Lobby --Leave--> Start
//!                        |               |  ^                 |  |  ^                  |
//!                       Join            Back                Cancel  Failed          Failed (kicked, host left, link lost)
//!                        v               v                    |  v  |                  v
//!                       Join (LAN list | direct address | invite code) --Join--> Connecting    Error --Back--> where it came from
//!                                                                                       `-- Try again --> Connecting
//! ```
//!
//! Strings that vary (the texts of fields, rows, error messages, the sorted list of servers) are built at run time, so drawing, `items` and a few
//! accessors allocate short `String`s and `Vec`s; nothing is kept between frames. The text field is [`crate::data_wizard::TextField`].

use ss2_core::input::NavInput;

use crate::data_wizard::TextField;
use crate::draw::DrawList;
use crate::geometry::{CANVAS_W, Rect, Vec2};
use crate::menu::UiInput;

mod draw;
#[cfg(test)]
mod tests;
mod validate;

pub use validate::{
    AddressError, DEFAULT_PORT, DirectAddress, HostSpec, INVITE_LEN_V4, INVITE_LEN_V6, InviteError,
    MAX_ADDRESS_CHARS, MAX_INVITE_CHARS, normalize_invite, parse_address,
};

/// The longest server name, in bytes (the discovery protocol's limit).
pub const MAX_SERVER_NAME_BYTES: usize = 32;
/// The most characters the server name field takes.
pub const MAX_SERVER_NAME_CHARS: usize = 32;
/// The most characters a password field takes (at most 128 bytes, the session protocol's limit).
pub const MAX_PASSWORD_CHARS: usize = 32;
/// How many server rows are drawn at once; more are reached by moving the focus down.
pub const VISIBLE_SERVERS: usize = 6;
/// How many player rows are drawn at once.
pub const VISIBLE_PLAYERS: usize = 8;
/// How many character rows are drawn at once.
pub const VISIBLE_CHARACTERS: usize = 6;

// ---------------------------------------------------------------------------------------------------------------------------------
// Data the app gives the screens

/// Who a player is in the lobby: any number the app keeps stable while the player is in the session (the slot, or the `PeerId`'s number).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u64);

/// Which character a player plays: any number the app chose for it (an index into the list given to [`Lobby::set_characters`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterId(pub u32);

/// Whether a server found by the scan can be joined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerStatus {
    /// There is a free slot and the builds can talk.
    Open,
    /// Every slot is taken.
    Full,
    /// The builds cannot talk (`ss2_net::protocol::Compat` is not `Compatible`); the text says why in a few words ("newer game version").
    Incompatible(String),
}

/// One row of the LAN browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerRow {
    /// An opaque key the screens hand back in [`JoinTarget::Server`]: the address the server answered from (`SocketAddr::to_string`).
    pub id: String,
    /// The name in the list.
    pub name: String,
    /// The level.
    pub map: String,
    /// Players now.
    pub players: u8,
    /// Player slots.
    pub max_players: u8,
    /// Round-trip time in milliseconds, if known.
    pub ping_ms: Option<u32>,
    /// Whether joining needs a password.
    pub locked: bool,
    /// Whether it can be joined.
    pub status: ServerStatus,
}

/// How the browser orders the servers (a stable sort of what the app gave).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ServerSort {
    /// By ping, fastest first; a server with no ping last.
    #[default]
    Ping,
    /// By name, A to Z, ignoring case.
    Name,
    /// By players, the fullest first.
    Players,
}

impl ServerSort {
    fn next(self) -> Self {
        match self {
            Self::Ping => Self::Name,
            Self::Name => Self::Players,
            Self::Players => Self::Ping,
        }
    }

    /// The word on the sort button.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ping => "Ping",
            Self::Name => "Name",
            Self::Players => "Players",
        }
    }
}

/// One player in the lobby.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerRow {
    /// Who.
    pub id: PlayerId,
    /// The name.
    pub name: String,
    /// The character, once chosen.
    pub character: Option<CharacterId>,
    /// Whether the player is ready.
    pub ready: bool,
    /// Round-trip time in milliseconds, if known.
    pub ping_ms: Option<u32>,
    /// Whether this player is the host.
    pub host: bool,
    /// Whether this player is the one at this screen.
    pub local: bool,
}

/// A character to choose from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterChoice {
    /// The number the app uses for it.
    pub id: CharacterId,
    /// The name shown.
    pub name: String,
}

/// A level the host can choose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelChoice {
    /// The key the app uses for it (a map path).
    pub id: String,
    /// The name shown.
    pub name: String,
}

/// What the host chose on the setup screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSettings {
    /// The name in the server list (trimmed, 1 to [`MAX_SERVER_NAME_BYTES`] bytes).
    pub name: String,
    /// The player slots.
    pub max_players: u8,
    /// The session password, if there is one.
    pub password: Option<String>,
    /// The key of the level ([`LevelChoice::id`]).
    pub level: String,
}

/// Where a join goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinTarget {
    /// A server of the LAN list: the [`ServerRow::id`] it was given.
    Server(String),
    /// An address typed by the player, already read.
    Direct(DirectAddress),
    /// An invite code, in its written form (capitals, groups of four). The checksum is for `ss2_net::discovery::decode_invite` to check.
    Invite(String),
}

/// A join: where, and the password typed (none when the field is empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinRequest {
    /// Where.
    pub target: JoinTarget,
    /// The password, if one was typed.
    pub password: Option<String>,
}

/// What the app must do after an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyCommand {
    /// Start a server with these settings, and answer with [`Lobby::set_connection_state`] (`Connected` once the host is in its own lobby).
    Host(HostSettings),
    /// Connect, and answer with [`Lobby::set_connection_state`].
    Join(JoinRequest),
    /// Scan again, feed the answers to [`Lobby::set_servers`], and tell [`Lobby::set_scanning`].
    RefreshServers,
    /// Stop the connection attempt (the player pressed Cancel, or it timed out).
    Cancel,
    /// The local player is ready, or not any more.
    SetReady(bool),
    /// The local player chose a character. (Changing character clears the ready flag: the session must clear it too.)
    SelectCharacter(CharacterId),
    /// The host removes a player.
    Kick(PlayerId),
    /// The host starts the game.
    StartGame,
    /// The player leaves the lobby (a host's leaving closes the game for the others).
    Leave,
    /// The player leaves these screens for the main menu.
    Back,
}

/// How the connection stands, as the app tells it (`ss2_net::session::ClientState` and `join::JoinPhase` map to it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// Waiting for the other side (the hello is sent, or the server is starting).
    Connecting,
    /// Past the handshake and loading: what it is doing, and how far (0 to 100) if that is known.
    Loading {
        /// The step, in a few words ("Loading the level", "Receiving the game").
        what: String,
        /// How far, in percent.
        percent: Option<u8>,
    },
    /// In: the lobby shows.
    Connected,
    /// It did not work, or it ended.
    Failed(Failure),
}

/// Why a connection failed or ended (`ss2_net::session::RefuseReason`, `DisconnectReason`, `reconnect::Stopped` map to it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// No answer in time.
    TimedOut,
    /// The address cannot be reached.
    Unreachable,
    /// The host name did not resolve.
    HostNotFound,
    /// The invite code is not one the game made (the checksum failed).
    InvalidInvite,
    /// The server wants another password.
    BadPassword,
    /// No free slot.
    ServerFull,
    /// The player is banned.
    Banned,
    /// Another player has the name.
    NameTaken,
    /// The name is not allowed.
    InvalidName,
    /// The builds cannot talk: the explanation (`Compat::explanation`).
    Incompatible(String),
    /// Too many wrong passwords: try again in this many seconds.
    RetryLater {
        /// The wait, in seconds.
        seconds: u32,
    },
    /// The host removed the player, with a reason (may be empty).
    Kicked(String),
    /// The host left or closed the game.
    HostLeft,
    /// The connection dropped.
    ConnectionLost,
    /// Anything else, in a sentence.
    Other(String),
}

impl Failure {
    /// The heading of the error screen.
    pub fn title(&self) -> &'static str {
        match self {
            Self::TimedOut => "No answer",
            Self::Unreachable => "Could not reach the host",
            Self::HostNotFound => "Host not found",
            Self::InvalidInvite => "Invalid invite code",
            Self::BadPassword => "Wrong password",
            Self::ServerFull => "The game is full",
            Self::Banned => "You are banned",
            Self::NameTaken => "Name already in use",
            Self::InvalidName => "Name not accepted",
            Self::Incompatible(_) => "Different game version",
            Self::RetryLater { .. } => "Too many tries",
            Self::Kicked(_) => "Removed from the game",
            Self::HostLeft => "The host left",
            Self::ConnectionLost => "Connection lost",
            Self::Other(_) => "Something went wrong",
        }
    }

    /// What happened, in a sentence.
    pub fn message(&self) -> String {
        match self {
            Self::TimedOut => "The host did not answer in time.".to_owned(),
            Self::Unreachable => "Nothing could be reached at that address.".to_owned(),
            Self::HostNotFound => "That host name could not be found.".to_owned(),
            Self::InvalidInvite => "That code is not one the game made.".to_owned(),
            Self::BadPassword => "The password was not accepted.".to_owned(),
            Self::ServerFull => "There is no free slot in that game.".to_owned(),
            Self::Banned => "The host has banned you from this game.".to_owned(),
            Self::NameTaken => "Another player in that game has your name.".to_owned(),
            Self::InvalidName => "The game does not accept your player name.".to_owned(),
            Self::Incompatible(why) => why.clone(),
            Self::RetryLater { seconds } => {
                format!("Too many wrong passwords: wait {seconds} seconds.")
            }
            Self::Kicked(reason) if reason.is_empty() => {
                "The host removed you from the game.".to_owned()
            }
            Self::Kicked(reason) => format!("The host removed you from the game: {reason}"),
            Self::HostLeft => "The host closed the game.".to_owned(),
            Self::ConnectionLost => "The connection to the host was lost.".to_owned(),
            Self::Other(text) => text.clone(),
        }
    }

    /// What the player can do about it.
    pub fn hint(&self) -> &'static str {
        match self {
            Self::TimedOut => {
                "Check the address, and that the host is running the same version of the game. Then try again."
            }
            Self::Unreachable => {
                "Check the address and the port, and that the host's firewall or router lets the game in."
            }
            Self::HostNotFound => "Check the spelling, or type the host's IP address instead.",
            Self::InvalidInvite => "Ask the host for the code again, or use the host's address.",
            Self::BadPassword => "Go back, type the password again and join.",
            Self::ServerFull => "Try again in a moment, or ask the host for a free slot.",
            Self::Banned => "Ask the host to lift the ban.",
            Self::NameTaken | Self::InvalidName => {
                "Change your player name in the options, then join again."
            }
            Self::Incompatible(_) => {
                "Everyone must run the same version of the game: update it, or ask the host to."
            }
            Self::RetryLater { .. } => "Wait a little, then join again.",
            Self::Kicked(_) => "You can join again if the host lets you.",
            Self::HostLeft => "Host a game of your own, or join another one.",
            Self::ConnectionLost => "Check your network, then try again.",
            Self::Other(_) => "Go back and try again.",
        }
    }

    /// Whether sending the same request again can work (so the error screen offers "Try again").
    pub fn can_retry(&self) -> bool {
        matches!(
            self,
            Self::TimedOut
                | Self::Unreachable
                | Self::ServerFull
                | Self::ConnectionLost
                | Self::Other(_)
        )
    }
}

/// Settings of the screens that the app may want to change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LobbyConfig {
    /// The name the setup screen starts with.
    pub default_server_name: String,
    /// The fewest player slots a host can choose.
    pub min_players: u8,
    /// The most player slots a host can choose.
    pub max_players_limit: u8,
    /// The slots the setup screen starts with.
    pub default_max_players: u8,
    /// How long, in milliseconds, [`Lobby::update`] waits for an answer before it gives up ([`Failure::TimedOut`]).
    pub connect_timeout_ms: u32,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            default_server_name: "SS2RS game".to_owned(),
            min_players: 2,
            max_players_limit: 16,
            default_max_players: 4,
            connect_timeout_ms: 10_000,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Screens and items

/// Which screen shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyView {
    /// Host, Join, Back.
    Start,
    /// The host's choices.
    HostSetup,
    /// Joining: the three ways in.
    Join,
    /// Waiting for the server.
    Connecting,
    /// Players, characters, ready.
    Lobby,
    /// Something failed.
    Error,
}

/// The three ways to join.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JoinRoute {
    /// The list of games found on the local network.
    #[default]
    Lan,
    /// An address typed by the player.
    Direct,
    /// An invite code.
    Invite,
}

impl JoinRoute {
    const ALL: [JoinRoute; 3] = [JoinRoute::Lan, JoinRoute::Direct, JoinRoute::Invite];

    /// The tab's label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lan => "LAN games",
            Self::Direct => "Direct address",
            Self::Invite => "Invite code",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Lan => 0,
            Self::Direct => 1,
            Self::Invite => 2,
        }
    }
}

/// Something on a screen that can be focused, pointed at and activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemId {
    /// Start: host a game.
    StartHost,
    /// Start: join a game.
    StartJoin,
    /// Start: leave for the menu.
    StartBack,
    /// Setup: the server name field.
    HostName,
    /// Setup: the player slots (left and right change it).
    HostMaxPlayers,
    /// Setup: the password switch.
    HostPasswordOn,
    /// Setup: the password field (disabled while the switch is off).
    HostPassword,
    /// Setup: the level (left and right change it).
    HostLevel,
    /// Setup: start hosting.
    HostStart,
    /// Setup: back to the start screen.
    HostBack,
    /// Join: a tab.
    JoinTab(JoinRoute),
    /// Join (LAN): scan again.
    JoinRefresh,
    /// Join (LAN): the sort button.
    JoinSort,
    /// Join (LAN): a server, by its place in the sorted list.
    JoinServer(usize),
    /// Join (direct): the address field.
    JoinAddress,
    /// Join (invite): the code field.
    JoinCode,
    /// Join: the password field.
    JoinPassword,
    /// Join (direct, invite): connect.
    JoinConnect,
    /// Join: back to the start screen.
    JoinBack,
    /// Connecting: cancel.
    ConnectCancel,
    /// Lobby: a player (the host can remove one).
    LobbyPlayer(PlayerId),
    /// Lobby: a character.
    LobbyCharacter(CharacterId),
    /// Lobby: the ready button.
    LobbyReady,
    /// Lobby: the start button (the host's).
    LobbyStart,
    /// Lobby: leave.
    LobbyLeave,
    /// Error: send the request again.
    ErrorRetry,
    /// Error: go back.
    ErrorBack,
}

/// One focusable thing, where it is and whether it can be used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// Which.
    pub id: ItemId,
    /// Where it is on the canvas; `None` while it is scrolled out of its list.
    pub rect: Option<Rect>,
    /// Whether it can be focused and used. A disabled item is drawn dimmed and the focus skips it.
    pub enabled: bool,
}

/// An input event. The navigation and the pointer are the menu's [`UiInput`]; typing needs these as well.
#[derive(Clone, Debug, PartialEq)]
pub enum LobbyInput {
    /// Navigation or the pointer.
    Ui(UiInput),
    /// What the keyboard typed, or a paste.
    Text(String),
    /// The backspace key.
    Backspace,
    /// The delete key.
    Delete,
    /// The home key.
    Home,
    /// The end key.
    End,
}

impl From<UiInput> for LobbyInput {
    fn from(input: UiInput) -> Self {
        Self::Ui(input)
    }
}

// Layout (canvas units).
const BTN_W: f32 = 640.0;
const BTN_H: f32 = 72.0;
const BTN_STEP: f32 = 88.0;
const START_TOP: f32 = 380.0;
const SETUP_X: f32 = 360.0;
const SETUP_W: f32 = 1200.0;
const SETUP_TOP: f32 = 230.0;
const SETUP_STEP: f32 = 86.0;
const JOIN_X: f32 = 210.0;
const JOIN_W: f32 = 1500.0;
const TAB_Y: f32 = 170.0;
const TAB_W: f32 = 480.0;
const TAB_STEP: f32 = 510.0;
const TAB_H: f32 = 64.0;
const SERVER_Y: f32 = 372.0;
const SERVER_W: f32 = 1480.0;
const LIST_STEP: f32 = 72.0;
const ROW_H: f32 = 64.0;
const LAN_PASSWORD_Y: f32 = 840.0;
const LAN_BACK_Y: f32 = 930.0;
const PLAYERS_X: f32 = 100.0;
const PLAYERS_W: f32 = 1000.0;
const CHARS_X: f32 = 1160.0;
const CHARS_W: f32 = 660.0;
const LOBBY_LIST_Y: f32 = 210.0;
const LOBBY_BUTTON_Y: f32 = 900.0;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn centered_button(y: f32) -> Rect {
    rect((CANVAS_W - BTN_W) / 2.0, y, BTN_W, BTN_H)
}

/// Keep `index` inside the window that starts at `scroll` and shows `visible` rows of `len`.
fn scroll_to(scroll: &mut usize, index: usize, visible: usize, len: usize) {
    if index < *scroll {
        *scroll = index;
    } else if index >= *scroll + visible {
        *scroll = index + 1 - visible;
    }
    *scroll = (*scroll).min(len.saturating_sub(visible));
}

/// The host's form.
#[derive(Clone, Debug)]
struct HostForm {
    name: TextField,
    max_players: u8,
    password_on: bool,
    password: TextField,
    level: usize,
}

/// The join form.
#[derive(Clone, Debug)]
struct JoinForm {
    route: JoinRoute,
    address: TextField,
    code: TextField,
    password: TextField,
    sort: ServerSort,
    scroll: usize,
}

/// The last request, to send again from the error screen.
#[derive(Clone, Debug)]
enum Request {
    Host(HostSettings),
    Join(JoinRequest),
}

/// The co-op lobby screens. See the module documentation.
#[derive(Clone, Debug)]
pub struct Lobby {
    cfg: LobbyConfig,
    view: LobbyView,
    focus: ItemId,
    /// Where the focus was in the item list, to find a neighbour when its item goes away.
    focus_pos: usize,
    pressed: Option<ItemId>,
    host: HostForm,
    join: JoinForm,
    servers: Vec<ServerRow>,
    scanning: bool,
    levels: Vec<LevelChoice>,
    characters: Vec<CharacterChoice>,
    players: Vec<PlayerRow>,
    player_scroll: usize,
    char_scroll: usize,
    /// The character the local player chose and the session has not confirmed yet.
    pending_character: Option<CharacterId>,
    notice: Option<String>,
    kick_armed: Option<PlayerId>,
    request: Option<Request>,
    /// The screen Cancel and the error screen's Back return to.
    origin: LobbyView,
    connecting_label: String,
    stage: Option<(String, Option<u8>)>,
    elapsed_ms: u32,
    clock_ms: u32,
    failure: Option<Failure>,
    error_return: LobbyView,
}

impl Lobby {
    /// The screens at the start screen, with no servers, players, levels or characters yet.
    pub fn new(cfg: LobbyConfig) -> Self {
        let mut name = TextField::new(MAX_SERVER_NAME_CHARS);
        name.set(&cfg.default_server_name);
        let max_players = cfg
            .default_max_players
            .clamp(cfg.min_players, cfg.max_players_limit.max(cfg.min_players));
        Self {
            view: LobbyView::Start,
            focus: ItemId::StartHost,
            focus_pos: 0,
            pressed: None,
            host: HostForm {
                name,
                max_players,
                password_on: false,
                password: TextField::new(MAX_PASSWORD_CHARS),
                level: 0,
            },
            join: JoinForm {
                route: JoinRoute::Lan,
                address: TextField::new(MAX_ADDRESS_CHARS),
                code: TextField::new(MAX_INVITE_CHARS),
                password: TextField::new(MAX_PASSWORD_CHARS),
                sort: ServerSort::default(),
                scroll: 0,
            },
            servers: Vec::new(),
            scanning: false,
            levels: Vec::new(),
            characters: Vec::new(),
            players: Vec::new(),
            player_scroll: 0,
            char_scroll: 0,
            pending_character: None,
            notice: None,
            kick_armed: None,
            request: None,
            origin: LobbyView::Start,
            connecting_label: String::new(),
            stage: None,
            elapsed_ms: 0,
            clock_ms: 0,
            failure: None,
            error_return: LobbyView::Start,
            cfg,
        }
    }

    // -----------------------------------------------------------------------------------------------------------------------------
    // What the screens know

    /// The screen showing.
    pub fn view(&self) -> LobbyView {
        self.view
    }

    /// The join tab chosen.
    pub fn route(&self) -> JoinRoute {
        self.join.route
    }

    /// The focused item.
    pub fn focus(&self) -> ItemId {
        self.focus
    }

    /// The sort of the server list.
    pub fn sort(&self) -> ServerSort {
        self.join.sort
    }

    /// The servers as the app gave them (not sorted).
    pub fn servers(&self) -> &[ServerRow] {
        &self.servers
    }

    /// The servers in the order shown.
    pub fn sorted_servers(&self) -> Vec<&ServerRow> {
        let mut rows: Vec<&ServerRow> = self.servers.iter().collect();
        match self.join.sort {
            ServerSort::Ping => rows.sort_by_key(|s| (s.ping_ms.is_none(), s.ping_ms)),
            ServerSort::Name => rows.sort_by_key(|s| s.name.to_lowercase()),
            ServerSort::Players => rows.sort_by_key(|s| std::cmp::Reverse(s.players)),
        }
        rows
    }

    /// Whether a scan is running.
    pub fn scanning(&self) -> bool {
        self.scanning
    }

    /// The players, with the local player's choices as the screen shows them (a character or ready flag the session has not confirmed yet included).
    pub fn players(&self) -> Vec<PlayerRow> {
        self.players
            .iter()
            .map(|p| {
                let mut p = p.clone();
                if p.local && self.pending_character.is_some() {
                    p.character = self.pending_character;
                }
                p
            })
            .collect()
    }

    /// The characters to choose from.
    pub fn characters(&self) -> &[CharacterChoice] {
        &self.characters
    }

    /// The levels to choose from.
    pub fn levels(&self) -> &[LevelChoice] {
        &self.levels
    }

    /// The failure the error screen shows.
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// The line under the lobby lists (a character that was taken), if there is one. A kick that waits for its confirmation is [`Lobby::kick_armed`].
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// The player the host has pressed once to remove (a second press confirms).
    pub fn kick_armed(&self) -> Option<PlayerId> {
        self.kick_armed
    }

    /// The first server row shown (the list scrolls with the focus).
    pub fn server_scroll(&self) -> usize {
        self.join.scroll
    }

    /// The first player row shown.
    pub fn player_scroll(&self) -> usize {
        self.player_scroll
    }

    /// The first character row shown.
    pub fn character_scroll(&self) -> usize {
        self.char_scroll
    }

    /// The text of the field that `id` names, if it is one.
    pub fn field_text(&self, id: ItemId) -> Option<&str> {
        self.field(id).map(TextField::text)
    }

    fn field(&self, id: ItemId) -> Option<&TextField> {
        match id {
            ItemId::HostName => Some(&self.host.name),
            ItemId::HostPassword => Some(&self.host.password),
            ItemId::JoinAddress => Some(&self.join.address),
            ItemId::JoinCode => Some(&self.join.code),
            ItemId::JoinPassword => Some(&self.join.password),
            _ => None,
        }
    }

    fn field_mut(&mut self, id: ItemId) -> Option<&mut TextField> {
        match id {
            ItemId::HostName => Some(&mut self.host.name),
            ItemId::HostPassword => Some(&mut self.host.password),
            ItemId::JoinAddress => Some(&mut self.join.address),
            ItemId::JoinCode => Some(&mut self.join.code),
            ItemId::JoinPassword => Some(&mut self.join.password),
            _ => None,
        }
    }

    /// The slots chosen on the setup screen.
    pub fn host_max_players(&self) -> u8 {
        self.host.max_players
    }

    /// Whether the setup screen's password switch is on.
    pub fn host_password_on(&self) -> bool {
        self.host.password_on
    }

    /// The index of the level chosen on the setup screen (into [`Lobby::levels`]).
    pub fn host_level(&self) -> usize {
        self.host.level
    }

    /// What the setup screen would start now, or what is wrong with it (a sentence).
    pub fn host_settings(&self) -> Result<HostSettings, &'static str> {
        let name = self.host.name.text().trim();
        if name.is_empty() {
            return Err("Give the game a name");
        }
        if name.len() > MAX_SERVER_NAME_BYTES {
            return Err("The name is too long: 32 letters at most");
        }
        let Some(level) = self.levels.get(self.host.level) else {
            return Err("There are no levels to choose from");
        };
        if self.host.password_on && self.host.password.is_empty() {
            return Err("Type a password, or turn the password off");
        }
        Ok(HostSettings {
            name: name.to_owned(),
            max_players: self.host.max_players,
            password: self
                .host
                .password_on
                .then(|| self.host.password.text().to_owned()),
            level: level.id.clone(),
        })
    }

    /// What the join screen would send now (for the direct and invite tabs; the LAN tab sends the server picked), or what is wrong with the text.
    pub fn join_request(&self) -> Result<JoinRequest, String> {
        let password =
            (!self.join.password.is_empty()).then(|| self.join.password.text().to_owned());
        let target = match self.join.route {
            JoinRoute::Lan => return Err("Pick a game from the list".to_owned()),
            JoinRoute::Direct => JoinTarget::Direct(
                parse_address(self.join.address.text()).map_err(|e| e.message().to_owned())?,
            ),
            JoinRoute::Invite => JoinTarget::Invite(
                normalize_invite(self.join.code.text()).map_err(|e| e.message())?,
            ),
        };
        Ok(JoinRequest { target, password })
    }

    /// Why the text of the direct or invite tab cannot be sent yet; `None` when it can, and on the LAN tab.
    pub fn join_problem(&self) -> Option<String> {
        match self.join.route {
            JoinRoute::Lan => None,
            _ => self.join_request().err(),
        }
    }

    /// The local player's row, if the session has told of it.
    fn local(&self) -> Option<&PlayerRow> {
        self.players.iter().find(|p| p.local)
    }

    /// Whether the local player is the host.
    pub fn is_host(&self) -> bool {
        self.local().is_some_and(|p| p.host)
    }

    /// The character the local player has chosen, as the screen shows it.
    pub fn local_character(&self) -> Option<CharacterId> {
        self.pending_character
            .or_else(|| self.local().and_then(|p| p.character))
    }

    /// The other player who has this character, if any.
    pub fn taken_by(&self, character: CharacterId) -> Option<&PlayerRow> {
        self.players
            .iter()
            .find(|p| !p.local && p.character == Some(character))
    }

    /// Whether the start button works: the local player is the host (so there is at least one player: the host) and every player is ready.
    pub fn can_start(&self) -> bool {
        self.is_host() && self.players.iter().all(|p| p.ready)
    }

    fn can_ready(&self) -> bool {
        self.local().is_some() && (self.characters.is_empty() || self.local_character().is_some())
    }

    // -----------------------------------------------------------------------------------------------------------------------------
    // Data from the app

    /// Replace the LAN list. The focus stays on the same server when it is still there.
    pub fn set_servers(&mut self, servers: Vec<ServerRow>) {
        let keep = self.focused_server_id();
        self.servers = servers;
        self.refocus_server(keep);
        self.settle();
    }

    /// Tell whether a scan is running (the list says "Searching" while it is, and "No games found" when it ends with an empty list).
    pub fn set_scanning(&mut self, scanning: bool) {
        self.scanning = scanning;
    }

    /// Change the order of the server list.
    pub fn set_sort(&mut self, sort: ServerSort) {
        let keep = self.focused_server_id();
        self.join.sort = sort;
        self.refocus_server(keep);
        self.settle();
    }

    fn focused_server_id(&self) -> Option<String> {
        match self.focus {
            ItemId::JoinServer(i) => self.sorted_servers().get(i).map(|s| s.id.clone()),
            _ => None,
        }
    }

    fn refocus_server(&mut self, keep: Option<String>) {
        let Some(id) = keep else { return };
        if let Some(i) = self.sorted_servers().iter().position(|s| s.id == id) {
            self.focus = ItemId::JoinServer(i);
        }
    }

    /// Replace the levels the host can choose from. The level chosen stays when it is still in the list (by its key), else the first one is chosen.
    pub fn set_levels(&mut self, levels: Vec<LevelChoice>) {
        let keep = self.levels.get(self.host.level).map(|l| l.id.clone());
        self.levels = levels;
        self.host.level = keep
            .and_then(|id| self.levels.iter().position(|l| l.id == id))
            .unwrap_or(0);
        self.settle();
    }

    /// Replace the characters to choose from.
    pub fn set_characters(&mut self, characters: Vec<CharacterChoice>) {
        self.characters = characters;
        if let Some(c) = self.pending_character
            && !self.characters.iter().any(|x| x.id == c)
        {
            self.pending_character = None;
        }
        self.settle();
    }

    /// Replace the players: what the session says is the truth. A character the local player chose is shown until the session confirms it; when
    /// another player turns out to have it, the choice is dropped and a line says so. When the host is gone from the list while the screen is in
    /// the lobby, it becomes the "host left" error.
    pub fn set_players(&mut self, players: Vec<PlayerRow>) {
        let had_host = self.players.iter().any(|p| p.host);
        self.players = players;
        if let Some(c) = self.pending_character {
            if self.local().is_some_and(|p| p.character == Some(c)) {
                self.pending_character = None;
            } else if let Some(other) = self.taken_by(c) {
                let who = other.name.clone();
                let what = self
                    .characters
                    .iter()
                    .find(|x| x.id == c)
                    .map_or_else(|| "That character".to_owned(), |x| x.name.clone());
                self.notice = Some(format!("{what} was taken by {who}: choose another"));
                self.pending_character = None;
            }
        }
        if self.view == LobbyView::Lobby
            && had_host
            && !self.players.is_empty()
            && !self.players.iter().any(|p| p.host)
        {
            self.fail(Failure::HostLeft);
            return;
        }
        self.settle();
    }

    /// Tell how the connection stands. Only the connecting screen and the lobby listen: an answer that arrives after the player cancelled is ignored.
    pub fn set_connection_state(&mut self, state: ConnectionState) {
        match (self.view, state) {
            (LobbyView::Connecting, ConnectionState::Loading { what, percent }) => {
                self.stage = Some((what, percent.map(|p| p.min(100))));
                self.elapsed_ms = 0;
            }
            (LobbyView::Connecting, ConnectionState::Connected) => {
                self.notice = None;
                self.go(LobbyView::Lobby);
            }
            (LobbyView::Connecting | LobbyView::Lobby, ConnectionState::Failed(f)) => self.fail(f),
            _ => {}
        }
    }

    /// Time passes (`dt_ms` milliseconds since the last call): the dots on the connecting screen move, and when no answer came in
    /// [`LobbyConfig::connect_timeout_ms`] the screen gives up: it shows [`Failure::TimedOut`] and gives back [`LobbyCommand::Cancel`].
    pub fn update(&mut self, dt_ms: u32) -> Option<LobbyCommand> {
        self.clock_ms = self.clock_ms.wrapping_add(dt_ms);
        if self.view != LobbyView::Connecting {
            return None;
        }
        self.elapsed_ms = self.elapsed_ms.saturating_add(dt_ms);
        if self.elapsed_ms >= self.cfg.connect_timeout_ms {
            self.fail(Failure::TimedOut);
            return Some(LobbyCommand::Cancel);
        }
        None
    }

    // -----------------------------------------------------------------------------------------------------------------------------
    // Items

    /// Every focusable item of the screen showing, in focus order.
    pub fn items(&self) -> Vec<Item> {
        let item = |id, r: Rect, enabled| Item {
            id,
            rect: Some(r),
            enabled,
        };
        match self.view {
            LobbyView::Start => vec![
                item(ItemId::StartHost, centered_button(START_TOP), true),
                item(
                    ItemId::StartJoin,
                    centered_button(START_TOP + BTN_STEP),
                    true,
                ),
                item(
                    ItemId::StartBack,
                    centered_button(START_TOP + 2.0 * BTN_STEP),
                    true,
                ),
            ],
            LobbyView::HostSetup => {
                let row =
                    |k: usize| rect(SETUP_X, SETUP_TOP + k as f32 * SETUP_STEP, SETUP_W, BTN_H);
                vec![
                    item(ItemId::HostName, row(0), true),
                    item(ItemId::HostMaxPlayers, row(1), true),
                    item(ItemId::HostPasswordOn, row(2), true),
                    item(ItemId::HostPassword, row(3), self.host.password_on),
                    item(ItemId::HostLevel, row(4), !self.levels.is_empty()),
                    item(ItemId::HostStart, row(5), self.host_settings().is_ok()),
                    item(ItemId::HostBack, row(6), true),
                ]
            }
            LobbyView::Join => self.join_items(),
            LobbyView::Connecting => {
                vec![item(ItemId::ConnectCancel, centered_button(720.0), true)]
            }
            LobbyView::Lobby => self.lobby_items(),
            LobbyView::Error => {
                let mut v = Vec::new();
                let mut y = 760.0;
                if self.failure.as_ref().is_some_and(Failure::can_retry) && self.request.is_some() {
                    v.push(item(ItemId::ErrorRetry, centered_button(y), true));
                    y += BTN_STEP;
                }
                v.push(item(ItemId::ErrorBack, centered_button(y), true));
                v
            }
        }
    }

    fn join_items(&self) -> Vec<Item> {
        let mut v = Vec::new();
        for route in JoinRoute::ALL {
            v.push(Item {
                id: ItemId::JoinTab(route),
                rect: Some(rect(
                    JOIN_X + route.index() as f32 * TAB_STEP,
                    TAB_Y,
                    TAB_W,
                    TAB_H,
                )),
                enabled: true,
            });
        }
        let item = |id, r: Rect, enabled| Item {
            id,
            rect: Some(r),
            enabled,
        };
        match self.join.route {
            JoinRoute::Lan => {
                v.push(item(
                    ItemId::JoinRefresh,
                    rect(JOIN_X, 250.0, 300.0, 60.0),
                    true,
                ));
                v.push(item(
                    ItemId::JoinSort,
                    rect(JOIN_X + 330.0, 250.0, 420.0, 60.0),
                    true,
                ));
                let rows = self.sorted_servers();
                for (i, s) in rows.iter().enumerate() {
                    let slot = i
                        .checked_sub(self.join.scroll)
                        .filter(|s| *s < VISIBLE_SERVERS);
                    v.push(Item {
                        id: ItemId::JoinServer(i),
                        rect: slot.map(|k| {
                            rect(JOIN_X, SERVER_Y + k as f32 * LIST_STEP, SERVER_W, ROW_H)
                        }),
                        enabled: s.status == ServerStatus::Open,
                    });
                }
                v.push(item(
                    ItemId::JoinPassword,
                    rect(JOIN_X, LAN_PASSWORD_Y, JOIN_W, ROW_H),
                    true,
                ));
                v.push(item(
                    ItemId::JoinBack,
                    rect(JOIN_X, LAN_BACK_Y, 400.0, ROW_H),
                    true,
                ));
            }
            route => {
                let field = if route == JoinRoute::Direct {
                    ItemId::JoinAddress
                } else {
                    ItemId::JoinCode
                };
                v.push(item(field, rect(JOIN_X, 290.0, JOIN_W, 72.0), true));
                v.push(item(
                    ItemId::JoinPassword,
                    rect(JOIN_X, 470.0, JOIN_W, 72.0),
                    true,
                ));
                v.push(item(
                    ItemId::JoinConnect,
                    rect(JOIN_X, 580.0, BTN_W, BTN_H),
                    self.join_problem().is_none(),
                ));
                v.push(item(
                    ItemId::JoinBack,
                    rect(JOIN_X, 580.0 + BTN_STEP, BTN_W, BTN_H),
                    true,
                ));
            }
        }
        v
    }

    fn lobby_items(&self) -> Vec<Item> {
        let mut v = Vec::new();
        if self.is_host() {
            for (i, p) in self.players.iter().enumerate() {
                let slot = i
                    .checked_sub(self.player_scroll)
                    .filter(|s| *s < VISIBLE_PLAYERS);
                v.push(Item {
                    id: ItemId::LobbyPlayer(p.id),
                    rect: slot.map(|k| {
                        rect(
                            PLAYERS_X,
                            LOBBY_LIST_Y + k as f32 * LIST_STEP,
                            PLAYERS_W,
                            ROW_H,
                        )
                    }),
                    enabled: !p.host && !p.local,
                });
            }
        }
        for (i, c) in self.characters.iter().enumerate() {
            let slot = i
                .checked_sub(self.char_scroll)
                .filter(|s| *s < VISIBLE_CHARACTERS);
            v.push(Item {
                id: ItemId::LobbyCharacter(c.id),
                rect: slot
                    .map(|k| rect(CHARS_X, LOBBY_LIST_Y + k as f32 * LIST_STEP, CHARS_W, ROW_H)),
                enabled: self.taken_by(c.id).is_none(),
            });
        }
        let button = |id, x, w, enabled| Item {
            id,
            rect: Some(rect(x, LOBBY_BUTTON_Y, w, BTN_H)),
            enabled,
        };
        v.push(button(
            ItemId::LobbyReady,
            PLAYERS_X,
            480.0,
            self.can_ready(),
        ));
        v.push(button(ItemId::LobbyStart, 620.0, 480.0, self.can_start()));
        v.push(button(ItemId::LobbyLeave, CHARS_X, CHARS_W, true));
        v
    }

    /// The item at a point (canvas units): only enabled items that are drawn.
    fn hit(&self, p: Vec2) -> Option<ItemId> {
        self.items()
            .into_iter()
            .find(|i| i.enabled && i.rect.is_some_and(|r| r.contains(p)))
            .map(|i| i.id)
    }

    /// After anything changed: put the focus on a usable item (the same one when it still is, else the nearest), scroll the lists to show it, and
    /// drop what no longer applies.
    fn settle(&mut self) {
        // The lists never show past their end.
        let servers = self.servers.len();
        let players = self.players.len();
        let characters = self.characters.len();
        self.join.scroll = self
            .join
            .scroll
            .min(servers.saturating_sub(VISIBLE_SERVERS));
        self.player_scroll = self
            .player_scroll
            .min(players.saturating_sub(VISIBLE_PLAYERS));
        self.char_scroll = self
            .char_scroll
            .min(characters.saturating_sub(VISIBLE_CHARACTERS));
        let items = self.items();
        let here = items.iter().position(|i| i.id == self.focus);
        let usable = here.filter(|i| items[*i].enabled);
        // An item that went away is replaced by the nearest usable one of the same list (the next player, the next server), else the nearest of all.
        let target = usable
            .or_else(|| {
                nearest(&items, self.focus_pos, |i| {
                    i.enabled && same_list(i.id, self.focus)
                })
            })
            .or_else(|| nearest_enabled(&items, self.focus_pos));
        if let Some(i) = target {
            self.focus = items[i].id;
            self.focus_pos = i;
            match self.focus {
                ItemId::JoinServer(k) => {
                    scroll_to(&mut self.join.scroll, k, VISIBLE_SERVERS, servers)
                }
                ItemId::LobbyPlayer(id) => {
                    if let Some(k) = self.players.iter().position(|p| p.id == id) {
                        scroll_to(&mut self.player_scroll, k, VISIBLE_PLAYERS, players);
                    }
                }
                ItemId::LobbyCharacter(id) => {
                    if let Some(k) = self.characters.iter().position(|c| c.id == id) {
                        scroll_to(&mut self.char_scroll, k, VISIBLE_CHARACTERS, characters);
                    }
                }
                _ => {}
            }
        }
        if let Some(p) = self.kick_armed
            && self.focus != ItemId::LobbyPlayer(p)
        {
            self.kick_armed = None;
        }
    }

    /// Move the focus one step through the usable items, wrapping at the ends.
    fn move_focus(&mut self, step: isize) {
        let items = self.items();
        let n = items.len() as isize;
        let Some(here) = items.iter().position(|i| i.id == self.focus) else {
            return;
        };
        for k in 1..=n {
            let at = (here as isize + step * k).rem_euclid(n) as usize;
            if items[at].enabled {
                self.focus = items[at].id;
                break;
            }
        }
        self.settle();
    }

    fn go(&mut self, view: LobbyView) {
        self.view = view;
        self.pressed = None;
        self.kick_armed = None;
        self.focus = match view {
            LobbyView::Start => ItemId::StartHost,
            LobbyView::HostSetup => ItemId::HostName,
            LobbyView::Join => match self.join.route {
                JoinRoute::Lan => self
                    .sorted_servers()
                    .iter()
                    .position(|s| s.status == ServerStatus::Open)
                    .map_or(ItemId::JoinRefresh, ItemId::JoinServer),
                JoinRoute::Direct => ItemId::JoinAddress,
                JoinRoute::Invite => ItemId::JoinCode,
            },
            LobbyView::Connecting => ItemId::ConnectCancel,
            LobbyView::Lobby => ItemId::LobbyReady,
            // The usual way on is to try again; without that button the focus falls to Back.
            LobbyView::Error => ItemId::ErrorRetry,
        };
        self.focus_pos = 0;
        self.settle();
    }

    // -----------------------------------------------------------------------------------------------------------------------------
    // Events

    /// Feed one event.
    pub fn handle(&mut self, input: LobbyInput) -> Option<LobbyCommand> {
        match input {
            LobbyInput::Ui(ui) => self.handle_ui(ui),
            LobbyInput::Text(text) => {
                self.type_text(&text);
                None
            }
            LobbyInput::Backspace => self.edit(TextField::backspace),
            LobbyInput::Delete => self.edit(TextField::delete),
            LobbyInput::Home => self.edit(TextField::home),
            LobbyInput::End => self.edit(TextField::end),
        }
    }

    fn edit(&mut self, change: impl FnOnce(&mut TextField)) -> Option<LobbyCommand> {
        let focus = self.focus;
        if let Some(f) = self.field_mut(focus) {
            change(f);
            self.settle();
        }
        None
    }

    /// Typing goes to the focused field. On the direct and invite tabs it also takes the focus to the tab's field first.
    fn type_text(&mut self, text: &str) {
        if self.view == LobbyView::Join && self.field(self.focus).is_none() {
            match self.join.route {
                JoinRoute::Direct => self.focus = ItemId::JoinAddress,
                JoinRoute::Invite => self.focus = ItemId::JoinCode,
                JoinRoute::Lan => {}
            }
        }
        let focus = self.focus;
        if let Some(f) = self.field_mut(focus) {
            f.insert_str(text);
            self.settle();
        }
    }

    fn handle_ui(&mut self, input: UiInput) -> Option<LobbyCommand> {
        match input {
            UiInput::Nav(NavInput::Up) => {
                self.move_focus(-1);
                None
            }
            UiInput::Nav(NavInput::Down) => {
                self.move_focus(1);
                None
            }
            UiInput::Nav(NavInput::Left) => self.sideways(-1),
            UiInput::Nav(NavInput::Right) => self.sideways(1),
            UiInput::Nav(NavInput::Activate) => self.activate(self.focus),
            UiInput::Nav(NavInput::Back) => self.back(),
            UiInput::PointerMoved(p) => {
                if let Some(id) = self.hit(p) {
                    self.focus_on(id);
                }
                None
            }
            UiInput::PointerPressed(p) => {
                self.pressed = self.hit(p);
                if let Some(id) = self.pressed {
                    self.focus_on(id);
                }
                None
            }
            UiInput::PointerReleased(p) => match (self.pressed.take(), self.hit(p)) {
                // Pressing on a field only focuses it; every other item is a button.
                (Some(a), Some(b)) if a == b && self.field(a).is_none() => {
                    self.focus_on(a);
                    self.activate(a)
                }
                _ => None,
            },
        }
    }

    fn focus_on(&mut self, id: ItemId) {
        // `settle` finds where the item is, and keeps the focus on it when it is usable.
        if self.items().iter().any(|i| i.id == id) {
            self.focus = id;
            self.settle();
        }
    }

    fn back(&mut self) -> Option<LobbyCommand> {
        match self.view {
            LobbyView::Start => Some(LobbyCommand::Back),
            LobbyView::HostSetup | LobbyView::Join => {
                self.go(LobbyView::Start);
                None
            }
            LobbyView::Connecting => self.activate(ItemId::ConnectCancel),
            // With a kick waiting for its confirmation, Back only cancels that.
            LobbyView::Lobby if self.kick_armed.take().is_some() => None,
            LobbyView::Lobby => self.activate(ItemId::LobbyLeave),
            LobbyView::Error => self.activate(ItemId::ErrorBack),
        }
    }

    /// Left and right: the cursor in a field, a value on the setup screen, the tabs, or a neighbour in the lobby.
    fn sideways(&mut self, dir: isize) -> Option<LobbyCommand> {
        let focus = self.focus;
        if let Some(f) = self.field_mut(focus) {
            if dir < 0 {
                f.left();
            } else {
                f.right();
            }
            return None;
        }
        match focus {
            ItemId::HostMaxPlayers => {
                let lo = self.cfg.min_players;
                let hi = self.cfg.max_players_limit.max(lo);
                let now = i16::from(self.host.max_players);
                self.host.max_players =
                    (now + dir as i16).clamp(i16::from(lo), i16::from(hi)) as u8;
            }
            ItemId::HostLevel => {
                let n = self.levels.len();
                if n > 0 {
                    self.host.level =
                        (self.host.level as isize + dir).rem_euclid(n as isize) as usize;
                }
            }
            ItemId::JoinTab(route) => {
                let at = route.index() as isize + dir;
                if let Some(next) = usize::try_from(at).ok().and_then(|i| JoinRoute::ALL.get(i)) {
                    self.join.route = *next;
                    self.focus_on(ItemId::JoinTab(*next));
                }
            }
            ItemId::LobbyReady | ItemId::LobbyStart | ItemId::LobbyLeave => {
                let items = self.items();
                let row = [ItemId::LobbyReady, ItemId::LobbyStart, ItemId::LobbyLeave];
                let here = row.iter().position(|r| *r == focus).unwrap_or(0) as isize;
                let mut at = here + dir;
                while (0..3).contains(&at) {
                    let id = row[at as usize];
                    if items.iter().any(|i| i.id == id && i.enabled) {
                        self.focus_on(id);
                        break;
                    }
                    at += dir;
                }
            }
            ItemId::LobbyPlayer(_) if dir > 0 => {
                let first = self
                    .items()
                    .into_iter()
                    .find(|i| matches!(i.id, ItemId::LobbyCharacter(_)) && i.enabled);
                if let Some(i) = first {
                    self.focus_on(i.id);
                }
            }
            ItemId::LobbyCharacter(_) if dir < 0 => {
                let first = self
                    .items()
                    .into_iter()
                    .find(|i| matches!(i.id, ItemId::LobbyPlayer(_)) && i.enabled);
                if let Some(i) = first {
                    self.focus_on(i.id);
                }
            }
            _ => {}
        }
        self.settle();
        None
    }

    fn activate(&mut self, id: ItemId) -> Option<LobbyCommand> {
        // An item that is gone or disabled does nothing (a press that began on it, a stale focus).
        if !self.items().iter().any(|i| i.id == id && i.enabled) {
            return None;
        }
        match id {
            ItemId::StartHost => {
                self.go(LobbyView::HostSetup);
                None
            }
            ItemId::StartJoin => {
                self.go(LobbyView::Join);
                None
            }
            ItemId::StartBack => Some(LobbyCommand::Back),
            ItemId::HostName | ItemId::HostPassword | ItemId::JoinPassword => None,
            ItemId::HostMaxPlayers => {
                let (lo, hi) = (
                    self.cfg.min_players,
                    self.cfg.max_players_limit.max(self.cfg.min_players),
                );
                self.host.max_players = if self.host.max_players >= hi {
                    lo
                } else {
                    self.host.max_players + 1
                };
                None
            }
            ItemId::HostPasswordOn => {
                self.host.password_on = !self.host.password_on;
                self.settle();
                None
            }
            ItemId::HostLevel => {
                let n = self.levels.len().max(1);
                self.host.level = (self.host.level + 1) % n;
                None
            }
            ItemId::HostStart => {
                let settings = self.host_settings().ok()?;
                self.connecting_label = format!("Starting {}", settings.name);
                self.begin(LobbyView::HostSetup, Request::Host(settings.clone()));
                Some(LobbyCommand::Host(settings))
            }
            ItemId::HostBack | ItemId::JoinBack => {
                self.go(LobbyView::Start);
                None
            }
            ItemId::JoinTab(route) => {
                self.join.route = route;
                self.settle();
                None
            }
            ItemId::JoinRefresh => Some(LobbyCommand::RefreshServers),
            ItemId::JoinSort => {
                let keep = self.focused_server_id();
                self.join.sort = self.join.sort.next();
                self.refocus_server(keep);
                self.settle();
                None
            }
            ItemId::JoinServer(i) => {
                let server = (*self.sorted_servers().get(i)?).clone();
                let password =
                    (!self.join.password.is_empty()).then(|| self.join.password.text().to_owned());
                let request = JoinRequest {
                    target: JoinTarget::Server(server.id.clone()),
                    password,
                };
                self.connecting_label = format!("Connecting to {}", server.name);
                self.begin(LobbyView::Join, Request::Join(request.clone()));
                Some(LobbyCommand::Join(request))
            }
            ItemId::JoinAddress | ItemId::JoinCode | ItemId::JoinConnect => {
                let request = self.join_request().ok()?;
                self.connecting_label = match &request.target {
                    JoinTarget::Direct(_) => {
                        format!("Connecting to {}", self.join.address.text().trim())
                    }
                    _ => "Connecting with the invite code".to_owned(),
                };
                self.begin(LobbyView::Join, Request::Join(request.clone()));
                Some(LobbyCommand::Join(request))
            }
            ItemId::ConnectCancel => {
                let back = self.origin;
                self.go(back);
                Some(LobbyCommand::Cancel)
            }
            ItemId::LobbyPlayer(p) => {
                if self.kick_armed == Some(p) {
                    self.kick_armed = None;
                    return Some(LobbyCommand::Kick(p));
                }
                self.kick_armed = Some(p);
                None
            }
            ItemId::LobbyCharacter(c) => {
                if self.local_character() == Some(c) {
                    return None;
                }
                self.pending_character = Some(c);
                self.notice = None;
                // A new character is a new decision: the ready flag goes (the session clears it too).
                if let Some(me) = self.players.iter_mut().find(|p| p.local) {
                    me.ready = false;
                }
                self.settle();
                Some(LobbyCommand::SelectCharacter(c))
            }
            ItemId::LobbyReady => {
                let me = self.players.iter_mut().find(|p| p.local)?;
                me.ready = !me.ready;
                let ready = me.ready;
                self.settle();
                Some(LobbyCommand::SetReady(ready))
            }
            ItemId::LobbyStart => Some(LobbyCommand::StartGame),
            ItemId::LobbyLeave => {
                self.clear_session();
                self.go(LobbyView::Start);
                Some(LobbyCommand::Leave)
            }
            ItemId::ErrorRetry => {
                let request = self.request.clone()?;
                let origin = self.error_return;
                match request {
                    Request::Host(settings) => {
                        self.begin(origin, Request::Host(settings.clone()));
                        Some(LobbyCommand::Host(settings))
                    }
                    Request::Join(join) => {
                        self.begin(origin, Request::Join(join.clone()));
                        Some(LobbyCommand::Join(join))
                    }
                }
            }
            ItemId::ErrorBack => {
                let back = self.error_return;
                let fix_password = matches!(self.failure, Some(Failure::BadPassword));
                self.failure = None;
                self.go(back);
                if fix_password && back == LobbyView::Join {
                    self.focus_on(ItemId::JoinPassword);
                }
                None
            }
        }
    }

    /// Show the connecting screen for `request`; Cancel returns to `origin`.
    fn begin(&mut self, origin: LobbyView, request: Request) {
        self.request = Some(request);
        self.origin = origin;
        self.stage = None;
        self.elapsed_ms = 0;
        self.failure = None;
        self.go(LobbyView::Connecting);
    }

    /// Forget the session: players, the pending character, the notice.
    fn clear_session(&mut self) {
        self.players.clear();
        self.player_scroll = 0;
        self.char_scroll = 0;
        self.pending_character = None;
        self.notice = None;
        self.kick_armed = None;
    }

    /// Show the error screen for `failure`.
    fn fail(&mut self, failure: Failure) {
        self.error_return = if self.view == LobbyView::Lobby {
            LobbyView::Start
        } else {
            self.origin
        };
        if self.view == LobbyView::Lobby {
            self.clear_session();
        }
        self.failure = Some(failure);
        self.go(LobbyView::Error);
    }

    /// Replace the contents of `list` with the screen. (Texts built at run time allocate; see the module documentation.)
    pub fn draw_into(&self, list: &mut DrawList) {
        self.draw_view(list);
    }

    /// The screen as a new list.
    pub fn draw(&self) -> DrawList {
        let mut list = DrawList::default();
        self.draw_into(&mut list);
        list
    }
}

/// Whether two items are rows of the same list (servers, players or characters).
fn same_list(a: ItemId, b: ItemId) -> bool {
    matches!(
        (a, b),
        (ItemId::JoinServer(_), ItemId::JoinServer(_))
            | (ItemId::LobbyPlayer(_), ItemId::LobbyPlayer(_))
            | (ItemId::LobbyCharacter(_), ItemId::LobbyCharacter(_))
    )
}

/// The position of the item nearest to `pos` that `ok` accepts (a later one first when two are as near), if there is one.
fn nearest(items: &[Item], pos: usize, ok: impl Fn(&Item) -> bool) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let pos = pos.min(items.len() - 1);
    (0..items.len()).find_map(|d| {
        let after = pos + d;
        if after < items.len() && ok(&items[after]) {
            return Some(after);
        }
        let before = pos.checked_sub(d)?;
        ok(&items[before]).then_some(before)
    })
}

/// The position of the usable item nearest to `pos`, if there is one.
fn nearest_enabled(items: &[Item], pos: usize) -> Option<usize> {
    nearest(items, pos, |i| i.enabled)
}
