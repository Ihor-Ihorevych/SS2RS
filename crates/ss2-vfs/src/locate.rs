//! Finding the player's install and remembering the choice (roadmap T10-019, `docs/FIRST_RUN.md`).
//!
//! Three layers, none of which needs a window or a device:
//!
//! 1. **Discovery** ([`discover`]): the folders where a Serious Sam 2 install may be, from an
//!    injected [`Environment`] (variables, folders, the text of Steam's `libraryfolders.vdf`, a
//!    registry value), in order of how much the player meant them, without duplicates, each
//!    checked cheaply (the folder lists `All_PC_01.gro`).
//! 2. **Validation** ([`summarize`]): the report of [`check_install`] (the check behind
//!    `ss2-doctor`) turned into a verdict and a few lines a wizard can show.
//! 3. **Memory** ([`save_choice`], [`load_choice`], [`resolve_startup`]): the chosen folder and the
//!    time it was validated in one small TOML file, written atomically, read without ever failing,
//!    and checked again at every start.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ss2_core::{Error, Result};

use crate::{InstallReport, PatchLevel, Severity, check_install};

/// The archive that must be directly inside an install folder.
pub const ANCHOR_ARCHIVE: &str = "All_PC_01.gro";
/// The environment variable that points the project at the data (tools, tests and the app).
pub const DATA_DIR_ENV: &str = "SS2_DATA_DIR";
/// The name of the file that remembers the choice, inside the config folder.
pub const CHOICE_FILE_NAME: &str = "data_location.toml";
/// The newest patch the project has seen an install at (`Patch_02_100_800000.gro`): an install
/// at an older level is usable but is not what the project was checked against.
pub const NEWEST_KNOWN_PATCH: (u32, u32, u32) = (2, 100, 800_000);

const SCHEMA_VERSION: u32 = 1;
/// The longest path (in characters) that is stored; a path that long is not a real install.
const MAX_STORED_PATH_CHARS: usize = 4096;
/// The folder names a store may give the game.
const GAME_FOLDER_NAMES: [&str; 2] = ["serious sam 2", "serioussam2"];

// ---------------------------------------------------------------------------------------------------------------------------------
// The environment

/// The operating system family, for the places an install usually is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// Windows.
    Windows,
    /// Linux and the other Unixes.
    Linux,
    /// macOS.
    MacOs,
}

impl Os {
    /// The system this build runs on.
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

/// Everything discovery asks of the machine, so that a test can answer instead.
pub trait Environment {
    /// The system family.
    fn os(&self) -> Os;
    /// An environment variable (`None` when unset or not text).
    fn var(&self, name: &str) -> Option<String>;
    /// The current folder.
    fn current_dir(&self) -> Option<PathBuf>;
    /// The folder of the running executable.
    fn exe_dir(&self) -> Option<PathBuf>;
    /// The user's home folder.
    fn home_dir(&self) -> Option<PathBuf>;
    /// The names of the entries (files and folders) of `dir`; `None` when it does not exist or cannot be read.
    fn list_dir(&self, dir: &Path) -> Option<Vec<String>>;
    /// The text of a small file; `None` when it cannot be read.
    fn read_text(&self, file: &Path) -> Option<String>;
    /// A string value of the Windows registry (`key` such as `HKCU\Software\Valve\Steam`); `None` elsewhere and when it is not there.
    fn registry_string(&self, key: &str, value: &str) -> Option<String>;
}

/// The real machine.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEnvironment;

/// The biggest file [`SystemEnvironment`] reads as text (Steam's folder list is a few hundred bytes).
const MAX_TEXT_BYTES: u64 = 1 << 20;
/// The most entries of one folder it reports.
const MAX_LISTED: usize = 100_000;

impl Environment for SystemEnvironment {
    fn os(&self) -> Os {
        Os::current()
    }

    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn current_dir(&self) -> Option<PathBuf> {
        std::env::current_dir().ok()
    }

    fn exe_dir(&self) -> Option<PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
    }

    fn home_dir(&self) -> Option<PathBuf> {
        let name = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }

    fn list_dir(&self, dir: &Path) -> Option<Vec<String>> {
        let entries = std::fs::read_dir(dir).ok()?;
        Some(
            entries
                .flatten()
                .take(MAX_LISTED)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect(),
        )
    }

