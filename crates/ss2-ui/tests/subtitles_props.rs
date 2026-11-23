//! Property tests for the subtitle timing model (roadmap T06-012): random cues and random time against the invariants of `ss2_ui::subtitles`
//! (`docs/SUBTITLES.md`). Times are multiples of 1/64 so that adding them is exact.

use proptest::prelude::*;
use ss2_ui::subtitles::{
    Cue, CueId, DropReason, EventKind, SubtitleConfig, SubtitleEvent, Subtitles,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
enum Op {
    Add {
        start: u32,
        duration: u32,
        words: usize,
        priority: i32,
    },
    Advance(u32),
    Pause(bool),
    Seek(i32),
    Skip,
}

fn config_strategy() -> impl Strategy<Value = SubtitleConfig> {
    (1usize..5, 0u32..256, 0u32..96, 0u32..128, 0u32..192).prop_map(
        |(max_lines, fade_in, max_wait, late_grace, fade_out)| SubtitleConfig {
            max_lines,
            fade_in: f64::from(fade_in) / 256.0,
            fade_out: f64::from(fade_out) / 256.0,
            max_wait: f64::from(max_wait) / 32.0,
            late_grace: f64::from(late_grace) / 64.0,
            ..SubtitleConfig::default()
        },
    )
}

/// Operations that never jump or stop time.
fn plain_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => (0u32..2000, 0u32..640, 1usize..14, -2i32..3).prop_map(
            |(start, duration, words, priority)| Op::Add { start, duration, words, priority }
        ),
        3 => (1u32..400).prop_map(Op::Advance),
    ]
}

fn any_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => plain_op(),
        1 => any::<bool>().prop_map(Op::Pause),
        1 => (-200i32..2000).prop_map(Op::Seek),
        1 => Just(Op::Skip),
    ]
}

/// What a run produced.
struct Run {
    events: Vec<SubtitleEvent>,
    /// The effective duration of every cue that was accepted, by id.
    durations: BTreeMap<CueId, f64>,
    /// What was on screen after every operation: (id, alpha, row, shown_for).
    frames: Vec<Vec<(u64, f32, usize, f64)>>,
}

