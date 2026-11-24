//! Tests of the lobby screens: validation, focus, pointer, the three join routes, scrolling, the lobby rules, errors, drawing, and random play.

use std::net::{Ipv4Addr, Ipv6Addr};

use proptest::prelude::*;

use super::*;
use crate::geometry::{CANVAS_H, CANVAS_W};
use crate::theme;

// ---------------------------------------------------------------------------------------------------------------------------------
// Helpers

fn level(id: &str, name: &str) -> LevelChoice {
    LevelChoice {
        id: id.into(),
        name: name.into(),
    }
}

fn ch(id: u32, name: &str) -> CharacterChoice {
    CharacterChoice {
        id: CharacterId(id),
        name: name.into(),
    }
}

fn srv(
    id: &str,
    name: &str,
    players: u8,
    max: u8,
    ping: Option<u32>,
    locked: bool,
    status: ServerStatus,
) -> ServerRow {
    ServerRow {
        id: id.into(),
        name: name.into(),
        map: "Hatshepsut".into(),
        players,
        max_players: max,
        ping_ms: ping,
        locked,
        status,
    }
}

fn open(id: &str, name: &str, ping: u32) -> ServerRow {
    srv(id, name, 1, 4, Some(ping), false, ServerStatus::Open)
}

fn pl(
    id: u64,
    name: &str,
    character: Option<u32>,
    ready: bool,
    host: bool,
    local: bool,
) -> PlayerRow {
    PlayerRow {
        id: PlayerId(id),
        name: name.into(),
        character: character.map(CharacterId),
        ready,
        ping_ms: Some(20),
        host,
        local,
    }
}

fn lobby() -> Lobby {
    let mut l = Lobby::new(LobbyConfig::default());
    l.set_levels(vec![
        level("maps/hatshepsut", "Hatshepsut"),
        level("maps/jungle", "The Jungle"),
        level("maps/pyramid", "Pyramid"),
    ]);
    l.set_characters(vec![ch(0, "Sam"), ch(1, "Mia"), ch(2, "Zed"), ch(3, "Ann")]);
    l
}

fn nav(l: &mut Lobby, key: NavInput) -> Option<LobbyCommand> {
    l.handle(LobbyInput::Ui(UiInput::Nav(key)))
}

fn typ(l: &mut Lobby, text: &str) {
    assert_eq!(l.handle(LobbyInput::Text(text.into())), None);
}

fn rect_of(l: &Lobby, id: ItemId) -> Rect {
    l.items()
        .into_iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("{id:?} is on the screen"))
        .rect
        .unwrap_or_else(|| panic!("{id:?} is drawn"))
}

fn center(l: &Lobby, id: ItemId) -> Vec2 {
    let r = rect_of(l, id);
    Vec2 {
        x: r.x + r.w / 2.0,
        y: r.y + r.h / 2.0,
    }
}

fn pointer(l: &mut Lobby, make: fn(Vec2) -> UiInput, at: Vec2) -> Option<LobbyCommand> {
    l.handle(LobbyInput::Ui(make(at)))
}

/// A click on an item: press and release at its centre.
fn click(l: &mut Lobby, id: ItemId) -> Option<LobbyCommand> {
    let at = center(l, id);
    assert_eq!(pointer(l, UiInput::PointerPressed, at), None);
    pointer(l, UiInput::PointerReleased, at)
}

/// Move the focus to `id` with the down key.
fn go_to(l: &mut Lobby, id: ItemId) {
    for _ in 0..200 {
        if l.focus() == id {
            return;
        }
        nav(l, NavInput::Down);
    }
    panic!("the focus never reached {id:?}; it is on {:?}", l.focus());
}

fn press(l: &mut Lobby, id: ItemId) -> Option<LobbyCommand> {
    go_to(l, id);
    nav(l, NavInput::Activate)
}

fn to_join(l: &mut Lobby) {
    assert_eq!(press(l, ItemId::StartJoin), None);
    assert_eq!(l.view(), LobbyView::Join);
}

fn to_setup(l: &mut Lobby) {
    assert_eq!(press(l, ItemId::StartHost), None);
    assert_eq!(l.view(), LobbyView::HostSetup);
}

fn to_direct(l: &mut Lobby) {
    to_join(l);
    go_to(l, ItemId::JoinTab(JoinRoute::Direct));
    nav(l, NavInput::Activate);
    assert_eq!(l.route(), JoinRoute::Direct);
}

/// Through the direct route to the lobby, with these players. Use it on a new `Lobby`.
fn enter_lobby(l: &mut Lobby, players: Vec<PlayerRow>) {
    to_direct(l);
    typ(l, "10.0.0.2");
    assert!(matches!(
        nav(l, NavInput::Activate),
        Some(LobbyCommand::Join(_))
    ));
    assert_eq!(l.view(), LobbyView::Connecting);
    l.set_connection_state(ConnectionState::Connected);
    assert_eq!(l.view(), LobbyView::Lobby);
    l.set_players(players);
}

fn me(character: Option<u32>, ready: bool) -> PlayerRow {
    pl(1, "Ihor", character, ready, false, true)
}

fn host_row(character: Option<u32>, ready: bool) -> PlayerRow {
    pl(9, "Host", character, ready, true, false)
}

fn texts(l: &Lobby) -> Vec<String> {
    l.draw().texts.iter().map(|t| t.text.to_string()).collect()
}

fn has_text(l: &Lobby, needle: &str) -> bool {
    texts(l).iter().any(|t| t.contains(needle))
}

fn has_exact(l: &Lobby, text: &str) -> bool {
    texts(l).iter().any(|t| t == text)
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn is_enabled(l: &Lobby, id: ItemId) -> bool {
    l.items().iter().find(|i| i.id == id).unwrap().enabled
}

fn local_ready(l: &Lobby) -> bool {
    l.players().iter().find(|p| p.local).unwrap().ready
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Validation: addresses

fn addr(text: &str) -> Result<DirectAddress, AddressError> {
    parse_address(text)
}

fn v4(a: u8, b: u8, c: u8, d: u8, port: u16) -> DirectAddress {
    DirectAddress {
        host: HostSpec::Ipv4(Ipv4Addr::new(a, b, c, d)),
        port,
    }
}

#[test]
fn ipv4_addresses_read_with_and_without_a_port() {
    assert_eq!(addr("192.168.1.20"), Ok(v4(192, 168, 1, 20, DEFAULT_PORT)));
    assert_eq!(addr("192.168.1.20:7777"), Ok(v4(192, 168, 1, 20, 7777)));
    assert_eq!(addr("  10.0.0.1:1  "), Ok(v4(10, 0, 0, 1, 1)));
    assert_eq!(addr("0.0.0.0:65535"), Ok(v4(0, 0, 0, 0, 65535)));
    assert_eq!(
        addr("255.255.255.255"),
        Ok(v4(255, 255, 255, 255, DEFAULT_PORT))
    );
}

#[test]
fn bad_ipv4_addresses_are_refused_with_the_right_reason() {
    for bad in [
        "999.1.1.1",
        "1.2.3",
        "1.2.3.4.5",
        "256.0.0.1:80",
        "1..2.3",
        "1.2.3.",
        "1234",
    ] {
        assert_eq!(addr(bad), Err(AddressError::BadIpv4), "{bad}");
    }
}

#[test]
fn ports_must_be_one_to_65535_and_digits() {
    for bad in [
        "1.2.3.4:0",
        "1.2.3.4:65536",
        "1.2.3.4:99999",
        "1.2.3.4:",
        "1.2.3.4:-1",
        "1.2.3.4:+5",
        "1.2.3.4:5a",
        "1.2.3.4:123456",
        "host:0",
        "host:",
    ] {
        assert_eq!(addr(bad), Err(AddressError::BadPort), "{bad}");
    }
    assert_eq!(addr("1.2.3.4:00080"), Ok(v4(1, 2, 3, 4, 80)));
    assert_eq!(addr("1.2.3.4:65535"), Ok(v4(1, 2, 3, 4, 65535)));
    assert_eq!(addr("1.2.3.4:1"), Ok(v4(1, 2, 3, 4, 1)));
}

#[test]
fn ipv6_addresses_read_in_brackets_or_bare() {
    let loopback = HostSpec::Ipv6(Ipv6Addr::LOCALHOST);
    let at = |port| {
        Ok(DirectAddress {
            host: loopback.clone(),
            port,
        })
    };
    assert_eq!(addr("[::1]"), at(DEFAULT_PORT));
    assert_eq!(addr("[::1]:5000"), at(5000));
    assert_eq!(addr("::1"), at(DEFAULT_PORT));
    assert_eq!(
        addr("[2001:db8::1]:1"),
        Ok(DirectAddress {
            host: HostSpec::Ipv6("2001:db8::1".parse().unwrap()),
            port: 1
        })
    );
    for bad in [
        "[::1",
        "[::g]",
        "[::1]x",
        "[]",
        "[1.2.3.4]",
        "fe80::1::2",
        "[::1]:0",
        "[::1]:",
    ] {
        assert!(addr(bad).is_err(), "{bad}");
    }
    assert_eq!(addr("[::1"), Err(AddressError::BadIpv6));
    assert_eq!(addr("[::g]"), Err(AddressError::BadIpv6));
    assert_eq!(addr("[::1]x"), Err(AddressError::BadPort));
    assert_eq!(addr("[::1]:0"), Err(AddressError::BadPort));
    assert_eq!(
        addr("a:b:c"),
        Err(AddressError::BadPort),
        "the last colon is read as a port"
    );
    assert_eq!(
        addr("a:b:80"),
        Err(AddressError::BadIpv6),
        "a name does not have colons"
    );
}

#[test]
fn host_names_read_in_lower_case() {
    let name = |n: &str, port| {
        Ok(DirectAddress {
            host: HostSpec::Name(n.into()),
            port,
        })
    };
    assert_eq!(
        addr("Game.Example.COM"),
        name("game.example.com", DEFAULT_PORT)
    );
    assert_eq!(addr("my-host:99"), name("my-host", 99));
    assert_eq!(addr("localhost"), name("localhost", DEFAULT_PORT));
    assert_eq!(addr("a1.b2"), name("a1.b2", DEFAULT_PORT));
    for bad in [
        "-a",
        "a-",
        "a..b",
        ".a",
        "a.",
        "a_b",
        "a/b",
        "m\u{fc}nchen.de",
        "a.-b.c",
        "a.b-.c",
        "http://x",
        "x@y",
    ] {
        assert!(addr(bad).is_err(), "{bad}");
    }
    assert_eq!(addr("a_b"), Err(AddressError::BadHostName));
    assert_eq!(addr("a.-b"), Err(AddressError::BadHostName));
    assert_eq!(addr("a.b-"), Err(AddressError::BadHostName));
    assert_eq!(addr(&"a".repeat(64)), Err(AddressError::BadHostName));
    assert!(addr(&"a".repeat(63)).is_ok());
    // 63 + 1 + 63 + 1 + 63 + 1 + 62 = 254 bytes: one over.
    let long_name = format!("{0}.{0}.{0}.{1}", "a".repeat(63), "a".repeat(62));
    assert_eq!(long_name.len(), 254);
    assert_eq!(addr(&long_name), Err(AddressError::BadHostName));
    let longest = format!("{0}.{0}.{0}.{1}", "a".repeat(63), "a".repeat(61));
    assert_eq!(longest.len(), 253);
    assert!(addr(&longest).is_ok());
}

#[test]
fn empty_blank_long_and_spaced_text_has_its_own_error() {
    assert_eq!(addr(""), Err(AddressError::Empty));
    assert_eq!(addr("    "), Err(AddressError::Empty));
    assert_eq!(
        addr(&"a".repeat(MAX_ADDRESS_CHARS + 1)),
        Err(AddressError::TooLong)
    );
    assert_ne!(
        addr(&"a".repeat(MAX_ADDRESS_CHARS)),
        Err(AddressError::TooLong)
    );
    assert_eq!(addr("1.2.3.4 :80"), Err(AddressError::Whitespace));
    assert_eq!(
        addr("1.2.3.4\t"),
        Ok(v4(1, 2, 3, 4, DEFAULT_PORT)),
        "edges are trimmed"
    );
    assert_eq!(addr("a\u{0}b"), Err(AddressError::Whitespace));
    assert_eq!(
        addr("a\u{a0}b"),
        Err(AddressError::Whitespace),
        "a non-breaking space is a space"
    );
    for e in [
        AddressError::Empty,
        AddressError::TooLong,
        AddressError::Whitespace,
        AddressError::BadIpv6,
        AddressError::BadPort,
        AddressError::BadIpv4,
        AddressError::BadHostName,
    ] {
        assert!(e.message().len() > 10, "{e:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn any_text_is_read_or_refused_without_panicking(text in ".{0,400}") {
        let _ = parse_address(&text);
        let _ = normalize_invite(&text);
    }

    #[test]
    fn address_like_garbage_never_panics_and_what_is_accepted_is_sane(text in "[0-9a-fA-F.:\\[\\]xz -]{0,60}") {
        if let Ok(a) = parse_address(&text) {
            prop_assert!(a.port >= 1);
            if let HostSpec::Name(n) = a.host {
                prop_assert!(!n.is_empty() && n == n.to_ascii_lowercase());
                prop_assert!(!n.starts_with('-') && !n.ends_with('-') && !n.contains(".."));
            }
        }
    }

    #[test]
    fn a_formatted_ipv4_address_reads_back(a: u8, b: u8, c: u8, d: u8, port in 1u16..=65535) {
        prop_assert_eq!(parse_address(&format!("{a}.{b}.{c}.{d}:{port}")), Ok(v4(a, b, c, d, port)));
        prop_assert_eq!(parse_address(&format!("{a}.{b}.{c}.{d}")), Ok(v4(a, b, c, d, DEFAULT_PORT)));
    }

    #[test]
    fn a_formatted_ipv6_address_reads_back(bits: u128, port in 1u16..=65535) {
        let ip = Ipv6Addr::from(bits);
        prop_assert_eq!(
            parse_address(&format!("[{ip}]:{port}")),
            Ok(DirectAddress { host: HostSpec::Ipv6(ip), port })
        );
        prop_assert_eq!(
            parse_address(&ip.to_string()),
            Ok(DirectAddress { host: HostSpec::Ipv6(ip), port: DEFAULT_PORT })
        );
    }

    #[test]
    fn invite_shaped_garbage_is_normalised_once_and_for_all(text in "[0-9A-Za-z -]{0,80}") {
        if let Ok(code) = normalize_invite(&text) {
            // The written form reads back as itself.
            prop_assert_eq!(normalize_invite(&code), Ok(code.clone()));
            prop_assert!(code.len() == INVITE_LEN_V4 + 3 || code.len() == INVITE_LEN_V6 + 8);
            prop_assert!(code.split('-').all(|g| (1..=4).contains(&g.len())));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Validation: invite codes

/// Real codes, made by `ss2_net::discovery::encode_invite` for 192.168.1.5:27015 and [2001:db8::1]:1 (reproduced with a script that follows
/// `docs/DISCOVERY.md`, and checked against the real function in the differential run described in `docs/LOBBY_UI.md`).
const CODE_V4: &str = "2K0A-G085-D63K-T88";
const CODE_V6: &str = "2RG0-23DR-0000-0000-0000-0000-000G-00ET-Y0";

#[test]
fn real_invite_codes_pass_in_any_writing() {
    assert_eq!(normalize_invite(CODE_V4).as_deref(), Ok(CODE_V4));
    assert_eq!(normalize_invite(CODE_V6).as_deref(), Ok(CODE_V6));
    assert_eq!(
        normalize_invite(&CODE_V4.to_lowercase()).as_deref(),
        Ok(CODE_V4)
    );
    assert_eq!(normalize_invite("2k0ag085d63kt88").as_deref(), Ok(CODE_V4));
    assert_eq!(
        normalize_invite("  2K0A G085  D63K T88 ").as_deref(),
        Ok(CODE_V4)
    );
    assert_eq!(
        normalize_invite("2-K-0-A-G-0-8-5-D-6-3-K-T-8-8").as_deref(),
        Ok(CODE_V4)
    );
    // Look-alike letters are read as the digits.
    assert_eq!(
        normalize_invite("2KOA-GO85-D63K-T88").as_deref(),
        Ok(CODE_V4)
    );
    assert_eq!(
        normalize_invite("2RGO-23DR-OOOO-OOOO-OOOO-OOOO-OOOG-OOET-YO").as_deref(),
        Ok(CODE_V6)
    );
}

#[test]
fn invite_codes_are_refused_with_the_right_reason() {
    assert_eq!(normalize_invite(""), Err(InviteError::Empty));
    assert_eq!(normalize_invite(" - - "), Err(InviteError::Empty));
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-U88"),
        Err(InviteError::BadCharacter('U'))
    );
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-T8!"),
        Err(InviteError::BadCharacter('!'))
    );
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-T8\u{e9}"),
        Err(InviteError::BadCharacter('\u{e9}'))
    );
    assert_eq!(
        normalize_invite("2K0A"),
        Err(InviteError::Incomplete { have: 4 })
    );
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-T8"),
        Err(InviteError::Incomplete { have: 14 })
    );
    // Between the two lengths a code is still on its way to being an IPv6 one.
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-T88-0"),
        Err(InviteError::Incomplete { have: 16 })
    );
    assert_eq!(
        normalize_invite(&CODE_V6[..CODE_V6.len() - 1]),
        Err(InviteError::Incomplete { have: 33 })
    );
    assert_eq!(
        normalize_invite(&format!("{CODE_V6}0")),
        Err(InviteError::BadLength { have: 35 })
    );
    assert_eq!(
        normalize_invite(&"0".repeat(500)),
        Err(InviteError::BadLength {
            have: INVITE_LEN_V6 + 1
        })
    );
    // The padding bits of the last character must be zero: T88 -> T89 sets one.
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-T89"),
        Err(InviteError::BadEnding)
    );
    assert_eq!(
        normalize_invite("2K0A-G085-D63K-T8C"),
        Err(InviteError::BadEnding)
    );
    assert_eq!(
        normalize_invite("2RG0-23DR-0000-0000-0000-0000-000G-00ET-Y1"),
        Err(InviteError::BadEnding)
    );
    assert_eq!(
        normalize_invite("2RG0-23DR-0000-0000-0000-0000-000G-00ET-Y2"),
        Err(InviteError::BadEnding)
    );
    for e in [
        InviteError::Empty,
        InviteError::BadCharacter('x'),
        InviteError::Incomplete { have: 3 },
        InviteError::BadLength { have: 40 },
        InviteError::BadEnding,
    ] {
        assert!(e.message().len() > 10, "{e:?}");
    }
    assert!(InviteError::Incomplete { have: 3 }.message().contains('3'));
    assert!(InviteError::BadCharacter('x').message().contains('x'));
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Start screen

#[test]
fn the_start_screen_has_host_join_back_and_the_focus_wraps() {
    let mut l = lobby();
    assert_eq!(l.view(), LobbyView::Start);
    assert_eq!(l.focus(), ItemId::StartHost);
    let ids: Vec<ItemId> = l.items().iter().map(|i| i.id).collect();
    assert_eq!(
        ids,
        [ItemId::StartHost, ItemId::StartJoin, ItemId::StartBack]
    );
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::StartJoin);
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::StartBack);
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::StartHost, "down wraps");
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::StartBack, "up wraps");
}

