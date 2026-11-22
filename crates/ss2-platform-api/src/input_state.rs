//! The live input state that answers [`ActionQuery`] from real events (T03-005).
//!
//! [`InputState`] is what the platform loop feeds with [`InputEvent`]s and
//! [`LifecycleEvent`]s; game and UI code only ever see it through [`ActionQuery`] (and tests use
//! [`MockInput`](crate::MockInput) instead). It keeps one [`ActionMap`] per [`InputContext`] and
//! answers for the live context only, so the console swallows game input without any code in
//! the game. [`BindingSet`] is the on-disk form of all the maps (`bindings.toml`).
//!
//! ```
//! use ss2_platform_api::{ActionMap, ActionQuery, Binding, InputContext, InputEvent, InputState, KeyCode};
//!
//! let mut map = ActionMap::default();
//! map.bind("fire", Binding::Key(KeyCode::Space));
//! let mut input = InputState::default();
//! input.set_map(InputContext::Game, map);
//!
//! input.handle_event(&InputEvent::Key { key: KeyCode::Space, pressed: true, repeat: false });
//! assert!(input.just_pressed("fire"));
//! input.next_frame();
//! assert!(input.pressed("fire") && !input.just_pressed("fire"));
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ss2_core::{Error, Result};

use crate::{
    ActionMap, ActionQuery, Binding, GamepadAxis, GamepadButton, GamepadId, InputContext,
    InputEvent, KeyCode, LifecycleEvent, MouseButton,
};

/// An action counts as pressed when its value is above this (as in [`MockInput`](crate::MockInput)).
const PRESSED_ABOVE: f32 = 0.5;

static EMPTY_MAP: ActionMap = ActionMap::empty();

/// The bindings of every context: what `bindings.toml` holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BindingSet(BTreeMap<InputContext, ActionMap>);

impl BindingSet {
    /// Set the map of `context`.
    pub fn insert(&mut self, context: InputContext, map: ActionMap) {
        self.0.insert(context, map);
    }

    /// The map of `context`, if the set has one.
    pub fn get(&self, context: InputContext) -> Option<&ActionMap> {
        self.0.get(&context)
    }

    /// `true` if no context has a map.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The TOML text of the set.
    ///
    /// # Errors
    /// [`Error::Invalid`] if the set cannot be written (it cannot happen for valid maps).
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| Error::invalid("key bindings", e.to_string()))
    }

    /// Read a set from TOML text; an empty text is an empty set.
    ///
    /// # Errors
    /// [`Error::Invalid`] for text that is not valid TOML or names an unknown context or input.
    pub fn from_toml_str(text: &str) -> Result<Self> {
        toml::from_str(text).map_err(|e| Error::invalid("key bindings", e.to_string()))
    }
}

/// The held buttons and axis values, mapped through the live context's [`ActionMap`].
#[derive(Debug)]
pub struct InputState {
    maps: BTreeMap<InputContext, ActionMap>,
    context: InputContext,
    keys: BTreeSet<KeyCode>,
    mouse: BTreeSet<MouseButton>,
    pad_buttons: BTreeSet<(GamepadId, GamepadButton)>,
    axes: BTreeMap<(GamepadId, GamepadAxis), f32>,
    /// The live context's action values at the end of the previous frame (for the edge queries).
    before: BTreeMap<String, f32>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            maps: BTreeMap::new(),
            context: InputContext::Game,
            keys: BTreeSet::new(),
            mouse: BTreeSet::new(),
            pad_buttons: BTreeSet::new(),
            axes: BTreeMap::new(),
            before: BTreeMap::new(),
        }
    }
}

impl InputState {
    /// Replace the bindings of `context`.
    pub fn set_map(&mut self, context: InputContext, map: ActionMap) {
        self.maps.insert(context, map);
        self.snapshot();
    }

    /// The bindings of `context` (empty if it has none).
    pub fn map(&self, context: InputContext) -> &ActionMap {
        self.maps.get(&context).unwrap_or(&EMPTY_MAP)
    }

    /// The bindings of `context` for rebinding (created empty if missing).
    pub fn map_mut(&mut self, context: InputContext) -> &mut ActionMap {
        self.maps.entry(context).or_default()
    }

    /// Every context's bindings.
    pub fn maps(&self) -> impl Iterator<Item = (InputContext, &ActionMap)> {
        self.maps.iter().map(|(c, m)| (*c, m))
    }

