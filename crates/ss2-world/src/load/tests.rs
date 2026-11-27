//! Unit tests on synthetic files written with the testkit's CTSEMETA builder (no game data).
//!
//! The builders follow the shapes of the format crates' own test fixtures (`ss2-format-wld`,
//! `-mdl`, `-mtr`, `-tex`), reduced to what the loader needs: a world placing `Box.mdl` twice (once
//! mirrored), an embedded model and a missing model; the model with one embedded quad whose
//! surface uses an external `.mtr`; that material's base colour texture `body.tex`.

use std::collections::BTreeMap;
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2};
use std::io::Write;
use std::sync::Mutex;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use ss2_core::math::{Quat, Vec3};
use ss2_format_tex::{PixelFormat, Texture, WRAP_CLAMP, WRAP_REPEAT};
use ss2_render_api::scene_data::{AlphaMode, MaterialData, TexelFormat, TextureData};
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
use ss2_vfs::{DirEntry, MemoryVfs, ReadSeek, Stat};

use super::*;
use crate::scene::LoadStage;

const WLD: &str = "Content/Test/Levels/Box/Box.wld";
const BOX_MDL: &str = "Content/Test/Models/Box.mdl";
const MISSING_MDL: &str = "Content/Test/Models/Missing.mdl";
const BOX_MTR: &str = "Content/Test/Models/Box.mtr";
const BODY_TEX: &str = "Content/Test/Textures/body.tex";
const SKY_NORTH_TEX: &str = "Content/Test/Sky/north.tex";
const SKY_UP_TEX: &str = "Content/Test/Sky/up.tex";
const SKY_MISSING_TEX: &str = "Content/Test/Sky/missing.tex";
/// The sky colour of the synthetic world (ARGB, alpha 0 as in most retail skies).
const SKY_COLOR: u32 = 0x00D9_E1F0;

/// 90 degrees about +y, as stored (x, y, z, w).
const ROT_Y90: [f32; 4] = [0.0, FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2];
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn quat_vect(b: ObjectBytes, rotation: [f32; 4], translation: [f32; 3]) -> ObjectBytes {
    let mut b = b;
    for v in rotation.into_iter().chain(translation) {
        b = b.f32(v);
    }
    b
}

/// The bytes of a `CAspect`: parent, local placement, world placement, owner.
fn aspect(parent: Option<u32>, world: ([f32; 4], [f32; 3]), owner: u32) -> ObjectBytes {
    let b = ObjectBytes::new().object_ref(parent);
    let b = quat_vect(b, world.0, world.1);
    let b = quat_vect(b, world.0, world.1);
    b.object_ref(Some(owner))
}

/// A world (`wld-root.md`, `wld-geometry.md`) with:
///
/// - entity 3: `Box.mdl` with a hull, world placement 90 degrees about +y at (6, 0, 0), scale
///   (2, 2, -1) (mirrored);
/// - entity 4: an embedded model (not read by the loader);
/// - entity 7: `Box.mdl` again, no hull, at (0, 0, -5), scale 1;
/// - entity 8: `Missing.mdl` with a hull;
/// - entity 5: a spawn marker at (10, 0, 10) turned 90 degrees about +y;
/// - entity 6: a chapter info entity at (1, 2, 3).
fn world_file(with_spawn: bool) -> Vec<u8> {
    world_file_with(with_spawn, false)
}

/// [`world_file`], with `with_sky` also entity 9: a sky (no property record) whose faces are North
/// `north.tex`, West `body.tex` (the material's texture too), South a missing file, no East and
/// no Floor, Ceiling `up.tex`; colour [`SKY_COLOR`].
fn world_file_with(with_spawn: bool, with_sky: bool) -> Vec<u8> {
    world_file_full(with_spawn, with_sky, None)
}

/// [`world_file`] whose first `Box.mdl` placement (entity 3, instance 13) is lit: its modifier
/// list 100 holds an oven modifier (101) naming the world's own `CStaticTexture` 110, a 4 x 4
/// texture of format `code` with `blob` as its pixel data (`wld-lighting.md`).
fn world_file_lit(code: u32, blob: &[u8]) -> Vec<u8> {
    world_file_full(true, false, Some((code, blob)))
}

