//! Display modes: window mode, monitor and resolution choice, HiDPI scale (T03-017).
//!
//! Everything here is a pure function of plain data, so it is tested without a window: the
//! window layer lists the monitors and their video modes ([`MonitorInfo`], [`VideoMode`]), the
//! user's wish is a [`DisplayRequest`], and [`plan_display`] turns the two into a [`DisplayPlan`]:
//! which monitor, and windowed, borderless or exclusive fullscreen with a concrete video mode.
//! Wishes that cannot be met fall back (an unknown monitor to the primary one, exclusive
//! fullscreen without video modes to borderless) and say so in [`DisplayPlan::notes`] so the app
//! can log it. [`effective_ui_scale`] is the HiDPI policy for the UI.

use serde::{Deserialize, Serialize};

/// How the game window is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowMode {
    /// A normal window with a title bar.
    #[default]
    Windowed,
    /// A borderless window covering the monitor at its desktop resolution (alt-tab friendly).
    Borderless,
    /// Exclusive fullscreen with a chosen resolution and refresh rate.
    Exclusive,
}

/// One resolution, refresh rate and colour depth a monitor can be switched to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VideoMode {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Refresh rate in millihertz (59 940 = 59.94 Hz).
    pub refresh_millihertz: u32,
    /// Bits per pixel.
    pub bit_depth: u16,
}

impl VideoMode {
    /// The refresh rate in whole hertz, rounded.
    pub fn refresh_hz(&self) -> u32 {
        (self.refresh_millihertz + 500) / 1000
    }

    fn area(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// A connected monitor, as the window layer reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorInfo {
    /// The OS name of the monitor.
    pub name: String,
    /// Current (desktop) size in physical pixels.
    pub size: (u32, u32),
    /// Position of its top-left corner on the virtual desktop.
    pub position: (i32, i32),
    /// Physical pixels per logical pixel (HiDPI).
    pub scale_factor: f32,
    /// Whether it is the primary monitor.
    pub primary: bool,
    /// The video modes it supports (may be empty where the platform reports none).
    pub modes: Vec<VideoMode>,
}

/// What the user asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayRequest {
    /// Windowed, borderless or exclusive fullscreen.
    pub mode: WindowMode,
    /// The monitor by name (the primary one when `None` or unknown).
    pub monitor: Option<String>,
    /// The resolution for exclusive fullscreen (the largest available when `None`).
    pub resolution: Option<(u32, u32)>,
    /// The refresh rate for exclusive fullscreen in hertz (the highest available when `None`).
    pub refresh_hz: Option<u32>,
}

/// The mode the window will really use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedMode {
    /// A normal window.
    Windowed,
    /// Borderless fullscreen at the desktop resolution.
    Borderless,
    /// Exclusive fullscreen with exactly this video mode.
    Exclusive(VideoMode),
}

/// The outcome of [`plan_display`].
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayPlan {
    /// Index into the monitor list that was planned for (`None` if there are no monitors).
    pub monitor: Option<usize>,
    /// The mode to use.
    pub mode: ResolvedMode,
    /// Human-readable explanations of every fallback taken (worth logging).
    pub notes: Vec<String>,
}

/// The resolutions to offer for `monitor` (at least `min` in both directions), largest first,
/// each once. A monitor without video modes offers its desktop size.
pub fn resolution_list(monitor: &MonitorInfo, min: (u32, u32)) -> Vec<(u32, u32)> {
    let mut sizes: Vec<(u32, u32)> = if monitor.modes.is_empty() {
        vec![monitor.size]
    } else {
        monitor.modes.iter().map(|m| (m.width, m.height)).collect()
    };
    sizes.retain(|&(w, h)| w >= min.0 && h >= min.1);
    sizes.sort_by_key(|&(w, h)| std::cmp::Reverse((u64::from(w) * u64::from(h), w)));
    sizes.dedup();
    sizes
}

