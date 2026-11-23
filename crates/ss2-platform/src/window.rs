//! The window and the `winit` event loop, driving an application through the platform contract
//! (`ss2-platform-api`, RFC 0003).
//!
//! [`run`] creates the window, turns what `winit` reports into [`PlatformEvent`]s and calls
//! [`AppHandler::event`] / [`AppHandler::frame`]; the app answers with [`AppControl`]. The one thing
//! the windowing-free contract cannot carry is the window itself, which a renderer needs to create
//! its surface: that is the extra hook of [`PlatformApp`].
//!
//! Keyboard and mouse input arrive as [`InputEvent`]s (T03-002): physical keys with auto-repeat,
//! typed text, raw mouse motion for aiming, absolute cursor position for menus, buttons and
//! scrolling. Losing focus releases every held key and button, as the contract promises. Gamepads
//! (T03-003, see [`crate::gamepad`]) are polled once per loop iteration and arrive as the same
//! [`InputEvent`]s; the standard `SDL_GAMECONTROLLERCONFIG` environment variable adds pad mappings.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ss2_platform_api::{
    AppControl, AppHandler, DisplayPlan, DisplayRequest, FrameInfo, InputEvent, LifecycleEvent,
    MonitorInfo, PlatformEvent, SurfaceInfo, WindowMode,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, Ime, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

use crate::display;
use crate::gamepad::{GamepadConfig, GamepadSource};
use crate::input::{self, HeldInputs};
use crate::translate::{self, Observed, ResizeFilter};

/// Largest logical window size we accept (a typo guard, not a real limit).
const MAX_DIMENSION: u32 = 16_384;

/// Why [`run`] stopped abnormally.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// The [`WindowConfig`] is unusable.
    #[error("invalid window configuration: {0}")]
    InvalidConfig(String),
    /// The OS event loop could not be created or failed.
    #[error("event loop error: {0}")]
    EventLoop(String),
    /// The window could not be created.
    #[error("cannot create the window: {0}")]
    Window(String),
    /// The application refused to start (its [`PlatformApp::window_ready`] returned an error).
    #[error("application error: {0}")]
    App(String),
}

/// How to create the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowConfig {
    /// Title bar text.
    pub title: String,
    /// Initial client size in **logical** pixels.
    pub size: (u32, u32),
    /// Start borderless-fullscreen (shorthand for `display.mode = Borderless` when `display`
    /// is left at its default).
    pub fullscreen: bool,
    /// Which window mode, monitor, resolution and refresh rate to start with.
    pub display: DisplayRequest,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "SS2RS".to_owned(),
            size: (1280, 720),
            fullscreen: false,
            display: DisplayRequest::default(),
        }
    }
}

impl WindowConfig {
    /// The display request to start with: `display`, with `fullscreen` meaning borderless
    /// fullscreen when no other mode was asked for.
    pub fn display_request(&self) -> DisplayRequest {
        let mut request = self.display.clone();
        if self.fullscreen && request.mode == WindowMode::Windowed {
            request.mode = WindowMode::Borderless;
        }
        request
    }

    /// Check the values before touching the OS.
    ///
    /// # Errors
    /// [`PlatformError::InvalidConfig`] for an empty title or a size that is zero or absurd.
    pub fn validate(&self) -> Result<(), PlatformError> {
        if self.title.trim().is_empty() {
            return Err(PlatformError::InvalidConfig("the title is empty".into()));
        }
        let (w, h) = self.size;
        if w == 0 || h == 0 || w > MAX_DIMENSION || h > MAX_DIMENSION {
            return Err(PlatformError::InvalidConfig(format!(
                "size {w}x{h} must be within 1..={MAX_DIMENSION} in both directions"
            )));
        }
        Ok(())
    }
}

/// The live window, as handed to the application for creating a rendering surface and for the few
/// window operations an app needs. Cheap to clone.
#[derive(Clone)]
pub struct PlatformWindow {
    window: Arc<Window>,
    display: OwnedDisplayHandle,
    /// Shared with the loop, which must know whether the cursor is captured (to stop reporting
    /// absolute positions) and releases the capture when the window loses focus.
    captured: Arc<AtomicBool>,
}