fn world_file_full(with_spawn: bool, with_sky: bool, lit: Option<(u32, &[u8])>) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let module = b.file_ref("Bin/Sam2Game.module", 0);
    let mdl = b.file_ref(BOX_MDL, 0);
    let missing = b.file_ref(MISSING_MDL, 0);
    b.external_type(100, "CEntityClass");
    b.external_type(101, "CModelConfiguration");
    b.type_block(TypeBlock::primitive(0, "ULONG", 4, 4));
    b.type_block(TypeBlock::primitive(1, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(2, "CString", 4, 4));
    b.type_block(TypeBlock::class(3, "CObject", 1, 5, None, &[]));
    let xyz = [("1", 1), ("2", 1), ("3", 1)];
    b.type_block(TypeBlock::class(4, "Vector3f", 1, 5, None, &xyz));
    let xyzw = [("1", 1), ("2", 1), ("3", 1), ("4", 1)];
    b.type_block(TypeBlock::class(5, "Quaternion4f", 1, 5, None, &xyzw));
    b.type_block(TypeBlock::class(
        6,
        "QuatVect",
        1,
        5,
        None,
        &[("1", 5), ("2", 4)],
    ));
    b.type_block(TypeBlock::pointer(31, "Ptr<CEntityClass>", 100));
    let entity_members = [("1", 0), ("2", 31), ("4", 0)];
    b.type_block(TypeBlock::class(
        7,
        "CEntity",
        7,
        5,
        Some(3),
        &entity_members,
    ));
    b.type_block(TypeBlock::pointer(8, "CEntity*", 7));
    b.type_block(TypeBlock::dynamic_container(
        9,
        "CDynamicContainer<CEntity>",
        7,
    ));
    b.type_block(TypeBlock::pointer(11, "CEntityProperties*", 10));
    let record_members = [("2", 8), ("3", 6), ("4", 2), ("5", 11)];
    b.type_block(TypeBlock::class(
        10,
        "CEntityProperties",
        4,
        5,
        Some(3),
        &record_members,
    ));
    b.type_block(TypeBlock::dynamic_container(
        12,
        "CDynamicContainer<CEntityProperties>",
        10,
    ));
    let world_members = [("1", 9), ("5", 12), ("10", 4), ("11", 1)];
    b.type_block(TypeBlock::class(13, "CWorld", 14, 5, None, &world_members));
    b.type_block(TypeBlock::pointer(15, "CAspect*", 14));
    let aspect_members = [("1", 15), ("4", 6), ("7", 6), ("5", 8)];
    b.type_block(TypeBlock::class(14, "CAspect", 3, 5, None, &aspect_members));
    b.type_block(TypeBlock::pointer(17, "Ptr<CModelConfiguration>", 101));
    let instance_members = [("1", 4), ("3", 17), ("4", 54)];
    b.type_block(TypeBlock::class(
        16,
        "CModelInstance",
        5,
        5,
        None,
        &instance_members,
    ));
    b.type_block(TypeBlock::pointer(18, "CModelInstance*", 16));
    b.type_block(TypeBlock::class(
        19,
        "CModelRenderable",
        3,
        5,
        Some(14),
        &[("1", 18)],
    ));
    b.type_block(TypeBlock::class(
        20,
        "CEditorPrimitiveRenderable",
        3,
        5,
        Some(14),
        &[],
    ));
    b.type_block(TypeBlock::class(21, "CModelHull", 3, 5, Some(14), &[]));
    b.type_block(TypeBlock::pointer(22, "Handle<CModelRenderable>", 19));
    b.type_block(TypeBlock::class(23, "CMechanism", 3, 5, None, &[]));
    b.type_block(TypeBlock::pointer(24, "Handle<CMechanism>", 23));
    let sm_members = [("1", 22), ("10", 24), ("6", 17)];
    b.type_block(TypeBlock::class(
        25,
        "CStaticModelEntity",
        7,
        5,
        Some(7),
        &sm_members,
    ));
    b.type_block(TypeBlock::pointer(
        26,
        "Handle<CEditorPrimitiveRenderable>",
        20,
    ));
    let marker_members = [("1", 26), ("2", 26)];
    b.type_block(TypeBlock::class(
        27,
        "CSpawnMarkerEntity",
        7,
        5,
        Some(7),
        &marker_members,
    ));
    b.type_block(TypeBlock::class(
        28,
        "CChapterInfoEntity",
        7,
        5,
        Some(7),
        &[],
    ));
    b.type_block(TypeBlock::class(
        29,
        "CStaticModelProperties",
        4,
        5,
        Some(10),
        &[],
    ));
    b.type_block(TypeBlock::class(
        30,
        "CSpawnMarkerProperties",
        4,
        5,
        Some(10),
        &[],
    ));
    b.type_block(TypeBlock::class(
        32,
        "CChapterInfoProperties",
        4,
        5,
        Some(10),
        &[],
    ));
    b.type_block(TypeBlock::class(33, "CModelConfiguration", 3, 5, None, &[]));
    // The lightmap classes (`wld-lighting.md`; the texture as in `tex.md`).
    b.type_block(TypeBlock::primitive(49, "IDENT", 4, 4));
    b.type_block(TypeBlock::class(53, "CShaderModifier", 1, 5, Some(3), &[]));
    b.type_block(TypeBlock::dynamic_container(
        55,
        "CDynamicContainer<CShaderModifier>",
        53,
    ));
    b.type_block(TypeBlock::class(
        56,
        "CShaderModifierList",
        1,
        5,
        None,
        &[("1", 55)],
    ));
    b.type_block(TypeBlock::pointer(54, "CShaderModifierList*", 56));
    b.type_block(TypeBlock::pointer(57, "Ptr<CTexture>", 71));
    b.type_block(TypeBlock::class(
        58,
        "CShaderArgValTexture",
        1,
        5,
        None,
        &[("1", 57)],
    ));
    b.type_block(TypeBlock::class(
        59,
        "CShaderArgValUVMap",
        1,
        5,
        None,
        &[("1", 49)],
    ));
    b.type_block(TypeBlock::class(
        60,
        "CTextureOvenShaderModifier",
        4,
        5,
        Some(53),
        &[("4", 58), ("1", 49), ("3", 59), ("2", 49)],
    ));
    b.type_block(TypeBlock::primitive(62, "SLONG", 4, 4));
    b.type_block(TypeBlock::primitive(63, "UBYTE", 1, 1));
    b.type_block(TypeBlock::enumeration(64, "TexFlags", 4));
    b.type_block(TypeBlock::enumeration(65, "TexFormat", 4));
    b.type_block(TypeBlock::enumeration(66, "GfxTexWrap", 4));
    b.type_block(TypeBlock::short(67, "CStaticArray<UBYTE>", 0, 6, 63));
    b.type_block(TypeBlock::class(
        69,
        "CBaseTexture",
        8,
        5,
        None,
        &[
            ("1", 62),
            ("2", 62),
            ("4", 64),
            ("6", 65),
            ("18", 67),
            ("9", 66),
            ("10", 66),
            ("14", 0),
        ],
    ));
    b.type_block(TypeBlock::class(71, "CStaticTexture", 1, 5, Some(69), &[]));
    b.external_object(50, module, 10, 100);
    b.external_object(51, mdl, 1, 101);
    b.external_object(52, missing, 1, 101);
    // The sky classes (`environment-schema.md`).
    b.external_type(102, "CStaticTexture");
    b.type_block(TypeBlock::alias(40, "COLOR", 0));
    b.type_block(TypeBlock::alias(41, "BOOL", 0));
    b.type_block(TypeBlock::pointer(42, "Ptr<CTexture>", 102));
    let faces: Vec<(&str, u32)> = ["10", "11", "12", "13", "14", "15"]
        .into_iter()
        .map(|m| (m, 42))
        .collect();
    let cube_members: Vec<(&str, u32)> = [("1", 1), ("2", 1), ("3", 40)]
        .into_iter()
        .chain(faces)
        .collect();
    b.type_block(TypeBlock::class(
        43,
        "CCubeRenderable",
        3,
        5,
        Some(14),
        &cube_members,
    ));
    b.type_block(TypeBlock::pointer(44, "CCubeRenderable*", 43));
    b.type_block(TypeBlock::class(
        45,
        "CSimpleBackgroundViewerRenderable",
        1,
        5,
        Some(14),
        &[("1", 1)],
    ));
    b.type_block(TypeBlock::pointer(
        46,
        "CSimpleBackgroundViewerRenderable*",
        45,
    ));
    b.type_block(TypeBlock::class(
        47,
        "CCubeBackgroundEntity",
        5,
        5,
        Some(7),
        &[
            ("1", 44),
            ("3", 46),
            ("en_qvPlacement", 6),
            ("en_fSize", 1),
            ("en_fViewerY", 1),
            ("en_bActive", 41),
        ],
    ));
    for (id, path) in [
        (70, SKY_NORTH_TEX),
        (71, BODY_TEX),
        (72, SKY_MISSING_TEX),
        (74, SKY_UP_TEX),
    ] {
        let file = b.file_ref(path, 0);
        b.external_object(id, file, 1, 102);
    }

    let mut entities = vec![3, 4, 7, 8, 6];
    if with_spawn {
        entities.insert(4, 5);
    }
    if with_sky {
        entities.push(9);
    }
    b.object(
        1,
        13,
        ObjectBytes::new()
            .dynamic_container(&entities)
            .dynamic_container(&[60])
            .f32(0.0)
            .f32(-30.0)
            .f32(0.0)
            .f32(0.5)
            .finish(),
    );
    let entity = |serial: u32| {
        ObjectBytes::new()
            .u32(serial)
            .object_ref(Some(50))
            .u32(serial)
    };
    let static_model = |b: &mut MetaBuilder,
                        id: u32,
                        serial: u32,
                        ids: (u32, Option<u32>, u32), // renderable, mechanism, instance
                        model: u32,
                        world: ([f32; 4], [f32; 3]),
                        scale: [f32; 3]| {
        let (renderable, mechanism, instance) = ids;
        b.object(
            id,
            25,
            entity(serial)
                .object_ref(Some(renderable))
                .object_ref(mechanism)
                .object_ref(Some(model))
                .finish(),
        );
        b.object(
            renderable,
            19,
            aspect(None, world, id).object_ref(Some(instance)).finish(),
        );
        if let Some(m) = mechanism {
            b.object(m, 23, Vec::new());
        }
        let list = (lit.is_some() && instance == 13).then_some(100);
        b.object(
            instance,
            16,
            ObjectBytes::new()
                .f32(scale[0])
                .f32(scale[1])
                .f32(scale[2])
                .object_ref(Some(model))
                .object_ref(list)
                .finish(),
        );
    };
    if let Some((code, blob)) = lit {
        b.object(
            100,
            56,
            ObjectBytes::new().dynamic_container(&[101]).finish(),
        );
        b.object(
            101,
            60,
            ObjectBytes::new()
                .object_ref(Some(110))
                .u32(0)
                .u32(0)
                .u32(0)
                .finish(),
        );
        b.object(
            110,
            71,
            ObjectBytes::new()
                .i32(4)
                .i32(4)
                .u32(0x71)
                .u32(code)
                .array_header(b"STAR", blob.len() as u32)
                .raw(blob)
                .u32(0x51)
                .u32(0x51)
                .u32(0)
                .finish(),
        );
    }
    static_model(
        &mut b,
        3,
        1,
        (10, Some(11), 13),
        51,
        (ROT_Y90, [6.0, 0.0, 0.0]),
        [2.0, 2.0, -1.0],
    );
    static_model(
        &mut b,
        4,
        2,
        (14, None, 16),
        15,
        (IDENTITY, [0.0, 2.0, 0.0]),
        [1.0; 3],
    );
    b.object(15, 33, Vec::new());
    static_model(
        &mut b,
        7,
        5,
        (20, None, 21),
        51,
        (IDENTITY, [0.0, 0.0, -5.0]),
        [1.0; 3],
    );
    static_model(
        &mut b,
        8,
        6,
        (22, Some(23), 24),
        52,
        (IDENTITY, [0.0; 3]),
        [1.0; 3],
    );
    // The markers, each placed by one editor renderable.
    if with_spawn {
        b.object(
            5,
            27,
            entity(3).object_ref(Some(17)).object_ref(None).finish(),
        );
        b.object(
            17,
            20,
            aspect(None, (ROT_Y90, [10.0, 0.0, 10.0]), 5).finish(),
        );
    }
    b.object(6, 28, entity(4).finish());
    b.object(
        19,
        20,
        aspect(None, (IDENTITY, [1.0, 2.0, 3.0]), 6).finish(),
    );
    if with_sky {
        b.object(
            9,
            47,
            quat_vect(
                entity(7).object_ref(Some(40)).object_ref(None),
                IDENTITY,
                [0.0; 3],
            )
            .f32(64.0)
            .f32(0.0)
            .u32(1)
            .finish(),
        );
        let mut cube = aspect(None, (IDENTITY, [0.0; 3]), 9)
            .f32(64.0)
            .f32(1.0)
            .u32(SKY_COLOR);
        for face in [Some(70), Some(71), Some(72), None, None, Some(74)] {
            cube = cube.object_ref(face);
        }
        b.object(40, 43, cube.finish());
    }

    // One property record, in the extra section.
    let r = ObjectBytes::new().u32(60).u32(29).object_ref(Some(3));
    let record = quat_vect(r, IDENTITY, [0.0; 3])
        .cstring("Column")
        .object_ref(None)
        .finish();
    b.edty_pair(60, 29);
    b.after_edob(1, &record);
    b.build().unwrap()
}

