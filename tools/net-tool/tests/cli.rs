//! End-to-end tests: run the real `net-tool` binary.

use std::path::Path;
use std::process::{Command, Output};

use ss2_net::replay::{Record, ReplayHeader, ReplayWriter};

fn tool(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_net-tool"))
        .args(args)
        .output()
        .expect("failed to run net-tool")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn replay_bytes() -> Vec<u8> {
    let mut w = ReplayWriter::new(ReplayHeader {
        build: "ss2rs cli-test".into(),
        protocol: 1,
        schema_hash: 7,
        tick_hz: 30,
        seed: 99,
        map: "Sirius".into(),
        players: 1,
        meta: vec![],
    })
    .unwrap();
    for tick in 0..6u64 {
        if tick == 0 {
            w.push(Record::Snapshot {
                tick,
                data: vec![0; 8],
            })
            .unwrap();
        }
        w.push(Record::Input {
            tick,
            player: 0,
            data: vec![1, 2, 3],
        })
        .unwrap();
    }
    w.finish()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn inspect_prints_the_report_and_succeeds_even_for_a_cut_file() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = replay_bytes();
    let whole = write(dir.path(), "a.ss2replay", &bytes);
    let o = tool(&["replay", "inspect", &whole]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let text = stdout(&o);
    assert!(
        text.contains("map: Sirius") && text.contains("end: complete"),
        "{text}"
    );
    let cut = write(dir.path(), "b.ss2replay", &bytes[..bytes.len() - 10]);
    let o = tool(&["replay", "inspect", &cut]);
    assert_eq!(o.status.code(), Some(0));
    assert!(stdout(&o).contains("TRUNCATED"), "{}", stdout(&o));
}

#[test]
fn verify_says_ok_or_what_is_wrong_and_the_exit_code_tells_which() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = replay_bytes();
    let whole = write(dir.path(), "a.ss2replay", &bytes);
    let o = tool(&["replay", "verify", &whole]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(stdout(&o).starts_with("ok"), "{}", stdout(&o));
    let cut = write(dir.path(), "b.ss2replay", &bytes[..bytes.len() - 10]);
    let o = tool(&["replay", "verify", &cut]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stdout(&o).contains("TRUNCATED"), "{}", stdout(&o));
    let junk = write(dir.path(), "c.ss2replay", b"not a replay");
    let o = tool(&["replay", "verify", &junk]);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("not a replay file"), "{}", stderr(&o));
    let o = tool(&[
        "replay",
        "verify",
        &dir.path().join("missing").to_string_lossy(),
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert!(!stderr(&o).is_empty());
}

#[test]
fn dump_lists_records_and_honours_the_range() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "a.ss2replay", &replay_bytes());
    let o = tool(&["replay", "dump", &file, "--from", "4", "--to", "5"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(
        stdout(&o),
        "tick 4 input player 0 3 bytes\ntick 5 input player 0 3 bytes\n"
    );
    let all = tool(&["replay", "dump", &file]);
    assert_eq!(stdout(&all).lines().count(), 7);
}

#[test]
fn netsim_presets_lists_every_preset() {
    let o = tool(&["netsim", "presets"]);
    assert_eq!(o.status.code(), Some(0));
    let text = stdout(&o);
    for name in [
        "perfect",
        "lan",
        "broadband",
        "mobile",
        "bad_wifi",
        "satellite",
        "acceptance_100ms_2pct",
    ] {
        assert!(text.contains(name), "{name} in {text}");
    }
}

#[test]
fn netsim_run_reports_and_is_repeatable() {
    let args = [
        "netsim",
        "run",
        "--preset",
        "bad_wifi",
        "--seed",
        "5",
        "--packets",
        "500",
    ];
    let a = tool(&args);
    let b = tool(&args);
    assert_eq!(a.status.code(), Some(0), "{}", stderr(&a));
    assert!(
        stdout(&a).contains("received:") && stdout(&a).contains("latency (ms)"),
        "{}",
        stdout(&a)
    );
    assert_eq!(
        stdout(&a),
        stdout(&b),
        "the same seed gives the same report"
    );
    let c = tool(&[
        "netsim",
        "run",
        "--preset",
        "bad_wifi",
        "--seed",
        "6",
        "--packets",
        "500",
    ]);
    assert_ne!(stdout(&a), stdout(&c));
}

#[test]
fn netsim_run_overrides_change_the_link() {
    let o = tool(&[
        "netsim",
        "run",
        "--preset",
        "perfect",
        "--latency-ms",
        "50",
        "--packets",
        "10",
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(stdout(&o).contains("p50 50.000"), "{}", stdout(&o));
    let o = tool(&[
        "netsim",
        "run",
        "--preset",
        "perfect",
        "--loss-pct",
        "100",
        "--packets",
        "10",
    ]);
    assert_eq!(o.status.code(), Some(0));
    assert!(stdout(&o).contains("no packet arrived"), "{}", stdout(&o));
}

#[test]
fn netsim_run_rejects_what_makes_no_sense_with_exit_code_2() {
    let o = tool(&["netsim", "run", "--preset", "dialup"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        stderr(&o).contains("dialup") && stderr(&o).contains("bad_wifi"),
        "{}",
        stderr(&o)
    );
    let o = tool(&["netsim", "run", "--loss-pct", "150"]);
    assert_eq!(o.status.code(), Some(2));
    let o = tool(&["netsim", "run", "--packets", "0"]);
    assert_eq!(o.status.code(), Some(2));
}
