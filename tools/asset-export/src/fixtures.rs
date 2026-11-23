//! Hand-made `.tex` and `.bmf` files for the tests (built from the public specs
//! `docs/re/tex.md` and `docs/re/bmf.md` with the testkit's CTSEMETA builder; no game data).
//!
//! This file is compiled into the unit tests of the library and, through a `#[path]` include, into
//! the integration tests, which also use it to check the committed copies under
//! `tools/asset-export/testdata/`.

use flate2::Compression;
use flate2::write::ZlibEncoder;
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
use std::io::Write;

/// A version-8 `.tex` file with `pixels` zlib-compressed into its blob.
pub fn texture_file(
    width: i32,
    height: i32,
    flags: u32,
    format_code: u32,
    wrap: (u32, u32),
    pixels: &[u8],
) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(pixels).expect("writing to a Vec");
    texture_file_with_blob(
        width,
        height,
        flags,
        format_code,
        wrap,
        &z.finish().expect("finish"),
    )
}

/// Like [`texture_file`] with an already compressed (or deliberately broken) blob.
pub fn texture_file_with_blob(
    width: i32,
    height: i32,
    flags: u32,
    format_code: u32,
    wrap: (u32, u32),
    blob: &[u8],
) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::alias(2, "PIX", 0));
    b.type_block(TypeBlock::primitive(3, "ULONG", 4, 4));
    b.type_block(TypeBlock::alias(4, "COLOR", 3));
    b.type_block(TypeBlock::enumeration(5, "TexFlags", 4));
    b.type_block(TypeBlock::enumeration(6, "TexFormat", 4));
    b.type_block(TypeBlock::enumeration(7, "GfxTexWrap", 4));
    b.type_block(TypeBlock::primitive(8, "UBYTE", 1, 1));
    b.type_block(TypeBlock::short(9, "CStaticArray<UBYTE>", 0, 6, 8));
    b.type_block(TypeBlock::class(
        10,
        "CResourceFile",
        3,
        5,
        None,
        &[("1", 1)],
    ));
    b.type_block(TypeBlock::pointer(11, "CResourceFile*", 10));
    b.type_block(TypeBlock::class(
        12,
        "CResource",
        4,
        5,
        None,
        &[("1", 1), ("2", 11)],
    ));
    b.type_block(TypeBlock::class(
        13,
        "CBaseTexture",
        9,
        5,
        Some(12),
        &[
            ("1", 2),
            ("2", 2),
            ("4", 5),
            ("6", 6),
            ("18", 9),
            ("9", 7),
            ("10", 7),
            ("14", 4),
        ],
    ));
    b.type_block(TypeBlock::class(14, "CTexture", 1, 5, Some(13), &[]));
    b.type_block(TypeBlock::class(15, "CStaticTexture", 1, 5, Some(14), &[]));
    b.object(0, 10, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        15,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .i32(width)
            .i32(height)
            .u32(flags)
            .u32(format_code)
            .array_header(b"STAR", blob.len() as u32)
            .raw(blob)
            .u32(wrap.0)
            .u32(wrap.1)
            .u32(0)
            .finish(),
    );
    b.build().expect("a consistent synthetic texture")
}

/// One surface of a synthetic mesh: where its streams start in the two buffers.
#[derive(Clone, Debug)]
pub struct Spec {
    /// Triangle count.
    pub triangles: u32,
    /// Vertex count.
    pub vertices: u32,
    /// Byte offset of the index list in the index buffer.
    pub index_offset: u32,
    /// Byte offset of the positions in the vertex buffer.
    pub position_offset: u32,
    /// Byte offset of the normals.
    pub normal_offset: u32,
    /// Byte offset of the (single) UV set.
    pub uv_offset: u32,
    /// Byte offset of the tangents, if any.
    pub tangent_offset: Option<u32>,
    /// Bone weights offset, bone indices offset and the palette, if skinned.
    pub skin: Option<(u32, u32, Vec<i32>)>,
    /// The material: `None` for none, `Some(id)` for the embedded preset object `id`.
    pub material: Option<u32>,
}

