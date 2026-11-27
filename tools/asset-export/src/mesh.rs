//! `.bmf` to glTF 2.0 (`.gltf` with the buffer embedded as a data URI, or binary `.glb`).
//!
//! What is written: per LOD one glTF mesh, per surface one triangle-list primitive with
//! `POSITION`, `NORMAL`, optional `TANGENT` and one `TEXCOORD_n` per UV stream, `UNSIGNED_SHORT`
//! indices, and the bounding box of the LOD in the node's `extras`. Materials come from the
//! surface's `.mtr` reference or embedded preset (`ss2-format-mtr`); the textures are **external
//! references** (relative URIs of the PNG the texture exporter writes for the same path), never
//! embedded. Bone weights, bone indices and the bone palette are **not** exported (there is no
//! skeleton reader, so no glTF skin can be built); the export says so in its warnings.
//!
//! The engine's handedness, triangle winding and UV orientation are unknown (`bmf.md`, open
//! questions): coordinates and UVs are written as stored and the materials are double sided;
//! `--flip-winding` swaps the winding of every triangle.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use ss2_format_bmf::{Lod, MaterialRef, Mesh, Surface};
use ss2_format_mtr::{Material, MaterialFile, Param, ShaderClass, ShaderConfig, TextureRef};
use ss2_meta::Container;

use crate::base64;
use crate::error::{ExportError, Result};

const GENERATOR: &str = concat!("ss2 asset-export ", env!("CARGO_PKG_VERSION"));
const FLOAT: u32 = 5126;
const UNSIGNED_SHORT: u32 = 5123;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// Which LODs to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LodSel {
    /// One LOD by number (0 is the nearest).
    One(u32),
    /// Every LOD, each as its own node.
    All,
}

/// The file flavour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GltfFormat {
    /// JSON with the buffer as a base64 data URI (one self-contained file).
    Gltf,
    /// Binary container (one file).
    Glb,
}

/// Options of a mesh export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshOptions {
    /// LODs to write.
    pub lods: LodSel,
    /// Output flavour.
    pub format: GltfFormat,
    /// Swap two indices of every triangle.
    pub flip_winding: bool,
    /// Write normals and tangents exactly as stored. By default a normal or tangent whose xyz is
    /// zero or not of unit length (the Khronos validator rejects those, and some retail meshes
    /// have zero ones) is normalised, or replaced by +Y / +X when it is zero, and counted in the
    /// warnings.
    pub raw_vectors: bool,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            lods: LodSel::One(0),
            format: GltfFormat::Glb,
            flip_winding: false,
            raw_vectors: false,
        }
    }
}

/// Where the exporter reads the `.mtr` files a surface may reference.
pub trait Resolver {
    /// The bytes of the file at the install path `path`, if it can be read.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
}

/// A resolver that finds nothing (a mesh read from a plain file, without an install).
pub struct NoResolver;

impl Resolver for NoResolver {
    fn read(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

/// What the exporter needs to know about the file besides its bytes.
pub struct MeshContext<'a> {
    /// The name of the mesh (the file stem), used for node and mesh names.
    pub name: &'a str,
    /// The folder of the mesh inside the install, lower case, `/` separated, without a leading
    /// or trailing slash (`""` for the root or when unknown); texture URIs are relative to it.
    pub dir: &'a str,
    /// Reader of the referenced material files.
    pub resolver: &'a dyn Resolver,
}

/// The facts about an export, for reports and manifests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshInfo {
    /// LODs in the file.
    pub lods_in_file: usize,
    /// LODs written.
    pub lods_written: usize,
    /// Primitives (surfaces) written.
    pub primitives: usize,
    /// Vertices written.
    pub vertices: usize,
    /// Triangles written.
    pub triangles: usize,
    /// Materials written.
    pub materials: usize,
    /// External textures referenced, as lower-case install paths, sorted.
    pub textures: Vec<String>,
    /// Normals that were zero or not of unit length and were repaired (see
    /// [`MeshOptions::raw_vectors`]).
    pub repaired_normals: usize,
    /// Tangents that were zero, not of unit length or had a bad sign and were repaired.
    pub repaired_tangents: usize,
    /// Surfaces that carry bone data that was not exported.
    pub skinned_surfaces: usize,
    /// Union box of the written LODs (`None` if nothing was written).
    pub bounds: Option<([f32; 3], [f32; 3])>,
    /// Things that were left out or could not be resolved.
    pub warnings: Vec<String>,
}

/// A glTF file and what is in it.
#[derive(Clone, Debug)]
pub struct MeshExport {
    /// The file bytes (`.gltf` JSON or `.glb`).
    pub bytes: Vec<u8>,
    /// The facts about it.
    pub info: MeshInfo,
}

// ---------------------------------------------------------------------------------------------
// Paths and URIs

/// The lower-case PNG path (`/` separated) the texture exporter writes for the `.tex` path
/// `path` of a material file.
pub fn texture_png_key(path: &str) -> String {
    let p = path.replace('\\', "/").to_lowercase();
    let p = p.trim_start_matches('/');
    match p.strip_suffix(".tex") {
        Some(stem) => format!("{stem}.png"),
        None => format!("{p}.png"),
    }
}

