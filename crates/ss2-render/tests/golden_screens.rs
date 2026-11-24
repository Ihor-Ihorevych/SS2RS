//! Golden-image tests for every UI screen at three resolutions and two other aspect ratios (T10-015).
//!
//! The matrix is `SHOTS` x `SIZES`: the four menu screens (main, options, pause, about), the developer console
//! with some scrollback and typed text, and the frame-time overlay over the main menu, at 1280x720, 1920x1080
//! and 2560x1440 (16:9), and at 2560x1080 (21:9, pillarboxed) and 1024x768 (4:3, letterboxed).
//! Besides the golden comparison, each image is checked **without** a golden (so a regenerated golden cannot
//! hide the fault): it has the size asked for, the bars outside the canvas are exactly the background, the
//! canvas is centred, and (for the menu screens) the focused button is where `Canvas::fit` puts it and is orange.
//!
//! They need a GPU adapter (a software one is fine) and skip with a message when none exists; the
//! last test needs no adapter and fails when a golden of the matrix is missing, so a skipped run cannot hide
//! one. Regenerate with `UPDATE_GOLDEN=1 SS2_FORCE_SOFTWARE_ADAPTER=1 cargo test -p ss2-render --test golden_screens`
//! (the software adapter, so the goldens do not depend on the machine they were made on; `docs/GOLDEN_IMAGES.md`).

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_render::golden::{self, Mode, Tolerance};
use ss2_render::{
    Fill, Filter, ImageHandle, ImageRgba8, Insets, NineSliceDraw, RenderError, Renderer, UiCommand,
    UiFrame, UvRect,
};
use ss2_ui::coop_overlays::{
    ChatBox, ChatConfig, ChatKey, ChatKind, CoopOverlays, Nameplate, PingKind, PingProjection,
    Projected, ScoreEntry, VoiceSample,
};
use ss2_ui::data_wizard::{DataWizard, WizardCandidate, WizardCommand, WizardInput, WizardReport};
use ss2_ui::devconsole::{ConsoleView, Corner, FrameStats, LineKind, draw_overlay, stats_lines};
use ss2_ui::lobby::{
    CharacterChoice, CharacterId, ConnectionState, ItemId, JoinRoute, LevelChoice, Lobby,
    LobbyCommand, LobbyConfig, LobbyInput, PlayerId, PlayerRow, ServerRow, ServerStatus,
};
use ss2_ui::session_screens::{DeathMode, DeathScreen, LevelStats, SummaryScreen};
use ss2_ui::{Canvas, Color, DrawList, Menu, Rect, ScreenId, TextAlign, UiInput, Vec2, theme};

/// (width, height): three resolutions at 16:9, then 21:9, then 4:3.
const SIZES: [(u32, u32); 5] = [
    (1280, 720),
    (1920, 1080),
    (2560, 1440),
    (2560, 1080),
    (1024, 768),
];

/// One thing to photograph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shot {
    Menu(ScreenId),
    Console,
    Overlay,
    DeathSolo,
    DeathCoop,
    SummaryDone,
    SummaryCounting,
    CoopScoreboard,
    CoopChat,
    CoopNameplates,
    CoopPings,
    CoopVoice,
    WizardCandidates,
    WizardEmpty,
    WizardManual,
    WizardResultOk,
    WizardResultWarn,
    WizardResultBad,
    LobbyStart,
    LobbyHost,
    LobbyBrowser,
    LobbyDirectError,
    LobbyConnecting,
    LobbyRoom,
    LobbyError,
}

const SHOTS: [Shot; 28] = [
    Shot::Menu(ScreenId::Main),
    Shot::Menu(ScreenId::Options),
    Shot::Menu(ScreenId::Pause),
    Shot::Menu(ScreenId::About),
    Shot::Console,
    Shot::Overlay,
    Shot::DeathSolo,
    Shot::DeathCoop,
    Shot::SummaryDone,
    Shot::SummaryCounting,
    Shot::CoopScoreboard,
    Shot::CoopChat,
    Shot::CoopNameplates,
    Shot::CoopPings,
    Shot::CoopVoice,
    Shot::WizardCandidates,
    Shot::WizardEmpty,
    Shot::WizardManual,
    Shot::WizardResultOk,
    Shot::WizardResultWarn,
    Shot::WizardResultBad,
    Shot::LobbyStart,
    Shot::LobbyHost,
    Shot::LobbyBrowser,
    Shot::LobbyDirectError,
    Shot::LobbyConnecting,
    Shot::LobbyRoom,
    Shot::LobbyError,
];

/// The software adapter crashed a test process once when several tests made devices at the same time (see
/// `docs/GOLDEN_IMAGES.md`), so one test at a time owns the GPU.
static GPU: Mutex<()> = Mutex::new(());

fn renderer() -> Option<(MutexGuard<'static, ()>, Renderer)> {
    let guard = GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match Renderer::new_offscreen(SIZES[0]) {
        Ok(r) => Some((guard, r)),
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            None
        }
        Err(e) => panic!("renderer creation failed: {e}"),
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden")
}

fn name(shot: Shot, (w, h): (u32, u32)) -> String {
    match shot {
        Shot::Menu(_) => format!("menu_{}_{w}x{h}.png", word(shot)),
        _ => format!("ui_{}_{w}x{h}.png", word(shot)),
    }
}

fn word(shot: Shot) -> &'static str {
    match shot {
        Shot::Menu(ScreenId::Main) => "main",
        Shot::Menu(ScreenId::Options) => "options",
        Shot::Menu(ScreenId::Pause) => "pause",
        Shot::Menu(ScreenId::About) => "about",
        Shot::Console => "console",
        Shot::Overlay => "overlay",
        Shot::DeathSolo => "death_solo",
        Shot::DeathCoop => "death_coop",
        Shot::SummaryDone => "summary",
        Shot::SummaryCounting => "summary_counting",
        Shot::CoopScoreboard => "coop_scoreboard",
        Shot::CoopChat => "coop_chat",
        Shot::CoopNameplates => "coop_nameplates",
        Shot::CoopPings => "coop_pings",
        Shot::CoopVoice => "coop_voice",
        Shot::WizardCandidates => "wizard_candidates",
        Shot::WizardEmpty => "wizard_empty",
        Shot::WizardManual => "wizard_manual",
        Shot::WizardResultOk => "wizard_result_ok",
        Shot::WizardResultWarn => "wizard_result_warn",
        Shot::WizardResultBad => "wizard_result_bad",
        Shot::LobbyStart => "lobby_start",
        Shot::LobbyHost => "lobby_host",
        Shot::LobbyBrowser => "lobby_browser",
        Shot::LobbyDirectError => "lobby_direct_error",
        Shot::LobbyConnecting => "lobby_connecting",
        Shot::LobbyRoom => "lobby_room",
        Shot::LobbyError => "lobby_error",
    }
}

