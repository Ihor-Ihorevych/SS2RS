//! Menu model: screens, items, focus navigation and the commands the app must act on.

use crate::geometry::Vec2;
use ss2_core::input::NavInput;
use ss2_core::settings::Settings;

/// Volume slider step and number of steps across `0.0..=1.0`.
const STEP: f32 = 0.05;
const MAX_STEPS: i32 = 20;

/// Which menu screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    /// The main menu.
    Main,
    /// Video/audio options.
    Options,
    /// The in-game pause menu.
    Pause,
}

/// Input events the menu understands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiInput {
    /// Keyboard/gamepad navigation.
    Nav(NavInput),
    /// Pointer moved (canvas units).
    PointerMoved(Vec2),
    /// Pointer button pressed (canvas units).
    PointerPressed(Vec2),
    /// Pointer button released (canvas units).
    PointerReleased(Vec2),
}

/// What the app must do in response to menu input.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuCommand {
    /// Start a new game.
    StartGame,
    /// Leave the pause menu and continue playing.
    Resume,
    /// Leave the game and show the main menu.
    QuitToMenu,
    /// Exit the application.
    Quit,
    /// A setting changed; carries the full new settings.
    SettingsChanged(Settings),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToggleSetting {
    Vsync,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SliderSetting {
    Master,
    Music,
    Sfx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    StartGame,
    Resume,
    QuitToMenu,
    Quit,
    Open(ScreenId),
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
    Button {
        label: &'static str,
        action: Action,
    },
    Toggle {
        label: &'static str,
        setting: ToggleSetting,
    },
    Slider {
        label: &'static str,
        setting: SliderSetting,
    },
    Spacer,
}

pub(crate) fn items(screen: ScreenId) -> &'static [Item] {
    use Action::*;
    match screen {
        ScreenId::Main => &[
            Item::Button {
                label: "New Game",
                action: StartGame,
            },
            Item::Button {
                label: "Options",
                action: Open(ScreenId::Options),
            },
            Item::Button {
                label: "Quit",
                action: Quit,
            },
        ],
        ScreenId::Options => &[
            Item::Toggle {
                label: "Vsync",
                setting: ToggleSetting::Vsync,
            },
            Item::Toggle {
                label: "Fullscreen",
                setting: ToggleSetting::Fullscreen,
            },
            Item::Slider {
                label: "Master",
                setting: SliderSetting::Master,
            },
            Item::Slider {
                label: "Music",
                setting: SliderSetting::Music,
            },
            Item::Slider {
                label: "SFX",
                setting: SliderSetting::Sfx,
            },
            Item::Spacer,
            Item::Button {
                label: "Back",
                action: Back,
            },
        ],
        ScreenId::Pause => &[
            Item::Button {
                label: "Resume",
                action: Resume,
            },
            Item::Button {
                label: "Options",
                action: Open(ScreenId::Options),
            },
            Item::Button {
                label: "Quit to Menu",
                action: QuitToMenu,
            },
        ],
    }
}

fn focusable(item: &Item) -> bool {
    !matches!(item, Item::Spacer)
}

pub(crate) fn slider_value(s: &Settings, which: SliderSetting) -> f32 {
    match which {
        SliderSetting::Master => s.master_volume,
        SliderSetting::Music => s.music_volume,
        SliderSetting::Sfx => s.sfx_volume,
    }
}

fn set_slider(s: &mut Settings, which: SliderSetting, v: f32) {
    match which {
        SliderSetting::Master => s.master_volume = v,
        SliderSetting::Music => s.music_volume = v,
        SliderSetting::Sfx => s.sfx_volume = v,
    }
}

/// The menu state machine. Headless: it only consumes [`UiInput`] and produces [`MenuCommand`]s.
#[derive(Debug, Clone)]
pub struct Menu {
    pub(crate) stack: Vec<ScreenId>,
    pub(crate) focus: usize,
    pub(crate) settings: Settings,
}

impl Menu {
    /// A menu showing the main screen with the first item focused.
    pub fn new(settings: Settings) -> Menu {
        let mut m = Menu {
            stack: vec![ScreenId::Main],
            focus: 0,
            settings: settings.clamped(),
        };
        m.reset_focus();
        m
    }

    /// Show the pause screen (used when the player presses Escape in game).
    pub fn open_pause(&mut self) {
        self.stack = vec![ScreenId::Pause];
        self.reset_focus();
    }

