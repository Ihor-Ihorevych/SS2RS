//! Menu navigation from the platform's input events (T03-033): the app reads `ss2_platform_api` events
//! instead of `winit`/`gilrs` types.

use ss2_core::input::NavInput;
use ss2_platform_api::{GamepadButton, InputEvent, KeyCode};
use ss2_ui::UiInput;
use ss2_ui::data_wizard::WizardInput;

/// Keyboard navigation: arrows/WASD move, Enter/Space activate, Escape/Backspace go back.
pub(crate) fn nav_for_key(key: KeyCode) -> Option<NavInput> {
    Some(match key {
        KeyCode::ArrowUp | KeyCode::W => NavInput::Up,
        KeyCode::ArrowDown | KeyCode::S => NavInput::Down,
        KeyCode::ArrowLeft | KeyCode::A => NavInput::Left,
        KeyCode::ArrowRight | KeyCode::D => NavInput::Right,
        KeyCode::Enter | KeyCode::Space => NavInput::Activate,
        KeyCode::Escape | KeyCode::Backspace => NavInput::Back,
        _ => return None,
    })
}

/// Gamepad navigation: D-pad moves, South (A/Cross) activates, East (B/Circle) and Start go back.
pub(crate) fn nav_for_button(button: GamepadButton) -> Option<NavInput> {
    Some(match button {
        GamepadButton::DPadUp => NavInput::Up,
        GamepadButton::DPadDown => NavInput::Down,
        GamepadButton::DPadLeft => NavInput::Left,
        GamepadButton::DPadRight => NavInput::Right,
        GamepadButton::South => NavInput::Activate,
        GamepadButton::East | GamepadButton::Start => NavInput::Back,
        _ => return None,
    })
}

/// The keyboard for the first-run wizard (T10-019b): keys and typed text to [`WizardInput`].
///
/// While the player types a path (`typing`) only the keys that cannot be part of a path navigate: the arrows, Enter, Escape and
/// Tab; letters and Space arrive as [`InputEvent::Text`], and Backspace, Delete, Home and End edit the field. Otherwise the menu's
/// keys apply ([`nav_for_key`]), and a key that navigated does not also type: the window sends the text of a key right after the key
/// (`ss2_platform::window`), so Space on "Enter a folder path" would otherwise start the path with a space.
#[derive(Debug, Default)]
pub(crate) struct WizardKeys {
    /// The key just handled was used to navigate and also produces text: drop that text.
    swallow_text: bool,
}

impl WizardKeys {
    /// The wizard input for `event`, if any. Pointer, gamepad and stick input go through the menu's `UiInput` path instead.
    pub(crate) fn map(&mut self, event: &InputEvent, typing: bool) -> Option<WizardInput> {
        match event {
            InputEvent::Key {
                key, pressed: true, ..
            } => {
                self.swallow_text = false;
                if typing {
                    Self::typing_key(*key)
                } else {
                    let nav = nav_for_key(*key)?;
                    self.swallow_text = Self::types_text(*key);
                    Some(WizardInput::Ui(UiInput::Nav(nav)))
                }
            }
            InputEvent::Text(text) => {
                if std::mem::take(&mut self.swallow_text) {
                    None
                } else {
                    Some(WizardInput::Text(text.clone()))
                }
            }
            _ => None,
        }
    }

    fn typing_key(key: KeyCode) -> Option<WizardInput> {
        let nav = |n| Some(WizardInput::Ui(UiInput::Nav(n)));
        match key {
            KeyCode::ArrowUp => nav(NavInput::Up),
            KeyCode::ArrowDown | KeyCode::Tab => nav(NavInput::Down),
            KeyCode::ArrowLeft => nav(NavInput::Left),
            KeyCode::ArrowRight => nav(NavInput::Right),
            KeyCode::Enter => nav(NavInput::Activate),
            KeyCode::Escape => nav(NavInput::Back),
            KeyCode::Backspace => Some(WizardInput::Backspace),
            KeyCode::Delete => Some(WizardInput::Delete),
            KeyCode::Home => Some(WizardInput::Home),
            KeyCode::End => Some(WizardInput::End),
            _ => None,
        }
    }

