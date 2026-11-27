//! Mod validation (`ss2-doctor --mod`, roadmap T13-009; the lints and their codes are listed in
//! `docs/MOD_VALIDATION.md`).
//!
//! [`check_mods`] looks at one or more mods, given as stacks of [`ModLayer`]s on top of a base
//! install, and reports what is wrong with them as [`ModFinding`]s: a descriptor that is missing
//! or malformed, a version the install does not meet, a reference to a file that exists nowhere,
//! an unsafe path, a file that shadows a base file, two mods that both provide the same file.
//! [`check_install_mods`] does the same for mods that sit in an install folder.
//!
//! The checks never fail with an error of their own and never panic on damaged input: every
//! problem becomes a finding. The output is deterministic (sorted, no hash order, no time).
//!
//! What a *reference* is: the values of a few keys of the `.nfo` files in the mod (`MODEL`,
//! `THUMBNAIL`, `INTRO_MOVIE`, ...: [`Nfo`]). References inside the binary formats (`.ep`,
//! `.rsc`, `.wld`, `.mdl`) are not read: see `docs/MOD_VALIDATION.md`, "Not covered".

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ss2_core::Error;

use crate::check::{PatchLevel, Severity};
use crate::{
    ArchiveVfs, DirectoryVfs, EntryKind, LayeredVfs, ModManifest, Nfo, Stat, Vfs, VfsPath,
    mod_archive_names, mount_install,
};

/// The most bytes read from one descriptor or `.nfo` file; a larger one is reported, not read.
const MAX_TEXT_BYTES: u64 = 1 << 20;
/// Two files of two mods are compared byte for byte only up to this size.
const MAX_COMPARE_BYTES: u64 = 8 << 20;
/// How many example paths a summarising finding names.
const EXAMPLES: usize = 3;
/// The game module the base game loads: the only one a Rust engine can stand in for.
const BASE_GAME_MODULE: &str = "Sam2Game";

/// Every lint of the mod check. The code and the severity of a lint never change; a new lint
/// gets a new code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lint {
    /// `MOD001` the `.mod` file is missing.
    ModFileMissing,
    /// `MOD002` the `.mod` file cannot be read or parsed.
    ModFileMalformed,
    /// `MOD003` the `.nfo` file is missing.
    NfoMissing,
    /// `MOD004` an `.nfo` file cannot be read or parsed.
    NfoMalformed,
    /// `MOD005` the descriptor `.nfo` has no usable `NAME`.
    NfoNoName,
    /// `MOD006` the descriptor `.nfo` has no `AUTHOR`.
    NfoNoAuthor,
    /// `MOD007` a layer carries the descriptor of another mod id.
    ForeignDescriptor,
    /// `MOD008` the descriptor inside the mod's archives differs from the loose or base copy.
    DescriptorCopiesDiffer,
    /// `MOD009` the mod has no files at all.
    NothingToCheck,
    /// `MOD020` `REQUIRES_PATCH` is not a version.
    RequiresPatchMalformed,
    /// `MOD021` the install is older than the patch level the mod declares.
    PatchTooOld,
    /// `MOD022` an update archive's name asks for a newer patch than the install has.
    UpdateNeedsNewerPatch,
    /// `MOD023` the mod names a game module of its own (native code).
    NativeGameModule,
    /// `MOD024` the game module the `.mod` names is not in `Bin/`.
    GameModuleFileMissing,
    /// `MOD030` a player model's `MODEL` file exists nowhere.
    MissingModel,
    /// `MOD031` another referenced file (thumbnail, loading screen, movie) exists nowhere.
    MissingResource,
    /// `MOD032` a reference with `..`, an absolute path, a drive letter or control characters.
    UnsafeReference,
    /// `MOD033` a reference that mixes `/` and `\`.
    MixedSeparators,
    /// `MOD034` a reference key with an empty value.
    EmptyReference,
    /// `MOD035` a level `.nfo` without its `.wld`.
    LevelWithoutWorld,
    /// `MOD036` a reference to a file of an unexpected type.
    UnexpectedExtension,
    /// `MOD038` an archive entry whose name is not a valid path (not loaded at all).
    InvalidEntryName,
    /// `MOD039` two entries of one archive have the same path (case, separators or repeated).
    DuplicateEntry,
    /// `MOD040` a file name that is not valid on Windows.
    WindowsUnsafeName,
    /// `MOD041` a file outside `Content/`.
    OutsideContent,
    /// `MOD042` placeholder archives (only `dummy.txt`).
    PlaceholderLayers,
    /// `MOD043` a descriptor or `.nfo` too large to be one.
    NfoTooLarge,
    /// `MOD044` a file or folder of the mod cannot be read.
    Unreadable,
    /// `MOD045` a folder has names that differ only by case.
    CaseCollision,
    /// `MOD046` native code or foreign archives inside the mod.
    NativeCodeInside,
    /// `MOD050` how many files the mod overrides.
    OverrideSummary,
    /// `MOD051` the mod overrides a file type or file the engine treats as critical.
    CriticalOverride,
    /// `MOD060` two mods provide the same files, with different content.
    ModConflict,
    /// `MOD061` two mods provide the same files, with the same content.
    ModDuplicate,
    /// `MOD090` the install cannot be read.
    InstallUnreadable,
    /// `MOD091` the mod is not in the install.
    ModNotFound,
    /// `MOD092` an archive of the mod cannot be opened.
    ArchiveUnreadable,
}

impl Lint {
    /// Every lint, in code order.
    pub const ALL: [Lint; 37] = [
        Lint::ModFileMissing,
        Lint::ModFileMalformed,
        Lint::NfoMissing,
        Lint::NfoMalformed,
        Lint::NfoNoName,
        Lint::NfoNoAuthor,
        Lint::ForeignDescriptor,
        Lint::DescriptorCopiesDiffer,
        Lint::NothingToCheck,
        Lint::RequiresPatchMalformed,
        Lint::PatchTooOld,
        Lint::UpdateNeedsNewerPatch,
        Lint::NativeGameModule,
        Lint::GameModuleFileMissing,
        Lint::MissingModel,
        Lint::MissingResource,
        Lint::UnsafeReference,
        Lint::MixedSeparators,
        Lint::EmptyReference,
        Lint::LevelWithoutWorld,
        Lint::UnexpectedExtension,
        Lint::InvalidEntryName,
        Lint::DuplicateEntry,
        Lint::WindowsUnsafeName,
        Lint::OutsideContent,
        Lint::PlaceholderLayers,
        Lint::NfoTooLarge,
        Lint::Unreadable,
        Lint::CaseCollision,
        Lint::NativeCodeInside,
        Lint::OverrideSummary,
        Lint::CriticalOverride,
        Lint::ModConflict,
        Lint::ModDuplicate,
        Lint::InstallUnreadable,
        Lint::ModNotFound,
        Lint::ArchiveUnreadable,
    ];

    /// The stable code (`MOD030`).
    pub fn code(self) -> &'static str {
        match self {
            Lint::ModFileMissing => "MOD001",
            Lint::ModFileMalformed => "MOD002",
            Lint::NfoMissing => "MOD003",
            Lint::NfoMalformed => "MOD004",
            Lint::NfoNoName => "MOD005",
            Lint::NfoNoAuthor => "MOD006",
            Lint::ForeignDescriptor => "MOD007",
            Lint::DescriptorCopiesDiffer => "MOD008",
            Lint::NothingToCheck => "MOD009",
            Lint::RequiresPatchMalformed => "MOD020",
            Lint::PatchTooOld => "MOD021",
            Lint::UpdateNeedsNewerPatch => "MOD022",
            Lint::NativeGameModule => "MOD023",
            Lint::GameModuleFileMissing => "MOD024",
            Lint::MissingModel => "MOD030",
            Lint::MissingResource => "MOD031",
            Lint::UnsafeReference => "MOD032",
            Lint::MixedSeparators => "MOD033",
            Lint::EmptyReference => "MOD034",
            Lint::LevelWithoutWorld => "MOD035",
            Lint::UnexpectedExtension => "MOD036",
            Lint::InvalidEntryName => "MOD038",
            Lint::DuplicateEntry => "MOD039",
            Lint::WindowsUnsafeName => "MOD040",
            Lint::OutsideContent => "MOD041",
            Lint::PlaceholderLayers => "MOD042",
            Lint::NfoTooLarge => "MOD043",
            Lint::Unreadable => "MOD044",
            Lint::CaseCollision => "MOD045",
            Lint::NativeCodeInside => "MOD046",
            Lint::OverrideSummary => "MOD050",
            Lint::CriticalOverride => "MOD051",
            Lint::ModConflict => "MOD060",
            Lint::ModDuplicate => "MOD061",
            Lint::InstallUnreadable => "MOD090",
            Lint::ModNotFound => "MOD091",
            Lint::ArchiveUnreadable => "MOD092",
        }
    }

    /// How serious the lint is.
    pub fn severity(self) -> Severity {
        match self {
            Lint::ModFileMissing
            | Lint::ModFileMalformed
            | Lint::PatchTooOld
            | Lint::MissingModel
            | Lint::UnsafeReference
            | Lint::InvalidEntryName
            | Lint::CaseCollision
            | Lint::InstallUnreadable
            | Lint::ModNotFound
            | Lint::ArchiveUnreadable => Severity::Error,
            Lint::NfoMissing
            | Lint::NfoMalformed
            | Lint::NfoNoName
            | Lint::ForeignDescriptor
            | Lint::DescriptorCopiesDiffer
            | Lint::NothingToCheck
            | Lint::RequiresPatchMalformed
            | Lint::UpdateNeedsNewerPatch
            | Lint::NativeGameModule
            | Lint::GameModuleFileMissing
            | Lint::MissingResource
            | Lint::MixedSeparators
            | Lint::EmptyReference
            | Lint::LevelWithoutWorld
            | Lint::UnexpectedExtension
            | Lint::DuplicateEntry
            | Lint::WindowsUnsafeName
            | Lint::NfoTooLarge
            | Lint::Unreadable
            | Lint::NativeCodeInside
            | Lint::CriticalOverride
            | Lint::ModConflict => Severity::Warning,
            Lint::NfoNoAuthor
            | Lint::OutsideContent
            | Lint::PlaceholderLayers
            | Lint::OverrideSummary
            | Lint::ModDuplicate => Severity::Info,
        }
    }
}

/// One problem found in a mod, with what to do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModFinding {
    /// The lint that raised it.
    pub lint: Lint,
    /// The mod it is about (lower-case id; for a conflict, the mod that is loaded later).
    pub mod_id: String,
    /// The file it is about (a VFS path, or a host path for an install-level problem).
    pub path: Option<String>,
    /// What is wrong, in one sentence.
    pub message: String,
    /// What the mod author can do about it.
    pub hint: String,
}

impl ModFinding {
    /// The stable code, `MOD030`.
    pub fn code(&self) -> &'static str {
        self.lint.code()
    }

    /// The severity of the lint.
    pub fn severity(&self) -> Severity {
        self.lint.severity()
    }

    /// The message, led by the path when the message does not name it already.
    fn text(&self) -> String {
        match &self.path {
            Some(path) if !self.message.contains(path.as_str()) => {
                format!("{path}: {}", self.message)
            }
            _ => self.message.clone(),
        }
    }
}

fn label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "ERROR",
        Severity::Warning => "WARNING",
        Severity::Info => "INFO",
    }
}

/// One layer of a mod: an archive, a loose folder or a test file system.
pub struct ModLayer {
    name: String,
    vfs: Box<dyn Vfs>,
    skipped_entries: Vec<String>,
    duplicate_entries: Vec<String>,
}

impl ModLayer {
    /// A layer from any [`Vfs`]. `name` identifies it in messages (for an archive: its file name).
    pub fn new(name: impl Into<String>, vfs: impl Vfs + 'static) -> Self {
        Self {
            name: name.into(),
            vfs: Box::new(vfs),
            skipped_entries: Vec::new(),
            duplicate_entries: Vec::new(),
        }
    }

    /// A layer from an archive, keeping what the archive reader had to skip or merge.
    pub fn from_archive(name: impl Into<String>, archive: ArchiveVfs) -> Self {
        let skipped_entries = archive.skipped_entries().to_vec();
        let duplicate_entries = archive.duplicate_entries().to_vec();
        Self {
            name: name.into(),
            vfs: Box::new(archive),
            skipped_entries,
            duplicate_entries,
        }
    }

    /// The layer's name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// One mod to check: its id and its layers in mount order (the last one wins).
pub struct ModUnderTest {
    id: String,
    layers: Vec<ModLayer>,
    placeholders: Vec<String>,
}

impl ModUnderTest {
    /// A mod with no layers yet. `id` is the file stem of its `.mod` file (case is ignored).
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into().to_lowercase(),
            layers: Vec::new(),
            placeholders: Vec::new(),
        }
    }

    /// Add a layer on top of the ones added so far.
    #[must_use]
    pub fn with_layer(mut self, layer: ModLayer) -> Self {
        self.layers.push(layer);
        self
    }

    /// Note an archive that holds only the placeholder `dummy.txt` (it is not a layer).
    #[must_use]
    pub fn with_placeholder(mut self, name: impl Into<String>) -> Self {
        self.placeholders.push(name.into());
        self
    }

    /// The mod id (lower case).
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// What the mods are checked against.
pub struct ModCheckEnv<'a> {
    /// The base game: the archives and patches of the install. Override and conflict checks
    /// compare against this view only.
    pub base: &'a dyn Vfs,
    /// The loose files of the install folder (the mods' own `.mod`/`.nfo`, `Bin/`, thumbnails),
    /// when there is one. A reference may resolve here too.
    pub loose: Option<&'a dyn Vfs>,
    /// The newest patch of the install, if it has one.
    pub patch_level: Option<&'a PatchLevel>,
}

/// What the check learned about one mod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModSummary {
    /// The mod id (lower case).
    pub id: String,
    /// `NAME` of its `.nfo`, or the id.
    pub name: String,
    /// Number of layers that were read.
    pub layers: usize,
    /// Number of files in them (a path in two layers counts once).
    pub files: usize,
    /// How many of those files replace a file of the base game.
    pub overrides: usize,
}

/// Everything [`check_mods`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModReport {
    /// The patch level the mods were checked against.
    pub patch_level: Option<PatchLevel>,
    /// One entry per mod, in load order.
    pub mods: Vec<ModSummary>,
    /// The findings: by mod in load order, then most serious first, then by code and path.
    pub findings: Vec<ModFinding>,
}

