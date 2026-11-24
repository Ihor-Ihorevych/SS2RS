//! Drawing the lobby screens: the same items as [`Lobby::items`], with their contents.

use std::borrow::Cow;

use crate::data_wizard::{field_view, shorten_middle, wrap};
use crate::draw::{Color, DrawList, Quad, TextAlign, TextRun};
use crate::geometry::{Rect, Vec2};
use crate::theme;

use super::{
    CHARS_W, CHARS_X, CharacterId, HostSpec, ItemId, JOIN_X, JoinRoute, JoinTarget, LIST_STEP,
    LOBBY_LIST_Y, Lobby, LobbyView, PLAYERS_W, PLAYERS_X, PlayerRow, ROW_H, SERVER_W, SERVER_Y,
    ServerRow, ServerStatus, VISIBLE_CHARACTERS, VISIBLE_PLAYERS, VISIBLE_SERVERS,
};

const GOOD: Color = Color([0.55, 0.90, 0.55, 1.0]);
const WARN: Color = Color([0.95, 0.80, 0.35, 1.0]);
const BAD: Color = Color([1.00, 0.45, 0.40, 1.0]);
const DIM: Color = Color([0.75, 0.77, 0.82, 1.0]);
/// A row or tab that is chosen but does not have the focus.
const SELECTED: Color = Color([0.30, 0.33, 0.42, 1.0]);
/// An item that cannot be used.
const DISABLED: Color = Color([0.11, 0.115, 0.13, 1.0]);
/// A player row of the host's kick that waits for the second press.
const ARMED: Color = Color([0.62, 0.20, 0.20, 1.0]);
const DISABLED_TEXT: Color = Color([0.45, 0.46, 0.50, 1.0]);

const LABEL: f32 = 36.0;
const SMALL: f32 = 28.0;
/// The room a drawn row leaves above its text so that it sits in the middle.
fn text_top(r: Rect, size: f32) -> f32 {
    r.y + (r.h - size) / 2.0 - 2.0
}

fn text(
    list: &mut DrawList,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    align: TextAlign,
    text: impl Into<Cow<'static, str>>,
) {
    list.texts.push(TextRun {
        pos: Vec2 { x, y },
        text: text.into(),
        size,
        color,
        align,
    });
}

fn centered(list: &mut DrawList, y: f32, size: f32, color: Color, t: impl Into<Cow<'static, str>>) {
    text(list, 960.0, y, size, color, TextAlign::Center, t);
}

fn quad(list: &mut DrawList, rect: Rect, color: Color) {
    list.quads.push(Quad { rect, color });
}

fn fill_color(focused: bool, enabled: bool) -> Color {
    if focused {
        theme::FOCUSED
    } else if enabled {
        theme::IDLE
    } else {
        DISABLED
    }
}

fn ink(enabled: bool) -> Color {
    if enabled { theme::TEXT } else { DISABLED_TEXT }
}

fn ping_text(ping: Option<u32>) -> String {
    match ping {
        Some(ms) if ms > 999 => "999+ ms".to_owned(),
        Some(ms) => format!("{ms} ms"),
        None => "-- ms".to_owned(),
    }
}

impl Lobby {
    fn is_focus(&self, id: ItemId) -> bool {
        self.focus == id
    }

    /// The rectangle and state of an item of the screen showing.
    fn slot(&self, id: ItemId) -> Option<(Rect, bool)> {
        self.items()
            .into_iter()
            .find(|i| i.id == id)
            .and_then(|i| i.rect.map(|r| (r, i.enabled)))
    }

    /// A button: a row with its label in the middle.
    fn button(&self, list: &mut DrawList, id: ItemId, label: &'static str) {
        if let Some((r, enabled)) = self.slot(id) {
            quad(list, r, fill_color(self.is_focus(id), enabled));
            text(
                list,
                r.x + r.w / 2.0,
                text_top(r, LABEL),
                LABEL,
                ink(enabled),
                TextAlign::Center,
                label,
            );
        }
    }

