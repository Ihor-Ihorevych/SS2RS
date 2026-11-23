//! Graphics quality: **presets** (Ultra to Low), **per-setting values** a player can change and a console or settings screen can list, and a **dynamic
//! resolution** controller (roadmap T04-020, `docs/QUALITY.md`).
//!
//! All device-free logic: nothing here touches a GPU. The renderer reads a [`QualitySettings`] to decide what to draw and how well; what each setting does
//! in the renderer is up to the passes it controls (shadows, anti-aliasing and the rest do not exist yet: the values are the contract).
//!
//! * [`QualitySettings::preset`] gives the values of a [`QualityTier`]; the four tiers are **ordered**: every setting of a lower tier is no better than the
//!   tier above (a test checks it for every setting).
//! * Each setting is **named** ([`QualitySettings::options`], [`get`](QualitySettings::get), [`set`](QualitySettings::set)): `shadows`, `anisotropy`,
//!   `render_scale`... so a console command (`r_shadows high`) and the settings screen use the same list, and a value that is not allowed is **refused** with
//!   the reason and changes nothing.
//! * [`QualitySettings::tier_matching`] says which preset the values equal, or `None` for "Custom".
//! * [`DynamicResolution`] watches frame times and lowers or raises the render scale between limits: **quickly down** when frames are too slow, **slowly up**
//!   when there is room, in steps, with a pause after each change so it does not oscillate.

use std::fmt;

/// A quality preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QualityTier {
    /// The lowest: for weak or mobile GPUs.
    Low,
    /// Balanced for modest hardware.
    Medium,
    /// Good looking on a mid-range GPU.
    High,
    /// Everything on.
    Ultra,
}

impl QualityTier {
    /// Every tier, lowest first.
    pub const ALL: [QualityTier; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// The name a settings file or console uses.
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }

    /// The tier called `name` (any case).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.trim()))
    }
}

/// How good the shadows are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShadowQuality {
    /// No dynamic shadows.
    Off,
    /// A few lights, small maps, hard edges.
    Low,
    /// More lights, medium maps.
    Medium,
    /// Everything that can cast, large maps, soft edges.
    High,
}

/// The size of textures: how many of the top mip levels are skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureDetail {
    /// A quarter of the size (two levels skipped).
    Quarter,
    /// Half the size (one level skipped).
    Half,
    /// As authored.
    Full,
}

/// Anti-aliasing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AntiAliasing {
    /// None.
    Off,
    /// A cheap post-process filter.
    Fxaa,
    /// 2 samples per pixel.
    Msaa2,
    /// 4 samples per pixel.
    Msaa4,
}

/// Every graphics setting. Field values are always inside the ranges [`QualitySettings::validate`] checks.
#[derive(Clone, Debug, PartialEq)]
pub struct QualitySettings {
    /// Dynamic shadows.
    pub shadows: ShadowQuality,
    /// Texture size.
    pub texture_detail: TextureDetail,
    /// Anisotropic filtering: 1, 2, 4, 8 or 16.
    pub anisotropy: u8,
    /// Anti-aliasing.
    pub anti_aliasing: AntiAliasing,
    /// Model level-of-detail bias, -1.0 (sharper, further) to 3.0 (coarser, sooner): added to the LOD level.
    pub lod_bias: f32,
    /// Multiplier on how far things are drawn, 0.5 to 2.0.
    pub view_distance: f32,
    /// Multiplier on how many particles are spawned, 0.1 to 1.0.
    pub particle_density: f32,
    /// Bloom.
    pub bloom: bool,
    /// The resolution the 3D scene is rendered at, as a share of the window, 0.5 to 1.0 (the UI is always at full resolution).
    pub render_scale: f32,
    /// Let [`DynamicResolution`] lower `render_scale` when frames are slow (it never raises it above the setting).
    pub dynamic_resolution: bool,
    /// The most decals kept alive, 0 to 1000.
    pub max_decals: u32,
}

/// What kind of value a setting takes.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionKind {
    /// One of these words, lowest first.
    Choice(&'static [&'static str]),
    /// A number from `min` to `max` (inclusive).
    Range {
        /// The smallest value.
        min: f32,
        /// The largest value.
        max: f32,
    },
    /// An integer from `min` to `max` (inclusive).
    Count {
        /// The smallest value.
        min: u32,
        /// The largest value.
        max: u32,
    },
    /// `true` or `false` (`on`, `off`, `1`, `0`).
    Toggle,
}

/// A setting a player can change, for a list or a console.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionDesc {
    /// The name (lower case, underscores).
    pub name: &'static str,
    /// What it takes.
    pub kind: OptionKind,
}

const SHADOW_WORDS: &[&str] = &["off", "low", "medium", "high"];
const TEXTURE_WORDS: &[&str] = &["quarter", "half", "full"];
const AA_WORDS: &[&str] = &["off", "fxaa", "msaa2", "msaa4"];

/// Why a setting could not be set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetError {
    /// There is no setting of that name.
    UnknownOption(String),
    /// The text is not a valid value of that setting (what was expected).
    BadValue {
        /// The setting.
        option: &'static str,
        /// What it takes.
        expected: String,
    },
}

impl fmt::Display for SetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption(n) => write!(f, "there is no graphics setting called {n:?}"),
            Self::BadValue { option, expected } => write!(f, "{option} takes {expected}"),
        }
    }
}

