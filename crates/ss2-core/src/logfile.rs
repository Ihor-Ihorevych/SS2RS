//! A size-rotated log file for the binaries (T03-019).
//!
//! [`RotatingFile`] is a plain [`std::io::Write`]: wrap it in a `Mutex` and hand it to
//! `tracing_subscriber::fmt().with_writer(..)` (see
//! [`init_logging_with_file`](crate::logging::init_logging_with_file)). It writes `<stem>.log`;
//! when a write would push the file over the size limit, the current file becomes `<stem>.1.log`
//! (older ones shift up: `.1` -> `.2`, ...) and at most `keep` old files are kept. A file left
//! over from the previous run is rotated when the log is opened, so every run starts a fresh
//! file and the last runs stay available for crash reports.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::PathBuf;

use crate::{Error, Result};

/// Where and how a log file is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileLogConfig {
    /// Folder for the log files (created if missing), usually `PlatformPaths::log_dir`.
    pub dir: PathBuf,
    /// File name stem: the current file is `<stem>.log`, old ones `<stem>.1.log`, ...
    pub stem: String,
    /// Rotate before a write would make the file larger than this many bytes.
    pub max_bytes: u64,
    /// How many rotated files to keep besides the current one (0 keeps none).
    pub keep: usize,
}

impl FileLogConfig {
    /// `ss2.log` in `dir`, rotated at 5 MiB, keeping 5 old files.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            stem: "ss2".into(),
            max_bytes: 5 * 1024 * 1024,
            keep: 5,
        }
    }

    fn current(&self) -> PathBuf {
        self.dir.join(format!("{}.log", self.stem))
    }

    fn old(&self, index: usize) -> PathBuf {
        self.dir.join(format!("{}.{index}.log", self.stem))
    }
}

/// A log file that rotates by size. Not internally synchronized: share it behind a `Mutex`.
#[derive(Debug)]
pub struct RotatingFile {
    cfg: FileLogConfig,
    /// `None` only while rotating (the file must be closed before it is renamed on Windows).
    file: Option<File>,
    size: u64,
}

impl RotatingFile {
    /// Create the folder, rotate a non-empty file left over from the previous run and open a
    /// fresh `<stem>.log`.
    ///
    /// # Errors
    /// [`Error::Io`] naming the folder or file that could not be created, rotated or opened.
    pub fn open(cfg: FileLogConfig) -> Result<Self> {
        fs::create_dir_all(&cfg.dir).map_err(|e| Error::io(&cfg.dir, e))?;
        let current = cfg.current();
        if fs::metadata(&current).is_ok_and(|m| m.len() > 0) {
            rotate_files(&cfg).map_err(|e| Error::io(&current, e))?;
        }
        let file = open_current(&cfg).map_err(|e| Error::io(&current, e))?;
        let size = file.metadata().map_or(0, |m| m.len());
        Ok(Self {
            cfg,
            file: Some(file),
            size,
        })
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file = None; // close before renaming
        rotate_files(&self.cfg)?;
        self.file = Some(open_current(&self.cfg)?);
        self.size = 0;
        Ok(())
    }
}

impl io::Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Never split one write (one log record) across two files; a record larger than the
        // limit goes whole into a fresh file.
        let would_exceed = self.size > 0 && self.size + buf.len() as u64 > self.cfg.max_bytes;
        if would_exceed {
            self.rotate()?;
        }
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("log file is closed"))?;
        let n = file.write(buf)?;
        self.size += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

fn open_current(cfg: &FileLogConfig) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(cfg.current())
}

/// Shift `<stem>.N.log` up by one (dropping the oldest beyond `keep`) and move the current file
/// to `<stem>.1.log`; with `keep == 0` the current file is simply removed.
fn rotate_files(cfg: &FileLogConfig) -> io::Result<()> {
    let current = cfg.current();
    if cfg.keep == 0 {
        return remove_if_exists(&current);
    }
    remove_if_exists(&cfg.old(cfg.keep))?;
    for index in (1..cfg.keep).rev() {
        let from = cfg.old(index);
        if from.exists() {
            fs::rename(&from, cfg.old(index + 1))?;
        }
    }
    if current.exists() {
        fs::rename(&current, cfg.old(1))?;
    }
    Ok(())
}

