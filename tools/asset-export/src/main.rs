//! `asset-export`: textures to PNG, models to glTF 2.0.
//!
//! ```text
//! asset-export tex  <FILE> -o out.png                          one texture file
//! asset-export tex  --install DIR <VFS PATH> -o out.png        one texture of the install
//! asset-export tex  --all <FOLDER> -o OUTDIR [--install DIR]   a whole folder + manifest
//! asset-export sheet <FILES...> -o sheet.png                   contact sheet (+ sheet.json)
//! asset-export mesh <FILE> -o out.glb | out.gltf               one model
//! asset-export mesh --all <FOLDER> -o OUTDIR [--install DIR]   a whole folder + manifest
//! ```
//!
//! Exit codes: 0 success · 1 a batch or sheet finished but some files failed · 2 error (usage,
//! unreadable or damaged input, unwritable output) · 3 the single input is of a kind the readers
//! do not handle yet (the old development-build textures).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, anyhow, bail};
use asset_export::batch::{BatchSettings, Kind, Options, Status, convert, export_all, write_file};
use asset_export::error::ExportError;
use asset_export::mesh::{GltfFormat, LodSel, MeshOptions};
use asset_export::sheet::export_sheet;
use asset_export::source::Source;
use asset_export::tex::{Channels, FaceSel, TexOptions};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "asset-export",
    version,
    about = "Export textures to PNG and models to glTF 2.0, one file, a contact sheet or a whole install"
)]
struct Cli {
    /// Read inputs through the VFS of this install (the folder with All_PC_01.gro) instead of
    /// from disk; the newest patch wins, as in the game.
    #[arg(long, value_name = "DIR", global = true)]
    install: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export a `.tex` texture to PNG.
    Tex(TexArgs),
    /// Make a contact sheet PNG of several textures.
    Sheet(SheetArgs),
    /// Export a `.bmf` model to glTF 2.0.
    Mesh(MeshArgs),
}

#[derive(Args)]
struct Select {
    /// With --all or --folder: only files whose path matches this wildcard (`*` any run, `?` one
    /// character), for example `content/*/menu/*`.
    #[arg(long, value_name = "GLOB")]
    filter: Option<String>,
    /// With --all or --folder: stop after this many files (sorted order).
    #[arg(long, value_name = "N")]
    limit: Option<usize>,
}

#[derive(Args)]
struct TexSel {
    /// Mip level to write (0 is the largest; only DXT textures have more than one).
    #[arg(long, default_value_t = 0, value_name = "N")]
    mip: u32,
    /// Face of a cube map: a number 0-5, or `all` for the six faces side by side.
    #[arg(long, default_value = "0", value_name = "N|all")]
    face: String,
}

impl TexSel {
    fn face(&self) -> anyhow::Result<FaceSel> {
        if self.face.eq_ignore_ascii_case("all") {
            return Ok(FaceSel::All);
        }
        self.face
            .parse::<u32>()
            .map(FaceSel::One)
            .map_err(|_| anyhow!("--face must be a number or `all`, not `{}`", self.face))
    }
}

