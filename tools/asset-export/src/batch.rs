//! One conversion, a whole-folder batch and its manifest.
//!
//! The batch walks a folder (of an install or on disk) in sorted order, converts every `.tex`
//! (PNG) or `.bmf` (glTF) it finds, mirrors the tree under the output folder, and records every
//! file in a manifest: exported, unsupported (a known gap, for example the old development
//! textures) or failed, with the reason. One bad file never stops the run.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::{ExportError, Result};
use crate::glob;
use crate::mesh::{GltfFormat, MeshContext, MeshOptions, Resolver, export_mesh};
use crate::source::{Entry, Source};
use crate::tex::{FaceSel, TexOptions, export_texture};

/// What is exported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `.tex` to PNG.
    Texture,
    /// `.bmf` to glTF.
    Mesh,
}

impl Kind {
    /// The input extension.
    pub fn input_extension(self) -> &'static str {
        match self {
            Self::Texture => "tex",
            Self::Mesh => "bmf",
        }
    }

    /// The name used in manifests.
    pub fn name(self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::Mesh => "mesh",
        }
    }
}

/// All options of a conversion.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Texture options.
    pub tex: TexOptions,
    /// Mesh options.
    pub mesh: MeshOptions,
}

/// The result of converting one file.
#[derive(Clone, Debug)]
pub struct Converted {
    /// The output file.
    pub bytes: Vec<u8>,
    /// The extension of the output file (no dot).
    pub extension: &'static str,
    /// What is in it, for manifests and reports.
    pub details: Value,
    /// Warnings of the conversion.
    pub warnings: Vec<String>,
}

/// Convert the bytes of one `.tex` or `.bmf` file named `entry`.
///
/// # Errors
/// Any [`ExportError`] of the readers and writers; never a panic.
pub fn convert(
    kind: Kind,
    bytes: &[u8],
    entry: &Entry,
    resolver: &dyn Resolver,
    opts: &Options,
) -> Result<Converted> {
    match kind {
        Kind::Texture => {
            let out = export_texture(bytes, &opts.tex)?;
            let i = &out.info;
            Ok(Converted {
                bytes: out.png,
                extension: "png",
                details: json!({
                    "format": format!("{:?}", i.format),
                    "source_size": [i.source_width, i.source_height],
                    "mip_levels": i.mip_levels,
                    "faces": i.faces,
                    "size": [i.width, i.height],
                    "png_alpha": i.png_has_alpha,
                }),
                warnings: Vec::new(),
            })
        }
        Kind::Mesh => {
            let ctx = MeshContext {
                name: entry.stem(),
                dir: entry.dir(),
                resolver,
            };
            let out = export_mesh(bytes, &ctx, &opts.mesh)?;
            let i = &out.info;
            Ok(Converted {
                bytes: out.bytes,
                extension: match opts.mesh.format {
                    GltfFormat::Gltf => "gltf",
                    GltfFormat::Glb => "glb",
                },
                details: json!({
                    "lods_in_file": i.lods_in_file,
                    "lods_written": i.lods_written,
                    "primitives": i.primitives,
                    "vertices": i.vertices,
                    "triangles": i.triangles,
                    "materials": i.materials,
                    "textures": i.textures,
                    "skinned_surfaces": i.skinned_surfaces,
                    "repaired_normals": i.repaired_normals,
                    "repaired_tangents": i.repaired_tangents,
                }),
                warnings: i.warnings.clone(),
            })
        }
    }
}

/// How one file ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Written.
    Exported,
    /// A known gap: the file is of a kind the readers do not handle yet.
    Unsupported,
    /// Anything else that went wrong.
    Failed,
}

impl Status {
    fn name(self) -> &'static str {
        match self {
            Self::Exported => "exported",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
        }
    }
}

