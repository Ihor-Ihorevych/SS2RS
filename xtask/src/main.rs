//! `cargo xtask <command>` — repository tooling.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use xtask::guard::{self, Mode};

#[derive(Parser)]
#[command(name = "xtask", about = "SS2RS repository tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fail if proprietary game content (or a suspiciously large blob) is tracked or staged.
    Guard {
        /// Check only files staged for the next commit (used by the pre-commit hook).
        #[arg(long)]
        staged: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Guard { staged } => {
            let mode = if staged {
                Mode::Staged
            } else {
                Mode::AllTracked
            };
            match guard::run(mode) {
                Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
                Ok(violations) => {
                    eprintln!("guard: proprietary or oversized content must not be committed:");
                    for v in &violations {
                        eprintln!("  {v}");
                    }
                    ExitCode::from(1)
                }
                Err(e) => {
                    eprintln!("guard: {e:#}");
                    ExitCode::from(2)
                }
            }
        }
    }
}