    fn read_text(&self, file: &Path) -> Option<String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(file)
            .ok()?
            .take(MAX_TEXT_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        (bytes.len() as u64 <= MAX_TEXT_BYTES).then(|| String::from_utf8_lossy(&bytes).into_owned())
    }

    #[cfg(windows)]
    fn registry_string(&self, key: &str, value: &str) -> Option<String> {
        use std::os::windows::process::CommandExt;
        // `reg query` is part of every Windows; no window is opened for it (CREATE_NO_WINDOW).
        let output = std::process::Command::new("reg")
            .args(["query", key, "/v", value])
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| parse_reg_query(&String::from_utf8_lossy(&output.stdout), value))
            .flatten()
    }

    #[cfg(not(windows))]
    fn registry_string(&self, _key: &str, _value: &str) -> Option<String> {
        None
    }
}

/// The value of `value` in the output of `reg query <key> /v <value>`
/// (`    SteamPath    REG_SZ    c:/program files (x86)/steam`).
pub fn parse_reg_query(text: &str, value: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let Some((name, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !name.eq_ignore_ascii_case(value) {
            continue;
        }
        let rest = rest.trim_start();
        let (kind, data) = rest.split_once(char::is_whitespace)?;
        if kind != "REG_SZ" && kind != "REG_EXPAND_SZ" {
            return None;
        }
        // `line` was trimmed and split after the type, so `data` is not empty.
        return Some(data.trim().to_owned());
    }
    None
}

/// An [`Environment`] that answers from tables you fill in, for tests of this crate and of the code that uses discovery.
#[derive(Clone, Debug)]
pub struct FakeEnvironment {
    /// The system family answered.
    pub os: Os,
    /// The current folder.
    pub current_dir: Option<PathBuf>,
    /// The executable's folder.
    pub exe_dir: Option<PathBuf>,
    /// The home folder.
    pub home_dir: Option<PathBuf>,
    vars: Vec<(String, String)>,
    dirs: Vec<(String, Vec<String>)>,
    files: Vec<(String, String)>,
    registry: Vec<(String, String, String)>,
}

impl FakeEnvironment {
    /// An empty machine of the given family: no variables, no folders.
    pub fn new(os: Os) -> Self {
        Self {
            os,
            current_dir: None,
            exe_dir: None,
            home_dir: None,
            vars: Vec::new(),
            dirs: Vec::new(),
            files: Vec::new(),
            registry: Vec::new(),
        }
    }

    /// Set an environment variable.
    #[must_use]
    pub fn with_var(mut self, name: &str, value: &str) -> Self {
        self.vars.push((name.to_owned(), value.to_owned()));
        self
    }

    /// A folder that exists and holds these entries. Paths are compared the way discovery compares them (case and separators do not matter).
    #[must_use]
    pub fn with_dir(mut self, dir: impl AsRef<Path>, entries: &[&str]) -> Self {
        self.dirs.push((
            dedupe_key(dir.as_ref()),
            entries.iter().map(|s| (*s).to_owned()).collect(),
        ));
        self
    }

    /// A file with this text.
    #[must_use]
    pub fn with_file(mut self, file: impl AsRef<Path>, text: &str) -> Self {
        self.files
            .push((dedupe_key(file.as_ref()), text.to_owned()));
        self
    }

    /// A registry string.
    #[must_use]
    pub fn with_registry(mut self, key: &str, value: &str, data: &str) -> Self {
        self.registry
            .push((key.to_owned(), value.to_owned(), data.to_owned()));
        self
    }
}

impl Environment for FakeEnvironment {
    fn os(&self) -> Os {
        self.os
    }

    fn var(&self, name: &str) -> Option<String> {
        self.vars
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
    }

    fn current_dir(&self) -> Option<PathBuf> {
        self.current_dir.clone()
    }

    fn exe_dir(&self) -> Option<PathBuf> {
        self.exe_dir.clone()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.home_dir.clone()
    }

    fn list_dir(&self, dir: &Path) -> Option<Vec<String>> {
        let key = dedupe_key(dir);
        self.dirs
            .iter()
            .find(|(d, _)| *d == key)
            .map(|(_, e)| e.clone())
    }

    fn read_text(&self, file: &Path) -> Option<String> {
        let key = dedupe_key(file);
        self.files
            .iter()
            .find(|(f, _)| *f == key)
            .map(|(_, t)| t.clone())
    }

    fn registry_string(&self, key: &str, value: &str) -> Option<String> {
        self.registry
            .iter()
            .find(|(k, v, _)| k == key && v == value)
            .map(|(_, _, d)| d.clone())
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Steam's list of library folders

/// The library folders in the text of Steam's `libraryfolders.vdf`, in file order.
///
/// Both layouts are read: the current one (`"0" { "path" "D:\\SteamLibrary" ... }`) and the old
/// one (`"1" "D:\\SteamLibrary"`). Anything else, damaged text included, gives what could be read
/// and never panics.
pub fn parse_library_folders(text: &str) -> Vec<PathBuf> {
    enum Token {
        Str(String),
        Open,
        Close,
    }
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match chars.next() {
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => {
                                s.push('\\');
                                s.push(other);
                            }
                            None => break,
                        },
                        _ => s.push(c),
                    }
                }
                if !closed {
                    break;
                }
                tokens.push(Token::Str(s));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Open => depth += 1,
            Token::Close => depth = depth.saturating_sub(1),
            Token::Str(key) => {
                if let Some(Token::Str(value)) = tokens.get(i + 1) {
                    let is_path = key.eq_ignore_ascii_case("path");
                    let old_style =
                        depth == 1 && !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit());
                    if (is_path || old_style) && !value.trim().is_empty() {
                        out.push(PathBuf::from(value));
                    }
                    i += 1;
                }
            }
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Discovery

/// Where a candidate folder came from. The order is the order of preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateSource {
    /// The folder remembered from an earlier run.
    Remembered,
    /// The `SS2_DATA_DIR` variable.
    Environment,
    /// The folder the program was started from.
    CurrentDir,
    /// The folder of the program itself.
    ExeDir,
    /// A Steam library.
    Steam,
    /// The usual places of a GOG install.
    Gog,
    /// The usual places of a disc install.
    Retail,
}

