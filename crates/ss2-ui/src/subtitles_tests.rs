//! Unit tests for the subtitle timing model (roadmap T06-012).

use crate::accessibility::AccessibilitySettings;
use crate::subtitles::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn subs() -> Subtitles {
    Subtitles::new(SubtitleConfig::default())
}

fn lines(max_lines: usize) -> Subtitles {
    Subtitles::new(SubtitleConfig {
        max_lines,
        ..SubtitleConfig::default()
    })
}

fn kinds(s: &mut Subtitles) -> Vec<(u64, EventKind)> {
    s.take_events()
        .into_iter()
        .map(|e| (e.id.0, e.kind))
        .collect()
}

fn ids(s: &Subtitles) -> Vec<u64> {
    s.visible().iter().map(|l| l.id.0).collect()
}

// ---- reading time -----------------------------------------------------------------------------------------------

#[test]
fn reading_time_is_words_over_speed_but_not_below_the_minimum() {
    let s = subs(); // 3 words a second, minimum 1 s
    assert!(close(s.reading_time("one"), 1.0));
    assert!(close(s.reading_time("one two three"), 1.0));
    assert!(close(s.reading_time("one two three four five six"), 2.0));
    assert!(close(s.reading_time(&"w ".repeat(9)), 3.0));
    assert!(close(s.reading_time(""), 1.0));
}

#[test]
fn text_without_spaces_counts_in_groups_of_six_characters() {
    let s = subs();
    assert!(close(s.reading_time(&"x".repeat(18)), 1.0)); // 3 groups
    assert!(close(s.reading_time(&"x".repeat(19)), 4.0 / 3.0)); // 4 groups: rounded up
    assert!(close(s.reading_time(&"x".repeat(36)), 2.0));
    // Spaces do not count as characters: "ab cd" is two words and four characters.
    assert!(close(s.reading_time("ab cd"), 1.0));
    // Many words of one letter: the words win over the character groups.
    assert!(close(s.reading_time(&"a ".repeat(12)), 4.0));
}

#[test]
fn few_long_words_use_the_character_count() {
    let s = subs();
    // 2 words, 31 characters (30 + 1): ceil(31 / 6) = 6 groups, so 6 / 3 = 2 s.
    assert!(close(s.reading_time(&format!("{} z", "y".repeat(30))), 2.0));
}

#[test]
fn reading_time_follows_the_configured_speed_and_minimum() {
    let s = Subtitles::new(SubtitleConfig {
        words_per_second: 2.0,
        min_duration: 0.5,
        ..SubtitleConfig::default()
    });
    assert!(close(s.reading_time("a b c d"), 2.0));
    assert!(close(s.reading_time("a"), 0.5));
}

#[test]
fn a_cue_lasts_as_long_as_its_duration_or_its_reading_time_whichever_is_more() {
    let s = subs();
    let long = Cue::new("one two", 0.0).duration(5.0);
    assert!(close(s.effective_duration(&long), 5.0));
    let short = Cue::new("one two", 0.0).duration(0.1);
    assert!(close(s.effective_duration(&short), 1.0));
    let unset = Cue::new("a b c d e f g h i", 0.0);
    assert!(close(s.effective_duration(&unset), 3.0));
}

// ---- appearing and leaving --------------------------------------------------------------------------------------

#[test]
fn a_cue_appears_at_its_start_and_leaves_at_its_end() {
    let mut s = subs();
    let id = s
        .add(Cue::new("Here they come", 2.0).speaker("Sam").priority(4))
        .unwrap();
    assert_eq!(id, CueId(0));
    assert_eq!(s.pending_len(), 1);
    assert!(!s.is_idle());
    s.advance(1.5);
    assert!(s.visible().is_empty());
    assert_eq!(s.pending_len(), 1);
    s.advance(0.5); // t = 2: exactly its start
    let v = s.visible();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].text, "Here they come");
    assert_eq!(v[0].speaker, Some("Sam"));
    assert_eq!(v[0].priority, 4);
    assert_eq!(v[0].id, id);
    assert_eq!(s.active_len(), 1);
    assert_eq!(s.pending_len(), 0);
    s.advance(0.75);
    assert!(close(s.visible()[0].shown_for, 0.75));
    s.advance(0.25); // t = 3: its end
    assert!(s.visible().is_empty());
    assert!(s.is_idle());
    let ev = s.take_events();
    assert_eq!(ev.len(), 2);
    assert_eq!(
        (ev[0].id, ev[0].kind, ev[0].at),
        (id, EventKind::Started, 2.0)
    );
    assert_eq!(
        (ev[1].id, ev[1].kind, ev[1].at),
        (id, EventKind::Finished, 3.0)
    );
    assert!(s.take_events().is_empty());
}

#[test]
fn a_cue_without_a_speaker_has_none() {
    let mut s = subs();
    s.add(Cue::new("hello", 0.0)).unwrap();
    assert_eq!(s.visible()[0].speaker, None);
}

#[test]
fn a_cue_is_shown_for_exactly_its_effective_time_whatever_the_step() {
    let mut s = subs();
    s.add(Cue::new("one two three four five six", 1.0)).unwrap(); // 2 s
    s.advance(100.0); // one huge step
    let ev = s.take_events();
    assert_eq!(ev.len(), 2);
    assert!(close(ev[0].at, 1.0) && ev[0].kind == EventKind::Started);
    assert!(close(ev[1].at, 3.0) && ev[1].kind == EventKind::Finished);
    assert!(close(s.now(), 100.0));
}

