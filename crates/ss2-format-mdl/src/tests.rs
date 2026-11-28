use std::sync::Arc;

use ss2_assets::{AssetManager, LoadState};
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
use ss2_vfs::MemoryVfs;

use super::*;

/// What a mesh instance of the synthetic model refers to.
#[derive(Clone, Copy)]
enum Inst {
    /// The embedded `CMesh` (object 2).
    Embedded,
    /// The external `.bmf` (object 7).
    External,
    /// Null.
    Null,
    /// An id that is neither an object nor external.
    Dangling,
}

/// What the surface of the embedded mesh uses as its material.
#[derive(Clone, Copy)]
enum Mat {
    /// The embedded preset (object 6).
    Embedded,
    /// None.
    None,
    /// The external `.mtr` (object 8).
    External,
    /// An embedded object that is not a preset (the index buffer, object 4).
    NotAPreset,
}

struct Spec {
    instances: Vec<Inst>,
    /// `CResource.1` of the configuration.
    role: i32,
    /// Add a second configuration (object 9) with this `CResource.1`.
    second: Option<i32>,
    material: Mat,
}

impl Spec {
    fn new(instances: &[Inst]) -> Self {
        Self {
            instances: instances.to_vec(),
            role: 1,
            second: None,
            material: Mat::Embedded,
        }
    }
}

fn path(code: u32, buffer: u32, offset: u32) -> Vec<u8> {
    ObjectBytes::new()
        .u32(code)
        .u32(buffer)
        .u32(offset)
        .finish()
}

fn absent() -> Vec<u8> {
    path(0, 255, 0xFFFF_FFFF)
}

