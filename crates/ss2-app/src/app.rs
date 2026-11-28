//! The interactive window: the platform loop (`ss2_platform::window::run`, T03-033) drives the app through
//! `PlatformEvent`s; the app routes input to the menus or the game, renders and applies settings effects.
//!
//! "New Game" opens the level picker; a picked level loads on a worker thread behind a progress bar and is then
//! played (T03-037): the walker or the free-fly camera of [`PlaySession`], the mouse captured, Esc for the pause
//! menu, "Quit to Menu" drops the level.
//!
//! Without a usable install the window first shows the data-location wizard (T10-019b, [`WizardScene`]); the menu
//! follows once a folder is chosen.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_game::play::{Mode, PlaySession};
use ss2_physics::query::QueryWorld;
use ss2_platform::nav::StickNavigator;
use ss2_platform::window::{PlatformApp, PlatformWindow, WindowConfig};
use ss2_platform_api::{
    AppControl, AppHandler, FrameInfo, GamepadAxis, GamepadButton, InputEvent, KeyCode,
    LifecycleEvent, MouseButton, PlatformEvent, PlatformPaths,
};
use ss2_render::Renderer;
use ss2_ui::data_wizard::WizardInput;
use ss2_ui::level_select::{LevelPicker, PickerCommand};
use ss2_ui::{Canvas, DrawList, Menu, MenuCommand, UiInput, Vec2, theme};
use ss2_world::scene::LevelScene;

use crate::input::{WizardKeys, nav_for_button, nav_for_key};
use crate::level_session::{SKY, upload};
use crate::play::{Levels, Loading, draw_hud};
use crate::play_input::PlayControls;
use crate::state::{AppState, Effects, Scene};
use crate::wizard_scene::{Outcome, WizardScene};
use crate::{profiling, screenshot};

/// What the game scene is doing.
enum Phase {
    /// Choosing a level.
    Picking(LevelPicker),
    /// Loading one.
    Loading(Loading),
    /// Playing one.
    Playing(Box<Playing>),
}

struct Playing {
    session: PlaySession,
    name: String,
    /// The renderer's world generation right after this level was uploaded; another value means it must be
    /// uploaded again (a GPU device rebuild dropped it).
    uploaded: Option<u64>,
}

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
    levels: Arc<Levels>,
    phase: Option<Phase>,
    controls: PlayControls,
    /// `--level NAME`: load this level at the first frame (after the wizard, when it shows).
    start_level: Option<String>,
    /// The first-run wizard, shown before everything else while it is here.
    wizard: Option<WizardScene>,
    wizard_keys: WizardKeys,
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

/// What a run of the window starts with besides the settings.
pub(crate) struct RunOptions<'a> {
    /// Exit after this many frames (automated smoke test).
    pub smoke_frames: Option<u32>,
    /// The `--set KEY=VALUE` overrides.
    pub overrides: &'a [String],
    /// Settings and capture locations of an interactive run.
    pub paths: Option<&'a PlatformPaths>,
    /// The levels the picker offers.
    pub levels: Levels,
    /// `--level NAME`: go straight into this level.
    pub start_level: Option<String>,
    /// The first-run wizard to show first (no usable install was found).
    pub wizard: Option<WizardScene>,
}

