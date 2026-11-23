//! The **settings screen's contents** (roadmap T10-012, `docs/SETTINGS_SCREEN.md`): which rows there are, how the game's real settings fill them, and how the values the
//! player applied turn back into those settings.
//!
//! The generic model (tabs, pending edits, apply, revert, the confirmation countdown) is [`ss2_ui::settings_model`]; it knows nothing about graphics or sound. This module
//! is the **adapter**: it describes the real option sets as rows and reads them back. It lives here, not in `ss2-ui`, because `ss2-render-api` (which owns
//! `QualitySettings`) depends on `ss2-ui`, so `ss2-ui` cannot name it; `ss2-game` already depends on the render API and sits at the top of the dependency graph with the audio,
//! platform and UI contracts below it. (`ss2-app` is a binary crate: it has no library target to test, and an unused public API in a binary is dead code.)
//!
//! Four tabs: **Video** (window mode, resolution, vsync, field of view, UI aspect and scale), **Graphics** (every `QualitySettings` option), **Audio** (one volume per
//! `ss2_audio_api::Bus`) and **Accessibility** (every `AccessibilitySettings` option). Row names are the option names the console and files already use.
//!
//! ```
//! use ss2_game::settings::{GameSettings, applied_settings, build_screen};
//! use ss2_ui::settings_model::Step;
//!
//! let mut screen = build_screen(&GameSettings::default(), &[(1280, 720), (1920, 1080)], 15.0).unwrap();
//! screen.step("shadows", Step::Decrease).unwrap(); // pending only
//! assert_eq!(applied_settings(&screen).unwrap(), GameSettings::default());
//! screen.apply(0.0).unwrap();
//! assert_ne!(applied_settings(&screen).unwrap(), GameSettings::default());
//! ```

use ss2_audio_api::{Audio, Bus};
use ss2_core::settings::Settings;
use ss2_platform_api::{DisplayRequest, WindowMode};
use ss2_render_api::quality::{OptionDesc, OptionKind, QualitySettings, QualityTier};
use ss2_ui::accessibility::{AccessibilitySettings, OPTION_NAMES};
use ss2_ui::scale::{AspectMode, ScaleSettings};
use ss2_ui::settings_model::{
    Change, RowKind, RowSpec, SettingsError, SettingsScreen, TabSpec, ValueFormat,
};

/// The smallest and largest field of view the screen offers, in degrees (the range of the `r_fov` console variable).
pub const FOV_RANGE: (f32, f32) = (30.0, 120.0);
/// The field of view of a fresh install (the default of `r_fov`).
pub const DEFAULT_FOV: f32 = 80.0;

/// The part of the game a setting belongs to; what to re-apply when one of its rows changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    /// Window, resolution, vsync, field of view, UI scale: the window layer and the camera.
    Video,
    /// `QualitySettings`: the renderer.
    Graphics,
    /// Bus volumes: the mixer.
    Audio,
    /// `AccessibilitySettings`: the HUD, subtitles, effects and input.
    Accessibility,
}

impl Section {
    /// The tab title.
    pub fn title(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Graphics => "Graphics",
            Self::Audio => "Audio",
            Self::Accessibility => "Accessibility",
        }
    }
}

/// Window, resolution and camera settings.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoSettings {
    /// Windowed, borderless or exclusive fullscreen.
    pub window_mode: WindowMode,
    /// The window or fullscreen resolution in pixels.
    pub resolution: (u32, u32),
    /// Wait for the display refresh.
    pub vsync: bool,
    /// The field of view in degrees, within [`FOV_RANGE`] (the meaning of the `r_fov` console variable).
    pub fov: f32,
    /// How the UI meets the window (aspect mode and the player's smaller-UI scale).
    pub ui: ScaleSettings,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            window_mode: WindowMode::Windowed,
            resolution: (1920, 1080),
            vsync: true,
            fov: DEFAULT_FOV,
            ui: ScaleSettings::default(),
        }
    }
}

