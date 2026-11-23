//! `net-tool` — inspect replay files and see what a bad network does to a packet stream.
//!
//! Exit codes: 0 success · 1 `replay verify` found a replay that is cut short or damaged after its
//! header · 2 error (unreadable file, not a replay, a value that makes no sense).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use clap::{Args, Parser, Subcommand};
use net_tool::{Overrides, PRESETS, Scenario, dump_records, inspect_replay, preset, run_scenario};

#[derive(Parser)]
#[command(
    name = "net-tool",
    version,
    about = "Inspect replay files; run packets through the network simulator"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Replay files.
    #[command(subcommand)]
    Replay(ReplayCommand),
    /// The network simulator.
    #[command(subcommand)]
    Netsim(NetsimCommand),
}

#[derive(Subcommand)]
enum ReplayCommand {
    /// Print the header, what the file holds and how it ended (exit 0 whenever the header is readable).
    Inspect { file: PathBuf },
    /// Say whether the file is complete: exit 0 if so, 1 if it is cut short or damaged, 2 if it is not a replay.
    Verify { file: PathBuf },
    /// List the records, one per line, optionally only ticks `--from` to `--to` (inclusive).
    Dump {
        file: PathBuf,
        #[arg(long)]
        from: Option<u64>,
        #[arg(long)]
        to: Option<u64>,
    },
}

#[derive(Subcommand)]
enum NetsimCommand {
    /// List the named link profiles.
    Presets,
    /// Send a steady stream of packets through a link and report what arrived.
    Run(RunArgs),
}

#[derive(Args)]
struct RunArgs {
    /// A profile from `netsim presets`.
    #[arg(long, default_value = "broadband")]
    preset: String,
    /// The random seed; the same seed gives the same report.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// How many packets to send.
    #[arg(long, default_value_t = 1000)]
    packets: u32,
    /// The size of each packet in bytes.
    #[arg(long, default_value_t = 100)]
    size: usize,
    /// The time between packets in microseconds (16667 is 60 per second).
    #[arg(long, default_value_t = 16_667)]
    interval_us: u64,
    /// Override the one-way delay, in milliseconds.
    #[arg(long)]
    latency_ms: Option<u64>,
    /// Override the jitter, in milliseconds.
    #[arg(long)]
    jitter_ms: Option<u64>,
    /// Override the loss, in percent.
    #[arg(long)]
    loss_pct: Option<f64>,
    /// Override the duplication, in percent.
    #[arg(long)]
    duplicate_pct: Option<f64>,
    /// Override the reordering, in percent.
    #[arg(long)]
    reorder_pct: Option<f64>,
    /// Limit the bandwidth, in kilobytes per second.
    #[arg(long)]
    bandwidth_kbytes: Option<u64>,
    /// The tail-drop queue in bytes (with a bandwidth limit).
    #[arg(long)]
    queue_bytes: Option<usize>,
}

fn read(file: &PathBuf) -> anyhow::Result<Vec<u8>> {
    std::fs::read(file).with_context(|| format!("cannot read {}", file.display()))
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    match cli.command {
        Command::Replay(ReplayCommand::Inspect { file }) => {
            let report =
                inspect_replay(&read(&file)?).map_err(|e| anyhow!("{}: {e}", file.display()))?;
            print!("{}", report.render());
            Ok(ExitCode::SUCCESS)
        }
        Command::Replay(ReplayCommand::Verify { file }) => {
            let report =
                inspect_replay(&read(&file)?).map_err(|e| anyhow!("{}: {e}", file.display()))?;
            if report.is_complete() {
                println!(
                    "ok: complete, {} records, {} bytes",
                    report.records, report.file_bytes
                );
                Ok(ExitCode::SUCCESS)
            } else {
                print!("{}", report.render());
                Ok(ExitCode::from(1))
            }
        }
        Command::Replay(ReplayCommand::Dump { file, from, to }) => {
            let lines = dump_records(&read(&file)?, from, to)
                .map_err(|e| anyhow!("{}: {e}", file.display()))?;
            for line in lines {
                println!("{line}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Netsim(NetsimCommand::Presets) => {
            for (name, description) in PRESETS {
                println!("{name:<24}{description}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Netsim(NetsimCommand::Run(a)) => {
            let base = preset(&a.preset).ok_or_else(|| {
                let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
                anyhow!(
                    "unknown preset {:?}; the presets are: {}",
                    a.preset,
                    names.join(", ")
                )
            })?;
            let overrides = Overrides {
                latency_ms: a.latency_ms,
                jitter_ms: a.jitter_ms,
                loss_pct: a.loss_pct,
                duplicate_pct: a.duplicate_pct,
                reorder_pct: a.reorder_pct,
                bandwidth_kbytes_per_sec: a.bandwidth_kbytes,
                queue_limit_bytes: a.queue_bytes,
            };
            let link = overrides.apply(&base).map_err(|e| anyhow!(e))?;
            let scenario = Scenario {
                link,
                seed: a.seed,
                packets: a.packets,
                size_bytes: a.size,
                interval_us: a.interval_us,
            };
            let report = run_scenario(&scenario).map_err(|e| anyhow!(e))?;
            print!("{}", report.render());
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("net-tool: {e:#}");
            ExitCode::from(2)
        }
    }
}