fn remove_if_exists(path: &std::path::Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;

    fn cfg(dir: &Path, max_bytes: u64, keep: usize) -> FileLogConfig {
        FileLogConfig {
            dir: dir.to_path_buf(),
            stem: "ss2".into(),
            max_bytes,
            keep,
        }
    }

    fn read(dir: &Path, name: &str) -> String {
        std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    #[test]
    fn creates_the_directory_and_writes_the_current_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested").join("logs");
        let mut log = RotatingFile::open(cfg(&dir, 1000, 3)).unwrap();
        log.write_all(b"hello\n").unwrap();
        assert_eq!(read(&dir, "ss2.log"), "hello\n");
    }

    #[test]
    fn a_previous_session_is_rotated_on_open() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("ss2.log"), "old session\n").unwrap();
        let mut log = RotatingFile::open(cfg(tmp.path(), 1000, 3)).unwrap();
        log.write_all(b"new\n").unwrap();
        assert_eq!(read(tmp.path(), "ss2.1.log"), "old session\n");
        assert_eq!(read(tmp.path(), "ss2.log"), "new\n");
    }

    #[test]
    fn an_empty_previous_file_is_not_rotated() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("ss2.log"), "").unwrap();
        let _log = RotatingFile::open(cfg(tmp.path(), 1000, 3)).unwrap();
        assert!(!tmp.path().join("ss2.1.log").exists());
    }

    #[test]
    fn rotates_when_the_size_limit_would_be_exceeded() {
        let tmp = tempfile::tempdir().unwrap();
        let mut log = RotatingFile::open(cfg(tmp.path(), 10, 3)).unwrap();
        log.write_all(b"aaaaaa\n").unwrap();
        log.write_all(b"bbbbbb\n").unwrap();
        assert_eq!(read(tmp.path(), "ss2.1.log"), "aaaaaa\n");
        assert_eq!(read(tmp.path(), "ss2.log"), "bbbbbb\n");
    }

    #[test]
    fn a_write_larger_than_the_limit_goes_whole_into_a_fresh_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut log = RotatingFile::open(cfg(tmp.path(), 4, 3)).unwrap();
        log.write_all(b"ab").unwrap();
        log.write_all(b"a line much longer than four bytes\n")
            .unwrap();
        assert_eq!(read(tmp.path(), "ss2.1.log"), "ab");
        assert_eq!(
            read(tmp.path(), "ss2.log"),
            "a line much longer than four bytes\n"
        );
    }

    #[test]
    fn old_files_shift_up_and_only_keep_of_them_survive() {
        let tmp = tempfile::tempdir().unwrap();
        let mut log = RotatingFile::open(cfg(tmp.path(), 3, 2)).unwrap();
        for line in ["1\n", "2\n", "3\n", "4\n", "5\n"] {
            log.write_all(line.as_bytes()).unwrap();
            log.write_all(b"x").unwrap(); // pushes the next line into a new file
        }
        assert_eq!(read(tmp.path(), "ss2.log"), "5\nx");
        assert_eq!(read(tmp.path(), "ss2.1.log"), "4\nx");
        assert_eq!(read(tmp.path(), "ss2.2.log"), "3\nx");
        assert!(!tmp.path().join("ss2.3.log").exists(), "only 2 old files");
    }

    #[test]
    fn keep_zero_discards_the_old_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let mut log = RotatingFile::open(cfg(tmp.path(), 4, 0)).unwrap();
        log.write_all(b"aaa\n").unwrap();
        log.write_all(b"bbb\n").unwrap();
        assert_eq!(read(tmp.path(), "ss2.log"), "bbb\n");
        assert!(!tmp.path().join("ss2.1.log").exists());
    }

    #[test]
    fn an_unusable_directory_is_an_error_naming_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("file");
        std::fs::write(&blocker, b"x").unwrap();
        let err = RotatingFile::open(cfg(&blocker.join("logs"), 100, 1))
            .map(|_| ())
            .unwrap_err();
        assert!(err.to_string().contains("logs"), "{err}");
    }

    #[test]
    fn works_as_a_tracing_writer_without_colour_codes() {
        let tmp = tempfile::tempdir().unwrap();
        let writer = std::sync::Mutex::new(RotatingFile::open(cfg(tmp.path(), 10_000, 1)).unwrap());
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!("disk almost full");
        });
        let text = read(tmp.path(), "ss2.log");
        assert!(text.contains("disk almost full"), "{text}");
        assert!(text.contains("WARN"), "{text}");
        assert!(!text.contains('\u{1b}'), "no ANSI escapes in a file");
    }

    #[test]
    fn default_config_has_sensible_limits() {
        let c = FileLogConfig::new("logs");
        assert_eq!(c.stem, "ss2");
        assert!(c.max_bytes >= 1024 * 1024);
        assert!(c.keep >= 1);
    }
}