#[test]
fn back_on_the_start_screen_asks_the_app_to_leave_and_the_other_screens_return_to_it() {
    let mut l = lobby();
    assert_eq!(nav(&mut l, NavInput::Back), Some(LobbyCommand::Back));
    assert_eq!(press(&mut l, ItemId::StartBack), Some(LobbyCommand::Back));
    to_setup(&mut l);
    assert_eq!(nav(&mut l, NavInput::Back), None);
    assert_eq!(l.view(), LobbyView::Start);
    assert_eq!(l.focus(), ItemId::StartHost);
    to_join(&mut l);
    assert_eq!(nav(&mut l, NavInput::Back), None);
    assert_eq!(l.view(), LobbyView::Start);
    to_join(&mut l);
    assert_eq!(press(&mut l, ItemId::JoinBack), None);
    assert_eq!(l.view(), LobbyView::Start);
    to_setup(&mut l);
    assert_eq!(press(&mut l, ItemId::HostBack), None);
    assert_eq!(l.view(), LobbyView::Start);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Host setup

#[test]
fn the_setup_screen_starts_with_the_defaults_and_they_make_good_settings() {
    let mut l = lobby();
    to_setup(&mut l);
    assert_eq!(l.focus(), ItemId::HostName);
    assert_eq!(l.field_text(ItemId::HostName), Some("SS2RS game"));
    assert_eq!(l.host_max_players(), 4);
    assert!(!l.host_password_on());
    assert_eq!(l.host_level(), 0);
    assert_eq!(
        l.host_settings(),
        Ok(HostSettings {
            name: "SS2RS game".into(),
            max_players: 4,
            password: None,
            level: "maps/hatshepsut".into()
        })
    );
}

#[test]
fn the_default_player_count_is_kept_inside_the_limits() {
    let cfg = LobbyConfig {
        default_max_players: 40,
        ..LobbyConfig::default()
    };
    assert_eq!(Lobby::new(cfg).host_max_players(), 16);
    let cfg = LobbyConfig {
        default_max_players: 1,
        ..LobbyConfig::default()
    };
    assert_eq!(Lobby::new(cfg).host_max_players(), 2);
}

#[test]
fn left_and_right_change_the_player_slots_inside_the_limits_and_activate_cycles() {
    let mut l = lobby();
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostMaxPlayers);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.host_max_players(), 5);
    for _ in 0..30 {
        nav(&mut l, NavInput::Right);
    }
    assert_eq!(l.host_max_players(), 16, "stops at the limit");
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    assert_eq!(
        l.host_max_players(),
        2,
        "activate wraps from the top to the bottom"
    );
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.host_max_players(), 3);
    for _ in 0..30 {
        nav(&mut l, NavInput::Left);
    }
    assert_eq!(l.host_max_players(), 2, "stops at the bottom");
    assert_eq!(l.focus(), ItemId::HostMaxPlayers, "the focus stays");
}

#[test]
fn the_password_field_is_skipped_until_the_switch_is_on_and_then_is_needed() {
    let mut l = lobby();
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostPasswordOn);
    nav(&mut l, NavInput::Down);
    assert_eq!(
        l.focus(),
        ItemId::HostLevel,
        "the disabled password field is skipped going down"
    );
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::HostPasswordOn, "and going up");
    nav(&mut l, NavInput::Activate);
    assert!(l.host_password_on());
    assert_eq!(
        l.host_settings(),
        Err("Type a password, or turn the password off")
    );
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::HostPassword);
    typ(&mut l, "secret");
    assert_eq!(
        l.host_settings().unwrap().password.as_deref(),
        Some("secret")
    );
    go_to(&mut l, ItemId::HostPasswordOn);
    nav(&mut l, NavInput::Activate);
    assert_eq!(
        l.host_settings().unwrap().password,
        None,
        "off again: no password is sent, though the text is kept"
    );
    assert_eq!(l.field_text(ItemId::HostPassword), Some("secret"));
}

#[test]
fn turning_the_switch_off_with_the_pointer_takes_the_focus_off_the_password_field() {
    let mut l = lobby();
    to_setup(&mut l);
    click(&mut l, ItemId::HostPasswordOn);
    assert!(l.host_password_on());
    click(&mut l, ItemId::HostPassword);
    assert_eq!(l.focus(), ItemId::HostPassword);
    click(&mut l, ItemId::HostPasswordOn);
    assert!(!l.host_password_on());
    assert_eq!(l.focus(), ItemId::HostPasswordOn);
}

#[test]
fn the_level_is_picked_with_left_right_and_activate_and_wraps() {
    let mut l = lobby();
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostLevel);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.host_settings().unwrap().level, "maps/jungle");
    nav(&mut l, NavInput::Right);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.host_level(), 0, "right wraps");
    nav(&mut l, NavInput::Left);
    assert_eq!(l.host_level(), 2, "left wraps");
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.host_level(), 0, "activate goes on");
}

#[test]
fn the_chosen_level_survives_a_new_list_that_still_has_it_and_resets_when_it_does_not() {
    let mut l = lobby();
    l.host.level = 1;
    l.set_levels(vec![
        level("maps/pyramid", "Pyramid"),
        level("maps/jungle", "The Jungle"),
    ]);
    assert_eq!(l.host_level(), 1, "still the jungle");
    l.set_levels(vec![level("maps/a", "A"), level("maps/b", "B")]);
    assert_eq!(l.host_level(), 0);
    l.set_levels(Vec::new());
    assert_eq!(l.host_level(), 0);
    assert!(l.levels().is_empty());
}

#[test]
fn without_levels_the_level_row_and_the_start_button_are_disabled_and_the_reason_shows() {
    let mut l = Lobby::new(LobbyConfig::default());
    to_setup(&mut l);
    assert!(!is_enabled(&l, ItemId::HostLevel));
    assert!(!is_enabled(&l, ItemId::HostStart));
    assert_eq!(l.host_settings(), Err("There are no levels to choose from"));
    assert!(has_text(&l, "There are no levels"));
    assert!(has_text(&l, "(no levels)"));
    go_to(&mut l, ItemId::HostMaxPlayers);
    nav(&mut l, NavInput::Down);
    nav(&mut l, NavInput::Down);
    assert_eq!(
        l.focus(),
        ItemId::HostBack,
        "down from the switch skips the password, the level and start"
    );
}

#[test]
fn the_server_name_is_trimmed_and_checked_in_bytes() {
    let mut l = lobby();
    to_setup(&mut l);
    for _ in 0..40 {
        assert_eq!(nav(&mut l, NavInput::Activate), None); // does nothing on a field
        l.handle(LobbyInput::Backspace);
    }
    assert_eq!(l.host_settings(), Err("Give the game a name"));
    typ(&mut l, "   ");
    assert_eq!(
        l.host_settings(),
        Err("Give the game a name"),
        "blank is empty"
    );
    typ(&mut l, "  Fun  ");
    assert_eq!(l.host_settings().unwrap().name, "Fun");
    // 17 two-byte letters are 17 characters (the field takes them) but 34 bytes (the protocol does not).
    l.handle(LobbyInput::Home);
    for _ in 0..20 {
        l.handle(LobbyInput::Delete);
    }
    typ(&mut l, &"\u{e9}".repeat(17));
    assert_eq!(
        l.field_text(ItemId::HostName).map(|t| t.chars().count()),
        Some(17)
    );
    assert_eq!(
        l.host_settings(),
        Err("The name is too long: 32 letters at most")
    );
    assert!(!is_enabled(&l, ItemId::HostStart));
    l.handle(LobbyInput::Backspace);
    assert!(
        l.host_settings().is_ok(),
        "16 two-byte letters are 32 bytes"
    );
    assert!(is_enabled(&l, ItemId::HostStart));
}

