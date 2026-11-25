//! The example mods of `examples/mods/` (T13-010) and the modder documentation in `docs/modding/`.
//!
//! Every folder of `examples/mods/` must pass `ss2-doctor --mod-dir` with exit code 0 and no
//! finding at all, so an example cannot rot when a lint changes. The reports printed in the
//! documentation are the real output of these runs: the tests below fail when a page and the tool
//! disagree. The base game is an invented install made here (no game data is needed).
//!
//! `UPDATE_MODDING_DOCS=1 cargo test -p ss2-vfs --test example_mods -- --nocapture` prints every
//! report, to paste into the pages.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ss2_testkit::fixtures::ZipBuilder;
use ss2_vfs::{
    DirectoryVfs, LayeredVfs, Lint, ModManifest, Nfo, Severity, Vfs, VfsPath, archive_load_order,
    discover_mods, mount_install,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn examples_dir() -> PathBuf {
    repo_root().join("examples/mods")
}

fn page(name: &str) -> String {
    let path = repo_root().join("docs/modding").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

fn write_gro(dir: &Path, name: &str, files: &[(&str, &str)]) {
    let mut b = ZipBuilder::new();
    for (n, d) in files {
        b = b.add(*n, d.as_bytes().to_vec());
    }
    std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
}

/// An invented install: the archive names of the real one (patch level 2.100, build 800000) and a
/// few files that the scenarios replace or refer to. Nothing here comes from the game.
fn fake_install(dir: &Path) {
    write_gro(
        dir,
        "All_PC_01.gro",
        &[
            ("Content/SeriousSam2/Models/Sample/Box.mdl", "box"),
            ("Content/SeriousSam2/Databases/Sample.ep", "parameters"),
            ("Content/SeriousSam2/Textures/Sample.tex", "texture"),
        ],
    );
    for name in [
        "All_PC_02.gro",
        "All_PC_03.gro",
        "Movies_PC_01.gro",
        "Movies_PC_02.gro",
    ] {
        write_gro(dir, name, &[("Content/placeholder.txt", "x")]);
    }
    write_gro(
        dir,
        "Patch_02_100_800000.gro",
        &[("Content/placeholder.txt", "x")],
    );
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn doctor(install: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ss2-doctor"))
        .arg(install)
        .args(args)
        .env_remove("SS2_DATA_DIR")
        .output()
        .unwrap()
}

/// The part of the report that belongs to the mod check (it follows the install check).
fn mod_section(out: &Output) -> String {
    let text = String::from_utf8(out.stdout.clone()).unwrap();
    let at = text.find("SS2RS mod check").expect("a mod report");
    text[at..].to_owned()
}

/// Run `ss2-doctor --mod-dir` on a copy of `HelloMod` after `change` and return (exit code, report).
fn run_variant(change: impl FnOnce(&Path)) -> (i32, String) {
    let work = tempfile::tempdir().unwrap();
    let install = work.path().join("install");
    std::fs::create_dir(&install).unwrap();
    fake_install(&install);
    let root = work.path().join("HelloMod");
    copy_dir(&examples_dir().join("HelloMod"), &root);
    change(&root);
    let out = doctor(&install, &["--mod-dir", root.to_str().unwrap()]);
    let report = mod_section(&out);
    assert!(report.is_ascii(), "{report}");
    (out.status.code().unwrap(), report)
}

#[test]
fn every_example_mod_passes_with_exit_zero_and_no_finding() {
    let work = tempfile::tempdir().unwrap();
    fake_install(work.path());
    let mut seen = 0;
    let mut folders: Vec<_> = std::fs::read_dir(examples_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    for folder in folders {
        let out = doctor(work.path(), &["--mod-dir", folder.to_str().unwrap()]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{}: {stdout}", folder.display());
        assert!(
            stdout.contains("no findings"),
            "{}: {stdout}",
            folder.display()
        );
        assert!(stdout.contains("No problems found (0 notes)."), "{stdout}");
        seen += 1;
    }
    assert!(seen >= 1, "examples/mods/ holds no example mod");
}

#[test]
fn the_example_mod_stays_small_and_original() {
    let mut files = 0;
    let mut stack = vec![examples_dir()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            files += 1;
            let len = std::fs::metadata(&path).unwrap().len();
            assert!(len < 256 * 1024, "{} is {len} bytes", path.display());
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(
                !["gro", "dll", "exe", "wld", "mdl", "bmf", "ogv", "xmv"].contains(&ext),
                "{} is a kind of file an example mod must not hold",
                path.display()
            );
        }
    }
    assert!(files >= 5, "{files}");
}

#[test]
fn the_example_mod_mounts_over_the_base_and_is_listed_with_its_name() {
    let work = tempfile::tempdir().unwrap();
    fake_install(work.path());
    let mut vfs: LayeredVfs = mount_install(work.path()).unwrap();
    // The way the tutorial describes it: the loose mod folder is one more layer on top.
    vfs.mount(
        "HelloMod",
        DirectoryVfs::new(examples_dir().join("HelloMod")).unwrap(),
    );

    let mods = discover_mods(&vfs).unwrap();
    assert_eq!(mods.len(), 1);
    assert_eq!(mods[0].id, "hellomod");
    assert_eq!(mods[0].name, "Hello Mod");
    assert_eq!(
        mods[0].manifest.game_modules().collect::<Vec<_>>(),
        ["Sam2Game"]
    );
    assert_eq!(
        mods[0].thumbnail,
        Some(VfsPath::new("content/hellomod/textures/checker.tex").unwrap())
    );
    // Names are found whatever their case, and the mod's file comes from the mod's layer.
    let texture = VfsPath::new(r"Content\HelloMod\TEXTURES\checker.TEX").unwrap();
    assert_eq!(vfs.source_of(&texture), Some("HelloMod"));
    assert_eq!(&vfs.read(&texture).unwrap()[..8], b"CTSEMETA");
    // The base file is still there, untouched.
    let base = VfsPath::new("content/seriousSam2/models/sample/box.mdl").unwrap();
    assert_eq!(vfs.read(&base).unwrap(), b"box");

    let nfo = Nfo::parse(
        &vfs.read(&VfsPath::new("content/hellomod/levels/helloepisode.nfo").unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(nfo.kind.as_deref(), Some("EPISODE"));
    assert_eq!(nfo.get("name"), Some("Hello Episode"));
}

/// Every report printed in `docs/modding/` and the scenario that makes it.
type Scenario = (&'static str, i32, fn(&Path));

fn scenarios() -> Vec<(&'static str, Scenario)> {
    fn append(root: &Path, file: &str, text: &str) {
        let path = root.join(file);
        let mut old = std::fs::read_to_string(&path).unwrap();
        old.push_str(text);
        std::fs::write(path, old).unwrap();
    }
    fn put(root: &Path, file: &str, text: &str) {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    vec![
        (
            "tutorial-clean",
            ("the example as shipped", 0, (|_| {}) as fn(&Path)),
        ),
        (
            "missing-thumbnail",
            (
                "the picture the descriptor names is not in the mod",
                0,
                |root| {
                    std::fs::remove_file(root.join("Content/HelloMod/Textures/Checker.tex"))
                        .unwrap()
                },
            ),
        ),
        (
            "missing-model",
            ("a MODEL that exists nowhere", 1, |root| {
                append(
                    root,
                    "Content/HelloMod/Levels/HelloEpisode.nfo",
                    "MODEL = \"Content/HelloMod/Models/Box.mdl\"\n",
                )
            }),
        ),
        (
            "level-without-world",
            ("a level description without its .wld", 0, |root| {
                put(
                    root,
                    "Content/HelloMod/Levels/HelloLevel.nfo",
                    "LEVEL\nNAME = \"Hello Level\"\nMAP_TYPE = \"SINGLE; COOP;\"\n",
                )
            }),
        ),
        (
            "unclosed-quote",
            ("a quote that is never closed", 0, |root| {
                put(
                    root,
                    "Content/HelloMod.nfo",
                    "NAME = \"Hello Mod ;\nAUTHOR = \"Me\" ;\n",
                )
            }),
        ),
        (
            "broken-mod-file",
            (
                "the closing semicolon of the .mod file is missing",
                1,
                |root| put(root, "Content/HelloMod.mod", "MODULES = Sam2Game\n"),
            ),
        ),
        (
            "no-mod-file",
            ("no .mod file", 1, |root| {
                std::fs::remove_file(root.join("Content/HelloMod.mod")).unwrap()
            }),
        ),
        (
            "patch-too-new",
            (
                "REQUIRES_PATCH names a patch the install does not have",
                1,
                |root| {
                    append(
                        root,
                        "Content/HelloMod.nfo",
                        "REQUIRES_PATCH = \"2.200\" ;\n",
                    )
                },
            ),
        ),
        (
            "patch-not-a-version",
            ("REQUIRES_PATCH is not a version", 0, |root| {
                append(
                    root,
                    "Content/HelloMod.nfo",
                    "REQUIRES_PATCH = \"latest\" ;\n",
                )
            }),
        ),
        (
            "unsafe-reference",
            (
                "a reference that climbs out of the game folder",
                1,
                |root| {
                    put(
                        root,
                        "Content/HelloMod.nfo",
                        "NAME = \"Hello Mod\" ;\nAUTHOR = \"Me\" ;\nTHUMBNAIL = \"../Checker.tex\" ;\n",
                    )
                },
            ),
        ),
        (
            "wrong-extension",
            ("a thumbnail that is a .png", 0, |root| {
                put(
                    root,
                    "Content/HelloMod/Textures/Checker.png",
                    "not a texture",
                );
                put(
                    root,
                    "Content/HelloMod.nfo",
                    "NAME = \"Hello Mod\" ;\nAUTHOR = \"Me\" ;\nTHUMBNAIL = \"Content/HelloMod/Textures/Checker.png\" ;\n",
                );
            }),
        ),
        (
            "native-code",
            ("a .dll inside the mod", 0, |root| {
                put(root, "Content/HelloMod/Helper.dll", "not code")
            }),
        ),
        (
            "own-game-module",
            ("the .mod names a game module of its own", 0, |root| {
                put(root, "Content/HelloMod.mod", "MODULES = HelloModGame ;\n")
            }),
        ),
        (
            "files-outside-content",
            (
                "a README inside the mod folder, outside Content/",
                0,
                |root| put(root, "README.txt", "hello"),
            ),
        ),
        (
            "replaces-base-parameters",
            ("a file that replaces a base .ep", 0, |root| {
                put(
                    root,
                    "Content/SeriousSam2/Databases/Sample.ep",
                    "my parameters",
                )
            }),
        ),
        (
            "no-descriptor-text",
            ("no .nfo at all", 0, |root| {
                std::fs::remove_file(root.join("Content/HelloMod.nfo")).unwrap()
            }),
        ),
    ]
}

#[test]
fn the_scenarios_give_the_documented_exit_codes_and_reports() {
    let print = std::env::var_os("UPDATE_MODDING_DOCS").is_some();
    let (troubleshooting, tutorial) = if print {
        (String::new(), String::new())
    } else {
        (page("troubleshooting.md"), page("tutorial.md"))
    };
    for (id, (what, exit, change)) in scenarios() {
        let (code, report) = run_variant(change);
        if print {
            println!("=== {id}: {what} (exit {code})\n{report}");
        }
        assert_eq!(code, exit, "{id}: {report}");
        if print {
            continue;
        }
        assert!(
            tutorial.contains(&report) || troubleshooting.contains(&report),
            "docs/modding does not contain the report of `{id}` ({what}); run with UPDATE_MODDING_DOCS=1 to print it:\n{report}"
        );
    }
}

/// Every file of a loose folder as `(path inside the archive, bytes)`, sorted.
fn files_below(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/");
                out.push((name, std::fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// Pack the example into `Content/HelloMod/HelloMod.gro` of an invented install, with the two
/// descriptors loose next to it (the layout of the tutorial's last step), and run `--mod HelloMod`.
fn packed_run(extra: &[(&str, &[u8])]) -> (i32, String) {
    let work = tempfile::tempdir().unwrap();
    let install = work.path();
    fake_install(install);
    std::fs::create_dir_all(install.join("Content/HelloMod")).unwrap();
    let mut zip = ZipBuilder::new();
    let mut files = files_below(&examples_dir().join("HelloMod"));
    for (name, bytes) in extra {
        files.retain(|(n, _)| n != name);
        files.push(((*name).to_owned(), bytes.to_vec()));
    }
    for (name, bytes) in files {
        zip = zip.add(&name, bytes);
    }
    std::fs::write(
        install.join("Content/HelloMod/HelloMod.gro"),
        zip.finish().unwrap(),
    )
    .unwrap();
    for name in ["HelloMod.mod", "HelloMod.nfo"] {
        std::fs::copy(
            examples_dir().join("HelloMod/Content").join(name),
            install.join("Content").join(name),
        )
        .unwrap();
    }
    let out = doctor(install, &["--mod", "HelloMod"]);
    (out.status.code().unwrap(), mod_section(&out))
}

#[test]
fn the_packed_example_is_clean_and_its_report_is_in_the_tutorial() {
    let (code, report) = packed_run(&[]);
    if std::env::var_os("UPDATE_MODDING_DOCS").is_some() {
        println!("=== tutorial-packed (exit {code})\n{report}");
        return;
    }
    assert_eq!(code, 0, "{report}");
    assert!(report.contains("no findings"), "{report}");
    assert!(
        page("tutorial.md").contains(&report),
        "the tutorial lacks the packed report:\n{report}"
    );
}

#[test]
fn a_packed_mod_that_differs_from_its_loose_descriptor_is_reported() {
    // The tutorial warns about two copies of the descriptor (MOD008): this is that case.
    let (code, report) = packed_run(&[(
        "Content/HelloMod.nfo",
        b"NAME = \"Hello Mod, older copy\" ;\n",
    )]);
    assert_eq!(code, 0, "{report}");
    assert!(report.contains("MOD008"), "{report}");
}

// ---------------------------------------------------------------------------------------------
// The pages and the code must agree: these tests fail when a documented code, flag, key or default
// disappears or changes.

fn severity_word(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

/// The cells of every table row of `doc`.
fn rows(doc: &str) -> Vec<Vec<String>> {
    doc.lines()
        .filter(|l| l.starts_with('|'))
        .map(|l| {
            l.trim()
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|c| c.trim().trim_matches('|').trim().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn the_troubleshooting_table_has_every_lint_with_its_real_severity() {
    let doc = page("troubleshooting.md");
    let table = rows(&doc);
    for lint in Lint::ALL {
        let row = table
            .iter()
            .find(|r| r.first().map(String::as_str) == Some(lint.code()))
            .unwrap_or_else(|| panic!("{} has no row in troubleshooting.md", lint.code()));
        assert_eq!(
            row.get(1).map(String::as_str),
            Some(severity_word(lint.severity())),
            "{}: {row:?}",
            lint.code()
        );
        assert!(
            row.len() >= 4 && !row[2].is_empty() && !row[3].is_empty(),
            "{row:?}"
        );
    }
    // No code in the table that the tool does not have.
    let known: Vec<&str> = Lint::ALL.iter().map(|l| l.code()).collect();
    for row in &table {
        if row[0].starts_with("MOD") {
            assert!(known.contains(&row[0].as_str()), "{} is not a lint", row[0]);
        }
    }
    assert_eq!(Lint::ALL.len(), 37);
    assert!(doc.contains("all 37 of them"));
}

#[test]
fn the_api_page_groups_every_lint_under_its_real_severity() {
    let doc = page("api.md");
    let table = rows(&doc);
    let mut seen = Vec::new();
    for row in table.iter().filter(|r| {
        r.len() == 4 && !r[0].starts_with('`') && r[1..].iter().any(|c| c.contains("`MOD"))
    }) {
        for (cell, expected) in
            row[1..]
                .iter()
                .zip([Severity::Error, Severity::Warning, Severity::Info])
        {
            for code in cell
                .split(',')
                .map(|c| c.trim().trim_matches('`'))
                .filter(|c| c.starts_with("MOD"))
            {
                let lint = Lint::ALL
                    .iter()
                    .find(|l| l.code() == code)
                    .unwrap_or_else(|| panic!("{code} is not a lint"));
                assert_eq!(lint.severity(), expected, "{code} is in the wrong column");
                seen.push(code.to_owned());
            }
        }
    }
    for lint in Lint::ALL {
        assert!(
            seen.iter().any(|c| c == lint.code()),
            "{} is not in the groups table",
            lint.code()
        );
    }
}

#[test]
fn the_documented_command_line_exists() {
    let bin = env!("CARGO_BIN_EXE_ss2-doctor");
    let help = String::from_utf8(Command::new(bin).arg("--help").output().unwrap().stdout).unwrap();
    let api = page("api.md");
    for flag in ["--mod", "--mod-dir", "--all-mods", "--help", "--version"] {
        assert!(help.contains(flag), "{flag} is not in the usage text");
        assert!(
            api.contains(&format!("| `{flag}"))
                || (flag == "--version" && api.contains("/ `--version`")),
            "{flag} has no row in the flag table of api.md"
        );
    }
    for (args, code) in [
        (vec!["--frobnicate"], 2),
        (vec!["--mod"], 2),
        (vec!["--version"], 0),
    ] {
        let out = Command::new(bin)
            .args(&args)
            .env_remove("SS2_DATA_DIR")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(code), "{args:?}");
    }
    assert!(
        api.contains("`0` no error")
            && api.contains("`1` at least one error")
            && api.contains("`2` wrong command line")
    );
    assert!(api.contains("SS2_DATA_DIR"));
    // The sentence for `--all-mods` on an install without mods.
    let sentence = "No mod folders (Content/<Mod>/ with .gro archives) in the install.";
    let work = tempfile::tempdir().unwrap();
    fake_install(work.path());
    let out = doctor(work.path(), &["--all-mods"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains(sentence));
    assert!(api.contains(sentence));
}

/// The file keys of `api.md`: each is checked by the tool with the documented severity; the keys
/// the page says are not checked are not.
#[test]
fn the_documented_reference_keys_are_the_ones_the_doctor_checks() {
    let api = page("api.md");
    let add_key = |key: &str| {
        let key = key.to_owned();
        move |root: &Path| {
            let path = root.join("Content/HelloMod/Levels/HelloEpisode.nfo");
            let mut text = std::fs::read_to_string(&path).unwrap();
            text.push_str(&format!(
                "{key} = \"Content/HelloMod/Nowhere/missing.bin\"\n"
            ));
            std::fs::write(path, text).unwrap();
        }
    };
    let keys = [
        ("MODEL", "MOD030"),
        ("LOADING_MODEL", "MOD031"),
        ("THUMBNAIL", "MOD031"),
        ("LOADING_SCREEN", "MOD031"),
        ("INTRO_MOVIE", "MOD031"),
        ("OUTRO_MOVIE", "MOD031"),
        ("EPISODE_MOVIE", "MOD031"),
    ];
    for (key, code) in keys {
        assert!(api.contains(&format!("`{key}`")), "{key} is not in api.md");
        let (exit, report) = run_variant(add_key(key));
        assert!(report.contains(code), "{key}: {report}");
        assert_eq!(exit, i32::from(code == "MOD030"), "{key}: {report}");
    }
    for key in [
        "MODEL_NAME",
        "MAP_TYPE",
        "ETA",
        "PLAYERS",
        "LOCKED",
        "DESCRIPTION",
        "LICENSE",
        "UNKNOWN_KEY",
    ] {
        let (exit, report) = run_variant(add_key(key));
        assert_eq!(exit, 0, "{key}: {report}");
        assert!(report.contains("no findings"), "{key}: {report}");
    }
}

#[test]
fn requires_patch_accepts_the_documented_forms() {
    for (value, ok) in [
        ("2.100", true),
        ("2.100.800000", true),
        ("2", true),
        ("2.101", false),
        ("2.100.800001", false),
    ] {
        let (exit, report) = run_variant(|root| {
            let path = root.join("Content/HelloMod.nfo");
            let mut text = std::fs::read_to_string(&path).unwrap();
            text.push_str(&format!("REQUIRES_PATCH = \"{value}\" ;\n"));
            std::fs::write(path, text).unwrap();
        });
        assert_eq!(exit == 0, ok, "{value}: {report}");
    }
}

#[test]
fn the_descriptor_syntax_of_start_here_is_what_the_parsers_do() {
    // `.mod`: the keyword in any case, <TOOLS> names are editor-only, a BOM is fine.
    let m = ModManifest::parse("\u{feff}modules = Sam2Game <TOOLS>Sam2GameGui ;").unwrap();
    assert_eq!(m.game_modules().collect::<Vec<_>>(), ["Sam2Game"]);
    for bad in [
        "MODULES = Sam2Game",
        "MODULES Sam2Game ;",
        "MODULES = ;",
        "MODULES = <TOOLS> ;",
    ] {
        assert!(ModManifest::parse(bad).is_err(), "{bad}");
    }
    // `.nfo`: the first duplicate wins, keys in any case, an unquoted value runs to the line end or
    // to `;`, a value may be on the next line, there are no escapes, Latin-1 and BOM are accepted.
    let n = Nfo::parse(
        b"name = \"One\" ;\nNAME = \"Two\"\nTIME = 10:00 ; X = 1\nAUTHOR =\n  \"Next line\"\n",
    )
    .unwrap();
    assert_eq!(n.get("NAME"), Some("One"));
    assert_eq!(n.get("TIME"), Some("10:00"));
    assert_eq!(n.get("AUTHOR"), Some("Next line"));
    assert_eq!(
        Nfo::parse(b"NAME = \"a\\\" ;").unwrap().get("NAME"),
        Some("a\\")
    );
    assert_eq!(
        Nfo::parse(b"NAME = \"Caf\xE9\"").unwrap().get("NAME"),
        Some("Caf\u{e9}")
    );
    assert_eq!(
        Nfo::parse(b"\xEF\xBB\xBFNAME = \"B\"").unwrap().get("NAME"),
        Some("B")
    );
    // A quote that is never closed swallows the following lines.
    let err = Nfo::parse(b"NAME = \"Hello Mod ;\nAUTHOR = \"Me\" ;\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("expected `=` after `Me\"`"), "{err}");
}

#[test]
fn load_order_follows_the_documented_rule() {
    let names = [
        "z_HelloModUpdate_01.gro",
        "Patch_02_100_800000.gro",
        "HelloMod.gro",
        "Patch_02_068.gro",
        "All_PC_02.gro",
        "All_PC_01.gro",
    ]
    .map(String::from);
    assert_eq!(
        archive_load_order(names),
        [
            "All_PC_01.gro",
            "All_PC_02.gro",
            "HelloMod.gro",
            "z_HelloModUpdate_01.gro",
            "Patch_02_068.gro",
            "Patch_02_100_800000.gro"
        ]
    );
}

#[test]
fn the_documented_defaults_of_the_settings_are_the_real_ones() {
    use ss2_core::settings::Settings;
    let mut s = Settings::default();
    assert!(s.vsync && !s.fullscreen);
    assert_eq!(
        (s.master_volume, s.music_volume, s.sfx_volume),
        (0.8, 0.6, 0.8)
    );
    let doc = page("api.md");
    assert!(doc.contains(
        "`vsync` (default `true`), `fullscreen` (`false`), `master_volume` (`0.8`), `music_volume` (`0.6`), `sfx_volume` (`0.8`)"
    ));
    for key in [
        "vsync=off",
        "fullscreen=on",
        "master_volume=2",
        "music_volume=0.1",
        "sfx_volume=0",
    ] {
        s.apply_override(key).unwrap();
    }
    assert_eq!(s.master_volume, 1.0, "volumes are clamped");
    assert!(s.apply_override("gamma=1").is_err());
}

#[test]
fn links_to_the_spec_pages_point_at_files_that_exist() {
    let mut checked = 0;
    for name in [
        "start-here.md",
        "formats.md",
        "api.md",
        "tutorial.md",
        "troubleshooting.md",
    ] {
        let doc = page(name);
        for part in doc.split("blob/main/").skip(1) {
            let target: String = part
                .chars()
                .take_while(|c| !matches!(c, ')' | ' ' | '#' | '`'))
                .collect();
            assert!(
                repo_root().join(&target).exists(),
                "{name} links to {target}, which does not exist"
            );
            checked += 1;
        }
    }
    assert!(checked >= 20, "{checked}");
}

#[test]
fn the_example_mod_names_no_game_file() {
    let nfo =
        std::fs::read_to_string(examples_dir().join("HelloMod/Content/HelloMod.nfo")).unwrap();
    assert!(!nfo.to_lowercase().contains("serioussam2"));
    let readme = std::fs::read_to_string(examples_dir().join("README.md")).unwrap();
    assert!(readme.contains("CC0-1.0"));
}
