//! Process-wide `tracing` setup shared by the binaries and by tests.

use tracing_subscriber::EnvFilter;

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