#[test]
fn a_cue_added_at_the_current_time_is_visible_at_once() {
    let mut s = subs();
    s.advance(5.0);
    s.add(Cue::new("now", 5.0)).unwrap();
    assert_eq!(s.visible().len(), 1);
}

// ---- fades ------------------------------------------------------------------------------------------------------

#[test]
fn the_alpha_rises_over_the_fade_in_and_falls_over_the_fade_out() {
    let mut s = subs(); // fade in 0.15 s, fade out 0.25 s
    s.add(Cue::new("hi", 0.0).duration(4.0)).unwrap();
    let alpha = |s: &Subtitles| f64::from(s.visible()[0].alpha);
    assert!(close(alpha(&s), 0.0));
    s.advance(0.075);
    assert!((alpha(&s) - 0.5).abs() < 1e-6);
    s.advance(0.075);
    assert!((alpha(&s) - 1.0).abs() < 1e-6);
    s.advance(1.0);
    assert!((alpha(&s) - 1.0).abs() < 1e-6);
    s.advance(2.725); // t = 3.875: 0.125 s left of a 0.25 s fade
    assert!((alpha(&s) - 0.5).abs() < 1e-6);
    s.advance(0.1); // t = 3.975: 0.025 s left
    assert!((alpha(&s) - 0.1).abs() < 1e-6);
}