fn element_path(code: u32, buffer: u32, offset: u32) -> Vec<u8> {
    ObjectBytes::new()
        .u32(code)
        .u32(buffer)
        .u32(offset)
        .finish()
}

fn absent() -> Vec<u8> {
    element_path(0, 255, 0xFFFF_FFFF)
}

fn f32s(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A model (`mdl.md`) with one embedded mesh (`bmf.md`): one LOD for every platform, one quad
/// surface (y = 0, from -1 to 1 in x and z) whose material is the external `Box.mtr`.
fn model_file() -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let mtr = b.file_ref(BOX_MTR, 0);
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
    let box_members = [("1", 3), ("2", 3), ("3", 3), ("4", 3), ("5", 3), ("6", 3)];
    b.type_block(TypeBlock::class(13, "Box3f", 1, 5, None, &box_members));
    let path_members = [("1", 2), ("2", 2), ("3", 2)];
    b.type_block(TypeBlock::class(
        14,
        "CRenderMeshElementPath",
        1,
        5,
        None,
        &path_members,
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
    let lod_members = [("3", 13), ("5", 22), ("6", 3), ("7", 2)];
    b.type_block(TypeBlock::class(
        23,
        "CRenderMeshLOD",
        1,
        5,
        None,
        &lod_members,
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
    let render_members = [
        ("5", 24),
        ("12", 25),
        ("13", 25),
        ("7", 3),
        ("8", 3),
        ("9", 3),
    ];
    b.type_block(TypeBlock::class(
        26,
        "CRenderMesh",
        1,
        5,
        None,
        &render_members,
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
    let config_members = [("1", 19), ("2", 19), ("3", 2)];
    b.type_block(TypeBlock::class(
        29,
        "CShaderConfig",
        1,
        5,
        None,
        &config_members,
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
    let config = [("3", 34), ("1", 13), ("6", 3), ("7", 3), ("12", 3)];
    b.type_block(TypeBlock::class(
        35,
        "CModelConfiguration",
        12,
        5,
        Some(9),
        &config,
    ));
    b.external_object(8, mtr, 1, 31);

    b.object(0, 7, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        35,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .array_header(b"STAR", 1)
            .object_ref(Some(2))
            .f32(-1.0)
            .f32(0.0)
            .f32(-1.0)
            .f32(1.0)
            .f32(0.0)
            .f32(1.0)
            .f32(990.0)
            .f32(1000.0)
            .f32(150.0)
            .finish(),
    );
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
    // Indices at 0 of the index buffer; positions at 0, normals at 48 and UVs (map id 3) at 96 of
    // the vertex buffer.
    let mut surface = ObjectBytes::new()
        .u32(2)
        .u32(4)
        .u32(1)
        .raw(&element_path(135, 0, 0))
        .raw(&element_path(133, 0, 0))
        .raw(&element_path(133, 0, 48))
        .raw(&absent())
        .raw(&absent())
        .raw(&absent());
    for slot in 0..8u32 {
        surface = surface.u32(if slot == 0 { 3 } else { 0 });
    }
    surface = surface.raw(&element_path(132, 0, 96));
    for _ in 1..8 {
        surface = surface.raw(&absent());
    }
    surface = surface.object_ref(Some(8)).array_header(b"STAR", 0);
    let lod = ObjectBytes::new()
        .f32(-1.0)
        .f32(0.0)
        .f32(-1.0)
        .f32(1.0)
        .f32(0.0)
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
    let indices: Vec<u8> = [0u16, 2, 1, 1, 2, 3]
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
    b.build().unwrap()
}

/// A material file (`mtr.md`) with one preset: one specular config (PC and Xbox) with blend
/// type `blend` whose base colour slot (`.21`) is `body.tex`.
fn material_file(blend: u32) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let module = b.file_ref("Bin/Shaders.module", 0);
    let tex = b.file_ref(BODY_TEX, 0);
    b.external_type(100, "CShader");
    b.external_type(101, "CStaticTexture");
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
    b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
    b.type_block(TypeBlock::enumeration(4, "EPlatform", 4));
    b.type_block(TypeBlock::enumeration(5, "GfuBlendType", 4));
    b.type_block(TypeBlock::class(
        6,
        "CResourceFile",
        3,
        5,
        None,
        &[("1", 1)],
    ));
    b.type_block(TypeBlock::pointer(7, "CResourceFile*", 6));
    b.type_block(TypeBlock::class(
        8,
        "CResource",
        4,
        5,
        None,
        &[("1", 1), ("2", 7)],
    ));
    b.type_block(TypeBlock::pointer(9, "CShader*", 100));
    b.type_block(TypeBlock::pointer(10, "CStaticTexture*", 101));
    b.type_block(TypeBlock::class(
        11,
        "CShaderArgValCOLOR",
        1,
        5,
        None,
        &[("1", 2)],
    ));
    b.type_block(TypeBlock::class(
        13,
        "CShaderArgValTexture",
        1,
        5,
        None,
        &[("1", 10)],
    ));
    b.type_block(TypeBlock::class(
        14,
        "CShaderArgValUVMap",
        1,
        5,
        None,
        &[("1", 2)],
    ));
    b.type_block(TypeBlock::class(
        15,
        "CShaderArgValBlendType",
        1,
        5,
        None,
        &[("1", 5)],
    ));
    b.type_block(TypeBlock::class(
        16,
        "CShaderModifier",
        1,
        5,
        None,
        &[("1", 4)],
    ));
    b.type_block(TypeBlock::short(
        19,
        "CDynamicContainer<CShaderModifier>",
        0,
        8,
        16,
    ));
    b.type_block(TypeBlock::class(
        20,
        "CShaderModifierList",
        1,
        5,
        None,
        &[("1", 19)],
    ));
    b.type_block(TypeBlock::class(
        21,
        "CShaderArgs",
        1,
        5,
        None,
        &[("1", 20)],
    ));
    b.type_block(TypeBlock::class(
        22,
        "CShaderCommonParams",
        1,
        5,
        Some(21),
        &[("1", 15)],
    ));
    let args = [("9", 11), ("21", 13), ("22", 14)];
    b.type_block(TypeBlock::class(
        23,
        "CSpecularShaderArgs",
        1,
        5,
        Some(22),
        &args,
    ));
    b.type_block(TypeBlock::pointer(24, "CSpecularShaderArgs*", 23));
    let config_members = [("1", 9), ("2", 24), ("3", 4)];
    b.type_block(TypeBlock::class(
        25,
        "CShaderConfig",
        1,
        5,
        None,
        &config_members,
    ));
    b.type_block(TypeBlock::short(
        26,
        "CStaticArray<CShaderConfig>",
        0,
        6,
        25,
    ));
    b.type_block(TypeBlock::pointer(27, "CResource*", 8));
    let preset_members = [("1", 26), ("2", 27), ("3", 0)];
    b.type_block(TypeBlock::class(
        28,
        "CShaderPreset",
        1,
        5,
        Some(8),
        &preset_members,
    ));
    b.external_object(2, module, 1010, 100);
    b.external_object(4, tex, 1, 101);

    b.object(0, 6, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        28,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .array_header(b"STAR", 1)
            .object_ref(Some(2))
            .object_ref(Some(3))
            .u32(0x3)
            .object_ref(None)
            .i32(255)
            .finish(),
    );
    b.object(
        3,
        23,
        ObjectBytes::new()
            .raw(&ObjectBytes::new().dynamic_container(&[]).finish())
            .u32(blend)
            .u32(0xFF7F_7F7F)
            .object_ref(Some(4))
            .u32(3)
            .finish(),
    );
    b.build().unwrap()
}

/// A version-8 texture file (`tex.md`) with `pixels` compressed into its blob.
fn texture_file(width: i32, height: i32, format_code: u32, pixels: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(pixels).unwrap();
    let blob = z.finish().unwrap();
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
            .u32(0)
            .u32(format_code)
            .array_header(b"STAR", blob.len() as u32)
            .raw(&blob)
            .u32(WRAP_REPEAT)
            .u32(WRAP_CLAMP)
            .u32(0)
            .finish(),
    );
    b.build().unwrap()
}

/// An 8 x 8 DXT1 texture with its four levels (32 + 8 + 8 + 8 bytes).
fn dxt1_8x8() -> Vec<u8> {
    texture_file(8, 8, 7, &[0x11; 56])
}

/// A [`MemoryVfs`] that counts the reads of each path.
struct CountingVfs {
    inner: MemoryVfs,
    reads: Mutex<BTreeMap<String, usize>>,
}

impl CountingVfs {
    fn new(files: &[(&str, Vec<u8>)]) -> Self {
        let mut inner = MemoryVfs::new();
        for (path, bytes) in files {
            inner.insert(path, bytes.clone()).unwrap();
        }
        Self {
            inner,
            reads: Mutex::new(BTreeMap::new()),
        }
    }

    fn reads(&self, path: &str) -> usize {
        let key = VfsPath::new(path).unwrap().as_str().to_string();
        self.reads.lock().unwrap().get(&key).copied().unwrap_or(0)
    }

    fn count(&self, path: &VfsPath) {
        *self
            .reads
            .lock()
            .unwrap()
            .entry(path.as_str().to_string())
            .or_default() += 1;
    }
}

impl Vfs for CountingVfs {
    fn stat(&self, path: &VfsPath) -> Result<Stat> {
        self.inner.stat(path)
    }

    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>> {
        self.count(path);
        self.inner.open(path)
    }

    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        self.inner.list(dir)
    }

    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        self.count(path);
        self.inner.read(path)
    }
}

const OPTIONS: LoadOptions = LoadOptions {
    max_texture_size: 2048,
};

fn all_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (WLD, world_file(true)),
        (BOX_MDL, model_file()),
        (BOX_MTR, material_file(0x1F4)),
        (BODY_TEX, dxt1_8x8()),
    ]
}

/// A 4 x 2 BGRA texture (`TexFormat` 1): blue, a red texel in its top-left corner.
fn marked_tex() -> Vec<u8> {
    let mut pixels = Vec::new();
    for i in 0..8 {
        // Stored B, G, R, A.
        pixels.extend(if i == 0 {
            [0, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        });
    }
    texture_file(4, 2, 1, &pixels)
}

fn sky_files() -> Vec<(&'static str, Vec<u8>)> {
    let mut files = without(WLD);
    files.push((WLD, world_file_with(true, true)));
    files.push((SKY_NORTH_TEX, marked_tex()));
    files.push((SKY_UP_TEX, dxt1_8x8()));
    files
}

fn load(vfs: &dyn Vfs) -> (LevelScene, Vec<LoadProgress>) {
    let mut events = Vec::new();
    let scene = load_level(vfs, &VfsPath::new(WLD).unwrap(), &OPTIONS, &mut |p| {
        events.push(p)
    })
    .unwrap();
    scene.validate().unwrap();
    (scene, events)
}

fn without(name: &str) -> Vec<(&'static str, Vec<u8>)> {
    all_files()
        .into_iter()
        .filter(|(p, _)| *p != name)
        .collect()
}

fn skipped_mentions(scene: &LevelScene, what: &str) -> usize {
    let what = what.to_ascii_lowercase();
    scene
        .skipped
        .iter()
        .filter(|l| l.to_ascii_lowercase().contains(&what))
        .count()
}

#[test]
fn a_world_places_one_model_twice_once_mirrored() {
    let vfs = CountingVfs::new(&all_files());
    let (scene, _) = load(&vfs);
    assert_eq!(scene.name, "box");
    assert_eq!(scene.gravity, Vec3::new(0.0, -30.0, 0.0));
    assert_eq!(scene.models.len(), 1, "{:?}", scene.skipped);
    assert_eq!(scene.instances.len(), 2);
    let [a, b] = [scene.instances[0], scene.instances[1]];
    assert_eq!((a.model, b.model), (0, 0));
    assert_eq!(a.transform.scale, Vec3::new(2.0, 2.0, -1.0));
    assert_eq!(a.transform.translation, Vec3::new(6.0, 0.0, 0.0));
    assert!(
        a.transform
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(FRAC_PI_2), 1e-6)
    );
    assert!(a.collidable && !b.collidable);
    assert_eq!(b.transform.translation, Vec3::new(0.0, 0.0, -5.0));
    assert_eq!(b.transform.scale, Vec3::ONE);

    let model = &scene.models[0];
    assert_eq!(model.parts.len(), 1);
    let mesh = &model.parts[0].mesh;
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices, [0, 2, 1, 1, 2, 3]);
    assert_eq!(mesh.vertices[3].position, [1.0, 0.0, 1.0]);
    assert_eq!(mesh.vertices[3].normal, [0.0, 1.0, 0.0]);
    assert_eq!(mesh.vertices[3].uv, [1.0, 1.0]);
    let collision = model.collision.as_ref().unwrap();
    assert_eq!(collision.positions.len(), 4);
    assert_eq!(collision.indices, [0, 2, 1, 1, 2, 3]);

    // The material keeps its texture, an 8 x 8 BC1 with its whole chain.
    assert_eq!(scene.materials.len(), 1);
    let material = scene.materials[model.parts[0].material as usize];
    assert_eq!(material.base_texture, Some(0));
    assert_eq!(material.alpha, AlphaMode::Opaque);
    assert_eq!(scene.textures.len(), 1);
    let t = &scene.textures[0];
    assert_eq!(
        (t.width, t.height, t.format, t.mips.len()),
        (8, 8, TexelFormat::Bc1, 4)
    );
    assert!(t.repeat_u && !t.repeat_v);

    // Each file is read once although the model is placed twice.
    for path in [WLD, BOX_MDL, BOX_MTR, BODY_TEX] {
        assert_eq!(vfs.reads(path), 1, "{path}");
    }
}

/// The lightmap of the lit placement: 4 x 4 BGR, each texel's blue byte its index.
fn lightmap_pixels() -> Vec<u8> {
    (0..16u8).flat_map(|i| [i, 100, 200]).collect()
}

fn zlib(bytes: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(bytes).unwrap();
    z.finish().unwrap()
}

fn lit_files(blob: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    let mut files = without(WLD);
    files.push((WLD, world_file_lit(0, blob)));
    files
}

#[test]
fn a_lit_placement_gets_its_lightmap_and_the_model_its_lightmap_uvs() {
    let vfs = CountingVfs::new(&lit_files(&zlib(&lightmap_pixels())));
    let (scene, report) =
        load_level_with_report(&vfs, &VfsPath::new(WLD).unwrap(), &OPTIONS, &mut |_| {}).unwrap();
    scene.validate().unwrap();
    assert!(
        scene.skipped.iter().all(|l| !l.contains("lightmap")),
        "{:?}",
        scene.skipped
    );
    // Instance 0 (entity 3) is lit; instance 1 (entity 7) places the same model unlit.
    let [a, b] = [scene.instances[0], scene.instances[1]];
    let lightmap = a.lightmap.expect("the lit placement has a lightmap") as usize;
    assert_eq!(b.lightmap, None);
    // After the material's texture; BGR became RGBA8, clamped as the wrap code 0x51 says.
    assert_eq!(lightmap, 1);
    let t = &scene.textures[lightmap];
    assert_eq!((t.width, t.height, t.format), (4, 4, TexelFormat::Rgba8));
    assert_eq!(&t.mips[0][..8], &[200, 100, 0, 255, 200, 100, 1, 255]);
    assert!(!t.repeat_u && !t.repeat_v);
    // The quad's one UV stream lies in [0, 1] without overlap: it is the lightmap's.
    let mesh = &scene.models[0].parts[0].mesh;
    let uvs = mesh.lightmap_uvs.as_ref().expect("lightmap UVs");
    let base: Vec<[f32; 2]> = mesh.vertices.iter().map(|v| v.uv).collect();
    assert_eq!(uvs, &base);
    assert_eq!(
        report,
        LightmapReport {
            lit_placements: 1,
            lightmapped: 1,
            lit_models: 1,
            one_candidate: BTreeMap::from([(0, 1)]),
            ..LightmapReport::default()
        }
    );
}

#[test]
fn a_lightmap_that_cannot_be_read_leaves_the_placement_unlit_with_a_line() {
    let vfs = CountingVfs::new(&lit_files(b"not zlib"));
    let (scene, report) =
        load_level_with_report(&vfs, &VfsPath::new(WLD).unwrap(), &OPTIONS, &mut |_| {}).unwrap();
    scene.validate().unwrap();
    assert_eq!(scene.instances[0].lightmap, None);
    assert_eq!(skipped_mentions(&scene, "lightmap (oven modifier 101)"), 1);
    assert_eq!((report.lightmapped, report.unlit_texture), (0, 1));
    // The model still has its lightmap UVs (another placement could use them).
    assert!(scene.models[0].parts[0].mesh.lightmap_uvs.is_some());
}

#[test]
fn an_unlit_world_has_no_lightmap_uvs() {
    let vfs = CountingVfs::new(&all_files());
    let (scene, _) = load(&vfs);
    assert!(scene.instances.iter().all(|i| i.lightmap.is_none()));
    assert!(
        scene
            .models
            .iter()
            .flat_map(|m| &m.parts)
            .all(|p| p.mesh.lightmap_uvs.is_none())
    );
}

#[test]
fn append_surface_keeps_lightmap_uvs_in_step_with_the_vertices() {
    use ss2_format_bmf::MaterialRef;
    let surface = |n: usize, streams: usize| Surface {
        indices: vec![0, 1, 2],
        positions: vec![[0.0; 3]; n],
        normals: vec![[0.0, 1.0, 0.0]; n],
        tangents: None,
        uv_sets: (0..streams)
            .map(|k| (0..n).map(|i| [k as f32, i as f32 / 10.0]).collect())
            .collect(),
        uv_map_ids: (0..streams as u32).collect(),
        bone_weights: None,
        bone_indices: None,
        bone_palette: Vec::new(),
        material: MaterialRef::None,
    };
    let mut mesh = MeshData::default();
    append_surface(&mut mesh, &surface(3, 2), None, Some(1));
    // A surface without that stream gets zeros.
    append_surface(&mut mesh, &surface(4, 1), None, Some(1));
    let uvs = mesh.lightmap_uvs.as_ref().unwrap();
    assert_eq!(uvs.len(), mesh.vertices.len());
    assert_eq!(&uvs[..3], &[[1.0, 0.0], [1.0, 0.1], [1.0, 0.2]]);
    assert_eq!(&uvs[3..], &[[0.0, 0.0]; 4]);
    mesh.validate().unwrap();
}

#[test]
fn the_missing_and_the_embedded_model_are_skipped_with_a_line_each() {
    let (scene, _) = load(&CountingVfs::new(&all_files()));
    assert_eq!(
        skipped_mentions(&scene, MISSING_MDL),
        1,
        "{:?}",
        scene.skipped
    );
    assert_eq!(
        skipped_mentions(&scene, "embedded"),
        1,
        "{:?}",
        scene.skipped
    );
    assert_eq!(scene.skipped.len(), 2, "{:?}", scene.skipped);
}

#[test]
fn a_missing_texture_becomes_white_and_keeps_the_material() {
    let (scene, _) = load(&CountingVfs::new(&without(BODY_TEX)));
    assert_eq!(skipped_mentions(&scene, BODY_TEX), 1, "{:?}", scene.skipped);
    assert_eq!(scene.materials.len(), 1);
    let t = scene.materials[0].base_texture.unwrap();
    assert_eq!(scene.textures[t as usize], TextureData::solid([255; 4]));
    assert_eq!(scene.instances.len(), 2);
}

#[test]
fn a_missing_material_becomes_the_grey_fallback() {
    let (scene, _) = load(&CountingVfs::new(&without(BOX_MTR)));
    assert_eq!(skipped_mentions(&scene, BOX_MTR), 1, "{:?}", scene.skipped);
    assert_eq!(scene.materials, [MaterialData::FALLBACK]);
    assert!(scene.textures.is_empty());
    assert_eq!(scene.instances.len(), 2);
}

#[test]
fn a_broken_model_drops_its_instances() {
    let mut files = without(BOX_MDL);
    files.push((BOX_MDL, b"not a model".to_vec()));
    let (scene, _) = load(&CountingVfs::new(&files));
    assert!(scene.instances.is_empty());
    assert!(scene.models.is_empty());
    assert_eq!(skipped_mentions(&scene, BOX_MDL), 1, "{:?}", scene.skipped);
}

#[test]
fn a_blended_material_blends() {
    let mut files = without(BOX_MTR);
    files.push((BOX_MTR, material_file(0x1F5)));
    let (scene, _) = load(&CountingVfs::new(&files));
    assert_eq!(scene.materials[0].alpha, AlphaMode::Blend);
}

#[test]
fn only_the_world_itself_can_fail_the_load() {
    let vfs = CountingVfs::new(&without(WLD));
    let wld = VfsPath::new(WLD).unwrap();
    assert!(load_level(&vfs, &wld, &OPTIONS, &mut |_| {}).is_err());
    let mut files = without(WLD);
    files.push((WLD, b"garbage".to_vec()));
    let vfs = CountingVfs::new(&files);
    assert!(load_level(&vfs, &wld, &OPTIONS, &mut |_| {}).is_err());
}

#[test]
fn spawn_markers_come_first_and_chapters_are_the_fallback() {
    let (scene, _) = load(&CountingVfs::new(&all_files()));
    assert_eq!(scene.spawns.len(), 1);
    assert_eq!(scene.spawns[0].position, Vec3::new(10.0, 0.0, 10.0));
    assert!((scene.spawns[0].yaw - FRAC_PI_2).abs() < 1e-5);

    let mut files = without(WLD);
    files.push((WLD, world_file(false)));
    let (scene, _) = load(&CountingVfs::new(&files));
    assert_eq!(scene.spawns.len(), 1);
    assert_eq!(scene.spawns[0].position, Vec3::new(1.0, 2.0, 3.0));
    assert!(scene.spawns[0].yaw.abs() < 1e-6);
}

#[test]
fn progress_goes_through_world_models_and_textures() {
    let (_, events) = load(&CountingVfs::new(&all_files()));
    let last = |stage: LoadStage| events.iter().rev().find(|e| e.stage == stage).copied();
    assert_eq!(events.first().map(|e| e.stage), Some(LoadStage::World));
    // Two distinct external models (Box and Missing), one texture.
    let models = last(LoadStage::Models).unwrap();
    assert_eq!((models.done, models.total), (2, 2));
    let textures = last(LoadStage::Textures).unwrap();
    assert_eq!((textures.done, textures.total), (1, 1));
    let order: Vec<LoadStage> = events.iter().map(|e| e.stage).collect();
    let first = |s: LoadStage| order.iter().position(|x| *x == s).unwrap();
    assert!(first(LoadStage::World) < first(LoadStage::Models));
    assert!(first(LoadStage::Models) < first(LoadStage::Textures));
}

#[test]
fn yaw_is_the_heading_of_minus_z() {
    assert!(yaw_of(Quat::IDENTITY).abs() < 1e-6);
    for deg in [-170.0f32, -90.0, 30.0, 135.0] {
        let r = deg.to_radians();
        assert!((yaw_of(Quat::from_rotation_y(r)) - r).abs() < 1e-5, "{deg}");
    }
    // A pitched or rolled marker keeps its heading.
    let q = Quat::from_rotation_y(0.5) * Quat::from_rotation_x(0.3);
    assert!((yaw_of(q) - 0.5).abs() < 1e-5);
}

fn tex(format: PixelFormat, width: u32, height: u32, data: Vec<u8>) -> Texture {
    let mip_levels = if format.is_compressed() {
        u32::BITS - width.max(height).leading_zeros()
    } else {
        1
    };
    Texture {
        width,
        height,
        format,
        format_code: 0,
        flags: 0,
        wrap_u: WRAP_REPEAT,
        wrap_v: WRAP_REPEAT,
        border_color: 0,
        faces: 1,
        mip_levels,
        data,
    }
}

fn dxt_bytes(width: u32, height: u32, block: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut w, mut h, mut level) = (width, height, 0u8);
    loop {
        let blocks = (w.div_ceil(4) * h.div_ceil(4)) as usize;
        out.extend(std::iter::repeat_n(level, blocks * block));
        if w == 1 && h == 1 {
            break;
        }
        (w, h, level) = ((w / 2).max(1), (h / 2).max(1), level + 1);
    }
    out
}

