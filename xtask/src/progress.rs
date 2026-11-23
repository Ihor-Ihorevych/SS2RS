//! `cargo xtask progress`: how far the roadmap is (roadmap T11-012, `docs/PROGRESS.md`).
//!
//! Reads the task lines of `docs/roadmap/T*.md` (the same parser as `roadmap-sync`) and counts, per track and overall, the tasks that are **done**, **claimed** and **open**,
//! separately for the work scheduled now and for `PHASE 2` (Android and the rest, not before milestone M8). It also counts the **follow-up tasks** (`T04-016b` and so on:
//! the part of a task that was left for later when it was ticked with a narrower scope), because a roadmap that looks 80 % done while its follow-ups pile up
//! is not what it seems.
//!
//! The output is plain Markdown (default) or a small JSON document (`--json`) for other tools. There is no clock and no network here, and the numbers come from the files in
//! the working tree, so the same tree always gives the same report.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::roadmap::RoadmapTask;

/// Counts for a set of tasks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Ticked `[x]`.
    pub done: usize,
    /// Claimed `[~]` (and not ticked).
    pub claimed: usize,
    /// Not started `[ ]`.
    pub open: usize,
}

impl Counts {
    /// All tasks counted.
    pub fn total(&self) -> usize {
        self.done + self.claimed + self.open
    }

    /// The share done, as a whole percentage rounded down (0 for no tasks).
    pub fn percent_done(&self) -> usize {
        if self.total() == 0 {
            0
        } else {
            self.done * 100 / self.total()
        }
    }

    fn add(&mut self, t: &RoadmapTask) {
        if t.done {
            self.done += 1;
        } else if t.in_progress {
            self.claimed += 1;
        } else {
            self.open += 1;
        }
    }
}

/// One track (`T04`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackProgress {
    /// The track prefix, such as `T04`.
    pub track: String,
    /// Tasks scheduled now (not `PHASE 2`).
    pub now: Counts,
    /// `PHASE 2` tasks.
    pub phase2: Counts,
    /// Follow-up tasks (a letter after the number) among the tasks scheduled now, that are not done.
    pub followups_open: usize,
}

/// The whole roadmap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Per track, in track order.
    pub tracks: Vec<TrackProgress>,
}

/// Whether an ID is a follow-up (`T04-016b`: a letter after the three digits).
pub fn is_followup(id: &str) -> bool {
    id.chars().last().is_some_and(|c| c.is_ascii_lowercase())
}

/// The track prefix of an ID (`T04-016b` gives `T04`).
pub fn track_of(id: &str) -> &str {
    id.split('-').next().unwrap_or(id)
}

/// Count the tasks.
pub fn compute(tasks: &[RoadmapTask]) -> Progress {
    let mut by_track: BTreeMap<String, TrackProgress> = BTreeMap::new();
    for t in tasks {
        let track = track_of(&t.id).to_owned();
        let entry = by_track
            .entry(track.clone())
            .or_insert_with(|| TrackProgress {
                track,
                now: Counts::default(),
                phase2: Counts::default(),
                followups_open: 0,
            });
        if t.phase2 {
            entry.phase2.add(t);
        } else {
            entry.now.add(t);
            if is_followup(&t.id) && !t.done {
                entry.followups_open += 1;
            }
        }
    }
    Progress {
        tracks: by_track.into_values().collect(),
    }
}

impl Progress {
    /// The sum over all tracks of the work scheduled now.
    pub fn now(&self) -> Counts {
        self.tracks.iter().fold(Counts::default(), |a, t| Counts {
            done: a.done + t.now.done,
            claimed: a.claimed + t.now.claimed,
            open: a.open + t.now.open,
        })
    }

    /// The sum over all tracks of `PHASE 2`.
    pub fn phase2(&self) -> Counts {
        self.tracks.iter().fold(Counts::default(), |a, t| Counts {
            done: a.done + t.phase2.done,
            claimed: a.claimed + t.phase2.claimed,
            open: a.open + t.phase2.open,
        })
    }

