//! The About screen: the unofficial-fan-engine notice, the licence and the third-party credits
//! (roadmap T10-016).

use crate::menu::{Item, Menu, MenuCommand, ScreenId, UiInput, items};
use crate::{TextAlign, Vec2};
use ss2_core::input::NavInput;
use ss2_core::settings::Settings;

fn lines() -> Vec<&'static str> {
    items(ScreenId::About)
        .iter()
        .filter_map(|i| match i {
            Item::Text { line } => Some(*line),
            _ => None,
        })
        .collect()
}

fn open_about(m: &mut Menu) {
    // Main menu: New Game, Options, About, Quit.
    m.handle(UiInput::Nav(NavInput::Down));
    m.handle(UiInput::Nav(NavInput::Down));
    assert!(m.handle(UiInput::Nav(NavInput::Activate)).is_empty());
    assert_eq!(m.screen(), ScreenId::About);
}

fn all_text() -> String {
    lines().join("\n")
}

#[test]
fn the_main_menu_has_an_about_button_between_options_and_quit() {
    let labels: Vec<&str> = items(ScreenId::Main)
        .iter()
        .filter_map(|i| match i {
            Item::Button { label, .. } => Some(*label),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["New Game", "Options", "About", "Quit"]);
}

#[test]
fn about_opens_from_the_main_menu_and_back_returns_to_it() {
    let mut m = Menu::new(Settings::default());
    open_about(&mut m);
    assert_eq!(
        m.handle(UiInput::Nav(NavInput::Activate)),
        Vec::<MenuCommand>::new()
    );
    assert_eq!(m.screen(), ScreenId::Main, "the only button is Back");
    open_about(&mut m);
    m.handle(UiInput::Nav(NavInput::Back));
    assert_eq!(m.screen(), ScreenId::Main);
}

#[test]
fn only_the_back_button_can_take_focus_on_the_about_screen() {
    let mut m = Menu::new(Settings::default());
    open_about(&mut m);
    let list = items(ScreenId::About);
    let back = list
        .iter()
        .position(|i| matches!(i, Item::Button { label: "Back", .. }))
        .expect("a Back button");
    assert_eq!(m.focused(), Some(back));
    for nav in [NavInput::Up, NavInput::Down, NavInput::Down, NavInput::Up] {
        m.handle(UiInput::Nav(nav));
        assert_eq!(m.focused(), Some(back), "text lines never take focus");
    }
}

#[test]
fn clicking_a_text_line_does_nothing_and_clicking_back_leaves() {
    let mut m = Menu::new(Settings::default());
    open_about(&mut m);
    let list = items(ScreenId::About);
    let rects = m.item_rects();
    let first_text = list
        .iter()
        .position(|i| matches!(i, Item::Text { .. }))
        .unwrap();
    let r = rects[first_text];
    let p = Vec2 {
        x: r.x + r.w / 2.0,
        y: r.y + r.h / 2.0,
    };
    m.handle(UiInput::PointerPressed(p));
    assert_eq!(
        m.handle(UiInput::PointerReleased(p)),
        Vec::<MenuCommand>::new()
    );
    assert_eq!(m.screen(), ScreenId::About);
    let back = list
        .iter()
        .position(|i| matches!(i, Item::Button { .. }))
        .unwrap();
    let r = rects[back];
    let p = Vec2 {
        x: r.x + r.w / 2.0,
        y: r.y + r.h / 2.0,
    };
    m.handle(UiInput::PointerPressed(p));
    m.handle(UiInput::PointerReleased(p));
    assert_eq!(m.screen(), ScreenId::Main);
}

#[test]
fn the_notice_says_what_the_project_is_and_is_not() {
    let text = all_text().to_lowercase();
    for needle in [
        "unofficial",
        "fan engine",
        "not affiliated with, endorsed by or sponsored by croteam or devolver digital",
        "no game assets",
        "your own copy",
        "trademarks of their respective owners",
        "apache license",
        "the ss2rs contributors",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
}

#[test]
fn the_third_party_credits_name_the_bundled_font_and_point_to_notice() {
    let text = all_text();
    assert!(text.contains("Inter"), "{text}");
    assert!(text.contains("SIL Open Font License 1.1"), "{text}");
    assert!(text.contains("NOTICE"), "{text}");
}

#[test]
fn the_screen_agrees_with_the_notice_file_that_ships_with_the_program() {
    // The repository's NOTICE is the legal text; the screen must not drift from it.
    let notice = include_str!("../../../NOTICE");
    for needle in [
        "not affiliated with, endorsed by, or sponsored by",
        "Croteam or Devolver Digital",
        "Inter Regular v4.1",
        "SIL Open Font License 1.1",
        "trademarks of their respective owners",
    ] {
        assert!(notice.contains(needle), "NOTICE no longer says {needle:?}");
    }
    let screen = all_text();
    assert!(screen.contains("Croteam or Devolver Digital"));
    assert!(screen.contains("SIL Open Font License 1.1"));
}

#[test]
fn every_line_is_plain_ascii_and_short_enough_for_the_row() {
    for line in lines() {
        assert!(line.is_ascii(), "{line:?}");
        assert!(
            !line.is_empty() && line.len() <= 90,
            "{} chars: {line:?}",
            line.len()
        );
    }
    assert!(lines().len() >= 6);
}

#[test]
fn the_whole_screen_fits_in_the_canvas() {
    let m = {
        let mut m = Menu::new(Settings::default());
        open_about(&mut m);
        m
    };
    let rects = m.item_rects();
    let bottom = rects.iter().map(|r| r.y + r.h).fold(0.0, f32::max);
    assert!(bottom <= crate::CANVAS_H - 40.0, "rows end at {bottom}");
    for w in rects.windows(2) {
        assert!(
            w[1].y >= w[0].y + w[0].h,
            "rows overlap: {:?} {:?}",
            w[0],
            w[1]
        );
    }
    let right = rects.iter().map(|r| r.x + r.w).fold(0.0, f32::max);
    assert!(right <= crate::CANVAS_W, "rows reach {right}");
}

#[test]
fn text_lines_are_drawn_left_aligned_without_a_box_and_the_title_is_about() {
    let mut m = Menu::new(Settings::default());
    open_about(&mut m);
    let list = m.draw();
    assert_eq!(list.quads.len(), 1, "only the Back button has a box");
    assert_eq!(list.texts[0].text, "About");
    for line in lines() {
        let run = list
            .texts
            .iter()
            .find(|t| t.text == line)
            .unwrap_or_else(|| panic!("{line:?} is not drawn"));
        assert_eq!(run.align, TextAlign::Left);
    }
    assert!(list.texts.iter().any(|t| t.text == "Back"));
}