fn f32s(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A model built from the structure of `mdl.md`, with one embedded quad mesh (`bmf.md`), one
/// embedded shader preset without configs (`mtr.md`) and two external references.
fn model_file(spec: &Spec) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let bmf = b.file_ref("Content/Test/Models/Ext.bmf", 0);
    let mtr = b.file_ref("Content/Test/Models/Ext.mtr", 0);
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
    b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(4, "UBYTE", 1, 1));
    b.type_block(TypeBlock::enumeration(5, "GfxBufferType", 4));
    b.type_block(TypeBlock::short(6, "CStaticArray<UBYTE>", 0, 6, 4));
    b.type_block(TypeBlock::class(
        7,
        "CResourceFile",
        3,
        5,
        None,
        &[("1", 1)],
    ));
    b.type_block(TypeBlock::pointer(8, "CResourceFile*", 7));
    b.type_block(TypeBlock::class(
        9,
        "CResource",
        4,
        5,
        None,
        &[("1", 1), ("2", 8)],
    ));
    b.type_block(TypeBlock::class(
        10,
        "GFXHANDLE",
        1,
        5,
        Some(9),
        &[("1", 5), ("2", 6)],
    ));
    b.type_block(TypeBlock::pointer(11, "GFXHANDLE*", 10));
    b.type_block(TypeBlock::class(
        12,
        "VBufferHolder",
        1,
        5,
        None,
        &[("1", 11)],
    ));
    b.type_block(TypeBlock::class(
        13,
        "Box3f",
        1,
        5,
        None,
        &[("1", 3), ("2", 3), ("3", 3), ("4", 3), ("5", 3), ("6", 3)],
    ));
    b.type_block(TypeBlock::class(
        14,
        "CRenderMeshElementPath",
        1,
        5,
        None,
        &[("1", 2), ("2", 2), ("3", 2)],
    ));
    b.type_block(TypeBlock::short(15, "ULONG[8]", 0, 4, 2));
    b.type_block(TypeBlock::short(16, "CRenderMeshElementPath[8]", 0, 4, 14));
    b.type_block(TypeBlock::short(17, "CStaticArray<INDEX>", 0, 6, 1));
    b.type_block(TypeBlock::class(
        18,
        "CRenderMeshSurface",
        1,
        5,
        None,
        &[
            ("2", 2),
            ("3", 2),
            ("4", 2),
            ("6", 14),
            ("7", 14),
            ("8", 14),
            ("9", 14),
            ("10", 14),
            ("11", 14),
            ("12", 15),
            ("14", 16),
            ("16", 19),
            ("18", 17),
        ],
    ));
    b.type_block(TypeBlock::pointer(19, "CResource*", 9));
    b.type_block(TypeBlock::short(
        20,
        "CStaticArray<CRenderMeshSurface>",
        0,
        6,
        18,
    ));
    b.type_block(TypeBlock::class(
        21,
        "CRenderMeshSection",
        1,
        5,
        None,
        &[("1", 20)],
    ));
    b.type_block(TypeBlock::short(
        22,
        "CStaticArray<CRenderMeshSection>",
        0,
        6,
        21,
    ));
    b.type_block(TypeBlock::class(
        23,
        "CRenderMeshLOD",
        1,
        5,
        None,
        &[("3", 13), ("5", 22), ("6", 3), ("7", 2)],
    ));
    b.type_block(TypeBlock::short(
        24,
        "CStaticArray<CRenderMeshLOD>",
        0,
        6,
        23,
    ));
    b.type_block(TypeBlock::short(
        25,
        "CStaticArray<VBufferHolder>",
        0,
        6,
        12,
    ));
    b.type_block(TypeBlock::class(
        26,
        "CRenderMesh",
        1,
        5,
        None,
        &[
            ("5", 24),
            ("12", 25),
            ("13", 25),
            ("7", 3),
            ("8", 3),
            ("9", 3),
        ],
    ));
    b.type_block(TypeBlock::pointer(27, "CRenderMesh*", 26));
    b.type_block(TypeBlock::class(
        28,
        "CMesh",
        1,
        5,
        Some(9),
        &[("1", 27), ("2", 27)],
    ));
    b.type_block(TypeBlock::class(
        29,
        "CShaderConfig",
        1,
        5,
        None,
        &[("1", 19), ("2", 19), ("3", 2)],
    ));
    b.type_block(TypeBlock::short(
        30,
        "CStaticArray<CShaderConfig>",
        0,
        6,
        29,
    ));
    b.type_block(TypeBlock::class(
        31,
        "CShaderPreset",
        1,
        5,
        Some(9),
        &[("1", 30), ("2", 19)],
    ));
    b.type_block(TypeBlock::pointer(32, "CMesh*", 28));
    b.type_block(TypeBlock::class(
        33,
        "CMeshInstance",
        2,
        5,
        None,
        &[("1", 32)],
    ));
    b.type_block(TypeBlock::short(
        34,
        "CStaticArray<CMeshInstance>",
        0,
        6,
        33,
    ));
    b.type_block(TypeBlock::class(
        35,
        "CModelConfiguration",
        12,
        5,
        Some(9),
        &[("3", 34), ("1", 13), ("6", 3), ("7", 3), ("12", 3)],
    ));
    b.external_object(7, bmf, 1, 28);
    b.external_object(8, mtr, 1, 31);

    let configuration = |role: i32, instances: &[Inst]| {
        let mut o = ObjectBytes::new()
            .i32(role)
            .object_ref(Some(0))
            .array_header(b"STAR", instances.len() as u32);
        for i in instances {
            o = o.object_ref(match i {
                Inst::Embedded => Some(2),
                Inst::External => Some(7),
                Inst::Null => None,
                Inst::Dangling => Some(99),
            });
        }
        o.f32(-1.0)
            .f32(0.0)
            .f32(-2.0)
            .f32(1.0)
            .f32(3.5)
            .f32(1.5)
            .f32(990.0)
            .f32(1000.0)
            .f32(150.0)
            .finish()
    };
    b.object(0, 7, ObjectBytes::new().i32(1).finish());
    b.object(1, 35, configuration(spec.role, &spec.instances));
    b.object(
        2,
        28,
        ObjectBytes::new()
            .i32(-1)
            .object_ref(Some(0))
            .object_ref(Some(3))
            .object_ref(None)
            .finish(),
    );
    // The render mesh: one LOD, one section, one quad surface (indices in buffer 0 of the index
    // list, positions at 0, normals at 48 and one UV set at 96 of the vertex buffer).
    let mut surface = ObjectBytes::new()
        .u32(2)
        .u32(4)
        .u32(1)
        .raw(&path(135, 0, 0))
        .raw(&path(133, 0, 0))
        .raw(&path(133, 0, 48))
        .raw(&absent())
        .raw(&absent())
        .raw(&absent());
    for slot in 0..8u32 {
        surface = surface.u32(if slot == 0 { 3 } else { 0 });
    }
    surface = surface.raw(&path(132, 0, 96));
    for _ in 1..8 {
        surface = surface.raw(&absent());
    }
    surface = surface
        .object_ref(match spec.material {
            Mat::Embedded => Some(6),
            Mat::None => None,
            Mat::External => Some(8),
            Mat::NotAPreset => Some(4),
        })
        .array_header(b"STAR", 0);
    let lod = ObjectBytes::new()
        .f32(-1.0)
        .f32(-1.0)
        .f32(-1.0)
        .f32(1.0)
        .f32(1.0)
        .f32(1.0)
        .array_header(b"STAR", 1)
        .array_header(b"STAR", 1)
        .raw(&surface.finish())
        .f32(0.0)
        .u32(0xFFFF_FFFF);
    let holder = |id: u32| ObjectBytes::new().object_ref(Some(id)).finish();
    b.object(
        3,
        26,
        ObjectBytes::new()
            .array_header(b"STAR", 1)
            .raw(&lod.finish())
            .array_header(b"STAR", 1)
            .raw(&holder(4))
            .array_header(b"STAR", 1)
            .raw(&holder(5))
            .f32(0.0)
            .f32(500.0)
            .f32(500.0)
            .finish(),
    );
    let buffer = |kind: u32, data: &[u8]| {
        ObjectBytes::new()
            .i32(-1)
            .object_ref(None)
            .u32(kind)
            .array_header(b"STAR", data.len() as u32)
            .raw(data)
            .finish()
    };
    let indices: Vec<u8> = [0u16, 1, 2, 2, 1, 3]
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    let mut vertices = f32s(&[
        -1.0, 0.0, -1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, 1.0,
    ]);
    vertices.extend(f32s(&[0.0, 1.0, 0.0].repeat(4)));
    vertices.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
    b.object(4, 10, buffer(0x5F, &indices));
    b.object(5, 10, buffer(0x1F, &vertices));
    // The preset: no configs, no physical material.
    b.object(
        6,
        31,
        ObjectBytes::new()
            .i32(-1)
            .object_ref(None)
            .array_header(b"STAR", 0)
            .object_ref(None)
            .finish(),
    );
    if let Some(role) = spec.second {
        b.object(9, 35, configuration(role, &[]));
    }
    b.build().unwrap()
}