fn percent_encode_segment(segment: &str) -> String {
    let mut out = String::new();
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The relative URI that leads from the folder `from_dir` to the file `to` (both install paths
/// without leading slash, `/` separated, `from_dir` may be empty). Segments are percent-encoded.
pub fn relative_uri(from_dir: &str, to: &str) -> String {
    let from: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let to: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    // The file name itself is never "common" (it is not a folder of `from`).
    let common = common.min(to.len().saturating_sub(1));
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
    parts.extend(to[common..].iter().map(|s| percent_encode_segment(s)));
    parts.join("/")
}

// ---------------------------------------------------------------------------------------------
// Materials

/// What the exporter keeps of one material.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialInfo {
    /// Name for the glTF material.
    pub name: String,
    /// Where it came from (`embedded preset 5` or the `.mtr` path).
    pub origin: String,
    /// The shader class, if known.
    pub shader: Option<String>,
    /// Every external texture of the chosen config: `(member number, install path)`.
    pub textures: Vec<(u32, String)>,
    /// The texture used as the base colour: install path and the UV map id it names.
    pub base_color: Option<(String, Option<u32>)>,
    /// Why the material is incomplete, if it is.
    pub warning: Option<String>,
}

/// The member number of the base (diffuse) map of a shader's arguments (`mtr.md`: the shade
/// shader's group 1 at `.10`, the specular shader's base colour map at `.21`, the reflection and
/// pixel-light shaders' base map at `.10`). Other shaders have none we can name.
pub fn base_color_member(shader: ShaderClass) -> Option<u32> {
    match shader {
        ShaderClass::Shade | ShaderClass::Reflection | ShaderClass::PolyBump => Some(10),
        ShaderClass::Specular => Some(21),
        _ => None,
    }
}

fn pick_config(material: &Material) -> Option<&ShaderConfig> {
    material
        .configs
        .iter()
        .find(|c| c.platform_mask & 1 != 0)
        .or_else(|| material.configs.first())
}

/// Describe `material` (a preset of a `.mtr` or a `.bmf`) for glTF.
pub fn describe_material(material: &Material, name: String, origin: String) -> MaterialInfo {
    let Some(config) = pick_config(material) else {
        return MaterialInfo {
            name,
            origin,
            shader: None,
            textures: Vec::new(),
            base_color: None,
            warning: Some("the preset has no shader config".to_string()),
        };
    };
    let textures: Vec<(u32, String)> = config
        .textures()
        .map(|(m, p)| (m, p.replace('\\', "/").to_lowercase()))
        .collect();
    let base_color =
        base_color_member(config.shader).and_then(|member| match config.param(member) {
            Some(Param::Texture(TextureRef::External(path))) => Some((
                path.replace('\\', "/").to_lowercase(),
                config.uv_map_of(member),
            )),
            _ => None,
        });
    MaterialInfo {
        name,
        origin,
        shader: Some(format!("{:?}", config.shader)),
        textures,
        base_color,
        warning: None,
    }
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

fn resolve_material(
    reference: &MaterialRef,
    container: &Container<'_>,
    decoded: &ss2_meta::Decoded,
    resolver: &dyn Resolver,
) -> Option<MaterialInfo> {
    match reference {
        MaterialRef::None => None,
        MaterialRef::Embedded(id) => {
            let name = format!("preset-{id}");
            let origin = format!("embedded preset {id}");
            Some(match Material::from_preset(container, decoded, *id) {
                Ok(m) => describe_material(&m, name, origin),
                Err(e) => incomplete(name, origin, format!("the preset cannot be read: {e}")),
            })
        }
        MaterialRef::External(path) => {
            let name = file_stem(path).to_string();
            let origin = path.clone();
            let lookup = path.replace('\\', "/").to_lowercase();
            Some(match resolver.read(&lookup) {
                None => incomplete(
                    name,
                    origin,
                    "the material file was not read (no install given, or not found)".to_string(),
                ),
                Some(bytes) => match MaterialFile::from_bytes(&bytes) {
                    Ok(file) => match file.presets.first() {
                        Some(m) => describe_material(m, name, origin),
                        None => incomplete(name, origin, "the file has no preset".to_string()),
                    },
                    Err(e) => incomplete(name, origin, format!("the file cannot be read: {e}")),
                },
            })
        }
    }
}

fn incomplete(name: String, origin: String, warning: String) -> MaterialInfo {
    MaterialInfo {
        name,
        origin,
        shader: None,
        textures: Vec::new(),
        base_color: None,
        warning: Some(warning),
    }
}

// ---------------------------------------------------------------------------------------------
// The glTF document

/// Binary buffer plus the views and accessors that index it.
#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Builder {
    fn pad4(&mut self) {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
    }

    /// Append `bytes` as a buffer view (4-byte aligned) and add an accessor for it. Returns the
    /// accessor index.
    fn accessor(&mut self, bytes: &[u8], target: u32, spec: Value) -> usize {
        self.pad4();
        let view = self.views.len();
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": self.bin.len(),
            "byteLength": bytes.len(),
            "target": target,
        }));
        self.bin.extend_from_slice(bytes);
        let mut acc = spec;
        acc["bufferView"] = json!(view);
        acc["byteOffset"] = json!(0);
        self.accessors.push(acc);
        self.accessors.len() - 1
    }
}

fn check_finite(what: &str, surface: usize, values: impl Iterator<Item = f32>) -> Result<()> {
    for v in values {
        if !v.is_finite() {
            return Err(ExportError::Encode(format!(
                "surface {surface}: a {what} value is not a finite number ({v}); glTF cannot hold it"
            )));
        }
    }
    Ok(())
}

fn f32_bytes<const N: usize>(values: &[[f32; N]]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|v| v.iter().flat_map(|c| c.to_le_bytes()))
        .collect()
}

fn min_max(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in positions {
        for c in 0..3 {
            min[c] = min[c].min(p[c]);
            max[c] = max[c].max(p[c]);
        }
    }
    (min, max)
}

fn num3(v: [f32; 3]) -> Value {
    json!([f64::from(v[0]), f64::from(v[1]), f64::from(v[2])])
}

struct Doc<'a> {
    builder: Builder,
    materials: Vec<Value>,
    material_index: BTreeMap<String, usize>,
    images: Vec<Value>,
    textures: Vec<Value>,
    texture_index: BTreeMap<String, usize>,
    dir: &'a str,
    info: MeshInfo,
}

impl Doc<'_> {
    fn texture(&mut self, path: &str) -> usize {
        if let Some(i) = self.texture_index.get(path) {
            return *i;
        }
        let uri = relative_uri(self.dir, &texture_png_key(path));
        self.images.push(json!({ "uri": uri, "name": path }));
        self.textures
            .push(json!({ "source": self.images.len() - 1 }));
        let i = self.textures.len() - 1;
        self.texture_index.insert(path.to_string(), i);
        i
    }

    /// The glTF material index for `info` used on a surface with `uv_count` UV streams and the
    /// UV map ids `uv_map_ids`.
    fn material(&mut self, info: &MaterialInfo, uv_map_ids: &[u32], uv_count: usize) -> usize {
        // Which TEXCOORD the base colour reads: the stream whose map id the material names.
        let texcoord = info.base_color.as_ref().and_then(|(_, uv_map)| {
            let at = uv_map
                .and_then(|id| uv_map_ids.iter().position(|m| *m == id))
                .unwrap_or(0);
            (at < uv_count).then_some(at)
        });
        let key = format!("{}|{}|{:?}", info.origin, info.name, texcoord);
        if let Some(i) = self.material_index.get(&key) {
            return *i;
        }
        let mut pbr = json!({ "metallicFactor": 0.0, "roughnessFactor": 1.0 });
        if let (Some((path, _)), Some(tc)) = (&info.base_color, texcoord) {
            let index = self.texture(path);
            pbr["baseColorTexture"] = json!({ "index": index, "texCoord": tc });
        }
        let textures: Vec<Value> = info
            .textures
            .iter()
            .map(|(m, p)| json!({ "member": m, "path": p }))
            .collect();
        for (_, p) in &info.textures {
            if !self.info.textures.contains(p) {
                self.info.textures.push(p.clone());
            }
        }
        let mut ss2 = json!({ "origin": info.origin, "textures": textures });
        if let Some(shader) = &info.shader {
            ss2["shader"] = json!(shader);
        }
        if let Some(w) = &info.warning {
            ss2["warning"] = json!(w);
            self.info
                .warnings
                .push(format!("material {}: {w}", info.name));
        }
        self.materials.push(json!({
            "name": info.name,
            "doubleSided": true,
            "pbrMetallicRoughness": pbr,
            "extras": { "ss2": ss2 },
        }));
        let i = self.materials.len() - 1;
        self.material_index.insert(key, i);
        i
    }
}

/// How far from 1 the length of a unit vector may be before it is repaired (the Khronos validator
/// allows 0.0005).
const UNIT_TOLERANCE: f32 = 1e-4;

/// `normal` made a unit vector: kept if it is one, normalised if it only has the wrong length,
/// +Y if it is zero. The flag says whether it was changed.
fn repair_normal(n: [f32; 3]) -> ([f32; 3], bool) {
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if (len - 1.0).abs() <= UNIT_TOLERANCE {
        (n, false)
    } else if len > 1e-6 {
        ([n[0] / len, n[1] / len, n[2] / len], true)
    } else {
        ([0.0, 1.0, 0.0], true)
    }
}

/// `tangent` with a unit xyz (as for normals; +X if zero) and a handedness `w` of exactly +1 or -1.
fn repair_tangent(t: [f32; 4]) -> ([f32; 4], bool) {
    let (xyz, changed) = repair_normal([t[0], t[1], t[2]]);
    let xyz = if changed && xyz == [0.0, 1.0, 0.0] {
        [1.0, 0.0, 0.0]
    } else {
        xyz
    };
    let w = if t[3] < 0.0 { -1.0 } else { 1.0 };
    ([xyz[0], xyz[1], xyz[2], w], changed || w != t[3])
}

