//! Keeps Croteam's proprietary game content out of the repository, and decompiler text out of the docs.
//!
//! Besides paths and sizes, every Markdown file under `docs/` is read for the two marks decompiled output leaves
//! when it is pasted: a Ghidra function name (`FUN_` and eight hex digits) and an address in the original DLLs'
//! image (`0x10` and six hex digits). `CLEANROOM.md` forbids both; the specs are prose and numbers.
//!
//! Run as `cargo xtask guard` (all tracked files) or `cargo xtask guard --staged` (pre-commit).

use std::fmt;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

/// Largest file allowed anywhere in the repository.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Largest file allowed under `testdata/` and `examples/mods/` (hand-made fixtures only).
pub const TESTDATA_MAX_BYTES: u64 = 256 * 1024;

/// Extensions that are never allowed, not even as fixtures.
const ALWAYS_FORBIDDEN: &[&str] = &["gro", "dll", "exe", "chm"];
/// Game-data extensions: forbidden everywhere except tiny hand-made fixtures under `testdata/` and the
/// invented example mods under `examples/mods/` (T13-010).
const FIXTURE_ONLY: &[&str] = &["wld", "tex", "mdl", "bmf", "ogv", "xmv"];

/// Why a path was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The path lies inside a `Serious Sam 2` install folder.
    ProprietaryDir,
    /// The file has a proprietary game-data extension (lower-cased, without the dot).
    ForbiddenExtension(String),
    /// The file is larger than allowed (catches proprietary blobs renamed to something else).
    TooLarge {
        /// Actual size in bytes.
        bytes: u64,
        /// Limit that applied.
        limit: u64,
    },
    /// A doc quotes decompiler output: a Ghidra function name or an address in the original's image.
    DecompilerText {
        /// 1-based line number.
        line: usize,
        /// The matched text.
        found: String,
    },
}

/// A rejected path with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Normalized path (forward slashes).
    pub path: String,
    /// Why it was rejected.
    pub reason: Reason,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            Reason::ProprietaryDir => {
                write!(f, "{}: inside a 'Serious Sam 2' install folder", self.path)
            }
            Reason::ForbiddenExtension(e) => {
                write!(f, "{}: proprietary game-data extension '.{e}'", self.path)
            }
            Reason::TooLarge { bytes, limit } => {
                write!(
                    f,
                    "{}: {bytes} bytes exceeds the {limit}-byte limit",
                    self.path
                )
            }
            Reason::DecompilerText { line, found } => {
                write!(
                    f,
                    "{}:{line}: decompiler text '{found}' (CLEANROOM.md: no function names or addresses)",
                    self.path
                )
            }
        }
    }
}

/// Check one repository path (any separator style) of the given size.
pub fn check_path(path: &str, size_bytes: u64) -> Option<Violation> {
    let normalized = path.replace('\\', "/");
    let normalized = normalized.trim_start_matches("./").to_string();
    let lower = normalized.to_ascii_lowercase();
    let in_testdata = lower.starts_with("testdata/") || lower.starts_with("examples/mods/");
    let violation = |reason| {
        Some(Violation {
            path: normalized.clone(),
            reason,
        })
    };

    if normalized
        .split('/')
        .any(|c| c.eq_ignore_ascii_case("serious sam 2"))
    {
        return violation(Reason::ProprietaryDir);
    }
    let extension = lower
        .rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, e)| e);
    if let Some(ext) = extension {
        let forbidden =
            ALWAYS_FORBIDDEN.contains(&ext) || (!in_testdata && FIXTURE_ONLY.contains(&ext));
        if forbidden {
            return violation(Reason::ForbiddenExtension(ext.to_string()));
        }
    }
    let limit = if in_testdata {
        TESTDATA_MAX_BYTES
    } else {
        MAX_BYTES
    };
    if size_bytes > limit {
        return violation(Reason::TooLarge {
            bytes: size_bytes,
            limit,
        });
    }
    None
}

