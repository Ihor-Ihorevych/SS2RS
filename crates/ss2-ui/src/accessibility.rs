//! **Accessibility** options and the maths behind them (roadmap T10-010, `docs/ACCESSIBILITY.md`): text scale, colour-blind-safe colours, subtitle size and
//! background, reduced flashing and motion, and hold-or-toggle input.
//!
//! No drawing and no input device here: [`AccessibilitySettings`] is a plain value with named, strictly parsed options (for a settings screen or a console), and
//! the functions are the decisions the screens ask for: how big is this text, which colours stay distinguishable for this player, may this effect flash now,
//! is this action held or toggled.
//!
//! Colours are **linear RGB** in `0.0..=1.0`, like [`Color`](crate::Color).

use std::fmt;

/// How a player sees colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorblindMode {
    /// Typical colour vision.
    #[default]
    None,
    /// No working long-wavelength (red) cones.
    Protanopia,
    /// No working medium-wavelength (green) cones.
    Deuteranopia,
    /// No working short-wavelength (blue) cones.
    Tritanopia,
}

/// Machado, Oliveira and Fernandes (2009), severity 1.0, for linear RGB. Every row sums to 1 so greys stay grey.
const PROTAN: [[f32; 3]; 3] = [
    [0.152_286, 1.052_583, -0.204_868],
    [0.114_503, 0.786_281, 0.099_216],
    [-0.003_882, -0.048_116, 1.051_998],
];
const DEUTAN: [[f32; 3]; 3] = [
    [0.367_322, 0.860_646, -0.227_968],
    [0.280_085, 0.672_501, 0.047_413],
    [-0.011_820, 0.042_940, 0.968_881],
];
const TRITAN: [[f32; 3]; 3] = [
    [1.255_528, -0.076_749, -0.178_779],
    [-0.078_411, 0.930_809, 0.147_602],
    [0.004_733, 0.691_367, 0.303_900],
];

fn clamp01(x: f32) -> f32 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

/// How a colour looks to a person with this kind of colour blindness (linear RGB in, linear RGB out; channels are limited to 0 to 1 and NaN counts as 0).
pub fn simulate(mode: ColorblindMode, rgb: [f32; 3]) -> [f32; 3] {
    let m = match mode {
        ColorblindMode::None => return rgb.map(clamp01),
        ColorblindMode::Protanopia => &PROTAN,
        ColorblindMode::Deuteranopia => &DEUTAN,
        ColorblindMode::Tritanopia => &TRITAN,
    };
    let c = rgb.map(clamp01);
    [0, 1, 2].map(|i| clamp01(m[i][0] * c[0] + m[i][1] * c[1] + m[i][2] * c[2]))
}

fn to_srgb(l: f32) -> f32 {
    let l = clamp01(l);
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// How different two colours look, for a person with this kind of colour vision: the "redmean" distance of their simulated sRGB values (0 for the same colour,
/// about 765 between black and white). Both colours are simulated first, so two colours that differ only along the axis the person cannot see give a small number.
pub fn perceived_difference(mode: ColorblindMode, a: [f32; 3], b: [f32; 3]) -> f32 {
    let to255 = |c: [f32; 3]| simulate(mode, c).map(|v| to_srgb(v) * 255.0);
    let (a, b) = (to255(a), to255(b));
    let rmean = (a[0] + b[0]) / 2.0;
    let (dr, dg, db) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    ((2.0 + rmean / 256.0) * dr * dr + 4.0 * dg * dg + (2.0 + (255.0 - rmean) / 256.0) * db * db)
        .sqrt()
}

/// The least [`perceived_difference`] the HUD accepts between two colours that must not be confused (health good against bad, one team against another).
pub const MIN_DISTINGUISHABLE: f32 = 200.0;

/// The colours of the HUD's status indicators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusPalette {
    /// Healthy, full, safe.
    pub good: [f32; 3],
    /// Getting low, a warning.
    pub caution: [f32; 3],
    /// Critical, danger.
    pub danger: [f32; 3],
}