impl std::error::Error for SetError {}

fn index_of(words: &[&str], text: &str) -> Option<usize> {
    words
        .iter()
        .position(|w| w.eq_ignore_ascii_case(text.trim()))
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "1" | "yes" => Some(true),
        "off" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn parse_range(text: &str, min: f32, max: f32) -> Option<f32> {
    let v: f32 = text.trim().parse().ok()?;
    (v.is_finite() && (min..=max).contains(&v)).then_some(v)
}

fn describe(kind: &OptionKind) -> String {
    match kind {
        OptionKind::Choice(words) => format!("one of {}", words.join(", ")),
        OptionKind::Range { min, max } => format!("a number from {min} to {max}"),
        OptionKind::Count { min, max } => format!("a whole number from {min} to {max}"),
        OptionKind::Toggle => "on or off".to_owned(),
    }
}

impl QualitySettings {
    /// The values of a preset.
    pub fn preset(tier: QualityTier) -> Self {
        match tier {
            QualityTier::Low => Self {
                shadows: ShadowQuality::Off,
                texture_detail: TextureDetail::Quarter,
                anisotropy: 1,
                anti_aliasing: AntiAliasing::Off,
                lod_bias: 2.0,
                view_distance: 0.6,
                particle_density: 0.3,
                bloom: false,
                render_scale: 0.7,
                dynamic_resolution: true,
                max_decals: 50,
            },
            QualityTier::Medium => Self {
                shadows: ShadowQuality::Low,
                texture_detail: TextureDetail::Half,
                anisotropy: 4,
                anti_aliasing: AntiAliasing::Fxaa,
                lod_bias: 1.0,
                view_distance: 0.8,
                particle_density: 0.6,
                bloom: true,
                render_scale: 0.85,
                dynamic_resolution: true,
                max_decals: 150,
            },
            QualityTier::High => Self {
                shadows: ShadowQuality::Medium,
                texture_detail: TextureDetail::Full,
                anisotropy: 8,
                anti_aliasing: AntiAliasing::Msaa2,
                lod_bias: 0.0,
                view_distance: 1.0,
                particle_density: 0.85,
                bloom: true,
                render_scale: 1.0,
                dynamic_resolution: false,
                max_decals: 400,
            },
            QualityTier::Ultra => Self {
                shadows: ShadowQuality::High,
                texture_detail: TextureDetail::Full,
                anisotropy: 16,
                anti_aliasing: AntiAliasing::Msaa4,
                lod_bias: -0.5,
                view_distance: 1.5,
                particle_density: 1.0,
                bloom: true,
                render_scale: 1.0,
                dynamic_resolution: false,
                max_decals: 1000,
            },
        }
    }

    /// Which preset these values equal exactly, or `None` (a "Custom" setup).
    pub fn tier_matching(&self) -> Option<QualityTier> {
        QualityTier::ALL
            .into_iter()
            .find(|t| Self::preset(*t) == *self)
    }

    /// Whether every value is inside its allowed range.
    pub fn validate(&self) -> Result<(), SetError> {
        let bad = |option: &'static str, kind: OptionKind| {
            Err(SetError::BadValue {
                option,
                expected: describe(&kind),
            })
        };
        if !matches!(self.anisotropy, 1 | 2 | 4 | 8 | 16) {
            return bad(
                "anisotropy",
                OptionKind::Choice(&["1", "2", "4", "8", "16"]),
            );
        }
        let checks: [(&'static str, f32, f32, f32); 4] = [
            ("lod_bias", self.lod_bias, -1.0, 3.0),
            ("view_distance", self.view_distance, 0.5, 2.0),
            ("particle_density", self.particle_density, 0.1, 1.0),
            ("render_scale", self.render_scale, 0.5, 1.0),
        ];
        for (name, v, min, max) in checks {
            if !(v.is_finite() && (min..=max).contains(&v)) {
                return bad(name, OptionKind::Range { min, max });
            }
        }
        if self.max_decals > 1000 {
            return bad("max_decals", OptionKind::Count { min: 0, max: 1000 });
        }
        Ok(())
    }

    /// Every setting, in the order a settings screen lists them.
    pub fn options() -> Vec<OptionDesc> {
        vec![
            OptionDesc {
                name: "shadows",
                kind: OptionKind::Choice(SHADOW_WORDS),
            },
            OptionDesc {
                name: "texture_detail",
                kind: OptionKind::Choice(TEXTURE_WORDS),
            },
            OptionDesc {
                name: "anisotropy",
                kind: OptionKind::Choice(&["1", "2", "4", "8", "16"]),
            },
            OptionDesc {
                name: "anti_aliasing",
                kind: OptionKind::Choice(AA_WORDS),
            },
            OptionDesc {
                name: "lod_bias",
                kind: OptionKind::Range {
                    min: -1.0,
                    max: 3.0,
                },
            },
            OptionDesc {
                name: "view_distance",
                kind: OptionKind::Range { min: 0.5, max: 2.0 },
            },
            OptionDesc {
                name: "particle_density",
                kind: OptionKind::Range { min: 0.1, max: 1.0 },
            },
            OptionDesc {
                name: "bloom",
                kind: OptionKind::Toggle,
            },
            OptionDesc {
                name: "render_scale",
                kind: OptionKind::Range { min: 0.5, max: 1.0 },
            },
            OptionDesc {
                name: "dynamic_resolution",
                kind: OptionKind::Toggle,
            },
            OptionDesc {
                name: "max_decals",
                kind: OptionKind::Count { min: 0, max: 1000 },
            },
        ]
    }

    /// The current value of a setting as text (the form [`set`](Self::set) reads back); `None` for an unknown name.
    pub fn get(&self, name: &str) -> Option<String> {
        Some(match name {
            "shadows" => SHADOW_WORDS[self.shadows as usize].to_owned(),
            "texture_detail" => TEXTURE_WORDS[self.texture_detail as usize].to_owned(),
            "anisotropy" => self.anisotropy.to_string(),
            "anti_aliasing" => AA_WORDS[self.anti_aliasing as usize].to_owned(),
            "lod_bias" => self.lod_bias.to_string(),
            "view_distance" => self.view_distance.to_string(),
            "particle_density" => self.particle_density.to_string(),
            "bloom" => if self.bloom { "on" } else { "off" }.to_owned(),
            "render_scale" => self.render_scale.to_string(),
            "dynamic_resolution" => if self.dynamic_resolution { "on" } else { "off" }.to_owned(),
            "max_decals" => self.max_decals.to_string(),
            _ => return None,
        })
    }

    /// Set a setting from text (a console command or a settings file). An unknown name or a value that is not allowed is refused and **nothing changes**.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), SetError> {
        let desc = Self::options()
            .into_iter()
            .find(|d| d.name == name)
            .ok_or_else(|| SetError::UnknownOption(name.to_owned()))?;
        let bad = || SetError::BadValue {
            option: desc.name,
            expected: describe(&desc.kind),
        };
        match name {
            "shadows" => {
                self.shadows = [
                    ShadowQuality::Off,
                    ShadowQuality::Low,
                    ShadowQuality::Medium,
                    ShadowQuality::High,
                ][index_of(SHADOW_WORDS, value).ok_or_else(bad)?];
            }
            "texture_detail" => {
                self.texture_detail = [
                    TextureDetail::Quarter,
                    TextureDetail::Half,
                    TextureDetail::Full,
                ][index_of(TEXTURE_WORDS, value).ok_or_else(bad)?];
            }
            "anisotropy" => {
                self.anisotropy = match value.trim() {
                    "1" => 1,
                    "2" => 2,
                    "4" => 4,
                    "8" => 8,
                    "16" => 16,
                    _ => return Err(bad()),
                };
            }
            "anti_aliasing" => {
                self.anti_aliasing = [
                    AntiAliasing::Off,
                    AntiAliasing::Fxaa,
                    AntiAliasing::Msaa2,
                    AntiAliasing::Msaa4,
                ][index_of(AA_WORDS, value).ok_or_else(bad)?];
            }
            "lod_bias" => self.lod_bias = parse_range(value, -1.0, 3.0).ok_or_else(bad)?,
            "view_distance" => self.view_distance = parse_range(value, 0.5, 2.0).ok_or_else(bad)?,
            "particle_density" => {
                self.particle_density = parse_range(value, 0.1, 1.0).ok_or_else(bad)?
            }
            "bloom" => self.bloom = parse_bool(value).ok_or_else(bad)?,
            "render_scale" => self.render_scale = parse_range(value, 0.5, 1.0).ok_or_else(bad)?,
            "dynamic_resolution" => self.dynamic_resolution = parse_bool(value).ok_or_else(bad)?,
            "max_decals" => {
                let n: u32 = value.trim().parse().map_err(|_| bad())?;
                if n > 1000 {
                    return Err(bad());
                }
                self.max_decals = n;
            }
            _ => return Err(SetError::UnknownOption(name.to_owned())),
        }
        Ok(())
    }
}

