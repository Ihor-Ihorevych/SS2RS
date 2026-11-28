use crate::menu::{Menu, MenuCommand, ScreenId, UiInput};
use crate::{Vec2, theme};
use ss2_core::input::NavInput;
use ss2_core::settings::Settings;

fn center(r: crate::Rect) -> Vec2 {
    Vec2 {
        x: r.x + r.w / 2.0,
        y: r.y + r.h / 2.0,
    }
}
fn click(m: &mut Menu, p: Vec2) -> Vec<MenuCommand> {
    m.handle(UiInput::PointerPressed(p));
    m.handle(UiInput::PointerReleased(p))
}
fn to_options(m: &mut Menu) {
    m.handle(UiInput::Nav(NavInput::Down));
    m.handle(UiInput::Nav(NavInput::Activate));
    assert_eq!(m.screen(), ScreenId::Options);
}

#[test]
fn focused_item_uses_accent_color_others_idle() {
    let m = Menu::new(Settings::default());
    let q = m.draw().quads;
    assert_eq!(q.len(), 5);
    assert_eq!(q[0].color, theme::FOCUSED);
    for other in &q[1..] {
        assert_eq!(other.color, theme::IDLE);
    }
}

#[test]
fn draw_has_title_and_one_label_per_button() {
    let texts: Vec<String> = Menu::new(Settings::default())
        .draw()
        .texts
        .into_iter()
        .map(|t| t.text.into_owned())
        .collect();
    for want in ["SS2RS", "New Game", "Options", "About", "Quit"] {
        assert!(
            texts.iter().any(|t| t == want),
            "{want} missing in {texts:?}"
        );
    }
}

#[test]
fn hover_moves_focus_without_command() {
    let mut m = Menu::new(Settings::default());
    let r = m.item_rects();
    let cmds = m.handle(UiInput::PointerMoved(center(r[2])));
    assert!(cmds.is_empty());
    assert_eq!(m.focused(), Some(2));
}

#[test]
fn press_release_same_item_activates() {
    let mut m = Menu::new(Settings::default());
    let r = m.item_rects();
    assert_eq!(click(&mut m, center(r[0])), vec![MenuCommand::StartGame]);
}

#[test]
fn press_a_release_b_does_nothing() {
    let mut m = Menu::new(Settings::default());
    let r = m.item_rects();
    m.handle(UiInput::PointerPressed(center(r[0])));
    assert!(m.handle(UiInput::PointerReleased(center(r[2]))).is_empty());
}

#[test]
fn press_outside_all_items_does_nothing() {
    let mut m = Menu::new(Settings::default());
    let before = m.focused();
    assert!(click(&mut m, Vec2 { x: 10.0, y: 10.0 }).is_empty());
    assert_eq!(m.focused(), before);
}

#[test]
fn slider_track_press_sets_value_snapped() {
    let mut m = Menu::new(Settings::default());
    to_options(&mut m);
    let r = m.item_rects()[2]; // Master slider
    let p = Vec2 {
        x: r.x + 300.0 + 150.0,
        y: r.y + 36.0,
    };
    let cmds = m.handle(UiInput::PointerPressed(p));
    assert!((m.settings().master_volume - 0.5).abs() < 1e-4);
    assert_eq!(
        cmds,
        vec![MenuCommand::SettingsChanged(m.settings().clone())]
    );
    assert!(
        m.handle(UiInput::PointerReleased(p)).is_empty(),
        "release after a track press must not activate"
    );
}

#[test]
fn pointer_and_nav_fuzz_never_panics_and_keeps_focus_valid() {
    let mut seed = 0x1234_5678_u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seed
    };
    for screen in [
        ScreenId::Main,
        ScreenId::Options,
        ScreenId::Pause,
        ScreenId::About,
    ] {
        let mut m = Menu::new(Settings::default());
        match screen {
            ScreenId::Pause => m.open_pause(),
            ScreenId::Options => to_options(&mut m),
            ScreenId::About => {
                m.handle(UiInput::Nav(NavInput::Down));
                m.handle(UiInput::Nav(NavInput::Down));
                m.handle(UiInput::Nav(NavInput::Down));
                m.handle(UiInput::Nav(NavInput::Activate));
                assert_eq!(m.screen(), ScreenId::About);
            }
            ScreenId::Main => {}
        }
        for _ in 0..200 {
            let p = Vec2 {
                x: (next() % 2200) as f32 - 100.0,
                y: (next() % 1300) as f32 - 100.0,
            };
            let input = match next() % 8 {
                0 => UiInput::PointerMoved(p),
                1 => UiInput::PointerPressed(p),
                2 => UiInput::PointerReleased(p),
                3 => UiInput::Nav(NavInput::Up),
                4 => UiInput::Nav(NavInput::Down),
                5 => UiInput::Nav(NavInput::Left),
                6 => UiInput::Nav(NavInput::Right),
                _ => UiInput::PointerMoved(p),
            };
            m.handle(input);
            assert!(m.focused().is_some(), "focus invalid on {:?}", m.screen());
        }
    }
}