#[test]
fn dxt_textures_keep_their_mips_and_drop_the_top_ones_above_the_limit() {
    let t = tex(PixelFormat::Dxt5, 64, 16, dxt_bytes(64, 16, 16));
    let full = texture_data(&t, 2048).unwrap();
    assert_eq!(
        (full.width, full.height, full.format),
        (64, 16, TexelFormat::Bc3)
    );
    assert_eq!(full.mips.len(), 7);
    full.validate().unwrap();
    let small = texture_data(&t, 16).unwrap();
    assert_eq!((small.width, small.height, small.mips.len()), (16, 4, 5));
    assert_eq!(
        small.mips[0][0], 2,
        "level 2 of the file is the new level 0"
    );
    small.validate().unwrap();
    let dxt3 = tex(PixelFormat::Dxt3, 4, 4, dxt_bytes(4, 4, 16));
    assert_eq!(texture_data(&dxt3, 2048).unwrap().format, TexelFormat::Bc2);
}

#[test]
fn a_block_texture_that_is_not_a_multiple_of_four_or_a_cube_map_is_refused() {
    let t = tex(PixelFormat::Dxt1, 2, 2, dxt_bytes(2, 2, 8));
    assert!(
        texture_data(&t, 2048)
            .unwrap_err()
            .contains("multiple of 4")
    );
    // Dropping mips down to 2 x 2 is refused the same way.
    let t = tex(PixelFormat::Dxt1, 8, 8, dxt_bytes(8, 8, 8));
    assert!(texture_data(&t, 2).is_err());
    let mut cube = tex(PixelFormat::Dxt1, 4, 4, dxt_bytes(4, 4, 8).repeat(6));
    cube.faces = 6;
    assert!(texture_data(&cube, 2048).unwrap_err().contains("cube"));
}

