//! Where `ss2-app` writes its log file, crash reports and settings (wires T03-012, T03-016,
//! T03-019 and T03-020 together).
//!
//! An interactive run resolves its folders once ([`paths_for`]: the per-user folders of the OS,
//! or everything next to the executable with `--portable` or an `ss2-portable` marker file), then
//! writes `ss2-app.log` (rotated, [`ss2_core::logfile`]) and one text file per panic
//! ([`ss2_core::crash`]) to the log folder, and reads and saves `settings.toml` in the config
//! folder. The other modes (`--help`, `--version`, `--smoke-frames`, `--screenshot`) skip writing
//! diagnostics files.

use std::path::{Path, PathBuf};

use ss2_core::Result;
use ss2_core::cli::LaunchOptions;
use ss2_core::crash::{CrashConfig, install_panic_hook};
use ss2_core::logfile::FileLogConfig;
use ss2_core::logging::{init_logging, init_logging_with_file};
use ss2_platform_api::{PlatformPaths, PortableMode, UserDirs};

/// The log file and crash report settings of one run.
pub struct Diagnostics {
    /// Where and how the log file is written.
    pub log: FileLogConfig,
    /// Where crash reports go and which log file their tail comes from.
    pub crash: CrashConfig,
}

/// The diagnostics settings for `paths`: `ss2-app.log` in the log folder and crash reports in a
/// `crashes` folder inside it. A crash report reads the tail of that same log file.
pub fn configure(paths: &PlatformPaths, version: &str) -> Diagnostics {
    let log = FileLogConfig {
        stem: "ss2-app".into(),
        ..FileLogConfig::new(&paths.log_dir)
    };
    let mut crash = CrashConfig::new(paths.log_dir.join("crashes"), version);
    crash.log_file = Some(paths.log_dir.join(format!("{}.log", log.stem)));
    Diagnostics { log, crash }
}

/// Choose the folders for this run: portable with `--portable` (or an `ss2-portable` marker
/// file next to the executable), the per-user OS folders otherwise.
///
/// # Errors
/// [`ss2_core::Error::Invalid`] when the per-user folders are needed but the user has no home
/// directory (`user_dirs` is `None`).
pub fn paths_for(
    options: &LaunchOptions,
    exe_dir: &Path,
    user_dirs: Option<UserDirs>,
) -> Result<PlatformPaths> {
    let mode = if options.portable {
        PortableMode::On
    } else {
        PortableMode::Auto
    };
    PlatformPaths::resolve(exe_dir, mode, user_dirs)
}

/// Set up logging and crash reports for an interactive run and return the folders in use.
/// Problems never stop the app: without usable folders it logs to the terminal only (and says
/// why), and the return value is `None` (the caller then uses the default settings location).
pub fn start(options: &LaunchOptions) -> Option<PlatformPaths> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let paths = match paths_for(options, &exe_dir, UserDirs::detect().ok()) {
        Ok(paths) => paths,
        Err(e) => {
            init_logging("info");
            tracing::warn!("no folders for logs and settings ({e}); logging to the terminal only");
            return None;
        }
    };
    let diagnostics = configure(&paths, env!("CARGO_PKG_VERSION"));
    if let Err(e) = init_logging_with_file("info", diagnostics.log) {
        init_logging("info");
        tracing::warn!("cannot write the log file ({e}); logging to the terminal only");
    }
    install_panic_hook(diagnostics.crash);
    Some(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::cli::LaunchOptions;
    use ss2_platform_api::{PlatformPaths, UserDirs};
    use std::path::Path;

    fn user_dirs() -> UserDirs {
        UserDirs {
            config: "u/cfg".into(),
            data: "u/data".into(),
            cache: "u/cache".into(),
            state: "u/state".into(),
        }
    }

    #[test]
    fn the_log_and_the_crash_reports_go_to_the_log_folder() {
        let paths = PlatformPaths::under("root");
        let d = configure(&paths, "1.2.3");
        assert_eq!(d.log.dir, paths.log_dir);
        assert_eq!(d.log.stem, "ss2-app");
        assert_eq!(d.crash.dir, paths.log_dir.join("crashes"));
        assert_eq!(
            d.crash.log_file,
            Some(paths.log_dir.join("ss2-app.log")),
            "a crash report carries the tail of this run's log"
        );
        assert_eq!(d.crash.app_version, "1.2.3");
    }

    #[test]
    fn the_crash_report_reads_the_file_the_log_actually_writes() {
        let paths = PlatformPaths::under("root");
        let d = configure(&paths, "v");
        let written = d.log.dir.join(format!("{}.log", d.log.stem));
        assert_eq!(d.crash.log_file.as_deref(), Some(written.as_path()));
    }

    #[test]
    fn the_portable_flag_keeps_everything_next_to_the_executable() {
        let options = LaunchOptions {
            portable: true,
            ..LaunchOptions::default()
        };
        let paths = paths_for(&options, Path::new("app"), Some(user_dirs())).unwrap();
        assert_eq!(paths, PlatformPaths::portable_in("app"));
    }

    #[test]
    fn without_the_flag_the_per_user_folders_are_used() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = paths_for(&LaunchOptions::default(), tmp.path(), Some(user_dirs())).unwrap();
        assert_eq!(paths, PlatformPaths::from_user_dirs(&user_dirs()));
    }

    #[test]
    fn a_marker_file_next_to_the_executable_also_selects_portable_mode() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(ss2_platform_api::PORTABLE_MARKER), b"").unwrap();
        let paths = paths_for(&LaunchOptions::default(), tmp.path(), Some(user_dirs())).unwrap();
        assert_eq!(paths, PlatformPaths::portable_in(tmp.path()));
    }

    #[test]
    fn no_home_directory_is_an_error_unless_portable() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(paths_for(&LaunchOptions::default(), tmp.path(), None).is_err());
        let portable = LaunchOptions {
            portable: true,
            ..LaunchOptions::default()
        };
        assert!(paths_for(&portable, tmp.path(), None).is_ok());
    }

    #[test]
    fn the_settings_file_of_the_user_layout_is_where_the_core_default_expects_it() {
        // Switching the app to PlatformPaths must not move existing users' settings.
        let (Ok(dirs), Some(core)) = (UserDirs::detect(), ss2_core::settings::default_path())
        else {
            return;
        };
        assert_eq!(PlatformPaths::from_user_dirs(&dirs).settings_file(), core);
    }
}
