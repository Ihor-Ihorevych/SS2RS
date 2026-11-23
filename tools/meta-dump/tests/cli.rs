//! End-to-end tests: run the real `meta-dump` binary on fixture files and on a synthetic install.

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::*;
use ss2_testkit::fixtures::ZipBuilder;

fn tool(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_meta-dump"))
        .env_remove("SS2_DATA_DIR")
        .args(args)
        .output()
        .expect("failed to run meta-dump")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn prints_the_annotated_text_of_a_file() {
    let d = tempfile::tempdir().unwrap();
    let path = write(d.path(), "sample.meta", &sample());
    let o = tool(&[&path]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let text = out(&o);
    assert!(text.starts_with("# meta-dump"));
    assert!(text.contains("== chunks"));
    assert!(text.contains("CTestHolder { CTestItem.1 = -1") || text.contains("CTestHolder {"));
    assert!(text.contains("explained   860 bytes (100.0000%)"));
    assert!(err(&o).is_empty());
}

#[test]
fn format_flags_select_json_and_ron() {
    let d = tempfile::tempdir().unwrap();
    let path = write(d.path(), "rich.meta", &rich());
    let j = tool(&["--format", "json", &path]);
    assert_eq!(j.status.code(), Some(0));
    let jv: serde_json::Value = serde_json::from_str(&out(&j)).unwrap();
    assert_eq!(jv["status"], "ok");
    let r = tool(&["--format", "ron", &path]);
    assert_eq!(r.status.code(), Some(0));
    assert_eq!(parse_ron(&out(&r)).unwrap(), jv);
    let bad = tool(&["--format", "yaml", &path]);
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn raw_preview_and_skip_values_flags_are_honored() {
    let d = tempfile::tempdir().unwrap();
    let path = write(d.path(), "sample.meta", &sample());
    let o = tool(&["--raw", "--preview", "4", &path]);
    assert!(out(&o).contains("| 43 54 53 45 ..."));
    let o = tool(&["--skip-values", &path]);
    assert!(!out(&o).contains("CTestItem.2 = 30.0"));
}

#[test]
fn unknown_mode_exits_1_when_bytes_are_undecoded_and_0_when_all_is_explained() {
    let d = tempfile::tempdir().unwrap();
    let good = write(d.path(), "sample.meta", &sample());
    let o = tool(&["--unknown", &good]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(out(&o).contains("undecoded   0 bytes in 0 range(s)"));
    let broken = write(d.path(), "broken.meta", &broken());
    let o = tool(&["--unknown", &broken]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(out(&o).contains("0x00000169..0x00000186  29 bytes  in OBJS"));
    // Without --unknown a file with undecoded bytes still exits 0.
    assert_eq!(tool(&[&broken]).status.code(), Some(0));
}

#[test]
fn a_damaged_file_prints_a_report_and_exits_2() {
    let d = tempfile::tempdir().unwrap();
    let mut bytes = sample();
    bytes.truncate(100);
    let path = write(d.path(), "cut.meta", &bytes);
    let o = tool(&[&path]);
    assert_eq!(o.status.code(), Some(2));
    assert!(out(&o).contains("status: damaged"));
    assert!(err(&o).contains("meta-dump:"));
    let o = tool(&["--format", "json", &path]);
    assert_eq!(o.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_str(&out(&o)).unwrap();
    assert_eq!(v["status"], "damaged");
}

#[test]
fn a_missing_file_is_an_error_message_and_exit_2() {
    let o = tool(&["definitely/not/here.meta"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("cannot read"), "{}", err(&o));
    assert!(out(&o).is_empty());
}

fn install(dir: &Path) {
    let base = ZipBuilder::new()
        .add("Content/Test/a.meta", sample())
        .add("Content/Test/b.meta", rich())
        .finish()
        .unwrap();
    std::fs::write(dir.join("All_PC_01.gro"), base).unwrap();
    let patch = ZipBuilder::new()
        .add("content/test/b.meta", broken())
        .finish()
        .unwrap();
    std::fs::write(dir.join("Patch_02_068.gro"), patch).unwrap();
}

#[test]
fn reads_a_file_of_the_install_through_the_vfs_and_the_newest_patch_wins() {
    let d = tempfile::tempdir().unwrap();
    install(d.path());
    let dir = d.path().to_str().unwrap();
    let o = tool(&["--install", dir, "--format", "json", "content/test/a.meta"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let v: serde_json::Value = serde_json::from_str(&out(&o)).unwrap();
    assert_eq!(v["file"], "content/test/a.meta");
    assert_eq!(v["size"], 860);
    // The patch replaced b.meta with the broken fixture.
    let o = tool(&["--install", dir, "--unknown", "Content/Test/B.meta"]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(out(&o).contains("29 bytes in 1 range(s)"));
}

#[test]
fn install_errors_are_messages() {
    let d = tempfile::tempdir().unwrap();
    install(d.path());
    let dir = d.path().to_str().unwrap();
    let o = tool(&["--install", dir, "content/nothing.meta"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        err(&o).contains("cannot read content/nothing.meta"),
        "{}",
        err(&o)
    );
    let empty = tempfile::tempdir().unwrap();
    let o = tool(&["--install", empty.path().to_str().unwrap(), "x"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("cannot mount"), "{}", err(&o));
}

#[test]
fn output_is_identical_between_runs() {
    let d = tempfile::tempdir().unwrap();
    let path = write(d.path(), "rich.meta", &rich());
    for fmt in ["text", "json", "ron"] {
        let a = tool(&["--format", fmt, "--raw", &path]);
        let b = tool(&["--format", fmt, "--raw", &path]);
        assert_eq!(a.stdout, b.stdout);
    }
}
