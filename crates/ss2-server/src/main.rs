//! `ss2-server` — the headless SS2RS dedicated server (no window, GPU or audio).
//!
//! It reads a configuration file (`--config FILE`; `--print-config` prints an annotated example),
//! the command line overrides the file, logs to the terminal and optionally a rotating file, serves
//! metrics if asked, and runs the simulation loop with a console on standard input. **There is no
//! network yet** (the transport does not exist): the server runs an empty world and nobody can
//! join. See `docs/DEDICATED_SERVER.md`.

mod config;
mod metrics;
mod runtime;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use ss2_core::cli::{Parsed, Role, parse_launch_options, usage};
use ss2_core::logfile::FileLogConfig;

use config::ServerSettings;
use metrics::{Metrics, MetricsServer};
use runtime::{Runtime, SystemClock};

/// What a run of the server binary should do.
#[derive(Debug)]
enum Outcome {
    /// Print this text and exit successfully (`--help`, `--version`, `--print-config`).
    Print(String),
    /// A normal start with these (validated) settings.
    Run(Box<ServerSettings>),
}

/// Take `--config FILE` / `--config=FILE` and `--print-config` out of the arguments; the rest go to
/// the shared launch-option parser.
fn split_server_args<I: IntoIterator<Item = String>>(
    args: I,
) -> Result<(Option<PathBuf>, bool, Vec<String>), String> {
    let (mut config, mut print_config, mut rest) = (None, false, Vec::new());
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--print-config" {
            print_config = true;
        } else if arg == "--config" {
            config = Some(PathBuf::from(
                args.next().ok_or("--config needs a file name")?,
            ));
        } else if let Some(value) = arg.strip_prefix("--config=") {
            config = Some(PathBuf::from(value));
        } else {
            rest.push(arg);
        }
    }
    Ok((config, print_config, rest))
}

/// Parse the arguments after the program name; `read` loads a file's text.
fn execute<I: IntoIterator<Item = String>>(
    args: I,
    read: &dyn Fn(&Path) -> std::io::Result<String>,
) -> Result<Outcome, String> {
    let (config_path, print_config, rest) = split_server_args(args)?;
    if print_config {
        return Ok(Outcome::Print(ServerSettings::example_toml()));
    }
    match parse_launch_options(Role::Server, rest).map_err(|e| e.to_string())? {
        Parsed::Help => Ok(Outcome::Print(format!(
            "{}  --config FILE  read the server configuration from a TOML file (see --print-config)\n  --print-config  print an annotated example configuration and exit\n",
            usage(Role::Server, "ss2-server")
        ))),
        Parsed::Version => Ok(Outcome::Print(format!(
            "ss2-server {}\n",
            env!("CARGO_PKG_VERSION")
        ))),
        Parsed::Run(options) => {
            let mut settings = match &config_path {
                Some(path) => {
                    let text =
                        read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                    ServerSettings::from_toml(&text)
                        .map_err(|e| format!("{}: {e}", path.display()))?
                }
                None => ServerSettings::default(),
            };
            settings.apply_launch_options(&options);
            let problems = settings.validate();
            if !problems.is_empty() {
                return Err(format!(
                    "invalid configuration:\n  {}",
                    problems.join("\n  ")
                ));
            }
            Ok(Outcome::Run(Box::new(settings)))
        }
    }
}