fn primitive(
    doc: &mut Doc<'_>,
    surface: &Surface,
    number: usize,
    opts: &MeshOptions,
    material: Option<usize>,
) -> Result<Option<Value>> {
    let flip = opts.flip_winding;
    let vertices = surface.positions.len();
    if vertices == 0 || surface.indices.is_empty() {
        doc.info.warnings.push(format!(
            "surface {number} has no vertices or no triangles and was left out"
        ));
        return Ok(None);
    }
    if !surface.indices.len().is_multiple_of(3) {
        return Err(ExportError::Encode(format!(
            "surface {number}: {} indices are not a whole number of triangles",
            surface.indices.len()
        )));
    }
    check_finite(
        "position",
        number,
        surface.positions.iter().flatten().copied(),
    )?;
    check_finite("normal", number, surface.normals.iter().flatten().copied())?;
    if let Some(t) = &surface.tangents {
        check_finite("tangent", number, t.iter().flatten().copied())?;
    }
    for set in &surface.uv_sets {
        check_finite("UV", number, set.iter().flatten().copied())?;
    }
    let (min, max) = min_max(&surface.positions);
    let mut normals = surface.normals.clone();
    let mut tangents = surface.tangents.clone();
    if !opts.raw_vectors {
        for n in &mut normals {
            let (fixed, changed) = repair_normal(*n);
            *n = fixed;
            doc.info.repaired_normals += usize::from(changed);
        }
        for t in tangents.iter_mut().flatten() {
            let (fixed, changed) = repair_tangent(*t);
            *t = fixed;
            doc.info.repaired_tangents += usize::from(changed);
        }
    }
    let mut attributes = Map::new();
    let b = &mut doc.builder;
    attributes.insert(
        "POSITION".into(),
        json!(b.accessor(
            &f32_bytes(&surface.positions),
            ARRAY_BUFFER,
            json!({
                "componentType": FLOAT, "count": vertices, "type": "VEC3",
                "min": num3(min), "max": num3(max),
            })
        )),
    );
    if normals.len() == vertices {
        attributes.insert(
            "NORMAL".into(),
            json!(b.accessor(
                &f32_bytes(&normals),
                ARRAY_BUFFER,
                json!({ "componentType": FLOAT, "count": vertices, "type": "VEC3" })
            )),
        );
    }
    if let Some(t) = tangents.as_ref().filter(|t| t.len() == vertices) {
        attributes.insert(
            "TANGENT".into(),
            json!(b.accessor(
                &f32_bytes(t),
                ARRAY_BUFFER,
                json!({ "componentType": FLOAT, "count": vertices, "type": "VEC4" })
            )),
        );
    }
    for (n, set) in surface.uv_sets.iter().enumerate() {
        if set.len() != vertices {
            continue;
        }
        attributes.insert(
            format!("TEXCOORD_{n}"),
            json!(b.accessor(
                &f32_bytes(set),
                ARRAY_BUFFER,
                json!({ "componentType": FLOAT, "count": vertices, "type": "VEC2" })
            )),
        );
    }
    let mut indices = surface.indices.clone();
    if flip {
        for tri in indices.as_chunks_mut::<3>().0 {
            tri.swap(1, 2);
        }
    }
    let index_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    let index_accessor = b.accessor(
        &index_bytes,
        ELEMENT_ARRAY_BUFFER,
        json!({ "componentType": UNSIGNED_SHORT, "count": indices.len(), "type": "SCALAR" }),
    );
    let mut prim = json!({
        "attributes": Value::Object(attributes),
        "indices": index_accessor,
        "mode": 4,
    });
    if let Some(m) = material {
        prim["material"] = json!(m);
    }
    doc.info.primitives += 1;
    doc.info.vertices += vertices;
    doc.info.triangles += indices.len() / 3;
    Ok(Some(prim))
}

fn union(a: Option<([f32; 3], [f32; 3])>, b: ([f32; 3], [f32; 3])) -> Option<([f32; 3], [f32; 3])> {
    Some(match a {
        None => b,
        Some((amin, amax)) => (
            [0, 1, 2].map(|c| amin[c].min(b.0[c])),
            [0, 1, 2].map(|c| amax[c].max(b.1[c])),
        ),
    })
}

