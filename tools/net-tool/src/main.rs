//! `net-tool` — inspect replay files and see what a bad network does to a packet stream.
//!
//! Exit codes: 0 success · 1 `replay verify` found a replay that is cut short or damaged after its
//! header · 2 error (unreadable file, not a replay, a value that makes no sense).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use clap::{Args, Parser, Subcommand};
use net_tool::soak::{SoakConfig, run_soak};
use net_tool::{Overrides, PRESETS, Scenario, dump_records, inspect_replay, preset, run_scenario};
use ss2_net::netsim::LinkConfig;

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
    /// Run headless bots against a session server for a number of virtual minutes, check the invariants, and exit 1 if any fails.
    Soak(SoakArgs),
}

/// The link: a profile and changes to it.
#[derive(Args)]
struct LinkArgs {
    /// A profile from `netsim presets`.
    #[arg(long, default_value = "broadband")]
    preset: String,
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

impl LinkArgs {
    fn link(&self) -> anyhow::Result<LinkConfig> {
        let base = preset(&self.preset).ok_or_else(|| {
            let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
            anyhow!(
                "unknown preset {:?}; the presets are: {}",
                self.preset,
                names.join(", ")
            )
        })?;
        let overrides = Overrides {
            latency_ms: self.latency_ms,
            jitter_ms: self.jitter_ms,
            loss_pct: self.loss_pct,
            duplicate_pct: self.duplicate_pct,
            reorder_pct: self.reorder_pct,
            bandwidth_kbytes_per_sec: self.bandwidth_kbytes,
            queue_limit_bytes: self.queue_bytes,
        };
        overrides.apply(&base).map_err(|e| anyhow!(e))
    }
}

/// Headless bots against a server over a conditioned link, in virtual time.
#[derive(Args)]
struct SoakArgs {
    #[command(flatten)]
    link: LinkArgs,
    /// How many bots (1 to 64).
    #[arg(long, default_value_t = 8)]
    bots: u16,
    /// Minutes of virtual time.
    #[arg(long, default_value_t = 5)]
    minutes: u32,
    /// The simulation tick rate.
    #[arg(long, default_value_t = 30)]
    tick_hz: u32,
    /// The random seed; the same seed gives the same report.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// How many ticks ahead of its estimate of the server's tick a bot stamps its input: it must cover the round trip, the jitter and one more tick.
    #[arg(long, default_value_t = 6)]
    lead_ticks: u64,
    /// The server's player slots.
    #[arg(long, default_value_t = 64)]
    max_players: u8,
    /// Every this many seconds one bot leaves and rejoins (0: never).
    #[arg(long, default_value_t = 0)]
    churn_seconds: u32,
    /// The share of ticks without an input above which the run fails (0 to 1).
    #[arg(long, default_value_t = 0.2)]
    max_missing: f64,
    /// Only session messages count as proof of life (as before `heard_from` existed): shows how a lossy link drops players whose game traffic flows.
    #[arg(long)]
    session_liveness_only: bool,
}

#[derive(Args)]
struct RunArgs {
    #[command(flatten)]
    link: LinkArgs,
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
        Command::Netsim(NetsimCommand::Soak(a)) => soak(a),
        Command::Netsim(NetsimCommand::Run(a)) => {
            let link = a.link.link()?;
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

fn soak(a: SoakArgs) -> anyhow::Result<ExitCode> {
    let config = SoakConfig {
        bots: a.bots,
        minutes: a.minutes,
        tick_hz: a.tick_hz,
        link: a.link.link()?,
        seed: a.seed,
        lead_ticks: a.lead_ticks,
        max_players: a.max_players,
        churn_every_seconds: a.churn_seconds,
        max_missing_fraction: a.max_missing,
        traffic_is_liveness: !a.session_liveness_only,
        corrupt_every: 0,
    };
    let report = run_soak(&config).map_err(|e| anyhow!(e))?;
    print!("{}", report.render(u32::from(a.bots)));
    let problems = report.problems(u32::from(a.bots));
    if problems.is_empty() {
        println!("result: passed");
        Ok(ExitCode::SUCCESS)
    } else {
        for p in &problems {
            println!("problem: {p}");
        }
        println!("result: FAILED");
        Ok(ExitCode::from(1))
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
