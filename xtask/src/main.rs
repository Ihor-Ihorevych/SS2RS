//! `cargo xtask <command>` — repository tooling.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use xtask::guard::{self, Mode};
use xtask::roadmap;

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
    /// Mirror docs/roadmap/*.md as GitHub issues (dry run unless --apply).
    RoadmapSync {
        /// Actually create the missing labels and issues.
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::RoadmapSync { apply } => {
            let result = guard::repo_root().and_then(|root| {
                roadmap::sync(&std::path::Path::new(&root).join("docs/roadmap"), apply)
            });
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("roadmap-sync: {e:#}");
                    ExitCode::from(2)
                }
            }
        }
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
