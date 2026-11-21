//! Opens a window through the platform loop and prints what the contract delivers.
//!
//!     cargo run -p ss2-platform --example window            # until you close it
//!     cargo run -p ss2-platform --example window -- 600     # exit after 600 frames
//!
//! Try: resize or minimize it, type (text and keys are separate events), hold a key (repeat), click,
//! scroll, press **C** to capture the cursor (mouse-look: motion keeps flowing, absolute positions
//! stop) and **C** again to release it, then click another window while holding a key to see the
//! releases the platform synthesizes when focus is lost.

use ss2_platform::window::{PlatformApp, PlatformWindow, WindowConfig, run};
use ss2_platform_api::{
    AppControl, AppHandler, FrameInfo, InputEvent, KeyCode, LifecycleEvent, LifecycleTracker,
    PlatformEvent,
};

#[derive(Default)]
struct Demo {
    life: LifecycleTracker,
    window: Option<PlatformWindow>,
    frames_left: Option<u64>,
    drawn: u64,
    cursor_moves: u64,
    motion_events: u64,
    motion_sum: (f32, f32),
}

impl AppHandler for Demo {
    fn event(&mut self, event: PlatformEvent) -> AppControl {
        match &event {
            // Too chatty to print every one; counted and summarized below.
            PlatformEvent::Input(InputEvent::CursorMoved { .. }) => self.cursor_moves += 1,
            PlatformEvent::Input(InputEvent::MouseMotion { dx, dy }) => {
                self.motion_events += 1;
                self.motion_sum.0 += dx;
                self.motion_sum.1 += dy;
            }
            other => println!("event: {other:?}"),
        }
        match &event {
            PlatformEvent::Lifecycle(e) => {
                self.life.apply(e);
                if matches!(e, LifecycleEvent::CloseRequested) {
                    return AppControl::Exit;
                }
            }
            PlatformEvent::Input(InputEvent::Key {
                key: KeyCode::C,
                pressed: true,
                repeat: false,
            }) => {
                if let Some(window) = &self.window {
                    let want = !window.is_cursor_captured();
                    match window.set_cursor_captured(want) {
                        Ok(()) => println!("cursor captured: {want}"),
                        Err(e) => println!("capture failed: {e}"),
                    }
                }
            }
            _ => {}
        }
        AppControl::Continue
    }

    fn frame(&mut self, frame: &FrameInfo) -> AppControl {
        if self.life.can_render() {
            self.drawn += 1;
        }
        if frame.index.is_multiple_of(240) {
            println!(
                "frame {}: can_render={} cursor_moves={} motion_events={} motion_sum={:?}",
                frame.index,
                self.life.can_render(),
                self.cursor_moves,
                self.motion_events,
                self.motion_sum
            );
        }
        match self.frames_left.as_mut() {
            Some(left) if *left <= 1 => AppControl::Exit,
            Some(left) => {
                *left -= 1;
                AppControl::Continue
            }
            None => AppControl::Continue,
        }
    }
}

impl PlatformApp for Demo {
    fn window_ready(&mut self, window: &PlatformWindow) -> Result<(), String> {
        println!(
            "window_ready: {:?} px at scale {}",
            window.size(),
            window.scale_factor()
        );
        window.set_text_input(true);
        self.window = Some(window.clone());
        Ok(())
    }

    fn window_lost(&mut self) {
        println!("window_lost: drop the renderer here");
        self.window = None;
    }
}

fn main() {
    let mut app = Demo {
        frames_left: std::env::args().nth(1).and_then(|a| a.parse().ok()),
        ..Demo::default()
    };
    match run(&WindowConfig::default(), &mut app) {
        Ok(()) => println!("done after drawing {} frames", app.drawn),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
