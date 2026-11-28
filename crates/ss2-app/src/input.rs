//! Menu navigation from the platform's input events (T03-033): the app reads `ss2_platform_api` events
//! instead of `winit`/`gilrs` types.

use ss2_core::input::NavInput;
use ss2_platform_api::{Clipboard, GamepadButton, InputEvent, KeyCode, MockClipboard};
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
///
/// Ctrl+V (and Shift+Insert) while typing paste the clipboard's text (T10-035): it arrives as [`WizardInput::Text`], which the field
/// inserts at the cursor without its new lines and other control characters. The platform reports no modifier state, so the Ctrl and
/// Shift keys are followed here (the platform releases every held key when the window loses the focus, so none sticks).
pub(crate) struct WizardKeys {
    /// The key just handled was used to navigate and also produces text: drop that text.
    swallow_text: bool,
    /// The Ctrl keys held (left, right).
    ctrl: [bool; 2],
    /// The Shift keys held (left, right).
    shift: [bool; 2],
    clipboard: Box<dyn Clipboard>,
}

impl Default for WizardKeys {
    /// Keys with an empty clipboard (tests).
    fn default() -> Self {
        Self::new(Box::new(MockClipboard::default()))
    }
}

impl WizardKeys {
    /// Keys that paste from `clipboard`.
    pub(crate) fn new(clipboard: Box<dyn Clipboard>) -> Self {
        Self {
            swallow_text: false,
            ctrl: [false; 2],
            shift: [false; 2],
            clipboard,
        }
    }

    /// The wizard input for `event`, if any. Pointer, gamepad and stick input go through the menu's `UiInput` path instead.
    pub(crate) fn map(&mut self, event: &InputEvent, typing: bool) -> Option<WizardInput> {
        match event {
            InputEvent::Key { key, pressed, .. } if self.modifier(*key, *pressed) => None,
            InputEvent::Key {
                key, pressed: true, ..
            } => {
                self.swallow_text = false;
                if typing && self.pastes(*key) {
                    // Whatever text the key makes with the modifier held is not typed as well.
                    self.swallow_text = true;
                    return self.clipboard.text().map(WizardInput::Text);
                }
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

    /// Follow the Ctrl and Shift keys; whether `key` is one of them.
    fn modifier(&mut self, key: KeyCode, pressed: bool) -> bool {
        let slot = match key {
            KeyCode::ControlLeft => &mut self.ctrl[0],
            KeyCode::ControlRight => &mut self.ctrl[1],
            KeyCode::ShiftLeft => &mut self.shift[0],
            KeyCode::ShiftRight => &mut self.shift[1],
            _ => return false,
        };
        *slot = pressed;
        true
    }

    /// Whether pressing `key` now pastes: Ctrl+V, or Shift+Insert.
    fn pastes(&self, key: KeyCode) -> bool {
        let ctrl = self.ctrl.contains(&true);
        let shift = self.shift.contains(&true);
        (ctrl && key == KeyCode::V) || (shift && !ctrl && key == KeyCode::Insert)
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

    fn release(key: KeyCode) -> InputEvent {
        InputEvent::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[test]
    fn ctrl_v_while_typing_pastes_the_clipboard_and_its_key_text_is_dropped() {
        let copied = "C:\\Games\\Serious Sam 2\r\n";
        let mut k = WizardKeys::new(Box::new(MockClipboard::with_text(copied)));
        assert_eq!(k.map(&press(KeyCode::ControlLeft), true), None);
        assert_eq!(
            k.map(&press(KeyCode::V), true),
            Some(WizardInput::Text(copied.into())),
            "as copied: the field strips the new line"
        );
        // Ctrl+V's own character, if the layout makes one, is not typed.
        assert_eq!(k.map(&text("\u{16}"), true), None);
        assert_eq!(k.map(&release(KeyCode::V), true), None);
        assert_eq!(k.map(&release(KeyCode::ControlLeft), true), None);
        // Ctrl let go: V types again.
        assert_eq!(k.map(&press(KeyCode::V), true), None);
        assert_eq!(k.map(&text("v"), true), Some(WizardInput::Text("v".into())));
        // Shift+Insert pastes too, with the right-hand Shift as well.
        assert_eq!(k.map(&press(KeyCode::ShiftRight), true), None);
        assert!(matches!(
            k.map(&press(KeyCode::Insert), true),
            Some(WizardInput::Text(_))
        ));
        assert_eq!(k.map(&release(KeyCode::ShiftRight), true), None);
        assert_eq!(k.map(&press(KeyCode::Insert), true), None);
    }

    #[test]
    fn ctrl_v_outside_the_field_or_with_an_empty_clipboard_does_nothing() {
        let mut k = WizardKeys::new(Box::new(MockClipboard::with_text("D:/SS2")));
        k.map(&press(KeyCode::ControlRight), false);
        assert_eq!(k.map(&press(KeyCode::V), false), None, "not typing");
        let mut empty = WizardKeys::default();
        empty.map(&press(KeyCode::ControlLeft), true);
        assert_eq!(empty.map(&press(KeyCode::V), true), None);
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
