//! Copied from the unit tests of `ss2-format-bmf` (`crates/ss2-format-bmf/src/lib.rs`): one quad
//! surface, optionally with a collision mesh (`bmf.md`, "Collision mesh").
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// A surface description for the synthetic file.
pub struct Spec {
    pub triangles: u32,
    pub vertices: u32,
    pub index_offset: u32,
    pub position_offset: u32,
    pub normal_offset: u32,
    pub uv_offset: u32,
    pub tangent_offset: Option<u32>,
    pub skin: Option<(u32, u32, Vec<i32>)>,
    pub material: Option<u32>,
}

pub fn path(code: u32, buffer: u32, offset: u32) -> Vec<u8> {
    ObjectBytes::new()
        .u32(code)
        .u32(buffer)
        .u32(offset)
        .finish()
}

pub fn absent() -> Vec<u8> {
    path(0, 255, 0xFFFF_FFFF)
}

/// One LOD with one surface; `index_bytes` and `vertex_bytes` are the two buffers.
pub fn mesh_file(spec: &Spec, platform: u32, index_bytes: &[u8], vertex_bytes: &[u8]) -> Vec<u8> {
    mesh_file_with(spec, platform, index_bytes, vertex_bytes, None)
}

/// [`mesh_file`] with an optional collision mesh (object 6).
pub fn mesh_file_with(
    spec: &Spec,
    platform: u32,
    index_bytes: &[u8],
    vertex_bytes: &[u8],
    collision: Option<&Coll>,
) -> Vec<u8> {
    let mut b = MetaBuilder::new();
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
    b.type_block(TypeBlock::pointer(19, "CResource*", 9));
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
        &[("1", 27), ("2", 42)],
    ));
    // The material is a second, empty resource object (a stand-in for a CShaderPreset).
    b.type_block(TypeBlock::class(29, "CShaderPreset", 1, 5, Some(9), &[]));
    collision_types(&mut b);

    b.object(0, 7, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        28,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .object_ref(Some(2))
            .object_ref(collision.map(|_| 6))
            .finish(),
    );
    // The render mesh, its LOD with one section and one surface (inline).
    let mut surface = ObjectBytes::new()
        .u32(spec.triangles)
        .u32(spec.vertices)
        .u32(1)
        .raw(&path(135, 0, spec.index_offset))
        .raw(&path(133, 0, spec.position_offset))
        .raw(&path(133, 0, spec.normal_offset));
    surface = surface.raw(&spec.tangent_offset.map_or_else(absent, |o| path(134, 0, o)));
    match &spec.skin {
        Some((w, i, _)) => surface = surface.raw(&path(128, 0, *w)).raw(&path(128, 0, *i)),
        None => surface = surface.raw(&absent()).raw(&absent()),
    }
    for slot in 0..8u32 {
        surface = surface.u32(if slot == 0 { 3 } else { 0 });
    }
    surface = surface.raw(&path(132, 0, spec.uv_offset));
    for _ in 1..8 {
        surface = surface.raw(&absent());
    }
    surface = surface.object_ref(spec.material);
    let palette = spec.skin.as_ref().map_or(&[][..], |s| s.2.as_slice());
    surface = surface.array_header(b"STAR", palette.len() as u32);
    for p in palette {
        surface = surface.i32(*p);
    }
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
        .f32(150.0)
        .u32(platform);
    let holder = |id: u32| ObjectBytes::new().object_ref(Some(id)).finish();
    b.object(
        2,
        26,
        ObjectBytes::new()
            .array_header(b"STAR", 1)
            .raw(&lod.finish())
            .array_header(b"STAR", 1)
            .raw(&holder(3))
            .array_header(b"STAR", 1)
            .raw(&holder(4))
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
    b.object(3, 10, buffer(0x5F, index_bytes));
    b.object(4, 10, buffer(0x1F, vertex_bytes));
    b.object(5, 29, ObjectBytes::new().i32(-1).object_ref(None).finish());
    if let Some(c) = collision {
        b.object(6, 41, collision_object(c));
    }
    b.build().unwrap()
}

/// A collision mesh description for the synthetic file (`bmf.md`, "Collision mesh").
#[derive(Clone)]
pub struct Coll {
    pub render_geometry: bool,
    pub holder: u8,
    pub position_offset: u32,
    pub normal_offset: u32,
    pub vertex_count: u32,
    pub own: Vec<[f32; 3]>,
    pub idx16: Vec<u16>,
    pub idx32: Vec<u32>,
    pub surface_ids: Vec<u8>,
    /// The double-sided flag of each table entry; the first refers to the preset (object 5).
    pub surfaces: Vec<bool>,
    pub edge_flags: Vec<u8>,
    pub nodes: Vec<[u16; 4]>,
    pub refs: Vec<i32>,
    pub bounds: [f32; 6],
}