fn glb(json_text: &str, bin: &[u8]) -> Vec<u8> {
    let mut json_bytes = json_text.as_bytes().to_vec();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let mut bin = bin.to_vec();
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((json_bytes.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(&json_bytes);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(&bin);
    out
}

fn lod_bounds_json(lod: &Lod) -> Value {
    json!({ "min": num3(lod.bounds.min), "max": num3(lod.bounds.max) })
}

/// Read the `.bmf` file in `bytes` and write it as glTF.
///
/// # Errors
/// The reader's refusal ([`ExportError::Format`]); a LOD that does not exist
/// ([`ExportError::Option`]); a file without any geometry or with a value glTF cannot hold
/// ([`ExportError::Encode`]).
pub fn export_mesh(bytes: &[u8], ctx: &MeshContext<'_>, opts: &MeshOptions) -> Result<MeshExport> {
    let mesh = Mesh::from_bytes(bytes).map_err(|e| ExportError::from_reader(&e))?;
    let lods: Vec<(usize, &Lod)> = match opts.lods {
        LodSel::One(n) => match mesh.lods.get(n as usize) {
            Some(l) => vec![(n as usize, l)],
            None => {
                return Err(ExportError::Option(format!(
                    "LOD {n} does not exist: the mesh has {} LOD{}",
                    mesh.lods.len(),
                    if mesh.lods.len() == 1 { "" } else { "s" }
                )));
            }
        },
        LodSel::All => mesh.lods.iter().enumerate().collect(),
    };

    // The container is parsed a second time for the embedded material presets.
    let container = Container::parse(bytes).map_err(|e| ExportError::from_reader(&e))?;
    let decoded = container
        .decode_objects()
        .map_err(|e| ExportError::from_reader(&e))?;

    let mut doc = Doc {
        builder: Builder::default(),
        materials: Vec::new(),
        material_index: BTreeMap::new(),
        images: Vec::new(),
        textures: Vec::new(),
        texture_index: BTreeMap::new(),
        dir: ctx.dir,
        info: MeshInfo {
            lods_in_file: mesh.lods.len(),
            ..MeshInfo::default()
        },
    };
    let mut resolved: BTreeMap<String, Option<MaterialInfo>> = BTreeMap::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();

    for (number, lod) in &lods {
        let mut prims = Vec::new();
        for (s, surface) in lod.surfaces.iter().enumerate() {
            if surface.is_skinned() {
                doc.info.skinned_surfaces += 1;
            }
            let key = format!("{:?}", surface.material);
            let info = resolved
                .entry(key)
                .or_insert_with(|| {
                    resolve_material(&surface.material, &container, &decoded, ctx.resolver)
                })
                .clone();
            let material = info
                .as_ref()
                .map(|i| doc.material(i, &surface.uv_map_ids, surface.uv_sets.len()));
            if let Some(p) = primitive(&mut doc, surface, s, opts, material)? {
                prims.push(p);
            }
        }
        if prims.is_empty() {
            doc.info
                .warnings
                .push(format!("LOD {number} has no geometry"));
            continue;
        }
        let name = if matches!(opts.lods, LodSel::All) && mesh.lods.len() > 1 {
            format!("{}_lod{number}", ctx.name)
        } else {
            ctx.name.to_string()
        };
        meshes.push(json!({ "name": name, "primitives": prims }));
        nodes.push(json!({
            "name": name,
            "mesh": meshes.len() - 1,
            "extras": { "ss2": {
                "lod": number,
                "switch_distance": f64::from(lod.switch_distance),
                "platform_mask": lod.platform_mask,
                "bounds": lod_bounds_json(lod),
            } },
        }));
        doc.info.lods_written += 1;
        doc.info.bounds = union(doc.info.bounds, (lod.bounds.min, lod.bounds.max));
    }
    if meshes.is_empty() {
        return Err(ExportError::Encode(
            "the mesh has no geometry to export (no surface with vertices and triangles)"
                .to_string(),
        ));
    }
    if doc.info.skinned_surfaces > 0 {
        doc.info.warnings.push(format!(
            "{} skinned surface{}: bone weights, bone indices and the palette are not exported (no skeleton reader)",
            doc.info.skinned_surfaces,
            if doc.info.skinned_surfaces == 1 { "" } else { "s" }
        ));
    }
    if doc.info.repaired_normals + doc.info.repaired_tangents > 0 {
        doc.info.warnings.push(format!(
            "{} normals and {} tangents were zero or not of unit length and were repaired (--raw-vectors keeps them as stored)",
            doc.info.repaired_normals, doc.info.repaired_tangents
        ));
    }
    doc.info.materials = doc.materials.len();
    doc.info.textures.sort();

    let node_ids: Vec<usize> = (0..nodes.len()).collect();
    let mut ss2 = json!({
        "source": ctx.name,
        "lods_in_file": mesh.lods.len(),
        "distances": mesh.distances.map(f64::from),
        "has_collision_mesh": mesh.collision.is_some(),
        "winding_flipped": opts.flip_winding,
        "raw_vectors": opts.raw_vectors,
        "notes": [
            "coordinates, winding and UV orientation are written as stored; the engine's conventions are not confirmed (docs/re/bmf.md)",
            "bone data, morph maps and the collision mesh are not exported",
        ],
    });
    if !doc.info.warnings.is_empty() {
        ss2["warnings"] = json!(doc.info.warnings);
    }
    let mut root = json!({
        "asset": { "version": "2.0", "generator": GENERATOR },
        "scene": 0,
        "scenes": [{ "name": ctx.name, "nodes": node_ids, "extras": { "ss2": ss2 } }],
        "nodes": nodes,
        "meshes": meshes,
        "accessors": doc.builder.accessors,
        "bufferViews": doc.builder.views,
    });
    if !doc.materials.is_empty() {
        root["materials"] = json!(doc.materials);
    }
    if !doc.textures.is_empty() {
        root["images"] = json!(doc.images);
        root["textures"] = json!(doc.textures);
    }
    let bin_len = {
        doc.builder.pad4();
        doc.builder.bin.len()
    };
    let bytes = match opts.format {
        GltfFormat::Gltf => {
            root["buffers"] = json!([{
                "byteLength": bin_len,
                "uri": format!(
                    "data:application/octet-stream;base64,{}",
                    base64::encode(&doc.builder.bin)
                ),
            }]);
            serde_json::to_vec_pretty(&root)
                .map_err(|e| ExportError::Encode(format!("JSON: {e}")))?
        }
        GltfFormat::Glb => {
            root["buffers"] = json!([{ "byteLength": bin_len }]);
            let text = serde_json::to_string(&root)
                .map_err(|e| ExportError::Encode(format!("JSON: {e}")))?;
            glb(&text, &doc.builder.bin)
        }
    };
    Ok(MeshExport {
        bytes,
        info: doc.info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;
    use ss2_format_mtr::{PhysicalMaterial, ShaderConfig};

    const NONE: &NoResolver = &NoResolver;

    fn ctx() -> MeshContext<'static> {
        MeshContext {
            name: "thing",
            dir: "content/models/props",
            resolver: NONE,
        }
    }

    fn glb_opts() -> MeshOptions {
        MeshOptions::default()
    }

    fn gltf_opts() -> MeshOptions {
        MeshOptions {
            format: GltfFormat::Gltf,
            ..MeshOptions::default()
        }
    }

    /// Split a GLB into its JSON and BIN chunks, checking the container rules.
    fn split_glb(bytes: &[u8]) -> (Value, Vec<u8>) {
        assert_eq!(&bytes[..4], b"glTF");
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as usize;
        assert_eq!(u32_at(4), 2, "version");
        assert_eq!(u32_at(8), bytes.len(), "total length");
        let json_len = u32_at(12);
        assert_eq!(&bytes[16..20], b"JSON");
        assert_eq!(json_len % 4, 0, "JSON chunk is 4-byte aligned");
        let json: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        let bin_at = 20 + json_len;
        let bin_len = u32_at(bin_at);
        assert_eq!(&bytes[bin_at + 4..bin_at + 8], b"BIN\0");
        assert_eq!(bin_len % 4, 0, "BIN chunk is 4-byte aligned");
        assert_eq!(bin_at + 8 + bin_len, bytes.len());
        (json, bytes[bin_at + 8..].to_vec())
    }

    fn f32_at(bin: &[u8], at: usize) -> f32 {
        f32::from_le_bytes(bin[at..at + 4].try_into().unwrap())
    }

    /// The accessor `n` of `doc` as a flat list of floats, read straight from the buffer.
    fn floats(doc: &Value, bin: &[u8], n: usize) -> Vec<f32> {
        let acc = &doc["accessors"][n];
        let view = &doc["bufferViews"][acc["bufferView"].as_u64().unwrap() as usize];
        let comps = match acc["type"].as_str().unwrap() {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            t => panic!("{t}"),
        };
        let start = view["byteOffset"].as_u64().unwrap() as usize
            + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
        (0..acc["count"].as_u64().unwrap() as usize * comps)
            .map(|i| f32_at(bin, start + 4 * i))
            .collect()
    }

    fn indices(doc: &Value, bin: &[u8], n: usize) -> Vec<u16> {
        let acc = &doc["accessors"][n];
        assert_eq!(acc["componentType"], 5123);
        let view = &doc["bufferViews"][acc["bufferView"].as_u64().unwrap() as usize];
        let start = view["byteOffset"].as_u64().unwrap() as usize;
        (0..acc["count"].as_u64().unwrap() as usize)
            .map(|i| u16::from_le_bytes(bin[start + 2 * i..start + 2 * i + 2].try_into().unwrap()))
            .collect()
    }

    #[test]
    fn a_quad_becomes_a_well_formed_glb() {
        let out = export_mesh(&quad_file(), &ctx(), &glb_opts()).unwrap();
        let (doc, bin) = split_glb(&out.bytes);
        assert_eq!(doc["asset"]["version"], "2.0");
        assert!(
            doc["asset"]["generator"]
                .as_str()
                .unwrap()
                .contains("asset-export")
        );
        assert_eq!(doc["scene"], 0);
        assert_eq!(doc["scenes"][0]["nodes"], json!([0]));
        assert_eq!(doc["nodes"][0]["name"], "thing");
        assert_eq!(doc["nodes"][0]["mesh"], 0);
        let prim = &doc["meshes"][0]["primitives"][0];
        assert_eq!(prim["mode"], 4);
        let attrs = prim["attributes"].as_object().unwrap();
        let mut names: Vec<_> = attrs.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["NORMAL", "POSITION", "TEXCOORD_0"]);
        // Accessor contents are exactly the input streams.
        let pos = attrs["POSITION"].as_u64().unwrap() as usize;
        assert_eq!(
            floats(&doc, &bin, pos),
            [
                -1.0, 0.0, -1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, 1.0
            ]
        );
        let acc = &doc["accessors"][pos];
        assert_eq!(
            (
                acc["count"].as_u64(),
                acc["type"].as_str(),
                acc["componentType"].as_u64()
            ),
            (Some(4), Some("VEC3"), Some(5126))
        );
        assert_eq!(acc["min"], json!([-1.0, 0.0, -1.0]));
        assert_eq!(acc["max"], json!([1.0, 0.0, 1.0]));
        let nrm = attrs["NORMAL"].as_u64().unwrap() as usize;
        assert_eq!(floats(&doc, &bin, nrm), [0.0, 1.0, 0.0].repeat(4));
        let uv = attrs["TEXCOORD_0"].as_u64().unwrap() as usize;
        assert_eq!(
            floats(&doc, &bin, uv),
            [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        );
        let idx = prim["indices"].as_u64().unwrap() as usize;
        assert_eq!(indices(&doc, &bin, idx), [0, 1, 2, 2, 1, 3]);
        // Every buffer view is aligned and inside the buffer; every accessor inside its view.
        let buffer_len = doc["buffers"][0]["byteLength"].as_u64().unwrap();
        assert_eq!(
            buffer_len as usize,
            bin.len(),
            "buffer length is the padded BIN chunk"
        );
        assert!(doc["buffers"][0].get("uri").is_none());
        for v in doc["bufferViews"].as_array().unwrap() {
            let (o, l) = (
                v["byteOffset"].as_u64().unwrap(),
                v["byteLength"].as_u64().unwrap(),
            );
            assert_eq!(o % 4, 0, "view offset aligned");
            assert!(o + l <= buffer_len);
            assert!(v["target"] == 34962 || v["target"] == 34963);
        }
        assert_eq!(doc["accessors"].as_array().unwrap().len(), 4);
        // The node carries the LOD facts and the bounding box of the file.
        let ss2 = &doc["nodes"][0]["extras"]["ss2"];
        assert_eq!(ss2["lod"], 0);
        assert_eq!(ss2["switch_distance"], 150.0);
        assert_eq!(ss2["bounds"]["min"], json!([-1.0, 0.0, -1.0]));
        assert_eq!(ss2["bounds"]["max"], json!([1.0, 1.0, 1.0]));
        assert_eq!(
            doc["scenes"][0]["extras"]["ss2"]["has_collision_mesh"],
            false
        );
        // The info record.
        let i = out.info;
        assert_eq!(
            (
                i.lods_in_file,
                i.lods_written,
                i.primitives,
                i.vertices,
                i.triangles
            ),
            (1, 1, 1, 4, 2)
        );
        assert_eq!(i.bounds, Some(([-1.0, 0.0, -1.0], [1.0, 1.0, 1.0])));
        assert!(i.warnings.is_empty());
    }

    #[test]
    fn gltf_embeds_the_same_buffer_as_a_data_uri() {
        let glb = export_mesh(&quad_file(), &ctx(), &glb_opts()).unwrap();
        let gltf = export_mesh(&quad_file(), &ctx(), &gltf_opts()).unwrap();
        let (mut glb_doc, glb_bin) = split_glb(&glb.bytes);
        let doc: Value = serde_json::from_slice(&gltf.bytes).unwrap();
        let uri = doc["buffers"][0]["uri"].as_str().unwrap();
        let prefix = "data:application/octet-stream;base64,";
        assert!(uri.starts_with(prefix));
        // Decode with the independent reader's base64 (python is not needed: round trip by hand).
        let decoded = decode_base64(&uri[prefix.len()..]);
        assert_eq!(decoded, glb_bin);
        assert_eq!(doc["buffers"][0]["byteLength"], decoded.len());
        // Apart from the buffer the documents are equal.
        glb_doc["buffers"] = doc["buffers"].clone();
        assert_eq!(glb_doc, doc);
    }

    fn decode_base64(s: &str) -> Vec<u8> {
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
            let bytes = n.to_be_bytes();
            out.extend(&bytes[1..4 - pad]);
        }
        out
    }

    #[test]
    fn the_independent_gltf_reader_reads_back_what_was_written() {
        let src = rich_file();
        for opts in [
            MeshOptions {
                lods: LodSel::All,
                ..glb_opts()
            },
            MeshOptions {
                lods: LodSel::All,
                ..gltf_opts()
            },
        ] {
            let out = export_mesh(&src, &ctx(), &opts).unwrap();
            let parsed = gltf::Gltf::from_slice(&out.bytes).expect("the gltf crate accepts it");
            let blob: Vec<u8> = match opts.format {
                GltfFormat::Glb => parsed.blob.clone().expect("BIN chunk"),
                GltfFormat::Gltf => {
                    let doc: Value = serde_json::from_slice(&out.bytes).unwrap();
                    let uri = doc["buffers"][0]["uri"].as_str().unwrap();
                    decode_base64(uri.split_once(',').unwrap().1)
                }
            };
            let mesh = Mesh::from_bytes(&src).unwrap();
            let doc = parsed.document;
            assert_eq!(doc.meshes().len(), 2);
            for (lod, gm) in mesh.lods.iter().zip(doc.meshes()) {
                assert_eq!(gm.primitives().len(), lod.surfaces.len());
                for (surface, prim) in lod.surfaces.iter().zip(gm.primitives()) {
                    let reader = prim.reader(|_| Some(&blob[..]));
                    let pos: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
                    assert_eq!(pos, surface.positions);
                    let nrm: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
                    assert_eq!(nrm, surface.normals);
                    let uv: Vec<[f32; 2]> = reader.read_tex_coords(0).unwrap().into_f32().collect();
                    assert_eq!(uv, surface.uv_sets[0]);
                    let idx: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
                    let want: Vec<u32> = surface.indices.iter().map(|&i| u32::from(i)).collect();
                    assert_eq!(idx, want);
                    assert_eq!(
                        reader.read_tangents().map(Iterator::collect::<Vec<_>>),
                        surface.tangents.clone()
                    );
                    let bb = prim.bounding_box();
                    let (min, max) = min_max(&surface.positions);
                    assert_eq!((bb.min, bb.max), (min, max));
                }
            }
        }
    }

    #[test]
    fn lods_surfaces_tangents_materials_and_warnings() {
        let src = rich_file();
        // Default: LOD 0 only, three surfaces.
        let out = export_mesh(&src, &ctx(), &glb_opts()).unwrap();
        let (doc, _) = split_glb(&out.bytes);
        assert_eq!(doc["meshes"].as_array().unwrap().len(), 1);
        let prims = doc["meshes"][0]["primitives"].as_array().unwrap();
        assert_eq!(prims.len(), 3);
        assert!(prims[1]["attributes"].get("TANGENT").is_some());
        assert!(prims[0]["attributes"].get("TANGENT").is_none());
        let tangent = prims[1]["attributes"]["TANGENT"].as_u64().unwrap() as usize;
        assert_eq!(doc["accessors"][tangent]["type"], "VEC4");
        // The first surface names the embedded (empty) preset 5: a material with a warning.
        assert_eq!(prims[0]["material"], 0);
        assert!(prims[1].get("material").is_none());
        assert_eq!(doc["materials"][0]["name"], "preset-5");
        assert_eq!(doc["materials"][0]["doubleSided"], true);
        assert!(doc["materials"][0]["extras"]["ss2"]["warning"].is_string());
        let i = &out.info;
        assert_eq!(
            (i.primitives, i.vertices, i.triangles, i.materials),
            (3, 10, 4, 1)
        );
        assert_eq!(i.skinned_surfaces, 1);
        assert!(
            i.warnings.iter().any(|w| w.contains("bone weights")),
            "{:?}",
            i.warnings
        );
        assert!(
            i.warnings
                .iter()
                .any(|w| w.starts_with("material preset-5"))
        );
        assert_eq!(
            doc["scenes"][0]["extras"]["ss2"]["warnings"]
                .as_array()
                .unwrap()
                .len(),
            2
        );

        // LOD 1 has one surface; all LODs make two nodes with the LOD in the name.
        let lod1 = export_mesh(
            &src,
            &ctx(),
            &MeshOptions {
                lods: LodSel::One(1),
                ..glb_opts()
            },
        )
        .unwrap();
        let (doc1, _) = split_glb(&lod1.bytes);
        assert_eq!(doc1["meshes"][0]["primitives"].as_array().unwrap().len(), 1);
        assert_eq!(doc1["nodes"][0]["extras"]["ss2"]["lod"], 1);
        assert_eq!(doc1["nodes"][0]["extras"]["ss2"]["switch_distance"], 600.0);
        assert_eq!(
            doc1["nodes"][0]["name"], "thing",
            "a single LOD keeps the plain name"
        );
        let all = export_mesh(
            &src,
            &ctx(),
            &MeshOptions {
                lods: LodSel::All,
                ..glb_opts()
            },
        )
        .unwrap();
        let (doc_all, _) = split_glb(&all.bytes);
        assert_eq!(doc_all["scenes"][0]["nodes"], json!([0, 1]));
        assert_eq!(doc_all["nodes"][0]["name"], "thing_lod0");
        assert_eq!(doc_all["nodes"][1]["name"], "thing_lod1");
        assert_eq!(all.info.lods_written, 2);
        // The material is shared between LODs (the same preset).
        assert_eq!(doc_all["materials"].as_array().unwrap().len(), 1);
        // The union of both LOD boxes.
        assert_eq!(all.info.bounds, Some(([-1.0, 0.0, -1.0], [1.0, 1.0, 1.0])));

        let err = export_mesh(
            &src,
            &ctx(),
            &MeshOptions {
                lods: LodSel::One(2),
                ..glb_opts()
            },
        )
        .unwrap_err();
        assert!(matches!(err, ExportError::Option(_)));
        assert!(err.to_string().contains("LOD 2") && err.to_string().contains("2 LODs"));
    }

    #[test]
    fn flipping_the_winding_swaps_two_indices_of_every_triangle() {
        let plain = export_mesh(&quad_file(), &ctx(), &glb_opts()).unwrap();
        let flipped = export_mesh(
            &quad_file(),
            &ctx(),
            &MeshOptions {
                flip_winding: true,
                ..glb_opts()
            },
        )
        .unwrap();
        let get = |bytes: &[u8]| {
            let (doc, bin) = split_glb(bytes);
            let n = doc["meshes"][0]["primitives"][0]["indices"]
                .as_u64()
                .unwrap() as usize;
            (indices(&doc, &bin, n), doc)
        };
        let (a, _) = get(&plain.bytes);
        let (b, doc) = get(&flipped.bytes);
        assert_eq!(a, [0, 1, 2, 2, 1, 3]);
        assert_eq!(b, [0, 2, 1, 2, 3, 1]);
        assert_eq!(doc["scenes"][0]["extras"]["ss2"]["winding_flipped"], true);
    }

    #[test]
    fn non_finite_values_are_refused_not_written() {
        let (spec, idx, mut vtx) = quad();
        vtx[0..4].copy_from_slice(&f32::NAN.to_le_bytes());
        let file = mesh_file(&[lod(vec![spec.clone()])], &idx, &vtx);
        let err = export_mesh(&file, &ctx(), &glb_opts()).unwrap_err();
        assert!(matches!(err, ExportError::Encode(_)), "{err}");
        assert!(err.to_string().contains("position"));
        // Infinite normal.
        let (spec, idx, mut vtx) = quad();
        vtx[48..52].copy_from_slice(&f32::INFINITY.to_le_bytes());
        let file = mesh_file(&[lod(vec![spec])], &idx, &vtx);
        let err = export_mesh(&file, &ctx(), &glb_opts()).unwrap_err();
        assert!(err.to_string().contains("normal"));
        // UV.
        let (spec, idx, mut vtx) = quad();
        vtx[96..100].copy_from_slice(&f32::NEG_INFINITY.to_le_bytes());
        let file = mesh_file(&[lod(vec![spec])], &idx, &vtx);
        let err = export_mesh(&file, &ctx(), &glb_opts()).unwrap_err();
        assert!(err.to_string().contains("UV"));
    }

    #[test]
    fn a_mesh_without_triangles_is_an_error_and_junk_is_a_format_error() {
        let (mut spec, idx, vtx) = quad();
        spec.triangles = 0;
        let file = mesh_file(&[lod(vec![spec])], &idx, &vtx);
        // Whether the reader or the exporter refuses it, it is an error and not a panic.
        assert!(export_mesh(&file, &ctx(), &glb_opts()).is_err());
        assert!(matches!(
            export_mesh(b"junk", &ctx(), &glb_opts()),
            Err(ExportError::Format(_))
        ));
    }

    #[test]
    fn output_is_deterministic() {
        let a = export_mesh(&rich_file(), &ctx(), &gltf_opts())
            .unwrap()
            .bytes;
        let b = export_mesh(&rich_file(), &ctx(), &gltf_opts())
            .unwrap()
            .bytes;
        assert_eq!(a, b);
    }

    #[test]
    fn every_truncation_and_single_byte_corruption_is_survived() {
        let good = rich_file();
        let opts = MeshOptions {
            lods: LodSel::All,
            ..glb_opts()
        };
        for len in (0..good.len()).step_by(5) {
            let _ = export_mesh(&good[..len], &ctx(), &opts);
        }
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            let _ = export_mesh(&bad, &ctx(), &opts);
        }
    }

    // ----- paths, URIs and materials ---------------------------------------------------------

    #[test]
    fn texture_paths_become_lower_case_png_keys() {
        assert_eq!(
            texture_png_key("Content/SeriousSam2/Models/Tank/Textures/Body.tex"),
            "content/seriousSam2/models/tank/textures/body.png".to_lowercase()
        );
        assert_eq!(texture_png_key("A\\B\\C.TEX"), "a/b/c.png");
        assert_eq!(texture_png_key("/x/y"), "x/y.png");
        assert_eq!(texture_png_key("t.tga"), "t.tga.png");
    }

    #[test]
    fn relative_uris_walk_up_and_down() {
        assert_eq!(relative_uri("a/b", "a/b/t.png"), "t.png");
        assert_eq!(relative_uri("a/b", "a/c/t.png"), "../c/t.png");
        assert_eq!(relative_uri("a/b/c", "x/t.png"), "../../../x/t.png");
        assert_eq!(relative_uri("", "x/y/t.png"), "x/y/t.png");
        assert_eq!(relative_uri("a", "a/t.png"), "t.png");
        // A folder that is a prefix of the file name is not common.
        assert_eq!(relative_uri("a/t", "a/t.png"), "../t.png");
        assert_eq!(relative_uri("", "t.png"), "t.png");
        assert_eq!(relative_uri("a/b", "a/b c/t#1.png"), "../b%20c/t%231.png");
        // A folder named like the file does not swallow the file name.
        assert_eq!(relative_uri("a/t.png", "a/t.png"), "../t.png");
    }

    fn config(shader: ShaderClass, mask: u32, params: Vec<(u32, Param)>) -> ShaderConfig {
        ShaderConfig {
            shader,
            platform_mask: mask,
            args_class: "CShadeShaderArgs".to_string(),
            common: Vec::new(),
            params,
            modifiers: Vec::new(),
        }
    }

    fn tex_param(path: &str) -> Param {
        Param::Texture(TextureRef::External(path.to_string()))
    }

    fn material(configs: Vec<ShaderConfig>) -> Material {
        Material {
            id: 7,
            configs,
            physical: PhysicalMaterial::None,
        }
    }

    #[test]
    fn the_base_colour_member_depends_on_the_shader() {
        assert_eq!(base_color_member(ShaderClass::Shade), Some(10));
        assert_eq!(base_color_member(ShaderClass::Reflection), Some(10));
        assert_eq!(base_color_member(ShaderClass::PolyBump), Some(10));
        assert_eq!(base_color_member(ShaderClass::Specular), Some(21));
        for other in [
            ShaderClass::Refraction,
            ShaderClass::BlendLayers,
            ShaderClass::Terrain,
            ShaderClass::Water,
            ShaderClass::Other(5),
        ] {
            assert_eq!(base_color_member(other), None);
        }
    }

    #[test]
    fn a_material_is_described_from_the_pc_config() {
        let xbox = config(
            ShaderClass::Shade,
            0x2,
            vec![(10, tex_param("Xbox/Only.tex"))],
        );
        let pc = config(
            ShaderClass::Shade,
            0x1,
            vec![
                (10, tex_param("Content\\Models\\Body.TEX")),
                (11, Param::UvMap(4)),
                (20, tex_param("content/models/normal.tex")),
                (30, Param::Texture(TextureRef::None)),
                (31, Param::Float(1.0)),
            ],
        );
        let info = describe_material(&material(vec![xbox, pc]), "m".into(), "origin".into());
        assert_eq!(info.shader.as_deref(), Some("Shade"));
        assert_eq!(
            info.base_color,
            Some(("content/models/body.tex".to_string(), Some(4)))
        );
        assert_eq!(
            info.textures,
            [
                (10, "content/models/body.tex".to_string()),
                (20, "content/models/normal.tex".to_string())
            ]
        );
        assert!(info.warning.is_none());
        // No PC config: the first one is used.
        let only = config(ShaderClass::Specular, 0x2, vec![(21, tex_param("a/b.tex"))]);
        let info = describe_material(&material(vec![only]), "m".into(), "o".into());
        assert_eq!(info.base_color, Some(("a/b.tex".to_string(), None)));
        // A shader without a known base map has textures but no base colour.
        let water = config(ShaderClass::Water, 0x1, vec![(2, tex_param("w.tex"))]);
        let info = describe_material(&material(vec![water]), "m".into(), "o".into());
        assert!(info.base_color.is_none());
        assert_eq!(info.textures.len(), 1);
        // No config at all.
        let info = describe_material(&material(vec![]), "m".into(), "o".into());
        assert!(info.warning.is_some() && info.shader.is_none());
    }

    fn doc_for(dir: &str) -> Doc<'_> {
        Doc {
            builder: Builder::default(),
            materials: Vec::new(),
            material_index: BTreeMap::new(),
            images: Vec::new(),
            textures: Vec::new(),
            texture_index: BTreeMap::new(),
            dir,
            info: MeshInfo::default(),
        }
    }

    #[test]
    fn material_json_references_the_texture_by_relative_uri_and_picks_the_uv_set() {
        let info = MaterialInfo {
            name: "body".into(),
            origin: "x.mtr".into(),
            shader: Some("Shade".into()),
            textures: vec![
                (10, "content/models/tank/textures/body.tex".into()),
                (20, "content/models/tank/textures/body_n.tex".into()),
            ],
            base_color: Some(("content/models/tank/textures/body.tex".into(), Some(7))),
            warning: None,
        };
        let mut doc = doc_for("content/models/tank");
        // The surface has UV streams with map ids [3, 7]: map 7 is TEXCOORD_1.
        let m = doc.material(&info, &[3, 7], 2);
        assert_eq!(m, 0);
        let mat = &doc.materials[0];
        assert_eq!(
            mat["pbrMetallicRoughness"]["baseColorTexture"],
            json!({"index": 0, "texCoord": 1})
        );
        assert_eq!(doc.images[0]["uri"], "textures/body.png");
        assert_eq!(doc.textures[0]["source"], 0);
        assert_eq!(mat["extras"]["ss2"]["shader"], "Shade");
        assert_eq!(
            mat["extras"]["ss2"]["textures"].as_array().unwrap().len(),
            2
        );
        assert_eq!(doc.info.textures.len(), 2);
        // The same material on a surface with the same layout is reused; another layout is not.
        assert_eq!(doc.material(&info, &[3, 7], 2), 0);
        assert_eq!(doc.material(&info, &[7], 1), 1, "map 7 is TEXCOORD_0 here");
        assert_eq!(doc.textures.len(), 1, "the image is shared");
        assert_eq!(
            doc.materials[1]["pbrMetallicRoughness"]["baseColorTexture"]["texCoord"],
            0
        );
        // An unknown map id falls back to the first stream; no UV stream at all: no texture.
        assert_eq!(doc.material(&info, &[5], 1), 1);
        let none = doc.material(&info, &[], 0);
        assert!(
            doc.materials[none]["pbrMetallicRoughness"]
                .get("baseColorTexture")
                .is_none()
        );
    }

    struct MapResolver(Vec<(String, Vec<u8>)>);

    impl Resolver for MapResolver {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, b)| b.clone())
        }
    }

    #[test]
    fn an_unreadable_external_material_is_a_warning_not_an_error() {
        let d = decoded_for_tests();
        let r = MapResolver(vec![("a/m.mtr".to_string(), b"junk".to_vec())]);
        let reference = MaterialRef::External("A\\M.mtr".to_string());
        let info = resolve_material(&reference, &d.0, &d.1, &r).unwrap();
        assert_eq!(info.name, "M");
        assert!(info.warning.unwrap().contains("cannot be read"));
        let info =
            resolve_material(&MaterialRef::External("zz/none.mtr".into()), &d.0, &d.1, &r).unwrap();
        assert!(info.warning.unwrap().contains("not read"));
        assert!(resolve_material(&MaterialRef::None, &d.0, &d.1, &r).is_none());
        let info = resolve_material(&MaterialRef::Embedded(9999), &d.0, &d.1, &r).unwrap();
        assert!(info.warning.unwrap().contains("cannot be read"));
        assert_eq!(info.name, "preset-9999");
    }

    fn decoded_for_tests() -> (Container<'static>, ss2_meta::Decoded) {
        let bytes: &'static [u8] = Box::leak(quad_file().into_boxed_slice());
        let container = Container::parse(bytes).unwrap();
        let decoded = container.decode_objects().unwrap();
        (container, decoded)
    }

    #[test]
    fn glb_chunks_are_padded() {
        // JSON of 5 bytes -> padded with spaces; BIN of 3 bytes -> padded with zeros.
        let out = glb("{\"a\"}", &[1, 2, 3]);
        assert_eq!(out.len(), 12 + 8 + 8 + 8 + 4);
        assert_eq!(&out[20..28], b"{\"a\"}   ");
        assert_eq!(&out[28..32], &4u32.to_le_bytes());
        assert_eq!(&out[36..40], &[1, 2, 3, 0]);
        assert_eq!(
            u32::from_le_bytes(out[8..12].try_into().unwrap()) as usize,
            out.len()
        );
    }
}