/// A menu showing `screen`, reached the way a player reaches it.
fn menu_at(screen: ScreenId) -> Menu {
    let mut menu = Menu::new(Settings::default());
    let press = |menu: &mut Menu, nav| menu.handle(UiInput::Nav(nav));
    match screen {
        ScreenId::Main => {}
        ScreenId::Options => {
            press(&mut menu, NavInput::Down);
            press(&mut menu, NavInput::Activate);
        }
        ScreenId::About => {
            press(&mut menu, NavInput::Down);
            press(&mut menu, NavInput::Down);
            press(&mut menu, NavInput::Activate);
        }
        ScreenId::Pause => menu.open_pause(),
    }
    assert_eq!(
        menu.screen(),
        screen,
        "the test reaches the screen it names"
    );
    menu
}

/// What the shot draws, in canvas units.
fn level_stats() -> LevelStats {
    LevelStats {
        level: "The Jungle".into(),
        kills: 137,
        kills_total: 164,
        secrets: 2,
        secrets_total: 5,
        elapsed_secs: 1325,
        par_secs: Some(1500),
        score: Some(48_250),
    }
}

fn score(
    id: u32,
    name: &str,
    kills: u32,
    deaths: u32,
    score: i64,
    ping: Option<u32>,
) -> ScoreEntry {
    ScoreEntry {
        id,
        name: name.into(),
        kills,
        deaths,
        score,
        ping_ms: ping,
        local: false,
        talking: false,
    }
}

fn projected(x: f32, y: f32, distance: f32) -> Projected {
    Projected {
        screen: Vec2 { x, y },
        distance,
        in_front: true,
    }
}

fn teammate(id: u32, name: &str, health: f32, armor: f32, at: Projected) -> Nameplate {
    Nameplate {
        id,
        name: name.into(),
        health,
        max_health: 100.0,
        armor,
        max_armor: 100.0,
        talking: false,
        at,
    }
}

fn voice(id: u32, name: &str, level: f32, local: bool) -> VoiceSample {
    VoiceSample {
        id,
        name: name.into(),
        level,
        local,
    }
}

/// A co-op overlay shot: the same `CoopOverlays` a game would hold, with only the part under test filled in.
fn coop_draw(shot: Shot) -> DrawList {
    let mut all = CoopOverlays::default();
    let mut players: Vec<Nameplate> = Vec::new();
    let mut projections: Vec<PingProjection> = Vec::new();
    match shot {
        Shot::CoopScoreboard => {
            all.scoreboard.set_held(true);
            let mut me = score(3, "Ihor", 21, 6, 2100, Some(32));
            me.local = true;
            let mut talker = score(5, "Mia", 25, 4, 2650, Some(95));
            talker.talking = true;
            all.scoreboard.set_entries(&[
                score(1, "Tom", 18, 9, 1650, Some(48)),
                score(2, "Ann", 12, 3, 1300, Some(171)),
                me,
                score(
                    4,
                    "a very long player name that is cut",
                    8,
                    12,
                    500,
                    Some(60),
                ),
                talker,
                score(6, "Bot Sam", 15, 7, 1450, None),
                score(7, "Zed", 2, 14, -150, Some(1200)),
            ]);
        }
        Shot::CoopChat => {
            let mut chat = ChatBox::new(ChatConfig::default());
            chat.push(ChatKind::Player, "Tom", "anyone near the gate?");
            // Ten seconds go by; with the input open (below) old lines are shown at full strength anyway.
            for _ in 0..630 {
                chat.update();
            }
            chat.push(ChatKind::System, "", "Ann joined the game");
            chat.push(ChatKind::Player, "Ann", "on my way, wait for me before you open the big door because there is a lot behind it");
            chat.push(ChatKind::Team, "Mia", "left side is clear");
            chat.push(ChatKind::Player, "Ihor", "ok go");
            chat.open(true);
            chat.handle_text("push right now");
            chat.handle_key(ChatKey::Left);
            chat.handle_key(ChatKey::Left);
            all.chat = chat;
        }
        Shot::CoopNameplates => {
            players = vec![
                teammate(1, "Tom", 100.0, 0.0, projected(520.0, 620.0, 6.0)),
                teammate(2, "Ann", 62.0, 80.0, projected(960.0, 560.0, 22.0)),
                teammate(3, "Mia", 18.0, 0.0, projected(1380.0, 640.0, 38.0)),
                teammate(4, "Edge Walker", 80.0, 30.0, projected(20.0, 300.0, 10.0)),
                teammate(5, "Far Away", 100.0, 0.0, projected(1700.0, 420.0, 58.0)),
                teammate(6, "Zed", 0.0, 0.0, projected(1100.0, 840.0, 12.0)),
                Nameplate {
                    talking: true,
                    ..teammate(7, "Talker", 45.0, 0.0, projected(760.0, 330.0, 15.0))
                },
                Nameplate {
                    at: Projected {
                        in_front: false,
                        ..projected(300.0, 300.0, 5.0)
                    },
                    ..teammate(8, "Behind", 100.0, 0.0, projected(0.0, 0.0, 5.0))
                },
            ];
        }
        Shot::CoopPings => {
            let a = all
                .pings
                .add(1, "Tom", PingKind::Danger, "Boss here", 400)
                .unwrap();
            let b = all
                .pings
                .add(2, "Ann", PingKind::Item, "Ammo", 400)
                .unwrap();
            let c = all.pings.add(3, "Mia", PingKind::Go, "Go", 400).unwrap();
            let d = all
                .pings
                .add(4, "Zed", PingKind::Look, "Secret?", 400)
                .unwrap();
            let e = all
                .pings
                .add(5, "Bot Sam", PingKind::Danger, "", 20)
                .unwrap();
            projections = vec![
                PingProjection {
                    id: a,
                    at: projected(1280.0, 380.0, 64.0),
                },
                PingProjection {
                    id: b,
                    at: projected(560.0, 760.0, 12.0),
                },
                PingProjection {
                    id: c,
                    at: projected(2900.0, 500.0, 40.0),
                },
                PingProjection {
                    id: d,
                    at: Projected {
                        screen: Vec2 { x: 700.0, y: 400.0 },
                        distance: 25.0,
                        in_front: false,
                    },
                },
                PingProjection {
                    id: e,
                    at: projected(180.0, 1200.0, 8.0),
                },
            ];
        }
        Shot::CoopVoice => {
            // Tom spoke a moment ago and is still listed, dimmed.
            let mut earlier = CoopOverlays::default();
            earlier.update(&[voice(1, "Tom", 0.5, false)]);
            earlier.update(&[
                voice(2, "Ann", 0.8, false),
                voice(3, "Ihor", 0.35, true),
                voice(5, "Mia", 1.0, false),
                voice(7, "A very long player name", 0.1, false),
            ]);
            all = earlier;
        }
        _ => unreachable!("not a co-op shot"),
    }
    let mut list = DrawList::default();
    all.draw_into(&players, &projections, &mut list);
    list
}

fn nav(wizard: &mut DataWizard, key: NavInput) -> Option<WizardCommand> {
    wizard.handle(WizardInput::Ui(UiInput::Nav(key)))
}

fn wizard_candidate(dir: &str, detail: &str) -> WizardCandidate {
    WizardCandidate {
        dir: PathBuf::from(dir),
        detail: detail.into(),
    }
}

