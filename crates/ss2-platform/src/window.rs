//! The window and the `winit` event loop, driving an application through the platform contract
//! (`ss2-platform-api`, RFC 0003).
//!
//! [`run`] creates the window, turns what `winit` reports into [`PlatformEvent`]s and calls
//! [`AppHandler::event`] / [`AppHandler::frame`]; the app answers with [`AppControl`]. The one thing
//! the windowing-free contract cannot carry is the window itself, which a renderer needs to create
//! its surface: that is the extra hook of [`PlatformApp`].
//!
//! Input events (keyboard, mouse, gamepad) are T03-002 and T03-003; this module delivers the
//! lifecycle and the frame loop they will plug into.

use std::sync::Arc;
use std::time::{Duration, Instant};

use ss2_platform_api::{
    AppControl, AppHandler, FrameInfo, LifecycleEvent, PlatformEvent, SurfaceInfo,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::window::{Fullscreen, Window, WindowId};

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
    /// Start borderless-fullscreen on the current monitor.
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "SS2RS".to_owned(),
            size: (1280, 720),
            fullscreen: false,
        }
    }
}

impl WindowConfig {
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

    /// Change the title bar text.
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
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
    let mut driver = Driver {
        config,
        app,
        window: None,
        main_window: None,
        resizes: ResizeFilter::default(),
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
    window: Option<PlatformWindow>,
    main_window: Option<WindowId>,
    resizes: ResizeFilter,
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
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(f64::from(w), f64::from(h)))
            .with_fullscreen(
                self.config
                    .fullscreen
                    .then_some(Fullscreen::Borderless(None)),
            );
        let window = event_loop
            .create_window(attributes)
            .map_err(|e| PlatformError::Window(e.to_string()))?;
        self.main_window = Some(window.id());
        self.window = Some(PlatformWindow {
            window: Arc::new(window),
            display: event_loop.owned_display_handle(),
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
            WindowEvent::Focused(focused) => Observed::Focused(focused),
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
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