impl PlatformWindow {
    /// The `winit` window (implements `raw-window-handle`; `wgpu` surfaces are created from it).
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// The display connection, needed by `wgpu` to create an instance on some platforms.
    pub fn display_handle(&self) -> OwnedDisplayHandle {
        self.display.clone()
    }

    /// Current client size in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        let s = self.window.inner_size();
        (s.width, s.height)
    }

    /// Physical pixels per logical pixel.
    pub fn scale_factor(&self) -> f32 {
        translate::sanitize_scale(self.window.scale_factor())
    }

    /// Switch between windowed and borderless fullscreen.
    pub fn set_fullscreen(&self, fullscreen: bool) {
        self.window
            .set_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)));
    }

    /// The connected monitors with their video modes, for the options screen.
    pub fn monitors(&self) -> Vec<MonitorInfo> {
        display::list_monitors(
            self.window.available_monitors(),
            self.window.primary_monitor().as_ref(),
        )
    }

    /// Apply a window mode, monitor, resolution and refresh rate. Wishes that cannot be met
    /// fall back (see [`ss2_platform_api::plan_display`]) and are logged; the plan that was
    /// really applied is returned.
    pub fn set_display(&self, request: &DisplayRequest) -> DisplayPlan {
        let (fullscreen, plan) = display::resolve_fullscreen(
            self.window.available_monitors(),
            self.window.primary_monitor().as_ref(),
            request,
        );
        for note in &plan.notes {
            tracing::warn!("{note}");
        }
        self.window.set_fullscreen(fullscreen);
        plan
    }

    /// Change the title bar text.
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    /// Capture the cursor for mouse-look: hide it and keep it from leaving the window, so
    /// [`InputEvent::MouseMotion`] keeps flowing and no absolute [`InputEvent::CursorMoved`] is
    /// sent meanwhile. Tries a true lock first and falls back to confining the cursor to the window
    /// (Windows only offers the latter). `false` releases it and shows the cursor again.
    ///
    /// **Losing focus releases the capture** (the OS drops the grab anyway); capture again from
    /// the `FocusChanged(true)` event if the game wants it back.
    ///
    /// # Errors
    /// The OS refused both grab modes; the cursor is left as it was.
    pub fn set_cursor_captured(&self, captured: bool) -> Result<(), String> {
        if captured {
            self.window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
                .map_err(|e| format!("cannot capture the cursor: {e}"))?;
            self.window.set_cursor_visible(false);
        } else {
            // Releasing cannot meaningfully fail; the cursor must come back regardless.
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            self.window.set_cursor_visible(true);
        }
        self.captured.store(captured, Ordering::Relaxed);
        Ok(())
    }

    /// Whether the cursor is currently captured.
    pub fn is_cursor_captured(&self) -> bool {
        self.captured.load(Ordering::Relaxed)
    }

    /// Allow input-method (IME) composition for the console and chat. Plain typing produces
    /// [`InputEvent::Text`] either way; this is for text that needs composing (CJK, accents).
    pub fn set_text_input(&self, enabled: bool) {
        self.window.set_ime_allowed(enabled);
    }
}

/// An [`AppHandler`] that also wants the window.
///
/// Call order at startup, which a renderer can rely on:
/// 1. `event(Lifecycle(Resumed))`
/// 2. [`window_ready`](Self::window_ready): create the surface from the window here
/// 3. `event(Lifecycle(SurfaceReady))`, then frames
///
/// When the surface goes away ([`SurfaceLost`](LifecycleEvent::SurfaceLost), a suspend, or the
/// loop ending) [`window_lost`](Self::window_lost) is called so the app can drop everything that
/// holds the window (the renderer) **before** the window is destroyed.
pub trait PlatformApp: AppHandler {
    /// The window exists. Return an error to abort startup; [`run`] then returns
    /// [`PlatformError::App`] with that message.
    ///
    /// # Errors
    /// Any message; it ends the loop.
    fn window_ready(&mut self, window: &PlatformWindow) -> Result<(), String>;

    /// The window is going away; release anything tied to it. Default: nothing.
    fn window_lost(&mut self) {}
}