#[cfg(test)]
mod repair_tests {
    use super::*;
    use crate::fixtures::*;

    #[test]
    fn normals_are_kept_normalised_or_replaced() {
        assert_eq!(repair_normal([0.0, 0.0, 1.0]), ([0.0, 0.0, 1.0], false));
        // Within the tolerance: kept as stored.
        let almost = [0.0, 1.00005, 0.0];
        assert_eq!(repair_normal(almost), (almost, false));
        // Wrong length: normalised.
        assert_eq!(repair_normal([0.0, 2.0, 0.0]), ([0.0, 1.0, 0.0], true));
        let (n, changed) = repair_normal([3.0, 0.0, 4.0]);
        assert!(changed && (n[0] - 0.6).abs() < 1e-6 && (n[2] - 0.8).abs() < 1e-6);
        // Just outside the tolerance.
        assert!(repair_normal([0.0, 1.0002, 0.0]).1);
        // Zero: +Y.
        assert_eq!(repair_normal([0.0, 0.0, 0.0]), ([0.0, 1.0, 0.0], true));
        assert_eq!(repair_normal([1e-8, 0.0, 0.0]), ([0.0, 1.0, 0.0], true));
    }

    #[test]
    fn tangents_get_a_unit_xyz_and_a_sign() {
        assert_eq!(
            repair_tangent([1.0, 0.0, 0.0, 1.0]),
            ([1.0, 0.0, 0.0, 1.0], false)
        );
        assert_eq!(
            repair_tangent([0.0, 0.0, 1.0, -1.0]),
            ([0.0, 0.0, 1.0, -1.0], false)
        );
        assert_eq!(
            repair_tangent([0.0, 0.0, 0.0, 1.0]),
            ([1.0, 0.0, 0.0, 1.0], true)
        );
        assert_eq!(
            repair_tangent([0.0, 0.0, 2.0, 0.5]),
            ([0.0, 0.0, 1.0, 1.0], true)
        );
        assert_eq!(
            repair_tangent([0.0, 0.0, 1.0, -3.0]),
            ([0.0, 0.0, 1.0, -1.0], true)
        );
        // A real +Y tangent is not mistaken for the zero fallback.
        assert_eq!(
            repair_tangent([0.0, 1.0, 0.0, 1.0]),
            ([0.0, 1.0, 0.0, 1.0], false)
        );
    }

