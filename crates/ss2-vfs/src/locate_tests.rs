use std::fs;
use std::path::{Path, PathBuf};

use ss2_testkit::fixtures::ZipBuilder;

use super::*;

// ---------------------------------------------------------------------------------------------------------------------------------
// Helpers

const ALL: [&str; 14] = [
    "All_PC_01.gro",
    "All_PC_02.gro",
    "All_PC_03.gro",
    "Movies_PC_01.gro",
    "Movies_PC_02.gro",
    "Patch_02_068.gro",
    "Patch_02_080_210639.gro",
    "Patch_02_080_243026.gro",
    "Patch_02_080_243893.gro",
    "Patch_02_080_244138.gro",
    "Patch_02_080_244697.gro",
    "Patch_02_080_252822.gro",
    "Patch_02_090_593613.gro",
    "Patch_02_100_800000.gro",
];

fn write_gro(dir: &Path, name: &str) {
    let bytes = ZipBuilder::new()
        .add("readme.txt", b"x".to_vec())
        .finish()
        .unwrap();
    fs::write(dir.join(name), bytes).unwrap();
}

/// A real folder with the given archives (each a valid, tiny ZIP).
fn install(dir: &Path, names: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    for name in names {
        write_gro(dir, name);
    }
}

fn windows() -> FakeEnvironment {
    FakeEnvironment::new(Os::Windows)
}

fn linux() -> FakeEnvironment {
    let mut env = FakeEnvironment::new(Os::Linux);
    env.home_dir = Some(PathBuf::from("/home/ann"));
    env
}

fn dirs_of(d: &Discovery) -> Vec<String> {
    d.candidates
        .iter()
        .map(|c| c.dir.to_string_lossy().replace('\\', "/"))
        .collect()
}

// ---------------------------------------------------------------------------------------------------------------------------------
// dedupe_key

#[test]
fn spellings_of_one_folder_have_one_key() {
    let key = dedupe_key(Path::new("C:/Games/Serious Sam 2"));
    for same in [
        r"C:\Games\Serious Sam 2",
        r"c:\games\SERIOUS SAM 2\",
        "C:/Games//Serious Sam 2/",
        r"C:\Games\.\Serious Sam 2",
        r"\\?\C:\Games\Serious Sam 2",
    ] {
        assert_eq!(dedupe_key(Path::new(same)), key, "{same}");
    }
    assert_ne!(dedupe_key(Path::new("C:/Games/Serious Sam 3")), key);
    assert_ne!(dedupe_key(Path::new("C:/Games")), key);
}

#[test]
fn keys_tell_absolute_relative_and_network_paths_apart() {
    let abs = dedupe_key(Path::new("/games/x"));
    let rel = dedupe_key(Path::new("games/x"));
    let unc = dedupe_key(Path::new(r"\\games\x"));
    assert_eq!(abs, "/games/x");
    assert_eq!(rel, "games/x");
    assert_eq!(unc, "//games/x");
    assert_eq!(dedupe_key(Path::new("/")), "/");
    assert_eq!(dedupe_key(Path::new("")), "");
}

