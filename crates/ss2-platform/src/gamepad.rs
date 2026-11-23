//! Gamepads for the platform contract (roadmap T03-003): `gilrs` events become the
//! [`InputEvent`]s of `ss2-platform-api`, in the standard layout, with dead zones.
//!
//! The translation is pure and tested without a pad: [`GamepadTranslator`] takes a [`PadEvent`]
//! (what `gilrs` said, reduced to values that can be built anywhere) and returns the events the app
//! should see. [`GamepadSource`] is the thin shell around a real `gilrs` instance (the mapping
//! database of `gilrs`, which knows hundreds of pads, plus any mappings the player adds) and force
//! feedback. What the contract promises (`ss2-platform-api`):
//!
//! - buttons are the **standard layout** (`South` is `A` on Xbox, `Cross` on PlayStation);
//! - sticks run `-1..=1` with **up and right positive**, triggers `0..=1`;
//! - values are **dead-zone filtered**: a stick at rest reads exactly 0, and the usable range is
//!   stretched to the full `-1..=1` so a small dead zone costs no top speed;
//! - a pad that disconnects **releases** everything it held.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Replay, Ticks};
use gilrs::{Axis, Button, EventType, Gilrs, GilrsBuilder};
use ss2_platform_api::{GamepadAxis, GamepadButton, GamepadId, InputEvent};

/// How the raw values are filtered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GamepadConfig {
    /// The radius of the dead zone of each stick, as a fraction of full deflection (limited to
    /// `0.0..=0.95`).
    pub stick_dead_zone: f32,
    /// The dead zone of each analog trigger (limited to `0.0..=0.95`).
    pub trigger_dead_zone: f32,
}

impl Default for GamepadConfig {
    /// A stick dead zone of 0.15 (typical for worn thumb sticks) and a trigger dead zone of 0.05.
    fn default() -> Self {
        Self {
            stick_dead_zone: 0.15,
            trigger_dead_zone: 0.05,
        }
    }
}

/// What `gilrs` reported, reduced to values that can be built anywhere (a `gilrs` event cannot be
/// made without a real pad).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PadEvent {
    /// A pad appeared.
    Connected,
    /// The pad went away.
    Disconnected,
    /// A button changed state.
    Button {
        /// Which, in `gilrs` terms.
        button: Button,
        /// Pressed or released.
        pressed: bool,
    },
    /// A button that has an analog value (the triggers) changed it.
    ButtonValue {
        /// Which.
        button: Button,
        /// `0.0..=1.0`.
        value: f32,
    },
    /// An axis moved.
    Axis {
        /// Which, in `gilrs` terms.
        axis: Axis,
        /// `-1.0..=1.0`.
        value: f32,
    },
}

/// A `gilrs` button in the standard layout; `None` for the analog triggers (they arrive as axes)
/// and for buttons the layout has no place for.
pub fn button_from_gilrs(button: Button) -> Option<GamepadButton> {
    Some(match button {
        Button::South => GamepadButton::South,
        Button::East => GamepadButton::East,
        Button::West => GamepadButton::West,
        Button::North => GamepadButton::North,
        Button::LeftTrigger => GamepadButton::LeftBumper,
        Button::RightTrigger => GamepadButton::RightBumper,
        Button::LeftThumb => GamepadButton::LeftStick,
        Button::RightThumb => GamepadButton::RightStick,
        Button::DPadUp => GamepadButton::DPadUp,
        Button::DPadDown => GamepadButton::DPadDown,
        Button::DPadLeft => GamepadButton::DPadLeft,
        Button::DPadRight => GamepadButton::DPadRight,
        Button::Start => GamepadButton::Start,
        Button::Select => GamepadButton::Select,
        Button::Mode => GamepadButton::Guide,
        Button::LeftTrigger2 | Button::RightTrigger2 | Button::C | Button::Z | Button::Unknown => {
            return None;
        }
    })
}

