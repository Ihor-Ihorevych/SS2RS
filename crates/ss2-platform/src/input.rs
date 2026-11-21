//! Pure translation from `winit` keyboard and mouse input to the platform contract's
//! [`InputEvent`]s (`ss2-platform-api`, RFC 0003).
//!
//! Like `translate.rs`, nothing here touches a window: `winit` cannot build a `KeyEvent` outside a
//! running loop, so `window.rs` pulls the few fields it needs out of each event and calls these
//! functions, which can then be tested exhaustively without a display.

use std::collections::BTreeSet;

use ss2_platform_api::{InputEvent, KeyCode, MouseButton};
use winit::event::{ElementState, MouseButton as WinitButton, MouseScrollDelta};
use winit::keyboard::KeyCode as W;

/// How many device pixels one "line" of scrolling is, for devices (touchpads) that report pixels.
/// The contract's `Scroll` is in lines (one wheel notch = 1.0).
pub const PIXELS_PER_LINE: f64 = 20.0;

/// Our physical key for a `winit` physical key; `None` for keys the contract does not name yet
/// (numpad, media keys, ...). Physical position, never the typed character: that is
/// [`InputEvent::Text`].
pub fn map_key(code: W) -> Option<KeyCode> {
    Some(match code {
        W::KeyA => KeyCode::A,
        W::KeyB => KeyCode::B,
        W::KeyC => KeyCode::C,
        W::KeyD => KeyCode::D,
        W::KeyE => KeyCode::E,
        W::KeyF => KeyCode::F,
        W::KeyG => KeyCode::G,
        W::KeyH => KeyCode::H,
        W::KeyI => KeyCode::I,
        W::KeyJ => KeyCode::J,
        W::KeyK => KeyCode::K,
        W::KeyL => KeyCode::L,
        W::KeyM => KeyCode::M,
        W::KeyN => KeyCode::N,
        W::KeyO => KeyCode::O,
        W::KeyP => KeyCode::P,
        W::KeyQ => KeyCode::Q,
        W::KeyR => KeyCode::R,
        W::KeyS => KeyCode::S,
        W::KeyT => KeyCode::T,
        W::KeyU => KeyCode::U,
        W::KeyV => KeyCode::V,
        W::KeyW => KeyCode::W,
        W::KeyX => KeyCode::X,
        W::KeyY => KeyCode::Y,
        W::KeyZ => KeyCode::Z,
        W::Digit0 => KeyCode::Digit0,
        W::Digit1 => KeyCode::Digit1,
        W::Digit2 => KeyCode::Digit2,
        W::Digit3 => KeyCode::Digit3,
        W::Digit4 => KeyCode::Digit4,
        W::Digit5 => KeyCode::Digit5,
        W::Digit6 => KeyCode::Digit6,
        W::Digit7 => KeyCode::Digit7,
        W::Digit8 => KeyCode::Digit8,
        W::Digit9 => KeyCode::Digit9,
        W::F1 => KeyCode::F1,
        W::F2 => KeyCode::F2,
        W::F3 => KeyCode::F3,
        W::F4 => KeyCode::F4,
        W::F5 => KeyCode::F5,
        W::F6 => KeyCode::F6,
        W::F7 => KeyCode::F7,
        W::F8 => KeyCode::F8,
        W::F9 => KeyCode::F9,
        W::F10 => KeyCode::F10,
        W::F11 => KeyCode::F11,
        W::F12 => KeyCode::F12,
        W::ArrowUp => KeyCode::ArrowUp,
        W::ArrowDown => KeyCode::ArrowDown,
        W::ArrowLeft => KeyCode::ArrowLeft,
        W::ArrowRight => KeyCode::ArrowRight,
        W::Space => KeyCode::Space,
        W::Enter => KeyCode::Enter,
        W::Escape => KeyCode::Escape,
        W::Tab => KeyCode::Tab,
        W::Backspace => KeyCode::Backspace,
        W::Delete => KeyCode::Delete,
        W::Insert => KeyCode::Insert,
        W::Home => KeyCode::Home,
        W::End => KeyCode::End,
        W::PageUp => KeyCode::PageUp,
        W::PageDown => KeyCode::PageDown,
        W::ShiftLeft => KeyCode::ShiftLeft,
        W::ShiftRight => KeyCode::ShiftRight,
        W::ControlLeft => KeyCode::ControlLeft,
        W::ControlRight => KeyCode::ControlRight,
        W::AltLeft => KeyCode::AltLeft,
        W::AltRight => KeyCode::AltRight,
        W::Backquote => KeyCode::Backquote,
        W::Minus => KeyCode::Minus,
        W::Equal => KeyCode::Equal,
        W::Comma => KeyCode::Comma,
        W::Period => KeyCode::Period,
        W::Slash => KeyCode::Slash,
        W::Backslash => KeyCode::Backslash,
        W::Semicolon => KeyCode::Semicolon,
        W::Quote => KeyCode::Quote,
        W::BracketLeft => KeyCode::BracketLeft,
        W::BracketRight => KeyCode::BracketRight,
        _ => return None,
    })
}

