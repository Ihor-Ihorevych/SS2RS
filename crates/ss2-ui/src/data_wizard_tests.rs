use std::path::PathBuf;

use ss2_core::input::NavInput;

use crate::data_wizard::*;
use crate::draw::DrawList;
use crate::geometry::{CANVAS_H, CANVAS_W, Rect, Vec2};
use crate::menu::UiInput;
use crate::theme;

// ---------------------------------------------------------------------------------------------------------------------------------
// Helpers

fn nav(w: &mut DataWizard, n: NavInput) -> Option<WizardCommand> {
    w.handle(WizardInput::Ui(UiInput::Nav(n)))
}

fn typed(w: &mut DataWizard, text: &str) {
    assert_eq!(w.handle(WizardInput::Text(text.to_owned())), None);
}

fn candidate(name: &str) -> WizardCandidate {
    WizardCandidate {
        dir: PathBuf::from(name),
        detail: format!("Steam library, patch 2.100 ({name})"),
    }
}

fn candidates(n: usize) -> Vec<WizardCandidate> {
    (0..n)
        .map(|i| candidate(&format!("/games/sam{i}")))
        .collect()
}

fn report(usable: bool, warnings: usize, lines: &[&str]) -> WizardReport {
    WizardReport {
        dir: PathBuf::from("/games/sam0"),
        usable,
        warnings,
        headline: if usable {
            "Serious Sam 2 found"
        } else {
            "This folder cannot be used"
        }
        .to_owned(),
        lines: lines.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// A wizard that has searched and found `n` folders.
fn found(n: usize) -> DataWizard {
    let mut w = DataWizard::new(None);
    assert_eq!(w.start(), WizardCommand::Search);
    w.set_candidates(candidates(n));
    assert_eq!(w.view(), WizardView::Candidates);
    w
}

/// A wizard on the manual screen.
fn manual() -> DataWizard {
    let mut w = found(0);
    assert_eq!(nav(&mut w, NavInput::Activate), None);
    assert_eq!(w.view(), WizardView::Manual);
    w
}

/// A wizard showing a result for the first of two candidates.
fn result(usable: bool) -> DataWizard {
    let mut w = found(2);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from("/games/sam0")))
    );
    w.set_report(report(usable, 0, &["Archives: 14 found, 14 readable"]));
    assert_eq!(w.view(), WizardView::Result);
    w
}

fn texts(list: &DrawList) -> Vec<String> {
    list.texts.iter().map(|t| t.text.to_string()).collect()
}

fn center_of(r: Rect) -> Vec2 {
    Vec2 {
        x: r.x + r.w / 2.0,
        y: r.y + r.h / 2.0,
    }
}

fn rect_of(w: &DataWizard, row: usize) -> Rect {
    w.row_rects()
        .into_iter()
        .find(|(i, _)| *i == row)
        .unwrap()
        .1
}

