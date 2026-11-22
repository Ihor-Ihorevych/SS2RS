//! The panic hook runs in its own process (an integration test) because it is global state.

use std::sync::atomic::{AtomicBool, Ordering};

use ss2_core::crash::{CrashConfig, install_panic_hook};

static PREVIOUS_HOOK_RAN: AtomicBool = AtomicBool::new(false);

#[test]
fn a_panic_writes_a_report_with_the_log_tail_and_still_calls_the_previous_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("ss2.log");
    std::fs::write(&log, "starting\nloading level\nabout to fail\n").unwrap();

    std::panic::set_hook(Box::new(|_| {
        PREVIOUS_HOOK_RAN.store(true, Ordering::SeqCst)
    }));
    let mut cfg = CrashConfig::new(tmp.path().join("crashes"), "9.9.9-test");
    cfg.log_file = Some(log);
    cfg.tail_lines = 2;
    install_panic_hook(cfg);

    let result = std::panic::catch_unwind(|| {
        panic!("deliberate test panic");
    });
    assert!(result.is_err());
    assert!(
        PREVIOUS_HOOK_RAN.load(Ordering::SeqCst),
        "the hook installed before ours must still run"
    );

    let reports: Vec<_> = std::fs::read_dir(tmp.path().join("crashes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(reports.len(), 1, "{reports:?}");
    let text = std::fs::read_to_string(&reports[0]).unwrap();
    for needle in [
        "deliberate test panic",
        "9.9.9-test",
        "crash_hook.rs",
        "loading level",
        "about to fail",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
    assert!(!text.contains("starting"), "only the last 2 log lines");
}