/// One line of the manifest.
#[derive(Clone, Debug)]
pub struct ManifestEntry {
    /// The input key.
    pub source: String,
    /// How it ended.
    pub status: Status,
    /// The output path relative to the output folder.
    pub output: Option<String>,
    /// Input size.
    pub bytes_in: u64,
    /// Output size.
    pub bytes_out: u64,
    /// Error class (`read`, `format`, ...), for `Unsupported` and `Failed`.
    pub error_kind: Option<&'static str>,
    /// The reason, for `Unsupported` and `Failed`.
    pub error: Option<String>,
    /// Facts about the conversion (formats, sizes, counts).
    pub details: Value,
    /// Warnings.
    pub warnings: Vec<String>,
}

impl ManifestEntry {
    fn to_json(&self) -> Value {
        let mut v = json!({ "source": self.source, "status": self.status.name() });
        if let Some(o) = &self.output {
            v["output"] = json!(o);
            v["bytes_out"] = json!(self.bytes_out);
        }
        v["bytes_in"] = json!(self.bytes_in);
        if let (Some(k), Some(e)) = (self.error_kind, &self.error) {
            v["error_kind"] = json!(k);
            v["error"] = json!(e);
        }
        if !self.details.is_null() {
            v["details"] = self.details.clone();
        }
        if !self.warnings.is_empty() {
            v["warnings"] = json!(self.warnings);
        }
        v
    }
}

/// What a batch run found and did.
#[derive(Clone, Debug)]
pub struct Manifest {
    /// The kind of file exported.
    pub kind: Kind,
    /// The folder that was walked: its install path (`""` for the whole install), or `"."` for
    /// a folder on disk (an absolute path would make the manifest differ between machines).
    pub folder: String,
    /// The filter, if any.
    pub filter: Option<String>,
    /// One entry per file, in sorted source order.
    pub entries: Vec<ManifestEntry>,
}

/// Counts of a manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Files found.
    pub found: usize,
    /// Written.
    pub exported: usize,
    /// Known gaps.
    pub unsupported: usize,
    /// Failures.
    pub failed: usize,
    /// Input bytes of all files that were read.
    pub bytes_in: u64,
    /// Output bytes written.
    pub bytes_out: u64,
}

impl Manifest {
    /// The totals.
    pub fn summary(&self) -> Summary {
        let mut s = Summary {
            found: self.entries.len(),
            ..Summary::default()
        };
        for e in &self.entries {
            match e.status {
                Status::Exported => s.exported += 1,
                Status::Unsupported => s.unsupported += 1,
                Status::Failed => s.failed += 1,
            }
            s.bytes_in += e.bytes_in;
            s.bytes_out += e.bytes_out;
        }
        s
    }

    /// Count of each distinct reason among the unsupported and failed files.
    pub fn reasons(&self) -> BTreeMap<String, usize> {
        let mut m = BTreeMap::new();
        for e in &self.entries {
            if let (Status::Unsupported | Status::Failed, Some(reason)) = (e.status, &e.error) {
                let short: String = reason.chars().take(160).collect();
                *m.entry(format!("{}: {short}", e.status.name()))
                    .or_default() += 1;
            }
        }
        m
    }

    /// The manifest as pretty JSON (deterministic: no time stamps, no absolute paths).
    pub fn to_json(&self, opts: &Options) -> String {
        let s = self.summary();
        let options = match self.kind {
            Kind::Texture => json!({
                "mip": opts.tex.mip,
                "face": match opts.tex.face { FaceSel::One(n) => json!(n), FaceSel::All => json!("all") },
                "channels": format!("{:?}", opts.tex.channels),
            }),
            Kind::Mesh => json!({
                "lods": match opts.mesh.lods {
                    crate::mesh::LodSel::One(n) => json!(n),
                    crate::mesh::LodSel::All => json!("all"),
                },
                "format": format!("{:?}", opts.mesh.format),
                "flip_winding": opts.mesh.flip_winding,
                "raw_vectors": opts.mesh.raw_vectors,
            }),
        };
        let v = json!({
            "tool": concat!("asset-export ", env!("CARGO_PKG_VERSION")),
            "kind": self.kind.name(),
            "folder": self.folder,
            "filter": self.filter,
            "options": options,
            "summary": {
                "found": s.found,
                "exported": s.exported,
                "unsupported": s.unsupported,
                "failed": s.failed,
                "bytes_in": s.bytes_in,
                "bytes_out": s.bytes_out,
            },
            "reasons": self.reasons(),
            "entries": self.entries.iter().map(ManifestEntry::to_json).collect::<Vec<_>>(),
        });
        let mut text = serde_json::to_string_pretty(&v).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// Settings of a batch run.
#[derive(Clone, Debug, Default)]
pub struct BatchSettings {
    /// Only files whose key matches this wildcard.
    pub filter: Option<String>,
    /// At most this many files (after sorting and filtering).
    pub limit: Option<usize>,
    /// Do not write any file, only convert and report (the manifest is still returned).
    pub dry_run: bool,
}

/// Write `bytes` to `path`, creating the folders.
///
/// # Errors
/// [`ExportError::Write`].
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ExportError::write(path.display(), e))?;
    }
    std::fs::write(path, bytes).map_err(|e| ExportError::write(path.display(), e))
}