fn read(spec: &Spec) -> Result<Model> {
    Model::from_bytes(&model_file(spec))
}

fn err(spec: &Spec) -> String {
    read(spec).unwrap_err().to_string()
}

#[test]
fn reads_the_embedded_mesh_and_its_material() {
    let model = read(&Spec::new(&[Inst::Embedded])).unwrap();
    assert_eq!(
        model.bounds,
        Aabb {
            min: [-1.0, 0.0, -2.0],
            max: [1.0, 3.5, 1.5]
        }
    );
    assert_eq!(model.distances, [990.0, 1000.0, 150.0]);
    assert_eq!(model.meshes.len(), 1);
    let MeshSource::Embedded(mesh) = model.mesh() else {
        panic!("an embedded mesh expected, got {:?}", model.mesh());
    };
    assert_eq!(mesh.lods.len(), 1);
    let surface = &mesh.lods[0].surfaces[0];
    assert_eq!(surface.indices, [0, 1, 2, 2, 1, 3]);
    assert_eq!(surface.positions[3], [1.0, 0.0, 1.0]);
    assert_eq!(surface.normals, vec![[0.0, 1.0, 0.0]; 4]);
    assert_eq!(surface.uv_sets[0][3], [1.0, 1.0]);
    assert_eq!(surface.material, MaterialRef::Embedded(6));
    assert_eq!(model.materials.len(), 1);
    assert_eq!(model.materials[0].id, 6);
    assert_eq!(model.material(6).map(|m| m.id), Some(6));
    assert_eq!(model.material_of(surface).map(|m| m.id), Some(6));
    assert!(model.material(5).is_none());
    // The preset has no configs, so no base colour texture.
    assert_eq!(model.materials[0].base_color_texture(), None);
}

