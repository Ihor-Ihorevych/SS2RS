//! The Windows executable carries an application manifest (T03-026): per-monitor DPI awareness
//! (without it Windows scales the whole window as a blurry bitmap on a HiDPI screen), a Windows 10
//! baseline, long paths and the UTF-8 code page. The manifest is read back from the built
//! executable, so a broken build script or a dropped link argument fails here.

#![cfg(windows)]

fn manifest() -> String {
    let bytes = std::fs::read(env!("CARGO_BIN_EXE_ss2-app")).expect("the built executable");
    // A manifest is stored as plain UTF-8 text in the executable's resources.
    let start = bytes
        .windows(b"<assembly".len())
        .position(|w| w == b"<assembly")
        .expect("the executable has no embedded manifest");
    let end = bytes[start..]
        .windows(b"</assembly>".len())
        .position(|w| w == b"</assembly>")
        .expect("the embedded manifest is not closed")
        + start
        + b"</assembly>".len();
    String::from_utf8(bytes[start..end].to_vec()).expect("the manifest is UTF-8")
}

#[test]
fn the_window_is_per_monitor_dpi_aware() {
    let m = manifest();
    // Windows compares the value without regard to case; the list is in order of preference, so
    // Windows 10 1703+ takes per-monitor v2 and older Windows 10 falls back to per-monitor.
    let lower = m.to_lowercase();
    assert!(
        lower.contains("<dpiawareness") && lower.contains(">permonitorv2,permonitor<"),
        "{m}"
    );
    // The older setting for Windows versions that do not know `dpiAwareness`.
    assert!(lower.contains(">true/pm<"), "{m}");
}

#[test]
fn windows_10_is_the_baseline() {
    let m = manifest();
    // The supportedOS id of Windows 10 and 11.
    assert!(m.contains("{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"), "{m}");
    for (id, name) in [
        ("{35138b9a-5d96-4fbd-8e2d-a2440225f93a}", "Windows 7"),
        ("{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}", "Windows 8"),
        ("{1f676c76-80e1-4239-95bb-83d0f6d0da78}", "Windows 8.1"),
    ] {
        assert!(
            !m.contains(id),
            "{name} must not be listed as supported: {m}"
        );
    }
}

#[test]
fn long_paths_and_the_utf8_code_page_are_on() {
    let m = manifest();
    assert!(m.contains("longPathAware"), "{m}");
    assert!(m.contains("UTF-8"), "{m}");
}

#[test]
fn the_manifest_names_the_program() {
    assert!(manifest().contains("SS2RS"));
}
