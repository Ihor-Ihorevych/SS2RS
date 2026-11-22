//! Schema version, migrations, per-platform defaults and overrides of [`Settings`] (T03-014).

use crate::settings::{
    MIGRATIONS, Migration, PlatformClass, SCHEMA_VERSION, Settings, migrate_table,
};
use std::fs;

fn table(text: &str) -> toml::Table {
    text.parse().unwrap()
}

#[test]
fn saved_files_carry_the_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    Settings::default().save_to(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains(&format!("version = {SCHEMA_VERSION}")),
        "{text}"
    );
}

#[test]
fn the_production_migrations_reach_the_current_version() {
    // One migration per version step: MIGRATIONS[n] upgrades version n to n + 1.
    assert_eq!(MIGRATIONS.len(), SCHEMA_VERSION as usize);
}

#[test]
fn a_file_without_a_version_is_version_zero_and_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(&path, "vsync = false\nmaster_volume = 0.3\n").unwrap();
    let s = Settings::load_from(&path);
    assert!(!s.vsync);
    assert_eq!(s.master_volume, 0.3);
}

#[test]
fn a_file_from_a_newer_build_loads_the_fields_it_knows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(
        &path,
        format!(
            "version = {}\nfullscreen = true\nhdr = \"future\"\n",
            SCHEMA_VERSION + 5
        ),
    )
    .unwrap();
    let s = Settings::load_from(&path);
    assert!(s.fullscreen);
}

#[test]
fn a_roundtrip_keeps_every_value_with_the_version_present() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    let saved = Settings {
        vsync: false,
        fullscreen: true,
        master_volume: 0.25,
        music_volume: 0.5,
        sfx_volume: 0.75,
    };
    saved.save_to(&path).unwrap();
    assert_eq!(Settings::load_from(&path), saved);
}

fn rename_old_volume(t: &mut toml::Table) {
    if let Some(v) = t.remove("old_volume") {
        t.insert("master_volume".into(), v);
    }
}

fn halve_master_volume(t: &mut toml::Table) {
    if let Some(toml::Value::Float(v)) = t.get_mut("master_volume") {
        *v /= 2.0;
    }
}

const TEST_MIGRATIONS: [Migration; 2] = [rename_old_volume, halve_master_volume];

#[test]
fn migrations_run_in_order_from_the_files_version() {
    let mut t = table("old_volume = 0.8\n");
    let reached = migrate_table(&mut t, 0, &TEST_MIGRATIONS);
    assert_eq!(reached, 2);
    assert_eq!(t["master_volume"].as_float(), Some(0.4));
    assert!(!t.contains_key("old_volume"));
}

#[test]
fn a_newer_file_skips_the_migrations_it_already_has() {
    let mut t = table("master_volume = 0.8\n");
    let reached = migrate_table(&mut t, 1, &TEST_MIGRATIONS);
    assert_eq!(reached, 2);
    assert_eq!(
        t["master_volume"].as_float(),
        Some(0.4),
        "only the second step ran"
    );
}

#[test]
fn a_current_or_future_file_is_left_alone() {
    let mut t = table("master_volume = 0.8\n");
    assert_eq!(migrate_table(&mut t, 2, &TEST_MIGRATIONS), 2);
    assert_eq!(migrate_table(&mut t, 9, &TEST_MIGRATIONS), 9);
    assert_eq!(t["master_volume"].as_float(), Some(0.8));
}

#[test]
fn mobile_defaults_are_fullscreen_and_desktop_defaults_are_unchanged() {
    assert!(Settings::for_platform(PlatformClass::Mobile).fullscreen);
    assert_eq!(
        Settings::for_platform(PlatformClass::Desktop),
        Settings::default()
    );
    let current = Settings::for_platform(PlatformClass::current());
    assert_eq!(
        current.fullscreen,
        cfg!(any(target_os = "android", target_os = "ios"))
    );
}

#[test]
fn a_missing_file_gives_the_platform_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let s = Settings::load_from(&dir.path().join("nope.toml"));
    assert_eq!(s, Settings::for_platform(PlatformClass::current()));
}

#[test]
fn overrides_set_values_by_name() {
    let mut s = Settings::default();
    s.apply_override("vsync=off").unwrap();
    s.apply_override("master_volume=0.25").unwrap();
    s.apply_override("fullscreen=true").unwrap();
    assert!(!s.vsync && s.fullscreen);
    assert_eq!(s.master_volume, 0.25);
}

#[test]
fn override_values_are_clamped_like_loaded_ones() {
    let mut s = Settings::default();
    s.apply_override("music_volume=3").unwrap();
    assert_eq!(s.music_volume, 1.0);
}

#[test]
fn bad_overrides_are_errors_that_name_the_problem() {
    let mut s = Settings::default();
    let unknown = s.apply_override("brightness=1").unwrap_err().to_string();
    assert!(
        unknown.contains("brightness") && unknown.contains("master_volume"),
        "{unknown}"
    );
    let bad_value = s.apply_override("vsync=maybe").unwrap_err().to_string();
    assert!(
        bad_value.contains("vsync") && bad_value.contains("maybe"),
        "{bad_value}"
    );
    let bad_number = s.apply_override("sfx_volume=loud").unwrap_err().to_string();
    assert!(bad_number.contains("sfx_volume"), "{bad_number}");
    let no_equals = s.apply_override("vsync").unwrap_err().to_string();
    assert!(no_equals.contains("KEY=VALUE"), "{no_equals}");
    assert_eq!(s, Settings::default(), "failed overrides change nothing");
}

#[test]
fn a_list_of_overrides_applies_all_or_nothing() {
    let mut s = Settings::default();
    let result = s.apply_overrides(["vsync=off", "master_volume=0.1", "nonsense=1"]);
    assert!(result.is_err());
    assert_eq!(s, Settings::default());
    s.apply_overrides(["vsync=off", "master_volume=0.1"])
        .unwrap();
    assert!(!s.vsync);
    assert_eq!(s.master_volume, 0.1);
}