/// What applying a settings text did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplyReport {
    /// The names of the settings whose value changed, in the order of [`QualitySettings::options`].
    pub changed: Vec<&'static str>,
    /// Lines that could not be applied, as `(line number from 1, message)`. When there are any, **nothing** was applied.
    pub errors: Vec<(usize, String)>,
}

impl ApplyReport {
    /// Whether the text was applied.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl QualitySettings {
    /// Apply a settings file's text: one `name = value` (or `name value`) per line, `#` starts a comment, blank lines are skipped, and the special name `preset`
    /// loads a whole [`QualityTier`] at that point so that the lines after it can adjust it. **All or nothing**: if any line is wrong, the settings are left exactly as
    /// they were and the report lists every bad line, so a half-saved edit never leaves the renderer in a mixed state. This is what a hot reload of the settings file
    /// calls; the report's `changed` says which settings to rebuild things for.
    ///
    /// ```
    /// use ss2_render_api::quality::QualitySettings;
    ///
    /// let mut s = QualitySettings::default();
    /// let report = s.apply_text("preset = low\n# a little better than low\nshadows = medium\n");
    /// assert!(report.is_ok());
    /// assert_eq!(s.get("shadows").as_deref(), Some("medium"));
    /// assert_eq!(s.get("bloom").as_deref(), Some("off")); // from the preset
    /// assert!(report.changed.contains(&"bloom")); // high had it on
    /// assert!(!report.changed.contains(&"shadows")); // medium is what high already had
    /// ```
    pub fn apply_text(&mut self, text: &str) -> ApplyReport {
        let mut next = self.clone();
        let mut errors = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (name, value) = match line.split_once('=') {
                Some((n, v)) => (n.trim(), v.trim()),
                None => match line.split_once(char::is_whitespace) {
                    Some((n, v)) => (n.trim(), v.trim()),
                    None => (line, ""),
                },
            };
            if name == "preset" {
                match QualityTier::from_name(value) {
                    Some(tier) => next = Self::preset(tier),
                    None => errors.push((
                        i + 1,
                        format!("preset takes one of low, medium, high, ultra (not {value:?})"),
                    )),
                }
            } else if let Err(e) = next.set(name, value) {
                errors.push((i + 1, e.to_string()));
            }
        }
        if !errors.is_empty() {
            return ApplyReport {
                changed: Vec::new(),
                errors,
            };
        }
        let changed = Self::options()
            .into_iter()
            .map(|o| o.name)
            .filter(|n| self.get(n) != next.get(n))
            .collect();
        *self = next;
        ApplyReport { changed, errors }
    }
}

