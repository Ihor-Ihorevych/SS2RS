//! The application side of the platform loop: [`AppHandler`], and a scripted driver that stands
//! in for the real window so app logic can be tested without one.

use std::time::Duration;

use crate::event::PlatformEvent;

/// Whether the app wants the loop to keep going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppControl {
    /// Keep running.
    Continue,
    /// Leave the loop (quit).
    Exit,
}

/// Timing of one display frame, supplied by the platform loop.
///
/// This is **wall-clock** time for rendering, UI animation and the fixed-timestep accumulator
/// (T03-013). Simulation code never reads it; sim time is the tick counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameInfo {
    /// 0 for the first frame, then increasing by 1.
    pub index: u64,
    /// Time since the previous frame (zero for the first).
    pub delta: Duration,
}

/// What the platform loop calls. The real loop (`winit`, T03-001), the headless server loop and
/// [`run_script`] all drive the same trait, so app logic is written once.
///
/// All calls happen on one thread (the platform's main thread); implementations need not be
/// `Send`. Neither method may block for long: a slow `event` delays input, a slow `frame` drops
/// frames.
pub trait AppHandler {
    /// An input or lifecycle event arrived. Called before the frame that should see its effect.
    /// Return [`AppControl::Exit`] to quit (typically on `CloseRequested`, after saving).
    fn event(&mut self, event: PlatformEvent) -> AppControl;

    /// Run one display frame: read input, advance, render (only if the lifecycle allows it, see
    /// [`LifecycleTracker`](crate::LifecycleTracker)). Called repeatedly while the app is
    /// running, including while suspended (so the app can keep ticking audio/net or sleep).
    fn frame(&mut self, frame: &FrameInfo) -> AppControl;
}

/// One step of a test script.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptStep {
    /// Deliver this event.
    Event(PlatformEvent),
    /// Run one frame of 1/60 s.
    Frame,
}

impl<E: Into<PlatformEvent>> From<E> for ScriptStep {
    fn from(e: E) -> Self {
        Self::Event(e.into())
    }
}

/// What [`run_script`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptReport {
    /// Events delivered.
    pub events: usize,
    /// Frames run.
    pub frames: u64,
    /// `true` if the handler asked to exit before the script ended (remaining steps skipped).
    pub exited: bool,
}