#[test]
fn the_name_field_edits_at_the_cursor_and_takes_at_most_32_characters() {
    let mut l = lobby();
    to_setup(&mut l);
    l.handle(LobbyInput::Home);
    for _ in 0..20 {
        l.handle(LobbyInput::Delete);
    }
    assert_eq!(l.field_text(ItemId::HostName), Some(""));
    typ(&mut l, "abcd");
    nav(&mut l, NavInput::Left);
    nav(&mut l, NavInput::Left);
    typ(&mut l, "XY");
    assert_eq!(l.field_text(ItemId::HostName), Some("abXYcd"));
    l.handle(LobbyInput::Backspace);
    assert_eq!(l.field_text(ItemId::HostName), Some("abXcd"));
    l.handle(LobbyInput::Delete);
    assert_eq!(l.field_text(ItemId::HostName), Some("abXd"));
    l.handle(LobbyInput::Home);
    typ(&mut l, ">");
    l.handle(LobbyInput::End);
    typ(&mut l, "<");
    assert_eq!(l.field_text(ItemId::HostName), Some(">abXd<"));
    nav(&mut l, NavInput::Right);
    typ(&mut l, &"z".repeat(100));
    assert_eq!(
        l.field_text(ItemId::HostName).map(|t| t.chars().count()),
        Some(MAX_SERVER_NAME_CHARS)
    );
    typ(&mut l, "line\nbreak\ttab");
    assert_eq!(
        l.field_text(ItemId::HostName).map(|t| t.chars().count()),
        Some(MAX_SERVER_NAME_CHARS)
    );
    l.handle(LobbyInput::Home);
    l.handle(LobbyInput::Delete);
    typ(&mut l, "a\nb");
    let text = l.field_text(ItemId::HostName).unwrap();
    assert!(text.starts_with('a'));
    assert!(!text.contains('\n'), "control characters are left out");
}

#[test]
fn typing_on_a_screen_without_a_focused_field_does_nothing() {
    let mut l = lobby();
    typ(&mut l, "hello");
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostMaxPlayers);
    typ(&mut l, "hello");
    l.handle(LobbyInput::Backspace);
    l.handle(LobbyInput::Delete);
    l.handle(LobbyInput::Home);
    l.handle(LobbyInput::End);
    assert_eq!(l.field_text(ItemId::HostName), Some("SS2RS game"));
    assert_eq!(l.host_max_players(), 4);
    assert_eq!(l.field_text(ItemId::HostMaxPlayers), None);
}

#[test]
fn starting_to_host_sends_the_settings_and_shows_the_connecting_screen() {
    let mut l = lobby();
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostPasswordOn);
    nav(&mut l, NavInput::Activate);
    nav(&mut l, NavInput::Down);
    typ(&mut l, "pw");
    go_to(&mut l, ItemId::HostLevel);
    nav(&mut l, NavInput::Right);
    go_to(&mut l, ItemId::HostMaxPlayers);
    nav(&mut l, NavInput::Left);
    let cmd = press(&mut l, ItemId::HostStart);
    assert_eq!(
        cmd,
        Some(LobbyCommand::Host(HostSettings {
            name: "SS2RS game".into(),
            max_players: 3,
            password: Some("pw".into()),
            level: "maps/jungle".into(),
        }))
    );
    assert_eq!(l.view(), LobbyView::Connecting);
    assert!(has_text(&l, "Starting SS2RS game"));
    // Cancelling goes back to the setup screen, with the form as it was.
    assert_eq!(nav(&mut l, NavInput::Activate), Some(LobbyCommand::Cancel));
    assert_eq!(l.view(), LobbyView::HostSetup);
    assert_eq!(l.host_max_players(), 3);
    assert_eq!(l.field_text(ItemId::HostPassword), Some("pw"));
}

#[test]
fn a_disabled_start_button_cannot_be_activated_by_the_pointer() {
    let mut l = Lobby::new(LobbyConfig::default());
    to_setup(&mut l);
    let r = rect_of(&l, ItemId::HostStart);
    let at = Vec2 {
        x: r.x + 5.0,
        y: r.y + 5.0,
    };
    assert_eq!(pointer(&mut l, UiInput::PointerPressed, at), None);
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, at), None);
    assert_eq!(l.view(), LobbyView::HostSetup);
    assert_ne!(l.focus(), ItemId::HostStart);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Join: tabs, direct address, invite code

#[test]
fn the_tabs_switch_with_left_and_right_and_stop_at_the_ends() {
    let mut l = lobby();
    to_join(&mut l);
    assert_eq!(l.route(), JoinRoute::Lan);
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Lan));
    nav(&mut l, NavInput::Left);
    assert_eq!(
        (l.route(), l.focus()),
        (JoinRoute::Lan, ItemId::JoinTab(JoinRoute::Lan))
    );
    nav(&mut l, NavInput::Right);
    assert_eq!(
        (l.route(), l.focus()),
        (JoinRoute::Direct, ItemId::JoinTab(JoinRoute::Direct))
    );
    nav(&mut l, NavInput::Right);
    assert_eq!(l.route(), JoinRoute::Invite);
    nav(&mut l, NavInput::Right);
    assert_eq!(
        (l.route(), l.focus()),
        (JoinRoute::Invite, ItemId::JoinTab(JoinRoute::Invite))
    );
    nav(&mut l, NavInput::Left);
    assert_eq!(l.route(), JoinRoute::Direct);
    assert_eq!(JoinRoute::Lan.label(), "LAN games");
    assert_eq!(JoinRoute::Direct.label(), "Direct address");
    assert_eq!(JoinRoute::Invite.label(), "Invite code");
}

#[test]
fn the_join_screen_opens_on_the_field_of_the_route_last_used() {
    let mut l = lobby();
    to_direct(&mut l);
    assert_eq!(nav(&mut l, NavInput::Back), None);
    to_join(&mut l);
    assert_eq!(l.focus(), ItemId::JoinAddress);
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Invite));
    nav(&mut l, NavInput::Activate);
    nav(&mut l, NavInput::Back);
    to_join(&mut l);
    assert_eq!(l.focus(), ItemId::JoinCode);
}

#[test]
fn typing_with_the_focus_on_a_tab_goes_to_the_field_of_a_typed_route_only() {
    let mut l = lobby();
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Lan));
    typ(&mut l, "ignored");
    assert_eq!(l.field_text(ItemId::JoinAddress), Some(""));
    assert_eq!(l.field_text(ItemId::JoinCode), Some(""));
    nav(&mut l, NavInput::Right);
    assert_eq!(l.focus(), ItemId::JoinTab(JoinRoute::Direct));
    typ(&mut l, "1.2.3.4");
    assert_eq!(l.focus(), ItemId::JoinAddress);
    assert_eq!(l.field_text(ItemId::JoinAddress), Some("1.2.3.4"));
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Direct));
    nav(&mut l, NavInput::Right);
    typ(&mut l, "2k0a");
    assert_eq!(l.focus(), ItemId::JoinCode);
    assert_eq!(l.field_text(ItemId::JoinCode), Some("2k0a"));
}

#[test]
fn the_connect_button_follows_the_text_and_its_problem_is_told() {
    let mut l = lobby();
    to_direct(&mut l);
    assert!(!is_enabled(&l, ItemId::JoinConnect));
    assert_eq!(
        l.join_problem().as_deref(),
        Some(AddressError::Empty.message())
    );
    typ(&mut l, "192.168.1.999");
    assert!(!is_enabled(&l, ItemId::JoinConnect));
    assert_eq!(
        l.join_problem().as_deref(),
        Some(AddressError::BadIpv4.message())
    );
    assert!(has_text(&l, AddressError::BadIpv4.message()));
    assert_eq!(
        nav(&mut l, NavInput::Activate),
        None,
        "enter on a bad address does nothing"
    );
    assert_eq!(l.view(), LobbyView::Join);
    l.handle(LobbyInput::Backspace);
    l.handle(LobbyInput::Backspace);
    assert!(is_enabled(&l, ItemId::JoinConnect));
    assert_eq!(l.join_problem(), None);
    assert!(has_text(&l, "OK: 192.168.1.9, port 27015"));
    // With the button enabled the focus visits it.
    nav(&mut l, NavInput::Down);
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::JoinConnect);
    nav(&mut l, NavInput::Up);
    nav(&mut l, NavInput::Up);
    // Making the text bad again takes the button out of the focus order.
    l.handle(LobbyInput::Backspace);
    assert!(!is_enabled(&l, ItemId::JoinConnect));
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::JoinPassword);
    nav(&mut l, NavInput::Down);
    assert_eq!(
        l.focus(),
        ItemId::JoinBack,
        "the disabled button is skipped"
    );
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::JoinPassword);
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::JoinAddress);
}

#[test]
fn a_direct_join_sends_the_address_read_and_the_password_when_there_is_one() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "Game.Example.com:7001");
    let cmd = nav(&mut l, NavInput::Activate);
    assert_eq!(
        cmd,
        Some(LobbyCommand::Join(JoinRequest {
            target: JoinTarget::Direct(DirectAddress {
                host: HostSpec::Name("game.example.com".into()),
                port: 7001
            }),
            password: None
        }))
    );
    assert_eq!(l.view(), LobbyView::Connecting);
    assert!(has_text(&l, "Connecting to Game.Example.com:7001"));
    assert_eq!(nav(&mut l, NavInput::Back), Some(LobbyCommand::Cancel));
    assert_eq!(l.view(), LobbyView::Join);
    assert_eq!(l.focus(), ItemId::JoinAddress);
    go_to(&mut l, ItemId::JoinPassword);
    typ(&mut l, "pass word");
    let cmd = press(&mut l, ItemId::JoinConnect);
    let Some(LobbyCommand::Join(req)) = cmd else {
        panic!("{cmd:?}")
    };
    assert_eq!(
        req.password.as_deref(),
        Some("pass word"),
        "a password may have spaces"
    );
}

#[test]
fn an_invite_join_sends_the_code_in_its_written_form() {
    let mut l = lobby();
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Invite));
    nav(&mut l, NavInput::Activate);
    typ(&mut l, "2k0a g085 d63k t88");
    assert!(has_text(&l, "OK: 2K0A-G085-D63K-T88"));
    let cmd = nav(&mut l, NavInput::Activate);
    assert_eq!(
        cmd,
        Some(LobbyCommand::Join(JoinRequest {
            target: JoinTarget::Invite(CODE_V4.into()),
            password: None
        }))
    );
    assert!(has_text(&l, "invite code"));
}

#[test]
fn a_partial_invite_code_says_how_far_it_is_and_cannot_be_sent() {
    let mut l = lobby();
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Invite));
    nav(&mut l, NavInput::Activate);
    typ(&mut l, "2K0A-G08");
    assert_eq!(
        l.join_problem(),
        Some(InviteError::Incomplete { have: 7 }.message())
    );
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    assert_eq!(l.view(), LobbyView::Join);
    typ(&mut l, "5-D63K-T8!");
    assert!(l.join_problem().unwrap().contains('!'));
    assert!(l.join_request().is_err());
    assert!(has_text(&l, "'!' is not used in invite codes"));
}

#[test]
fn the_lan_route_has_no_typed_request() {
    let l = lobby();
    assert!(l.join_request().is_err());
    assert_eq!(l.join_problem(), None);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Join: the LAN browser

fn ids(l: &Lobby) -> Vec<String> {
    l.sorted_servers().iter().map(|s| s.id.clone()).collect()
}

#[test]
fn servers_are_sorted_by_ping_then_name_then_players_and_the_order_is_stable() {
    let mut l = lobby();
    l.set_servers(vec![
        open("a", "Bravo", 50),
        srv("b", "alpha", 3, 4, None, false, ServerStatus::Open),
        open("c", "Charlie", 10),
        srv("d", "delta", 4, 4, Some(50), false, ServerStatus::Full),
        srv("e", "Echo", 0, 4, Some(5), false, ServerStatus::Open),
    ]);
    assert_eq!(l.sort(), ServerSort::Ping);
    assert_eq!(
        ids(&l),
        ["e", "c", "a", "d", "b"],
        "fastest first, equal pings keep their order, none last"
    );
    l.set_sort(ServerSort::Name);
    assert_eq!(
        ids(&l),
        ["b", "a", "c", "d", "e"],
        "names without regard to case"
    );
    l.set_sort(ServerSort::Players);
    assert_eq!(
        ids(&l),
        ["d", "b", "a", "c", "e"],
        "the fullest first, equal counts keep their order"
    );
    assert_eq!(l.servers().len(), 5);
    assert_eq!(l.servers()[0].id, "a", "the list as given is kept");
    assert_eq!(ServerSort::Ping.label(), "Ping");
    assert_eq!(ServerSort::Name.label(), "Name");
    assert_eq!(ServerSort::Players.label(), "Players");
}

#[test]
fn the_sort_button_goes_round_and_refresh_asks_for_a_scan() {
    let mut l = lobby();
    to_join(&mut l);
    assert_eq!(press(&mut l, ItemId::JoinSort), None);
    assert_eq!(l.sort(), ServerSort::Name);
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.sort(), ServerSort::Players);
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.sort(), ServerSort::Ping);
    assert_eq!(
        press(&mut l, ItemId::JoinRefresh),
        Some(LobbyCommand::RefreshServers)
    );
    assert_eq!(l.view(), LobbyView::Join);
    assert!(has_text(&l, "Sort by: Ping"));
}