#[test]
fn uncompressed_textures_are_swizzled_to_rgba_and_halved_above_the_limit() {
    let bgr = tex(PixelFormat::Bgr8, 2, 1, vec![1, 2, 3, 4, 5, 6]);
    let t = texture_data(&bgr, 2048).unwrap();
    assert_eq!(t.format, TexelFormat::Rgba8);
    assert_eq!(t.mips, [vec![3, 2, 1, 255, 6, 5, 4, 255]]);

    let bgra = tex(PixelFormat::Bgra8, 1, 1, vec![10, 20, 30, 40]);
    assert_eq!(
        texture_data(&bgra, 2048).unwrap().mips,
        [vec![30, 20, 10, 40]]
    );

    // 4 x 2 with a limit of 2: averaged down to 2 x 1.
    let px = |v: u8| [v, v, v, 255];
    let data: Vec<u8> = [0, 100, 200, 40, 0, 100, 200, 40]
        .into_iter()
        .flat_map(px)
        .collect();
    let t = texture_data(&tex(PixelFormat::Bgra8, 4, 2, data), 2).unwrap();
    assert_eq!((t.width, t.height), (2, 1));
    assert_eq!(t.mips, [vec![50, 50, 50, 255, 120, 120, 120, 255]]);
    t.validate().unwrap();
}

