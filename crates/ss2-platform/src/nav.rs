//! Maps keyboard keys, gamepad buttons and analog sticks onto [`NavInput`].
//!
//! Everything here is pure: no window or device access, so it is fully unit-testable.

use gilrs::Button;
use ss2_core::input::NavInput;
use winit::keyboard::KeyCode;

/// Stick magnitude (on the dominant axis) required before it counts as a push.
const DEADZONE: f32 = 0.5;
/// Delay before a held stick starts repeating.
const FIRST_REPEAT_MS: u64 = 350;
/// Interval between repeats once repeating.
const REPEAT_MS: u64 = 120;

/// Keyboard navigation: arrows/WASD move, Enter/Space activate, Escape/Backspace go back.
pub fn map_key(key: KeyCode) -> Option<NavInput> {
    Some(match key {
        KeyCode::ArrowUp | KeyCode::KeyW => NavInput::Up,
        KeyCode::ArrowDown | KeyCode::KeyS => NavInput::Down,
        KeyCode::ArrowLeft | KeyCode::KeyA => NavInput::Left,
        KeyCode::ArrowRight | KeyCode::KeyD => NavInput::Right,
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => NavInput::Activate,
        KeyCode::Escape | KeyCode::Backspace => NavInput::Back,
        _ => return None,
    })
}

/// Gamepad navigation: D-pad moves, South (A/Cross) activates, East (B/Circle) and Start go back.
pub fn map_gamepad_button(button: Button) -> Option<NavInput> {
    Some(match button {
        Button::DPadUp => NavInput::Up,
        Button::DPadDown => NavInput::Down,
        Button::DPadLeft => NavInput::Left,
        Button::DPadRight => NavInput::Right,
        Button::South => NavInput::Activate,
        Button::East | Button::Start => NavInput::Back,
        _ => return None,
    })
}

/// Turns a continuously sampled analog stick into discrete navigation steps with key-repeat.
#[derive(Debug, Default)]
pub struct StickNavigator {
    held: Option<NavInput>,
    next_repeat_ms: u64,
}

impl StickNavigator {
    /// A navigator with nothing held.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one stick sample (`y > 0` is up, the gilrs convention) taken at `now_ms`.
    /// Returns a step when the stick is first pushed, changes direction, or repeats while held.
    pub fn update(&mut self, x: f32, y: f32, now_ms: u64) -> Option<NavInput> {
        let magnitude = x.abs().max(y.abs());
        if magnitude.is_nan() || magnitude < DEADZONE {
            self.held = None;
            return None;
        }
        let dir = if x.abs() >= y.abs() {
            if x > 0.0 {
                NavInput::Right
            } else {
                NavInput::Left
            }
        } else if y > 0.0 {
            NavInput::Up
        } else {
            NavInput::Down
        };
        if self.held != Some(dir) {
            self.held = Some(dir);
            self.next_repeat_ms = now_ms + FIRST_REPEAT_MS;
            Some(dir)
        } else if now_ms >= self.next_repeat_ms {
            self.next_repeat_ms = now_ms + REPEAT_MS;
            Some(dir)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gilrs::Button;
    use ss2_core::input::NavInput;
    use winit::keyboard::KeyCode;

    #[test]
    fn keyboard_mapping_table() {
        let table = [
            (KeyCode::ArrowUp, NavInput::Up),
            (KeyCode::KeyW, NavInput::Up),
            (KeyCode::ArrowDown, NavInput::Down),
            (KeyCode::KeyS, NavInput::Down),
            (KeyCode::ArrowLeft, NavInput::Left),
            (KeyCode::KeyA, NavInput::Left),
            (KeyCode::ArrowRight, NavInput::Right),
            (KeyCode::KeyD, NavInput::Right),
            (KeyCode::Enter, NavInput::Activate),
            (KeyCode::NumpadEnter, NavInput::Activate),
            (KeyCode::Space, NavInput::Activate),
            (KeyCode::Escape, NavInput::Back),
            (KeyCode::Backspace, NavInput::Back),
        ];
        for (key, want) in table {
            assert_eq!(map_key(key), Some(want), "{key:?}");
        }
        assert_eq!(map_key(KeyCode::KeyQ), None);
    }

    #[test]
    fn gamepad_buttons() {
        assert_eq!(map_gamepad_button(Button::DPadUp), Some(NavInput::Up));
        assert_eq!(map_gamepad_button(Button::DPadDown), Some(NavInput::Down));
        assert_eq!(map_gamepad_button(Button::DPadLeft), Some(NavInput::Left));
        assert_eq!(map_gamepad_button(Button::DPadRight), Some(NavInput::Right));
        assert_eq!(map_gamepad_button(Button::South), Some(NavInput::Activate));
        assert_eq!(map_gamepad_button(Button::East), Some(NavInput::Back));
        assert_eq!(map_gamepad_button(Button::Start), Some(NavInput::Back));
        assert_eq!(map_gamepad_button(Button::North), None);
    }

    #[test]
    fn stick_under_deadzone_is_ignored() {
        let mut s = StickNavigator::new();
        for t in 0..100 {
            assert_eq!(s.update(0.49, 0.3, t * 16), None);
        }
    }

    #[test]
    fn stick_push_fires_once_immediately() {
        let mut s = StickNavigator::new();
        assert_eq!(s.update(0.9, 0.0, 0), Some(NavInput::Right));
        assert_eq!(s.update(0.9, 0.0, 16), None);
    }

    #[test]
    fn stick_hold_repeats_after_350_then_120_ms() {
        let mut s = StickNavigator::new();
        assert_eq!(s.update(0.0, 0.9, 0), Some(NavInput::Up));
        assert_eq!(s.update(0.0, 0.9, 100), None);
        assert_eq!(s.update(0.0, 0.9, 349), None);
        assert_eq!(s.update(0.0, 0.9, 350), Some(NavInput::Up));
        assert_eq!(s.update(0.0, 0.9, 400), None);
        assert_eq!(s.update(0.0, 0.9, 470), Some(NavInput::Up));
    }

    #[test]
    fn stick_release_and_repush_fires_immediately() {
        let mut s = StickNavigator::new();
        assert_eq!(s.update(0.9, 0.0, 0), Some(NavInput::Right));
        assert_eq!(s.update(0.0, 0.0, 50), None);
        assert_eq!(s.update(0.9, 0.0, 60), Some(NavInput::Right));
    }

    #[test]
    fn stick_diagonal_picks_dominant_axis() {
        let mut s = StickNavigator::new();
        assert_eq!(s.update(0.6, -0.9, 0), Some(NavInput::Down));
    }

    #[test]
    fn stick_direction_change_fires_immediately() {
        let mut s = StickNavigator::new();
        assert_eq!(s.update(0.9, 0.0, 0), Some(NavInput::Right));
        assert_eq!(s.update(-0.9, 0.0, 10), Some(NavInput::Left));
    }

    #[test]
    fn nan_stick_is_ignored() {
        let mut s = StickNavigator::new();
        assert_eq!(s.update(f32::NAN, f32::NAN, 0), None);
        assert_eq!(s.update(f32::NAN, 0.9, 10), Some(NavInput::Up));
    }
}
