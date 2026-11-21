//! `cargo xtask <command>` — repository tooling.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use xtask::claim;
use xtask::doctor;
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
    /// Claim a roadmap task: edit its line, push a signed-off commit straight to main.
    Claim {
        /// Task ID, e.g. T02-006.
        id: String,
        /// Your GitHub handle (default: the logged-in `gh` user). `name` or `name/agent`.
        #[arg(long)]
        who: Option<String>,
        /// Work branch name (default: derived from the ID and title).
        #[arg(long)]
        branch: Option<String>,
        /// Show what would change, change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Claim although dependencies are not ticked (only if merged stubs/mocks exist).
        #[arg(long)]
        allow_open_deps: bool,
    },
    /// Give a claim back with a [RELEASED] commit.
    Release {
        /// Task ID, e.g. T02-006.
        id: String,
        /// Why you are releasing it (goes into the commit message).
        #[arg(long)]
        reason: String,
        /// Your GitHub handle (default: the logged-in `gh` user).
        #[arg(long)]
        who: Option<String>,
        /// Release somebody else's claim (stale claims only, see AGENTS.md rule 8).
        #[arg(long)]
        force: bool,
        /// Show what would change, change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Check this machine against docs/DEV_TOOLS.md; exits 1 only if a required tool is missing.
    Doctor {
        /// Also require the tools of a track (T01, T02, T04, T05, T06, T00, T12); repeatable.
        #[arg(long = "role", value_name = "TRACK")]
        roles: Vec<String>,
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
        Command::Claim {
            id,
            who,
            branch,
            dry_run,
            allow_open_deps,
        } => finish_claim(|ctx| {
            let who = match who {
                Some(w) => w,
                None => claim::default_who()?,
            };
            claim::claim(
                ctx,
                &claim::ClaimReq {
                    id,
                    who,
                    branch,
                    date: claim::today_utc(),
                    dry_run,
                    allow_open_deps,
                },
            )
        }),
        Command::Release {
            id,
            reason,
            who,
            force,
            dry_run,
        } => finish_claim(|ctx| {
            let who = match who {
                Some(w) => w,
                None => claim::default_who()?,
            };
            claim::release(
                ctx,
                &claim::ReleaseReq {
                    id,
                    who,
                    reason,
                    force,
                    dry_run,
                },
            )
        }),
        Command::Doctor { roles } => {
            let result = guard::repo_root().map(|root| {
                doctor::run_checks(&doctor::SystemProbe, std::path::Path::new(&root), &roles)
            });
            match result {
                Ok(report) => {
                    print!("{}", doctor::render(&report));
                    if report.has_blocking_problem() {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("doctor: {e:#}");
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

/// Run a claim/release and print the outcome. Exit status: 0 done, 3 task not available (taken,
/// blocked, finished, lost the race: pick another), 2 real error.
fn finish_claim(run: impl FnOnce(&claim::Ctx) -> anyhow::Result<claim::Done>) -> ExitCode {
    let result = guard::repo_root().and_then(|root| run(&claim::Ctx::new(root)));
    match result {
        Ok(done) => {
            let verb = if done.dry_run {
                "would commit"
            } else {
                "committed and pushed"
            };
            println!("{verb}: {}", done.message);
            println!("{}", done.line);
            if let (Some(branch), false) = (&done.branch, done.dry_run) {
                println!(
                    "
next: git switch -c {branch}"
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) if e.downcast_ref::<claim::NotAvailable>().is_some() => {
            eprintln!("not available: {e}");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