impl ModReport {
    fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity() == severity)
            .count()
    }

    /// Number of errors.
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }

    /// Number of warnings.
    pub fn warnings(&self) -> usize {
        self.count(Severity::Warning)
    }

    /// Number of notes (info findings).
    pub fn infos(&self) -> usize {
        self.count(Severity::Info)
    }

    /// `0` when there is no error, `1` otherwise (warnings and notes do not change it).
    pub fn exit_code(&self) -> i32 {
        i32::from(self.errors() > 0)
    }

    /// The report as plain ASCII text for a terminal or a bug report.
    pub fn render(&self) -> String {
        let mut out = String::from("SS2RS mod check\n\n");
        match &self.patch_level {
            Some(level) => {
                let _ = writeln!(out, "Install patch level: {level}");
            }
            None => out.push_str("Install patch level: none found\n"),
        }
        for m in &self.mods {
            let _ = write!(out, "\nMod {}", m.id);
            if m.name != m.id {
                let _ = write!(out, " (\"{}\")", m.name);
            }
            let _ = writeln!(
                out,
                ": {} layer{}, {} file{}, {} override base files",
                m.layers,
                plural(m.layers),
                m.files,
                plural(m.files),
                m.overrides
            );
            let mut any = false;
            for f in self.findings.iter().filter(|f| f.mod_id == m.id) {
                any = true;
                let _ = writeln!(out, "  {:<8}{} {}", label(f.severity()), f.code(), f.text());
                let _ = writeln!(out, "          -> {}", f.hint);
            }
            if !any {
                out.push_str("  no findings\n");
            }
        }
        // Findings about no particular mod (the install could not be read, a mod is missing).
        for f in self
            .findings
            .iter()
            .filter(|f| !self.mods.iter().any(|m| m.id == f.mod_id))
        {
            let who = if f.mod_id.is_empty() {
                String::new()
            } else {
                format!(" [{}]", f.mod_id)
            };
            let _ = writeln!(
                out,
                "\n{:<8}{}{who} {}",
                label(f.severity()),
                f.code(),
                f.text()
            );
            let _ = writeln!(out, "          -> {}", f.hint);
        }
        let (e, w, i) = (self.errors(), self.warnings(), self.infos());
        out.push('\n');
        out.push_str(&match (e, w) {
            (0, 0) => format!("No problems found ({i} note{}).\n", plural(i)),
            (0, _) => format!(
                "No errors, {w} warning{}, {i} note{}.\n",
                plural(w),
                plural(i)
            ),
            _ => format!(
                "{e} error{}, {w} warning{}, {i} note{}: fix the errors first.\n",
                plural(e),
                plural(w),
                plural(i)
            ),
        });
        ascii(&out)
    }
}

pub(crate) fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// `s` with everything that is not printable ASCII (or a newline) written as an escape, so a
/// hostile name cannot garble a terminal.
pub(crate) fn ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '\n' || c.is_ascii_graphic() || c == ' ' {
            out.push(c);
        } else {
            out.extend(c.escape_default());
        }
    }
    out
}

fn is_file(vfs: &dyn Vfs, path: &VfsPath) -> bool {
    matches!(
        vfs.stat(path),
        Ok(Stat {
            kind: EntryKind::File,
            ..
        })
    )
}

fn vp(s: &str) -> VfsPath {
    VfsPath::new(s).unwrap_or_default()
}

/// One file of a mod: the topmost layer that has it, and its size there.
#[derive(Clone, Copy)]
struct ModFile {
    layer: usize,
    size: u64,
}

pub(crate) struct ScanProblem {
    lint: Lint,
    pub(crate) path: String,
    pub(crate) message: String,
}

/// Every file below the root of `vfs`, sorted, plus what could not be read.
pub(crate) fn scan_layer(vfs: &dyn Vfs) -> (Vec<(VfsPath, u64)>, Vec<ScanProblem>) {
    let mut files = Vec::new();
    let mut problems = Vec::new();
    let mut pending = vec![VfsPath::root()];
    while let Some(dir) = pending.pop() {
        let entries = match vfs.list(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                let lint = if matches!(error, Error::Invalid { .. }) {
                    Lint::CaseCollision
                } else {
                    Lint::Unreadable
                };
                problems.push(ScanProblem {
                    lint,
                    path: dir.to_string(),
                    message: format!("the folder cannot be listed ({error})"),
                });
                continue;
            }
        };
        for entry in entries {
            let Ok(path) = dir.join(&entry.name) else {
                continue;
            };
            match entry.kind {
                EntryKind::Dir => pending.push(path),
                EntryKind::File => match vfs.stat(&path) {
                    Ok(stat) => files.push((path, stat.size)),
                    Err(error) => problems.push(ScanProblem {
                        lint: Lint::Unreadable,
                        path: path.to_string(),
                        message: format!("the file cannot be inspected ({error})"),
                    }),
                },
            }
        }
    }
    files.sort();
    (files, problems)
}

/// File types whose replacement changes how the game plays (levels, entity parameters, global
/// databases and menus). A mod that does it is not a pure reskin, and in a game with several
/// players every peer must have the same ones.
const CRITICAL_EXTENSIONS: [(&str, &str); 3] = [
    ("wld", "levels"),
    ("ep", "entity parameters"),
    ("rsc", "resource databases and menu screens"),
];
/// Files that choose or describe the game itself.
const CRITICAL_PATHS: [&str; 4] = [
    "content/defaultmod.txt",
    "content/currentmod.txt",
    "content/serioussam2.mod",
    "content/serioussam2.nfo",
];
/// Native code and archives: a mod may not carry them (docs/MODDING_POLICY.md).
const NATIVE_EXTENSIONS: [&str; 5] = ["dll", "exe", "module", "gro", "so"];

/// The `.nfo` keys whose value is a path to a game file, the type the file should have, and a
/// name for messages. (Only a missing `MODEL` is an error, see [`Lint::MissingModel`].)
struct ReferenceKey {
    key: &'static str,
    extensions: &'static [&'static str],
    kind: &'static str,
}

const REFERENCE_KEYS: [ReferenceKey; 7] = [
    ReferenceKey {
        key: "MODEL",
        extensions: &["mdl"],
        kind: "model",
    },
    ReferenceKey {
        key: "LOADING_MODEL",
        extensions: &["mdl"],
        kind: "loading-screen model",
    },
    ReferenceKey {
        key: "THUMBNAIL",
        extensions: &["tex"],
        kind: "thumbnail",
    },
    ReferenceKey {
        key: "LOADING_SCREEN",
        extensions: &["tex"],
        kind: "loading screen",
    },
    ReferenceKey {
        key: "INTRO_MOVIE",
        extensions: &["ogv"],
        kind: "intro movie",
    },
    ReferenceKey {
        key: "OUTRO_MOVIE",
        extensions: &["ogv"],
        kind: "outro movie",
    },
    ReferenceKey {
        key: "EPISODE_MOVIE",
        extensions: &["ogv"],
        kind: "episode movie",
    },
];

/// Why a reference value is not a path.
enum Reference {
    Empty,
    Unsafe(&'static str),
    Path { path: VfsPath, mixed: bool },
}

fn classify_reference(raw: &str) -> Reference {
    if raw.trim().is_empty() {
        return Reference::Empty;
    }
    if raw.chars().any(char::is_control) {
        return Reference::Unsafe("it has control characters");
    }
    if raw.contains(':') {
        return Reference::Unsafe("it has a drive letter or a URL scheme (`:`)");
    }
    if raw.starts_with(['/', '\\']) {
        return Reference::Unsafe("it is an absolute path");
    }
    if raw.split(['/', '\\']).any(|segment| segment == "..") {
        return Reference::Unsafe("it climbs out of the game folder with `..`");
    }
    match VfsPath::new(raw) {
        Ok(path) => Reference::Path {
            path,
            mixed: raw.contains('/') && raw.contains('\\'),
        },
        Err(_) => Reference::Unsafe("it is not a valid game path"),
    }
}

/// Name of a Windows device that no file may be called, whatever its extension.
fn is_windows_device(stem: &str) -> bool {
    let lower = stem.to_ascii_lowercase();
    matches!(lower.as_str(), "con" | "prn" | "aux" | "nul")
        || ((lower.starts_with("com") || lower.starts_with("lpt"))
            && lower.len() == 4
            && lower.as_bytes()[3].is_ascii_digit()
            && lower.as_bytes()[3] != b'0')
}

fn windows_unsafe_segment(segment: &str) -> bool {
    segment.contains(['<', '>', '"', '|', '?', '*'])
        || segment.ends_with(' ')
        || segment.ends_with('.')
        || is_windows_device(segment.split('.').next().unwrap_or(segment))
}

/// `(update number, minor)` of an update archive name such as `z_Mod_Update_02_090.gro`: the two
/// last `_` parts are digits and the last has three. The reading (the minor of the patch it
/// needs) is an inference from the shipped names, see `docs/MOD_VALIDATION.md`.
fn update_patch_minor(archive_name: &str) -> Option<u32> {
    let stem = archive_name
        .rsplit_once('.')
        .map_or(archive_name, |(s, _)| s);
    let parts: Vec<&str> = stem.split('_').collect();
    let [.., update, minor] = parts.as_slice() else {
        return None;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(update) && digits(minor) && minor.len() == 3 && parts.len() > 2)
        .then(|| minor.parse().ok())
        .flatten()
}

/// `2`, `2.100` or `2.100.800000` as `(major, minor, build)`.
fn parse_patch_requirement(text: &str) -> Option<(u32, u32, u32)> {
    let number = |p: &str| {
        // Digits only: `parse` alone would accept a `+` sign (and it rejects the empty string).
        p.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| p.parse::<u32>().ok())
            .flatten()
    };
    let mut parts = text.trim().split('.');
    let major = number(parts.next()?)?;
    let minor = parts.next().map_or(Some(0), number)?;
    let build = parts.next().map_or(Some(0), number)?;
    parts.next().is_none().then_some((major, minor, build))
}

struct Checker<'a> {
    env: &'a ModCheckEnv<'a>,
    m: &'a ModUnderTest,
    findings: Vec<ModFinding>,
}

/// What reading a small text file gave.
enum Text {
    Missing,
    TooLarge(u64),
    Failed(String),
    Found(Vec<u8>),
}