/// The monitor to use: the one called `wanted` (case-insensitive), else the primary monitor,
/// else the first. `None` only when there are no monitors.
pub fn select_monitor(monitors: &[MonitorInfo], wanted: Option<&str>) -> Option<usize> {
    wanted
        .and_then(|name| {
            monitors
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(name))
        })
        .or_else(|| monitors.iter().position(|m| m.primary))
        .or_else(|| (!monitors.is_empty()).then_some(0))
}

/// The video mode that best matches a request. The size is the requested one, or the nearest
/// available one (by the sum of the width and height differences, larger on a tie), or the
/// largest when none is requested; among the modes of that size the refresh rate is the closest
/// to the requested one, or the highest, and the colour depth the deepest. `None` without modes.
pub fn best_video_mode(
    modes: &[VideoMode],
    resolution: Option<(u32, u32)>,
    refresh_hz: Option<u32>,
) -> Option<VideoMode> {
    let target = match resolution {
        None => modes.iter().max_by_key(|m| m.area())?,
        Some((w, h)) => modes.iter().min_by_key(|m| {
            (
                m.width.abs_diff(w) + m.height.abs_diff(h),
                std::cmp::Reverse(m.area()),
            )
        })?,
    };
    let size = (target.width, target.height);
    modes
        .iter()
        .filter(|m| (m.width, m.height) == size)
        .min_by_key(|m| {
            let distance = refresh_hz.map_or(0, |hz| m.refresh_millihertz.abs_diff(hz * 1000));
            (
                distance,
                std::cmp::Reverse(m.refresh_millihertz),
                std::cmp::Reverse(m.bit_depth),
            )
        })
        .copied()
}

/// Turn a wish into a plan for the connected monitors (see the module documentation).
pub fn plan_display(request: &DisplayRequest, monitors: &[MonitorInfo]) -> DisplayPlan {
    let mut notes = Vec::new();
    let monitor = select_monitor(monitors, request.monitor.as_deref());
    if let (Some(name), Some(index)) = (&request.monitor, monitor)
        && !monitors[index].name.eq_ignore_ascii_case(name)
    {
        notes.push(format!(
            "monitor {name:?} was not found; using {:?}",
            monitors[index].name
        ));
    }
    let mode = match request.mode {
        WindowMode::Windowed => ResolvedMode::Windowed,
        WindowMode::Borderless => ResolvedMode::Borderless,
        WindowMode::Exclusive => {
            let chosen = monitor.and_then(|i| {
                best_video_mode(&monitors[i].modes, request.resolution, request.refresh_hz)
            });
            match chosen {
                Some(video_mode) => ResolvedMode::Exclusive(video_mode),
                None => {
                    notes.push(
                        "exclusive fullscreen is not available on this monitor; using borderless fullscreen"
                            .to_owned(),
                    );
                    ResolvedMode::Borderless
                }
            }
        }
    };
    DisplayPlan {
        monitor,
        mode,
        notes,
    }
}

/// How the UI scale is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiScale {
    /// Follow the monitor's HiDPI scale factor.
    Auto,
    /// A fixed factor chosen by the user.
    Fixed(f32),
}