    /// A mesh whose first normal and first tangent are zero.
    fn damaged_vectors_file() -> Vec<u8> {
        let (spec, idx, mut vtx) = quad();
        let mut spec = spec;
        // Zero the first normal (bytes 48..60).
        vtx[48..60].fill(0);
        // Append tangents: first one zero, the rest fine.
        let at = vtx.len() as u32;
        vtx.extend(f32s(&[0.0, 0.0, 0.0, 1.0]));
        vtx.extend(f32s(&[1.0, 0.0, 0.0, 1.0].repeat(3)));
        spec.tangent_offset = Some(at);
        mesh_file(&[lod(vec![spec])], &idx, &vtx)
    }

    fn glb_chunks(bytes: &[u8]) -> (Value, Vec<u8>) {
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let doc: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        (doc, bytes[28 + json_len..].to_vec())
    }

    fn accessor_floats(doc: &Value, bin: &[u8], n: usize) -> Vec<f32> {
        let acc = &doc["accessors"][n];
        let view = &doc["bufferViews"][acc["bufferView"].as_u64().unwrap() as usize];
        let comps = if acc["type"] == "VEC4" { 4 } else { 3 };
        let start = view["byteOffset"].as_u64().unwrap() as usize;
        (0..acc["count"].as_u64().unwrap() as usize * comps)
            .map(|i| f32::from_le_bytes(bin[start + 4 * i..start + 4 * i + 4].try_into().unwrap()))
            .collect()
    }