impl Checker<'_> {
    fn add(&mut self, lint: Lint, path: Option<&str>, message: String, hint: &str) {
        self.findings.push(ModFinding {
            lint,
            mod_id: self.m.id.clone(),
            path: path.map(str::to_owned),
            message,
            hint: hint.to_owned(),
        });
    }

    /// One finding for a list of paths: the count and the first few.
    fn add_list(&mut self, lint: Lint, what: &str, paths: &[String], hint: &str) {
        let Some(first) = paths.first() else {
            return;
        };
        let shown: Vec<&str> = paths.iter().take(EXAMPLES).map(String::as_str).collect();
        let more = if paths.len() > EXAMPLES { ", ..." } else { "" };
        let message = format!("{what} ({}): {}{more}", paths.len(), shown.join(", "));
        let first = first.clone();
        self.add(lint, Some(&first), message, hint);
    }

    fn resolves(&self, path: &VfsPath) -> bool {
        self.m.layers.iter().any(|l| is_file(l.vfs.as_ref(), path))
            || is_file(self.env.base, path)
            || self.env.loose.is_some_and(|l| is_file(l, path))
    }

    fn read_small(vfs: &dyn Vfs, path: &VfsPath) -> Text {
        match vfs.stat(path) {
            Ok(Stat {
                kind: EntryKind::File,
                size,
            }) => {
                if size > MAX_TEXT_BYTES {
                    return Text::TooLarge(size);
                }
                match vfs.read(path) {
                    Ok(bytes) => Text::Found(bytes),
                    Err(error) => Text::Failed(error.to_string()),
                }
            }
            Ok(_) => Text::Failed("it is a folder".to_owned()),
            Err(Error::NotFound(_)) => Text::Missing,
            Err(error) => Text::Failed(error.to_string()),
        }
    }

    /// A descriptor: the topmost layer that has it, else the base, else the loose files. The
    /// second value is "the other copy" for the comparison of the two.
    fn read_descriptor(&self, path: &VfsPath) -> (Text, Option<Vec<u8>>) {
        let in_layer = self
            .m
            .layers
            .iter()
            .rev()
            .find(|l| is_file(l.vfs.as_ref(), path));
        let outside: Vec<&dyn Vfs> = std::iter::once(self.env.base)
            .chain(self.env.loose)
            .collect();
        let outside_copy = outside.iter().find(|v| is_file(**v, path)).and_then(|v| {
            match Self::read_small(*v, path) {
                Text::Found(b) => Some(b),
                _ => None,
            }
        });
        match in_layer {
            Some(layer) => (Self::read_small(layer.vfs.as_ref(), path), outside_copy),
            None => match outside.iter().find(|v| is_file(**v, path)) {
                Some(v) => (Self::read_small(*v, path), None),
                None => (Text::Missing, None),
            },
        }
    }

    fn descriptors(&mut self) -> Option<String> {
        let id = self.m.id.clone();
        let mod_path = vp(&format!("content/{id}.mod"));
        let nfo_path = vp(&format!("content/{id}.nfo"));

        let (mod_text, mod_other) = self.read_descriptor(&mod_path);
        match &mod_text {
            Text::Missing => self.add(
                Lint::ModFileMissing,
                Some(mod_path.as_str()),
                format!("the mod has no {mod_path}"),
                "Add Content/<Mod>.mod with `MODULES = Sam2Game ;` (inside the mod's archive or loose next to it).",
            ),
            Text::TooLarge(size) => self.add(
                Lint::ModFileMalformed,
                Some(mod_path.as_str()),
                format!("{mod_path} is {size} bytes: far too large for a .mod file"),
                "A .mod file is a few dozen bytes: `MODULES = <name> ... ;`.",
            ),
            Text::Failed(error) => self.add(
                Lint::ModFileMalformed,
                Some(mod_path.as_str()),
                format!("{mod_path} cannot be read ({error})"),
                "Check that the file is intact; repack the mod if it came from a damaged archive.",
            ),
            Text::Found(bytes) => {
                let text = String::from_utf8_lossy(bytes);
                match ModManifest::parse(&text) {
                    Ok(manifest) => self.game_modules(&manifest, &mod_path),
                    Err(error) => self.add(
                        Lint::ModFileMalformed,
                        Some(mod_path.as_str()),
                        format!("{mod_path} is not a valid .mod file ({error})"),
                        "The format is `MODULES = <name> <name> ... ;` (optional `<TOOLS>` prefix on a name).",
                    ),
                }
            }
        }

        if let (Text::Found(a), Some(b)) = (&mod_text, &mod_other)
            && a != b
        {
            self.copies_differ(&mod_path);
        }

        let mut display = None;
        let (nfo, other) = self.read_descriptor(&nfo_path);
        match nfo {
            Text::Missing => self.add(
                Lint::NfoMissing,
                Some(nfo_path.as_str()),
                format!("the mod has no {nfo_path}, so the mod list shows only its id"),
                "Add Content/<Mod>.nfo with NAME, AUTHOR and THUMBNAIL.",
            ),
            Text::TooLarge(size) => self.add(
                Lint::NfoTooLarge,
                Some(nfo_path.as_str()),
                format!("{nfo_path} is {size} bytes: far too large for a descriptor"),
                "A descriptor is a few lines of KEY = \"value\".",
            ),
            Text::Failed(error) => self.add(
                Lint::NfoMalformed,
                Some(nfo_path.as_str()),
                format!("{nfo_path} cannot be read ({error})"),
                "Check that the file is intact; repack the mod if it came from a damaged archive.",
            ),
            Text::Found(bytes) => {
                match Nfo::parse(&bytes) {
                    Ok(nfo) => {
                        match nfo.get("NAME").map(str::trim).filter(|n| !n.is_empty()) {
                            Some(name) => display = Some(name.to_owned()),
                            None => self.add(
                                Lint::NfoNoName,
                                Some(nfo_path.as_str()),
                                format!("{nfo_path} has no NAME, so the mod list shows the id"),
                                "Add `NAME = \"My Mod\" ;`.",
                            ),
                        }
                        if nfo.get("AUTHOR").is_none_or(|a| a.trim().is_empty()) {
                            self.add(
                                Lint::NfoNoAuthor,
                                Some(nfo_path.as_str()),
                                format!("{nfo_path} names no AUTHOR"),
                                "Add `AUTHOR = \"Your Name\" ;` so players know whom to credit.",
                            );
                        }
                        self.version(&nfo, &nfo_path);
                    }
                    Err(error) => self.add(
                        Lint::NfoMalformed,
                        Some(nfo_path.as_str()),
                        format!("{nfo_path} is not a valid .nfo file ({error})"),
                        "Each line is `KEY = \"value\" ;`; check for a missing `=` or a quote that is not closed.",
                    ),
                }
                if other.is_some_and(|other| other != bytes) {
                    self.copies_differ(&nfo_path);
                }
            }
        }
        display
    }

    fn copies_differ(&mut self, path: &VfsPath) {
        self.add(
            Lint::DescriptorCopiesDiffer,
            Some(path.as_str()),
            format!("{path} inside the mod's archive differs from the copy in the install folder"),
            "Make the two copies the same, or remove one, so that the mod list and the game agree.",
        );
    }

    fn game_modules(&mut self, manifest: &ModManifest, mod_path: &VfsPath) {
        for name in manifest.game_modules() {
            if name.eq_ignore_ascii_case(BASE_GAME_MODULE) {
                continue;
            }
            self.add(
                Lint::NativeGameModule,
                Some(mod_path.as_str()),
                format!(
                    "the mod loads its own game module `{name}`, native code that this engine cannot load; only its content can be used"
                ),
                "Its gameplay changes will not run. A data-only mod names `Sam2Game` (docs/MODDING_POLICY.md).",
            );
            let dll = vp(&format!("bin/{name}.dll"));
            let bin = vp("bin");
            let has_bin = |v: &dyn Vfs| v.stat(&bin).is_ok_and(|s| s.kind == EntryKind::Dir);
            let bin_known = has_bin(self.env.base) || self.env.loose.is_some_and(has_bin);
            if bin_known && !self.resolves(&dll) {
                self.add(
                    Lint::GameModuleFileMissing,
                    Some(dll.as_str()),
                    format!("the game module `{name}` is not in Bin/ ({dll} not found)"),
                    "Copy the module's DLL into the install's Bin folder, or name Sam2Game.",
                );
            }
        }
    }

    fn version(&mut self, nfo: &Nfo, nfo_path: &VfsPath) {
        let Some(raw) = nfo.get("REQUIRES_PATCH") else {
            return;
        };
        let Some(needed) = parse_patch_requirement(raw) else {
            self.add(
                Lint::RequiresPatchMalformed,
                Some(nfo_path.as_str()),
                format!("REQUIRES_PATCH = {raw:?} is not a version"),
                "Write it like the patch archives: \"2.100\" or \"2.100.800000\" (major.minor[.build]).",
            );
            return;
        };
        let have = self.env.patch_level.map(PatchLevel::key);
        if have.is_none_or(|have| have < needed) {
            let have_text = self
                .env
                .patch_level
                .map_or_else(|| "no patch".to_owned(), ToString::to_string);
            let need_text = if needed.2 == 0 {
                format!("{}.{:03}", needed.0, needed.1)
            } else {
                format!("{}.{:03} (build {})", needed.0, needed.1, needed.2)
            };
            self.add(
                Lint::PatchTooOld,
                Some(nfo_path.as_str()),
                format!("the mod needs patch level {need_text}, the install has {have_text}"),
                "Install the newer Patch_02_*.gro files, or ask the mod author for a version that works with the install.",
            );
        }
    }

    fn update_archives(&mut self) {
        let have = self.env.patch_level.map_or(0, |l| l.minor);
        let names: Vec<String> = self
            .m
            .layers
            .iter()
            .map(|l| l.name.clone())
            .chain(self.m.placeholders.iter().cloned())
            .collect();
        for name in names {
            if let Some(minor) = update_patch_minor(&name)
                && minor > have
            {
                let have_text = self
                    .env
                    .patch_level
                    .map_or_else(|| "no patch".to_owned(), ToString::to_string);
                self.add(
                    Lint::UpdateNeedsNewerPatch,
                    Some(&name),
                    format!(
                        "{name} is named for patch 2.{minor:03} (inferred from the name), the install has {have_text}"
                    ),
                    "Install the matching Patch_02_*.gro, or check with the mod author whether this update pack still applies.",
                );
            }
        }
    }

    fn layer_notes(&mut self) {
        for index in 0..self.m.layers.len() {
            let layer = &self.m.layers[index];
            let name = layer.name.clone();
            let skipped: Vec<String> = layer.skipped_entries.clone();
            let duplicates: Vec<String> = layer.duplicate_entries.clone();
            self.add_list(
                Lint::InvalidEntryName,
                &format!("entries of {name} with invalid names (`..`, a drive letter or control characters), which are not loaded"),
                &skipped,
                "Repack the archive with plain relative names such as Content/MyMod/x.tex.",
            );
            self.add_list(
                Lint::DuplicateEntry,
                &format!("entries of {name} that repeat an earlier path (same name, or only the case or the slashes differ), the last one wins"),
                &duplicates,
                "Keep one entry per path: on a case-sensitive system the two names are different files.",
            );
        }
    }

    fn scan(&mut self) -> BTreeMap<VfsPath, ModFile> {
        let mut all: BTreeMap<VfsPath, ModFile> = BTreeMap::new();
        for index in 0..self.m.layers.len() {
            let (files, problems) = scan_layer(self.m.layers[index].vfs.as_ref());
            let name = self.m.layers[index].name.clone();
            for p in problems {
                let hint = if p.lint == Lint::CaseCollision {
                    "Rename one of them: two names that differ only by case are one file to the game and two on a case-sensitive system."
                } else {
                    "Check that the folder or archive is intact and that you may read it."
                };
                self.add(
                    p.lint,
                    Some(&p.path),
                    format!("{name}: {}", p.message),
                    hint,
                );
            }
            for (path, size) in files {
                all.insert(path, ModFile { layer: index, size });
            }
        }
        all
    }

    fn name_lints(&mut self, files: &BTreeMap<VfsPath, ModFile>) {
        let mut unsafe_names = Vec::new();
        let mut outside = Vec::new();
        let mut native: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for path in files.keys() {
            let s = path.as_str();
            if s.split('/').any(windows_unsafe_segment) {
                unsafe_names.push(s.to_owned());
            }
            if !s.starts_with("content/") && s != "dummy.txt" {
                outside.push(s.to_owned());
            }
            if let Some(ext) = path.extension()
                && let Some(known) = NATIVE_EXTENSIONS.iter().find(|n| **n == ext)
            {
                native.entry(known).or_default().push(s.to_owned());
            }
        }
        self.add_list(
            Lint::WindowsUnsafeName,
            "file names that are not valid on Windows (`<>\"|?*`, a trailing space or dot, a device name such as CON)",
            &unsafe_names,
            "Rename them: the mod cannot be unpacked on Windows as it is.",
        );
        self.add_list(
            Lint::OutsideContent,
            "files outside Content/, where the game's loaders do not look",
            &outside,
            "Move them below Content/<Mod>/ (a mod's own files) or Content/SeriousSam2/ (overrides).",
        );
        for (ext, paths) in native {
            self.add_list(
                Lint::NativeCodeInside,
                &format!("files of type .{ext}: native code or foreign archives that cannot be loaded and may not be redistributed"),
                &paths,
                "Remove them; see docs/MODDING_POLICY.md (what a mod may contain).",
            );
        }
        let names = self.m.placeholders.clone();
        self.add_list(
            Lint::PlaceholderLayers,
            "archives that hold only the placeholder dummy.txt (they mark an optional pack and are ignored)",
            &names,
            "Nothing to do; they are skipped.",
        );
    }

    fn overrides(&mut self, files: &BTreeMap<VfsPath, ModFile>) -> usize {
        let mut by_ext: BTreeMap<String, usize> = BTreeMap::new();
        let mut critical: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        let mut critical_paths = Vec::new();
        let mut count = 0usize;
        for path in files.keys() {
            if !is_file(self.env.base, path) {
                continue;
            }
            count += 1;
            let ext = path.extension().unwrap_or("(none)");
            *by_ext.entry(ext.to_owned()).or_default() += 1;
            if let Some((ext, _)) = CRITICAL_EXTENSIONS
                .iter()
                .find(|(e, _)| Some(*e) == path.extension())
            {
                critical.entry(ext).or_default().push(path.to_string());
            }
            if CRITICAL_PATHS.contains(&path.as_str()) {
                critical_paths.push(path.to_string());
            }
        }
        if count > 0 {
            let mut kinds: Vec<(&String, &usize)> = by_ext.iter().collect();
            kinds.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let list: Vec<String> = kinds
                .iter()
                .take(8)
                .map(|(e, n)| format!(".{e} {n}"))
                .collect();
            let more = if kinds.len() > 8 { ", ..." } else { "" };
            self.add(
                Lint::OverrideSummary,
                None,
                format!(
                    "{count} of {} files replace base game files, {} are new (replaced by type: {}{more})",
                    files.len(),
                    files.len() - count,
                    list.join(", ")
                ),
                "Nothing to fix: this is what the mod changes. See the critical overrides below, if any.",
            );
        }
        for (ext, paths) in critical {
            let why = CRITICAL_EXTENSIONS
                .iter()
                .find(|(e, _)| *e == ext)
                .map_or("", |(_, why)| *why);
            self.add_list(
                Lint::CriticalOverride,
                &format!("files of type .{ext} ({why}) that replace the base game's: the mod changes how the game plays, not only how it looks"),
                &paths,
                "Intended? In a game with several players every player needs the same files.",
            );
        }
        self.add_list(
            Lint::CriticalOverride,
            "files that choose or describe the game itself and are replaced",
            &critical_paths,
            "A mod should not replace the game's own descriptor or the mod selection file.",
        );
        count
    }

    /// `.nfo` files of the mod (and the descriptor, when it is outside the layers).
    fn nfo_files(&mut self, files: &BTreeMap<VfsPath, ModFile>) -> Vec<(VfsPath, Vec<u8>)> {
        let own = vp(&format!("content/{}.nfo", self.m.id));
        let mut out = Vec::new();
        for (path, file) in files {
            if path.extension() != Some("nfo") || *path == own {
                continue;
            }
            let layer = &self.m.layers[file.layer];
            match Self::read_small(layer.vfs.as_ref(), path) {
                Text::Found(bytes) => out.push((path.clone(), bytes)),
                Text::TooLarge(size) => self.add(
                    Lint::NfoTooLarge,
                    Some(path.as_str()),
                    format!("{path} is {size} bytes: far too large for an .nfo file"),
                    "An .nfo file is a few lines of KEY = \"value\".",
                ),
                Text::Failed(error) => self.add(
                    Lint::Unreadable,
                    Some(path.as_str()),
                    format!("{path} cannot be read ({error})"),
                    "Check that the archive is intact.",
                ),
                Text::Missing => {}
            }
        }
        out
    }

    /// `own` is the mod's descriptor: its syntax errors are already reported.
    fn references(&mut self, nfos: &[(VfsPath, Vec<u8>)], own: &VfsPath) {
        for (path, bytes) in nfos {
            let nfo = match Nfo::parse(bytes) {
                Ok(nfo) => nfo,
                Err(error) => {
                    if path != own {
                        self.add(
                            Lint::NfoMalformed,
                            Some(path.as_str()),
                            format!("{path} is not a valid .nfo file ({error})"),
                            "Each line is `KEY = \"value\"`; check for a missing `=` or a quote that is not closed.",
                        );
                    }
                    continue;
                }
            };
            if nfo
                .kind
                .as_deref()
                .is_some_and(|k| k.eq_ignore_ascii_case("LEVEL"))
            {
                let world = path
                    .as_str()
                    .strip_suffix(".nfo")
                    .map(|stem| vp(&format!("{stem}.wld")));
                if let Some(world) = world
                    && !self.resolves(&world)
                {
                    self.add(
                        Lint::LevelWithoutWorld,
                        Some(path.as_str()),
                        format!("the level {path} has no {world} next to it"),
                        "A level is a .nfo and a .wld of the same name in one folder; add the .wld or remove the .nfo.",
                    );
                }
            }
            for (key, value) in &nfo.entries {
                let Some(rule) = REFERENCE_KEYS
                    .iter()
                    .find(|r| r.key.eq_ignore_ascii_case(key))
                else {
                    continue;
                };
                self.reference(path, rule, key, value);
            }
        }
    }

    fn reference(&mut self, nfo: &VfsPath, rule: &ReferenceKey, key: &str, value: &str) {
        let at = Some(nfo.as_str());
        match classify_reference(value) {
            Reference::Empty => self.add(
                Lint::EmptyReference,
                at,
                format!("{nfo}: {key} is empty"),
                "Give the path of the file, or remove the key.",
            ),
            Reference::Unsafe(why) => self.add(
                Lint::UnsafeReference,
                at,
                format!("{nfo}: {key} = {value:?} is not allowed: {why}"),
                "Use a relative path below Content/, written like Content/<Mod>/Models/x.mdl.",
            ),
            Reference::Path { path, mixed } => {
                if mixed {
                    self.add(
                        Lint::MixedSeparators,
                        at,
                        format!("{nfo}: {key} = {value:?} mixes `/` and `\\`"),
                        "Write the path with `/` only.",
                    );
                }
                if !self.resolves(&path) {
                    let (lint, hint) = if rule.key.eq_ignore_ascii_case("MODEL") {
                        (
                            Lint::MissingModel,
                            "Add the model to the mod, or point MODEL at a model of the base game.",
                        )
                    } else {
                        (
                            Lint::MissingResource,
                            "Add the file to the mod, or point the key at a file of the base game (or another mod you require).",
                        )
                    };
                    self.add(
                        lint,
                        at,
                        format!(
                            "{nfo}: {key} = {value:?} ({}) exists neither in the mod nor in the install",
                            rule.kind
                        ),
                        hint,
                    );
                } else if path
                    .extension()
                    .is_none_or(|e| !rule.extensions.contains(&e))
                {
                    self.add(
                        Lint::UnexpectedExtension,
                        at,
                        format!(
                            "{nfo}: {key} = {value:?} should be a .{} file",
                            rule.extensions.join(" or .")
                        ),
                        "Check the path: the game expects this type of file here.",
                    );
                }
            }
        }
    }
}