#[test]
fn the_focus_skips_full_and_incompatible_servers_and_they_cannot_be_joined() {
    let mut l = lobby();
    l.set_servers(vec![
        srv("full", "Full", 4, 4, Some(1), false, ServerStatus::Full),
        open("ok1", "First", 20),
        srv(
            "old",
            "Old",
            1,
            4,
            Some(30),
            false,
            ServerStatus::Incompatible("older game".into()),
        ),
        open("ok2", "Second", 40),
    ]);
    to_join(&mut l);
    assert_eq!(
        l.focus(),
        ItemId::JoinServer(1),
        "the first open server is focused on entering"
    );
    nav(&mut l, NavInput::Down);
    assert_eq!(
        l.focus(),
        ItemId::JoinServer(3),
        "the incompatible one is skipped"
    );
    nav(&mut l, NavInput::Up);
    nav(&mut l, NavInput::Up);
    assert_eq!(
        l.focus(),
        ItemId::JoinSort,
        "going up skips the full one too"
    );
    // The pointer cannot reach a disabled row either.
    let r = rect_of(&l, ItemId::JoinServer(0));
    let at = Vec2 {
        x: r.x + 10.0,
        y: r.y + 10.0,
    };
    pointer(&mut l, UiInput::PointerMoved, at);
    assert_eq!(l.focus(), ItemId::JoinSort);
    assert_eq!(pointer(&mut l, UiInput::PointerPressed, at), None);
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, at), None);
    assert_eq!(l.view(), LobbyView::Join);
    assert!(has_text(&l, "Full") && has_text(&l, "older game"));
}

#[test]
fn joining_a_listed_server_sends_its_key_and_the_password_typed() {
    let mut l = lobby();
    l.set_servers(vec![srv(
        "10.0.0.5:27015",
        "Lock",
        1,
        4,
        Some(5),
        true,
        ServerStatus::Open,
    )]);
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinPassword);
    typ(&mut l, "hunter2");
    let cmd = press(&mut l, ItemId::JoinServer(0));
    assert_eq!(
        cmd,
        Some(LobbyCommand::Join(JoinRequest {
            target: JoinTarget::Server("10.0.0.5:27015".into()),
            password: Some("hunter2".into())
        }))
    );
    assert_eq!(l.view(), LobbyView::Connecting);
    assert!(has_text(&l, "Connecting to Lock"));
}

#[test]
fn a_click_on_a_row_joins_it() {
    let mut l = lobby();
    l.set_servers(vec![open("a", "A", 10), open("b", "B", 20)]);
    to_join(&mut l);
    assert_eq!(
        click(&mut l, ItemId::JoinServer(1)),
        Some(LobbyCommand::Join(JoinRequest {
            target: JoinTarget::Server("b".into()),
            password: None
        }))
    );
}

fn many_servers(n: usize) -> Vec<ServerRow> {
    (0..n)
        .map(|i| open(&format!("s{i:02}"), &format!("Game {i:02}"), i as u32 + 1))
        .collect()
}

#[test]
fn a_long_list_scrolls_with_the_focus_and_the_focused_row_is_always_drawn() {
    let mut l = lobby();
    l.set_servers(many_servers(30));
    to_join(&mut l);
    assert_eq!(l.focus(), ItemId::JoinServer(0));
    assert_eq!(l.server_scroll(), 0);
    let mut last_scroll = 0;
    for k in 1..30 {
        nav(&mut l, NavInput::Down);
        assert_eq!(l.focus(), ItemId::JoinServer(k));
        let scroll = l.server_scroll();
        assert!(scroll >= last_scroll, "only ever scrolls down");
        assert_eq!(
            scroll,
            (k + 1).saturating_sub(VISIBLE_SERVERS),
            "the window moves by one row when the focus passes its edge"
        );
        assert!(
            l.items()
                .iter()
                .find(|i| i.id == ItemId::JoinServer(k))
                .unwrap()
                .rect
                .is_some()
        );
        last_scroll = scroll;
    }
    assert_eq!(l.server_scroll(), 24, "the last window is full");
    let rows_drawn = l
        .items()
        .iter()
        .filter(|i| matches!(i.id, ItemId::JoinServer(_)) && i.rect.is_some())
        .count();
    assert_eq!(rows_drawn, VISIBLE_SERVERS);
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::JoinPassword);
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::JoinServer(29));
    for k in (0..29).rev() {
        nav(&mut l, NavInput::Up);
        assert_eq!(l.focus(), ItemId::JoinServer(k));
        assert!(l.server_scroll() <= k, "the focus is not above the window");
    }
    assert_eq!(l.server_scroll(), 0);
    assert!(has_text(&l, "30 games found (showing 1-6)"));
}

#[test]
fn up_from_the_top_of_a_long_list_and_down_to_wrap_land_on_the_ends() {
    let mut l = lobby();
    l.set_servers(many_servers(10));
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinBack);
    nav(&mut l, NavInput::Down);
    assert_eq!(
        l.focus(),
        ItemId::JoinTab(JoinRoute::Lan),
        "down from the last item wraps to the first"
    );
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::JoinBack);
    nav(&mut l, NavInput::Up);
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::JoinServer(9));
    assert_eq!(l.server_scroll(), 4);
}

#[test]
fn the_scroll_follows_a_list_that_shrinks() {
    let mut l = lobby();
    l.set_servers(many_servers(30));
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinServer(29));
    assert_eq!(l.server_scroll(), 24);
    l.set_servers(many_servers(8));
    assert!(l.server_scroll() <= 2);
    assert_eq!(
        l.focus(),
        ItemId::JoinServer(7),
        "the nearest row of the same list"
    );
    l.set_servers(Vec::new());
    assert_eq!(l.server_scroll(), 0);
    assert!(is_enabled(&l, l.focus()));
}

#[test]
fn the_pointer_reaches_only_the_rows_in_the_window() {
    let mut l = lobby();
    l.set_servers(many_servers(30));
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinServer(10));
    // Row 3 is scrolled out (the window is 5..=10): no rectangle, so no hit.
    assert!(
        l.items()
            .iter()
            .find(|i| i.id == ItemId::JoinServer(3))
            .unwrap()
            .rect
            .is_none()
    );
    let visible = rect_of(&l, ItemId::JoinServer(7));
    let at = Vec2 {
        x: visible.x + 5.0,
        y: visible.y + 5.0,
    };
    pointer(&mut l, UiInput::PointerMoved, at);
    assert_eq!(l.focus(), ItemId::JoinServer(7));
}

fn focus_index(l: &Lobby) -> usize {
    match l.focus() {
        ItemId::JoinServer(i) => i,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_focus_stays_on_the_same_server_when_the_list_is_replaced_or_re_sorted() {
    let mut l = lobby();
    l.set_servers(vec![
        open("a", "A", 10),
        open("b", "B", 20),
        open("c", "C", 30),
    ]);
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinServer(1));
    // A faster server appears and pushes B to the third place.
    l.set_servers(vec![
        open("a", "A", 10),
        open("n", "N", 5),
        open("b", "B", 20),
        open("c", "C", 30),
    ]);
    assert_eq!(l.focus(), ItemId::JoinServer(2));
    assert_eq!(l.sorted_servers()[2].id, "b");
    // Sorting by name puts B second.
    l.set_sort(ServerSort::Name);
    assert_eq!(l.sorted_servers()[focus_index(&l)].id, "b");
    assert_eq!(focus_index(&l), 1);
    // The sort button does the same.
    nav(&mut l, NavInput::Up);
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::JoinSort);
    nav(&mut l, NavInput::Down);
    nav(&mut l, NavInput::Down);
    assert_eq!(l.focus(), ItemId::JoinServer(1));
    nav(&mut l, NavInput::Activate); // joins b
    assert_eq!(l.view(), LobbyView::Connecting);
}

#[test]
fn when_the_focused_server_disappears_the_focus_moves_to_a_neighbour() {
    let mut l = lobby();
    l.set_servers(vec![
        open("a", "A", 10),
        open("b", "B", 20),
        open("c", "C", 30),
    ]);
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinServer(1));
    l.set_servers(vec![open("a", "A", 10), open("c", "C", 30)]);
    assert_eq!(l.focus(), ItemId::JoinServer(1));
    assert_eq!(l.sorted_servers()[1].id, "c");
    // The last one goes: the focus falls back.
    l.set_servers(vec![open("a", "A", 10)]);
    assert_eq!(l.focus(), ItemId::JoinServer(0));
}

#[test]
fn the_empty_list_says_what_to_do_and_searching_is_told() {
    let mut l = lobby();
    to_join(&mut l);
    assert!(has_text(&l, "No games found on your network"));
    assert!(has_text(&l, "Press Refresh"));
    l.set_scanning(true);
    assert!(l.scanning());
    assert!(has_text(&l, "Searching the local network..."));
    assert!(!has_text(&l, "No games found"));
    l.set_servers(vec![open("a", "A", 10)]);
    assert!(!has_text(&l, "Press Refresh"));
    l.set_scanning(false);
    assert!(has_text(&l, "1 game found"));
    l.set_servers(vec![open("a", "A", 10), open("b", "B", 10)]);
    assert!(has_text(&l, "2 games found"));
}

#[test]
fn the_rows_show_name_players_ping_and_the_lock() {
    let mut l = lobby();
    l.set_servers(vec![
        srv("a", "Alpha", 2, 4, Some(23), true, ServerStatus::Open),
        srv("b", "Beta", 4, 4, Some(2000), false, ServerStatus::Full),
        srv("c", "Gamma", 0, 8, None, false, ServerStatus::Open),
    ]);
    to_join(&mut l);
    for needle in [
        "Alpha", "2/4", "23 ms", "Locked", "Beta", "4/4", "999+ ms", "Full", "Gamma", "0/8",
        "-- ms",
    ] {
        assert!(has_text(&l, needle), "{needle}: {:?}", texts(&l));
    }
    assert_eq!(texts(&l).iter().filter(|t| *t == "Locked").count(), 1);
}

#[test]
fn a_very_long_server_name_is_cut_and_the_scroll_bar_appears_only_for_a_long_list() {
    let mut l = lobby();
    l.set_servers(vec![open("a", &"N".repeat(200), 1)]);
    to_join(&mut l);
    assert!(texts(&l).iter().all(|t| t.chars().count() < 100));
    l.set_servers(many_servers(7));
    assert!(
        l.draw().quads.iter().any(|q| q.color == theme::TRACK_FILL),
        "a scroll thumb"
    );
    l.set_servers(many_servers(6));
    assert!(
        l.draw().quads.iter().all(|q| q.color != theme::TRACK_FILL),
        "no thumb when everything fits"
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Pointer rules (the menu's)

#[test]
fn hovering_focuses_pressing_and_releasing_on_the_same_item_activates() {
    let mut l = lobby();
    let join = center(&l, ItemId::StartJoin);
    pointer(&mut l, UiInput::PointerMoved, join);
    assert_eq!(l.focus(), ItemId::StartJoin);
    assert_eq!(pointer(&mut l, UiInput::PointerPressed, join), None);
    assert_eq!(l.view(), LobbyView::Start, "a press alone does nothing");
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, join), None);
    assert_eq!(l.view(), LobbyView::Join, "the release opens the screen");
}

#[test]
fn a_press_released_over_another_item_or_nothing_does_nothing() {
    let mut l = lobby();
    let host = center(&l, ItemId::StartHost);
    let back = center(&l, ItemId::StartBack);
    pointer(&mut l, UiInput::PointerPressed, host);
    assert_eq!(l.focus(), ItemId::StartHost);
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, back), None);
    assert_eq!(l.view(), LobbyView::Start);
    pointer(&mut l, UiInput::PointerPressed, back);
    assert_eq!(
        pointer(&mut l, UiInput::PointerReleased, Vec2 { x: 1.0, y: 1.0 }),
        None
    );
    // A release with no press (the press began off the screen) does nothing either.
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, back), None);
    // Pressing on empty space forgets an earlier press.
    pointer(&mut l, UiInput::PointerPressed, back);
    pointer(&mut l, UiInput::PointerPressed, Vec2 { x: 1.0, y: 1.0 });
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, back), None);
}

#[test]
fn pressing_on_a_field_only_focuses_it() {
    let mut l = lobby();
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostMaxPlayers);
    assert_eq!(click(&mut l, ItemId::HostName), None);
    assert_eq!(l.focus(), ItemId::HostName);
    assert_eq!(l.view(), LobbyView::HostSetup);
    // A click on a value row of the form acts: it is a button.
    click(&mut l, ItemId::HostMaxPlayers);
    assert_eq!(l.host_max_players(), 5);
    click(&mut l, ItemId::HostPasswordOn);
    assert!(l.host_password_on());
}

