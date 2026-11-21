use crate::settings::{Settings, default_path};
use std::fs;
use std::path::{Path, PathBuf};

fn in_dir(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("settings.toml")
}

#[test]
fn roundtrip_preserves_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = in_dir(&dir);
    let s = Settings {
        vsync: false,
        fullscreen: true,
        master_volume: 0.35,
        music_volume: 0.1,
        sfx_volume: 0.9,
    };
    s.save_to(&path).unwrap();
    assert_eq!(Settings::load_from(&path), s);
}

#[test]
fn missing_file_gives_defaults() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(Settings::load_from(&in_dir(&dir)), Settings::default());
}

#[test]
fn corrupt_file_gives_defaults() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "not toml {{{").unwrap();
    assert_eq!(Settings::load_from(&in_dir(&dir)), Settings::default());
}

#[test]
fn wrong_type_gives_defaults() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "vsync = \"yes\"\n").unwrap();
    assert_eq!(Settings::load_from(&in_dir(&dir)), Settings::default());
}

#[test]
fn empty_file_gives_defaults() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "").unwrap();
    assert_eq!(Settings::load_from(&in_dir(&dir)), Settings::default());
}

#[test]
fn partial_file_fills_missing_fields() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "vsync = false\n").unwrap();
    let s = Settings::load_from(&in_dir(&dir));
    assert!(!s.vsync);
    assert_eq!(s.master_volume, Settings::default().master_volume);
}

#[test]
fn unknown_fields_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "banana = 3\nvsync = false\n").unwrap();
    assert!(!Settings::load_from(&in_dir(&dir)).vsync);
}

#[test]
fn out_of_range_volume_is_clamped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "master_volume = 7.0\nmusic_volume = -1.0\n").unwrap();
    let s = Settings::load_from(&in_dir(&dir));
    assert_eq!(s.master_volume, 1.0);
    assert_eq!(s.music_volume, 0.0);
}

#[test]
fn nan_volume_falls_back_to_default() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(in_dir(&dir), "music_volume = nan\n").unwrap();
    assert_eq!(Settings::load_from(&in_dir(&dir)).music_volume, 0.6);
}

#[test]
fn save_creates_missing_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a").join("b").join("settings.toml");
    Settings::default().save_to(&path).unwrap();
    assert!(path.is_file());
}

#[test]
fn save_leaves_no_tmp_file() {
    let dir = tempfile::tempdir().unwrap();
    Settings::default().save_to(&in_dir(&dir)).unwrap();
    Settings::default().save_to(&in_dir(&dir)).unwrap(); // overwrite path too
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["settings.toml".to_string()]);
}

#[test]
fn save_to_unwritable_path_returns_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "i am a file").unwrap();
    let result = Settings::default().save_to(&blocker.join("settings.toml"));
    assert!(result.is_err());
}

#[test]
fn default_path_is_under_an_ss2rs_folder() {
    if let Some(p) = default_path() {
        assert!(
            p.ends_with(Path::new("SS2RS").join("settings.toml")),
            "{p:?}"
        );
    }
}
