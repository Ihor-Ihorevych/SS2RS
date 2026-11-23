//! **File change detection** for hot reload (roadmap T04-033 and the loose-file reload of T02-011b, `docs/HOT_RELOAD.md`): decides *when a file has really changed*, without
//! touching the file system or a clock.
//!
//! The caller lists the watched files and their stamps (modification time and size) every poll; [`FileWatcher::poll`] returns what was **added, changed or removed**, once,
//! in path order. A change is reported only after the stamp has **stayed the same for a number of polls** (`settle`), so a file that is still being written, or
//! an editor that saves by deleting and recreating, produces one event for the final state and not a burst of half-written ones.

use std::collections::BTreeMap;

/// What a file looked like: any pair that changes when the file does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileStamp {
    /// A modification time in any unit that increases when the file is written (nanoseconds since the epoch is typical).
    pub modified: u64,
    /// The size in bytes.
    pub size: u64,
}

/// What happened to a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// The file is new.
    Added,
    /// The file is different.
    Changed,
    /// The file is gone.
    Removed,
}

/// A change to report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    /// The path as given to [`FileWatcher::poll`].
    pub path: String,
    /// What happened.
    pub kind: ChangeKind,
}

#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    /// What was last reported to the caller (`None`: never reported or reported removed).
    reported: Option<FileStamp>,
    /// What the last poll saw.
    seen: Option<FileStamp>,
    /// How many polls in a row have seen exactly that.
    stable: u32,
}

/// Detects settled changes among watched files.
///
/// ```
/// use ss2_core::hotreload::{ChangeKind, FileStamp, FileWatcher};
///
/// let mut w = FileWatcher::new(1); // a stamp must be seen twice in a row
/// let a = FileStamp { modified: 10, size: 100 };
/// assert!(w.poll(&[("shaders/sky.wgsl".into(), a)]).is_empty());       // seen once: maybe still being written
/// let events = w.poll(&[("shaders/sky.wgsl".into(), a)]);              // seen twice: settled
/// assert_eq!(events[0].kind, ChangeKind::Added);
/// assert!(w.poll(&[("shaders/sky.wgsl".into(), a)]).is_empty());       // nothing new
/// ```
#[derive(Clone, Debug)]
pub struct FileWatcher {
    entries: BTreeMap<String, Entry>,
    settle: u32,
}

impl FileWatcher {
    /// A watcher that reports a change once the same stamp has been seen on `settle` further polls (0 reports at once, 1 needs two polls in a row, and so on).
    pub fn new(settle: u32) -> Self {
        Self {
            entries: BTreeMap::new(),
            settle,
        }
    }

    /// Take `current` as the starting point: the files that exist now are known and **not** reported.
    pub fn baseline(&mut self, current: &[(String, FileStamp)]) {
        self.entries.clear();
        for (path, stamp) in current {
            self.entries.insert(
                path.clone(),
                Entry {
                    reported: Some(*stamp),
                    seen: Some(*stamp),
                    stable: 0,
                },
            );
        }
    }

    /// How many files are known (reported present).
    pub fn known(&self) -> usize {
        self.entries
            .values()
            .filter(|e| e.reported.is_some())
            .count()
    }