#[test]
fn the_edges_of_a_row_are_half_open_like_the_menus() {
    let mut l = lobby();
    let r = rect_of(&l, ItemId::StartBack);
    pointer(&mut l, UiInput::PointerMoved, Vec2 { x: r.x, y: r.y });
    assert_eq!(
        l.focus(),
        ItemId::StartBack,
        "the left and top edges are inside"
    );
    let host = center(&l, ItemId::StartHost);
    pointer(&mut l, UiInput::PointerMoved, host);
    assert_eq!(l.focus(), ItemId::StartHost);
    pointer(
        &mut l,
        UiInput::PointerMoved,
        Vec2 {
            x: r.x + r.w,
            y: r.y + 1.0,
        },
    );
    assert_eq!(l.focus(), ItemId::StartHost, "the right edge is outside");
}

#[test]
fn input_from_the_menu_converts() {
    let ui = UiInput::Nav(NavInput::Up);
    assert_eq!(LobbyInput::from(ui), LobbyInput::Ui(ui));
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Connecting

#[test]
fn the_connecting_screen_gives_up_after_the_timeout_and_offers_to_try_again() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9:5000");
    let join = nav(&mut l, NavInput::Activate);
    assert!(matches!(join, Some(LobbyCommand::Join(_))));
    assert_eq!(l.update(9_999), None);
    assert_eq!(l.view(), LobbyView::Connecting);
    assert!(has_text(&l, "gives up in 1 s"));
    assert_eq!(l.update(1), Some(LobbyCommand::Cancel));
    assert_eq!(l.view(), LobbyView::Error);
    assert_eq!(l.failure(), Some(&Failure::TimedOut));
    assert!(has_text(&l, "No answer"));
    let ids: Vec<ItemId> = l.items().iter().map(|i| i.id).collect();
    assert_eq!(ids, [ItemId::ErrorRetry, ItemId::ErrorBack]);
    // Try again sends the same request, and the clock starts over.
    assert_eq!(press(&mut l, ItemId::ErrorRetry), join);
    assert_eq!(l.view(), LobbyView::Connecting);
    assert_eq!(l.update(9_999), None);
    // Back from the error goes to the form.
    assert_eq!(l.update(1), Some(LobbyCommand::Cancel));
    assert_eq!(press(&mut l, ItemId::ErrorBack), None);
    assert_eq!(l.view(), LobbyView::Join);
    assert_eq!(l.failure(), None);
}

#[test]
fn the_timeout_is_the_configured_one() {
    let mut l = Lobby::new(LobbyConfig {
        connect_timeout_ms: 2_500,
        ..LobbyConfig::default()
    });
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9");
    nav(&mut l, NavInput::Activate);
    assert!(has_text(&l, "gives up in 3 s"));
    assert_eq!(l.update(2_499), None);
    assert_eq!(l.update(1), Some(LobbyCommand::Cancel));
}

#[test]
fn only_the_connecting_screen_has_a_timeout() {
    let mut l = lobby();
    assert_eq!(l.update(1_000_000), None);
    to_setup(&mut l);
    assert_eq!(l.update(1_000_000), None);
    assert_eq!(l.view(), LobbyView::HostSetup);
}

#[test]
fn loading_progress_resets_the_clock_shows_the_step_and_clamps_the_percent() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9");
    nav(&mut l, NavInput::Activate);
    l.update(9_000);
    l.set_connection_state(ConnectionState::Loading {
        what: "Loading the level".into(),
        percent: Some(40),
    });
    assert_eq!(l.update(9_000), None, "progress counts as an answer");
    assert!(has_text(&l, "Loading the level: 40%"));
    assert!(has_text(&l, "gives up in 1 s"));
    l.set_connection_state(ConnectionState::Loading {
        what: "Receiving the game".into(),
        percent: Some(250),
    });
    assert!(has_text(&l, "Receiving the game: 100%"));
    l.set_connection_state(ConnectionState::Loading {
        what: "Waiting".into(),
        percent: None,
    });
    assert!(has_text(&l, "Waiting"));
    assert!(!has_text(&l, "Waiting:"));
    // A plain Connecting does not reset the clock.
    l.update(9_000);
    l.set_connection_state(ConnectionState::Connecting);
    assert_eq!(l.update(1_000), Some(LobbyCommand::Cancel));
}

#[test]
fn the_progress_bar_fills_with_the_percent() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9");
    nav(&mut l, NavInput::Activate);
    assert!(
        l.draw().quads.iter().all(|q| q.color != theme::TRACK_FILL),
        "no bar before there is a step"
    );
    l.set_connection_state(ConnectionState::Loading {
        what: "x".into(),
        percent: Some(25),
    });
    let fill = l
        .draw()
        .quads
        .into_iter()
        .find(|q| q.color == theme::TRACK_FILL)
        .unwrap();
    assert!((fill.rect.w - 200.0).abs() < 0.01, "{}", fill.rect.w);
    l.set_connection_state(ConnectionState::Loading {
        what: "x".into(),
        percent: None,
    });
    assert!(l.draw().quads.iter().all(|q| q.color != theme::TRACK_FILL));
}

#[test]
fn the_dots_move_with_the_clock() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9");
    nav(&mut l, NavInput::Activate);
    let line = |l: &Lobby| {
        texts(l)
            .into_iter()
            .find(|t| t.starts_with("Waiting for a reply"))
            .unwrap()
    };
    assert_eq!(line(&l), "Waiting for a reply");
    l.update(400);
    assert_eq!(line(&l), "Waiting for a reply.");
    l.update(800);
    assert_eq!(line(&l), "Waiting for a reply...");
    l.update(400);
    assert_eq!(line(&l), "Waiting for a reply");
}

#[test]
fn an_answer_that_comes_after_cancel_is_ignored() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9");
    nav(&mut l, NavInput::Activate);
    assert_eq!(nav(&mut l, NavInput::Activate), Some(LobbyCommand::Cancel));
    assert_eq!(l.view(), LobbyView::Join);
    l.set_connection_state(ConnectionState::Connected);
    assert_eq!(l.view(), LobbyView::Join);
    l.set_connection_state(ConnectionState::Failed(Failure::TimedOut));
    assert_eq!(l.view(), LobbyView::Join);
    l.set_connection_state(ConnectionState::Loading {
        what: "x".into(),
        percent: None,
    });
    assert_eq!(l.view(), LobbyView::Join);
}

#[test]
fn a_refusal_shows_an_error_with_what_to_do_and_a_wrong_password_goes_back_to_the_password() {
    let mut l = lobby();
    l.set_servers(vec![srv(
        "a",
        "Locked",
        1,
        4,
        Some(5),
        true,
        ServerStatus::Open,
    )]);
    to_join(&mut l);
    press(&mut l, ItemId::JoinServer(0));
    l.set_connection_state(ConnectionState::Failed(Failure::BadPassword));
    assert_eq!(l.view(), LobbyView::Error);
    assert!(has_text(&l, "Wrong password"));
    assert!(has_text(&l, "type the password again"));
    let ids: Vec<ItemId> = l.items().iter().map(|i| i.id).collect();
    assert_eq!(
        ids,
        [ItemId::ErrorBack],
        "sending the same wrong password again is no help"
    );
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    assert_eq!(l.view(), LobbyView::Join);
    assert_eq!(l.focus(), ItemId::JoinPassword);
}

#[test]
fn a_failed_host_goes_back_to_the_setup_screen_and_can_try_again() {
    let mut l = lobby();
    to_setup(&mut l);
    let cmd = press(&mut l, ItemId::HostStart);
    assert!(matches!(cmd, Some(LobbyCommand::Host(_))));
    l.set_connection_state(ConnectionState::Failed(Failure::Other(
        "The port 27015 is already in use".into(),
    )));
    assert!(has_text(&l, "The port 27015 is already in use"));
    assert_eq!(press(&mut l, ItemId::ErrorRetry), cmd);
    assert_eq!(l.view(), LobbyView::Connecting);
    l.set_connection_state(ConnectionState::Failed(Failure::Other("again".into())));
    assert_eq!(press(&mut l, ItemId::ErrorBack), None);
    assert_eq!(l.view(), LobbyView::HostSetup);
    assert_eq!(l.focus(), ItemId::HostName);
}

#[test]
fn every_failure_has_a_title_a_message_and_a_hint_and_the_retry_rules_hold() {
    let all = [
        (Failure::TimedOut, true),
        (Failure::Unreachable, true),
        (Failure::HostNotFound, false),
        (Failure::InvalidInvite, false),
        (Failure::BadPassword, false),
        (Failure::ServerFull, true),
        (Failure::Banned, false),
        (Failure::NameTaken, false),
        (Failure::InvalidName, false),
        (
            Failure::Incompatible("The host runs a newer version".into()),
            false,
        ),
        (Failure::RetryLater { seconds: 30 }, false),
        (Failure::Kicked("afk".into()), false),
        (Failure::HostLeft, false),
        (Failure::ConnectionLost, true),
        (Failure::Other("an odd thing".into()), true),
    ];
    let mut titles = std::collections::BTreeSet::new();
    for (f, retry) in all {
        assert!(
            f.title().len() > 5 && f.message().len() > 5 && f.hint().len() > 15,
            "{f:?}"
        );
        assert_eq!(f.can_retry(), retry, "{f:?}");
        titles.insert(f.title());
    }
    assert_eq!(titles.len(), 15, "no two share a title");
    assert!(
        Failure::RetryLater { seconds: 30 }
            .message()
            .contains("30 seconds")
    );
    assert_eq!(
        Failure::Kicked(String::new()).message(),
        "The host removed you from the game."
    );
    assert!(Failure::Kicked("afk".into()).message().ends_with("afk"));
    assert_eq!(Failure::Incompatible("why".into()).message(), "why");
}

#[test]
fn a_long_error_message_is_wrapped_and_cut_to_fit() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.9");
    nav(&mut l, NavInput::Activate);
    l.set_connection_state(ConnectionState::Failed(Failure::Other("word ".repeat(300))));
    let list = l.draw();
    assert!(list.texts.len() <= 1 + 4 + 4 + 2, "{}", list.texts.len());
    assert!(list.texts.iter().all(|t| t.text.chars().count() <= 90));
    assert!(
        list.texts
            .iter()
            .filter(|t| t.size < 36.0)
            .all(|t| t.pos.y < 700.0)
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The lobby

#[test]
fn connected_opens_the_lobby_and_the_focus_lands_on_a_usable_item() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(None, false), me(None, false)]);
    assert_eq!(l.view(), LobbyView::Lobby);
    assert!(
        !is_enabled(&l, ItemId::LobbyReady),
        "a character comes first"
    );
    assert!(is_enabled(&l, ItemId::LobbyCharacter(CharacterId(0))));
    assert!(matches!(l.focus(), ItemId::LobbyCharacter(_)));
    assert!(!l.is_host());
    assert!(has_text(&l, "Waiting for the host"));
}

#[test]
fn ready_needs_a_character_when_there_are_some_and_toggles_with_the_session_told() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(None, false)]);
    assert!(!is_enabled(&l, ItemId::LobbyReady));
    // A click focuses and activates: it chose Sam.
    assert_eq!(
        click(&mut l, ItemId::LobbyCharacter(CharacterId(0))),
        Some(LobbyCommand::SelectCharacter(CharacterId(0)))
    );
    assert!(
        is_enabled(&l, ItemId::LobbyReady),
        "the choice enables it at once, before the session confirms"
    );
    assert_eq!(
        press(&mut l, ItemId::LobbyReady),
        Some(LobbyCommand::SetReady(true))
    );
    assert!(local_ready(&l), "shown at once");
    assert!(has_exact(&l, "Not ready"), "the host is not ready");
    assert_eq!(
        nav(&mut l, NavInput::Activate),
        Some(LobbyCommand::SetReady(false))
    );
    assert!(!local_ready(&l));
    // The session's word wins.
    nav(&mut l, NavInput::Activate);
    assert!(local_ready(&l));
    l.set_players(vec![host_row(Some(1), false), me(Some(0), false)]);
    assert!(!local_ready(&l));
}

#[test]
fn ready_needs_no_character_when_there_are_none_to_choose() {
    let mut l = lobby();
    l.set_characters(Vec::new());
    enter_lobby(&mut l, vec![host_row(None, false), me(None, false)]);
    assert!(is_enabled(&l, ItemId::LobbyReady));
    assert_eq!(
        press(&mut l, ItemId::LobbyReady),
        Some(LobbyCommand::SetReady(true))
    );
    assert!(has_text(&l, "No characters to choose from"));
}

#[test]
fn ready_is_disabled_when_the_session_has_not_told_of_the_local_player() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(0), false)]);
    assert!(!is_enabled(&l, ItemId::LobbyReady));
    assert_eq!(l.local_character(), None);
}

