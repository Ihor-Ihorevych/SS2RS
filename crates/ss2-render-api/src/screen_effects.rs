//! Screen-space effects: **damage and pickup flashes, screen fades, the underwater overlay and cutscene letterboxing** (roadmap T04-031, `docs/SCREEN_EFFECTS.md`).
//!
//! Device-free. [`ScreenEffects`] is a small state machine the game drives with events (`flash`, `fade_to`, `set_underwater`, `set_letterbox`) and with **time**
//! ([`update`](ScreenEffects::update), in seconds, supplied by the caller: no clock inside, so it is deterministic and testable). Each frame
//! [`frame`](ScreenEffects::frame) turns the state into what a renderer draws: a list of full-screen tinted quads in draw order, the height of the letterbox bars in
//! pixels and how strongly to distort the picture for the underwater look. The renderer owns the drawing.
//!
//! Colours are **linear RGBA, not premultiplied**: a quad of colour `c` and alpha `a` replaces `a` of the picture with `c`.

/// A kind of flash. Re-triggering a kind restarts it instead of stacking a second copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlashKind {
    /// The player took damage.
    Damage,
    /// The player picked something up.
    Pickup,
    /// The player healed, or a shield recharged.
    Heal,
    /// Anything else (a number picks which of 256 slots).
    Custom(u8),
}

/// A full-screen quad to draw, in this order (first is at the back).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorQuad {
    /// Linear RGBA, alpha in `[0, 1]`.
    pub color: [f32; 4],
}

/// What to draw this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenEffectsFrame {
    /// Full-screen quads, back to front: the underwater tint, then the flashes, then the fade.
    pub quads: Vec<ColorQuad>,
    /// Height in pixels of the bar at the top **and** of the bar at the bottom (0 for no letterbox).
    pub letterbox_bar_px: f32,
    /// How much to distort the picture for the underwater look, `[0, 1]` (0 is none).
    pub underwater_distortion: f32,
}

#[derive(Clone, Copy, Debug)]
struct Flash {
    kind: FlashKind,
    color: [f32; 3],
    peak: f32,
    duration: f32,
    age: f32,
}

impl Flash {
    /// Strongest at the start and easing out: `peak x (1 - age / duration)^2`.
    fn alpha(&self) -> f32 {
        let left = (1.0 - self.age / self.duration).clamp(0.0, 1.0);
        self.peak * left * left
    }
}

#[derive(Clone, Copy, Debug)]
struct Fade {
    color: [f32; 3],
    from: f32,
    to: f32,
    duration: f32,
    age: f32,
}

impl Fade {
    fn alpha(&self) -> f32 {
        if self.age >= self.duration {
            return self.to;
        }
        let t = self.age / self.duration;
        let eased = t * t * (3.0 - 2.0 * t); // smoothstep: gentle at both ends
        self.from + (self.to - self.from) * eased
    }
}

/// How long the underwater look takes to fade in or out, in seconds.
pub const UNDERWATER_BLEND_SECONDS: f32 = 0.35;

/// How long a letterbox takes to slide in or out when no duration is given, in seconds.
pub const DEFAULT_LETTERBOX_SECONDS: f32 = 0.6;

/// The tint of the underwater overlay (linear RGB) and its strongest alpha.
const UNDERWATER_TINT: [f32; 3] = [0.05, 0.25, 0.4];
const UNDERWATER_ALPHA: f32 = 0.35;

fn clamp01(x: f32) -> f32 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

fn clamp_rgb(c: [f32; 3]) -> [f32; 3] {
    [clamp01(c[0]), clamp01(c[1]), clamp01(c[2])]
}

/// The effect state.
///
/// ```
/// use ss2_render_api::screen_effects::{FlashKind, ScreenEffects};
///
/// let mut fx = ScreenEffects::new();
/// fx.flash(FlashKind::Damage, [1.0, 0.0, 0.0], 0.5, 0.4); // a red flash, half strength at most, for 0.4 s
/// let first = fx.frame(1920.0, 1080.0);
/// assert!((first.quads[0].color[3] - 0.5).abs() < 1e-6);
/// fx.update(0.4);
/// assert!(fx.frame(1920.0, 1080.0).quads.is_empty()); // gone
/// ```
#[derive(Clone, Debug)]
pub struct ScreenEffects {
    flashes: Vec<Flash>,
    fade: Option<Fade>,
    /// The fade's alpha when no fade is running (a completed fade-out holds black).
    fade_hold: f32,
    fade_hold_color: [f32; 3],
    underwater_target: f32,
    underwater: f32,
    letterbox_aspect: Option<f32>,
    letterbox_target: f32,
    letterbox: f32,
    letterbox_seconds: f32,
}

