//! Opens a window through the platform loop and prints what the contract delivers.
//!
//!     cargo run -p ss2-platform --example window            # until you close it
//!     cargo run -p ss2-platform --example window -- 120     # exit after 120 frames
//!
//! Resize it, minimize it, move it between monitors with different scaling, click away and back,
//! then close it: each step shows up as a line below.

use ss2_platform::window::{PlatformApp, PlatformWindow, WindowConfig, run};
use ss2_platform_api::{
    AppControl, AppHandler, FrameInfo, LifecycleEvent, LifecycleTracker, PlatformEvent,
};

struct Demo {
    life: LifecycleTracker,
    frames_left: Option<u64>,
    drawn: u64,
}

impl AppHandler for Demo {
    fn event(&mut self, event: PlatformEvent) -> AppControl {
        println!("event: {event:?}");
        if let PlatformEvent::Lifecycle(e) = &event {
            self.life.apply(e);
            if matches!(e, LifecycleEvent::CloseRequested) {
                return AppControl::Exit;
            }
        }
        AppControl::Continue
    }

    fn frame(&mut self, frame: &FrameInfo) -> AppControl {
        if self.life.can_render() {
            self.drawn += 1;
        }
        if frame.index.is_multiple_of(60) {
            println!(
                "frame {} delta {:?} can_render={} drawn={}",
                frame.index,
                frame.delta,
                self.life.can_render(),
                self.drawn
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
        Ok(())
    }

    fn window_lost(&mut self) {
        println!("window_lost: drop the renderer here");
    }
}

fn main() {
    let frames_left = std::env::args().nth(1).and_then(|a| a.parse().ok());
    let mut app = Demo {
        life: LifecycleTracker::new(),
        frames_left,
        drawn: 0,
    };
    match run(&WindowConfig::default(), &mut app) {
        Ok(()) => println!("done after drawing {} frames", app.drawn),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