fn click(w: &mut DataWizard, p: Vec2) -> Option<WizardCommand> {
    w.handle(WizardInput::Ui(UiInput::PointerPressed(p)));
    w.handle(WizardInput::Ui(UiInput::PointerReleased(p)))
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The text field

#[test]
fn typed_text_goes_in_at_the_cursor_and_the_cursor_moves() {
    let mut f = TextField::new(100);
    f.insert_str("abd");
    assert_eq!((f.text(), f.cursor(), f.len()), ("abd", 3, 3));
    f.left();
    f.insert_str("c");
    assert_eq!((f.text(), f.cursor()), ("abcd", 3));
    f.home();
    assert_eq!(f.cursor(), 0);
    f.left();
    assert_eq!(f.cursor(), 0, "the cursor stops at the start");
    f.insert_str(">");
    assert_eq!((f.text(), f.cursor()), (">abcd", 1));
    f.end();
    assert_eq!(f.cursor(), 5);
    f.right();
    assert_eq!(f.cursor(), 5, "and at the end");
    f.left();
    f.right();
    assert_eq!(f.cursor(), 5);
}

#[test]
fn backspace_and_delete_remove_around_the_cursor_and_stop_at_the_edges() {
    let mut f = TextField::new(100);
    f.insert_str("abcd");
    f.left();
    f.left();
    f.backspace();
    assert_eq!((f.text(), f.cursor()), ("acd", 1));
    f.delete();
    assert_eq!((f.text(), f.cursor()), ("ad", 1));
    f.home();
    f.backspace();
    assert_eq!(
        (f.text(), f.cursor()),
        ("ad", 0),
        "nothing before the start"
    );
    f.end();
    f.delete();
    assert_eq!(f.text(), "ad", "nothing after the end");
    f.backspace();
    f.backspace();
    f.backspace();
    assert_eq!((f.text(), f.cursor(), f.is_empty()), ("", 0, true));
}

#[test]
fn editing_is_by_character_not_by_byte() {
    let mut f = TextField::new(100);
    f.insert_str("Игры\u{1f3ae}Ñ");
    assert_eq!((f.len(), f.cursor()), (6, 6));
    f.left();
    f.left();
    f.backspace();
    assert_eq!((f.text(), f.cursor()), ("Игр\u{1f3ae}Ñ", 3));
    f.delete();
    assert_eq!(f.text(), "ИгрÑ");
    f.home();
    f.delete();
    assert_eq!(f.text(), "грÑ");
}

#[test]
fn control_characters_are_not_typed_and_the_limit_cuts_what_does_not_fit() {
    let mut f = TextField::new(5);
    f.insert_str("a\nb\r\tc\u{7f}d\0");
    assert_eq!(f.text(), "abcd");
    f.insert_str("efgh");
    assert_eq!((f.text(), f.len()), ("abcde", 5));
    f.left();
    f.insert_str("X");
    assert_eq!(
        f.text(),
        "abcde",
        "a full field takes nothing, in the middle too"
    );
    f.backspace();
    f.insert_str("XY");
    assert_eq!(f.text(), "abcXe", "one place was free: X fits, Y does not");
}

#[test]
fn set_replaces_the_text_with_the_cursor_at_the_end() {
    let mut f = TextField::new(4);
    f.insert_str("zz");
    f.left();
    f.set("hello there");
    assert_eq!((f.text(), f.cursor()), ("hell", 4));
    f.set("");
    assert_eq!((f.text(), f.cursor()), ("", 0));
}

#[test]
fn a_zero_sized_field_takes_nothing() {
    let mut f = TextField::new(0);
    f.insert_str("abc");
    assert!(f.is_empty());
}

#[test]
fn the_field_shows_a_window_around_the_cursor() {
    assert_eq!(field_view("", 0, true), "|");
    assert_eq!(field_view("", 0, false), "");
    assert_eq!(field_view("abc", 1, true), "a|bc");
    assert_eq!(field_view("abc", 3, true), "abc|");
    assert_eq!(field_view("abc", 0, true), "|abc");
    assert_eq!(
        field_view("abc", 99, true),
        "abc|",
        "a cursor past the end is the end"
    );
    assert_eq!(field_view("abc", 1, false), "abc");

    let long: String = ('a'..='z').cycle().take(200).collect();
    let at_start = field_view(&long, 0, true);
    assert!(
        at_start.starts_with('|') && at_start.ends_with("..."),
        "{at_start}"
    );
    assert_eq!(at_start.chars().count(), 1 + FIELD_VISIBLE_CHARS + 3);
    let at_end = field_view(&long, 200, true);
    assert!(
        at_end.starts_with("...") && at_end.ends_with('|'),
        "{at_end}"
    );
    assert_eq!(at_end.chars().count(), 3 + FIELD_VISIBLE_CHARS + 1);
    let middle = field_view(&long, 100, true);
    assert!(
        middle.starts_with("...") && middle.ends_with("..."),
        "{middle}"
    );
    assert!(middle.contains('|'));
    assert_eq!(middle.chars().count(), 3 + FIELD_VISIBLE_CHARS + 1 + 3);
    // The window keeps forty characters before the cursor and twenty after it.
    let chars: Vec<char> = long.chars().collect();
    let at = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    assert_eq!(
        field_view(&long, 100, true),
        format!("...{}|{}...", at(60, 100), at(100, 120))
    );
    assert!(
        !field_view(&long, 40, true).starts_with("..."),
        "the window starts at the start"
    );
    assert!(
        field_view(&long, 41, true).starts_with("..."),
        "one character in, the dots show"
    );
    // Wherever the cursor is, it is inside the window.
    for cursor in 0..=200 {
        assert_eq!(
            field_view(&long, cursor, true).matches('|').count(),
            1,
            "{cursor}"
        );
    }
    // Exactly the visible size shows whole, without dots.
    let exact: String = "x".repeat(FIELD_VISIBLE_CHARS);
    assert_eq!(field_view(&exact, 0, false), exact);
    let one_more: String = "x".repeat(FIELD_VISIBLE_CHARS + 1);
    assert!(field_view(&one_more, 0, false).ends_with("..."));
}

#[test]
fn a_long_path_loses_its_middle_and_a_short_one_is_left_alone() {
    assert_eq!(shorten_middle("abc", 10), "abc");
    assert_eq!(shorten_middle("abcdefghij", 10), "abcdefghij");
    assert_eq!(shorten_middle("abcdefghijk", 10), "abcd...ijk");
    let long = format!("C:\\{}\\Serious Sam 2", "x".repeat(300));
    let s = shorten_middle(&long, MAX_SHOWN_PATH_CHARS);
    assert_eq!(s.chars().count(), MAX_SHOWN_PATH_CHARS);
    assert!(s.starts_with("C:\\xxx"), "{s}");
    assert!(s.ends_with("\\Serious Sam 2"), "{s}");
    assert_eq!(shorten_middle("Игры и Ñandú", 8).chars().count(), 8);
    assert_eq!(
        shorten_middle("abcdef", 2),
        "...",
        "a limit smaller than the dots still gives something short"
    );
}

#[test]
fn wrapping_breaks_at_spaces_keeps_the_indent_and_splits_a_word_that_is_too_long() {
    assert_eq!(wrap("short", 20), ["short"]);
    assert_eq!(wrap("", 20), [""]);
    assert_eq!(wrap("aaa bbb ccc", 8), ["aaa bbb", "ccc"]);
    assert_eq!(wrap("aaa bbb ccc", 7), ["aaa bbb", "ccc"]);
    assert_eq!(
        wrap("aaa bbb", 7),
        ["aaa bbb"],
        "a line may be exactly the width"
    );
    assert_eq!(wrap("aaa bbb ccc ddd", 8), ["aaa bbb", "ccc ddd"]);
    assert_eq!(
        wrap("aaaa bbb", 8),
        ["aaaa bbb"],
        "a line may be exactly the width"
    );
    assert_eq!(wrap("aaaa bbbb", 8), ["aaaa", "bbbb"]);
    assert_eq!(
        wrap("  -> Reinstall the game or copy the file back", 22),
        [
            "  -> Reinstall the",
            "     game or copy the",
            "     file back"
        ]
    );
    assert_eq!(
        wrap("  note one two three", 12),
        ["  note one", "  two three"]
    );
    let lines = wrap(&"x".repeat(25), 10);
    assert_eq!(lines, ["xxxxxxxxxx", "xxxxxxxxxx", "xxxxx"]);
    let path = format!("ERROR: C:\\{} is bad", "folder\\".repeat(20));
    for line in wrap(&path, 30) {
        assert!(line.chars().count() <= 30, "{line:?}");
    }
    assert_eq!(wrap("a  b", 20), ["a b"], "runs of spaces are one space");
    assert!(
        wrap("anything at all", 0)
            .iter()
            .all(|l| l.chars().count() <= 8),
        "a tiny width is raised to 8"
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The flow

#[test]
fn the_wizard_starts_by_asking_for_a_search_and_ignores_input_while_it_waits() {
    let mut w = DataWizard::new(Some("why".into()));
    assert_eq!(w.view(), WizardView::Searching);
    assert_eq!(w.start(), WizardCommand::Search);
    for input in [
        WizardInput::Ui(UiInput::Nav(NavInput::Activate)),
        WizardInput::Ui(UiInput::Nav(NavInput::Back)),
        WizardInput::Ui(UiInput::PointerPressed(Vec2 { x: 960.0, y: 500.0 })),
        WizardInput::Text("x".into()),
        WizardInput::Backspace,
    ] {
        assert_eq!(w.handle(input), None);
        assert_eq!(w.view(), WizardView::Searching);
    }
    assert!(w.field().is_empty());
}

#[test]
fn answers_that_were_not_asked_for_are_ignored() {
    let mut w = found(2);
    w.set_report(report(true, 0, &[]));
    assert_eq!(w.view(), WizardView::Candidates, "no check is running");
    assert_eq!(w.report(), None);
    w.set_candidates(candidates(7));
    assert_eq!(w.candidates().len(), 2, "no search is running");
    let mut v = result(true);
    v.set_candidates(candidates(1));
    assert_eq!(v.view(), WizardView::Result);
    v.set_report(report(false, 0, &[]));
    assert!(v.report().unwrap().usable, "no check is running");
}

#[test]
fn no_candidates_leaves_three_rows_and_the_first_is_typing_a_path() {
    let mut w = found(0);
    assert!(w.candidates().is_empty());
    assert_eq!(w.row_rects().len(), 3);
    assert_eq!(nav(&mut w, NavInput::Activate), None);
    assert_eq!(w.view(), WizardView::Manual);
}

#[test]
fn a_candidate_is_checked_when_chosen_and_the_wizard_waits_for_the_report() {
    let mut w = found(3);
    assert_eq!(nav(&mut w, NavInput::Down), None);
    assert_eq!(w.focus(), 1);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from("/games/sam1")))
    );
    assert_eq!(w.view(), WizardView::Validating);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        None,
        "nothing to press while it checks"
    );
    assert_eq!(nav(&mut w, NavInput::Back), None, "and no way out of it");
    assert_eq!(w.view(), WizardView::Validating);
}