/// The first-run wizard as the app drives it, up to the screen under test. Only ASCII text, so that a golden does not depend on the fonts of the machine.
fn wizard_draw(shot: Shot) -> DrawList {
    let reason =
        r"The folder you chose before no longer works (D:\Games\Serious Sam 2): choose it again";
    let mut wizard = DataWizard::new(Some(reason.into()));
    assert_eq!(wizard.start(), WizardCommand::Search);
    let steam = r"C:\Program Files (x86)\Steam\steamapps\common\Serious Sam 2";
    let found = match shot {
        Shot::WizardEmpty | Shot::WizardManual => Vec::new(),
        _ => vec![
            wizard_candidate(steam, "Steam library - patch 2.100 (build 800000)"),
            wizard_candidate(
                r"D:\Games\Serious Sam 2",
                "usual GOG place, current folder - patch 2.068",
            ),
            wizard_candidate(
                r"E:\Backups\2019\a very long folder name that goes on and on\More Games\Serious Sam 2 (copy)",
                "SS2_DATA_DIR - no patch",
            ),
        ],
    };
    wizard.set_candidates(found);
    let report =
        |dir: &str, usable: bool, warnings: usize, headline: &str, lines: &[&str]| WizardReport {
            dir: PathBuf::from(dir),
            usable,
            warnings,
            headline: headline.into(),
            lines: lines.iter().map(|l| (*l).to_owned()).collect(),
        };
    match shot {
        Shot::WizardCandidates | Shot::WizardEmpty => {}
        Shot::WizardManual => {
            // "Enter a folder path" is the first row of an empty list.
            assert_eq!(nav(&mut wizard, NavInput::Activate), None);
            let typed = r"D:\Old Disks\Backup 2019\Games and Programs\Croteam\Serious Sam 2\Serious Sam 2 (Retail)";
            assert_eq!(wizard.handle(WizardInput::Text(typed.into())), None);
            for _ in 0..30 {
                nav(&mut wizard, NavInput::Left);
            }
        }
        Shot::WizardResultOk => {
            assert!(matches!(
                nav(&mut wizard, NavInput::Activate),
                Some(WizardCommand::Validate(_))
            ));
            wizard.set_report(report(
                steam,
                true,
                0,
                "Serious Sam 2 found: the install looks complete",
                &[
                    "Archives: 14 found, 14 readable",
                    "Patch level: 2.100 (build 800000)",
                ],
            ));
        }
        Shot::WizardResultWarn => {
            nav(&mut wizard, NavInput::Down);
            assert!(matches!(
                nav(&mut wizard, NavInput::Activate),
                Some(WizardCommand::Validate(_))
            ));
            wizard.set_report(report(
                r"D:\Games\Serious Sam 2",
                true,
                2,
                "Usable, with 2 warnings",
                &[
                    "Archives: 6 found, 6 readable",
                    "Patch level: 2.068",
                    "WARNING: Movies_PC_02.gro is missing, so the cutscenes will not play",
                    "  -> Copy the file back from the original install if you want the movies.",
                    "WARNING: the patch level 2.068 is older than 2.100 (build 800000), the newest the project has seen",
                    "  -> Update the game, or copy all the Patch_02_*.gro files of the newest patch into the folder",
                ],
            ));
        }
        Shot::WizardResultBad => {
            nav(&mut wizard, NavInput::Down);
            nav(&mut wizard, NavInput::Down);
            assert!(matches!(
                nav(&mut wizard, NavInput::Activate),
                Some(WizardCommand::Validate(_))
            ));
            wizard.set_report(report(
                r"E:\Backups\2019\a very long folder name that goes on and on\More Games\Serious Sam 2 (copy)",
                false,
                1,
                "This folder cannot be used: 2 errors",
                &[
                    "Archives: 2 found, 1 readable",
                    "Patch level: none found",
                    "ERROR: All_PC_02.gro is damaged or is not a .gro archive (invalid Zip archive)",
                    "  -> Copy the file again from the original install, or reinstall the game.",
                    "ERROR: All_PC_03.gro is missing",
                    "  -> Reinstall the game, or copy the file back from the original install.",
                    "WARNING: no patch archives (Patch_*.gro) found: only the original release data is available",
                ],
            ));
        }
        _ => unreachable!("not a wizard shot"),
    }
    wizard.draw()
}

fn lobby_nav(lobby: &mut Lobby, key: NavInput) -> Option<LobbyCommand> {
    lobby.handle(LobbyInput::Ui(UiInput::Nav(key)))
}

/// Move the lobby focus to `id` the way a player does: with the down key.
fn lobby_go_to(lobby: &mut Lobby, id: ItemId) {
    for _ in 0..200 {
        if lobby.focus() == id {
            return;
        }
        lobby_nav(lobby, NavInput::Down);
    }
    panic!("the focus never reached {id:?}");
}

fn lobby_server(
    id: &str,
    name: &str,
    map: &str,
    players: (u8, u8),
    ping: Option<u32>,
    locked: bool,
    status: ServerStatus,
) -> ServerRow {
    ServerRow {
        id: id.into(),
        name: name.into(),
        map: map.into(),
        players: players.0,
        max_players: players.1,
        ping_ms: ping,
        locked,
        status,
    }
}

fn lobby_player(
    id: u64,
    name: &str,
    character: Option<u32>,
    ready: bool,
    ping: Option<u32>,
    (host, local): (bool, bool),
) -> PlayerRow {
    PlayerRow {
        id: PlayerId(id),
        name: name.into(),
        character: character.map(CharacterId),
        ready,
        ping_ms: ping,
        host,
        local,
    }
}