#[test]
fn choosing_a_character_tells_the_session_and_is_shown_until_it_confirms() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(None, false)]);
    assert_eq!(
        press(&mut l, ItemId::LobbyCharacter(CharacterId(2))),
        Some(LobbyCommand::SelectCharacter(CharacterId(2)))
    );
    assert_eq!(l.local_character(), Some(CharacterId(2)));
    assert_eq!(
        l.players().iter().find(|p| p.local).unwrap().character,
        Some(CharacterId(2))
    );
    assert!(has_text(&l, "Yours"));
    // Choosing it again is nothing to tell.
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    // The session confirms.
    l.set_players(vec![host_row(Some(1), false), me(Some(2), false)]);
    assert_eq!(l.local_character(), Some(CharacterId(2)));
    assert_eq!(l.notice(), None);
    assert_eq!(
        nav(&mut l, NavInput::Activate),
        None,
        "still the same character"
    );
}

#[test]
fn a_character_another_player_has_is_disabled_and_skipped_and_cannot_be_clicked() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(None, false)]);
    assert!(!is_enabled(&l, ItemId::LobbyCharacter(CharacterId(1))));
    assert_eq!(
        l.taken_by(CharacterId(1)).map(|p| p.name.as_str()),
        Some("Host")
    );
    assert_eq!(l.taken_by(CharacterId(0)), None);
    go_to(&mut l, ItemId::LobbyCharacter(CharacterId(0)));
    nav(&mut l, NavInput::Down);
    assert_eq!(
        l.focus(),
        ItemId::LobbyCharacter(CharacterId(2)),
        "Mia (taken) is skipped"
    );
    nav(&mut l, NavInput::Up);
    assert_eq!(l.focus(), ItemId::LobbyCharacter(CharacterId(0)));
    let r = rect_of(&l, ItemId::LobbyCharacter(CharacterId(1)));
    let at = Vec2 {
        x: r.x + 5.0,
        y: r.y + 5.0,
    };
    pointer(&mut l, UiInput::PointerMoved, at);
    assert_eq!(l.focus(), ItemId::LobbyCharacter(CharacterId(0)));
    pointer(&mut l, UiInput::PointerPressed, at);
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, at), None);
    assert!(has_text(&l, "Taken by Host"));
}

#[test]
fn a_character_taken_while_the_player_is_choosing_drops_the_choice_and_says_so() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(None, false), me(None, false)]);
    assert_eq!(
        press(&mut l, ItemId::LobbyCharacter(CharacterId(2))),
        Some(LobbyCommand::SelectCharacter(CharacterId(2)))
    );
    assert_eq!(l.local_character(), Some(CharacterId(2)));
    // Before the session answered, the host took Zed.
    l.set_players(vec![host_row(Some(2), false), me(None, false)]);
    assert_eq!(l.local_character(), None, "the choice is gone");
    assert_eq!(l.notice(), Some("Zed was taken by Host: choose another"));
    assert!(has_text(&l, "Zed was taken by Host"));
    assert!(!is_enabled(&l, ItemId::LobbyReady));
    assert!(!is_enabled(&l, ItemId::LobbyCharacter(CharacterId(2))));
    assert_eq!(
        l.focus(),
        ItemId::LobbyCharacter(CharacterId(3)),
        "the focus left the taken character for the next"
    );
    // The next choice clears the line.
    assert_eq!(
        nav(&mut l, NavInput::Activate),
        Some(LobbyCommand::SelectCharacter(CharacterId(3)))
    );
    assert_eq!(l.notice(), None);
}

#[test]
fn a_pending_choice_that_is_no_longer_offered_is_dropped() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(None, false), me(None, false)]);
    press(&mut l, ItemId::LobbyCharacter(CharacterId(3)));
    assert_eq!(l.local_character(), Some(CharacterId(3)));
    l.set_characters(vec![ch(0, "Sam")]);
    assert_eq!(l.local_character(), None);
    assert_eq!(l.characters().len(), 1);
    press(&mut l, ItemId::LobbyCharacter(CharacterId(0)));
    l.set_characters(vec![ch(0, "Sam"), ch(2, "Zed")]);
    assert_eq!(l.local_character(), Some(CharacterId(0)), "still on offer");
}

#[test]
fn the_focus_moves_off_a_character_that_becomes_taken_to_the_nearest_free_one() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(None, false), me(None, false)]);
    go_to(&mut l, ItemId::LobbyCharacter(CharacterId(1)));
    l.set_players(vec![host_row(Some(1), false), me(None, false)]);
    assert_eq!(
        l.focus(),
        ItemId::LobbyCharacter(CharacterId(2)),
        "the next row slides into the place"
    );
    // The last character taken while focused: nearest on the other side.
    go_to(&mut l, ItemId::LobbyCharacter(CharacterId(3)));
    l.set_players(vec![
        host_row(Some(1), false),
        pl(5, "Other", Some(3), false, false, false),
        me(None, false),
    ]);
    assert_eq!(l.focus(), ItemId::LobbyCharacter(CharacterId(2)));
}

#[test]
fn changing_character_clears_the_ready_flag_and_tells_the_session_the_new_character() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), true)]);
    assert!(local_ready(&l));
    let cmd = press(&mut l, ItemId::LobbyCharacter(CharacterId(3)));
    assert_eq!(cmd, Some(LobbyCommand::SelectCharacter(CharacterId(3))));
    let mine = l.players().into_iter().find(|p| p.local).unwrap();
    assert!(!mine.ready, "ready is cleared by the change");
    assert_eq!(mine.character, Some(CharacterId(3)));
    // A press on the same character changes nothing and does not clear ready.
    l.set_players(vec![host_row(Some(1), false), me(Some(3), true)]);
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    assert!(local_ready(&l));
}

#[test]
fn start_works_only_for_the_host_when_everyone_is_ready() {
    let mut l = lobby();
    let host_me = |ready| pl(1, "Ihor", Some(0), ready, true, true);
    let ann = |ready| pl(2, "Ann", Some(1), ready, false, false);
    enter_lobby(&mut l, vec![host_me(true), ann(false)]);
    assert!(l.is_host());
    assert!(!l.can_start());
    assert!(!is_enabled(&l, ItemId::LobbyStart));
    assert!(has_text(&l, "can start when everyone is ready"));
    l.set_players(vec![host_me(false), ann(true)]);
    assert!(!l.can_start(), "the host must be ready too");
    l.set_players(vec![host_me(true), ann(true)]);
    assert!(l.can_start());
    assert!(is_enabled(&l, ItemId::LobbyStart));
    assert!(!has_text(&l, "can start when everyone is ready"));
    assert_eq!(
        press(&mut l, ItemId::LobbyStart),
        Some(LobbyCommand::StartGame)
    );
    // Alone and ready is enough.
    l.set_players(vec![host_me(true)]);
    assert!(l.can_start());
    // Nobody at all: no.
    l.set_players(Vec::new());
    assert!(!l.can_start());
}

#[test]
fn a_guest_never_gets_a_start_button_even_when_all_are_ready() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), true), me(Some(0), true)]);
    assert!(!l.is_host());
    assert!(!l.can_start());
    assert!(!is_enabled(&l, ItemId::LobbyStart));
    let r = rect_of(&l, ItemId::LobbyStart);
    let at = Vec2 {
        x: r.x + 5.0,
        y: r.y + 5.0,
    };
    pointer(&mut l, UiInput::PointerPressed, at);
    assert_eq!(pointer(&mut l, UiInput::PointerReleased, at), None);
}

fn host_lobby(l: &mut Lobby, others: usize) {
    let mut players = vec![pl(1, "Ihor", Some(0), true, true, true)];
    for k in 0..others {
        players.push(pl(
            10 + k as u64,
            &format!("P{k}"),
            None,
            false,
            false,
            false,
        ));
    }
    enter_lobby(l, players);
}

#[test]
fn only_the_host_can_focus_players_and_not_the_host_row_or_themselves() {
    let mut l = lobby();
    enter_lobby(
        &mut l,
        vec![
            host_row(Some(1), false),
            me(Some(0), false),
            pl(3, "Ann", None, false, false, false),
        ],
    );
    assert!(
        l.items()
            .iter()
            .all(|i| !matches!(i.id, ItemId::LobbyPlayer(_))),
        "a guest has no player rows to act on"
    );
    let mut h = lobby();
    host_lobby(&mut h, 2);
    let players: Vec<(ItemId, bool)> = h
        .items()
        .iter()
        .filter(|i| matches!(i.id, ItemId::LobbyPlayer(_)))
        .map(|i| (i.id, i.enabled))
        .collect();
    assert_eq!(
        players,
        [
            (ItemId::LobbyPlayer(PlayerId(1)), false),
            (ItemId::LobbyPlayer(PlayerId(10)), true),
            (ItemId::LobbyPlayer(PlayerId(11)), true)
        ]
    );
}

#[test]
fn a_kick_needs_a_second_press_and_moving_away_cancels_it() {
    let mut l = lobby();
    host_lobby(&mut l, 2);
    go_to(&mut l, ItemId::LobbyPlayer(PlayerId(10)));
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    assert_eq!(l.kick_armed(), Some(PlayerId(10)));
    assert!(has_text(&l, "Remove P0 from the game?"));
    assert_eq!(
        nav(&mut l, NavInput::Activate),
        Some(LobbyCommand::Kick(PlayerId(10)))
    );
    assert_eq!(l.kick_armed(), None);
    // Moving the focus disarms.
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.kick_armed(), Some(PlayerId(10)));
    nav(&mut l, NavInput::Down);
    assert_eq!(l.kick_armed(), None);
    assert!(!has_text(&l, "Remove P"));
    // Pressing on a different player arms that one, not the first.
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.kick_armed(), Some(PlayerId(11)));
    nav(&mut l, NavInput::Up);
    assert_eq!(l.kick_armed(), None);
}

#[test]
fn back_while_a_kick_waits_only_cancels_it_and_the_pointer_kicks_the_same_way() {
    let mut l = lobby();
    host_lobby(&mut l, 1);
    go_to(&mut l, ItemId::LobbyPlayer(PlayerId(10)));
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.kick_armed(), Some(PlayerId(10)));
    assert_eq!(
        nav(&mut l, NavInput::Back),
        None,
        "does not leave the lobby"
    );
    assert_eq!(l.view(), LobbyView::Lobby);
    assert_eq!(l.kick_armed(), None);
    assert_eq!(click(&mut l, ItemId::LobbyPlayer(PlayerId(10))), None);
    assert_eq!(l.kick_armed(), Some(PlayerId(10)));
    assert_eq!(
        click(&mut l, ItemId::LobbyPlayer(PlayerId(10))),
        Some(LobbyCommand::Kick(PlayerId(10)))
    );
    // A second Back, with nothing armed, leaves.
    assert_eq!(nav(&mut l, NavInput::Back), Some(LobbyCommand::Leave));
}

#[test]
fn a_player_who_leaves_while_the_focus_is_on_them_moves_the_focus_to_a_neighbour() {
    let mut l = lobby();
    host_lobby(&mut l, 3);
    go_to(&mut l, ItemId::LobbyPlayer(PlayerId(11)));
    nav(&mut l, NavInput::Activate);
    assert_eq!(l.kick_armed(), Some(PlayerId(11)));
    let players = |ids: &[u64]| -> Vec<PlayerRow> {
        let mut v = vec![pl(1, "Ihor", Some(0), true, true, true)];
        for id in ids {
            v.push(pl(*id, &format!("P{id}"), None, false, false, false));
        }
        v
    };
    l.set_players(players(&[10, 12]));
    assert_eq!(
        l.focus(),
        ItemId::LobbyPlayer(PlayerId(12)),
        "the next player takes the place"
    );
    assert_eq!(l.kick_armed(), None, "the armed kick went with the player");
    // The focused last player leaves: the nearest player before is focused.
    l.set_players(players(&[10]));
    assert_eq!(l.focus(), ItemId::LobbyPlayer(PlayerId(10)));
    // Everyone else left: the focus is on a character or a button.
    l.set_players(players(&[]));
    assert!(is_enabled(&l, l.focus()));
    assert!(!matches!(l.focus(), ItemId::LobbyPlayer(_)));
}

#[test]
fn a_host_with_many_players_scrolls_the_player_list_with_the_focus() {
    let mut l = lobby();
    host_lobby(&mut l, 14);
    assert_eq!(l.players().len(), 15);
    go_to(&mut l, ItemId::LobbyPlayer(PlayerId(10)));
    assert_eq!(l.player_scroll(), 0);
    go_to(&mut l, ItemId::LobbyPlayer(PlayerId(23)));
    assert_eq!(l.player_scroll(), 15 - VISIBLE_PLAYERS);
    assert!(
        l.items()
            .iter()
            .find(|i| i.id == ItemId::LobbyPlayer(PlayerId(23)))
            .unwrap()
            .rect
            .is_some()
    );
    assert!(has_text(&l, "Players: 15 (showing 8-15)"));
}

#[test]
fn a_long_character_list_scrolls_too() {
    let mut l = lobby();
    l.set_characters((0..10).map(|i| ch(i, &format!("Hero {i}"))).collect());
    enter_lobby(&mut l, vec![host_row(None, false), me(None, false)]);
    go_to(&mut l, ItemId::LobbyCharacter(CharacterId(9)));
    assert_eq!(l.character_scroll(), 10 - VISIBLE_CHARACTERS);
    let rect_of_hero = |l: &Lobby, n| {
        l.items()
            .iter()
            .find(|i| i.id == ItemId::LobbyCharacter(CharacterId(n)))
            .unwrap()
            .rect
    };
    assert!(rect_of_hero(&l, 9).is_some());
    assert!(rect_of_hero(&l, 0).is_none());
}

