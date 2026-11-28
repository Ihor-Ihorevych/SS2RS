//! User-adjustable settings shared by the UI, platform layer and app.
//!
//! The settings file is TOML. Every file written by this build carries `version = N`
//! ([`SCHEMA_VERSION`]); older files are upgraded in memory by the [`MIGRATIONS`] chain when they
//! are loaded, and a file from a newer build still loads the fields this build knows. Defaults
//! depend on the platform class ([`PlatformClass`]), and individual values can be overridden
//! from the command line ([`Settings::apply_overrides`], `--set KEY=VALUE`).

use crate::{Error, Result};

/// The version of the settings file format written by this build.
pub const SCHEMA_VERSION: u32 = 1;

/// One step of the migration chain: rewrites the table of a file by one version.
pub type Migration = fn(&mut toml::Table);

/// `MIGRATIONS[n]` upgrades a version-`n` file to version `n + 1`. Add a function here (and bump
/// [`SCHEMA_VERSION`]) whenever a key is renamed, removed or changes meaning; adding a field with
/// a default needs no migration.
pub const MIGRATIONS: [Migration; SCHEMA_VERSION as usize] = [v0_to_v1];

/// Version 0 files had no version stamp and the same fields as version 1.
fn v0_to_v1(_table: &mut toml::Table) {}

/// Run the migrations from version `from` on and return the version the table has now. A
/// table that is already at or beyond `migrations.len()` is left alone.
pub fn migrate_table(table: &mut toml::Table, from: u32, migrations: &[Migration]) -> u32 {
    let mut version = from;
    while let Some(step) = usize::try_from(version)
        .ok()
        .and_then(|index| migrations.get(index))
    {
        step(table);
        version += 1;
    }
    version
}

/// The kind of device, for defaults that differ between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformClass {
    /// Windows, Linux and macOS: windowed by default.
    Desktop,
    /// Android and iOS: always fullscreen.
    Mobile,
}

impl PlatformClass {
    /// The class of the platform this build runs on.
    pub fn current() -> Self {
        if cfg!(any(target_os = "android", target_os = "ios")) {
            Self::Mobile
        } else {
            Self::Desktop
        }
    }
}

const OVERRIDE_KEYS: [&str; 6] = [
    "vsync",
    "fullscreen",
    "master_volume",
    "music_volume",
    "sfx_volume",
    "sky_fog",
];

/// Video/audio settings.
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
    /// Draw the sky as the flat fog colour while a haze is active, as the original does (`docs/re/environment.md`,
    /// T01-429); off keeps the sky visible in fog.
    pub sky_fog: bool,
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
            sky_fog: false,
        }
    }
}

impl Settings {
    /// `<config dir>/SS2RS/settings.toml` (for example `%APPDATA%\SS2RS\settings.toml` on
    /// Windows), or `None` if the platform has no config directory.
    pub fn default_path() -> Option<std::path::PathBuf> {
        default_path()
    }

    /// The defaults for a kind of device: mobile is always fullscreen.
    pub fn for_platform(class: PlatformClass) -> Settings {
        match class {
            PlatformClass::Desktop => Settings::default(),
            PlatformClass::Mobile => Settings {
                fullscreen: true,
                ..Settings::default()
            },
        }
    }

    /// Set one value from a `KEY=VALUE` text such as `vsync=off` or `master_volume=0.5` (the
    /// keys are the field names; booleans accept on/off, 1/0, true/false). Volumes are clamped.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming the problem: no `=`, an unknown key (the valid ones are listed)
    /// or a value that does not parse. A failed override changes nothing.
    pub fn apply_override(&mut self, text: &str) -> Result<()> {
        let bad = |detail: String| Error::invalid("settings override", detail);
        let (key, value) = text
            .split_once('=')
            .ok_or_else(|| bad(format!("{text:?} is not KEY=VALUE")))?;
        let (key, value) = (key.trim(), value.trim());
        let parse_bool = || match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "on" | "yes" => Ok(true),
            "0" | "false" | "off" | "no" => Ok(false),
            _ => Err(bad(format!(
                "{key}: {value:?} is not on/off, 1/0 or true/false"
            ))),
        };
        let parse_volume = || {
            value
                .parse::<f32>()
                .map_err(|_| bad(format!("{key}: {value:?} is not a number")))
        };
        let mut next = self.clone();
        match key {
            "vsync" => next.vsync = parse_bool()?,
            "fullscreen" => next.fullscreen = parse_bool()?,
            "master_volume" => next.master_volume = parse_volume()?,
            "music_volume" => next.music_volume = parse_volume()?,
            "sfx_volume" => next.sfx_volume = parse_volume()?,
            "sky_fog" => next.sky_fog = parse_bool()?,
            _ => {
                return Err(bad(format!(
                    "unknown setting {key:?}; valid settings: {}",
                    OVERRIDE_KEYS.join(", ")
                )));
            }
        }
        *self = next.clamped();
        Ok(())
    }

    /// Apply several overrides; if any of them is invalid nothing is changed.
    ///
    /// # Errors
    /// The first [`Error::Invalid`] from [`Settings::apply_override`].
    pub fn apply_overrides<I, S>(&mut self, overrides: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut trial = self.clone();
        for text in overrides {
            trial.apply_override(text.as_ref())?;
        }
        *self = trial;
        Ok(())
    }

    /// Load settings from `path`. Never fails: a missing, unreadable or malformed file yields
    /// the platform defaults (with a warning for the latter two), files from older builds are
    /// migrated, and values are clamped.
    pub fn load_from(path: &std::path::Path) -> Settings {
        let defaults = || Settings::for_platform(PlatformClass::current());
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return defaults(),
            Err(e) => {
                tracing::warn!(
                    "cannot read settings {}: {e}; using defaults",
                    path.display()
                );
                return defaults();
            }
        };
        let parsed = text
            .parse::<toml::Table>()
            .map_err(|e| e.to_string())
            .and_then(|mut table| {
                let version = table
                    .get("version")
                    .and_then(toml::Value::as_integer)
                    .and_then(|v| u32::try_from(v).ok())
                    .unwrap_or(0);
                if version > SCHEMA_VERSION {
                    tracing::warn!(
                        "settings {} were written by a newer build (version {version}); \
                         reading the fields this build knows",
                        path.display()
                    );
                }
                migrate_table(&mut table, version, &MIGRATIONS);
                table.try_into::<Settings>().map_err(|e| e.to_string())
            });
        match parsed {
            Ok(settings) => settings.clamped(),
            Err(e) => {
                tracing::warn!("malformed settings {}: {e}; using defaults", path.display());
                defaults()
            }
        }
    }

    /// Save settings to `path`, creating parent directories. The file is written to a
    /// temporary sibling and renamed, so a crash cannot leave a half-written settings file.
    pub fn save_to(&self, path: &std::path::Path) -> crate::Result<()> {
        let mut table = toml::Table::try_from(self)
            .map_err(|e| crate::Error::invalid("settings", e.to_string()))?;
        table.insert(
            "version".to_owned(),
            toml::Value::Integer(i64::from(SCHEMA_VERSION)),
        );
        let text = toml::to_string_pretty(&table)
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