    /// Replace all bindings with `set` (for example just loaded from `bindings.toml`).
    pub fn load(&mut self, set: &BindingSet) {
        self.maps = set.0.clone();
        self.snapshot();
    }

    /// All bindings as a [`BindingSet`], to save.
    pub fn binding_set(&self) -> BindingSet {
        BindingSet(self.maps.clone())
    }

    /// Make `context` live. Inputs that are already held do not produce a "just pressed" edge
    /// in the new context (typing the key that opens the console must not also fire).
    pub fn set_context(&mut self, context: InputContext) {
        self.context = context;
        self.snapshot();
    }

    /// Feed one platform input event.
    pub fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Key {
                key,
                pressed,
                repeat,
            } => {
                if !repeat {
                    toggle(&mut self.keys, *key, *pressed);
                }
            }
            InputEvent::MouseButton { button, pressed } => {
                toggle(&mut self.mouse, *button, *pressed);
            }
            InputEvent::GamepadButton {
                pad,
                button,
                pressed,
            } => toggle(&mut self.pad_buttons, (*pad, *button), *pressed),
            InputEvent::GamepadAxis { pad, axis, value } => {
                let value = if value.is_finite() {
                    value.clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                if value == 0.0 {
                    self.axes.remove(&(*pad, *axis));
                } else {
                    self.axes.insert((*pad, *axis), value);
                }
            }
            InputEvent::GamepadDisconnected(pad) => {
                self.pad_buttons.retain(|(p, _)| p != pad);
                self.axes.retain(|(p, _), _| p != pad);
            }
            _ => {}
        }
    }

    /// Feed a lifecycle event: losing focus releases everything that is held.
    pub fn handle_lifecycle(&mut self, event: &LifecycleEvent) {
        if matches!(event, LifecycleEvent::FocusChanged(false)) {
            self.keys.clear();
            self.mouse.clear();
            self.pad_buttons.clear();
            self.axes.clear();
        }
    }

    /// End the frame: what is held now becomes "last frame" for the edge queries.
    pub fn next_frame(&mut self) {
        self.snapshot();
    }

    fn snapshot(&mut self) {
        self.before = self
            .map(self.context)
            .actions()
            .map(|id| (id.as_str().to_owned(), self.live_value(id.as_str())))
            .collect();
    }

    fn binding_value(&self, binding: Binding) -> f32 {
        let held = |down: bool| if down { 1.0 } else { 0.0 };
        match binding {
            Binding::Key(key) => held(self.keys.contains(&key)),
            Binding::Mouse(button) => held(self.mouse.contains(&button)),
            Binding::GamepadButton(button) => {
                held(self.pad_buttons.iter().any(|(_, b)| *b == button))
            }
            Binding::GamepadAxis { axis, positive } => self
                .axes
                .iter()
                .filter(|((_, a), _)| *a == axis)
                .map(|(_, v)| if positive { *v } else { -*v })
                .fold(0.0, f32::max),
        }
    }

    fn live_value(&self, action: &str) -> f32 {
        self.map(self.context)
            .bindings(action)
            .iter()
            .map(|b| self.binding_value(*b))
            .fold(0.0, f32::max)
    }

    fn before_pressed(&self, action: &str) -> bool {
        self.before.get(action).copied().unwrap_or(0.0) > PRESSED_ABOVE
    }
}

fn toggle<T: Ord>(set: &mut BTreeSet<T>, item: T, down: bool) {
    if down {
        set.insert(item);
    } else {
        set.remove(&item);
    }
}

impl ActionQuery for InputState {
    fn pressed(&self, action: &str) -> bool {
        self.live_value(action) > PRESSED_ABOVE
    }

    fn just_pressed(&self, action: &str) -> bool {
        self.pressed(action) && !self.before_pressed(action)
    }

    fn just_released(&self, action: &str) -> bool {
        !self.pressed(action) && self.before_pressed(action)
    }

    fn value(&self, action: &str) -> f32 {
        self.live_value(action)
    }