impl Default for QualitySettings {
    /// The `High` preset.
    fn default() -> Self {
        Self::preset(QualityTier::High)
    }
}

/// How [`DynamicResolution`] behaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicResolutionConfig {
    /// The frame time to stay under, in milliseconds (16.6 for 60 fps).
    pub target_ms: f32,
    /// The lowest scale it may go to (0.5 to 1.0).
    pub min_scale: f32,
    /// The size of one step (0.01 to 0.25).
    pub step: f32,
    /// Frames averaged before a decision (at least 1).
    pub window: usize,
    /// Lower the scale when the average is above `target_ms * down_threshold`.
    pub down_threshold: f32,
    /// Raise it when the average is below `target_ms * up_threshold` for `up_after` windows in a row.
    pub up_threshold: f32,
    /// How many windows in a row with room before going up.
    pub up_after: u32,
}

impl Default for DynamicResolutionConfig {
    /// 60 fps, down to 0.5 in steps of 0.05, deciding on 8 frames, down above 100 % of the target, up after 4 windows below 80 %.
    fn default() -> Self {
        Self {
            target_ms: 1000.0 / 60.0,
            min_scale: 0.5,
            step: 0.05,
            window: 8,
            down_threshold: 1.0,
            up_threshold: 0.8,
            up_after: 4,
        }
    }
}

impl DynamicResolutionConfig {
    /// Whether the values make sense.
    pub fn validate(&self) -> Result<(), &'static str> {
        let ok_f = |x: f32| x.is_finite() && x > 0.0;
        if !ok_f(self.target_ms) {
            return Err("the target frame time must be positive");
        }
        if !(0.5..=1.0).contains(&self.min_scale) || !(0.01..=0.25).contains(&self.step) {
            return Err("min_scale must be 0.5 to 1.0 and step 0.01 to 0.25");
        }
        if self.window == 0 || self.up_after == 0 {
            return Err("window and up_after must be at least 1");
        }
        if !(ok_f(self.down_threshold)
            && ok_f(self.up_threshold)
            && self.up_threshold < self.down_threshold)
        {
            return Err("up_threshold must be positive and below down_threshold");
        }
        Ok(())
    }
}

/// Lowers and raises the render scale from frame times. Feed it **one frame time per frame** ([`DynamicResolution::frame`]) and render at the scale
/// it returns. Deterministic: no clock inside; the same frame times give the same scales.
///
/// ```
/// use ss2_render_api::quality::{DynamicResolution, DynamicResolutionConfig};
///
/// let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
/// let mut scale = 1.0;
/// for _ in 0..200 {
///     // A scene whose cost grows with the pixel count: 25 ms at full scale.
///     scale = dr.frame(25.0 * scale * scale);
/// }
/// assert!(scale < 0.85 && scale >= 0.5, "settles where the frame fits: {scale}");
/// ```
#[derive(Clone, Debug)]
pub struct DynamicResolution {
    cfg: DynamicResolutionConfig,
    max_scale: f32,
    scale: f32,
    sum_ms: f32,
    count: usize,
    calm_windows: u32,
}

