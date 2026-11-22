//! Where the app may read and write files. **No crate other than the platform layer decides
//! these**: everything receives a [`PlatformPaths`] and never calls `std::env::current_dir`,
//! `directories` or `dirs` itself, so a portable install, a test sandbox and Android's app
//! storage all work the same way.

use std::path::{Path, PathBuf};

use ss2_core::{Error, Result};

/// The name of the app's folder inside each per-user base directory.
const APP_DIR: &str = "SS2RS";

/// Name of the folder (next to the executable) that holds everything in portable mode.
pub const PORTABLE_DIR: &str = "ss2-data";

/// An empty file with this name next to the executable switches the app to portable mode.
pub const PORTABLE_MARKER: &str = "ss2-portable";

/// The base directories the OS defines for the current user (`directories::BaseDirs`):
/// `%APPDATA%` and `%LOCALAPPDATA%` on Windows, `~/Library/...` on macOS, the XDG directories on
/// Linux. It is a plain value so the layout rules can be tested without touching the real OS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDirs {
    /// Per-user configuration (roaming on Windows).
    pub config: PathBuf,
    /// Per-user data that should be kept (saves).
    pub data: PathBuf,
    /// Per-user cache: rebuildable, safe to delete.
    pub cache: PathBuf,
    /// Per-user state such as logs (`XDG_STATE_HOME` on Linux, the local data dir elsewhere).
    pub state: PathBuf,
}

impl UserDirs {
    /// Ask the OS for the current user's base directories.
    ///
    /// # Errors
    /// [`Error::Invalid`] when the platform has no home directory for this user (for example a
    /// service account without a profile): use portable mode there.
    pub fn detect() -> Result<Self> {
        let base = directories::BaseDirs::new().ok_or_else(|| {
            Error::invalid(
                "user directories",
                "no home directory is known for this user; use portable mode",
            )
        })?;
        Ok(Self {
            config: base.config_dir().to_path_buf(),
            data: base.data_dir().to_path_buf(),
            cache: base.cache_dir().to_path_buf(),
            state: base
                .state_dir()
                .unwrap_or_else(|| base.data_local_dir())
                .to_path_buf(),
        })
    }
}

/// Whether the app keeps its files next to the executable (portable) or in the per-user
/// directories.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PortableMode {
    /// Portable when [`PORTABLE_MARKER`] exists next to the executable, per-user otherwise.
    #[default]
    Auto,
    /// Always portable (for example the `--portable` command line flag).
    On,
    /// Never portable, even if the marker file exists.
    Off,
}

