//! Pure application state: which scene is active and what each menu command means.

use ss2_core::settings::Settings;
use ss2_ui::MenuCommand;

/// What the app is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    /// A menu is open.
    Menu,
    /// The (placeholder) game scene.
    Playing,
}

/// Side effects the window layer must perform after a command.
#[derive(Debug, Default, PartialEq)]
pub struct Effects {
    /// Exit the application.
    pub exit: bool,
    /// New vsync value, if it changed.
    pub set_vsync: Option<bool>,
    /// New fullscreen value, if it changed.
    pub set_fullscreen: Option<bool>,
    /// Persist the settings to disk.
    pub save_settings: bool,
}

/// Scene plus the current settings.
#[derive(Debug)]
pub struct AppState {
    /// Active scene.
    pub scene: Scene,
    /// Current settings.
    pub settings: Settings,
}

impl AppState {
    /// Start in the menu with the given settings.
    pub fn new(settings: Settings) -> Self {
        Self {
            scene: Scene::Menu,
            settings,
        }
    }

    /// Apply one menu command and report the side effects.
    pub fn apply(&mut self, cmd: MenuCommand) -> Effects {
        let mut fx = Effects::default();
        match cmd {
            MenuCommand::StartGame | MenuCommand::Resume => self.scene = Scene::Playing,
            // The menu stays the scene: the wizard shows over it until it closes (`app`).
            MenuCommand::QuitToMenu | MenuCommand::ChooseGameFolder => self.scene = Scene::Menu,
            MenuCommand::Quit => fx.exit = true,
            MenuCommand::SettingsChanged(new) => {
                if new.vsync != self.settings.vsync {
                    fx.set_vsync = Some(new.vsync);
                }
                if new.fullscreen != self.settings.fullscreen {
                    fx.set_fullscreen = Some(new.fullscreen);
                }
                fx.save_settings = true;
                self.settings = new;
            }
        }
        fx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::settings::Settings;
    use ss2_ui::MenuCommand;

    fn state() -> AppState {
        AppState::new(Settings::default())
    }

    #[test]
    fn start_game_enters_playing() {
        let mut s = state();
        assert_eq!(s.apply(MenuCommand::StartGame), Effects::default());
        assert_eq!(s.scene, Scene::Playing);
    }

    #[test]
    fn resume_enters_playing() {
        let mut s = state();
        s.scene = Scene::Menu;
        s.apply(MenuCommand::Resume);
        assert_eq!(s.scene, Scene::Playing);
    }

    #[test]
    fn quit_to_menu_returns_to_menu() {
        let mut s = state();
        s.apply(MenuCommand::StartGame);
        s.apply(MenuCommand::QuitToMenu);
        assert_eq!(s.scene, Scene::Menu);
    }

    #[test]
    fn quit_sets_exit() {
        assert!(state().apply(MenuCommand::Quit).exit);
    }

    #[test]
    fn settings_changed_reports_only_changed_fields() {
        let mut s = state();
        let new = Settings {
            vsync: false,
            ..Settings::default()
        };
        let fx = s.apply(MenuCommand::SettingsChanged(new.clone()));
        assert_eq!(
            fx,
            Effects {
                exit: false,
                set_vsync: Some(false),
                set_fullscreen: None,
                save_settings: true
            }
        );
        assert_eq!(s.settings, new);
    }

    #[test]
    fn volume_only_change_has_no_vsync_or_fullscreen_effect() {
        let mut s = state();
        let fx = s.apply(MenuCommand::SettingsChanged(Settings {
            master_volume: 0.5,
            ..Settings::default()
        }));
        assert_eq!(fx.set_vsync, None);
        assert_eq!(fx.set_fullscreen, None);
        assert!(fx.save_settings);
    }

    #[test]
    fn fullscreen_change_is_reported() {
        let mut s = state();
        let fx = s.apply(MenuCommand::SettingsChanged(Settings {
            fullscreen: true,
            ..Settings::default()
        }));
        assert_eq!(fx.set_fullscreen, Some(true));
    }
}
