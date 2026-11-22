//! Install validation for end users (`ss2-doctor`, roadmap T11-013).
//!
//! [`check_install`] looks at the folder a player points the engine at and says, in plain
//! language, what is wrong with it: the folder is not the one that holds the archives, a base
//! archive is missing or damaged, there are no patches, or a foreign `.gro` file would be loaded
//! on top of the game. It never fails with an error of its own: every problem becomes a
//! [`Finding`] with advice, so a report can always be printed.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::{ArchiveVfs, archive_load_order};

/// The base data archives; the game cannot run without them.
const BASE_ARCHIVES: [&str; 3] = ["All_PC_01.gro", "All_PC_02.gro", "All_PC_03.gro"];
/// The movie archives; without them only the cutscenes are missing.
const MOVIE_ARCHIVES: [&str; 2] = ["Movies_PC_01.gro", "Movies_PC_02.gro"];

/// How bad a [`Finding`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The install is usable but probably not what the player expects.
    Warning,
    /// The install cannot be used as it is.
    Error,
}

/// One problem, with what to do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Warning or error.
    pub severity: Severity,
    /// What is wrong, in one sentence that names the file or folder.
    pub message: String,
    /// What the player can do about it.
    pub advice: Option<String>,
}

/// The part an archive plays in an install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveRole {
    /// One of the base data archives.
    Base,
    /// One of the movie archives.
    Movies,
    /// A recognised patch archive.
    Patch,
    /// Anything else with the extension `.gro`.
    Unexpected,
}

/// One `.gro` file found in the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveInfo {
    /// The file name as it is on disk.
    pub name: String,
    /// What the archive is.
    pub role: ArchiveRole,
    /// Size on disk in bytes.
    pub bytes: u64,
    /// Number of files inside, when the archive could be read.
    pub files: Option<usize>,
    /// Why the archive could not be read.
    pub error: Option<String>,
}

/// The version a patch archive brings the game to, from its file name
/// (`Patch_02_068.gro` is 2.068, `Patch_02_100_800000.gro` is 2.100 build 800000).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchLevel {
    /// First number of the name (2 in every shipped patch).
    pub major: u32,
    /// Second number of the name.
    pub minor: u32,
    /// The build number, when the name has one.
    pub build: Option<u32>,
    /// The file name this was read from.
    pub archive: String,
}

impl PatchLevel {
    /// Read a patch level from an archive file name; `None` when it does not look like one.
    pub fn from_file_name(name: &str) -> Option<Self> {
        let (stem, ext) = name.rsplit_once('.')?;
        if !ext.eq_ignore_ascii_case("gro") {
            return None;
        }
        let parts: Vec<&str> = stem.split('_').collect();
        if !(3..=4).contains(&parts.len()) || !parts[0].eq_ignore_ascii_case("patch") {
            return None;
        }
        let number = |s: &str| {
            (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .then(|| s.parse::<u32>().ok())
                .flatten()
        };
        Some(Self {
            major: number(parts[1])?,
            minor: number(parts[2])?,
            build: match parts.get(3) {
                Some(b) => Some(number(b)?),
                None => None,
            },
            archive: name.to_owned(),
        })
    }

    fn key(&self) -> (u32, u32, u32) {
        (self.major, self.minor, self.build.unwrap_or(0))
    }
}

impl fmt::Display for PatchLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:03}", self.major, self.minor)?;
        if let Some(build) = self.build {
            write!(f, " (build {build})")?;
        }
        Ok(())
    }
}

/// Everything [`check_install`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallReport {
    /// The folder that was checked.
    pub dir: PathBuf,
    /// The archives found, in load order (base data first, then patches in numeric order).
    pub archives: Vec<ArchiveInfo>,
    /// The problems, most serious first.
    pub findings: Vec<Finding>,
    /// The newest patch found, if any.
    pub patch_level: Option<PatchLevel>,
}

impl InstallReport {
    /// The findings that make the install unusable.
    pub fn errors(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .collect()
    }