/// Drive `handler` through `steps` exactly like the real loop would, with a fixed 1/60 s frame
/// delta so tests are deterministic. Stops at the first [`AppControl::Exit`].
///
/// ```
/// use ss2_platform_api::{
///     run_script, AppControl, AppHandler, FrameInfo, LifecycleEvent, PlatformEvent, ScriptStep,
/// };
///
/// struct Quitter;
/// impl AppHandler for Quitter {
///     fn event(&mut self, e: PlatformEvent) -> AppControl {
///         match e {
///             PlatformEvent::Lifecycle(LifecycleEvent::CloseRequested) => AppControl::Exit,
///             _ => AppControl::Continue,
///         }
///     }
///     fn frame(&mut self, _: &FrameInfo) -> AppControl { AppControl::Continue }
/// }
///
/// let report = run_script(&mut Quitter, [
///     LifecycleEvent::Resumed.into(),
///     ScriptStep::Frame,
///     LifecycleEvent::CloseRequested.into(),
///     ScriptStep::Frame, // never runs
/// ]);
/// assert_eq!((report.events, report.frames, report.exited), (2, 1, true));
/// ```
pub fn run_script(
    handler: &mut dyn AppHandler,
    steps: impl IntoIterator<Item = ScriptStep>,
) -> ScriptReport {
    const DELTA: Duration = Duration::from_nanos(16_666_667);
    let mut report = ScriptReport::default();
    for step in steps {
        let control = match step {
            ScriptStep::Event(event) => {
                report.events += 1;
                handler.event(event)
            }
            ScriptStep::Frame => {
                let info = FrameInfo {
                    index: report.frames,
                    delta: if report.frames == 0 {
                        Duration::ZERO
                    } else {
                        DELTA
                    },
                };
                report.frames += 1;
                handler.frame(&info)
            }
        };
        if control == AppControl::Exit {
            report.exited = true;
            break;
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{InputEvent, LifecycleEvent, SurfaceInfo};
    use crate::keys::KeyCode;
    use crate::lifecycle::LifecycleTracker;

    /// A tiny app: tracks lifecycle, counts the frames it would have rendered, quits on Escape.
    #[derive(Default)]
    struct TestApp {
        life: LifecycleTracker,
        rendered: u64,
        frame_indices: Vec<u64>,
        deltas: Vec<Duration>,
        keys: Vec<KeyCode>,
    }

    impl AppHandler for TestApp {
        fn event(&mut self, event: PlatformEvent) -> AppControl {
            match event {
                PlatformEvent::Lifecycle(e) => {
                    self.life.apply(&e);
                    AppControl::Continue
                }
                PlatformEvent::Input(InputEvent::Key {
                    key, pressed: true, ..
                }) => {
                    self.keys.push(key);
                    if key == KeyCode::Escape {
                        AppControl::Exit
                    } else {
                        AppControl::Continue
                    }
                }
                _ => AppControl::Continue,
            }
        }

        fn frame(&mut self, frame: &FrameInfo) -> AppControl {
            self.frame_indices.push(frame.index);
            self.deltas.push(frame.delta);
            if self.life.can_render() {
                self.rendered += 1;
            }
            AppControl::Continue
        }
    }

    fn surface() -> LifecycleEvent {
        LifecycleEvent::SurfaceReady(SurfaceInfo {
            width: 640,
            height: 480,
            scale_factor: 1.0,
        })
    }

    fn key(k: KeyCode) -> ScriptStep {
        InputEvent::Key {
            key: k,
            pressed: true,
            repeat: false,
        }
        .into()
    }

    #[test]
    fn frames_render_only_while_the_lifecycle_allows() {
        let mut app = TestApp::default();
        let report = run_script(
            &mut app,
            [
                ScriptStep::Frame, // before Resumed: no render
                LifecycleEvent::Resumed.into(),
                surface().into(),
                ScriptStep::Frame,
                ScriptStep::Frame,
                LifecycleEvent::SurfaceLost.into(),
                LifecycleEvent::Suspended.into(),
                ScriptStep::Frame, // suspended: frames still run, nothing rendered
                LifecycleEvent::Resumed.into(),
                surface().into(),
                ScriptStep::Frame,
            ],
        );
        assert_eq!(report.frames, 5);
        assert_eq!(app.rendered, 3);
        assert!(!report.exited);
    }

    #[test]
    fn frame_info_is_deterministic() {
        let mut app = TestApp::default();
        run_script(
            &mut app,
            [ScriptStep::Frame, ScriptStep::Frame, ScriptStep::Frame],
        );
        assert_eq!(app.frame_indices, [0, 1, 2]);
        assert_eq!(app.deltas[0], Duration::ZERO);
        assert_eq!(app.deltas[1], app.deltas[2]);
        assert!((app.deltas[1].as_secs_f64() - 1.0 / 60.0).abs() < 1e-6);
    }

    #[test]
    fn exit_stops_the_script_immediately() {
        let mut app = TestApp::default();
        let report = run_script(
            &mut app,
            [
                key(KeyCode::W),
                key(KeyCode::Escape),
                key(KeyCode::S),
                ScriptStep::Frame,
            ],
        );
        assert_eq!(
            report,
            ScriptReport {
                events: 2,
                frames: 0,
                exited: true
            }
        );
        assert_eq!(app.keys, [KeyCode::W, KeyCode::Escape]);
    }

    #[test]
    fn handler_is_usable_as_trait_object() {
        let mut app = TestApp::default();
        let dynamic: &mut dyn AppHandler = &mut app;
        assert_eq!(
            dynamic.frame(&FrameInfo {
                index: 0,
                delta: Duration::ZERO
            }),
            AppControl::Continue
        );
    }
}