impl CandidateSource {
    /// A short name for a list.
    pub fn label(self) -> &'static str {
        match self {
            Self::Remembered => "remembered choice",
            Self::Environment => "SS2_DATA_DIR",
            Self::CurrentDir => "current folder",
            Self::ExeDir => "program folder",
            Self::Steam => "Steam library",
            Self::Gog => "usual GOG place",
            Self::Retail => "usual install place",
        }
    }
}

/// A folder that looks like an install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The folder, spelled as the first source that named it spelled it.
    pub dir: PathBuf,
    /// Every source that named it, most preferred first.
    pub sources: Vec<CandidateSource>,
    /// How many `.gro` archives the folder has.
    pub archives: usize,
    /// The newest patch named by an archive (from the file names only).
    pub patch: Option<PatchLevel>,
}

/// A folder the player or the machine named on purpose that does not look like an install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejected {
    /// The folder.
    pub dir: PathBuf,
    /// Who named it.
    pub source: CandidateSource,
    /// What is wrong, in a sentence.
    pub reason: String,
}

/// What [`discover`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    /// The folders that look like an install, most preferred first, each once.
    pub candidates: Vec<Candidate>,
    /// Folders named by the remembered choice or by `SS2_DATA_DIR` that do not look like one
    /// (a guess that does not exist is not worth a mention and is not listed).
    pub rejected: Vec<Rejected>,
}

/// A text key under which two spellings of the same folder are equal: separators and case do not
/// matter, trailing separators, `.` segments and the `\\?\` prefix are dropped. (Links and 8.3
/// short names are not resolved.)
pub fn dedupe_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    let lead = if text.starts_with("//") {
        "//"
    } else if text.starts_with('/') {
        "/"
    } else {
        ""
    };
    let body: Vec<String> = text
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .map(str::to_lowercase)
        .collect();
    format!("{lead}{}", body.join("/"))
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

fn steam_roots(env: &dyn Environment) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    match env.os() {
        Os::Windows => {
            for (key, value) in [
                (r"HKCU\Software\Valve\Steam", "SteamPath"),
                (r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
                (r"HKLM\SOFTWARE\Valve\Steam", "InstallPath"),
            ] {
                if let Some(path) = nonempty(env.registry_string(key, value)) {
                    roots.push(PathBuf::from(path));
                }
            }
            for var in ["ProgramFiles(x86)", "ProgramFiles"] {
                if let Some(base) = nonempty(env.var(var)) {
                    roots.push(PathBuf::from(base).join("Steam"));
                }
            }
            roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        }
        Os::Linux => {
            if let Some(home) = env.home_dir() {
                roots.push(home.join(".steam").join("steam"));
                roots.push(home.join(".local").join("share").join("Steam"));
                roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
                roots.push(home.join(".wine/drive_c/Program Files (x86)/Steam"));
            }
        }
        Os::MacOs => {
            if let Some(home) = env.home_dir() {
                roots.push(home.join("Library/Application Support/Steam"));
            }
        }
    }
    roots
}