    /// The findings that are only worth a look.
    pub fn warnings(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .collect()
    }

    /// `0` when there is no error, `1` otherwise (warnings do not change it).
    pub fn exit_code(&self) -> i32 {
        i32::from(!self.errors().is_empty())
    }

    /// The report as plain ASCII text for a terminal or a bug report.
    pub fn render(&self) -> String {
        let mut out = format!("SS2RS install check: {}\n\n", self.dir.display());
        if !self.archives.is_empty() {
            out.push_str("Archives (load order):\n");
            for a in &self.archives {
                let detail = match (&a.files, &a.error) {
                    (_, Some(error)) => format!("damaged: {error}"),
                    (Some(files), None) => {
                        format!("{}, {files} file{}", size(a.bytes), plural(*files))
                    }
                    (None, None) => size(a.bytes),
                };
                let mark = if a.error.is_some() { "ERROR" } else { "OK" };
                out.push_str(&format!("  {mark:<6}{:<26}{detail}\n", a.name));
            }
            out.push('\n');
        }
        match &self.patch_level {
            Some(level) => out.push_str(&format!("Patch level: {level}\n")),
            None => out.push_str("Patch level: none found\n"),
        }
        if !self.findings.is_empty() {
            out.push_str("\nProblems:\n");
            for f in &self.findings {
                let label = if f.severity == Severity::Error {
                    "ERROR"
                } else {
                    "WARNING"
                };
                out.push_str(&format!("  {label}: {}\n", f.message));
                if let Some(advice) = &f.advice {
                    out.push_str(&format!("      -> {advice}\n"));
                }
            }
        }
        let (errors, warnings) = (self.errors().len(), self.warnings().len());
        out.push('\n');
        out.push_str(&match (errors, warnings) {
            (0, 0) => "Install looks complete.\n".to_owned(),
            (0, w) => format!("Install is usable, with {w} warning{}.\n", plural(w)),
            (e, w) => format!(
                "{e} error{}, {w} warning{}: fix the errors first.\n",
                plural(e),
                plural(w)
            ),
        });
        out
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn size(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

fn finding(severity: Severity, message: String, advice: &str) -> Finding {
    Finding {
        severity,
        message,
        advice: Some(advice.to_owned()),
    }
}

/// Check the folder `dir` as an install of the game. See the module documentation.
pub fn check_install(dir: impl AsRef<Path>) -> InstallReport {
    let dir = dir.as_ref();
    let mut report = InstallReport {
        dir: dir.to_path_buf(),
        archives: Vec::new(),
        findings: Vec::new(),
        patch_level: None,
    };
    if !dir.exists() {
        report.findings.push(finding(
            Severity::Error,
            format!("the folder {} does not exist", dir.display()),
            "Check the spelling of the path, or give the folder that holds All_PC_01.gro.",
        ));
        return report;
    }
    if !dir.is_dir() {
        report.findings.push(finding(Severity::Error, format!("{} is a file, not a folder", dir.display()), "Give the folder that holds the .gro archives, for example the one that contains All_PC_01.gro."));
        return report;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            report.findings.push(finding(
                Severity::Error,
                format!("cannot read {}: {e}", dir.display()),
                "Check that the folder exists and that you may read it.",
            ));
            return report;
        }
    };
    let mut names = Vec::new();
    let mut subfolders = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            subfolders.push(path);
        } else if path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("gro"))
        {
            names.push(name);
        }
    }
    if names.is_empty() {
        let inner = subfolders.iter().find(|d| {
            std::fs::read_dir(d).is_ok_and(|mut it| {
                it.any(|e| {
                    e.is_ok_and(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(BASE_ARCHIVES[0])
                    })
                })
            })
        });
        let advice = match inner {
            Some(inner) => format!("The archives are one level down: give the folder {} instead.", inner.display()),
            None => "Point at the folder that contains All_PC_01.gro (the archives themselves, not a parent folder).".to_owned(),
        };
        report.findings.push(Finding {
            severity: Severity::Error,
            message: format!("no .gro archives in {}", dir.display()),
            advice: Some(advice),
        });
        return report;
    }

    let present = |wanted: &str| names.iter().any(|n| n.eq_ignore_ascii_case(wanted));
    for name in archive_load_order(names.clone()) {
        let path = dir.join(&name);
        let lower = name.to_lowercase();
        let patch = PatchLevel::from_file_name(&name);
        let role = if BASE_ARCHIVES.iter().any(|b| b.eq_ignore_ascii_case(&name)) {
            ArchiveRole::Base
        } else if MOVIE_ARCHIVES.iter().any(|b| b.eq_ignore_ascii_case(&name)) {
            ArchiveRole::Movies
        } else if patch.is_some() {
            ArchiveRole::Patch
        } else {
            ArchiveRole::Unexpected
        };
        let bytes = std::fs::metadata(&path).map_or(0, |m| m.len());
        let (files, error) = match ArchiveVfs::from_file(&path) {
            Ok(archive) => (Some(archive.file_count()), None),
            Err(e) => (None, Some(e.to_string())),
        };
        if let Some(error) = &error {
            report.findings.push(finding(
                Severity::Error,
                format!("{name} is damaged or is not a .gro archive ({error})"),
                "Copy the file again from the original install, or reinstall the game.",
            ));
        }
        match role {
            ArchiveRole::Unexpected if lower.starts_with("patch") => report.findings.push(finding(Severity::Warning, format!("{name} looks like a patch but its name is not understood"), "It is loaded after the base data in name order. Keep the original names (Patch_02_100_800000.gro).")),
            ArchiveRole::Unexpected => report.findings.push(finding(Severity::Warning, format!("{name} is not part of the original game; it is loaded over the game data and can change it"), "If the game behaves differently from the original, move this file out of the folder.")),
            _ => {}
        }
        if let Some(level) = patch {
            if report
                .patch_level
                .as_ref()
                .is_none_or(|best| level.key() > best.key())
            {
                report.patch_level = Some(level);
            }
        }
        report.archives.push(ArchiveInfo {
            name,
            role,
            bytes,
            files,
            error,
        });
    }
    for wanted in BASE_ARCHIVES {
        if !present(wanted) {
            report.findings.push(finding(
                Severity::Error,
                format!("{wanted} is missing"),
                "Reinstall the game, or copy the file back from the original install.",
            ));
        }
    }
    for wanted in MOVIE_ARCHIVES {
        if !present(wanted) {
            report.findings.push(finding(
                Severity::Warning,
                format!("{wanted} is missing, so the cutscenes will not play"),
                "Copy the file back from the original install if you want the movies.",
            ));
        }
    }
    if report.patch_level.is_none() {
        report.findings.push(finding(Severity::Warning, "no patch archives (Patch_*.gro) found: only the original release data is available".to_owned(), "Copy all the Patch_02_*.gro files from your install; without them the game data and behaviour differ from the patched game."));
    }
    report
        .findings
        .sort_by_key(|f| f.severity != Severity::Error);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_testkit::fixtures::ZipBuilder;
    use std::path::Path;

    fn write_gro(dir: &Path, name: &str) {
        let bytes = ZipBuilder::new()
            .add("readme.txt", b"x".to_vec())
            .finish()
            .unwrap();
        std::fs::write(dir.join(name), bytes).unwrap();
    }

    const BASE: [&str; 5] = [
        "All_PC_01.gro",
        "All_PC_02.gro",
        "All_PC_03.gro",
        "Movies_PC_01.gro",
        "Movies_PC_02.gro",
    ];

    fn complete(dir: &Path) {
        for name in BASE {
            write_gro(dir, name);
        }
        write_gro(dir, "Patch_02_068.gro");
        write_gro(dir, "Patch_02_080_210639.gro");
        write_gro(dir, "Patch_02_100_800000.gro");
    }

    fn messages(report: &InstallReport, severity: Severity) -> Vec<String> {
        report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| f.message.clone())
            .collect()
    }

    #[test]
    fn a_complete_install_has_no_problems_and_a_patch_level() {
        let dir = tempfile::tempdir().unwrap();
        complete(dir.path());
        let report = check_install(dir.path());
        assert!(report.errors().is_empty(), "{:?}", report.findings);
        assert!(report.warnings().is_empty(), "{:?}", report.findings);
        assert_eq!(report.exit_code(), 0);
        assert_eq!(
            report.patch_level,
            Some(PatchLevel {
                major: 2,
                minor: 100,
                build: Some(800_000),
                archive: "Patch_02_100_800000.gro".into(),
            })
        );
        let names: Vec<_> = report.archives.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "All_PC_01.gro",
                "All_PC_02.gro",
                "All_PC_03.gro",
                "Movies_PC_01.gro",
                "Movies_PC_02.gro",
                "Patch_02_068.gro",
                "Patch_02_080_210639.gro",
                "Patch_02_100_800000.gro"
            ],
            "listed in load order"
        );
        assert!(report.archives.iter().all(|a| a.files == Some(1)));
    }

    #[test]
    fn the_newest_patch_is_found_by_number_not_by_text() {
        let dir = tempfile::tempdir().unwrap();
        for name in BASE {
            write_gro(dir.path(), name);
        }
        write_gro(dir.path(), "Patch_02_068.gro");
        write_gro(dir.path(), "Patch_02_100_800000.gro");
        write_gro(dir.path(), "Patch_02_090_593613.gro");
        let level = check_install(dir.path()).patch_level.unwrap();
        assert_eq!((level.major, level.minor), (2, 100));
        assert_eq!(level.to_string(), "2.100 (build 800000)");
        assert_eq!(
            PatchLevel::from_file_name("Patch_02_068.gro")
                .unwrap()
                .to_string(),
            "2.068"
        );
    }

    #[test]
    fn patch_names_are_parsed_loosely_but_not_guessed() {
        let p = PatchLevel::from_file_name;
        assert_eq!(
            p("patch_02_080_210639.GRO").map(|l| (l.major, l.minor, l.build)),
            Some((2, 80, Some(210_639)))
        );
        assert_eq!(
            p("Patch_02_068.gro").map(|l| (l.major, l.minor, l.build)),
            Some((2, 68, None))
        );
        assert!(p("Patch_beta.gro").is_none());
        assert!(p("All_PC_01.gro").is_none());
        assert!(p("Patch_02.gro").is_none());
        assert!(p("Patch_02_068.zip").is_none());
    }

    #[test]
    fn an_empty_folder_is_an_error_that_names_the_file_to_look_for() {
        let dir = tempfile::tempdir().unwrap();
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 1);
        let errors = messages(&report, Severity::Error);
        assert!(errors.iter().any(|m| m.contains("no .gro")), "{errors:?}");
        let advice: String = report
            .findings
            .iter()
            .filter_map(|f| f.advice.clone())
            .collect();
        assert!(advice.contains("All_PC_01.gro"), "{advice}");
    }

    #[test]
    fn the_parent_of_the_install_is_recognised_and_the_right_folder_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("Serious Sam 2");
        std::fs::create_dir(&inner).unwrap();
        complete(&inner);
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 1);
        let advice: String = report
            .findings
            .iter()
            .filter_map(|f| f.advice.clone())
            .collect();
        assert!(advice.contains("Serious Sam 2"), "{advice}");
    }

    #[test]
    fn a_missing_base_archive_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        complete(dir.path());
        std::fs::remove_file(dir.path().join("All_PC_03.gro")).unwrap();
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 1);
        let errors = messages(&report, Severity::Error);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("All_PC_03.gro"), "{}", errors[0]);
    }

    #[test]
    fn base_names_are_matched_without_regard_to_case() {
        let dir = tempfile::tempdir().unwrap();
        for name in BASE {
            write_gro(dir.path(), &name.to_lowercase());
        }
        write_gro(dir.path(), "Patch_02_100_800000.gro");
        assert!(check_install(dir.path()).errors().is_empty());
    }

    #[test]
    fn missing_movies_are_only_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        complete(dir.path());
        std::fs::remove_file(dir.path().join("Movies_PC_02.gro")).unwrap();
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 0, "the game data itself is complete");
        let warnings = messages(&report, Severity::Warning);
        assert!(
            warnings.iter().any(|m| m.contains("Movies_PC_02.gro")),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_damaged_archive_is_an_error_and_the_others_are_still_reported() {
        let dir = tempfile::tempdir().unwrap();
        complete(dir.path());
        std::fs::write(dir.path().join("All_PC_02.gro"), b"this is not a zip file").unwrap();
        std::fs::write(dir.path().join("Patch_02_068.gro"), b"").unwrap();
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 1);
        let errors = messages(&report, Severity::Error).join("\n");
        assert!(errors.contains("All_PC_02.gro"), "{errors}");
        assert!(
            errors.contains("Patch_02_068.gro"),
            "an empty file is damaged too: {errors}"
        );
        let bad: Vec<_> = report
            .archives
            .iter()
            .filter(|a| a.error.is_some())
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(bad, ["All_PC_02.gro", "Patch_02_068.gro"]);
        let ok = report
            .archives
            .iter()
            .filter(|a| a.files == Some(1))
            .count();
        assert_eq!(ok, 6, "the intact archives are still read");
    }

    #[test]
    fn no_patches_is_a_warning_with_advice() {
        let dir = tempfile::tempdir().unwrap();
        for name in BASE {
            write_gro(dir.path(), name);
        }
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 0);
        assert_eq!(report.patch_level, None);
        let warnings = messages(&report, Severity::Warning).join("\n");
        assert!(warnings.contains("patch"), "{warnings}");
    }

    #[test]
    fn a_foreign_gro_file_is_a_warning_because_it_is_loaded_over_the_game() {
        let dir = tempfile::tempdir().unwrap();
        complete(dir.path());
        write_gro(dir.path(), "MyMod.gro");
        write_gro(dir.path(), "Patch_beta.gro");
        let report = check_install(dir.path());
        assert_eq!(report.exit_code(), 0);
        let warnings = messages(&report, Severity::Warning).join("\n");
        assert!(warnings.contains("MyMod.gro"), "{warnings}");
        assert!(warnings.contains("Patch_beta.gro"), "{warnings}");
    }

    #[test]
    fn a_missing_folder_and_a_file_instead_of_a_folder_are_errors_not_panics() {
        let dir = tempfile::tempdir().unwrap();
        let missing = check_install(dir.path().join("nope"));
        assert_eq!(missing.exit_code(), 1);
        assert!(messages(&missing, Severity::Error)[0].contains("nope"));
        let file = dir.path().join("a.gro");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(check_install(&file).exit_code(), 1);
    }

    #[test]
    fn the_rendered_report_is_plain_ascii_and_says_what_to_do() {
        let dir = tempfile::tempdir().unwrap();
        complete(dir.path());
        std::fs::remove_file(dir.path().join("All_PC_01.gro")).unwrap();
        let text = check_install(dir.path()).render();
        assert!(text.is_ascii(), "{text}");
        assert!(text.contains("ERROR"), "{text}");
        assert!(text.contains("All_PC_01.gro"), "{text}");
        assert!(text.contains("Patch level: 2.100 (build 800000)"), "{text}");
        assert!(text.contains("1 error"), "{text}");
        assert!(
            text.contains(
                ", 1 file
"
            ) && !text.contains("1 files"),
            "{text}"
        );
        let healthy = tempfile::tempdir().unwrap();
        complete(healthy.path());
        let ok = check_install(healthy.path()).render();
        assert!(ok.contains("Install looks complete"), "{ok}");
    }
}
