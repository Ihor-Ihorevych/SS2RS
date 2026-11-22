//! The `winit` side of display modes (T03-017): list the monitors and their video modes as the
//! plain types of [`ss2_platform_api`], and turn a [`DisplayPlan`] into a `winit` [`Fullscreen`].
//! The choosing itself is pure and tested in `ss2_platform_api::display`.

use ss2_platform_api::{
    DisplayPlan, DisplayRequest, MonitorInfo, ResolvedMode, VideoMode, plan_display,
};
use winit::monitor::{MonitorHandle, VideoModeHandle};
use winit::window::Fullscreen;

use crate::translate;

/// A monitor with the `winit` handles needed to act on a plan.
struct Snapshot {
    info: MonitorInfo,
    handle: MonitorHandle,
    /// `video_modes[i]` is the handle of `info.modes[i]`.
    video_modes: Vec<VideoModeHandle>,
}

fn video_mode(mode: &VideoModeHandle) -> VideoMode {
    let size = mode.size();
    VideoMode {
        width: size.width,
        height: size.height,
        refresh_millihertz: mode.refresh_rate_millihertz(),
        bit_depth: mode.bit_depth(),
    }
}

fn snapshot(
    monitors: impl IntoIterator<Item = MonitorHandle>,
    primary: Option<&MonitorHandle>,
) -> Vec<Snapshot> {
    monitors
        .into_iter()
        .map(|handle| {
            let video_modes: Vec<VideoModeHandle> = handle.video_modes().collect();
            let size = handle.size();
            let position = handle.position();
            let info = MonitorInfo {
                name: handle.name().unwrap_or_default(),
                size: (size.width, size.height),
                position: (position.x, position.y),
                scale_factor: translate::sanitize_scale(handle.scale_factor()),
                primary: primary == Some(&handle),
                modes: video_modes.iter().map(video_mode).collect(),
            };
            Snapshot {
                info,
                handle,
                video_modes,
            }
        })
        .collect()
}

/// The monitors as plain data (for the options screen: monitor names and resolution lists).
pub fn list_monitors(
    monitors: impl IntoIterator<Item = MonitorHandle>,
    primary: Option<&MonitorHandle>,
) -> Vec<MonitorInfo> {
    snapshot(monitors, primary)
        .into_iter()
        .map(|s| s.info)
        .collect()
}

/// Plan `request` for the given monitors and return the `winit` fullscreen setting for the
/// plan (`None` = a normal window) together with the plan itself (log its `notes`).
pub fn resolve_fullscreen(
    monitors: impl IntoIterator<Item = MonitorHandle>,
    primary: Option<&MonitorHandle>,
    request: &DisplayRequest,
) -> (Option<Fullscreen>, DisplayPlan) {
    let snapshots = snapshot(monitors, primary);
    let infos: Vec<MonitorInfo> = snapshots.iter().map(|s| s.info.clone()).collect();
    let plan = plan_display(request, &infos);
    let chosen = plan.monitor.and_then(|i| snapshots.get(i));
    let borderless = || Fullscreen::Borderless(chosen.map(|s| s.handle.clone()));
    let fullscreen = match plan.mode {
        ResolvedMode::Windowed => None,
        ResolvedMode::Borderless => Some(borderless()),
        ResolvedMode::Exclusive(wanted) => Some(
            chosen
                .and_then(|s| {
                    let index = s.info.modes.iter().position(|m| *m == wanted)?;
                    s.video_modes.get(index).cloned().map(Fullscreen::Exclusive)
                })
                .unwrap_or_else(borderless),
        ),
    };
    (fullscreen, plan)
}
