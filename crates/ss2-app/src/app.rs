//! The interactive window (T03-033): the platform loop (`ss2_platform::window::run`) drives the app through
//! `PlatformEvent`s; the app routes input to the menu, renders and applies settings effects.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_platform::nav::StickNavigator;
use ss2_platform::window::{PlatformApp, PlatformWindow, WindowConfig};
use ss2_platform_api::{
    AppControl, AppHandler, FrameInfo, GamepadAxis, InputEvent, KeyCode, LifecycleEvent,
    MouseButton, PlatformEvent, PlatformPaths,
};
use ss2_render::Renderer;
use ss2_ui::{Canvas, Color, DrawList, Menu, UiInput, Vec2, theme};

use crate::input::{nav_for_button, nav_for_key};
use crate::state::{AppState, Effects, Scene};
use crate::{profiling, screenshot};

/// Clear color of the placeholder "game" scene.
const PLAYING_CLEAR: Color = Color([0.02, 0.03, 0.05, 1.0]);

struct App {
    state: AppState,
    menu: Menu,
    /// Reused every frame, so that drawing a screen allocates nothing (`docs/UI_BUDGET.md`).
    draw_list: DrawList,
    settings_path: Option<PathBuf>,
    screenshot_dir: Option<PathBuf>,
    capture_pending: bool,
    window: Option<PlatformWindow>,
    renderer: Option<Renderer>,
    /// The left stick, for menu navigation with key-repeat.
    stick: StickNavigator,
    stick_xy: (f32, f32),
    /// Time since start, summed from frame deltas (for the stick's repeat).
    clock: Duration,
    cursor: Vec2,
    frames_left: Option<u32>,
    exit: bool,
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
/// the `--set KEY=VALUE` settings overrides of this run. `paths` provides the settings and capture
/// locations for an interactive run.
pub fn run(
    smoke_frames: Option<u32>,
    overrides: &[String],
    paths: Option<&PlatformPaths>,
) -> Result<(), String> {
    let settings_path = paths
        .map(PlatformPaths::settings_file)
        .or_else(Settings::default_path);
    let settings = effective_settings(settings_path.as_deref(), overrides)?;
    let config = WindowConfig {
        fullscreen: settings.fullscreen,
        ..WindowConfig::default()
    };
    let mut app = App::new(
        settings,
        settings_path,
        paths.map(|paths| paths.save_dir.join("screenshots")),
        smoke_frames,
    );
    ss2_platform::window::run(&config, &mut app).map_err(|e| e.to_string())?;
    match app.failure {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

impl App {
    fn new(
        settings: Settings,
        settings_path: Option<PathBuf>,
        screenshot_dir: Option<PathBuf>,
        frames_left: Option<u32>,
    ) -> Self {
        Self {
            state: AppState::new(settings.clone()),
            menu: Menu::new(settings),
            draw_list: DrawList::default(),
            settings_path,
            screenshot_dir,
            capture_pending: false,
            window: None,
            renderer: None,
            stick: StickNavigator::new(),
            stick_xy: (0.0, 0.0),
            clock: Duration::ZERO,
            cursor: Vec2::default(),
            frames_left,
            exit: false,
            failure: None,
        }
    }

    fn control(&self) -> AppControl {
        if self.exit {
            AppControl::Exit
        } else {
            AppControl::Continue
        }
    }

    fn canvas(&self) -> Option<Canvas> {
        let (w, h) = self.window.as_ref()?.size();
        Canvas::fit(w, h)
    }

    fn pointer(&mut self, make: fn(Vec2) -> UiInput) {
        if let Some(canvas) = self.canvas() {
            let p = canvas.to_canvas(self.cursor);
            self.dispatch(make(p));
        }
    }

    fn dispatch(&mut self, input: UiInput) {
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
                    self.run_effects(effects);
                }
            }
        }
    }

    fn run_effects(&mut self, fx: Effects) {
        if fx.exit {
            self.exit = true;
        }
        if let (Some(vsync), Some(r)) = (fx.set_vsync, self.renderer.as_mut()) {
            r.set_vsync(vsync);
        }
        if let (Some(full), Some(w)) = (fx.set_fullscreen, self.window.as_ref()) {
            w.set_fullscreen(full);
        }
        if fx.save_settings
            && let Some(path) = &self.settings_path
            && let Err(e) = self.state.settings.save_to(path)
        {
            tracing::warn!("could not save settings: {e}");
        }
    }