#[test]
fn an_external_mesh_is_named_by_its_path() {
    let model = read(&Spec::new(&[Inst::External])).unwrap();
    assert_eq!(
        model.mesh(),
        &MeshSource::External("Content/Test/Models/Ext.bmf".into())
    );
    assert!(
        model.materials.is_empty(),
        "an external mesh's materials are in its .bmf"
    );
}

#[test]
fn mesh_instances_can_be_null_missing_or_several() {
    let null = read(&Spec::new(&[Inst::Null])).unwrap();
    assert_eq!(null.meshes, [MeshSource::None]);
    let none = read(&Spec::new(&[])).unwrap();
    assert!(none.meshes.is_empty());
    assert_eq!(none.mesh(), &MeshSource::None);
    let two = read(&Spec::new(&[Inst::External, Inst::Embedded])).unwrap();
    assert_eq!(two.meshes.len(), 2);
    assert!(matches!(two.mesh(), MeshSource::External(_)));
    assert!(matches!(two.meshes[1], MeshSource::Embedded(_)));
    assert_eq!(
        two.materials.len(),
        1,
        "materials of every embedded instance"
    );
}

#[test]
fn surfaces_without_or_with_an_external_material_add_no_preset() {
    for (material, expected) in [
        (Mat::None, MaterialRef::None),
        (
            Mat::External,
            MaterialRef::External("Content/Test/Models/Ext.mtr".into()),
        ),
    ] {
        let mut spec = Spec::new(&[Inst::Embedded]);
        spec.material = material;
        let model = read(&spec).unwrap();
        let MeshSource::Embedded(mesh) = model.mesh() else {
            panic!("an embedded mesh expected");
        };
        let surface = &mesh.lods[0].surfaces[0];
        assert_eq!(surface.material, expected);
        assert!(model.materials.is_empty());
        assert!(model.material_of(surface).is_none());
    }
}

#[test]
fn only_the_main_configuration_counts() {
    // A helper configuration (CResource.1 = −1) next to the main one is ignored.
    let mut helper = Spec::new(&[Inst::Embedded]);
    helper.second = Some(-1);
    assert_eq!(read(&helper).unwrap().meshes.len(), 1);
    // No main configuration.
    let mut no_main = Spec::new(&[Inst::Embedded]);
    no_main.role = -1;
    assert!(err(&no_main).contains("no CModelConfiguration"));
    // Two main configurations.
    let mut two = Spec::new(&[Inst::Embedded]);
    two.second = Some(1);
    assert!(err(&two).contains("more than one"));
}

