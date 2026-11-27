//! `ss2-app` — the SS2RS client. Currently a placeholder-skin menu (no game data used).

mod app;
mod cli;
// The first-run wizard's host: complete and tested, not called from `main` until T10-019b (see its module documentation).
#[cfg_attr(not(test), allow(dead_code))]
mod data_location;
#[cfg(test)]
mod data_location_tests;
mod diagnostics;
mod input;
mod level_session;
mod play;
mod play_input;
mod profiling;
mod screenshot;
mod state;

use std::process::ExitCode;

fn main() -> ExitCode {
    let mode = match cli::parse_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("ss2-app: {message}");
            return ExitCode::from(2);
        }
    };
    // An interactive run writes a log file and crash reports; --help and --version touch
    // nothing; the developer modes only log to the terminal.
    let paths = match &mode {
        cli::Mode::Window(options) => diagnostics::start(options),
        cli::Mode::Help | cli::Mode::Version => None,
        cli::Mode::Smoke { .. }
        | cli::Mode::Screenshot { .. }
        | cli::Mode::LevelScreenshot { .. } => {
            ss2_core::logging::init_logging("info");
            None
        }
    };
    // Only the modes that run frames can be profiled; the guard lives until `main` returns.
    let _profiler = match &mode {
        cli::Mode::Window(_) | cli::Mode::Smoke { .. } => profiling::start(),
        _ => None,
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
        cli::Mode::Window(options) => launch(&options, paths.as_ref()),
        cli::Mode::Smoke { frames } => app::run(app::RunOptions {
            smoke_frames: Some(frames),
            overrides: &[],
            paths: None,
            levels: play::Levels::new(None),
            start_level: None,
        }),
        cli::Mode::Screenshot { screen, out, size } => screenshot::take(screen, &out, size),
        cli::Mode::LevelScreenshot {
            level,
            out,
            size,
            data,
        } => level_session::take_level_screenshot(&level, &out, size, data.as_deref()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("ss2-app: {message}");
            ExitCode::from(1)
        }
    }
}

/// Open the interactive window with the levels of the player's install (`--data`, `SS2_DATA_DIR` or the search;
/// without an install only the test room is offered). The launch options that need features which do not exist
/// yet are reported instead of being silently ignored.
fn launch(
    options: &ss2_core::cli::LaunchOptions,
    paths: Option<&ss2_platform_api::PlatformPaths>,
) -> Result<(), String> {
    if options.connect.is_some() || options.listen {
        return Err("--connect and --listen need multiplayer, which is not implemented yet".into());
    }
    let game = level_session::data_dir(options.data_dir.as_deref())
        .and_then(|dir| {
            tracing::info!("game data folder {}", dir.display());
            level_session::GameData::open(&dir)
        })
        .inspect_err(|e| tracing::warn!("no game data: {e}"))
        .ok();
    app::run(app::RunOptions {
        smoke_frames: None,
        overrides: &options.set,
        paths,
        levels: play::Levels::new(game),
        start_level: options.level.clone(),
    })
}