    fn key_pressed(&mut self, key: KeyCode, repeat: bool) {
        if key == KeyCode::F12 {
            if !repeat {
                self.capture_pending = self.screenshot_dir.is_some();
                if !self.capture_pending && self.frames_left.is_none() {
                    tracing::warn!("no screenshot folder is available");
                }
            }
        } else if let Some(nav) = nav_for_key(key) {
            self.dispatch(UiInput::Nav(nav));
        }
    }

    fn redraw(&mut self) {
        profiling::frame();
        profiling::profile_scope!("redraw");
        let clear = match self.state.scene {
            Scene::Menu => {
                self.menu.draw_into(&mut self.draw_list);
                theme::BACKGROUND
            }
            Scene::Playing => {
                self.draw_list.clear();
                PLAYING_CLEAR
            }
        };
        let rendered = {
            profiling::profile_scope!("render");
            self.renderer
                .as_mut()
                .map(|renderer| renderer.render(&self.draw_list, clear))
        };
        if let Some(Err(e)) = rendered {
            self.failure = Some(format!("rendering failed: {e}"));
            self.exit = true;
            return;
        }
        if self.capture_pending && self.canvas().is_some() {
            self.capture_pending = false;
            if let (Some(renderer), Some(dir)) = (&mut self.renderer, &self.screenshot_dir) {
                let capture = renderer
                    .capture_frame(Some(&self.draw_list), clear)
                    .map_err(|error| error.to_string())
                    .and_then(|image| screenshot::save_interactive(dir, &image));
                match capture {
                    Ok(path) => tracing::info!(path = %path.display(), "saved screenshot"),
                    Err(error) => tracing::warn!(%error, "cannot save screenshot"),
                }
            }
        }
    }
}

impl AppHandler for App {
    fn event(&mut self, event: PlatformEvent) -> AppControl {
        match event {
            PlatformEvent::Lifecycle(LifecycleEvent::CloseRequested) => self.exit = true,
            PlatformEvent::Lifecycle(
                LifecycleEvent::Resized { width, height }
                | LifecycleEvent::SurfaceReady(ss2_platform_api::SurfaceInfo {
                    width, height, ..
                }),
            ) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(width, height);
                }
            }
            PlatformEvent::Input(InputEvent::Key {
                key,
                pressed: true,
                repeat,
            }) => self.key_pressed(key, repeat),
            PlatformEvent::Input(InputEvent::CursorMoved { x, y }) => {
                self.cursor = Vec2 { x, y };
                self.pointer(UiInput::PointerMoved);
            }
            PlatformEvent::Input(InputEvent::MouseButton {
                button: MouseButton::Left,
                pressed,
            }) => {
                let make: fn(Vec2) -> UiInput = if pressed {
                    UiInput::PointerPressed
                } else {
                    UiInput::PointerReleased
                };
                self.pointer(make);
            }
            PlatformEvent::Input(InputEvent::GamepadButton {
                button,
                pressed: true,
                ..
            }) => {
                if let Some(nav) = nav_for_button(button) {
                    self.dispatch(UiInput::Nav(nav));
                }
            }
            PlatformEvent::Input(InputEvent::GamepadAxis { axis, value, .. }) => match axis {
                GamepadAxis::LeftStickX => self.stick_xy.0 = value,
                GamepadAxis::LeftStickY => self.stick_xy.1 = value,
                _ => {}
            },
            _ => {}
        }
        self.control()
    }

    fn frame(&mut self, frame: &FrameInfo) -> AppControl {
        self.clock += frame.delta;
        let now_ms = u64::try_from(self.clock.as_millis()).unwrap_or(u64::MAX);
        if let Some(nav) = self.stick.update(self.stick_xy.0, self.stick_xy.1, now_ms) {
            self.dispatch(UiInput::Nav(nav));
        }
        self.redraw();
        if let Some(left) = self.frames_left.as_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                self.exit = true;
            }
        }
        self.control()
    }
}

impl PlatformApp for App {
    fn window_ready(&mut self, window: &PlatformWindow) -> Result<(), String> {
        if self.renderer.is_none() {
            let renderer = Renderer::new_windowed(
                Box::new(window.display_handle()),
                window.window().clone(),
                window.size(),
                self.state.settings.vsync,
            )
            .map_err(|e| format!("cannot start the renderer: {e}"))?;
            self.renderer = Some(renderer);
        }
        self.window = Some(window.clone());
        Ok(())
    }

