//! Process-wide `tracing` setup shared by the binaries and by tests.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use crate::logfile::{FileLogConfig, RotatingFile};

/// Environment variable that overrides the default log filter, e.g. `SS2_LOG=ss2_vfs=debug`.
pub const LOG_ENV: &str = "SS2_LOG";

/// Install the global subscriber. Returns `true` only for the call that installed it and
/// `false` if one already existed; it never panics, so it is safe to call from any thread.
pub fn init_logging(default_filter: &str) -> bool {
    let filter =
        EnvFilter::try_from_env(LOG_ENV).unwrap_or_else(|_| EnvFilter::new(default_filter));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .is_ok()
}

/// Like [`init_logging`], but every record also goes to a size-rotated log file
/// ([`FileLogConfig`]): stdout and `<dir>/<stem>.log`, without colour codes. Returns whether this
/// call installed the subscriber (`false` if one already existed; the file is still opened and
/// a previous run's file rotated).
///
/// # Errors
/// [`Error::Io`](crate::Error::Io) when the log folder or file cannot be created or rotated:
/// callers should fall back to [`init_logging`] and carry on.
pub fn init_logging_with_file(default_filter: &str, cfg: FileLogConfig) -> crate::Result<bool> {
    let file = std::sync::Mutex::new(RotatingFile::open(cfg)?);
    let filter =
        EnvFilter::try_from_env(LOG_ENV).unwrap_or_else(|_| EnvFilter::new(default_filter));
    Ok(tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(std::io::stdout.and(file))
        .try_init()
        .is_ok())
}

/// Logging for tests: `warn` by default, output captured by the test harness.
pub fn init_for_tests() -> bool {
    let filter = EnvFilter::try_from_env(LOG_ENV).unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_init_returns_false() {
        let _first = init_logging("warn");
        assert!(!init_logging("warn"));
    }

    #[test]
    fn init_for_tests_is_safe_to_call_repeatedly() {
        let _first = init_for_tests();
        assert!(
            !init_for_tests(),
            "a subscriber must already exist after the first call"
        );
    }

    #[test]
    fn init_with_file_opens_the_log_file_even_if_a_subscriber_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let _ = init_logging("warn");
        let installed = init_logging_with_file("warn", FileLogConfig::new(tmp.path())).unwrap();
        assert!(!installed, "a subscriber was installed above");
        assert!(tmp.path().join("ss2.log").is_file());
    }

    #[test]
    fn init_with_file_reports_an_unusable_log_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("file");
        std::fs::write(&blocker, b"x").unwrap();
        let cfg = FileLogConfig::new(blocker.join("logs"));
        assert!(init_logging_with_file("warn", cfg).is_err());
    }

    #[test]
    fn concurrent_init_installs_exactly_once() {
        let handles: Vec<_> = (0..8)
            .map(|_| std::thread::spawn(|| init_logging("warn")))
            .collect();
        let installed = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|&b| b)
            .count();
        assert!(installed <= 1, "{installed} threads installed a subscriber");
    }
}