    #[test]
    fn zero_vectors_are_repaired_by_default_and_kept_with_raw_vectors() {
        let file = damaged_vectors_file();
        let ctx = MeshContext {
            name: "t",
            dir: "",
            resolver: &NoResolver,
        };
        let out = export_mesh(&file, &ctx, &MeshOptions::default()).unwrap();
        assert_eq!(
            (out.info.repaired_normals, out.info.repaired_tangents),
            (1, 1)
        );
        assert!(
            out.info
                .warnings
                .iter()
                .any(|w| w.contains("1 normals and 1 tangents"))
        );
        let (doc, bin) = glb_chunks(&out.bytes);
        let attrs = &doc["meshes"][0]["primitives"][0]["attributes"];
        let normals = accessor_floats(&doc, &bin, attrs["NORMAL"].as_u64().unwrap() as usize);
        assert_eq!(&normals[..6], &[0.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
        let tangents = accessor_floats(&doc, &bin, attrs["TANGENT"].as_u64().unwrap() as usize);
        assert_eq!(&tangents[..8], &[1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);

        let raw = MeshOptions {
            raw_vectors: true,
            ..MeshOptions::default()
        };
        let out = export_mesh(&file, &ctx, &raw).unwrap();
        assert_eq!(
            (out.info.repaired_normals, out.info.repaired_tangents),
            (0, 0)
        );
        let (doc, bin) = glb_chunks(&out.bytes);
        let attrs = &doc["meshes"][0]["primitives"][0]["attributes"];
        let normals = accessor_floats(&doc, &bin, attrs["NORMAL"].as_u64().unwrap() as usize);
        assert_eq!(&normals[..3], &[0.0, 0.0, 0.0]);
        assert_eq!(doc["scenes"][0]["extras"]["ss2"]["raw_vectors"], true);
    }

    #[test]
    fn intact_meshes_report_no_repairs() {
        let ctx = MeshContext {
            name: "t",
            dir: "",
            resolver: &NoResolver,
        };
        let out = export_mesh(&rich_file(), &ctx, &MeshOptions::default()).unwrap();
        assert_eq!(
            (out.info.repaired_normals, out.info.repaired_tangents),
            (0, 0)
        );
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::fixtures::*;

    const NONE: &NoResolver = &NoResolver;

    fn ctx() -> MeshContext<'static> {
        MeshContext {
            name: "thing",
            dir: "content/models/props",
            resolver: NONE,
        }
    }

    fn glb_opts() -> MeshOptions {
        MeshOptions::default()
    }

    fn split_glb(bytes: &[u8]) -> (Value, Vec<u8>) {
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let doc: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        (doc, bytes[28 + json_len..].to_vec())
    }

    #[test]
    fn every_buffer_view_of_a_rich_mesh_is_four_byte_aligned() {
        // Odd index counts (3 indices = 6 bytes) would misalign the next view without padding.
        let out = export_mesh(&rich_file(), &ctx(), &glb_opts()).unwrap();
        let (doc, bin) = split_glb(&out.bytes);
        let views = doc["bufferViews"].as_array().unwrap();
        assert!(views.len() > 8);
        let mut end = 0;
        for v in views {
            let (o, l) = (
                v["byteOffset"].as_u64().unwrap() as usize,
                v["byteLength"].as_u64().unwrap() as usize,
            );
            assert_eq!(o % 4, 0, "view at {o}");
            assert!(o >= end, "views do not overlap");
            end = o + l;
        }
        assert!(end <= bin.len() && bin.len() - end < 4);
        assert!(
            views
                .iter()
                .any(|v| v["byteLength"].as_u64().unwrap() % 4 == 2)
        );
    }

    #[test]
    fn the_union_of_lod_boxes_takes_the_extremes_of_each_side() {
        let (spec, idx, vtx) = quad();
        let mut near = lod(vec![spec.clone()]);
        near.bounds = ([-1.0, 0.0, -1.0], [1.0, 1.0, 1.0]);
        let mut far = lod(vec![spec]);
        far.bounds = ([-3.0, 0.5, 0.0], [0.5, 2.0, 1.0]);
        let file = mesh_file(&[near, far], &idx, &vtx);
        let opts = MeshOptions {
            lods: LodSel::All,
            ..glb_opts()
        };
        let out = export_mesh(&file, &ctx(), &opts).unwrap();
        assert_eq!(out.info.bounds, Some(([-3.0, 0.0, -1.0], [1.0, 2.0, 1.0])));
    }
}