/// The lobby screens as the app drives them, up to the screen under test. Only ASCII text, so that a golden does not depend on the fonts of the machine.
fn lobby_draw(shot: Shot) -> DrawList {
    let mut lobby = Lobby::new(LobbyConfig {
        default_server_name: "Ihor's co-op".into(),
        ..LobbyConfig::default()
    });
    lobby.set_levels(vec![
        LevelChoice {
            id: "maps/hatshepsut".into(),
            name: "Valley of the Kings".into(),
        },
        LevelChoice {
            id: "maps/jungle".into(),
            name: "The Jungle".into(),
        },
    ]);
    lobby.set_characters(
        ["Sam", "Mia", "Zed", "Ann"]
            .iter()
            .enumerate()
            .map(|(i, n)| CharacterChoice {
                id: CharacterId(i as u32),
                name: (*n).into(),
            })
            .collect(),
    );
    let typed = |lobby: &mut Lobby, text: &str| {
        assert_eq!(lobby.handle(LobbyInput::Text(text.into())), None);
    };
    match shot {
        Shot::LobbyStart => {}
        Shot::LobbyHost => {
            lobby_go_to(&mut lobby, ItemId::StartHost);
            lobby_nav(&mut lobby, NavInput::Activate);
            lobby_go_to(&mut lobby, ItemId::HostPasswordOn);
            lobby_nav(&mut lobby, NavInput::Activate);
            lobby_nav(&mut lobby, NavInput::Down);
            typed(&mut lobby, "secret");
            lobby_go_to(&mut lobby, ItemId::HostLevel);
            lobby_nav(&mut lobby, NavInput::Right);
        }
        Shot::LobbyBrowser => {
            let open = ServerStatus::Open;
            lobby.set_servers(vec![
                lobby_server(
                    "192.168.1.20:27015",
                    "Tom's game",
                    "Valley of the Kings",
                    (2, 4),
                    Some(3),
                    false,
                    open.clone(),
                ),
                lobby_server(
                    "192.168.1.31:27015",
                    "Friday night co-op",
                    "The Jungle",
                    (3, 4),
                    Some(18),
                    true,
                    open.clone(),
                ),
                lobby_server(
                    "192.168.1.44:27015",
                    "Full house",
                    "Karnak",
                    (4, 4),
                    Some(7),
                    false,
                    ServerStatus::Full,
                ),
                lobby_server(
                    "192.168.1.52:27015",
                    "Old build",
                    "Karnak",
                    (1, 4),
                    Some(24),
                    false,
                    ServerStatus::Incompatible("older version".into()),
                ),
                lobby_server(
                    "192.168.1.60:27015",
                    "Ann",
                    "Sand Canyon",
                    (1, 8),
                    None,
                    false,
                    open.clone(),
                ),
                lobby_server(
                    "10.0.0.7:27015",
                    "a very long server name that is cut off",
                    "Pyramid",
                    (5, 8),
                    Some(1450),
                    true,
                    open.clone(),
                ),
                lobby_server(
                    "10.0.0.9:27015",
                    "Zed's place",
                    "Tomb",
                    (0, 2),
                    Some(41),
                    false,
                    open,
                ),
            ]);
            lobby_go_to(&mut lobby, ItemId::StartJoin);
            lobby_nav(&mut lobby, NavInput::Activate);
            lobby_nav(&mut lobby, NavInput::Down);
        }
        Shot::LobbyDirectError => {
            lobby_go_to(&mut lobby, ItemId::StartJoin);
            lobby_nav(&mut lobby, NavInput::Activate);
            lobby_go_to(&mut lobby, ItemId::JoinTab(JoinRoute::Direct));
            lobby_nav(&mut lobby, NavInput::Activate);
            typed(&mut lobby, "192.168.1.300:27015");
        }
        Shot::LobbyConnecting | Shot::LobbyRoom | Shot::LobbyError => {
            lobby_go_to(&mut lobby, ItemId::StartJoin);
            lobby_nav(&mut lobby, NavInput::Activate);
            lobby_go_to(&mut lobby, ItemId::JoinTab(JoinRoute::Direct));
            lobby_nav(&mut lobby, NavInput::Activate);
            typed(&mut lobby, "192.168.1.20");
            assert!(matches!(
                lobby_nav(&mut lobby, NavInput::Activate),
                Some(LobbyCommand::Join(_))
            ));
            match shot {
                Shot::LobbyConnecting => {
                    lobby.update(2400);
                    lobby.set_connection_state(ConnectionState::Loading {
                        what: "Loading the level".into(),
                        percent: Some(45),
                    });
                    lobby.update(3600);
                }
                Shot::LobbyError => {
                    assert_eq!(lobby.update(10_000), Some(LobbyCommand::Cancel));
                }
                _ => {
                    lobby.set_connection_state(ConnectionState::Connected);
                    lobby.set_players(vec![
                        lobby_player(1, "Tom", Some(0), true, Some(3), (true, false)),
                        lobby_player(2, "Ann", Some(1), false, Some(41), (false, false)),
                        lobby_player(3, "Ihor", Some(2), true, Some(12), (false, true)),
                        lobby_player(4, "Cy", None, false, None, (false, false)),
                    ]);
                    lobby_go_to(&mut lobby, ItemId::LobbyReady);
                }
            }
        }
        _ => unreachable!("not a lobby shot"),
    }
    lobby.draw()
}

fn draw(shot: Shot) -> DrawList {
    match shot {
        Shot::LobbyStart
        | Shot::LobbyHost
        | Shot::LobbyBrowser
        | Shot::LobbyDirectError
        | Shot::LobbyConnecting
        | Shot::LobbyRoom
        | Shot::LobbyError => lobby_draw(shot),
        Shot::WizardCandidates
        | Shot::WizardEmpty
        | Shot::WizardManual
        | Shot::WizardResultOk
        | Shot::WizardResultWarn
        | Shot::WizardResultBad => wizard_draw(shot),
        Shot::CoopScoreboard
        | Shot::CoopChat
        | Shot::CoopNameplates
        | Shot::CoopPings
        | Shot::CoopVoice => coop_draw(shot),
        Shot::DeathSolo => DeathScreen::new(DeathMode::Solo).draw(),
        Shot::DeathCoop => {
            let mut screen = DeathScreen::new(DeathMode::Coop {
                respawn_in_ticks: Some(250),
                ticks_per_second: 60,
            });
            for _ in 0..10 {
                screen.update();
            }
            screen.draw()
        }
        Shot::SummaryDone => {
            let mut screen = SummaryScreen::new(level_stats());
            while screen.counting() {
                screen.update();
            }
            screen.draw()
        }
        Shot::SummaryCounting => {
            let mut screen = SummaryScreen::new(level_stats());
            for _ in 0..70 {
                screen.update();
            }
            screen.draw()
        }
        Shot::Menu(screen) => menu_at(screen).draw(),
        Shot::Console => {
            let mut view = ConsoleView::new(100);
            view.toggle();
            view.print("> map tutorial".into(), LineKind::Input);
            view.print("loading tutorial".into(), LineKind::Output);
            view.print("unknown variable r_nosuch".into(), LineKind::Error);
            view.print("> set r_vsync 1".into(), LineKind::Input);
            view.print("r_vsync = 1".into(), LineKind::Output);
            view.handle_text("god");
            view.draw()
        }
        Shot::Overlay => {
            let mut stats = FrameStats::new(120);
            for i in 0..120u64 {
                stats.record_micros(if i % 40 == 0 { 33_000 } else { 16_600 });
            }
            let mut list = menu_at(ScreenId::Main).draw();
            let overlay = draw_overlay(&stats_lines(&stats), Corner::TopRight);
            list.quads.extend(overlay.quads);
            list.texts.extend(overlay.texts);
            list
        }
    }
}

fn render(r: &mut Renderer, list: &DrawList, (w, h): (u32, u32)) -> ImageRgba8 {
    r.resize(w, h);
    r.render_to_image(list, theme::BACKGROUND).unwrap()
}

fn pixel(img: &ImageRgba8, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.pixels[i..i + 4].try_into().unwrap()
}

/// The background as the 8-bit image holds it.
fn background_bytes() -> [u8; 4] {
    let Color(c) = theme::BACKGROUND;
    [0, 1, 2, 3].map(|i| (c[i] * 255.0).round() as u8)
}

fn near(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) <= tolerance)
}

fn check_golden(name: &str, img: &ImageRgba8) {
    let failures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-failures");
    let mode = Mode::from_env(failures);
    if let Err(e) = golden::check_golden(&golden_dir(), name, img, &Tolerance::default(), &mode) {
        panic!("{e}");
    }
}

