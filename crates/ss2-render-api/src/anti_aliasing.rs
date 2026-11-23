//! **Anti-aliasing planning**: which method to use, given what the player asked for and what the device can do (roadmap T04-029, `docs/ANTI_ALIASING.md`).
//!
//! Device-free. [`plan_anti_aliasing`] turns the quality setting ([`AntiAliasing`]) and an optional temporal request into an [`AaPlan`]: the number of MSAA samples
//! to create the render targets with, whether a post-process FXAA pass runs, whether temporal anti-aliasing (TAA) does. It **degrades gracefully**: MSAA the device
//! lacks falls to the best it has, and to FXAA when it has none, and the plan says it did so (`degraded`). [`render_target_plan`] sizes the targets and counts their
//! memory, because MSAA multiplies it and a phone must be able to see that before it asks.
//!
//! The drawing (the MSAA resolve, the FXAA shader, the TAA history blend) is the renderer's.

use ss2_core::math::Vec2;

use crate::jitter_for_frame;
use crate::quality::AntiAliasing;

/// The MSAA sample counts a device supports for the colour and depth formats in use.
///
/// One sample (no multisampling) is always supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MsaaSupport(u8);

impl MsaaSupport {
    /// No multisampling at all.
    pub const NONE: Self = Self(0);

    /// From the counts the device reports. Only 2, 4, 8 and 16 mean anything; other values are ignored.
    pub fn from_counts(counts: &[u32]) -> Self {
        let mut bits = 0u8;
        for c in counts {
            match c {
                2 => bits |= 1,
                4 => bits |= 2,
                8 => bits |= 4,
                16 => bits |= 8,
                _ => {}
            }
        }
        Self(bits)
    }

    /// Whether `count` samples are supported (1 always is).
    pub fn supports(self, count: u32) -> bool {
        match count {
            1 => true,
            2 => self.0 & 1 != 0,
            4 => self.0 & 2 != 0,
            8 => self.0 & 4 != 0,
            16 => self.0 & 8 != 0,
            _ => false,
        }
    }

    /// The largest supported count that is at most `wanted` (never below 1).
    pub fn best_at_most(self, wanted: u32) -> u32 {
        [16, 8, 4, 2]
            .into_iter()
            .find(|c| *c <= wanted && self.supports(*c))
            .unwrap_or(1)
    }
}

/// What the renderer should do about aliasing this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AaPlan {
    /// The sample count for the colour and depth targets: 1 means no multisampling.
    pub msaa_samples: u32,
    /// Run an FXAA pass after the scene.
    pub fxaa: bool,
    /// Use temporal anti-aliasing (jitter the projection, blend with the history).
    pub taa: bool,
    /// The setting asked for something the device cannot do, and this is what it got instead.
    pub degraded: bool,
}

impl AaPlan {
    /// Whether the scene is drawn multisampled (and so needs a resolve).
    pub fn multisampled(&self) -> bool {
        self.msaa_samples > 1
    }
}