fn srgb8(r: u8, g: u8, b: u8) -> [f32; 3] {
    let f = |v: u8| {
        let c = f32::from(v) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [f(r), f(g), f(b)]
}

impl StatusPalette {
    /// The traditional green, yellow and red.
    pub fn traditional() -> Self {
        Self {
            good: srgb8(60, 200, 60),
            caution: srgb8(240, 200, 40),
            danger: srgb8(220, 40, 40),
        }
    }

    /// Blue, yellow and vermilion from the Okabe-Ito palette, chosen to stay apart for every kind of colour vision.
    pub fn safe() -> Self {
        Self {
            good: srgb8(0, 114, 178),
            caution: srgb8(240, 228, 66),
            danger: srgb8(213, 94, 0),
        }
    }

    /// The palette to use for a player: the traditional one for typical colour vision, the safe one otherwise.
    pub fn for_mode(mode: ColorblindMode) -> Self {
        if mode == ColorblindMode::None {
            Self::traditional()
        } else {
            Self::safe()
        }
    }

    /// The smallest [`perceived_difference`] between any two of the three colours.
    pub fn min_difference(&self, mode: ColorblindMode) -> f32 {
        let d = |a, b| perceived_difference(mode, a, b);
        d(self.good, self.caution)
            .min(d(self.good, self.danger))
            .min(d(self.caution, self.danger))
    }
}

/// The relative luminance of a linear RGB colour (the WCAG weights), 0 for black to 1 for white.
pub fn relative_luminance(rgb: [f32; 3]) -> f32 {
    let c = rgb.map(clamp01);
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The WCAG contrast ratio between two colours: 1 (identical) to 21 (black on white).
pub fn contrast_ratio(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The contrast WCAG asks of normal text (AA).
pub const MIN_TEXT_CONTRAST: f32 = 4.5;

/// `fg`, moved toward black or white (whichever has more room) by the least amount that gives a contrast of at least `min_ratio` against `bg`. Unchanged if it
/// already has enough; if even pure black or white cannot reach `min_ratio` (it asks for more than 21), whichever of the two is better.
pub fn ensure_contrast(fg: [f32; 3], bg: [f32; 3], min_ratio: f32) -> [f32; 3] {
    let fg = fg.map(clamp01);
    if contrast_ratio(fg, bg) >= min_ratio {
        return fg;
    }
    let target = if relative_luminance(bg) > 0.18 {
        [0.0; 3]
    } else {
        [1.0; 3]
    };
    if contrast_ratio(target, bg) < min_ratio {
        let other = if target[0] == 0.0 { [1.0; 3] } else { [0.0; 3] };
        return if contrast_ratio(other, bg) > contrast_ratio(target, bg) {
            other
        } else {
            target
        };
    }
    // Binary search on how far to move toward the target.
    let mix = |t: f32| [0, 1, 2].map(|i| fg[i] + (target[i] - fg[i]) * t);
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if contrast_ratio(mix(mid), bg) >= min_ratio {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    mix(hi)
}

/// How subtitles are sized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubtitleSize {
    /// 0.8 times the base size.
    Small,
    /// The base size.
    #[default]
    Medium,
    /// 1.3 times.
    Large,
    /// 1.7 times.
    ExtraLarge,
}

impl SubtitleSize {
    /// The multiplier on the base subtitle size.
    pub fn factor(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Medium => 1.0,
            Self::Large => 1.3,
            Self::ExtraLarge => 1.7,
        }
    }
}

/// What goes behind subtitles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubtitleBackground {
    /// Nothing (text with an outline only).
    None,
    /// A translucent dark box.
    #[default]
    Translucent,
    /// A solid black box.
    Opaque,
}

impl SubtitleBackground {
    /// The alpha of the box.
    pub fn alpha(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Translucent => 0.6,
            Self::Opaque => 1.0,
        }
    }
}

/// Whether an action is held down or switched on and off by presses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoldMode {
    /// Active while the button is down.
    #[default]
    Hold,
    /// Each press switches it.
    Toggle,
}

/// The state of one hold-or-toggle action.
///
/// ```
/// use ss2_ui::accessibility::{HoldMode, HoldToggle};
///
/// let mut crouch = HoldToggle::new(HoldMode::Toggle);
/// assert!(!crouch.update(false));
/// assert!(crouch.update(true));   // pressed: on
/// assert!(crouch.update(false));  // released: stays on
/// assert!(!crouch.update(true));  // pressed again: off
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldToggle {
    mode: HoldMode,
    was_down: bool,
    on: bool,
}

impl HoldToggle {
    /// An inactive action.
    pub fn new(mode: HoldMode) -> Self {
        Self {
            mode,
            was_down: false,
            on: false,
        }
    }

    /// Feed this frame's button state; returns whether the action is active now.
    pub fn update(&mut self, down: bool) -> bool {
        let pressed = down && !self.was_down;
        self.was_down = down;
        match self.mode {
            HoldMode::Hold => self.on = down,
            HoldMode::Toggle => {
                if pressed {
                    self.on = !self.on;
                }
            }
        }
        self.on
    }

    /// Switch the action off (the player stopped sprinting, a level loaded) without waiting for a press.
    pub fn cancel(&mut self) {
        self.on = false;
    }

    /// Change the mode (the player changed the option); the action is switched off.
    pub fn set_mode(&mut self, mode: HoldMode) {
        self.mode = mode;
        self.on = false;
    }

    /// Whether the action is active.
    pub fn is_on(&self) -> bool {
        self.on
    }
}

/// How many flashes a second the safe-flashing rule allows (WCAG 2.3.1: no more than three).
pub const MAX_FLASHES_PER_SECOND: usize = 3;

/// Decides whether a screen flash may fire now, so that a player who asked for **reduced flashing** never sees more than three a second.
///
/// ```
/// use ss2_ui::accessibility::FlashLimiter;
///
/// let mut limiter = FlashLimiter::new();
/// let fired = (0..10).filter(|i| limiter.allow(*i as f32 * 0.1, true)).count();
/// assert_eq!(fired, 3); // ten flashes in a second: three get through
/// assert!(FlashLimiter::new().allow(0.0, false), "without the option everything is allowed");
/// ```
#[derive(Clone, Debug, Default)]
pub struct FlashLimiter {
    recent: Vec<f32>,
}

