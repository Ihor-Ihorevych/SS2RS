//! The texture of the example mod `examples/mods/HelloMod` (T13-010): it is made here, byte for
//! byte, so that nobody has to believe it is hand-edited game content, and it is read back by the
//! tool the modder tutorial uses (`asset-export tex`).

#[path = "../src/fixtures.rs"]
#[allow(dead_code)]
mod fixtures;

use std::path::PathBuf;
use std::process::Command;

const TEXTURE: &str = "examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex";

/// An 8 x 8 24-bit texture: a checkerboard of 2 x 2 pixel squares, orange and dark blue.
fn checker_file() -> Vec<u8> {
    let mut px = Vec::new();
    for y in 0..8u8 {
        for x in 0..8u8 {
            if (x / 2 + y / 2) % 2 == 0 {
                px.extend([0x20, 0x90, 0xF0]); // B, G, R: orange
            } else {
                px.extend([0x60, 0x20, 0x10]); // B, G, R: dark blue
            }
        }
    }
    fixtures::texture_file(8, 8, 0x20, 0, (0x50, 0x50), &px)
}

fn texture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(TEXTURE)
}

/// The committed file is exactly what the builder makes. `UPDATE_FIXTURES=1 cargo test -p
/// asset-export --test example_mod` rewrites it.
#[test]
fn the_example_mods_texture_is_what_the_builder_makes() {
    let bytes = checker_file();
    let path = texture_path();
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
    }
    let on_disk = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(on_disk, bytes, "{TEXTURE} differs from its builder");
    assert!(bytes.len() < 256 * 1024);
}

/// The tutorial's command works on it and gives the picture the builder describes.
#[test]
fn asset_export_turns_the_example_texture_into_an_8x8_png() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("checker.png");
    let status = Command::new(env!("CARGO_BIN_EXE_asset-export"))
        .args(["tex", texture_path().to_str().unwrap(), "-o"])
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    let mut reader = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&out).unwrap()))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (8, 8));
    // Pixel (0, 0) is orange, pixel (2, 0) dark blue; the PNG is RGB.
    assert_eq!(&buf[0..3], [0xF0, 0x90, 0x20]);
    assert_eq!(&buf[6..9], [0x10, 0x20, 0x60]);
}