    /// A row of a form: the label on the left, and the value on the right (in a box when it is a field).
    fn form_row(
        &self,
        list: &mut DrawList,
        id: ItemId,
        label: &'static str,
    ) -> Option<(Rect, bool)> {
        let (r, enabled) = self.slot(id)?;
        quad(list, r, fill_color(self.is_focus(id), enabled));
        text(
            list,
            r.x + 24.0,
            text_top(r, LABEL),
            LABEL,
            ink(enabled),
            TextAlign::Left,
            label,
        );
        Some((r, enabled))
    }

    /// The text of a field inside its box, with the cursor when it has the focus (and the blink is on).
    fn field_box(&self, list: &mut DrawList, id: ItemId, boxed: Rect, masked: bool, enabled: bool) {
        quad(list, boxed, theme::TRACK);
        let Some(field) = self.field(id) else { return };
        let focused = self.is_focus(id);
        let blink_on = (self.clock_ms / 500).is_multiple_of(2);
        let shown = if masked {
            let stars = "*".repeat(field.len());
            field_view(&stars, field.cursor(), focused && blink_on)
        } else {
            field_view(field.text(), field.cursor(), focused && blink_on)
        };
        text(
            list,
            boxed.x + 14.0,
            text_top(boxed, 32.0),
            32.0,
            ink(enabled),
            TextAlign::Left,
            shown,
        );
    }

    fn labeled_field(&self, list: &mut DrawList, id: ItemId, label: &'static str, masked: bool) {
        if let Some((r, enabled)) = self.form_row(list, id, label) {
            let boxed = Rect {
                x: r.x + 360.0,
                y: r.y + 10.0,
                w: r.w - 360.0 - 16.0,
                h: r.h - 20.0,
            };
            self.field_box(list, id, boxed, masked, enabled);
        }
    }

    pub(super) fn draw_view(&self, list: &mut DrawList) {
        list.clear();
        match self.view {
            LobbyView::Start => self.draw_start(list),
            LobbyView::HostSetup => self.draw_setup(list),
            LobbyView::Join => self.draw_join(list),
            LobbyView::Connecting => self.draw_connecting(list),
            LobbyView::Lobby => self.draw_lobby(list),
            LobbyView::Error => self.draw_error(list),
        }
    }

    fn draw_start(&self, list: &mut DrawList) {
        centered(list, 160.0, 96.0, theme::TEXT, "Co-op");
        centered(list, 290.0, 36.0, theme::TEXT, "Host a game, or join one");
        self.button(list, ItemId::StartHost, "Host a game");
        self.button(list, ItemId::StartJoin, "Join a game");
        self.button(list, ItemId::StartBack, "Back");
    }

    fn draw_setup(&self, list: &mut DrawList) {
        centered(list, 60.0, 72.0, theme::TEXT, "Host a game");
        self.labeled_field(list, ItemId::HostName, "Game name", false);
        if let Some((r, enabled)) = self.form_row(list, ItemId::HostMaxPlayers, "Players") {
            text(
                list,
                r.x + 384.0,
                text_top(r, LABEL),
                LABEL,
                ink(enabled),
                TextAlign::Left,
                format!("<  {}  >", self.host.max_players),
            );
        }
        if let Some((r, enabled)) = self.form_row(list, ItemId::HostPasswordOn, "Password") {
            let on = if self.host.password_on { "On" } else { "Off" };
            text(
                list,
                r.x + 384.0,
                text_top(r, LABEL),
                LABEL,
                ink(enabled),
                TextAlign::Left,
                on,
            );
        }
        self.labeled_field(list, ItemId::HostPassword, "Password text", true);
        if let Some((r, enabled)) = self.form_row(list, ItemId::HostLevel, "Level") {
            let value = match self.levels.get(self.host.level) {
                Some(l) => format!("<  {}  >", shorten_middle(&l.name, 36)),
                None => "(no levels)".to_owned(),
            };
            text(
                list,
                r.x + 384.0,
                text_top(r, LABEL),
                LABEL,
                ink(enabled),
                TextAlign::Left,
                value,
            );
        }
        self.button(list, ItemId::HostStart, "Start hosting");
        self.button(list, ItemId::HostBack, "Back");
        if let Err(why) = self.host_settings() {
            centered(list, 870.0, 30.0, WARN, why);
        }
    }