    /// Compare `current` (every watched file that exists now, with its stamp) with what was seen before and return the changes that have settled, sorted by path.
    /// If a path is listed twice the last stamp wins.
    pub fn poll(&mut self, current: &[(String, FileStamp)]) -> Vec<FileChange> {
        let now: BTreeMap<&str, FileStamp> =
            current.iter().map(|(p, s)| (p.as_str(), *s)).collect();
        for path in now.keys() {
            self.entries.entry((*path).to_owned()).or_default();
        }
        let mut changes = Vec::new();
        let mut gone = Vec::new();
        for (path, entry) in &mut self.entries {
            let cur = now.get(path.as_str()).copied();
            if cur == entry.seen {
                entry.stable = entry.stable.saturating_add(1);
            } else {
                entry.seen = cur;
                entry.stable = 0;
            }
            if entry.seen != entry.reported && entry.stable >= self.settle {
                let kind = match (entry.reported, entry.seen) {
                    (None, Some(_)) => ChangeKind::Added,
                    (Some(_), Some(_)) => ChangeKind::Changed,
                    (Some(_), None) => ChangeKind::Removed,
                    (None, None) => unreachable!("unequal options cannot both be None"),
                };
                changes.push(FileChange {
                    path: path.clone(),
                    kind,
                });
                entry.reported = entry.seen;
            }
            if entry.reported.is_none() && entry.seen.is_none() {
                gone.push(path.clone());
            }
        }
        for path in gone {
            self.entries.remove(&path);
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(m: u64) -> FileStamp {
        FileStamp {
            modified: m,
            size: m * 10,
        }
    }

    fn files(list: &[(&str, u64)]) -> Vec<(String, FileStamp)> {
        list.iter()
            .map(|(p, m)| ((*p).to_owned(), stamp(*m)))
            .collect()
    }

    fn kinds(changes: &[FileChange]) -> Vec<(&str, ChangeKind)> {
        changes.iter().map(|c| (c.path.as_str(), c.kind)).collect()
    }

    // ---- the normal life of a file --------------------------------------------------------------------

    #[test]
    fn a_file_is_added_changed_and_removed_each_once_after_it_settles() {
        let mut w = FileWatcher::new(1);
        let v1 = files(&[("a.txt", 1)]);
        assert!(w.poll(&v1).is_empty(), "seen once");
        assert_eq!(w.known(), 0, "not reported yet, so not known");
        assert_eq!(kinds(&w.poll(&v1)), [("a.txt", ChangeKind::Added)]);
        assert_eq!(w.known(), 1);
        assert!(
            w.poll(&v1).is_empty() && w.poll(&v1).is_empty(),
            "nothing new"
        );
        let v2 = files(&[("a.txt", 2)]);
        assert!(w.poll(&v2).is_empty());
        assert_eq!(kinds(&w.poll(&v2)), [("a.txt", ChangeKind::Changed)]);
        assert!(w.poll(&[]).is_empty());
        assert_eq!(
            w.known(),
            1,
            "gone but not yet reported removed: still known"
        );
        assert_eq!(kinds(&w.poll(&[])), [("a.txt", ChangeKind::Removed)]);
        assert!(w.poll(&[]).is_empty());
        assert_eq!(w.known(), 0);
    }

    #[test]
    fn a_stamp_that_keeps_changing_is_not_reported_until_it_stops() {
        let mut w = FileWatcher::new(2);
        for m in 1..=10 {
            assert!(
                w.poll(&files(&[("big.pak", m)])).is_empty(),
                "still being written at {m}"
            );
        }
        let last = files(&[("big.pak", 10)]);
        assert!(w.poll(&last).is_empty(), "seen twice");
        assert_eq!(
            kinds(&w.poll(&last)),
            [("big.pak", ChangeKind::Added)],
            "seen three times with settle 2"
        );
    }

    #[test]
    fn size_alone_is_enough_to_see_a_change() {
        let mut w = FileWatcher::new(0);
        let a = vec![(
            "f".to_owned(),
            FileStamp {
                modified: 5,
                size: 1,
            },
        )];
        let b = vec![(
            "f".to_owned(),
            FileStamp {
                modified: 5,
                size: 2,
            },
        )];
        assert_eq!(kinds(&w.poll(&a)), [("f", ChangeKind::Added)]);
        assert_eq!(kinds(&w.poll(&b)), [("f", ChangeKind::Changed)]);
    }

    #[test]
    fn an_editor_that_saves_by_deleting_and_recreating_gives_one_changed_event() {
        let mut w = FileWatcher::new(1);
        w.baseline(&files(&[("settings.toml", 1)]));
        assert!(w.poll(&[]).is_empty(), "gone for one poll");
        assert!(
            w.poll(&files(&[("settings.toml", 2)])).is_empty(),
            "back with a new stamp, not settled"
        );
        assert_eq!(
            kinds(&w.poll(&files(&[("settings.toml", 2)]))),
            [("settings.toml", ChangeKind::Changed)],
            "one Changed, no Removed"
        );
    }

    #[test]
    fn a_file_that_vanishes_and_returns_identical_within_the_settle_time_is_not_an_event() {
        let mut w = FileWatcher::new(2);
        w.baseline(&files(&[("a", 1)]));
        assert!(w.poll(&[]).is_empty());
        assert!(w.poll(&files(&[("a", 1)])).is_empty());
        for _ in 0..5 {
            assert!(w.poll(&files(&[("a", 1)])).is_empty());
        }
        assert_eq!(w.known(), 1);
    }

    #[test]
    fn a_file_that_appears_and_vanishes_before_settling_leaves_no_trace() {
        let mut w = FileWatcher::new(2);
        assert!(w.poll(&files(&[("tmp", 1)])).is_empty());
        assert!(w.poll(&[]).is_empty());
        for _ in 0..6 {
            assert!(w.poll(&[]).is_empty());
        }
        assert_eq!(w.known(), 0);
        assert!(w.entries.is_empty(), "the bookkeeping for it is gone too");
    }

    // ---- baseline, settle, ordering -------------------------------------------------------------------------

    #[test]
    fn a_baseline_makes_existing_files_known_without_events() {
        let mut w = FileWatcher::new(3);
        w.baseline(&files(&[("a", 1), ("b", 2)]));
        assert_eq!(w.known(), 2);
        assert!(w.poll(&files(&[("a", 1), ("b", 2)])).is_empty());
        // A change to a baseline file still has to settle.
        let changed = files(&[("a", 9), ("b", 2)]);
        for _ in 0..3 {
            assert!(w.poll(&changed).is_empty());
        }
        assert_eq!(kinds(&w.poll(&changed)), [("a", ChangeKind::Changed)]);
        // A second baseline replaces the first.
        w.baseline(&files(&[("z", 1)]));
        assert_eq!(w.known(), 1);
    }

    #[test]
    fn a_settle_of_zero_reports_immediately() {
        let mut w = FileWatcher::new(0);
        assert_eq!(
            kinds(&w.poll(&files(&[("a", 1)]))),
            [("a", ChangeKind::Added)]
        );
        assert_eq!(
            kinds(&w.poll(&files(&[("a", 2)]))),
            [("a", ChangeKind::Changed)]
        );
        assert_eq!(kinds(&w.poll(&[])), [("a", ChangeKind::Removed)]);
    }

    #[test]
    fn events_come_in_path_order_and_duplicates_in_the_list_use_the_last_stamp() {
        let mut w = FileWatcher::new(0);
        let events = w.poll(&files(&[("zeta", 1), ("alpha", 1), ("mid", 1)]));
        assert_eq!(
            events.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
            ["alpha", "mid", "zeta"]
        );
        let mut dup = files(&[("a", 1), ("a", 7)]);
        let mut w2 = FileWatcher::new(0);
        w2.poll(&dup);
        dup.truncate(1); // now the list says stamp 1 only
        assert_eq!(
            kinds(&w2.poll(&dup)),
            [("a", ChangeKind::Changed)],
            "the earlier poll had settled on 7"
        );
    }

    #[test]
    fn several_files_move_independently() {
        let mut w = FileWatcher::new(1);
        let both = files(&[("a", 1), ("b", 1)]);
        w.poll(&both);
        assert_eq!(
            kinds(&w.poll(&both)),
            [("a", ChangeKind::Added), ("b", ChangeKind::Added)]
        );
        // a changes now, b a poll later: each is reported when its own stamp has settled.
        assert!(w.poll(&files(&[("a", 2), ("b", 1)])).is_empty());
        assert_eq!(
            kinds(&w.poll(&files(&[("a", 2), ("b", 5)]))),
            [("a", ChangeKind::Changed)]
        );
        assert_eq!(
            kinds(&w.poll(&files(&[("a", 2), ("b", 5)]))),
            [("b", ChangeKind::Changed)]
        );
    }

    // ---- a model-checked property ------------------------------------------------------------------------------------

    #[test]
    fn replaying_the_events_rebuilds_the_final_state_whatever_the_flapping_before_it() {
        let mut seed = 0x1357_9BDF_2468_ACE0u64;
        let mut rnd = move |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        for settle in 0..4u32 {
            let mut w = FileWatcher::new(settle);
            let mut model: BTreeMap<String, FileStamp> = BTreeMap::new();
            let names = ["a", "b", "c", "d", "e"];
            for _ in 0..300 {
                // A random snapshot: each file present with a small random stamp, or absent.
                let mut snap: Vec<(String, FileStamp)> = Vec::new();
                for n in names {
                    if rnd(3) != 0 {
                        snap.push((n.to_owned(), stamp(1 + rnd(3))));
                    }
                }
                apply(&mut model, &w.poll(&snap), &snap, false);
            }
            // Now hold one final snapshot long enough to settle everything.
            let mut fin: Vec<(String, FileStamp)> = Vec::new();
            for n in names {
                if rnd(2) == 0 {
                    fin.push((n.to_owned(), stamp(1 + rnd(3))));
                }
            }
            for _ in 0..=settle + 1 {
                apply(&mut model, &w.poll(&fin), &fin, true);
            }
            let want: BTreeMap<String, FileStamp> = fin.iter().cloned().collect();
            assert_eq!(
                model, want,
                "settle {settle}: the events add up to the final snapshot"
            );
            assert_eq!(w.known(), want.len());
            // Nothing further is reported for an unchanged snapshot.
            assert!(w.poll(&fin).is_empty());
        }
    }

    /// Apply events to the model the way a caller would (re-reading the file's stamp from the snapshot when told Added or Changed), checking each event makes sense.
    fn apply(
        model: &mut BTreeMap<String, FileStamp>,
        events: &[FileChange],
        snap: &[(String, FileStamp)],
        strict: bool,
    ) {
        for e in events {
            match e.kind {
                ChangeKind::Added => {
                    assert!(!model.contains_key(&e.path), "{} added twice", e.path);
                    let s = snap
                        .iter()
                        .rev()
                        .find(|(p, _)| *p == e.path)
                        .map(|(_, s)| *s)
                        .expect("added files are present");
                    model.insert(e.path.clone(), s);
                }
                ChangeKind::Changed => {
                    assert!(
                        model.contains_key(&e.path),
                        "{} changed before it was added",
                        e.path
                    );
                    let s = snap
                        .iter()
                        .rev()
                        .find(|(p, _)| *p == e.path)
                        .map(|(_, s)| *s)
                        .expect("changed files are present");
                    if strict {
                        assert_ne!(
                            model[&e.path], s,
                            "{} reported changed to what it already was",
                            e.path
                        );
                    }
                    model.insert(e.path.clone(), s);
                }
                ChangeKind::Removed => {
                    assert!(
                        model.remove(&e.path).is_some(),
                        "{} removed but never added",
                        e.path
                    );
                    assert!(
                        snap.iter().all(|(p, _)| *p != e.path),
                        "{} removed while present",
                        e.path
                    );
                }
            }
        }
    }
}