/// Choose the method.
///
/// * `temporal`: the player (or the preset) wants TAA. It **replaces** MSAA and FXAA, because the three fighting each other costs a lot and looks worse.
/// * [`Off`](AntiAliasing::Off): nothing.
/// * [`Fxaa`](AntiAliasing::Fxaa): one sample, FXAA on.
/// * [`Msaa2`](AntiAliasing::Msaa2) / [`Msaa4`](AntiAliasing::Msaa4): that many samples if supported; otherwise the best the device has below it; and if it has
///   none, one sample **with FXAA** so the player still gets some smoothing. `degraded` is set when it did not get what it asked for.
///
/// ```
/// use ss2_render_api::anti_aliasing::{plan_anti_aliasing, MsaaSupport};
/// use ss2_render_api::quality::AntiAliasing;
///
/// let phone = MsaaSupport::from_counts(&[2, 4]);
/// let plan = plan_anti_aliasing(AntiAliasing::Msaa4, false, phone);
/// assert_eq!((plan.msaa_samples, plan.fxaa, plan.degraded), (4, false, false));
/// let weak = plan_anti_aliasing(AntiAliasing::Msaa4, false, MsaaSupport::NONE);
/// assert_eq!((weak.msaa_samples, weak.fxaa, weak.degraded), (1, true, true));
/// ```
pub fn plan_anti_aliasing(mode: AntiAliasing, temporal: bool, support: MsaaSupport) -> AaPlan {
    if temporal {
        return AaPlan {
            msaa_samples: 1,
            fxaa: false,
            taa: true,
            degraded: false,
        };
    }
    let wanted = match mode {
        AntiAliasing::Off => {
            return AaPlan {
                msaa_samples: 1,
                fxaa: false,
                taa: false,
                degraded: false,
            };
        }
        AntiAliasing::Fxaa => {
            return AaPlan {
                msaa_samples: 1,
                fxaa: true,
                taa: false,
                degraded: false,
            };
        }
        AntiAliasing::Msaa2 => 2,
        AntiAliasing::Msaa4 => 4,
    };
    let got = support.best_at_most(wanted);
    AaPlan {
        msaa_samples: got,
        fxaa: got == 1,
        taa: false,
        degraded: got != wanted,
    }
}

/// The TAA projection jitter for this frame, in normalized device coordinates: [`jitter_for_frame`] when the plan uses TAA, and **zero otherwise**, so a camera can
/// always add it without checking.
pub fn taa_jitter(plan: &AaPlan, frame: u32, width: u32, height: u32) -> Vec2 {
    if plan.taa {
        jitter_for_frame(frame, width, height)
    } else {
        Vec2::ZERO
    }
}

/// FXAA tuning (the usual parameters of the algorithm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxaaSettings {
    /// How much sub-pixel aliasing to remove, 0 (off) to 1 (soft). The usual default is 0.75.
    pub subpixel: f32,
    /// The least contrast an edge needs to be smoothed, as a share of the local maximum luma: 0.063 (visible) to 0.333 (fast). The usual default is 0.166.
    pub edge_threshold: f32,
    /// Edges darker than this are skipped, 0.0312 (visible) to 0.0833 (fast). The usual default is 0.0833.
    pub edge_threshold_min: f32,
}

impl Default for FxaaSettings {
    fn default() -> Self {
        Self {
            subpixel: 0.75,
            edge_threshold: 0.166,
            edge_threshold_min: 0.0833,
        }
    }
}

impl FxaaSettings {
    /// Whether the values are in the useful ranges above.
    pub fn validate(&self) -> Result<(), &'static str> {
        let ok = |v: f32, lo: f32, hi: f32| v.is_finite() && (lo..=hi).contains(&v);
        if !ok(self.subpixel, 0.0, 1.0) {
            return Err("subpixel must be 0 to 1");
        }
        if !ok(self.edge_threshold, 0.063, 0.333) {
            return Err("edge_threshold must be 0.063 to 0.333");
        }
        if !ok(self.edge_threshold_min, 0.0312, 0.0833) {
            return Err("edge_threshold_min must be 0.0312 to 0.0833");
        }
        Ok(())
    }
}

/// The size and memory of the render targets for a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderTargetPlan {
    /// Width of the scene targets in pixels (the window width times the render scale, at least 1).
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Samples per pixel.
    pub samples: u32,
    /// Bytes of the (possibly multisampled) colour target.
    pub color_bytes: u64,
    /// Bytes of the single-sample target an MSAA resolve writes to (0 without MSAA).
    pub resolve_bytes: u64,
    /// Bytes of the TAA history buffer (0 without TAA).
    pub history_bytes: u64,
}

impl RenderTargetPlan {
    /// Everything together.
    pub fn total_bytes(&self) -> u64 {
        self.color_bytes
            .saturating_add(self.resolve_bytes)
            .saturating_add(self.history_bytes)
    }
}