    fn context(&self) -> InputContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionId, GamepadAxis, GamepadButton, GamepadId, KeyCode, MouseButton};

    const PAD: GamepadId = GamepadId(1);

    fn key(key: KeyCode, pressed: bool) -> InputEvent {
        InputEvent::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn axis(pad: GamepadId, axis: GamepadAxis, value: f32) -> InputEvent {
        InputEvent::GamepadAxis { pad, axis, value }
    }

    fn state() -> InputState {
        let mut game = ActionMap::default();
        game.bind("fire", Binding::Mouse(MouseButton::Left));
        game.bind("fire", Binding::Key(KeyCode::Space));
        game.bind("jump", Binding::GamepadButton(GamepadButton::South));
        game.bind(
            "move_right",
            Binding::GamepadAxis {
                axis: GamepadAxis::LeftStickX,
                positive: true,
            },
        );
        game.bind(
            "move_left",
            Binding::GamepadAxis {
                axis: GamepadAxis::LeftStickX,
                positive: false,
            },
        );
        game.bind(
            "alt_fire",
            Binding::GamepadAxis {
                axis: GamepadAxis::RightTrigger,
                positive: true,
            },
        );
        let mut menu = ActionMap::default();
        menu.bind("accept", Binding::Key(KeyCode::Enter));
        let mut s = InputState::default();
        s.set_map(InputContext::Game, game);
        s.set_map(InputContext::Menu, menu);
        s
    }

    #[test]
    fn a_key_goes_through_press_hold_and_release_with_one_frame_edges() {
        let mut s = state();
        s.handle_event(&key(KeyCode::Space, true));
        assert!(s.pressed("fire") && s.just_pressed("fire") && !s.just_released("fire"));
        s.next_frame();
        assert!(s.pressed("fire") && !s.just_pressed("fire"));
        s.handle_event(&key(KeyCode::Space, false));
        assert!(!s.pressed("fire") && s.just_released("fire"));
        s.next_frame();
        assert!(!s.pressed("fire") && !s.just_released("fire"));
    }

    #[test]
    fn os_key_repeat_changes_nothing() {
        let mut s = state();
        s.handle_event(&key(KeyCode::Space, true));
        s.next_frame();
        s.handle_event(&InputEvent::Key {
            key: KeyCode::Space,
            pressed: true,
            repeat: true,
        });
        assert!(s.pressed("fire") && !s.just_pressed("fire"));
    }

    #[test]
    fn several_bindings_for_one_action_work_as_an_or() {
        let mut s = state();
        s.handle_event(&InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        });
        s.handle_event(&key(KeyCode::Space, true));
        s.next_frame();
        s.handle_event(&InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: false,
        });
        assert!(s.pressed("fire"), "the key still holds the action");
        assert!(!s.just_released("fire"));
        s.handle_event(&key(KeyCode::Space, false));
        assert!(!s.pressed("fire") && s.just_released("fire"));
    }

    #[test]
    fn axis_halves_are_separate_actions_and_pressed_means_over_half() {
        let mut s = state();
        s.handle_event(&axis(PAD, GamepadAxis::LeftStickX, 0.8));
        assert_eq!(s.value("move_right"), 0.8);
        assert_eq!(s.value("move_left"), 0.0);
        assert!(s.pressed("move_right") && !s.pressed("move_left"));
        s.handle_event(&axis(PAD, GamepadAxis::LeftStickX, -0.3));
        assert_eq!(s.value("move_left"), 0.3);
        assert!(
            !s.pressed("move_left"),
            "0.3 is below the pressed threshold"
        );
        assert_eq!(s.value("move_right"), 0.0);
    }

    #[test]
    fn a_trigger_gives_an_analog_value_and_non_finite_axes_count_as_zero() {
        let mut s = state();
        s.handle_event(&axis(PAD, GamepadAxis::RightTrigger, 0.6));
        assert_eq!(s.value("alt_fire"), 0.6);
        assert!(s.pressed("alt_fire"));
        s.handle_event(&axis(PAD, GamepadAxis::RightTrigger, f32::NAN));
        assert_eq!(s.value("alt_fire"), 0.0);
        s.handle_event(&axis(PAD, GamepadAxis::RightTrigger, 7.0));
        assert_eq!(s.value("alt_fire"), 1.0, "clamped");
    }

    #[test]
    fn buttons_have_the_value_one_and_unknown_actions_are_never_active() {
        let mut s = state();
        s.handle_event(&InputEvent::GamepadButton {
            pad: PAD,
            button: GamepadButton::South,
            pressed: true,
        });
        assert_eq!(s.value("jump"), 1.0);
        assert!(!s.pressed("nope") && !s.just_pressed("nope") && s.value("nope") == 0.0);
    }

    #[test]
    fn only_the_live_contexts_bindings_count() {
        let mut s = state();
        s.handle_event(&key(KeyCode::Enter, true));
        s.handle_event(&key(KeyCode::Space, true));
        assert_eq!(s.context(), InputContext::Game);
        assert!(s.pressed("fire") && !s.pressed("accept"));

        s.set_context(InputContext::Menu);
        assert_eq!(s.context(), InputContext::Menu);
        assert!(s.pressed("accept"), "Enter is still held");
        assert!(!s.just_pressed("accept"), "no edge from a context switch");
        assert!(!s.pressed("fire"));

        s.set_context(InputContext::Console);
        assert!(
            !s.pressed("accept") && !s.pressed("fire"),
            "Console has no bindings"
        );
    }

    #[test]
    fn disconnecting_a_pad_releases_only_its_inputs() {
        let mut s = state();
        let other = GamepadId(2);
        for pad in [PAD, other] {
            s.handle_event(&InputEvent::GamepadButton {
                pad,
                button: GamepadButton::South,
                pressed: true,
            });
        }
        s.next_frame();
        s.handle_event(&InputEvent::GamepadDisconnected(PAD));
        assert!(s.pressed("jump"), "the other pad still holds it");
        s.handle_event(&InputEvent::GamepadDisconnected(other));
        assert!(!s.pressed("jump") && s.just_released("jump"));
    }

    #[test]
    fn losing_focus_releases_everything() {
        let mut s = state();
        s.handle_event(&key(KeyCode::Space, true));
        s.handle_event(&axis(PAD, GamepadAxis::LeftStickX, 1.0));
        s.next_frame();
        s.handle_lifecycle(&LifecycleEvent::FocusChanged(false));
        assert!(!s.pressed("fire") && s.just_released("fire"));
        assert!(!s.pressed("move_right") && s.just_released("move_right"));
        s.next_frame();
        s.handle_lifecycle(&LifecycleEvent::FocusChanged(true));
        assert!(
            !s.pressed("fire"),
            "regaining focus does not press anything"
        );
    }

    #[test]
    fn it_is_usable_wherever_an_action_query_is_expected() {
        fn fired(input: &impl ActionQuery) -> bool {
            input.just_pressed("fire")
        }
        let mut s = state();
        s.handle_event(&key(KeyCode::Space, true));
        assert!(fired(&s));
    }

    #[test]
    fn a_binding_set_survives_toml_and_rejects_nonsense() {
        let mut set = BindingSet::default();
        for (context, map) in state().maps() {
            set.insert(context, map.clone());
        }
        let text = set.to_toml_string().unwrap();
        assert!(text.contains("Game") && text.contains("Menu"), "{text}");
        let back = BindingSet::from_toml_str(&text).unwrap();
        assert_eq!(back, set);
        assert!(
            BindingSet::from_toml_str("").unwrap().is_empty(),
            "an empty file is fine"
        );
        assert!(BindingSet::from_toml_str("[Nowhere]\nfire = []").is_err());
        assert!(BindingSet::from_toml_str("not toml at all [").is_err());
    }

    #[test]
    fn loading_a_binding_set_replaces_the_maps() {
        let mut s = state();
        let mut set = BindingSet::default();
        let mut game = ActionMap::default();
        game.bind("fire", Binding::Key(KeyCode::F));
        set.insert(InputContext::Game, game);
        s.load(&set);
        s.handle_event(&key(KeyCode::Space, true));
        assert!(!s.pressed("fire"), "the old binding is gone");
        s.handle_event(&key(KeyCode::F, true));
        assert!(s.pressed("fire"));
        assert!(s.map(InputContext::Menu).actions().next().is_none());
    }

    #[test]
    fn an_exclusive_bind_takes_the_input_from_other_actions_and_reports_them() {
        let mut map = ActionMap::default();
        map.bind("fire", Binding::Key(KeyCode::Space));
        map.bind("jump", Binding::Key(KeyCode::Space));
        let taken = map.bind_exclusive("crouch", Binding::Key(KeyCode::Space));
        let names: Vec<&str> = taken.iter().map(ActionId::as_str).collect();
        assert_eq!(names, ["fire", "jump"]);
        assert_eq!(map.bindings("crouch"), [Binding::Key(KeyCode::Space)]);
        assert!(map.bindings("fire").is_empty() && map.bindings("jump").is_empty());
        assert!(
            map.bind_exclusive("crouch", Binding::Key(KeyCode::Space))
                .is_empty()
        );
    }
}
