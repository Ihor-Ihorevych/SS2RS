//! `meta-dump`: print any CTSEMETA file as annotated text, JSON or RON.
//!
//! ```text
//! meta-dump <FILE>                                  a file on disk
//! meta-dump --install <DIR> content/some/file.ep    a file of the install, through the VFS
//! meta-dump --format json|ron <FILE>                structured output
//! meta-dump --raw <FILE>                            add the raw bytes of every region
//! meta-dump --unknown <FILE>                        only the coverage and the undecoded ranges
//! ```
//!
//! Exit codes: 0 success · 1 `--unknown` found undecoded bytes · 2 error (unreadable file,
//! damaged frame, usage).

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use clap::{Parser, ValueEnum};
use meta_dump::{Format, Options, dump};
use ss2_vfs::{Vfs, VfsPath, mount_install};

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Text,
    Json,
    Ron,
}

#[derive(Parser)]
#[command(
    name = "meta-dump",
    version,
    about = "Print any CTSEMETA file as annotated text, JSON or RON, with byte offsets"
)]
struct Cli {
    /// A file on disk, or with --install the path of a file inside the install.
    path: String,
    /// Read PATH through the VFS of this install (the folder with All_PC_01.gro) instead of from
    /// disk; the newest patch wins, as in the game.
    #[arg(long, value_name = "DIR")]
    install: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value = "text")]
    format: FormatArg,
    /// Add the first bytes of every region it prints.
    #[arg(long)]
    raw: bool,
    /// Print only the coverage and the byte ranges no decoder explained (exit 1 if there are any).
    #[arg(long)]
    unknown: bool,
    /// Do not print decoded object values (offsets, sizes and types stay).
    #[arg(long)]
    skip_values: bool,
    /// How many bytes a hex preview shows.
    #[arg(long, default_value_t = 32, value_name = "N")]
    preview: usize,
}

fn read_input(cli: &Cli) -> anyhow::Result<(String, Vec<u8>)> {
    if let Some(dir) = &cli.install {
        let vfs = mount_install(dir).with_context(|| format!("cannot mount {}", dir.display()))?;
        let path = VfsPath::new(&cli.path)?;
        let bytes = vfs
            .read(&path)
            .with_context(|| format!("cannot read {} from the install", path.as_str()))?;
        Ok((path.as_str().to_string(), bytes))
    } else {
        let bytes =
            std::fs::read(&cli.path).with_context(|| format!("cannot read {}", cli.path))?;
        Ok((cli.path.clone(), bytes))
    }
}

fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    let (label, bytes) = read_input(cli)?;
    let opts = Options {
        preview: cli.preview,
        raw: cli.raw,
        skip_values: cli.skip_values,
        unknown_only: cli.unknown,
    };
    let format = match cli.format {
        FormatArg::Text => Format::Text,
        FormatArg::Json => Format::Json,
        FormatArg::Ron => Format::Ron,
    };
    let dumped = dump(&bytes, &label, format, &opts);
    let mut out = io::stdout().lock();
    out.write_all(dumped.text.as_bytes())
        .and_then(|()| out.flush())
        .context("cannot write to standard output")?;
    if !dumped.parsed {
        return Err(anyhow!(
            "{label}: the CTSEMETA frame could not be read (see the report above)"
        ));
    }
    Ok(if cli.unknown && dumped.undecoded_bytes > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.chain()
        .filter_map(|c| c.downcast_ref::<io::Error>())
        .any(|io| io.kind() == io::ErrorKind::BrokenPipe)
}

fn main() -> ExitCode {
    match run(&Cli::parse()) {
        Ok(code) => code,
        Err(e) if is_broken_pipe(&e) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("meta-dump: {e:#}");
            ExitCode::from(2)
        }
    }
}