/// Check `mods` (in load order, the last wins) against `env`.
///
/// Every mod gets the checks of `docs/MOD_VALIDATION.md`; with more than one mod the files that
/// two of them provide are reported as conflicts.
pub fn check_mods(env: &ModCheckEnv<'_>, mods: &[ModUnderTest]) -> ModReport {
    let mut per_mod: Vec<(ModSummary, Vec<ModFinding>)> = Vec::new();
    let mut tables: Vec<BTreeMap<VfsPath, ModFile>> = Vec::new();
    for m in mods {
        let mut checker = Checker {
            env,
            m,
            findings: Vec::new(),
        };
        let display = checker.descriptors();
        checker.update_archives();
        checker.layer_notes();
        let files = checker.scan();
        if files.is_empty() {
            checker.add(
                Lint::NothingToCheck,
                None,
                "the mod has no files in any of its layers".to_owned(),
                "Check that the archives are in Content/<Mod>/ and are not empty.",
            );
        }
        checker.name_lints(&files);
        let overrides = checker.overrides(&files);
        let mut nfos = checker.nfo_files(&files);
        // A descriptor that is only loose (not in an archive of the mod) is read too.
        let own_nfo = vp(&format!("content/{}.nfo", m.id));
        if let (Text::Found(bytes), _) = checker.read_descriptor(&own_nfo) {
            nfos.push((own_nfo.clone(), bytes));
        }
        checker.references(&nfos, &own_nfo);
        let others: Vec<String> = files
            .keys()
            .filter(|p| {
                p.as_str().strip_prefix("content/").is_some_and(|rest| {
                    !rest.contains('/')
                        && rest
                            .strip_suffix(".mod")
                            .or_else(|| rest.strip_suffix(".nfo"))
                            .is_some_and(|stem| stem != m.id)
                })
            })
            .map(ToString::to_string)
            .collect();
        checker.add_list(
            Lint::ForeignDescriptor,
            "descriptors of another mod id inside this mod's layers",
            &others,
            "Keep only Content/<Mod>.mod and Content/<Mod>.nfo of this mod in its archives.",
        );
        let summary = ModSummary {
            id: m.id.clone(),
            name: display.unwrap_or_else(|| m.id.clone()),
            layers: m.layers.len(),
            files: files.len(),
            overrides,
        };
        per_mod.push((summary, checker.findings));
        tables.push(files);
    }

    for later in 1..mods.len() {
        for earlier in 0..later {
            let found = conflicts(
                &mods[earlier],
                &tables[earlier],
                &mods[later],
                &tables[later],
            );
            if let Some(finding) = found {
                per_mod[later].1.push(finding);
            }
        }
    }

    let mut report = ModReport {
        patch_level: env.patch_level.cloned(),
        mods: Vec::new(),
        findings: Vec::new(),
    };
    for (summary, mut findings) in per_mod {
        findings.sort_by(|a, b| {
            rank(a.severity())
                .cmp(&rank(b.severity()))
                .then(a.code().cmp(b.code()))
                .then(a.path.cmp(&b.path))
                .then(a.message.cmp(&b.message))
        });
        report.mods.push(summary);
        report.findings.extend(findings);
    }
    report
}

fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

fn read_mod_file(m: &ModUnderTest, file: ModFile, path: &VfsPath) -> Option<Vec<u8>> {
    m.layers.get(file.layer)?.vfs.read(path).ok()
}

fn conflicts(
    first: &ModUnderTest,
    first_files: &BTreeMap<VfsPath, ModFile>,
    second: &ModUnderTest,
    second_files: &BTreeMap<VfsPath, ModFile>,
) -> Option<ModFinding> {
    let mut same = 0usize;
    let mut other = 0usize;
    let mut examples: Vec<String> = Vec::new();
    for (path, a) in first_files {
        let Some(b) = second_files.get(path) else {
            continue;
        };
        let identical = a.size == b.size
            && a.size <= MAX_COMPARE_BYTES
            && match (
                read_mod_file(first, *a, path),
                read_mod_file(second, *b, path),
            ) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            };
        if identical {
            same += 1;
        } else {
            other += 1;
            if examples.len() < EXAMPLES {
                examples.push(path.to_string());
            }
        }
    }
    let total = same + other;
    if total == 0 {
        return None;
    }
    let (lint, message, hint, path) = if other > 0 {
        let more = if other > EXAMPLES { ", ..." } else { "" };
        (
            Lint::ModConflict,
            format!(
                "{} replaces {total} files of {} ({other} differ or could not be compared, {same} are identical): {}{more}",
                second.id,
                first.id,
                examples.join(", ")
            ),
            "Only one of the two can win for these files. Enable one mod at a time, or load them in the order you want the files from.",
            examples.first().cloned(),
        )
    } else {
        (
            Lint::ModDuplicate,
            format!(
                "{} and {} provide {total} identical files",
                second.id, first.id
            ),
            "Nothing breaks; one of the copies is redundant.",
            None,
        )
    };
    Some(ModFinding {
        lint,
        mod_id: second.id.clone(),
        path,
        message,
        hint: hint.to_owned(),
    })
}

/// A mod to check in an install folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModSpec {
    /// A mod of the install: the archives of `Content/<id>/`.
    Installed(String),
    /// A loose folder that holds a mod's files as `Content/...` below it (a mod being made).
    Folder(PathBuf),
}

/// The names of the folders `Content/<name>/` of the install that hold at least one `.gro`
/// archive: the mods that can be mounted. Sorted by lower-case name; empty when there is no
/// `Content` folder.
pub fn installed_mod_names(install_dir: impl AsRef<Path>) -> Vec<String> {
    let content = install_dir.as_ref().join("Content");
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&content) {
        for entry in entries.flatten() {
            let path = entry.path();
            // `read_dir` fails on a plain file, so only folders get through.
            let has_gro = std::fs::read_dir(&path).is_ok_and(|it| {
                it.flatten().any(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("gro"))
                })
            });
            if has_gro {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names
}

fn install_finding(
    lint: Lint,
    mod_id: &str,
    path: Option<String>,
    message: String,
    hint: &str,
) -> ModFinding {
    ModFinding {
        lint,
        mod_id: mod_id.to_lowercase(),
        path,
        message,
        hint: hint.to_owned(),
    }
}

fn newest_patch(dir: &Path) -> Option<PatchLevel> {
    // The newest by number; two names with the same numbers (`Patch_02_100.gro` and
    // `Patch_02_100_0.gro`) are told apart by name, so the answer never depends on the order
    // the folder is read in.
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| PatchLevel::from_file_name(&e.file_name().to_string_lossy()))
        .max_by(|a, b| {
            a.key()
                .cmp(&b.key())
                .then_with(|| a.archive.cmp(&b.archive))
        })
}

fn load_installed(dir: &Path, id: &str, findings: &mut Vec<ModFinding>) -> Option<ModUnderTest> {
    let names = match mod_archive_names(dir, id) {
        Ok(names) => names,
        Err(error) => {
            findings.push(install_finding(
                Lint::ModNotFound,
                id,
                None,
                format!("the mod `{id}` cannot be found in the install ({error})"),
                "Check the spelling: the mod's archives must be in Content/<Mod>/ of the install folder.",
            ));
            return None;
        }
    };
    let folder = dir.join("Content");
    let mut m = ModUnderTest::new(id);
    // The folder name as it is on disk (found without regard to case by `mod_archive_names`).
    let on_disk = std::fs::read_dir(&folder).ok().and_then(|it| {
        it.flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(id) && e.path().is_dir())
            .map(|e| e.path())
    })?;
    for name in names {
        match ArchiveVfs::from_file(on_disk.join(&name)) {
            Ok(archive) => {
                let only_dummy = archive.file_count() == 1
                    && archive.exists(&vp("dummy.txt"));
                if only_dummy || archive.file_count() == 0 {
                    m = m.with_placeholder(name);
                } else {
                    m = m.with_layer(ModLayer::from_archive(name, archive));
                }
            }
            Err(error) => findings.push(install_finding(
                Lint::ArchiveUnreadable,
                id,
                Some(name.clone()),
                format!("the archive {name} of the mod cannot be opened ({error})"),
                "Copy the file again from the original download; the rest of the mod is still checked.",
            )),
        }
    }
    Some(m)
}

fn load_folder(path: &Path, findings: &mut Vec<ModFinding>) -> Option<ModUnderTest> {
    let label = path.display().to_string();
    let vfs = match DirectoryVfs::new(path) {
        Ok(vfs) => vfs,
        Err(error) => {
            findings.push(install_finding(
                Lint::ModNotFound,
                "",
                Some(label.clone()),
                format!("the mod folder {label} cannot be read ({error})"),
                "Give a folder that holds Content/<Mod>.mod and the mod's files below Content/.",
            ));
            return None;
        }
    };
    let folder_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| label.clone());
    let id = vfs
        .list(&vp("content"))
        .ok()
        .and_then(|entries| {
            entries
                .into_iter()
                .find_map(|e| e.name.strip_suffix(".mod").map(str::to_owned))
        })
        .unwrap_or_else(|| folder_name.to_lowercase());
    Some(ModUnderTest::new(id).with_layer(ModLayer::new(folder_name, vfs)))
}