impl VideoSettings {
    /// What to ask the window layer for (`ss2_platform_api::plan_display` turns it into a plan): the mode and the resolution, on the primary monitor.
    pub fn display_request(&self) -> DisplayRequest {
        DisplayRequest {
            mode: self.window_mode,
            monitor: None,
            resolution: Some(self.resolution),
            refresh_hz: None,
        }
    }
}

/// One volume per mixer bus, each 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioSettings {
    /// Scales everything.
    pub master: f32,
    /// Music.
    pub music: f32,
    /// Sound effects.
    pub sfx: f32,
    /// Voice-overs and voice chat.
    pub voice: f32,
    /// Menu and HUD sounds.
    pub ui: f32,
}

impl Default for AudioSettings {
    /// The master, music and effects defaults of `ss2_core::settings::Settings` (0.8, 0.6, 0.8); voice and UI at full.
    fn default() -> Self {
        let core = Settings::default();
        Self {
            master: core.master_volume,
            music: core.music_volume,
            sfx: core.sfx_volume,
            voice: 1.0,
            ui: 1.0,
        }
    }
}

impl AudioSettings {
    /// The volume of a bus, or `None` for a bus this build does not know (`Bus` is `#[non_exhaustive]`).
    pub fn volume(&self, bus: Bus) -> Option<f32> {
        match bus {
            Bus::Master => Some(self.master),
            Bus::Music => Some(self.music),
            Bus::Sfx => Some(self.sfx),
            Bus::Voice => Some(self.voice),
            Bus::Ui => Some(self.ui),
            _ => None,
        }
    }

    /// Set every bus volume on the mixer.
    pub fn apply(&self, audio: &mut dyn Audio) {
        for bus in Bus::ALL {
            if let Some(v) = self.volume(bus) {
                audio.set_bus_volume(bus, v);
            }
        }
    }

    /// The volumes the mixer has now.
    pub fn read(audio: &dyn Audio) -> Self {
        Self {
            master: audio.bus_volume(Bus::Master),
            music: audio.bus_volume(Bus::Music),
            sfx: audio.bus_volume(Bus::Sfx),
            voice: audio.bus_volume(Bus::Voice),
            ui: audio.bus_volume(Bus::Ui),
        }
    }
}

/// Everything the settings screen edits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameSettings {
    /// Window, resolution, camera.
    pub video: VideoSettings,
    /// Graphics quality.
    pub quality: QualitySettings,
    /// Bus volumes.
    pub audio: AudioSettings,
    /// Accessibility options.
    pub accessibility: AccessibilitySettings,
}

impl GameSettings {
    /// The settings of the existing settings file (`vsync`, `fullscreen` as borderless, the master, music and effects volumes) on top of the defaults.
    pub fn from_core(core: &Settings) -> Self {
        let mut s = Self::default();
        s.video.vsync = core.vsync;
        s.video.window_mode = if core.fullscreen {
            WindowMode::Borderless
        } else {
            WindowMode::Windowed
        };
        s.audio.master = core.master_volume;
        s.audio.music = core.music_volume;
        s.audio.sfx = core.sfx_volume;
        s
    }

    /// Write the five values the settings file has into `core` (fullscreen is true for borderless and exclusive); everything else in `core` is left alone.
    pub fn write_core(&self, core: &mut Settings) {
        core.vsync = self.video.vsync;
        core.fullscreen = self.video.window_mode != WindowMode::Windowed;
        core.master_volume = self.audio.master;
        core.music_volume = self.audio.music;
        core.sfx_volume = self.audio.sfx;
    }
}

// ------------------------------------------------------------------------ names

const WINDOW_MODES: [(&str, WindowMode); 3] = [
    ("windowed", WindowMode::Windowed),
    ("borderless", WindowMode::Borderless),
    ("exclusive", WindowMode::Exclusive),
];
const ASPECTS: [(&str, AspectMode); 2] = [
    ("letterbox", AspectMode::Letterbox),
    ("expand", AspectMode::Expand),
];