/// The UI scale to use: the fixed factor, or the OS scale factor for [`UiScale::Auto`] (also
/// when the fixed factor is not a usable number), always finite and within `0.5..=4.0`.
pub fn effective_ui_scale(policy: UiScale, os_scale: f32) -> f32 {
    let usable = |v: f32| v.is_finite() && v > 0.0;
    let wanted = match policy {
        UiScale::Fixed(v) if usable(v) => v,
        _ if usable(os_scale) => os_scale,
        _ => 1.0,
    };
    wanted.clamp(0.5, 4.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(width: u32, height: u32, hz: u32, depth: u16) -> VideoMode {
        VideoMode {
            width,
            height,
            refresh_millihertz: hz * 1000,
            bit_depth: depth,
        }
    }

    fn monitor(name: &str, primary: bool, modes: Vec<VideoMode>) -> MonitorInfo {
        let size = modes
            .iter()
            .map(|m| (m.width, m.height))
            .max_by_key(|(w, h)| w * h)
            .unwrap_or((1920, 1080));
        MonitorInfo {
            name: name.to_owned(),
            size,
            position: (0, 0),
            scale_factor: 1.0,
            primary,
            modes,
        }
    }

    fn typical_modes() -> Vec<VideoMode> {
        vec![
            mode(1920, 1080, 60, 32),
            mode(1920, 1080, 144, 32),
            mode(1920, 1080, 144, 24),
            mode(1280, 720, 60, 32),
            mode(1280, 720, 120, 32),
            mode(800, 600, 60, 32),
            mode(640, 480, 60, 16),
        ]
    }

    #[test]
    fn resolutions_are_unique_big_first_and_respect_the_minimum() {
        let m = monitor("A", true, typical_modes());
        assert_eq!(
            resolution_list(&m, (800, 600)),
            [(1920, 1080), (1280, 720), (800, 600)]
        );
        assert_eq!(resolution_list(&m, (1, 1)).len(), 4);
    }

    #[test]
    fn a_monitor_without_modes_offers_its_own_size() {
        let mut m = monitor("A", true, vec![]);
        m.size = (2560, 1440);
        assert_eq!(resolution_list(&m, (800, 600)), [(2560, 1440)]);
        m.size = (320, 200);
        assert!(resolution_list(&m, (800, 600)).is_empty());
    }

    #[test]
    fn a_monitor_is_chosen_by_name_then_primary_then_first() {
        let monitors = [
            monitor("Left", false, vec![]),
            monitor("Main", true, vec![]),
            monitor("Right", false, vec![]),
        ];
        assert_eq!(
            select_monitor(&monitors, Some("right")),
            Some(2),
            "case-insensitive"
        );
        assert_eq!(
            select_monitor(&monitors, Some("Gone")),
            Some(1),
            "falls back to primary"
        );
        assert_eq!(select_monitor(&monitors, None), Some(1));
        let no_primary = [monitor("A", false, vec![]), monitor("B", false, vec![])];
        assert_eq!(select_monitor(&no_primary, None), Some(0));
        assert_eq!(select_monitor(&[], Some("A")), None);
    }

    #[test]
    fn the_best_mode_for_an_exact_size_has_the_highest_refresh_and_depth() {
        let best = best_video_mode(&typical_modes(), Some((1920, 1080)), None).unwrap();
        assert_eq!(best, mode(1920, 1080, 144, 32));
    }

    #[test]
    fn a_requested_refresh_rate_picks_the_closest_one() {
        let modes = typical_modes();
        assert_eq!(
            best_video_mode(&modes, Some((1280, 720)), Some(60)).unwrap(),
            mode(1280, 720, 60, 32)
        );
        assert_eq!(
            best_video_mode(&modes, Some((1280, 720)), Some(100)).unwrap(),
            mode(1280, 720, 120, 32),
            "120 is closer to 100 than 60 is"
        );
    }

    #[test]
    fn an_unavailable_size_falls_back_to_the_nearest_one() {
        let best = best_video_mode(&typical_modes(), Some((1366, 768)), None).unwrap();
        assert_eq!((best.width, best.height), (1280, 720));
        let best = best_video_mode(&typical_modes(), Some((4000, 3000)), None).unwrap();
        assert_eq!((best.width, best.height), (1920, 1080));
    }

    #[test]
    fn without_a_requested_size_the_largest_mode_wins_and_no_modes_means_none() {
        let best = best_video_mode(&typical_modes(), None, None).unwrap();
        assert_eq!(best, mode(1920, 1080, 144, 32));
        assert_eq!(best_video_mode(&[], Some((800, 600)), None), None);
    }

    #[test]
    fn windowed_and_borderless_need_no_video_mode() {
        let monitors = [monitor("A", true, typical_modes())];
        for requested in [WindowMode::Windowed, WindowMode::Borderless] {
            let plan = plan_display(
                &DisplayRequest {
                    mode: requested,
                    ..DisplayRequest::default()
                },
                &monitors,
            );
            let expected = match requested {
                WindowMode::Windowed => ResolvedMode::Windowed,
                _ => ResolvedMode::Borderless,
            };
            assert_eq!(plan.mode, expected);
            assert_eq!(plan.monitor, Some(0));
            assert!(plan.notes.is_empty());
        }
    }

    #[test]
    fn exclusive_fullscreen_resolves_to_a_concrete_video_mode() {
        let monitors = [monitor("A", true, typical_modes())];
        let plan = plan_display(
            &DisplayRequest {
                mode: WindowMode::Exclusive,
                resolution: Some((1280, 720)),
                refresh_hz: Some(120),
                ..DisplayRequest::default()
            },
            &monitors,
        );
        assert_eq!(plan.mode, ResolvedMode::Exclusive(mode(1280, 720, 120, 32)));
        assert!(plan.notes.is_empty());
    }

    #[test]
    fn exclusive_fullscreen_without_modes_downgrades_to_borderless_and_says_so() {
        let monitors = [monitor("A", true, vec![])];
        let plan = plan_display(
            &DisplayRequest {
                mode: WindowMode::Exclusive,
                ..DisplayRequest::default()
            },
            &monitors,
        );
        assert_eq!(plan.mode, ResolvedMode::Borderless);
        assert!(
            plan.notes.iter().any(|n| n.contains("exclusive")),
            "{:?}",
            plan.notes
        );
    }

    #[test]
    fn an_unknown_monitor_name_is_reported_and_the_primary_is_used() {
        let monitors = [
            monitor("Side", false, typical_modes()),
            monitor("Main", true, typical_modes()),
        ];
        let plan = plan_display(
            &DisplayRequest {
                monitor: Some("Unplugged".into()),
                ..DisplayRequest::default()
            },
            &monitors,
        );
        assert_eq!(plan.monitor, Some(1));
        assert!(
            plan.notes.iter().any(|n| n.contains("Unplugged")),
            "{:?}",
            plan.notes
        );
    }

    #[test]
    fn no_monitors_still_gives_a_plan() {
        let plan = plan_display(&DisplayRequest::default(), &[]);
        assert_eq!(plan.monitor, None);
        assert_eq!(plan.mode, ResolvedMode::Windowed);
        let plan = plan_display(
            &DisplayRequest {
                mode: WindowMode::Exclusive,
                ..DisplayRequest::default()
            },
            &[],
        );
        assert_eq!(plan.mode, ResolvedMode::Borderless);
    }

    #[test]
    fn the_ui_scale_follows_the_os_unless_fixed_and_is_always_usable() {
        assert_eq!(effective_ui_scale(UiScale::Auto, 2.0), 2.0);
        assert_eq!(effective_ui_scale(UiScale::Auto, f32::NAN), 1.0);
        assert_eq!(effective_ui_scale(UiScale::Auto, 0.0), 1.0);
        assert_eq!(effective_ui_scale(UiScale::Auto, 9.0), 4.0, "clamped");
        assert_eq!(effective_ui_scale(UiScale::Fixed(1.5), 3.0), 1.5);
        assert_eq!(effective_ui_scale(UiScale::Fixed(0.1), 1.0), 0.5, "clamped");
        assert_eq!(effective_ui_scale(UiScale::Fixed(f32::INFINITY), 2.0), 2.0);
    }

    #[test]
    fn refresh_rates_are_shown_in_whole_hertz() {
        let m = VideoMode {
            width: 1,
            height: 1,
            refresh_millihertz: 59_940,
            bit_depth: 32,
        };
        assert_eq!(m.refresh_hz(), 60);
        assert_eq!(mode(1, 1, 144, 32).refresh_hz(), 144);
    }

    #[test]
    fn window_modes_survive_toml() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Wrapper {
            mode: WindowMode,
        }
        for mode in [
            WindowMode::Windowed,
            WindowMode::Borderless,
            WindowMode::Exclusive,
        ] {
            let text = toml::to_string(&Wrapper { mode }).unwrap();
            assert_eq!(
                toml::from_str::<Wrapper>(&text).unwrap().mode,
                mode,
                "{text}"
            );
        }
        assert_eq!(WindowMode::default(), WindowMode::Windowed);
    }
}
