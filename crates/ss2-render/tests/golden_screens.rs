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
use ss2_render::{ImageRgba8, RenderError, Renderer};
use ss2_ui::coop_overlays::{
    ChatBox, ChatConfig, ChatKey, ChatKind, CoopOverlays, Nameplate, PingKind, PingProjection,
    Projected, ScoreEntry, VoiceSample,
};
use ss2_ui::data_wizard::{DataWizard, WizardCandidate, WizardCommand, WizardInput, WizardReport};
use ss2_ui::devconsole::{ConsoleView, Corner, FrameStats, LineKind, draw_overlay, stats_lines};
use ss2_ui::session_screens::{DeathMode, DeathScreen, LevelStats, SummaryScreen};
use ss2_ui::{Canvas, Color, DrawList, Menu, ScreenId, UiInput, Vec2, theme};

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
}

const SHOTS: [Shot; 21] = [
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

fn draw(shot: Shot) -> DrawList {
    match shot {
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
        105,
        "the matrix is twenty-one screens by five sizes"
    );
}