impl DynamicResolution {
    /// A controller starting at `max_scale` (the render scale the player chose, 0.5 to 1.0), which it never exceeds.
    pub fn new(cfg: DynamicResolutionConfig, max_scale: f32) -> Result<Self, &'static str> {
        cfg.validate()?;
        if !(0.5..=1.0).contains(&max_scale) || max_scale < cfg.min_scale {
            return Err("the starting scale must be between the minimum scale and 1.0");
        }
        Ok(Self {
            cfg,
            max_scale,
            scale: max_scale,
            sum_ms: 0.0,
            count: 0,
            calm_windows: 0,
        })
    }

    /// The scale to render at now.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Change the ceiling (the player moved the render scale slider); the current scale follows if it is above it.
    pub fn set_max_scale(&mut self, max_scale: f32) {
        if (0.5..=1.0).contains(&max_scale) {
            self.max_scale = max_scale.max(self.cfg.min_scale);
            self.scale = self.scale.min(self.max_scale);
        }
    }

    /// Start again from the ceiling (a new level, a changed window).
    pub fn reset(&mut self) {
        self.scale = self.max_scale;
        self.sum_ms = 0.0;
        self.count = 0;
        self.calm_windows = 0;
    }

    /// Report the time the last frame took in milliseconds; returns the scale for the next. A time that is not a positive number is ignored
    /// (a frame that stalled on a window event should not shrink the picture).
    pub fn frame(&mut self, frame_ms: f32) -> f32 {
        if !(frame_ms.is_finite() && frame_ms > 0.0) {
            return self.scale;
        }
        self.sum_ms += frame_ms;
        self.count += 1;
        if self.count < self.cfg.window {
            return self.scale;
        }
        let average = self.sum_ms / self.count as f32;
        self.sum_ms = 0.0;
        self.count = 0;
        if average > self.cfg.target_ms * self.cfg.down_threshold {
            // Too slow: down at once, by more when it is much too slow.
            let over = average / (self.cfg.target_ms * self.cfg.down_threshold);
            let steps = if over > 1.5 { 2.0 } else { 1.0 };
            self.scale = quantize(
                (self.scale - steps * self.cfg.step).max(self.cfg.min_scale),
                self.cfg.step,
            );
            self.calm_windows = 0;
        } else if average < self.cfg.target_ms * self.cfg.up_threshold {
            self.calm_windows += 1;
            if self.calm_windows >= self.cfg.up_after && self.scale < self.max_scale {
                self.scale = quantize(
                    (self.scale + self.cfg.step).min(self.max_scale),
                    self.cfg.step,
                )
                .min(self.max_scale);
                self.calm_windows = 0;
            }
        } else {
            self.calm_windows = 0;
        }
        self.scale
    }
}

