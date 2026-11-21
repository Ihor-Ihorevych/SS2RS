//! `ss2-app` — the SS2RS client. Currently a placeholder-skin menu (no game data used).

mod app;
mod cli;
mod screenshot;
mod state;

use std::process::ExitCode;

fn main() -> ExitCode {
    ss2_core::logging::init_logging("info");
    let mode = match cli::parse_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("ss2-app: {message}");
            return ExitCode::from(2);
        }
    };
    let result = match mode {
        cli::Mode::Window => app::run(None),
        cli::Mode::Smoke { frames } => app::run(Some(frames)),
        cli::Mode::Screenshot { screen, out, size } => screenshot::take(screen, &out, size),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("ss2-app: {message}");
            ExitCode::from(1)
        }
    }
}