/// Check mods in the install folder `dir`: mount the base archives and patches, open every mod
/// as layers and run [`check_mods`]. A mod or archive that cannot be opened becomes a finding
/// (`MOD090` to `MOD092`) and the others are still checked.
pub fn check_install_mods(dir: impl AsRef<Path>, specs: &[ModSpec]) -> ModReport {
    let dir = dir.as_ref();
    let base: LayeredVfs = match mount_install(dir) {
        Ok(base) => base,
        Err(error) => {
            return ModReport {
                patch_level: None,
                mods: Vec::new(),
                findings: vec![install_finding(
                    Lint::InstallUnreadable,
                    "",
                    Some(dir.display().to_string()),
                    format!("the install cannot be read ({error})"),
                    "Run ss2-doctor on the folder without --mod to see what is wrong with the install.",
                )],
            };
        }
    };
    let loose = DirectoryVfs::new(dir).ok();
    let patch_level = newest_patch(dir);
    let mut load_findings = Vec::new();
    let mut mods = Vec::new();
    for spec in specs {
        let loaded = match spec {
            ModSpec::Installed(id) => load_installed(dir, id, &mut load_findings),
            ModSpec::Folder(path) => load_folder(path, &mut load_findings),
        };
        mods.extend(loaded);
    }
    let env = ModCheckEnv {
        base: &base,
        loose: loose.as_ref().map(|l| l as &dyn Vfs),
        patch_level: patch_level.as_ref(),
    };
    let mut report = check_mods(&env, &mods);
    load_findings.append(&mut report.findings);
    // Keep the order: findings of a mod stay grouped by mod in `render`; the install-level ones
    // (no mod in the summary) come last there.
    report.findings = load_findings;
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryVfs;
    use ss2_core::Result;

    fn mem(files: &[(&str, &str)]) -> MemoryVfs {
        let mut v = MemoryVfs::new();
        for (n, d) in files {
            v.insert(n, d.as_bytes().to_vec()).unwrap();
        }
        v
    }

    const MOD: &str = "MODULES = Sam2Game <TOOLS>Sam2GameGui ;";
    const NFO: &str =
        "NAME = \"Test Mod\" ;\nAUTHOR = \"Me\" ;\nTHUMBNAIL = \"Content/TestMod/t.tex\" ;";

    fn good_files() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Content/TestMod.mod", MOD),
            ("Content/TestMod.nfo", NFO),
            ("Content/TestMod/t.tex", "pixels"),
        ]
    }

    fn base() -> MemoryVfs {
        mem(&[
            ("Content/SeriousSam2/Models/a.mdl", "m"),
            ("Content/SeriousSam2/Textures/b.tex", "t"),
            ("Content/SeriousSam2/Movies/Ntsc/i.ogv", "v"),
            ("Content/SeriousSam2/Databases/x.ep", "e"),
        ])
    }

    fn run_one(base: &MemoryVfs, files: &[(&str, &str)]) -> ModReport {
        let patch = PatchLevel::from_file_name("Patch_02_100_800000.gro");
        let env = ModCheckEnv {
            base,
            loose: None,
            patch_level: patch.as_ref(),
        };
        let m = ModUnderTest::new("TestMod").with_layer(ModLayer::new("TestMod.gro", mem(files)));
        check_mods(&env, &[m])
    }

    fn codes(report: &ModReport) -> Vec<&'static str> {
        report.findings.iter().map(ModFinding::code).collect()
    }

    fn with<'a>(extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        let mut files = good_files();
        files.extend_from_slice(extra);
        files
    }

    fn only(report: &ModReport, lint: Lint) -> Vec<&ModFinding> {
        report.findings.iter().filter(|f| f.lint == lint).collect()
    }

    #[test]
    fn a_clean_mod_has_no_findings_and_exit_code_zero() {
        let r = run_one(&base(), &good_files());
        assert!(r.findings.is_empty(), "{:#?}", r.findings);
        assert_eq!(r.exit_code(), 0);
        assert_eq!(r.mods[0].name, "Test Mod");
        assert_eq!(r.mods[0].files, 3);
        assert_eq!(r.mods[0].overrides, 0);
        assert!(r.render().contains("no findings"));
        assert!(r.render().contains("No problems found (0 notes)."));
    }

    #[test]
    fn lint_codes_are_unique_stable_and_in_order() {
        let codes: Vec<&str> = Lint::ALL.iter().map(|l| l.code()).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(codes.len(), sorted.len(), "codes are unique");
        assert_eq!(codes, {
            let mut c = codes.clone();
            c.sort_unstable();
            c
        });
        assert!(codes.iter().all(|c| c.len() == 6 && c.starts_with("MOD")));
        assert_eq!(Lint::ALL.len(), 37);
        assert_eq!(Lint::MissingModel.code(), "MOD030");
        assert_eq!(Lint::MissingModel.severity(), Severity::Error);
        assert_eq!(Lint::MissingResource.severity(), Severity::Warning);
        assert_eq!(Lint::OverrideSummary.severity(), Severity::Info);
    }

    #[test]
    fn the_docs_list_every_code() {
        let docs = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/MOD_VALIDATION.md"),
        )
        .expect("docs/MOD_VALIDATION.md exists");
        for lint in Lint::ALL {
            assert!(
                docs.contains(lint.code()),
                "{} is not documented",
                lint.code()
            );
        }
        assert!(docs.contains("MOD092"));
    }

    #[test]
    fn missing_mod_file_is_an_error_and_a_broken_one_too() {
        let r = run_one(
            &base(),
            &[("Content/TestMod.nfo", NFO), ("Content/TestMod/t.tex", "x")],
        );
        assert_eq!(codes(&r), ["MOD001"]);
        assert_eq!(r.exit_code(), 1);
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", "MODULES = Sam2Game"),
                ("Content/TestMod.nfo", NFO),
                ("Content/TestMod/t.tex", "x"),
            ],
        );
        let f = &r.findings[0];
        assert_eq!(f.code(), "MOD002");
        assert!(f.message.contains("not closed"), "{}", f.message);
        assert_eq!(r.exit_code(), 1);
    }

    #[test]
    fn nfo_problems_are_warnings_with_their_own_codes() {
        let r = run_one(
            &base(),
            &[("Content/TestMod.mod", MOD), ("Content/TestMod/t.tex", "x")],
        );
        assert_eq!(codes(&r), ["MOD003"]);
        assert_eq!(r.exit_code(), 0);
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", MOD),
                ("Content/TestMod.nfo", "NAME \"x\""),
                ("Content/TestMod/t.tex", "x"),
            ],
        );
        assert_eq!(codes(&r), ["MOD004"]);
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", MOD),
                ("Content/TestMod.nfo", "NAME = \"  \" ;\nAUTHOR = \"\" ;"),
                ("Content/TestMod/t.tex", "x"),
            ],
        );
        assert_eq!(codes(&r), ["MOD005", "MOD006"]);
        assert_eq!(r.findings[1].severity(), Severity::Info);
        assert_eq!(r.mods[0].name, "testmod", "no NAME: the id is shown");
    }

    #[test]
    fn a_descriptor_next_to_the_archive_counts_and_a_differing_copy_is_reported() {
        let b = base();
        let loose = mem(&[("Content/TestMod.mod", MOD), ("Content/TestMod.nfo", NFO)]);
        let patch = PatchLevel::from_file_name("Patch_02_100.gro");
        let env = ModCheckEnv {
            base: &b,
            loose: Some(&loose),
            patch_level: patch.as_ref(),
        };
        let m = ModUnderTest::new("testmod").with_layer(ModLayer::new(
            "a.gro",
            mem(&[("Content/TestMod/t.tex", "x")]),
        ));
        let r = check_mods(&env, &[m]);
        assert!(r.findings.is_empty(), "{:#?}", r.findings);
        // The copy inside the archive differs from the loose one.
        let m = ModUnderTest::new("testmod").with_layer(ModLayer::new(
            "a.gro",
            mem(&[
                ("Content/TestMod/t.tex", "x"),
                ("Content/TestMod.mod", "MODULES = Sam2Game ;"),
                (
                    "Content/TestMod.nfo",
                    "NAME = \"Other\" ;\nAUTHOR = \"Me\" ;",
                ),
            ]),
        ));
        let r = check_mods(&env, &[m]);
        assert_eq!(codes(&r), ["MOD008", "MOD008"]);
        assert!(
            r.findings
                .iter()
                .any(|f| f.path.as_deref() == Some("content/testmod.mod"))
        );
        assert!(
            r.findings
                .iter()
                .any(|f| f.path.as_deref() == Some("content/testmod.nfo"))
        );
        assert_eq!(
            r.mods[0].name, "Other",
            "the archive's copy is the one in effect"
        );
    }

    #[test]
    fn identical_copies_of_the_descriptor_are_fine() {
        let b = base();
        let loose = mem(&[("Content/TestMod.mod", MOD), ("Content/TestMod.nfo", NFO)]);
        let env = ModCheckEnv {
            base: &b,
            loose: Some(&loose),
            patch_level: None,
        };
        let m = ModUnderTest::new("testmod").with_layer(ModLayer::new("a.gro", mem(&good_files())));
        let r = check_mods(&env, &[m]);
        assert!(r.findings.is_empty(), "{:#?}", r.findings);
    }

    #[test]
    fn a_foreign_descriptor_in_the_layers_is_reported() {
        let r = run_one(
            &base(),
            &with(&[("Content/Other.mod", MOD), ("Content/other.nfo", NFO)]),
        );
        let f = only(&r, Lint::ForeignDescriptor);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("(2)"), "{}", f[0].message);
    }

    #[test]
    fn a_custom_game_module_is_a_warning_and_a_missing_dll_another() {
        let b = mem(&[
            ("Bin/Sam2Game.dll", "x"),
            ("Content/SeriousSam2/Models/a.mdl", "m"),
        ]);
        let files = [
            (
                "Content/TestMod.mod",
                "MODULES = TestModGame <TOOLS>Sam2GameGui ;",
            ),
            ("Content/TestMod.nfo", NFO),
            ("Content/TestMod/t.tex", "x"),
        ];
        let r = run_one(&b, &files);
        assert_eq!(codes(&r), ["MOD023", "MOD024"]);
        assert_eq!(r.exit_code(), 0, "warnings only");
        assert!(r.findings[0].message.contains("TestModGame"));
        // The DLL is present: only the native-module warning remains.
        let b = mem(&[("Bin/TestModGame.dll", "x")]);
        assert_eq!(codes(&run_one(&b, &files)), ["MOD023"]);
        // No Bin folder known at all: the file cannot be judged.
        assert_eq!(codes(&run_one(&base(), &files)), ["MOD023"]);
        // The base module name, in any case, is fine.
        let ok = [
            ("Content/TestMod.mod", "MODULES = sam2game ;"),
            ("Content/TestMod.nfo", NFO),
            ("Content/TestMod/t.tex", "x"),
        ];
        assert!(run_one(&b, &ok).findings.is_empty());
    }

    #[test]
    fn requires_patch_is_compared_with_the_install() {
        let nfo = |v: &str| format!("NAME = \"T\" ;\nAUTHOR = \"A\" ;\nREQUIRES_PATCH = \"{v}\" ;");
        let go = |v: &str| {
            let n = nfo(v);
            let files = [
                ("Content/TestMod.mod", MOD),
                ("Content/TestMod.nfo", n.as_str()),
                ("Content/TestMod/t.tex", "x"),
            ];
            let b = base();
            let patch = PatchLevel::from_file_name("Patch_02_090_593613.gro");
            let env = ModCheckEnv {
                base: &b,
                loose: None,
                patch_level: patch.as_ref(),
            };
            let m = ModUnderTest::new("testmod").with_layer(ModLayer::new("a.gro", mem(&files)));
            check_mods(&env, &[m])
        };
        assert!(go("2.090").findings.is_empty(), "equal is fine");
        assert!(go("2.068").findings.is_empty(), "older is fine");
        assert!(go("2").findings.is_empty());
        assert!(go("2.090.593613").findings.is_empty(), "equal build");
        let too_new = go("2.100");
        assert_eq!(codes(&too_new), ["MOD021"]);
        assert_eq!(too_new.exit_code(), 1);
        assert!(too_new.findings[0].message.contains("2.100"));
        assert!(too_new.findings[0].message.contains("2.090 (build 593613)"));
        assert_eq!(codes(&go("2.090.593614")), ["MOD021"], "build is compared");
        assert_eq!(codes(&go("3")), ["MOD021"]);
        for bad in ["abc", "", "2.", "2..1", "2.1.2.3", "-1", "2.x"] {
            assert_eq!(codes(&go(bad)), ["MOD020"], "{bad:?}");
        }
        // No patch at all.
        let n = nfo("2.068");
        let files = [
            ("Content/TestMod.mod", MOD),
            ("Content/TestMod.nfo", n.as_str()),
            ("Content/TestMod/t.tex", "x"),
        ];
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let m = ModUnderTest::new("testmod").with_layer(ModLayer::new("a.gro", mem(&files)));
        let r = check_mods(&env, &[m]);
        assert_eq!(codes(&r), ["MOD021"]);
        assert!(r.findings[0].message.contains("no patch"));
    }

    #[test]
    fn requirement_parsing() {
        assert_eq!(
            parse_patch_requirement("2.100.800000"),
            Some((2, 100, 800_000))
        );
        assert_eq!(parse_patch_requirement(" 2.068 "), Some((2, 68, 0)));
        assert_eq!(parse_patch_requirement("2"), Some((2, 0, 0)));
        assert_eq!(parse_patch_requirement("2.1.3.4"), None);
        assert_eq!(parse_patch_requirement("99999999999.1"), None);
        assert_eq!(parse_patch_requirement(""), None);
    }

    #[test]
    fn update_archive_names_are_read_as_a_patch_minor() {
        assert_eq!(
            update_patch_minor("z_InSamnity2Update_02_090.gro"),
            Some(90)
        );
        assert_eq!(
            update_patch_minor("z_InSamnity2Update_12_100.gro"),
            Some(100)
        );
        assert_eq!(update_patch_minor("z_InSamnity2Update_08.gro"), None);
        assert_eq!(update_patch_minor("Mod.gro"), None);
        assert_eq!(
            update_patch_minor("a_1_2.gro"),
            None,
            "minor needs three digits"
        );
        assert_eq!(update_patch_minor("_02_090.gro"), Some(90));
        assert_eq!(update_patch_minor("x_ab_090.gro"), None);
        assert_eq!(
            update_patch_minor("02_090.gro"),
            None,
            "needs a name part too"
        );
    }

    #[test]
    fn an_update_pack_for_a_newer_patch_is_a_warning() {
        let b = base();
        let patch = PatchLevel::from_file_name("Patch_02_068.gro");
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: patch.as_ref(),
        };
        let m = ModUnderTest::new("testmod")
            .with_layer(ModLayer::new("a.gro", mem(&good_files())))
            .with_layer(ModLayer::new(
                "z_TestModUpdate_12_100.gro",
                mem(&[("Content/TestMod/u.txt", "u")]),
            ))
            .with_layer(ModLayer::new(
                "z_TestModUpdate_02_068.gro",
                mem(&[("Content/TestMod/v.txt", "v")]),
            ));
        let r = check_mods(&env, &[m]);
        assert_eq!(codes(&r), ["MOD022"]);
        assert!(r.findings[0].message.contains("2.100"));
        assert!(r.findings[0].message.contains("inferred"));
        assert_eq!(r.exit_code(), 0);
        // Placeholders are looked at too; with no patch at all every minor is too new.
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let m = ModUnderTest::new("testmod")
            .with_layer(ModLayer::new("a.gro", mem(&good_files())))
            .with_placeholder("z_TestModUpdate_02_090.gro");
        let r = check_mods(&env, &[m]);
        assert!(codes(&r).contains(&"MOD022"), "{:?}", codes(&r));
    }

    #[test]
    fn missing_assets_are_found_in_the_mod_and_the_base_and_nowhere() {
        let nfo = "NAME = \"T\" ;\nAUTHOR = \"A\" ;\nMODEL = \"Content/TestMod/gone.mdl\" ;\nTHUMBNAIL = \"Content/SeriousSam2/Textures/missing.tex\" ;";
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", MOD),
                ("Content/TestMod.nfo", NFO),
                ("Content/TestMod/Models/p.nfo", nfo),
                ("Content/TestMod/t.tex", "x"),
            ],
        );
        assert_eq!(codes(&r), ["MOD030", "MOD031"]);
        assert_eq!(r.findings[0].severity(), Severity::Error);
        assert_eq!(r.findings[1].severity(), Severity::Warning);
        assert!(r.findings[0].message.contains("gone.mdl"));
        assert_eq!(
            r.findings[0].path.as_deref(),
            Some("content/testmod/models/p.nfo")
        );
        assert_eq!(r.exit_code(), 1);
        // Present in the mod, present in the base: nothing.
        let nfo = "NAME = \"T\" ;\nMODEL = \"content/testmod/own.MDL\" ;\nTHUMBNAIL = \"Content\\SeriousSam2\\Textures\\b.tex\" ;\nLOADING_MODEL=\"Content/SeriousSam2/Models/a.mdl\"\nINTRO_MOVIE=\"Content/SeriousSam2/Movies/Ntsc/i.ogv\"";
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", MOD),
                ("Content/TestMod.nfo", NFO),
                ("Content/TestMod/Models/p.nfo", nfo),
                ("Content/TestMod/own.mdl", "m"),
                ("Content/TestMod/t.tex", "x"),
            ],
        );
        assert!(r.findings.is_empty(), "{:#?}", r.findings);
    }

    #[test]
    fn every_reference_key_is_checked() {
        for key in [
            "MODEL",
            "LOADING_MODEL",
            "THUMBNAIL",
            "LOADING_SCREEN",
            "INTRO_MOVIE",
            "OUTRO_MOVIE",
            "EPISODE_MOVIE",
        ] {
            let nfo = format!("NAME = \"T\" ;\n{key} = \"Content/nowhere/x.bin\" ;");
            let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo.as_str())]));
            let miss: Vec<_> = r
                .findings
                .iter()
                .filter(|f| f.code() == "MOD030" || f.code() == "MOD031")
                .collect();
            assert_eq!(miss.len(), 1, "{key}: {:#?}", r.findings);
            assert!(miss[0].message.contains(key), "{}", miss[0].message);
            let want = if key == "MODEL" { "MOD030" } else { "MOD031" };
            assert_eq!(miss[0].code(), want, "{key}");
        }
        // Keys compare without regard to case; unknown keys are ignored.
        let nfo = "name = \"T\" ;\nmodel = \"x/y.mdl\" ;\nDESCRIPTION = \"Content/nowhere.txt\" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD030"]);
    }

    #[test]
    fn a_reference_to_a_folder_is_not_a_file() {
        let nfo = "NAME = \"T\" ;\nMODEL = \"Content/SeriousSam2/Models\" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD030"]);
    }

    #[test]
    fn references_that_escape_are_errors() {
        for (value, why) in [
            ("../x.mdl", "climbs"),
            ("Content/../../x.mdl", "climbs"),
            ("C:/x.mdl", "drive"),
            ("/etc/x.mdl", "absolute"),
            ("\\\\server\\x.mdl", "absolute"),
            ("http://a/x.mdl", "drive"),
            ("Content/a\u{1}b.mdl", "control"),
            ("Content\\..\\x.mdl", "climbs"),
        ] {
            let nfo = format!("NAME = \"T\" ;\nMODEL = \"{value}\" ;");
            let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo.as_str())]));
            assert_eq!(codes(&r), ["MOD032"], "{value:?}: {:#?}", r.findings);
            assert!(
                r.findings[0].message.contains(why),
                "{}",
                r.findings[0].message
            );
            assert_eq!(r.exit_code(), 1);
        }
    }

    #[test]
    fn mixed_separators_empty_values_and_wrong_types_are_warnings() {
        let nfo = "NAME = \"T\" ;\nMODEL = \"Content/SeriousSam2\\Models/a.mdl\" ;\nTHUMBNAIL = \"  \" ;\nLOADING_SCREEN = \"Content/SeriousSam2/Models/a.mdl\" ;\nINTRO_MOVIE = \"\" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD033", "MOD034", "MOD034", "MOD036"]);
        assert!(r.findings.iter().all(|f| f.severity() == Severity::Warning));
        assert_eq!(r.exit_code(), 0);
        let wrong = r.findings.iter().find(|f| f.code() == "MOD036").unwrap();
        assert!(wrong.message.contains(".tex"), "{}", wrong.message);
        // Pure backslashes are fine: the game treats them like slashes.
        let nfo = "NAME = \"T\" ;\nMODEL = \"Content\\SeriousSam2\\Models\\a.mdl\" ;";
        assert!(
            run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]))
                .findings
                .is_empty()
        );
        // A missing file does not also complain about its type.
        let nfo = "NAME = \"T\" ;\nTHUMBNAIL = \"Content/x/missing.png\" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD031"]);
        // A reference without any extension is of the wrong type too.
        let nfo = "NAME = \"T\" ;\nMODEL = \"Content/SeriousSam2/Models\" ;";
        let b = mem(&[("Content/SeriousSam2/Models", "file")]);
        let r = run_one(&b, &with(&[("Content/TestMod/p.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD036"]);
    }

    #[test]
    fn a_quoted_value_with_a_trailing_space_is_shown_exactly() {
        let nfo = "NAME = \"T\" ;\nMODEL = \"Content/SeriousSam2/Models/a.mdl \" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD030"]);
        assert!(
            r.findings[0].message.contains("a.mdl \""),
            "{}",
            r.findings[0].message
        );
    }

    #[test]
    fn a_level_needs_its_world_file() {
        let nfo = "LEVEL\nNAME = \"L\" ;\nMAP_TYPE = \"SINGLE\" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/Levels/a/a.nfo", nfo)]));
        assert_eq!(codes(&r), ["MOD035"]);
        assert!(r.findings[0].message.contains("a.wld"));
        let ok = run_one(
            &base(),
            &with(&[
                ("Content/TestMod/Levels/a/a.nfo", nfo),
                ("Content/TestMod/Levels/a/a.wld", "w"),
            ]),
        );
        assert!(ok.findings.is_empty(), "{:#?}", ok.findings);
        // A world in the base game is fine too, and a kind word in any case.
        let b = mem(&[("Content/TestMod/Levels/a/a.wld", "w")]);
        let nfo2 = "level\nNAME = \"L\" ;";
        assert!(
            run_one(&b, &with(&[("Content/TestMod/Levels/a/a.nfo", nfo2)]))
                .findings
                .is_empty()
        );
        // Other kinds do not need a world.
        let ep = "EPISODE\nNAME = \"E\" ;";
        assert!(
            run_one(&base(), &with(&[("Content/TestMod/e.nfo", ep)]))
                .findings
                .is_empty()
        );
    }

    #[test]
    fn a_broken_nfo_inside_the_mod_is_reported_with_its_path() {
        let r = run_one(
            &base(),
            &with(&[("Content/TestMod/Models/bad.nfo", "NAME = \"open")]),
        );
        assert_eq!(codes(&r), ["MOD004"]);
        assert_eq!(
            r.findings[0].path.as_deref(),
            Some("content/testmod/models/bad.nfo")
        );
        let r = run_one(
            &base(),
            &with(&[("Content/TestMod/Models/bin.nfo", "KIND\nKEY")]),
        );
        assert_eq!(codes(&r), ["MOD004"]);
    }

    #[test]
    fn overrides_are_counted_and_critical_ones_warned_about() {
        let r = run_one(
            &base(),
            &with(&[
                ("Content/SeriousSam2/Models/a.mdl", "new"),
                ("Content/SeriousSam2/Databases/x.ep", "new"),
                ("Content/SeriousSam2/Textures/new.tex", "n"),
                ("Content/DefaultMod.txt", "CURRENTMOD = TestMod.mod ;"),
            ]),
        );
        assert_eq!(codes(&r), ["MOD051", "MOD050"]);
        let info = &r.findings[1];
        assert_eq!(info.severity(), Severity::Info);
        assert!(
            info.message
                .starts_with("2 of 7 files replace base game files, 5 are new"),
            "{}",
            info.message
        );
        assert!(info.message.contains(".ep 1") && info.message.contains(".mdl 1"));
        assert!(
            r.findings[0].message.contains(".ep (entity parameters)"),
            "{}",
            r.findings[0].message
        );
        assert_eq!(r.mods[0].overrides, 2);
        // The mod selection file is critical by path.
        let b = mem(&[("Content/DefaultMod.txt", "CURRENTMOD = SeriousSam2.mod ;")]);
        let r = run_one(
            &b,
            &with(&[("Content/DefaultMod.txt", "CURRENTMOD = TestMod.mod ;")]),
        );
        assert!(codes(&r).contains(&"MOD051"));
        assert_eq!(
            only(&r, Lint::CriticalOverride)[0].path.as_deref(),
            Some("content/defaultmod.txt")
        );
        // Levels and databases count as critical, models and textures do not.
        let b = mem(&[
            ("Content/l.wld", "w"),
            ("Content/r.rsc", "r"),
            ("Content/m.mdl", "m"),
            ("Content/t.tex", "t"),
        ]);
        let r = run_one(
            &b,
            &with(&[
                ("Content/l.wld", "x"),
                ("Content/r.rsc", "x"),
                ("Content/m.mdl", "x"),
                ("Content/t.tex", "x"),
            ]),
        );
        assert_eq!(only(&r, Lint::CriticalOverride).len(), 2);
    }

    #[test]
    fn each_file_that_describes_the_game_is_critical_by_path() {
        for path in [
            "Content/DefaultMod.txt",
            "Content/CurrentMod.txt",
            "Content/SeriousSam2.mod",
            "Content/SeriousSam2.nfo",
        ] {
            let b = mem(&[(path, "base")]);
            let r = run_one(&b, &with(&[(path, "mod")]));
            let f = only(&r, Lint::CriticalOverride);
            assert_eq!(f.len(), 1, "{path}: {:#?}", r.findings);
            assert_eq!(f[0].path.as_deref(), Some(path.to_lowercase().as_str()));
            assert!(
                f[0].message.contains("describe the game itself"),
                "{}",
                f[0].message
            );
        }
        // Not replaced: not reported, even when the base has them.
        let b = mem(&[("Content/DefaultMod.txt", "base")]);
        assert!(only(&run_one(&b, &good_files()), Lint::CriticalOverride).is_empty());
    }

    #[test]
    fn the_override_summary_cuts_after_exactly_eight_types() {
        let mk = |n: usize| {
            let exts = ["aa", "bb", "cc", "dd", "ee", "ff", "gg", "hh", "ii"];
            let mut b = MemoryVfs::new();
            let mut files: Vec<(String, &'static str)> = good_files()
                .iter()
                .map(|(a, d)| ((*a).to_owned(), *d))
                .collect();
            for ext in &exts[..n] {
                b.insert(&format!("Content/x.{ext}"), vec![1]).unwrap();
                files.push((format!("Content/x.{ext}"), "y"));
            }
            let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), *b)).collect();
            run_one(&b, &refs)
        };
        let r = mk(8);
        let msg = &only(&r, Lint::OverrideSummary)[0].message;
        assert!(
            msg.ends_with(".hh 1)"),
            "exactly eight types: no cut: {msg}"
        );
        let r = mk(9);
        let msg = &only(&r, Lint::OverrideSummary)[0].message;
        assert!(msg.contains(".hh 1") && msg.ends_with(", ...)"), "{msg}");
    }

    #[test]
    fn lists_fold_after_exactly_three_examples() {
        let named = |n: usize| {
            let owned: Vec<String> = (0..n)
                .map(|i| format!("Content/TestMod/n{i}?.tex"))
                .collect();
            let mut files: Vec<(&str, &str)> = good_files();
            for name in &owned {
                files.push((name.as_str(), "x"));
            }
            run_one(&base(), &files)
        };
        let three = named(3);
        let m = &only(&three, Lint::WindowsUnsafeName)[0].message;
        assert!(m.contains("(3)") && !m.contains("..."), "{m}");
        let four = named(4);
        let m = &only(&four, Lint::WindowsUnsafeName)[0].message;
        assert!(m.contains("(4)") && m.ends_with(", ..."), "{m}");
    }

    #[test]
    fn conflict_messages_name_at_most_three_examples() {
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let pair = |n: usize| {
            let mk = |id: &str, tag: &str| {
                let mut v = MemoryVfs::new();
                for i in 0..n {
                    v.insert(
                        &format!("Content/f{i}.bin"),
                        format!("{id}{tag}").into_bytes(),
                    )
                    .unwrap();
                }
                ModUnderTest::new(id).with_layer(ModLayer::new(format!("{id}.gro"), v))
            };
            check_mods(&env, &[mk("a", "1"), mk("b", "2")])
        };
        let r = pair(3);
        let m = &only(&r, Lint::ModConflict)[0].message;
        assert!(
            m.contains("f0.bin, content/f1.bin, content/f2.bin") && !m.contains("..."),
            "{m}"
        );
        let r = pair(5);
        let m = &only(&r, Lint::ModConflict)[0].message;
        assert!(m.contains("replaces 5 files of a"), "{m}");
        assert!(m.ends_with("content/f2.bin, ..."), "{m}");
        assert!(!m.contains("f3.bin"), "{m}");
    }

    #[test]
    fn foreign_descriptors_are_only_mod_and_nfo_files_at_the_content_root() {
        let r = run_one(
            &base(),
            &with(&[
                ("Content/readme.txt", "x"),
                ("Content/Other.nfo", "NAME = \"o\""),
                ("Content/Sub/Other2.nfo", "NAME = \"o\""),
                ("readme.md", "x"),
            ]),
        );
        let f = only(&r, Lint::ForeignDescriptor);
        assert_eq!(f.len(), 1, "{:#?}", r.findings);
        assert!(
            f[0].message.contains("(1): content/other.nfo"),
            "{}",
            f[0].message
        );
    }

    #[test]
    fn requires_patch_with_a_build_names_the_build_and_updates_need_a_patch_minor() {
        let nfo = "NAME = \"T\" ;\nAUTHOR = \"A\" ;\nREQUIRES_PATCH = \"2.095.5\" ;";
        let files = [
            ("Content/TestMod.mod", MOD),
            ("Content/TestMod.nfo", nfo),
            ("Content/TestMod/t.tex", "x"),
        ];
        let r = run_one(&base(), &files);
        let m = &only(&r, Lint::PatchTooOld);
        assert_eq!(m.len(), 0, "install 2.100 is newer: {:#?}", r.findings);
        let b = base();
        let patch = PatchLevel::from_file_name("Patch_02_090.gro");
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: patch.as_ref(),
        };
        let m2 = ModUnderTest::new("testmod").with_layer(ModLayer::new("a.gro", mem(&files)));
        let r = check_mods(&env, &[m2]);
        assert!(
            r.findings[0]
                .message
                .contains("needs patch level 2.095 (build 5)"),
            "{}",
            r.findings[0].message
        );
        // With no patch at all, even the lowest update minor is too new.
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let m3 = ModUnderTest::new("testmod")
            .with_layer(ModLayer::new("a.gro", mem(&good_files())))
            .with_layer(ModLayer::new(
                "z_Up_01_001.gro",
                mem(&[("Content/TestMod/u.txt", "u")]),
            ));
        let r = check_mods(&env, &[m3]);
        assert_eq!(codes(&r), ["MOD022"], "{:#?}", r.findings);
        // Equal to the install's minor is fine.
        let patch = PatchLevel::from_file_name("Patch_02_001.gro");
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: patch.as_ref(),
        };
        let m4 = ModUnderTest::new("testmod")
            .with_layer(ModLayer::new("a.gro", mem(&good_files())))
            .with_layer(ModLayer::new(
                "z_Up_01_001.gro",
                mem(&[("Content/TestMod/u.txt", "u")]),
            ));
        assert!(check_mods(&env, &[m4]).findings.is_empty());
    }

    #[test]
    fn unreadable_folders_come_with_the_matching_hint() {
        let mut layer = flaky(&good_files());
        layer.fail_list = Some(("content/testmod", true));
        let r = run_layer(layer);
        assert!(
            only(&r, Lint::CaseCollision)[0]
                .hint
                .contains("differ only by case")
        );
        let mut layer = flaky(&good_files());
        layer.fail_list = Some(("content/testmod", false));
        let r = run_layer(layer);
        assert!(only(&r, Lint::Unreadable)[0].hint.contains("intact"));
    }

    #[test]
    fn the_override_summary_lists_the_biggest_types_first_and_cuts_the_rest() {
        let mut b = MemoryVfs::new();
        let mut files: Vec<(String, &'static str)> = vec![
            ("Content/TestMod.mod".into(), MOD),
            ("Content/TestMod.nfo".into(), NFO),
            ("Content/TestMod/t.tex".into(), "x"),
        ];
        for ext in ["aa", "bb", "cc", "dd", "ee", "ff", "gg", "hh", "ii"] {
            b.insert(&format!("Content/x.{ext}"), vec![1]).unwrap();
            files.push((format!("Content/x.{ext}"), "y"));
        }
        b.insert("Content/y.aa", vec![1]).unwrap();
        files.push(("Content/y.aa".into(), "y"));
        let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), *b)).collect();
        let r = run_one(&b, &refs);
        let msg = &only(&r, Lint::OverrideSummary)[0].message;
        assert!(
            msg.contains("(replaced by type: .aa 2, .bb 1, .cc 1,"),
            "{msg}"
        );
        assert!(msg.ends_with(", ...)"), "{msg}");
        assert!(!msg.contains(".ii"), "{msg}");
    }

    #[test]
    fn no_override_means_no_summary_note() {
        let r = run_one(&base(), &good_files());
        assert!(only(&r, Lint::OverrideSummary).is_empty());
    }

    #[test]
    fn unsafe_file_names_outside_content_and_native_code_are_reported() {
        let r = run_one(
            &base(),
            &with(&[
                ("Content/TestMod/con.tex", "x"),
                ("Content/TestMod/a?b.tex", "x"),
                ("Content/TestMod/trailing.", "x"),
                ("Content/TestMod/COM1.txt", "x"),
                ("Content/TestMod/ok1.tex", "x"),
                ("Content/TestMod/com0.tex", "x"),
                ("Content/TestMod/lpt9.tex", "x"),
                ("readme.txt", "x"),
                ("dummy.txt", ""),
                ("Bin/TestModGame.dll", "x"),
                ("Content/TestMod/x.EXE", "x"),
                ("Content/TestMod/more.gro", "x"),
            ]),
        );
        let unsafe_names = only(&r, Lint::WindowsUnsafeName);
        assert_eq!(unsafe_names.len(), 1);
        assert!(
            unsafe_names[0].message.contains("(5)"),
            "{}",
            unsafe_names[0].message
        );
        let outside = only(&r, Lint::OutsideContent);
        assert_eq!(outside.len(), 1);
        assert!(outside[0].message.contains("(2)"), "{}", outside[0].message);
        assert_eq!(outside[0].severity(), Severity::Info);
        let native = only(&r, Lint::NativeCodeInside);
        assert_eq!(
            native.len(),
            3,
            "dll, exe and gro: one finding each: {native:#?}"
        );
        assert!(native.iter().all(|f| f.severity() == Severity::Warning));
    }

    #[test]
    fn long_lists_name_three_examples_and_the_count() {
        let mut files = good_files();
        let names: Vec<String> = (0..5)
            .map(|i| format!("Content/TestMod/n{i}?.tex"))
            .collect();
        for n in &names {
            files.push((n.as_str(), "x"));
        }
        let r = run_one(&base(), &files);
        let f = &only(&r, Lint::WindowsUnsafeName)[0];
        assert!(f.message.contains("(5)"), "{}", f.message);
        assert!(f.message.ends_with(", ..."), "{}", f.message);
        assert_eq!(f.message.matches(".tex").count(), 3);
        assert_eq!(f.path.as_deref(), Some("content/testmod/n0?.tex"));
    }

    #[test]
    fn placeholders_and_empty_mods_are_reported() {
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let m = ModUnderTest::new("testmod")
            .with_layer(ModLayer::new("a.gro", mem(&good_files())))
            .with_placeholder("p1.gro")
            .with_placeholder("p2.gro");
        let r = check_mods(&env, &[m]);
        assert_eq!(codes(&r), ["MOD042"]);
        assert!(
            r.findings[0].message.contains("placeholder dummy.txt"),
            "{}",
            r.findings[0].message
        );
        assert!(
            r.findings[0].message.contains("(2): p1.gro, p2.gro"),
            "{}",
            r.findings[0].message
        );
        let m = ModUnderTest::new("testmod");
        let r = check_mods(&env, &[m]);
        assert!(codes(&r).contains(&"MOD009"), "{:?}", codes(&r));
        assert_eq!(r.mods[0].layers, 0);
        assert!(codes(&r).contains(&"MOD001"));
    }

    /// A file system whose folders or files can be made to fail.
    struct Flaky {
        inner: MemoryVfs,
        fail_list: Option<(&'static str, bool)>,
        fail_read: Vec<&'static str>,
        fail_stat: Vec<&'static str>,
    }

    impl Vfs for Flaky {
        fn stat(&self, path: &VfsPath) -> Result<Stat> {
            if self.fail_stat.contains(&path.as_str()) {
                return Err(Error::invalid("test", "stat failed"));
            }
            self.inner.stat(path)
        }
        fn open(&self, path: &VfsPath) -> Result<Box<dyn crate::ReadSeek>> {
            if self.fail_read.contains(&path.as_str()) {
                return Err(Error::invalid("test", "read failed"));
            }
            self.inner.open(path)
        }
        fn list(&self, dir: &VfsPath) -> Result<Vec<crate::DirEntry>> {
            if let Some((name, as_invalid)) = self.fail_list
                && dir.as_str() == name
            {
                return Err(if as_invalid {
                    Error::invalid("test", "names collide")
                } else {
                    Error::NotFound("gone".into())
                });
            }
            self.inner.list(dir)
        }
    }

    fn flaky(files: &[(&str, &str)]) -> Flaky {
        Flaky {
            inner: mem(files),
            fail_list: None,
            fail_read: Vec::new(),
            fail_stat: Vec::new(),
        }
    }

    fn run_layer(layer: Flaky) -> ModReport {
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let m = ModUnderTest::new("testmod").with_layer(ModLayer::new("a.gro", layer));
        check_mods(&env, &[m])
    }

    #[test]
    fn folders_that_cannot_be_listed_are_findings_not_panics() {
        let mut layer = flaky(&good_files());
        layer.fail_list = Some(("content/testmod", true));
        let r = run_layer(layer);
        let f = only(&r, Lint::CaseCollision);
        assert_eq!(f.len(), 1, "{:#?}", r.findings);
        assert_eq!(f[0].severity(), Severity::Error);
        assert_eq!(f[0].path.as_deref(), Some("content/testmod"));
        let mut layer = flaky(&good_files());
        layer.fail_list = Some(("content/testmod", false));
        let r = run_layer(layer);
        assert_eq!(only(&r, Lint::Unreadable).len(), 1, "{:#?}", r.findings);
        assert_eq!(r.exit_code(), 0, "a warning");
    }

    #[test]
    fn files_that_cannot_be_read_or_inspected_are_findings_not_panics() {
        let mut layer = flaky(&good_files());
        layer.fail_read = vec!["content/testmod.nfo", "content/testmod.mod"];
        let r = run_layer(layer);
        assert_eq!(codes(&r), ["MOD002", "MOD004"], "{:#?}", r.findings);
        let mut layer = flaky(&with(&[("Content/TestMod/p.nfo", "NAME = \"x\"")]));
        layer.fail_read = vec!["content/testmod/p.nfo"];
        let r = run_layer(layer);
        assert_eq!(only(&r, Lint::Unreadable).len(), 1, "{:#?}", r.findings);
        assert!(r.findings.iter().any(|f| f.message.contains("read failed")));
        let mut layer = flaky(&good_files());
        layer.fail_stat = vec!["content/testmod/t.tex"];
        let r = run_layer(layer);
        assert_eq!(only(&r, Lint::Unreadable).len(), 1, "{:#?}", r.findings);
    }

    #[test]
    fn huge_descriptors_are_not_read() {
        let big = "x".repeat(1_048_577);
        let files = [
            ("Content/TestMod.mod", big.as_str()),
            ("Content/TestMod.nfo", big.as_str()),
            ("Content/TestMod/t.tex", "x"),
            ("Content/TestMod/p.nfo", big.as_str()),
        ];
        let r = run_one(&base(), &files);
        assert_eq!(
            only(&r, Lint::NfoTooLarge).len(),
            2,
            "{:#?}",
            r.findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        assert_eq!(only(&r, Lint::ModFileMalformed).len(), 1);
        assert_eq!(r.exit_code(), 1);
        // Exactly at the limit is still read.
        let at = "x".repeat(1_048_576);
        let files = [
            ("Content/TestMod.mod", MOD),
            ("Content/TestMod.nfo", at.as_str()),
            ("Content/TestMod/t.tex", "x"),
        ];
        let r = run_one(&base(), &files);
        assert!(only(&r, Lint::NfoTooLarge).is_empty());
    }

    #[test]
    fn a_folder_where_a_descriptor_should_be_is_malformed_not_a_panic() {
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod/x.txt", "x"),
                ("Content/TestMod.nfo/x.txt", "x"),
                ("Content/TestMod/t.tex", "x"),
            ],
        );
        assert_eq!(codes(&r), ["MOD001", "MOD003"]);
    }

    #[test]
    fn two_mods_that_provide_the_same_file_conflict() {
        let b = base();
        let patch = PatchLevel::from_file_name("Patch_02_100.gro");
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: patch.as_ref(),
        };
        let a = ModUnderTest::new("a").with_layer(ModLayer::new(
            "a.gro",
            mem(&[
                ("Content/a.mod", MOD),
                ("Content/a.nfo", "NAME = \"A\" ;\nAUTHOR = \"x\" ;"),
                ("Content/SeriousSam2/Textures/b.tex", "from a"),
                ("Content/SeriousSam2/Models/z.mdl", "same"),
                ("Content/SeriousSam2/Models/only_a.mdl", "a"),
            ]),
        ));
        let c = ModUnderTest::new("c").with_layer(ModLayer::new(
            "c.gro",
            mem(&[
                ("Content/c.mod", MOD),
                ("Content/c.nfo", "NAME = \"C\" ;\nAUTHOR = \"x\" ;"),
                ("Content/SeriousSam2/Textures/b.tex", "from c"),
                ("Content/SeriousSam2/Models/z.mdl", "same"),
                ("Content/SeriousSam2/Models/only_c.mdl", "c"),
            ]),
        ));
        let r = check_mods(&env, &[a, c]);
        let f = only(&r, Lint::ModConflict);
        assert_eq!(f.len(), 1, "{:#?}", r.findings);
        assert_eq!(f[0].mod_id, "c", "reported on the mod loaded later");
        assert_eq!(f[0].severity(), Severity::Warning);
        assert!(
            f[0].message.contains(
                "replaces 2 files of a (1 differ or could not be compared, 1 are identical)"
            ),
            "{}",
            f[0].message
        );
        assert!(
            f[0].message.contains("content/serioussam2/textures/b.tex"),
            "{}",
            f[0].message
        );
        assert_eq!(r.mods.len(), 2);
        assert_eq!(r.exit_code(), 0);
        // The order changes who is named, not the finding.
        let a = ModUnderTest::new("a").with_layer(ModLayer::new(
            "a.gro",
            mem(&[("Content/a.mod", MOD), ("Content/q.txt", "1")]),
        ));
        let c = ModUnderTest::new("c").with_layer(ModLayer::new(
            "c.gro",
            mem(&[("Content/c.mod", MOD), ("Content/q.txt", "1")]),
        ));
        let r = check_mods(&env, &[a, c]);
        let d = only(&r, Lint::ModDuplicate);
        assert_eq!(d.len(), 1, "{:#?}", r.findings);
        assert_eq!(d[0].severity(), Severity::Info);
        assert!(
            d[0].message.contains("c and a provide 1 identical files"),
            "{}",
            d[0].message
        );
        assert!(only(&r, Lint::ModConflict).is_empty());
    }

    #[test]
    fn same_size_but_different_bytes_and_huge_files_are_conflicts() {
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let mk = |id: &str, data: &[u8]| {
            let mut v = MemoryVfs::new();
            v.insert("Content/shared.bin", data.to_vec()).unwrap();
            ModUnderTest::new(id).with_layer(ModLayer::new(format!("{id}.gro"), v))
        };
        let r = check_mods(&env, &[mk("a", b"abcd"), mk("b", b"abce")]);
        assert_eq!(only(&r, Lint::ModConflict).len(), 1, "{:#?}", r.findings);
        let big = vec![7u8; 8_388_609];
        let r = check_mods(&env, &[mk("a", &big), mk("b", &big)]);
        assert_eq!(
            only(&r, Lint::ModConflict).len(),
            1,
            "too big to compare: not claimed identical"
        );
        let exactly = vec![7u8; 8_388_608];
        let r = check_mods(&env, &[mk("a", &exactly), mk("b", &exactly)]);
        assert_eq!(
            only(&r, Lint::ModDuplicate).len(),
            1,
            "{:#?}",
            r.findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        let r = check_mods(&env, &[mk("a", b"abc"), mk("b", b"abcd")]);
        assert_eq!(only(&r, Lint::ModConflict).len(), 1, "different sizes");
        // Three mods: every pair is looked at.
        let r = check_mods(&env, &[mk("a", b"1"), mk("b", b"2"), mk("c", b"3")]);
        assert_eq!(only(&r, Lint::ModConflict).len(), 3);
        // Unreadable files are not claimed identical.
        let mut flaky_mod = flaky(&[("Content/shared.bin", "same")]);
        flaky_mod.fail_read = vec!["content/shared.bin"];
        let a = ModUnderTest::new("a").with_layer(ModLayer::new("a.gro", flaky_mod));
        let r = check_mods(&env, &[a, mk("b", b"same")]);
        assert_eq!(only(&r, Lint::ModConflict).len(), 1);
    }

    #[test]
    fn disjoint_mods_do_not_conflict() {
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let a = ModUnderTest::new("a")
            .with_layer(ModLayer::new("a.gro", mem(&[("Content/a/x.txt", "1")])));
        let c = ModUnderTest::new("c")
            .with_layer(ModLayer::new("c.gro", mem(&[("Content/c/x.txt", "1")])));
        let r = check_mods(&env, &[a, c]);
        assert!(only(&r, Lint::ModConflict).is_empty() && only(&r, Lint::ModDuplicate).is_empty());
    }

    #[test]
    fn the_report_text_is_golden() {
        let nfo =
            "NAME = \"T\" ;\nMODEL = \"Content/TestMod/gone.mdl\" ;\nTHUMBNAIL = \"../t.tex\" ;";
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", MOD),
                ("Content/TestMod.nfo", "NAME = \"Test Mod\" ;"),
                ("Content/TestMod/p.nfo", nfo),
                ("Content/SeriousSam2/Models/a.mdl", "new"),
            ],
        );
        let expected = "\
SS2RS mod check