/// The checks that need no golden.
fn check_geometry(shot: Shot, size: (u32, u32), list: &DrawList, img: &ImageRgba8) {
    let what = format!("{} at {}x{}", word(shot), size.0, size.1);
    assert_eq!(
        (img.width, img.height),
        size,
        "{what}: the image has the size asked for"
    );
    let canvas = Canvas::fit(size.0, size.1).unwrap();
    let background = background_bytes();

    // The bars outside the canvas (left and right on 21:9, top and bottom on 4:3) are the background.
    let left = canvas.offset.x.floor() as i64 - 2;
    let top = canvas.offset.y.floor() as i64 - 2;
    let mut bars = 0;
    if left > 0 {
        for y in (0..size.1).step_by(7) {
            for x in [0, (left / 2) as u32, left as u32] {
                assert!(
                    near(pixel(img, x, y), background, 1),
                    "{what}: left bar at {x},{y}"
                );
                let mirrored = size.0 - 1 - x;
                assert!(
                    near(pixel(img, mirrored, y), background, 1),
                    "{what}: right bar at {mirrored},{y}"
                );
            }
        }
        bars += 1;
    }
    if top > 0 {
        for x in (0..size.0).step_by(7) {
            for y in [0, (top / 2) as u32, top as u32] {
                assert!(
                    near(pixel(img, x, y), background, 1),
                    "{what}: top bar at {x},{y}"
                );
                let mirrored = size.1 - 1 - y;
                assert!(
                    near(pixel(img, x, mirrored), background, 1),
                    "{what}: bottom bar at {x},{mirrored}"
                );
            }
        }
        bars += 1;
    }
    let expected_bars = usize::from(size.0 * 9 != size.1 * 16);
    assert_eq!(
        bars, expected_bars,
        "{what}: bars exactly when the window is not 16:9"
    );

    // The canvas is centred (the same offset on both sides of each axis).
    let right_gap = size.0 as f32 - (canvas.offset.x + 1920.0 * canvas.scale);
    let bottom_gap = size.1 as f32 - (canvas.offset.y + 1080.0 * canvas.scale);
    assert!(
        (right_gap - canvas.offset.x).abs() < 0.01 && (bottom_gap - canvas.offset.y).abs() < 0.01,
        "{what}"
    );

    // The focused button of a menu is where the canvas puts it, and orange. Sample near its left edge, away from the label.
    if matches!(
        shot,
        Shot::Menu(_)
            | Shot::Overlay
            | Shot::DeathSolo
            | Shot::DeathCoop
            | Shot::SummaryDone
            | Shot::WizardCandidates
            | Shot::WizardEmpty
            | Shot::WizardManual
            | Shot::WizardResultOk
            | Shot::WizardResultWarn
            | Shot::WizardResultBad
            | Shot::LobbyStart
            | Shot::LobbyHost
            | Shot::LobbyBrowser
            | Shot::LobbyDirectError
            | Shot::LobbyConnecting
            | Shot::LobbyRoom
            | Shot::LobbyError
    ) {
        let focused = list
            .quads
            .iter()
            .find(|q| q.color == theme::FOCUSED)
            .unwrap_or_else(|| panic!("{what}: a screen with a focused button"));
        // The text field's focus is a ring four units wide around the field: sample inside the ring, not 12 units in.
        let inset = if shot == Shot::WizardManual {
            2.0
        } else {
            12.0
        };
        let at = canvas.to_screen(Vec2 {
            x: focused.rect.x + inset,
            y: focused.rect.y + focused.rect.h / 2.0,
        });
        let p = pixel(img, at.x as u32, at.y as u32);
        assert!(
            near(p, [217, 115, 26, 255], 3),
            "{what}: the focused button is orange at {at:?}, got {p:?}"
        );
    } else {
        assert!(
            list.quads.iter().all(|q| q.color != theme::FOCUSED),
            "{what}: no focused button on this screen"
        );
    }
}

fn one_shot(shot: Shot) {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    let list = draw(shot);
    for size in SIZES {
        let img = render(&mut r, &list, size);
        check_geometry(shot, size, &list, &img);
        check_golden(&name(shot, size), &img);
    }
}

#[test]
fn main_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::Menu(ScreenId::Main));
}

#[test]
fn options_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::Menu(ScreenId::Options));
}

#[test]
fn pause_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::Menu(ScreenId::Pause));
}

#[test]
fn about_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::Menu(ScreenId::About));
}

#[test]
fn console_matches_its_goldens_at_every_size() {
    one_shot(Shot::Console);
}

#[test]
fn frame_time_overlay_matches_its_goldens_at_every_size() {
    one_shot(Shot::Overlay);
}

#[test]
fn solo_death_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::DeathSolo);
}

#[test]
fn coop_death_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::DeathCoop);
}

#[test]
fn finished_summary_matches_its_goldens_at_every_size() {
    one_shot(Shot::SummaryDone);
}

#[test]
fn counting_summary_matches_its_goldens_at_every_size() {
    one_shot(Shot::SummaryCounting);
}

#[test]
fn coop_scoreboard_matches_its_goldens_at_every_size() {
    one_shot(Shot::CoopScoreboard);
}

#[test]
fn coop_chat_matches_its_goldens_at_every_size() {
    one_shot(Shot::CoopChat);
}

#[test]
fn coop_nameplates_match_their_goldens_at_every_size() {
    one_shot(Shot::CoopNameplates);
}

#[test]
fn coop_ping_markers_match_their_goldens_at_every_size() {
    one_shot(Shot::CoopPings);
}

#[test]
fn coop_voice_indicators_match_their_goldens_at_every_size() {
    one_shot(Shot::CoopVoice);
}

#[test]
fn wizard_candidates_match_their_goldens_at_every_size() {
    one_shot(Shot::WizardCandidates);
}

#[test]
fn wizard_with_nothing_found_matches_its_goldens_at_every_size() {
    one_shot(Shot::WizardEmpty);
}

#[test]
fn wizard_path_entry_matches_its_goldens_at_every_size() {
    one_shot(Shot::WizardManual);
}

#[test]
fn wizard_clean_result_matches_its_goldens_at_every_size() {
    one_shot(Shot::WizardResultOk);
}

#[test]
fn wizard_result_with_warnings_matches_its_goldens_at_every_size() {
    one_shot(Shot::WizardResultWarn);
}

#[test]
fn wizard_refusal_matches_its_goldens_at_every_size() {
    one_shot(Shot::WizardResultBad);
}

#[test]
fn lobby_start_screen_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyStart);
}

#[test]
fn lobby_host_setup_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyHost);
}

#[test]
fn lobby_server_browser_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyBrowser);
}

#[test]
fn lobby_direct_address_error_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyDirectError);
}

#[test]
fn lobby_connecting_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyConnecting);
}

#[test]
fn lobby_room_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyRoom);
}

#[test]
fn lobby_error_matches_its_goldens_at_every_size() {
    one_shot(Shot::LobbyError);
}

