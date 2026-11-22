//! `ss2-server` — the headless SS2RS dedicated server (no window, GPU or audio).
//! Currently a stub: it understands its command line but does not serve anything yet.

use std::process::ExitCode;

use ss2_core::cli::{LaunchOptions, Parsed, Role, parse_launch_options, usage};

/// What a run of the server binary should do.
#[derive(Debug)]
enum Outcome {
    /// Print this text and exit successfully (`--help`, `--version`).
    Print(String),
    /// A normal start: the options are valid, but there is no server to run yet.
    NotImplemented(Box<LaunchOptions>),
}

/// Parse the arguments after the program name.
fn execute<I: IntoIterator<Item = String>>(args: I) -> Result<Outcome, String> {
    match parse_launch_options(Role::Server, args).map_err(|e| e.to_string())? {
        Parsed::Run(options) => Ok(Outcome::NotImplemented(options)),
        Parsed::Help => Ok(Outcome::Print(usage(Role::Server, "ss2-server"))),
        Parsed::Version => Ok(Outcome::Print(format!(
            "ss2-server {}\n",
            env!("CARGO_PKG_VERSION")
        ))),
    }
}

fn main() -> ExitCode {
    match execute(std::env::args().skip(1)) {
        Ok(Outcome::Print(text)) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Ok(Outcome::NotImplemented(options)) => {
            println!("ss2-server: not implemented yet");
            if let Some(addr) = &options.bind {
                println!("  requested bind address: {addr}");
            }
            if let Some(level) = &options.level {
                println!("  requested level: {level}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("ss2-server: {message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec(args: &[&str]) -> Result<Outcome, String> {
        execute(args.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn without_arguments_it_reports_that_it_is_a_stub() {
        let Ok(Outcome::NotImplemented(options)) = exec(&[]) else {
            panic!("expected the stub outcome");
        };
        assert_eq!(*options, ss2_core::cli::LaunchOptions::default());
    }

    #[test]
    fn help_prints_the_server_usage_and_not_the_client_options() {
        let Ok(Outcome::Print(text)) = exec(&["--help"]) else {
            panic!("expected usage text");
        };
        assert!(
            text.contains("ss2-server") && text.contains("--bind"),
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
    fn server_options_are_parsed() {
        let Ok(Outcome::NotImplemented(o)) = exec(&["--bind", "0.0.0.0:27960", "--level=dm_x"])
        else {
            panic!("expected the stub outcome");
        };
        assert_eq!(o.bind.as_deref(), Some("0.0.0.0:27960"));
        assert_eq!(o.level.as_deref(), Some("dm_x"));
    }

    #[test]
    fn client_only_options_are_rejected() {
        for flag in [&["--connect", "host"][..], &["--listen"]] {
            let message = exec(flag).unwrap_err();
            assert!(message.contains(flag[0]), "{message}");
        }
    }
}
