//! Everything the platform tells the application: input and lifecycle events.

use crate::keys::{GamepadAxis, GamepadButton, GamepadId, KeyCode, MouseButton};

/// Size and scale of the drawable surface, in **physical pixels**.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceInfo {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Physical pixels per logical pixel (HiDPI factor, `1.0` = 96 dpi).
    pub scale_factor: f32,
}

/// Phase of a touch (Phase 2: never produced before milestone M9, listed so UI code is written
/// without assuming a mouse).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    /// Finger down.
    Started,
    /// Finger moved.
    Moved,
    /// Finger lifted.
    Ended,
    /// The system took the touch away.
    Cancelled,
}

/// A raw input event. Coordinates are **window pixels, origin top-left, y down**.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum InputEvent {
    /// A physical key changed state. `repeat` is `true` for OS key-repeat presses.
    Key {
        /// Which key.
        key: KeyCode,
        /// `true` on press, `false` on release.
        pressed: bool,
        /// OS auto-repeat (only with `pressed == true`).
        repeat: bool,
    },
    /// Typed text (after IME and layout), for the console and chat. Not for gameplay keys.
    Text(String),
    /// Raw relative mouse movement (for aiming); valid while the cursor is captured.
    MouseMotion {
        /// Horizontal movement in device units.
        dx: f32,
        /// Vertical movement in device units, down positive.
        dy: f32,
    },
    /// The cursor moved to an absolute window position (for menus).
    CursorMoved {
        /// X in window pixels.
        x: f32,
        /// Y in window pixels.
        y: f32,
    },
    /// A mouse button changed state.
    MouseButton {
        /// Which button.
        button: MouseButton,
        /// `true` on press.
        pressed: bool,
    },
    /// Scroll wheel / trackpad, in lines (one notch = `1.0`).
    Scroll {
        /// Horizontal.
        dx: f32,
        /// Vertical, away from the user positive.
        dy: f32,
    },
    /// A gamepad was connected.
    GamepadConnected(GamepadId),
    /// A gamepad was disconnected; all its buttons count as released.
    GamepadDisconnected(GamepadId),
    /// A gamepad button changed state.
    GamepadButton {
        /// Which pad.
        pad: GamepadId,
        /// Which button.
        button: GamepadButton,
        /// `true` on press.
        pressed: bool,
    },
    /// A gamepad axis moved (dead-zone filtered).
    GamepadAxis {
        /// Which pad.
        pad: GamepadId,
        /// Which axis.
        axis: GamepadAxis,
        /// New value (sticks `-1..=1`, triggers `0..=1`).
        value: f32,
    },
    /// A touch (Phase 2).
    Touch {
        /// Identifies the finger for the duration of the touch.
        id: u64,
        /// What happened.
        phase: TouchPhase,
        /// X in window pixels.
        x: f32,
        /// Y in window pixels.
        y: f32,
    },
}

/// App lifecycle and window state changes. The ordering the platform guarantees is documented
/// on [`LifecycleTracker`](crate::LifecycleTracker).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum LifecycleEvent {
    /// The app is (again) allowed to run and use the GPU. First event on every platform.
    Resumed,
    /// The app must stop rendering and free GPU resources and audio output (Android `onPause`;
    /// desktop minimize does **not** send this). Always preceded by
    /// [`SurfaceLost`](Self::SurfaceLost).
    Suspended,
    /// A drawable surface exists (or was re-created) with this size.
    SurfaceReady(SurfaceInfo),
    /// The surface is gone; stop presenting and drop anything tied to it.
    SurfaceLost,
    /// The surface size changed. `0 x 0` means minimized: skip rendering.
    Resized {
        /// New width in physical pixels.
        width: u32,
        /// New height in physical pixels.
        height: u32,
    },
    /// The HiDPI scale changed (window moved to another monitor).
    ScaleFactorChanged {
        /// New physical-per-logical pixel ratio.
        scale_factor: f32,
    },
    /// The window gained or lost keyboard focus. Losing focus releases all inputs.
    FocusChanged(bool),
    /// The OS asks to free memory (caches first).
    LowMemory,
    /// The user asked to close (window close button, Alt+F4, Android back at root).
    CloseRequested,
}

/// One thing the platform tells the app.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PlatformEvent {
    /// Lifecycle / window state.
    Lifecycle(LifecycleEvent),
    /// Raw input.
    Input(InputEvent),
}

impl From<LifecycleEvent> for PlatformEvent {
    fn from(e: LifecycleEvent) -> Self {
        Self::Lifecycle(e)
    }
}

impl From<InputEvent> for PlatformEvent {
    fn from(e: InputEvent) -> Self {
        Self::Input(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_convert_into_platform_events() {
        let e: PlatformEvent = LifecycleEvent::LowMemory.into();
        assert_eq!(e, PlatformEvent::Lifecycle(LifecycleEvent::LowMemory));
        let e: PlatformEvent = InputEvent::Text("x".into()).into();
        assert!(matches!(e, PlatformEvent::Input(InputEvent::Text(_))));
    }
}