#[test]
fn the_same_screen_at_two_sizes_is_the_same_picture_scaled_not_a_different_layout() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    // 1280x720 and 2560x1440 are the same layout at 1x and 2x: every flat region of one lies on the other.
    let list = draw(Shot::Menu(ScreenId::Main));
    let small = render(&mut r, &list, (1280, 720));
    let big = render(&mut r, &list, (2560, 1440));
    let mut compared = 0;
    for y in (3..720).step_by(11) {
        for x in (3..1280).step_by(13) {
            let a = pixel(&small, x, y);
            let b = pixel(&big, x * 2, y * 2);
            // Away from an edge both sample the same fill (text is skipped: it is not scale-exact).
            let neighbours_agree = [(1u32, 0u32), (0, 1)].iter().all(|&(dx, dy)| {
                pixel(&small, x + dx, y + dy) == a
                    && pixel(&big, x * 2 + dx * 2, y * 2 + dy * 2) == b
            });
            if neighbours_agree {
                assert!(near(a, b, 2), "at {x},{y}: {a:?} against {b:?}");
                compared += 1;
            }
        }
    }
    assert!(
        compared > 500,
        "enough flat pixels were compared: {compared}"
    );
}

#[test]
fn a_different_screen_fails_against_the_golden_at_every_size() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    for size in SIZES {
        let other = render(&mut r, &draw(Shot::Menu(ScreenId::Pause)), size);
        let failures = std::env::temp_dir().join(format!(
            "ss2-golden-screens-{}-{}x{}",
            std::process::id(),
            size.0,
            size.1
        ));
        let mode = Mode {
            update: false,
            failures_dir: Some(failures.clone()),
        };
        let err = golden::check_golden(
            &golden_dir(),
            &name(Shot::Menu(ScreenId::Main), size),
            &other,
            &Tolerance::default(),
            &mode,
        )
        .expect_err("the pause screen is not the main screen");
        assert!(matches!(err, golden::GoldenError::Differs { .. }), "{err}");
        let _ = std::fs::remove_dir_all(&failures);
    }
}

// ---- the 2D renderer screens (T04-017): 9-slice panels, a clipped list, render-to-texture -------------------
//
// These do not come from `ss2-ui` (it does not produce the new commands yet, T04-017b): each is a `UiFrame` built
// here, from images made in code, drawn by the real `Renderer` at the same five sizes as the screens above.

/// One picture of the 2D renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shot2d {
    Panels,
    List,
    Target,
}

const SHOTS_2D: [Shot2d; 3] = [Shot2d::Panels, Shot2d::List, Shot2d::Target];

fn word_2d(shot: Shot2d) -> &'static str {
    match shot {
        Shot2d::Panels => "panels",
        Shot2d::List => "list",
        Shot2d::Target => "target",
    }
}

fn name_2d(shot: Shot2d, (w, h): (u32, u32)) -> String {
    format!("ui2d_{}_{w}x{h}.png", word_2d(shot))
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn image_from(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> ImageRgba8 {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.extend(f(x, y));
        }
    }
    ImageRgba8 {
        width: w,
        height: h,
        pixels,
    }
}

/// A 48x48 panel with a 12-texel border: dark outline, gold bevel, steel frame with a rivet in each corner,
/// and a translucent blue centre with a faint vertical gradient.
fn panel_image() -> ImageRgba8 {
    image_from(48, 48, |x, y| {
        let d = x.min(y).min(47 - x).min(47 - y);
        let (cx, cy) = (x.min(47 - x) as f32, y.min(47 - y) as f32);
        let rivet = ((cx - 6.0).powi(2) + (cy - 6.0).powi(2)).sqrt();
        if d < 2 {
            [16, 20, 28, 255]
        } else if d < 5 {
            let t = (d - 2) as f32 / 3.0;
            [(210.0 - 60.0 * t) as u8, (170.0 - 50.0 * t) as u8, 70, 255]
        } else if d < 12 {
            if x.min(47 - x) < 12 && y.min(47 - y) < 12 && rivet < 2.6 {
                [235, 235, 245, 255]
            } else {
                [58, 68, 92, 255]
            }
        } else {
            [24, 34 + (y / 3) as u8, 62 + (y / 2) as u8, 225]
        }
    })
}

/// A 24x24 tile image: 4-texel border, and a 16x16 centre that is a checker of 8-texel squares.
fn tile_image() -> ImageRgba8 {
    image_from(24, 24, |x, y| {
        if x.min(y).min(23 - x).min(23 - y) < 4 {
            [120, 90, 50, 255]
        } else if ((x - 4) / 8 + (y - 4) / 8) % 2 == 0 {
            [200, 70, 60, 255]
        } else {
            [230, 200, 120, 255]
        }
    })
}

/// A 32x32 round badge with a radial gradient in `rgb`.
fn badge_image(rgb: [u8; 3]) -> ImageRgba8 {
    image_from(32, 32, |x, y| {
        let r = ((x as f32 - 15.5).powi(2) + (y as f32 - 15.5).powi(2)).sqrt();
        if r > 15.0 {
            [0, 0, 0, 0]
        } else {
            let k = 1.0 - r / 22.0;
            [
                (f32::from(rgb[0]) * k) as u8,
                (f32::from(rgb[1]) * k) as u8,
                (f32::from(rgb[2]) * k) as u8,
                255,
            ]
        }
    })
}

fn label(f: &mut UiFrame, x: f32, y: f32, size: f32, text: &'static str) {
    f.text(
        Vec2 { x, y },
        text,
        size,
        Color([0.92, 0.92, 0.95, 1.0]),
        TextAlign::Left,
    );
}

fn panel_nine(image: ImageHandle, dest: Rect, inset: f32, scale: f32, fill: Fill) -> UiCommand {
    UiCommand::NineSlice(NineSliceDraw {
        image,
        rect: dest,
        insets: Insets::uniform(inset),
        scale,
        fill,
        tint: Color([1.0; 4]),
        filter: Filter::Linear,
    })
}

/// A "card" scene in the pixels of whatever it is drawn into, at `(ox, oy)`, 200x120.
fn card(f: &mut UiFrame, panel: ImageHandle, badge: ImageHandle, (ox, oy): (f32, f32)) {
    f.quad(rect(ox, oy, 200.0, 120.0), Color([0.12, 0.14, 0.2, 1.0]));
    f.push(panel_nine(
        panel,
        rect(ox + 6.0, oy + 6.0, 188.0, 108.0),
        12.0,
        1.0,
        Fill::Stretch,
    ));
    f.image(badge, rect(ox + 18.0, oy + 18.0, 32.0, 32.0));
    label(f, ox + 60.0, oy + 18.0, 28.0, "Card");
    f.push_clip(rect(ox + 18.0, oy + 62.0, 110.0, 34.0));
    label(f, ox + 18.0, oy + 62.0, 26.0, "Clipped text here");
    f.pop_clip();
}

