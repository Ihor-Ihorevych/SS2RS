//! The stable-toolchain smoke test of the fuzz targets (T02-030).
//!
//! `cargo-fuzz` needs nightly Rust, so it cannot run on every pull request. This test feeds every target of
//! `ss2-fuzzkit` its synthetic seeds and a few thousand deterministic inputs made from them (bit flips, edge-value
//! numbers, cut and repeated ranges, spliced seeds, random bytes), and fails on a panic (also one that the code under
//! test catches itself), on an input that takes more than a few seconds, and on a seed that no longer reads.
//!
//! The same properties run under libFuzzer in `.github/workflows/nightly.yml`.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ss2_fuzzkit::mutate::{Rng, mutate, random_bytes};
use ss2_fuzzkit::{MAX_INPUT, TARGETS, Target};

/// An input must be handled within this time (debug build, shared machine).
const SLOW: Duration = Duration::from_secs(10);

fn iterations() -> usize {
    std::env::var("SS2_FUZZKIT_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000)
}

thread_local! {
    static WATCHING: Cell<bool> = const { Cell::new(false) };
}
static PANICS: AtomicUsize = AtomicUsize::new(0);

/// The input that is made at step `i` of the sweep for `seeds` (never empty).
fn input(rng: &mut Rng, seeds: &[Vec<u8>], i: usize) -> Vec<u8> {
    let pick = |rng: &mut Rng| seeds[rng.below(seeds.len())].clone();
    match i % 16 {
        // Random bytes, short and long.
        0 => {
            let len = rng.below(600);
            random_bytes(rng, len)
        }
        // A seed cut anywhere.
        1 => {
            let mut s = pick(rng);
            s.truncate(rng.below(s.len() + 1));
            s
        }
        // Two seeds spliced.
        2 => {
            let (a, b) = (pick(rng), pick(rng));
            let cut = rng.below(a.len() + 1);
            let from = rng.below(b.len() + 1);
            let mut out = a[..cut].to_vec();
            out.extend_from_slice(&b[from..]);
            out.truncate(MAX_INPUT);
            out
        }
        // The seed itself, unchanged.
        3 => pick(rng),
        // Damage: one edit most of the time, up to 12 now and then.
        _ => {
            let mut s = pick(rng);
            let edits = if i.is_multiple_of(5) {
                1 + rng.below(12)
            } else {
                1 + rng.below(2)
            };
            mutate(rng, &mut s, edits, MAX_INPUT);
            s
        }
    }
}

fn report(target: &Target, i: usize, data: &[u8]) -> String {
    let path: PathBuf =
        std::env::temp_dir().join(format!("ss2-fuzzkit-failure-{}-{i}.bin", target.name));
    let saved = std::fs::write(&path, data).is_ok();
    let head: String = data.iter().take(48).map(|b| format!("{b:02x}")).collect();
    format!(
        "target `{}` input #{i} ({} bytes, starts {head}{}){}",
        target.name,
        data.len(),
        if data.len() > 48 { "..." } else { "" },
        if saved {
            format!(", saved to {}", path.display())
        } else {
            String::new()
        }
    )
}

#[test]
fn every_target_survives_its_seeds_and_a_few_thousand_damaged_inputs() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if WATCHING.with(Cell::get) {
            PANICS.fetch_add(1, Ordering::SeqCst);
        }
        previous(info);
    }));
    WATCHING.with(|w| w.set(true));

    let n = iterations();
    let mut failures = Vec::new();
    let mut slowest = (Duration::ZERO, "");
    for (t, target) in TARGETS.iter().enumerate() {
        let seeds: Vec<Vec<u8>> = (target.seeds)().into_iter().map(|s| s.bytes).collect();
        assert!(!seeds.is_empty(), "target `{}` has no seeds", target.name);
        let mut rng = Rng::new(0xF00D_0000 + t as u64);
        let before = PANICS.load(Ordering::SeqCst);
        let mut run = |i: usize, data: &[u8]| {
            let start = Instant::now();
            let caught = panic::catch_unwind(AssertUnwindSafe(|| (target.run)(data)));
            let took = start.elapsed();
            if took > slowest.0 {
                slowest = (took, target.name);
            }
            if caught.is_err() {
                failures.push(format!("panicked: {}", report(target, i, data)));
            } else if took > SLOW {
                failures.push(format!("took {took:?}: {}", report(target, i, data)));
            }
        };
        for (i, s) in seeds.iter().enumerate() {
            run(usize::MAX - i, s);
        }
        for i in 0..n {
            let data = input(&mut rng, &seeds, i);
            run(i, &data);
        }
        let panics = PANICS.load(Ordering::SeqCst) - before;
        // A panic that the code catches itself (the Vorbis decoder is run under `catch_unwind`) is still a bug of the decoder
        // that libFuzzer would report: count it too.
        let reported = failures
            .iter()
            .filter(|f| f.contains(&format!("`{}`", target.name)))
            .count();
        if panics > reported {
            failures.push(format!(
                "target `{}`: {} panic(s) were caught inside the code under test",
                target.name,
                panics - reported
            ));
        }
    }
    WATCHING.with(|w| w.set(false));
    eprintln!("slowest input: {:?} ({})", slowest.0, slowest.1);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn seeds_are_small_unique_and_not_empty() {
    for target in TARGETS {
        let seeds = (target.seeds)();
        let names: BTreeSet<&str> = seeds.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names.len(),
            seeds.len(),
            "`{}` has two seeds of one name",
            target.name
        );
        for s in &seeds {
            assert!(!s.bytes.is_empty(), "{}/{} is empty", target.name, s.name);
            assert!(
                s.bytes.len() <= 8192,
                "{}/{} is {} bytes: seeds must stay tiny",
                target.name,
                s.name,
                s.bytes.len()
            );
            assert!(
                s.name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{}/{}: seed names are lower-case words",
                target.name,
                s.name
            );
        }
    }
}