fn steam_candidates(env: &dyn Environment, out: &mut Vec<(CandidateSource, PathBuf)>) {
    let mut libraries: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut add = |libraries: &mut Vec<PathBuf>, dir: PathBuf| {
        let key = dedupe_key(&dir);
        if !seen.contains(&key) {
            seen.push(key);
            libraries.push(dir);
        }
    };
    for root in steam_roots(env) {
        add(&mut libraries, root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Some(text) = env.read_text(&vdf) {
            for library in parse_library_folders(&text) {
                add(&mut libraries, library);
            }
        }
    }
    for library in libraries {
        let common = library.join("steamapps").join("common");
        let Some(entries) = env.list_dir(&common) else {
            continue;
        };
        for entry in entries {
            if GAME_FOLDER_NAMES.contains(&entry.to_lowercase().as_str()) {
                out.push((CandidateSource::Steam, common.join(entry)));
            }
        }
    }
}

fn store_candidates(env: &dyn Environment, out: &mut Vec<(CandidateSource, PathBuf)>) {
    match env.os() {
        Os::Windows => {
            let mut program_files: Vec<PathBuf> = ["ProgramFiles(x86)", "ProgramFiles"]
                .iter()
                .filter_map(|v| nonempty(env.var(v)))
                .map(PathBuf::from)
                .collect();
            program_files.push(PathBuf::from(r"C:\Program Files (x86)"));
            program_files.push(PathBuf::from(r"C:\Program Files"));
            for base in &program_files {
                out.push((
                    CandidateSource::Gog,
                    base.join("GOG Galaxy").join("Games").join("Serious Sam 2"),
                ));
                out.push((
                    CandidateSource::Gog,
                    base.join("GOG.com").join("Serious Sam 2"),
                ));
            }
            for base in &program_files {
                for sub in [
                    "Serious Sam 2",
                    "Croteam/Serious Sam 2",
                    "Devolver Digital/Serious Sam 2",
                ] {
                    out.push((CandidateSource::Retail, base.join(sub)));
                }
            }
            // The forward slash (not `\`) matters here: on a real Windows build `Path` accepts either as a
            // separator, but on a non-Windows host (this file's own tests simulate Windows candidates on every
            // host) a bare `C:\` is one opaque component ending in `\`, and the next `.join` then adds its own
            // separator right after it, producing a doubled separator (`C://...`) once displayed. `C:/` has no
            // such trailing character for `.join` to double up against.
            for drive in ["C:/", "D:/"] {
                let drive = PathBuf::from(drive);
                out.push((
                    CandidateSource::Gog,
                    drive.join("GOG Games").join("Serious Sam 2"),
                ));
                out.push((
                    CandidateSource::Retail,
                    drive.join("Games").join("Serious Sam 2"),
                ));
                out.push((CandidateSource::Retail, drive.join("Serious Sam 2")));
            }
        }
        Os::Linux | Os::MacOs => {
            if let Some(home) = env.home_dir() {
                out.push((
                    CandidateSource::Gog,
                    home.join("GOG Games").join("Serious Sam 2"),
                ));
                out.push((
                    CandidateSource::Gog,
                    home.join(".wine/drive_c/Program Files (x86)/GOG Galaxy/Games/Serious Sam 2"),
                ));
                out.push((
                    CandidateSource::Retail,
                    home.join("Games").join("Serious Sam 2"),
                ));
                out.push((
                    CandidateSource::Retail,
                    home.join(".wine/drive_c/Program Files (x86)/Serious Sam 2"),
                ));
            }
        }
    }
}

/// Every folder worth looking at, most preferred first, duplicates not yet removed.
fn raw_candidates(
    env: &dyn Environment,
    remembered: Option<&Path>,
) -> Vec<(CandidateSource, PathBuf)> {
    let mut out = Vec::new();
    if let Some(dir) = remembered {
        out.push((CandidateSource::Remembered, dir.to_path_buf()));
    }
    if let Some(value) = nonempty(env.var(DATA_DIR_ENV)) {
        out.push((CandidateSource::Environment, PathBuf::from(value)));
    }
    if let Some(dir) = env.current_dir() {
        out.push((CandidateSource::CurrentDir, dir));
    }
    if let Some(dir) = env.exe_dir() {
        out.push((CandidateSource::ExeDir, dir));
    }
    steam_candidates(env, &mut out);
    store_candidates(env, &mut out);
    out
}

/// The cheap look at one folder: it lists the anchor archive.
fn cheap_check(
    env: &dyn Environment,
    dir: &Path,
) -> std::result::Result<(usize, Option<PatchLevel>), String> {
    let Some(names) = env.list_dir(dir) else {
        return Err(format!(
            "{} does not exist or cannot be read",
            dir.display()
        ));
    };
    let gro = |n: &&String| {
        Path::new(n.as_str())
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("gro"))
    };
    let archives: Vec<&String> = names.iter().filter(gro).collect();
    if !archives
        .iter()
        .any(|n| n.eq_ignore_ascii_case(ANCHOR_ARCHIVE))
    {
        return Err(format!(
            "{} does not contain {ANCHOR_ARCHIVE}",
            dir.display()
        ));
    }
    let patch = archives
        .iter()
        .filter_map(|n| PatchLevel::from_file_name(n))
        .max_by_key(PatchLevel::key);
    Ok((archives.len(), patch))
}