/// Export every file of `kind` below `folder` into `out_dir`, mirroring the tree. Calls
/// `progress` after each file.
///
/// # Errors
/// Only when the folder cannot be listed; per-file problems are manifest entries.
pub fn export_all(
    source: &Source,
    kind: Kind,
    folder: &str,
    out_dir: &Path,
    opts: &Options,
    settings: &BatchSettings,
    progress: &mut dyn FnMut(&ManifestEntry),
) -> Result<Manifest> {
    let mut files = source.list(folder, &[kind.input_extension()])?;
    if let Some(f) = &settings.filter {
        files.retain(|e| glob::matches(f, &e.key));
    }
    if let Some(n) = settings.limit {
        files.truncate(n);
    }
    let mut entries = Vec::with_capacity(files.len());
    for file in &files {
        let entry = export_one(source, kind, file, out_dir, opts, settings.dry_run);
        progress(&entry);
        entries.push(entry);
    }
    Ok(Manifest {
        kind,
        folder: match source {
            Source::Disk => ".".to_string(),
            Source::Install(_) => folder.trim_matches(&['/', '\\'][..]).to_string(),
        },
        filter: settings.filter.clone(),
        entries,
    })
}

/// How an error is counted: only [`ExportError::Unsupported`] is a known gap, the rest failures.
pub fn status_of(e: &ExportError) -> Status {
    match e {
        ExportError::Unsupported(_) => Status::Unsupported,
        _ => Status::Failed,
    }
}

