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

const MOD_FILE: &str = "MODULES = Sam2Game <TOOLS>Sam2GameGui ;";

fn write_gro_with(dir: &Path, name: &str, files: &[(&str, &str)]) {
    let mut b = ZipBuilder::new();
    for (n, d) in files {
        b = b.add(*n, d.as_bytes().to_vec());
    }
    std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
}

/// An install with one mod `TestMod` whose `.nfo` points at `model`.
fn install_with_mod(dir: &Path, model: &str) {
    install(dir);
    let mod_dir = dir.join("Content/TestMod");
    std::fs::create_dir_all(&mod_dir).unwrap();
    std::fs::write(dir.join("Content/TestMod.mod"), MOD_FILE).unwrap();
    std::fs::write(
        dir.join("Content/TestMod.nfo"),
        "NAME = \"Test Mod\" ;
AUTHOR = \"Me\" ;
",
    )
    .unwrap();
    let nfo = format!(
        "NAME = \"Pal\"
MODEL = \"{model}\"
"
    );
    write_gro_with(
        &mod_dir,
        "TestMod.gro",
        &[
            ("Content/TestMod/pal.nfo", nfo.as_str()),
            ("Content/TestMod/own.mdl", "model"),
        ],
    );
}

#[test]
fn a_clean_mod_is_checked_after_the_install_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    install_with_mod(dir.path(), "Content/TestMod/own.mdl");
    let out = run(&[dir.path().to_str().unwrap(), "--mod", "TestMod"], None);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    assert!(stdout.contains("Install looks complete"), "{stdout}");
    assert!(stdout.contains("SS2RS mod check"), "{stdout}");
    assert!(stdout.contains("Mod testmod (\"Test Mod\")"), "{stdout}");
    assert!(stdout.contains("no findings"), "{stdout}");
}

#[test]
fn a_mod_with_a_missing_asset_exits_one_and_names_the_code() {
    let dir = tempfile::tempdir().unwrap();
    install_with_mod(dir.path(), "Content/TestMod/gone.mdl");
    let out = run(&["--mod", "testmod", dir.path().to_str().unwrap()], None);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("ERROR   MOD030"), "{stdout}");
    assert!(stdout.contains("gone.mdl"), "{stdout}");
    assert!(stdout.contains("1 error"), "{stdout}");
}

#[test]
fn an_unknown_mod_is_an_error_finding_not_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path());
    let out = run(&["--mod", "Nope", dir.path().to_str().unwrap()], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout).contains("MOD091"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn all_mods_checks_every_mod_folder_and_says_so_when_there_is_none() {
    let dir = tempfile::tempdir().unwrap();
    install_with_mod(dir.path(), "Content/TestMod/own.mdl");
    let out = run(
        &[
            "--all-mods",
            "--mod",
            "TESTMOD",
            dir.path().to_str().unwrap(),
        ],
        None,
    );
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert_eq!(
        stdout
            .matches(
                "
Mod testmod"
            )
            .count(),
        1,
        "listed once: {stdout}"
    );
    let none = tempfile::tempdir().unwrap();
    install(none.path());
    let out = run(&["--all-mods", none.path().to_str().unwrap()], None);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        text(&out.stdout).contains("No mod folders"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn a_loose_mod_folder_can_be_checked_against_the_install() {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path());
    let work = tempfile::tempdir().unwrap();
    let root = work.path().join("MyMod");
    std::fs::create_dir_all(root.join("Content/MyMod")).unwrap();
    std::fs::write(root.join("Content/MyMod.mod"), MOD_FILE).unwrap();
    std::fs::write(
        root.join("Content/MyMod.nfo"),
        "NAME = \"Mine\"
AUTHOR = \"Me\"
",
    )
    .unwrap();
    let out = run(
        &[
            dir.path().to_str().unwrap(),
            "--mod-dir",
            root.to_str().unwrap(),
        ],
        None,
    );
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("Mod mymod (\"Mine\")"), "{stdout}");
}

#[test]
fn mod_options_need_a_value_and_help_lists_them() {
    let out = run(&["--mod"], None);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("--mod needs a value"),
        "{}",
        text(&out.stderr)
    );
    let out = run(&["--mod-dir", ""], None);
    assert_eq!(out.status.code(), Some(2));
    let help = text(&run(&["--help"], None).stdout);
    for word in [
        "--mod NAME",
        "--mod-dir DIR",
        "--all-mods",
        "MOD_VALIDATION",
    ] {
        assert!(help.contains(word), "{word}: {help}");
    }
}

#[test]
fn a_broken_install_makes_the_mod_check_one_finding() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(&[dir.path().to_str().unwrap(), "--mod", "x"], None);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout.contains("no .gro archives"), "{stdout}");
    assert!(stdout.contains("MOD090"), "{stdout}");
}

/// Needs the player's own install (`SS2_DATA_DIR`); skipped without it.
#[test]
fn the_real_mods_pass_through_the_command() {
    let dir = ss2_testkit::require_game_data!();
    let out = run(&[dir.to_str().unwrap(), "--all-mods"], None);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("SS2RS mod check"), "{stdout}");
    assert!(stdout.is_ascii());
}