#[test]
fn the_list_wraps_in_both_directions_over_candidates_and_the_three_actions() {
    let mut w = found(2);
    for _ in 0..5 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(w.focus(), 0, "five rows, five steps down");
    nav(&mut w, NavInput::Up);
    assert_eq!(w.focus(), 4);
    nav(&mut w, NavInput::Left);
    nav(&mut w, NavInput::Right);
    assert_eq!(w.focus(), 4, "left and right do nothing in the list");
}

#[test]
fn the_actions_are_enter_a_path_search_again_and_quit() {
    let mut w = found(2);
    nav(&mut w, NavInput::Down);
    nav(&mut w, NavInput::Down);
    assert_eq!(nav(&mut w, NavInput::Activate), None);
    assert_eq!(w.view(), WizardView::Manual);

    let mut w = found(2);
    for _ in 0..3 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(nav(&mut w, NavInput::Activate), Some(WizardCommand::Search));
    assert_eq!(w.view(), WizardView::Searching);
    w.set_candidates(candidates(1));
    assert_eq!(
        w.candidates().len(),
        1,
        "the new answer replaces the old list"
    );
    assert_eq!(w.focus(), 0);

    let mut w = found(2);
    nav(&mut w, NavInput::Up);
    assert_eq!(nav(&mut w, NavInput::Activate), Some(WizardCommand::Quit));
}

#[test]
fn back_at_the_list_first_points_at_quit_and_only_a_second_back_quits() {
    let mut w = found(3);
    assert_eq!(nav(&mut w, NavInput::Back), None);
    assert_eq!(
        w.focus(),
        5,
        "the Quit row of three candidates and three actions"
    );
    assert_eq!(w.view(), WizardView::Candidates);
    assert_eq!(nav(&mut w, NavInput::Back), Some(WizardCommand::Quit));
    let mut w = found(0);
    assert_eq!(nav(&mut w, NavInput::Back), None);
    assert_eq!(w.focus(), 2);
    assert_eq!(nav(&mut w, NavInput::Back), Some(WizardCommand::Quit));
}

fn visible(w: &DataWizard, row: usize) -> bool {
    w.row_rects().iter().any(|(i, _)| *i == row)
}