fn serve(settings: &ServerSettings) -> Result<(), String> {
    let log_ready = match &settings.log_dir {
        Some(dir) => ss2_core::logging::init_logging_with_file("info", FileLogConfig::new(dir))
            .map_err(|e| format!("cannot open the log file: {e}"))?,
        None => ss2_core::logging::init_logging("info"),
    };
    let _ = log_ready;
    tracing::info!(name = %settings.name, bind = %settings.bind, max_players = settings.max_players, tick_hz = settings.tick_hz, "ss2-server starting (no network yet: the server runs an empty world)");

    let shared = Arc::new(Mutex::new(Metrics::default()));
    let metrics_server = match &settings.metrics_bind {
        Some(addr) => {
            let server = MetricsServer::start(addr, Arc::clone(&shared))
                .map_err(|e| format!("cannot serve metrics on {addr}: {e}"))?;
            tracing::info!(addr = %server.local_addr(), "metrics are served at /metrics and /healthz");
            Some(server)
        }
        None => None,
    };

    let mut runtime = Runtime::new(settings, shared)?;
    let (lines_tx, lines_rx) = mpsc::channel::<String>();
    if settings.console {
        std::thread::Builder::new()
            .name("ss2-console".into())
            .spawn(move || {
                for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                    if lines_tx.send(line).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| format!("cannot start the console reader: {e}"))?;
    }
    let reason = runtime.run(
        &mut SystemClock::new(),
        &mut || lines_rx.try_recv().ok(),
        None,
    );
    tracing::info!(?reason, "ss2-server stopping");
    if let Some(server) = metrics_server {
        server.stop();
    }
    Ok(())
}

fn main() -> ExitCode {
    match execute(std::env::args().skip(1), &|path| {
        std::fs::read_to_string(path)
    }) {
        Ok(Outcome::Print(text)) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Ok(Outcome::Run(settings)) => match serve(&settings) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("ss2-server: {message}");
                ExitCode::from(1)
            }
        },
        Err(message) => {
            eprintln!("ss2-server: {message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_files(_: &Path) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no such file",
        ))
    }

    fn exec(args: &[&str]) -> Result<Outcome, String> {
        execute(args.iter().map(|s| (*s).to_owned()), &no_files)
    }

    fn exec_with_file(args: &[&str], text: &'static str) -> Result<Outcome, String> {
        execute(args.iter().map(|s| (*s).to_owned()), &|_| {
            Ok(text.to_owned())
        })
    }

    #[test]
    fn without_arguments_it_runs_with_the_defaults() {
        let Ok(Outcome::Run(settings)) = exec(&[]) else {
            panic!("expected a run");
        };
        assert_eq!(*settings, ServerSettings::default());
    }

    #[test]
    fn help_prints_the_server_usage_and_the_config_options_and_not_the_client_options() {
        let Ok(Outcome::Print(text)) = exec(&["--help"]) else {
            panic!("expected usage text");
        };
        assert!(
            text.contains("ss2-server") && text.contains("--bind"),
            "{text}"
        );
        assert!(
            text.contains("--config") && text.contains("--print-config"),
            "{text}"
        );
        assert!(!text.contains("--connect"), "{text}");
    }

    #[test]
    fn version_names_the_program_and_its_version() {
        let Ok(Outcome::Print(text)) = exec(&["--version"]) else {
            panic!("expected the version");
        };
        assert!(text.starts_with("ss2-server "), "{text}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
    }

    #[test]
    fn print_config_prints_an_example_that_is_a_valid_file() {
        let Ok(Outcome::Print(text)) = exec(&["--print-config"]) else {
            panic!("expected the example");
        };
        assert_eq!(
            ServerSettings::from_toml(&text).unwrap(),
            ServerSettings::default()
        );
    }

    #[test]
    fn server_options_are_parsed_and_override_the_file() {
        let Ok(Outcome::Run(s)) = exec(&["--bind", "0.0.0.0:27961", "--level=dm_x"]) else {
            panic!("expected a run");
        };
        assert_eq!(s.bind, "0.0.0.0:27961");
        assert_eq!(s.level.as_deref(), Some("dm_x"));
        let Ok(Outcome::Run(s)) = exec_with_file(
            &["--config", "s.toml", "--bind=1.2.3.4:5"],
            "bind = \"9.9.9.9:9\"\nmax_players = 3",
        ) else {
            panic!("expected a run");
        };
        assert_eq!(
            (s.bind.as_str(), s.max_players),
            ("1.2.3.4:5", 3),
            "the command line wins, the file's other values stay"
        );
    }

    #[test]
    fn a_missing_or_broken_file_and_bad_values_are_refused_with_every_problem() {
        let missing = exec(&["--config", "nope.toml"]).unwrap_err();
        assert!(
            missing.contains("cannot read") && missing.contains("nope.toml"),
            "{missing}"
        );
        let broken = exec_with_file(&["--config=s.toml"], "max_player = 3").unwrap_err();
        assert!(
            broken.contains("s.toml") && broken.contains("max_player"),
            "{broken}"
        );
        let invalid =
            exec_with_file(&["--config=s.toml"], "max_players = 0\ntick_hz = 0").unwrap_err();
        assert!(
            invalid.contains("max_players") && invalid.contains("tick_hz"),
            "all problems at once: {invalid}"
        );
        assert!(exec(&["--config"]).unwrap_err().contains("needs a file"));
        assert!(
            exec(&["--bind", "localhost:1"])
                .unwrap_err()
                .contains("bind"),
            "a bad address from the command line is caught too"
        );
    }

    #[test]
    fn client_only_options_are_rejected() {
        for flag in [&["--connect", "host"][..], &["--listen"]] {
            let message = exec(flag).unwrap_err();
            assert!(message.contains(flag[0]), "{message}");
        }
    }
}
