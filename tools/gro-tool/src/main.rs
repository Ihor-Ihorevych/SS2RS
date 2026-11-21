//! `gro-tool` — look inside a Serious Sam 2 install without extracting anything first.
//!
//! The install folder (the one containing `All_PC_01.gro`) comes from `--data` or the
//! `SS2_DATA_DIR` environment variable. All archives are mounted together in patch order, so
//! what you see is what the game sees: the newest patch wins. Paths are shown normalized
//! (lower-case, `/`).
//!
//! Exit codes: 0 success · 1 nothing matched (like `grep`) · 2 error.

use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use gro_tool::{Matcher, extension_stats, extract_file, format_size, walk_files};
use ss2_vfs::{LayeredVfs, Vfs, VfsPath, mount_install};

#[derive(Parser)]
#[command(
    name = "gro-tool",
    version,
    about = "List, search, print and extract files from a Serious Sam 2 install"
)]
struct Cli {
    /// Folder containing the .gro archives (the one with All_PC_01.gro).
    #[arg(long, env = "SS2_DATA_DIR", global = true)]
    data: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List files (optionally only those matching a glob such as `*.tex` or `content/**/*.rsc`).
    List {
        /// Glob (default) or regex; without it every file is listed.
        pattern: Option<String>,
        /// Treat the pattern as a regular expression searched in the full path.
        #[arg(long)]
        regex: bool,
        /// Show size and the archive that provides each file.
        #[arg(long)]
        long: bool,
    },
    /// Like `list --long` but the pattern is required; exits 1 when nothing matches.
    Find {
        pattern: String,
        #[arg(long)]
        regex: bool,
    },
    /// Print the archive that provides a file (the newest patch that contains it).
    Where { path: String },
    /// Write a file's bytes to standard output.
    Cat { path: String },
    /// Extract matching files below a folder, keeping their paths.
    Extract {
        /// One or more globs (or regexes with --regex).
        #[arg(required = true)]
        patterns: Vec<String>,
        /// Output folder (created if needed).
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        regex: bool,
        /// Overwrite files that already exist.
        #[arg(long)]
        force: bool,
    },
    /// File counts and sizes by extension.
    Stats,
}

/// What a successful command found.
enum Outcome {
    Done,
    NothingMatched,
}

fn matcher(pattern: &str, regex: bool) -> anyhow::Result<Matcher> {
    if regex {
        Matcher::regex(pattern)
    } else {
        Matcher::glob(pattern)
    }
}

fn mount(data: Option<PathBuf>) -> anyhow::Result<LayeredVfs> {
    let dir = data.ok_or_else(|| {
        anyhow!("no game folder: pass --data <folder containing All_PC_01.gro> or set SS2_DATA_DIR")
    })?;
    Ok(mount_install(&dir)?)
}

fn run(cli: Cli) -> anyhow::Result<Outcome> {
    let vfs = mount(cli.data)?;
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    match cli.command {
        Command::List {
            pattern,
            regex,
            long,
        } => {
            let m = pattern.as_deref().map(|p| matcher(p, regex)).transpose()?;
            let mut shown = 0usize;
            for f in walk_files(&vfs)? {
                if m.as_ref().is_some_and(|m| !m.is_match(&f.path)) {
                    continue;
                }
                shown += 1;
                if long {
                    let source = vfs.source_of(&f.path).unwrap_or("?");
                    writeln!(out, "{:>12}  {:<26}  {}", f.size, source, f.path)?;
                } else {
                    writeln!(out, "{}", f.path)?;
                }
            }
            out.flush()?;
            Ok(if shown == 0 && pattern.is_some() {
                Outcome::NothingMatched
            } else {
                Outcome::Done
            })
        }
        Command::Find { pattern, regex } => {
            let m = matcher(&pattern, regex)?;
            let mut shown = 0usize;
            for f in walk_files(&vfs)?
                .into_iter()
                .filter(|f| m.is_match(&f.path))
            {
                shown += 1;
                let source = vfs.source_of(&f.path).unwrap_or("?");
                writeln!(out, "{:>12}  {:<26}  {}", f.size, source, f.path)?;
            }
            out.flush()?;
            Ok(if shown == 0 {
                Outcome::NothingMatched
            } else {
                Outcome::Done
            })
        }
        Command::Where { path } => match vfs.source_of(&VfsPath::new(&path)?) {
            Some(source) => {
                writeln!(out, "{source}")?;
                out.flush()?;
                Ok(Outcome::Done)
            }
            None => Ok(Outcome::NothingMatched),
        },
        Command::Cat { path } => {
            let mut file = vfs.open(&VfsPath::new(&path)?)?;
            io::copy(&mut file, &mut out).context("cannot write to standard output")?;
            out.flush()?;
            Ok(Outcome::Done)
        }
        Command::Extract {
            patterns,
            out: dir,
            regex,
            force,
        } => {
            let matchers = patterns
                .iter()
                .map(|p| matcher(p, regex))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let files: Vec<_> = walk_files(&vfs)?
                .into_iter()
                .filter(|f| matchers.iter().any(|m| m.is_match(&f.path)))
                .collect();
            if files.is_empty() {
                return Ok(Outcome::NothingMatched);
            }
            for f in &files {
                let written = extract_file(&vfs, &f.path, &dir, force)?;
                writeln!(out, "{}", written.display())?;
            }
            out.flush()?;
            eprintln!("extracted {} file(s) to {}", files.len(), dir.display());
            Ok(Outcome::Done)
        }
        Command::Stats => {
            let files = walk_files(&vfs)?;
            let total: u64 = files.iter().map(|f| f.size).sum();
            writeln!(out, "archives: {}", vfs.layer_names().len())?;
            writeln!(out, "files: {} ({})", files.len(), format_size(total))?;
            writeln!(out, "{:<12} {:>8} {:>12}", "extension", "files", "size")?;
            for s in extension_stats(&files).iter().take(25) {
                let ext = if s.ext == "(none)" {
                    s.ext.clone()
                } else {
                    format!(".{}", s.ext)
                };
                writeln!(
                    out,
                    "{:<12} {:>8} {:>12}",
                    ext,
                    s.files,
                    format_size(s.bytes)
                )?;
            }
            out.flush()?;
            Ok(Outcome::Done)
        }
    }
}

/// `gro-tool list | head` closes the pipe early; that is not an error.
fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.chain()
        .filter_map(|c| c.downcast_ref::<io::Error>())
        .any(|io| io.kind() == io::ErrorKind::BrokenPipe)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(Outcome::Done) => ExitCode::SUCCESS,
        Ok(Outcome::NothingMatched) => ExitCode::from(1),
        Err(e) if is_broken_pipe(&e) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gro-tool: {e:#}");
            ExitCode::from(2)
        }
    }
}