/// Create the window and run the application until it asks to exit (or the window closes with
/// nobody handling `CloseRequested`: the app decides, so it must return [`AppControl::Exit`]).
///
/// Must be called on the main thread (an OS requirement on macOS and Windows event loops).
///
/// # Errors
/// [`PlatformError`] if the configuration is invalid, the event loop or window cannot be created,
/// or the app's `window_ready` fails.
pub fn run(config: &WindowConfig, app: &mut dyn PlatformApp) -> Result<(), PlatformError> {
    config.validate()?;
    let event_loop =
        EventLoop::new().map_err(|e| PlatformError::EventLoop(format!("cannot create it: {e}")))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    // A machine without gamepad support (or one where the system library will not start) just has no pads.
    let mappings = std::env::var("SDL_GAMECONTROLLERCONFIG").ok();
    let gamepads = GamepadSource::new(GamepadConfig::default(), mappings.as_deref()).ok();
    let mut driver = Driver {
        config,
        app,
        gamepads,
        window: None,
        main_window: None,
        resizes: ResizeFilter::default(),
        held: HeldInputs::default(),
        focused: false,
        frame_index: 0,
        last_frame: None,
        failure: None,
    };
    event_loop
        .run_app(&mut driver)
        .map_err(|e| PlatformError::EventLoop(e.to_string()))?;
    match driver.failure.take() {
        Some(failure) => Err(failure),
        None => Ok(()),
    }
}

struct Driver<'a> {
    config: &'a WindowConfig,
    app: &'a mut dyn PlatformApp,
    gamepads: Option<GamepadSource>,
    window: Option<PlatformWindow>,
    main_window: Option<WindowId>,
    resizes: ResizeFilter,
    held: HeldInputs,
    focused: bool,
    frame_index: u64,
    last_frame: Option<Instant>,
    failure: Option<PlatformError>,
}

