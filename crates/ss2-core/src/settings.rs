//! User-adjustable settings shared by the UI, platform layer and app.

/// Video/audio settings (persistence is added alongside the platform config paths).
#[derive(Debug, Clone, PartialEq)]
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