/// Find the folders where an install may be. `remembered` is the folder of an earlier choice.
///
/// The order is: the remembered folder, `SS2_DATA_DIR`, the current folder, the program's folder,
/// the Steam libraries, the usual GOG places, the usual install places. A folder named twice (a
/// different spelling, case or a trailing separator included) appears once, with every source that
/// named it. Only a folder that lists `All_PC_01.gro` is a candidate.
pub fn discover(env: &dyn Environment, remembered: Option<&Path>) -> Discovery {
    let mut found = Discovery::default();
    let mut seen: Vec<String> = Vec::new();
    for (source, dir) in raw_candidates(env, remembered) {
        let key = dedupe_key(&dir);
        if let Some(i) = found
            .candidates
            .iter()
            .position(|c| dedupe_key(&c.dir) == key)
        {
            if !found.candidates[i].sources.contains(&source) {
                found.candidates[i].sources.push(source);
            }
            continue;
        }
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        match cheap_check(env, &dir) {
            Ok((archives, patch)) => found.candidates.push(Candidate {
                dir,
                sources: vec![source],
                archives,
                patch,
            }),
            Err(reason) => {
                if matches!(
                    source,
                    CandidateSource::Remembered | CandidateSource::Environment
                ) {
                    found.rejected.push(Rejected {
                        dir,
                        source,
                        reason,
                    });
                }
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Validation, for people

/// The report of [`check_install`] boiled down for a screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationSummary {
    /// The folder that was checked.
    pub dir: PathBuf,
    /// No error was found: the install can be used (warnings do not stop it).
    pub usable: bool,
    /// How many errors the check found.
    pub errors: usize,
    /// How many warnings there are (an outdated patch level counts as one).
    pub warnings: usize,
    /// The patch level is older than [`NEWEST_KNOWN_PATCH`].
    pub outdated: bool,
    /// One line: the verdict.
    pub headline: String,
    /// The details: the patch level, then every problem with its advice.
    pub lines: Vec<String>,
}

/// Turn a report into a verdict and lines for a screen. Nothing in it is a path to a file inside
/// the install other than the archive names the report already holds.
pub fn summarize(report: &InstallReport) -> ValidationSummary {
    let errors = report.errors().len();
    let mut lines = Vec::new();
    let readable = report.archives.iter().filter(|a| a.error.is_none()).count();
    lines.push(format!(
        "Archives: {} found, {readable} readable",
        report.archives.len()
    ));
    let mut outdated = false;
    match &report.patch_level {
        Some(level) => {
            let (major, minor, build) = level.key();
            let (n_major, n_minor, n_build) = NEWEST_KNOWN_PATCH;
            outdated = (major, minor, build) < (n_major, n_minor, n_build);
            lines.push(format!("Patch level: {level}"));
        }
        None => lines.push("Patch level: none found".to_owned()),
    }
    for finding in &report.findings {
        let label = match finding.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
            Severity::Info => "NOTE",
        };
        lines.push(format!("{label}: {}", finding.message));
        if let Some(advice) = &finding.advice {
            lines.push(format!("  -> {advice}"));
        }
    }
    if outdated && let Some(level) = &report.patch_level {
        lines.push(format!(
            "WARNING: the patch level {level} is older than 2.100 (build 800000), the newest the project has seen"
        ));
        lines.push(
            "  -> Update the game, or copy all the Patch_02_*.gro files of the newest patch into the folder"
                .to_owned(),
        );
    }
    let warnings = report.warnings().len() + usize::from(outdated);
    let headline = match (errors, warnings) {
        (0, 0) => "Serious Sam 2 found: the install looks complete".to_owned(),
        (0, w) => format!("Usable, with {w} warning{}", if w == 1 { "" } else { "s" }),
        (e, _) => format!(
            "This folder cannot be used: {e} error{}",
            if e == 1 { "" } else { "s" }
        ),
    };
    ValidationSummary {
        dir: report.dir.clone(),
        usable: errors == 0,
        errors,
        warnings,
        outdated,
        headline,
        lines,
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Memory

/// The remembered choice: the folder and when it was validated. Nothing else is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberedChoice {
    /// The folder of the install.
    pub dir: PathBuf,
    /// When it was last validated, in seconds since 1970-01-01 UTC.
    pub validated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct StoredChoice {
    version: u32,
    path: String,
    validated_at: u64,
}

/// What [`load_choice`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadedChoice {
    /// There is no file: nothing was chosen yet.
    Missing,
    /// The file is there but cannot be used; the text says why.
    Damaged(String),
    /// A choice that is well-formed (it may no longer be valid: check it).
    Found(RememberedChoice),
}

/// `<config dir>/data_location.toml`.
pub fn choice_file(config_dir: &Path) -> PathBuf {
    config_dir.join(CHOICE_FILE_NAME)
}

/// Read the remembered choice. **Never fails**: a missing file, an unreadable one, text that is
/// not TOML, a path that is missing, empty, relative, too long or holds a control character are
/// all reported in the result. Unknown extra keys (from a newer build) are ignored, and a missing
/// or invalid `validated_at` reads as 0.
pub fn load_choice(file: &Path) -> LoadedChoice {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LoadedChoice::Missing,
        Err(e) => return LoadedChoice::Damaged(format!("cannot read {}: {e}", file.display())),
    };
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(e) => {
            return LoadedChoice::Damaged(format!(
                "{} is not valid TOML: {}",
                file.display(),
                e.message()
            ));
        }
    };
    let Some(path) = table.get("path").and_then(toml::Value::as_str) else {
        return LoadedChoice::Damaged(format!("{} has no folder path", file.display()));
    };
    if path.trim().is_empty() {
        return LoadedChoice::Damaged(format!("{} has an empty folder path", file.display()));
    }
    if path.chars().count() > MAX_STORED_PATH_CHARS || path.chars().any(char::is_control) {
        return LoadedChoice::Damaged(format!(
            "{} has a folder path that cannot be real",
            file.display()
        ));
    }
    let dir = PathBuf::from(path);
    if !dir.is_absolute() {
        return LoadedChoice::Damaged(format!(
            "{} has a folder path that is not absolute: {path}",
            file.display()
        ));
    }
    let validated_at = table
        .get("validated_at")
        .and_then(toml::Value::as_integer)
        .and_then(|v| u64::try_from(v).ok())
        .unwrap_or(0);
    LoadedChoice::Found(RememberedChoice { dir, validated_at })
}

/// Remember a choice. The file is written to a sibling and renamed over the old one, so a crash
/// leaves the old file or the new one and never half of one; missing folders are created.
///
/// # Errors
/// [`Error::Invalid`] when the path is not absolute or is not text (a path that is not valid
/// UTF-8 cannot be stored); [`Error::Io`] when the file cannot be written.
pub fn save_choice(file: &Path, choice: &RememberedChoice) -> Result<()> {
    let path = choice.dir.to_str().ok_or_else(|| {
        Error::invalid(
            "data location",
            "the folder path is not valid text, so it cannot be remembered",
        )
    })?;
    if !choice.dir.is_absolute() {
        return Err(Error::invalid(
            "data location",
            format!("{path:?} is not an absolute path, so it cannot be remembered"),
        ));
    }
    let text = toml::to_string_pretty(&StoredChoice {
        version: SCHEMA_VERSION,
        path: path.to_owned(),
        validated_at: choice.validated_at,
    })
    .map_err(|e| Error::invalid("data location", e.to_string()))?;
    if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let tmp = file.with_extension("toml.tmp");
    std::fs::write(&tmp, text).map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, file).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::io(file, e)
    })
}