/// Our mouse button for a `winit` one.
pub fn map_mouse_button(button: WinitButton) -> MouseButton {
    match button {
        WinitButton::Left => MouseButton::Left,
        WinitButton::Right => MouseButton::Right,
        WinitButton::Middle => MouseButton::Middle,
        WinitButton::Back => MouseButton::Back,
        WinitButton::Forward => MouseButton::Forward,
        WinitButton::Other(n) => MouseButton::Other(n),
    }
}

/// A key press or release. `repeat` is `winit`'s OS auto-repeat flag and is only meaningful (and
/// only ever `true`) for presses. `None` if the key has no contract name.
pub fn key_event(code: W, state: ElementState, repeat: bool) -> Option<InputEvent> {
    let key = map_key(code)?;
    let pressed = state == ElementState::Pressed;
    Some(InputEvent::Key {
        key,
        pressed,
        repeat: pressed && repeat,
    })
}

/// A mouse button press or release.
pub fn mouse_button_event(button: WinitButton, state: ElementState) -> InputEvent {
    InputEvent::MouseButton {
        button: map_mouse_button(button),
        pressed: state == ElementState::Pressed,
    }
}

/// A scroll in lines. Positive `dy` is the wheel turned **away** from the user (content moves
/// down), matching the contract; touchpad pixel deltas are converted at [`PIXELS_PER_LINE`].
pub fn scroll_event(delta: MouseScrollDelta) -> InputEvent {
    let (dx, dy) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x, y),
        MouseScrollDelta::PixelDelta(p) => (
            (p.x / PIXELS_PER_LINE) as f32,
            (p.y / PIXELS_PER_LINE) as f32,
        ),
    };
    InputEvent::Scroll { dx, dy }
}

/// The text a key press typed, for the console and chat. Control characters are dropped: Enter,
/// Tab, Backspace and Ctrl+letter arrive as text on some platforms but are *keys* here. `None` if
/// nothing printable is left.
pub fn printable_text(text: &str) -> Option<String> {
    let filtered: String = text.chars().filter(|c| !c.is_control()).collect();
    (!filtered.is_empty()).then_some(filtered)
}

/// Remembers which keys and mouse buttons are down so that losing focus can release them all:
/// the contract promises "losing focus releases all inputs", and the OS does not send the missing
/// key-up events (a held `W` would otherwise walk the player forever after an Alt+Tab).
#[derive(Clone, Debug, Default)]
pub struct HeldInputs {
    keys: BTreeSet<KeyCode>,
    buttons: BTreeSet<MouseButton>,
}