    /// Open follow-up tasks among the work scheduled now.
    pub fn followups_open(&self) -> usize {
        self.tracks.iter().map(|t| t.followups_open).sum()
    }
}

fn bar(percent: usize) -> String {
    let filled = (percent.min(100) + 2) / 5; // 20 cells, 5 % each, rounded
    format!("{}{}", "#".repeat(filled), ".".repeat(20 - filled))
}

/// The report as Markdown.
pub fn render_markdown(p: &Progress) -> String {
    let mut s = String::new();
    let now = p.now();
    let phase2 = p.phase2();
    let _ = writeln!(s, "# Roadmap progress\n");
    let _ = writeln!(
        s,
        "Scheduled now: **{} of {}** tasks done ({} %), {} claimed, {} open, of which **{} are follow-ups** left by tasks that were ticked with a narrower scope. `PHASE 2`: {} of {} done.\n",
        now.done,
        now.total(),
        now.percent_done(),
        now.claimed,
        now.open,
        p.followups_open(),
        phase2.done,
        phase2.total()
    );
    let _ = writeln!(
        s,
        "| Track | Done | Claimed | Open | Follow-ups open | Progress | `PHASE 2` done |"
    );
    let _ = writeln!(s, "|---|---:|---:|---:|---:|---|---:|");
    for t in &p.tracks {
        let _ = writeln!(
            s,
            "| {} | {} / {} | {} | {} | {} | `{}` {} % | {} / {} |",
            t.track,
            t.now.done,
            t.now.total(),
            t.now.claimed,
            t.now.open,
            t.followups_open,
            bar(t.now.percent_done()),
            t.now.percent_done(),
            t.phase2.done,
            t.phase2.total()
        );
    }
    let _ = writeln!(
        s,
        "\nA task ticked with a narrower scope counts as done here; its left-over part is a follow-up task (an ID with a letter, like `T04-016b`) and is counted as open. This page counts roadmap lines, not the size of the work: a `S` task and an `XL` task weigh the same."
    );
    s
}