/// Forget the choice (the file is removed; no file is not an error).
///
/// # Errors
/// [`Error::Io`] when the file exists and cannot be removed.
pub fn forget_choice(file: &Path) -> Result<()> {
    match std::fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(file, e)),
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Start-up

/// Where the folder in use came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupSource {
    /// Given on the command line (`--data`).
    Explicit,
    /// The `SS2_DATA_DIR` variable.
    Environment,
    /// The choice remembered from an earlier run, validated again now.
    Remembered,
}

/// Why the wizard has to be shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WizardReason {
    /// Nothing was chosen yet.
    FirstRun,
    /// The file that remembers the choice is damaged (the text says how); it is ignored.
    ChoiceDamaged(String),
    /// The remembered folder no longer passes the check.
    RememberedUnusable(ValidationSummary),
    /// The folder named on the command line or in `SS2_DATA_DIR` does not pass the check.
    GivenUnusable(StartupSource, ValidationSummary),
}

impl WizardReason {
    /// A sentence for the top of the wizard.
    pub fn message(&self) -> String {
        match self {
            Self::FirstRun => "Choose the folder where Serious Sam 2 is installed".to_owned(),
            Self::ChoiceDamaged(_) => {
                "The saved folder choice could not be read: choose the folder again".to_owned()
            }
            Self::RememberedUnusable(s) => {
                format!(
                    "The folder you chose before no longer works ({}): choose it again",
                    short_dir(&s.dir)
                )
            }
            Self::GivenUnusable(source, s) => {
                let who = match source {
                    StartupSource::Explicit => "The folder given with --data",
                    StartupSource::Environment => "The folder in SS2_DATA_DIR",
                    StartupSource::Remembered => "The folder",
                };
                format!(
                    "{who} does not work ({}): choose the folder again",
                    short_dir(&s.dir)
                )
            }
        }
    }
}

