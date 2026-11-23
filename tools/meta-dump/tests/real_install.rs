//! Data-gated: the report of the CTSEMETA files of the player's install. Skips without
//! `SS2_DATA_DIR`. Prints the totals (run with `--nocapture`).
//!
//! By default a deterministic sample is walked: every 20th candidate file in path order plus the
//! five largest, both among the files of at most 4 MiB (no worlds). Set `SS2_META_DUMP_FULL=1` (and use `--release`) to walk every file; that takes
//! several minutes.

use std::collections::BTreeMap;
use std::io::Read;

use meta_dump::{Format, Options, Outcome, analyze, dump};
use ss2_vfs::{EntryKind, Vfs, VfsPath};

/// Files up to this size are also printed with all their values.
const FULL_PRINT_LIMIT: usize = 64 * 1024;
/// The default sample leaves out files above this size (bytes).
const SAMPLE_MAX_SIZE: u64 = 4 * 1024 * 1024;
/// Every n-th candidate is in the sample.
const SAMPLE_STEP: usize = 20;
/// Extensions of the CTSEMETA files of the retail install (`docs/re/inventory.md`); the magic is
/// checked on every candidate, so a file that is not CTSEMETA is skipped, not failed.
const EXTENSIONS: &[&str] = &[
    "ans", "bmf", "cb", "ctl", "ctr", "ep", "fnt", "lfx", "mac", "mas", "mat", "mch", "mdl", "mtr",
    "pfx", "pss", "rsc", "sca", "scp", "scs", "sfc", "skl", "tex", "tfx", "vfx", "wld",
];

struct Totals {
    files: usize,
    bytes: usize,
    decoded: usize,
    opaque: usize,
    undecoded_files: usize,
    undecoded_bytes: usize,
    printed_in_full: usize,
    by_ext: BTreeMap<String, (usize, usize, usize)>,
    failures: Vec<String>,
    problems: Vec<String>,
}

fn walk(vfs: &dyn Vfs) -> Vec<(VfsPath, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![VfsPath::root()];
    while let Some(d) = stack.pop() {
        for e in vfs.list(&d).unwrap() {
            let path = d.join(&e.name).unwrap();
            if e.kind == EntryKind::Dir {
                stack.push(path);
            } else {
                let size = vfs.stat(&path).unwrap().size;
                out.push((path, size));
            }
        }
    }
    out.sort();
    out
}

fn run(vfs: &dyn Vfs, candidates: &[(VfsPath, u64)]) -> Totals {
    // Decoding and coverage do not need the value trees; the printers of the values run on the
    // files up to `FULL_PRINT_LIMIT` bytes (a world prints 200 MB of JSON).
    let opts = Options {
        skip_values: true,
        ..Options::default()
    };
    let full = Options::default();
    let mut t = Totals {
        files: 0,
        bytes: 0,
        decoded: 0,
        opaque: 0,
        undecoded_files: 0,
        undecoded_bytes: 0,
        printed_in_full: 0,
        by_ext: BTreeMap::new(),
        failures: Vec::new(),
        problems: Vec::new(),
    };
    for (path, _) in candidates {
        let mut magic = [0u8; 8];
        let is_meta = vfs
            .open(path)
            .map(|mut f| f.read_exact(&mut magic).is_ok())
            .unwrap_or(false)
            && &magic == b"CTSEMETA";
        if !is_meta {
            continue;
        }
        let data = vfs.read(path).unwrap();
        let out = analyze(&data, path.as_str(), &opts);
        let cov = out.coverage();
        assert_eq!(cov.decoded + cov.undecoded_bytes(), data.len());
        match &out {
            Outcome::Damaged(d) => t.failures.push(format!("{}: {}", path.as_str(), d.error)),
            Outcome::Parsed(r) => {
                for p in &r.problems {
                    t.problems.push(format!("{}: {p}", path.as_str()));
                }
            }
        }
        let small = data.len() <= FULL_PRINT_LIMIT;
        t.printed_in_full += usize::from(small);
        for f in [Format::Text, Format::Json, Format::Ron] {
            let d = dump(&data, path.as_str(), f, if small { &full } else { &opts });
            assert!(!d.text.is_empty());
        }
        t.files += 1;
        t.bytes += data.len();
        t.decoded += cov.decoded;
        t.opaque += cov.opaque;
        if cov.undecoded_bytes() > 0 {
            t.undecoded_files += 1;
            t.undecoded_bytes += cov.undecoded_bytes();
        }
        let ext = path.extension().unwrap_or("(none)").to_string();
        let s = t.by_ext.entry(ext).or_default();
        s.0 += 1;
        s.1 += data.len();
        s.2 += cov.decoded;
    }
    t
}

fn pct(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole.max(1) as f64
}

#[test]
fn ctsemeta_files_of_the_real_install_are_dumped() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let full_walk = std::env::var_os("SS2_META_DUMP_FULL").is_some_and(|v| v == "1");
    let mut candidates: Vec<(VfsPath, u64)> = walk(&vfs)
        .into_iter()
        .filter(|(p, _)| p.extension().is_some_and(|e| EXTENSIONS.contains(&e)))
        .collect();
    if !full_walk {
        // Debug builds are slow on the 20 MB worlds; the sample leaves out what is over
        // `SAMPLE_MAX_SIZE` (the full walk covers them).
        candidates.retain(|(_, size)| *size <= SAMPLE_MAX_SIZE);
        let mut largest = candidates.clone();
        largest.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut sample: Vec<(VfsPath, u64)> = candidates
            .iter()
            .step_by(SAMPLE_STEP)
            .cloned()
            .chain(largest.into_iter().take(5))
            .collect();
        sample.sort();
        sample.dedup();
        candidates = sample;
    }
    let started = std::time::Instant::now();
    let t = run(&vfs, &candidates);
    eprintln!(
        "meta-dump over the install ({}): {} CTSEMETA files, {} bytes; explained {} bytes \
         ({:.4}%), of which opaque {} ({:.4}% of all bytes); undecoded {} bytes in {} files; {} \
         frame failures; {} problems; {} files printed with values; {:.1}s",
        if full_walk { "full walk" } else { "sample" },
        t.files,
        t.bytes,
        t.decoded,
        pct(t.decoded, t.bytes),
        t.opaque,
        pct(t.opaque, t.bytes),
        t.undecoded_bytes,
        t.undecoded_files,
        t.failures.len(),
        t.problems.len(),
        t.printed_in_full,
        started.elapsed().as_secs_f64()
    );
    for (ext, (n, total, ok)) in &t.by_ext {
        eprintln!(
            "  .{ext:<6} {n:>5} files {total:>12} bytes  explained {:.4}%",
            pct(*ok, *total)
        );
    }
    for p in t.problems.iter().take(20) {
        eprintln!("  problem: {p}");
    }
    assert!(t.failures.is_empty(), "{:#?}", t.failures);
    assert!(t.files > 0, "the install holds CTSEMETA files");
    assert!(
        t.problems.is_empty(),
        "{} problems, first: {:?}",
        t.problems.len(),
        t.problems.first()
    );
    assert_eq!(
        t.undecoded_bytes, 0,
        "every byte of the install files is explained"
    );
}
