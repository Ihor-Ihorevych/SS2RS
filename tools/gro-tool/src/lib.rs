//! Library behind the `gro-tool` command: matching, listing, statistics and extraction over any
//! [`Vfs`] (normally a mounted install, see [`ss2_vfs::mount_install`]).

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use regex::{Regex, RegexBuilder};
use ss2_vfs::{EntryKind, Vfs, VfsPath};

/// Matches [`VfsPath`]s (always lower-case, `/`-separated) against a glob or a regex,
/// case-insensitively.
pub struct Matcher {
    re: Regex,
    /// Glob without a `/`: match the file name only, wherever it is.
    name_only: bool,
}

/// Translate a glob into an anchored regex source: `*` stays inside one segment, `**` crosses
/// segments (`**/` also matches zero segments), `?` is one non-slash character; everything else
/// is literal.
fn glob_to_regex(glob: &str) -> String {
    let chars: Vec<char> = glob.chars().collect();
    let mut re = String::from("^");
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                if chars.get(i) == Some(&'/') {
                    re.push_str("(?:.*/)?");
                    i += 1;
                } else {
                    re.push_str(".*");
                }
            }
            '*' => {
                re.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                re.push_str("[^/]");
                i += 1;
            }
            c => {
                re.push_str(&regex::escape(&c.to_string()));
                i += 1;
            }
        }
    }
    re.push('$');
    re
}

impl Matcher {
    /// A glob matcher (`*.tex`, `content/**/menus/*.rsc`, …).
    ///
    /// # Errors
    /// If the glob cannot be compiled (practically never).
    pub fn glob(pattern: &str) -> anyhow::Result<Self> {
        let re = RegexBuilder::new(&glob_to_regex(pattern))
            .case_insensitive(true)
            .build()
            .with_context(|| format!("invalid glob {pattern:?}"))?;
        Ok(Self {
            re,
            name_only: !pattern.contains('/'),
        })
    }

    /// A regex matcher, searched (unanchored) in the full path.
    ///
    /// # Errors
    /// If the regex is invalid.
    pub fn regex(pattern: &str) -> anyhow::Result<Self> {
        let re = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .with_context(|| format!("invalid regex {pattern:?}"))?;
        Ok(Self {
            re,
            name_only: false,
        })
    }

    /// Does `path` match?
    pub fn is_match(&self, path: &VfsPath) -> bool {
        let text = if self.name_only {
            path.file_name().unwrap_or("")
        } else {
            path.as_str()
        };
        self.re.is_match(text)
    }
}

/// Human-readable size in binary units: `0 B`, `1023 B`, `1.0 KiB`, `1.2 GiB`.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = "B";
    for next in UNITS {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    format!("{value:.1} {unit}")
}

/// A file in the VFS and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// Normalized path.
    pub path: VfsPath,
    /// Size in bytes.
    pub size: u64,
}

