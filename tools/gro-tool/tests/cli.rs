//! End-to-end tests: run the real `gro-tool` binary against a synthetic install folder.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ss2_testkit::fixtures::ZipBuilder;

fn write_gro(dir: &Path, name: &str, files: &[(&str, &str)]) {
    let mut b = ZipBuilder::new();
    for (n, d) in files {
        b = b.add(*n, d.as_bytes().to_vec());
    }
    std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
}

/// An install with a base archive and two patches (the highest patch overrides `content/x.txt`).
fn install() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_gro(
        dir.path(),
        "All_PC_01.gro",
        &[
            ("Content/x.txt", "base"),
            ("Content/Textures/wall.tex", "pixels"),
            ("Content/m.mdl", "mesh"),
        ],
    );
    write_gro(dir.path(), "Patch_02_068.gro", &[("Content/y.txt", "p68")]);
    write_gro(dir.path(), "Patch_02_100.gro", &[("content/X.TXT", "p100")]);
    dir
}

fn tool(args: &[&str], data: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gro-tool"));
    cmd.env_remove("SS2_DATA_DIR");
    if let Some(d) = data {
        cmd.arg("--data").arg(d);
    }
    cmd.args(args).output().expect("failed to run gro-tool")
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn list_prints_every_file_sorted() {
    let d = install();
    let o = tool(&["list"], Some(d.path()));
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(
        text(&o).lines().collect::<Vec<_>>(),
        [
            "content/m.mdl",
            "content/textures/wall.tex",
            "content/x.txt",
            "content/y.txt"
        ]
    );
}

#[test]
fn list_filters_by_glob_and_by_regex() {
    let d = install();
    assert_eq!(
        text(&tool(&["list", "*.txt"], Some(d.path())))
            .lines()
            .count(),
        2
    );
    let o = tool(&["list", "--regex", r"textures/.*\.tex$"], Some(d.path()));
    assert_eq!(text(&o).trim(), "content/textures/wall.tex");
}

#[test]
fn long_listing_shows_size_and_the_archive_that_wins() {
    let d = install();
    let o = tool(&["list", "--long", "content/x.txt"], Some(d.path()));
    let line = text(&o);
    assert!(line.contains("Patch_02_100.gro"), "{line}");
    assert!(line.contains("content/x.txt"), "{line}");
    assert!(line.contains(" 4 "), "size 4 expected in {line}");
}

#[test]
fn where_names_the_providing_archive() {
    let d = install();
    assert_eq!(
        text(&tool(&["where", "content/x.txt"], Some(d.path()))).trim(),
        "Patch_02_100.gro"
    );
    assert_eq!(
        text(&tool(&["where", "CONTENT\\M.MDL"], Some(d.path()))).trim(),
        "All_PC_01.gro"
    );
    let o = tool(&["where", "content/nope"], Some(d.path()));
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn cat_writes_the_winning_file_bytes_to_stdout() {
    let d = install();
    let o = tool(&["cat", "Content\\X.txt"], Some(d.path()));
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(o.stdout, b"p100");
}

#[test]
fn cat_of_a_missing_file_fails_with_a_message() {
    let d = install();
    let o = tool(&["cat", "content/nope.txt"], Some(d.path()));
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("nope.txt"), "{}", err(&o));
}

#[test]
fn find_exits_1_when_nothing_matches_like_grep() {
    let d = install();
    assert!(tool(&["find", "*.tex"], Some(d.path())).status.success());
    let none = tool(&["find", "*.zzz"], Some(d.path()));
    assert_eq!(none.status.code(), Some(1));
    assert!(text(&none).is_empty());
}

#[test]
fn extract_writes_matching_files_and_refuses_to_overwrite_without_force() {
    let d = install();
    let out = tempfile::tempdir().unwrap();
    let out_arg = out.path().to_str().unwrap();
    let o = tool(&["extract", "*.txt", "--out", out_arg], Some(d.path()));
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(
        std::fs::read(out.path().join("content").join("x.txt")).unwrap(),
        b"p100"
    );
    assert_eq!(
        std::fs::read(out.path().join("content").join("y.txt")).unwrap(),
        b"p68"
    );

    let again = tool(&["extract", "*.txt", "--out", out_arg], Some(d.path()));
    assert_eq!(again.status.code(), Some(2));
    assert!(err(&again).contains("--force"), "{}", err(&again));
    assert!(
        tool(
            &["extract", "*.txt", "--out", out_arg, "--force"],
            Some(d.path())
        )
        .status
        .success()
    );
}

#[test]
fn extract_with_no_match_exits_1_and_writes_nothing() {
    let d = install();
    let out = tempfile::tempdir().unwrap();
    let o = tool(
        &["extract", "*.zzz", "--out", out.path().to_str().unwrap()],
        Some(d.path()),
    );
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}

#[test]
fn stats_summarize_files_and_extensions() {
    let d = install();
    let s = text(&tool(&["stats"], Some(d.path())));
    assert!(s.contains("archives: 3"), "{s}");
    assert!(s.contains("files: 4"), "{s}");
    assert!(s.contains(".txt"), "{s}");
    assert!(s.contains(".tex"), "{s}");
}

#[test]
fn a_missing_data_folder_option_explains_how_to_set_it() {
    let o = tool(&["list"], None);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("SS2_DATA_DIR"), "{}", err(&o));
}

#[test]
fn the_wrong_folder_says_it_has_no_archives() {
    let empty = tempfile::tempdir().unwrap();
    let o = tool(&["list"], Some(empty.path()));
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("no .gro archives"), "{}", err(&o));
}

#[test]
fn the_data_folder_can_come_from_the_environment() {
    let d = install();
    let o = Command::new(env!("CARGO_BIN_EXE_gro-tool"))
        .env("SS2_DATA_DIR", d.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(text(&o).lines().count(), 4);
}

/// Data-gated: run against the real install (skips without `SS2_DATA_DIR`).
#[test]
fn the_real_install_lists_the_menu_screens() {
    let dir: PathBuf = ss2_testkit::require_game_data!();
    let o = tool(&["find", "content/serioussam2/menus/*.rsc"], Some(&dir));
    assert!(o.status.success(), "{}", err(&o));
    // Retail 1.0 has 99 distinct menu screens (the archives hold more entries because patches
    // repeat files); require a sane lower bound instead of an exact number.
    assert!(text(&o).lines().count() > 90);
}