#[test]
fn a_configuration_object_reads_on_its_own_whatever_its_role() {
    // As a world embeds it: any configuration object of a decoded container, the
    // `CResource.1` rule of a `.mdl` file aside.
    let mut spec = Spec::new(&[Inst::Embedded]);
    spec.role = -1;
    spec.second = Some(-1);
    let bytes = model_file(&spec);
    let container = Container::parse(&bytes).unwrap();
    let decoded = container.decode_objects().unwrap();
    let model = Model::from_object(&container, &decoded, 1).unwrap();
    assert_eq!(model, read(&Spec::new(&[Inst::Embedded])).unwrap());
    let helper = Model::from_object(&container, &decoded, 9).unwrap();
    assert!(helper.meshes.is_empty());
    // Not a configuration, or no object at all.
    let not = Model::from_object(&container, &decoded, 2).unwrap_err();
    assert!(
        not.to_string().contains("not a CModelConfiguration"),
        "{not}"
    );
    assert!(matches!(
        Model::from_object(&container, &decoded, 99),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn broken_references_are_refused() {
    assert!(err(&Spec::new(&[Inst::Dangling])).contains("object 99"));
    let mut not_a_preset = Spec::new(&[Inst::Embedded]);
    not_a_preset.material = Mat::NotAPreset;
    assert!(err(&not_a_preset).contains("not a CShaderPreset"));
    assert!(Model::from_bytes(b"junk").is_err());
    assert!(Model::from_bytes(&[]).is_err());
}

#[test]
fn nothing_panics_on_damaged_files() {
    let good = model_file(&Spec::new(&[Inst::Embedded, Inst::External]));
    for len in (0..good.len()).step_by(7) {
        let _ = Model::from_bytes(&good[..len]);
    }
    for i in (0..good.len()).step_by(3) {
        let mut bad = good.clone();
        bad[i] ^= 0xA5;
        let _ = Model::from_bytes(&bad);
    }
}

#[test]
fn the_loader_plugs_into_the_asset_manager() {
    let file = model_file(&Spec::new(&[Inst::Embedded]));
    let vfs = MemoryVfs::from_files([("models/q.mdl", file.as_slice())]).unwrap();
    let assets = AssetManager::new(Arc::new(vfs));
    assets.register_loader(MdlLoader).unwrap();
    let h = assets.load::<Model>("Models/Q.MDL").unwrap();
    assert_eq!(h.wait(), LoadState::Ready);
    assert_eq!(h.get().unwrap().materials.len(), 1);
}

/// Data-gated: every `.mdl` of the retail install reads, with the census of `mdl.md` (3,707
/// files; mesh instances: 3,517 embedded, 127 external `.bmf`, 1 null; 69 models without and 7
/// with two instances; the embedded meshes hold 4,087 LODs and 7,351 surfaces). It also measures
/// what a level loader will meet: the kinds of surface material references, how many embedded
/// presets name a base colour texture, whether those textures and the external `.bmf`/`.mtr`
/// files exist in the install, and that every external `.bmf` reads. Skips without
/// `SS2_DATA_DIR`.
#[test]
fn every_model_of_the_real_install_reads() {
    use ss2_vfs::{EntryKind, Vfs, VfsPath};
    use std::collections::{BTreeMap, BTreeSet};
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let exists = |p: &str| VfsPath::new(p).is_ok_and(|p| vfs.stat(&p).is_ok());
    let mut stack = vec![VfsPath::root()];
    let mut failures = Vec::new();
    let mut files = 0usize;
    let mut instances_per_model: BTreeMap<usize, usize> = BTreeMap::new();
    let (mut embedded, mut external, mut null) = (0usize, 0usize, 0usize);
    let (mut lods, mut surfaces, mut triangles) = (0usize, 0usize, 0usize);
    let (mut mat_embedded, mut mat_external, mut mat_none) = (0usize, 0usize, 0usize);
    let (mut presets, mut with_base, mut surfaces_with_base) = (0usize, 0usize, 0usize);
    let mut base_textures = BTreeSet::new();
    let mut external_meshes = BTreeSet::new();
    let mut external_materials = BTreeSet::new();
    let mut no_base_classes: BTreeMap<String, usize> = BTreeMap::new();
    // Collision meshes of the embedded meshes (T02-042): models with one, meshes, empty ones,
    // triangles and tree references.
    let (mut col_models, mut col_meshes, mut col_empty) = (0usize, 0usize, 0usize);
    let (mut col_tris, mut col_refs) = (0usize, 0usize);
    while let Some(d) = stack.pop() {
        for e in vfs.list(&d).unwrap() {
            let path = d.join(&e.name).unwrap();
            if e.kind == EntryKind::Dir {
                stack.push(path);
                continue;
            }
            if path.extension() != Some("mdl") {
                continue;
            }
            let model = match Model::from_bytes(&vfs.read(&path).unwrap()) {
                Ok(m) => m,
                Err(e) => {
                    failures.push(format!("{}: {e}", path.as_str()));
                    continue;
                }
            };
            files += 1;
            col_models += usize::from(
                model
                    .meshes
                    .iter()
                    .any(|m| matches!(m, MeshSource::Embedded(mesh) if mesh.collision.is_some())),
            );
            *instances_per_model.entry(model.meshes.len()).or_default() += 1;
            presets += model.materials.len();
            for m in &model.materials {
                match m.base_color_texture() {
                    Some(t) => {
                        with_base += 1;
                        base_textures.insert(t.to_owned());
                    }
                    None => {
                        let class = m
                            .configs
                            .first()
                            .map_or("(no config)", |c| c.args_class.as_str());
                        *no_base_classes.entry(class.to_owned()).or_default() += 1;
                    }
                }
            }
            for source in &model.meshes {
                let mesh = match source {
                    MeshSource::Embedded(mesh) => mesh,
                    MeshSource::External(p) => {
                        external += 1;
                        external_meshes.insert(p.clone());
                        continue;
                    }
                    MeshSource::None => {
                        null += 1;
                        continue;
                    }
                };
                embedded += 1;
                if let Some(c) = &mesh.collision {
                    col_meshes += 1;
                    col_empty += usize::from(c.is_empty());
                    col_tris += c.triangle_count();
                    col_refs += c.node_triangles.len();
                }
                for lod in &mesh.lods {
                    lods += 1;
                    for s in &lod.surfaces {
                        surfaces += 1;
                        triangles += s.triangle_count();
                        match &s.material {
                            MaterialRef::Embedded(_) => {
                                mat_embedded += 1;
                                let m = model.material_of(s).expect("every embedded preset");
                                surfaces_with_base += usize::from(m.base_color_texture().is_some());
                            }
                            MaterialRef::External(p) => {
                                mat_external += 1;
                                external_materials.insert(p.clone());
                            }
                            MaterialRef::None => mat_none += 1,
                        }
                    }
                }
            }
        }
    }
    let missing_textures: Vec<_> = base_textures.iter().filter(|t| !exists(t)).collect();
    let missing_meshes: Vec<_> = external_meshes.iter().filter(|p| !exists(p)).collect();
    let missing_materials: Vec<_> = external_materials.iter().filter(|p| !exists(p)).collect();
    // Every external .bmf that exists reads, and its surfaces' material references are counted.
    let (mut ext_embedded, mut ext_external, mut ext_none) = (0usize, 0usize, 0usize);
    for p in external_meshes.iter().filter(|p| exists(p)) {
        match Mesh::from_bytes(&vfs.read(&VfsPath::new(p).unwrap()).unwrap()) {
            Ok(mesh) => {
                for s in mesh.lods.iter().flat_map(|l| &l.surfaces) {
                    match s.material {
                        MaterialRef::Embedded(_) => ext_embedded += 1,
                        MaterialRef::External(_) => ext_external += 1,
                        MaterialRef::None => ext_none += 1,
                    }
                }
            }
            Err(e) => failures.push(format!("external mesh {p}: {e}")),
        }
    }
    eprintln!(
        "{files} models; instances per model {instances_per_model:?}; mesh instances: {embedded} embedded, {external} external, {null} null; \
         embedded meshes: {lods} LODs, {surfaces} surfaces, {triangles} triangles; \
         surface materials: {mat_embedded} embedded, {mat_external} external .mtr, {mat_none} none; \
         {presets} presets, {with_base} with a base colour texture (no base colour by first config class: {no_base_classes:?}); \
         {surfaces_with_base} of {mat_embedded} embedded-material surfaces have a base colour texture; \
         {} distinct base colour textures, {} missing from the install {:?}; \
         {} distinct external meshes ({} missing {:?}), their surfaces: {ext_embedded} embedded, {ext_external} external, {ext_none} no material; \
         {} distinct external materials ({} missing {:?}); {} failures",
        base_textures.len(),
        missing_textures.len(),
        &missing_textures[..missing_textures.len().min(5)],
        external_meshes.len(),
        missing_meshes.len(),
        &missing_meshes[..missing_meshes.len().min(5)],
        external_materials.len(),
        missing_materials.len(),
        &missing_materials[..missing_materials.len().min(5)],
        failures.len()
    );
    eprintln!(
        "collision meshes: {col_models} models, {col_meshes} embedded meshes ({col_empty} empty), {col_tris} triangles, {col_refs} tree references"
    );
    assert!(
        failures.is_empty(),
        "first failures: {:?}",
        &failures[..failures.len().min(10)]
    );
    assert_eq!(
        col_models, 3023,
        "mdl.md: 3,023 models have a collision mesh"
    );
    assert_eq!(files, 3707, "mdl.md counts 3,707 models");
    assert_eq!((embedded, external, null), (3517, 127, 1), "mdl.md");
    assert_eq!(instances_per_model.get(&0), Some(&69), "mdl.md");
    assert_eq!(instances_per_model.get(&2), Some(&7), "mdl.md");
    assert_eq!((lods, surfaces), (4087, 7351), "mdl.md");
    // Measured by this test on the retail install with patches (not stated by the spec).
    assert_eq!(
        (mat_embedded, mat_external, mat_none),
        (7127, 1, 223),
        "surface material references of the embedded meshes"
    );
    assert_eq!(
        (presets, with_base),
        (7127, 6888),
        "presets with a base colour"
    );
    assert!(missing_meshes.is_empty() && missing_materials.is_empty());
}