#[test]
fn a_world_without_a_sky_loads_none() {
    let vfs = CountingVfs::new(&all_files());
    assert_eq!(load(&vfs).0.sky, None);
}

#[test]
fn the_sky_faces_go_to_their_cube_faces_through_the_texture_path() {
    let vfs = CountingVfs::new(&sky_files());
    let (scene, _) = load(&vfs);
    let sky = scene.sky.expect("a sky");
    let face = |f: CubeFace| sky.faces[f.index()].map(|i| &scene.textures[i as usize]);
    // North is -Z: the marked 4 x 2 image, converted to RGBA like any other texture.
    let north = face(CubeFace::NegZ).expect("north");
    assert_eq!(
        (north.width, north.height, north.format),
        (4, 2, TexelFormat::Rgba8)
    );
    assert_eq!(&north.mips[0][..4], &[255, 0, 0, 255]);
    assert_eq!(&north.mips[0][4..8], &[0, 0, 255, 255]);
    // West is -X and shares the material's texture: one texture, read once.
    let material_texture = scene.materials.iter().find_map(|m| m.base_texture);
    assert_eq!(sky.faces[CubeFace::NegX.index()], material_texture);
    assert_eq!(vfs.reads(BODY_TEX), 1);
    // South (+Z) is missing: a white stand-in and one line.
    assert_eq!(face(CubeFace::PosZ), Some(&TextureData::solid([255; 4])));
    assert_eq!(skipped_mentions(&scene, "missing.tex"), 1);
    // No East (+X) and no Floor (-Y) face: those show the tint alone.
    assert_eq!(face(CubeFace::PosX), None);
    assert_eq!(face(CubeFace::NegY), None);
    // The ceiling (+Y) keeps its block format.
    assert_eq!(
        face(CubeFace::PosY).map(|t| t.format),
        Some(TexelFormat::Bc1)
    );
    assert_eq!(sky.tint, sky::tint(SKY_COLOR));
}

#[test]
fn the_marked_corner_of_a_game_face_lands_where_the_convention_puts_it() {
    // The game's North image is upright seen from inside, so its top-left texel is the
    // north-west corner at the top: (-X, +Y, -Z). In the convention, texel (0, 0) of the face is
    // at forward - right - down.
    let vfs = CountingVfs::new(&sky_files());
    let (scene, _) = load(&vfs);
    let sky = scene.sky.unwrap();
    let b = CubeFace::NegZ.basis();
    assert_eq!(b.forward - b.right - b.down, Vec3::new(-1.0, 1.0, -1.0));
    let north = &scene.textures[sky.faces[CubeFace::NegZ.index()].unwrap() as usize];
    assert_eq!(
        &north.mips[0][..4],
        &[255, 0, 0, 255],
        "the mark stays at texel (0, 0)"
    );
}