    /// Whether the navigation key also types a character (WASD and Space do; the arrows, Enter, Escape and Backspace do not).
    fn types_text(key: KeyCode) -> bool {
        matches!(
            key,
            KeyCode::W | KeyCode::A | KeyCode::S | KeyCode::D | KeyCode::Space
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_like_the_winit_mapping_did() {
        assert_eq!(nav_for_key(KeyCode::W), Some(NavInput::Up));
        assert_eq!(nav_for_key(KeyCode::ArrowDown), Some(NavInput::Down));
        assert_eq!(nav_for_key(KeyCode::A), Some(NavInput::Left));
        assert_eq!(nav_for_key(KeyCode::ArrowRight), Some(NavInput::Right));
        assert_eq!(nav_for_key(KeyCode::Space), Some(NavInput::Activate));
        assert_eq!(nav_for_key(KeyCode::Enter), Some(NavInput::Activate));
        assert_eq!(nav_for_key(KeyCode::Escape), Some(NavInput::Back));
        assert_eq!(nav_for_key(KeyCode::Backspace), Some(NavInput::Back));
        assert_eq!(nav_for_key(KeyCode::F12), None);
        assert_eq!(nav_for_key(KeyCode::Q), None);
    }

    #[test]
    fn gamepad_buttons_map_like_the_gilrs_mapping_did() {
        assert_eq!(nav_for_button(GamepadButton::DPadUp), Some(NavInput::Up));
        assert_eq!(
            nav_for_button(GamepadButton::DPadLeft),
            Some(NavInput::Left)
        );
        assert_eq!(
            nav_for_button(GamepadButton::South),
            Some(NavInput::Activate)
        );
        assert_eq!(nav_for_button(GamepadButton::East), Some(NavInput::Back));
        assert_eq!(nav_for_button(GamepadButton::Start), Some(NavInput::Back));
        assert_eq!(nav_for_button(GamepadButton::North), None);
    }

    fn press(key: KeyCode) -> InputEvent {
        InputEvent::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn text(t: &str) -> InputEvent {
        InputEvent::Text(t.into())
    }

    fn nav(n: NavInput) -> Option<WizardInput> {
        Some(WizardInput::Ui(UiInput::Nav(n)))
    }

    #[test]
    fn while_typing_letters_and_space_are_text_and_the_editing_keys_edit() {
        let mut k = WizardKeys::default();
        // "w" and " " as the window sends them: the key, then its text.
        assert_eq!(k.map(&press(KeyCode::W), true), None);
        assert_eq!(k.map(&text("w"), true), Some(WizardInput::Text("w".into())));
        assert_eq!(k.map(&press(KeyCode::Space), true), None);
        assert_eq!(k.map(&text(" "), true), Some(WizardInput::Text(" ".into())));
        assert_eq!(
            k.map(&press(KeyCode::Backspace), true),
            Some(WizardInput::Backspace)
        );
        assert_eq!(
            k.map(&press(KeyCode::Delete), true),
            Some(WizardInput::Delete)
        );
        assert_eq!(k.map(&press(KeyCode::Home), true), Some(WizardInput::Home));
        assert_eq!(k.map(&press(KeyCode::End), true), Some(WizardInput::End));
        assert_eq!(k.map(&press(KeyCode::Enter), true), nav(NavInput::Activate));
        assert_eq!(k.map(&press(KeyCode::Escape), true), nav(NavInput::Back));
        assert_eq!(k.map(&press(KeyCode::ArrowLeft), true), nav(NavInput::Left));
        assert_eq!(
            k.map(&press(KeyCode::ArrowRight), true),
            nav(NavInput::Right)
        );
        assert_eq!(k.map(&press(KeyCode::ArrowUp), true), nav(NavInput::Up));
        assert_eq!(k.map(&press(KeyCode::Tab), true), nav(NavInput::Down));
        assert_eq!(k.map(&press(KeyCode::F12), true), None);
    }

    #[test]
    fn outside_the_field_the_menu_keys_navigate_and_their_text_is_dropped() {
        let mut k = WizardKeys::default();
        assert_eq!(k.map(&press(KeyCode::S), false), nav(NavInput::Down));
        assert_eq!(k.map(&text("s"), false), None, "S moved, it does not type");
        assert_eq!(
            k.map(&press(KeyCode::Space), false),
            nav(NavInput::Activate)
        );
        // Space opened the path screen: its text must not land in the field.
        assert_eq!(k.map(&text(" "), true), None);
        // The next text is typed again.
        assert_eq!(k.map(&text("C"), true), Some(WizardInput::Text("C".into())));
        // Backspace goes back outside the field, like in the menus, and makes no text to drop.
        assert_eq!(
            k.map(&press(KeyCode::Backspace), false),
            nav(NavInput::Back)
        );
        assert_eq!(k.map(&text("x"), true), Some(WizardInput::Text("x".into())));
        // A key that is no navigation gives nothing, and its text is not dropped (the wizard ignores text outside the field anyway).
        assert_eq!(k.map(&press(KeyCode::Q), false), None);
        assert_eq!(
            k.map(&text("q"), false),
            Some(WizardInput::Text("q".into()))
        );
    }

    #[test]
    fn releases_and_other_events_give_nothing() {
        let mut k = WizardKeys::default();
        let release = InputEvent::Key {
            key: KeyCode::Enter,
            pressed: false,
            repeat: false,
        };
        assert_eq!(k.map(&release, false), None);
        assert_eq!(
            k.map(&InputEvent::CursorMoved { x: 1.0, y: 2.0 }, true),
            None
        );
        // A held key repeats (Backspace through a long path).
        let repeat = InputEvent::Key {
            key: KeyCode::Backspace,
            pressed: true,
            repeat: true,
        };
        assert_eq!(k.map(&repeat, true), Some(WizardInput::Backspace));
    }
}
