//! Placeholder color theme (not original art).

use crate::draw::Color;

/// Window clear color behind the menu.
pub const BACKGROUND: Color = Color([0.06, 0.06, 0.08, 1.0]);
/// Unfocused item background.
pub const IDLE: Color = Color([0.16, 0.17, 0.20, 1.0]);
/// Focused item background.
pub const FOCUSED: Color = Color([0.85, 0.45, 0.10, 1.0]);
/// Text color.
pub const TEXT: Color = Color([1.0, 1.0, 1.0, 1.0]);
/// Slider track background.
pub const TRACK: Color = Color([0.05, 0.05, 0.06, 1.0]);
/// Slider filled part.
pub const TRACK_FILL: Color = Color([0.95, 0.95, 0.95, 1.0]);