#[test]
fn many_candidates_scroll_with_the_focus_and_keep_it_in_view() {
    let mut w = found(12);
    let shown = |w: &DataWizard| w.row_rects().into_iter().filter(|(i, _)| *i < 12).count();
    assert_eq!(shown(&w), VISIBLE_CANDIDATES);
    for _ in 0..VISIBLE_CANDIDATES - 1 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(w.focus(), 4);
    assert!(visible(&w, 4));
    assert!(!visible(&w, 5));
    nav(&mut w, NavInput::Down);
    assert!(visible(&w, 5), "the sixth scrolls in");
    assert!(!visible(&w, 0), "and the first scrolls out");
    for _ in 0..6 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(w.focus(), 11);
    assert!(visible(&w, 11));
    nav(&mut w, NavInput::Down);
    assert_eq!(w.focus(), 12, "the first action");
    assert_eq!(shown(&w), VISIBLE_CANDIDATES, "the window stays");
    for _ in 0..3 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(w.focus(), 0, "wrapped to the top");
    assert!(visible(&w, 0), "and scrolled back");
    nav(&mut w, NavInput::Up);
    assert_eq!(w.focus(), 14);
    for _ in 0..3 {
        nav(&mut w, NavInput::Up);
    }
    assert_eq!(w.focus(), 11);
    assert!(visible(&w, 11));
    for _ in 0..5 {
        nav(&mut w, NavInput::Up);
    }
    assert_eq!(w.focus(), 6);
    nav(&mut w, NavInput::Up);
    assert_eq!(w.focus(), 5);
    nav(&mut w, NavInput::Up);
    assert_eq!(w.focus(), 4);
    assert!(visible(&w, 4));
    // Choosing a scrolled candidate checks the right folder.
    for _ in 0..7 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from("/games/sam11")))
    );
}

#[test]
fn rows_on_screen_never_overlap_or_leave_the_canvas_even_with_the_most_content() {
    let views = [
        found(0),
        found(1),
        found(5),
        found(40),
        manual(),
        result(true),
        result(false),
    ];
    for w in &views {
        let rects = w.row_rects();
        for (i, (_, a)) in rects.iter().enumerate() {
            assert!(
                a.x >= 0.0 && a.y >= 0.0 && a.x + a.w <= CANVAS_W && a.y + a.h <= CANVAS_H,
                "{a:?}"
            );
            for (_, b) in &rects[i + 1..] {
                let apart =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
        let list = w.draw();
        for q in &list.quads {
            let r = q.rect;
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= CANVAS_W && r.y + r.h <= CANVAS_H,
                "{r:?}"
            );
        }
        for t in &list.texts {
            assert!(
                t.pos.x >= 0.0
                    && t.pos.x <= CANVAS_W
                    && t.pos.y >= 0.0
                    && t.pos.y + t.size <= CANVAS_H,
                "{t:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The result

#[test]
fn a_usable_result_offers_use_choose_another_and_quit() {
    let mut w = result(true);
    assert_eq!(w.row_rects().len(), 3);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Accept(PathBuf::from("/games/sam0")))
    );
    assert!(w.is_finished());
    assert_eq!(w.view(), WizardView::Done);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        None,
        "a folder is accepted once"
    );
    assert_eq!(nav(&mut w, NavInput::Back), None);

    let mut w = result(true);
    nav(&mut w, NavInput::Down);
    assert_eq!(nav(&mut w, NavInput::Activate), None);
    assert_eq!(
        w.view(),
        WizardView::Candidates,
        "back to where the folder came from"
    );
    assert_eq!(w.candidates().len(), 2, "the list is still there");

    let mut w = result(true);
    nav(&mut w, NavInput::Down);
    nav(&mut w, NavInput::Down);
    assert_eq!(nav(&mut w, NavInput::Activate), Some(WizardCommand::Quit));
}

#[test]
fn an_unusable_result_cannot_be_accepted() {
    let mut w = result(false);
    assert_eq!(w.row_rects().len(), 2, "no 'Use this folder' button");
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        None,
        "the first button goes back"
    );
    assert_eq!(w.view(), WizardView::Candidates);
    let mut w = result(false);
    nav(&mut w, NavInput::Down);
    assert_eq!(nav(&mut w, NavInput::Activate), Some(WizardCommand::Quit));
    let mut w = result(false);
    for _ in 0..4 {
        nav(&mut w, NavInput::Down);
    }
    assert_eq!(w.focus(), 0);
    assert!(!w.is_finished());
}

#[test]
fn back_on_a_result_returns_to_the_screen_it_came_from_with_the_text_kept() {
    let mut w = result(true);
    assert_eq!(nav(&mut w, NavInput::Back), None);
    assert_eq!(w.view(), WizardView::Candidates);

    let mut w = manual();
    typed(&mut w, "/some/where");
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from("/some/where")))
    );
    w.set_report(report(false, 0, &["ERROR: nope"]));
    assert_eq!(nav(&mut w, NavInput::Back), None);
    assert_eq!(w.view(), WizardView::Manual);
    assert_eq!(
        w.field().text(),
        "/some/where",
        "the player can fix a typo, not retype everything"
    );
    assert_eq!(w.field().cursor(), 11);
    assert_eq!(w.focus(), 0);
}

#[test]
fn the_next_check_forgets_the_old_report() {
    let mut w = result(true);
    nav(&mut w, NavInput::Down);
    nav(&mut w, NavInput::Activate);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from("/games/sam0")))
    );
    assert_eq!(w.report(), None, "no old report while the new check runs");
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Typing a path

#[test]
fn the_manual_screen_starts_with_the_field_focused_and_empty() {
    let w = manual();
    assert_eq!(w.focus(), 0);
    assert!(w.field().is_empty());
    assert_eq!(w.row_rects().len(), 3);
}

#[test]
fn typing_and_enter_check_the_path() {
    let mut w = manual();
    typed(&mut w, "C:\\Games\\Serious Sam 2");
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from(
            "C:\\Games\\Serious Sam 2"
        )))
    );
    assert_eq!(w.view(), WizardView::Validating);
}

