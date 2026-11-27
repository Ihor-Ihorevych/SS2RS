//! Mounting a whole Serious Sam 2 install: every `.gro` archive of a folder, stacked in load
//! order (roadmap T02-002).
//!
//! The game ships base archives (`All_PC_*.gro`, `Movies_PC_*.gro`) and a series of patches
//! (`Patch_02_068.gro`, `Patch_02_080_210639.gro`, …, `Patch_02_100_800000.gro`). Patches
//! replace files of earlier archives, so they must be mounted after the base archives, in
//! **numeric** order of their version numbers (`Patch_02_100` is newer than `Patch_02_068`).

use std::path::Path;

use ss2_core::{Error, Result};

use crate::{ArchiveVfs, LayeredVfs};

/// One run of the name: text (compared case-insensitively) or a number (compared by value).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Text(String),
    /// Digits without leading zeros: a longer number is larger; equal lengths compare as text.
    Num {
        len: usize,
        digits: String,
    },
}

fn parts(lower: &str) -> Vec<Part> {
    let mut out = Vec::new();
    let mut chars = lower.chars().peekable();
    while let Some(&c) = chars.peek() {
        let numeric = c.is_ascii_digit();
        let mut run = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() != numeric {
                break;
            }
            run.push(c);
            chars.next();
        }
        out.push(if numeric {
            let trimmed = run.trim_start_matches('0');
            let digits = if trimmed.is_empty() {
                "0".to_string()
            } else {
                trimmed.to_string()
            };
            Part::Num {
                len: digits.len(),
                digits,
            }
        } else {
            Part::Text(run)
        });
    }
    out
}

/// Load order of archive file names: **earlier entries are overridden by later ones**.
///
/// Base archives (anything not starting with `patch`, case-insensitive) come first, then patches;
/// within each group names are compared "naturally" (numbers by value). The result is
/// deterministic for any input order.
pub fn archive_load_order(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut keyed: Vec<_> = names
        .into_iter()
        .map(|name| {
            let lower = name.to_lowercase();
            let group = u8::from(lower.starts_with("patch"));
            (group, parts(&lower), lower, name)
        })
        .collect();
    keyed.sort();
    keyed.into_iter().map(|(_, _, _, name)| name).collect()
}