/// Every file (no directories) below the root, sorted by path.
///
/// # Errors
/// Propagates VFS errors.
pub fn walk_files(vfs: &dyn Vfs) -> ss2_core::Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let mut stack = vec![VfsPath::root()];
    while let Some(dir) = stack.pop() {
        for entry in vfs.list(&dir)? {
            let path = dir.join(&entry.name)?;
            match entry.kind {
                EntryKind::Dir => stack.push(path),
                EntryKind::File => {
                    let size = vfs.stat(&path)?.size;
                    files.push(FileInfo { path, size });
                }
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Files, total bytes and extension for [`extension_stats`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtStat {
    /// Lower-case extension without the dot, or `(none)`.
    pub ext: String,
    /// Number of files.
    pub files: usize,
    /// Total size in bytes.
    pub bytes: u64,
}

/// Per-extension totals, largest total size first (ties by extension).
pub fn extension_stats(files: &[FileInfo]) -> Vec<ExtStat> {
    let mut map: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    for f in files {
        let ext = f.path.extension().unwrap_or("(none)").to_string();
        let e = map.entry(ext).or_default();
        e.0 += 1;
        e.1 += f.size;
    }
    let mut stats: Vec<_> = map
        .into_iter()
        .map(|(ext, (files, bytes))| ExtStat { ext, files, bytes })
        .collect();
    stats.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.ext.cmp(&b.ext)));
    stats
}

/// Copy one file out of the VFS to `out_dir/<normalized path>`, creating folders as needed and
/// streaming the data (large videos are not loaded into memory). Existing files are only
/// replaced when `overwrite` is set. Returns the path written.
///
/// A [`VfsPath`] can never contain `..` or a drive letter, so the target always stays inside
/// `out_dir`.
///
/// # Errors
/// If `path` is not a file, the target exists (and `overwrite` is false) or an I/O error occurs.
pub fn extract_file(
    vfs: &dyn Vfs,
    path: &VfsPath,
    out_dir: &Path,
    overwrite: bool,
) -> anyhow::Result<PathBuf> {
    let stat = vfs.stat(path)?;
    if stat.kind != EntryKind::File {
        bail!("{path} is a directory, not a file");
    }
    let mut target = out_dir.to_path_buf();
    target.extend(path.as_str().split('/'));
    if target.exists() && !overwrite {
        bail!(
            "{} already exists (use --force to overwrite)",
            target.display()
        );
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    let mut source = vfs.open(path)?;
    let mut file =
        File::create(&target).with_context(|| format!("cannot create {}", target.display()))?;
    io::copy(&mut source, &mut file)
        .with_context(|| format!("cannot write {}", target.display()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_vfs::{MemoryVfs, VfsPath};

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    fn glob(g: &str) -> Matcher {
        Matcher::glob(g).unwrap()
    }

    #[test]
    fn star_stays_within_one_path_segment() {
        let m = glob("content/*.tex");
        assert!(m.is_match(&p("content/a.tex")));
        assert!(!m.is_match(&p("content/sub/a.tex")));
        assert!(!m.is_match(&p("other/a.tex")));
    }

    #[test]
    fn double_star_crosses_segments() {
        let m = glob("content/**/*.tex");
        assert!(m.is_match(&p("content/a/b/c.tex")));
        assert!(!m.is_match(&p("content/a/b/c.mdl")));
        assert!(glob("content/**").is_match(&p("content/a/b/c.tex")));
    }

    #[test]
    fn a_pattern_without_a_slash_matches_the_file_name_anywhere() {
        let m = glob("*.tex");
        assert!(m.is_match(&p("a/b/c.tex")));
        assert!(m.is_match(&p("c.tex")));
        assert!(!m.is_match(&p("a/b/c.tex.bak")));
    }

    #[test]
    fn question_mark_matches_one_character_that_is_not_a_slash() {
        let m = glob("wall?.tex");
        assert!(m.is_match(&p("wall1.tex")));
        assert!(!m.is_match(&p("wall12.tex")));
        assert!(!glob("a?b").is_match(&p("a/b")));
    }

    #[test]
    fn globs_ignore_case() {
        assert!(glob("*.TEX").is_match(&p("Content/Wall.Tex")));
        assert!(glob("CONTENT/WALL.TEX").is_match(&p("content/wall.tex")));
    }

    #[test]
    fn regex_metacharacters_in_a_glob_are_literal() {
        assert!(glob("a+b(1).txt").is_match(&p("a+b(1).txt")));
        assert!(!glob("a+b.txt").is_match(&p("aab.txt")));
        assert!(glob("[x].txt").is_match(&p("[x].txt")));
    }

    #[test]
    fn regex_mode_searches_the_full_path_and_reports_bad_patterns() {
        let m = Matcher::regex(r"textures/.*wall\d+").unwrap();
        assert!(m.is_match(&p("content/textures/brick_wall12.tex")));
        assert!(!m.is_match(&p("content/models/wall.mdl")));
        assert!(Matcher::regex("(unclosed").is_err());
    }

    #[test]
    fn sizes_are_formatted_in_binary_units() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(1_048_576), "1.0 MiB");
        assert_eq!(format_size(1_310_000_000), "1.2 GiB");
        assert!(format_size(u64::MAX).ends_with("EiB"));
    }

    fn sample() -> MemoryVfs {
        MemoryVfs::from_files([
            ("Content/B/z.tex", b"zzzz".as_slice()),
            ("Content/a.mdl", b"aa".as_slice()),
            ("Content/B/y.tex", b"yy".as_slice()),
            ("README", b"r".as_slice()),
        ])
        .unwrap()
    }

    #[test]
    fn walking_returns_every_file_sorted_by_path_and_no_directories() {
        let files = walk_files(&sample()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.path.as_str().to_string()).collect();
        assert_eq!(
            names,
            [
                "content/a.mdl",
                "content/b/y.tex",
                "content/b/z.tex",
                "readme"
            ]
        );
        assert_eq!(files[2].size, 4);
    }

    #[test]
    fn extension_statistics_group_by_extension_and_sort_by_size() {
        let stats = extension_stats(&walk_files(&sample()).unwrap());
        assert_eq!(
            stats[0],
            ExtStat {
                ext: "tex".into(),
                files: 2,
                bytes: 6
            }
        );
        assert!(stats.contains(&ExtStat {
            ext: "mdl".into(),
            files: 1,
            bytes: 2
        }));
        assert!(stats.contains(&ExtStat {
            ext: "(none)".into(),
            files: 1,
            bytes: 1
        }));
    }

    #[test]
    fn extracting_writes_under_the_output_folder_and_creates_directories() {
        let out = tempfile::tempdir().unwrap();
        let written = extract_file(&sample(), &p("content/b/z.tex"), out.path(), false).unwrap();
        assert_eq!(written, out.path().join("content").join("b").join("z.tex"));
        assert_eq!(std::fs::read(&written).unwrap(), b"zzzz");
    }

    #[test]
    fn extracting_refuses_to_overwrite_unless_asked() {
        let out = tempfile::tempdir().unwrap();
        extract_file(&sample(), &p("readme"), out.path(), false).unwrap();
        std::fs::write(out.path().join("readme"), b"changed").unwrap();
        assert!(extract_file(&sample(), &p("readme"), out.path(), false).is_err());
        assert_eq!(
            std::fs::read(out.path().join("readme")).unwrap(),
            b"changed"
        );
        extract_file(&sample(), &p("readme"), out.path(), true).unwrap();
        assert_eq!(std::fs::read(out.path().join("readme")).unwrap(), b"r");
    }

    #[test]
    fn extracting_a_directory_or_a_missing_file_is_an_error() {
        let out = tempfile::tempdir().unwrap();
        assert!(extract_file(&sample(), &p("content"), out.path(), false).is_err());
        assert!(extract_file(&sample(), &p("nope.tex"), out.path(), false).is_err());
    }
}
