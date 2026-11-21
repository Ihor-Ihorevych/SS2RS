//! Input types shared between the platform layer (producer) and the UI (consumer).

/// A device-independent navigation action: keyboard, gamepad and touch all map onto this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavInput {
    /// Move focus up.
    Up,
    /// Move focus down.
    Down,
    /// Move focus left / decrease a value.
    Left,
    /// Move focus right / increase a value.
    Right,
    /// Confirm / activate the focused item.
    Activate,
    /// Go back / cancel.
    Back,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn nav_input_is_copy_and_hashable() {
        let a = NavInput::Up;
        let b = a; // Copy
        let set: HashSet<NavInput> = [a, b, NavInput::Down].into_iter().collect();
        assert_eq!(set.len(), 2);
    }
}
