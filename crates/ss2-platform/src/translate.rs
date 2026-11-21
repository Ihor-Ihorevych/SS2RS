//! Pure translation from what `winit` reports about a window to the lifecycle events of the
//! platform contract (`ss2-platform-api`, RFC 0003).
//!
//! Kept free of `winit` types on purpose: `winit` cannot construct several of its events outside a
//! running event loop, so the loop in `window.rs` reduces each event to an [`Observed`] value and
//! everything that can be wrong about the translation is tested here without a display.

use ss2_platform_api::{LifecycleEvent, SurfaceInfo};

/// The part of a `winit` window event that matters for the lifecycle contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Observed {
    /// The drawable size changed (physical pixels).
    Resized {
        /// New width.
        width: u32,
        /// New height.
        height: u32,
    },
    /// The HiDPI factor changed (window moved to another monitor, or the user changed scaling).
    ScaleFactor(f64),
    /// Keyboard focus gained or lost.
    Focused(bool),
    /// The user asked to close the window.
    CloseRequested,
    /// The window no longer exists.
    Destroyed,
}

/// Drops `Resized` events that do not change the size. Windows reports the same size several
/// times (after DPI changes, on focus, around fullscreen switches); a renderer should not
/// reconfigure its swapchain for each.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResizeFilter {
    last: Option<(u32, u32)>,
}

impl ResizeFilter {
    /// Remember `size` as the current one (call when the surface becomes ready, so the first real
    /// resize is compared against the size the app was told about).
    pub fn reset(&mut self, width: u32, height: u32) {
        self.last = Some((width, height));
    }

    /// `true` if this is a real change that should be delivered. A size seen for the first time
    /// (nothing recorded yet) always passes.
    pub fn accept(&mut self, width: u32, height: u32) -> bool {
        let changed = self.last != Some((width, height));
        self.last = Some((width, height));
        changed
    }
}

/// A usable scale factor: finite and positive, else `1.0` (a bad value from the OS must not turn
/// into NaN matrices later).
pub fn sanitize_scale(scale: f64) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale as f32
    } else {
        tracing::warn!(scale, "ignoring an invalid window scale factor, using 1.0");
        1.0
    }
}

/// The [`SurfaceInfo`] for a window of `width` x `height` physical pixels.
pub fn surface_info(width: u32, height: u32, scale: f64) -> SurfaceInfo {
    SurfaceInfo {
        width,
        height,
        scale_factor: sanitize_scale(scale),
    }
}

/// The lifecycle events to deliver for one observation, in order.
///
/// `Resized` to `0 x 0` is passed through unchanged: that is how the contract says "minimized".
/// A destroyed window loses its surface; it is **not** a `Suspended` (desktop platforms never
/// suspend, see `LifecycleTracker`).
pub fn lifecycle_events(observed: Observed) -> Vec<LifecycleEvent> {
    match observed {
        Observed::Resized { width, height } => vec![LifecycleEvent::Resized { width, height }],
        Observed::ScaleFactor(scale) => vec![LifecycleEvent::ScaleFactorChanged {
            scale_factor: sanitize_scale(scale),
        }],
        Observed::Focused(focused) => vec![LifecycleEvent::FocusChanged(focused)],
        Observed::CloseRequested => vec![LifecycleEvent::CloseRequested],
        Observed::Destroyed => vec![LifecycleEvent::SurfaceLost],
    }
}

#[cfg(test)]
mod tests {
    use ss2_platform_api::LifecycleTracker;

    use super::*;

    #[test]
    fn each_observation_maps_to_the_documented_event() {
        assert_eq!(
            lifecycle_events(Observed::Resized {
                width: 800,
                height: 600
            }),
            [LifecycleEvent::Resized {
                width: 800,
                height: 600
            }]
        );
        assert_eq!(
            lifecycle_events(Observed::ScaleFactor(1.5)),
            [LifecycleEvent::ScaleFactorChanged { scale_factor: 1.5 }]
        );
        assert_eq!(
            lifecycle_events(Observed::Focused(false)),
            [LifecycleEvent::FocusChanged(false)]
        );
        assert_eq!(
            lifecycle_events(Observed::CloseRequested),
            [LifecycleEvent::CloseRequested]
        );
        assert_eq!(
            lifecycle_events(Observed::Destroyed),
            [LifecycleEvent::SurfaceLost]
        );
    }

    #[test]
    fn repeated_sizes_are_dropped_but_every_real_change_passes() {
        let mut f = ResizeFilter::default();
        assert!(f.accept(800, 600), "the first size is news");
        assert!(!f.accept(800, 600), "same size again");
        assert!(f.accept(1024, 768));
        assert!(f.accept(0, 0), "minimize is a change");
        assert!(!f.accept(0, 0), "still minimized");
        assert!(f.accept(1024, 768), "restoring is a change");

        let mut f = ResizeFilter::default();
        f.reset(1600, 900);
        assert!(
            !f.accept(1600, 900),
            "the size the app was told about is not news"
        );
        assert!(f.accept(1422, 783));
        assert!(
            f.accept(1600, 900),
            "going back counts: it differs from the last one"
        );
    }

    #[test]
    fn bad_scale_factors_never_reach_the_app() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(sanitize_scale(bad), 1.0, "{bad}");
            assert_eq!(surface_info(10, 10, bad).scale_factor, 1.0);
        }
        assert_eq!(sanitize_scale(2.0), 2.0);
        assert_eq!(sanitize_scale(1.25), 1.25);
    }

    #[test]
    fn surface_info_carries_size_and_scale() {
        assert_eq!(
            surface_info(1920, 1080, 1.0),
            SurfaceInfo {
                width: 1920,
                height: 1080,
                scale_factor: 1.0
            }
        );
    }

    /// The whole desktop story through the tracker of the contract: start up, resize, minimize,
    /// restore, lose focus, close.
    #[test]
    fn a_desktop_session_drives_the_tracker_as_the_contract_promises() {
        let mut life = LifecycleTracker::new();
        fn feed(life: &mut LifecycleTracker, events: Vec<LifecycleEvent>) {
            events.iter().for_each(|e| life.apply(e));
        }

        feed(
            &mut life,
            vec![
                LifecycleEvent::Resumed,
                LifecycleEvent::SurfaceReady(surface_info(1280, 720, 1.0)),
            ],
        );
        assert!(life.can_render());

        feed(
            &mut life,
            lifecycle_events(Observed::Resized {
                width: 1600,
                height: 900,
            }),
        );
        assert_eq!(life.aspect(), Some(1600.0 / 900.0));

        feed(
            &mut life,
            lifecycle_events(Observed::Resized {
                width: 0,
                height: 0,
            }),
        );
        assert!(!life.can_render(), "minimized: skip rendering");
        assert!(
            !life.is_suspended(),
            "but desktop minimize is not a suspend"
        );

        feed(
            &mut life,
            lifecycle_events(Observed::Resized {
                width: 1600,
                height: 900,
            }),
        );
        assert!(life.can_render());

        feed(&mut life, lifecycle_events(Observed::ScaleFactor(2.0)));
        assert_eq!(life.surface().unwrap().scale_factor, 2.0);

        feed(&mut life, lifecycle_events(Observed::Focused(false)));
        assert!(!life.is_focused());

        feed(&mut life, lifecycle_events(Observed::CloseRequested));
        assert!(life.close_requested());

        feed(&mut life, lifecycle_events(Observed::Destroyed));
        assert!(!life.can_render() && life.surface().is_none());
    }
}
