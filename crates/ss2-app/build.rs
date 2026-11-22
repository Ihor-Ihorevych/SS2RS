//! Embeds the Windows application manifest into `ss2-app.exe` (T03-026). It does nothing for
//! other targets. The manifest turns on per-monitor DPI awareness (so the window is not blurred
//! by Windows' bitmap scaling on a HiDPI screen), names Windows 10 as the supported baseline,
//! and enables long paths and the UTF-8 code page. `tests/windows_manifest.rs` reads it back
//! from the built executable.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    embed();
}

#[cfg(windows)]
fn embed() {
    use embed_manifest::manifest::{ActiveCodePage, DpiAwareness, Setting, SupportedOS::Windows10};
    use embed_manifest::{embed_manifest, new_manifest};

    // Only when the *target* is Windows (a build script runs on the host).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let manifest = new_manifest("SS2RS")
        .supported_os(Windows10..=Windows10)
        .dpi_awareness(DpiAwareness::PerMonitorV2)
        .long_path_aware(Setting::Enabled)
        .active_code_page(ActiveCodePage::Utf8);
    embed_manifest(manifest).expect("cannot embed the Windows manifest");
}
