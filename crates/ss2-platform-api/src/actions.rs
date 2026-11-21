//! Input **actions**: what gameplay and UI code asks about ("is `fire` held?"), as opposed to
//! raw keys. Bindings from physical inputs to actions are data, so players can rebind.

use std::borrow::Borrow;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::keys::{GamepadAxis, GamepadButton, KeyCode, MouseButton};

/// The name of an action, such as `"fire"` or `"move_forward"`. Names are data (they come from
/// action-map files and mods), so this is a string, not an enum.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionId(String);

impl ActionId {
    /// An action named `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ActionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ActionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Which set of bindings is live. Only one context receives input at a time (the console
/// swallows `Game` input while open).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputContext {
    /// In-world gameplay.
    Game,
    /// Menus and HUD navigation.
    Menu,
    /// The developer console / chat text entry.
    Console,
}

/// One physical input that can drive an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Binding {
    /// A keyboard key.
    Key(KeyCode),
    /// A mouse button.
    Mouse(MouseButton),
    /// A gamepad button (any connected pad).
    GamepadButton(GamepadButton),
    /// One direction of a gamepad axis: `positive: true` is the `+` half (right/up/pulled).
    GamepadAxis {
        /// The axis.
        axis: GamepadAxis,
        /// Which half of the axis counts.
        positive: bool,
    },
}

/// A set of rebindable bindings for one [`InputContext`]: action → bindings. Several bindings per
/// action (keyboard **and** gamepad) are normal. Stored in a `BTreeMap` so the serialized form and
/// iteration order are stable.
///
/// ```
/// use ss2_platform_api::{ActionMap, Binding, KeyCode, MouseButton};
///
/// let mut map = ActionMap::default();
/// map.bind("fire", Binding::Mouse(MouseButton::Left));
/// map.bind("fire", Binding::Key(KeyCode::Space));
/// assert_eq!(map.bindings("fire").len(), 2);
/// assert!(map.bindings("jump").is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionMap {
    actions: BTreeMap<ActionId, Vec<Binding>>,
}

impl ActionMap {
    /// Add `binding` to `action` (no effect if it is already bound there).
    pub fn bind(&mut self, action: impl Into<ActionId>, binding: Binding) {
        let list = self.actions.entry(action.into()).or_default();
        if !list.contains(&binding) {
            list.push(binding);
        }
    }

    /// Remove `binding` from `action`; returns whether it was bound.
    pub fn unbind(&mut self, action: &str, binding: Binding) -> bool {
        let Some(list) = self.actions.get_mut(action) else {
            return false;
        };
        let before = list.len();
        list.retain(|b| *b != binding);
        let removed = list.len() != before;
        if list.is_empty() {
            self.actions.remove(action);
        }
        removed
    }

    /// Remove all bindings of `action` (the first half of "rebind").
    pub fn clear(&mut self, action: &str) {
        self.actions.remove(action);
    }

    /// The bindings of `action` (empty if none), in the order they were added.
    pub fn bindings(&self, action: &str) -> &[Binding] {
        self.actions.get(action).map_or(&[], Vec::as_slice)
    }

    /// Every bound action, in name order.
    pub fn actions(&self) -> impl Iterator<Item = &ActionId> {
        self.actions.keys()
    }

    /// Actions that use `binding` (to warn about conflicts when rebinding).
    pub fn actions_using(&self, binding: Binding) -> Vec<&ActionId> {
        self.actions
            .iter()
            .filter(|(_, list)| list.contains(&binding))
            .map(|(a, _)| a)
            .collect()
    }
}

/// What game and UI code reads each frame. The real implementation (T03-005) folds
/// [`InputEvent`](crate::InputEvent)s through the live [`ActionMap`] and
/// [`InputContext`]; code written against this trait runs unchanged on [`MockInput`].
///
/// Edge queries (`just_*`) are relative to the previous frame and are true for exactly one frame.
pub trait ActionQuery {
    /// `true` while the action is held.
    fn pressed(&self, action: &str) -> bool;
    /// `true` on the first frame the action is held.
    fn just_pressed(&self, action: &str) -> bool;
    /// `true` on the first frame after the action was released.
    fn just_released(&self, action: &str) -> bool;
    /// Analog value: `0.0..=1.0` for buttons and triggers (buttons are `0` or `1`); sticks are
    /// expressed as two bound actions (`move_left` / `move_right`), never as a negative value.
    fn value(&self, action: &str) -> f32;
    /// The context currently receiving input.
    fn context(&self) -> InputContext;
}

/// An [`ActionQuery`] driven by hand, for testing gameplay and UI code with no devices.
///
/// ```
/// use ss2_platform_api::{ActionQuery, MockInput};
///
/// let mut input = MockInput::new();
/// input.set_pressed("fire", true);
/// assert!(input.just_pressed("fire"));
/// input.next_frame();                      // one frame later...
/// assert!(input.pressed("fire") && !input.just_pressed("fire"));
/// input.set_pressed("fire", false);
/// assert!(input.just_released("fire"));
/// ```
#[derive(Clone, Debug)]
pub struct MockInput {
    context: InputContext,
    now: BTreeMap<ActionId, f32>,
    before: BTreeMap<ActionId, f32>,
}