impl Default for ScreenEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenEffects {
    /// No effects.
    pub fn new() -> Self {
        Self {
            flashes: Vec::new(),
            fade: None,
            fade_hold: 0.0,
            fade_hold_color: [0.0; 3],
            underwater_target: 0.0,
            underwater: 0.0,
            letterbox_aspect: None,
            letterbox_target: 0.0,
            letterbox: 0.0,
            letterbox_seconds: DEFAULT_LETTERBOX_SECONDS,
        }
    }

    /// Start a flash of `color` that begins at alpha `peak` (clamped to 0 to 1) and eases out over `duration` seconds. A flash of the same kind that is
    /// already running is **replaced**, keeping the stronger of the two current alphas as its start so a burst of hits does not weaken the flash. A non-positive
    /// or non-finite duration, or a peak of 0, does nothing.
    pub fn flash(&mut self, kind: FlashKind, color: [f32; 3], peak: f32, duration: f32) {
        let peak = clamp01(peak);
        if !(duration.is_finite() && duration > 0.0) || peak == 0.0 {
            return;
        }
        let mut start = peak;
        if let Some(i) = self.flashes.iter().position(|f| f.kind == kind) {
            start = start.max(self.flashes[i].alpha());
            self.flashes.remove(i);
        }
        self.flashes.push(Flash {
            kind,
            color: clamp_rgb(color),
            peak: start,
            duration,
            age: 0.0,
        });
        self.flashes.sort_by_key(|f| f.kind);
    }

    /// Fade the screen to `alpha` of `color` over `seconds` (alpha 1 is solid colour: a fade to black; alpha 0 fades back in). A duration of zero or less jumps at once.
    /// The fade starts from wherever the screen is now, so reversing mid-fade is smooth.
    pub fn fade_to(&mut self, color: [f32; 3], alpha: f32, seconds: f32) {
        let to = clamp01(alpha);
        let from = self.fade_alpha();
        let color = clamp_rgb(color);
        if !(seconds.is_finite() && seconds > 0.0) {
            self.fade = None;
            self.fade_hold = to;
            self.fade_hold_color = color;
            return;
        }
        self.fade_hold_color = color;
        self.fade = Some(Fade {
            color,
            from,
            to,
            duration: seconds,
            age: 0.0,
        });
    }

    /// The fade's alpha now (0 when there is none).
    pub fn fade_alpha(&self) -> f32 {
        self.fade.map_or(self.fade_hold, |f| f.alpha())
    }

    /// Whether a fade is still moving.
    pub fn fade_running(&self) -> bool {
        self.fade.is_some()
    }

    /// Turn the underwater look on or off; it blends over [`UNDERWATER_BLEND_SECONDS`].
    pub fn set_underwater(&mut self, on: bool) {
        self.underwater_target = if on { 1.0 } else { 0.0 };
    }

    /// How far the underwater blend has got, `[0, 1]`.
    pub fn underwater_amount(&self) -> f32 {
        self.underwater
    }

    /// Slide cinematic bars in so the picture has `aspect` (width over height, for example 2.39) on a taller screen, over `seconds`; `None` slides them out.
    /// An aspect that is not a positive finite number is treated as `None`. A screen that is already as wide as `aspect` or wider gets no bars (it would need
    /// pillarboxing, which is not done here).
    pub fn set_letterbox(&mut self, aspect: Option<f32>, seconds: f32) {
        let aspect = aspect.filter(|a| a.is_finite() && *a > 0.0);
        self.letterbox_aspect = aspect.or(self.letterbox_aspect);
        self.letterbox_target = if aspect.is_some() { 1.0 } else { 0.0 };
        self.letterbox_seconds = if seconds.is_finite() && seconds > 0.0 {
            seconds
        } else {
            0.0
        };
        if self.letterbox_seconds == 0.0 {
            self.letterbox = self.letterbox_target;
        }
    }

    /// How far the bars have slid in, `[0, 1]`.
    pub fn letterbox_amount(&self) -> f32 {
        self.letterbox
    }

