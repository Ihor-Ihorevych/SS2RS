//! A settings file of any content loads: broken files give the defaults, never a panic, and the
//! values come out clamped.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_core::settings::Settings;

fuzz_target!(|data: &[u8]| {
    let path = std::env::temp_dir().join(format!("ss2-fuzz-settings-{}.toml", std::process::id()));
    if std::fs::write(&path, data).is_err() {
        return;
    }
    let settings = Settings::load_from(&path);
    for v in [
        settings.master_volume,
        settings.music_volume,
        settings.sfx_volume,
    ] {
        assert!((0.0..=1.0).contains(&v), "volume {v} escaped the clamp");
    }
});