/// One LOD of a synthetic mesh.
#[derive(Clone, Debug)]
pub struct LodSpec {
    /// `EPlatform` mask.
    pub platform: u32,
    /// Switch distance.
    pub distance: f32,
    /// Bounding box minimum and maximum.
    pub bounds: ([f32; 3], [f32; 3]),
    /// The surfaces.
    pub surfaces: Vec<Spec>,
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

fn surface_bytes(spec: &Spec) -> Vec<u8> {
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
    surface.finish()
}

/// A `.bmf` with the given LODs; `index_bytes` and `vertex_bytes` are the two buffers. Object 5
/// is an empty `CShaderPreset` (a stand-in material).
pub fn mesh_file(lods: &[LodSpec], index_bytes: &[u8], vertex_bytes: &[u8]) -> Vec<u8> {
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
        &[("1", 27), ("2", 27)],
    ));
    b.type_block(TypeBlock::class(29, "CShaderPreset", 1, 5, Some(9), &[]));

    b.object(0, 7, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        28,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .object_ref(Some(2))
            .object_ref(None)
            .finish(),
    );
    let mut lod_list = ObjectBytes::new().array_header(b"STAR", lods.len() as u32);
    for lod in lods {
        let mut section = ObjectBytes::new()
            .f32(lod.bounds.0[0])
            .f32(lod.bounds.0[1])
            .f32(lod.bounds.0[2])
            .f32(lod.bounds.1[0])
            .f32(lod.bounds.1[1])
            .f32(lod.bounds.1[2])
            .array_header(b"STAR", 1)
            .array_header(b"STAR", lod.surfaces.len() as u32);
        for s in &lod.surfaces {
            section = section.raw(&surface_bytes(s));
        }
        lod_list = lod_list.raw(&section.f32(lod.distance).u32(lod.platform).finish());
    }
    let holder = |id: u32| ObjectBytes::new().object_ref(Some(id)).finish();
    b.object(
        2,
        26,
        ObjectBytes::new()
            .raw(&lod_list.finish())
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
    b.build().expect("a consistent synthetic mesh")
}

/// `f32` values as little-endian bytes.
pub fn f32s(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// `u16` values as little-endian bytes.
pub fn u16s(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A quad in the XZ plane: 4 vertices, 2 triangles, positions, normals and one UV set. Returns
/// the surface spec (offsets inside the returned buffers), the index bytes and the vertex bytes.
pub fn quad() -> (Spec, Vec<u8>, Vec<u8>) {
    let idx = u16s(&[0, 1, 2, 2, 1, 3]);
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
            material: None,
        },
        idx,
        vtx,
    )
}

/// One LOD with `surfaces` and a box of -1..1 in X and Z, 0..1 in Y.
pub fn lod(surfaces: Vec<Spec>) -> LodSpec {
    LodSpec {
        platform: 0xFFFF_FFFF,
        distance: 150.0,
        bounds: ([-1.0, 0.0, -1.0], [1.0, 1.0, 1.0]),
        surfaces,
    }
}

/// The smallest useful mesh file: the quad of [`quad`], one LOD.
pub fn quad_file() -> Vec<u8> {
    let (spec, idx, vtx) = quad();
    mesh_file(&[lod(vec![spec])], &idx, &vtx)
}

/// A mesh with two LODs and, in the first, two surfaces (a quad and a tetrahedron-like set with
/// tangents), an embedded (empty) material on the first surface, and a skinned third surface;
/// every feature the exporter writes. Returns the file.
pub fn rich_file() -> Vec<u8> {
    let (quad_spec, quad_idx, quad_vtx) = quad();
    // Second surface: a triangle with tangents, appended to the same buffers.
    let mut idx = quad_idx.clone();
    idx.extend(u16s(&[0, 1, 2]));
    let mut vtx = quad_vtx.clone();
    let base = vtx.len() as u32;
    vtx.extend(f32s(&[0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0])); // positions
    vtx.extend(f32s(&[0.0, 0.0, 1.0].repeat(3))); // normals
    vtx.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0])); // uv
    vtx.extend(f32s(&[1.0, 0.0, 0.0, 1.0].repeat(3))); // tangents
    let tri = Spec {
        triangles: 1,
        vertices: 3,
        index_offset: quad_idx.len() as u32,
        position_offset: base,
        normal_offset: base + 36,
        uv_offset: base + 72,
        tangent_offset: Some(base + 96),
        skin: None,
        material: None,
    };
    let mut first = quad_spec.clone();
    first.material = Some(5);
    // Third surface: a skinned triangle.
    let skin_base = vtx.len() as u32;
    let mut idx2 = idx.clone();
    let skin_idx_offset = idx2.len() as u32;
    idx2.extend(u16s(&[0, 1, 2]));
    vtx.extend(f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]));
    vtx.extend(f32s(&[0.0, 0.0, 1.0].repeat(3)));
    vtx.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]));
    vtx.extend([255u8, 0, 0, 0, 200, 55, 0, 0, 0, 0, 0, 0]);
    vtx.extend([0u8, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0]);
    let skinned = Spec {
        triangles: 1,
        vertices: 3,
        index_offset: skin_idx_offset,
        position_offset: skin_base,
        normal_offset: skin_base + 36,
        uv_offset: skin_base + 72,
        tangent_offset: None,
        skin: Some((skin_base + 96, skin_base + 108, vec![4, 5, 6])),
        material: None,
    };
    let near = lod(vec![first.clone(), tri.clone(), skinned]);
    let mut far = lod(vec![first]);
    far.distance = 600.0;
    mesh_file(&[near, far], &idx2, &vtx)
}

