//! What a load order of mods does to the files: which paths each mod adds or replaces, and where
//! two mods provide the same path, which one wins (roadmap T13-001; the UI of T13-007 and the
//! validator of T13-009 read this).
//!
//! The rule being reported is the one of [`crate::LayeredVfs`]: **the layer mounted last wins**,
//! case-insensitively by path. A mod's layers are its archives in mount order; the mods are in
//! load order; the base game (archives and patches) is below all of them.
//!
//! Only files are compared. A folder is never a conflict (listings are merged).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use ss2_core::Result;

use crate::modcheck::{ascii, plural, scan_layer};
use crate::modinstall::{InstalledMods, open_mod_archives};
use crate::modload::LoadPlan;
use crate::{EntryKind, Stat, Vfs, VfsPath, mount_install};

/// Two files are compared byte for byte only up to this size; a larger pair is `Unknown`.
const MAX_COMPARE_BYTES: u64 = 8 << 20;
/// How many example paths a [`PairSummary`] keeps.
const EXAMPLES: usize = 3;

/// One mod's layers for [`analyze_overlays`]: the id and the layers in **mount order** (the last
/// one wins).
pub struct ModLayers<'a> {
    /// The mod's id.
    pub id: String,
    /// `(name, file system)` per layer; the name is for messages (an archive's file name).
    pub layers: Vec<(String, &'a dyn Vfs)>,
}

/// Whether two copies of a file are the same bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sameness {
    /// Same size and same bytes: the conflict is harmless.
    Identical,
    /// Different size or different bytes.
    Different,
    /// Not compared: a file over 8 MiB with the same size, or one that could not be read.
    Unknown,
}

/// One file that two or more mods provide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathConflict {
    /// The path (lower case, `/`).
    pub path: VfsPath,
    /// The mod whose copy the game sees (the one loaded last).
    pub winner: String,
    /// The other mods that provide the path, in load order.
    pub losers: Vec<String>,
    /// `true` if the base game has this path too (every mod here replaces it).
    pub replaces_base: bool,
    /// How the winner's copy compares with the losers': `Identical` only when it is the same
    /// as every one of them, `Different` when it differs from at least one.
    pub content: Sameness,
}

/// How one mod overrides another across all their shared files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairSummary {
    /// The mod that is loaded later and wins.
    pub winner: String,
    /// The mod that is overridden.
    pub loser: String,
    /// How many files both provide.
    pub paths: usize,
    /// ...of which the same bytes.
    pub identical: usize,
    /// ...of which different.
    pub different: usize,
    /// ...of which not compared.
    pub unknown: usize,
    /// The first paths (in path order), at most three.
    pub examples: Vec<VfsPath>,
}

/// What one mod does to the files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModOverlay {
    /// The mod's id.
    pub id: String,
    /// Distinct files the mod provides.
    pub files: usize,
    /// ...that replace a file of the base game.
    pub replaces_base: usize,
    /// ...that the base game does not have.
    pub new_files: usize,
    /// Files that a later archive of the same mod replaces again (not counted in conflicts).
    pub inner_overrides: usize,
    /// Files that another, later mod replaces: this mod's copy is not seen.
    pub lost_to_other_mods: usize,
    /// Files this mod provides and an earlier mod provides too: this mod's copy is seen.
    pub won_against_other_mods: usize,
}

/// The result of [`analyze_overlays`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayReport {
    /// One entry per mod, in load order.
    pub mods: Vec<ModOverlay>,
    /// Every file that two or more mods provide, sorted by path.
    pub conflicts: Vec<PathConflict>,
    /// The same, summed per pair of mods: sorted by the winner's place in the load order, then
    /// the loser's.
    pub pairs: Vec<PairSummary>,
    /// Folders or files that could not be read; what they would have provided is missing from
    /// the numbers above.
    pub problems: Vec<String>,
}