    fn draw_join(&self, list: &mut DrawList) {
        centered(list, 60.0, 72.0, theme::TEXT, "Join a game");
        for route in JoinRoute::ALL {
            let id = ItemId::JoinTab(route);
            if let Some((r, _)) = self.slot(id) {
                let color = if self.is_focus(id) {
                    theme::FOCUSED
                } else if self.join.route == route {
                    SELECTED
                } else {
                    theme::IDLE
                };
                quad(list, r, color);
                text(
                    list,
                    r.x + r.w / 2.0,
                    text_top(r, 32.0),
                    32.0,
                    theme::TEXT,
                    TextAlign::Center,
                    route.label(),
                );
            }
        }
        match self.join.route {
            JoinRoute::Lan => self.draw_lan(list),
            route => self.draw_typed(list, route),
        }
        self.labeled_field(
            list,
            ItemId::JoinPassword,
            if self.join.route == JoinRoute::Lan {
                "Password"
            } else {
                "Password (if any)"
            },
            true,
        );
        self.button(list, ItemId::JoinBack, "Back");
    }

    fn draw_lan(&self, list: &mut DrawList) {
        self.button(list, ItemId::JoinRefresh, "Refresh");
        if let Some((r, _)) = self.slot(ItemId::JoinSort) {
            quad(list, r, fill_color(self.is_focus(ItemId::JoinSort), true));
            text(
                list,
                r.x + r.w / 2.0,
                text_top(r, 32.0),
                32.0,
                theme::TEXT,
                TextAlign::Center,
                format!("Sort by: {}", self.join.sort.label()),
            );
        }
        let rows = self.sorted_servers();
        let n = rows.len();
        let status = if self.scanning {
            "Searching the local network...".to_owned()
        } else if n == 0 {
            String::new()
        } else if n > VISIBLE_SERVERS {
            format!(
                "{n} games found (showing {}-{})",
                self.join.scroll + 1,
                self.join.scroll + VISIBLE_SERVERS
            )
        } else {
            format!("{n} game{} found", if n == 1 { "" } else { "s" })
        };
        text(
            list,
            JOIN_X + 800.0,
            262.0,
            SMALL,
            DIM,
            TextAlign::Left,
            status,
        );
        // The column titles.
        for (x, title) in [
            (24.0, "Name"),
            (470.0, "Level"),
            (800.0, "Players"),
            (940.0, "Ping"),
            (1070.0, "Lock"),
        ] {
            text(list, JOIN_X + x, 332.0, 24.0, DIM, TextAlign::Left, title);
        }
        if n == 0 {
            let r = Rect {
                x: JOIN_X,
                y: SERVER_Y,
                w: SERVER_W,
                h: 2.0 * ROW_H + 8.0,
            };
            quad(list, r, theme::TRACK);
            let first = if self.scanning {
                "Searching the local network..."
            } else {
                "No games found on your network"
            };
            centered(list, r.y + 16.0, 34.0, theme::TEXT, first);
            centered(
                list,
                r.y + 72.0,
                SMALL,
                DIM,
                "Press Refresh, or join with an address or an invite code.",
            );
        }
        for (i, server) in rows.iter().enumerate() {
            if let Some((r, enabled)) = self
                .slot(ItemId::JoinServer(i))
                .filter(|_| i >= self.join.scroll && i < self.join.scroll + VISIBLE_SERVERS)
            {
                self.draw_server(
                    list,
                    server,
                    r,
                    self.is_focus(ItemId::JoinServer(i)),
                    enabled,
                );
            }
        }
        if n > VISIBLE_SERVERS {
            let track = Rect {
                x: JOIN_X + SERVER_W + 10.0,
                y: SERVER_Y,
                w: 14.0,
                h: VISIBLE_SERVERS as f32 * LIST_STEP - 8.0,
            };
            quad(list, track, theme::TRACK);
            let thumb_h = track.h * VISIBLE_SERVERS as f32 / n as f32;
            quad(
                list,
                Rect {
                    y: track.y + track.h * self.join.scroll as f32 / n as f32,
                    h: thumb_h,
                    ..track
                },
                theme::TRACK_FILL,
            );
        }
    }