impl Driver<'_> {
    /// Deliver events in order; stop and quit the loop at the first `Exit`.
    fn deliver(&mut self, event_loop: &ActiveEventLoop, events: Vec<LifecycleEvent>) {
        for event in events {
            if self.app.event(PlatformEvent::Lifecycle(event)) == AppControl::Exit {
                event_loop.exit();
                return;
            }
        }
    }

    /// Deliver input from the OS in order, remembering what is held. A release of something the
    /// app never saw pressed (a key pressed in another window, a release that follows our own
    /// focus-loss release) is dropped so the app's view stays consistent. Stops at the first `Exit`.
    fn deliver_input(&mut self, event_loop: &ActiveEventLoop, events: Vec<InputEvent>) {
        let accepted: Vec<InputEvent> = events
            .into_iter()
            .filter(|event| self.held.accept(event))
            .collect();
        self.deliver_input_unfiltered(event_loop, accepted);
    }

    /// Deliver input exactly as given (the releases the platform synthesizes itself).
    fn deliver_input_unfiltered(&mut self, event_loop: &ActiveEventLoop, events: Vec<InputEvent>) {
        for event in events {
            if self.app.event(PlatformEvent::Input(event)) == AppControl::Exit {
                event_loop.exit();
                return;
            }
        }
    }

    fn cursor_is_captured(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(PlatformWindow::is_cursor_captured)
    }

    /// Focus changed. Losing it releases every held key and button first (the OS will not send the
    /// key-ups) and gives the cursor back, then the app hears `FocusChanged`.
    fn focus_changed(&mut self, event_loop: &ActiveEventLoop, focused: bool) {
        self.focused = focused;
        if !focused {
            let releases = self.held.release_all();
            self.deliver_input_unfiltered(event_loop, releases);
            if let Some(window) = &self.window
                && window.is_cursor_captured()
            {
                let _ = window.set_cursor_captured(false);
            }
        }
        self.deliver(event_loop, vec![LifecycleEvent::FocusChanged(focused)]);
    }

    fn keyboard(&mut self, event_loop: &ActiveEventLoop, event: &KeyEvent) {
        let mut out = Vec::new();
        if let PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = input::key_event(code, event.state, event.repeat)
        {
            out.push(key);
        }
        if event.state == ElementState::Pressed
            && let Some(text) = event.text.as_deref().and_then(input::printable_text)
        {
            out.push(InputEvent::Text(text));
        }
        self.deliver_input(event_loop, out);
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, failure: PlatformError) {
        self.failure = Some(failure);
        event_loop.exit();
    }

    fn surface_info(&self) -> Option<SurfaceInfo> {
        let w = self.window.as_ref()?;
        let (width, height) = w.size();
        Some(translate::surface_info(
            width,
            height,
            f64::from(w.scale_factor()),
        ))
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), PlatformError> {
        let (w, h) = self.config.size;
        let (fullscreen, plan) = display::resolve_fullscreen(
            event_loop.available_monitors(),
            event_loop.primary_monitor().as_ref(),
            &self.config.display_request(),
        );
        for note in &plan.notes {
            tracing::warn!("{note}");
        }
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(f64::from(w), f64::from(h)))
            .with_fullscreen(fullscreen);
        let window = event_loop
            .create_window(attributes)
            .map_err(|e| PlatformError::Window(e.to_string()))?;
        self.main_window = Some(window.id());
        self.window = Some(PlatformWindow {
            window: Arc::new(window),
            display: event_loop.owned_display_handle(),
            captured: Arc::new(AtomicBool::new(false)),
        });
        Ok(())
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let delta = self
            .last_frame
            .map_or(Duration::ZERO, |last| now.duration_since(last));
        self.last_frame = Some(now);
        let info = FrameInfo {
            index: self.frame_index,
            delta,
        };
        self.frame_index += 1;
        if self.app.frame(&info) == AppControl::Exit {
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for Driver<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Android calls this again after every background trip; desktop calls it once.
        if self.window.is_none()
            && let Err(e) = self.create_window(event_loop)
        {
            self.fail(event_loop, e);
            return;
        }
        self.deliver(event_loop, vec![LifecycleEvent::Resumed]);
        let Some(window) = self.window.clone() else {
            return;
        };
        if let Err(message) = self.app.window_ready(&window) {
            self.fail(event_loop, PlatformError::App(message));
            return;
        }
        if let Some(info) = self.surface_info() {
            self.resizes.reset(info.width, info.height);
            self.deliver(event_loop, vec![LifecycleEvent::SurfaceReady(info)]);
        }
        window.window().request_redraw();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        // The contract promises SurfaceLost before Suspended, and that the app has let go of the
        // window by the time the OS takes it away.
        self.deliver(
            event_loop,
            vec![LifecycleEvent::SurfaceLost, LifecycleEvent::Suspended],
        );
        self.app.window_lost();
        self.window = None;
        self.main_window = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.main_window != Some(id) {
            return;
        }
        let observed = match event {
            WindowEvent::Resized(size) => {
                if !self.resizes.accept(size.width, size.height) {
                    return;
                }
                Observed::Resized {
                    width: size.width,
                    height: size.height,
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Observed::ScaleFactor(scale_factor)
            }
            WindowEvent::Focused(focused) => {
                self.focus_changed(event_loop, focused);
                return;
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                // `winit` replays held keys as "synthetic" presses when the window gains focus
                // and releases when it loses it. We track held keys ourselves, so these would
                // only duplicate (or invent) events.
                if !is_synthetic {
                    self.keyboard(event_loop, &event);
                }
                return;
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Some(text) = input::printable_text(&text) {
                    self.deliver_input(event_loop, vec![InputEvent::Text(text)]);
                }
                return;
            }
            WindowEvent::CursorMoved { position, .. } => {
                // While captured the position is meaningless; the app uses raw motion.
                if !self.cursor_is_captured() {
                    let event = InputEvent::CursorMoved {
                        x: position.x as f32,
                        y: position.y as f32,
                    };
                    self.deliver_input(event_loop, vec![event]);
                }
                return;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let event = input::mouse_button_event(button, state);
                self.deliver_input(event_loop, vec![event]);
                return;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.deliver_input(event_loop, vec![input::scroll_event(delta)]);
                return;
            }
            WindowEvent::CloseRequested => Observed::CloseRequested,
            WindowEvent::Destroyed => Observed::Destroyed,
            WindowEvent::RedrawRequested => {
                self.frame(event_loop);
                return;
            }
            _ => return,
        };
        self.deliver(event_loop, translate::lifecycle_events(observed));
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        // Raw, unaccelerated relative motion, the thing mouse-look wants. Only while the window
        // has focus: other applications' mouse movement must not turn the camera.
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && self.focused
        {
            let motion = InputEvent::MouseMotion {
                dx: dx as f32,
                dy: dy as f32,
            };
            self.deliver_input(event_loop, vec![motion]);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(source) = &mut self.gamepads {
            let mut events = Vec::new();
            source.poll(&mut events);
            if !events.is_empty() {
                self.deliver_input(event_loop, events);
            }
        }
        if let Some(window) = &self.window {
            window.window().request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            self.app.window_lost();
        }
        self.window = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_display_request_is_a_normal_window() {
        assert_eq!(
            WindowConfig::default().display_request().mode,
            WindowMode::Windowed
        );
    }

    #[test]
    fn the_fullscreen_flag_means_borderless_unless_another_mode_was_asked_for() {
        let flag = WindowConfig {
            fullscreen: true,
            ..WindowConfig::default()
        };
        assert_eq!(flag.display_request().mode, WindowMode::Borderless);

        let exclusive = WindowConfig {
            fullscreen: true,
            display: DisplayRequest {
                mode: WindowMode::Exclusive,
                resolution: Some((1920, 1080)),
                ..DisplayRequest::default()
            },
            ..WindowConfig::default()
        };
        let request = exclusive.display_request();
        assert_eq!(request.mode, WindowMode::Exclusive, "an explicit mode wins");
        assert_eq!(request.resolution, Some((1920, 1080)));

        let borderless = WindowConfig {
            display: DisplayRequest {
                mode: WindowMode::Borderless,
                ..DisplayRequest::default()
            },
            ..WindowConfig::default()
        };
        assert_eq!(borderless.display_request().mode, WindowMode::Borderless);
    }

    #[test]
    fn default_config_is_valid() {
        assert!(WindowConfig::default().validate().is_ok());
    }

    #[test]
    fn bad_configs_are_rejected_before_touching_the_os() {
        let bad = |edit: &dyn Fn(&mut WindowConfig)| {
            let mut c = WindowConfig::default();
            edit(&mut c);
            let err = c.validate().unwrap_err();
            assert!(matches!(err, PlatformError::InvalidConfig(_)), "{err}");
        };
        bad(&|c| c.title = "  ".into());
        bad(&|c| c.size = (0, 720));
        bad(&|c| c.size = (1280, 0));
        bad(&|c| c.size = (MAX_DIMENSION + 1, 720));
    }

    #[test]
    fn run_rejects_an_invalid_config_without_creating_an_event_loop() {
        struct Never;
        impl AppHandler for Never {
            fn event(&mut self, _: PlatformEvent) -> AppControl {
                unreachable!("no event must be delivered for an invalid config")
            }
            fn frame(&mut self, _: &FrameInfo) -> AppControl {
                unreachable!()
            }
        }
        impl PlatformApp for Never {
            fn window_ready(&mut self, _: &PlatformWindow) -> Result<(), String> {
                unreachable!()
            }
        }
        let config = WindowConfig {
            size: (0, 0),
            ..WindowConfig::default()
        };
        let err = run(&config, &mut Never).unwrap_err();
        assert!(
            err.to_string().contains("invalid window configuration"),
            "{err}"
        );
    }

    #[test]
    fn platform_app_is_object_safe_and_defaults_window_lost() {
        struct Quiet;
        impl AppHandler for Quiet {
            fn event(&mut self, _: PlatformEvent) -> AppControl {
                AppControl::Continue
            }
            fn frame(&mut self, _: &FrameInfo) -> AppControl {
                AppControl::Continue
            }
        }
        impl PlatformApp for Quiet {
            fn window_ready(&mut self, _: &PlatformWindow) -> Result<(), String> {
                Ok(())
            }
        }
        let mut app = Quiet;
        let dynamic: &mut dyn PlatformApp = &mut app;
        dynamic.window_lost(); // the default is a no-op
    }

    #[test]
    fn errors_name_what_went_wrong() {
        assert!(
            PlatformError::Window("no display".into())
                .to_string()
                .contains("no display")
        );
        assert!(PlatformError::App("gpu".into()).to_string().contains("gpu"));
        assert!(
            PlatformError::EventLoop("x".into())
                .to_string()
                .contains("event loop")
        );
    }
}
