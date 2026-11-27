//! Play controls (T03-037): platform input events in, one [`PlayInput`] per frame out.
//!
//! Keyboard and mouse: W/A/S/D move, the mouse looks, Space jumps (rises while flying), Ctrl or C sinks while
//! flying, Shift flies faster, F switches between walking and flying. Gamepad: the left stick moves, the right
//! stick looks, South (A/Cross) jumps or rises, North (Y/Triangle) switches flying.

use std::collections::BTreeSet;

use ss2_core::math::{Vec2, Vec3};
use ss2_game::play::PlayInput;
use ss2_platform_api::{GamepadAxis, GamepadButton, InputEvent, KeyCode};

/// How fast a fully pushed right stick turns, in look units (mouse pixels) per second.
pub(crate) const STICK_LOOK_SPEED: f32 = 1400.0;

/// What is held and what happened since the last frame.
#[derive(Debug, Default)]
pub(crate) struct PlayControls {
    keys: BTreeSet<KeyCode>,
    look: Vec2,
    left_stick: Vec2,
    right_stick: Vec2,
    pad_south: bool,
    toggle_fly: bool,
}

impl PlayControls {
    /// Take one input event.
    pub(crate) fn on_event(&mut self, event: &InputEvent) {
        match *event {
            InputEvent::Key {
                key,
                pressed,
                repeat,
            } => {
                if pressed {
                    if !repeat && key == KeyCode::F {
                        self.toggle_fly = true;
                    }
                    self.keys.insert(key);
                } else {
                    self.keys.remove(&key);
                }
            }
            InputEvent::MouseMotion { dx, dy } => {
                if dx.is_finite() && dy.is_finite() {
                    self.look += Vec2::new(dx, dy);
                }
            }
            InputEvent::GamepadAxis { axis, value, .. } => {
                let value = if value.is_finite() {
                    value.clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                match axis {
                    GamepadAxis::LeftStickX => self.left_stick.x = value,
                    GamepadAxis::LeftStickY => self.left_stick.y = value,
                    GamepadAxis::RightStickX => self.right_stick.x = value,
                    GamepadAxis::RightStickY => self.right_stick.y = value,
                    _ => {}
                }
            }
            InputEvent::GamepadButton {
                button, pressed, ..
            } => match button {
                GamepadButton::South => self.pad_south = pressed,
                GamepadButton::North if pressed => self.toggle_fly = true,
                _ => {}
            },
            InputEvent::GamepadDisconnected(_) => {
                self.left_stick = Vec2::ZERO;
                self.right_stick = Vec2::ZERO;
                self.pad_south = false;
            }
            _ => {}
        }
    }

    /// Forget everything held (pausing, losing focus).
    pub(crate) fn release_all(&mut self) {
        *self = Self::default();
    }

    fn held(&self, key: KeyCode) -> f32 {
        if self.keys.contains(&key) { 1.0 } else { 0.0 }
    }

    /// This frame's input after `dt` seconds; the look and the fly switch are used up.
    pub(crate) fn frame(&mut self, dt: f32) -> PlayInput {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let keys_x = self.held(KeyCode::D) - self.held(KeyCode::A);
        let keys_z = self.held(KeyCode::W) - self.held(KeyCode::S);
        let up = self
            .held(KeyCode::Space)
            .max(f32::from(u8::from(self.pad_south)));
        let down = self.held(KeyCode::ControlLeft).max(self.held(KeyCode::C));
        let mut movement = Vec3::new(
            keys_x + self.left_stick.x,
            up - down,
            keys_z + self.left_stick.y,
        );
        movement.x = movement.x.clamp(-1.0, 1.0);
        movement.z = movement.z.clamp(-1.0, 1.0);
        // Stick up (positive) looks up, which is a negative look.y.
        let stick = Vec2::new(self.right_stick.x, -self.right_stick.y) * STICK_LOOK_SPEED * dt;
        PlayInput {
            movement,
            look: std::mem::take(&mut self.look) + stick,
            jump: self.held(KeyCode::Space) > 0.0 || self.pad_south,
            boost: self.keys.contains(&KeyCode::ShiftLeft)
                || self.keys.contains(&KeyCode::ShiftRight),
            toggle_fly: std::mem::take(&mut self.toggle_fly),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_platform_api::GamepadId;

    fn key(key: KeyCode, pressed: bool) -> InputEvent {
        InputEvent::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    #[test]
    fn held_keys_move_and_a_release_stops() {
        let mut c = PlayControls::default();
        c.on_event(&key(KeyCode::W, true));
        c.on_event(&key(KeyCode::D, true));
        let i = c.frame(0.016);
        assert_eq!(i.movement, Vec3::new(1.0, 0.0, 1.0));
        assert_eq!(
            c.frame(0.016).movement,
            Vec3::new(1.0, 0.0, 1.0),
            "still held"
        );
        c.on_event(&key(KeyCode::W, false));
        c.on_event(&key(KeyCode::A, true));
        assert_eq!(c.frame(0.016).movement, Vec3::ZERO, "A and D cancel");
        c.on_event(&key(KeyCode::S, true));
        assert_eq!(c.frame(0.016).movement.z, -1.0);
    }

    #[test]
    fn mouse_motion_adds_up_until_the_frame_takes_it() {
        let mut c = PlayControls::default();
        c.on_event(&InputEvent::MouseMotion { dx: 3.0, dy: -1.0 });
        c.on_event(&InputEvent::MouseMotion { dx: 2.0, dy: 4.0 });
        c.on_event(&InputEvent::MouseMotion {
            dx: f32::NAN,
            dy: 1.0,
        });
        assert_eq!(c.frame(0.016).look, Vec2::new(5.0, 3.0));
        assert_eq!(c.frame(0.016).look, Vec2::ZERO);
    }

    #[test]
    fn f_switches_flying_once_per_press_and_space_jumps_or_rises() {
        let mut c = PlayControls::default();
        c.on_event(&key(KeyCode::F, true));
        c.on_event(&InputEvent::Key {
            key: KeyCode::F,
            pressed: true,
            repeat: true,
        });
        assert!(c.frame(0.016).toggle_fly);
        assert!(!c.frame(0.016).toggle_fly, "used up; repeats do not count");
        c.on_event(&key(KeyCode::Space, true));
        c.on_event(&key(KeyCode::ShiftLeft, true));
        let i = c.frame(0.016);
        assert!(i.jump && i.boost);
        assert_eq!(i.movement.y, 1.0);
        c.on_event(&key(KeyCode::Space, false));
        c.on_event(&key(KeyCode::ControlLeft, true));
        assert_eq!(c.frame(0.016).movement.y, -1.0);
    }

    #[test]
    fn the_gamepad_moves_looks_jumps_and_switches_flying() {
        let mut c = PlayControls::default();
        let pad = GamepadId(0);
        let axis = |axis, value| InputEvent::GamepadAxis { pad, axis, value };
        c.on_event(&axis(GamepadAxis::LeftStickY, 1.0));
        c.on_event(&axis(GamepadAxis::LeftStickX, -0.5));
        c.on_event(&axis(GamepadAxis::RightStickX, 1.0));
        c.on_event(&axis(GamepadAxis::RightStickY, 1.0));
        c.on_event(&InputEvent::GamepadButton {
            pad,
            button: GamepadButton::South,
            pressed: true,
        });
        c.on_event(&InputEvent::GamepadButton {
            pad,
            button: GamepadButton::North,
            pressed: true,
        });
        let i = c.frame(0.5);
        assert_eq!(i.movement, Vec3::new(-0.5, 1.0, 1.0));
        assert_eq!(
            i.look,
            Vec2::new(700.0, -700.0),
            "right and up, for half a second"
        );
        assert!(i.jump && i.toggle_fly);
        c.on_event(&InputEvent::GamepadDisconnected(pad));
        let i = c.frame(0.5);
        assert_eq!(
            (i.movement, i.look, i.jump),
            (Vec3::ZERO, Vec2::ZERO, false)
        );
    }

    #[test]
    fn release_all_forgets_everything() {
        let mut c = PlayControls::default();
        c.on_event(&key(KeyCode::W, true));
        c.on_event(&InputEvent::MouseMotion { dx: 9.0, dy: 9.0 });
        c.release_all();
        assert_eq!(c.frame(0.016), PlayInput::default());
    }
}
