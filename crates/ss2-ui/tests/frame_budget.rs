//! The UI frame budget (roadmap T10-023; `docs/UI_BUDGET.md`): how many draw calls a screen may cost, and that a **warm frame
//! does not allocate**.
//!
//! This test is safe code only (the workspace denies `unsafe_code`; see `docs/CODING_STANDARDS.md`, "Unsafe"), so it cannot
//! count allocations. It checks the two things that make this code allocate: **an owned `String` in a text run** (fixed text is
//! `Cow::Borrowed`) and **a draw list that has to grow**. A counting-allocator version was run once to measure the real number (0 allocations
//! in 200 warm frames of each screen) and is not committed, because `unsafe` needs the approval of the crate owner and a second maintainer
//! (T10-023b).

use std::borrow::Cow;

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_ui::{DrawList, Menu, UiInput};

/// The budget of one frame of one screen: quads and text runs (each is a draw item in the renderer).
const MAX_QUADS: usize = 24;
const MAX_TEXTS: usize = 32;

fn press(menu: &mut Menu, steps: &[NavInput]) {
    for s in steps {
        menu.handle(UiInput::Nav(*s));
    }
}

/// A menu on each of its screens: the main menu, options (down, activate) and about (down, down, activate).
fn screens() -> Vec<(&'static str, Menu)> {
    let main = Menu::new(Settings::default());
    let mut options = Menu::new(Settings::default());
    press(&mut options, &[NavInput::Down, NavInput::Activate]);
    let mut about = Menu::new(Settings::default());
    press(
        &mut about,
        &[NavInput::Down, NavInput::Down, NavInput::Activate],
    );
    vec![("main", main), ("options", options), ("about", about)]
}

#[test]
fn a_warm_frame_stays_inside_the_draw_call_budget_and_needs_nothing_that_allocates() {
    for (name, mut menu) in screens() {
        let mut list = DrawList::default();
        // Warm up: the first frames may grow the lists.
        for _ in 0..4 {
            menu.draw_into(&mut list);
        }
        let (quads, texts) = (list.quads.len(), list.texts.len());
        assert!(
            quads <= MAX_QUADS && texts <= MAX_TEXTS,
            "{name}: {quads} quads and {texts} text runs exceed {MAX_QUADS} and {MAX_TEXTS}"
        );
        assert!(
            quads + texts > 0,
            "{name}: an empty screen would make the checks below meaningless"
        );

        let capacities = (list.quads.capacity(), list.texts.capacity());
        for frame in 0..200 {
            menu.draw_into(&mut list);
            assert!(
                list.texts
                    .iter()
                    .all(|t| matches!(t.text, Cow::Borrowed(_))),
                "{name} frame {frame}: a text run owns its string, which allocates every frame: {:?}",
                list.texts.iter().find(|t| matches!(t.text, Cow::Owned(_)))
            );
            assert_eq!(
                (list.quads.capacity(), list.texts.capacity()),
                capacities,
                "{name} frame {frame}: the draw list had to grow"
            );
        }

        // Changing what is shown must not make a warm frame need more either: step through every item.
        for _ in 0..8 {
            menu.handle(UiInput::Nav(NavInput::Down));
            menu.draw_into(&mut list);
            assert!(
                list.texts
                    .iter()
                    .all(|t| matches!(t.text, Cow::Borrowed(_))),
                "{name} after moving the focus"
            );
            assert_eq!(
                (list.quads.capacity(), list.texts.capacity()),
                capacities,
                "{name} after moving the focus"
            );
        }
    }
}

#[test]
fn a_reused_list_equals_a_fresh_one_and_keeps_nothing_of_the_previous_frame() {
    for (name, menu) in screens() {
        let mut reused = DrawList::default();
        menu.draw_into(&mut reused);
        assert_eq!(reused, menu.draw(), "{name}");
        // A list that held another screen is cleared first.
        let mut other = Menu::new(Settings::default()).draw();
        let leftover = other.quads.len();
        menu.draw_into(&mut other);
        assert_eq!(
            other,
            menu.draw(),
            "{name}: a reused list must not keep the previous frame (it had {leftover} quads)"
        );
    }
}