Install patch level: 2.100 (build 800000)

Mod testmod (\"Test Mod\"): 1 layer, 4 files, 1 override base files
  ERROR   MOD030 content/testmod/p.nfo: MODEL = \"Content/TestMod/gone.mdl\" (model) exists neither in the mod nor in the install
          -> Add the model to the mod, or point MODEL at a model of the base game.
  ERROR   MOD032 content/testmod/p.nfo: THUMBNAIL = \"../t.tex\" is not allowed: it climbs out of the game folder with `..`
          -> Use a relative path below Content/, written like Content/<Mod>/Models/x.mdl.
  INFO    MOD006 content/testmod.nfo names no AUTHOR
          -> Add `AUTHOR = \"Your Name\" ;` so players know whom to credit.
  INFO    MOD050 1 of 4 files replace base game files, 3 are new (replaced by type: .mdl 1)
          -> Nothing to fix: this is what the mod changes. See the critical overrides below, if any.

2 errors, 0 warnings, 2 notes: fix the errors first.
";
        assert_eq!(r.render(), expected);
        assert_eq!(r.exit_code(), 1);
    }

    #[test]
    fn the_report_text_for_warnings_only_and_for_install_level_findings() {
        let r = run_one(
            &base(),
            &[("Content/TestMod.mod", MOD), ("Content/TestMod/t.tex", "x")],
        );
        let text = r.render();
        assert!(
            text.contains("  WARNING MOD003 the mod has no content/testmod.nfo, so"),
            "{text}"
        );
        assert!(text.ends_with("No errors, 1 warning, 0 notes.\n"), "{text}");
        let r = ModReport {
            patch_level: None,
            mods: Vec::new(),
            findings: vec![install_finding(
                Lint::InstallUnreadable,
                "",
                Some("D:/x".into()),
                "broken".into(),
                "fix it",
            )],
        };
        let text = r.render();
        assert!(text.contains("Install patch level: none found"), "{text}");
        assert!(
            text.contains("\nERROR   MOD090 D:/x: broken\n          -> fix it\n"),
            "{text}"
        );
        assert!(
            text.ends_with("1 error, 0 warnings, 0 notes: fix the errors first.\n"),
            "{text}"
        );
        let r = ModReport {
            patch_level: None,
            mods: Vec::new(),
            findings: vec![install_finding(
                Lint::ModNotFound,
                "Nope",
                None,
                "no".into(),
                "h",
            )],
        };
        assert!(
            r.render().contains("ERROR   MOD091 [nope] no"),
            "{}",
            r.render()
        );
    }

    #[test]
    fn rendering_is_ascii_even_for_hostile_names() {
        let nfo = "NAME = \"T\" ;\nMODEL = \"Content/caf\u{e9}/\u{202e}x.mdl\" ;";
        let r = run_one(&base(), &with(&[("Content/TestMod/p.nfo", nfo)]));
        let text = r.render();
        assert!(text.is_ascii(), "{text}");
        assert!(text.contains("\\u{e9}"), "{text}");
        assert!(text.contains("\\u{202e}"), "{text}");
    }

    #[test]
    fn output_is_deterministic_and_does_not_depend_on_insertion_order() {
        let mut files = with(&[
            ("Content/TestMod/z.nfo", "NAME = \"x\"\nMODEL = \"a/1.mdl\""),
            ("Content/TestMod/a.nfo", "NAME = \"x\"\nMODEL = \"a/2.mdl\""),
            (
                "Content/TestMod/m.nfo",
                "NAME = \"x\"\nTHUMBNAIL = \"a/3.tex\"",
            ),
        ]);
        let first = run_one(&base(), &files);
        files.reverse();
        let second = run_one(&base(), &files);
        assert_eq!(first, second);
        assert_eq!(first.render(), second.render());
        let again = run_one(&base(), &files);
        assert_eq!(second.render(), again.render());
        let paths: Vec<_> = first
            .findings
            .iter()
            .map(|f| f.path.clone().unwrap())
            .collect();
        assert_eq!(
            paths[0], "content/testmod/a.nfo",
            "errors sorted by path: {paths:?}"
        );
    }

    #[test]
    fn findings_are_sorted_most_serious_first() {
        let nfo = "NAME = \"x\"\nTHUMBNAIL = \"a/3.tex\"\nMODEL = \"a/b.mdl\"";
        let r = run_one(
            &base(),
            &[
                ("Content/TestMod.mod", "bad"),
                ("Content/TestMod/p.nfo", nfo),
            ],
        );
        let ranks: Vec<u8> = r.findings.iter().map(|f| rank(f.severity())).collect();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        assert_eq!(ranks, sorted, "{:#?}", r.findings);
        assert!(ranks.contains(&0) && ranks.contains(&1));
    }

    #[test]
    fn counts_and_plural_in_the_summary_line() {
        let r = run_one(
            &base(),
            &with(&[(
                "Content/TestMod/p.nfo",
                "NAME = \"x\"\nMODEL = \"a.mdl\"\nTHUMBNAIL = \"b.tex\"",
            )]),
        );
        let text = r.render();
        assert!(text.contains("1 error, 1 warning, 0 notes: fix"), "{text}");
        assert!(text.contains("1 layer, 4 files, 0 override"), "{text}");
        assert_eq!((r.errors(), r.warnings(), r.infos()), (1, 1, 0));
    }

    #[test]
    fn mod_ids_are_case_insensitive_and_shown_lower_case() {
        let r = run_one(&base(), &good_files());
        assert_eq!(r.mods[0].id, "testmod");
        assert_eq!(ModUnderTest::new("AbC").id(), "abc");
        assert_eq!(ModLayer::new("n", MemoryVfs::new()).name(), "n");
    }

    fn write_gro(dir: &Path, name: &str, files: &[(&str, &str)]) {
        let mut b = ss2_testkit::fixtures::ZipBuilder::new();
        for (n, d) in files {
            b = b.add(*n, d.as_bytes().to_vec());
        }
        std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
    }

    fn install_with_mod() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        write_gro(
            p,
            "All_PC_01.gro",
            &[
                ("Content/SeriousSam2/Models/a.mdl", "m"),
                ("Content/SeriousSam2/Databases/x.ep", "e"),
            ],
        );
        write_gro(p, "Patch_02_068.gro", &[("Content/SeriousSam2.mod", MOD)]);
        write_gro(p, "Patch_02_090_593613.gro", &[("Content/x.txt", "x")]);
        std::fs::create_dir_all(p.join("Content/TestMod")).unwrap();
        std::fs::create_dir_all(p.join("Bin")).unwrap();
        std::fs::write(p.join("Bin/Sam2Game.dll"), b"x").unwrap();
        std::fs::write(p.join("Content/TestMod.mod"), MOD).unwrap();
        std::fs::write(
            p.join("Content/TestMod.nfo"),
            "NAME = \"Test Mod\" ;\nAUTHOR = \"Me\" ;\nTHUMBNAIL = \"Content/TestMod/thumb.tex\" ;",
        )
        .unwrap();
        std::fs::write(p.join("Content/TestMod/thumb.tex"), b"t").unwrap();
        write_gro(
            &p.join("Content/TestMod"),
            "TestMod.gro",
            &[
                ("Content/SeriousSam2/Models/a.mdl", "new"),
                ("Content/TestMod/own.mdl", "o"),
            ],
        );
        write_gro(
            &p.join("Content/TestMod"),
            "z_Update_02_090.gro",
            &[("Content/TestMod/u.txt", "u")],
        );
        write_gro(
            &p.join("Content/TestMod"),
            "marker.gro",
            &[("dummy.txt", "")],
        );
        dir
    }

    #[test]
    fn an_installed_mod_is_mounted_and_checked() {
        let dir = install_with_mod();
        let r = check_install_mods(dir.path(), &[ModSpec::Installed("TestMod".into())]);
        assert_eq!(codes(&r), ["MOD042", "MOD050"], "{:#?}", r.findings);
        assert_eq!(r.exit_code(), 0);
        assert_eq!(
            r.patch_level.as_ref().map(ToString::to_string).as_deref(),
            Some("2.090 (build 593613)")
        );
        assert_eq!(r.mods[0].layers, 2);
        assert_eq!(r.mods[0].files, 3);
        assert_eq!(r.mods[0].overrides, 1);
        assert_eq!(r.mods[0].name, "Test Mod");
        assert_eq!(installed_mod_names(dir.path()), ["TestMod"]);
    }

    #[test]
    fn the_newest_patch_ties_are_broken_by_name_and_only_folders_with_archives_are_mods() {
        let dir = install_with_mod();
        let p = dir.path();
        assert_eq!(newest_patch(p).unwrap().archive, "Patch_02_090_593613.gro");
        write_gro(p, "Patch_02_100.gro", &[("Content/y.txt", "y")]);
        write_gro(p, "Patch_02_100_0.gro", &[("Content/z.txt", "z")]);
        let best = newest_patch(p).unwrap();
        assert_eq!(
            best.archive, "Patch_02_100_0.gro",
            "same numbers: the later name"
        );
        assert_eq!(newest_patch(&p.join("absent")), None);
        std::fs::create_dir_all(p.join("Content/NoGro")).unwrap();
        std::fs::write(p.join("Content/NoGro/readme"), b"x").unwrap();
        std::fs::create_dir_all(p.join("Content/Texts")).unwrap();
        std::fs::write(p.join("Content/Texts/notes.txt"), b"x").unwrap();
        std::fs::create_dir_all(p.join("Content/Empty")).unwrap();
        std::fs::write(p.join("Content/loose.gro"), b"not a folder").unwrap();
        assert_eq!(installed_mod_names(p), ["TestMod"]);
    }

    #[test]
    fn installed_mod_errors_become_findings() {
        let dir = install_with_mod();
        let r = check_install_mods(
            dir.path(),
            &[
                ModSpec::Installed("Nope".into()),
                ModSpec::Installed("TestMod".into()),
            ],
        );
        assert_eq!(only(&r, Lint::ModNotFound).len(), 1, "{:#?}", r.findings);
        assert_eq!(r.mods.len(), 1, "the other mod is still checked");
        assert_eq!(r.exit_code(), 1);
        std::fs::write(dir.path().join("Content/TestMod/broken.gro"), b"not a zip").unwrap();
        let r = check_install_mods(dir.path(), &[ModSpec::Installed("testmod".into())]);
        let f = only(&r, Lint::ArchiveUnreadable);
        assert_eq!(f.len(), 1, "{:#?}", r.findings);
        assert_eq!(f[0].path.as_deref(), Some("broken.gro"));
        assert_eq!(f[0].mod_id, "testmod");
        assert_eq!(r.mods[0].layers, 2, "the other archives are still read");
        assert!(r.render().contains("MOD092"));
        // An empty archive is a placeholder, not a layer.
        write_gro(&dir.path().join("Content/TestMod"), "empty.gro", &[]);
        let r = check_install_mods(dir.path(), &[ModSpec::Installed("testmod".into())]);
        assert_eq!(r.mods[0].layers, 2);
    }

    #[test]
    fn a_missing_or_empty_install_is_one_finding() {
        let dir = tempfile::tempdir().unwrap();
        let r = check_install_mods(dir.path(), &[ModSpec::Installed("x".into())]);
        assert_eq!(codes(&r), ["MOD090"]);
        assert_eq!(r.exit_code(), 1);
        assert!(r.mods.is_empty());
        let r = check_install_mods(dir.path().join("nope"), &[]);
        assert_eq!(codes(&r), ["MOD090"]);
        assert!(installed_mod_names(dir.path().join("nope")).is_empty());
    }

    #[test]
    fn a_loose_mod_folder_is_checked_like_an_archive() {
        let dir = install_with_mod();
        let work = tempfile::tempdir().unwrap();
        let root = work.path().join("MyWork");
        std::fs::create_dir_all(root.join("Content/MyMod")).unwrap();
        std::fs::write(root.join("Content/MyMod.mod"), MOD).unwrap();
        std::fs::write(
            root.join("Content/MyMod.nfo"),
            "NAME = \"Mine\" ;\nAUTHOR = \"Me\" ;",
        )
        .unwrap();
        std::fs::write(
            root.join("Content/MyMod/p.nfo"),
            "NAME = \"p\"\nMODEL = \"Content/MyMod/none.mdl\"",
        )
        .unwrap();
        let r = check_install_mods(dir.path(), &[ModSpec::Folder(root)]);
        assert_eq!(r.mods[0].id, "mymod");
        assert_eq!(r.mods[0].name, "Mine");
        assert_eq!(codes(&r), ["MOD030"], "{:#?}", r.findings);
        let r = check_install_mods(dir.path(), &[ModSpec::Folder(work.path().join("absent"))]);
        assert_eq!(codes(&r), ["MOD091"]);
        assert_eq!(r.exit_code(), 1);
        // No .mod file: the id is the folder name.
        let bare = work.path().join("Bare");
        std::fs::create_dir_all(&bare).unwrap();
        let r = check_install_mods(dir.path(), &[ModSpec::Folder(bare)]);
        assert_eq!(r.mods[0].id, "bare");
        assert!(codes(&r).contains(&"MOD001"));
    }

    #[test]
    fn two_installed_mods_are_compared() {
        let dir = install_with_mod();
        let p = dir.path();
        std::fs::create_dir_all(p.join("Content/Other")).unwrap();
        std::fs::write(p.join("Content/Other.mod"), MOD).unwrap();
        std::fs::write(
            p.join("Content/Other.nfo"),
            "NAME = \"O\" ;\nAUTHOR = \"o\" ;",
        )
        .unwrap();
        write_gro(
            &p.join("Content/Other"),
            "Other.gro",
            &[("Content/SeriousSam2/Models/a.mdl", "other")],
        );
        assert_eq!(installed_mod_names(p), ["Other", "TestMod"]);
        let r = check_install_mods(
            p,
            &[
                ModSpec::Installed("TestMod".into()),
                ModSpec::Installed("Other".into()),
            ],
        );
        assert_eq!(only(&r, Lint::ModConflict).len(), 1, "{:#?}", r.findings);
        assert_eq!(only(&r, Lint::ModConflict)[0].mod_id, "other");
    }

    #[test]
    fn archive_entries_with_bad_names_and_duplicates_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.gro");
        let bytes = ss2_testkit::fixtures::ZipBuilder::new()
            .add("Content/TestMod.mod", MOD.as_bytes().to_vec())
            .add("Content/TestMod.nfo", NFO.as_bytes().to_vec())
            .add("Content/TestMod/t.tex", b"1".to_vec())
            .add("Content/TestMod/T.TEX", b"2".to_vec())
            .add("../evil.txt", b"x".to_vec())
            .add("C:/evil.txt", b"x".to_vec())
            .finish()
            .unwrap();
        std::fs::write(&path, bytes).unwrap();
        let archive = ArchiveVfs::from_file(&path).unwrap();
        let b = base();
        let env = ModCheckEnv {
            base: &b,
            loose: None,
            patch_level: None,
        };
        let m = ModUnderTest::new("testmod").with_layer(ModLayer::from_archive("m.gro", archive));
        let r = check_mods(&env, &[m]);
        assert_eq!(codes(&r), ["MOD038", "MOD039"], "{:#?}", r.findings);
        assert_eq!(r.findings[0].severity(), Severity::Error);
        assert!(
            r.findings[0]
                .message
                .starts_with("entries of m.gro with invalid names"),
            "{}",
            r.findings[0].message
        );
        assert!(
            r.findings[0]
                .message
                .contains("(2): ../evil.txt, C:/evil.txt"),
            "{}",
            r.findings[0].message
        );
        assert!(
            r.findings[1].message.contains("Content/TestMod/T.TEX"),
            "{}",
            r.findings[1].message
        );
        assert_eq!(r.exit_code(), 1);
    }

    #[test]
    fn archives_keep_the_duplicates_they_merged() {
        let dir = tempfile::tempdir().unwrap();
        write_gro(
            dir.path(),
            "a.gro",
            &[("A/x.txt", "1"), ("a\\X.TXT", "2"), ("b.txt", "3")],
        );
        let archive = ArchiveVfs::from_file(dir.path().join("a.gro")).unwrap();
        assert_eq!(archive.duplicate_entries(), ["a\\X.TXT"]);
        assert_eq!(
            archive.read(&vp("a/x.txt")).unwrap(),
            b"2",
            "the last one wins"
        );
        write_gro(dir.path(), "b.gro", &[("x.txt", "1"), ("y.txt", "2")]);
        assert!(
            ArchiveVfs::from_file(dir.path().join("b.gro"))
                .unwrap()
                .duplicate_entries()
                .is_empty()
        );
    }

    /// Data-gated: the real mods of the player's install (`Sam2Renovation`, `InSamnity2`).
    /// Skips without `SS2_DATA_DIR`; a mod that is not installed is skipped.
    #[test]
    fn the_real_mods_validate_without_errors_or_false_positives() {
        let dir = ss2_testkit::require_game_data!();
        let installed = installed_mod_names(&dir);
        let mut ids = Vec::new();
        for want in ["InSamnity2", "Sam2Renovation"] {
            if installed.iter().any(|n| n.eq_ignore_ascii_case(want)) {
                ids.push(want);
            }
        }
        if ids.is_empty() {
            eprintln!("no known mod installed: {installed:?}; skipping");
            return;
        }
        // The findings these two mods are known to deserve (docs/MOD_VALIDATION.md): anything
        // else is a false positive of a lint, or a regression in the mods.
        let expected = ["MOD023", "MOD031", "MOD042", "MOD050", "MOD051", "MOD060"];
        let mut specs: Vec<ModSpec> = Vec::new();
        for id in &ids {
            let report = check_install_mods(&dir, &[ModSpec::Installed((*id).to_owned())]);
            let m = &report.mods[0];
            eprintln!(
                "{}: {} layers, {} files, {} overrides; {} errors, {} warnings, {} notes",
                m.id,
                m.layers,
                m.files,
                m.overrides,
                report.errors(),
                report.warnings(),
                report.infos()
            );
            for f in &report.findings {
                eprintln!("  {} {:?} {}", f.code(), f.severity(), f.message);
                assert!(expected.contains(&f.code()), "unexpected finding: {f:#?}");
            }
            assert_eq!(report.exit_code(), 0, "{}", report.render());
            assert!(report.render().is_ascii());
            match m.id.as_str() {
                "sam2renovation" => {
                    assert_eq!((m.layers, m.files, m.overrides), (1, 3194, 1963));
                    assert!(report.findings.iter().any(|f| f.code() == "MOD051"));
                }
                "insamnity2" => {
                    assert_eq!((m.layers, m.overrides), (8, 399));
                    assert!(report.findings.iter().any(|f| f.code() == "MOD023"));
                    assert!(
                        report.findings.iter().any(|f| f.code() == "MOD031"
                            && f.message.contains("FELevels/01/01_thumbnail.tex")),
                        "the dangling thumbnail of FELevels.nfo is found"
                    );
                }
                other => panic!("unexpected mod {other}"),
            }
            specs.push(ModSpec::Installed((*id).to_owned()));
        }
        if specs.len() == 2 {
            let both = check_install_mods(&dir, &specs);
            let conflicts: Vec<_> = both
                .findings
                .iter()
                .filter(|f| f.lint == Lint::ModConflict)
                .collect();
            eprintln!("both mods: {} conflict finding(s)", conflicts.len());
            assert_eq!(conflicts.len(), 1, "{}", both.render());
            assert_eq!(both.exit_code(), 0);
            // The same input gives the same text.
            assert_eq!(both.render(), check_install_mods(&dir, &specs).render());
        }
    }

    #[test]
    fn garbage_everywhere_never_panics() {
        let junk: Vec<Vec<u8>> = vec![
            vec![],
            vec![0xFF; 64],
            b"\xEF\xBB\xBF".to_vec(),
            b"=\n=\n;;;\"".to_vec(),
            b"LEVEL\nMODEL=\"".to_vec(),
            "NAME = \"\u{0}\"\nMODEL = \"\u{ffff}\"".as_bytes().to_vec(),
            b"MODEL = ".to_vec(),
            b"\r\n\r\n".to_vec(),
        ];
        for bytes in junk {
            let mut v = MemoryVfs::new();
            for name in [
                "Content/TestMod.mod",
                "Content/TestMod.nfo",
                "Content/TestMod/a.nfo",
                "Content/TestMod/b.nfo",
            ] {
                v.insert(name, bytes.clone()).unwrap();
            }
            let b = base();
            let env = ModCheckEnv {
                base: &b,
                loose: None,
                patch_level: None,
            };
            let m = ModUnderTest::new("testmod").with_layer(ModLayer::new("a.gro", v));
            let r = check_mods(&env, &[m]);
            assert!(r.render().is_ascii());
            let _ = r.exit_code();
        }
    }
}
