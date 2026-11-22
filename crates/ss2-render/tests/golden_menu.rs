//! Golden-image and highlight tests for the placeholder menu. They need a GPU adapter (a
//! software fallback is fine) and skip with a message when none exists.
//! Regenerate goldens with: `UPDATE_GOLDEN=1 cargo test -p ss2-render --test golden_menu`.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_render::{ImageRgba8, RenderError, Renderer, mean_abs_diff};
use ss2_ui::{Menu, UiInput, theme};

const SIZE: (u32, u32) = (1280, 720);
const TOLERANCE: f32 = 4.0;

fn renderer() -> Option<Renderer> {
    match Renderer::new_offscreen(SIZE) {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            None
        }
        Err(e) => panic!("renderer creation failed: {e}"),
    }
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/golden")
        .join(name)
}

fn save_png(path: &PathBuf, img: &ImageRgba8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut enc = png::Encoder::new(
        BufWriter::new(File::create(path).unwrap()),
        img.width,
        img.height,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&img.pixels)
        .unwrap();
}

fn load_png(path: &PathBuf) -> ImageRgba8 {
    let decoder = png::Decoder::new(BufReader::new(
        File::open(path).expect("golden file missing"),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba, "golden must be RGBA");
    buf.truncate(info.buffer_size());
    ImageRgba8 {
        width: info.width,
        height: info.height,
        pixels: buf,
    }
}

fn check_golden(name: &str, img: &ImageRgba8) {
    let path = golden_path(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        save_png(&path, img);
        return;
    }
    let golden = load_png(&path);
    let diff = mean_abs_diff(img, &golden).expect("golden size differs from render size");
    assert!(
        diff <= TOLERANCE,
        "{name}: mean abs diff {diff} > {TOLERANCE}"
    );
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
    let Some(mut r) = renderer() else { return };
    check_golden(
        "menu_main_1280x720.png",
        &render(&mut r, &Menu::new(Settings::default())),
    );
}

#[test]
fn options_menu_matches_golden() {
    let Some(mut r) = renderer() else { return };
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
    let Some(mut r) = renderer() else { return };
    check_golden("menu_about_1280x720.png", &render(&mut r, &about_menu()));
}

// Canvas 1920x1080 -> 1280x720 is scale 2/3. First button: canvas (640..1280, 320..392),
// second: y 408..480. Sample near the left edge, away from the label text.
const SAMPLE_X: u32 = 440;
const FIRST_Y: u32 = 220;
const SECOND_Y: u32 = 280;

#[test]
fn focus_highlight_is_orange_at_first_button() {
    let Some(mut r) = renderer() else { return };
    let img = render(&mut r, &Menu::new(Settings::default()));
    let p = pixel(&img, SAMPLE_X, FIRST_Y);
    assert!(p[0] > 180 && p[2] < 60, "expected orange, got {p:?}");
}

#[test]
fn moving_focus_moves_highlight() {
    let Some(mut r) = renderer() else { return };
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