/// Work out what the mods in `mods` (in load order) do on top of `base`.
///
/// Reads every layer's directory tree once and, for the files that two mods provide, compares
/// them (up to 8 MiB each). Never panics on damaged input: a layer that cannot be listed is
/// reported in [`OverlayReport::problems`]. Mod ids should be distinct.
pub fn analyze_overlays(base: &dyn Vfs, mods: &[ModLayers<'_>]) -> OverlayReport {
    let mut problems = Vec::new();
    // Every path, with who provides it: (mod, layer, size) in load order.
    let mut provided: BTreeMap<VfsPath, Vec<(usize, usize, u64)>> = BTreeMap::new();
    for (mi, m) in mods.iter().enumerate() {
        for (li, (name, vfs)) in m.layers.iter().enumerate() {
            let (files, errors) = scan_layer(*vfs);
            for e in errors {
                problems.push(format!("{}/{name}: {}: {}", m.id, e.path, e.message));
            }
            for (path, size) in files {
                provided.entry(path).or_default().push((mi, li, size));
            }
        }
    }

    let mut stats: Vec<ModOverlay> = mods
        .iter()
        .map(|m| ModOverlay {
            id: m.id.clone(),
            files: 0,
            replaces_base: 0,
            new_files: 0,
            inner_overrides: 0,
            lost_to_other_mods: 0,
            won_against_other_mods: 0,
        })
        .collect();
    let mut conflicts = Vec::new();
    let mut pairs: BTreeMap<(usize, usize), PairSummary> = BTreeMap::new();
    for (path, list) in provided {
        let in_base = matches!(
            base.stat(&path),
            Ok(Stat {
                kind: EntryKind::File,
                ..
            })
        );
        // The last layer of each mod decides what that mod provides: (mod, layer, size, layers).
        let mut per_mod: Vec<(usize, usize, u64, usize)> = Vec::new();
        for (mi, li, size) in list {
            match per_mod.last_mut() {
                Some(last) if last.0 == mi => {
                    last.1 = li;
                    last.2 = size;
                    last.3 += 1;
                }
                _ => per_mod.push((mi, li, size, 1)),
            }
        }
        for &(mi, _, _, layers) in &per_mod {
            let s = &mut stats[mi];
            s.files += 1;
            if in_base {
                s.replaces_base += 1;
            } else {
                s.new_files += 1;
            }
            s.inner_overrides += layers - 1;
        }
        let Some((&winner, losers)) = per_mod.split_last().filter(|(_, l)| !l.is_empty()) else {
            continue;
        };
        stats[winner.0].won_against_other_mods += 1;
        let mut content = Sameness::Identical;
        for &loser in losers {
            stats[loser.0].lost_to_other_mods += 1;
            let same = compare(
                mods,
                (winner.0, winner.1, winner.2),
                (loser.0, loser.1, loser.2),
                &path,
            );
            content = match (content, same) {
                (Sameness::Different, _) | (_, Sameness::Different) => Sameness::Different,
                (Sameness::Unknown, _) | (_, Sameness::Unknown) => Sameness::Unknown,
                _ => Sameness::Identical,
            };
            let pair = pairs
                .entry((winner.0, loser.0))
                .or_insert_with(|| PairSummary {
                    winner: mods[winner.0].id.clone(),
                    loser: mods[loser.0].id.clone(),
                    paths: 0,
                    identical: 0,
                    different: 0,
                    unknown: 0,
                    examples: Vec::new(),
                });
            pair.paths += 1;
            match same {
                Sameness::Identical => pair.identical += 1,
                Sameness::Different => pair.different += 1,
                Sameness::Unknown => pair.unknown += 1,
            }
            if pair.examples.len() < EXAMPLES {
                pair.examples.push(path.clone());
            }
        }
        conflicts.push(PathConflict {
            path,
            winner: mods[winner.0].id.clone(),
            losers: losers.iter().map(|l| mods[l.0].id.clone()).collect(),
            replaces_base: in_base,
            content,
        });
    }
    OverlayReport {
        mods: stats,
        conflicts,
        pairs: pairs.into_values().collect(),
        problems,
    }
}

/// Compare the final copies `a` and `b` (mod, layer, size) of `path`.
fn compare(
    mods: &[ModLayers<'_>],
    a: (usize, usize, u64),
    b: (usize, usize, u64),
    path: &VfsPath,
) -> Sameness {
    if a.2 != b.2 {
        return Sameness::Different;
    }
    if a.2 > MAX_COMPARE_BYTES {
        return Sameness::Unknown;
    }
    let read = |(mi, li, _): (usize, usize, u64)| mods[mi].layers[li].1.read(path).ok();
    match (read(a), read(b)) {
        (Some(x), Some(y)) if x == y => Sameness::Identical,
        (Some(_), Some(_)) => Sameness::Different,
        _ => Sameness::Unknown,
    }
}

impl OverlayReport {
    /// A plain ASCII report: the mods with their counts, then the conflicts per pair.
    pub fn render(&self) -> String {
        if self.mods.is_empty() {
            return "No mods are loaded: only the base game.\n".to_owned();
        }
        let mut out = String::from("Mods in load order (a later mod wins):\n");
        for (i, m) in self.mods.iter().enumerate() {
            let _ = writeln!(
                out,
                "  {}. {}: {} file{}; base game files replaced: {}; new: {}; replaced again inside the mod: {}",
                i + 1,
                ascii(&m.id),
                m.files,
                plural(m.files),
                m.replaces_base,
                m.new_files,
                m.inner_overrides
            );
            let _ = writeln!(
                out,
                "     overridden by later mods: {}; overriding earlier mods: {}",
                m.lost_to_other_mods, m.won_against_other_mods
            );
        }
        if self.conflicts.is_empty() {
            out.push_str("No file is provided by more than one mod.\n");
        } else {
            let _ = writeln!(
                out,
                "Files that more than one mod provides: {}",
                self.conflicts.len()
            );
            for s in &self.pairs {
                let more = if s.paths > s.examples.len() {
                    ", ..."
                } else {
                    ""
                };
                let examples: Vec<String> = s.examples.iter().map(|p| ascii(p.as_str())).collect();
                let _ = writeln!(
                    out,
                    "  {} wins over {}: {} file{} (identical: {}, different: {}, not compared: {}): {}{more}",
                    ascii(&s.winner),
                    ascii(&s.loser),
                    s.paths,
                    plural(s.paths),
                    s.identical,
                    s.different,
                    s.unknown,
                    examples.join(", ")
                );
            }
        }
        for problem in &self.problems {
            let _ = writeln!(out, "Not read: {}", ascii(problem));
        }
        out
    }
}

/// [`analyze_overlays`] for the mods of `plan` in the install `dir`, as [`crate::mount_mods`]
/// mounts them (the archives of `installed` are opened again).
///
/// # Errors
/// As [`crate::mount_mods`]: the install cannot be read, a planned mod is not installed or has
/// an archive that cannot be opened.
pub fn overlay_report(
    dir: impl AsRef<Path>,
    installed: &InstalledMods,
    plan: &LoadPlan,
) -> Result<OverlayReport> {
    let dir = dir.as_ref();
    let base = mount_install(dir)?;
    let mut opened = Vec::new();
    for planned in &plan.mods {
        opened.push((
            planned.id.clone(),
            open_mod_archives(dir, installed, &planned.id)?,
        ));
    }
    let mods: Vec<ModLayers<'_>> = opened
        .iter()
        .map(|(id, archives)| ModLayers {
            id: id.clone(),
            layers: archives
                .iter()
                .map(|(name, archive)| (name.clone(), archive as &dyn Vfs))
                .collect(),
        })
        .collect();
    Ok(analyze_overlays(&base, &mods))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryVfs;
    use crate::modinstall::{fixture::fake_install, mount_mods, scan_installed_mods};
    use ss2_core::Error;

    fn mem(files: &[(&str, &str)]) -> MemoryVfs {
        let mut v = MemoryVfs::new();
        for (n, d) in files {
            v.insert(n, d.as_bytes().to_vec()).unwrap();
        }
        v
    }

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    struct World {
        base: MemoryVfs,
        alpha_a: MemoryVfs,
        alpha_z: MemoryVfs,
        beta: MemoryVfs,
        gamma: MemoryVfs,
    }

    fn world() -> World {
        World {
            base: mem(&[("x.txt", "base"), ("b.txt", "b"), ("dir/deep.txt", "d")]),
            alpha_a: mem(&[
                ("x.txt", "alpha"),
                ("a.txt", "1"),
                ("shared.txt", "same"),
                ("only-alpha.txt", "o"),
            ]),
            alpha_z: mem(&[("a.txt", "2")]),
            beta: mem(&[
                ("X.TXT", "beta"),
                ("a.txt", "1"),
                ("shared.txt", "same"),
                ("only-beta.txt", "o"),
            ]),
            gamma: mem(&[("x.txt", "beta")]),
        }
    }

    fn layers(w: &World) -> Vec<ModLayers<'_>> {
        vec![
            ModLayers {
                id: "alpha".into(),
                layers: vec![("a.gro".into(), &w.alpha_a), ("z.gro".into(), &w.alpha_z)],
            },
            ModLayers {
                id: "beta".into(),
                layers: vec![("b.gro".into(), &w.beta)],
            },
            ModLayers {
                id: "gamma".into(),
                layers: vec![("g.gro".into(), &w.gamma)],
            },
        ]
    }

    #[test]
    fn counts_per_mod() {
        let w = world();
        let report = analyze_overlays(&w.base, &layers(&w));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let n = |id: &str| report.mods.iter().find(|m| m.id == id).unwrap().clone();
        assert_eq!(
            report
                .mods
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta", "gamma"]
        );
        assert_eq!(
            n("alpha"),
            ModOverlay {
                id: "alpha".into(),
                files: 4,
                replaces_base: 1,
                new_files: 3,
                inner_overrides: 1,
                lost_to_other_mods: 3,
                won_against_other_mods: 0,
            }
        );
        assert_eq!(
            n("beta"),
            ModOverlay {
                id: "beta".into(),
                files: 4,
                replaces_base: 1,
                new_files: 3,
                inner_overrides: 0,
                lost_to_other_mods: 1,
                won_against_other_mods: 2,
            }
        );
        assert_eq!(
            n("gamma"),
            ModOverlay {
                id: "gamma".into(),
                files: 1,
                replaces_base: 1,
                new_files: 0,
                inner_overrides: 0,
                lost_to_other_mods: 0,
                won_against_other_mods: 1,
            }
        );
    }

    #[test]
    fn the_conflicts_name_the_winner_and_the_losers() {
        let w = world();
        let report = analyze_overlays(&w.base, &layers(&w));
        let got: Vec<_> = report
            .conflicts
            .iter()
            .map(|c| {
                (
                    c.path.as_str(),
                    c.winner.as_str(),
                    c.losers.iter().map(String::as_str).collect::<Vec<_>>(),
                    c.replaces_base,
                    c.content,
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("a.txt", "beta", vec!["alpha"], false, Sameness::Different),
                (
                    "shared.txt",
                    "beta",
                    vec!["alpha"],
                    false,
                    Sameness::Identical
                ),
                (
                    "x.txt",
                    "gamma",
                    vec!["alpha", "beta"],
                    true,
                    Sameness::Different
                ),
            ]
        );
    }

    #[test]
    fn the_pairs_sum_the_conflicts() {
        let w = world();
        let report = analyze_overlays(&w.base, &layers(&w));
        let got: Vec<_> = report
            .pairs
            .iter()
            .map(|s| {
                (
                    s.winner.as_str(),
                    s.loser.as_str(),
                    s.paths,
                    s.identical,
                    s.different,
                    s.unknown,
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("beta", "alpha", 2, 1, 1, 0),
                ("gamma", "alpha", 1, 0, 1, 0),
                ("gamma", "beta", 1, 1, 0, 0),
            ]
        );
        assert_eq!(report.pairs[0].examples, [p("a.txt"), p("shared.txt")]);
    }

    #[test]
    fn only_three_examples_are_kept() {
        let base = mem(&[]);
        let a = mem(&[("1", "a"), ("2", "a"), ("3", "a"), ("4", "a")]);
        let b = mem(&[("1", "b"), ("2", "b"), ("3", "b"), ("4", "b")]);
        let report = analyze_overlays(
            &base,
            &[
                ModLayers {
                    id: "a".into(),
                    layers: vec![("a".into(), &a)],
                },
                ModLayers {
                    id: "b".into(),
                    layers: vec![("b".into(), &b)],
                },
            ],
        );
        assert_eq!(report.pairs[0].paths, 4);
        assert_eq!(report.pairs[0].examples, [p("1"), p("2"), p("3")]);
    }

    #[test]
    fn one_mod_or_none_has_no_conflicts() {
        let w = world();
        let one = analyze_overlays(
            &w.base,
            &[ModLayers {
                id: "alpha".into(),
                layers: vec![("a.gro".into(), &w.alpha_a), ("z.gro".into(), &w.alpha_z)],
            }],
        );
        assert!(one.conflicts.is_empty() && one.pairs.is_empty());
        assert_eq!(one.mods[0].inner_overrides, 1);
        assert_eq!(one.mods[0].lost_to_other_mods, 0);
        let none = analyze_overlays(&w.base, &[]);
        assert_eq!(none, OverlayReport::default());
    }

    #[test]
    fn a_directory_in_the_base_is_not_a_replaced_file() {
        // The mod has a *file* where the base has a folder `dir`: it is new, not a replacement.
        let base = mem(&[("dir/deep.txt", "d")]);
        let a = mem(&[("dir", "now a file")]);
        let report = analyze_overlays(
            &base,
            &[ModLayers {
                id: "a".into(),
                layers: vec![("a".into(), &a)],
            }],
        );
        assert_eq!(report.mods[0].replaces_base, 0);
        assert_eq!(report.mods[0].new_files, 1);
    }

    #[test]
    fn big_and_unreadable_copies_are_unknown_not_different() {
        let big = vec![7u8; (8 << 20) + 1];
        let mut a = MemoryVfs::new();
        a.insert("big.bin", big.clone()).unwrap();
        a.insert("small.txt", b"s".to_vec()).unwrap();
        a.insert("sized.bin", vec![1, 2]).unwrap();
        let mut b = MemoryVfs::new();
        b.insert("big.bin", big).unwrap();
        b.insert("small.txt", b"s".to_vec()).unwrap();
        b.insert("sized.bin", vec![1, 2, 3]).unwrap();
        let failing = Failing {
            inner: b,
            fail: p("small.txt"),
        };
        let report = analyze_overlays(
            &MemoryVfs::new(),
            &[
                ModLayers {
                    id: "a".into(),
                    layers: vec![("a".into(), &a)],
                },
                ModLayers {
                    id: "b".into(),
                    layers: vec![("b".into(), &failing)],
                },
            ],
        );
        let by: BTreeMap<_, _> = report
            .conflicts
            .iter()
            .map(|c| (c.path.as_str(), c.content))
            .collect();
        assert_eq!(by["big.bin"], Sameness::Unknown);
        assert_eq!(by["small.txt"], Sameness::Unknown);
        // Different sizes are different without reading anything.
        assert_eq!(by["sized.bin"], Sameness::Different);
        let pair = &report.pairs[0];
        assert_eq!((pair.identical, pair.different, pair.unknown), (0, 1, 2));
    }

    /// A file system that fails to read one file and to list one folder.
    struct Failing {
        inner: MemoryVfs,
        fail: VfsPath,
    }

    impl Vfs for Failing {
        fn stat(&self, path: &VfsPath) -> ss2_core::Result<crate::Stat> {
            self.inner.stat(path)
        }
        fn open(&self, path: &VfsPath) -> ss2_core::Result<Box<dyn crate::ReadSeek>> {
            if *path == self.fail {
                return Err(Error::io(path.as_str(), std::io::Error::other("boom")));
            }
            self.inner.open(path)
        }
        fn list(&self, dir: &VfsPath) -> ss2_core::Result<Vec<crate::DirEntry>> {
            if dir.as_str() == "broken" {
                return Err(Error::io(dir.as_str(), std::io::Error::other("no list")));
            }
            self.inner.list(dir)
        }
    }

    #[test]
    fn a_folder_that_cannot_be_listed_is_reported() {
        let mut inner = MemoryVfs::new();
        inner.insert("broken/x.txt", b"x".to_vec()).unwrap();
        inner.insert("fine.txt", b"f".to_vec()).unwrap();
        let layer = Failing {
            inner,
            fail: p("nothing"),
        };
        let report = analyze_overlays(
            &MemoryVfs::new(),
            &[ModLayers {
                id: "a".into(),
                layers: vec![("a.gro".into(), &layer)],
            }],
        );
        assert_eq!(report.mods[0].files, 1);
        assert_eq!(report.problems.len(), 1);
        assert!(
            report.problems[0].contains("a.gro"),
            "{:?}",
            report.problems
        );
        assert!(
            report.problems[0].contains("broken"),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn the_report_does_not_depend_on_how_the_layers_were_inserted() {
        let w = world();
        let a = analyze_overlays(&w.base, &layers(&w));
        // Same files, inserted in the opposite order into the memory file systems.
        let rev = |files: &[(&str, &str)]| {
            let mut r = files.to_vec();
            r.reverse();
            mem(&r)
        };
        let w2 = World {
            base: rev(&[("x.txt", "base"), ("b.txt", "b"), ("dir/deep.txt", "d")]),
            alpha_a: rev(&[
                ("x.txt", "alpha"),
                ("a.txt", "1"),
                ("shared.txt", "same"),
                ("only-alpha.txt", "o"),
            ]),
            alpha_z: mem(&[("a.txt", "2")]),
            beta: rev(&[
                ("X.TXT", "beta"),
                ("a.txt", "1"),
                ("shared.txt", "same"),
                ("only-beta.txt", "o"),
            ]),
            gamma: mem(&[("x.txt", "beta")]),
        };
        assert_eq!(a, analyze_overlays(&w2.base, &layers(&w2)));
    }

    #[test]
    fn the_rendering_is_plain_ascii_and_complete() {
        let w = world();
        let text = analyze_overlays(&w.base, &layers(&w)).render();
        let expected = "\
Mods in load order (a later mod wins):
  1. alpha: 4 files; base game files replaced: 1; new: 3; replaced again inside the mod: 1
     overridden by later mods: 3; overriding earlier mods: 0
  2. beta: 4 files; base game files replaced: 1; new: 3; replaced again inside the mod: 0
     overridden by later mods: 1; overriding earlier mods: 2
  3. gamma: 1 file; base game files replaced: 1; new: 0; replaced again inside the mod: 0
     overridden by later mods: 0; overriding earlier mods: 1
Files that more than one mod provides: 3
  beta wins over alpha: 2 files (identical: 1, different: 1, not compared: 0): a.txt, shared.txt
  gamma wins over alpha: 1 file (identical: 0, different: 1, not compared: 0): x.txt
  gamma wins over beta: 1 file (identical: 1, different: 0, not compared: 0): x.txt
";
        assert_eq!(text, expected);
        assert!(text.is_ascii());
        let empty = OverlayReport::default().render();
        assert_eq!(empty, "No mods are loaded: only the base game.\n");
    }

    #[test]
    fn hostile_names_cannot_garble_the_report() {
        let a = mem(&[("caf\u{e9}.txt", "a")]);
        let b = mem(&[("caf\u{e9}.txt", "b")]);
        let text = analyze_overlays(
            &MemoryVfs::new(),
            &[
                ModLayers {
                    id: "a\u{1b}[31m".into(),
                    layers: vec![("a".into(), &a)],
                },
                ModLayers {
                    id: "b".into(),
                    layers: vec![("b".into(), &b)],
                },
            ],
        )
        .render();
        assert!(text.is_ascii(), "{text:?}");
        assert!(!text.contains('\u{1b}'));
        assert!(text.contains("\\u{e9}"), "{text}");
    }

    #[test]
    fn a_report_for_an_install_matches_what_the_mounted_stack_serves() {
        let dir = tempfile::tempdir().unwrap();
        fake_install(dir.path());
        let found = scan_installed_mods(dir.path()).unwrap();
        let plan = found.plan(&["beta".to_string()]).unwrap();
        let report = overlay_report(dir.path(), &found, &plan).unwrap();
        assert_eq!(
            report
                .mods
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta"]
        );
        let alpha = &report.mods[0];
        assert_eq!(
            (alpha.files, alpha.replaces_base, alpha.new_files),
            (3, 1, 2)
        );
        assert_eq!(alpha.inner_overrides, 1);
        let beta = &report.mods[1];
        // Beta's own descriptor (`.mod`, `.nfo`) is two of its five files.
        assert_eq!((beta.files, beta.replaces_base, beta.new_files), (5, 1, 4));
        let conflicts: Vec<_> = report
            .conflicts
            .iter()
            .map(|c| (c.path.as_str(), c.winner.as_str(), c.content))
            .collect();
        assert_eq!(
            conflicts,
            [
                ("content/shared.txt", "beta", Sameness::Identical),
                ("content/x.txt", "beta", Sameness::Different),
            ]
        );
        // The winner of every conflict is the layer the mounted stack really serves.
        let mounted = mount_mods(dir.path(), &found, &plan).unwrap();
        for c in &report.conflicts {
            let source = mounted.vfs.source_of(&c.path).unwrap();
            assert!(
                source.starts_with(&format!("{}/", c.winner)),
                "{} {source}",
                c.path
            );
        }
        // A mod that is not installed is refused.
        let ghost = crate::resolve_load_order(
            &[crate::ModNode::new("ghost", [] as [&str; 0])],
            &["ghost".into()],
        )
        .unwrap();
        assert!(overlay_report(dir.path(), &found, &ghost).is_err());
    }

    /// Data-gated: the retail install's two mods. Skips without `SS2_DATA_DIR`.
    #[test]
    fn the_real_mods_overlay_the_base_as_the_spec_measured() {
        let dir = ss2_testkit::require_game_data!();
        let found = scan_installed_mods(&dir).unwrap();
        let (Some(_), Some(_)) = (found.get("sam2renovation"), found.get("insamnity2")) else {
            eprintln!("the install has no mods: nothing to check");
            return;
        };
        let plan = found
            .plan(&["sam2renovation".to_string(), "insamnity2".to_string()])
            .unwrap();
        let report = overlay_report(&dir, &found, &plan).unwrap();
        eprintln!("{}", report.render());
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let by = |id: &str| report.mods.iter().find(|m| m.id == id).unwrap();
        // docs/re/mods.md: 3,194 entries, 1,963 of them replace base entries.
        assert_eq!(by("sam2renovation").files, 3194);
        assert_eq!(by("sam2renovation").replaces_base, 1963);
        assert_eq!(by("sam2renovation").new_files, 1231);
        // docs/MOD_VALIDATION.md: 3,169 files, 399 replace base files.
        assert_eq!(by("insamnity2").files, 3169);
        assert_eq!(by("insamnity2").replaces_base, 399);
        // InSamnity! 2 is loaded last here, so it wins every shared file.
        assert!(report.conflicts.iter().all(|c| c.winner == "insamnity2"));
        let pair = &report.pairs[0];
        assert_eq!(
            (pair.winner.as_str(), pair.loser.as_str()),
            ("insamnity2", "sam2renovation")
        );
        assert_eq!(pair.paths, pair.identical + pair.different + pair.unknown);
        assert_eq!(pair.different + pair.unknown, 168);
    }
}
