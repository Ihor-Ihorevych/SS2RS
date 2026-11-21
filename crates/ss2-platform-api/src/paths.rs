//! Where the app may read and write files. **No crate other than the platform layer decides
//! these**: everything receives a [`PlatformPaths`] and never calls `std::env::current_dir`,
//! `directories` or `dirs` itself, so a portable install, a test sandbox and Android's app
//! storage all work the same way.

use std::path::{Path, PathBuf};

use ss2_core::{Error, Result};

/// The writable and readable locations of one running app.
///
/// The OS-specific resolution (per-user config dir, `%APPDATA%`, XDG, Android app storage,
/// portable mode) is T03-012; this type is what it produces.
///
/// ```
/// use ss2_platform_api::PlatformPaths;
///
/// let paths = PlatformPaths::under("/tmp/ss2-sandbox");
/// assert!(paths.settings_file().ends_with("settings.toml"));
/// assert!(paths.save_file("slot1.sav").unwrap().starts_with(&paths.save_dir));
/// assert!(paths.save_file("../escape").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformPaths {
    /// Settings, key bindings, profiles (small, user-editable).
    pub config_dir: PathBuf,
    /// Savegames and per-player progress; the data the player would be sad to lose.
    pub save_dir: PathBuf,
    /// Rebuildable data: cooked assets, shader/pipeline caches. Safe to delete.
    pub cache_dir: PathBuf,
    /// Log files and crash reports.
    pub log_dir: PathBuf,
    /// The folder with the user's own Serious Sam 2 install (the one containing
    /// `All_PC_01.gro`), once known. `None` triggers the first-run "where is your game?" flow.
    /// Desktop only: the Phase 2 Android port reads game data through the file-provider trait
    /// (T02-005) instead of a path.
    pub game_data_dir: Option<PathBuf>,
}

impl PlatformPaths {
    /// All four writable directories under `root` (`config`, `saves`, `cache`, `logs`). This is
    /// both **portable mode** (`root` next to the executable) and the right choice for tests
    /// (`root` = a temp dir).
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            config_dir: root.join("config"),
            save_dir: root.join("saves"),
            cache_dir: root.join("cache"),
            log_dir: root.join("logs"),
            game_data_dir: None,
        }
    }

    /// The same paths with the game data folder set.
    #[must_use]
    pub fn with_game_data_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.game_data_dir = Some(dir.into());
        self
    }

    /// `<config_dir>/settings.toml`.
    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.toml")
    }

    /// `<config_dir>/bindings.toml`: the serialized [`ActionMap`](crate::ActionMap).
    pub fn bindings_file(&self) -> PathBuf {
        self.config_dir.join("bindings.toml")
    }

    /// A file inside `save_dir`. `name` must be a plain file name: no separators, no `..`, no
    /// drive letters, not empty. This keeps player- or mod-supplied names from escaping the
    /// save folder.
    ///
    /// # Errors
    /// [`Error::Invalid`] for an unsafe name.
    pub fn save_file(&self, name: &str) -> Result<PathBuf> {
        let unsafe_name = name.is_empty()
            || name == "."
            || name == ".."
            || name.contains(['/', '\\', ':', '\0'])
            || name.chars().any(char::is_control);
        if unsafe_name {
            return Err(Error::invalid(
                "save file name",
                format!("{name:?} is not a plain file name"),
            ));
        }
        Ok(self.save_dir.join(name))
    }

    /// Create the four writable directories if they do not exist.
    ///
    /// # Errors
    /// [`Error::Io`] naming the directory that could not be created.
    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            &self.config_dir,
            &self.save_dir,
            &self.cache_dir,
            &self.log_dir,
        ] {
            std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_places_everything_below_the_root() {
        let p = PlatformPaths::under("root");
        for dir in [&p.config_dir, &p.save_dir, &p.cache_dir, &p.log_dir] {
            assert!(dir.starts_with("root"), "{dir:?}");
        }
        assert_ne!(
            p.save_dir, p.cache_dir,
            "saves and cache must never share a folder"
        );
        assert_eq!(p.game_data_dir, None);
        assert_eq!(
            p.settings_file(),
            Path::new("root").join("config").join("settings.toml")
        );
        assert_eq!(
            p.bindings_file(),
            Path::new("root").join("config").join("bindings.toml")
        );
    }

    #[test]
    fn game_data_dir_is_set_separately() {
        let p = PlatformPaths::under("r").with_game_data_dir("D:/Games/Serious Sam 2");
        assert_eq!(
            p.game_data_dir.as_deref(),
            Some(Path::new("D:/Games/Serious Sam 2"))
        );
    }

    #[test]
    fn save_file_accepts_plain_names_only() {
        let p = PlatformPaths::under("r");
        assert_eq!(
            p.save_file("slot 1.sav").unwrap(),
            p.save_dir.join("slot 1.sav")
        );
        for bad in ["", ".", "..", "../x", "a/b", r"a\b", "C:x", "a\0b", "a\nb"] {
            assert!(p.save_file(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn ensure_dirs_creates_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let p = PlatformPaths::under(tmp.path().join("nested").join("root"));
        p.ensure_dirs().unwrap();
        p.ensure_dirs().unwrap();
        for dir in [&p.config_dir, &p.save_dir, &p.cache_dir, &p.log_dir] {
            assert!(dir.is_dir(), "{dir:?}");
        }
    }

    #[test]
    fn ensure_dirs_reports_the_failing_path() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("file");
        std::fs::write(&blocker, b"x").unwrap();
        let err = PlatformPaths::under(blocker.join("under-a-file"))
            .ensure_dirs()
            .unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
    }
}