    /// Show the main screen.
    pub fn open_main(&mut self) {
        self.stack = vec![ScreenId::Main];
        self.reset_focus();
    }

    /// The screen currently shown.
    pub fn screen(&self) -> ScreenId {
        *self.stack.last().expect("menu stack is never empty")
    }

    /// Index (into the screen's items) of the focused item.
    pub fn focused(&self) -> Option<usize> {
        items(self.screen())
            .get(self.focus)
            .filter(|i| focusable(i))
            .map(|_| self.focus)
    }

    /// Current settings as modified through the menu.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Feed one input event; returns what the app must do.
    pub fn handle(&mut self, input: UiInput) -> Vec<MenuCommand> {
        match input {
            UiInput::Nav(n) => self.handle_nav(n),
            UiInput::PointerMoved(_) | UiInput::PointerPressed(_) | UiInput::PointerReleased(_) => {
                Vec::new()
            }
        }
    }

    fn reset_focus(&mut self) {
        self.focus = items(self.screen()).iter().position(focusable).unwrap_or(0);
    }

    fn handle_nav(&mut self, n: NavInput) -> Vec<MenuCommand> {
        match n {
            NavInput::Up => self.move_focus(-1),
            NavInput::Down => self.move_focus(1),
            NavInput::Left => return self.adjust_slider(-1),
            NavInput::Right => return self.adjust_slider(1),
            NavInput::Activate => return self.activate(),
            NavInput::Back => return self.go_back(),
        }
        Vec::new()
    }

    fn move_focus(&mut self, dir: isize) {
        let list = items(self.screen());
        let n = list.len() as isize;
        let mut i = self.focus as isize;
        for _ in 0..n {
            i = (i + dir).rem_euclid(n);
            if focusable(&list[i as usize]) {
                self.focus = i as usize;
                return;
            }
        }
    }

    fn adjust_slider(&mut self, dir: i32) -> Vec<MenuCommand> {
        let Some(Item::Slider { setting, .. }) = items(self.screen()).get(self.focus).copied()
        else {
            return Vec::new();
        };
        let old = (slider_value(&self.settings, setting) / STEP).round() as i32;
        let new = (old + dir).clamp(0, MAX_STEPS);
        if new == old {
            return Vec::new();
        }
        set_slider(&mut self.settings, setting, new as f32 * STEP);
        vec![MenuCommand::SettingsChanged(self.settings.clone())]
    }