/// The UI scale the screen offers (the smaller-UI range of `ss2_ui::scale`).
const UI_SCALE_RANGE: (f32, f32) = (0.5, 1.0);

/// The names of the rows of the Video tab, in screen order.
pub const VIDEO_ROWS: [&str; 6] = [
    "window_mode",
    "resolution",
    "vsync",
    "fov",
    "aspect",
    "ui_scale",
];

/// The names of the rows of the Audio tab, in screen order (the bus order of `Bus::ALL`).
pub const AUDIO_ROWS: [&str; 5] = [
    "master_volume",
    "music_volume",
    "sfx_volume",
    "voice_volume",
    "ui_volume",
];

fn on_off(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

fn name_of<T: Copy + PartialEq>(table: &[(&'static str, T)], v: T) -> &'static str {
    table.iter().find(|(_, x)| *x == v).map_or("?", |(n, _)| n)
}

fn size_text(size: (u32, u32)) -> String {
    format!("{}x{}", size.0, size.1)
}

fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Which part of the game a row belongs to, or `None` for a name the screen does not have.
pub fn section_of(id: &str) -> Option<Section> {
    if VIDEO_ROWS.contains(&id) {
        Some(Section::Video)
    } else if AUDIO_ROWS.contains(&id) {
        Some(Section::Audio)
    } else if OPTION_NAMES.contains(&id) {
        Some(Section::Accessibility)
    } else if QualitySettings::options().iter().any(|o| o.name == id) {
        Some(Section::Graphics)
    } else {
        None
    }
}

/// The parts of the game a list of changes touches, in a fixed order, each once: what to re-apply.
pub fn affected_sections(changes: &[Change]) -> Vec<Section> {
    let mut out: Vec<Section> = changes.iter().filter_map(|c| section_of(&c.id)).collect();
    out.sort();
    out.dedup();
    out
}

// ------------------------------------------------------------------------ rows

fn humanize_id(id: &str) -> String {
    let mut s = id.replace('_', " ");
    if let Some(first) = s.get(..1) {
        let up = first.to_uppercase();
        s.replace_range(..1, &up);
    }
    s
}

fn quality_label(id: &str) -> String {
    match id {
        "shadows" => "Shadows".to_owned(),
        "texture_detail" => "Texture detail".to_owned(),
        "anisotropy" => "Anisotropic filtering".to_owned(),
        "anti_aliasing" => "Anti-aliasing".to_owned(),
        "lod_bias" => "Model detail bias".to_owned(),
        "view_distance" => "View distance".to_owned(),
        "particle_density" => "Particle density".to_owned(),
        "bloom" => "Bloom".to_owned(),
        "render_scale" => "Render scale".to_owned(),
        "dynamic_resolution" => "Dynamic resolution".to_owned(),
        "max_decals" => "Maximum decals".to_owned(),
        other => humanize_id(other),
    }
}

fn quality_row(desc: &OptionDesc) -> RowSpec {
    let id = desc.name;
    let defaults = QualitySettings::default();
    let (kind, format) = match &desc.kind {
        OptionKind::Choice(words) => (
            RowKind::Choice(words.iter().map(|w| (*w).to_owned()).collect()),
            if id == "anisotropy" {
                ValueFormat::Suffix("x")
            } else {
                ValueFormat::Plain
            },
        ),
        OptionKind::Range { min, max } => {
            let step = match id {
                "lod_bias" => 0.25,
                "view_distance" => 0.1,
                "particle_density" | "render_scale" => 0.05,
                _ => (max - min) / 20.0,
            };
            let format = match id {
                "view_distance" | "particle_density" | "render_scale" => ValueFormat::Percent,
                _ => ValueFormat::Plain,
            };
            (
                RowKind::Range {
                    min: *min,
                    max: *max,
                    step,
                },
                format,
            )
        }
        OptionKind::Count { min, max } => (
            RowKind::Count {
                min: *min,
                max: *max,
                step: if id == "max_decals" {
                    50
                } else {
                    ((max - min) / 20).max(1)
                },
            },
            ValueFormat::Plain,
        ),
        OptionKind::Toggle => (RowKind::Toggle, ValueFormat::Plain),
    };
    // An option this build does not know has no default of ours: use the lowest value.
    let default = defaults.get(id).unwrap_or_else(|| match &desc.kind {
        OptionKind::Choice(words) => words.first().map_or_else(String::new, |w| (*w).to_owned()),
        OptionKind::Range { min, .. } => min.to_string(),
        OptionKind::Count { min, .. } => min.to_string(),
        OptionKind::Toggle => "off".to_owned(),
    });
    RowSpec::new(id, &quality_label(id), kind, &default).format(format)
}

fn accessibility_row(id: &str) -> Option<RowSpec> {
    let words = |w: &[&str]| RowKind::Choice(w.iter().map(|s| (*s).to_owned()).collect());
    let hold = || words(&["hold", "toggle"]);
    let (label, kind, format) = match id {
        "text_scale" => (
            "Text size",
            RowKind::Range {
                min: 0.8,
                max: 2.0,
                step: 0.1,
            },
            ValueFormat::Percent,
        ),
        "colorblind" => (
            "Colour-blind mode",
            words(&["off", "protanopia", "deuteranopia", "tritanopia"]),
            ValueFormat::Plain,
        ),
        "subtitle_size" => (
            "Subtitle size",
            words(&["small", "medium", "large", "extra_large"]),
            ValueFormat::Plain,
        ),
        "subtitle_background" => (
            "Subtitle background",
            words(&["none", "translucent", "opaque"]),
            ValueFormat::Plain,
        ),
        "reduce_flashing" => ("Reduce flashing", RowKind::Toggle, ValueFormat::Plain),
        "reduce_motion" => ("Reduce motion", RowKind::Toggle, ValueFormat::Plain),
        "high_contrast" => ("High contrast", RowKind::Toggle, ValueFormat::Plain),
        "crouch" => ("Crouch", hold(), ValueFormat::Plain),
        "sprint" => ("Sprint", hold(), ValueFormat::Plain),
        "aim" => ("Aim down sights", hold(), ValueFormat::Plain),
        _ => return None,
    };
    let default = AccessibilitySettings::default().get(id)?;
    Some(RowSpec::new(id, label, kind, &default).format(format))
}

/// The resolutions to offer: the monitor's list (see `ss2_platform_api::resolution_list`) and `current`, each once, smallest first.
fn resolution_words(resolutions: &[(u32, u32)], current: (u32, u32)) -> Vec<(u32, u32)> {
    let mut sizes: Vec<(u32, u32)> = resolutions.to_vec();
    sizes.push(current);
    sizes.sort_by_key(|&(w, h)| (u64::from(w) * u64::from(h), w));
    sizes.dedup();
    sizes
}

fn video_rows(resolutions: &[(u32, u32)], current: (u32, u32)) -> Vec<RowSpec> {
    let sizes = resolution_words(resolutions, current);
    let defaults = VideoSettings::default();
    // The default resolution is the one of a fresh install if the monitor offers it, else the largest it offers.
    let default_size = if sizes.contains(&defaults.resolution) {
        defaults.resolution
    } else {
        sizes.last().copied().unwrap_or(defaults.resolution)
    };
    vec![
        RowSpec::new(
            "window_mode",
            "Window mode",
            RowKind::Choice(WINDOW_MODES.iter().map(|(n, _)| (*n).to_owned()).collect()),
            name_of(&WINDOW_MODES, defaults.window_mode),
        )
        .risky(),
        RowSpec::new(
            "resolution",
            "Resolution",
            RowKind::Choice(sizes.iter().map(|s| size_text(*s)).collect()),
            &size_text(default_size),
        )
        .risky(),
        RowSpec::new(
            "vsync",
            "Vertical sync",
            RowKind::Toggle,
            on_off(defaults.vsync),
        ),
        RowSpec::new(
            "fov",
            "Field of view",
            RowKind::Range {
                min: FOV_RANGE.0,
                max: FOV_RANGE.1,
                step: 5.0,
            },
            &defaults.fov.to_string(),
        )
        .format(ValueFormat::Suffix("°")),
        RowSpec::new(
            "aspect",
            "UI on wide screens",
            RowKind::Choice(ASPECTS.iter().map(|(n, _)| (*n).to_owned()).collect()),
            name_of(&ASPECTS, defaults.ui.aspect),
        ),
        RowSpec::new(
            "ui_scale",
            "UI size",
            RowKind::Range {
                min: UI_SCALE_RANGE.0,
                max: UI_SCALE_RANGE.1,
                step: 0.05,
            },
            &defaults.ui.user_scale.to_string(),
        )
        .format(ValueFormat::Percent),
    ]
}

fn audio_rows() -> Vec<RowSpec> {
    let defaults = AudioSettings::default();
    let labels = [
        "Master volume",
        "Music volume",
        "Effects volume",
        "Voice volume",
        "Interface volume",
    ];
    AUDIO_ROWS
        .iter()
        .zip(labels)
        .zip(Bus::ALL)
        .map(|((id, label), bus)| {
            RowSpec::new(
                id,
                label,
                RowKind::Range {
                    min: 0.0,
                    max: 1.0,
                    step: 0.1,
                },
                &defaults.volume(bus).unwrap_or(1.0).to_string(),
            )
            .format(ValueFormat::Percent)
        })
        .collect()
}

/// The current values as text, in the spelling the rows use.
fn values_of(settings: &GameSettings) -> Vec<(String, String)> {
    let v = &settings.video;
    let mut out = vec![
        (
            "window_mode".to_owned(),
            name_of(&WINDOW_MODES, v.window_mode).to_owned(),
        ),
        ("resolution".to_owned(), size_text(v.resolution)),
        ("vsync".to_owned(), on_off(v.vsync).to_owned()),
        ("fov".to_owned(), v.fov.to_string()),
        (
            "aspect".to_owned(),
            name_of(&ASPECTS, v.ui.aspect).to_owned(),
        ),
        ("ui_scale".to_owned(), v.ui.user_scale.to_string()),
    ];
    for desc in QualitySettings::options() {
        out.push((
            desc.name.to_owned(),
            settings.quality.get(desc.name).unwrap_or_default(),
        ));
    }
    for (id, bus) in AUDIO_ROWS.iter().zip(Bus::ALL) {
        out.push((
            (*id).to_owned(),
            settings.audio.volume(bus).unwrap_or(1.0).to_string(),
        ));
    }
    for id in OPTION_NAMES {
        out.push((
            id.to_owned(),
            settings.accessibility.get(id).unwrap_or_default(),
        ));
    }
    out
}

/// Build the settings screen showing `current`.
///
/// `resolutions` is the list the window layer offers (`ss2_platform_api::resolution_list` of the monitor); the current resolution is added if it is not in it.
/// `confirm_secs` is how long a change of window mode or resolution is shown before it is put back unless the player keeps it.
///
/// # Errors
/// [`SettingsError`] if a value of `current` is outside what its row takes (for example a quality field assigned out of range), or `confirm_secs` is not above 0.
pub fn build_screen(
    current: &GameSettings,
    resolutions: &[(u32, u32)],
    confirm_secs: f32,
) -> Result<SettingsScreen, SettingsError> {
    let tabs = vec![
        TabSpec::new(
            Section::Video.title(),
            video_rows(resolutions, current.video.resolution),
        ),
        TabSpec::new(
            Section::Graphics.title(),
            QualitySettings::options().iter().map(quality_row).collect(),
        ),
        TabSpec::new(Section::Audio.title(), audio_rows()),
        TabSpec::new(
            Section::Accessibility.title(),
            OPTION_NAMES
                .iter()
                .filter_map(|id| accessibility_row(id))
                .collect(),
        ),
    ];
    let mut screen = SettingsScreen::new(tabs, confirm_secs)?;
    for (id, text) in values_of(current) {
        screen.load(&id, &text)?;
    }
    Ok(screen)
}

// ------------------------------------------------------------------------ reading back

fn read_back(
    screen: &SettingsScreen,
    value: impl Fn(&SettingsScreen, &str) -> Option<String>,
) -> Result<GameSettings, String> {
    let get = |id: &str| value(screen, id).ok_or_else(|| format!("the screen has no row {id}"));
    let number = |id: &str| -> Result<f32, String> {
        get(id)?
            .parse::<f32>()
            .map_err(|_| format!("{id} is not a number"))
    };
    let mut s = GameSettings::default();
    let mode = get("window_mode")?;
    s.video.window_mode = WINDOW_MODES
        .iter()
        .find(|(n, _)| *n == mode)
        .map(|(_, m)| *m)
        .ok_or_else(|| format!("window_mode {mode:?} is not a mode"))?;
    let size = get("resolution")?;
    s.video.resolution =
        parse_size(&size).ok_or_else(|| format!("resolution {size:?} is not WIDTHxHEIGHT"))?;
    s.video.vsync = get("vsync")? == "on";
    s.video.fov = number("fov")?;
    let aspect = get("aspect")?;
    s.video.ui.aspect = ASPECTS
        .iter()
        .find(|(n, _)| *n == aspect)
        .map(|(_, a)| *a)
        .ok_or_else(|| format!("aspect {aspect:?} is not an aspect mode"))?;
    s.video.ui.user_scale = number("ui_scale")?;
    for desc in QualitySettings::options() {
        s.quality
            .set(desc.name, &get(desc.name)?)
            .map_err(|e| e.to_string())?;
    }
    s.audio.master = number(AUDIO_ROWS[0])?;
    s.audio.music = number(AUDIO_ROWS[1])?;
    s.audio.sfx = number(AUDIO_ROWS[2])?;
    s.audio.voice = number(AUDIO_ROWS[3])?;
    s.audio.ui = number(AUDIO_ROWS[4])?;
    for id in OPTION_NAMES {
        s.accessibility
            .set(id, &get(id)?)
            .map_err(|e| e.to_string())?;
    }
    Ok(s)
}

/// The settings the player has **applied** (what the game is using).
///
/// # Errors
/// A message if a row's text cannot be read back; cannot happen for a screen from [`build_screen`] (tested over every row).
pub fn applied_settings(screen: &SettingsScreen) -> Result<GameSettings, String> {
    read_back(screen, |s, id| s.applied(id).map(str::to_owned))
}

/// The settings with the player's **pending** edits: for a live preview (the UI scale, the text size) before Apply.
///
/// # Errors
/// As [`applied_settings`].
pub fn pending_settings(screen: &SettingsScreen) -> Result<GameSettings, String> {
    read_back(screen, |s, id| s.pending(id).map(str::to_owned))
}

// ------------------------------------------------------------------------ presets

/// Set the pending value of every graphics row to the preset (it still needs Apply).
///
/// # Errors
/// [`SettingsError::Locked`] while a confirmation is open (nothing changes).
pub fn set_quality_preset(
    screen: &mut SettingsScreen,
    tier: QualityTier,
) -> Result<(), SettingsError> {
    let preset = QualitySettings::preset(tier);
    for desc in QualitySettings::options() {
        let text = preset.get(desc.name).unwrap_or_default();
        screen.set_text(desc.name, &text)?;
    }
    Ok(())
}

/// The name of the preset the **pending** graphics values equal, or `Custom` (what a settings screen shows beside the graphics rows).
pub fn quality_label_of(screen: &SettingsScreen) -> String {
    pending_settings(screen)
        .ok()
        .and_then(|s| s.quality.tier_matching())
        .map_or_else(|| "Custom".to_owned(), |t| t.name().to_owned())
}

#[cfg(test)]
mod tests;