fn export_one(
    source: &Source,
    kind: Kind,
    file: &Entry,
    out_dir: &Path,
    opts: &Options,
    dry_run: bool,
) -> ManifestEntry {
    let mut entry = ManifestEntry {
        source: file.key.clone(),
        status: Status::Failed,
        output: None,
        bytes_in: 0,
        bytes_out: 0,
        error_kind: None,
        error: None,
        details: Value::Null,
        warnings: Vec::new(),
    };
    let result = source.read(file).and_then(|bytes| {
        entry.bytes_in = bytes.len() as u64;
        convert(kind, &bytes, file, source, opts)
    });
    let converted = match result {
        Ok(c) => c,
        Err(e) => {
            entry.status = status_of(&e);
            entry.error_kind = Some(e.kind());
            entry.error = Some(e.to_string());
            return entry;
        }
    };
    let relative = file.with_extension(converted.extension);
    if !dry_run && let Err(e) = write_file(&out_dir.join(&relative), &converted.bytes) {
        entry.error_kind = Some(e.kind());
        entry.error = Some(e.to_string());
        return entry;
    }
    entry.status = Status::Exported;
    entry.output = Some(relative);
    entry.bytes_out = converted.bytes.len() as u64;
    entry.details = converted.details;
    entry.warnings = converted.warnings;
    entry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;

    /// A folder with good and bad files of both kinds.
    fn sample_tree(root: &Path) {
        let put = |name: &str, bytes: &[u8]| {
            let p = root.join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        };
        put("Textures/B_dxt1.tex", &dxt1_file());
        put("Textures/a_bgr.TEX", &bgr_file());
        put("Textures/Deep/c_bgra.tex", &bgra_file());
        put("Textures/broken.tex", b"not a texture");
        put("Models/quad.bmf", &quad_file());
        put("Models/rich.bmf", &rich_file());
        put("Models/broken.bmf", &quad_file()[..40]);
        put("readme.txt", b"ignored");
    }

    fn run(
        root: &Path,
        out: &Path,
        kind: Kind,
        settings: &BatchSettings,
    ) -> Result<(Manifest, Vec<String>)> {
        let mut seen = Vec::new();
        let m = export_all(
            &Source::Disk,
            kind,
            root.to_str().unwrap(),
            out,
            &default(),
            settings,
            &mut |e| seen.push(e.source.clone()),
        )?;
        Ok((m, seen))
    }

    fn default() -> Options {
        Options::default()
    }

    #[test]
    fn a_texture_batch_mirrors_the_tree_and_records_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let (src, out) = (dir.path().join("in"), dir.path().join("out"));
        sample_tree(&src);
        let (m, seen) = run(&src, &out, Kind::Texture, &BatchSettings::default()).unwrap();
        // Sorted, lower case, only .tex files; the progress callback saw them in the same order.
        let sources: Vec<_> = m.entries.iter().map(|e| e.source.as_str()).collect();
        assert_eq!(
            sources,
            [
                "textures/a_bgr.tex",
                "textures/b_dxt1.tex",
                "textures/broken.tex",
                "textures/deep/c_bgra.tex"
            ]
        );
        assert_eq!(seen, sources);
        let s = m.summary();
        assert_eq!((s.found, s.exported, s.unsupported, s.failed), (4, 3, 0, 1));
        assert_eq!(m.entries[2].status, Status::Failed);
        assert_eq!(m.entries[2].error_kind, Some("format"));
        assert!(m.entries[2].error.as_deref().unwrap().contains("CTSEMETA"));
        assert_eq!(m.entries[2].bytes_in, 13);
        assert!(m.entries[2].output.is_none());
        // The tree is mirrored with .png files that decode.
        for rel in [
            "textures/a_bgr.png",
            "textures/b_dxt1.png",
            "textures/deep/c_bgra.png",
        ] {
            let bytes = std::fs::read(out.join(rel)).unwrap();
            assert!(bytes.starts_with(b"\x89PNG"), "{rel}");
        }
        assert!(!out.join("textures/broken.png").exists());
        assert_eq!(m.entries[0].output.as_deref(), Some("textures/a_bgr.png"));
        assert_eq!(
            s.bytes_out,
            m.entries.iter().map(|e| e.bytes_out).sum::<u64>()
        );
        assert_eq!(
            m.entries[0].bytes_out,
            std::fs::metadata(out.join("textures/a_bgr.png"))
                .unwrap()
                .len()
        );
        assert_eq!(m.entries[0].details["format"], "Bgr8");
        assert_eq!(m.entries[1].details["mip_levels"], 3);
        let reasons = m.reasons();
        assert_eq!(reasons.len(), 1);
        assert!(reasons.keys().next().unwrap().starts_with("failed: "));
    }

    #[test]
    fn a_mesh_batch_writes_glb_or_gltf_and_notes_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let (src, out) = (dir.path().join("in"), dir.path().join("out"));
        sample_tree(&src);
        let (m, _) = run(&src, &out, Kind::Mesh, &BatchSettings::default()).unwrap();
        let sources: Vec<_> = m.entries.iter().map(|e| e.source.as_str()).collect();
        assert_eq!(
            sources,
            ["models/broken.bmf", "models/quad.bmf", "models/rich.bmf"]
        );
        let s = m.summary();
        assert_eq!((s.exported, s.failed), (2, 1));
        assert!(
            std::fs::read(out.join("models/quad.glb"))
                .unwrap()
                .starts_with(b"glTF")
        );
        assert_eq!(m.entries[2].details["primitives"], 3);
        assert_eq!(m.entries[2].details["skinned_surfaces"], 1);
        assert!(!m.entries[2].warnings.is_empty());
        // The gltf flavour.
        let out2 = dir.path().join("out2");
        let opts = Options {
            mesh: MeshOptions {
                format: GltfFormat::Gltf,
                ..MeshOptions::default()
            },
            ..Options::default()
        };
        let m = export_all(
            &Source::Disk,
            Kind::Mesh,
            src.to_str().unwrap(),
            &out2,
            &opts,
            &BatchSettings::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(m.entries[1].output.as_deref(), Some("models/quad.gltf"));
        let text = std::fs::read_to_string(out2.join("models/quad.gltf")).unwrap();
        assert!(serde_json::from_str::<Value>(&text).is_ok());
    }

    #[test]
    fn filter_limit_and_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let (src, out) = (dir.path().join("in"), dir.path().join("out"));
        sample_tree(&src);
        let only_deep = BatchSettings {
            filter: Some("*/deep/*".to_string()),
            ..BatchSettings::default()
        };
        let (m, _) = run(&src, &out, Kind::Texture, &only_deep).unwrap();
        assert_eq!(m.entries.len(), 1);
        assert_eq!(m.entries[0].source, "textures/deep/c_bgra.tex");
        let first_two = BatchSettings {
            limit: Some(2),
            ..BatchSettings::default()
        };
        let (m, _) = run(&src, &out, Kind::Texture, &first_two).unwrap();
        assert_eq!(m.entries.len(), 2);
        assert_eq!(m.entries[1].source, "textures/b_dxt1.tex");
        // Filter then limit.
        let both = BatchSettings {
            filter: Some("textures/*".to_string()),
            limit: Some(1),
            dry_run: false,
        };
        assert_eq!(
            run(&src, &out, Kind::Texture, &both)
                .unwrap()
                .0
                .entries
                .len(),
            1
        );
        // Dry run converts (statuses are real) but writes nothing.
        let out3 = dir.path().join("out3");
        let dry = BatchSettings {
            dry_run: true,
            ..BatchSettings::default()
        };
        let (m, _) = run(&src, &out3, Kind::Texture, &dry).unwrap();
        assert_eq!(m.summary().exported, 3);
        assert!(!out3.exists());
    }

    #[test]
    fn the_manifest_is_deterministic_and_has_no_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let (src, out) = (dir.path().join("in"), dir.path().join("out"));
        sample_tree(&src);
        let json = |out: &Path| {
            let (m, _) = run(&src, out, Kind::Texture, &BatchSettings::default()).unwrap();
            m.to_json(&default())
        };
        let a = json(&out);
        let b = json(&dir.path().join("elsewhere"));
        assert_eq!(a, b, "the output folder does not leak into the manifest");
        let v: Value = serde_json::from_str(&a).unwrap();
        assert_eq!(v["kind"], "texture");
        assert_eq!(v["summary"]["found"], 4);
        assert_eq!(v["summary"]["failed"], 1);
        assert_eq!(v["options"]["mip"], 0);
        assert_eq!(v["options"]["face"], 0);
        assert_eq!(v["entries"].as_array().unwrap().len(), 4);
        assert_eq!(v["entries"][2]["status"], "failed");
        assert_eq!(v["entries"][2]["error_kind"], "format");
        assert_eq!(v["entries"][0]["status"], "exported");
        assert!(v["entries"][0].get("error").is_none());
        assert!(a.ends_with("}\n"));
        let tmp = dir.path().to_string_lossy().replace('\\', "/");
        assert!(!a.replace("\\\\", "/").contains(&tmp));
    }

    #[test]
    fn a_missing_folder_is_an_error_and_an_unwritable_output_is_a_failed_entry() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        assert!(matches!(
            run(
                &missing,
                dir.path(),
                Kind::Texture,
                &BatchSettings::default()
            ),
            Err(ExportError::Read { .. })
        ));
        // The output "folder" is a file: every write fails, nothing panics.
        let src = dir.path().join("in");
        sample_tree(&src);
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let (m, _) = run(&src, &blocker, Kind::Texture, &BatchSettings::default()).unwrap();
        let s = m.summary();
        assert_eq!((s.exported, s.failed), (0, 4));
        assert!(m.entries.iter().any(|e| e.error_kind == Some("write")));
    }

    #[test]
    fn only_unsupported_errors_count_as_known_gaps() {
        assert_eq!(
            status_of(&ExportError::Unsupported("old".into())),
            Status::Unsupported
        );
        for e in [
            ExportError::Format("x".into()),
            ExportError::Option("x".into()),
            ExportError::Encode("x".into()),
            ExportError::read("p", "d"),
            ExportError::write("p", "d"),
        ] {
            assert_eq!(status_of(&e), Status::Failed, "{e}");
        }
    }

    #[test]
    fn error_kinds_are_stable_names() {
        assert_eq!(ExportError::Format(String::new()).kind(), "format");
        assert_eq!(
            ExportError::Unsupported(String::new()).kind(),
            "unsupported"
        );
        assert_eq!(ExportError::Option(String::new()).kind(), "option");
        assert_eq!(ExportError::Encode(String::new()).kind(), "encode");
        assert_eq!(ExportError::read("p", "d").kind(), "read");
        assert_eq!(ExportError::write("p", "d").kind(), "write");
        assert!(ExportError::read("a/b", "gone").to_string().contains("a/b"));
    }

    #[test]
    fn batch_options_reach_the_converter() {
        let dir = tempfile::tempdir().unwrap();
        let (src, out) = (dir.path().join("in"), dir.path().join("out"));
        sample_tree(&src);
        let opts = Options {
            tex: TexOptions {
                mip: 1,
                ..TexOptions::default()
            },
            ..Options::default()
        };
        let m = export_all(
            &Source::Disk,
            Kind::Texture,
            src.to_str().unwrap(),
            &out,
            &opts,
            &BatchSettings::default(),
            &mut |_| {},
        )
        .unwrap();
        // Only the DXT1 texture has a second level; the 24 and 32-bit ones fail with an option error.
        let dxt = &m.entries[1];
        assert_eq!(dxt.status, Status::Exported);
        assert_eq!(dxt.details["size"], json!([2, 2]));
        assert_eq!(m.entries[0].error_kind, Some("option"));
        let v: Value = serde_json::from_str(&m.to_json(&opts)).unwrap();
        assert_eq!(v["options"]["mip"], 1);
    }

    fn entry(status: Status, error: Option<&str>) -> ManifestEntry {
        ManifestEntry {
            source: "a.tex".into(),
            status,
            output: None,
            bytes_in: 10,
            bytes_out: 4,
            error_kind: error.map(|_| "format"),
            error: error.map(String::from),
            details: Value::Null,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn summary_and_reasons_count_each_status_separately() {
        let m = Manifest {
            kind: Kind::Texture,
            folder: String::new(),
            filter: None,
            entries: vec![
                entry(Status::Exported, None),
                entry(Status::Exported, None),
                entry(Status::Unsupported, Some("format version 5")),
                entry(Status::Unsupported, Some("format version 5")),
                entry(Status::Unsupported, Some("format version 6")),
                entry(Status::Failed, Some("broken")),
            ],
        };
        assert_eq!(
            m.summary(),
            Summary {
                found: 6,
                exported: 2,
                unsupported: 3,
                failed: 1,
                bytes_in: 60,
                bytes_out: 24,
            }
        );
        let r = m.reasons();
        assert_eq!(r.len(), 3);
        assert_eq!(r["unsupported: format version 5"], 2);
        assert_eq!(r["unsupported: format version 6"], 1);
        assert_eq!(r["failed: broken"], 1);
        let v: Value = serde_json::from_str(&m.to_json(&Options::default())).unwrap();
        assert_eq!(v["summary"]["unsupported"], 3);
        assert_eq!(v["reasons"]["failed: broken"], 1);
    }
}