#[test]
fn the_targets_have_distinct_names() {
    let names: BTreeSet<&str> = TARGETS.iter().map(|t| t.name).collect();
    assert_eq!(names.len(), TARGETS.len());
}

// ---- the seeds really are valid files of their format -----------------------------------------------------------------

fn seed(target: &str, name: &str) -> Vec<u8> {
    let t = TARGETS.iter().find(|t| t.name == target).expect("a target");
    (t.seeds)()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no seed {target}/{name}"))
        .bytes
}

#[test]
fn the_format_seeds_are_valid_files() {
    use ss2_format_fx::{lfx, pfx, tfx, vfx};
    let ok =
        |what: &str, r: Result<(), ss2_core::Error>| r.unwrap_or_else(|e| panic!("{what}: {e}"));
    ok(
        "meta sample",
        ss2_meta::Container::parse(&seed("meta_container", "sample"))
            .and_then(|c| c.decode_objects())
            .map(drop),
    );
    for name in ["bgr_8x4", "bgra_4x4", "dxt1_4x4", "cube_4x4"] {
        ok(
            name,
            ss2_format_tex::Texture::from_bytes(&seed("format_tex", name)).map(drop),
        );
    }
    for name in ["quad", "rich"] {
        ok(
            name,
            ss2_format_bmf::Mesh::from_bytes(&seed("format_bmf", name)).map(drop),
        );
    }
    ok(
        "ans",
        ss2_format_ans::AnimSet::from_bytes(&seed("format_ans", "walk")).map(drop),
    );
    ok(
        "ep",
        ss2_format_ep::EntityParams::from_bytes(&seed("format_ep", "projectile")).map(drop),
    );
    for name in ["sample", "one_glyph"] {
        ok(
            name,
            ss2_format_fnt::Font::from_bytes(&seed("format_fnt", name)).map(drop),
        );
    }
    ok(
        "lfx",
        lfx::FlareConfiguration::from_bytes(&seed("format_fx", "flare")).map(drop),
    );
    ok(
        "lfx2",
        lfx::FlareConfiguration::from_bytes(&seed("format_fx", "flare2")).map(drop),
    );
    ok(
        "tfx",
        tfx::TextEffectConfiguration::from_bytes(&seed("format_fx", "text")).map(drop),
    );
    ok(
        "vfx",
        vfx::VibrationEffect::from_bytes(&seed("format_fx", "vibro")).map(drop),
    );
    ok(
        "pfx",
        pfx::Effect::from_bytes(&seed("format_fx", "particles")).map(drop),
    );
    ok(
        "pfx box",
        pfx::Effect::from_bytes(&seed("format_fx", "particles_box")).map(drop),
    );
    ok(
        "scs",
        ss2_format_mat::SoundScheme::from_bytes(&seed("format_mat", "sound_scheme")).map(drop),
    );
    ok(
        "mat",
        ss2_format_mat::PhysicalMaterial::from_bytes(&seed("format_mat", "material")).map(drop),
    );
    ok(
        "mtr",
        ss2_format_mtr::MaterialFile::from_bytes(&seed("format_mtr", "preset")).map(drop),
    );
    for name in ["embedded", "external", "helper"] {
        ok(
            "mdl",
            ss2_format_mdl::Model::from_bytes(&seed("format_mdl", name)).map(drop),
        );
    }
    ok(
        "rsc",
        ss2_format_rsc::ResourceDatabase::from_bytes(&seed("format_rsc", "menu")).map(drop),
    );
    ok(
        "ctl",
        ss2_format_rsc::CharacterTool::from_bytes(&seed("format_rsc", "tool")).map(drop),
    );
}