/// The types of a `CCollisionMesh` (ids 30 to 42), with the member order of the retail files.
pub fn collision_types(b: &mut MetaBuilder) {
    b.type_block(TypeBlock::primitive(30, "UWORD", 2, 2));
    b.type_block(TypeBlock::short(31, "CStaticArray<UWORD>", 0, 6, 30));
    b.type_block(TypeBlock::short(32, "CStaticArray<ULONG>", 0, 6, 2));
    b.type_block(TypeBlock::alias(33, "BOOL", 0));
    b.type_block(TypeBlock::class(
        34,
        "CCollisionMeshSurface",
        3,
        5,
        None,
        &[("4", 33), ("5", 19)],
    ));
    b.type_block(TypeBlock::short(
        35,
        "CStaticArray<CCollisionMeshSurface>",
        0,
        6,
        34,
    ));
    b.type_block(TypeBlock::short(36, "CStaticStackArray<UBYTE>", 0, 7, 4));
    b.type_block(TypeBlock::class(
        37,
        "CCollisionMeshNode",
        1,
        5,
        None,
        &[("1", 30), ("2", 30), ("3", 30), ("4", 30)],
    ));
    b.type_block(TypeBlock::short(
        38,
        "CStaticArray<CCollisionMeshNode>",
        0,
        6,
        37,
    ));
    b.type_block(TypeBlock::class(
        39,
        "Vector3f",
        1,
        5,
        None,
        &[("1", 3), ("2", 3), ("3", 3)],
    ));
    b.type_block(TypeBlock::short(40, "CStaticArray<Vector3f>", 0, 6, 39));
    b.type_block(TypeBlock::class(
        41,
        "CCollisionMesh",
        18,
        5,
        None,
        &[
            ("12", 40),
            ("13", 31),
            ("14", 32),
            ("9", 6),
            ("11", 35),
            ("26", 36),
            ("16", 38),
            ("17", 17),
            ("18", 13),
            ("19", 33),
            ("20", 4),
            ("21", 2),
            ("23", 2),
            ("22", 1),
        ],
    ));
    b.type_block(TypeBlock::pointer(42, "CCollisionMesh*", 41));
}

pub fn collision_object(c: &Coll) -> Vec<u8> {
    let mut o = ObjectBytes::new().array_header(b"STAR", c.own.len() as u32);
    for p in &c.own {
        o = o.f32(p[0]).f32(p[1]).f32(p[2]);
    }
    o = o.array_header(b"STAR", c.idx16.len() as u32);
    for &i in &c.idx16 {
        o = o.u16(i);
    }
    o = o.array_header(b"STAR", c.idx32.len() as u32);
    for &i in &c.idx32 {
        o = o.u32(i);
    }
    o = o
        .array_header(b"STAR", c.surface_ids.len() as u32)
        .raw(&c.surface_ids)
        .array_header(b"STAR", c.surfaces.len() as u32);
    for (k, &double) in c.surfaces.iter().enumerate() {
        o = o
            .u32(u32::from(double))
            .object_ref(if k == 0 { Some(5) } else { None });
    }
    o = o
        .array_header(b"SSAR", c.edge_flags.len() as u32)
        .raw(&c.edge_flags)
        .array_header(b"STAR", c.nodes.len() as u32);
    for n in &c.nodes {
        o = o.u16(n[0]).u16(n[1]).u16(n[2]).u16(n[3]);
    }
    o = o.array_header(b"STAR", c.refs.len() as u32);
    for &r in &c.refs {
        o = o.i32(r);
    }
    for v in c.bounds {
        o = o.f32(v);
    }
    o.u32(u32::from(c.render_geometry))
        .u8(c.holder)
        .u32(c.position_offset)
        .u32(c.normal_offset)
        .u32(c.vertex_count)
        .finish()
}

/// The quad's two triangles as a collision mesh on the render vertices, with a root split at
/// x = 0 and one triangle in each child.
pub fn quad_collision() -> Coll {
    Coll {
        render_geometry: true,
        holder: 0,
        position_offset: 0,
        normal_offset: 48,
        vertex_count: 4,
        own: vec![],
        idx16: vec![0, 1, 2, 2, 1, 3],
        idx32: vec![],
        surface_ids: vec![0, 1],
        surfaces: vec![false, true],
        edge_flags: vec![0x21, 0x30],
        nodes: vec![
            [8192, 1, 0, 0],
            [0xFFFF, 0xFFFF, 1, 0],
            [0xFFFF, 0xFFFF, 1, 1],
        ],
        refs: vec![0, 1],
        bounds: [-1.0, 0.0, -1.0, 1.0, 0.0, 1.0],
    }
}

pub fn f32s(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A quad: 4 vertices, 2 triangles, positions + normals + one UV set.
pub fn quad() -> (Spec, Vec<u8>, Vec<u8>) {
    let idx: Vec<u8> = [0u16, 1, 2, 2, 1, 3]
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    let mut vtx = f32s(&[
        -1.0, 0.0, -1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, 1.0,
    ]);
    vtx.extend(f32s(&[0.0, 1.0, 0.0].repeat(4)));
    vtx.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
    (
        Spec {
            triangles: 2,
            vertices: 4,
            index_offset: 0,
            position_offset: 0,
            normal_offset: 48,
            uv_offset: 96,
            tangent_offset: None,
            skin: None,
            material: Some(5),
        },
        idx,
        vtx,
    )
}

/// The quad with its two-triangle collision mesh and a three-node tree.
pub fn quad_with_collision() -> Vec<u8> {
    let (spec, idx, vtx) = quad();
    mesh_file_with(&spec, 0xFFFF_FFFF, &idx, &vtx, Some(&quad_collision()))
}

/// The quad with a collision mesh using 32-bit indices and its own positions (no tree).
pub fn quad_with_own_collision() -> Vec<u8> {
    let (spec, idx, vtx) = quad();
    let mut c = quad_collision();
    c.render_geometry = false;
    c.vertex_count = 0;
    c.own = vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    c.idx16.clear();
    c.idx32 = vec![0, 2, 1];
    c.surface_ids = vec![3];
    c.edge_flags = vec![];
    c.nodes = vec![[0xFFFF, 0xFFFF, 1, 0]];
    c.refs = vec![0];
    mesh_file_with(&spec, 0xFFFF_FFFF, &idx, &vtx, Some(&c))
}
