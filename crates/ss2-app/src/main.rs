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
        cli::Mode::Help => {
            print!(
                "{}",
                ss2_core::cli::usage(ss2_core::cli::Role::Client, "ss2-app")
            );
            Ok(())
        }
        cli::Mode::Version => {
            println!("ss2-app {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        cli::Mode::Window(options) => launch(&options),
        cli::Mode::Smoke { frames } => app::run(Some(frames), &[]),
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

/// Open the interactive window. The launch options that need features which do not exist yet
/// are reported instead of being silently ignored.
fn launch(options: &ss2_core::cli::LaunchOptions) -> Result<(), String> {
    if options.connect.is_some() || options.listen {
        return Err("--connect and --listen need multiplayer, which is not implemented yet".into());
    }
    if let Some(level) = &options.level {
        tracing::warn!("--level {level} is ignored: the app has no game data support yet");
    }
    if let Some(dir) = &options.data_dir {
        tracing::info!("game data folder {} (not used yet)", dir.display());
    }
    if options.portable {
        tracing::info!("portable mode requested (not wired into the app yet)");
    }
    app::run(None, &options.set)
}