#[test]
fn non_ascii_names_compare_without_regard_to_case() {
    assert_eq!(
        dedupe_key(Path::new("D:/Игры/Serious Sam 2")),
        dedupe_key(Path::new("d:/игры/serious sam 2"))
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Steam's list of libraries

#[test]
fn the_current_vdf_layout_gives_every_path_and_ignores_the_apps() {
    let text = r#"
"libraryfolders"
{
    "0"
    {
        "path"      "C:\\Program Files (x86)\\Steam"
        "label"     ""
        "contentid" "123456"
        "apps"
        {
            "227300"    "6953850"
            "9"         "1"
        }
    }
    "1"
    {
        "path"      "D:\\Steam Library"
        "apps"      { "10" "20" }
    }
}
"#;
    assert_eq!(
        library_texts(text),
        [r"C:\Program Files (x86)\Steam", r"D:\Steam Library"]
    );
}

/// The folders of a VDF as text: `PathBuf` equality on Windows treats `/` and `\` alike, which would hide a wrong escape.
fn library_texts(text: &str) -> Vec<String> {
    parse_library_folders(text)
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_key_path_may_be_written_in_any_case_and_old_style_pairs_follow_sections() {
    assert_eq!(
        library_texts("\"x\" { \"Path\" \"/a\" \"PATH\" \"/b\" }"),
        ["/a", "/b"]
    );
    // After a section has closed, a numbered pair is at the top level again.
    assert_eq!(
        library_texts("\"LibraryFolders\" { \"0\" { \"x\" \"y\" } \"1\" \"/after\" }"),
        ["/after"]
    );
    // A numbered pair inside a section is not a library.
    assert!(library_texts("\"LibraryFolders\" { \"0\" { \"1\" \"/inside\" } }").is_empty());
}

#[test]
fn the_old_vdf_layout_is_read_too() {
    let text = "\"LibraryFolders\"\n{\n\t\"TimeNextStatsReport\"\t\"1400000000\"\n\t\"ContentStatsID\"\t\"-1\"\n\t\"1\"\t\"E:\\\\Games\\\\Steam\"\n\t\"2\"\t\"/mnt/games/steam\"\n}\n";
    assert_eq!(library_texts(text), [r"E:\Games\Steam", "/mnt/games/steam"]);
}

#[test]
fn escapes_comments_and_unicode_in_the_vdf_are_handled() {
    let text = "// a comment \"path\" \"nope\"\n\"x\" { \"path\" \"C:\\\\Игры\\\\a \\\"b\\\"\" }";
    assert_eq!(library_texts(text), ["C:\\Игры\\a \"b\""]);
}

#[test]
fn damaged_vdf_text_gives_what_can_be_read_and_never_panics() {
    assert!(parse_library_folders("").is_empty());
    assert!(parse_library_folders("\u{0}\u{1}garbage }}}{{{").is_empty());
    assert!(parse_library_folders("\"path\"").is_empty());
    assert!(parse_library_folders("\"path\" \"unterminated").is_empty());
    assert!(parse_library_folders("\"x\" { \"path\" \"  \" }").is_empty());
    assert!(parse_library_folders("\"path\" \"C:\\\\a\\").len() <= 1);
    assert_eq!(
        parse_library_folders("\"x\" { \"path\" \"/ok\" \"path\" \"/cut"),
        [PathBuf::from("/ok")]
    );
    // An unknown escape keeps its backslash.
    assert_eq!(
        parse_library_folders("\"x\" { \"path\" \"C:\\a\" }"),
        [PathBuf::from("C:\\a")]
    );
}

#[test]
fn the_reg_query_output_gives_the_value() {
    let out = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamPath    REG_SZ    c:/program files (x86)/steam\r\n\r\n";
    assert_eq!(
        parse_reg_query(out, "SteamPath").as_deref(),
        Some("c:/program files (x86)/steam")
    );
    assert_eq!(
        parse_reg_query(out, "steampath").as_deref(),
        Some("c:/program files (x86)/steam"),
        "value names do not depend on case"
    );
    assert_eq!(parse_reg_query(out, "Other"), None);
    assert_eq!(
        parse_reg_query("    SteamPath    REG_DWORD    0x1\r\n", "SteamPath"),
        None
    );
    assert_eq!(
        parse_reg_query("    SteamPath    REG_SZ    \r\n", "SteamPath"),
        None
    );
    assert_eq!(parse_reg_query("", "SteamPath"), None);
    assert_eq!(
        parse_reg_query(
            "    InstallPath    REG_EXPAND_SZ    D:\\Steam\r\n",
            "InstallPath"
        )
        .as_deref(),
        Some("D:\\Steam")
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Discovery

#[test]
fn a_machine_with_nothing_gives_no_candidates_and_nothing_rejected() {
    let d = discover(&windows(), None);
    assert!(d.candidates.is_empty());
    assert!(d.rejected.is_empty());
    let d = discover(&linux(), None);
    assert!(d.candidates.is_empty() && d.rejected.is_empty());
    let d = discover(&FakeEnvironment::new(Os::MacOs), None);
    assert!(d.candidates.is_empty() && d.rejected.is_empty());
}

#[test]
fn the_candidates_come_in_the_order_of_preference() {
    let mut env = windows()
        .with_var("SS2_DATA_DIR", r"E:\env")
        .with_dir(r"E:\env", &["All_PC_01.gro"])
        .with_dir(r"E:\remembered", &["All_PC_01.gro"])
        .with_dir(r"E:\cwd", &["All_PC_01.gro"])
        .with_dir(r"E:\exe", &["all_pc_01.GRO"])
        .with_dir(r"C:\GOG Games\Serious Sam 2", &["All_PC_01.gro"])
        .with_dir(r"D:\Serious Sam 2", &["All_PC_01.gro"])
        .with_dir(r"E:\lib\steamapps\common", &["Serious Sam 2"])
        .with_dir(r"E:\lib\steamapps\common\Serious Sam 2", &["All_PC_01.gro"])
        .with_file(
            r"C:\Program Files (x86)\Steam\steamapps\libraryfolders.vdf",
            r#""libraryfolders" { "0" { "path" "E:\\lib" } }"#,
        );
    env.current_dir = Some(PathBuf::from(r"E:\cwd"));
    env.exe_dir = Some(PathBuf::from(r"E:\exe"));
    let d = discover(&env, Some(Path::new(r"E:\remembered")));
    assert_eq!(
        dirs_of(&d),
        [
            "E:/remembered",
            "E:/env",
            "E:/cwd",
            "E:/exe",
            "E:/lib/steamapps/common/Serious Sam 2",
            "C:/GOG Games/Serious Sam 2",
            "D:/Serious Sam 2",
        ]
    );
    let sources: Vec<_> = d.candidates.iter().map(|c| c.sources.clone()).collect();
    assert_eq!(
        sources,
        [
            vec![CandidateSource::Remembered],
            vec![CandidateSource::Environment],
            vec![CandidateSource::CurrentDir],
            vec![CandidateSource::ExeDir],
            vec![CandidateSource::Steam],
            vec![CandidateSource::Gog],
            vec![CandidateSource::Retail],
        ]
    );
}

#[test]
fn a_folder_named_twice_appears_once_with_both_sources_even_if_only_the_case_differs() {
    let mut env = windows()
        .with_var("SS2_DATA_DIR", r"e:\spiele\serious sam 2\")
        .with_dir(r"E:\Spiele\Serious Sam 2", &["All_PC_01.gro"]);
    env.current_dir = Some(PathBuf::from("E:/Spiele/Serious Sam 2"));
    let d = discover(&env, Some(Path::new(r"E:\SPIELE\Serious Sam 2")));
    assert_eq!(d.candidates.len(), 1, "{d:?}");
    let c = &d.candidates[0];
    assert_eq!(
        c.dir,
        PathBuf::from(r"E:\SPIELE\Serious Sam 2"),
        "the first spelling stays"
    );
    assert_eq!(
        c.sources,
        [
            CandidateSource::Remembered,
            CandidateSource::Environment,
            CandidateSource::CurrentDir
        ]
    );
}

#[test]
fn the_same_source_twice_is_listed_once() {
    let env = windows()
        .with_dir(r"C:\Program Files (x86)\Serious Sam 2", &["All_PC_01.gro"])
        .with_var("ProgramFiles(x86)", r"C:\Program Files (x86)");
    let d = discover(&env, None);
    assert_eq!(d.candidates.len(), 1);
    assert_eq!(d.candidates[0].sources, [CandidateSource::Retail]);
}

#[test]
fn only_a_folder_that_lists_the_first_archive_is_a_candidate() {
    let mut env = windows()
        .with_dir(r"E:\cwd", &["All_PC_02.gro", "readme.txt"])
        .with_dir(r"E:\exe", &["All_PC_01.gro.bak", "All_PC_01.gro.txt"])
        .with_dir(r"C:\GOG Games\Serious Sam 2", &["Serious Sam 2"]);
    env.current_dir = Some(PathBuf::from(r"E:\cwd"));
    env.exe_dir = Some(PathBuf::from(r"E:\exe"));
    let d = discover(&env, None);
    assert!(d.candidates.is_empty(), "{d:?}");
    assert!(
        d.rejected.is_empty(),
        "guesses that fail are not worth a line"
    );
}

#[test]
fn a_remembered_or_variable_folder_that_fails_is_rejected_with_a_reason() {
    let env = windows()
        .with_var("SS2_DATA_DIR", r"E:\wrong")
        .with_dir(r"E:\wrong", &["notes.txt"]);
    let d = discover(&env, Some(Path::new(r"E:\gone")));
    assert!(d.candidates.is_empty());
    assert_eq!(d.rejected.len(), 2);
    assert_eq!(d.rejected[0].source, CandidateSource::Remembered);
    assert_eq!(d.rejected[0].dir, PathBuf::from(r"E:\gone"));
    assert!(
        d.rejected[0].reason.contains("does not exist"),
        "{:?}",
        d.rejected[0]
    );
    assert_eq!(d.rejected[1].source, CandidateSource::Environment);
    assert!(
        d.rejected[1].reason.contains("All_PC_01.gro"),
        "{:?}",
        d.rejected[1]
    );
}

#[test]
fn a_bad_folder_named_twice_is_rejected_once() {
    let env = windows().with_var("SS2_DATA_DIR", r"E:\gone");
    let d = discover(&env, Some(Path::new("e:/GONE/")));
    assert_eq!(d.rejected.len(), 1, "{d:?}");
    assert_eq!(d.rejected[0].source, CandidateSource::Remembered);
}

#[test]
fn an_empty_variable_is_not_a_folder() {
    let env = windows().with_var("SS2_DATA_DIR", "  ");
    let d = discover(&env, None);
    assert!(d.candidates.is_empty() && d.rejected.is_empty());
}

#[test]
fn a_candidate_knows_its_archives_and_its_newest_patch() {
    let env = windows().with_var("SS2_DATA_DIR", r"E:\g").with_dir(
        r"E:\g",
        &[
            "All_PC_01.gro",
            "Patch_02_100_800000.gro",
            "Patch_02_068.gro",
            "Patch_02_090_593613.gro",
            "Patch_old.gro",
            "Content",
            "readme.txt",
            "Bin.gro.d",
        ],
    );
    let d = discover(&env, None);
    let c = &d.candidates[0];
    assert_eq!(
        c.archives, 5,
        "the .gro names, patches and the odd one included"
    );
    let level = c.patch.as_ref().unwrap();
    assert_eq!((level.minor, level.build), (100, Some(800_000)));
}

#[test]
fn steam_libraries_come_from_the_vdf_the_registry_and_the_default_place() {
    let env = windows()
        .with_registry(r"HKCU\Software\Valve\Steam", "SteamPath", "d:/steam")
        .with_file(
            r"d:\steam\steamapps\libraryfolders.vdf",
            r#""libraryfolders" { "0" { "path" "d:\\steam" } "1" { "path" "F:\\Big Library" } }"#,
        )
        .with_dir(
            r"d:\steam\steamapps\common",
            &["Half-Life", "serious sam 2"],
        )
        .with_dir(
            r"d:\steam\steamapps\common\serious sam 2",
            &["All_PC_01.gro"],
        )
        .with_dir(r"F:\Big Library\steamapps\common", &["Serious Sam 2"])
        .with_dir(
            r"F:\Big Library\steamapps\common\Serious Sam 2",
            &["All_PC_01.gro"],
        )
        .with_dir(
            r"C:\Program Files (x86)\Steam\steamapps\common",
            &["SeriousSam2"],
        )
        .with_dir(
            r"C:\Program Files (x86)\Steam\steamapps\common\SeriousSam2",
            &["All_PC_01.gro"],
        );
    let d = discover(&env, None);
    let found = dirs_of(&d);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found.contains(&"d:/steam/steamapps/common/serious sam 2".to_owned()),
        "{found:?}"
    );
    assert!(found.contains(&"F:/Big Library/steamapps/common/Serious Sam 2".to_owned()));
    assert!(
        found.contains(&"C:/Program Files (x86)/Steam/steamapps/common/SeriousSam2".to_owned())
    );
    assert!(
        d.candidates
            .iter()
            .all(|c| c.sources == [CandidateSource::Steam])
    );
}

#[test]
fn a_damaged_or_missing_vdf_or_a_library_without_the_game_is_skipped() {
    let env = windows()
        .with_file(
            r"C:\Program Files (x86)\Steam\steamapps\libraryfolders.vdf",
            "}{ not vdf \"path\" \"Z:\\\\nowhere\"",
        )
        .with_dir(
            r"C:\Program Files (x86)\Steam\steamapps\common",
            &["Portal 2"],
        );
    let d = discover(&env, None);
    assert!(d.candidates.is_empty() && d.rejected.is_empty());
}

#[test]
fn linux_steam_gog_and_wine_places_are_searched_under_the_home_folder() {
    let env = linux()
        .with_dir(
            "/home/ann/.local/share/Steam/steamapps/common",
            &["Serious Sam 2"],
        )
        .with_dir(
            "/home/ann/.local/share/Steam/steamapps/common/Serious Sam 2",
            &["All_PC_01.gro"],
        )
        .with_dir("/home/ann/GOG Games/Serious Sam 2", &["All_PC_01.gro"])
        .with_dir(
            "/home/ann/.wine/drive_c/Program Files (x86)/Serious Sam 2",
            &["All_PC_01.gro"],
        );
    let d = discover(&env, None);
    let sources: Vec<_> = d.candidates.iter().map(|c| c.sources[0]).collect();
    assert_eq!(
        sources,
        [
            CandidateSource::Steam,
            CandidateSource::Gog,
            CandidateSource::Retail
        ]
    );
}

/// A machine of this family where the game sits in `steamapps/common` of the Steam folder `root`.
fn steam_at(mut env: FakeEnvironment, root: &str) -> Discovery {
    let common = format!("{root}/steamapps/common");
    env = env
        .with_dir(&common, &["Serious Sam 2"])
        .with_dir(format!("{common}/Serious Sam 2"), &["All_PC_01.gro"]);
    discover(&env, None)
}

#[test]
fn every_linux_steam_folder_is_searched() {
    for root in [
        "/home/ann/.steam/steam",
        "/home/ann/.local/share/Steam",
        "/home/ann/.var/app/com.valvesoftware.Steam/.local/share/Steam",
        "/home/ann/.wine/drive_c/Program Files (x86)/Steam",
    ] {
        let d = steam_at(linux(), root);
        assert_eq!(d.candidates.len(), 1, "{root}: {d:?}");
        assert_eq!(d.candidates[0].sources, [CandidateSource::Steam], "{root}");
    }
}

#[test]
fn every_windows_steam_folder_is_searched() {
    // The registry values, the two program files folders (from the variables) and the default place.
    let hkcu = windows().with_registry(r"HKCU\Software\Valve\Steam", "SteamPath", "E:/s1");
    assert_eq!(steam_at(hkcu, "E:/s1").candidates.len(), 1);
    let wow = windows().with_registry(
        r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam",
        "InstallPath",
        "E:/s2",
    );
    assert_eq!(steam_at(wow, "E:/s2").candidates.len(), 1);
    let hklm = windows().with_registry(r"HKLM\SOFTWARE\Valve\Steam", "InstallPath", "E:/s3");
    assert_eq!(steam_at(hklm, "E:/s3").candidates.len(), 1);
    let x86 = windows().with_var("ProgramFiles(x86)", r"F:\PF86");
    assert_eq!(steam_at(x86, "F:/PF86/Steam").candidates.len(), 1);
    let pf = windows().with_var("ProgramFiles", r"F:\PF");
    assert_eq!(steam_at(pf, "F:/PF/Steam").candidates.len(), 1);
    let default = steam_at(windows(), "C:/Program Files (x86)/Steam");
    assert_eq!(default.candidates.len(), 1);
    // A blank registry value is no folder.
    let blank = windows().with_registry(r"HKCU\Software\Valve\Steam", "SteamPath", "  ");
    assert!(discover(&blank, None).candidates.is_empty());
}

#[test]
fn the_store_places_follow_the_program_files_variables() {
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        let env = windows()
            .with_var(var, r"G:\Apps")
            .with_dir(
                r"G:\Apps\GOG Galaxy\Games\Serious Sam 2",
                &["All_PC_01.gro"],
            )
            .with_dir(r"G:\Apps\Croteam\Serious Sam 2", &["All_PC_01.gro"]);
        let d = discover(&env, None);
        let sources: Vec<_> = d.candidates.iter().map(|c| c.sources[0]).collect();
        assert_eq!(
            sources,
            [CandidateSource::Gog, CandidateSource::Retail],
            "{var}"
        );
    }
}

#[test]
fn the_macos_steam_folder_is_searched() {
    let mut env = FakeEnvironment::new(Os::MacOs)
        .with_dir(
            "/Users/ann/Library/Application Support/Steam/steamapps/common",
            &["Serious Sam 2"],
        )
        .with_dir(
            "/Users/ann/Library/Application Support/Steam/steamapps/common/Serious Sam 2",
            &["All_PC_01.gro"],
        );
    env.home_dir = Some(PathBuf::from("/Users/ann"));
    assert_eq!(discover(&env, None).candidates.len(), 1);
}

#[test]
fn many_candidates_all_survive_in_order() {
    let mut env = windows();
    let mut libs = String::from("\"libraryfolders\" {");
    for i in 0..40 {
        let lib = format!("G:\\Lib{i:02}");
        libs.push_str(&format!("\"{i}\" {{ \"path\" \"G:\\\\Lib{i:02}\" }}"));
        env = env
            .with_dir(format!(r"{lib}\steamapps\common"), &["Serious Sam 2"])
            .with_dir(
                format!(r"{lib}\steamapps\common\Serious Sam 2"),
                &["All_PC_01.gro"],
            );
    }
    libs.push('}');
    env = env
        .with_registry(r"HKCU\Software\Valve\Steam", "SteamPath", "G:/Lib00")
        .with_file(r"G:\Lib00\steamapps\libraryfolders.vdf", &libs);
    let d = discover(&env, None);
    assert_eq!(d.candidates.len(), 40);
    assert!(dirs_of(&d)[0].starts_with("G:/Lib00"));
    assert!(dirs_of(&d)[39].starts_with("G:/Lib39"));
}

#[test]
fn spaces_non_ascii_and_very_long_paths_are_fine() {
    let long = format!(
        r"E:\{}\Serious Sam 2",
        "very long folder name ".repeat(40).trim()
    );
    assert!(long.len() > 800);
    let env = windows()
        .with_var("SS2_DATA_DIR", &long)
        .with_dir(&long, &["All_PC_01.gro"])
        .with_dir(r"D:\Игры и Ñandú\Serious Sam 2", &["All_PC_01.gro"])
        .with_dir(r"C:\Program Files (x86)\Serious Sam 2", &["All_PC_01.gro"]);
    let d = discover(&env, Some(Path::new(r"D:\Игры и Ñandú\Serious Sam 2")));
    assert_eq!(d.candidates.len(), 3);
    assert_eq!(d.candidates[1].dir, PathBuf::from(long));
}

#[test]
fn the_real_machine_environment_answers_without_panicking() {
    let tmp = tempfile::tempdir().unwrap();
    install(tmp.path(), &["All_PC_01.gro"]);
    let sys = SystemEnvironment;
    assert_eq!(sys.os(), Os::current());
    let names = sys.list_dir(tmp.path()).unwrap();
    assert_eq!(names, ["All_PC_01.gro"]);
    assert_eq!(sys.list_dir(&tmp.path().join("nope")), None);
    fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    assert_eq!(
        sys.read_text(&tmp.path().join("a.txt")).as_deref(),
        Some("hello")
    );
    assert_eq!(sys.read_text(&tmp.path().join("nope.txt")), None);
    let big = tmp.path().join("big.txt");
    fs::write(&big, vec![b'a'; (1 << 20) + 1]).unwrap();
    assert_eq!(sys.read_text(&big), None, "a file over 1 MiB is not read");
    fs::write(&big, vec![b'a'; 1 << 20]).unwrap();
    assert!(sys.read_text(&big).is_some(), "exactly 1 MiB is read");
    assert_eq!(sys.var("SS2_SURELY_NOT_SET_ANYWHERE"), None);
    let _ = (sys.current_dir(), sys.exe_dir(), sys.home_dir());
    let _ = sys.registry_string(r"HKCU\Software\SS2RS-surely-not-there", "x");
    let _ = discover(&sys, None);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Validation for people

fn report_of(names: &[&str]) -> InstallReport {
    let tmp = tempfile::tempdir().unwrap();
    install(tmp.path(), names);
    check_install(tmp.path())
}

#[test]
fn a_complete_install_is_usable_with_no_warning() {
    let s = summarize(&report_of(&ALL));
    assert!(s.usable && !s.outdated, "{s:?}");
    assert_eq!(
        s.headline,
        "Serious Sam 2 found: the install looks complete"
    );
    assert_eq!(s.lines[0], "Archives: 14 found, 14 readable");
    assert_eq!(s.lines[1], "Patch level: 2.100 (build 800000)");
    assert_eq!(s.lines.len(), 2, "{:?}", s.lines);
}

#[test]
fn an_old_patch_level_is_usable_but_says_so() {
    let mut names: Vec<&str> = ALL[..5].to_vec();
    names.push("Patch_02_068.gro");
    let s = summarize(&report_of(&names));
    assert!(s.usable && s.outdated, "{s:?}");
    assert_eq!(s.headline, "Usable, with 1 warning");
    assert!(
        s.lines.contains(&"Patch level: 2.068".to_owned()),
        "{:?}",
        s.lines
    );
    assert!(
        s.lines
            .iter()
            .any(|l| l.starts_with("WARNING: the patch level 2.068 is older than 2.100")),
        "{:?}",
        s.lines
    );
    assert!(
        s.lines
            .iter()
            .any(|l| l.starts_with("  -> Update the game"))
    );
}

#[test]
fn the_newest_known_patch_is_not_outdated_and_a_later_build_is_not_either() {
    let mut names: Vec<&str> = ALL[..5].to_vec();
    names.push("Patch_02_100_800001.gro");
    assert!(!summarize(&report_of(&names)).outdated);
    let mut names: Vec<&str> = ALL[..5].to_vec();
    names.push("Patch_02_100_799999.gro");
    assert!(
        summarize(&report_of(&names)).outdated,
        "one build older counts"
    );
    let mut names: Vec<&str> = ALL[..5].to_vec();
    names.push("Patch_02_101.gro");
    assert!(!summarize(&report_of(&names)).outdated);
}

#[test]
fn no_patch_at_all_is_a_warning_from_the_check_and_not_double_counted() {
    let s = summarize(&report_of(&ALL[..5]));
    assert!(s.usable && !s.outdated);
    assert_eq!(s.headline, "Usable, with 1 warning");
    assert!(s.lines.contains(&"Patch level: none found".to_owned()));
}

#[test]
fn a_missing_base_archive_is_unusable_and_the_errors_come_with_advice() {
    let s = summarize(&report_of(&ALL[1..]));
    assert!(!s.usable);
    assert_eq!(s.headline, "This folder cannot be used: 1 error");
    assert!(
        s.lines
            .iter()
            .any(|l| l == "ERROR: All_PC_01.gro is missing"),
        "{:?}",
        s.lines
    );
    assert!(
        s.lines
            .iter()
            .any(|l| l.starts_with("  -> Reinstall the game"))
    );
}

#[test]
fn several_errors_are_counted_and_a_damaged_archive_is_not_readable() {
    let tmp = tempfile::tempdir().unwrap();
    install(tmp.path(), &["All_PC_02.gro"]);
    fs::write(tmp.path().join("Patch_02_068.gro"), b"not a zip").unwrap();
    let s = summarize(&check_install(tmp.path()));
    assert!(!s.usable);
    assert!(
        s.headline.starts_with("This folder cannot be used: "),
        "{}",
        s.headline
    );
    assert!(s.headline.ends_with("errors"), "{}", s.headline);
    assert_eq!(s.lines[0], "Archives: 2 found, 1 readable");
}

#[test]
fn a_folder_without_the_game_and_a_missing_folder_are_unusable_with_one_line_each() {
    let tmp = tempfile::tempdir().unwrap();
    let s = summarize(&check_install(tmp.path()));
    assert!(!s.usable);
    assert_eq!(s.dir, tmp.path());
    assert!(
        s.lines.iter().any(|l| l.contains("no .gro archives")),
        "{:?}",
        s.lines
    );
    let s = summarize(&check_install(tmp.path().join("gone")));
    assert!(!s.usable);
    assert!(
        s.lines.iter().any(|l| l.contains("does not exist")),
        "{:?}",
        s.lines
    );
}

#[test]
fn foreign_archives_are_warnings_in_the_lines() {
    let mut names: Vec<&str> = ALL.to_vec();
    names.push("zz_mod.gro");
    let s = summarize(&report_of(&names));
    assert!(s.usable);
    assert_eq!(s.headline, "Usable, with 1 warning");
    assert!(
        s.lines.iter().any(|l| l.starts_with("WARNING: zz_mod.gro")),
        "{:?}",
        s.lines
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Memory

fn choice(dir: &Path, at: u64) -> RememberedChoice {
    RememberedChoice {
        dir: dir.to_path_buf(),
        validated_at: at,
    }
}

#[test]
fn a_choice_survives_a_round_trip_whatever_the_path_looks_like() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    let base = tmp.path().to_path_buf();
    let long = base.join("x".repeat(200)).join("y".repeat(200));
    for dir in [
        base.join("Serious Sam 2"),
        base.join("Игры и Ñandú"),
        base.join("it's \"quoted\" and #hash = [brackets]"),
        base.join("trailing space "),
        long,
    ] {
        save_choice(&file, &choice(&dir, 1_700_000_000)).unwrap();
        assert_eq!(
            load_choice(&file),
            LoadedChoice::Found(choice(&dir, 1_700_000_000)),
            "{dir:?}"
        );
    }
}

#[test]
fn only_the_version_the_path_and_the_time_are_stored() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    save_choice(&file, &choice(&tmp.path().join("g"), 42)).unwrap();
    let table: toml::Table = fs::read_to_string(&file).unwrap().parse().unwrap();
    let mut keys: Vec<_> = table.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["path", "validated_at", "version"]);
    assert_eq!(table["version"].as_integer(), Some(1));
    assert_eq!(table["validated_at"].as_integer(), Some(42));
    assert_eq!(file.file_name().unwrap(), CHOICE_FILE_NAME);
}

#[test]
fn saving_creates_the_folders_replaces_the_old_file_and_leaves_no_temporary_file() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(&tmp.path().join("a").join("b"));
    save_choice(&file, &choice(&tmp.path().join("one"), 1)).unwrap();
    save_choice(&file, &choice(&tmp.path().join("two"), 2)).unwrap();
    assert_eq!(
        load_choice(&file),
        LoadedChoice::Found(choice(&tmp.path().join("two"), 2))
    );
    let names: Vec<_> = fs::read_dir(file.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, [CHOICE_FILE_NAME]);
}

#[test]
fn a_failed_save_leaves_the_old_choice_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    save_choice(&file, &choice(&tmp.path().join("keep"), 7)).unwrap();
    // The temporary name is taken by a folder, so the write cannot happen.
    fs::create_dir(file.with_extension("toml.tmp")).unwrap();
    assert!(save_choice(&file, &choice(&tmp.path().join("new"), 8)).is_err());
    assert_eq!(
        load_choice(&file),
        LoadedChoice::Found(choice(&tmp.path().join("keep"), 7))
    );
}

#[test]
fn a_failed_rename_leaves_no_temporary_file_behind() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    // A folder with something in it stands where the file should go: the rename must fail.
    fs::create_dir(&file).unwrap();
    fs::write(file.join("x"), "x").unwrap();
    let err = save_choice(&file, &choice(&tmp.path().join("g"), 1)).unwrap_err();
    assert!(matches!(err, Error::Io { .. }), "{err:?}");
    assert!(
        !file.with_extension("toml.tmp").exists(),
        "the temporary file was removed"
    );
}

#[test]
fn saving_into_a_place_that_cannot_be_written_is_an_error_not_a_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let blocker = tmp.path().join("blocker");
    fs::write(&blocker, "a file").unwrap();
    let err = save_choice(&choice_file(&blocker), &choice(&tmp.path().join("g"), 1)).unwrap_err();
    assert!(matches!(err, Error::Io { .. }), "{err:?}");
}