    /// Whether any effect is active or visible (a renderer can skip the pass when this is false).
    pub fn is_active(&self) -> bool {
        !self.flashes.is_empty()
            || self.fade_running()
            || self.fade_hold > 0.0
            || self.underwater > 0.0
            || self.letterbox > 0.0
            || self.underwater_target > 0.0
            || self.letterbox_target > 0.0
    }

    /// Advance by `dt` seconds. A `dt` that is negative or not finite is ignored.
    pub fn update(&mut self, dt: f32) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        for f in &mut self.flashes {
            f.age += dt;
        }
        self.flashes.retain(|f| f.age < f.duration);
        if let Some(fade) = &mut self.fade {
            fade.age += dt;
            if fade.age >= fade.duration {
                self.fade_hold = fade.to;
                self.fade_hold_color = fade.color;
                self.fade = None;
            }
        }
        self.underwater = approach(
            self.underwater,
            self.underwater_target,
            dt / UNDERWATER_BLEND_SECONDS,
        );
        if self.letterbox_seconds > 0.0 {
            self.letterbox = approach(
                self.letterbox,
                self.letterbox_target,
                dt / self.letterbox_seconds,
            );
        }
    }

    /// What to draw for a screen of `width` x `height` pixels.
    pub fn frame(&self, width: f32, height: f32) -> ScreenEffectsFrame {
        let mut quads = Vec::new();
        if self.underwater > 0.0 {
            quads.push(ColorQuad {
                color: [
                    UNDERWATER_TINT[0],
                    UNDERWATER_TINT[1],
                    UNDERWATER_TINT[2],
                    UNDERWATER_ALPHA * self.underwater,
                ],
            });
        }
        for f in &self.flashes {
            let a = f.alpha();
            if a > 0.0 {
                quads.push(ColorQuad {
                    color: [f.color[0], f.color[1], f.color[2], a],
                });
            }
        }
        let fade = self.fade_alpha();
        if fade > 0.0 {
            let c = self.fade.map_or(self.fade_hold_color, |f| f.color);
            quads.push(ColorQuad {
                color: [c[0], c[1], c[2], fade],
            });
        }
        ScreenEffectsFrame {
            quads,
            letterbox_bar_px: self.bar_height(width, height),
            underwater_distortion: self.underwater,
        }
    }

    fn bar_height(&self, width: f32, height: f32) -> f32 {
        let Some(aspect) = self.letterbox_aspect else {
            return 0.0;
        };
        if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
            return 0.0;
        }
        // The picture is `width / aspect` tall; the rest is split over two bars. A screen already wider than the aspect has no room for them.
        let full = ((height - width / aspect) * 0.5).max(0.0);
        full * self.letterbox
    }
}

