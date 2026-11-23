//! Tests of the death and summary screens.

use ss2_core::input::NavInput;

use crate::draw::DrawList;
use crate::geometry::Vec2;
use crate::menu::UiInput;
use crate::session_screens::*;
use crate::theme;

fn texts(list: &DrawList) -> Vec<String> {
    list.texts.iter().map(|t| t.text.to_string()).collect()
}

fn nav(n: NavInput) -> UiInput {
    UiInput::Nav(n)
}

fn at(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

fn coop(ticks: Option<u32>) -> DeathScreen {
    DeathScreen::new(DeathMode::Coop {
        respawn_in_ticks: ticks,
        ticks_per_second: 60,
    })
}

// ---- death ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_solo_death_offers_retry_and_quit() {
    let mut s = DeathScreen::new(DeathMode::Solo);
    let t = texts(&s.draw());
    assert_eq!(
        t,
        [
            "You Died",
            "Retry from the last checkpoint",
            "Retry",
            "Quit to Menu"
        ]
    );
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::Retry),
        "the first button is focused"
    );
    assert_eq!(s.handle(nav(NavInput::Down)), None);
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::QuitToMenu)
    );
    assert_eq!(s.handle(nav(NavInput::Down)), None);
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::Retry),
        "down wraps"
    );
    assert_eq!(s.handle(nav(NavInput::Up)), None);
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::QuitToMenu),
        "up wraps"
    );
}

#[test]
fn back_does_not_dismiss_the_death_screen() {
    let mut s = DeathScreen::new(DeathMode::Solo);
    assert_eq!(s.handle(nav(NavInput::Back)), None);
    assert_eq!(s.handle(nav(NavInput::Left)), None);
    assert_eq!(s.handle(nav(NavInput::Right)), None);
}

#[test]
fn a_solo_screen_has_no_countdown_and_ignores_one() {
    let mut s = DeathScreen::new(DeathMode::Solo);
    s.set_respawn_in(Some(5));
    assert_eq!(s.mode(), DeathMode::Solo);
    assert_eq!(s.seconds_left(), None);
    for _ in 0..100 {
        assert_eq!(s.update(), None);
    }
}

#[test]
fn the_coop_countdown_shows_whole_seconds_rounded_up_and_respawns_once_at_zero() {
    let mut s = coop(Some(150));
    assert_eq!(s.seconds_left(), Some(3));
    assert!(texts(&s.draw()).contains(&"Respawning in 3".to_string()));
    let mut respawns = 0;
    let mut shown = Vec::new();
    for _ in 0..200 {
        if s.update() == Some(DeathCommand::Respawn) {
            respawns += 1;
        }
        shown.push(s.seconds_left().unwrap());
    }
    assert_eq!(respawns, 1, "once, not on every tick after zero");
    // shown[k - 1] is the display after k ticks, with 150 - k ticks left.
    assert_eq!(shown[0], 3, "149 ticks left is still 3 s");
    assert_eq!(shown[29], 2, "after 30 ticks 120 are left: exactly 2 s");
    assert_eq!(shown[88], 2, "after 89 ticks 61 are left: still 2 s");
    assert_eq!(shown[89], 1, "after 90 ticks 60 are left: 1 s");
    assert_eq!(shown[148], 1, "after 149 ticks 1 is left: 1 s");
    assert_eq!(shown[149], 0, "after 150 ticks the countdown has ended");
    assert_eq!(*shown.last().unwrap(), 0);
}

#[test]
fn an_unknown_wait_says_so_and_a_new_countdown_rearms_the_respawn() {
    let mut s = coop(None);
    assert_eq!(s.seconds_left(), None);
    assert!(texts(&s.draw()).contains(&"Waiting for your team".to_string()));
    for _ in 0..10 {
        assert_eq!(s.update(), None, "no countdown, no respawn");
    }
    s.set_respawn_in(Some(2));
    assert_eq!(s.update(), None);
    assert_eq!(s.update(), Some(DeathCommand::Respawn));
    assert_eq!(s.update(), None);
    s.set_respawn_in(Some(1));
    assert_eq!(
        s.update(),
        Some(DeathCommand::Respawn),
        "a new countdown can finish again"
    );
}

#[test]
fn the_coop_buttons_are_spectate_and_quit() {
    let mut s = coop(Some(600));
    let t = texts(&s.draw());
    assert!(
        t.contains(&"Spectate".to_string()) && t.contains(&"Quit to Menu".to_string()),
        "{t:?}"
    );
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::Spectate)
    );
    s.handle(nav(NavInput::Down));
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::QuitToMenu)
    );
}

#[test]
fn a_zero_tick_rate_does_not_divide_by_zero() {
    let s = DeathScreen::new(DeathMode::Coop {
        respawn_in_ticks: Some(10),
        ticks_per_second: 0,
    });
    assert_eq!(s.seconds_left(), Some(10));
}

