//! The `ss2-doctor` executable (T11-013): arguments, output and exit codes.

use std::path::Path;
use std::process::{Command, Output};

use ss2_testkit::fixtures::ZipBuilder;

fn write_gro(dir: &Path, name: &str) {
    let bytes = ZipBuilder::new()
        .add("a.txt", b"x".to_vec())
        .finish()
        .unwrap();
    std::fs::write(dir.join(name), bytes).unwrap();
}

fn install(dir: &Path) {
    for name in [
        "All_PC_01.gro",
        "All_PC_02.gro",
        "All_PC_03.gro",
        "Movies_PC_01.gro",
        "Movies_PC_02.gro",
        "Patch_02_100_800000.gro",
    ] {
        write_gro(dir, name);
    }
}

fn run(args: &[&str], data_dir: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ss2-doctor"));
    cmd.args(args).env_remove("SS2_DATA_DIR");
    if let Some(dir) = data_dir {
        cmd.env("SS2_DATA_DIR", dir);
    }
    cmd.output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_complete_install_exits_zero_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path());
    let out = run(&[dir.path().to_str().unwrap()], None);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("Install looks complete"), "{stdout}");
    assert!(
        stdout.contains("Patch level: 2.100 (build 800000)"),
        "{stdout}"
    );
}

#[test]
fn a_broken_install_exits_one_and_names_the_problem() {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path());
    std::fs::remove_file(dir.path().join("All_PC_02.gro")).unwrap();
    let out = run(&[dir.path().to_str().unwrap()], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout).contains("All_PC_02.gro is missing"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn without_an_argument_the_data_dir_variable_is_used() {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path());
    let out = run(&[], Some(dir.path()));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stdout));
}

#[test]
fn without_an_argument_and_without_the_variable_it_explains_and_exits_two() {
    let out = run(&[], None);
    assert_eq!(out.status.code(), Some(2));
    let err = text(&out.stderr);
    assert!(
        err.contains("SS2_DATA_DIR") && err.contains("Usage"),
        "{err}"
    );
}

#[test]
fn help_and_version_work_and_unknown_options_are_refused() {
    let help = run(&["--help"], None);
    assert_eq!(help.status.code(), Some(0));
    assert!(
        text(&help.stdout).contains("Usage: ss2-doctor"),
        "{}",
        text(&help.stdout)
    );
    let version = run(&["--version"], None);
    assert_eq!(version.status.code(), Some(0));
    assert!(text(&version.stdout).starts_with("ss2-doctor "));
    let bad = run(&["--frobnicate"], None);
    assert_eq!(bad.status.code(), Some(2));
    assert!(text(&bad.stderr).contains("--frobnicate"));
    let two = run(&["a", "b"], None);
    assert_eq!(two.status.code(), Some(2));
}

/// Needs the player's own install (`SS2_DATA_DIR`); skipped without it.
#[test]
fn the_real_install_passes_and_reports_a_patch_level() {
    let dir = ss2_testkit::require_game_data!();
    let out = run(&[dir.to_str().unwrap()], None);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("Install looks complete"), "{stdout}");
    assert!(stdout.contains("Patch level: 2."), "{stdout}");
}
