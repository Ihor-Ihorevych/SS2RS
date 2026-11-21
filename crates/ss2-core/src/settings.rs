//! User-adjustable settings shared by the UI, platform layer and app.

/// Video/audio settings (persistence is added alongside the platform config paths).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Synchronize presentation with the display refresh.
    pub vsync: bool,
    /// Borderless fullscreen.
    pub fullscreen: bool,
    /// Master volume, 0.0..=1.0.
    pub master_volume: f32,
    /// Music volume, 0.0..=1.0.
    pub music_volume: f32,
    /// Sound-effects volume, 0.0..=1.0.
    pub sfx_volume: f32,
}

/// Free-function form of [`Settings::default_path`].
pub fn default_path() -> Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join("SS2RS").join("settings.toml"))
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vsync: true,
            fullscreen: false,
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 0.8,
        }
    }
}

impl Settings {
    /// `<config dir>/SS2RS/settings.toml` (for example `%APPDATA%\SS2RS\settings.toml` on
    /// Windows), or `None` if the platform has no config directory.
    pub fn default_path() -> Option<std::path::PathBuf> {
        default_path()
    }

    /// Load settings from `path`. Never fails: a missing, unreadable or malformed file yields
    /// the defaults (with a warning for the latter two) and values are clamped.
    pub fn load_from(path: &std::path::Path) -> Settings {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Settings::default(),
            Err(e) => {
                tracing::warn!(
                    "cannot read settings {}: {e}; using defaults",
                    path.display()
                );
                return Settings::default();
            }
        };
        match toml::from_str::<Settings>(&text) {
            Ok(settings) => settings.clamped(),
            Err(e) => {
                tracing::warn!("malformed settings {}: {e}; using defaults", path.display());
                Settings::default()
            }
        }
    }

    /// Save settings to `path`, creating parent directories. The file is written to a
    /// temporary sibling and renamed, so a crash cannot leave a half-written settings file.
    pub fn save_to(&self, path: &std::path::Path) -> crate::Result<()> {
        let text = toml::to_string_pretty(self)
            .map_err(|e| crate::Error::invalid("settings", e.to_string()))?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| crate::Error::io(parent, e))?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text).map_err(|e| crate::Error::io(&tmp, e))?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            crate::Error::io(path, e)
        })
    }

    /// Clamp volumes into `0.0..=1.0`; `NaN` volumes fall back to their default.
    pub fn clamped(self) -> Settings {
        let d = Settings::default();
        let fix = |v: f32, default: f32| {
            if v.is_nan() {
                default
            } else {
                v.clamp(0.0, 1.0)
            }
        };
        Settings {
            master_volume: fix(self.master_volume, d.master_volume),
            music_volume: fix(self.music_volume, d.music_volume),
            sfx_volume: fix(self.sfx_volume, d.sfx_volume),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_documented_values() {
        let s = Settings::default();
        assert!(s.vsync && !s.fullscreen);
        assert_eq!(
            (s.master_volume, s.music_volume, s.sfx_volume),
            (0.8, 0.6, 0.8)
        );
    }

    #[test]
    fn clamped_limits_volumes_and_replaces_nan() {
        let s = Settings {
            master_volume: 7.0,
            music_volume: -1.0,
            sfx_volume: f32::NAN,
            ..Settings::default()
        }
        .clamped();
        assert_eq!(s.master_volume, 1.0);
        assert_eq!(s.music_volume, 0.0);
        assert_eq!(s.sfx_volume, Settings::default().sfx_volume);
    }
}