#[test]
fn pointer_rules_are_the_menus() {
    let mut s = DeathScreen::new(DeathMode::Solo);
    // Buttons: 640 wide centred (x 640..1280), 72 high, 16 apart, the first at y 380.
    let first = at(960.0, 400.0);
    let second = at(960.0, 380.0 + 88.0 + 20.0);
    s.handle(UiInput::PointerMoved(second));
    assert_eq!(
        s.handle(UiInput::PointerReleased(second)),
        None,
        "released without a press"
    );
    assert_eq!(s.handle(UiInput::PointerPressed(second)), None);
    assert_eq!(
        s.handle(UiInput::PointerReleased(first)),
        None,
        "released over another button"
    );
    assert_eq!(s.handle(UiInput::PointerPressed(second)), None);
    assert_eq!(
        s.handle(UiInput::PointerReleased(second)),
        Some(DeathCommand::QuitToMenu)
    );
    assert_eq!(
        s.handle(UiInput::PointerPressed(at(100.0, 100.0))),
        None,
        "outside every button"
    );
    assert_eq!(s.handle(UiInput::PointerReleased(at(100.0, 100.0))), None);
    // Hover focuses: after moving over the second button, Activate gives the second.
    s.handle(UiInput::PointerMoved(second));
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::QuitToMenu)
    );
    s.handle(UiInput::PointerMoved(first));
    assert_eq!(s.handle(nav(NavInput::Activate)), Some(DeathCommand::Retry));
}

#[test]
fn pressing_a_button_focuses_it_even_without_hovering_first() {
    // A touch screen sends a press with no move before it.
    let mut s = DeathScreen::new(DeathMode::Solo);
    let second = at(960.0, 380.0 + 88.0 + 20.0);
    assert_eq!(s.handle(UiInput::PointerPressed(second)), None);
    let colors: Vec<_> = s.draw().quads.iter().map(|q| q.color).collect();
    assert_eq!(
        colors,
        [theme::IDLE, theme::FOCUSED],
        "the pressed button is the focused one"
    );
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(DeathCommand::QuitToMenu)
    );
}

#[test]
fn the_focused_button_is_orange_and_the_other_is_not() {
    let mut s = DeathScreen::new(DeathMode::Solo);
    let colors: Vec<_> = s.draw().quads.iter().map(|q| q.color).collect();
    assert_eq!(colors, [theme::FOCUSED, theme::IDLE]);
    s.handle(nav(NavInput::Down));
    let colors: Vec<_> = s.draw().quads.iter().map(|q| q.color).collect();
    assert_eq!(colors, [theme::IDLE, theme::FOCUSED]);
}

// ---- summary ---------------------------------------------------------------------------------------------------------------------

fn stats() -> LevelStats {
    LevelStats {
        level: "The Jungle".into(),
        kills: 12,
        kills_total: 20,
        secrets: 1,
        secrets_total: 3,
        elapsed_secs: 205,
        par_secs: None,
        score: None,
    }
}

fn finish(s: &mut SummaryScreen) {
    while s.counting() {
        s.update();
    }
}

#[test]
fn a_finished_summary_shows_the_numbers_and_the_button() {
    let mut s = SummaryScreen::new(stats());
    finish(&mut s);
    assert_eq!(
        texts(&s.draw()),
        [
            "Level Complete",
            "The Jungle",
            "Kills",
            "12 / 20  60%",
            "Secrets",
            "1 / 3  33%",
            "Time",
            "3:25",
            "Continue"
        ]
    );
}

#[test]
fn par_and_score_add_rows_and_lengthen_the_count() {
    let mut full = stats();
    full.par_secs = Some(300);
    full.score = Some(12_345);
    let mut s = SummaryScreen::new(full);
    assert_eq!(
        SummaryScreen::new(stats()).counting_ticks(),
        2 * ROW_DELAY_TICKS + COUNT_TICKS
    );
    assert_eq!(s.counting_ticks(), 4 * ROW_DELAY_TICKS + COUNT_TICKS);
    finish(&mut s);
    let t = texts(&s.draw());
    assert!(
        t.contains(&"Par".to_string()) && t.contains(&"5:00".to_string()),
        "{t:?}"
    );
    assert!(
        t.contains(&"Score".to_string()) && t.contains(&"12345".to_string()),
        "{t:?}"
    );
}

#[test]
fn the_rows_count_up_one_after_the_other() {
    let mut s = SummaryScreen::new(stats());
    let value = |s: &SummaryScreen, row: usize| texts(&s.draw())[3 + row * 2].clone();
    assert_eq!(value(&s, 0), "0 / 20  0%", "nothing at tick 0");
    for _ in 0..(COUNT_TICKS / 2) {
        s.update();
    }
    // 22 of 45 ticks: 12 * 22/45 = 5.87, shown as 5. The second row has not started.
    assert_eq!(value(&s, 0), "5 / 20  25%");
    assert_eq!(value(&s, 1), "0 / 3  0%");
    for _ in 0..ROW_DELAY_TICKS {
        s.update();
    }
    // 52 ticks: the second row started at tick 30 and is 22 of 45 through: 1 * 22/45 is shown as 0.
    assert_eq!(value(&s, 1), "0 / 3  0%");
    assert!(s.counting());
    finish(&mut s);
    assert_eq!(value(&s, 0), "12 / 20  60%");
    assert!(!s.counting());
    s.update();
    assert_eq!(
        value(&s, 0),
        "12 / 20  60%",
        "update after the end changes nothing"
    );
}

