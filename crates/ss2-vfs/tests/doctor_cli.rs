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

/// An install with two mods: `Base2` (data only) and `Addon` (needs `Base2`, has a native module).
fn install_with_two_mods(dir: &Path) {
    install(dir);
    let content = dir.join("Content");
    for (name, module, extra) in [
        ("Base2", "Sam2Game", ""),
        ("Addon", "AddonGame", "REQUIRES_MOD = \"base2.mod\" ;\n"),
    ] {
        std::fs::create_dir_all(content.join(name)).unwrap();
        std::fs::write(
            content.join(format!("{name}.mod")),
            format!("MODULES = {module} ;"),
        )
        .unwrap();
        std::fs::write(
            content.join(format!("{name}.nfo")),
            format!("NAME = \"{name} mod\" ;\n{extra}"),
        )
        .unwrap();
        write_gro_with(
            &content.join(name),
            &format!("{name}.gro"),
            &[
                ("Content/shared.txt", name),
                (&format!("Content/{name}.txt"), "own"),
            ],
        );
    }
}

#[test]
fn list_mods_prints_the_mods_with_what_they_need() {
    let dir = tempfile::tempdir().unwrap();
    install_with_two_mods(dir.path());
    let out = run(&[dir.path().to_str().unwrap(), "--list-mods"], None);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stdout));
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains("Mods in the install: 2; the game starts: (not selected)"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "addon: \"Addon mod\"; 1 archive; native module AddonGame (content only); needs: base2"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("base2: \"Base2 mod\"; 1 archive; data only; needs: -"),
        "{stdout}"
    );
}

#[test]
fn load_order_adds_dependencies_and_names_the_winner() {
    let dir = tempfile::tempdir().unwrap();
    install_with_two_mods(dir.path());
    let out = run(
        &[
            dir.path().to_str().unwrap(),
            "--load-order",
            "--mod",
            "addon",
        ],
        None,
    );
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains("  1. base2 (needed by addon)\n  2. addon (requested)\n"),
        "{stdout}"
    );
    assert!(stdout.contains("addon wins over base2: 1 file (identical: 0, different: 1, not compared: 0): content/shared.txt"), "{stdout}");
    assert!(
        stdout.contains("Note: addon names the native module AddonGame"),
        "{stdout}"
    );
}

#[test]
fn load_order_without_mods_says_only_the_base_and_a_bad_request_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    install_with_two_mods(dir.path());
    let out = run(&[dir.path().to_str().unwrap(), "--load-order"], None);
    assert!(
        text(&out.stdout).contains("Load order: only the base game."),
        "{}",
        text(&out.stdout)
    );
    assert_eq!(out.status.code(), Some(0));
    let out = run(
        &[
            dir.path().to_str().unwrap(),
            "--load-order",
            "--mod",
            "ghost",
        ],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout)
            .contains("The load order cannot be made: the mod `ghost` is not installed"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn help_lists_the_mod_loading_options() {
    let help = text(&run(&["--help"], None).stdout);
    for word in ["--list-mods", "--load-order", "MOD_LOADING"] {
        assert!(help.contains(word), "{word}: {help}");
    }
}
