//! [`LifecycleTracker`]: folds [`LifecycleEvent`]s into the one question the app keeps asking,
//! "may I render and present right now?".

use crate::event::{LifecycleEvent, SurfaceInfo};

/// Tracks suspend / surface / focus state from the stream of [`LifecycleEvent`]s.
///
/// ## Ordering the platform guarantees (and the tracker tolerates violations of)
///
/// 1. [`Resumed`](LifecycleEvent::Resumed) comes first, then
///    [`SurfaceReady`](LifecycleEvent::SurfaceReady).
/// 2. [`Suspended`](LifecycleEvent::Suspended) is always preceded by
///    [`SurfaceLost`](LifecycleEvent::SurfaceLost); after the next `Resumed` a new
///    `SurfaceReady` arrives before drawing is possible again.
/// 3. `Resized` and `ScaleFactorChanged` only occur while a surface exists.
/// 4. Desktop platforms never send `Suspended`/`SurfaceLost` (minimizing sends `Resized` to
///    `0 x 0`); Android sends the full cycle on every background/foreground switch.
///
/// The tracker never panics and never trusts the guarantees: `Suspended` also drops the surface,
/// and size events without a surface are ignored.
///
/// ```
/// use ss2_platform_api::{LifecycleEvent as E, LifecycleTracker, SurfaceInfo};
///
/// let mut life = LifecycleTracker::new();
/// assert!(!life.can_render());                       // nothing happened yet
/// life.apply(&E::Resumed);
/// life.apply(&E::SurfaceReady(SurfaceInfo { width: 1280, height: 720, scale_factor: 1.0 }));
/// assert!(life.can_render());
/// life.apply(&E::SurfaceLost);
/// life.apply(&E::Suspended);
/// assert!(!life.can_render());                       // free GPU resources now
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleTracker {
    suspended: bool,
    surface: Option<SurfaceInfo>,
    focused: bool,
    close_requested: bool,
    low_memory: bool,
}

impl Default for LifecycleTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleTracker {
    /// A fresh tracker: suspended (nothing may render until `Resumed` and `SurfaceReady`),
    /// focused, no surface.
    pub fn new() -> Self {
        Self {
            suspended: true,
            surface: None,
            focused: true,
            close_requested: false,
            low_memory: false,
        }
    }

    /// Fold one event into the state.
    pub fn apply(&mut self, event: &LifecycleEvent) {
        match *event {
            LifecycleEvent::Resumed => self.suspended = false,
            LifecycleEvent::Suspended => {
                self.suspended = true;
                self.surface = None;
            }
            LifecycleEvent::SurfaceReady(info) => self.surface = Some(info),
            LifecycleEvent::SurfaceLost => self.surface = None,
            LifecycleEvent::Resized { width, height } => {
                if let Some(s) = &mut self.surface {
                    s.width = width;
                    s.height = height;
                }
            }
            LifecycleEvent::ScaleFactorChanged { scale_factor } => {
                if let Some(s) = &mut self.surface {
                    s.scale_factor = scale_factor;
                }
            }
            LifecycleEvent::FocusChanged(focused) => self.focused = focused,
            LifecycleEvent::LowMemory => self.low_memory = true,
            LifecycleEvent::CloseRequested => self.close_requested = true,
        }
    }

    /// `true` when the app is running, has a surface, and the surface is not minimized
    /// (non-zero size). Rendering and presenting are only legal then.
    pub fn can_render(&self) -> bool {
        !self.suspended && self.surface.is_some_and(|s| s.width > 0 && s.height > 0)
    }

    /// `true` between `Suspended` and the next `Resumed` (and before the first `Resumed`).
    pub fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// The current surface, if any.
    pub fn surface(&self) -> Option<SurfaceInfo> {
        self.surface
    }

    /// Width / height of the surface, if it exists and has non-zero size.
    pub fn aspect(&self) -> Option<f32> {
        self.surface
            .filter(|s| s.width > 0 && s.height > 0)
            .map(|s| s.width as f32 / s.height as f32)
    }

