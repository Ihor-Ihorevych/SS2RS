//! `--screenshot`: render one menu screen offscreen and save it as a PNG.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_render::Renderer;
use ss2_ui::{Menu, ScreenId, UiInput, theme};

/// Render `screen` at `size` into `out` (PNG, RGBA).
pub fn take(screen: ScreenId, out: &Path, size: (u32, u32)) -> Result<(), String> {
    let mut renderer = Renderer::new_offscreen(size).map_err(|e| e.to_string())?;
    let mut menu = Menu::new(Settings::default());
    match screen {
        ScreenId::Main => {}
        ScreenId::Options => {
            menu.handle(UiInput::Nav(NavInput::Down));
            menu.handle(UiInput::Nav(NavInput::Activate));
        }
        ScreenId::Pause => menu.open_pause(),
    }
    let image = renderer
        .render_to_image(&menu.draw(), theme::BACKGROUND)
        .map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let file = File::create(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&image.pixels))
        .map_err(|e| format!("cannot write {}: {e}", out.display()))
}
