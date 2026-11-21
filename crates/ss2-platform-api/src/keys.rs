//! Device-independent names for physical inputs. The platform layer translates its windowing /
//! gamepad library's types into these, so nothing outside `ss2-platform` ever names `winit` or
//! `gilrs`.

use serde::{Deserialize, Serialize};

/// A **physical key position** (layout independent: `W` is the key where QWERTY has W, wherever
/// the user's layout puts it). Typed characters arrive separately as
/// [`InputEvent::Text`](crate::InputEvent::Text).
///
/// Variant names are the serialization format of saved key bindings: **never rename one**.
/// New variants may be added (`#[non_exhaustive]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)] // variant names are the documentation
#[rustfmt::skip]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Space, Enter, Escape, Tab, Backspace, Delete, Insert, Home, End, PageUp, PageDown,
    ShiftLeft, ShiftRight, ControlLeft, ControlRight, AltLeft, AltRight,
    /// The key left of `1` (the console key in most Serious Sam games).
    Backquote,
    Minus, Equal, Comma, Period, Slash, Backslash, Semicolon, Quote, BracketLeft, BracketRight,
}

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Wheel click.
    Middle,
    /// "Back" side button.
    Back,
    /// "Forward" side button.
    Forward,
    /// Any other button, by platform index.
    Other(u16),
}

/// Identifies one connected gamepad for as long as it stays connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GamepadId(pub u32);

/// A gamepad button in the *standard layout* (Xbox naming for position, so `South` is `A` on
/// Xbox and `Cross` on PlayStation). The platform maps every supported pad onto this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)] // names follow the standard gamepad layout
#[rustfmt::skip]
pub enum GamepadButton {
    South, East, West, North,
    LeftBumper, RightBumper,
    LeftStick, RightStick,
    DPadUp, DPadDown, DPadLeft, DPadRight,
    Start, Select, Guide,
}

/// A gamepad axis. Sticks run `-1.0..=1.0` with **up/right positive**; triggers `0.0..=1.0`.
/// Values in events are already dead-zone filtered by the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)] // names follow the standard gamepad layout
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn keys_are_ordered_hashable_and_distinct() {
        let set: BTreeSet<KeyCode> = [KeyCode::W, KeyCode::A, KeyCode::W, KeyCode::Space].into();
        assert_eq!(set.len(), 3);
        assert_ne!(MouseButton::Other(4), MouseButton::Other(5));
    }

    #[test]
    fn variant_names_are_the_serialized_form() {
        // Saved bindings depend on these exact strings; this test is the tripwire for renames.
        #[derive(Serialize)]
        struct Wrap {
            key: KeyCode,
            pad: GamepadButton,
            axis: GamepadAxis,
            mouse: MouseButton,
        }
        let text = toml::to_string(&Wrap {
            key: KeyCode::Backquote,
            pad: GamepadButton::LeftBumper,
            axis: GamepadAxis::RightTrigger,
            mouse: MouseButton::Left,
        })
        .unwrap();
        assert!(text.contains("key = \"Backquote\""), "{text}");
        assert!(text.contains("pad = \"LeftBumper\""), "{text}");
        assert!(text.contains("axis = \"RightTrigger\""), "{text}");
        assert!(text.contains("mouse = \"Left\""), "{text}");
    }
}