impl FlashLimiter {
    /// A limiter that has seen nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a flash at time `now` (seconds, never going backwards) may fire. With `limit` off every flash is allowed (and not counted); with it on, a flash is
    /// refused when [`MAX_FLASHES_PER_SECOND`] have already fired in the last second. A refused flash is not counted. A `now` that is not finite is refused.
    pub fn allow(&mut self, now: f32, limit: bool) -> bool {
        if !limit {
            return true;
        }
        if !now.is_finite() {
            return false;
        }
        self.recent.retain(|t| now - *t < 1.0);
        if self.recent.len() >= MAX_FLASHES_PER_SECOND {
            return false;
        }
        self.recent.push(now);
        true
    }
}

/// Why a setting could not be set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingError(pub String);

impl fmt::Display for SettingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SettingError {}

/// The smallest and largest text scale.
pub const TEXT_SCALE_RANGE: (f32, f32) = (0.8, 2.0);

/// What the player chose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibilitySettings {
    /// Multiplies the size of all text, 0.8 to 2.0.
    pub text_scale: f32,
    /// How the player sees colour; picks the HUD palette.
    pub colorblind: ColorblindMode,
    /// How big subtitles are.
    pub subtitle_size: SubtitleSize,
    /// What is behind subtitles.
    pub subtitle_background: SubtitleBackground,
    /// Weaken flashes and allow at most three a second.
    pub reduce_flashing: bool,
    /// Turn off screen shake, the underwater warp and camera bob.
    pub reduce_motion: bool,
    /// Use the highest-contrast colours for text and the HUD.
    pub high_contrast: bool,
    /// Crouch: held or toggled.
    pub crouch: HoldMode,
    /// Sprint: held or toggled.
    pub sprint: HoldMode,
    /// Aim down sights: held or toggled.
    pub aim: HoldMode,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            text_scale: 1.0,
            colorblind: ColorblindMode::None,
            subtitle_size: SubtitleSize::Medium,
            subtitle_background: SubtitleBackground::Translucent,
            reduce_flashing: false,
            reduce_motion: false,
            high_contrast: false,
            crouch: HoldMode::Hold,
            sprint: HoldMode::Hold,
            aim: HoldMode::Hold,
        }
    }
}

/// The names of the settings, in the order a screen lists them.
pub const OPTION_NAMES: [&str; 10] = [
    "text_scale",
    "colorblind",
    "subtitle_size",
    "subtitle_background",
    "reduce_flashing",
    "reduce_motion",
    "high_contrast",
    "crouch",
    "sprint",
    "aim",
];

fn pick<T: Copy>(table: &[(&str, T)], text: &str) -> Option<T> {
    let t = text.trim();
    table
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(t))
        .map(|(_, v)| *v)
}

fn parse_bool(text: &str) -> Option<bool> {
    pick(
        &[
            ("on", true),
            ("true", true),
            ("yes", true),
            ("1", true),
            ("off", false),
            ("false", false),
            ("no", false),
            ("0", false),
        ],
        text,
    )
}

const COLORBLIND_NAMES: [(&str, ColorblindMode); 4] = [
    ("off", ColorblindMode::None),
    ("protanopia", ColorblindMode::Protanopia),
    ("deuteranopia", ColorblindMode::Deuteranopia),
    ("tritanopia", ColorblindMode::Tritanopia),
];
const SIZE_NAMES: [(&str, SubtitleSize); 4] = [
    ("small", SubtitleSize::Small),
    ("medium", SubtitleSize::Medium),
    ("large", SubtitleSize::Large),
    ("extra_large", SubtitleSize::ExtraLarge),
];
const BACKGROUND_NAMES: [(&str, SubtitleBackground); 3] = [
    ("none", SubtitleBackground::None),
    ("translucent", SubtitleBackground::Translucent),
    ("opaque", SubtitleBackground::Opaque),
];
const HOLD_NAMES: [(&str, HoldMode); 2] = [("hold", HoldMode::Hold), ("toggle", HoldMode::Toggle)];

