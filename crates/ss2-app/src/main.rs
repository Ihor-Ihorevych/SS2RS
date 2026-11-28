//! `ss2-app` — the SS2RS client: a placeholder-skin menu, the first-run data-location wizard and the level player.

mod app;
mod cli;
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
mod wizard_scene;

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
        cli::Mode::Window { options, .. } => diagnostics::start(options),
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
        cli::Mode::Window { .. } | cli::Mode::Smoke { .. } => profiling::start(),
        _ => None,
    };
    let result = match mode {
        cli::Mode::Help => {
            print!(
                "{}{}",
                ss2_core::cli::usage(ss2_core::cli::Role::Client, "ss2-app"),
                cli::APP_HELP
            );
            Ok(())
        }
        cli::Mode::Version => {
            println!("ss2-app {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        cli::Mode::Window {
            options,
            forget_data,
        } => launch(&options, forget_data, paths.as_ref()),
        cli::Mode::Smoke { frames } => app::run(app::RunOptions {
            smoke_frames: Some(frames),
            overrides: &[],
            paths: None,
            levels: play::Levels::new(None),
            start_level: None,
            wizard: None,
            data_store: None,
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

/// Open the interactive window with the levels of the player's install: `--data`, then `SS2_DATA_DIR`, then the folder
/// remembered from the first-run wizard, each checked again ([`data_location::startup`]); when none of them is usable the
/// wizard shows first (T10-019b, `docs/FIRST_RUN.md`). `forget_data` (`--forget-data`) forgets the remembered folder first.
/// Without the configuration folder (no home folder and not portable) nothing can be remembered, and the folder is found the
/// old way (`--data`, `SS2_DATA_DIR`, the first install the search finds; without one only the test room is offered).
/// The main menu's "Choose Game Folder..." shows the wizard again when there is a configuration folder (T10-035).
/// The launch options that need features which do not exist yet are reported instead of being silently ignored.
fn launch(
    options: &ss2_core::cli::LaunchOptions,
    forget_data: bool,
    paths: Option<&ss2_platform_api::PlatformPaths>,
) -> Result<(), String> {
    if options.connect.is_some() || options.listen {
        return Err("--connect and --listen need multiplayer, which is not implemented yet".into());
    }
    let store = paths.map(|p| data_location::store_path(&p.config_dir));
    if forget_data {
        match &store {
            Some(store) => {
                data_location::forget(store)?;
                tracing::info!(
                    "forgot the remembered game data folder ({})",
                    store.display()
                );
            }
            None => tracing::warn!("--forget-data: there is no configuration folder"),
        }
    }
    let open = |dir: &std::path::Path| {
        tracing::info!("game data folder {}", dir.display());
        level_session::GameData::open(dir)
            .inspect_err(|e| tracing::warn!("no game data: {e}"))
            .ok()
    };
    let (game, wizard) = match &store {
        Some(store) => match data_location::startup(
            &ss2_vfs::SystemEnvironment,
            options.data_dir.as_deref(),
            store,
        ) {
            ss2_vfs::Startup::Ready { dir, source, .. } => {
                tracing::info!(?source, "the game data folder passed the check");
                (open(&dir), None)
            }
            ss2_vfs::Startup::NeedsWizard(reason) => {
                tracing::info!(?reason, "showing the first-run wizard");
                let wizard = wizard_scene::WizardScene::for_system(
                    data_location::wizard_for(&reason),
                    store.clone(),
                );
                (None, Some(wizard))
            }
        },
        None => (
            level_session::data_dir(options.data_dir.as_deref())
                .inspect_err(|e| tracing::warn!("no game data: {e}"))
                .ok()
                .and_then(|dir| open(&dir)),
            None,
        ),
    };
    app::run(app::RunOptions {
        smoke_frames: None,
        overrides: &options.set,
        paths,
        levels: play::Levels::new(game),
        start_level: options.level.clone(),
        wizard,
        data_store: store,
    })
}