#[derive(Args)]
struct TexArgs {
    /// The texture: a file, or with --install a path inside the install. Not used with --all.
    input: Option<String>,
    /// Output: a PNG file, or with --all a folder.
    #[arg(short, long, value_name = "PATH")]
    output: PathBuf,
    /// Export every `.tex` below this folder (a VFS folder with --install, `/` for all of it;
    /// else a directory) into the output folder, mirroring the tree, and write
    /// `manifest-texture.json` there.
    #[arg(long, value_name = "FOLDER")]
    all: Option<String>,
    #[command(flatten)]
    sel: TexSel,
    /// Drop the alpha channel (colours are not changed).
    #[arg(long, conflicts_with_all = ["alpha_only", "rgba"])]
    rgb_only: bool,
    /// Write only the alpha channel, as a grayscale image.
    #[arg(long, conflicts_with = "rgba")]
    alpha_only: bool,
    /// Always write RGBA (the default is RGB for 24-bit textures and for DXT1 without
    /// transparent pixels, RGBA otherwise).
    #[arg(long)]
    rgba: bool,
    #[command(flatten)]
    select: Select,
    /// Convert but write nothing (the manifest is printed, not written).
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct SheetArgs {
    /// The textures: files, or with --install paths inside the install.
    inputs: Vec<String>,
    /// Output PNG; a JSON list of the cells is written next to it with the extension `.json`.
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,
    /// Also take every `.tex` below this folder.
    #[arg(long, value_name = "FOLDER")]
    folder: Option<String>,
    /// Cell size in pixels.
    #[arg(long, default_value_t = 128, value_name = "PX")]
    cell: u32,
    /// Cells per row (default: about a square sheet).
    #[arg(long, value_name = "N")]
    columns: Option<u32>,
    #[command(flatten)]
    sel: TexSel,
    #[command(flatten)]
    select: Select,
}

#[derive(Args)]
struct MeshArgs {
    /// The model: a file, or with --install a path inside the install. Not used with --all.
    input: Option<String>,
    /// Output: a `.glb` or `.gltf` file (by extension), or with --all a folder.
    #[arg(short, long, value_name = "PATH")]
    output: PathBuf,
    /// Export every `.bmf` below this folder into the output folder, mirroring the tree, and
    /// write `manifest-mesh.json` there.
    #[arg(long, value_name = "FOLDER")]
    all: Option<String>,
    /// With --all: `glb` (default) or `gltf`.
    #[arg(long, value_name = "glb|gltf")]
    format: Option<String>,
    /// LOD to write (0 is the nearest).
    #[arg(
        long,
        default_value_t = 0,
        value_name = "N",
        conflicts_with = "all_lods"
    )]
    lod: u32,
    /// Write every LOD, each as its own node.
    #[arg(long)]
    all_lods: bool,
    /// Swap the winding of every triangle (the engine's winding is not confirmed).
    #[arg(long)]
    flip_winding: bool,
    /// Write normals and tangents exactly as stored (by default zero or non-unit ones are
    /// repaired, because strict readers reject them).
    #[arg(long)]
    raw_vectors: bool,
    #[command(flatten)]
    select: Select,
    /// Convert but write nothing.
    #[arg(long)]
    dry_run: bool,
}

fn source(cli: &Cli) -> anyhow::Result<Source> {
    match &cli.install {
        Some(dir) => Source::install(dir).map_err(Into::into),
        None => Ok(Source::Disk),
    }
}

fn channels(args: &TexArgs) -> Channels {
    if args.rgb_only {
        Channels::RgbOnly
    } else if args.alpha_only {
        Channels::AlphaOnly
    } else if args.rgba {
        Channels::Rgba
    } else {
        Channels::Auto
    }
}

/// How the exit code follows from one failed conversion.
fn failure_code(e: &ExportError) -> ExitCode {
    match e {
        ExportError::Unsupported(_) => ExitCode::from(3),
        _ => ExitCode::from(2),
    }
}

fn print_summary(m: &asset_export::batch::Manifest, manifest_path: Option<&Path>) {
    let s = m.summary();
    eprintln!(
        "{}: {} found, {} exported, {} unsupported, {} failed; {} bytes in, {} bytes out",
        m.kind.name(),
        s.found,
        s.exported,
        s.unsupported,
        s.failed,
        s.bytes_in,
        s.bytes_out
    );
    for (reason, n) in m.reasons() {
        eprintln!("  {n} x {reason}");
    }
    if let Some(p) = manifest_path {
        eprintln!("manifest: {}", p.display());
    }
}

