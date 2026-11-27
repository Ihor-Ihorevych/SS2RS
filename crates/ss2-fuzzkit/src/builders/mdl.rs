//! Copied from the unit tests of `ss2-format-mdl` (`crates/ss2-format-mdl/src/tests.rs`).
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// What a mesh instance of the synthetic model refers to.
#[derive(Clone, Copy)]
pub enum Inst {
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
pub enum Mat {
    /// The embedded preset (object 6).
    Embedded,
    /// None.
    None,
    /// The external `.mtr` (object 8).
    External,
    /// An embedded object that is not a preset (the index buffer, object 4).
    NotAPreset,
}

pub struct Spec {
    pub instances: Vec<Inst>,
    /// `CResource.1` of the configuration.
    pub role: i32,
    /// Add a second configuration (object 9) with this `CResource.1`.
    pub second: Option<i32>,
    pub material: Mat,
}

impl Spec {
    pub fn new(instances: &[Inst]) -> Self {
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
pub fn model_file(spec: &Spec) -> Vec<u8> {
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