/// What a shot draws (canvas units), with the images and render targets it needs made on `r`.
fn frame_2d(shot: Shot2d, r: &mut Renderer) -> UiFrame {
    let panel = r.create_image(&panel_image()).unwrap();
    let mut f = UiFrame::new();
    match shot {
        Shot2d::Panels => {
            let tile = r.create_image(&tile_image()).unwrap();
            label(&mut f, 80.0, 40.0, 44.0, "9-slice panels");
            // (x, y, w, h, scale, caption): the same 48x48 image at six sizes; the last is smaller than its borders.
            let panels: [(f32, f32, f32, f32, f32, &'static str); 5] = [
                (80.0, 140.0, 96.0, 64.0, 1.0, "96x64"),
                (220.0, 140.0, 220.0, 110.0, 1.0, "220x110"),
                (480.0, 140.0, 360.0, 170.0, 1.0, "360x170"),
                (880.0, 140.0, 300.0, 300.0, 1.0, "300x300"),
                (80.0, 520.0, 700.0, 120.0, 1.0, "700x120"),
            ];
            for (x, y, w, h, scale, caption) in panels {
                f.push(panel_nine(
                    panel,
                    rect(x, y, w, h),
                    12.0,
                    scale,
                    Fill::Stretch,
                ));
                label(&mut f, x, y + h + 10.0, 26.0, caption);
            }
            // Smaller than the borders: they shrink, no centre.
            f.push(panel_nine(
                panel,
                rect(1220.0, 140.0, 20.0, 20.0),
                12.0,
                1.0,
                Fill::Stretch,
            ));
            label(&mut f, 1220.0, 170.0, 26.0, "20x20");
            // The same image with its border drawn at 3x: texels are not squeezed, they are enlarged.
            f.push(UiCommand::NineSlice(NineSliceDraw {
                image: panel,
                rect: rect(1340.0, 140.0, 300.0, 200.0),
                insets: Insets::uniform(12.0),
                scale: 2.0,
                fill: Fill::Stretch,
                tint: Color([1.0, 0.8, 0.8, 1.0]),
                filter: Filter::Nearest,
            }));
            label(&mut f, 1340.0, 350.0, 26.0, "border x2, tinted, nearest");
            // Tiling.
            f.push(panel_nine(
                tile,
                rect(880.0, 520.0, 400.0, 200.0),
                4.0,
                1.0,
                Fill::Tile,
            ));
            f.push(panel_nine(
                tile,
                rect(1320.0, 520.0, 400.0, 200.0),
                4.0,
                2.0,
                Fill::Tile,
            ));
            label(&mut f, 880.0, 730.0, 26.0, "tiled, 1x and 2x");
        }
        Shot2d::List => {
            let badges = [
                r.create_image(&badge_image([240, 120, 40])).unwrap(),
                r.create_image(&badge_image([60, 200, 120])).unwrap(),
                r.create_image(&badge_image([90, 140, 250])).unwrap(),
            ];
            f.push(panel_nine(
                panel,
                rect(60.0, 40.0, 700.0, 1000.0),
                12.0,
                1.0,
                Fill::Stretch,
            ));
            label(&mut f, 100.0, 62.0, 40.0, "Clipped list");
            let (top, height) = (140.0, 780.0);
            f.push_clip(rect(100.0, top, 620.0, height));
            let names = [
                "Rusty Gate",
                "Sewer Run",
                "Courtyard",
                "The Long Bridge",
                "Cliffside",
                "Boiler Room",
                "Temple Steps",
                "Market Square",
                "Dry Dock",
                "Signal Tower",
            ];
            for (i, name) in names.iter().enumerate() {
                // Scrolled by 47 units: the first row is cut at the top, the last at the bottom.
                let y = top - 47.0 + i as f32 * 90.0;
                let color = if i % 2 == 0 { ROW_A } else { ROW_B };
                f.quad(rect(100.0, y, 620.0, 84.0), color);
                f.image(badges[i % 3], rect(112.0, y + 10.0, 64.0, 64.0));
                f.push_clip(rect(196.0, y, 420.0, 84.0));
                f.text(
                    Vec2 {
                        x: 196.0,
                        y: y + 18.0,
                    },
                    format!("{name}: a label that does not fit its cell"),
                    34.0,
                    Color([1.0; 4]),
                    TextAlign::Left,
                );
                f.pop_clip();
                f.quad(
                    rect(628.0, y + 28.0, 80.0, 28.0),
                    Color([0.0, 0.0, 0.0, 0.35]),
                );
            }
            f.pop_clip();
            // Scrollbar.
            f.quad(rect(724.0, top, 8.0, height), Color([0.0, 0.0, 0.0, 0.4]));
            f.quad(
                rect(724.0, top + 60.0, 8.0, 240.0),
                Color([0.9, 0.7, 0.3, 1.0]),
            );

            // A second clip: big text in a small window, then the window's frame.
            f.push_clip(rect(820.0, 140.0, 400.0, 110.0));
            f.quad(
                rect(820.0, 140.0, 400.0, 110.0),
                Color([0.1, 0.1, 0.15, 1.0]),
            );
            f.text(
                Vec2 { x: 790.0, y: 150.0 },
                "WIDE LARGE TEXT CUT BY ITS WINDOW",
                72.0,
                Color([1.0, 0.8, 0.3, 1.0]),
                TextAlign::Left,
            );
            f.pop_clip();
            // An image and a 9-slice cut by a window with fractional edges.
            f.push_clip(rect(820.5, 300.4, 200.3, 120.6));
            f.image_ex(
                badges[0],
                rect(800.0, 280.0, 256.0, 256.0),
                UvRect::FULL,
                Color([1.0; 4]),
                Filter::Linear,
            );
            f.pop_clip();
            f.push_clip(rect(1230.5, 270.4, 200.3, 140.6));
            f.push(panel_nine(
                panel,
                rect(1260.0, 300.0, 400.0, 300.0),
                12.0,
                1.0,
                Fill::Stretch,
            ));
            f.pop_clip();
        }
        Shot2d::Target => {
            let badge = r.create_image(&badge_image([240, 120, 40])).unwrap();
            let a = r.create_render_target(200, 120).unwrap();
            let mut card_a = UiFrame::new();
            card(&mut card_a, panel, badge, (0.0, 0.0));
            r.render_to_target(a, &card_a, Color([0.0; 4])).unwrap();

            // A second target drawn from the first, twice (the second time faded), over a translucent plate.
            let b = r.create_render_target(440, 150).unwrap();
            let mut nested = UiFrame::new();
            nested.quad(rect(0.0, 0.0, 440.0, 150.0), Color([0.0, 0.0, 0.0, 0.45]));
            nested.image(a, rect(10.0, 15.0, 200.0, 120.0));
            nested.image_ex(
                a,
                rect(230.0, 15.0, 200.0, 120.0),
                UvRect::FULL,
                Color([1.0, 1.0, 1.0, 0.5]),
                Filter::Nearest,
            );
            r.render_to_target(b, &nested, Color([0.0; 4])).unwrap();

            label(&mut f, 80.0, 40.0, 44.0, "Render to texture");
            f.quad(
                rect(60.0, 120.0, 1800.0, 900.0),
                Color([0.18, 0.3, 0.22, 1.0]),
            );
            // 1:1, nearest.
            f.image(a, rect(80.0, 140.0, 200.0, 120.0));
            label(&mut f, 80.0, 268.0, 26.0, "1:1");
            // 2x, linear.
            f.image_ex(
                a,
                rect(340.0, 140.0, 400.0, 240.0),
                UvRect::FULL,
                Color([1.0; 4]),
                Filter::Linear,
            );
            label(&mut f, 340.0, 388.0, 26.0, "2x linear");
            // Half strength over a pattern.
            f.quad(
                rect(800.0, 140.0, 100.0, 240.0),
                Color([0.9, 0.2, 0.2, 1.0]),
            );
            f.quad(
                rect(900.0, 140.0, 100.0, 240.0),
                Color([0.2, 0.2, 0.9, 1.0]),
            );
            f.image_ex(
                a,
                rect(800.0, 200.0, 200.0, 120.0),
                UvRect::FULL,
                Color([1.0, 1.0, 1.0, 0.6]),
                Filter::Nearest,
            );
            label(&mut f, 800.0, 388.0, 26.0, "60% over a pattern");
            // Nested.
            f.image(b, rect(80.0, 520.0, 440.0, 150.0));
            label(&mut f, 80.0, 680.0, 26.0, "a target drawn from a target");
            f.image_ex(
                b,
                rect(600.0, 520.0, 660.0, 225.0),
                UvRect::FULL,
                Color([1.0; 4]),
                Filter::Linear,
            );
            // A part of a target (its right half).
            f.image_ex(
                a,
                rect(1340.0, 520.0, 300.0, 180.0),
                UvRect {
                    u0: 0.5,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
                Color([1.0; 4]),
                Filter::Linear,
            );
            label(&mut f, 1340.0, 710.0, 26.0, "right half");
        }
    }
    f
}

const ROW_A: Color = Color([0.24, 0.46, 0.30, 1.0]);
const ROW_B: Color = Color([0.20, 0.34, 0.58, 1.0]);

fn one_shot_2d(shot: Shot2d) {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    let frame = frame_2d(shot, &mut r);
    for size in SIZES {
        r.resize(size.0, size.1);
        let img = r.render_frame_to_image(&frame, theme::BACKGROUND).unwrap();
        check_2d(shot, size, &img);
        check_golden(&name_2d(shot, size), &img);
    }
}

/// The checks that need no golden.
fn check_2d(shot: Shot2d, size: (u32, u32), img: &ImageRgba8) {
    let what = format!("{} at {}x{}", word_2d(shot), size.0, size.1);
    assert_eq!(
        (img.width, img.height),
        size,
        "{what}: the image has the size asked for"
    );
    let canvas = Canvas::fit(size.0, size.1).unwrap();
    let background = background_bytes();
    // Nothing is drawn in the bars outside the canvas.
    let (left, top) = (
        canvas.offset.x.floor() as i64 - 2,
        canvas.offset.y.floor() as i64 - 2,
    );
    for y in (0..size.1).step_by(5) {
        for x in 0..size.0 {
            let in_bar = (left > 0
                && (i64::from(x) < left || i64::from(x) > i64::from(size.0) - left - 1))
                || (top > 0 && (i64::from(y) < top || i64::from(y) > i64::from(size.1) - top - 1));
            if in_bar {
                assert!(
                    near(pixel(img, x, y), background, 1),
                    "{what}: bar at {x},{y}"
                );
            }
        }
    }
    let colour_bytes = |c: Color| [0, 1, 2, 3].map(|i| (c.0[i] * 255.0).round() as u8);
    match shot {
        Shot2d::List => {
            // The list rows are clipped to their window: strips just above and just below it hold no row colour.
            let strips = [
                (100.0, 140.0 - 28.0, 620.0, 26.0),
                (100.0, 140.0 + 780.0 + 2.0, 620.0, 26.0),
            ];
            for (x, y, w, h) in strips {
                let a = canvas.to_screen(Vec2 { x, y });
                let b = canvas.to_screen(Vec2 { x: x + w, y: y + h });
                for py in a.y.ceil() as u32..b.y.floor() as u32 {
                    for px in a.x.ceil() as u32..b.x.floor() as u32 {
                        let p = pixel(img, px, py);
                        assert!(
                            !near(p, colour_bytes(ROW_A), 8) && !near(p, colour_bytes(ROW_B), 8),
                            "{what}: a row colour {p:?} outside the clip at {px},{py}"
                        );
                    }
                }
            }
            // And inside the window the rows are there: a pixel of the second row's left margin.
            let at = canvas.to_screen(Vec2 {
                x: 104.0,
                y: 140.0 - 47.0 + 90.0 + 4.0,
            });
            assert!(
                near(pixel(img, at.x as u32, at.y as u32), colour_bytes(ROW_B), 8),
                "{what}: the second row is where it should be"
            );
        }
        Shot2d::Panels | Shot2d::Target => {}
    }
}

#[test]
fn nine_slice_panels_match_their_goldens_at_every_size() {
    one_shot_2d(Shot2d::Panels);
}

#[test]
fn a_scissored_list_matches_its_goldens_at_every_size() {
    one_shot_2d(Shot2d::List);
}

#[test]
fn render_to_texture_matches_its_goldens_at_every_size() {
    one_shot_2d(Shot2d::Target);
}

/// Needs no golden: the card drawn through a render target at 1:1 is the same picture as the card drawn in place.
#[test]
fn a_card_through_a_render_target_is_the_card_drawn_in_place() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    r.resize(1920, 1080);
    let panel = r.create_image(&panel_image()).unwrap();
    let badge = r.create_image(&badge_image([240, 120, 40])).unwrap();
    let target = r.create_render_target(200, 120).unwrap();
    let mut inner = UiFrame::new();
    card(&mut inner, panel, badge, (0.0, 0.0));
    r.render_to_target(target, &inner, Color([0.0; 4])).unwrap();
    let mut through = UiFrame::new();
    through.image(target, rect(300.0, 200.0, 200.0, 120.0));
    let mut direct = UiFrame::new();
    card(&mut direct, panel, badge, (300.0, 200.0));
    let a = r
        .render_frame_to_image(&through, theme::BACKGROUND)
        .unwrap();
    let b = r.render_frame_to_image(&direct, theme::BACKGROUND).unwrap();
    assert_eq!(a, b);
}

/// Needs no adapter: a golden missing from the 2D matrix is a failure even where the rendering tests skip.
#[test]
fn every_2d_renderer_screen_has_a_golden_at_every_size() {
    if Mode::from_env(PathBuf::new()).update {
        return;
    }
    let mut missing = Vec::new();
    for shot in SHOTS_2D {
        for size in SIZES {
            if !golden_dir().join(name_2d(shot, size)).is_file() {
                missing.push(name_2d(shot, size));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing goldens (run with UPDATE_GOLDEN=1): {missing:?}"
    );
    assert_eq!(SHOTS_2D.len() * SIZES.len(), 15);
}

/// Needs no adapter: a golden missing from the matrix is a failure even where the rendering tests skip.
#[test]
fn every_screen_has_a_golden_at_every_size() {
    if Mode::from_env(PathBuf::new()).update {
        // The other tests are writing the goldens at this very moment.
        return;
    }
    let mut missing = Vec::new();
    for shot in SHOTS {
        for size in SIZES {
            if !golden_dir().join(name(shot, size)).is_file() {
                missing.push(name(shot, size));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing goldens (run with UPDATE_GOLDEN=1): {missing:?}"
    );
    assert_eq!(
        SHOTS.len() * SIZES.len(),
        140,
        "the matrix is twenty-eight screens by five sizes"
    );
}