/// The writable and readable locations of one running app.
///
/// [`PlatformPaths::resolve`] chooses between the per-user OS directories and portable mode;
/// [`PlatformPaths::under`] is the building block for tests and portable installs. Android's app
/// storage is Phase 2.
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

    /// The per-user layout: `<config>/SS2RS` (so `settings.toml` is where
    /// `ss2_core::settings::default_path` expects it), `<data>/SS2RS/saves`, `<cache>/SS2RS` and
    /// `<state>/SS2RS/logs`. Saves and cache never share a folder.
    pub fn from_user_dirs(dirs: &UserDirs) -> Self {
        Self {
            config_dir: dirs.config.join(APP_DIR),
            save_dir: dirs.data.join(APP_DIR).join("saves"),
            cache_dir: dirs.cache.join(APP_DIR),
            log_dir: dirs.state.join(APP_DIR).join("logs"),
            game_data_dir: None,
        }
    }

    /// Portable layout: everything below `<exe_dir>/ss2-data`.
    pub fn portable_in(exe_dir: impl AsRef<Path>) -> Self {
        Self::under(exe_dir.as_ref().join(PORTABLE_DIR))
    }

    /// Choose the layout for this run. `exe_dir` is the folder of the running executable and
    /// `user_dirs` the OS base directories (`UserDirs::detect().ok()` in the app; a fixed value
    /// in tests). Portable mode never needs `user_dirs`.
    ///
    /// # Errors
    /// [`Error::Invalid`] when the per-user layout is needed but `user_dirs` is `None`.
    pub fn resolve(
        exe_dir: &Path,
        mode: PortableMode,
        user_dirs: Option<UserDirs>,
    ) -> Result<Self> {
        let portable = match mode {
            PortableMode::On => true,
            PortableMode::Off => false,
            PortableMode::Auto => exe_dir.join(PORTABLE_MARKER).is_file(),
        };
        if portable {
            return Ok(Self::portable_in(exe_dir));
        }
        let dirs = user_dirs.ok_or_else(|| {
            Error::invalid(
                "user directories",
                "no home directory is known for this user; use portable mode",
            )
        })?;
        Ok(Self::from_user_dirs(&dirs))
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

    fn user_dirs() -> UserDirs {
        UserDirs {
            config: "u/cfg".into(),
            data: "u/data".into(),
            cache: "u/cache".into(),
            state: "u/state".into(),
        }
    }

    #[test]
    fn from_user_dirs_uses_a_folder_named_after_the_app() {
        let p = PlatformPaths::from_user_dirs(&user_dirs());
        assert_eq!(p.config_dir, Path::new("u/cfg").join("SS2RS"));
        assert_eq!(p.save_dir, Path::new("u/data").join("SS2RS").join("saves"));
        assert_eq!(p.cache_dir, Path::new("u/cache").join("SS2RS"));
        assert_eq!(p.log_dir, Path::new("u/state").join("SS2RS").join("logs"));
        assert_eq!(p.game_data_dir, None);
    }

    #[test]
    fn user_settings_file_matches_the_core_default_path() {
        // `ss2_core::settings::default_path` is `<config dir>/SS2RS/settings.toml`; the two must
        // agree so a user's existing settings are found by either.
        let base = std::env::temp_dir();
        let dirs = UserDirs {
            config: base.join("cfg"),
            data: base.join("data"),
            cache: base.join("cache"),
            state: base.join("state"),
        };
        let p = PlatformPaths::from_user_dirs(&dirs);
        assert_eq!(
            p.settings_file(),
            base.join("cfg").join("SS2RS").join("settings.toml")
        );
    }

    #[test]
    fn user_layout_agrees_with_the_core_default_settings_path_on_this_host() {
        // The real OS directories: if the host has a home directory, the two must be identical.
        let (Ok(dirs), Some(core_path)) = (UserDirs::detect(), ss2_core::settings::default_path())
        else {
            return;
        };
        assert_eq!(PlatformPaths::from_user_dirs(&dirs).settings_file(), core_path);
    }

    #[test]
    fn portable_in_keeps_everything_in_one_folder_next_to_the_exe() {
        let p = PlatformPaths::portable_in("app");
        for dir in [&p.config_dir, &p.save_dir, &p.cache_dir, &p.log_dir] {
            assert!(dir.starts_with(Path::new("app").join("ss2-data")), "{dir:?}");
        }
    }

    #[test]
    fn marker_file_next_to_the_exe_selects_portable_mode() {
        let exe_dir = tempfile::tempdir().unwrap();
        std::fs::write(exe_dir.path().join(PORTABLE_MARKER), b"").unwrap();
        let p = PlatformPaths::resolve(exe_dir.path(), PortableMode::Auto, Some(user_dirs()))
            .unwrap();
        assert_eq!(p, PlatformPaths::portable_in(exe_dir.path()));
    }

    #[test]
    fn without_a_marker_auto_mode_uses_the_user_directories() {
        let exe_dir = tempfile::tempdir().unwrap();
        let p = PlatformPaths::resolve(exe_dir.path(), PortableMode::Auto, Some(user_dirs()))
            .unwrap();
        assert_eq!(p, PlatformPaths::from_user_dirs(&user_dirs()));
    }

    #[test]
    fn an_explicit_mode_overrides_the_marker() {
        let exe_dir = tempfile::tempdir().unwrap();
        let on = PlatformPaths::resolve(exe_dir.path(), PortableMode::On, Some(user_dirs()))
            .unwrap();
        assert_eq!(on, PlatformPaths::portable_in(exe_dir.path()), "no marker");

        std::fs::write(exe_dir.path().join(PORTABLE_MARKER), b"").unwrap();
        let off = PlatformPaths::resolve(exe_dir.path(), PortableMode::Off, Some(user_dirs()))
            .unwrap();
        assert_eq!(off, PlatformPaths::from_user_dirs(&user_dirs()), "marker");
    }

    #[test]
    fn missing_user_directories_only_fail_when_they_are_needed() {
        let exe_dir = tempfile::tempdir().unwrap();
        assert!(PlatformPaths::resolve(exe_dir.path(), PortableMode::On, None).is_ok());
        let err = PlatformPaths::resolve(exe_dir.path(), PortableMode::Off, None).unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }), "{err}");
    }

    #[test]
    fn detect_gives_absolute_directories_or_a_clear_error() {
        match UserDirs::detect() {
            Ok(d) => {
                for dir in [&d.config, &d.data, &d.cache, &d.state] {
                    assert!(dir.is_absolute(), "{dir:?}");
                }
            }
            Err(e) => assert!(matches!(e, Error::Invalid { .. }), "{e}"),
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