#[test]
fn left_and_right_move_between_the_lists_and_along_the_buttons() {
    let mut l = lobby();
    host_lobby(&mut l, 1);
    go_to(&mut l, ItemId::LobbyPlayer(PlayerId(10)));
    nav(&mut l, NavInput::Left);
    assert_eq!(
        l.focus(),
        ItemId::LobbyPlayer(PlayerId(10)),
        "nothing to the left of the players"
    );
    nav(&mut l, NavInput::Right);
    assert_eq!(
        l.focus(),
        ItemId::LobbyCharacter(CharacterId(0)),
        "the first usable character"
    );
    nav(&mut l, NavInput::Right);
    assert_eq!(l.focus(), ItemId::LobbyCharacter(CharacterId(0)));
    nav(&mut l, NavInput::Left);
    assert_eq!(
        l.focus(),
        ItemId::LobbyPlayer(PlayerId(10)),
        "the first usable player (not the host row)"
    );
    // The buttons: start is disabled (P0 is not ready), so right from ready goes to leave.
    go_to(&mut l, ItemId::LobbyReady);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.focus(), ItemId::LobbyLeave);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.focus(), ItemId::LobbyLeave);
    nav(&mut l, NavInput::Left);
    assert_eq!(l.focus(), ItemId::LobbyReady);
    nav(&mut l, NavInput::Left);
    assert_eq!(l.focus(), ItemId::LobbyReady);
    // With everyone ready the start button is between them.
    l.set_players(vec![
        pl(1, "Ihor", Some(0), true, true, true),
        pl(10, "P0", Some(1), true, false, false),
    ]);
    go_to(&mut l, ItemId::LobbyReady);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.focus(), ItemId::LobbyStart);
    nav(&mut l, NavInput::Right);
    assert_eq!(l.focus(), ItemId::LobbyLeave);
    nav(&mut l, NavInput::Left);
    assert_eq!(l.focus(), ItemId::LobbyStart);
}

#[test]
fn leaving_tells_the_app_and_goes_to_the_start_screen_with_the_lobby_forgotten() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), false)]);
    press(&mut l, ItemId::LobbyCharacter(CharacterId(2)));
    assert_eq!(press(&mut l, ItemId::LobbyLeave), Some(LobbyCommand::Leave));
    assert_eq!(l.view(), LobbyView::Start);
    assert_eq!(l.focus(), ItemId::StartHost);
    assert!(l.players().is_empty());
    assert_eq!(l.local_character(), None);
}

#[test]
fn the_back_key_leaves_the_lobby_too() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), false)]);
    assert_eq!(nav(&mut l, NavInput::Back), Some(LobbyCommand::Leave));
    assert_eq!(l.view(), LobbyView::Start);
}

#[test]
fn when_the_host_leaves_the_others_get_the_error_and_a_way_back() {
    let mut l = lobby();
    let ann = || pl(3, "Ann", None, false, false, false);
    enter_lobby(
        &mut l,
        vec![host_row(Some(1), false), me(Some(0), false), ann()],
    );
    l.set_players(vec![me(Some(0), false), ann()]);
    assert_eq!(l.view(), LobbyView::Error);
    assert_eq!(l.failure(), Some(&Failure::HostLeft));
    assert!(has_text(&l, "The host left"));
    assert!(l.players().is_empty(), "the lobby is forgotten");
    assert_eq!(
        l.items().iter().map(|i| i.id).collect::<Vec<_>>(),
        [ItemId::ErrorBack]
    );
    assert_eq!(nav(&mut l, NavInput::Activate), None);
    assert_eq!(l.view(), LobbyView::Start);
}

#[test]
fn the_host_leaving_by_the_connection_state_is_the_same_error() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), false)]);
    l.set_connection_state(ConnectionState::Failed(Failure::HostLeft));
    assert_eq!(l.view(), LobbyView::Error);
    assert!(has_text(&l, "closed the game"));
}

#[test]
fn an_empty_player_list_or_a_list_without_a_host_before_any_host_was_seen_is_not_host_left() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), false)]);
    l.set_players(Vec::new());
    assert_eq!(
        l.view(),
        LobbyView::Lobby,
        "a list in the middle of being rebuilt"
    );
    l.set_players(vec![me(Some(0), false)]);
    assert_eq!(l.view(), LobbyView::Lobby, "no host was known just before");
    // A host who is the local player never trips it.
    let mut h = lobby();
    host_lobby(&mut h, 1);
    h.set_players(vec![pl(1, "Ihor", Some(0), true, true, true)]);
    assert_eq!(h.view(), LobbyView::Lobby);
}

#[test]
fn a_player_list_arriving_outside_the_lobby_never_raises_the_host_left_error() {
    let mut l = lobby();
    l.set_players(vec![host_row(Some(1), false), me(Some(0), false)]);
    l.set_players(vec![me(Some(0), false)]);
    assert_eq!(l.view(), LobbyView::Start);
}

#[test]
fn being_kicked_shows_the_reason_and_offers_no_retry() {
    let mut l = lobby();
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), false)]);
    l.set_connection_state(ConnectionState::Failed(Failure::Kicked(
        "away from keyboard".into(),
    )));
    assert_eq!(l.view(), LobbyView::Error);
    assert!(has_text(&l, "away from keyboard"));
    assert_eq!(
        l.items().iter().map(|i| i.id).collect::<Vec<_>>(),
        [ItemId::ErrorBack]
    );
    nav(&mut l, NavInput::Back);
    assert_eq!(l.view(), LobbyView::Start);
}

#[test]
fn a_lost_connection_can_be_retried_with_the_same_request() {
    let mut l = lobby();
    to_direct(&mut l);
    typ(&mut l, "10.0.0.2");
    let join = nav(&mut l, NavInput::Activate);
    l.set_connection_state(ConnectionState::Connected);
    l.set_players(vec![host_row(Some(1), false), me(Some(0), false)]);
    l.set_connection_state(ConnectionState::Failed(Failure::ConnectionLost));
    assert_eq!(l.view(), LobbyView::Error);
    assert_eq!(press(&mut l, ItemId::ErrorRetry), join);
    assert_eq!(l.view(), LobbyView::Connecting);
    assert_eq!(nav(&mut l, NavInput::Activate), Some(LobbyCommand::Cancel));
    assert_eq!(
        l.view(),
        LobbyView::Start,
        "cancelling a retry from the lobby goes to the start screen"
    );
}

#[test]
fn connection_states_that_do_not_apply_to_the_screen_are_ignored() {
    let mut l = lobby();
    l.set_connection_state(ConnectionState::Connected);
    l.set_connection_state(ConnectionState::Failed(Failure::TimedOut));
    assert_eq!(l.view(), LobbyView::Start);
    enter_lobby(&mut l, vec![host_row(Some(1), false), me(Some(0), false)]);
    l.set_connection_state(ConnectionState::Connecting);
    l.set_connection_state(ConnectionState::Connected);
    l.set_connection_state(ConnectionState::Loading {
        what: "x".into(),
        percent: None,
    });
    assert_eq!(l.view(), LobbyView::Lobby);
}

#[test]
fn the_player_rows_show_name_character_ready_ping_host_and_you() {
    let mut l = lobby();
    let mut a = pl(2, "Ann", Some(1), true, false, false);
    a.ping_ms = Some(1500);
    let mut c = pl(3, "Cy", None, false, false, false);
    c.ping_ms = None;
    enter_lobby(
        &mut l,
        vec![host_row(Some(0), true), me(Some(3), false), a, c],
    );
    for needle in [
        "HOST",
        "Host",
        "Ihor (you)",
        "Sam",
        "Ann",
        "Mia",
        "Ready",
        "Not ready",
        "999+ ms",
        "-- ms",
        "20 ms",
        "Players: 4",
        "Cy",
    ] {
        assert!(has_text(&l, needle), "{needle}: {:?}", texts(&l));
    }
    assert_eq!(texts(&l).iter().filter(|t| *t == "HOST").count(), 1);
}

#[test]
fn a_long_player_name_is_cut() {
    let mut l = lobby();
    enter_lobby(
        &mut l,
        vec![
            host_row(Some(1), false),
            PlayerRow {
                name: "N".repeat(200),
                ..me(None, false)
            },
        ],
    );
    assert!(texts(&l).iter().all(|t| t.chars().count() < 100));
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Drawing

fn every_state() -> Vec<Lobby> {
    let mut all = Vec::new();
    let mut l = lobby();
    all.push(l.clone());
    to_setup(&mut l);
    all.push(l.clone());
    typ(&mut l, "x");
    go_to(&mut l, ItemId::HostPasswordOn);
    nav(&mut l, NavInput::Activate);
    nav(&mut l, NavInput::Down);
    all.push(l.clone());
    nav(&mut l, NavInput::Back);
    to_join(&mut l);
    all.push(l.clone());
    l.set_servers(many_servers(9));
    all.push(l.clone());
    go_to(&mut l, ItemId::JoinServer(8));
    all.push(l.clone());
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Direct));
    nav(&mut l, NavInput::Activate);
    typ(&mut l, "256.1.1.1");
    all.push(l.clone());
    go_to(&mut l, ItemId::JoinTab(JoinRoute::Invite));
    nav(&mut l, NavInput::Activate);
    typ(&mut l, CODE_V4);
    all.push(l.clone());
    nav(&mut l, NavInput::Activate);
    all.push(l.clone());
    l.set_connection_state(ConnectionState::Loading {
        what: "Loading the level".into(),
        percent: Some(60),
    });
    all.push(l.clone());
    l.set_connection_state(ConnectionState::Connected);
    l.set_players(vec![
        host_row(Some(1), true),
        me(Some(0), false),
        pl(3, "Ann", None, false, false, false),
    ]);
    all.push(l.clone());
    l.set_connection_state(ConnectionState::Failed(Failure::ConnectionLost));
    all.push(l.clone());
    let mut h = lobby();
    host_lobby(&mut h, 3);
    all.push(h.clone());
    go_to(&mut h, ItemId::LobbyPlayer(PlayerId(11)));
    nav(&mut h, NavInput::Activate);
    all.push(h);
    all
}

#[test]
fn every_screen_draws_inside_the_canvas_with_one_focused_item_where_the_item_is() {
    let states = every_state();
    let views: std::collections::BTreeSet<String> =
        states.iter().map(|l| format!("{:?}", l.view())).collect();
    assert_eq!(views.len(), 6, "every screen is among them: {views:?}");
    for l in states {
        let list = l.draw();
        assert!(
            !list.quads.is_empty() && !list.texts.is_empty(),
            "{:?}",
            l.view()
        );
        for q in &list.quads {
            assert!(
                q.rect.x >= 0.0
                    && q.rect.y >= 0.0
                    && q.rect.x + q.rect.w <= CANVAS_W
                    && q.rect.y + q.rect.h <= CANVAS_H,
                "{:?}: {q:?}",
                l.view()
            );
            assert!(q.rect.w > 0.0 && q.rect.h > 0.0);
        }
        for t in &list.texts {
            assert!(
                t.pos.x >= 0.0
                    && t.pos.x < CANVAS_W
                    && t.pos.y >= 0.0
                    && t.pos.y + t.size < CANVAS_H,
                "{:?}: {t:?}",
                l.view()
            );
            assert!(!t.text.contains('\n') && !t.text.contains('\t'));
        }
        let focused: Vec<_> = list
            .quads
            .iter()
            .filter(|q| q.color == theme::FOCUSED)
            .collect();
        if l.kick_armed().is_none() {
            assert_eq!(focused.len(), 1, "{:?} {:?}", l.view(), l.focus());
            assert_eq!(
                Some(focused[0].rect),
                l.items()
                    .iter()
                    .find(|i| i.id == l.focus())
                    .and_then(|i| i.rect),
                "{:?}",
                l.view()
            );
        } else {
            assert!(focused.is_empty());
        }
    }
}

#[test]
fn items_stay_inside_the_canvas_and_never_overlap() {
    for l in every_state() {
        let items = l.items();
        for (a, ia) in items.iter().enumerate() {
            let Some(ra) = ia.rect else { continue };
            assert!(
                ra.x >= 0.0 && ra.y >= 0.0 && ra.x + ra.w <= CANVAS_W && ra.y + ra.h <= CANVAS_H,
                "{:?}",
                ia.id
            );
            for ib in &items[a + 1..] {
                if let Some(rb) = ib.rect {
                    assert!(
                        !overlap(ra, rb),
                        "{:?} and {:?} overlap on {:?}",
                        ia.id,
                        ib.id,
                        l.view()
                    );
                }
            }
        }
    }
}

#[test]
fn drawing_into_a_list_replaces_it_and_keeps_its_capacity() {
    let l = lobby();
    let mut list = DrawList::default();
    l.draw_into(&mut list);
    let first = list.clone();
    let caps = (list.quads.capacity(), list.texts.capacity());
    l.draw_into(&mut list);
    assert_eq!(list, first, "drawing twice is not drawing twice over");
    assert_eq!((list.quads.capacity(), list.texts.capacity()), caps);
    assert_eq!(l.draw(), first);
}

