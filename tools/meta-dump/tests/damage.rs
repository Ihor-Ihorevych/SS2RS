//! Damaged input never panics: every truncation and every single-byte corruption of the fixtures,
//! in all three formats, plus absurd counts.

mod common;

use common::*;
use meta_dump::{Format, Options, Outcome, analyze, dump};

fn fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("sample", sample()),
        ("rich", rich()),
        ("broken", broken()),
        ("old4", old_file(4)),
        ("old7", old_file(7)),
    ]
}

/// The invariants of any report, whatever the input.
fn check(data: &[u8], what: &str) {
    for opts in [
        Options::default(),
        Options {
            raw: true,
            unknown_only: true,
            ..Options::default()
        },
    ] {
        let out = analyze(data, what, &opts);
        let cov = out.coverage();
        assert_eq!(cov.total, data.len(), "{what}");
        assert_eq!(cov.decoded + cov.undecoded_bytes(), data.len(), "{what}");
        let mut last = 0;
        for g in &cov.undecoded {
            assert!(g.span.start >= last && g.span.start < g.span.end, "{what}");
            assert!(g.span.end <= data.len(), "{what}");
            last = g.span.end;
        }
        if let Outcome::Parsed(r) = &out {
            for c in &r.chunks {
                assert!(
                    c.span.start <= c.span.end && c.span.end <= data.len(),
                    "{what}"
                );
            }
        }
        for f in [Format::Text, Format::Json, Format::Ron] {
            let d = dump(data, what, f, &opts);
            assert!(!d.text.is_empty());
            assert_eq!(d.parsed, out.is_parsed());
        }
    }
}

#[test]
fn every_truncation_is_reported_not_a_panic() {
    for (name, bytes) in fixtures() {
        for len in 0..bytes.len() {
            let cut = &bytes[..len];
            check(cut, &format!("{name}[..{len}]"));
            assert!(
                !analyze(cut, "t", &Options::default()).is_parsed(),
                "{name}[..{len}] must be rejected (the footer is missing)"
            );
        }
    }
}

#[test]
fn every_single_byte_corruption_is_survived() {
    for (name, bytes) in fixtures() {
        for i in 0..bytes.len() {
            for v in [0xFFu8, bytes[i] ^ 0x01, bytes[i].wrapping_add(0x80)] {
                let mut b = bytes.clone();
                b[i] = v;
                check(&b, &format!("{name}[{i}]={v:#04x}"));
            }
        }
    }
}

#[test]
fn absurd_counts_are_rejected_without_allocating() {
    let bytes = rich();
    let Outcome::Parsed(r) = analyze(&bytes, "t", &Options::default()) else {
        panic!("rich must parse");
    };
    for c in &r.chunks {
        if c.entries.is_none() {
            continue;
        }
        let mut b = bytes.clone();
        b[c.span.start + 4..c.span.start + 8].copy_from_slice(&u32::MAX.to_le_bytes());
        check(&b, &format!("count of {}", c.tag));
    }
    // A string length of 4 GiB in the header.
    let mut b = bytes.clone();
    b[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    check(&b, "header string length");
}

#[test]
fn garbage_and_tiny_inputs() {
    check(&[], "empty");
    check(b"CTSEMETA", "magic only");
    check(&[0xFF; 100], "ff");
    check(&[0; 100], "zeros");
    let mut x = 0x1234_5678u32;
    let noise: Vec<u8> = (0..2000)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect();
    check(&noise, "noise");
    // Noise behind a valid header.
    let mut b = b"CTSEMETA".to_vec();
    b.extend_from_slice(&0x1234_ABCDu32.to_le_bytes());
    b.extend_from_slice(&8u32.to_le_bytes());
    b.extend_from_slice(&noise);
    check(&b, "valid magic, noise");
}
