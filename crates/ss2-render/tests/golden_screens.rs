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
use ss2_ui::devconsole::{ConsoleView, Corner, FrameStats, LineKind, draw_overlay, stats_lines};
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
}

const SHOTS: [Shot; 6] = [
    Shot::Menu(ScreenId::Main),
    Shot::Menu(ScreenId::Options),
    Shot::Menu(ScreenId::Pause),
    Shot::Menu(ScreenId::About),
    Shot::Console,
    Shot::Overlay,
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
fn draw(shot: Shot) -> DrawList {
    match shot {
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
    if matches!(shot, Shot::Menu(_) | Shot::Overlay) {
        let focused = list
            .quads
            .iter()
            .find(|q| q.color == theme::FOCUSED)
            .unwrap_or_else(|| panic!("{what}: a screen with a focused button"));
        let at = canvas.to_screen(Vec2 {
            x: focused.rect.x + 12.0,
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
        30,
        "the matrix is six screens by five sizes"
    );
}