    fn draw_server(
        &self,
        list: &mut DrawList,
        s: &ServerRow,
        r: Rect,
        focused: bool,
        enabled: bool,
    ) {
        quad(list, r, fill_color(focused, enabled));
        let y = text_top(r, 32.0);
        let color = ink(enabled);
        text(
            list,
            r.x + 24.0,
            y,
            32.0,
            color,
            TextAlign::Left,
            shorten_middle(&s.name, 22),
        );
        text(
            list,
            r.x + 470.0,
            y + 2.0,
            SMALL,
            if enabled { DIM } else { DISABLED_TEXT },
            TextAlign::Left,
            shorten_middle(&s.map, 14),
        );
        text(
            list,
            r.x + 800.0,
            y,
            32.0,
            color,
            TextAlign::Left,
            format!("{}/{}", s.players, s.max_players),
        );
        text(
            list,
            r.x + 940.0,
            y,
            32.0,
            color,
            TextAlign::Left,
            ping_text(s.ping_ms),
        );
        if s.locked {
            text(
                list,
                r.x + 1070.0,
                y,
                32.0,
                if enabled { WARN } else { DISABLED_TEXT },
                TextAlign::Left,
                "Locked",
            );
        }
        let note: Option<Cow<'static, str>> = match &s.status {
            ServerStatus::Open => None,
            ServerStatus::Full => Some("Full".into()),
            ServerStatus::Incompatible(why) => Some(shorten_middle(why, 16).into()),
        };
        if let Some(note) = note {
            text(list, r.x + 1220.0, y, SMALL, BAD, TextAlign::Left, note);
        }
    }

    fn draw_typed(&self, list: &mut DrawList, route: JoinRoute) {
        let (id, caption) = if route == JoinRoute::Direct {
            (
                ItemId::JoinAddress,
                "Address of the host, with the port if it is not 27015",
            )
        } else {
            (ItemId::JoinCode, "The invite code the host gave you")
        };
        text(list, JOIN_X, 246.0, 30.0, DIM, TextAlign::Left, caption);
        if let Some((r, enabled)) = self.slot(id) {
            quad(list, r, fill_color(self.is_focus(id), enabled));
            let boxed = Rect {
                x: r.x + 16.0,
                y: r.y + 10.0,
                w: r.w - 32.0,
                h: r.h - 20.0,
            };
            self.field_box(list, id, boxed, false, enabled);
        }
        let empty = self.field(id).is_some_and(|f| f.is_empty());
        let (color, line): (Color, String) = match (self.join_request(), empty) {
            (Ok(request), _) => (
                GOOD,
                match request.target {
                    JoinTarget::Direct(a) => match a.host {
                        HostSpec::Ipv4(ip) => format!("OK: {ip}, port {}", a.port),
                        HostSpec::Ipv6(ip) => format!("OK: {ip}, port {}", a.port),
                        HostSpec::Name(n) => format!("OK: {n}, port {}", a.port),
                    },
                    JoinTarget::Invite(code) => format!("OK: {code}"),
                    JoinTarget::Server(_) => String::new(),
                },
            ),
            (Err(why), true) => (DIM, why),
            (Err(why), false) => (BAD, why),
        };
        text(list, JOIN_X, 376.0, 30.0, color, TextAlign::Left, line);
        self.button(list, ItemId::JoinConnect, "Connect");
    }

    fn draw_connecting(&self, list: &mut DrawList) {
        centered(list, 60.0, 72.0, theme::TEXT, "Please wait");
        centered(
            list,
            260.0,
            44.0,
            theme::TEXT,
            shorten_middle(&self.connecting_label, 50),
        );
        match &self.stage {
            Some((what, percent)) => {
                let line = match percent {
                    Some(p) => format!("{what}: {p}%"),
                    None => what.clone(),
                };
                centered(list, 340.0, 36.0, theme::TEXT, shorten_middle(&line, 60));
                let bar = Rect {
                    x: 560.0,
                    y: 420.0,
                    w: 800.0,
                    h: 28.0,
                };
                quad(list, bar, theme::TRACK);
                if let Some(p) = percent {
                    quad(
                        list,
                        Rect {
                            w: bar.w * f32::from(*p) / 100.0,
                            ..bar
                        },
                        theme::TRACK_FILL,
                    );
                }
            }
            None => {
                let dots = ".".repeat((self.clock_ms / 400 % 4) as usize);
                centered(
                    list,
                    340.0,
                    36.0,
                    theme::TEXT,
                    format!("Waiting for a reply{dots}"),
                );
            }
        }
        let left = self
            .cfg
            .connect_timeout_ms
            .saturating_sub(self.elapsed_ms)
            .div_ceil(1000);
        centered(
            list,
            500.0,
            SMALL,
            DIM,
            format!("It gives up in {left} s if nothing answers"),
        );
        self.button(list, ItemId::ConnectCancel, "Cancel");
    }

    fn draw_error(&self, list: &mut DrawList) {
        let Some(failure) = &self.failure else { return };
        centered(list, 60.0, 72.0, BAD, failure.title());
        let mut y = 250.0;
        for line in wrap(&failure.message(), 62).into_iter().take(4) {
            centered(list, y, LABEL, theme::TEXT, line.trim().to_owned());
            y += 50.0;
        }
        y += 24.0;
        for line in wrap(failure.hint(), 84).into_iter().take(4) {
            centered(list, y, SMALL + 2.0, DIM, line.trim().to_owned());
            y += 42.0;
        }
        self.button(list, ItemId::ErrorRetry, "Try again");
        self.button(list, ItemId::ErrorBack, "Back");
    }

    fn draw_lobby(&self, list: &mut DrawList) {
        centered(list, 60.0, 72.0, theme::TEXT, "Lobby");
        let players = self.players();
        text(
            list,
            PLAYERS_X,
            158.0,
            SMALL,
            DIM,
            TextAlign::Left,
            self.player_header(players.len()),
        );
        text(
            list,
            CHARS_X,
            158.0,
            SMALL,
            DIM,
            TextAlign::Left,
            "Character",
        );
        for (i, p) in players.iter().enumerate() {
            if i < self.player_scroll || i >= self.player_scroll + VISIBLE_PLAYERS {
                continue;
            }
            let r = Rect {
                x: PLAYERS_X,
                y: LOBBY_LIST_Y + (i - self.player_scroll) as f32 * LIST_STEP,
                w: PLAYERS_W,
                h: ROW_H,
            };
            self.draw_player(list, p, r);
        }
        for (i, c) in self.characters.iter().enumerate() {
            if i < self.char_scroll || i >= self.char_scroll + VISIBLE_CHARACTERS {
                continue;
            }
            let r = Rect {
                x: CHARS_X,
                y: LOBBY_LIST_Y + (i - self.char_scroll) as f32 * LIST_STEP,
                w: CHARS_W,
                h: ROW_H,
            };
            let id = ItemId::LobbyCharacter(c.id);
            let enabled = self.slot(id).is_some_and(|(_, e)| e);
            let mine = self.local_character() == Some(c.id);
            let color = if self.is_focus(id) {
                theme::FOCUSED
            } else if mine {
                SELECTED
            } else if enabled {
                theme::IDLE
            } else {
                DISABLED
            };
            quad(list, r, color);
            let y = text_top(r, 32.0);
            text(
                list,
                r.x + 20.0,
                y,
                32.0,
                ink(enabled),
                TextAlign::Left,
                shorten_middle(&c.name, 18),
            );
            if mine {
                text(
                    list,
                    r.x + 400.0,
                    y + 2.0,
                    SMALL,
                    GOOD,
                    TextAlign::Left,
                    "Yours",
                );
            } else if let Some(other) = self.taken_by(c.id) {
                text(
                    list,
                    r.x + 400.0,
                    y + 2.0,
                    SMALL,
                    DISABLED_TEXT,
                    TextAlign::Left,
                    format!("Taken by {}", shorten_middle(&other.name, 10)),
                );
            }
        }
        if self.characters.is_empty() {
            let r = Rect {
                x: CHARS_X,
                y: LOBBY_LIST_Y,
                w: CHARS_W,
                h: ROW_H,
            };
            quad(list, r, theme::TRACK);
            text(
                list,
                r.x + 20.0,
                text_top(r, SMALL),
                SMALL,
                DIM,
                TextAlign::Left,
                "No characters to choose from",
            );
        }
        let line: Option<(Color, String)> = if let Some(p) = self.kick_armed {
            let who = self
                .players
                .iter()
                .find(|r| r.id == p)
                .map_or("this player", |r| r.name.as_str());
            Some((
                BAD,
                format!(
                    "Remove {} from the game? Press again to confirm, or move away.",
                    shorten_middle(who, 20)
                ),
            ))
        } else if let Some(n) = &self.notice {
            Some((WARN, n.clone()))
        } else if self.local().is_some() {
            if self.is_host() {
                (!self.can_start())
                    .then(|| (DIM, "The game can start when everyone is ready.".to_owned()))
            } else {
                Some((DIM, "Waiting for the host to start the game.".to_owned()))
            }
        } else {
            None
        };
        if let Some((color, line)) = line {
            text(list, PLAYERS_X, 820.0, 30.0, color, TextAlign::Left, line);
        }
        let ready = self.local().is_some_and(|p| p.ready);
        self.button(
            list,
            ItemId::LobbyReady,
            if ready { "Not ready" } else { "I am ready" },
        );
        self.button(list, ItemId::LobbyStart, "Start game");
        self.button(list, ItemId::LobbyLeave, "Leave");
    }

    fn player_header(&self, n: usize) -> String {
        if n > VISIBLE_PLAYERS {
            format!(
                "Players: {n} (showing {}-{})",
                self.player_scroll + 1,
                self.player_scroll + VISIBLE_PLAYERS
            )
        } else {
            format!("Players: {n}")
        }
    }

    fn draw_player(&self, list: &mut DrawList, p: &PlayerRow, r: Rect) {
        let id = ItemId::LobbyPlayer(p.id);
        let focused = self.is_focus(id);
        let color = if self.kick_armed == Some(p.id) {
            ARMED
        } else if focused {
            theme::FOCUSED
        } else {
            theme::IDLE
        };
        quad(list, r, color);
        let y = text_top(r, 32.0);
        if p.host {
            text(
                list,
                r.x + 16.0,
                y + 6.0,
                22.0,
                WARN,
                TextAlign::Left,
                "HOST",
            );
        }
        let name = if p.local {
            format!("{} (you)", shorten_middle(&p.name, 14))
        } else {
            shorten_middle(&p.name, 20)
        };
        text(
            list,
            r.x + 100.0,
            y,
            32.0,
            theme::TEXT,
            TextAlign::Left,
            name,
        );
        let character = p
            .character
            .and_then(|c: CharacterId| self.characters.iter().find(|x| x.id == c))
            .map_or_else(|| "-".to_owned(), |c| shorten_middle(&c.name, 12));
        text(
            list,
            r.x + 430.0,
            y + 2.0,
            SMALL,
            theme::TEXT,
            TextAlign::Left,
            character,
        );
        let (word, tone) = if p.ready {
            ("Ready", GOOD)
        } else {
            ("Not ready", WARN)
        };
        text(
            list,
            r.x + 650.0,
            y + 2.0,
            SMALL,
            tone,
            TextAlign::Left,
            word,
        );
        text(
            list,
            r.x + 850.0,
            y + 2.0,
            SMALL,
            DIM,
            TextAlign::Left,
            ping_text(p.ping_ms),
        );
    }
}
