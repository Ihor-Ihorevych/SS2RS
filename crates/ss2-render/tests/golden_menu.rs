//! Golden-image and highlight tests for the placeholder menu. They need a GPU adapter (a
//! software fallback is fine) and skip with a message when none exists.
//! Regenerate goldens with: `UPDATE_GOLDEN=1 cargo test -p ss2-render --test golden_menu`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_render::golden::{self, Mode, Tolerance};
use ss2_render::{ImageRgba8, RenderError, Renderer};
use ss2_ui::{Menu, UiInput, theme};

const SIZE: (u32, u32) = (1280, 720);

/// The software adapter crashed the test process (an access violation) when several tests created
/// devices at the same time, so one test at a time owns the GPU.
static GPU: Mutex<()> = Mutex::new(());

fn renderer() -> Option<(MutexGuard<'static, ()>, Renderer)> {
    let guard = GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match Renderer::new_offscreen(SIZE) {
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

/// Compare with the golden file; a failure leaves the rendered and the difference images in
/// `target/golden-failures/` and says where.
fn check_golden(name: &str, img: &ImageRgba8) {
    let failures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-failures");
    let mode = Mode::from_env(failures);
    if let Err(e) = golden::check_golden(&golden_dir(), name, img, &Tolerance::default(), &mode) {
        panic!("{e}");
    }
}

fn render(r: &mut Renderer, menu: &Menu) -> ImageRgba8 {
    r.render_to_image(&menu.draw(), theme::BACKGROUND).unwrap()
}

fn pixel(img: &ImageRgba8, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.pixels[i..i + 4].try_into().unwrap()
}

fn options_menu() -> Menu {
    let mut m = Menu::new(Settings::default());
    m.handle(UiInput::Nav(NavInput::Down));
    m.handle(UiInput::Nav(NavInput::Activate));
    m
}

#[test]
fn main_menu_matches_golden() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    check_golden(
        "menu_main_1280x720.png",
        &render(&mut r, &Menu::new(Settings::default())),
    );
}

#[test]
fn options_menu_matches_golden() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    check_golden(
        "menu_options_1280x720.png",
        &render(&mut r, &options_menu()),
    );
}

fn about_menu() -> Menu {
    let mut m = Menu::new(Settings::default());
    m.handle(UiInput::Nav(NavInput::Down));
    m.handle(UiInput::Nav(NavInput::Down));
    m.handle(UiInput::Nav(NavInput::Activate));
    m
}

#[test]
fn about_menu_matches_golden() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    check_golden("menu_about_1280x720.png", &render(&mut r, &about_menu()));
}

// Canvas 1920x1080 -> 1280x720 is scale 2/3. First button: canvas (640..1280, 320..392),
// second: y 408..480. Sample near the left edge, away from the label text.
const SAMPLE_X: u32 = 440;
const FIRST_Y: u32 = 220;
const SECOND_Y: u32 = 280;

#[test]
fn focus_highlight_is_orange_at_first_button() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    let img = render(&mut r, &Menu::new(Settings::default()));
    let p = pixel(&img, SAMPLE_X, FIRST_Y);
    assert!(p[0] > 180 && p[2] < 60, "expected orange, got {p:?}");
}

#[test]
fn moving_focus_moves_highlight() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    let mut menu = Menu::new(Settings::default());
    menu.handle(UiInput::Nav(NavInput::Down));
    let img = render(&mut r, &menu);
    let first = pixel(&img, SAMPLE_X, FIRST_Y);
    let second = pixel(&img, SAMPLE_X, SECOND_Y);
    assert!(first[0] < 100, "first button should be idle, got {first:?}");
    assert!(
        second[0] > 180 && second[2] < 60,
        "second should be orange, got {second:?}"
    );
}

#[test]
fn a_different_screen_fails_against_a_golden_and_leaves_the_evidence() {
    let Some((_gpu, mut r)) = renderer() else {
        return;
    };
    let failures = std::env::temp_dir().join(format!("ss2-golden-menu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&failures);
    let mode = Mode {
        update: false,
        failures_dir: Some(failures.clone()),
    };
    let main = render(&mut r, &Menu::new(Settings::default()));
    let err = golden::check_golden(
        &golden_dir(),
        "menu_options_1280x720.png",
        &main,
        &Tolerance::default(),
        &mode,
    )
    .expect_err("the main menu is not the options menu");
    assert!(matches!(err, golden::GoldenError::Differs { .. }), "{err}");
    assert!(failures.join("menu_options_1280x720.actual.png").exists());
    assert!(failures.join("menu_options_1280x720.diff.png").exists());
    let _ = std::fs::remove_dir_all(&failures);
}