impl Default for MockInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MockInput {
    /// No actions held, [`InputContext::Game`].
    pub fn new() -> Self {
        Self {
            context: InputContext::Game,
            now: BTreeMap::new(),
            before: BTreeMap::new(),
        }
    }

    /// Hold or release a button-style action.
    pub fn set_pressed(&mut self, action: &str, pressed: bool) {
        self.set_value(action, if pressed { 1.0 } else { 0.0 });
    }

    /// Set an analog value (clamped to `0.0..=1.0`; non-finite counts as `0`). An action counts as
    /// pressed when its value is above `0.5`.
    pub fn set_value(&mut self, action: &str, value: f32) {
        let v = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.now.insert(ActionId::new(action), v);
    }

    /// Switch the live context.
    pub fn set_context(&mut self, context: InputContext) {
        self.context = context;
    }

    /// End the frame: what is held now becomes "last frame" for the edge queries.
    pub fn next_frame(&mut self) {
        self.before = self.now.clone();
    }

    fn held(map: &BTreeMap<ActionId, f32>, action: &str) -> bool {
        map.get(action).is_some_and(|v| *v > 0.5)
    }
}

impl ActionQuery for MockInput {
    fn pressed(&self, action: &str) -> bool {
        Self::held(&self.now, action)
    }

    fn just_pressed(&self, action: &str) -> bool {
        Self::held(&self.now, action) && !Self::held(&self.before, action)
    }

    fn just_released(&self, action: &str) -> bool {
        !Self::held(&self.now, action) && Self::held(&self.before, action)
    }

    fn value(&self, action: &str) -> f32 {
        self.now.get(action).copied().unwrap_or(0.0)
    }

    fn context(&self) -> InputContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ActionMap {
        let mut m = ActionMap::default();
        m.bind("fire", Binding::Mouse(MouseButton::Left));
        m.bind(
            "fire",
            Binding::GamepadAxis {
                axis: GamepadAxis::RightTrigger,
                positive: true,
            },
        );
        m.bind("jump", Binding::Key(KeyCode::Space));
        m.bind("jump", Binding::GamepadButton(GamepadButton::South));
        m.bind("console", Binding::Key(KeyCode::Backquote));
        m
    }

    #[test]
    fn bind_is_idempotent_and_ordered() {
        let mut m = sample();
        m.bind("jump", Binding::Key(KeyCode::Space));
        assert_eq!(
            m.bindings("jump"),
            [
                Binding::Key(KeyCode::Space),
                Binding::GamepadButton(GamepadButton::South)
            ]
        );
        let names: Vec<_> = m.actions().map(ActionId::as_str).collect();
        assert_eq!(names, ["console", "fire", "jump"], "name order");
    }

    #[test]
    fn unbind_clear_and_conflicts() {
        let mut m = sample();
        assert!(m.unbind("jump", Binding::Key(KeyCode::Space)));
        assert!(!m.unbind("jump", Binding::Key(KeyCode::Space)));
        assert!(!m.unbind("nope", Binding::Key(KeyCode::A)));
        assert_eq!(
            m.actions_using(Binding::GamepadButton(GamepadButton::South))
                .len(),
            1
        );
        m.clear("jump");
        assert!(m.bindings("jump").is_empty());
        assert!(
            m.actions_using(Binding::GamepadButton(GamepadButton::South))
                .is_empty()
        );
    }

    #[test]
    fn action_map_round_trips_through_toml() {
        let map = sample();
        let text = toml::to_string(&map).unwrap();
        let back: ActionMap = toml::from_str(&text).unwrap();
        assert_eq!(back, map, "{text}");
        assert!(text.contains("fire"), "{text}");
    }

    #[test]
    fn unknown_binding_in_a_file_is_an_error_not_a_panic() {
        let bad = "[actions]\nfire = [{ Key = \"NoSuchKey\" }]\n";
        assert!(toml::from_str::<ActionMap>(bad).is_err());
    }

    #[test]
    fn mock_edges_last_exactly_one_frame() {
        let mut i = MockInput::new();
        assert!(!i.pressed("fire") && !i.just_pressed("fire") && !i.just_released("fire"));
        i.set_pressed("fire", true);
        assert!(i.pressed("fire") && i.just_pressed("fire"));
        i.next_frame();
        assert!(i.pressed("fire") && !i.just_pressed("fire"));
        i.set_pressed("fire", false);
        assert!(!i.pressed("fire") && i.just_released("fire"));
        i.next_frame();
        assert!(!i.just_released("fire"));
    }

    #[test]
    fn mock_analog_values_are_clamped_and_threshold_pressed() {
        let mut i = MockInput::new();
        i.set_value("fire", 0.4);
        assert!(!i.pressed("fire") && (i.value("fire") - 0.4).abs() < 1e-6);
        i.set_value("fire", 7.0);
        assert!(i.pressed("fire") && i.value("fire") == 1.0);
        i.set_value("fire", f32::NAN);
        assert_eq!(i.value("fire"), 0.0);
        assert_eq!(i.value("unknown"), 0.0);
    }

    #[test]
    fn mock_context_and_trait_object() {
        let mut i = MockInput::new();
        assert_eq!(i.context(), InputContext::Game);
        i.set_context(InputContext::Console);
        let q: &dyn ActionQuery = &i;
        assert_eq!(q.context(), InputContext::Console);
    }
}
