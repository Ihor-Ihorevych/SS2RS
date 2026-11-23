//! Encode offscreen menu screenshots and in-window captures as PNG.

use std::fs::File;
use std::io::BufWriter;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_render::{ImageRgba8, Renderer};
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
        ScreenId::About => {
            menu.handle(UiInput::Nav(NavInput::Down));
            menu.handle(UiInput::Nav(NavInput::Down));
            menu.handle(UiInput::Nav(NavInput::Activate));
        }
    }
    let image = renderer
        .render_to_image(&menu.draw(), theme::BACKGROUND)
        .map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let file = File::create(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    write_png(file, &image, out)
}

/// Write a capture to a new file without overwriting an existing screenshot.
pub fn save_interactive(dir: &Path, image: &ImageRgba8) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    save_interactive_at(dir, image, timestamp)
}

fn save_interactive_at(dir: &Path, image: &ImageRgba8, timestamp: u128) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    for sequence in 1..=u32::MAX {
        let out = dir.join(format!("ss2rs-{timestamp}-{sequence}.png"));
        match File::options().write(true).create_new(true).open(&out) {
            Ok(file) => {
                let result = write_png(file, image, &out);
                if result.is_err() {
                    let _ = std::fs::remove_file(&out);
                }
                return result.map(|()| out);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("cannot create {}: {e}", out.display())),
        }
    }
    Err(format!("no screenshot name available in {}", dir.display()))
}

fn write_png(file: File, image: &ImageRgba8, out: &Path) -> Result<(), String> {
    let mut encoder = png::Encoder::new(BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| {
            writer.write_image_data(&image.pixels)?;
            writer.finish()
        })
        .map_err(|e| format!("cannot write {}: {e}", out.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_time_captures_get_distinct_pngs_without_overwriting() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("screenshots");
        let mut image = ImageRgba8 {
            width: 1,
            height: 1,
            pixels: vec![12, 34, 56, 255],
        };
        let first = save_interactive_at(&folder, &image, 123).unwrap();
        image.pixels[0] = 78;
        let second = save_interactive_at(&folder, &image, 123).unwrap();
        assert_ne!(first, second);

        for (path, first_channel) in [(first, 12), (second, 78)] {
            assert_eq!(path.parent(), Some(folder.as_path()));
            assert_eq!(path.extension().unwrap(), "png");
            let file = std::io::BufReader::new(File::open(path).unwrap());
            let mut decoder = png::Decoder::new(file).read_info().unwrap();
            let mut pixels = [0; 4];
            decoder.next_frame(&mut pixels).unwrap();
            assert_eq!(pixels, [first_channel, 34, 56, 255]);
        }
    }
}