/// Run the interactive app.
pub(crate) fn run(options: RunOptions<'_>) -> Result<(), String> {
    let settings_path = options
        .paths
        .map(PlatformPaths::settings_file)
        .or_else(Settings::default_path);
    let settings = effective_settings(settings_path.as_deref(), options.overrides)?;
    let config = WindowConfig {
        fullscreen: settings.fullscreen,
        ..WindowConfig::default()
    };
    let mut app = App::new(
        settings,
        settings_path,
        options
            .paths
            .map(|paths| paths.save_dir.join("screenshots")),
        options.smoke_frames,
        options.levels,
    );
    app.start_level = options.start_level;
    app.wizard = options.wizard;
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
        levels: Levels,
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
            levels: Arc::new(levels),
            phase: None,
            controls: PlayControls::default(),
            start_level: None,
            wizard: None,
            wizard_keys: WizardKeys::default(),
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

    fn is_playing(&self) -> bool {
        self.state.scene == Scene::Playing && matches!(self.phase, Some(Phase::Playing(_)))
    }

    fn capture_cursor(&self, captured: bool) {
        if let Some(w) = &self.window
            && let Err(e) = w.set_cursor_captured(captured)
        {
            tracing::debug!("cursor capture {captured}: {e}");
        }
    }

    fn new_picker(&self) -> LevelPicker {
        LevelPicker::new(self.levels.entries())
    }

    fn start_loading(&mut self, id: &str) {
        self.state.scene = Scene::Playing;
        self.phase = Some(Phase::Loading(Loading::start(self.levels.clone(), id)));
    }

    fn enter_play(&mut self, scene: Arc<LevelScene>, physics: QueryWorld, name: String) {
        self.controls.release_all();
        self.phase = Some(Phase::Playing(Box::new(Playing {
            session: PlaySession::new(scene, physics),
            name,
            uploaded: None,
        })));
        self.capture_cursor(true);
    }

    fn pause(&mut self) {
        self.state.scene = Scene::Menu;
        self.menu.open_pause();
        self.controls.release_all();
        self.capture_cursor(false);
    }

    fn leave_game(&mut self) {
        self.phase = None;
        self.controls.release_all();
        self.capture_cursor(false);
        if let Some(r) = self.renderer.as_mut() {
            r.clear_world();
        }
    }

    fn dispatch(&mut self, input: UiInput) {
        if let Some(wizard) = &mut self.wizard {
            wizard.handle(WizardInput::Ui(input));
            return;
        }
        match self.state.scene {
            Scene::Playing => match &mut self.phase {
                Some(Phase::Picking(picker)) => match picker.handle(input) {
                    Some(PickerCommand::Start(id)) => self.start_loading(&id),
                    Some(PickerCommand::Back) => {
                        self.leave_game();
                        self.state.scene = Scene::Menu;
                        self.menu.open_main();
                    }
                    None => {}
                },
                // The worker finishes on its own; its result is dropped.
                Some(Phase::Loading(_)) if input == UiInput::Nav(NavInput::Back) => {
                    self.phase = Some(Phase::Picking(self.new_picker()));
                }
                Some(Phase::Playing(_)) if input == UiInput::Nav(NavInput::Back) => self.pause(),
                _ => {}
            },
            Scene::Menu => {
                for cmd in self.menu.handle(input) {
                    let effects = self.state.apply(cmd.clone());
                    match cmd {
                        MenuCommand::StartGame => {
                            self.phase = Some(Phase::Picking(self.new_picker()));
                        }
                        MenuCommand::Resume => {
                            if self.phase.is_none() {
                                self.phase = Some(Phase::Picking(self.new_picker()));
                            }
                            if self.is_playing() {
                                self.capture_cursor(true);
                            }
                        }
                        MenuCommand::QuitToMenu => self.leave_game(),
                        _ => {}
                    }
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

    /// Input while a level is being played: Esc and Start pause, a click takes the mouse back (after focus was
    /// lost), F12 still takes a screenshot, everything else goes to the controls.
    fn play_input(&mut self, input: InputEvent) {
        match input {
            InputEvent::Key {
                key: KeyCode::Escape,
                pressed: true,
                ..
            }
            | InputEvent::GamepadButton {
                button: GamepadButton::Start,
                pressed: true,
                ..
            } => self.pause(),
            InputEvent::Key {
                key: KeyCode::F12,
                pressed: true,
                repeat,
            } => self.key_pressed(KeyCode::F12, repeat),
            InputEvent::MouseButton {
                button: MouseButton::Left,
                pressed: true,
            } => self.capture_cursor(true),
            other => self.controls.on_event(&other),
        }
    }

    /// Keyboard input while the wizard shows: F12 still takes a screenshot, keys and text go to the wizard.
    fn wizard_input(&mut self, input: &InputEvent) {
        if let InputEvent::Key {
            key: KeyCode::F12,
            pressed: true,
            repeat,
        } = input
        {
            self.key_pressed(KeyCode::F12, *repeat);
            return;
        }
        if let Some(wizard) = &mut self.wizard {
            let typing = wizard.typing();
            if let Some(input) = self.wizard_keys.map(input, typing) {
                wizard.handle(input);
            }
        }
    }

    /// Collect the wizard's finished work: when a folder is ready its levels replace the ones offered so far.
    fn poll_wizard(&mut self) {
        let Some(wizard) = &mut self.wizard else {
            return;
        };
        match wizard.poll() {
            Outcome::Continue => {}
            Outcome::Ready(levels) => {
                self.levels = Arc::new(levels);
                self.wizard = None;
                self.state.scene = Scene::Menu;
                self.menu.open_main();
            }
            Outcome::Quit => self.exit = true,
        }
    }

    /// Advance the game scene by `dt` seconds.
    fn advance(&mut self, dt: f32) {
        match &mut self.phase {
            Some(Phase::Loading(loading)) => {
                if let Some(result) = loading.poll(dt) {
                    let name = loading.name.clone();
                    match result {
                        Ok((scene, physics)) => self.enter_play(scene, physics, name),
                        Err(e) => {
                            tracing::warn!("cannot load {name}: {e}");
                            self.phase = Some(Phase::Picking(self.new_picker()));
                        }
                    }
                }
            }
            Some(Phase::Playing(playing)) if self.state.scene == Scene::Playing => {
                let input = self.controls.frame(dt);
                playing.session.update(&input, dt);
            }
            _ => {}
        }
    }

    fn redraw(&mut self) {
        profiling::frame();
        profiling::profile_scope!("redraw");
        let mut world = None;
        // The fog where the camera is (RFC 0013 amendment 4); none outside a level.
        let mut fog = None;
        let clear = match (self.state.scene, &mut self.phase) {
            _ if self.wizard.is_some() => {
                if let Some(wizard) = &self.wizard {
                    wizard.draw_into(&mut self.draw_list);
                }
                theme::BACKGROUND
            }
            (Scene::Menu, _) => {
                self.menu.draw_into(&mut self.draw_list);
                theme::BACKGROUND
            }
            (Scene::Playing, Some(Phase::Picking(picker))) => {
                self.draw_list.clear();
                picker.draw(&mut self.draw_list);
                theme::BACKGROUND
            }
            (Scene::Playing, Some(Phase::Loading(loading))) => {
                loading.draw(&mut self.draw_list);
                theme::BACKGROUND
            }
            (Scene::Playing, Some(Phase::Playing(playing))) => {
                if let Some(renderer) = self.renderer.as_mut()
                    && playing.uploaded != Some(renderer.world_generation())
                {
                    match upload(renderer, playing.session.scene()) {
                        Ok(report) => {
                            tracing::info!(level = %playing.name, ?report, "level on the GPU");
                            playing.uploaded = Some(renderer.world_generation());
                        }
                        Err(e) => {
                            self.failure = Some(format!("cannot upload {}: {e}", playing.name));
                            self.exit = true;
                            return;
                        }
                    }
                }
                draw_hud(
                    &mut self.draw_list,
                    &playing.name,
                    playing.session.mode() == Mode::Fly,
                );
                let camera = playing.session.camera();
                fog = playing.session.scene().fog_at(camera.position);
                world = Some(camera);
                SKY
            }
            (Scene::Playing, None) => {
                self.draw_list.clear();
                theme::BACKGROUND
            }
        };
        if let Some(renderer) = self.renderer.as_mut()
            && let Err(e) = renderer.set_fog(fog)
        {
            tracing::warn!(error = %e, "fog refused");
        }
        let rendered = {
            profiling::profile_scope!("render");
            self.renderer.as_mut().map(|renderer| match &world {
                Some(camera) => renderer.render_world(camera, &self.draw_list, clear),
                None => renderer.render(&self.draw_list, clear),
            })
        };
        if let Some(Err(e)) = rendered {
            self.failure = Some(format!("rendering failed: {e}"));
            self.exit = true;
            return;
        }
        if self.capture_pending && self.canvas().is_some() {
            self.capture_pending = false;
            if let (Some(renderer), Some(dir)) = (&mut self.renderer, &self.screenshot_dir) {
                let image = match &world {
                    Some(camera) => {
                        renderer.render_world_to_image(camera, Some(&self.draw_list), clear)
                    }
                    None => renderer.capture_frame(Some(&self.draw_list), clear),
                };
                let capture = image
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
        if let PlatformEvent::Input(input @ (InputEvent::Key { .. } | InputEvent::Text(_))) = &event
            && self.wizard.is_some()
        {
            self.wizard_input(input);
            return self.control();
        }
        if let PlatformEvent::Input(input) = &event
            && self.is_playing()
        {
            self.play_input(input.clone());
            return self.control();
        }
        match event {
            PlatformEvent::Lifecycle(LifecycleEvent::CloseRequested) => self.exit = true,
            PlatformEvent::Lifecycle(LifecycleEvent::FocusChanged(false)) => {
                self.controls.release_all();
            }
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
        self.poll_wizard();
        if self.wizard.is_none()
            && let Some(id) = self.start_level.take()
        {
            self.start_loading(&id);
        }
        let dt = frame.delta.as_secs_f32();
        self.clock += frame.delta;
        let now_ms = u64::try_from(self.clock.as_millis()).unwrap_or(u64::MAX);
        if !self.is_playing()
            && let Some(nav) = self.stick.update(self.stick_xy.0, self.stick_xy.1, now_ms)
        {
            self.dispatch(UiInput::Nav(nav));
        }
        self.advance(dt);
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
        if self.is_playing() {
            self.capture_cursor(true);
        }
        Ok(())
    }

    fn window_lost(&mut self) {
        self.renderer = None;
        self.window = None;
        if let Some(Phase::Playing(playing)) = &mut self.phase {
            playing.uploaded = None;
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