/// A folder name short enough for one line.
fn short_dir(dir: &Path) -> String {
    let text = dir.display().to_string();
    if text.chars().count() <= 48 {
        return text;
    }
    let tail: String = text
        .chars()
        .rev()
        .take(45)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("...{tail}")
}

/// What to do at start-up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Startup {
    /// A folder that passes the check: go on.
    Ready {
        /// The folder.
        dir: PathBuf,
        /// Where it came from.
        source: StartupSource,
        /// The report of the check just made.
        report: InstallReport,
    },
    /// Show the wizard.
    NeedsWizard(WizardReason),
}

/// Decide what the start-up does about the game data. In order: `explicit` (the command line),
/// the `SS2_DATA_DIR` variable, the remembered choice in `store`. The first that **passes
/// [`check_install`]** is used; a folder that was named but fails is **not** skipped silently (the
/// wizard says so), and nothing is written. Needs no window.
pub fn resolve_startup(env: &dyn Environment, explicit: Option<&Path>, store: &Path) -> Startup {
    let from_env = nonempty(env.var(DATA_DIR_ENV)).map(PathBuf::from);
    let given = explicit
        .map(|p| (StartupSource::Explicit, p.to_path_buf()))
        .or_else(|| from_env.map(|p| (StartupSource::Environment, p)));
    if let Some((source, dir)) = given {
        let report = check_install(&dir);
        return if report.errors().is_empty() {
            Startup::Ready {
                dir,
                source,
                report,
            }
        } else {
            Startup::NeedsWizard(WizardReason::GivenUnusable(source, summarize(&report)))
        };
    }
    match load_choice(store) {
        LoadedChoice::Missing => Startup::NeedsWizard(WizardReason::FirstRun),
        LoadedChoice::Damaged(why) => Startup::NeedsWizard(WizardReason::ChoiceDamaged(why)),
        LoadedChoice::Found(choice) => {
            let report = check_install(&choice.dir);
            if report.errors().is_empty() {
                Startup::Ready {
                    dir: choice.dir,
                    source: StartupSource::Remembered,
                    report,
                }
            } else {
                Startup::NeedsWizard(WizardReason::RememberedUnusable(summarize(&report)))
            }
        }
    }
}
