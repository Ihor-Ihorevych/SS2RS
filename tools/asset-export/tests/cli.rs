//! The committed fixtures and the command line, end to end, with independent readers
//! (the `png` decoder and the `gltf` crate) checking what the binary wrote.

#[path = "../src/fixtures.rs"]
#[allow(dead_code)]
mod fixtures;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fixtures::*;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_asset-export"))
}

fn run(args: &[&str]) -> Output {
    bin().args(args).output().expect("the binary starts")
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("exit code")
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn fixture_dir() -> PathBuf {
    ss2_testkit::fixtures::testdata_path("asset-export")
}

fn fixture(name: &str) -> String {
    fixture_dir().join(name).to_string_lossy().into_owned()
}

/// Every committed fixture and the builder that made it.
fn builders() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("bgr_8x4.tex", bgr_file()),
        ("bgra_4x4.tex", bgra_file()),
        ("dxt1_4x4.tex", dxt1_file()),
        ("cube_4x4.tex", cube_file()),
        ("quad.bmf", quad_file()),
        ("rich.bmf", rich_file()),
    ]
}

/// The committed files are exactly what the builders make. `UPDATE_FIXTURES=1 cargo test -p
/// asset-export --test cli` rewrites them.
#[test]
fn committed_fixtures_match_the_builders() {
    let update = std::env::var_os("UPDATE_FIXTURES").is_some();
    for (name, bytes) in builders() {
        let path = fixture_dir().join(name);
        if update {
            std::fs::create_dir_all(fixture_dir()).unwrap();
            std::fs::write(&path, &bytes).unwrap();
        }
        let on_disk = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(on_disk, bytes, "{name} differs from its builder");
        assert!(bytes.len() < 256 * 1024);
    }
}

fn decode_png(path: &Path) -> (u32, u32, png::ColorType, Vec<u8>) {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    (info.width, info.height, info.color_type, buf)
}