    fn window_lost(&mut self) {
        self.renderer = None;
        self.window = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_platform_api::{GamepadButton, GamepadId, ScriptStep, run_script};
    use ss2_ui::ScreenId;

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

    fn app() -> App {
        App::new(
            Settings::for_platform(ss2_core::settings::PlatformClass::current()),
            None,
            None,
            None,
        )
    }

    fn key(key: KeyCode) -> ScriptStep {
        InputEvent::Key {
            key,
            pressed: true,
            repeat: false,
        }
        .into()
    }

    #[test]
    fn closing_the_window_exits() {
        let mut a = app();
        let report = run_script(
            &mut a,
            [
                ScriptStep::Frame,
                LifecycleEvent::CloseRequested.into(),
                ScriptStep::Frame,
            ],
        );
        assert!(report.exited);
        assert_eq!(report.frames, 1);
    }

    #[test]
    fn enter_starts_the_game_and_escape_pauses_it() {
        let mut a = app();
        run_script(&mut a, [key(KeyCode::Enter), ScriptStep::Frame]);
        assert_eq!(a.state.scene, Scene::Playing);
        run_script(&mut a, [key(KeyCode::Escape), ScriptStep::Frame]);
        assert_eq!(a.state.scene, Scene::Menu);
        assert_eq!(a.menu.screen(), ScreenId::Pause);
    }

    #[test]
    fn quit_from_the_main_menu_exits() {
        let mut a = app();
        let report = run_script(
            &mut a,
            [
                key(KeyCode::ArrowDown),
                key(KeyCode::ArrowDown),
                key(KeyCode::ArrowDown),
                key(KeyCode::Enter),
                ScriptStep::Frame,
            ],
        );
        assert!(report.exited);
    }

    #[test]
    fn released_and_repeated_keys_do_not_navigate_twice() {
        let mut a = app();
        run_script(
            &mut a,
            [InputEvent::Key {
                key: KeyCode::Enter,
                pressed: false,
                repeat: false,
            }
            .into()],
        );
        assert_eq!(a.state.scene, Scene::Menu, "a release does nothing");
    }

    #[test]
    fn the_gamepad_navigates_with_buttons_and_the_held_stick_repeats() {
        let mut a = app();
        let start = a.menu.focused();
        let pad = GamepadId(0);
        let mut steps: Vec<ScriptStep> = vec![
            InputEvent::GamepadAxis {
                pad,
                axis: GamepadAxis::LeftStickY,
                value: -1.0,
            }
            .into(),
        ];
        steps.push(ScriptStep::Frame);
        run_script(&mut a, steps);
        assert_ne!(a.menu.focused(), start, "the stick moved the focus down");
        let after_one = a.menu.focused();
        // Held for half a second: it repeats.
        run_script(&mut a, std::iter::repeat_n(ScriptStep::Frame, 30));
        assert_ne!(a.menu.focused(), after_one, "held, it repeats");
        // Released, then South activates whatever is focused (here: not New Game, so still the menu).
        run_script(
            &mut a,
            [InputEvent::GamepadAxis {
                pad,
                axis: GamepadAxis::LeftStickY,
                value: 0.0,
            }
            .into()],
        );
        let mut b = app();
        run_script(
            &mut b,
            [InputEvent::GamepadButton {
                pad,
                button: GamepadButton::South,
                pressed: true,
            }
            .into()],
        );
        assert_eq!(b.state.scene, Scene::Playing, "South activates New Game");
    }

    #[test]
    fn smoke_frames_exit_after_the_count() {
        let mut a = App::new(
            Settings::for_platform(ss2_core::settings::PlatformClass::current()),
            None,
            None,
            Some(3),
        );
        let report = run_script(&mut a, std::iter::repeat_n(ScriptStep::Frame, 10));
        assert_eq!((report.frames, report.exited), (3, true));
    }

    #[test]
    fn f12_without_a_screenshot_folder_does_nothing() {
        let mut a = app();
        run_script(&mut a, [key(KeyCode::F12), ScriptStep::Frame]);
        assert!(!a.capture_pending);
        assert_eq!(a.state.scene, Scene::Menu);
    }
}
