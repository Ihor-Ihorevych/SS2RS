//! ss2-platform-api: the windowing-free platform contract (tracks T00/T03).
//!
//! This is the **v0 contract** (roadmap T00-016, `docs/rfcs/0003-platform-api.md`): the only
//! platform types the app, UI and game code may use. Nothing here mentions `winit`, `gilrs`, a
//! window handle or an OS, so the headless server compiles it and every consumer can be tested
//! with no devices. The real implementation (`ss2-platform`, T03-001..) translates the windowing
//! and gamepad libraries into these types; [`run_script`], [`MockInput`] and
//! [`LifecycleTracker`] stand in for it until then.
//!
//! - **Events** ([`PlatformEvent`]): [`InputEvent`] (keys, text, mouse, gamepad, touch) and
//!   [`LifecycleEvent`] (resume/suspend, surface ready/lost, resize, focus, close), built on
//!   device-independent [`KeyCode`], [`MouseButton`], [`GamepadButton`], [`GamepadAxis`].
//! - **Lifecycle** ([`LifecycleTracker`]): "may I render right now?" including Android-style
//!   surface loss and desktop minimize.
//! - **Input actions** ([`ActionMap`], [`ActionQuery`]): rebindable, serializable bindings, and
//!   the query trait gameplay reads, with [`MockInput`] for tests.
//! - **Locations** ([`PlatformPaths`]): config, saves, cache, logs and the game-data folder.
//! - **The loop** ([`AppHandler`], [`run_script`]): what the platform calls each event and frame.
//!
//! ```
//! use ss2_platform_api::{
//!     run_script, AppControl, AppHandler, FrameInfo, InputEvent, KeyCode, LifecycleEvent,
//!     LifecycleTracker, PlatformEvent, ScriptStep, SurfaceInfo,
//! };
//!
//! #[derive(Default)]
//! struct Game { life: LifecycleTracker, frames_drawn: u32 }
//!
//! impl AppHandler for Game {
//!     fn event(&mut self, event: PlatformEvent) -> AppControl {
//!         match event {
//!             PlatformEvent::Lifecycle(e) => { self.life.apply(&e); }
//!             PlatformEvent::Input(InputEvent::Key { key: KeyCode::Escape, pressed: true, .. }) => {
//!                 return AppControl::Exit;
//!             }
//!             _ => {}
//!         }
//!         AppControl::Continue
//!     }
//!     fn frame(&mut self, _: &FrameInfo) -> AppControl {
//!         if self.life.can_render() { self.frames_drawn += 1; }
//!         AppControl::Continue
//!     }
//! }
//!
//! let mut game = Game::default();
//! run_script(&mut game, [
//!     LifecycleEvent::Resumed.into(),
//!     LifecycleEvent::SurfaceReady(SurfaceInfo { width: 1280, height: 720, scale_factor: 1.0 }).into(),
//!     ScriptStep::Frame,
//! ]);
//! assert_eq!(game.frames_drawn, 1);
//! ```

mod actions;
mod app;
mod event;
mod keys;
mod lifecycle;
mod paths;

pub use actions::{ActionId, ActionMap, ActionQuery, Binding, InputContext, MockInput};
pub use app::{AppControl, AppHandler, FrameInfo, ScriptReport, ScriptStep, run_script};
pub use event::{InputEvent, LifecycleEvent, PlatformEvent, SurfaceInfo, TouchPhase};
pub use keys::{GamepadAxis, GamepadButton, GamepadId, KeyCode, MouseButton};
pub use lifecycle::LifecycleTracker;
pub use paths::{PlatformPaths, PortableMode, UserDirs, PORTABLE_DIR, PORTABLE_MARKER};