impl HeldInputs {
    /// Track `event` (key and mouse-button presses and releases; everything else is ignored).
    pub fn observe(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Key { key, pressed, .. } => {
                if *pressed {
                    self.keys.insert(*key);
                } else {
                    self.keys.remove(key);
                }
            }
            InputEvent::MouseButton { button, pressed } => {
                if *pressed {
                    self.buttons.insert(*button);
                } else {
                    self.buttons.remove(button);
                }
            }
            _ => {}
        }
    }

    /// Track `event` and say whether the app should see it. Everything is accepted except the
    /// **release of a key or button that is not held**: that release belongs to a press the app
    /// never saw (it happened in another window, or it follows the releases `release_all` already
    /// sent), and delivering it would make the app's view of the keyboard inconsistent.
    pub fn accept(&mut self, event: &InputEvent) -> bool {
        let stray_release = match event {
            InputEvent::Key {
                key,
                pressed: false,
                ..
            } => !self.keys.contains(key),
            InputEvent::MouseButton {
                button,
                pressed: false,
            } => !self.buttons.contains(button),
            _ => false,
        };
        if stray_release {
            return false;
        }
        self.observe(event);
        true
    }

    /// `true` if nothing is held.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.buttons.is_empty()
    }

    /// Release events for everything held, in a stable order (keys, then buttons), and forget it
    /// all. Call when the window loses focus.
    pub fn release_all(&mut self) -> Vec<InputEvent> {
        let keys = std::mem::take(&mut self.keys);
        let buttons = std::mem::take(&mut self.buttons);
        keys.into_iter()
            .map(|key| InputEvent::Key {
                key,
                pressed: false,
                repeat: false,
            })
            .chain(buttons.into_iter().map(|button| InputEvent::MouseButton {
                button,
                pressed: false,
            }))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;

    use super::*;

    #[test]
    fn letters_digits_and_function_keys_map_one_to_one() {
        let pairs = [
            (W::KeyA, KeyCode::A),
            (W::KeyW, KeyCode::W),
            (W::KeyZ, KeyCode::Z),
            (W::Digit0, KeyCode::Digit0),
            (W::Digit9, KeyCode::Digit9),
            (W::F1, KeyCode::F1),
            (W::F12, KeyCode::F12),
            (W::Backquote, KeyCode::Backquote),
            (W::ArrowLeft, KeyCode::ArrowLeft),
            (W::ShiftRight, KeyCode::ShiftRight),
            (W::BracketRight, KeyCode::BracketRight),
            (W::Space, KeyCode::Space),
            (W::Escape, KeyCode::Escape),
        ];
        for (winit, ours) in pairs {
            assert_eq!(map_key(winit), Some(ours), "{winit:?}");
        }
    }

    #[test]
    fn the_mapping_is_injective_and_total_over_the_named_keys() {
        // Every winit key we map goes to a distinct contract key, so two physical keys can never
        // be confused. (The list is every contract key's source; unmapped winit keys are `None`.)
        let all = [
            W::KeyA,
            W::KeyB,
            W::KeyC,
            W::KeyD,
            W::KeyE,
            W::KeyF,
            W::KeyG,
            W::KeyH,
            W::KeyI,
            W::KeyJ,
            W::KeyK,
            W::KeyL,
            W::KeyM,
            W::KeyN,
            W::KeyO,
            W::KeyP,
            W::KeyQ,
            W::KeyR,
            W::KeyS,
            W::KeyT,
            W::KeyU,
            W::KeyV,
            W::KeyW,
            W::KeyX,
            W::KeyY,
            W::KeyZ,
            W::Digit0,
            W::Digit1,
            W::Digit2,
            W::Digit3,
            W::Digit4,
            W::Digit5,
            W::Digit6,
            W::Digit7,
            W::Digit8,
            W::Digit9,
            W::F1,
            W::F2,
            W::F3,
            W::F4,
            W::F5,
            W::F6,
            W::F7,
            W::F8,
            W::F9,
            W::F10,
            W::F11,
            W::F12,
            W::ArrowUp,
            W::ArrowDown,
            W::ArrowLeft,
            W::ArrowRight,
            W::Space,
            W::Enter,
            W::Escape,
            W::Tab,
            W::Backspace,
            W::Delete,
            W::Insert,
            W::Home,
            W::End,
            W::PageUp,
            W::PageDown,
            W::ShiftLeft,
            W::ShiftRight,
            W::ControlLeft,
            W::ControlRight,
            W::AltLeft,
            W::AltRight,
            W::Backquote,
            W::Minus,
            W::Equal,
            W::Comma,
            W::Period,
            W::Slash,
            W::Backslash,
            W::Semicolon,
            W::Quote,
            W::BracketLeft,
            W::BracketRight,
        ];
        let mapped: BTreeSet<KeyCode> = all.iter().map(|k| map_key(*k).expect("mapped")).collect();
        assert_eq!(
            mapped.len(),
            all.len(),
            "two winit keys map to the same contract key"
        );
    }

    #[test]
    fn unnamed_keys_are_dropped_not_guessed() {
        for k in [
            W::Numpad1,
            W::NumLock,
            W::MediaPlayPause,
            W::PrintScreen,
            W::CapsLock,
            W::SuperLeft,
        ] {
            assert_eq!(map_key(k), None, "{k:?}");
        }
        assert_eq!(key_event(W::Numpad1, ElementState::Pressed, false), None);
    }

    #[test]
    fn key_events_carry_press_release_and_repeat() {
        assert_eq!(
            key_event(W::KeyW, ElementState::Pressed, false),
            Some(InputEvent::Key {
                key: KeyCode::W,
                pressed: true,
                repeat: false
            })
        );
        assert_eq!(
            key_event(W::KeyW, ElementState::Pressed, true),
            Some(InputEvent::Key {
                key: KeyCode::W,
                pressed: true,
                repeat: true
            })
        );
        // The contract says `repeat` is only for presses: a release is never a repeat.
        assert_eq!(
            key_event(W::KeyW, ElementState::Released, true),
            Some(InputEvent::Key {
                key: KeyCode::W,
                pressed: false,
                repeat: false
            })
        );
    }

    #[test]
    fn mouse_buttons_map_including_unknown_ones() {
        assert_eq!(map_mouse_button(WinitButton::Left), MouseButton::Left);
        assert_eq!(map_mouse_button(WinitButton::Right), MouseButton::Right);
        assert_eq!(map_mouse_button(WinitButton::Middle), MouseButton::Middle);
        assert_eq!(map_mouse_button(WinitButton::Back), MouseButton::Back);
        assert_eq!(map_mouse_button(WinitButton::Forward), MouseButton::Forward);
        assert_eq!(
            map_mouse_button(WinitButton::Other(7)),
            MouseButton::Other(7)
        );
        assert_eq!(
            mouse_button_event(WinitButton::Right, ElementState::Released),
            InputEvent::MouseButton {
                button: MouseButton::Right,
                pressed: false
            }
        );
    }

    #[test]
    fn scroll_is_in_lines_and_pixels_are_converted() {
        assert_eq!(
            scroll_event(MouseScrollDelta::LineDelta(0.0, 1.0)),
            InputEvent::Scroll { dx: 0.0, dy: 1.0 }
        );
        assert_eq!(
            scroll_event(MouseScrollDelta::LineDelta(-2.0, -3.0)),
            InputEvent::Scroll { dx: -2.0, dy: -3.0 }
        );
        assert_eq!(
            scroll_event(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                40.0, -10.0
            ))),
            InputEvent::Scroll { dx: 2.0, dy: -0.5 }
        );
    }

    #[test]
    fn text_keeps_printable_characters_and_drops_control_ones() {
        assert_eq!(printable_text("a"), Some("a".to_owned()));
        assert_eq!(
            printable_text("é"),
            Some("é".to_owned()),
            "non-ASCII is text"
        );
        assert_eq!(printable_text("字"), Some("字".to_owned()));
        assert_eq!(printable_text(" "), Some(" ".to_owned()), "space is text");
        // Enter, Tab, Backspace, Ctrl+C and Delete arrive as control characters: keys, not text.
        for control in ["\r", "\n", "\t", "\u{8}", "\u{3}", "\u{7f}", "\u{1b}"] {
            assert_eq!(printable_text(control), None, "{control:?}");
        }
        assert_eq!(printable_text("a\u{8}b"), Some("ab".to_owned()));
        assert_eq!(printable_text(""), None);
    }

    fn press(key: KeyCode) -> InputEvent {
        InputEvent::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: KeyCode) -> InputEvent {
        InputEvent::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[test]
    fn held_inputs_are_released_in_a_stable_order_exactly_once() {
        let mut held = HeldInputs::default();
        assert!(held.is_empty());
        for e in [
            press(KeyCode::W),
            press(KeyCode::A),
            InputEvent::MouseButton {
                button: MouseButton::Left,
                pressed: true,
            },
            InputEvent::Key {
                key: KeyCode::W,
                pressed: true,
                repeat: true,
            },
            press(KeyCode::Space),
            release(KeyCode::A),
            InputEvent::Text("x".into()),
            InputEvent::CursorMoved { x: 1.0, y: 2.0 },
        ] {
            held.observe(&e);
        }
        assert!(!held.is_empty());
        assert_eq!(
            held.release_all(),
            [
                release(KeyCode::W),
                release(KeyCode::Space),
                InputEvent::MouseButton {
                    button: MouseButton::Left,
                    pressed: false
                },
            ],
            "W (despite the repeat) and Space, then the mouse button; A was already released"
        );
        assert!(held.is_empty());
        assert!(held.release_all().is_empty(), "nothing is released twice");
    }

    #[test]
    fn stray_releases_are_dropped_and_everything_else_passes() {
        let mut held = HeldInputs::default();
        // A release for something never pressed (pressed in another window): dropped.
        assert!(!held.accept(&release(KeyCode::Q)));
        assert!(!held.accept(&InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: false
        }));
        // Presses, repeats, and the matching release pass; a second release is stray again.
        assert!(held.accept(&press(KeyCode::Q)));
        assert!(held.accept(&InputEvent::Key {
            key: KeyCode::Q,
            pressed: true,
            repeat: true
        }));
        assert!(held.accept(&release(KeyCode::Q)));
        assert!(!held.accept(&release(KeyCode::Q)), "already released");
        // Non-key events always pass.
        assert!(held.accept(&InputEvent::Text("x".into())));
        assert!(held.accept(&InputEvent::CursorMoved { x: 0.0, y: 0.0 }));
        assert!(held.is_empty());
    }

    /// The Windows focus-loss sequence seen on a real window: we release what is held, then the
    /// OS's own (synthetic or late) release for the same key arrives and must not reach the app.
    #[test]
    fn the_release_after_a_focus_loss_release_is_not_a_duplicate() {
        let mut held = HeldInputs::default();
        assert!(held.accept(&press(KeyCode::Q)));
        let ours = held.release_all();
        assert_eq!(ours, [release(KeyCode::Q)]);
        assert!(
            !held.accept(&release(KeyCode::Q)),
            "the OS's late key-up is dropped"
        );
    }

    #[test]
    fn a_release_for_a_key_never_seen_pressed_is_harmless() {
        let mut held = HeldInputs::default();
        held.observe(&release(KeyCode::Q));
        assert!(held.is_empty());
    }
}