/// A 4 x 4 DXT1 texture with its three levels (red, green and blue blocks).
pub fn dxt1_file() -> Vec<u8> {
    let block = |c0: u16, c1: u16| {
        let mut b = Vec::new();
        b.extend(c0.to_le_bytes());
        b.extend(c1.to_le_bytes());
        b.extend([0x00, 0x00, 0x00, 0x00]);
        b
    };
    let mut px = block(0xF800, 0x0000);
    px.extend(block(0x07E0, 0x0000));
    px.extend(block(0x001F, 0x0000));
    texture_file(4, 4, 0x21, 7, (0x50, 0x50), &px)
}

/// An 8 x 4 24-bit texture: a horizontal ramp with distinct R, G and B.
pub fn bgr_file() -> Vec<u8> {
    let mut px = Vec::new();
    for _y in 0..4u8 {
        for x in 0..8u8 {
            px.extend([x * 30, 100 + x, 255 - x * 10]); // B, G, R
        }
    }
    texture_file(8, 4, 0x20, 0, (0x50, 0x50), &px)
}

/// A 4 x 4 32-bit texture with a varying alpha.
pub fn bgra_file() -> Vec<u8> {
    let mut px = Vec::new();
    for i in 0..16u8 {
        px.extend([i, 255 - i, i * 3, i * 16]);
    }
    texture_file(4, 4, 0x04, 1, (0x51, 0x51), &px)
}

/// Bytes of one DXT level (`block_bytes` is 8 for DXT1, 16 for DXT3 and DXT5).
pub fn dxt_level_bytes(w: u32, h: u32, block_bytes: u32) -> usize {
    (w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * block_bytes) as usize
}

/// A DXT texture file (format code 7, 8 or 9): `level0` is the largest level, the other levels of
/// the full chain are filled with `fill` bytes.
pub fn dxt_file(code: u32, w: u32, h: u32, flags: u32, level0: &[u8], fill: u8) -> Vec<u8> {
    let block = if code == 7 { 8 } else { 16 };
    assert_eq!(level0.len(), dxt_level_bytes(w, h, block), "level 0 size");
    let levels = u32::BITS - w.max(h).leading_zeros();
    let mut data = level0.to_vec();
    for l in 1..levels {
        data.extend(std::iter::repeat_n(
            fill,
            dxt_level_bytes((w >> l).max(1), (h >> l).max(1), block),
        ));
    }
    texture_file(w as i32, h as i32, flags, code, (0x50, 0x50), &data)
}

fn expand565(c: u16) -> [i32; 3] {
    let (r, g, b) = (
        i32::from(c >> 11) & 31,
        i32::from(c >> 5) & 63,
        i32::from(c) & 31,
    );
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    ]
}

fn to565(rgb: [u8; 3]) -> u16 {
    ((u16::from(rgb[0]) >> 3) << 11) | ((u16::from(rgb[1]) >> 2) << 5) | (u16::from(rgb[2]) >> 3)
}