#[test]
fn an_empty_or_blank_field_gives_a_notice_not_a_check() {
    let mut w = manual();
    assert_eq!(nav(&mut w, NavInput::Activate), None);
    assert_eq!(w.view(), WizardView::Manual);
    assert_eq!(w.notice(), Some("Type or paste the folder path first"));
    typed(&mut w, "   ");
    assert_eq!(w.notice(), None, "typing clears the notice");
    assert_eq!(nav(&mut w, NavInput::Activate), None);
    assert!(w.notice().is_some());
    typed(&mut w, "\"\"");
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        None,
        "a pair of quotes around nothing is nothing"
    );
}

#[test]
fn surrounding_spaces_and_one_pair_of_quotes_are_not_part_of_the_path() {
    for (text, want) in [
        ("  /a/b  ", "/a/b"),
        (
            "\"C:\\Program Files (x86)\\Serious Sam 2\"",
            "C:\\Program Files (x86)\\Serious Sam 2",
        ),
        ("  \" /a b/c \" ", "/a b/c"),
        ("/a/\"b\"", "/a/\"b\""),
        ("\"/half", "\"/half"),
        ("/dir with spaces/and Игры", "/dir with spaces/and Игры"),
    ] {
        let mut w = manual();
        typed(&mut w, text);
        assert_eq!(w.typed_path(), Some(PathBuf::from(want)), "{text:?}");
    }
    assert_eq!(manual().typed_path(), None);
}

#[test]
fn a_pasted_line_with_a_new_line_at_the_end_is_the_path() {
    let mut w = manual();
    typed(&mut w, "/a/b\r\n");
    assert_eq!(w.field().text(), "/a/b");
}

#[test]
fn a_very_long_path_is_cut_at_the_limit_and_still_checked() {
    let mut w = manual();
    typed(&mut w, &"a".repeat(MAX_PATH_CHARS + 500));
    assert_eq!(w.field().len(), MAX_PATH_CHARS);
    let Some(WizardCommand::Validate(p)) = nav(&mut w, NavInput::Activate) else {
        panic!("expected a check");
    };
    assert_eq!(p.to_string_lossy().chars().count(), MAX_PATH_CHARS);
}

#[test]
fn left_right_home_end_backspace_and_delete_edit_the_field_while_it_has_focus() {
    let mut w = manual();
    typed(&mut w, "abcd");
    nav(&mut w, NavInput::Left);
    nav(&mut w, NavInput::Left);
    assert_eq!(w.field().cursor(), 2);
    w.handle(WizardInput::Backspace);
    assert_eq!(w.field().text(), "acd");
    w.handle(WizardInput::Delete);
    assert_eq!(w.field().text(), "ad");
    w.handle(WizardInput::Home);
    assert_eq!(w.field().cursor(), 0);
    w.handle(WizardInput::End);
    assert_eq!(w.field().cursor(), 2);
    nav(&mut w, NavInput::Right);
    assert_eq!(w.field().cursor(), 2);
    nav(&mut w, NavInput::Left);
    typed(&mut w, "X");
    assert_eq!(w.field().text(), "aXd");
}

#[test]
fn edits_do_nothing_while_a_button_has_the_focus_except_that_typing_returns_to_the_field() {
    let mut w = manual();
    typed(&mut w, "abc");
    w.handle(WizardInput::Home);
    nav(&mut w, NavInput::Right);
    assert_eq!(w.field().cursor(), 1, "the cursor is in the middle");
    nav(&mut w, NavInput::Down);
    assert_eq!(w.focus(), 1);
    for input in [
        WizardInput::Backspace,
        WizardInput::Delete,
        WizardInput::Home,
        WizardInput::End,
    ] {
        w.handle(input);
    }
    nav(&mut w, NavInput::Left);
    nav(&mut w, NavInput::Right);
    assert_eq!(
        (w.field().text(), w.field().cursor()),
        ("abc", 1),
        "nothing changed"
    );
    assert_eq!(w.focus(), 1);
    typed(&mut w, "d");
    assert_eq!(w.field().text(), "adbc");
    assert_eq!(w.focus(), 0, "typing puts the focus on the field");
}

#[test]
fn text_events_outside_the_manual_screen_do_nothing() {
    let mut w = found(2);
    typed(&mut w, "abc");
    w.handle(WizardInput::Backspace);
    assert!(w.field().is_empty());
    let mut r = result(true);
    typed(&mut r, "abc");
    assert!(r.field().is_empty());
}

#[test]
fn the_manual_screen_has_three_rows_that_wrap_and_back_returns_to_the_list() {
    let mut w = manual();
    nav(&mut w, NavInput::Up);
    assert_eq!(w.focus(), 2);
    nav(&mut w, NavInput::Down);
    assert_eq!(w.focus(), 0);
    nav(&mut w, NavInput::Down);
    nav(&mut w, NavInput::Down);
    assert_eq!(nav(&mut w, NavInput::Activate), None, "the Back button");
    assert_eq!(w.view(), WizardView::Candidates);
    assert_eq!(w.focus(), 0);

    let mut w = manual();
    assert_eq!(
        nav(&mut w, NavInput::Back),
        None,
        "Escape goes back and never quits from here"
    );
    assert_eq!(w.view(), WizardView::Candidates);
}

#[test]
fn the_check_button_checks_like_enter() {
    let mut w = manual();
    typed(&mut w, "/x");
    nav(&mut w, NavInput::Down);
    assert_eq!(
        nav(&mut w, NavInput::Activate),
        Some(WizardCommand::Validate(PathBuf::from("/x")))
    );
}