/// Size the scene targets: the window times `render_scale` (clamped to 0.25 to 1, NaN counts as 1), and the memory the plan needs for a colour format of
/// `bytes_per_pixel`. Depth is not counted: it is multisampled too and a renderer adds it with its own format.
pub fn render_target_plan(
    plan: &AaPlan,
    window_width: u32,
    window_height: u32,
    render_scale: f32,
    bytes_per_pixel: u32,
) -> RenderTargetPlan {
    let scale = if render_scale.is_nan() {
        1.0
    } else {
        render_scale.clamp(0.25, 1.0)
    };
    let dim = |n: u32| ((n as f32 * scale).round() as u32).max(1);
    let (width, height) = (dim(window_width), dim(window_height));
    let single = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(u64::from(bytes_per_pixel));
    RenderTargetPlan {
        width,
        height,
        samples: plan.msaa_samples,
        color_bytes: single.saturating_mul(u64::from(plan.msaa_samples)),
        resolve_bytes: if plan.multisampled() { single } else { 0 },
        history_bytes: if plan.taa { single } else { 0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(mode: AntiAliasing, support: &[u32]) -> AaPlan {
        plan_anti_aliasing(mode, false, MsaaSupport::from_counts(support))
    }

    // ---- support ------------------------------------------------------------------------------------

    #[test]
    fn support_reads_the_reported_counts_and_always_has_one_sample() {
        let s = MsaaSupport::from_counts(&[4, 2, 8, 3, 0, 1, 32, 4]);
        assert!(s.supports(1) && s.supports(2) && s.supports(4) && s.supports(8));
        assert!(!s.supports(16) && !s.supports(3) && !s.supports(0) && !s.supports(32));
        assert!(MsaaSupport::NONE.supports(1) && !MsaaSupport::NONE.supports(2));
        assert_eq!(MsaaSupport::from_counts(&[]), MsaaSupport::NONE);
        assert!(MsaaSupport::from_counts(&[16]).supports(16));
    }

    #[test]
    fn the_best_count_is_the_largest_supported_not_above_the_wish() {
        let s = MsaaSupport::from_counts(&[2, 8]);
        assert_eq!(s.best_at_most(16), 8);
        assert_eq!(s.best_at_most(8), 8);
        assert_eq!(s.best_at_most(4), 2, "4 is not supported: fall to 2");
        assert_eq!(s.best_at_most(2), 2);
        assert_eq!(s.best_at_most(1), 1);
        assert_eq!(s.best_at_most(0), 1);
        assert_eq!(MsaaSupport::NONE.best_at_most(4), 1);
        assert_eq!(MsaaSupport::from_counts(&[16]).best_at_most(u32::MAX), 16);
    }

    // ---- the plan -------------------------------------------------------------------------------------

    #[test]
    fn off_and_fxaa_use_one_sample() {
        let all = [2, 4, 8, 16];
        assert_eq!(
            plan(AntiAliasing::Off, &all),
            AaPlan {
                msaa_samples: 1,
                fxaa: false,
                taa: false,
                degraded: false
            }
        );
        assert_eq!(
            plan(AntiAliasing::Fxaa, &all),
            AaPlan {
                msaa_samples: 1,
                fxaa: true,
                taa: false,
                degraded: false
            }
        );
        assert!(!plan(AntiAliasing::Off, &all).multisampled());
    }

    #[test]
    fn msaa_gets_what_it_asks_for_when_the_device_has_it() {
        let all = [2, 4, 8];
        let two = plan(AntiAliasing::Msaa2, &all);
        assert_eq!(
            two,
            AaPlan {
                msaa_samples: 2,
                fxaa: false,
                taa: false,
                degraded: false
            }
        );
        let four = plan(AntiAliasing::Msaa4, &all);
        assert_eq!(
            four,
            AaPlan {
                msaa_samples: 4,
                fxaa: false,
                taa: false,
                degraded: false
            }
        );
        assert!(four.multisampled());
    }

    #[test]
    fn msaa_the_device_lacks_degrades_to_the_best_it_has_and_then_to_fxaa() {
        let only_two = plan(AntiAliasing::Msaa4, &[2]);
        assert_eq!(
            only_two,
            AaPlan {
                msaa_samples: 2,
                fxaa: false,
                taa: false,
                degraded: true
            }
        );
        let none = plan(AntiAliasing::Msaa4, &[]);
        assert_eq!(
            none,
            AaPlan {
                msaa_samples: 1,
                fxaa: true,
                taa: false,
                degraded: true
            },
            "no MSAA: FXAA instead, and say so"
        );
        let none2 = plan(AntiAliasing::Msaa2, &[]);
        assert_eq!(
            (none2.msaa_samples, none2.fxaa, none2.degraded),
            (1, true, true)
        );
        // Only 4 supported and 2 asked for: 2 is not available, so 1 and FXAA (it does not round up).
        let four_only = plan(AntiAliasing::Msaa2, &[4]);
        assert_eq!(
            (four_only.msaa_samples, four_only.fxaa, four_only.degraded),
            (1, true, true)
        );
    }

    #[test]
    fn temporal_replaces_the_others_and_is_never_degraded() {
        for mode in [
            AntiAliasing::Off,
            AntiAliasing::Fxaa,
            AntiAliasing::Msaa2,
            AntiAliasing::Msaa4,
        ] {
            let p = plan_anti_aliasing(mode, true, MsaaSupport::from_counts(&[2, 4, 8]));
            assert_eq!(
                p,
                AaPlan {
                    msaa_samples: 1,
                    fxaa: false,
                    taa: true,
                    degraded: false
                },
                "{mode:?}"
            );
        }
    }

    #[test]
    fn the_quality_presets_plan_sensibly_on_a_full_and_an_empty_device() {
        use crate::quality::{QualitySettings, QualityTier};
        let full = MsaaSupport::from_counts(&[2, 4, 8, 16]);
        let expect_full = [
            (QualityTier::Low, 1, false),
            (QualityTier::Medium, 1, true),
            (QualityTier::High, 2, false),
            (QualityTier::Ultra, 4, false),
        ];
        for (tier, samples, fxaa) in expect_full {
            let p = plan_anti_aliasing(QualitySettings::preset(tier).anti_aliasing, false, full);
            assert_eq!(
                (p.msaa_samples, p.fxaa, p.degraded),
                (samples, fxaa, false),
                "{tier:?}"
            );
        }
        // On a device with no MSAA the high presets still get something smoothing.
        for tier in [QualityTier::High, QualityTier::Ultra] {
            let p = plan_anti_aliasing(
                QualitySettings::preset(tier).anti_aliasing,
                false,
                MsaaSupport::NONE,
            );
            assert!(p.fxaa && p.degraded && p.msaa_samples == 1, "{tier:?}");
        }
    }

    // ---- jitter -----------------------------------------------------------------------------------------

    #[test]
    fn only_a_temporal_plan_jitters_and_it_matches_the_camera_jitter() {
        let taa = plan_anti_aliasing(AntiAliasing::Off, true, MsaaSupport::NONE);
        let off = plan(AntiAliasing::Msaa4, &[4]);
        for frame in 0..20 {
            assert_eq!(taa_jitter(&off, frame, 1280, 720), Vec2::ZERO);
            assert_eq!(
                taa_jitter(&taa, frame, 1280, 720),
                jitter_for_frame(frame, 1280, 720)
            );
        }
        assert_ne!(taa_jitter(&taa, 1, 1280, 720), Vec2::ZERO);
        assert_ne!(
            taa_jitter(&taa, 1, 1280, 720),
            taa_jitter(&taa, 2, 1280, 720),
            "it moves from frame to frame"
        );
        assert_eq!(
            taa_jitter(&taa, 1, 0, 720),
            Vec2::ZERO,
            "an empty viewport has none"
        );
    }

    // ---- FXAA settings -----------------------------------------------------------------------------------

    #[test]
    fn fxaa_defaults_are_valid_and_out_of_range_values_are_refused() {
        assert!(FxaaSettings::default().validate().is_ok());
        let d = FxaaSettings::default();
        for bad in [
            FxaaSettings {
                subpixel: -0.1,
                ..d
            },
            FxaaSettings { subpixel: 1.1, ..d },
            FxaaSettings {
                subpixel: f32::NAN,
                ..d
            },
            FxaaSettings {
                edge_threshold: 0.06,
                ..d
            },
            FxaaSettings {
                edge_threshold: 0.34,
                ..d
            },
            FxaaSettings {
                edge_threshold_min: 0.03,
                ..d
            },
            FxaaSettings {
                edge_threshold_min: 0.09,
                ..d
            },
            FxaaSettings {
                edge_threshold_min: f32::INFINITY,
                ..d
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        for edge in [
            FxaaSettings { subpixel: 0.0, ..d },
            FxaaSettings { subpixel: 1.0, ..d },
            FxaaSettings {
                edge_threshold: 0.063,
                ..d
            },
            FxaaSettings {
                edge_threshold: 0.333,
                ..d
            },
            FxaaSettings {
                edge_threshold_min: 0.0312,
                ..d
            },
            FxaaSettings {
                edge_threshold_min: 0.0833,
                ..d
            },
        ] {
            assert!(edge.validate().is_ok(), "{edge:?}");
        }
    }

    // ---- memory ---------------------------------------------------------------------------------------------

    #[test]
    fn msaa_multiplies_the_colour_memory_and_adds_a_resolve_target() {
        let single = plan(AntiAliasing::Off, &[]);
        let rt1 = render_target_plan(&single, 1920, 1080, 1.0, 8);
        assert_eq!((rt1.width, rt1.height, rt1.samples), (1920, 1080, 1));
        assert_eq!(rt1.color_bytes, 1920 * 1080 * 8);
        assert_eq!((rt1.resolve_bytes, rt1.history_bytes), (0, 0));
        assert_eq!(rt1.total_bytes(), 1920 * 1080 * 8);
        let msaa4 = plan(AntiAliasing::Msaa4, &[2, 4]);
        let rt4 = render_target_plan(&msaa4, 1920, 1080, 1.0, 8);
        assert_eq!(rt4.color_bytes, 4 * 1920 * 1080 * 8);
        assert_eq!(rt4.resolve_bytes, 1920 * 1080 * 8);
        assert_eq!(rt4.total_bytes(), 5 * 1920 * 1080 * 8);
        let taa = plan_anti_aliasing(AntiAliasing::Off, true, MsaaSupport::NONE);
        let rt_t = render_target_plan(&taa, 1920, 1080, 1.0, 8);
        assert_eq!(
            (rt_t.color_bytes, rt_t.resolve_bytes, rt_t.history_bytes),
            (1920 * 1080 * 8, 0, 1920 * 1080 * 8)
        );
    }

    #[test]
    fn the_render_scale_shrinks_the_targets_and_is_clamped() {
        let p = plan(AntiAliasing::Off, &[]);
        let half = render_target_plan(&p, 1920, 1080, 0.5, 4);
        assert_eq!((half.width, half.height), (960, 540));
        assert_eq!(half.color_bytes, 960 * 540 * 4);
        let odd = render_target_plan(&p, 1001, 563, 0.7, 4);
        assert_eq!(
            (odd.width, odd.height),
            (701, 394),
            "rounded to the nearest pixel"
        );
        assert_eq!(
            render_target_plan(&p, 1920, 1080, 0.01, 4).width,
            480,
            "clamped at a quarter"
        );
        assert_eq!(
            render_target_plan(&p, 1920, 1080, 5.0, 4).width,
            1920,
            "never larger than the window"
        );
        assert_eq!(render_target_plan(&p, 1920, 1080, f32::NAN, 4).width, 1920);
        let tiny = render_target_plan(&p, 1, 1, 0.25, 4);
        assert_eq!((tiny.width, tiny.height), (1, 1), "never zero");
        let empty = render_target_plan(&p, 0, 0, 1.0, 4);
        assert_eq!((empty.width, empty.height), (1, 1));
    }

    #[test]
    fn huge_targets_saturate_instead_of_overflowing() {
        let p = plan(AntiAliasing::Msaa4, &[4]);
        let rt = render_target_plan(&p, u32::MAX, u32::MAX, 1.0, u32::MAX);
        assert_eq!(rt.color_bytes, u64::MAX);
        assert_eq!(rt.total_bytes(), u64::MAX);
    }
}