    /// Whether the window has keyboard focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// `true` once the user asked to close; stays set.
    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// `true` once per `LowMemory` event: reading it clears it, so the app frees caches once.
    pub fn take_low_memory(&mut self) -> bool {
        std::mem::take(&mut self.low_memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(w: u32, h: u32) -> LifecycleEvent {
        LifecycleEvent::SurfaceReady(SurfaceInfo {
            width: w,
            height: h,
            scale_factor: 1.0,
        })
    }

    fn run(events: &[LifecycleEvent]) -> LifecycleTracker {
        let mut t = LifecycleTracker::new();
        for e in events {
            t.apply(e);
        }
        t
    }

    #[test]
    fn desktop_startup_and_close() {
        let mut t = run(&[LifecycleEvent::Resumed, ready(1280, 720)]);
        assert!(t.can_render());
        assert_eq!(t.aspect(), Some(1280.0 / 720.0));
        t.apply(&LifecycleEvent::CloseRequested);
        assert!(t.close_requested());
        assert!(
            t.can_render(),
            "closing is the app's decision, not a state change"
        );
    }

    #[test]
    fn needs_both_resume_and_surface() {
        assert!(!run(&[LifecycleEvent::Resumed]).can_render());
        assert!(
            !run(&[ready(10, 10)]).can_render(),
            "surface before Resumed"
        );
    }

    #[test]
    fn android_background_foreground_cycle() {
        let mut t = run(&[LifecycleEvent::Resumed, ready(1920, 1080)]);
        t.apply(&LifecycleEvent::SurfaceLost);
        t.apply(&LifecycleEvent::Suspended);
        assert!(t.is_suspended() && t.surface().is_none() && !t.can_render());
        t.apply(&LifecycleEvent::Resumed);
        assert!(!t.can_render(), "still no surface after Resumed");
        t.apply(&ready(1080, 1920));
        assert!(t.can_render());
        assert_eq!(t.surface().unwrap().width, 1080, "new size after rotation");
    }

    #[test]
    fn minimize_is_zero_size_not_suspend() {
        let mut t = run(&[LifecycleEvent::Resumed, ready(800, 600)]);
        t.apply(&LifecycleEvent::Resized {
            width: 0,
            height: 0,
        });
        assert!(!t.can_render() && t.aspect().is_none() && !t.is_suspended());
        t.apply(&LifecycleEvent::Resized {
            width: 800,
            height: 600,
        });
        assert!(t.can_render());
    }

    #[test]
    fn tolerates_missing_surface_lost_before_suspended() {
        let mut t = run(&[LifecycleEvent::Resumed, ready(100, 100)]);
        t.apply(&LifecycleEvent::Suspended);
        assert!(t.surface().is_none());
    }

    #[test]
    fn size_events_without_surface_are_ignored() {
        let mut t = run(&[LifecycleEvent::Resumed]);
        t.apply(&LifecycleEvent::Resized {
            width: 5,
            height: 5,
        });
        t.apply(&LifecycleEvent::ScaleFactorChanged { scale_factor: 2.0 });
        assert!(t.surface().is_none());
    }

    #[test]
    fn scale_factor_and_focus_track() {
        let mut t = run(&[LifecycleEvent::Resumed, ready(100, 100)]);
        t.apply(&LifecycleEvent::ScaleFactorChanged { scale_factor: 1.5 });
        assert_eq!(t.surface().unwrap().scale_factor, 1.5);
        assert!(t.is_focused());
        t.apply(&LifecycleEvent::FocusChanged(false));
        assert!(!t.is_focused());
    }

    #[test]
    fn low_memory_is_reported_once() {
        let mut t = LifecycleTracker::new();
        assert!(!t.take_low_memory());
        t.apply(&LifecycleEvent::LowMemory);
        assert!(t.take_low_memory());
        assert!(!t.take_low_memory());
    }
}