#[test]
fn the_audio_and_state_seeds_are_valid() {
    use ss2_audio::decode;
    for name in [
        "ima_mono",
        "ima_stereo",
        "vorbis_mono",
        "vorbis_stereo",
        "pcm16_mono",
        "float_stereo",
    ] {
        decode::decode(&seed("audio_decode", name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    for name in ["one_shot", "loop_start", "loop_stop"] {
        ss2_audio_api::replication::AudioEvent::decode(&seed("audio_event", name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    ss2_game::campaign::CarryOver::from_bytes(&seed("game_state_bytes", "carry_empty"))
        .expect("carry");
    ss2_game::campaign::UnlockSet::from_bytes(&seed("game_state_bytes", "unlocks_empty"))
        .expect("unlocks");
    ss2_game::level_progress::LevelProgress::from_bytes(&seed("game_state_bytes", "progress"))
        .expect("progress");
    ss2_game::difficulty::CampaignDifficulty::from_bytes(&seed("game_state_bytes", "difficulty"))
        .expect("difficulty");
    ss2_game::campaign::CampaignNote::from_bytes(&seed("game_state_bytes", "note")).expect("note");
}

#[test]
fn the_text_seeds_are_valid() {
    let text = |t: &str, n: &str| String::from_utf8(seed(t, n)).expect("text");
    ss2_vfs::ModManifest::parse(&text("mods_text", "mod")).expect("mod");
    ss2_vfs::Nfo::parse(&seed("mods_text", "nfo_level")).expect("nfo");
    ss2_vfs::Nfo::parse(&seed("mods_text", "nfo_mod")).expect("nfo");
    ss2_vfs::parse_current_mod(&text("mods_text", "current")).expect("current");
    let reg = text("locate_text", "reg_query");
    let (value, rest) = reg.split_once('\n').expect("a value name line");
    assert!(
        ss2_vfs::parse_reg_query(rest, value).is_some(),
        "reg query seed"
    );
    assert_eq!(
        ss2_vfs::parse_library_folders(&text("locate_text", "library_folders")).len(),
        2
    );
    assert!(ss2_ui::lobby::parse_address(&text("lobby_text", "v4_port")).is_ok());
    assert!(ss2_ui::lobby::parse_address(&text("lobby_text", "v6_port")).is_ok());
    assert!(ss2_ui::lobby::parse_address(&text("lobby_text", "name")).is_ok());
    assert!(ss2_ui::lobby::parse_address(&text("lobby_text", "bad")).is_err());
}

// ---- the committed corpus equals what the builders write ---------------------------------------------------------------

fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds")
}

/// `UPDATE_FIXTURES=1 cargo test -p ss2-fuzzkit --test smoke committed_seeds` rewrites `fuzz/seeds/`.
#[test]
fn the_committed_seeds_are_what_the_builders_write() {
    let update = std::env::var_os("UPDATE_FIXTURES").is_some();
    let dir = seeds_dir();
    let mut expected = BTreeSet::new();
    let mut problems = Vec::new();
    for target in TARGETS {
        let folder = dir.join(target.name);
        for s in (target.seeds)() {
            let file = folder.join(format!("{}.seed", s.name));
            expected.insert(file.clone());
            if update {
                std::fs::create_dir_all(&folder).expect("create the seed folder");
                std::fs::write(&file, &s.bytes).expect("write a seed");
            } else {
                match std::fs::read(&file) {
                    Ok(on_disk) if on_disk == s.bytes => {}
                    Ok(_) => problems.push(format!("{} differs from the builder", file.display())),
                    Err(_) => problems.push(format!("{} is missing", file.display())),
                }
            }
        }
    }
    if let Ok(targets) = std::fs::read_dir(&dir) {
        for t in targets.flatten() {
            let Ok(files) = std::fs::read_dir(t.path()) else {
                continue;
            };
            for f in files.flatten() {
                let path = f.path();
                if !expected.contains(&path) {
                    if update {
                        let _ = std::fs::remove_file(&path);
                    } else {
                        problems.push(format!("{} belongs to no seed", path.display()));
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{}\n(run `UPDATE_FIXTURES=1 cargo test -p ss2-fuzzkit --test smoke the_committed_seeds` to rewrite them)",
        problems.join("\n")
    );
}

// ---- the fuzz crate calls what this crate defines -------------------------------------------------------------------------

#[test]
fn every_target_has_its_file_in_the_fuzz_crate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/fuzz_targets");
    for target in TARGETS {
        let path = root.join(format!("{}.rs", target.name));
        let source =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            source.contains(&format!("ss2_fuzzkit::{}(", target.name)),
            "{} does not call ss2_fuzzkit::{}",
            path.display(),
            target.name
        );
    }
}