#[test]
fn counting_never_shows_more_than_the_final_value() {
    let mut s = SummaryScreen::new(stats());
    let mut last = 0u32;
    while s.counting() {
        s.update();
        let kills: u32 = texts(&s.draw())[3]
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(kills >= last && kills <= 12, "{kills} after {last}");
        last = kills;
    }
    assert_eq!(last, 12);
}

#[test]
fn a_press_while_counting_skips_and_the_next_one_continues() {
    let mut s = SummaryScreen::new(stats());
    s.update();
    assert!(s.counting());
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        None,
        "the first press only skips"
    );
    assert!(!s.counting());
    assert_eq!(texts(&s.draw())[3], "12 / 20  60%");
    assert_eq!(
        s.handle(nav(NavInput::Activate)),
        Some(SummaryCommand::Continue)
    );
}

#[test]
fn a_click_that_skips_does_not_also_continue_on_release() {
    let mut s = SummaryScreen::new(stats());
    let on_button = at(960.0, 890.0);
    assert_eq!(s.handle(UiInput::PointerPressed(on_button)), None, "skips");
    assert!(!s.counting());
    assert_eq!(
        s.handle(UiInput::PointerReleased(on_button)),
        None,
        "the press that skipped is not a click on Continue"
    );
    assert_eq!(s.handle(UiInput::PointerPressed(on_button)), None);
    assert_eq!(
        s.handle(UiInput::PointerReleased(on_button)),
        Some(SummaryCommand::Continue)
    );
}

#[test]
fn other_keys_do_not_skip_or_continue() {
    let mut s = SummaryScreen::new(stats());
    for n in [
        NavInput::Up,
        NavInput::Down,
        NavInput::Left,
        NavInput::Right,
        NavInput::Back,
    ] {
        assert_eq!(s.handle(nav(n)), None);
        assert!(s.counting(), "{n:?} does not skip");
    }
    finish(&mut s);
    for n in [
        NavInput::Up,
        NavInput::Down,
        NavInput::Left,
        NavInput::Right,
        NavInput::Back,
    ] {
        assert_eq!(s.handle(nav(n)), None, "{n:?} does not continue");
    }
}

#[test]
fn the_button_appears_only_when_the_count_is_done() {
    let mut s = SummaryScreen::new(stats());
    assert!(!texts(&s.draw()).contains(&"Continue".to_string()));
    let quads_while_counting = s.draw().quads.len();
    finish(&mut s);
    assert_eq!(s.draw().quads.len(), quads_while_counting + 1);
    assert!(texts(&s.draw()).contains(&"Continue".to_string()));
}

#[test]
fn a_level_with_nothing_to_find_does_not_divide_by_zero() {
    let mut s = SummaryScreen::new(LevelStats {
        kills_total: 0,
        kills: 0,
        secrets_total: 0,
        secrets: 0,
        ..stats()
    });
    finish(&mut s);
    let t = texts(&s.draw());
    assert_eq!(t[3], "0 / 0");
    assert_eq!(t[5], "0 / 0");
    // More kills than the level said (an enemy spawned by a script): the percentage is capped, not a wrap.
    let mut s = SummaryScreen::new(LevelStats {
        kills: 5_000_000,
        kills_total: 1,
        ..stats()
    });
    finish(&mut s);
    assert_eq!(texts(&s.draw())[3], "5000000 / 1  999%");
}

#[test]
fn the_clock_shows_minutes_and_seconds_and_hours_from_an_hour() {
    for (secs, text) in [
        (0, "0:00"),
        (5, "0:05"),
        (59, "0:59"),
        (60, "1:00"),
        (205, "3:25"),
        (3599, "59:59"),
        (3600, "1:00:00"),
        (7325, "2:02:05"),
    ] {
        assert_eq!(clock(secs), text);
    }
}

#[test]
fn drawing_into_a_list_replaces_what_it_held() {
    let mut list = DrawList::default();
    DeathScreen::new(DeathMode::Solo).draw_into(&mut list);
    let n = list.texts.len();
    DeathScreen::new(DeathMode::Solo).draw_into(&mut list);
    assert_eq!(list.texts.len(), n);
    let s = SummaryScreen::new(stats());
    s.draw_into(&mut list);
    assert_eq!(texts(&list)[0], "Level Complete");
    assert_eq!(
        list.texts.len(),
        2 + 3 * 2,
        "title, level, three rows of two, no button yet"
    );
}
