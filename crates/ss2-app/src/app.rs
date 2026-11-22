//! The interactive window: winit event loop, input routing, rendering and settings effects.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use gilrs::{Axis, EventType, Gilrs};
use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_platform::nav::{StickNavigator, map_gamepad_button, map_key};
use ss2_render::Renderer;
use ss2_ui::{Canvas, Color, DrawList, Menu, UiInput, Vec2, theme};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Window, WindowId};

use crate::state::{AppState, Effects, Scene};

/// Clear color of the placeholder "game" scene.
const PLAYING_CLEAR: Color = Color([0.02, 0.03, 0.05, 1.0]);

struct App {
    state: AppState,
    menu: Menu,
    settings_path: Option<PathBuf>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gilrs: Option<Gilrs>,
    stick: StickNavigator,
    cursor: Vec2,
    started: Instant,
    frames_left: Option<u32>,
    failure: Option<String>,
}

/// The settings for this run: the saved file (or the platform defaults when there is none), then
/// the `--set KEY=VALUE` overrides from the command line.
///
/// # Errors
/// The message of the first invalid override.
pub(crate) fn effective_settings(
    path: Option<&Path>,
    overrides: &[String],
) -> Result<Settings, String> {
    let mut settings = path
        .map(Settings::load_from)
        .unwrap_or_else(|| Settings::for_platform(ss2_core::settings::PlatformClass::current()));
    settings
        .apply_overrides(overrides)
        .map_err(|e| e.to_string())?;
    Ok(settings)
}

/// Run the interactive app. With `smoke_frames`, exits after that many frames. `overrides` are
/// the `--set KEY=VALUE` settings overrides of this run.
pub fn run(smoke_frames: Option<u32>, overrides: &[String]) -> Result<(), String> {
    let settings_path = Settings::default_path();
    let settings = effective_settings(settings_path.as_deref(), overrides)?;
    let gilrs = match Gilrs::new() {
        Ok(g) => Some(g),
        Err(e) => {
            tracing::warn!("gamepad support unavailable: {e}");
            None
        }
    };
    let mut app = App {
        state: AppState::new(settings.clone()),
        menu: Menu::new(settings),
        settings_path,
        window: None,
        renderer: None,
        gilrs,
        stick: StickNavigator::new(),
        cursor: Vec2::default(),
        started: Instant::now(),
        frames_left: smoke_frames,
        failure: None,
    };
    let event_loop = EventLoop::new().map_err(|e| format!("cannot create the event loop: {e}"))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("event loop error: {e}"))?;
    match app.failure {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

impl App {
    fn canvas(&self) -> Option<Canvas> {
        let size = self.window.as_ref()?.inner_size();
        Canvas::fit(size.width, size.height)
    }

    fn pointer(&mut self, event_loop: &ActiveEventLoop, make: fn(Vec2) -> UiInput) {
        if let Some(canvas) = self.canvas() {
            let p = canvas.to_canvas(self.cursor);
            self.dispatch(event_loop, make(p));
        }
    }

    fn dispatch(&mut self, event_loop: &ActiveEventLoop, input: UiInput) {
        match self.state.scene {
            Scene::Playing => {
                if input == UiInput::Nav(NavInput::Back) {
                    self.state.scene = Scene::Menu;
                    self.menu.open_pause();
                }
            }
            Scene::Menu => {
                for cmd in self.menu.handle(input) {
                    let effects = self.state.apply(cmd);
                    self.run_effects(event_loop, effects);
                }
            }
        }
    }

    fn run_effects(&mut self, event_loop: &ActiveEventLoop, fx: Effects) {
        if fx.exit {
            event_loop.exit();
        }
        if let (Some(vsync), Some(r)) = (fx.set_vsync, self.renderer.as_mut()) {
            r.set_vsync(vsync);
        }
        if let (Some(full), Some(w)) = (fx.set_fullscreen, self.window.as_ref()) {
            w.set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
        }
        if fx.save_settings
            && let Some(path) = &self.settings_path
            && let Err(e) = self.state.settings.save_to(path)
        {
            tracing::warn!("could not save settings: {e}");
        }
    }

    fn poll_gamepad(&mut self) -> Vec<UiInput> {
        let mut inputs = Vec::new();
        let Some(gilrs) = self.gilrs.as_mut() else {
            return inputs;
        };
        while let Some(event) = gilrs.next_event() {
            if let EventType::ButtonPressed(button, _) = event.event
                && let Some(nav) = map_gamepad_button(button)
            {
                inputs.push(UiInput::Nav(nav));
            }
        }
        if let Some((_, pad)) = gilrs.gamepads().next() {
            let now_ms = self.started.elapsed().as_millis() as u64;
            let (x, y) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            if let Some(nav) = self.stick.update(x, y, now_ms) {
                inputs.push(UiInput::Nav(nav));
            }
        }
        inputs
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (list, clear) = match self.state.scene {
            Scene::Menu => (self.menu.draw(), theme::BACKGROUND),
            Scene::Playing => (DrawList::default(), PLAYING_CLEAR),
        };
        if let Some(renderer) = self.renderer.as_mut()
            && let Err(e) = renderer.render(&list, clear)
        {
            self.failure = Some(format!("rendering failed: {e}"));
            event_loop.exit();
            return;
        }
        if let Some(left) = self.frames_left.as_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("SS2RS")
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_fullscreen(
                self.state
                    .settings
                    .fullscreen
                    .then_some(Fullscreen::Borderless(None)),
            );
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.failure = Some(format!("cannot create the window: {e}"));
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        match Renderer::new_windowed(
            Box::new(event_loop.owned_display_handle()),
            window.clone(),
            (size.width, size.height),
            self.state.settings.vsync,
        ) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => {
                self.failure = Some(format!("cannot start the renderer: {e}"));
                event_loop.exit();
                return;
            }
        }
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Vec2 {
                    x: position.x as f32,
                    y: position.y as f32,
                };
                self.pointer(event_loop, UiInput::PointerMoved);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let make: fn(Vec2) -> UiInput = match state {
                    ElementState::Pressed => UiInput::PointerPressed,
                    ElementState::Released => UiInput::PointerReleased,
                };
                self.pointer(event_loop, make);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(code) = event.physical_key
                    && let Some(nav) = map_key(code)
                {
                    self.dispatch(event_loop, UiInput::Nav(nav));
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        for input in self.poll_gamepad() {
            self.dispatch(event_loop, input);
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_file_or_overrides_the_platform_defaults_apply() {
        let s = effective_settings(None, &[]).unwrap();
        assert_eq!(
            s,
            Settings::for_platform(ss2_core::settings::PlatformClass::current())
        );
    }

    #[test]
    fn overrides_are_applied_on_top_of_the_saved_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, "vsync = false\nmaster_volume = 0.3\n").unwrap();
        let overrides = ["fullscreen=1".to_owned(), "master_volume=0.9".to_owned()];
        let s = effective_settings(Some(&path), &overrides).unwrap();
        assert!(!s.vsync, "kept from the file");
        assert!(s.fullscreen, "set by the override");
        assert_eq!(s.master_volume, 0.9, "the override wins over the file");
    }

    #[test]
    fn a_bad_override_is_an_error_naming_it() {
        let message = effective_settings(None, &["brightness=1".to_owned()]).unwrap_err();
        assert!(message.contains("brightness"), "{message}");
    }
}
