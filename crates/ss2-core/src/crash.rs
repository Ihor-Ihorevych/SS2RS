//! Crash reports for panics (T03-016).
//!
//! [`install_panic_hook`] makes every panic write a plain-text report (the panic message and
//! location, the thread, the backtrace, the version and OS, and the tail of the log file) to a
//! new `crash-<unix time>.txt`, then runs the hook that was installed before, so the usual
//! message on stderr is kept. The report is ordinary text on purpose: the UI can show it and
//! offer "copy to clipboard" ([`render_report`]), and users can attach it to a bug report
//! (docs/TRIAGE.md). It is not a minidump.
//!
//! The hook is written to be robust: it never unwraps, a failure to write the report is only
//! printed, and the log tail is read from the end of the file with a fixed memory bound.

use std::any::Any;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// How much of the end of the log file is read for the tail.
const LOG_TAIL_WINDOW: u64 = 64 * 1024;

/// Where crash reports go and what goes into them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrashConfig {
    /// Folder for the reports (created when the first one is written).
    pub dir: PathBuf,
    /// The current log file to take the tail from (for example
    /// [`FileLogConfig`](crate::logfile::FileLogConfig)'s `<stem>.log`), if there is one.
    pub log_file: Option<PathBuf>,
    /// How many of the last log lines go into the report.
    pub tail_lines: usize,
    /// The application version string shown in the report.
    pub app_version: String,
}

impl CrashConfig {
    /// Reports in `dir`, no log file, the last 50 log lines.
    pub fn new(dir: impl Into<PathBuf>, app_version: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            log_file: None,
            tail_lines: 50,
            app_version: app_version.into(),
        }
    }
}

/// What the panic hook knows about a panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanicDetails {
    /// The panic message.
    pub message: String,
    /// `file:line:column` of the panic, if the runtime knows it.
    pub location: Option<String>,
    /// The name of the panicking thread (`<unnamed>` if it has none).
    pub thread: String,
}

/// The message of a panic payload: `&str` and `String` payloads (what `panic!` produces) are
/// returned as text, anything else is described.
pub fn payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "(non-string panic payload)".to_owned()
    }
}

/// The last `max_lines` lines of the file at `path`, reading at most the last 64 KiB. A missing
/// or unreadable file gives a short placeholder instead of an error, and invalid UTF-8 is
/// replaced, because this runs while the program is already crashing.
pub fn read_log_tail(path: &Path, max_lines: usize) -> String {
    match read_tail_window(path) {
        Ok((text, truncated)) => last_lines(&text, max_lines, truncated),
        Err(e) => format!("(no log available at {}: {e})\n", path.display()),
    }
}

fn read_tail_window(path: &Path) -> std::io::Result<(String, bool)> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let truncated = len > LOG_TAIL_WINDOW;
    if truncated {
        file.seek(SeekFrom::Start(len - LOG_TAIL_WINDOW))?;
    }
    let mut bytes = Vec::new();
    file.take(LOG_TAIL_WINDOW).read_to_end(&mut bytes)?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

fn last_lines(text: &str, max_lines: usize, truncated: bool) -> String {
    // When the window starts mid-file its first line is probably cut: drop it.
    let text = if truncated {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        text
    };
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].concat()
}

/// The text of a crash report.
pub fn render_report(
    cfg: &CrashConfig,
    panic: &PanicDetails,
    backtrace: &str,
    log_tail: &str,
) -> String {
    format!(
        "SS2RS crash report\n\
         ==================\n\
         Version:  {version}\n\
         System:   {os} {arch}\n\
         Thread:   {thread}\n\
         Panic:    {message}\n\
         Location: {location}\n\
         \n\
         Backtrace:\n{backtrace}\n\
         \n\
         Last {lines} log lines:\n{log_tail}",
        version = cfg.app_version,
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        thread = panic.thread,
        message = panic.message,
        location = panic.location.as_deref().unwrap_or("unknown"),
        lines = cfg.tail_lines,
    )
}

/// Write `text` to a new `crash-<unix time>.txt` in `dir` (created if missing) and return its
/// path. An existing report is never overwritten: a counter is added to the name instead.
///
/// # Errors
/// [`Error::Io`] naming the folder or file that could not be created.
pub fn write_report(dir: &Path, text: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut attempt = 0_u32;
    loop {
        let name = if attempt == 0 {
            format!("crash-{secs}.txt")
        } else {
            format!("crash-{secs}-{attempt}.txt")
        };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())
                    .map_err(|e| Error::io(&path, e))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempt < 1000 => {
                attempt += 1;
            }
            Err(e) => return Err(Error::io(&path, e)),
        }
    }
}