#[test]
fn a_relative_path_is_not_remembered() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    let err = save_choice(&file, &choice(Path::new("relative/dir"), 1)).unwrap_err();
    assert!(matches!(err, Error::Invalid { .. }), "{err:?}");
    assert!(!file.exists());
}

#[test]
fn a_missing_file_is_missing_not_damaged() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(load_choice(&choice_file(tmp.path())), LoadedChoice::Missing);
}

fn load_text(text: &str) -> LoadedChoice {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    fs::write(&file, text).unwrap();
    load_choice(&file)
}

fn abs() -> String {
    // A path that is absolute on this host.
    if cfg!(windows) {
        "C:/Games/SS2".to_owned()
    } else {
        "/games/ss2".to_owned()
    }
}

#[test]
fn every_kind_of_damaged_file_is_reported_and_none_panics() {
    for (text, needle) in [
        ("", "no folder path"),
        ("not toml {{{", "not valid TOML"),
        ("version = 1\n", "no folder path"),
        ("path = 5\n", "no folder path"),
        ("path = \"\"\n", "empty"),
        ("path = \"   \"\n", "empty"),
        ("path = \"relative/dir\"\n", "not absolute"),
        ("path = \"\\u0000\"\n", "cannot be real"),
        ("path = \"/a\\nb\"\n", "cannot be real"),
    ] {
        match load_text(text) {
            LoadedChoice::Damaged(why) => {
                assert!(why.contains(needle), "{text:?}: {why}");
                assert!(why.contains(CHOICE_FILE_NAME), "names the file: {why}");
            }
            other => panic!("{text:?} gave {other:?}"),
        }
    }
    let huge = format!("path = \"/{}\"\n", "a".repeat(5000));
    assert!(matches!(load_text(&huge), LoadedChoice::Damaged(_)));
    let at_limit = format!(
        "path = \"{}\"\n",
        if cfg!(windows) {
            format!("C:/{}", "a".repeat(4093))
        } else {
            format!("/{}", "a".repeat(4095))
        }
    );
    assert!(
        matches!(load_text(&at_limit), LoadedChoice::Found(_)),
        "4096 characters are accepted"
    );
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    fs::write(&file, [0xff, 0xfe, 0x00, 0x80]).unwrap();
    assert!(
        matches!(load_choice(&file), LoadedChoice::Damaged(_)),
        "bytes that are not text"
    );
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    assert!(
        matches!(load_choice(&file), LoadedChoice::Damaged(_)),
        "a folder in place of the file"
    );
}