#[test]
fn the_fades_of_a_very_short_cue_shrink_to_half_of_it_each() {
    let mut s = Subtitles::new(SubtitleConfig {
        min_duration: 0.2,
        words_per_second: 10.0,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("hi", 0.0)).unwrap(); // 0.2 s: each fade is 0.1 s, not 0.15 and 0.25
    let alpha = |s: &Subtitles| f64::from(s.visible()[0].alpha);
    s.advance(0.05);
    assert!((alpha(&s) - 0.5).abs() < 1e-6);
    s.advance(0.05); // t = 0.1: the peak
    assert!((alpha(&s) - 1.0).abs() < 1e-6);
    s.advance(0.05);
    assert!((alpha(&s) - 0.5).abs() < 1e-6);
}

#[test]
fn without_fades_the_alpha_is_one_throughout() {
    let mut s = Subtitles::new(SubtitleConfig {
        fade_in: 0.0,
        fade_out: 0.0,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("hi", 0.0).duration(2.0)).unwrap();
    assert_eq!(s.visible()[0].alpha, 1.0);
    s.advance(1.999);
    assert_eq!(s.visible()[0].alpha, 1.0);
}

#[test]
fn only_one_of_the_fades_can_be_off() {
    let mut s = Subtitles::new(SubtitleConfig {
        fade_in: 0.0,
        fade_out: 1.0,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("hi", 0.0).duration(4.0)).unwrap();
    assert_eq!(s.visible()[0].alpha, 1.0); // no fade-in
    s.advance(3.5);
    assert!((s.visible()[0].alpha - 0.5).abs() < 1e-6);
    let mut s = Subtitles::new(SubtitleConfig {
        fade_in: 1.0,
        fade_out: 0.0,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("hi", 0.0).duration(4.0)).unwrap();
    assert_eq!(s.visible()[0].alpha, 0.0);
    s.advance(3.999);
    assert_eq!(s.visible()[0].alpha, 1.0); // no fade-out
}

#[test]
fn the_box_fades_with_the_text_and_follows_the_players_settings() {
    let mut s = subs();
    s.add(Cue::new("hi", 0.0).duration(4.0)).unwrap();
    s.advance(0.075); // alpha 0.5
    let line = s.visible()[0];
    let normal = AccessibilitySettings::default().subtitle_style([1.0; 3]);
    assert!((line.box_alpha(&normal) - 0.3).abs() < 1e-6); // 0.5 x the translucent box (0.6)
    let strong = AccessibilitySettings {
        high_contrast: true,
        ..AccessibilitySettings::default()
    }
    .subtitle_style([1.0; 3]);
    assert!((line.box_alpha(&strong) - 0.5).abs() < 1e-6);
    let mut none = normal;
    none.background_alpha = 0.0;
    assert_eq!(line.box_alpha(&none), 0.0);
    let mut over = normal;
    over.background_alpha = 3.0;
    assert_eq!(line.box_alpha(&over), 1.0);
    let mut under = normal;
    under.background_alpha = -3.0;
    assert_eq!(line.box_alpha(&under), 0.0);
}

// ---- lines, priority, waiting -----------------------------------------------------------------------------------

#[test]
fn at_most_max_lines_are_shown_and_the_rest_wait() {
    let mut s = lines(3);
    for i in 0..5 {
        s.add(Cue::new(format!("line {i}"), 0.0).duration(4.0))
            .unwrap();
    }
    assert_eq!(s.active_len(), 3);
    assert_eq!(s.pending_len(), 2);
    assert_eq!(ids(&s), vec![0, 1, 2]);
}

#[test]
fn a_waiting_cue_gets_a_line_when_one_ends_and_is_then_shown_for_its_whole_time() {
    let mut s = lines(1);
    s.add(Cue::new("first", 0.0).duration(1.0)).unwrap();
    let b = s.add(Cue::new("second", 0.0).duration(2.0)).unwrap();
    s.take_events();
    s.advance(1.25); // the first ended at 1; the second has waited 1 s of its 1.5 s
    assert_eq!(ids(&s), vec![b.0]);
    assert!(close(s.visible()[0].shown_for, 0.25));
    s.advance(10.0);
    let ev = s.take_events();
    let started = ev.iter().find(|e| e.kind == EventKind::Started).unwrap();
    let finished = ev
        .iter()
        .find(|e| e.id == b && e.kind == EventKind::Finished)
        .unwrap();
    assert!(close(started.at, 1.0)); // the moment the line was free, not the end of a step
    assert!(close(finished.at - started.at, 2.0)); // the full 2 s
}

#[test]
fn a_cue_that_waits_longer_than_max_wait_is_dropped_at_that_moment() {
    let mut s = lines(1);
    s.add(Cue::new("long", 0.0).duration(10.0)).unwrap();
    let b = s.add(Cue::new("starved", 1.0).duration(1.0)).unwrap();
    s.take_events();
    s.advance(2.0); // t = 2: it has waited 1 s of 1.5
    assert_eq!(s.pending_len(), 1);
    assert!(s.take_events().is_empty());
    s.advance(0.5); // t = 2.5: 1.5 s
    assert_eq!(s.pending_len(), 0);
    let ev = s.take_events();
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].id, b);
    assert_eq!(ev[0].kind, EventKind::Dropped(DropReason::Overflow));
    assert!(close(ev[0].at, 2.5));
}

#[test]
fn the_giving_up_time_is_exact_even_in_one_large_step() {
    let mut s = lines(1);
    s.add(Cue::new("long", 0.0).duration(100.0)).unwrap();
    s.add(Cue::new("starved", 1.0).duration(1.0)).unwrap();
    s.take_events();
    s.advance(50.0);
    let ev = s.take_events();
    assert_eq!(ev.len(), 1);
    assert!(close(ev[0].at, 2.5));
}

#[test]
fn with_no_patience_a_cue_without_a_line_is_dropped_at_once() {
    let mut s = Subtitles::new(SubtitleConfig {
        max_lines: 1,
        max_wait: 0.0,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("a", 0.0).duration(5.0)).unwrap();
    let b = s.add(Cue::new("b", 0.0).duration(5.0)).unwrap();
    assert_eq!(s.pending_len(), 0);
    assert_eq!(
        kinds(&mut s),
        vec![
            (0, EventKind::Started),
            (b.0, EventKind::Dropped(DropReason::Overflow))
        ]
    );
}

#[test]
fn a_higher_priority_cue_takes_the_line_of_the_lowest_priority_one() {
    let mut s = lines(2);
    s.add(Cue::new("a", 0.0).duration(10.0).priority(1))
        .unwrap();
    s.add(Cue::new("b", 0.0).duration(10.0).priority(0))
        .unwrap();
    s.add(Cue::new("c", 1.0).duration(10.0).priority(5))
        .unwrap();
    s.take_events();
    s.advance(1.0);
    assert_eq!(ids(&s), vec![0, 2]); // b was pushed off
    assert_eq!(
        kinds(&mut s),
        vec![
            (1, EventKind::Dropped(DropReason::Evicted)),
            (2, EventKind::Started)
        ]
    );
}

#[test]
fn an_equal_priority_cue_does_not_evict_it_waits() {
    let mut s = lines(1);
    s.add(Cue::new("a", 0.0).duration(10.0).priority(3))
        .unwrap();
    s.add(Cue::new("b", 0.5).duration(10.0).priority(3))
        .unwrap();
    s.advance(1.0);
    assert_eq!(ids(&s), vec![0]);
    assert_eq!(s.pending_len(), 1);
    // And a lower priority one does not either.
    s.add(Cue::new("c", 1.0).duration(10.0).priority(-1))
        .unwrap();
    assert_eq!(ids(&s), vec![0]);
}

#[test]
fn among_equal_lowest_priorities_the_oldest_is_evicted() {
    let mut s = lines(2);
    s.add(Cue::new("x", 0.0).duration(10.0)).unwrap();
    s.add(Cue::new("y", 1.0).duration(10.0)).unwrap();
    s.add(Cue::new("z", 2.0).duration(10.0).priority(1))
        .unwrap();
    s.advance(2.0);
    assert_eq!(ids(&s), vec![1, 2]);
}

#[test]
fn the_lowest_priority_is_evicted_even_when_it_is_the_newest() {
    let mut s = lines(2);
    s.add(Cue::new("x", 0.0).duration(10.0).priority(2))
        .unwrap();
    s.add(Cue::new("y", 1.0).duration(10.0).priority(0))
        .unwrap();
    s.add(Cue::new("z", 2.0).duration(10.0).priority(1))
        .unwrap();
    s.advance(2.0);
    assert_eq!(ids(&s), vec![0, 2]);
}

#[test]
fn a_negative_priority_is_lower_than_the_default() {
    let mut s = lines(1);
    s.add(Cue::new("x", 0.0).duration(10.0).priority(-5))
        .unwrap();
    s.add(Cue::new("y", 1.0).duration(10.0)).unwrap();
    s.advance(1.0);
    assert_eq!(ids(&s), vec![1]);
}

#[test]
fn of_two_cues_starting_together_the_higher_priority_gets_the_only_line() {
    let mut s = lines(1);
    s.add(Cue::new("low", 1.0).duration(5.0).priority(0))
        .unwrap();
    s.add(Cue::new("high", 1.0).duration(5.0).priority(2))
        .unwrap();
    s.advance(1.0);
    assert_eq!(ids(&s), vec![1]);
    assert_eq!(s.pending_len(), 1); // the other waits
}

#[test]
fn of_two_equal_cues_starting_together_the_first_added_wins() {
    let mut s = lines(1);
    s.add(Cue::new("first", 1.0).duration(5.0)).unwrap();
    s.add(Cue::new("second", 1.0).duration(5.0)).unwrap();
    s.advance(1.0);
    assert_eq!(ids(&s), vec![0]);
}

#[test]
fn rows_count_up_from_the_newest_line() {
    let mut s = lines(3);
    for i in 0..3 {
        s.add(Cue::new(format!("c{i}"), f64::from(i)).duration(10.0))
            .unwrap();
    }
    s.advance(2.0);
    let v = s.visible();
    assert_eq!(
        v.iter().map(|l| (l.id.0, l.row)).collect::<Vec<_>>(),
        vec![(0, 2), (1, 1), (2, 0)]
    );
}

#[test]
fn rows_close_up_when_a_middle_line_ends() {
    let mut s = lines(3);
    s.add(Cue::new("a", 0.0).duration(10.0)).unwrap();
    s.add(Cue::new("b", 1.0).duration(1.0)).unwrap();
    s.add(Cue::new("c", 2.0).duration(10.0)).unwrap();
    s.advance(2.0); // b ends exactly now
    let v = s.visible();
    assert_eq!(
        v.iter().map(|l| (l.id.0, l.row)).collect::<Vec<_>>(),
        vec![(0, 1), (2, 0)]
    );
}

// ---- arriving late and out of order -----------------------------------------------------------------------------

#[test]
fn a_cue_a_little_late_is_shifted_to_now_and_shown_for_its_full_time() {
    let mut s = subs();
    s.advance(10.0);
    let id = s.add(Cue::new("late", 9.6).duration(2.0)).unwrap();
    let v = s.visible();
    assert_eq!(v.len(), 1);
    assert!(close(v[0].shown_for, 0.0));
    s.advance(1.9);
    assert_eq!(s.active_len(), 1);
    s.advance(0.1);
    assert!(s.is_idle());
    let ev = s.take_events();
    assert!(
        ev.iter()
            .any(|e| e.id == id && e.kind == EventKind::Finished && close(e.at, 12.0))
    );
}

#[test]
fn a_cue_exactly_at_the_grace_limit_is_accepted_and_one_beyond_it_is_late() {
    let mut s = subs(); // grace 0.5
    s.advance(10.0);
    assert!(s.add(Cue::new("edge", 9.5)).is_ok());
    assert_eq!(s.add(Cue::new("late", 9.49)), Err(DropReason::Late));
    assert_eq!(s.add(Cue::new("very late", -100.0)), Err(DropReason::Late));
}

#[test]
fn a_refused_cue_leaves_no_trace() {
    let mut s = subs();
    s.advance(10.0);
    assert!(s.add(Cue::new("late", 1.0)).is_err());
    assert!(s.take_events().is_empty());
    assert!(s.is_idle());
    // And its id was not used up.
    assert_eq!(s.add(Cue::new("ok", 10.0)).unwrap(), CueId(0));
}

#[test]
fn cues_added_out_of_order_start_in_time_order() {
    let mut s = lines(1);
    let c = s.add(Cue::new("c", 5.0)).unwrap();
    let a = s.add(Cue::new("a", 1.0)).unwrap();
    let b = s.add(Cue::new("b", 3.0)).unwrap();
    s.advance(20.0);
    let started: Vec<CueId> = s
        .take_events()
        .into_iter()
        .filter(|e| e.kind == EventKind::Started)
        .map(|e| e.id)
        .collect();
    assert_eq!(started, vec![a, b, c]);
}

#[test]
fn a_cue_may_be_added_while_others_are_on_screen() {
    let mut s = lines(2);
    s.add(Cue::new("a", 0.0).duration(10.0)).unwrap();
    s.advance(3.0);
    s.add(Cue::new("b", 3.0).duration(10.0)).unwrap();
    assert_eq!(ids(&s), vec![0, 1]);
}

// ---- refusals ---------------------------------------------------------------------------------------------------

#[test]
fn cues_that_make_no_sense_are_refused() {
    let mut s = subs();
    assert_eq!(s.add(Cue::new("", 0.0)), Err(DropReason::Invalid));
    assert_eq!(s.add(Cue::new("  \t\n", 0.0)), Err(DropReason::Invalid));
    assert_eq!(s.add(Cue::new("x", f64::NAN)), Err(DropReason::Invalid));
    assert_eq!(
        s.add(Cue::new("x", f64::INFINITY)),
        Err(DropReason::Invalid)
    );
    assert_eq!(
        s.add(Cue::new("x", f64::NEG_INFINITY)),
        Err(DropReason::Invalid)
    );
    assert_eq!(
        s.add(Cue::new("x", 0.0).duration(-1.0)),
        Err(DropReason::Invalid)
    );
    assert_eq!(
        s.add(Cue::new("x", 0.0).duration(f64::NAN)),
        Err(DropReason::Invalid)
    );
    assert_eq!(
        s.add(Cue::new("x", 0.0).duration(f64::INFINITY)),
        Err(DropReason::Invalid)
    );
    assert!(s.is_idle());
    assert!(s.add(Cue::new("x", 0.0).duration(0.0)).is_ok());
}

#[test]
fn the_queue_is_limited_but_cues_on_screen_do_not_count() {
    let mut s = Subtitles::new(SubtitleConfig {
        max_pending: 2,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("on screen", 0.0).duration(10.0)).unwrap();
    assert!(s.add(Cue::new("p1", 5.0)).is_ok());
    assert!(s.add(Cue::new("p2", 6.0)).is_ok());
    assert_eq!(s.add(Cue::new("p3", 7.0)), Err(DropReason::QueueFull));
    assert_eq!(s.pending_len(), 2);
}

#[test]
fn drop_reasons_explain_themselves() {
    for r in [
        DropReason::Invalid,
        DropReason::Late,
        DropReason::QueueFull,
        DropReason::Overflow,
        DropReason::Evicted,
        DropReason::Skipped,
    ] {
        assert!(!r.to_string().is_empty());
    }
    let texts: std::collections::BTreeSet<String> = [
        DropReason::Invalid,
        DropReason::Late,
        DropReason::QueueFull,
        DropReason::Overflow,
        DropReason::Evicted,
        DropReason::Skipped,
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(texts.len(), 6);
}

// ---- time: bad steps, pause -------------------------------------------------------------------------------------

#[test]
fn bad_time_steps_change_nothing() {
    let mut s = subs();
    s.add(Cue::new("x", 5.0)).unwrap();
    s.advance(2.0);
    for dt in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        s.advance(dt);
        assert_eq!(s.now(), 2.0);
    }
}

#[test]
fn nothing_moves_while_paused_and_it_goes_on_after() {
    let mut s = subs();
    s.add(Cue::new("x", 0.0).duration(4.0)).unwrap();
    s.advance(1.0);
    let before = (s.visible()[0].alpha, s.visible()[0].shown_for);
    assert!(!s.is_paused());
    s.set_paused(true);
    assert!(s.is_paused());
    s.advance(100.0);
    assert_eq!(s.now(), 1.0);
    assert_eq!((s.visible()[0].alpha, s.visible()[0].shown_for), before);
    s.set_paused(false);
    s.advance(2.0);
    assert_eq!(s.active_len(), 1);
    s.advance(1.0);
    assert!(s.is_idle());
    let ev = s.take_events();
    assert!(close(ev.last().unwrap().at, 4.0));
}

#[test]
fn cues_can_be_added_while_paused() {
    let mut s = subs();
    s.advance(2.0);
    s.set_paused(true);
    s.add(Cue::new("x", 2.0)).unwrap();
    assert_eq!(s.active_len(), 1);
    s.add(Cue::new("y", 9.0)).unwrap();
    assert_eq!(s.pending_len(), 1);
}

// ---- seeking and skipping ---------------------------------------------------------------------------------------

fn three_cues() -> Subtitles {
    let mut s = subs();
    s.add(Cue::new("a", 0.0).duration(2.0)).unwrap();
    s.add(Cue::new("b", 3.0).duration(2.0)).unwrap();
    s.add(Cue::new("c", 10.0).duration(2.0)).unwrap();
    s
}

#[test]
fn seeking_forward_drops_what_would_have_ended_keeps_what_it_lands_in_and_queues_the_rest() {
    let mut s = three_cues();
    s.take_events();
    s.seek(4.0);
    assert!(close(s.now(), 4.0));
    assert_eq!(ids(&s), vec![1]); // b is half-way
    assert_eq!(s.pending_len(), 1); // c is still to come
    assert_eq!(
        kinds(&mut s),
        vec![
            (0, EventKind::Dropped(DropReason::Skipped)),
            (1, EventKind::Started)
        ]
    );
    let line = s.visible()[0];
    assert!(close(line.shown_for, 1.0)); // it has been "on" since its own start
    assert_eq!(line.alpha, 1.0); // no new fade-in
    s.advance(1.0); // b ends at 5, its own time, not 2 s after the jump
    assert!(s.visible().is_empty());
    s.advance(5.0);
    assert_eq!(ids(&s), vec![2]);
}

#[test]
fn a_cue_shown_after_a_seek_does_not_fade_in_again_but_does_fade_out() {
    let mut s = subs();
    s.add(Cue::new("b", 3.0).duration(4.0)).unwrap(); // 3..7
    s.seek(6.875); // 0.125 s left of the 0.25 s fade-out
    assert!((s.visible()[0].alpha - 0.5).abs() < 1e-6);
}

#[test]
fn seeking_past_everything_leaves_nothing_but_events() {
    let mut s = three_cues();
    s.take_events();
    s.seek(100.0);
    assert!(s.is_idle());
    let ev = kinds(&mut s);
    assert_eq!(ev.len(), 3);
    assert!(
        ev.iter()
            .all(|(_, k)| *k == EventKind::Dropped(DropReason::Skipped))
    );
}

#[test]
fn seeking_forward_onto_a_cues_exact_end_drops_it_and_onto_its_start_shows_it() {
    let mut s = subs();
    s.add(Cue::new("a", 1.0).duration(2.0)).unwrap(); // 1..3
    s.seek(3.0);
    assert!(s.is_idle());
    let mut s = subs();
    s.add(Cue::new("a", 1.0).duration(2.0)).unwrap();
    s.seek(1.0);
    assert_eq!(s.active_len(), 1);
    assert!(close(s.visible()[0].shown_for, 0.0));
}

#[test]
fn a_seek_that_lands_in_more_cues_than_there_are_lines_shows_the_best_and_skips_the_others() {
    let mut s = lines(2);
    for (i, p) in [0, 5, 3, 1].into_iter().enumerate() {
        s.add(
            Cue::new(format!("c{i}"), 1.0 + i as f64)
                .duration(100.0)
                .priority(p),
        )
        .unwrap();
    }
    s.take_events();
    s.seek(50.0);
    assert_eq!(s.pending_len(), 0);
    let shown = ids(&s);
    assert_eq!(shown.len(), 2);
    // c0 (priority 0) came first, c1 (5) evicts nobody (a free line), c2 (3) evicts c0, c3 (1) is not above anyone and is skipped.
    assert_eq!(shown, vec![1, 2]);
    let ev = kinds(&mut s);
    assert!(ev.contains(&(0, EventKind::Dropped(DropReason::Evicted))));
    assert!(ev.contains(&(3, EventKind::Dropped(DropReason::Skipped))));
}

#[test]
fn a_seek_into_a_cue_with_a_line_taken_by_something_equal_skips_it() {
    let mut s = lines(1);
    s.add(Cue::new("p", 0.0).duration(100.0)).unwrap();
    s.add(Cue::new("q", 1.0).duration(100.0)).unwrap();
    s.take_events();
    s.seek(2.0);
    assert_eq!(ids(&s), vec![0]);
    assert_eq!(s.pending_len(), 0);
    assert_eq!(
        kinds(&mut s),
        vec![(1, EventKind::Dropped(DropReason::Skipped))]
    );
}

#[test]
fn seeking_forward_keeps_a_cue_that_is_already_on_screen_and_unfinished() {
    let mut s = subs();
    s.add(Cue::new("a", 0.0).duration(10.0)).unwrap();
    s.advance(1.0);
    s.seek(5.0);
    assert_eq!(ids(&s), vec![0]);
    assert!(close(s.visible()[0].shown_for, 5.0));
}

#[test]
fn seeking_backwards_requeues_cues_that_started_later_and_does_not_replay_finished_ones() {
    let mut s = subs();
    s.add(Cue::new("a", 0.0).duration(2.0)).unwrap();
    s.add(Cue::new("b", 3.0).duration(2.0)).unwrap();
    s.advance(4.0); // a finished; b on screen since 3
    s.take_events();
    s.seek(1.0);
    assert!(close(s.now(), 1.0));
    assert!(s.visible().is_empty()); // a is not replayed
    assert_eq!(s.pending_len(), 1);
    assert_eq!(kinds(&mut s), vec![(1, EventKind::Rewound)]);
    s.advance(1.9);
    assert!(s.visible().is_empty());
    s.advance(0.1); // t = 3
    assert_eq!(ids(&s), vec![1]);
    assert_eq!(kinds(&mut s), vec![(1, EventKind::Started)]);
}

#[test]
fn seeking_back_inside_a_cue_keeps_it_with_its_alpha_recomputed() {
    let mut s = subs();
    s.add(Cue::new("a", 0.0).duration(4.0)).unwrap();
    s.advance(3.0);
    s.seek(0.075);
    assert_eq!(ids(&s), vec![0]);
    assert!((s.visible()[0].alpha - 0.5).abs() < 1e-6);
    assert!(close(s.visible()[0].shown_for, 0.075));
    assert!(s.take_events().iter().all(|e| e.kind != EventKind::Rewound));
}

#[test]
fn seeking_to_exactly_a_cues_first_instant_keeps_it() {
    let mut s = subs();
    s.add(Cue::new("a", 1.0).duration(4.0)).unwrap();
    s.advance(2.0);
    s.seek(1.0); // shown_from == to: not "later than" the target
    assert_eq!(ids(&s), vec![0]);
    assert!(s.take_events().iter().all(|e| e.kind != EventKind::Rewound));
}

#[test]
fn a_rewound_cue_returns_in_order_with_the_others() {
    let mut s = lines(1);
    s.add(Cue::new("a", 2.0).duration(1.0)).unwrap();
    s.add(Cue::new("b", 4.0).duration(1.0)).unwrap();
    s.advance(4.5); // a done, b on screen
    s.seek(0.0);
    s.add(Cue::new("c", 3.0).duration(0.5)).unwrap();
    s.take_events();
    s.advance(10.0);
    let order: Vec<u64> = s
        .take_events()
        .into_iter()
        .filter(|e| e.kind == EventKind::Started)
        .map(|e| e.id.0)
        .collect();
    assert_eq!(order, vec![2, 1]); // c at 3, b at 4 (a is gone)
}

#[test]
fn a_seek_to_a_bad_time_is_ignored_and_a_seek_works_while_paused() {
    let mut s = three_cues();
    s.advance(1.0);
    s.seek(f64::NAN);
    s.seek(f64::INFINITY);
    s.seek(f64::NEG_INFINITY);
    assert_eq!(s.now(), 1.0);
    assert_eq!(ids(&s), vec![0]);
    s.set_paused(true);
    s.seek(4.0);
    assert_eq!(s.now(), 4.0);
    assert_eq!(ids(&s), vec![1]);
}

#[test]
fn seeking_to_the_current_time_changes_nothing() {
    let mut s = three_cues();
    s.advance(1.0);
    s.take_events();
    s.seek(1.0);
    assert_eq!(ids(&s), vec![0]);
    assert!(s.take_events().is_empty());
}

#[test]
fn skipping_a_cutscene_drops_everything_and_says_how_many() {
    let mut s = lines(2);
    for i in 0..5 {
        s.add(Cue::new(format!("c{i}"), f64::from(i)).duration(50.0))
            .unwrap();
    }
    s.advance(2.0); // two shown, one waiting (c2 started at 2: needs a line), two to come
    s.take_events();
    let n = s.skip_all();
    assert_eq!(n, 5);
    assert!(s.is_idle());
    assert!(close(s.now(), 2.0));
    let ev = kinds(&mut s);
    assert_eq!(ev.len(), 5);
    assert!(
        ev.iter()
            .all(|(_, k)| *k == EventKind::Dropped(DropReason::Skipped))
    );
    assert_eq!(s.skip_all(), 0);
    // Cues for what comes next are still accepted.
    s.add(Cue::new("next", 2.0)).unwrap();
    assert_eq!(s.active_len(), 1);
}

#[test]
fn ids_are_never_reused_after_a_skip() {
    let mut s = subs();
    let a = s.add(Cue::new("a", 0.0)).unwrap();
    s.skip_all();
    let b = s.add(Cue::new("b", 0.0)).unwrap();
    assert!(b > a);
}

// ---- configuration and events -----------------------------------------------------------------------------------

#[test]
fn the_configuration_is_clamped_into_its_ranges() {
    let wild = SubtitleConfig {
        max_lines: 0,
        words_per_second: 1000.0,
        min_duration: -4.0,
        fade_in: 99.0,
        fade_out: 99.0,
        late_grace: 99.0,
        max_wait: 99.0,
        max_pending: 0,
    }
    .sanitized();
    assert_eq!(wild.max_lines, 1);
    assert_eq!(wild.words_per_second, 10.0);
    assert_eq!(wild.min_duration, 0.0);
    assert_eq!(wild.fade_in, 5.0);
    assert_eq!(wild.fade_out, 5.0);
    assert_eq!(wild.late_grace, 10.0);
    assert_eq!(wild.max_wait, 30.0);
    assert_eq!(wild.max_pending, 1);
    let big = SubtitleConfig {
        max_lines: 100,
        words_per_second: 0.0,
        min_duration: 100.0,
        max_pending: 10_000,
        ..SubtitleConfig::default()
    }
    .sanitized();
    assert_eq!(big.max_lines, MAX_LINES_LIMIT);
    assert_eq!(big.words_per_second, 0.5);
    assert_eq!(big.min_duration, 30.0);
    assert_eq!(big.max_pending, 256);
}

#[test]
fn a_configuration_in_range_is_unchanged_and_nan_becomes_the_default() {
    let d = SubtitleConfig::default();
    assert_eq!(d.sanitized(), d);
    let nan = SubtitleConfig {
        words_per_second: f64::NAN,
        min_duration: f64::NAN,
        fade_in: f64::INFINITY,
        fade_out: f64::NAN,
        late_grace: f64::NAN,
        max_wait: f64::NEG_INFINITY,
        ..d
    }
    .sanitized();
    assert_eq!(nan, d);
    let edge = SubtitleConfig {
        max_lines: MAX_LINES_LIMIT,
        words_per_second: 0.5,
        min_duration: 30.0,
        fade_in: 5.0,
        fade_out: 5.0,
        late_grace: 10.0,
        max_wait: 30.0,
        max_pending: 256,
    };
    assert_eq!(edge.sanitized(), edge);
    let low = SubtitleConfig {
        max_lines: 1,
        words_per_second: 10.0,
        min_duration: 0.0,
        fade_in: 0.0,
        fade_out: 0.0,
        late_grace: 0.0,
        max_wait: 0.0,
        max_pending: 1,
    };
    assert_eq!(low.sanitized(), low);
}

#[test]
fn new_applies_the_sanitizing() {
    let s = Subtitles::new(SubtitleConfig {
        max_lines: 50,
        ..SubtitleConfig::default()
    });
    assert_eq!(s.config().max_lines, MAX_LINES_LIMIT);
    assert_eq!(s.now(), 0.0);
    assert!(!s.is_paused());
    assert!(s.is_idle());
}

#[test]
fn only_the_newest_events_are_kept_when_nobody_reads_them() {
    let mut s = Subtitles::new(SubtitleConfig {
        max_pending: 256,
        ..SubtitleConfig::default()
    });
    for i in 0..200 {
        s.add(Cue::new("x", f64::from(i) * 2.0).duration(1.0))
            .unwrap();
    }
    s.advance(1000.0); // 400 events
    let ev = s.take_events();
    assert_eq!(ev.len(), MAX_EVENTS);
    assert_eq!(ev.last().unwrap().id, CueId(199));
    assert_eq!(ev.last().unwrap().kind, EventKind::Finished);
    assert_eq!(ev[0].id, CueId(72)); // 400 - 256 = 144 events lost: the first 72 cues
}

#[test]
fn the_default_configuration_is_the_documented_one() {
    let d = SubtitleConfig::default();
    assert_eq!(d.max_lines, 3);
    assert_eq!(d.words_per_second, 3.0);
    assert_eq!(d.min_duration, 1.0);
    assert_eq!(d.fade_in, 0.15);
    assert_eq!(d.fade_out, 0.25);
    assert_eq!(d.late_grace, 0.5);
    assert_eq!(d.max_wait, 1.5);
    assert_eq!(d.max_pending, 64);
    assert_eq!(MAX_LINES_LIMIT, 8);
    assert_eq!(MAX_EVENTS, 256);
    assert_eq!(CHARS_PER_WORD, 6);
}

#[test]
fn a_shifted_late_cue_waits_from_the_time_it_was_shifted_to() {
    let mut s = lines(1);
    s.add(Cue::new("long", 0.0).duration(100.0)).unwrap();
    s.advance(10.0);
    let id = s.add(Cue::new("late", 9.75).duration(1.0)).unwrap(); // 0.25 late: waits from 10, not from 9.75
    s.take_events();
    s.advance(1.5); // t = 11.5: 1.5 s after 10
    let ev = s.take_events();
    assert_eq!(ev.len(), 1);
    assert_eq!(
        (ev[0].id, ev[0].kind),
        (id, EventKind::Dropped(DropReason::Overflow))
    );
    assert!(close(ev[0].at, 11.5));
}

#[test]
fn a_cue_of_higher_priority_evicts_the_lowest_while_another_cue_waits() {
    let mut s = lines(2);
    s.add(Cue::new("x", 0.0).duration(10.0).priority(1))
        .unwrap();
    s.add(Cue::new("y", 1.0).duration(10.0)).unwrap();
    s.add(Cue::new("z", 1.0).duration(10.0)).unwrap();
    s.add(Cue::new("w", 2.0).duration(10.0).priority(1))
        .unwrap();
    s.advance(2.0);
    // x (prio 1) and y (prio 0) are shown; z waits. w (prio 1) evicts y, the lowest.
    assert_eq!(ids(&s), vec![0, 3]);
    assert_eq!(s.pending_len(), 1);
}

#[test]
fn equal_cues_given_a_line_together_leave_in_the_order_they_were_added() {
    // Two cues that both waited for lines and got them at the same instant, though their starts (and so their order on screen) differ from the order added.
    let mut s = Subtitles::new(SubtitleConfig {
        max_lines: 2,
        max_wait: 5.0,
        ..SubtitleConfig::default()
    });
    s.add(Cue::new("p", 0.0).duration(3.0)).unwrap(); // 0
    s.add(Cue::new("q", 0.0).duration(3.0)).unwrap(); // 1
    let b = s.add(Cue::new("b", 2.0).duration(10.0)).unwrap(); // 2: starts later, added earlier
    let a = s.add(Cue::new("a", 1.0).duration(10.0)).unwrap(); // 3: starts earlier, added later
    s.advance(3.0); // p and q end; a then b get the lines, both from t = 3
    assert_eq!(ids(&s), vec![b.0, a.0]); // same shown_from: ordered by id
    s.add(Cue::new("z", 3.0).duration(10.0).priority(1))
        .unwrap();
    assert_eq!(ids(&s), vec![a.0, 4]); // the earlier added, b, is the one evicted
}

#[test]
fn the_cue_shown_longest_is_evicted_first_whatever_its_id() {
    let mut s = lines(2);
    let late_start = s.add(Cue::new("b", 5.0).duration(20.0)).unwrap(); // id 0, but it starts later
    let early_start = s.add(Cue::new("a", 0.0).duration(20.0)).unwrap(); // id 1, shown since 0
    s.advance(5.0);
    assert_eq!(ids(&s), vec![early_start.0, late_start.0]);
    s.add(Cue::new("z", 6.0).duration(20.0).priority(1))
        .unwrap();
    s.advance(1.0);
    assert_eq!(ids(&s), vec![late_start.0, 2]); // a, shown since 0, went; b, shown since 5, stayed
}

#[test]
fn seeking_exactly_to_the_end_of_a_cue_on_screen_ends_it() {
    let mut s = subs();
    s.add(Cue::new("a", 0.0).duration(2.0)).unwrap();
    s.advance(1.0);
    s.take_events();
    s.seek(2.0);
    assert!(s.visible().is_empty());
    assert!(s.is_idle());
    assert_eq!(
        kinds(&mut s),
        vec![(0, EventKind::Dropped(DropReason::Skipped))]
    );
}