/// Reduce a `gilrs` event to a [`PadEvent`]; auto-repeat and bookkeeping events are dropped.
pub fn pad_event_from_gilrs(event: &EventType) -> Option<PadEvent> {
    Some(match *event {
        EventType::Connected => PadEvent::Connected,
        EventType::Disconnected => PadEvent::Disconnected,
        EventType::ButtonPressed(button, _) => PadEvent::Button {
            button,
            pressed: true,
        },
        EventType::ButtonReleased(button, _) => PadEvent::Button {
            button,
            pressed: false,
        },
        EventType::ButtonChanged(button, value, _) => PadEvent::ButtonValue { button, value },
        EventType::AxisChanged(axis, value, _) => PadEvent::Axis { axis, value },
        // Auto-repeat (the app repeats on its own), dropped events, finished rumble, and whatever a newer `gilrs` adds.
        _ => return None,
    })
}

/// The radial dead zone of a stick: inside the circle of radius `dead_zone` the stick reads zero;
/// outside it the length is stretched so that the edge of the circle reads 0 and full deflection
/// reads 1, the direction is kept, and a square pad's corner (longer than 1) is limited to the unit circle.
pub fn radial_dead_zone(x: f32, y: f32, dead_zone: f32) -> (f32, f32) {
    if !x.is_finite() || !y.is_finite() {
        return (0.0, 0.0);
    }
    let dead_zone = dead_zone.clamp(0.0, 0.95);
    let length = x.hypot(y);
    if length <= dead_zone || length == 0.0 {
        return (0.0, 0.0);
    }
    let scaled = ((length - dead_zone) / (1.0 - dead_zone)).min(1.0);
    (x / length * scaled, y / length * scaled)
}

/// The dead zone of a trigger: `0.0..=dead_zone` reads 0, the rest is stretched to `0.0..=1.0`.
pub fn trigger_dead_zone(value: f32, dead_zone: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let dead_zone = dead_zone.clamp(0.0, 0.95);
    if value <= dead_zone {
        return 0.0;
    }
    ((value - dead_zone) / (1.0 - dead_zone)).min(1.0)
}

/// Changes smaller than this are not reported (noise in the last digits of an analog value).
const AXIS_EPSILON: f32 = 0.005;

#[derive(Default)]
struct PadState {
    buttons: BTreeSet<GamepadButton>,
    /// Raw stick values: left x, left y, right x, right y.
    sticks: [f32; 4],
    /// The last filtered value sent for each axis.
    sent: BTreeMap<GamepadAxis, f32>,
}

/// Turns [`PadEvent`]s into [`InputEvent`]s; see the module documentation.
pub struct GamepadTranslator {
    config: GamepadConfig,
    pads: BTreeMap<u32, PadState>,
}

impl GamepadTranslator {
    /// A translator with no pads.
    pub fn new(config: GamepadConfig) -> Self {
        Self {
            config,
            pads: BTreeMap::new(),
        }
    }

    /// How many pads are connected.
    pub fn connected(&self) -> usize {
        self.pads.len()
    }

    fn emit_axis(
        state: &mut PadState,
        pad: GamepadId,
        axis: GamepadAxis,
        value: f32,
        out: &mut Vec<InputEvent>,
    ) {
        let last = state.sent.get(&axis).copied().unwrap_or(0.0);
        let changed = if value == 0.0 {
            last != 0.0
        } else {
            (value - last).abs() > AXIS_EPSILON
        };
        if changed {
            state.sent.insert(axis, value);
            out.push(InputEvent::GamepadAxis { pad, axis, value });
        }
    }

    fn set_button(
        state: &mut PadState,
        pad: GamepadId,
        button: GamepadButton,
        pressed: bool,
        out: &mut Vec<InputEvent>,
    ) {
        let changed = if pressed {
            state.buttons.insert(button)
        } else {
            state.buttons.remove(&button)
        };
        if changed {
            out.push(InputEvent::GamepadButton {
                pad,
                button,
                pressed,
            });
        }
    }