/// A simple BC1 colour-block encoder for the tests: the two end points are the darkest and the
/// brightest pixel (by channel sum), every pixel takes the nearest of the four colours.
pub fn encode_bc1_colour(tile: &[[u8; 4]; 16]) -> [u8; 8] {
    let sum = |p: &[u8; 4]| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]);
    let lo = tile.iter().min_by_key(|p| sum(p)).unwrap();
    let hi = tile.iter().max_by_key(|p| sum(p)).unwrap();
    let (c0, c1) = (to565([hi[0], hi[1], hi[2]]), to565([lo[0], lo[1], lo[2]]));
    let mut out = [0u8; 8];
    out[..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    if c0 == c1 {
        return out; // every index 0: the single colour (three-colour mode is harmless)
    }
    let (a, b) = (expand565(c0), expand565(c1));
    let palette = [
        a,
        b,
        [0, 1, 2].map(|i| (2 * a[i] + b[i]) / 3),
        [0, 1, 2].map(|i| (a[i] + 2 * b[i]) / 3),
    ];
    let mut bits = 0u32;
    for (i, p) in tile.iter().enumerate() {
        let best = (0..4usize)
            .min_by_key(|&k| {
                (0..3)
                    .map(|c| (palette[k][c] - i32::from(p[c])).pow(2))
                    .sum::<i32>()
            })
            .unwrap();
        bits |= (best as u32) << (2 * i);
    }
    out[4..].copy_from_slice(&bits.to_le_bytes());
    out
}

/// BC1 (DXT1) block of an opaque tile.
pub fn encode_bc1(tile: &[[u8; 4]; 16]) -> Vec<u8> {
    encode_bc1_colour(tile).to_vec()
}

/// BC2 (DXT3) block: 4-bit alpha (the top four bits of each alpha) and the BC1 colour block.
pub fn encode_bc2(tile: &[[u8; 4]; 16]) -> Vec<u8> {
    let mut out = vec![0u8; 8];
    for (i, p) in tile.iter().enumerate() {
        out[i / 2] |= (p[3] >> 4) << (4 * (i % 2));
    }
    out.extend(encode_bc1_colour(tile));
    out
}

/// BC3 (DXT5) block: eight-value alpha between the largest and smallest alpha, and the BC1
/// colour block.
pub fn encode_bc3(tile: &[[u8; 4]; 16]) -> Vec<u8> {
    let hi = tile.iter().map(|p| p[3]).max().unwrap();
    let lo = tile.iter().map(|p| p[3]).min().unwrap();
    let mut out = vec![hi, lo];
    let (x, y) = (i32::from(hi), i32::from(lo));
    let mut table = vec![x, y];
    for i in 1..=6 {
        table.push(((7 - i) * x + i * y) / 7);
    }
    let mut bits = 0u64;
    for (i, p) in tile.iter().enumerate() {
        let best = if hi == lo {
            0
        } else {
            (0..8usize)
                .min_by_key(|&k| (table[k] - i32::from(p[3])).abs())
                .unwrap()
        };
        bits |= (best as u64) << (3 * i);
    }
    out.extend(&bits.to_le_bytes()[..6]);
    out.extend(encode_bc1_colour(tile));
    out
}

/// Encode a `w x h` RGBA image (sizes divisible by 4) with `encode` into DXT blocks, row by row.
pub fn encode_dxt(
    rgba: &[u8],
    w: usize,
    h: usize,
    encode: fn(&[[u8; 4]; 16]) -> Vec<u8>,
) -> Vec<u8> {
    let mut out = Vec::new();
    for by in 0..h / 4 {
        for bx in 0..w / 4 {
            let mut tile = [[0u8; 4]; 16];
            for (i, t) in tile.iter_mut().enumerate() {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                t.copy_from_slice(&rgba[(y * w + x) * 4..(y * w + x) * 4 + 4]);
            }
            out.extend(encode(&tile));
        }
    }
    out
}

/// A 4 x 4 DXT1 cube map; face n is flat colour n.
pub fn cube_file() -> Vec<u8> {
    // 4 x 4 DXT1 cube: per face 8 + 8 + 8 bytes; face n is flat colour n.
    let colours: [u16; 6] = [0xF800, 0x07E0, 0x001F, 0xFFE0, 0xF81F, 0x07FF];
    let mut data = Vec::new();
    for c in colours {
        for _ in 0..3 {
            data.extend(c.to_le_bytes());
            data.extend(c.to_le_bytes());
            data.extend([0u8; 4]);
        }
    }
    texture_file(4, 4, 0x23, 7, (0x51, 0x51), &data)
}