/// Runs the operations. With `pieces` set, every [`Op::Advance`] is delivered as that many steps of 1/64 s instead of one.
fn run(config: SubtitleConfig, ops: &[Op], pieces: bool) -> Run {
    let mut s = Subtitles::new(config);
    let mut events = Vec::new();
    let mut durations = BTreeMap::new();
    let mut frames = Vec::new();
    for op in ops {
        match op {
            Op::Add {
                start,
                duration,
                words,
                priority,
            } => {
                let cue = Cue::new("w ".repeat(*words), f64::from(*start) / 64.0)
                    .duration(f64::from(*duration) / 64.0)
                    .priority(*priority);
                let eff = s.effective_duration(&cue);
                if let Ok(id) = s.add(cue) {
                    durations.insert(id, eff);
                }
            }
            Op::Advance(n) => {
                if pieces {
                    for _ in 0..*n {
                        s.advance(1.0 / 64.0);
                    }
                } else {
                    s.advance(f64::from(*n) / 64.0);
                }
            }
            Op::Pause(p) => s.set_paused(*p),
            Op::Seek(t) => s.seek(f64::from(*t) / 64.0),
            Op::Skip => {
                s.skip_all();
            }
        }
        events.extend(s.take_events());
        frames.push(
            s.visible()
                .iter()
                .map(|l| (l.id.0, l.alpha, l.row, l.shown_for))
                .collect(),
        );
        assert!(s.active_len() <= s.config().max_lines);
    }
    // Let everything run out (unpaused, at the end of time).
    s.set_paused(false);
    s.advance(100_000.0);
    events.extend(s.take_events());
    assert!(
        s.is_idle(),
        "{} active, {} pending",
        s.active_len(),
        s.pending_len()
    );
    Run {
        events,
        durations,
        frames,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Whatever happens: no more lines than allowed, alphas in 0..=1, rows are 0..n once each, oldest first.
    #[test]
    fn the_screen_is_always_valid(config in config_strategy(), ops in prop::collection::vec(any_op(), 1..60)) {
        let r = run(config, &ops, false);
        let max = config.sanitized().max_lines;
        for frame in &r.frames {
            prop_assert!(frame.len() <= max);
            let mut rows: Vec<usize> = frame.iter().map(|f| f.2).collect();
            rows.sort_unstable();
            prop_assert_eq!(rows, (0..frame.len()).collect::<Vec<_>>());
            for w in frame.windows(2) {
                prop_assert!(w[0].2 > w[1].2, "oldest first, rows falling");
            }
            for f in frame {
                prop_assert!(f.1.is_finite() && (0.0..=1.0).contains(&f.1), "alpha {}", f.1);
                prop_assert!(f.3 >= 0.0);
            }
        }
    }

    /// Without seeks or skips, a cue that finishes was shown for exactly its effective time, which is never less than its reading time; and every cue ends
    /// in exactly one way, after at most one start.
    #[test]
    fn every_finished_cue_is_shown_for_at_least_its_time(config in config_strategy(), ops in prop::collection::vec(plain_op(), 1..60)) {
        let r = run(config, &ops, false);
        let minimum = config.sanitized().min_duration;
        let mut started: BTreeMap<CueId, f64> = BTreeMap::new();
        let mut ended: BTreeMap<CueId, EventKind> = BTreeMap::new();
        for e in &r.events {
            match e.kind {
                EventKind::Started => {
                    prop_assert!(started.insert(e.id, e.at).is_none(), "{:?} started twice", e.id);
                }
                EventKind::Finished => {
                    let s = started.get(&e.id).copied();
                    prop_assert!(s.is_some(), "{:?} finished without starting", e.id);
                    let shown = e.at - s.unwrap();
                    let want = r.durations[&e.id];
                    prop_assert!(want >= minimum - 1e-12);
                    prop_assert!((shown - want).abs() < 1e-9, "{:?} shown {} wanted {}", e.id, shown, want);
                    prop_assert!(ended.insert(e.id, e.kind).is_none());
                }
                EventKind::Dropped(reason) => {
                    // Without seeks and skips only these two happen to an accepted cue.
                    prop_assert!(matches!(reason, DropReason::Overflow | DropReason::Evicted), "{:?}", reason);
                    prop_assert!(ended.insert(e.id, e.kind).is_none());
                    if reason == DropReason::Evicted {
                        prop_assert!(started.contains_key(&e.id));
                    } else {
                        prop_assert!(!started.contains_key(&e.id), "overflow after start");
                    }
                }
                EventKind::Rewound => prop_assert!(false, "rewound without a seek"),
            }
        }
        for id in r.durations.keys() {
            prop_assert!(ended.contains_key(id), "{:?} never ended", id);
        }
    }

    /// The same operations always give the same result.
    #[test]
    fn it_is_deterministic(config in config_strategy(), ops in prop::collection::vec(any_op(), 1..60)) {
        let a = run(config, &ops, false);
        let b = run(config, &ops, false);
        prop_assert_eq!(a.events, b.events);
        prop_assert_eq!(a.frames, b.frames);
    }

    /// One big step and the same time in many small ones give the same events at the same times.
    #[test]
    fn the_size_of_the_steps_does_not_matter(config in config_strategy(), ops in prop::collection::vec(plain_op(), 1..40)) {
        let a = run(config, &ops, false);
        let b = run(config, &ops, true);
        prop_assert_eq!(a.events.len(), b.events.len());
        for (x, y) in a.events.iter().zip(&b.events) {
            prop_assert_eq!((x.id, x.kind), (y.id, y.kind));
            prop_assert!((x.at - y.at).abs() < 1e-9, "{:?} vs {:?}", x, y);
        }
        // What is on screen at the end of each operation is the same too.
        prop_assert_eq!(a.frames.len(), b.frames.len());
        for (fa, fb) in a.frames.iter().zip(&b.frames) {
            prop_assert_eq!(fa.len(), fb.len());
            for (x, y) in fa.iter().zip(fb) {
                prop_assert_eq!((x.0, x.2), (y.0, y.2));
                prop_assert!((x.1 - y.1).abs() < 1e-4 && (x.3 - y.3).abs() < 1e-9);
            }
        }
    }

    /// With one line and equal priorities no two cues are ever on screen together.
    #[test]
    fn with_one_line_cues_never_overlap(ops in prop::collection::vec(plain_op(), 1..60)) {
        let ops: Vec<Op> = ops
            .into_iter()
            .map(|op| match op {
                Op::Add { start, duration, words, .. } => Op::Add { start, duration, words, priority: 0 },
                other => other,
            })
            .collect();
        let config = SubtitleConfig { max_lines: 1, ..SubtitleConfig::default() };
        let r = run(config, &ops, false);
        let mut open: Option<CueId> = None;
        for e in &r.events {
            match e.kind {
                EventKind::Started => {
                    prop_assert!(open.is_none(), "{:?} started while {:?} was on screen", e.id, open);
                    open = Some(e.id);
                }
                EventKind::Finished | EventKind::Dropped(DropReason::Evicted) => {
                    prop_assert_eq!(open, Some(e.id));
                    open = None;
                }
                _ => {}
            }
        }
        prop_assert!(open.is_none());
    }

    /// Bookkeeping holds across seeks and skips: every cue that was started is ended or rewound and started again; none is on screen and ended.
    #[test]
    fn started_and_ended_balance_even_with_seeks(config in config_strategy(), ops in prop::collection::vec(any_op(), 1..60)) {
        let r = run(config, &ops, false);
        let mut on_screen: BTreeMap<CueId, bool> = BTreeMap::new();
        for e in &r.events {
            let was = on_screen.get(&e.id).copied().unwrap_or(false);
            match e.kind {
                EventKind::Started => {
                    prop_assert!(!was, "{:?} started while already on screen", e.id);
                    on_screen.insert(e.id, true);
                }
                EventKind::Rewound | EventKind::Finished => {
                    prop_assert!(was, "{:?} {:?} while not on screen", e.id, e.kind);
                    on_screen.insert(e.id, false);
                }
                EventKind::Dropped(DropReason::Evicted) => {
                    prop_assert!(was);
                    on_screen.insert(e.id, false);
                }
                EventKind::Dropped(_) => {
                    on_screen.insert(e.id, false);
                }
            }
        }
        prop_assert!(on_screen.values().all(|on| !on), "something is still on screen");
    }
}