#[test]
fn tex_writes_a_png_that_decodes() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("sub/ramp.png");
    let o = run(&["tex", &fixture("bgr_8x4.tex"), "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    let (w, h, color, px) = decode_png(&out);
    assert_eq!((w, h, color), (8, 4, png::ColorType::Rgb));
    // First pixel: B = 0, G = 100, R = 255 in the file.
    assert_eq!(&px[..3], &[255, 100, 0]);
    assert!(stderr(&o).contains("bgr_8x4.tex"));
}

#[test]
fn tex_options_select_faces_mips_and_channels() {
    let dir = tempfile::tempdir().unwrap();
    let p = |n: &str| dir.path().join(n);
    let o = run(&[
        "tex",
        &fixture("cube_4x4.tex"),
        "-o",
        p("f.png").to_str().unwrap(),
        "--face",
        "all",
        "--mip",
        "0",
    ]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    let (w, h, _, _) = decode_png(&p("f.png"));
    assert_eq!((w, h), (24, 4));
    let o = run(&[
        "tex",
        &fixture("cube_4x4.tex"),
        "-o",
        p("g.png").to_str().unwrap(),
        "--face",
        "2",
        "--mip",
        "1",
    ]);
    assert_eq!(code(&o), 0);
    assert_eq!(decode_png(&p("g.png")).3[..3], [0, 0, 255]);
    let o = run(&[
        "tex",
        &fixture("bgra_4x4.tex"),
        "-o",
        p("rgb.png").to_str().unwrap(),
        "--rgb-only",
    ]);
    assert_eq!(code(&o), 0);
    assert_eq!(decode_png(&p("rgb.png")).2, png::ColorType::Rgb);
    let o = run(&[
        "tex",
        &fixture("bgra_4x4.tex"),
        "-o",
        p("a.png").to_str().unwrap(),
        "--alpha-only",
    ]);
    assert_eq!(code(&o), 0);
    assert_eq!(decode_png(&p("a.png")).2, png::ColorType::Grayscale);
    // Options that do not fit the file or each other.
    let o = run(&[
        "tex",
        &fixture("bgra_4x4.tex"),
        "-o",
        p("x.png").to_str().unwrap(),
        "--mip",
        "5",
    ]);
    assert_eq!(code(&o), 2);
    assert!(stderr(&o).contains("mip level 5"), "{}", stderr(&o));
    assert!(!p("x.png").exists());
    let o = run(&[
        "tex",
        &fixture("bgra_4x4.tex"),
        "-o",
        p("x.png").to_str().unwrap(),
        "--rgb-only",
        "--alpha-only",
    ]);
    assert_eq!(code(&o), 2, "clap rejects conflicting options");
    let o = run(&[
        "tex",
        &fixture("bgra_4x4.tex"),
        "-o",
        p("x.png").to_str().unwrap(),
        "--face",
        "left",
    ]);
    assert_eq!(code(&o), 2);
    assert!(stderr(&o).contains("--face"));
}

#[test]
fn damaged_and_missing_inputs_exit_with_2_and_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.tex");
    std::fs::write(&bad, b"definitely not a texture").unwrap();
    let out = dir.path().join("o.png");
    let o = run(&["tex", bad.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 2);
    assert!(stderr(&o).contains("CTSEMETA"), "{}", stderr(&o));
    let o = run(&["tex", "no/such/file.tex", "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 2);
    assert!(stderr(&o).contains("cannot read"));
    assert!(!out.exists());
    let o = run(&[
        "mesh",
        bad.to_str().unwrap(),
        "-o",
        dir.path().join("o.glb").to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 2);
    // Neither an input nor --all, or both.
    let o = run(&["tex", "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 2);
    let o = run(&["tex", "a.tex", "--all", "x", "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 2);
    // A mesh output that is neither .glb nor .gltf.
    let o = run(&[
        "mesh",
        &fixture("quad.bmf"),
        "-o",
        dir.path().join("q.obj").to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 2);
    assert!(stderr(&o).contains(".glb"));
}

/// Parse a `.glb` or `.gltf` with the independent `gltf` crate and return the positions of
/// every primitive.
fn read_positions(path: &Path) -> Vec<Vec<[f32; 3]>> {
    let bytes = std::fs::read(path).unwrap();
    let g = gltf::Gltf::from_slice(&bytes).expect("the gltf crate accepts the file");
    let blob = match g.blob.clone() {
        Some(b) => b,
        None => {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let uri = v["buffers"][0]["uri"].as_str().unwrap();
            assert!(uri.starts_with("data:application/octet-stream;base64,"));
            // Positions are read through the accessor offsets; decode with a tiny decoder.
            let data = uri.split_once(',').unwrap().1;
            decode_b64(data)
        }
    };
    g.document
        .meshes()
        .flat_map(|m| m.primitives())
        .map(|p| {
            p.reader(|_| Some(&blob[..]))
                .read_positions()
                .unwrap()
                .collect()
        })
        .collect()
}

fn decode_b64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => 0,
    };
    let mut out = Vec::new();
    for chunk in s.as_bytes().chunks(4) {
        let n = chunk
            .iter()
            .fold(0u32, |a, &c| (a << 6) | u32::from(val(c)));
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        out.extend(&n.to_be_bytes()[1..4 - pad]);
    }
    out
}

#[test]
fn mesh_writes_glb_and_gltf_that_an_independent_reader_opens() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["q.glb", "q.gltf"] {
        let out = dir.path().join(name);
        let o = run(&["mesh", &fixture("quad.bmf"), "-o", out.to_str().unwrap()]);
        assert_eq!(code(&o), 0, "{}", stderr(&o));
        let pos = read_positions(&out);
        assert_eq!(pos.len(), 1);
        assert_eq!(pos[0][3], [1.0, 0.0, 1.0]);
    }
    let out = dir.path().join("rich.glb");
    let o = run(&[
        "mesh",
        &fixture("rich.bmf"),
        "-o",
        out.to_str().unwrap(),
        "--all-lods",
    ]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    assert_eq!(read_positions(&out).len(), 4, "3 + 1 primitives");
    assert!(
        stderr(&o).contains("warning: "),
        "the skinning gap is reported"
    );
    // --lod and --all-lods exclude each other; a LOD that does not exist is an error.
    let o = run(&[
        "mesh",
        &fixture("rich.bmf"),
        "-o",
        out.to_str().unwrap(),
        "--lod",
        "1",
        "--all-lods",
    ]);
    assert_eq!(code(&o), 2);
    let o = run(&[
        "mesh",
        &fixture("rich.bmf"),
        "-o",
        out.to_str().unwrap(),
        "--lod",
        "9",
    ]);
    assert_eq!(code(&o), 2);
    assert!(stderr(&o).contains("LOD 9"));
}

#[test]
fn batch_exports_a_folder_writes_the_manifest_and_signals_failures() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("Tex")).unwrap();
    for (n, b) in builders() {
        let sub = if n.ends_with("tex") { "Tex" } else { "." };
        std::fs::write(src.join(sub).join(n), b).unwrap();
    }
    let out = dir.path().join("out");
    let o = run(&[
        "tex",
        "--all",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("manifest-texture.json")).unwrap()).unwrap();
    assert_eq!(m["summary"]["found"], 4);
    assert_eq!(m["summary"]["exported"], 4);
    assert!(out.join("tex/cube_4x4.png").exists());

    // A damaged file makes the exit code 1 but the rest is exported.
    std::fs::write(src.join("Tex/zz_broken.tex"), b"junk").unwrap();
    let o = run(&[
        "tex",
        "--all",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 1, "{}", stderr(&o));
    assert!(stderr(&o).contains("failed: tex/zz_broken.tex"));
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("manifest-texture.json")).unwrap()).unwrap();
    assert_eq!(m["summary"]["failed"], 1);
    assert_eq!(m["summary"]["exported"], 4);

    // Meshes: a folder with a gltf flavour and a filter.
    let o = run(&[
        "mesh",
        "--all",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--format",
        "gltf",
        "--filter",
        "*rich*",
    ]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    assert!(out.join("rich.gltf").exists() && !out.join("quad.gltf").exists());
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("manifest-mesh.json")).unwrap()).unwrap();
    assert_eq!(m["summary"]["found"], 1);

    // The manifest is identical when the run is repeated.
    let first = std::fs::read(out.join("manifest-mesh.json")).unwrap();
    run(&[
        "mesh",
        "--all",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--format",
        "gltf",
        "--filter",
        "*rich*",
    ]);
    assert_eq!(
        std::fs::read(out.join("manifest-mesh.json")).unwrap(),
        first
    );
}

#[test]
fn sheet_makes_a_png_and_a_cell_list() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("s/sheet.png");
    let bad = dir.path().join("bad.tex");
    std::fs::write(&bad, b"junk").unwrap();
    let o = run(&[
        "sheet",
        &fixture("bgr_8x4.tex"),
        &fixture("bgra_4x4.tex"),
        &fixture("dxt1_4x4.tex"),
        &fixture("cube_4x4.tex"),
        bad.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--cell",
        "32",
        "--columns",
        "2",
    ]);
    assert_eq!(code(&o), 1, "one texture was left out: {}", stderr(&o));
    let (w, h, color, _) = decode_png(&out);
    assert_eq!((w, h, color), (4 + 2 * 36, 4 + 2 * 36, png::ColorType::Rgb));
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("s/sheet.json")).unwrap()).unwrap();
    assert_eq!(index["cells"].as_array().unwrap().len(), 4);
    assert_eq!(index["skipped"].as_array().unwrap().len(), 1);
    assert_eq!(index["columns"], 2);
    assert_eq!(index["cells"][3]["row"], 1);
    assert_eq!(index["cells"][3]["column"], 1);
    assert!(
        index["skipped"][0]["error"]
            .as_str()
            .unwrap()
            .contains("CTSEMETA")
    );
    // No textures at all.
    let o = run(&["sheet", "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 2);
    // A sheet of only broken files is an error.
    let o = run(&["sheet", bad.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(code(&o), 2);
}

#[test]
fn exports_of_the_same_input_are_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.glb"), dir.path().join("b.glb"));
    for out in [&a, &b] {
        assert_eq!(
            code(&run(&[
                "mesh",
                &fixture("rich.bmf"),
                "-o",
                out.to_str().unwrap()
            ])),
            0
        );
    }
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
}

/// With `GLTF_VALIDATOR_DIR` (a folder where `npm install gltf-validator` was run) and `node` on
/// the path, the Khronos validator checks the glTF files the binary writes for the fixtures.
#[test]
fn the_khronos_validator_accepts_the_fixtures_when_it_is_installed() {
    let Some(validator_dir) = std::env::var_os("GLTF_VALIDATOR_DIR") else {
        eprintln!("SKIPPED: GLTF_VALIDATOR_DIR is not set");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for (name, extra) in [
        ("quad.glb", &[][..]),
        ("rich.glb", &["--all-lods"][..]),
        ("rich.gltf", &["--all-lods"][..]),
    ] {
        let src = if name.starts_with("quad") {
            "quad.bmf"
        } else {
            "rich.bmf"
        };
        let out = dir.path().join(name);
        let mut args = vec![
            "mesh".to_string(),
            fixture(src),
            "-o".to_string(),
            out.to_string_lossy().into_owned(),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));
        let o = bin().args(&args).output().unwrap();
        assert_eq!(code(&o), 0, "{}", stderr(&o));
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/validate_gltf.mjs");
    let result = Command::new("node")
        .arg(&script)
        .arg(dir.path())
        .env("GLTF_VALIDATOR_DIR", validator_dir)
        .output();
    match result {
        Ok(o) => assert!(
            o.status.success(),
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => eprintln!("SKIPPED: node cannot be started: {e}"),
    }
}