/// Make every panic write a crash report as described in the module documentation, then run the
/// previously installed panic hook. Call once, early in `main`.
pub fn install_panic_hook(cfg: CrashConfig) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let details = PanicDetails {
            message: payload_message(info.payload()),
            location: info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())),
            thread: std::thread::current()
                .name()
                .unwrap_or("<unnamed>")
                .to_owned(),
        };
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let tail = cfg
            .log_file
            .as_deref()
            .map(|path| read_log_tail(path, cfg.tail_lines))
            .unwrap_or_default();
        let text = render_report(&cfg, &details, &backtrace, &tail);
        match write_report(&cfg.dir, &text) {
            Ok(path) => eprintln!("crash report written to {}", path.display()),
            Err(e) => eprintln!("could not write the crash report: {e}"),
        }
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn the_tail_is_the_last_lines_of_the_log() {
        let tmp = tempfile::tempdir().unwrap();
        let log = write(tmp.path(), "ss2.log", b"one\ntwo\nthree\nfour\nfive\n");
        assert_eq!(read_log_tail(&log, 2), "four\nfive\n");
    }

    #[test]
    fn a_short_log_is_returned_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let log = write(tmp.path(), "ss2.log", b"one\ntwo\n");
        assert_eq!(read_log_tail(&log, 50), "one\ntwo\n");
    }

    #[test]
    fn a_missing_log_gives_a_placeholder_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let text = read_log_tail(&tmp.path().join("nope.log"), 10);
        assert!(text.contains("no log"), "{text}");
    }

    #[test]
    fn only_the_end_of_a_huge_log_is_read() {
        let tmp = tempfile::tempdir().unwrap();
        let mut big = String::new();
        for i in 0..40_000 {
            big.push_str(&format!("line {i}\n"));
        }
        assert!(big.len() > 300_000);
        let log = write(tmp.path(), "ss2.log", big.as_bytes());
        assert_eq!(
            read_log_tail(&log, 3),
            "line 39997\nline 39998\nline 39999\n"
        );
    }

    #[test]
    fn invalid_utf8_in_the_log_does_not_break_the_tail() {
        let tmp = tempfile::tempdir().unwrap();
        let log = write(tmp.path(), "ss2.log", b"ok\n\xff\xfe broken\nlast\n");
        let text = read_log_tail(&log, 3);
        assert!(text.contains("last"), "{text}");
    }

    #[test]
    fn the_report_names_everything_needed_to_diagnose_a_crash() {
        let mut cfg = CrashConfig::new("crashes", "0.1.0-test");
        cfg.tail_lines = 5;
        let panic = PanicDetails {
            message: "index out of bounds".into(),
            location: Some("src/world.rs:42:9".into()),
            thread: "sim".into(),
        };
        let text = render_report(
            &cfg,
            &panic,
            "  0: frame_a\n  1: frame_b",
            "last log line\n",
        );
        for needle in [
            "SS2RS crash report",
            "0.1.0-test",
            std::env::consts::OS,
            std::env::consts::ARCH,
            "sim",
            "index out of bounds",
            "src/world.rs:42:9",
            "frame_a",
            "last log line",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
        }
    }

    #[test]
    fn a_panic_without_a_location_is_still_reported() {
        let cfg = CrashConfig::new("c", "v");
        let panic = PanicDetails {
            message: "boom".into(),
            location: None,
            thread: "main".into(),
        };
        let text = render_report(&cfg, &panic, "", "");
        assert!(text.contains("boom") && text.contains("unknown"), "{text}");
    }

    #[test]
    fn reports_are_written_to_new_files_and_never_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested").join("crashes");
        let a = write_report(&dir, "first").unwrap();
        let b = write_report(&dir, "second").unwrap();
        assert_ne!(a, b);
        for path in [&a, &b] {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            assert!(
                name.starts_with("crash-") && name.ends_with(".txt"),
                "{name}"
            );
        }
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "first");
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "second");
    }

    #[test]
    fn an_unusable_report_folder_is_an_error_naming_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = write(tmp.path(), "file", b"x");
        let err = write_report(&blocker.join("crashes"), "x").unwrap_err();
        assert!(err.to_string().contains("crashes"), "{err}");
    }

    #[test]
    fn payload_messages_cover_str_string_and_other_payloads() {
        let a: Box<dyn std::any::Any + Send> = Box::new("static message");
        let b: Box<dyn std::any::Any + Send> = Box::new(String::from("owned message"));
        let c: Box<dyn std::any::Any + Send> = Box::new(42_u32);
        assert_eq!(payload_message(a.as_ref()), "static message");
        assert_eq!(payload_message(b.as_ref()), "owned message");
        assert!(payload_message(c.as_ref()).contains("non-string"));
    }
}