fn run_batch(
    cli: &Cli,
    kind: Kind,
    folder: &str,
    out_dir: &Path,
    opts: &Options,
    select: &Select,
    dry_run: bool,
) -> anyhow::Result<ExitCode> {
    let src = source(cli)?;
    let settings = BatchSettings {
        filter: select.filter.clone(),
        limit: select.limit,
        dry_run,
    };
    let mut done = 0usize;
    let manifest = export_all(&src, kind, folder, out_dir, opts, &settings, &mut |e| {
        done += 1;
        if e.status == Status::Failed {
            eprintln!(
                "failed: {}: {}",
                e.source,
                e.error.as_deref().unwrap_or("?")
            );
        }
        if done.is_multiple_of(250) {
            eprintln!("... {done} files");
        }
    })?;
    let text = manifest.to_json(opts);
    let path = out_dir.join(format!("manifest-{}.json", kind.name()));
    if dry_run {
        print!("{text}");
        print_summary(&manifest, None);
    } else {
        write_file(&path, text.as_bytes())?;
        print_summary(&manifest, Some(&path));
    }
    Ok(if manifest.summary().failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn run_single(
    cli: &Cli,
    kind: Kind,
    input: &str,
    output: &Path,
    opts: &Options,
) -> anyhow::Result<ExitCode> {
    let src = source(cli)?;
    let entry = src.entry(input)?;
    let bytes = match src.read(&entry) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("asset-export: {e}");
            return Ok(ExitCode::from(2));
        }
    };
    match convert(kind, &bytes, &entry, &src, opts) {
        Ok(c) => {
            write_file(output, &c.bytes)?;
            eprintln!(
                "{} -> {} ({} bytes) {}",
                entry.key,
                output.display(),
                c.bytes.len(),
                c.details
            );
            for w in &c.warnings {
                eprintln!("warning: {w}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            eprintln!("asset-export: {}: {e}", entry.key);
            Ok(failure_code(&e))
        }
    }
}

fn run_tex(cli: &Cli, args: &TexArgs) -> anyhow::Result<ExitCode> {
    let opts = Options {
        tex: TexOptions {
            mip: args.sel.mip,
            face: args.sel.face()?,
            channels: channels(args),
        },
        ..Options::default()
    };
    match (&args.all, &args.input) {
        (Some(folder), None) => run_batch(
            cli,
            Kind::Texture,
            folder,
            &args.output,
            &opts,
            &args.select,
            args.dry_run,
        ),
        (None, Some(input)) => run_single(cli, Kind::Texture, input, &args.output, &opts),
        _ => bail!("give either one INPUT or --all FOLDER"),
    }
}

fn run_mesh(cli: &Cli, args: &MeshArgs) -> anyhow::Result<ExitCode> {
    let ext = args
        .output
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());
    let format = match (&args.all, args.format.as_deref(), ext.as_deref()) {
        (_, Some("glb"), _) | (None, None, Some("glb")) | (Some(_), None, _) => GltfFormat::Glb,
        (_, Some("gltf"), _) | (None, None, Some("gltf")) => GltfFormat::Gltf,
        (None, None, _) => bail!("the output file must end in .glb or .gltf"),
        (_, Some(other), _) => bail!("--format must be glb or gltf, not `{other}`"),
    };
    let opts = Options {
        mesh: MeshOptions {
            lods: if args.all_lods {
                LodSel::All
            } else {
                LodSel::One(args.lod)
            },
            format,
            flip_winding: args.flip_winding,
            raw_vectors: args.raw_vectors,
        },
        ..Options::default()
    };
    match (&args.all, &args.input) {
        (Some(folder), None) => run_batch(
            cli,
            Kind::Mesh,
            folder,
            &args.output,
            &opts,
            &args.select,
            args.dry_run,
        ),
        (None, Some(input)) => run_single(cli, Kind::Mesh, input, &args.output, &opts),
        _ => bail!("give either one INPUT or --all FOLDER"),
    }
}

fn run_sheet(cli: &Cli, args: &SheetArgs) -> anyhow::Result<ExitCode> {
    let src = source(cli)?;
    let mut files = Vec::new();
    for input in &args.inputs {
        files.push(src.entry(input)?);
    }
    if let Some(folder) = &args.folder {
        let mut listed = src.list(folder, &["tex"])?;
        if let Some(f) = &args.select.filter {
            listed.retain(|e| asset_export::glob::matches(f, &e.key));
        }
        files.extend(listed);
    }
    if let Some(n) = args.select.limit {
        files.truncate(n);
    }
    if files.is_empty() {
        bail!("no textures: name some files or use --folder");
    }
    let columns = args
        .columns
        .unwrap_or_else(|| (files.len() as f64).sqrt().ceil() as u32);
    let tex = TexOptions {
        mip: args.sel.mip,
        face: args.sel.face()?,
        channels: Channels::RgbOnly,
    };
    let sheet = export_sheet(&src, &files, &tex, args.cell, columns)?;
    write_file(&args.output, &sheet.png)?;
    let index_path = args.output.with_extension("json");
    let mut text = serde_json::to_string_pretty(&sheet.index).context("writing the cell list")?;
    text.push('\n');
    write_file(&index_path, text.as_bytes())?;
    eprintln!(
        "{} textures on the sheet, {} left out; {} and {}",
        sheet.placed,
        sheet.skipped,
        args.output.display(),
        index_path.display()
    );
    if sheet.placed == 0 {
        return Ok(ExitCode::from(2));
    }
    Ok(if sheet.skipped > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Tex(a) => run_tex(&cli, a),
        Command::Sheet(a) => run_sheet(&cli, a),
        Command::Mesh(a) => run_mesh(&cli, a),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("asset-export: {e:#}");
            ExitCode::from(2)
        }
    }
}