#[test]
fn the_cursor_blinks_with_the_clock() {
    let mut l = lobby();
    to_setup(&mut l);
    let field = |l: &Lobby| {
        texts(l)
            .into_iter()
            .find(|t| t.contains("SS2RS game"))
            .unwrap()
    };
    assert!(field(&l).contains('|'), "{}", field(&l));
    l.update(500);
    assert!(!field(&l).contains('|'));
    l.update(500);
    assert!(field(&l).contains('|'));
}

#[test]
fn passwords_are_drawn_as_stars() {
    let mut l = lobby();
    to_setup(&mut l);
    go_to(&mut l, ItemId::HostPasswordOn);
    nav(&mut l, NavInput::Activate);
    nav(&mut l, NavInput::Down);
    typ(&mut l, "hunter2");
    assert!(has_text(&l, "*******"));
    assert!(!has_text(&l, "hunter2"));
    nav(&mut l, NavInput::Back);
    to_join(&mut l);
    go_to(&mut l, ItemId::JoinPassword);
    typ(&mut l, "swordfish");
    assert!(has_text(&l, "*********"));
    assert!(!has_text(&l, "swordfish"));
}

#[test]
fn the_start_screen_text() {
    let l = lobby();
    for needle in ["Co-op", "Host a game", "Join a game", "Back"] {
        assert!(has_exact(&l, needle) || has_text(&l, needle), "{needle}");
    }
}

#[test]
fn the_setup_screen_shows_the_values_and_the_reason_the_start_is_off() {
    let mut l = lobby();
    to_setup(&mut l);
    for needle in [
        "Game name",
        "SS2RS game",
        "<  4  >",
        "Off",
        "Hatshepsut",
        "Start hosting",
    ] {
        assert!(has_text(&l, needle), "{needle}: {:?}", texts(&l));
    }
    assert!(!has_text(&l, "Give the game a name"));
    go_to(&mut l, ItemId::HostPasswordOn);
    nav(&mut l, NavInput::Activate);
    assert!(has_exact(&l, "On") && !has_exact(&l, "Off"));
    assert!(has_text(&l, "Type a password"));
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Small helpers

#[test]
fn the_scroll_window_rule() {
    let mut s = 0;
    scroll_to(&mut s, 3, 6, 30);
    assert_eq!(s, 0, "inside the window: stays");
    scroll_to(&mut s, 5, 6, 30);
    assert_eq!(s, 0, "the last row of the window: stays");
    scroll_to(&mut s, 6, 6, 30);
    assert_eq!(s, 1, "one past the end: one step");
    scroll_to(&mut s, 29, 6, 30);
    assert_eq!(s, 24);
    scroll_to(&mut s, 25, 6, 30);
    assert_eq!(s, 24, "inside again");
    scroll_to(&mut s, 24, 6, 30);
    assert_eq!(s, 24, "the first row of the window: stays");
    scroll_to(&mut s, 23, 6, 30);
    assert_eq!(s, 23, "one before the start: one step");
    scroll_to(&mut s, 0, 6, 30);
    assert_eq!(s, 0);
    let mut t = 9;
    scroll_to(&mut t, 2, 6, 4);
    assert_eq!(t, 0, "a list shorter than the window never scrolls");
    let mut u = 7;
    scroll_to(&mut u, 8, 6, 10);
    assert_eq!(u, 4, "the window never shows past the end");
}

#[test]
fn the_nearest_usable_item() {
    let it = |on: bool| Item {
        id: ItemId::StartBack,
        rect: None,
        enabled: on,
    };
    assert_eq!(nearest_enabled(&[], 0), None);
    assert_eq!(nearest_enabled(&[it(false), it(false)], 0), None);
    assert_eq!(nearest_enabled(&[it(true), it(true), it(true)], 1), Some(1));
    assert_eq!(
        nearest_enabled(&[it(true), it(false), it(true)], 1),
        Some(2),
        "a later one wins a tie"
    );
    assert_eq!(
        nearest_enabled(&[it(true), it(false), it(false)], 2),
        Some(0)
    );
    assert_eq!(
        nearest_enabled(&[it(false), it(false), it(true)], 0),
        Some(2)
    );
    assert_eq!(
        nearest_enabled(&[it(true), it(false)], 9),
        Some(0),
        "a position past the end is clamped"
    );
    assert_eq!(
        nearest_enabled(&[it(true), it(false), it(false), it(true)], 2),
        Some(3),
        "the nearer one"
    );
    assert_eq!(
        nearest_enabled(&[it(true), it(true), it(false), it(false), it(false)], 3),
        Some(1)
    );
}

#[test]
fn rows_of_the_same_list() {
    assert!(same_list(ItemId::JoinServer(1), ItemId::JoinServer(5)));
    assert!(same_list(
        ItemId::LobbyPlayer(PlayerId(1)),
        ItemId::LobbyPlayer(PlayerId(2))
    ));
    assert!(same_list(
        ItemId::LobbyCharacter(CharacterId(1)),
        ItemId::LobbyCharacter(CharacterId(2))
    ));
    assert!(!same_list(
        ItemId::JoinServer(1),
        ItemId::LobbyPlayer(PlayerId(1))
    ));
    assert!(!same_list(
        ItemId::LobbyPlayer(PlayerId(1)),
        ItemId::LobbyCharacter(CharacterId(1))
    ));
    assert!(!same_list(ItemId::StartBack, ItemId::StartBack));
    assert!(!same_list(ItemId::JoinServer(1), ItemId::JoinRefresh));
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Random play

#[derive(Clone, Debug)]
enum Op {
    Nav(u8),
    Move(f32, f32),
    Press(f32, f32),
    Release(f32, f32),
    Text(String),
    Backspace,
    Delete,
    Home,
    End,
    Servers(Vec<(u8, u8, bool)>),
    Players(Vec<(bool, bool, Option<u8>)>),
    Characters(u8),
    Levels(u8),
    Scanning(bool),
    Sort(u8),
    State(u8),
    Update(u32),
}

fn op() -> impl Strategy<Value = Op> {
    let coord = || (0.0f32..1920.0, 0.0f32..1080.0);
    prop_oneof![
        8 => (0u8..6).prop_map(Op::Nav),
        2 => coord().prop_map(|(x, y)| Op::Move(x, y)),
        3 => coord().prop_map(|(x, y)| Op::Press(x, y)),
        3 => coord().prop_map(|(x, y)| Op::Release(x, y)),
        2 => "[ -~]{0,12}".prop_map(Op::Text),
        1 => Just(Op::Backspace),
        1 => Just(Op::Delete),
        1 => Just(Op::Home),
        1 => Just(Op::End),
        2 => proptest::collection::vec((0u8..3, 0u8..9, any::<bool>()), 0..24).prop_map(Op::Servers),
        3 => proptest::collection::vec((any::<bool>(), any::<bool>(), proptest::option::of(0u8..6)), 0..14).prop_map(Op::Players),
        1 => (0u8..10).prop_map(Op::Characters),
        1 => (0u8..4).prop_map(Op::Levels),
        1 => any::<bool>().prop_map(Op::Scanning),
        1 => (0u8..3).prop_map(Op::Sort),
        2 => (0u8..8).prop_map(Op::State),
        1 => (0u32..6000).prop_map(Op::Update),
    ]
}

fn apply(l: &mut Lobby, op: Op) {
    let ui = |l: &mut Lobby, i: UiInput| {
        l.handle(LobbyInput::Ui(i));
    };
    match op {
        Op::Nav(k) => ui(
            l,
            UiInput::Nav(match k {
                0 => NavInput::Up,
                1 => NavInput::Down,
                2 => NavInput::Left,
                3 => NavInput::Right,
                4 => NavInput::Activate,
                _ => NavInput::Back,
            }),
        ),
        Op::Move(x, y) => ui(l, UiInput::PointerMoved(Vec2 { x, y })),
        Op::Press(x, y) => ui(l, UiInput::PointerPressed(Vec2 { x, y })),
        Op::Release(x, y) => ui(l, UiInput::PointerReleased(Vec2 { x, y })),
        Op::Text(t) => {
            l.handle(LobbyInput::Text(t));
        }
        Op::Backspace => {
            l.handle(LobbyInput::Backspace);
        }
        Op::Delete => {
            l.handle(LobbyInput::Delete);
        }
        Op::Home => {
            l.handle(LobbyInput::Home);
        }
        Op::End => {
            l.handle(LobbyInput::End);
        }
        Op::Servers(v) => l.set_servers(
            v.into_iter()
                .enumerate()
                .map(|(i, (status, ping, locked))| ServerRow {
                    id: format!("s{i}"),
                    name: format!("Server {i}"),
                    map: "m".into(),
                    players: ping % 5,
                    max_players: 4,
                    ping_ms: (ping > 0).then_some(u32::from(ping) * 10),
                    locked,
                    status: match status {
                        0 => ServerStatus::Open,
                        1 => ServerStatus::Full,
                        _ => ServerStatus::Incompatible("old".into()),
                    },
                })
                .collect(),
        ),
        Op::Players(v) => l.set_players(
            v.into_iter()
                .enumerate()
                .map(|(i, (ready, local, character))| PlayerRow {
                    id: PlayerId(i as u64 + 1),
                    name: format!("P{i}"),
                    character: character.map(|c| CharacterId(u32::from(c))),
                    ready,
                    ping_ms: Some(10),
                    host: i == 0,
                    local: (local && i != 0) || i == 1,
                })
                .collect(),
        ),
        Op::Characters(n) => {
            l.set_characters((0..u32::from(n)).map(|i| ch(i, &format!("C{i}"))).collect())
        }
        Op::Levels(n) => l.set_levels(
            (0..n)
                .map(|i| level(&format!("l{i}"), &format!("Level {i}")))
                .collect(),
        ),
        Op::Scanning(b) => l.set_scanning(b),
        Op::Sort(k) => l.set_sort(
            [ServerSort::Ping, ServerSort::Name, ServerSort::Players][usize::from(k % 3)],
        ),
        Op::State(k) => l.set_connection_state(match k {
            0 => ConnectionState::Connecting,
            1 => ConnectionState::Loading {
                what: "x".into(),
                percent: Some(k * 20),
            },
            2 | 3 => ConnectionState::Connected,
            4 => ConnectionState::Failed(Failure::HostLeft),
            5 => ConnectionState::Failed(Failure::TimedOut),
            6 => ConnectionState::Failed(Failure::BadPassword),
            _ => ConnectionState::Failed(Failure::Kicked("x".into())),
        }),
        Op::Update(dt) => {
            l.update(dt);
        }
    }
}

fn check_invariants(l: &Lobby) {
    let items = l.items();
    if items.iter().any(|i| i.enabled) {
        let focused = items
            .iter()
            .find(|i| i.id == l.focus())
            .unwrap_or_else(|| panic!("the focus {:?} is on {:?}", l.focus(), l.view()));
        assert!(
            focused.enabled,
            "the focus {:?} is usable on {:?}",
            l.focus(),
            l.view()
        );
        assert!(
            focused.rect.is_some(),
            "the focus {:?} is drawn on {:?}",
            l.focus(),
            l.view()
        );
    }
    let servers = l.servers().len();
    assert!(l.server_scroll() <= servers.saturating_sub(VISIBLE_SERVERS));
    assert!(l.player_scroll() <= l.players().len().saturating_sub(VISIBLE_PLAYERS));
    assert!(l.character_scroll() <= l.characters().len().saturating_sub(VISIBLE_CHARACTERS));
    if let Some(p) = l.kick_armed() {
        assert_eq!(l.focus(), ItemId::LobbyPlayer(p));
        assert!(l.is_host());
    }
    if let ItemId::JoinServer(i) = l.focus() {
        assert!(i < servers);
    }
    for (a, ia) in items.iter().enumerate() {
        let Some(ra) = ia.rect else { continue };
        for ib in &items[a + 1..] {
            if let Some(rb) = ib.rect {
                assert!(!overlap(ra, rb), "{:?} and {:?} overlap", ia.id, ib.id);
            }
        }
    }
    assert_eq!(l.view() == LobbyView::Error, l.failure().is_some());
    let list = l.draw();
    let focused = list
        .quads
        .iter()
        .filter(|q| q.color == theme::FOCUSED)
        .count();
    assert!(focused <= 1);
    for q in &list.quads {
        assert!(
            q.rect.x.is_finite()
                && q.rect.w >= 0.0
                && q.rect.x + q.rect.w <= CANVAS_W + 0.01
                && q.rect.y + q.rect.h <= CANVAS_H + 0.01
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn random_play_never_panics_and_keeps_the_focus_on_a_usable_drawn_item(ops in proptest::collection::vec(op(), 1..80)) {
        let mut l = lobby();
        for op in ops {
            apply(&mut l, op);
            check_invariants(&l);
        }
    }

    #[test]
    fn random_play_inside_the_lobby_keeps_the_rules(ops in proptest::collection::vec(op(), 1..80)) {
        let mut l = lobby();
        enter_lobby(&mut l, vec![host_row(Some(1), false), me(None, false)]);
        for op in ops {
            apply(&mut l, op);
            check_invariants(&l);
            // Start is usable exactly when the host can start.
            if let Some(start) = l.items().iter().find(|i| i.id == ItemId::LobbyStart) {
                prop_assert_eq!(start.enabled, l.can_start());
            }
        }
    }
}
