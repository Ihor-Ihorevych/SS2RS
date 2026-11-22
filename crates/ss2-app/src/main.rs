//! `ss2-app` — the SS2RS client. Currently a placeholder-skin menu (no game data used).

mod app;
mod cli;
mod diagnostics;
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
        cli::Mode::Smoke { .. } | cli::Mode::Screenshot { .. } => {
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
        cli::Mode::Smoke { frames } => app::run(Some(frames), &[], None),
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
fn launch(
    options: &ss2_core::cli::LaunchOptions,
    paths: Option<&ss2_platform_api::PlatformPaths>,
) -> Result<(), String> {
    if options.connect.is_some() || options.listen {
        return Err("--connect and --listen need multiplayer, which is not implemented yet".into());
    }
    if let Some(level) = &options.level {
        tracing::warn!("--level {level} is ignored: the app has no game data support yet");
    }
    if let Some(dir) = &options.data_dir {
        tracing::info!("game data folder {} (not used yet)", dir.display());
    }
    app::run(
        None,
        &options.set,
        paths.map(ss2_platform_api::PlatformPaths::settings_file),
    )
}