/// Round to a multiple of `step` (so a scale does not drift off the grid) without leaving `0.5..=1.0`.
fn quantize(scale: f32, step: f32) -> f32 {
    ((scale / step).round() * step).clamp(0.5, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- presets and settings ------------------------------------------------------------------

    #[test]
    fn every_preset_is_valid_distinct_and_recognised() {
        for tier in QualityTier::ALL {
            let s = QualitySettings::preset(tier);
            s.validate().unwrap_or_else(|e| panic!("{tier:?}: {e}"));
            assert_eq!(s.tier_matching(), Some(tier));
        }
        let all: Vec<QualitySettings> = QualityTier::ALL
            .into_iter()
            .map(QualitySettings::preset)
            .collect();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                assert_ne!(all[i], all[j], "{i} and {j} are the same preset");
            }
        }
        assert_eq!(
            QualitySettings::default(),
            QualitySettings::preset(QualityTier::High)
        );
        let mut custom = QualitySettings::preset(QualityTier::High);
        custom.bloom = false;
        assert_eq!(custom.tier_matching(), None, "a changed preset is custom");
    }

    #[test]
    fn a_lower_tier_is_never_better_than_the_one_above_in_any_setting() {
        let tiers: Vec<QualitySettings> = QualityTier::ALL
            .into_iter()
            .map(QualitySettings::preset)
            .collect();
        for pair in tiers.windows(2) {
            let (lo, hi) = (&pair[0], &pair[1]);
            assert!(
                lo.shadows <= hi.shadows
                    && lo.texture_detail <= hi.texture_detail
                    && lo.anisotropy <= hi.anisotropy
            );
            assert!(
                lo.anti_aliasing <= hi.anti_aliasing
                    && lo.bloom <= hi.bloom
                    && lo.max_decals <= hi.max_decals
            );
            assert!(
                lo.lod_bias >= hi.lod_bias,
                "a higher LOD bias is coarser: {} vs {}",
                lo.lod_bias,
                hi.lod_bias
            );
            assert!(
                lo.view_distance <= hi.view_distance
                    && lo.particle_density <= hi.particle_density
                    && lo.render_scale <= hi.render_scale
            );
        }
        assert!(QualityTier::Low < QualityTier::Ultra);
    }

    #[test]
    fn tiers_have_names_that_read_back() {
        for tier in QualityTier::ALL {
            assert_eq!(QualityTier::from_name(tier.name()), Some(tier));
            assert_eq!(
                QualityTier::from_name(&tier.name().to_uppercase()),
                Some(tier)
            );
        }
        assert_eq!(QualityTier::from_name("  High "), Some(QualityTier::High));
        assert_eq!(QualityTier::from_name("extreme"), None);
        assert_eq!(QualityTier::from_name(""), None);
    }

    #[test]
    fn every_option_can_be_read_and_set_back_for_every_preset() {
        for tier in QualityTier::ALL {
            let original = QualitySettings::preset(tier);
            let mut copy = QualitySettings::preset(QualityTier::Low);
            for option in QualitySettings::options() {
                let value = original
                    .get(option.name)
                    .unwrap_or_else(|| panic!("{} has no value", option.name));
                copy.set(option.name, &value)
                    .unwrap_or_else(|e| panic!("{}={value}: {e}", option.name));
            }
            assert_eq!(copy, original, "{tier:?}");
        }
    }

    #[test]
    fn the_option_list_covers_every_field_and_describes_how_to_set_each() {
        let options = QualitySettings::options();
        assert_eq!(options.len(), 11);
        let names: Vec<&str> = options.iter().map(|o| o.name).collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "names are unique");
        assert_eq!(QualitySettings::default().get("nonsense"), None);
        for o in &options {
            assert!(
                o.name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{}",
                o.name
            );
            match &o.kind {
                OptionKind::Choice(words) => assert!(words.len() >= 2),
                OptionKind::Range { min, max } => assert!(min < max),
                OptionKind::Count { min, max } => assert!(min < max),
                OptionKind::Toggle => {}
            }
        }
    }

    #[test]
    fn values_are_accepted_loosely_and_refused_strictly_without_changing_anything() {
        let mut s = QualitySettings::default();
        s.set("shadows", " HIGH ").unwrap();
        assert_eq!(s.shadows, ShadowQuality::High);
        s.set("bloom", "no").unwrap();
        assert!(!s.bloom);
        s.set("bloom", "1").unwrap();
        s.set("render_scale", "0.75").unwrap();
        assert_eq!(s.render_scale, 0.75);
        s.set("max_decals", "0").unwrap();
        s.set("max_decals", "1000").unwrap();
        s.set("lod_bias", "-1").unwrap();
        s.set("lod_bias", "3").unwrap();
        let before = s.clone();
        for (name, value) in [
            ("shadows", "ultra"),
            ("shadows", ""),
            ("texture_detail", "huge"),
            ("anisotropy", "3"),
            ("anisotropy", "32"),
            ("anisotropy", "x"),
            ("anti_aliasing", "taa"),
            ("lod_bias", "3.1"),
            ("lod_bias", "-1.1"),
            ("lod_bias", "NaN"),
            ("lod_bias", "inf"),
            ("view_distance", "0.4"),
            ("view_distance", "2.1"),
            ("particle_density", "0"),
            ("particle_density", "1.1"),
            ("render_scale", "0.49"),
            ("render_scale", "1.01"),
            ("render_scale", ""),
            ("bloom", "maybe"),
            ("dynamic_resolution", "2"),
            ("max_decals", "1001"),
            ("max_decals", "-1"),
            ("max_decals", "1.5"),
        ] {
            let err = s.set(name, value).unwrap_err();
            assert!(
                matches!(err, SetError::BadValue { .. }),
                "{name}={value}: {err}"
            );
            assert!(err.to_string().starts_with(name), "{err}");
        }
        assert_eq!(s, before, "refused values changed nothing");
        assert_eq!(
            s.set("nope", "1"),
            Err(SetError::UnknownOption("nope".into()))
        );
        assert!(
            SetError::UnknownOption("nope".into())
                .to_string()
                .contains("nope")
        );
    }

    #[test]
    fn validate_catches_values_set_directly() {
        let mut s = QualitySettings::default();
        assert!(s.validate().is_ok());
        for mutate in [
            (|s: &mut QualitySettings| s.anisotropy = 3) as fn(&mut QualitySettings),
            |s| s.lod_bias = f32::NAN,
            |s| s.view_distance = 0.1,
            |s| s.particle_density = 2.0,
            |s| s.render_scale = 0.2,
            |s| s.max_decals = 5_000,
        ] {
            mutate(&mut s);
            assert!(s.validate().is_err());
            s = QualitySettings::default();
        }
    }

    // ---- settings text ----------------------------------------------------------------------------------

    #[test]
    fn a_settings_text_applies_a_preset_then_the_lines_after_it() {
        let mut s = QualitySettings::preset(QualityTier::Ultra);
        let report = s.apply_text("preset = low\nshadows = medium\nrender_scale 0.9\n");
        assert!(report.is_ok(), "{report:?}");
        assert_eq!(s.get("shadows").as_deref(), Some("medium"));
        assert_eq!(s.get("render_scale").as_deref(), Some("0.9"));
        assert_eq!(
            s.get("bloom").as_deref(),
            Some("off"),
            "everything else is low"
        );
        assert_eq!(s.tier_matching(), None);
        // A line before the preset is replaced by it.
        let mut t = QualitySettings::default();
        assert!(t.apply_text("bloom = off\npreset = ultra").is_ok());
        assert_eq!(t, QualitySettings::preset(QualityTier::Ultra));
    }

    #[test]
    fn comments_blank_lines_crlf_and_spacing_are_all_fine() {
        let mut s = QualitySettings::default();
        let text = "# graphics\r\n\r\n   shadows   =   off   # no shadows\r\nanisotropy 2\r\n\t\r\n#bloom = off\r\n";
        let report = s.apply_text(text);
        assert!(report.is_ok(), "{report:?}");
        assert_eq!(
            (s.shadows, s.anisotropy, s.bloom),
            (ShadowQuality::Off, 2, true)
        );
        assert_eq!(report.changed, ["shadows", "anisotropy"]);
    }

    #[test]
    fn the_report_lists_only_what_actually_changed_in_option_order() {
        let mut s = QualitySettings::default(); // high
        let report =
            s.apply_text("max_decals = 400\nbloom = off\nshadows = medium\nanti_aliasing = msaa4");
        // High already has 400 decals and medium shadows, so only the anti-aliasing and the bloom change.
        assert_eq!(report.changed, ["anti_aliasing", "bloom"], "{report:?}");
    }

    #[test]
    fn one_bad_line_applies_nothing_and_every_bad_line_is_reported_with_its_number() {
        let mut s = QualitySettings::default();
        let before = s.clone();
        let report = s.apply_text("shadows = high\nbloom = maybe\n\n# fine\nnonsense = 1\npreset = extreme\nrender_scale = 0.75\n");
        assert!(!report.is_ok());
        assert_eq!(s, before, "nothing was applied");
        assert!(report.changed.is_empty());
        let lines: Vec<usize> = report.errors.iter().map(|(n, _)| *n).collect();
        assert_eq!(lines, [2, 5, 6]);
        assert!(
            report.errors[0].1.starts_with("bloom"),
            "{:?}",
            report.errors[0]
        );
        assert!(report.errors[1].1.contains("nonsense"));
        assert!(report.errors[2].1.contains("extreme"));
    }

    #[test]
    fn empty_text_and_a_name_without_a_value_are_handled() {
        let mut s = QualitySettings::default();
        let before = s.clone();
        assert!(
            s.apply_text("").is_ok() && s.apply_text("  \n# only a comment\n").changed.is_empty()
        );
        assert_eq!(s, before);
        let report = s.apply_text("shadows");
        assert_eq!(
            report.errors.len(),
            1,
            "a name with no value is an error: {report:?}"
        );
        assert_eq!(s, before);
        assert!(s.apply_text("preset").errors[0].1.contains("preset"));
        // The last of two lines for one setting wins.
        assert!(s.apply_text("anisotropy = 2\nanisotropy = 4").is_ok());
        assert_eq!(s.anisotropy, 4);
    }

    #[test]
    fn applying_the_same_text_twice_changes_nothing_the_second_time() {
        let mut s = QualitySettings::default();
        let text = "preset = medium\nlod_bias = 1.5\n";
        assert!(!s.apply_text(text).changed.is_empty());
        let again = s.apply_text(text);
        assert!(again.is_ok() && again.changed.is_empty());
    }

    // ---- dynamic resolution ----------------------------------------------------------------------

    fn run(dr: &mut DynamicResolution, frames: usize, cost: impl Fn(f32) -> f32) -> Vec<f32> {
        let mut scale = dr.scale();
        (0..frames)
            .map(|_| {
                scale = dr.frame(cost(scale));
                scale
            })
            .collect()
    }

    #[test]
    fn fast_frames_leave_the_scale_alone() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        let scales = run(&mut dr, 500, |_| 10.0);
        assert!(scales.iter().all(|s| *s == 1.0));
    }

    #[test]
    fn slow_frames_lower_the_scale_until_the_frame_fits_and_it_then_stays_put() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        // A scene that costs 28 ms at full scale and scales with the pixel count (scale squared): fits 16.7 ms at about 0.77.
        let scales = run(&mut dr, 600, |s| 28.0 * s * s);
        let last = *scales.last().unwrap();
        assert!(
            28.0 * last * last <= 1000.0 / 60.0 * 1.05,
            "the frame fits: {last}"
        );
        assert!(
            last >= 0.65,
            "and it did not shrink far more than it needed to: {last}"
        );
        let tail = &scales[scales.len() - 200..];
        let changes = tail.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(
            changes <= 2,
            "it settles instead of hunting: {changes} changes in the last 200 frames"
        );
    }

    #[test]
    fn it_never_goes_below_the_minimum_or_above_the_ceiling() {
        let cfg = DynamicResolutionConfig {
            min_scale: 0.6,
            ..DynamicResolutionConfig::default()
        };
        let mut dr = DynamicResolution::new(cfg, 0.9).unwrap();
        assert_eq!(dr.scale(), 0.9);
        let scales = run(&mut dr, 500, |_| 100.0); // hopelessly slow
        assert!(
            scales.iter().all(|s| (0.6..=0.9).contains(s)),
            "{:?}",
            scales.iter().cloned().fold(f32::MAX, f32::min)
        );
        assert_eq!(*scales.last().unwrap(), 0.6);
        let up = run(&mut dr, 2_000, |_| 2.0); // now very fast
        assert!(up.iter().all(|s| (0.6..=0.9).contains(s)));
        assert_eq!(
            *up.last().unwrap(),
            0.9,
            "back up to the ceiling and no further"
        );
    }

    #[test]
    fn going_up_is_slower_than_going_down() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        let down = run(&mut dr, 80, |_| 30.0);
        let first_down = down.iter().position(|s| *s < 1.0).unwrap();
        assert!(
            first_down < 16,
            "the first step down comes within two windows: frame {first_down}"
        );
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        run(&mut dr, 80, |_| 30.0);
        let low = dr.scale();
        let up = run(&mut dr, 200, |_| 5.0);
        let first_up = up.iter().position(|s| *s > low).unwrap();
        assert!(
            first_up >= 8 * 4 - 1,
            "a step up needs {} calm windows: frame {first_up}",
            4
        );
    }

    #[test]
    fn a_big_overshoot_steps_down_twice_as_far() {
        let step = DynamicResolutionConfig::default().step;
        let mut mild = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        run(&mut mild, 8, |_| 20.0); // 20 / 16.7 = 1.2 over
        let mut heavy = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        run(&mut heavy, 8, |_| 40.0); // 2.4 over
        assert!((1.0 - mild.scale() - step).abs() < 1e-4, "{}", mild.scale());
        assert!(
            (1.0 - heavy.scale() - 2.0 * step).abs() < 1e-4,
            "{}",
            heavy.scale()
        );
    }

    #[test]
    fn the_middle_band_changes_nothing_and_breaks_a_run_of_calm_windows() {
        let cfg = DynamicResolutionConfig::default();
        let mut dr = DynamicResolution::new(cfg, 1.0).unwrap();
        run(&mut dr, 8, |_| 30.0);
        let low = dr.scale();
        // Three calm windows, one in the middle band, then three calm windows: never four in a row, so no step up.
        for _ in 0..3 {
            run(&mut dr, 8, |_| 5.0);
        }
        run(&mut dr, 8, |_| 15.0); // between 80 % and 100 % of 16.7 ms
        for _ in 0..3 {
            run(&mut dr, 8, |_| 5.0);
        }
        assert_eq!(dr.scale(), low);
        run(&mut dr, 8, |_| 5.0);
        assert!(dr.scale() > low, "the fourth calm window in a row steps up");
    }

    #[test]
    fn frame_times_that_are_not_numbers_or_not_positive_are_ignored() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -5.0] {
            for _ in 0..50 {
                assert_eq!(dr.frame(bad), 1.0);
            }
        }
        // They do not even count toward a window: the next 8 good frames decide alone.
        run(&mut dr, 8, |_| 40.0);
        assert!(dr.scale() < 1.0);
    }

    #[test]
    fn a_frame_exactly_at_a_threshold_is_not_a_decision_to_move() {
        // Exact in binary floating point: target 10 ms, thresholds 1.0 and 0.5.
        let cfg = DynamicResolutionConfig {
            target_ms: 10.0,
            window: 1,
            up_after: 1,
            up_threshold: 0.5,
            down_threshold: 1.0,
            ..DynamicResolutionConfig::default()
        };
        let mut dr = DynamicResolution::new(cfg, 1.0).unwrap();
        assert_eq!(
            dr.frame(10.0),
            1.0,
            "exactly on the down threshold is not too slow"
        );
        assert!(dr.frame(10.01) < 1.0, "just over it is");
        let low = dr.scale();
        assert_eq!(
            dr.frame(5.0),
            low,
            "exactly on the up threshold is not room to grow"
        );
        assert!(dr.frame(4.99) > low, "just under it is");
    }

    #[test]
    fn a_zero_frame_time_does_not_count_toward_a_window() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        for _ in 0..7 {
            dr.frame(40.0);
        }
        dr.frame(0.0); // ignored: the window still has 7 of 8 frames
        assert_eq!(dr.scale(), 1.0);
        dr.frame(40.0);
        assert!(dr.scale() < 1.0);
    }

    #[test]
    fn validate_accepts_the_largest_decal_count_and_refuses_one_more() {
        let at_limit = QualitySettings {
            max_decals: 1000,
            ..QualitySettings::default()
        };
        assert!(at_limit.validate().is_ok());
        let over = QualitySettings {
            max_decals: 1001,
            ..QualitySettings::default()
        };
        assert!(over.validate().is_err());
    }

    #[test]
    fn the_ceiling_can_move_and_reset_starts_again() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 1.0).unwrap();
        dr.set_max_scale(0.8);
        assert_eq!(dr.scale(), 0.8, "the scale follows a lower ceiling at once");
        dr.set_max_scale(2.0); // out of range: ignored
        dr.set_max_scale(f32::NAN);
        run(&mut dr, 8, |_| 40.0);
        assert!(dr.scale() < 0.8);
        dr.reset();
        assert_eq!(dr.scale(), 0.8);
        // The ceiling cannot go below the configured minimum.
        dr.set_max_scale(0.5);
        assert_eq!(dr.scale(), 0.5);
    }

    #[test]
    fn scales_stay_on_the_step_grid() {
        let mut dr = DynamicResolution::new(DynamicResolutionConfig::default(), 0.93).unwrap();
        for s in run(&mut dr, 400, |s| 30.0 * s) {
            let steps = s / 0.05;
            assert!(
                (steps - steps.round()).abs() < 1e-3 || (s - 0.93).abs() < 1e-6,
                "{s} is off the grid"
            );
        }
    }

    #[test]
    fn bad_configurations_are_refused() {
        let ok = DynamicResolutionConfig::default();
        assert!(ok.validate().is_ok());
        for bad in [
            DynamicResolutionConfig {
                target_ms: 0.0,
                ..ok
            },
            DynamicResolutionConfig {
                target_ms: f32::NAN,
                ..ok
            },
            DynamicResolutionConfig {
                min_scale: 0.4,
                ..ok
            },
            DynamicResolutionConfig {
                min_scale: 1.1,
                ..ok
            },
            DynamicResolutionConfig { step: 0.0, ..ok },
            DynamicResolutionConfig { step: 0.3, ..ok },
            DynamicResolutionConfig { window: 0, ..ok },
            DynamicResolutionConfig { up_after: 0, ..ok },
            DynamicResolutionConfig {
                up_threshold: 1.0,
                down_threshold: 1.0,
                ..ok
            },
            DynamicResolutionConfig {
                up_threshold: 0.0,
                ..ok
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
            assert!(DynamicResolution::new(bad, 1.0).is_err());
        }
        assert!(DynamicResolution::new(ok, 0.4).is_err());
        assert!(DynamicResolution::new(ok, 1.2).is_err());
        assert!(
            DynamicResolution::new(
                DynamicResolutionConfig {
                    min_scale: 0.8,
                    ..ok
                },
                0.7
            )
            .is_err(),
            "a start below the minimum"
        );
    }
}