    /// Translate one event from `pad`. A pad that was not announced is announced first.
    pub fn translate(&mut self, pad: GamepadId, event: PadEvent) -> Vec<InputEvent> {
        let mut out = Vec::new();
        if event == PadEvent::Disconnected {
            if let Some(mut state) = self.pads.remove(&pad.0) {
                for button in std::mem::take(&mut state.buttons) {
                    out.push(InputEvent::GamepadButton {
                        pad,
                        button,
                        pressed: false,
                    });
                }
                for (axis, value) in std::mem::take(&mut state.sent) {
                    if value != 0.0 {
                        out.push(InputEvent::GamepadAxis {
                            pad,
                            axis,
                            value: 0.0,
                        });
                    }
                }
                out.push(InputEvent::GamepadDisconnected(pad));
            }
            return out;
        }
        if let std::collections::btree_map::Entry::Vacant(slot) = self.pads.entry(pad.0) {
            slot.insert(PadState::default());
            out.push(InputEvent::GamepadConnected(pad));
        }
        let config = self.config;
        let state = self.pads.get_mut(&pad.0).expect("inserted above");
        match event {
            PadEvent::Connected | PadEvent::Disconnected => {}
            PadEvent::Button { button, pressed } => {
                if let Some(button) = button_from_gilrs(button) {
                    Self::set_button(state, pad, button, pressed, &mut out);
                }
            }
            PadEvent::ButtonValue { button, value } => {
                let axis = match button {
                    Button::LeftTrigger2 => GamepadAxis::LeftTrigger,
                    Button::RightTrigger2 => GamepadAxis::RightTrigger,
                    _ => return out,
                };
                Self::emit_axis(
                    state,
                    pad,
                    axis,
                    trigger_dead_zone(value, config.trigger_dead_zone),
                    &mut out,
                );
            }
            PadEvent::Axis { axis, value } => {
                if !value.is_finite() {
                    return out;
                }
                match axis {
                    Axis::LeftStickX | Axis::LeftStickY | Axis::RightStickX | Axis::RightStickY => {
                        let (slot, left) = match axis {
                            Axis::LeftStickX => (0, true),
                            Axis::LeftStickY => (1, true),
                            Axis::RightStickX => (2, false),
                            _ => (3, false),
                        };
                        state.sticks[slot] = value;
                        let base = if left { 0 } else { 2 };
                        let (x, y) = radial_dead_zone(
                            state.sticks[base],
                            state.sticks[base + 1],
                            config.stick_dead_zone,
                        );
                        let (ax, ay) = if left {
                            (GamepadAxis::LeftStickX, GamepadAxis::LeftStickY)
                        } else {
                            (GamepadAxis::RightStickX, GamepadAxis::RightStickY)
                        };
                        Self::emit_axis(state, pad, ax, x, &mut out);
                        Self::emit_axis(state, pad, ay, y, &mut out);
                    }
                    Axis::LeftZ => Self::emit_axis(
                        state,
                        pad,
                        GamepadAxis::LeftTrigger,
                        trigger_dead_zone(value, config.trigger_dead_zone),
                        &mut out,
                    ),
                    Axis::RightZ => Self::emit_axis(
                        state,
                        pad,
                        GamepadAxis::RightTrigger,
                        trigger_dead_zone(value, config.trigger_dead_zone),
                        &mut out,
                    ),
                    Axis::DPadX => {
                        Self::set_button(
                            state,
                            pad,
                            GamepadButton::DPadLeft,
                            value < -0.5,
                            &mut out,
                        );
                        Self::set_button(
                            state,
                            pad,
                            GamepadButton::DPadRight,
                            value > 0.5,
                            &mut out,
                        );
                    }
                    Axis::DPadY => {
                        Self::set_button(
                            state,
                            pad,
                            GamepadButton::DPadDown,
                            value < -0.5,
                            &mut out,
                        );
                        Self::set_button(state, pad, GamepadButton::DPadUp, value > 0.5, &mut out);
                    }
                    Axis::Unknown => {}
                }
            }
        }
        out
    }
}

/// The real thing: a `gilrs` instance, the translator, and force feedback.
pub struct GamepadSource {
    gilrs: Gilrs,
    translator: GamepadTranslator,
    ids: BTreeMap<u32, gilrs::GamepadId>,
    effects: BTreeMap<u32, Effect>,
}

impl GamepadSource {
    /// Start `gilrs` (with its built-in mapping database for hundreds of pads) and add `mappings`,
    /// lines in the SDL `gamecontrollerdb` format (the same text as the `SDL_GAMECONTROLLERCONFIG`
    /// environment variable); lines that are not mappings are ignored by `gilrs`.
    pub fn new(config: GamepadConfig, mappings: Option<&str>) -> Result<Self, String> {
        let mut builder = GilrsBuilder::new();
        if let Some(text) = mappings {
            builder = builder.add_mappings(text);
        }
        let gilrs = builder.build().map_err(|e| e.to_string())?;
        Ok(Self {
            gilrs,
            translator: GamepadTranslator::new(config),
            ids: BTreeMap::new(),
            effects: BTreeMap::new(),
        })
    }