/// Move `value` toward `target` by `step` (a share of the whole 0 to 1 range), without overshooting.
fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [f32; 3] = [1.0, 0.0, 0.0];

    fn alphas(fx: &ScreenEffects) -> Vec<f32> {
        fx.frame(1920.0, 1080.0)
            .quads
            .iter()
            .map(|q| q.color[3])
            .collect()
    }

    // ---- flashes ----------------------------------------------------------------------------------

    #[test]
    fn a_flash_starts_at_its_peak_and_eases_out_to_nothing() {
        let mut fx = ScreenEffects::new();
        fx.flash(FlashKind::Damage, RED, 0.8, 1.0);
        let q = fx.frame(100.0, 100.0).quads;
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].color, [1.0, 0.0, 0.0, 0.8]);
        fx.update(0.5);
        assert!(
            (alphas(&fx)[0] - 0.8 * 0.25).abs() < 1e-6,
            "(1 - 0.5)^2 of the peak"
        );
        fx.update(0.499);
        assert!(alphas(&fx)[0] < 0.001);
        fx.update(0.001);
        assert!(alphas(&fx).is_empty(), "at the end it is removed");
        assert!(!fx.is_active());
    }

    #[test]
    fn a_flash_never_gets_stronger_as_time_passes() {
        let mut fx = ScreenEffects::new();
        fx.flash(FlashKind::Pickup, [0.2, 0.9, 0.3], 1.0, 2.0);
        let mut last = f32::MAX;
        for _ in 0..41 {
            // (41 steps of 0.05 s: 40 would stop a hair short of 2 s in f32)
            if let Some(a) = alphas(&fx).first() {
                assert!(*a <= last);
                last = *a;
            }
            fx.update(0.05);
        }
        assert!(alphas(&fx).is_empty());
    }

    #[test]
    fn retriggering_a_kind_restarts_it_keeping_the_stronger_alpha() {
        let mut fx = ScreenEffects::new();
        fx.flash(FlashKind::Damage, RED, 0.4, 1.0);
        fx.update(0.1); // alpha is now 0.4 x 0.81
        let now = alphas(&fx)[0];
        fx.flash(FlashKind::Damage, RED, 0.2, 1.0); // a weaker hit: the flash restarts from what it showed
        assert!(
            (alphas(&fx)[0] - now).abs() < 1e-6,
            "{} vs {now}",
            alphas(&fx)[0]
        );
        assert_eq!(alphas(&fx).len(), 1, "one flash per kind");
        fx.flash(FlashKind::Damage, RED, 0.9, 1.0); // a stronger one takes over
        assert!((alphas(&fx)[0] - 0.9).abs() < 1e-6);
    }

    #[test]
    fn different_kinds_are_drawn_together_in_a_fixed_order() {
        let mut fx = ScreenEffects::new();
        fx.flash(FlashKind::Custom(3), [0.0, 0.0, 1.0], 0.3, 1.0);
        fx.flash(FlashKind::Heal, [0.0, 1.0, 0.0], 0.2, 1.0);
        fx.flash(FlashKind::Damage, RED, 0.5, 1.0);
        let colors: Vec<[f32; 3]> = fx
            .frame(10.0, 10.0)
            .quads
            .iter()
            .map(|q| [q.color[0], q.color[1], q.color[2]])
            .collect();
        // Damage, Heal, then Custom: the order of the enum, whatever order they were triggered in.
        assert_eq!(colors, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    }

    #[test]
    fn useless_flashes_are_ignored_and_colours_and_peaks_are_clamped() {
        let mut fx = ScreenEffects::new();
        for (peak, duration) in [
            (0.5, 0.0),
            (0.5, -1.0),
            (0.5, f32::NAN),
            (0.5, f32::INFINITY),
            (0.0, 1.0),
            (f32::NAN, 1.0),
            (-0.3, 1.0),
        ] {
            fx.flash(FlashKind::Damage, RED, peak, duration);
        }
        assert!(!fx.is_active(), "none of those starts a flash");
        fx.flash(FlashKind::Damage, [2.0, -1.0, f32::NAN], 7.0, 1.0);
        assert_eq!(fx.frame(1.0, 1.0).quads[0].color, [1.0, 0.0, 0.0, 1.0]);
    }

    // ---- fades ----------------------------------------------------------------------------------------

    #[test]
    fn a_fade_to_black_follows_a_smoothstep_and_holds_at_the_end() {
        let mut fx = ScreenEffects::new();
        fx.fade_to([0.0; 3], 1.0, 2.0);
        assert!(fx.fade_running());
        assert_eq!(fx.fade_alpha(), 0.0);
        assert!(alphas(&fx).is_empty(), "alpha 0 draws nothing");
        fx.update(1.0);
        assert!(
            (fx.fade_alpha() - 0.5).abs() < 1e-6,
            "halfway of a smoothstep is one half"
        );
        fx.update(0.5); // t = 0.75: 0.75^2 x (3 - 1.5) = 0.84375
        assert!((fx.fade_alpha() - 0.84375).abs() < 1e-5);
        fx.update(0.5);
        assert!(!fx.fade_running());
        assert_eq!(fx.fade_alpha(), 1.0);
        fx.update(10.0);
        assert_eq!(
            fx.frame(1.0, 1.0).quads,
            [ColorQuad {
                color: [0.0, 0.0, 0.0, 1.0]
            }],
            "a completed fade-out stays black"
        );
        assert!(fx.is_active());
    }

    #[test]
    fn the_fade_alpha_is_clamped_to_zero_to_one() {
        let mut fx = ScreenEffects::new();
        fx.fade_to([0.0; 3], 5.0, 0.0);
        assert_eq!(fx.fade_alpha(), 1.0);
        fx.fade_to([0.0; 3], -3.0, 0.0);
        assert_eq!(fx.fade_alpha(), 0.0);
        fx.fade_to([0.0; 3], f32::NAN, 0.0);
        assert_eq!(fx.fade_alpha(), 0.0, "NaN counts as zero");
        fx.fade_to([0.0; 3], 9.0, 1.0);
        fx.update(10.0);
        assert_eq!(fx.fade_alpha(), 1.0);
    }

    #[test]
    fn fading_back_in_starts_from_where_the_screen_is_and_ends_clear() {
        let mut fx = ScreenEffects::new();
        fx.fade_to([0.0; 3], 1.0, 1.0);
        fx.update(0.5); // half black
        let mid = fx.fade_alpha();
        assert!((mid - 0.5).abs() < 1e-6);
        fx.fade_to([0.0; 3], 0.0, 1.0); // reverse mid-fade
        assert!(
            (fx.fade_alpha() - mid).abs() < 1e-6,
            "no jump when reversing"
        );
        fx.update(1.0);
        assert_eq!(fx.fade_alpha(), 0.0);
        assert!(!fx.is_active());
        assert!(fx.frame(1.0, 1.0).quads.is_empty());
    }

    #[test]
    fn a_fade_of_no_duration_jumps_and_the_colour_is_kept() {
        let mut fx = ScreenEffects::new();
        for secs in [0.0, -2.0, f32::NAN] {
            fx.fade_to([1.0, 1.0, 1.0], 0.7, secs);
            assert!(!fx.fade_running());
            assert_eq!(fx.fade_alpha(), 0.7);
        }
        assert_eq!(fx.frame(1.0, 1.0).quads[0].color, [1.0, 1.0, 1.0, 0.7]);
        fx.fade_to([0.0, 0.0, 0.0], 5.0, 1.0); // a white screen fading to a 5 (clamped to 1) black: the colour in use is the new one
        fx.update(0.5);
        assert_eq!(fx.frame(1.0, 1.0).quads[0].color[..3], [0.0, 0.0, 0.0]);
    }

    // ---- underwater -------------------------------------------------------------------------------------

    #[test]
    fn the_underwater_look_blends_in_and_out_over_its_time() {
        let mut fx = ScreenEffects::new();
        assert_eq!(fx.underwater_amount(), 0.0);
        fx.set_underwater(true);
        assert!(fx.is_active(), "switching it on counts as active at once");
        fx.update(UNDERWATER_BLEND_SECONDS / 2.0);
        assert!((fx.underwater_amount() - 0.5).abs() < 1e-5);
        let f = fx.frame(1.0, 1.0);
        assert!((f.underwater_distortion - 0.5).abs() < 1e-5);
        assert!((f.quads[0].color[3] - UNDERWATER_ALPHA * 0.5).abs() < 1e-5);
        fx.update(5.0);
        assert_eq!(fx.underwater_amount(), 1.0, "no overshoot");
        assert!((fx.frame(1.0, 1.0).quads[0].color[3] - UNDERWATER_ALPHA).abs() < 1e-6);
        fx.set_underwater(false);
        fx.update(UNDERWATER_BLEND_SECONDS * 2.0);
        assert_eq!(fx.underwater_amount(), 0.0);
        assert!(fx.frame(1.0, 1.0).quads.is_empty());
        assert!(!fx.is_active());
    }

    #[test]
    fn leaving_the_water_mid_blend_turns_around_smoothly() {
        let mut fx = ScreenEffects::new();
        fx.set_underwater(true);
        fx.update(UNDERWATER_BLEND_SECONDS * 0.6);
        let at = fx.underwater_amount();
        fx.set_underwater(false);
        assert_eq!(fx.underwater_amount(), at, "no jump");
        fx.update(UNDERWATER_BLEND_SECONDS * 0.1);
        assert!(fx.underwater_amount() < at);
    }

    // ---- letterbox --------------------------------------------------------------------------------------

    #[test]
    fn letterbox_bars_leave_a_picture_of_the_requested_shape() {
        let mut fx = ScreenEffects::new();
        fx.set_letterbox(Some(2.0), 1.0);
        fx.update(1.0);
        // 1920 x 1080: a 2:1 picture is 960 tall, so 120 left over per bar.
        let bar = fx.frame(1920.0, 1080.0).letterbox_bar_px;
        let picture = 1080.0 - 2.0 * bar;
        assert!(
            (1920.0 / picture - 2.0).abs() < 1e-3,
            "picture {picture} tall is 2:1"
        );
        // The bar height is 60: (1080 - 960) / 2.
        assert!((bar - 60.0).abs() < 1e-3);
    }

    #[test]
    fn the_bars_slide_in_over_their_duration_and_back_out() {
        let mut fx = ScreenEffects::new();
        fx.set_letterbox(Some(2.39), 2.0);
        assert_eq!(fx.frame(1280.0, 720.0).letterbox_bar_px, 0.0);
        fx.update(1.0);
        assert!((fx.letterbox_amount() - 0.5).abs() < 1e-6);
        let half = fx.frame(1280.0, 720.0).letterbox_bar_px;
        fx.update(1.0);
        let full = fx.frame(1280.0, 720.0).letterbox_bar_px;
        assert!((half - full * 0.5).abs() < 1e-3 && full > 0.0);
        fx.set_letterbox(None, 1.0);
        fx.update(0.5);
        let out = fx.frame(1280.0, 720.0).letterbox_bar_px;
        assert!(out > 0.0 && out < full, "the bars are still leaving: {out}");
        fx.update(0.5);
        assert_eq!(fx.frame(1280.0, 720.0).letterbox_bar_px, 0.0);
        assert!(!fx.is_active());
    }

    #[test]
    fn a_screen_as_wide_as_the_picture_or_wider_gets_no_bars_and_bad_sizes_are_safe() {
        let mut fx = ScreenEffects::new();
        fx.set_letterbox(Some(1.5), 0.0);
        assert_eq!(fx.letterbox_amount(), 1.0, "a duration of zero jumps");
        assert_eq!(
            fx.frame(1920.0, 1080.0).letterbox_bar_px,
            0.0,
            "16:9 is wider than 1.5:1"
        );
        assert_eq!(
            fx.frame(1500.0, 1000.0).letterbox_bar_px,
            0.0,
            "exactly the aspect"
        );
        assert!(
            fx.frame(1000.0, 1000.0).letterbox_bar_px > 0.0,
            "a square screen gets bars"
        );
        for (w, h) in [
            (0.0, 100.0),
            (100.0, 0.0),
            (f32::NAN, 100.0),
            (100.0, f32::INFINITY),
            (-5.0, 5.0),
        ] {
            assert_eq!(fx.frame(w, h).letterbox_bar_px, 0.0);
        }
        // A bad aspect counts as "off".
        let mut fx = ScreenEffects::new();
        for bad in [
            Some(0.0),
            Some(-1.0),
            Some(f32::NAN),
            Some(f32::INFINITY),
            None,
        ] {
            fx.set_letterbox(bad, 0.0);
            assert_eq!(fx.letterbox_amount(), 0.0);
        }
    }

    // ---- composition ---------------------------------------------------------------------------------------

    #[test]
    fn everything_at_once_is_drawn_back_to_front_and_time_is_robust() {
        let mut fx = ScreenEffects::new();
        fx.set_underwater(true);
        fx.flash(FlashKind::Damage, RED, 0.5, 1.0);
        fx.fade_to([0.0; 3], 0.6, 0.0);
        fx.update(UNDERWATER_BLEND_SECONDS);
        let q = fx.frame(100.0, 100.0).quads;
        assert_eq!(q.len(), 3);
        assert_eq!(
            q[0].color[..3],
            UNDERWATER_TINT,
            "the underwater tint at the back"
        );
        assert_eq!(q[1].color[..3], [1.0, 0.0, 0.0], "then the flash");
        assert_eq!(
            q[2].color,
            [0.0, 0.0, 0.0, 0.6],
            "the fade on top of everything"
        );
        let before = fx.frame(100.0, 100.0);
        for bad in [f32::NAN, -1.0, 0.0, f32::INFINITY, f32::NEG_INFINITY] {
            fx.update(bad);
        }
        assert_eq!(
            fx.frame(100.0, 100.0),
            before,
            "bad time steps change nothing"
        );
    }

    #[test]
    fn the_same_calls_give_the_same_frames() {
        let run = || {
            let mut fx = ScreenEffects::new();
            fx.flash(FlashKind::Damage, RED, 0.7, 0.8);
            fx.set_underwater(true);
            fx.fade_to([0.1, 0.2, 0.3], 0.9, 1.3);
            fx.set_letterbox(Some(2.35), 0.9);
            let mut frames = Vec::new();
            for _ in 0..60 {
                fx.update(1.0 / 60.0);
                frames.push(fx.frame(1280.0, 720.0));
            }
            frames
        };
        assert_eq!(run(), run());
    }
}
