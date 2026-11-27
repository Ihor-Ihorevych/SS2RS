//! Menu navigation from the platform's input events (T03-033): the app reads `ss2_platform_api` events
//! instead of `winit`/`gilrs` types.

use ss2_core::input::NavInput;
use ss2_platform_api::{GamepadButton, KeyCode};

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
}