#[test]
fn the_text_survives_a_trip_to_the_list_and_back() {
    let mut w = manual();
    typed(&mut w, "/keep/me");
    w.handle(WizardInput::Home);
    nav(&mut w, NavInput::Back);
    nav(&mut w, NavInput::Activate);
    assert_eq!(w.view(), WizardView::Manual);
    assert_eq!(w.field().text(), "/keep/me");
    assert_eq!(w.field().cursor(), 8, "the cursor is at the end");
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The pointer

#[test]
fn hovering_focuses_a_row() {
    let mut w = found(3);
    let p = center_of(rect_of(&w, 2));
    assert_eq!(w.handle(WizardInput::Ui(UiInput::PointerMoved(p))), None);
    assert_eq!(w.focus(), 2);
    w.handle(WizardInput::Ui(UiInput::PointerMoved(Vec2 {
        x: 5.0,
        y: 5.0,
    })));
    assert_eq!(w.focus(), 2, "moving over nothing keeps the focus");
}

#[test]
fn a_click_activates_only_when_released_on_the_row_it_was_pressed_on() {
    let mut w = found(3);
    let a = center_of(rect_of(&w, 1));
    let b = center_of(rect_of(&w, 2));
    assert_eq!(
        click(&mut w, a),
        Some(WizardCommand::Validate(PathBuf::from("/games/sam1")))
    );
    let mut w = found(3);
    w.handle(WizardInput::Ui(UiInput::PointerPressed(a)));
    assert_eq!(
        w.focus(),
        1,
        "a press focuses, with no hover before it (a touch screen)"
    );
    assert_eq!(w.handle(WizardInput::Ui(UiInput::PointerReleased(b))), None);
    assert_eq!(w.view(), WizardView::Candidates);
    assert_eq!(
        w.handle(WizardInput::Ui(UiInput::PointerReleased(a))),
        None,
        "the press was used up by the release before"
    );
    w.handle(WizardInput::Ui(UiInput::PointerPressed(Vec2 {
        x: 3.0,
        y: 3.0,
    })));
    assert_eq!(
        w.handle(WizardInput::Ui(UiInput::PointerReleased(a))),
        None,
        "pressed outside, released inside"
    );
}

#[test]
fn clicking_the_field_focuses_it_and_does_not_check_and_the_buttons_work() {
    let mut w = manual();
    typed(&mut w, "/x");
    nav(&mut w, NavInput::Down);
    let field = center_of(rect_of(&w, 0));
    assert_eq!(click(&mut w, field), None);
    assert_eq!(w.focus(), 0);
    assert_eq!(w.view(), WizardView::Manual);
    let check = center_of(rect_of(&w, 1));
    assert_eq!(
        click(&mut w, check),
        Some(WizardCommand::Validate(PathBuf::from("/x")))
    );

    let mut w = manual();
    let back = center_of(rect_of(&w, 2));
    assert_eq!(click(&mut w, back), None);
    assert_eq!(w.view(), WizardView::Candidates);
}

#[test]
fn the_pointer_works_on_the_result_buttons_and_does_nothing_while_waiting() {
    let mut w = result(true);
    let use_it = center_of(rect_of(&w, 0));
    assert_eq!(
        click(&mut w, use_it),
        Some(WizardCommand::Accept(PathBuf::from("/games/sam0")))
    );
    assert!(w.is_finished());
    let mut w = DataWizard::new(None);
    w.start();
    assert_eq!(click(&mut w, Vec2 { x: 960.0, y: 500.0 }), None);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Drawing

#[test]
fn every_screen_draws_the_title_and_replaces_what_was_in_the_list() {
    let mut list = DrawList::default();
    list.texts.push(crate::draw::TextRun {
        pos: Vec2::default(),
        text: "stale".into(),
        size: 1.0,
        color: theme::TEXT,
        align: crate::draw::TextAlign::Left,
    });
    for w in [
        DataWizard::new(None),
        found(0),
        found(3),
        manual(),
        result(true),
        result(false),
    ] {
        w.draw_into(&mut list);
        assert!(
            texts(&list).iter().any(|t| t == "Find Serious Sam 2"),
            "{:?}",
            w.view()
        );
        assert!(!texts(&list).iter().any(|t| t == "stale"));
    }
}

#[test]
fn the_message_is_shown_and_a_notice_takes_its_place() {
    let mut w = DataWizard::new(Some("The folder you chose before no longer works".into()));
    w.start();
    assert!(
        texts(&w.draw())
            .iter()
            .any(|t| t.contains("no longer works"))
    );
    w.set_notice("Could not save");
    let t = texts(&w.draw());
    assert!(t.iter().any(|t| t == "Could not save"));
    assert!(!t.iter().any(|t| t.contains("no longer works")));
    let w = DataWizard::new(None);
    assert_eq!(
        w.draw().texts.len(),
        3,
        "title and the two lines of the searching screen"
    );
}

#[test]
fn the_reason_for_the_wizard_is_not_drawn_over_a_result() {
    let mut w = DataWizard::new(Some("because of the reason".into()));
    w.start();
    w.set_candidates(candidates(1));
    assert!(
        texts(&w.draw())
            .iter()
            .any(|t| t == "because of the reason")
    );
    nav(&mut w, NavInput::Activate);
    w.set_report(report(true, 0, &["Archives: 14 found, 14 readable"]));
    assert!(
        !texts(&w.draw())
            .iter()
            .any(|t| t == "because of the reason")
    );
    w.set_notice("a notice still shows");
    assert!(texts(&w.draw()).iter().any(|t| t == "a notice still shows"));
}

#[test]
fn one_warning_already_tints_the_headline_and_starting_again_drops_a_notice() {
    let mut w = found(1);
    nav(&mut w, NavInput::Activate);
    w.set_report(report(true, 1, &["WARNING: one"]));
    let list = w.draw();
    let head = list
        .texts
        .iter()
        .find(|t| t.text == "Serious Sam 2 found")
        .unwrap();
    let line = list
        .texts
        .iter()
        .find(|t| t.text.starts_with("WARNING"))
        .unwrap();
    assert_eq!(
        head.color, line.color,
        "one warning is enough for the warning colour"
    );
    w.set_notice("old news");
    assert_eq!(w.start(), WizardCommand::Search);
    assert_eq!(w.notice(), None);
}

#[test]
fn more_lines_than_fit_are_counted_exactly() {
    let mut w = found(1);
    nav(&mut w, NavInput::Activate);
    let lines: Vec<String> = (0..12).map(|i| format!("line {i}")).collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    w.set_report(report(true, 0, &lines));
    let list = w.draw();
    let body: Vec<_> = list.texts.iter().filter(|t| t.pos.x == 210.0).collect();
    assert_eq!(body.len(), MAX_REPORT_LINES);
    assert_eq!(body[7].text, "line 7", "eight lines are shown");
    assert!(
        body[8].text.starts_with("... and 4 more lines"),
        "{}",
        body[8].text
    );
}

#[test]
fn the_searching_and_checking_screens_say_what_they_wait_for() {
    let w = DataWizard::new(None);
    assert!(texts(&w.draw()).iter().any(|t| t.contains("Looking for")));
    let mut w = found(1);
    nav(&mut w, NavInput::Activate);
    let t = texts(&w.draw());
    assert!(t.iter().any(|t| t.contains("Checking")), "{t:?}");
    assert!(
        t.iter().any(|t| t.contains("/games/sam0")),
        "the folder being checked: {t:?}"
    );
    assert!(w.draw().quads.is_empty(), "no buttons to press");
}

#[test]
fn the_list_shows_each_folder_with_its_detail_and_the_actions() {
    let w = found(3);
    let list = w.draw();
    let t = texts(&list);
    for i in 0..3 {
        assert!(t.contains(&format!("/games/sam{i}")), "{t:?}");
        assert!(t.iter().any(|x| x.contains(&format!("(/games/sam{i})"))));
    }
    for a in ["Enter a folder path", "Search again", "Quit"] {
        assert!(t.contains(&a.to_owned()), "{t:?}");
    }
    assert!(t.iter().any(|x| x == "Found 3 folders. Choose one:"));
    assert_eq!(list.quads.len(), 6);
    let focused: Vec<_> = list
        .quads
        .iter()
        .filter(|q| q.color == theme::FOCUSED)
        .collect();
    assert_eq!(focused.len(), 1, "exactly one row is focused");
    assert_eq!(focused[0].rect, rect_of(&w, 0));
    assert!(
        texts(&found(1).draw())
            .iter()
            .any(|x| x.starts_with("Found one folder"))
    );
    assert!(
        texts(&found(12).draw())
            .iter()
            .any(|x| x == "Found 12 folders (showing 1-5). Choose one:")
    );
}

#[test]
fn an_empty_list_says_so_and_still_offers_the_actions() {
    let t = texts(&found(0).draw());
    assert!(t.iter().any(|x| x.starts_with("Nothing found")), "{t:?}");
    assert!(t.iter().any(|x| x == "Enter a folder path"));
}

#[test]
fn a_long_path_in_the_list_is_shortened_and_a_short_one_is_not() {
    let mut w = DataWizard::new(None);
    w.start();
    let long = format!("C:\\{}\\Serious Sam 2", "x".repeat(400));
    w.set_candidates(vec![WizardCandidate {
        dir: PathBuf::from(&long),
        detail: "d".into(),
    }]);
    let t = texts(&w.draw());
    let shown = t.iter().find(|x| x.starts_with("C:\\")).unwrap();
    assert!(shown.chars().count() <= MAX_SHOWN_PATH_CHARS, "{shown}");
    assert!(
        shown.contains("...") && shown.ends_with("Serious Sam 2"),
        "{shown}"
    );
}

#[test]
fn the_field_draws_a_cursor_only_while_focused_and_a_window_of_a_long_path() {
    let mut w = manual();
    typed(&mut w, "abc");
    nav(&mut w, NavInput::Left);
    assert!(texts(&w.draw()).iter().any(|t| t == "ab|c"));
    nav(&mut w, NavInput::Down);
    assert!(
        texts(&w.draw()).iter().any(|t| t == "abc"),
        "no cursor on a button"
    );
    let mut w = manual();
    typed(&mut w, &"y".repeat(300));
    let t = texts(&w.draw());
    let shown = t.iter().find(|x| x.contains('|')).unwrap();
    assert!(shown.starts_with("...") && shown.ends_with('|'), "{shown}");
    assert!(shown.chars().count() <= FIELD_VISIBLE_CHARS + 5);
}

#[test]
fn the_manual_screen_draws_the_label_the_hint_and_two_buttons() {
    let list = manual().draw();
    let t = texts(&list);
    assert!(t.iter().any(|x| x.contains("All_PC_01.gro")));
    assert!(t.iter().any(|x| x.starts_with("Paste a path")));
    assert!(t.contains(&"Check this folder".to_owned()));
    assert!(t.contains(&"Back".to_owned()));
    // The field has a highlight ring while focused, then the field, a button and a button.
    assert_eq!(list.quads.len(), 4);
}

#[test]
fn the_result_shows_the_headline_the_lines_and_colours_them_by_severity() {
    let mut w = found(1);
    nav(&mut w, NavInput::Activate);
    w.set_report(WizardReport {
        dir: PathBuf::from("/games/sam0"),
        usable: true,
        warnings: 2,
        headline: "Usable, with 2 warnings".into(),
        lines: vec![
            "Archives: 5 found, 5 readable".into(),
            "WARNING: All_PC_01.gro is odd".into(),
            "  -> Reinstall".into(),
            "ERROR: pretend".into(),
        ],
    });
    let list = w.draw();
    let head = list
        .texts
        .iter()
        .find(|t| t.text == "Usable, with 2 warnings")
        .unwrap();
    let plain = list
        .texts
        .iter()
        .find(|t| t.text.starts_with("Archives"))
        .unwrap();
    let warn = list
        .texts
        .iter()
        .find(|t| t.text.starts_with("WARNING"))
        .unwrap();
    let err = list
        .texts
        .iter()
        .find(|t| t.text.starts_with("ERROR"))
        .unwrap();
    assert_eq!(plain.color, theme::TEXT);
    assert_ne!(warn.color, theme::TEXT);
    assert_ne!(err.color, theme::TEXT);
    assert_ne!(err.color, warn.color);
    assert_eq!(
        head.color, warn.color,
        "warnings tint the headline like a warning line"
    );
    let ok = result(true).draw();
    let ok_head = ok
        .texts
        .iter()
        .find(|t| t.text == "Serious Sam 2 found")
        .unwrap();
    assert_ne!(ok_head.color, head.color);
    let bad = result(false).draw();
    let bad_head = bad
        .texts
        .iter()
        .find(|t| t.text == "This folder cannot be used")
        .unwrap();
    assert_eq!(bad_head.color, err.color);
    assert_ne!(bad_head.color, ok_head.color);
    let ys: Vec<f32> = list
        .texts
        .iter()
        .filter(|t| t.pos.x == 210.0)
        .map(|t| t.pos.y)
        .collect();
    assert_eq!(ys.len(), 4);
    assert!(
        ys.windows(2).all(|p| p[1] > p[0]),
        "lines go down the screen: {ys:?}"
    );
}

#[test]
fn long_report_lines_wrap_and_too_many_lines_end_with_a_count() {
    let mut w = found(1);
    nav(&mut w, NavInput::Activate);
    let lines: Vec<String> = (0..30)
        .map(|i| format!("line {i} {}", "word ".repeat(30)))
        .collect();
    w.set_report(WizardReport {
        dir: PathBuf::from("/games/sam0"),
        usable: false,
        warnings: 0,
        headline: "h".into(),
        lines,
    });
    let list = w.draw();
    let body: Vec<_> = list.texts.iter().filter(|t| t.pos.x == 210.0).collect();
    assert_eq!(body.len(), MAX_REPORT_LINES);
    assert!(
        body.last().unwrap().text.starts_with("... and "),
        "{:?}",
        body.last()
    );
    assert!(body.last().unwrap().text.contains("more lines"));
    for t in &body[..body.len() - 1] {
        assert!(t.text.chars().count() <= WRAP_CHARS, "{}", t.text);
    }
    // Exactly the limit shows every line without the count.
    let mut w = found(1);
    nav(&mut w, NavInput::Activate);
    w.set_report(report(
        true,
        0,
        &["a", "b", "c", "d", "e", "f", "g", "h", "i"],
    ));
    let list = w.draw();
    let body: Vec<_> = list.texts.iter().filter(|t| t.pos.x == 210.0).collect();
    assert_eq!(body.len(), 9);
    assert_eq!(body[8].text, "i");
}

#[test]
fn the_focused_result_button_is_orange_and_the_labels_depend_on_the_verdict() {
    let ok = result(true);
    let t = texts(&ok.draw());
    for b in ["Use this folder", "Choose another folder", "Quit"] {
        assert!(t.contains(&b.to_owned()), "{t:?}");
    }
    let bad = result(false);
    let t = texts(&bad.draw());
    assert!(!t.contains(&"Use this folder".to_owned()));
    assert_eq!(
        bad.draw()
            .quads
            .iter()
            .filter(|q| q.color == theme::FOCUSED)
            .count(),
        1
    );
}

#[test]
fn the_finished_screen_says_the_choice_was_saved() {
    let mut w = result(true);
    nav(&mut w, NavInput::Activate);
    let t = texts(&w.draw());
    assert!(t.iter().any(|x| x.starts_with("Saved")), "{t:?}");
    assert!(w.draw().quads.is_empty());
}

#[test]
fn a_huge_amount_of_everything_does_not_panic_or_grow_the_screen() {
    let mut w = DataWizard::new(Some("m".repeat(10_000)));
    w.start();
    w.set_candidates(
        (0..500)
            .map(|i| WizardCandidate {
                dir: PathBuf::from("é".repeat(3000) + &i.to_string()),
                detail: "x".repeat(5000),
            })
            .collect(),
    );
    let list = w.draw();
    assert!(list.texts.len() < 40, "{}", list.texts.len());
    assert!(
        list.texts.iter().all(|t| t.text.chars().count() <= 130),
        "no text run is longer than a line"
    );
    for _ in 0..600 {
        nav(&mut w, NavInput::Down);
        let _ = w.draw();
    }
}