    /// Drain what the pads have reported since the last call into `out`, translated.
    pub fn poll(&mut self, out: &mut Vec<InputEvent>) {
        while let Some(event) = self.gilrs.next_event() {
            let pad = GamepadId(usize::from(event.id) as u32);
            self.ids.insert(pad.0, event.id);
            let Some(pad_event) = pad_event_from_gilrs(&event.event) else {
                continue;
            };
            if pad_event == PadEvent::Disconnected {
                self.effects.remove(&pad.0);
            }
            out.extend(self.translator.translate(pad, pad_event));
        }
    }

    /// The pad's name as the system reports it.
    pub fn name(&self, pad: GamepadId) -> Option<String> {
        let id = *self.ids.get(&pad.0)?;
        self.gilrs
            .connected_gamepad(id)
            .map(|g| g.name().to_owned())
    }

    /// Rumble: `strong` (the heavy motor) and `weak` (the light one) from 0 to 1 for `duration`.
    /// `false` if the pad is not there or cannot rumble.
    pub fn rumble(&mut self, pad: GamepadId, strong: f32, weak: f32, duration: Duration) -> bool {
        let Some(id) = self.ids.get(&pad.0).copied() else {
            return false;
        };
        let magnitude = |v: f32| (v.clamp(0.0, 1.0) * f32::from(u16::MAX)) as u16;
        let play_for = Ticks::from_ms(u32::try_from(duration.as_millis()).unwrap_or(u32::MAX));
        let effect = EffectBuilder::new()
            .add_effect(BaseEffect {
                kind: BaseEffectType::Strong {
                    magnitude: magnitude(strong),
                },
                scheduling: Replay {
                    play_for,
                    ..Default::default()
                },
                ..Default::default()
            })
            .add_effect(BaseEffect {
                kind: BaseEffectType::Weak {
                    magnitude: magnitude(weak),
                },
                scheduling: Replay {
                    play_for,
                    ..Default::default()
                },
                ..Default::default()
            })
            .gamepads(&[id])
            .finish(&mut self.gilrs);
        match effect {
            Ok(effect) if effect.play().is_ok() => {
                // The effect stops when it is dropped: keep it until the next one replaces it.
                self.effects.insert(pad.0, effect);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gilrs::{Axis, Button};
    use ss2_platform_api::{GamepadAxis, GamepadButton, GamepadId, InputEvent};

    const PAD: GamepadId = GamepadId(0);

    fn translator() -> GamepadTranslator {
        GamepadTranslator::new(GamepadConfig::default())
    }

    fn connected() -> GamepadTranslator {
        let mut t = translator();
        t.translate(PAD, PadEvent::Connected);
        t
    }

    fn axis(t: &mut GamepadTranslator, axis: Axis, value: f32) -> Vec<InputEvent> {
        t.translate(PAD, PadEvent::Axis { axis, value })
    }

    fn value_of(events: &[InputEvent], wanted: GamepadAxis) -> Option<f32> {
        events.iter().find_map(|e| match e {
            InputEvent::GamepadAxis { axis, value, .. } if *axis == wanted => Some(*value),
            _ => None,
        })
    }

    // ---- the standard layout ---------------------------------------------------------------

    #[test]
    fn every_gilrs_button_maps_to_the_standard_layout_or_to_nothing() {
        let expect = [
            (Button::South, Some(GamepadButton::South)),
            (Button::East, Some(GamepadButton::East)),
            (Button::West, Some(GamepadButton::West)),
            (Button::North, Some(GamepadButton::North)),
            (Button::LeftTrigger, Some(GamepadButton::LeftBumper)),
            (Button::RightTrigger, Some(GamepadButton::RightBumper)),
            (Button::LeftThumb, Some(GamepadButton::LeftStick)),
            (Button::RightThumb, Some(GamepadButton::RightStick)),
            (Button::DPadUp, Some(GamepadButton::DPadUp)),
            (Button::DPadDown, Some(GamepadButton::DPadDown)),
            (Button::DPadLeft, Some(GamepadButton::DPadLeft)),
            (Button::DPadRight, Some(GamepadButton::DPadRight)),
            (Button::Start, Some(GamepadButton::Start)),
            (Button::Select, Some(GamepadButton::Select)),
            (Button::Mode, Some(GamepadButton::Guide)),
            // The analog triggers are delivered as axes, not as buttons; C and Z have no place in the layout.
            (Button::LeftTrigger2, None),
            (Button::RightTrigger2, None),
            (Button::C, None),
            (Button::Z, None),
            (Button::Unknown, None),
        ];
        for (from, to) in expect {
            assert_eq!(button_from_gilrs(from), to, "{from:?}");
        }
    }

    #[test]
    fn gilrs_events_are_reduced_to_pad_events_and_the_rest_is_ignored() {
        use gilrs::EventType::*;
        // `gilrs::ev::Code` cannot be built outside `gilrs`, so the events that carry one (button
        // and axis changes) cannot be made here: their arms are plain field copies, checked by the
        // compiler, and everything they feed is tested through `PadEvent` below.
        assert_eq!(pad_event_from_gilrs(&Connected), Some(PadEvent::Connected));
        assert_eq!(
            pad_event_from_gilrs(&Disconnected),
            Some(PadEvent::Disconnected)
        );
        assert_eq!(pad_event_from_gilrs(&Dropped), None);
        assert_eq!(pad_event_from_gilrs(&ForceFeedbackEffectCompleted), None);
    }

    // ---- dead zones ------------------------------------------------------------------------

    #[test]
    fn the_radial_dead_zone_zeroes_the_inside_and_stretches_the_rest_to_full_range() {
        assert_eq!(radial_dead_zone(0.0, 0.0, 0.15), (0.0, 0.0));
        assert_eq!(
            radial_dead_zone(0.1, 0.1, 0.15),
            (0.0, 0.0),
            "inside the circle, though each axis is under it"
        );
        assert_eq!(
            radial_dead_zone(0.15, 0.0, 0.15),
            (0.0, 0.0),
            "the edge is still dead"
        );
        // Each axis is under the dead zone, but the stick is 0.198 from the centre: a circle, not a square.
        let (x, y) = radial_dead_zone(0.14, 0.14, 0.15);
        assert!(x > 0.0 && (x - y).abs() < 1e-6, "{x} {y}");
        let (x, y) = radial_dead_zone(1.0, 0.0, 0.15);
        assert!(
            (x - 1.0).abs() < 1e-6 && y == 0.0,
            "full deflection stays full"
        );
        let (x, _) = radial_dead_zone(0.575, 0.0, 0.15);
        assert!(
            (x - 0.5).abs() < 1e-5,
            "halfway between the edge and full reads one half: {x}"
        );
        let (x, y) = radial_dead_zone(0.8, 0.6, 0.2); // length 1.0, direction (0.8, 0.6)
        assert!(
            (x - 0.8).abs() < 1e-5 && (y - 0.6).abs() < 1e-5,
            "the direction is kept: {x} {y}"
        );
        let (x, y) = radial_dead_zone(1.0, 1.0, 0.15); // a square pad's corner: longer than 1
        assert!(
            ((x * x + y * y).sqrt() - 1.0).abs() < 1e-5 && (x - y).abs() < 1e-6,
            "limited to the unit circle: {x} {y}"
        );
        let (x, y) = radial_dead_zone(-0.9, 0.0, 0.15);
        assert!(x < 0.0 && y == 0.0, "signs are kept");
    }

    #[test]
    fn a_trigger_has_a_small_dead_zone_and_is_stretched_to_0_to_1() {
        assert_eq!(trigger_dead_zone(0.04, 0.05), 0.0);
        assert!((trigger_dead_zone(1.0, 0.05) - 1.0).abs() < 1e-6);
        assert!((trigger_dead_zone(0.525, 0.05) - 0.5).abs() < 1e-5);
        assert_eq!(
            trigger_dead_zone(-0.5, 0.05),
            0.0,
            "a trigger never goes negative"
        );
        assert!(
            (trigger_dead_zone(7.0, 0.05) - 1.0).abs() < 1e-6,
            "and never above one"
        );
        assert_eq!(trigger_dead_zone(f32::NAN, 0.05), 0.0);
    }

    // ---- the translator ---------------------------------------------------------------------

    #[test]
    fn a_new_pad_is_announced_once_and_an_unannounced_pad_is_announced_first() {
        let mut t = translator();
        assert_eq!(
            t.translate(PAD, PadEvent::Connected),
            vec![InputEvent::GamepadConnected(PAD)]
        );
        assert!(
            t.translate(PAD, PadEvent::Connected).is_empty(),
            "a repeated connect says nothing"
        );
        let mut late = translator();
        let events = late.translate(
            GamepadId(3),
            PadEvent::Button {
                button: Button::South,
                pressed: true,
            },
        );
        assert_eq!(
            events[0],
            InputEvent::GamepadConnected(GamepadId(3)),
            "the app hears of the pad before its first input"
        );
        assert_eq!(
            events[1],
            InputEvent::GamepadButton {
                pad: GamepadId(3),
                button: GamepadButton::South,
                pressed: true
            }
        );
        assert_eq!(late.connected(), 1);
    }

    #[test]
    fn buttons_report_changes_only() {
        let mut t = connected();
        let press = PadEvent::Button {
            button: Button::South,
            pressed: true,
        };
        assert_eq!(
            t.translate(PAD, press),
            vec![InputEvent::GamepadButton {
                pad: PAD,
                button: GamepadButton::South,
                pressed: true
            }]
        );
        assert!(
            t.translate(PAD, press).is_empty(),
            "pressed twice is pressed once"
        );
        let release = PadEvent::Button {
            button: Button::South,
            pressed: false,
        };
        assert_eq!(t.translate(PAD, release).len(), 1);
        assert!(
            t.translate(PAD, release).is_empty(),
            "a release of what is not held is dropped"
        );
        assert!(
            t.translate(
                PAD,
                PadEvent::Button {
                    button: Button::Unknown,
                    pressed: true
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn a_stick_at_rest_reads_zero_and_moves_are_filtered_in_pairs() {
        let mut t = connected();
        assert!(
            axis(&mut t, Axis::LeftStickX, 0.1).is_empty(),
            "inside the dead zone nothing changes"
        );
        let events = axis(&mut t, Axis::LeftStickX, 1.0);
        assert!((value_of(&events, GamepadAxis::LeftStickX).unwrap() - 1.0).abs() < 1e-6);
        // Pushing the other axis changes the direction and the length: the first axis is re-reported.
        let events = axis(&mut t, Axis::LeftStickY, 1.0);
        let (x, y) = (
            value_of(&events, GamepadAxis::LeftStickX).unwrap(),
            value_of(&events, GamepadAxis::LeftStickY).unwrap(),
        );
        assert!(
            (x - y).abs() < 1e-5 && (x * x + y * y).sqrt() <= 1.0 + 1e-5,
            "a diagonal: {x} {y}"
        );
        // Letting go returns both to exactly zero.
        axis(&mut t, Axis::LeftStickX, 0.0);
        let events = axis(&mut t, Axis::LeftStickY, 0.0);
        assert_eq!(value_of(&events, GamepadAxis::LeftStickY), Some(0.0));
    }

    #[test]
    fn the_two_sticks_are_independent_and_up_is_positive() {
        let mut t = connected();
        let right = axis(&mut t, Axis::RightStickY, 0.9);
        assert!(
            value_of(&right, GamepadAxis::RightStickY).unwrap() > 0.8,
            "gilrs reports up as positive, as the contract does"
        );
        assert!(value_of(&right, GamepadAxis::LeftStickY).is_none());
        let left = axis(&mut t, Axis::LeftStickX, -0.9);
        assert!(value_of(&left, GamepadAxis::LeftStickX).unwrap() < -0.8);
        assert!(
            value_of(&left, GamepadAxis::RightStickY).is_none(),
            "the other stick is not re-reported"
        );
    }

    #[test]
    fn tiny_changes_are_not_reported_but_zero_always_is() {
        let mut t = connected();
        axis(&mut t, Axis::LeftStickX, 0.5);
        assert!(
            axis(&mut t, Axis::LeftStickX, 0.5001).is_empty(),
            "noise in the last digits is not an event"
        );
        assert!(!axis(&mut t, Axis::LeftStickX, 0.52).is_empty());
        axis(&mut t, Axis::LeftStickX, 0.16);
        let back = axis(&mut t, Axis::LeftStickX, 0.0);
        assert_eq!(
            value_of(&back, GamepadAxis::LeftStickX),
            Some(0.0),
            "a stick that returns to rest says so even if the last step was small"
        );
    }

    #[test]
    fn a_stick_that_settles_just_off_zero_still_gets_its_final_zero() {
        let mut t = connected();
        axis(&mut t, Axis::LeftStickX, 0.5);
        // A tiny deflection just outside the dead zone reads about 0.004: under the noise limit from zero.
        let near = axis(&mut t, Axis::LeftStickX, 0.1535);
        let last = value_of(&near, GamepadAxis::LeftStickX).unwrap();
        assert!(last > 0.0 && last < 0.005, "{last}");
        let rest = axis(&mut t, Axis::LeftStickX, 0.0);
        assert_eq!(
            value_of(&rest, GamepadAxis::LeftStickX),
            Some(0.0),
            "the app must not be left with a stick stuck at {last}"
        );
    }

    #[test]
    fn analog_triggers_come_from_button_values_with_their_own_dead_zone() {
        let mut t = connected();
        assert!(
            t.translate(
                PAD,
                PadEvent::ButtonValue {
                    button: Button::LeftTrigger2,
                    value: 0.02
                }
            )
            .is_empty()
        );
        let events = t.translate(
            PAD,
            PadEvent::ButtonValue {
                button: Button::LeftTrigger2,
                value: 1.0,
            },
        );
        assert_eq!(value_of(&events, GamepadAxis::LeftTrigger), Some(1.0));
        let right = t.translate(
            PAD,
            PadEvent::ButtonValue {
                button: Button::RightTrigger2,
                value: 0.525,
            },
        );
        assert!((value_of(&right, GamepadAxis::RightTrigger).unwrap() - 0.5).abs() < 1e-4);
        // A trigger that is also reported as a button press does not produce a button event.
        assert!(
            t.translate(
                PAD,
                PadEvent::Button {
                    button: Button::LeftTrigger2,
                    pressed: true
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn a_pad_that_reports_its_d_pad_as_a_hat_axis_gets_d_pad_buttons() {
        let mut t = connected();
        let right = t.translate(
            PAD,
            PadEvent::Axis {
                axis: Axis::DPadX,
                value: 1.0,
            },
        );
        assert_eq!(
            right,
            vec![InputEvent::GamepadButton {
                pad: PAD,
                button: GamepadButton::DPadRight,
                pressed: true
            }]
        );
        let swing = t.translate(
            PAD,
            PadEvent::Axis {
                axis: Axis::DPadX,
                value: -1.0,
            },
        );
        assert_eq!(
            swing.len(),
            2,
            "right is released and left pressed: {swing:?}"
        );
        assert!(swing.contains(&InputEvent::GamepadButton {
            pad: PAD,
            button: GamepadButton::DPadRight,
            pressed: false
        }));
        assert!(swing.contains(&InputEvent::GamepadButton {
            pad: PAD,
            button: GamepadButton::DPadLeft,
            pressed: true
        }));
        let centre = t.translate(
            PAD,
            PadEvent::Axis {
                axis: Axis::DPadX,
                value: 0.0,
            },
        );
        assert_eq!(
            centre,
            vec![InputEvent::GamepadButton {
                pad: PAD,
                button: GamepadButton::DPadLeft,
                pressed: false
            }]
        );
        let up = t.translate(
            PAD,
            PadEvent::Axis {
                axis: Axis::DPadY,
                value: 1.0,
            },
        );
        assert_eq!(
            up,
            vec![InputEvent::GamepadButton {
                pad: PAD,
                button: GamepadButton::DPadUp,
                pressed: true
            }],
            "up is positive"
        );
    }

    #[test]
    fn a_disconnect_releases_everything_the_pad_held_and_zeroes_its_axes_first() {
        let mut t = connected();
        t.translate(
            PAD,
            PadEvent::Button {
                button: Button::South,
                pressed: true,
            },
        );
        t.translate(
            PAD,
            PadEvent::Button {
                button: Button::DPadUp,
                pressed: true,
            },
        );
        axis(&mut t, Axis::LeftStickX, 1.0);
        let events = t.translate(PAD, PadEvent::Disconnected);
        assert_eq!(
            events.last(),
            Some(&InputEvent::GamepadDisconnected(PAD)),
            "the disconnect comes last"
        );
        let released = events
            .iter()
            .filter(|e| matches!(e, InputEvent::GamepadButton { pressed: false, .. }))
            .count();
        assert_eq!(released, 2, "{events:?}");
        assert_eq!(value_of(&events, GamepadAxis::LeftStickX), Some(0.0));
        assert!(
            t.translate(PAD, PadEvent::Disconnected).is_empty(),
            "a second disconnect says nothing"
        );
        assert_eq!(t.connected(), 0);
        // Pressed again after a reconnect, the same button is a fresh press.
        let again = t.translate(
            PAD,
            PadEvent::Button {
                button: Button::South,
                pressed: true,
            },
        );
        assert_eq!(again.len(), 2, "connect and press: {again:?}");
    }

    #[test]
    fn two_pads_do_not_share_state() {
        let mut t = translator();
        let (a, b) = (GamepadId(0), GamepadId(1));
        t.translate(
            a,
            PadEvent::Button {
                button: Button::South,
                pressed: true,
            },
        );
        let b_press = t.translate(
            b,
            PadEvent::Button {
                button: Button::South,
                pressed: true,
            },
        );
        assert!(b_press.contains(&InputEvent::GamepadButton {
            pad: b,
            button: GamepadButton::South,
            pressed: true
        }));
        let drop_a = t.translate(a, PadEvent::Disconnected);
        assert!(
            drop_a
                .iter()
                .all(|e| !matches!(e, InputEvent::GamepadButton { pad, .. } if *pad == b))
        );
        assert_eq!(t.connected(), 1);
    }

    #[test]
    fn bad_values_are_ignored_and_a_configured_dead_zone_applies() {
        let mut t = connected();
        assert!(axis(&mut t, Axis::LeftStickX, f32::NAN).is_empty());
        assert!(axis(&mut t, Axis::LeftStickX, f32::INFINITY).is_empty());
        let mut wide = GamepadTranslator::new(GamepadConfig {
            stick_dead_zone: 0.4,
            ..GamepadConfig::default()
        });
        wide.translate(PAD, PadEvent::Connected);
        assert!(
            wide.translate(
                PAD,
                PadEvent::Axis {
                    axis: Axis::LeftStickX,
                    value: 0.35
                }
            )
            .is_empty()
        );
        assert!(
            !wide
                .translate(
                    PAD,
                    PadEvent::Axis {
                        axis: Axis::LeftStickX,
                        value: 0.5
                    }
                )
                .is_empty()
        );
        // A silly dead zone is limited rather than dividing by zero.
        let mut silly = GamepadTranslator::new(GamepadConfig {
            stick_dead_zone: 5.0,
            trigger_dead_zone: -1.0,
        });
        silly.translate(PAD, PadEvent::Connected);
        let events = silly.translate(
            PAD,
            PadEvent::Axis {
                axis: Axis::LeftStickX,
                value: 1.0,
            },
        );
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, InputEvent::GamepadAxis { value, .. } if !value.is_finite()))
        );
        assert_eq!(
            value_of(&events, GamepadAxis::LeftStickX),
            Some(1.0),
            "a silly dead zone is limited, so full deflection still reads full"
        );
    }

    // ---- the real source -------------------------------------------------------------------

    #[test]
    fn a_source_can_be_built_and_polled_without_any_pad_and_bad_mappings_do_not_panic() {
        let Ok(mut source) = GamepadSource::new(
            GamepadConfig::default(),
            Some("not a mapping line\n# a comment\n"),
        ) else {
            return; // the platform has no gamepad support here (a headless machine): nothing to test
        };
        let mut events = Vec::new();
        source.poll(&mut events);
        for e in &events {
            assert!(matches!(
                e,
                InputEvent::GamepadConnected(_)
                    | InputEvent::GamepadButton { .. }
                    | InputEvent::GamepadAxis { .. }
                    | InputEvent::GamepadDisconnected(_)
            ));
        }
        assert!(
            !source.rumble(
                GamepadId(9999),
                1.0,
                1.0,
                std::time::Duration::from_millis(10)
            ),
            "rumble on a pad that is not there is refused, not a panic"
        );
        assert!(source.name(GamepadId(9999)).is_none());
    }
}