    fn activate(&mut self) -> Vec<MenuCommand> {
        match items(self.screen()).get(self.focus).copied() {
            Some(Item::Button { action, .. }) => self.perform(action),
            Some(Item::Toggle { setting, .. }) => {
                match setting {
                    ToggleSetting::Vsync => self.settings.vsync = !self.settings.vsync,
                    ToggleSetting::Fullscreen => {
                        self.settings.fullscreen = !self.settings.fullscreen
                    }
                }
                vec![MenuCommand::SettingsChanged(self.settings.clone())]
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn perform(&mut self, action: Action) -> Vec<MenuCommand> {
        match action {
            Action::StartGame => vec![MenuCommand::StartGame],
            Action::Resume => vec![MenuCommand::Resume],
            Action::Quit => vec![MenuCommand::Quit],
            Action::QuitToMenu => {
                self.open_main();
                vec![MenuCommand::QuitToMenu]
            }
            Action::Open(screen) => {
                self.stack.push(screen);
                self.reset_focus();
                Vec::new()
            }
            Action::Back => self.go_back(),
        }
    }

    fn go_back(&mut self) -> Vec<MenuCommand> {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.reset_focus();
            Vec::new()
        } else if self.screen() == ScreenId::Pause {
            vec![MenuCommand::Resume]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::input::NavInput;
    use ss2_core::settings::Settings;

    fn nav(m: &mut Menu, i: NavInput) -> Vec<MenuCommand> {
        m.handle(UiInput::Nav(i))
    }
    fn nav_n(m: &mut Menu, i: NavInput, n: usize) {
        for _ in 0..n {
            nav(m, i);
        }
    }
    fn to_options(m: &mut Menu) {
        nav(m, NavInput::Down);
        nav(m, NavInput::Activate);
        assert_eq!(m.screen(), ScreenId::Options);
    }
    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn starts_on_main_with_first_item_focused() {
        let m = Menu::new(Settings::default());
        assert_eq!(m.screen(), ScreenId::Main);
        assert_eq!(m.focused(), Some(0));
    }

    #[test]
    fn down_up_wrap_around() {
        let mut m = Menu::new(Settings::default());
        nav(&mut m, NavInput::Up);
        assert_eq!(m.focused(), Some(2));
        nav(&mut m, NavInput::Down);
        assert_eq!(m.focused(), Some(0));
    }

    #[test]
    fn spacer_is_skipped_by_focus() {
        let mut m = Menu::new(Settings::default());
        to_options(&mut m);
        nav_n(&mut m, NavInput::Down, 4); // Vsync -> Fullscreen -> Master -> Music -> SFX
        assert_eq!(m.focused(), Some(4));
        nav(&mut m, NavInput::Down); // skips the spacer at index 5
        assert_eq!(m.focused(), Some(6));
    }

    #[test]
    fn activate_new_game_emits_start_game() {
        let mut m = Menu::new(Settings::default());
        assert_eq!(
            nav(&mut m, NavInput::Activate),
            vec![MenuCommand::StartGame]
        );
    }

    #[test]
    fn activate_options_opens_options_and_back_returns_to_main() {
        let mut m = Menu::new(Settings::default());
        to_options(&mut m);
        assert!(nav(&mut m, NavInput::Back).is_empty());
        assert_eq!(m.screen(), ScreenId::Main);
    }

    #[test]
    fn back_on_main_emits_nothing() {
        let mut m = Menu::new(Settings::default());
        assert!(nav(&mut m, NavInput::Back).is_empty());
        assert_eq!(m.screen(), ScreenId::Main);
    }

    #[test]
    fn back_on_pause_emits_resume() {
        let mut m = Menu::new(Settings::default());
        m.open_pause();
        assert_eq!(m.screen(), ScreenId::Pause);
        assert_eq!(nav(&mut m, NavInput::Back), vec![MenuCommand::Resume]);
    }

    #[test]
    fn toggle_vsync_flips_and_emits_settings_changed() {
        let mut m = Menu::new(Settings::default());
        to_options(&mut m);
        let cmds = nav(&mut m, NavInput::Activate);
        assert!(!m.settings().vsync);
        assert_eq!(
            cmds,
            vec![MenuCommand::SettingsChanged(m.settings().clone())]
        );
    }

    #[test]
    fn slider_right_adds_step_and_emits() {
        let mut m = Menu::new(Settings::default());
        to_options(&mut m);
        nav_n(&mut m, NavInput::Down, 2); // Master
        let cmds = nav(&mut m, NavInput::Right);
        assert!(approx(m.settings().master_volume, 0.85));
        assert_eq!(
            cmds,
            vec![MenuCommand::SettingsChanged(m.settings().clone())]
        );
    }

    #[test]
    fn slider_clamps_at_one_without_emitting() {
        let mut m = Menu::new(Settings {
            master_volume: 1.0,
            ..Settings::default()
        });
        to_options(&mut m);
        nav_n(&mut m, NavInput::Down, 2);
        assert!(nav(&mut m, NavInput::Right).is_empty());
        assert_eq!(m.settings().master_volume, 1.0);
    }

    #[test]
    fn slider_left_clamps_at_zero() {
        let mut m = Menu::new(Settings {
            master_volume: 0.0,
            ..Settings::default()
        });
        to_options(&mut m);
        nav_n(&mut m, NavInput::Down, 2);
        assert!(nav(&mut m, NavInput::Left).is_empty());
        assert_eq!(m.settings().master_volume, 0.0);
    }

    #[test]
    fn quit_to_menu_from_pause_emits_quit_to_menu_and_shows_main() {
        let mut m = Menu::new(Settings::default());
        m.open_pause();
        nav_n(&mut m, NavInput::Down, 2); // Resume -> Options -> Quit to Menu
        assert_eq!(
            nav(&mut m, NavInput::Activate),
            vec![MenuCommand::QuitToMenu]
        );
        assert_eq!(m.screen(), ScreenId::Main);
    }

    #[test]
    fn options_from_pause_returns_to_pause() {
        let mut m = Menu::new(Settings::default());
        m.open_pause();
        nav(&mut m, NavInput::Down);
        nav(&mut m, NavInput::Activate);
        assert_eq!(m.screen(), ScreenId::Options);
        nav(&mut m, NavInput::Back);
        assert_eq!(m.screen(), ScreenId::Pause);
    }
}