/// Whether `path` is a doc whose text is checked for decompiler output (Markdown under `docs/`).
pub fn is_checked_doc(path: &str) -> bool {
    let p = path.replace('\\', "/").to_ascii_lowercase();
    p.starts_with("docs/") && p.ends_with(".md")
}

/// The first decompiler mark in `text` (see the module docs), as a violation of `path`.
pub fn check_text(path: &str, text: &str) -> Option<Violation> {
    static MARK: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let mark = MARK.get_or_init(|| {
        regex::Regex::new(r"\bFUN_[0-9A-Fa-f]{8}\b|\b0x10[0-9A-Fa-f]{6}\b")
            .expect("a valid pattern")
    });
    text.lines().enumerate().find_map(|(i, line)| {
        mark.find(line).map(|m| Violation {
            path: path.replace('\\', "/"),
            reason: Reason::DecompilerText {
                line: i + 1,
                found: m.as_str().to_string(),
            },
        })
    })
}

/// Split `git ... -z` output (NUL-separated) into paths; empty entries are dropped.
pub fn parse_nul_list(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Which files to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Every tracked file (CI).
    AllTracked,
    /// Files staged for the next commit (pre-commit hook).
    Staged,
}

pub(crate) fn git(root: Option<&str>, args: &[&str]) -> anyhow::Result<Vec<u8>> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(root) = root {
        cmd.current_dir(root);
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to run git {args:?}"))?;
    if !output.status.success() {
        bail!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

/// The top-level directory of the repository containing the current directory.
pub fn repo_root() -> anyhow::Result<String> {
    Ok(
        String::from_utf8(git(None, &["rev-parse", "--show-toplevel"])?)?
            .trim()
            .to_string(),
    )
}

/// Check the repository containing the current directory.
pub fn run(mode: Mode) -> anyhow::Result<Vec<Violation>> {
    let root = String::from_utf8(git(None, &["rev-parse", "--show-toplevel"])?)?
        .trim()
        .to_string();
    let args: &[&str] = match mode {
        Mode::AllTracked => &["ls-files", "-z"],
        Mode::Staged => &["diff", "--cached", "--name-only", "--diff-filter=AM", "-z"],
    };
    let paths = parse_nul_list(&git(Some(&root), args)?);
    let mut violations = Vec::new();
    for path in paths {
        let size = std::fs::metadata(Path::new(&root).join(&path))
            .map(|m| m.len())
            .unwrap_or(0);
        if let Some(v) = check_path(&path, size) {
            violations.push(v);
        } else if is_checked_doc(&path)
            && let Ok(text) = std::fs::read_to_string(Path::new(&root).join(&path))
            && let Some(v) = check_text(&path, &text)
        {
            violations.push(v);
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(path: &str, size: u64) -> Option<Reason> {
        check_path(path, size).map(|v| v.reason)
    }

    #[test]
    fn forbids_gro_dll_exe_case_insensitively() {
        assert!(
            matches!(reason("ALL_PC_01.GRO", 10), Some(Reason::ForbiddenExtension(e)) if e == "gro")
        );
        assert!(
            matches!(reason("Bin/Engine.DLL", 10), Some(Reason::ForbiddenExtension(e)) if e == "dll")
        );
        assert!(matches!(
            reason("tools/Sam2.exe", 10),
            Some(Reason::ForbiddenExtension(_))
        ));
    }

    #[test]
    fn docs_may_not_quote_decompiler_names_or_addresses() {
        let found = |text| check_text("docs/re/x.md", text).map(|v| v.reason);
        assert_eq!(
            found("ok line\nthe comparator FUN_10155940 sorts"),
            Some(Reason::DecompilerText {
                line: 2,
                found: "FUN_10155940".into()
            })
        );
        assert!(matches!(
            found("at 0x1017b370 in Engine.dll"),
            Some(Reason::DecompilerText { line: 1, .. })
        ));
        // Prose about the convention, ordinary hex numbers and colours pass.
        assert_eq!(found("Ghidra names unnamed functions `FUN_...`"), None);
        assert_eq!(
            found("flag 0x10, mask 0x1F, colour 0xff463c26, size 0x100"),
            None
        );
        assert_eq!(found("a 0x10000 byte block"), None);
        assert!(is_checked_doc("docs/re/environment.md"));
        assert!(is_checked_doc("docs\\roadmap\\T01.md"));
        assert!(!is_checked_doc("crates/ss2-core/src/lib.rs"));
        assert!(!is_checked_doc("README.md"));
    }

    #[test]
    fn normalizes_backslashes() {
        assert!(matches!(
            reason("docs\\x.gro", 10),
            Some(Reason::ForbiddenExtension(_))
        ));
        assert_eq!(check_path("docs\\x.gro", 10).unwrap().path, "docs/x.gro");
    }

    #[test]
    fn flags_serious_sam_dir_with_spaces() {
        assert_eq!(
            reason("Serious Sam 2/readme.txt", 10),
            Some(Reason::ProprietaryDir)
        );
        assert_eq!(
            reason("a/serious sam 2/b.txt", 10),
            Some(Reason::ProprietaryDir)
        );
    }

    #[test]
    fn dir_name_must_match_a_whole_component() {
        assert_eq!(reason("docs/Serious Sam 2 notes.md", 10), None);
    }

    #[test]
    fn flags_renamed_blob_by_size() {
        assert_eq!(
            reason("assets/x.bin", 6 * 1024 * 1024),
            Some(Reason::TooLarge {
                bytes: 6 * 1024 * 1024,
                limit: MAX_BYTES
            })
        );
        assert_eq!(reason("assets/x.bin", MAX_BYTES), None);
    }

    #[test]
    fn allows_small_text_and_rs_files() {
        assert_eq!(reason("crates/ss2-core/src/lib.rs", 4000), None);
        assert_eq!(reason("README.md", 100), None);
        assert_eq!(
            reason("crates/ss2-render/assets/Inter-Regular.ttf", 411_640),
            None
        );
    }

    #[test]
    fn testdata_allows_tex_but_not_gro() {
        assert_eq!(reason("testdata/tiny.tex", 100), None);
        assert_eq!(reason("testdata/tiny.mdl", 100), None);
        assert!(matches!(
            reason("testdata/a.gro", 100),
            Some(Reason::ForbiddenExtension(_))
        ));
        assert!(matches!(
            reason("testdata/a.dll", 100),
            Some(Reason::ForbiddenExtension(_))
        ));
    }

    #[test]
    fn example_mods_may_hold_tiny_fixtures_but_never_archives_or_big_files() {
        assert_eq!(
            reason(
                "examples/mods/HelloMod/Content/HelloMod/Textures/Checker.tex",
                1000
            ),
            None
        );
        assert!(matches!(
            reason("examples/mods/HelloMod/HelloMod.gro", 100),
            Some(Reason::ForbiddenExtension(_))
        ));
        assert!(matches!(
            reason("examples/mods/HelloMod/Bin/Game.dll", 100),
            Some(Reason::ForbiddenExtension(_))
        ));
        assert!(matches!(
            reason("examples/mods/HelloMod/big.tex", TESTDATA_MAX_BYTES + 1),
            Some(Reason::TooLarge { limit, .. }) if limit == TESTDATA_MAX_BYTES
        ));
        // Only the folder `examples/mods/` is exempt, not any `examples/` folder.
        assert!(matches!(
            reason("examples/other/a.tex", 100),
            Some(Reason::ForbiddenExtension(_))
        ));
    }

    #[test]
    fn testdata_size_limit_is_256_kib() {
        assert_eq!(reason("testdata/ok.tex", TESTDATA_MAX_BYTES), None);
        assert!(matches!(
            reason("testdata/big.tex", TESTDATA_MAX_BYTES + 1),
            Some(Reason::TooLarge { limit, .. }) if limit == TESTDATA_MAX_BYTES
        ));
    }

    #[test]
    fn nul_list_keeps_spaces_and_skips_empty() {
        let paths = parse_nul_list(b"Serious Sam 2/a b.gro\0docs/x.md\0\0");
        assert_eq!(
            paths,
            vec!["Serious Sam 2/a b.gro".to_string(), "docs/x.md".to_string()]
        );
    }
}