/// Mount every `*.gro` file directly inside `dir` (case-insensitive extension) as one
/// [`LayeredVfs`], in [`archive_load_order`]. Sub-folders and other files are ignored.
///
/// Archive indexes are reused from `SS2RS/archive-index` under the user's OS cache directory
/// when available. Missing, stale or unreadable indexes are rebuilt from the archives.
///
/// Strict on purpose: if **any** archive is unreadable or corrupt the whole mount fails, because
/// silently skipping a patch would serve the wrong version of files.
///
/// # Errors
/// [`Error::Io`] if `dir` cannot be read; [`Error::NotFound`] if it contains no `.gro` archive
/// (usually the wrong folder: it should contain `All_PC_01.gro`); [`Error::Invalid`] naming the
/// first archive that is not a valid ZIP file.
pub fn mount_install(dir: impl AsRef<Path>) -> Result<LayeredVfs> {
    let dir = dir.as_ref();
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| Error::io(dir, e))? {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let path = entry.path();
        let is_gro = path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("gro"));
        if is_gro && path.is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    if names.is_empty() {
        return Err(Error::NotFound(format!(
            "no .gro archives in {}: point at the folder that contains All_PC_01.gro",
            dir.display()
        )));
    }
    let mut vfs = LayeredVfs::new();
    let cache_dir = ArchiveVfs::cache_dir();
    for name in archive_load_order(names) {
        let archive = ArchiveVfs::from_cached_file(&dir.join(&name), cache_dir.as_deref())?;
        vfs.mount(name, archive);
    }
    Ok(vfs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryKind, Vfs, VfsPath};
    use ss2_core::Error;
    use ss2_testkit::fixtures::ZipBuilder;

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    fn order(names: &[&str]) -> Vec<String> {
        archive_load_order(names.iter().map(|s| s.to_string()))
    }

    fn write_gro(dir: &std::path::Path, name: &str, files: &[(&str, &str)]) {
        let mut b = ZipBuilder::new();
        for (n, d) in files {
            b = b.add(*n, d.as_bytes().to_vec());
        }
        std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
    }

    #[test]
    fn base_archives_come_before_patches_and_patches_sort_numerically() {
        let got = order(&[
            "Patch_02_100_800000.gro",
            "All_PC_02.gro",
            "Patch_02_068.gro",
            "Movies_PC_01.gro",
            "All_PC_01.gro",
            "Patch_02_080_210639.gro",
            "patch_02_090_593613.GRO",
            "Patch_02_080_243026.gro",
        ]);
        assert_eq!(
            got,
            [
                "All_PC_01.gro",
                "All_PC_02.gro",
                "Movies_PC_01.gro",
                "Patch_02_068.gro",
                "Patch_02_080_210639.gro",
                "Patch_02_080_243026.gro",
                "patch_02_090_593613.GRO",
                "Patch_02_100_800000.gro",
            ]
        );
    }

    #[test]
    fn numbers_compare_by_value_not_alphabetically() {
        assert_eq!(
            order(&["Patch_10.gro", "Patch_2.gro", "Patch_9.gro"]),
            ["Patch_2.gro", "Patch_9.gro", "Patch_10.gro"]
        );
        // 100 must follow 68 although "1" < "6" alphabetically.
        assert_eq!(
            order(&["Patch_100.gro", "Patch_068.gro"]),
            ["Patch_068.gro", "Patch_100.gro"]
        );
    }

    #[test]
    fn ordering_is_total_and_deterministic_for_equal_numbers() {
        let a = order(&["Patch_01.gro", "Patch_1.gro"]);
        let b = order(&["Patch_1.gro", "Patch_01.gro"]);
        assert_eq!(a, b);
    }

    #[test]
    fn mounts_every_gro_and_the_highest_patch_wins() {
        let dir = tempfile::tempdir().unwrap();
        write_gro(
            dir.path(),
            "All_PC_01.gro",
            &[("Content/x.txt", "base"), ("Content/only_base.txt", "b")],
        );
        write_gro(
            dir.path(),
            "Patch_02_100_800000.gro",
            &[("content/X.TXT", "p100")],
        );
        write_gro(
            dir.path(),
            "Patch_02_068.gro",
            &[("Content/x.txt", "p68"), ("Content/new68.txt", "n")],
        );
        std::fs::write(dir.path().join("readme.txt"), "not an archive").unwrap();
        std::fs::create_dir(dir.path().join("folder.gro")).unwrap();

        let v = mount_install(dir.path()).unwrap();
        assert_eq!(v.read(&p("content/x.txt")).unwrap(), b"p100");
        assert_eq!(
            v.source_of(&p("content/x.txt")),
            Some("Patch_02_100_800000.gro")
        );
        assert_eq!(v.read(&p("content/only_base.txt")).unwrap(), b"b");
        assert_eq!(v.read(&p("content/new68.txt")).unwrap(), b"n");
        assert_eq!(
            v.layer_names(),
            [
                "All_PC_01.gro",
                "Patch_02_068.gro",
                "Patch_02_100_800000.gro"
            ]
        );
        let names: Vec<_> = v
            .list(&p("content"))
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["new68.txt", "only_base.txt", "x.txt"]);
        assert_eq!(v.stat(&p("content")).unwrap().kind, EntryKind::Dir);
    }

    #[test]
    fn a_folder_without_archives_is_an_error_that_names_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let err = mount_install(dir.path()).err().expect("must fail");
        assert!(matches!(err, Error::NotFound(_)), "{err:?}");
        assert!(
            err.to_string().contains(&dir.path().display().to_string()),
            "{err}"
        );
    }

    #[test]
    fn a_missing_folder_is_an_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = mount_install(dir.path().join("nope"))
            .err()
            .expect("must fail");
        assert!(matches!(err, Error::Io { .. }), "{err:?}");
    }

    #[test]
    fn one_corrupt_archive_fails_the_whole_mount_and_names_it() {
        let dir = tempfile::tempdir().unwrap();
        write_gro(dir.path(), "All_PC_01.gro", &[("a.txt", "1")]);
        std::fs::write(dir.path().join("Patch_02_068.gro"), b"definitely not a zip").unwrap();
        let err = mount_install(dir.path()).err().expect("must fail");
        assert!(matches!(err, Error::Invalid { .. }), "{err:?}");
        assert!(err.to_string().contains("Patch_02_068.gro"), "{err}");
    }

    /// Data-gated: the real retail install (all patches). Skips without `SS2_DATA_DIR`.
    #[test]
    fn the_real_install_mounts_and_patch_overrides_apply() {
        let dir = ss2_testkit::require_game_data!();
        let started = std::time::Instant::now();
        let v = mount_install(&dir).unwrap();
        let mounted_in = started.elapsed();

        // Count distinct files by walking the merged tree.
        let mut stack = vec![VfsPath::root()];
        let mut files = 0usize;
        while let Some(d) = stack.pop() {
            for e in v.list(&d).unwrap() {
                let child = d.join(&e.name).unwrap();
                match e.kind {
                    EntryKind::Dir => stack.push(child),
                    EntryKind::File => files += 1,
                }
            }
        }
        eprintln!(
            "mounted {} archives in {mounted_in:?}; {files} distinct files",
            v.layer_names().len()
        );
        assert!(files > 10_000, "only {files} files");
        assert!(mounted_in.as_secs() < 30, "mounting took {mounted_in:?}");

        let menu = p("content/serioussam2/menus/cmainmenu.rsc");
        assert!(v.stat(&menu).unwrap().size > 0);
        let head = v.read(&menu).unwrap();
        assert_eq!(&head[..8], b"CTSEMETA");

        // Known retail override: this file exists in base and in a patch; the patch must win.
        let overlay = p("content/serioussam2/menus/cmsoverlay.rsc");
        assert!(
            v.source_of(&overlay)
                .is_some_and(|s| s.to_lowercase().starts_with("patch"))
        );
    }
}