#[test]
fn unknown_keys_and_a_bad_time_do_not_spoil_a_good_path() {
    let p = abs();
    for (text, at) in [
        (
            format!("path = \"{p}\"\nvalidated_at = 99\nfuture = [1,2]\n"),
            99,
        ),
        (format!("path = \"{p}\"\n"), 0),
        (format!("path = \"{p}\"\nvalidated_at = -5\n"), 0),
        (format!("path = \"{p}\"\nvalidated_at = \"later\"\n"), 0),
        (
            format!("version = 9\npath = \"{p}\"\nvalidated_at = 1.5\n"),
            0,
        ),
    ] {
        assert_eq!(
            load_text(&text),
            LoadedChoice::Found(RememberedChoice {
                dir: PathBuf::from(&p),
                validated_at: at
            }),
            "{text:?}"
        );
    }
}

#[test]
fn forgetting_removes_the_file_and_a_second_time_is_fine() {
    let tmp = tempfile::tempdir().unwrap();
    let file = choice_file(tmp.path());
    save_choice(&file, &choice(&tmp.path().join("g"), 1)).unwrap();
    forget_choice(&file).unwrap();
    assert_eq!(load_choice(&file), LoadedChoice::Missing);
    forget_choice(&file).unwrap();
    fs::create_dir(&file).unwrap();
    fs::write(file.join("x"), "x").unwrap();
    assert!(
        forget_choice(&file).is_err(),
        "a non-empty folder cannot be removed as a file"
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Start-up

struct Rig {
    tmp: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        Self {
            tmp: tempfile::tempdir().unwrap(),
        }
    }
    fn store(&self) -> PathBuf {
        choice_file(&self.tmp.path().join("config"))
    }
    fn install(&self, name: &str, files: &[&str]) -> PathBuf {
        let dir = self.tmp.path().join(name);
        install(&dir, files);
        dir
    }
    fn remember(&self, dir: &Path) {
        save_choice(&self.store(), &choice(dir, 5)).unwrap();
    }
}

fn clean() -> FakeEnvironment {
    windows()
}

#[test]
fn with_nothing_chosen_the_wizard_is_needed_for_the_first_run() {
    let rig = Rig::new();
    assert_eq!(
        resolve_startup(&clean(), None, &rig.store()),
        Startup::NeedsWizard(WizardReason::FirstRun)
    );
}

#[test]
fn a_remembered_install_that_still_passes_is_used_without_the_wizard() {
    let rig = Rig::new();
    let dir = rig.install("Serious Sam 2", &ALL);
    rig.remember(&dir);
    let Startup::Ready {
        dir: got,
        source,
        report,
    } = resolve_startup(&clean(), None, &rig.store())
    else {
        panic!("expected Ready");
    };
    assert_eq!((got, source), (dir, StartupSource::Remembered));
    assert!(report.errors().is_empty());
    assert_eq!(report.patch_level.unwrap().minor, 100);
}

#[test]
fn a_remembered_install_with_only_warnings_still_passes() {
    let rig = Rig::new();
    let dir = rig.install("g", &ALL[..5]);
    rig.remember(&dir);
    assert!(matches!(
        resolve_startup(&clean(), None, &rig.store()),
        Startup::Ready { .. }
    ));
}

#[test]
fn a_remembered_folder_that_vanished_sends_the_player_back_to_the_wizard() {
    let rig = Rig::new();
    let dir = rig.install("g", &ALL);
    rig.remember(&dir);
    fs::remove_dir_all(&dir).unwrap();
    let Startup::NeedsWizard(WizardReason::RememberedUnusable(s)) =
        resolve_startup(&clean(), None, &rig.store())
    else {
        panic!("expected RememberedUnusable");
    };
    assert!(!s.usable);
    assert_eq!(s.dir, dir);
    assert!(
        s.lines.iter().any(|l| l.contains("does not exist")),
        "{:?}",
        s.lines
    );
}

#[test]
fn a_remembered_folder_that_lost_an_archive_or_got_a_damaged_one_fails_the_check_again() {
    let rig = Rig::new();
    let dir = rig.install("g", &ALL);
    rig.remember(&dir);
    fs::remove_file(dir.join("All_PC_02.gro")).unwrap();
    assert!(matches!(
        resolve_startup(&clean(), None, &rig.store()),
        Startup::NeedsWizard(WizardReason::RememberedUnusable(_))
    ));
    write_gro(&dir, "All_PC_02.gro");
    assert!(matches!(
        resolve_startup(&clean(), None, &rig.store()),
        Startup::Ready { .. }
    ));
    fs::write(dir.join("All_PC_03.gro"), b"damaged").unwrap();
    assert!(matches!(
        resolve_startup(&clean(), None, &rig.store()),
        Startup::NeedsWizard(WizardReason::RememberedUnusable(_))
    ));
}

#[test]
fn a_damaged_choice_file_goes_to_the_wizard_with_the_reason() {
    let rig = Rig::new();
    fs::create_dir_all(rig.store().parent().unwrap()).unwrap();
    fs::write(rig.store(), "path = {{{").unwrap();
    let Startup::NeedsWizard(WizardReason::ChoiceDamaged(why)) =
        resolve_startup(&clean(), None, &rig.store())
    else {
        panic!("expected ChoiceDamaged");
    };
    assert!(why.contains("not valid TOML"), "{why}");
}

#[test]
fn a_folder_given_on_the_command_line_wins_over_the_remembered_one() {
    let rig = Rig::new();
    let remembered = rig.install("old", &ALL);
    let given = rig.install("given", &ALL);
    rig.remember(&remembered);
    let Startup::Ready { dir, source, .. } = resolve_startup(&clean(), Some(&given), &rig.store())
    else {
        panic!("expected Ready");
    };
    assert_eq!((dir, source), (given, StartupSource::Explicit));
}

#[test]
fn the_variable_is_used_when_nothing_is_given_and_loses_to_the_command_line() {
    let rig = Rig::new();
    let from_env = rig.install("env", &ALL);
    let given = rig.install("given", &ALL);
    let env = clean().with_var("SS2_DATA_DIR", from_env.to_str().unwrap());
    let Startup::Ready { dir, source, .. } = resolve_startup(&env, None, &rig.store()) else {
        panic!("expected Ready");
    };
    assert_eq!(
        (dir, source),
        (from_env.clone(), StartupSource::Environment)
    );
    let Startup::Ready { dir, source, .. } = resolve_startup(&env, Some(&given), &rig.store())
    else {
        panic!("expected Ready");
    };
    assert_eq!((dir, source), (given, StartupSource::Explicit));
}

#[test]
fn a_given_folder_that_fails_is_not_replaced_silently_by_the_remembered_one() {
    let rig = Rig::new();
    let good = rig.install("good", &ALL);
    rig.remember(&good);
    let bad = rig.tmp.path().join("typo");
    let Startup::NeedsWizard(WizardReason::GivenUnusable(source, s)) =
        resolve_startup(&clean(), Some(&bad), &rig.store())
    else {
        panic!("expected GivenUnusable");
    };
    assert_eq!(source, StartupSource::Explicit);
    assert_eq!(s.dir, bad);
    let env = clean().with_var("SS2_DATA_DIR", bad.to_str().unwrap());
    assert!(matches!(
        resolve_startup(&env, None, &rig.store()),
        Startup::NeedsWizard(WizardReason::GivenUnusable(StartupSource::Environment, _))
    ));
}

#[test]
fn an_empty_variable_is_ignored_at_start_up() {
    let rig = Rig::new();
    let env = clean().with_var("SS2_DATA_DIR", "");
    assert_eq!(
        resolve_startup(&env, None, &rig.store()),
        Startup::NeedsWizard(WizardReason::FirstRun)
    );
}

#[test]
fn resolving_writes_nothing() {
    let rig = Rig::new();
    let dir = rig.install("g", &ALL);
    let _ = resolve_startup(&clean(), Some(&dir), &rig.store());
    assert!(!rig.store().exists());
    assert!(!rig.store().parent().unwrap().exists());
}

#[test]
fn the_reasons_say_what_happened_in_one_sentence() {
    let s = ValidationSummary {
        dir: PathBuf::from("/some/folder"),
        usable: false,
        errors: 1,
        warnings: 0,
        outdated: false,
        headline: String::new(),
        lines: Vec::new(),
    };
    assert!(
        WizardReason::FirstRun
            .message()
            .contains("Choose the folder")
    );
    assert!(
        WizardReason::ChoiceDamaged("x".into())
            .message()
            .contains("could not be read")
    );
    let m = WizardReason::RememberedUnusable(s.clone()).message();
    assert!(
        m.contains("chose before") && m.contains("/some/folder"),
        "{m}"
    );
    let m = WizardReason::GivenUnusable(StartupSource::Explicit, s.clone()).message();
    assert!(m.contains("--data"), "{m}");
    let m = WizardReason::GivenUnusable(StartupSource::Environment, s.clone()).message();
    assert!(m.contains("SS2_DATA_DIR"), "{m}");
    let long = ValidationSummary {
        dir: PathBuf::from(format!("/{}", "d".repeat(200))),
        ..s
    };
    let m = WizardReason::RememberedUnusable(long).message();
    assert!(m.len() < 200, "a long folder is shortened: {m}");
    assert!(m.contains("..."), "{m}");
}

#[test]
fn labels_name_every_source_and_the_sources_sort_by_preference() {
    use CandidateSource::*;
    let all = [
        Remembered,
        Environment,
        CurrentDir,
        ExeDir,
        Steam,
        Gog,
        Retail,
    ];
    let mut sorted = all;
    sorted.sort();
    assert_eq!(sorted, all);
    let labels: Vec<_> = all.iter().map(|s| s.label()).collect();
    for (i, a) in labels.iter().enumerate() {
        assert!(!a.is_empty());
        assert!(labels[i + 1..].iter().all(|b| a != b), "{labels:?}");
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Data-gated: the player's own install

/// Skips without `SS2_DATA_DIR`.
#[test]
fn the_real_install_is_found_from_the_variable_and_validates() {
    let dir = ss2_testkit::require_game_data!();
    let d = discover(&SystemEnvironment, None);
    let want = dedupe_key(&dir);
    let found = d
        .candidates
        .iter()
        .find(|c| dedupe_key(&c.dir) == want)
        .unwrap_or_else(|| panic!("{dir:?} not among {:?}", d.candidates));
    assert!(found.sources.contains(&CandidateSource::Environment));
    assert_eq!(found.archives, 14);
    assert_eq!(
        found.patch.as_ref().map(|p| p.key()),
        Some(NEWEST_KNOWN_PATCH)
    );

    let started = std::time::Instant::now();
    let summary = summarize(&check_install(&dir));
    let took = started.elapsed();
    eprintln!(
        "validated the real install in {took:?}: {} / {:?}",
        summary.headline, summary.lines
    );
    assert!(summary.usable && !summary.outdated, "{summary:?}");
    assert_eq!(summary.lines[0], "Archives: 14 found, 14 readable");
    assert_eq!(summary.lines[1], "Patch level: 2.100 (build 800000)");

    // The whole loop: remember, then resolve from the memory alone (no variable, no argument).
    let tmp = tempfile::tempdir().unwrap();
    let store = choice_file(tmp.path());
    save_choice(&store, &choice(&dir, 1_700_000_000)).unwrap();
    let started = std::time::Instant::now();
    let startup = resolve_startup(&FakeEnvironment::new(Os::current()), None, &store);
    eprintln!(
        "re-validated the remembered install in {:?}",
        started.elapsed()
    );
    assert!(
        matches!(
            &startup,
            Startup::Ready {
                source: StartupSource::Remembered,
                ..
            }
        ),
        "{startup:?}"
    );
}