fn name_of<T: Copy + PartialEq>(table: &[(&'static str, T)], v: T) -> &'static str {
    table.iter().find(|(_, x)| *x == v).map_or("?", |(n, _)| *n)
}

fn words<T>(table: &[(&str, T)]) -> String {
    table.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
}

impl AccessibilitySettings {
    /// The value of a setting as text (the form [`set`](Self::set) reads), or `None` for an unknown name.
    pub fn get(&self, name: &str) -> Option<String> {
        let onoff = |b: bool| if b { "on" } else { "off" }.to_owned();
        Some(match name {
            "text_scale" => self.text_scale.to_string(),
            "colorblind" => name_of(&COLORBLIND_NAMES, self.colorblind).to_owned(),
            "subtitle_size" => name_of(&SIZE_NAMES, self.subtitle_size).to_owned(),
            "subtitle_background" => {
                name_of(&BACKGROUND_NAMES, self.subtitle_background).to_owned()
            }
            "reduce_flashing" => onoff(self.reduce_flashing),
            "reduce_motion" => onoff(self.reduce_motion),
            "high_contrast" => onoff(self.high_contrast),
            "crouch" => name_of(&HOLD_NAMES, self.crouch).to_owned(),
            "sprint" => name_of(&HOLD_NAMES, self.sprint).to_owned(),
            "aim" => name_of(&HOLD_NAMES, self.aim).to_owned(),
            _ => return None,
        })
    }

    /// Set a setting from text. An unknown name or a value that is not allowed is refused with a message saying what is allowed, and **nothing changes**.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), SettingError> {
        let bad = |expected: String| SettingError(format!("{name} takes {expected}"));
        match name {
            "text_scale" => {
                let (lo, hi) = TEXT_SCALE_RANGE;
                match value.trim().parse::<f32>() {
                    Ok(v) if (lo..=hi).contains(&v) => self.text_scale = v,
                    _ => return Err(bad(format!("a number from {lo} to {hi}"))),
                }
            }
            "colorblind" => {
                self.colorblind = pick(&COLORBLIND_NAMES, value)
                    .ok_or_else(|| bad(format!("one of {}", words(&COLORBLIND_NAMES))))?
            }
            "subtitle_size" => {
                self.subtitle_size = pick(&SIZE_NAMES, value)
                    .ok_or_else(|| bad(format!("one of {}", words(&SIZE_NAMES))))?
            }
            "subtitle_background" => {
                self.subtitle_background = pick(&BACKGROUND_NAMES, value)
                    .ok_or_else(|| bad(format!("one of {}", words(&BACKGROUND_NAMES))))?
            }
            "reduce_flashing" => {
                self.reduce_flashing =
                    parse_bool(value).ok_or_else(|| bad("on or off".to_owned()))?
            }
            "reduce_motion" => {
                self.reduce_motion = parse_bool(value).ok_or_else(|| bad("on or off".to_owned()))?
            }
            "high_contrast" => {
                self.high_contrast = parse_bool(value).ok_or_else(|| bad("on or off".to_owned()))?
            }
            "crouch" => {
                self.crouch =
                    pick(&HOLD_NAMES, value).ok_or_else(|| bad("hold or toggle".to_owned()))?
            }
            "sprint" => {
                self.sprint =
                    pick(&HOLD_NAMES, value).ok_or_else(|| bad("hold or toggle".to_owned()))?
            }
            "aim" => {
                self.aim =
                    pick(&HOLD_NAMES, value).ok_or_else(|| bad("hold or toggle".to_owned()))?
            }
            _ => {
                return Err(SettingError(format!(
                    "there is no accessibility setting called {name:?}"
                )));
            }
        }
        Ok(())
    }

    /// The size of text of `base_px` pixels after the player's text scale (never negative; 0 or a non-number stays 0).
    pub fn scaled_text(&self, base_px: f32) -> f32 {
        if !base_px.is_finite() || base_px <= 0.0 {
            return 0.0;
        }
        base_px
            * self
                .text_scale
                .clamp(TEXT_SCALE_RANGE.0, TEXT_SCALE_RANGE.1)
    }

    /// The HUD status colours for this player.
    pub fn status_palette(&self) -> StatusPalette {
        StatusPalette::for_mode(self.colorblind)
    }

    /// How strong flashes may be, as a multiplier on their alpha: 1 normally, 0.25 with reduced flashing.
    pub fn flash_strength(&self) -> f32 {
        if self.reduce_flashing { 0.25 } else { 1.0 }
    }

    /// How strong screen warping and shake may be, as a multiplier: 1 normally, 0 with reduced motion.
    pub fn motion_strength(&self) -> f32 {
        if self.reduce_motion { 0.0 } else { 1.0 }
    }

    /// The contrast the player's text must have against its background: [`MIN_TEXT_CONTRAST`] normally, the highest possible (7, WCAG AAA) with high contrast.
    pub fn text_contrast(&self) -> f32 {
        if self.high_contrast {
            7.0
        } else {
            MIN_TEXT_CONTRAST
        }
    }

    /// How subtitles look: the size multiplier (on top of the text scale), the alpha of the box behind them, and the text colour made to meet the contrast the player needs
    /// against that box (black), starting from `preferred_text`. High contrast forces an opaque box.
    pub fn subtitle_style(&self, preferred_text: [f32; 3]) -> SubtitleStyle {
        let background_alpha = if self.high_contrast {
            1.0
        } else {
            self.subtitle_background.alpha()
        };
        let text = if background_alpha > 0.0 {
            ensure_contrast(preferred_text, [0.0; 3], self.text_contrast())
        } else {
            preferred_text.map(clamp01)
        };
        SubtitleStyle {
            size_factor: self.subtitle_size.factor()
                * self
                    .text_scale
                    .clamp(TEXT_SCALE_RANGE.0, TEXT_SCALE_RANGE.1),
            background_alpha,
            text,
        }
    }
}

/// How subtitles are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubtitleStyle {
    /// Multiplier on the base subtitle size (the subtitle size choice times the text scale).
    pub size_factor: f32,
    /// Alpha of the black box behind the text (0 for none).
    pub background_alpha: f32,
    /// The text colour.
    pub text: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    // ---- colour blindness -------------------------------------------------------------------------

    #[test]
    fn simulation_keeps_greys_grey_and_leaves_typical_vision_alone() {
        for mode in [
            ColorblindMode::Protanopia,
            ColorblindMode::Deuteranopia,
            ColorblindMode::Tritanopia,
        ] {
            for g in [0.0, 0.2, 0.5, 1.0] {
                let s = simulate(mode, [g, g, g]);
                assert!(s.iter().all(|c| close(*c, g, 1e-4)), "{mode:?} {g}: {s:?}");
            }
        }
        assert_eq!(
            simulate(ColorblindMode::None, [0.1, 0.5, 0.9]),
            [0.1, 0.5, 0.9]
        );
        assert_eq!(
            simulate(ColorblindMode::None, [2.0, -1.0, f32::NAN]),
            [1.0, 0.0, 0.0],
            "clamped, NaN is 0"
        );
        assert_eq!(
            simulate(ColorblindMode::Protanopia, [f32::NAN; 3]),
            [0.0; 3]
        );
    }

    #[test]
    fn red_and_green_collapse_for_red_green_blindness_but_not_for_blue_yellow() {
        let red = [1.0, 0.0, 0.0];
        let green = [0.0, 1.0, 0.0];
        let typical = perceived_difference(ColorblindMode::None, red, green);
        let prot = perceived_difference(ColorblindMode::Protanopia, red, green);
        let deut = perceived_difference(ColorblindMode::Deuteranopia, red, green);
        assert!(typical > 400.0, "{typical}");
        assert!(
            prot < typical * 0.6 && deut < typical * 0.6,
            "{prot} {deut} against {typical}"
        );
        // Blue and yellow are what a tritan person confuses.
        let blue = [0.0, 0.0, 1.0];
        let yellow = [1.0, 1.0, 0.0];
        assert!(
            perceived_difference(ColorblindMode::Tritanopia, blue, yellow)
                < perceived_difference(ColorblindMode::None, blue, yellow)
        );
        // Simulated red under protanopia keeps its green and blue content: it goes dark and olive, not pink.
        let s = simulate(ColorblindMode::Protanopia, red);
        assert!(s[0] < 0.2 && s[1] < 0.2, "{s:?}");
    }

    #[test]
    fn the_difference_is_zero_for_the_same_colour_symmetric_and_about_765_for_black_and_white() {
        for mode in [ColorblindMode::None, ColorblindMode::Deuteranopia] {
            let c = [0.3, 0.6, 0.1];
            assert_eq!(perceived_difference(mode, c, c), 0.0);
            let d = [0.9, 0.2, 0.5];
            assert!(close(
                perceived_difference(mode, c, d),
                perceived_difference(mode, d, c),
                1e-3
            ));
        }
        let bw = perceived_difference(ColorblindMode::None, [0.0; 3], [1.0; 3]);
        assert!(close(bw, 765.0, 15.0), "{bw}");
    }

    #[test]
    fn the_safe_palette_stays_apart_for_everyone_and_the_traditional_one_does_not() {
        let safe = StatusPalette::safe();
        let traditional = StatusPalette::traditional();
        for mode in [
            ColorblindMode::None,
            ColorblindMode::Protanopia,
            ColorblindMode::Deuteranopia,
            ColorblindMode::Tritanopia,
        ] {
            assert!(
                safe.min_difference(mode) >= 250.0,
                "{mode:?}: {}",
                safe.min_difference(mode)
            );
            assert!(safe.min_difference(mode) >= MIN_DISTINGUISHABLE);
        }
        // Green against yellow for a protan, and green against red for a deutan, are the confusions the traditional colours have.
        assert!(
            traditional.min_difference(ColorblindMode::Protanopia) < MIN_DISTINGUISHABLE,
            "{}",
            traditional.min_difference(ColorblindMode::Protanopia)
        );
        assert!(
            traditional.min_difference(ColorblindMode::Deuteranopia) < MIN_DISTINGUISHABLE,
            "{}",
            traditional.min_difference(ColorblindMode::Deuteranopia)
        );
        assert!(traditional.min_difference(ColorblindMode::None) >= MIN_DISTINGUISHABLE);
        assert_eq!(StatusPalette::for_mode(ColorblindMode::None), traditional);
        for mode in [
            ColorblindMode::Protanopia,
            ColorblindMode::Deuteranopia,
            ColorblindMode::Tritanopia,
        ] {
            assert_eq!(StatusPalette::for_mode(mode), safe);
        }
    }

    // ---- contrast ------------------------------------------------------------------------------------------

    #[test]
    fn dark_colours_use_the_linear_toe_of_the_srgb_curve() {
        // 0.01 linear is sRGB 25.46 (not 33 as a longer toe would give); a grey pair 25.46 apart in every channel is 76.37 by the redmean formula.
        let d = perceived_difference(ColorblindMode::None, [0.01; 3], [0.0; 3]);
        assert!(close(d, 76.37, 0.05), "{d}");
        // And just under the toe's edge (0.003) the curve is the straight line: 0.003 x 12.92 x 255 = 9.88 per channel.
        let small = perceived_difference(ColorblindMode::None, [0.003; 3], [0.0; 3]);
        let line = 0.003 * 12.92 * 255.0;
        assert!(
            close(small, 3.0 * line, 0.1),
            "{small} against {}",
            3.0 * line
        ); // the three terms of the formula add up to 9, so the root is 3
        assert!(small < d, "darker greys are closer together");
    }

    #[test]
    fn the_target_is_black_for_a_mid_light_background_and_white_for_a_mid_dark_one() {
        // Luminance 0.3: black gives 7:1, white only 3:1, so black is the direction and the colour is moved only as far as needed.
        let bg = [0.3; 3];
        let fixed = ensure_contrast(bg, bg, 4.5);
        let r = contrast_ratio(fixed, bg);
        assert!((4.5..4.6).contains(&r), "{r}");
        assert!(
            fixed[0] < 0.3 && fixed[0] > 0.0,
            "{fixed:?} is darker than the background but not pure black"
        );
        // Luminance 0.1: white gives 7:1 (1.05 / 0.15), black only 3:1.
        let dark = [0.1; 3];
        let up = ensure_contrast(dark, dark, 4.5);
        assert!(up[0] > 0.1 && up[0] < 1.0, "{up:?}");
        assert!((4.5..4.6).contains(&contrast_ratio(up, dark)));
    }

    #[test]
    fn the_direction_flips_where_black_and_white_give_equal_contrast_near_a_luminance_of_0_18() {
        // At 0.16 white gives 5.0:1 and black 4.2:1: only white reaches 4.5, so the colour moves up.
        let below = [0.16; 3];
        let up = ensure_contrast(below, below, 4.5);
        assert!(
            up[0] > 0.16 && (4.5..4.6).contains(&contrast_ratio(up, below)),
            "moved only as far as needed, not to pure white: {up:?}"
        );
        // At 0.20 black gives 5.0:1 and white 4.2:1: only black reaches 4.5, so it moves down.
        let above = [0.20; 3];
        let down = ensure_contrast(above, above, 4.5);
        assert!(
            down[0] < 0.20 && (4.5..4.6).contains(&contrast_ratio(down, above)),
            "moved only as far as needed, not to pure black: {down:?}"
        );
    }

    #[test]
    fn a_colour_exactly_at_the_required_ratio_is_returned_untouched() {
        let exact = contrast_ratio([1.0; 3], [0.0; 3]);
        assert_eq!(ensure_contrast([1.0; 3], [0.0; 3], exact), [1.0; 3]);
        let mid = [0.4, 0.2, 0.1];
        let r = contrast_ratio(mid, [1.0; 3]);
        assert_eq!(ensure_contrast(mid, [1.0; 3], r), mid);
    }

    #[test]
    fn contrast_ratios_match_the_wcag_definition() {
        assert!(close(contrast_ratio([0.0; 3], [1.0; 3]), 21.0, 1e-4));
        assert!(
            close(contrast_ratio([1.0; 3], [0.0; 3]), 21.0, 1e-4),
            "symmetric"
        );
        assert!(close(contrast_ratio([0.5; 3], [0.5; 3]), 1.0, 1e-6));
        // Mid grey (sRGB 119 = linear 0.184) on white is the classic 4.48:1... close to the 4.5 line.
        let r = contrast_ratio([0.184, 0.184, 0.184], [1.0; 3]);
        assert!(close(r, (1.05) / (0.184 + 0.05), 1e-4), "{r}");
        assert!(close(relative_luminance([1.0, 0.0, 0.0]), 0.2126, 1e-6));
        assert!(close(relative_luminance([0.0, 1.0, 0.0]), 0.7152, 1e-6));
        assert!(close(relative_luminance([0.0, 0.0, 1.0]), 0.0722, 1e-6));
        assert_eq!(
            relative_luminance([f32::NAN, 2.0, -1.0]),
            0.7152,
            "clamped; NaN is 0"
        );
    }

    #[test]
    fn ensure_contrast_leaves_good_colours_and_fixes_bad_ones_by_the_least_amount() {
        let white = [1.0; 3];
        let black = [0.0; 3];
        assert_eq!(ensure_contrast(white, black, 4.5), white, "already fine");
        // Light grey on white: pushed toward black until the ratio is met, and no further.
        let fixed = ensure_contrast([0.8; 3], white, 4.5);
        let r = contrast_ratio(fixed, white);
        assert!((4.5..4.6).contains(&r), "{r}");
        assert!(fixed[0] < 0.8, "moved toward black");
        // Dark grey on black: pushed toward white.
        let up = ensure_contrast([0.02; 3], black, 7.0);
        assert!(contrast_ratio(up, black) >= 7.0 && up[0] > 0.02);
        // A hue is kept as far as it can be: a dark red on white that already passes is returned as it was.
        assert_eq!(
            ensure_contrast([0.4, 0.0, 0.0], white, 4.5),
            [0.4, 0.0, 0.0]
        );
        // Impossible asks get the better of black and white.
        assert_eq!(ensure_contrast([0.5; 3], white, 30.0), black);
        assert_eq!(ensure_contrast([0.5; 3], black, 30.0), white);
        // Mid-luminance backgrounds: the result still meets what is possible.
        for g in [0.1, 0.2, 0.3, 0.5, 0.7, 0.9] {
            let bg = [g; 3];
            let out = ensure_contrast([g; 3], bg, 3.0);
            assert!(contrast_ratio(out, bg) >= 3.0, "{g}: {out:?}");
        }
        assert_eq!(
            ensure_contrast([2.0, f32::NAN, -1.0], black, 1.0),
            [1.0, 0.0, 0.0],
            "clamped"
        );
    }

    // ---- hold and toggle ----------------------------------------------------------------------------------------

    #[test]
    fn hold_follows_the_button_and_toggle_flips_on_each_press() {
        let mut hold = HoldToggle::new(HoldMode::Hold);
        let states: Vec<bool> = [false, true, true, false, true]
            .iter()
            .map(|d| hold.update(*d))
            .collect();
        assert_eq!(states, [false, true, true, false, true]);
        let mut toggle = HoldToggle::new(HoldMode::Toggle);
        let states: Vec<bool> = [
            false, true, true, true, false, false, true, false, true, true,
        ]
        .iter()
        .map(|d| toggle.update(*d))
        .collect();
        // Off; pressed: on; still held: on; held: on; released: on; idle: on; pressed: off; released: off; pressed: on; held: on.
        assert_eq!(
            states,
            [
                false, true, true, true, true, true, false, false, true, true
            ]
        );
    }

    #[test]
    fn cancel_and_changing_the_mode_switch_the_action_off() {
        let mut t = HoldToggle::new(HoldMode::Toggle);
        t.update(true);
        t.update(false);
        assert!(t.is_on());
        t.cancel();
        assert!(!t.is_on());
        assert!(!t.update(false), "no press, still off");
        t.update(true);
        t.update(false);
        t.set_mode(HoldMode::Hold);
        assert!(!t.is_on());
        assert!(t.update(true) && !t.update(false));
    }

    #[test]
    fn a_held_button_through_a_mode_change_is_not_a_new_press() {
        let mut t = HoldToggle::new(HoldMode::Hold);
        t.update(true);
        t.set_mode(HoldMode::Toggle);
        assert!(
            !t.update(true),
            "the button was already down: no press edge"
        );
        assert!(!t.update(false));
        assert!(t.update(true), "a new press toggles");
    }

    // ---- flash limiting -------------------------------------------------------------------------------------------

    #[test]
    fn no_more_than_three_flashes_in_any_second_when_limited() {
        let mut l = FlashLimiter::new();
        let times: Vec<f32> = (0..30).map(|i| i as f32 * 0.1).collect();
        let fired: Vec<f32> = times
            .iter()
            .copied()
            .filter(|t| l.allow(*t, true))
            .collect();
        // Three at 0.0, 0.1, 0.2; nothing until 1.0; then 1.0, 1.1, 1.2; and so on.
        assert_eq!(
            fired
                .iter()
                .map(|t| (t * 10.0).round() as i32)
                .collect::<Vec<_>>(),
            [0, 1, 2, 10, 11, 12, 20, 21, 22]
        );
        for w in fired.windows(4) {
            assert!(
                w[3] - w[0] >= 1.0 - 1e-5,
                "four flashes inside a second: {w:?}"
            );
        }
    }

    #[test]
    fn the_limiter_counts_only_what_fired_and_is_off_when_not_asked_for() {
        let mut l = FlashLimiter::new();
        for _ in 0..10 {
            assert!(l.allow(0.0, false), "unlimited, and not counted");
        }
        assert!(l.allow(0.0, true) && l.allow(0.0, true) && l.allow(0.0, true));
        assert!(!l.allow(0.5, true));
        assert!(
            !l.allow(0.99, true),
            "refused flashes are not counted, but the three are still within the second"
        );
        assert!(
            l.allow(1.0, true),
            "the first has left the window (exactly one second ago)"
        );
        assert!(!l.allow(f32::NAN, true));
        assert!(!l.allow(f32::INFINITY, true));
        assert!(l.allow(f32::NAN, false));
    }

    // ---- settings -------------------------------------------------------------------------------------------------

    #[test]
    fn every_option_reads_back_through_its_own_text() {
        let mut s = AccessibilitySettings::default();
        for (name, value) in [
            ("text_scale", "1.5"),
            ("colorblind", "deuteranopia"),
            ("subtitle_size", "extra_large"),
            ("subtitle_background", "opaque"),
            ("reduce_flashing", "on"),
            ("reduce_motion", "on"),
            ("high_contrast", "on"),
            ("crouch", "toggle"),
            ("sprint", "toggle"),
            ("aim", "toggle"),
        ] {
            s.set(name, value)
                .unwrap_or_else(|e| panic!("{name}={value}: {e}"));
        }
        let mut copy = AccessibilitySettings::default();
        for name in OPTION_NAMES {
            copy.set(name, &s.get(name).unwrap()).unwrap();
        }
        assert_eq!(copy, s);
        assert_eq!(OPTION_NAMES.len(), 10);
        assert!(OPTION_NAMES.iter().all(|n| s.get(n).is_some()));
        assert_eq!(s.get("nonsense"), None);
    }

    #[test]
    fn values_are_read_loosely_refused_strictly_and_a_refusal_changes_nothing() {
        let mut s = AccessibilitySettings::default();
        s.set("colorblind", " PROTANOPIA ").unwrap();
        s.set("reduce_motion", "Yes").unwrap();
        s.set("text_scale", "0.8").unwrap();
        s.set("text_scale", "2").unwrap();
        s.set("crouch", " Toggle").unwrap();
        let before = s;
        for (name, value) in [
            ("text_scale", "0.79"),
            ("text_scale", "2.01"),
            ("text_scale", "NaN"),
            ("text_scale", "inf"),
            ("text_scale", "big"),
            ("colorblind", "monochrome"),
            ("subtitle_size", "huge"),
            ("subtitle_background", "glass"),
            ("reduce_flashing", "maybe"),
            ("reduce_motion", ""),
            ("high_contrast", "2"),
            ("crouch", "press"),
            ("sprint", ""),
            ("aim", "hold please"),
        ] {
            let err = s.set(name, value).unwrap_err();
            assert!(err.to_string().starts_with(name), "{err}");
        }
        assert_eq!(s, before);
        assert!(s.set("nope", "1").unwrap_err().to_string().contains("nope"));
        assert_eq!(s, before);
    }

    // ---- what the settings decide ------------------------------------------------------------------------------------

    #[test]
    fn text_scale_multiplies_and_is_clamped() {
        let mut s = AccessibilitySettings::default();
        assert_eq!(s.scaled_text(20.0), 20.0);
        s.text_scale = 1.5;
        assert_eq!(s.scaled_text(20.0), 30.0);
        s.text_scale = 9.0; // never validated
        assert_eq!(s.scaled_text(20.0), 40.0);
        s.text_scale = 0.1;
        assert!(close(s.scaled_text(20.0), 16.0, 1e-5));
        s.text_scale = 1.0;
        for bad in [0.0, -3.0, f32::NAN, f32::INFINITY] {
            assert_eq!(s.scaled_text(bad), 0.0, "{bad}");
        }
    }

    #[test]
    fn reduced_flashing_motion_and_contrast_change_the_strengths() {
        let mut s = AccessibilitySettings::default();
        assert_eq!(
            (s.flash_strength(), s.motion_strength(), s.text_contrast()),
            (1.0, 1.0, MIN_TEXT_CONTRAST)
        );
        s.reduce_flashing = true;
        s.reduce_motion = true;
        s.high_contrast = true;
        assert_eq!(
            (s.flash_strength(), s.motion_strength(), s.text_contrast()),
            (0.25, 0.0, 7.0)
        );
        s.colorblind = ColorblindMode::Deuteranopia;
        assert_eq!(s.status_palette(), StatusPalette::safe());
        assert_eq!(
            AccessibilitySettings::default().status_palette(),
            StatusPalette::traditional()
        );
    }

    #[test]
    fn subtitles_combine_size_scale_background_and_contrast() {
        let mut s = AccessibilitySettings::default();
        let normal = s.subtitle_style([1.0, 1.0, 1.0]);
        assert_eq!(
            (normal.size_factor, normal.background_alpha, normal.text),
            (1.0, 0.6, [1.0, 1.0, 1.0])
        );
        s.subtitle_size = SubtitleSize::Large;
        s.text_scale = 1.5;
        assert!(close(
            s.subtitle_style([1.0; 3]).size_factor,
            1.3 * 1.5,
            1e-6
        ));
        s.subtitle_background = SubtitleBackground::None;
        assert_eq!(s.subtitle_style([0.3, 0.3, 0.3]).background_alpha, 0.0);
        assert_eq!(
            s.subtitle_style([0.3, 0.3, 0.3]).text,
            [0.3, 0.3, 0.3],
            "no box: the colour is left alone"
        );
        // A dark preferred colour on the dark box is lifted to the contrast the player needs; high contrast forces an opaque box and the stricter ratio.
        s.subtitle_background = SubtitleBackground::Opaque;
        let lifted = s.subtitle_style([0.05, 0.05, 0.05]);
        assert!(contrast_ratio(lifted.text, [0.0; 3]) >= MIN_TEXT_CONTRAST);
        s.subtitle_background = SubtitleBackground::None;
        s.high_contrast = true;
        let hc = s.subtitle_style([0.3, 0.3, 0.3]);
        assert_eq!(hc.background_alpha, 1.0);
        assert!(contrast_ratio(hc.text, [0.0; 3]) >= 7.0);
        assert_eq!(
            [
                SubtitleSize::Small,
                SubtitleSize::Medium,
                SubtitleSize::Large,
                SubtitleSize::ExtraLarge
            ]
            .map(SubtitleSize::factor),
            [0.8, 1.0, 1.3, 1.7]
        );
        assert_eq!(
            [
                SubtitleBackground::None,
                SubtitleBackground::Translucent,
                SubtitleBackground::Opaque
            ]
            .map(SubtitleBackground::alpha),
            [0.0, 0.6, 1.0]
        );
    }
}