/// The report as JSON (no dependencies: only numbers and track names, which never need escaping).
pub fn render_json(p: &Progress) -> String {
    let counts = |c: &Counts| {
        format!(
            "{{\"done\":{},\"claimed\":{},\"open\":{},\"total\":{},\"percent_done\":{}}}",
            c.done,
            c.claimed,
            c.open,
            c.total(),
            c.percent_done()
        )
    };
    let tracks: Vec<String> = p
        .tracks
        .iter()
        .map(|t| {
            format!(
                "{{\"track\":\"{}\",\"now\":{},\"phase2\":{},\"followups_open\":{}}}",
                t.track,
                counts(&t.now),
                counts(&t.phase2),
                t.followups_open
            )
        })
        .collect();
    format!(
        "{{\"now\":{},\"phase2\":{},\"followups_open\":{},\"tracks\":[{}]}}\n",
        counts(&p.now()),
        counts(&p.phase2()),
        p.followups_open(),
        tracks.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roadmap::parse_roadmap;

    fn tasks(text: &str) -> Vec<RoadmapTask> {
        parse_roadmap(text, "T04-test.md")
    }

    const SAMPLE: &str = "\
- [x] **T04-001** · done thing · `M` · `P0`
- [x] **T04-002** · another done thing · `L` · `P0`
- [~] **T04-003** [INPRG @someone · branch `x` · 2026-11-23] · claimed · `M` · `P0`
- [ ] **T04-004** · open · `S` · `P1`
- [ ] **T04-002b** · a follow-up left open · `S` · `P1`
- [x] **T04-001b** · a follow-up done · `S` · `P1`
- [ ] **T04-005** · phase two thing · `M` · `PHASE 2`
- [x] **T04-006** · phase two done · `S` · `PHASE 2`
- [ ] **T05-001** · other track · `M` · `P0`
";

    // ---- counting ---------------------------------------------------------------------------------------------

    #[test]
    fn tasks_are_counted_per_track_split_by_phase() {
        let p = compute(&tasks(SAMPLE));
        assert_eq!(
            p.tracks
                .iter()
                .map(|t| t.track.as_str())
                .collect::<Vec<_>>(),
            ["T04", "T05"]
        );
        let t04 = &p.tracks[0];
        assert_eq!(
            t04.now,
            Counts {
                done: 3,
                claimed: 1,
                open: 2
            },
            "done: T04-001, 002, 001b; claimed: 003; open: 004, 002b"
        );
        assert_eq!(
            t04.phase2,
            Counts {
                done: 1,
                claimed: 0,
                open: 1
            }
        );
        assert_eq!(
            p.tracks[1].now,
            Counts {
                done: 0,
                claimed: 0,
                open: 1
            }
        );
        assert_eq!(p.tracks[1].phase2, Counts::default());
    }

    #[test]
    fn follow_ups_are_counted_only_while_they_are_open_and_scheduled_now() {
        let p = compute(&tasks(SAMPLE));
        assert_eq!(
            p.tracks[0].followups_open, 1,
            "T04-002b is open; T04-001b is done"
        );
        assert_eq!(p.followups_open(), 1);
        // A phase-2 follow-up does not count against the work scheduled now.
        let q = compute(&tasks("- [ ] **T04-010b** · later · `S` · `PHASE 2`\n"));
        assert_eq!(q.followups_open(), 0);
        assert_eq!(q.phase2().open, 1);
        // A claimed follow-up is still "not done".
        let c = compute(&tasks(
            "- [~] **T04-011b** [INPRG @a · branch `b` · 2026-11-23] · x · `S` · `P1`\n",
        ));
        assert_eq!(c.followups_open(), 1);
    }

    #[test]
    fn totals_add_up_across_tracks() {
        let p = compute(&tasks(SAMPLE));
        assert_eq!(
            p.now(),
            Counts {
                done: 3,
                claimed: 1,
                open: 3
            }
        );
        assert_eq!(
            p.phase2(),
            Counts {
                done: 1,
                claimed: 0,
                open: 1
            }
        );
        assert_eq!(p.now().total(), 7);
        assert_eq!(p.now().percent_done(), 42, "3 of 7, rounded down");
        assert_eq!(
            Counts::default().percent_done(),
            0,
            "no tasks, no division by zero"
        );
        assert_eq!(
            Counts {
                done: 1,
                claimed: 0,
                open: 0
            }
            .percent_done(),
            100
        );
    }

    #[test]
    fn open_follow_ups_add_up_across_tracks() {
        let p = compute(&tasks(
            "- [ ] **T04-001b** · a · `S` · `P1`
- [ ] **T04-002b** · b · `S` · `P1`
- [ ] **T05-001b** · c · `S` · `P1`
- [x] **T05-002b** · d · `S` · `P1`
",
        ));
        assert_eq!(p.followups_open(), 3, "two in T04 and one in T05");
        assert_eq!(
            p.tracks
                .iter()
                .map(|t| t.followups_open)
                .collect::<Vec<_>>(),
            [2, 1]
        );
    }

    #[test]
    fn ids_and_tracks_are_read_correctly() {
        assert!(is_followup("T04-016b") && is_followup("T10-001z"));
        assert!(!is_followup("T04-016") && !is_followup(""));
        assert_eq!(track_of("T04-016b"), "T04");
        assert_eq!(track_of("T13-001"), "T13");
        assert_eq!(track_of("weird"), "weird");
    }

    #[test]
    fn an_empty_roadmap_is_all_zeroes() {
        let p = compute(&[]);
        assert!(p.tracks.is_empty());
        assert_eq!(
            (p.now(), p.phase2(), p.followups_open()),
            (Counts::default(), Counts::default(), 0)
        );
        let md = render_markdown(&p);
        assert!(md.contains("**0 of 0** tasks done (0 %)"), "{md}");
        assert_eq!(
            render_json(&p),
            "{\"now\":{\"done\":0,\"claimed\":0,\"open\":0,\"total\":0,\"percent_done\":0},\"phase2\":{\"done\":0,\"claimed\":0,\"open\":0,\"total\":0,\"percent_done\":0},\"followups_open\":0,\"tracks\":[]}\n"
        );
    }

    // ---- output -------------------------------------------------------------------------------------------------

    #[test]
    fn the_markdown_has_a_summary_and_one_row_per_track() {
        let md = render_markdown(&compute(&tasks(SAMPLE)));
        assert!(md.starts_with("# Roadmap progress\n"));
        assert!(
            md.contains(
                "**3 of 7** tasks done (42 %), 1 claimed, 3 open, of which **1 are follow-ups**"
            ),
            "{md}"
        );
        assert!(md.contains("`PHASE 2`: 1 of 2 done."), "{md}");
        assert!(
            md.contains("| T04 | 3 / 6 | 1 | 2 | 1 | `##########..........` 50 % | 1 / 2 |"),
            "{md}"
        );
        assert!(
            md.contains("| T05 | 0 / 1 | 0 | 1 | 0 | `....................` 0 % | 0 / 0 |"),
            "{md}"
        );
        assert!(
            md.contains("narrower scope"),
            "the caveat about follow-ups is in the page"
        );
    }

    #[test]
    fn the_bar_is_twenty_cells_and_rounds_to_five_percent_steps() {
        assert_eq!(bar(0), "....................");
        assert_eq!(bar(100), "####################");
        assert_eq!(bar(50), "##########..........");
        assert_eq!(bar(2), "....................", "2 % rounds down to no cell");
        assert_eq!(bar(3), "#...................", "3 % rounds up to one");
        assert_eq!(bar(97), "###################.", "19.4 cells rounds to 19");
        assert_eq!(bar(98), "####################", "19.6 rounds to 20");
        assert_eq!(bar(250), "####################", "never overflows");
        assert!(
            [0, 1, 33, 66, 99, 100]
                .iter()
                .all(|p| bar(*p).chars().count() == 20)
        );
    }

    #[test]
    fn the_json_is_one_object_with_every_number() {
        let json = render_json(&compute(&tasks(SAMPLE)));
        assert!(
            json.starts_with(
                "{\"now\":{\"done\":3,\"claimed\":1,\"open\":3,\"total\":7,\"percent_done\":42}"
            ),
            "{json}"
        );
        assert!(json.contains("\"followups_open\":1,\"tracks\":["), "{json}");
        assert!(json.contains("{\"track\":\"T04\",\"now\":{\"done\":3,\"claimed\":1,\"open\":2,\"total\":6,\"percent_done\":50},\"phase2\":{\"done\":1,\"claimed\":0,\"open\":1,\"total\":2,\"percent_done\":50},\"followups_open\":1}"), "{json}");
        assert!(json.ends_with("]}\n"));
        // Braces and brackets balance: a cheap check that the text is a well-formed object.
        let (mut depth, mut max) = (0i32, 0i32);
        for c in json.chars() {
            match c {
                '{' | '[' => {
                    depth += 1;
                    max = max.max(depth);
                }
                '}' | ']' => depth -= 1,
                _ => {}
            }
            assert!(depth >= 0);
        }
        assert_eq!((depth, max > 2), (0, true));
    }

    // ---- the real roadmap -------------------------------------------------------------------------------------------

    #[test]
    fn the_real_roadmap_counts_every_task_once() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("docs")
            .join("roadmap");
        let tasks = crate::roadmap::load_all(&root).unwrap();
        assert!(tasks.len() > 300, "{} tasks", tasks.len());
        let p = compute(&tasks);
        assert_eq!(
            p.now().total() + p.phase2().total(),
            tasks.len(),
            "every task is in exactly one bucket"
        );
        let ids: std::collections::BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(
            ids.len(),
            tasks.len(),
            "no task ID is repeated in the roadmap"
        );
        assert!(
            p.tracks.len() >= 10,
            "tracks T00 to T13 are all there: {}",
            p.tracks.len()
        );
        assert!(p.now().done > 100, "a lot has been done: {}", p.now().done);
    }
}
