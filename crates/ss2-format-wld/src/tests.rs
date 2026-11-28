//! Unit tests on synthetic worlds written with the testkit's CTSEMETA builder (no game data).

use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

use ss2_assets::{AssetManager, LoadState};
use ss2_core::math::{Quat, Vec3};
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
use ss2_vfs::MemoryVfs;

use super::*;

/// What to change in the synthetic world.
#[derive(Clone, Copy)]
struct Variant {
    /// How many `CWorld` objects (normally 1).
    worlds: usize,
    /// The external file of the first model (normally `Content/Test/Box.mdl`).
    model_file: &'static str,
    /// An entity id of `CWorld.1` that is replaced by this (to make it dangle).
    entity_override: Option<u32>,
    /// Whether and how the world has a sky entity.
    sky: SkyKind,
    /// The baked lightmap of the first static model.
    lightmap: LightmapKind,
    /// The chapter chain.
    chain: ChainKind,
    /// Whether and how the world has a haze entity.
    haze: HazeKind,
}

/// The haze entity of the synthetic world (entity 24, renderable 25, record 64).
#[derive(Clone, Copy, PartialEq, Eq)]
enum HazeKind {
    /// No `CHazeEntity`.
    Absent,
    /// A haze attached to entity 3, with its property record.
    WithRecord,
    /// The same haze without a property record and without a parent.
    WithoutRecord,
}

/// The chapter chain of the synthetic world.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChainKind {
    /// No world info entity; chapter 6 has no next chapter.
    Absent,
    /// A second chapter (entity 8, after chapter 6 in the entity list) and a world info entity
    /// (9) whose first chapter is 8; 8's next chapter is 6, 6 has none.
    Linked,
    /// [`ChainKind::Linked`], but 6's next chapter is 8 again.
    Cycle,
    /// [`ChainKind::Linked`], but the world info's first chapter names object 999, not in the file.
    Dangling,
}

/// The oven modifiers of the first static model's instance (object 13). Besides it, a model
/// instance that no static model uses (object 104, as a door's or a prop's would be) always has
/// one oven modifier, so that the world-wide counts see more than the static models.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LightmapKind {
    /// Modifier list 100: an oven modifier (101, texture 110) and a constant modifier (102).
    One,
    /// List 100 with a second oven modifier (103, texture 111) after the constant one.
    Two,
    /// The instance has no modifier list (`.4` null).
    NoList,
    /// The list has only the constant modifier.
    NoOven,
    /// The oven modifier's texture argument names object 999, which is not in the file.
    DanglingTexture,
    /// The texture's pixel blob is not zlib.
    BadPixels,
    /// The modifier list names object 998, which is not in the file.
    DanglingList,
}

/// The sky entity of the synthetic world.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SkyKind {
    /// No `CCubeBackgroundEntity`.
    Absent,
    /// A five-face sky (no floor) with its property record.
    WithRecord,
    /// The same sky without a property record.
    WithoutRecord,
    /// A sky whose renderable names no face texture at all (and a record without a texture).
    NoFaces,
}

const NORMAL: Variant = Variant {
    worlds: 1,
    model_file: "Content/Test/Box.mdl",
    entity_override: None,
    sky: SkyKind::Absent,
    lightmap: LightmapKind::One,
    chain: ChainKind::Absent,
    haze: HazeKind::Absent,
};

/// The lightmap pixels: 4 x 4 BGR (format code 0), so a test can check each texel.
fn lightmap_pixels(seed: u8) -> Vec<u8> {
    (0..48u8)
        .map(|i| i.wrapping_mul(5).wrapping_add(seed))
        .collect()
}

fn zlib(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(bytes).unwrap();
    z.finish().unwrap()
}

const NORTH_TEX: &str = "Content/Test/Sky/Night_0000.tex";
const WEST_TEX: &str = "Content/Test/Sky/Night_0003.tex";
const SOUTH_TEX: &str = "Content/Test/Sky/Night_0002.tex";
const EAST_TEX: &str = "Content/Test/Sky/Night_0001.tex";
const CEILING_TEX: &str = "Content/Test/Sky/Night_0004.tex";

/// 90 degrees about +y, as stored (x, y, z, w).
const ROT_Y90: [f32; 4] = [0.0, FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2];
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn quat_vect(b: ObjectBytes, rotation: [f32; 4], translation: [f32; 3]) -> ObjectBytes {
    let mut b = b;
    for v in rotation {
        b = b.f32(v);
    }
    for v in translation {
        b = b.f32(v);
    }
    b
}

/// The bytes of a `CAspect`: parent, local placement, world placement, owner.
fn aspect(
    parent: Option<u32>,
    local: ([f32; 4], [f32; 3]),
    world: ([f32; 4], [f32; 3]),
    owner: Option<u32>,
) -> ObjectBytes {
    let b = ObjectBytes::new().object_ref(parent);
    let b = quat_vect(b, local.0, local.1);
    let b = quat_vect(b, world.0, world.1);
    b.object_ref(owner)
}

/// A small world in the shape of `wld-root.md` / `wld-geometry.md`:
///
/// - entity 3: a static model (serial 1, record "Column") with a mechanism; its hull (12) is the
///   top of the aspect subtree, the renderable (10) its child with a different local placement;
///   the instance scale is (2, 2, -1) and the model the external `.mdl`;
/// - entity 4: a static model (serial 2, no record) without a mechanism, embedded model 15;
/// - entity 5: a spawn marker (serial 3, record "Start01") with two editor renderables;
/// - entity 6: a chapter info entity (serial 4, record "Chapter001") with one editor renderable.
fn world_file(v: Variant) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let module = b.file_ref("Bin/Sam2Game.module", 0);
    let mdl = b.file_ref(v.model_file, 0);
    b.external_type(100, "CEntityClass");
    b.external_type(101, "CModelConfiguration");
    b.type_block(TypeBlock::primitive(0, "ULONG", 4, 4));
    b.type_block(TypeBlock::primitive(1, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(2, "CString", 4, 4));
    b.type_block(TypeBlock::class(3, "CObject", 1, 5, None, &[]));
    b.type_block(TypeBlock::class(
        4,
        "Vector3f",
        1,
        5,
        None,
        &[("1", 1), ("2", 1), ("3", 1)],
    ));
    b.type_block(TypeBlock::class(
        5,
        "Quaternion4f",
        1,
        5,
        None,
        &[("1", 1), ("2", 1), ("3", 1), ("4", 1)],
    ));
    b.type_block(TypeBlock::class(
        6,
        "QuatVect",
        1,
        5,
        None,
        &[("1", 5), ("2", 4)],
    ));
    b.type_block(TypeBlock::pointer(31, "Ptr<CEntityClass>", 100));
    b.type_block(TypeBlock::class(
        7,
        "CEntity",
        7,
        5,
        Some(3),
        &[("1", 0), ("2", 31), ("4", 0)],
    ));
    b.type_block(TypeBlock::pointer(8, "CEntity*", 7));
    b.type_block(TypeBlock::dynamic_container(
        9,
        "CDynamicContainer<CEntity>",
        7,
    ));
    b.type_block(TypeBlock::pointer(11, "CEntityProperties*", 10));
    b.type_block(TypeBlock::class(
        10,
        "CEntityProperties",
        4,
        5,
        Some(3),
        &[("2", 8), ("3", 6), ("4", 2), ("5", 11)],
    ));
    b.type_block(TypeBlock::dynamic_container(
        12,
        "CDynamicContainer<CEntityProperties>",
        10,
    ));
    b.type_block(TypeBlock::class(
        13,
        "CWorld",
        14,
        5,
        None,
        &[("1", 9), ("5", 12), ("10", 4), ("11", 1)],
    ));
    b.type_block(TypeBlock::pointer(15, "CAspect*", 14));
    b.type_block(TypeBlock::class(
        14,
        "CAspect",
        3,
        5,
        None,
        &[("1", 15), ("4", 6), ("7", 6), ("5", 8)],
    ));
    b.type_block(TypeBlock::pointer(17, "Ptr<CModelConfiguration>", 101));
    b.type_block(TypeBlock::class(
        16,
        "CModelInstance",
        5,
        5,
        None,
        &[("1", 4), ("3", 17), ("4", 54)],
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
    b.type_block(TypeBlock::class(
        25,
        "CStaticModelEntity",
        7,
        5,
        Some(7),
        &[("1", 22), ("10", 24), ("6", 17)],
    ));
    b.type_block(TypeBlock::pointer(
        26,
        "Handle<CEditorPrimitiveRenderable>",
        20,
    ));
    b.type_block(TypeBlock::class(
        27,
        "CSpawnMarkerEntity",
        7,
        5,
        Some(7),
        &[("1", 26), ("2", 26)],
    ));
    b.type_block(TypeBlock::class(
        28,
        "CChapterInfoEntity",
        7,
        5,
        Some(7),
        &[("2", 34)],
    ));
    b.type_block(TypeBlock::pointer(34, "Handle<CChapterInfoEntity>", 28));
    b.type_block(TypeBlock::class(
        35,
        "CWorldInfoEntity",
        7,
        5,
        Some(7),
        &[("2", 34)],
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
    // The sky classes (`environment-schema.md`).
    b.external_type(102, "CStaticTexture");
    b.type_block(TypeBlock::alias(40, "COLOR", 0));
    b.type_block(TypeBlock::alias(41, "BOOL", 0));
    b.type_block(TypeBlock::pointer(42, "Ptr<CTexture>", 102));
    b.type_block(TypeBlock::class(
        43,
        "CCubeRenderable",
        3,
        5,
        Some(14),
        &[
            ("1", 1),
            ("2", 1),
            ("3", 40),
            ("10", 42),
            ("11", 42),
            ("12", 42),
            ("13", 42),
            ("14", 42),
            ("15", 42),
        ],
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
    b.type_block(TypeBlock::class(
        48,
        "CCubeBackgroundProperties",
        8,
        5,
        Some(10),
        &[
            ("1", 42),
            ("6", 40),
            ("2", 1),
            ("3", 1),
            ("4", 1),
            ("5", 41),
            ("ep_bActive", 41),
        ],
    ));
    // The shader modifier and texture classes of a lightmap (`wld-lighting.md`, "Baked
    // lightmaps"; the texture as in `tex.md`).
    b.type_block(TypeBlock::primitive(49, "IDENT", 4, 4));
    b.type_block(TypeBlock::enumeration(50, "EPlatform", 4));
    b.type_block(TypeBlock::class(
        51,
        "CShaderModifier",
        2,
        5,
        Some(3),
        &[("1", 50)],
    ));
    b.type_block(TypeBlock::dynamic_container(
        52,
        "CDynamicContainer<CShaderModifier>",
        51,
    ));
    b.type_block(TypeBlock::class(
        53,
        "CShaderModifierList",
        2,
        5,
        None,
        &[("1", 52)],
    ));
    b.type_block(TypeBlock::pointer(54, "CShaderModifierList*", 53));
    b.type_block(TypeBlock::class(
        55,
        "CShaderArgumentValue",
        1,
        5,
        Some(3),
        &[],
    ));
    b.type_block(TypeBlock::pointer(72, "Ptr<CTexture>", 71));
    b.type_block(TypeBlock::class(
        56,
        "CShaderArgValTexture",
        1,
        5,
        Some(55),
        &[("1", 72)],
    ));
    b.type_block(TypeBlock::class(
        57,
        "CShaderArgValUVMap",
        1,
        5,
        Some(55),
        &[("1", 49)],
    ));
    b.type_block(TypeBlock::class(58, "CTextureBaker", 1, 5, Some(3), &[]));
    b.type_block(TypeBlock::pointer(59, "CTextureBaker*", 58));
    b.type_block(TypeBlock::class(
        60,
        "CTextureOvenShaderModifier",
        3,
        5,
        Some(51),
        &[("4", 56), ("1", 49), ("3", 57), ("2", 49), ("6", 59)],
    ));
    b.type_block(TypeBlock::class(
        61,
        "CConstantShaderModifier",
        1,
        5,
        Some(51),
        &[("1", 49), ("2", 49)],
    ));
    b.type_block(TypeBlock::primitive(62, "SLONG", 4, 4));
    b.type_block(TypeBlock::primitive(63, "UBYTE", 1, 1));
    b.type_block(TypeBlock::enumeration(64, "TexFlags", 4));
    b.type_block(TypeBlock::enumeration(65, "TexFormat", 4));
    b.type_block(TypeBlock::enumeration(66, "GfxTexWrap", 4));
    b.type_block(TypeBlock::short(67, "CStaticArray<UBYTE>", 0, 6, 63));
    b.type_block(TypeBlock::class(
        68,
        "CResource",
        4,
        5,
        Some(3),
        &[("1", 62)],
    ));
    b.type_block(TypeBlock::class(
        69,
        "CBaseTexture",
        9,
        5,
        Some(68),
        &[
            ("1", 62),
            ("2", 62),
            ("4", 64),
            ("6", 65),
            ("18", 67),
            ("9", 66),
            ("10", 66),
            ("14", 40),
        ],
    ));
    b.type_block(TypeBlock::class(70, "CTexture", 1, 5, Some(69), &[]));
    b.type_block(TypeBlock::class(71, "CStaticTexture", 1, 5, Some(70), &[]));
    // The haze classes (`environment-schema.md`).
    b.type_block(TypeBlock::class(
        73,
        "Box3f",
        1,
        5,
        None,
        &[("1", 1), ("2", 1), ("3", 1), ("4", 1), ("5", 1), ("6", 1)],
    ));
    b.type_block(TypeBlock::class(
        74,
        "CHazeEffectRenderable",
        6,
        5,
        Some(14),
        &[("1", 73), ("2", 1), ("3", 40), ("4", 1), ("5", 1), ("7", 1)],
    ));
    b.type_block(TypeBlock::pointer(75, "CRenderable*", 74));
    b.type_block(TypeBlock::pointer(76, "Handle<CBaseEntity>", 7));
    b.type_block(TypeBlock::class(
        77,
        "CHazeEntity",
        3,
        5,
        Some(7),
        &[("1", 75), ("en_henParent", 76), ("en_idParentPart", 49)],
    ));
    b.type_block(TypeBlock::class(
        78,
        "CHazeProperties",
        9,
        5,
        Some(10),
        &[
            ("1", 40),
            ("2", 1),
            ("3", 1),
            ("4", 1),
            ("5", 1),
            ("6", 1),
            ("7", 1),
            ("10", 1),
            ("11", 49),
        ],
    ));
    let texture_ident = b.identifier("3 texture");
    let uv_ident = b.identifier("3 uvmap");
    let shadow_ident = b.identifier("Shadow");
    let bright_ident = b.identifier("full bright");
    b.external_object(50, module, 10, 100);
    b.external_object(51, mdl, 1, 101);
    for (id, path) in [
        (70, NORTH_TEX),
        (71, WEST_TEX),
        (72, SOUTH_TEX),
        (73, EAST_TEX),
        (74, CEILING_TEX),
    ] {
        let file = b.file_ref(path, 0);
        b.external_object(id, file, 1, 102);
    }

    let mut entities = vec![3, 4, 5, 6];
    if let Some(id) = v.entity_override {
        entities[1] = id;
    }
    let mut records = vec![60, 61, 62];
    if v.sky != SkyKind::Absent {
        entities.push(7);
    }
    if v.chain != ChainKind::Absent {
        entities.extend([8, 9]);
    }
    if v.haze != HazeKind::Absent {
        entities.push(24);
    }
    if v.haze == HazeKind::WithRecord {
        records.push(64);
    }
    if matches!(v.sky, SkyKind::WithRecord | SkyKind::NoFaces) {
        records.push(63);
    }
    for i in 0..v.worlds {
        let id = if i == 0 { 1 } else { 90 + i as u32 };
        b.object(
            id,
            13,
            ObjectBytes::new()
                .dynamic_container(&entities)
                .dynamic_container(&records)
                .f32(0.0)
                .f32(-30.0)
                .f32(0.0)
                .f32(0.5)
                .finish(),
        );
    }
    let entity = |serial: u32| {
        ObjectBytes::new()
            .u32(serial)
            .object_ref(Some(50))
            .u32(serial)
    };
    // Entity 3: the static model with a hull.
    b.object(
        3,
        25,
        entity(1)
            .object_ref(Some(10))
            .object_ref(Some(11))
            .object_ref(Some(51))
            .finish(),
    );
    // Entity 4: a static model without hull, embedded model.
    b.object(
        4,
        25,
        entity(2)
            .object_ref(Some(14))
            .object_ref(None)
            .object_ref(Some(15))
            .finish(),
    );
    // Entity 5: spawn marker; entity 6: chapter info.
    b.object(
        5,
        27,
        entity(3).object_ref(Some(17)).object_ref(Some(18)).finish(),
    );
    let next_of_6 = (v.chain == ChainKind::Cycle).then_some(8);
    b.object(6, 28, entity(4).object_ref(next_of_6).finish());
    // Entity 8: the chain's first chapter (no record), placed by renderable 23; entity 9: the
    // world info.
    if v.chain != ChainKind::Absent {
        b.object(8, 28, entity(6).object_ref(Some(6)).finish());
        b.object(
            23,
            20,
            aspect(
                None,
                (ROT_Y90, [4.0, 5.0, 6.0]),
                (ROT_Y90, [4.0, 5.0, 6.0]),
                Some(8),
            )
            .finish(),
        );
        let first = if v.chain == ChainKind::Dangling {
            999
        } else {
            8
        };
        b.object(9, 35, entity(7).object_ref(Some(first)).finish());
    }
    // Renderable 10 (child of hull 12): local differs from world.
    b.object(
        10,
        19,
        aspect(
            Some(12),
            (IDENTITY, [0.0, 0.0, 1.0]),
            (ROT_Y90, [6.0, 0.0, 0.0]),
            Some(3),
        )
        .object_ref(Some(13))
        .finish(),
    );
    b.object(11, 23, Vec::new());
    b.object(
        12,
        21,
        aspect(
            None,
            (ROT_Y90, [5.0, 0.0, 0.0]),
            (ROT_Y90, [5.0, 0.0, 0.0]),
            Some(3),
        )
        .finish(),
    );
    b.object(
        13,
        16,
        ObjectBytes::new()
            .f32(2.0)
            .f32(2.0)
            .f32(-1.0)
            .object_ref(Some(51))
            .object_ref(match v.lightmap {
                LightmapKind::NoList => None,
                LightmapKind::DanglingList => Some(998),
                _ => Some(100),
            })
            .finish(),
    );
    b.object(
        14,
        19,
        aspect(
            None,
            (IDENTITY, [0.0, 2.0, 0.0]),
            (IDENTITY, [0.0, 2.0, 0.0]),
            Some(4),
        )
        .object_ref(Some(16))
        .finish(),
    );
    b.object(15, 33, Vec::new());
    b.object(
        16,
        16,
        ObjectBytes::new()
            .f32(1.0)
            .f32(1.0)
            .f32(1.0)
            .object_ref(Some(15))
            .object_ref(None)
            .finish(),
    );
    // The marker's renderables: 17 the top, 18 its child.
    b.object(
        17,
        20,
        aspect(
            None,
            (ROT_Y90, [10.0, 0.0, 10.0]),
            (ROT_Y90, [10.0, 0.0, 10.0]),
            Some(5),
        )
        .finish(),
    );
    b.object(
        18,
        20,
        aspect(
            Some(17),
            (IDENTITY, [0.0, 0.0, -2.5]),
            (ROT_Y90, [7.5, 0.0, 10.0]),
            Some(5),
        )
        .finish(),
    );
    b.object(
        19,
        20,
        aspect(
            None,
            (IDENTITY, [1.0, 2.0, 3.0]),
            (IDENTITY, [1.0, 2.0, 3.0]),
            Some(6),
        )
        .finish(),
    );

    // The lightmap chain of instance 13 (list 100) and of the lone instance 104 (list 105).
    let oven = |texture: u32| {
        ObjectBytes::new()
            .u32(0xFFFF_FFFF)
            .object_ref(Some(texture))
            .u32(texture_ident)
            .u32(shadow_ident)
            .u32(uv_ident)
            .object_ref(None)
            .finish()
    };
    let mut list = match v.lightmap {
        LightmapKind::NoOven => vec![102],
        _ => vec![101, 102],
    };
    if v.lightmap == LightmapKind::Two {
        list.push(103);
    }
    b.object(
        100,
        53,
        ObjectBytes::new().dynamic_container(&list).finish(),
    );
    let first_texture = if v.lightmap == LightmapKind::DanglingTexture {
        999
    } else {
        110
    };
    b.object(101, 60, oven(first_texture));
    b.object(
        102,
        61,
        ObjectBytes::new()
            .u32(0xFFFF_FFFF)
            .u32(0)
            .u32(bright_ident)
            .finish(),
    );
    b.object(103, 60, oven(111));
    b.object(
        104,
        16,
        ObjectBytes::new()
            .f32(1.0)
            .f32(1.0)
            .f32(1.0)
            .object_ref(Some(51))
            .object_ref(Some(105))
            .finish(),
    );
    b.object(
        105,
        53,
        ObjectBytes::new().dynamic_container(&[106]).finish(),
    );
    b.object(106, 60, oven(112));
    let texture = |blob: &[u8]| {
        ObjectBytes::new()
            .i32(-1)
            .i32(4)
            .i32(4)
            .u32(0x71)
            .u32(0)
            .array_header(b"STAR", blob.len() as u32)
            .raw(blob)
            .u32(0x51)
            .u32(0x51)
            .u32(0)
            .finish()
    };
    let first_blob = if v.lightmap == LightmapKind::BadPixels {
        b"not zlib".to_vec()
    } else {
        zlib(&lightmap_pixels(1))
    };
    b.object(110, 71, texture(&first_blob));
    b.object(111, 71, texture(&zlib(&lightmap_pixels(2))));
    b.object(112, 71, texture(&zlib(&lightmap_pixels(3))));

    // Entity 7: the sky, its cube renderable 20 and viewer renderable 21.
    if v.sky != SkyKind::Absent {
        let placement = (ROT_Y90, [1.0, 2.0, 3.0]);
        b.object(
            7,
            47,
            quat_vect(
                entity(5).object_ref(Some(20)).object_ref(Some(21)),
                placement.0,
                placement.1,
            )
            .f32(64.0)
            .f32(0.0)
            .u32(1)
            .finish(),
        );
        let faces: [Option<u32>; 6] = if v.sky == SkyKind::NoFaces {
            [None; 6]
        } else {
            [Some(70), Some(71), Some(72), Some(73), None, Some(74)]
        };
        let mut cube = aspect(None, placement, placement, Some(7))
            .f32(64.0)
            .f32(1.0)
            .u32(0x00FF_FFFF);
        for f in faces {
            cube = cube.object_ref(f);
        }
        b.object(20, 43, cube.finish());
        b.object(
            21,
            45,
            aspect(None, placement, placement, Some(7))
                .f32(5.22)
                .finish(),
        );
    }

    // Entity 24: the haze, its renderable 25 (turned 90 degrees about +y at (10, 0, 20); the
    // record's values differ from the renderable's, to show which is read).
    if v.haze != HazeKind::Absent {
        let parent = (v.haze == HazeKind::WithRecord).then_some(3);
        b.object(
            24,
            77,
            entity(8)
                .object_ref(Some(25))
                .object_ref(parent)
                .u32(0)
                .finish(),
        );
        let placement = (ROT_Y90, [10.0, 0.0, 20.0]);
        b.object(
            25,
            74,
            aspect(None, placement, placement, Some(24))
                .f32(-150.0)
                .f32(-50.0)
                .f32(-200.0)
                .f32(150.0)
                .f32(50.0)
                .f32(200.0)
                .f32(5.0)
                .u32(0x112a_383d)
                .f32(-15.0)
                .f32(186.0)
                .f32(7.0)
                .finish(),
        );
    }

    // The property records, in the extra section.
    let record = |id: u32, type_id: u32, entity: u32, name: &str| {
        let r = ObjectBytes::new()
            .u32(id)
            .u32(type_id)
            .object_ref(Some(entity));
        quat_vect(r, IDENTITY, [0.0, 0.0, 0.0])
            .cstring(name)
            .object_ref(None)
            .finish()
    };
    let mut extra = Vec::new();
    for (id, type_id, entity, name) in [
        (60, 29, 3, "Column"),
        (61, 30, 5, "Start01"),
        (62, 32, 6, "Chapter001"),
    ] {
        b.edty_pair(id, type_id);
        extra.extend(record(id, type_id, entity, name));
    }
    if records.contains(&63) {
        // The sky's record: values that differ from the renderable's, to show which is read.
        let texture = (v.sky == SkyKind::WithRecord).then_some(72);
        b.edty_pair(63, 48);
        let r = quat_vect(
            ObjectBytes::new().u32(63).u32(48).object_ref(Some(7)),
            IDENTITY,
            [0.0, 0.0, 0.0],
        )
        .cstring("MDG_VillageDay_Bckg")
        .object_ref(None)
        .object_ref(texture)
        .u32(0xFA7E_7E7E)
        .f32(32.0)
        .f32(-2.5)
        .f32(0.0)
        .u32(1)
        .u32(1);
        extra.extend(r.finish());
    }
    if records.contains(&64) {
        b.edty_pair(64, 78);
        let r = quat_vect(
            ObjectBytes::new().u32(64).u32(78).object_ref(Some(24)),
            IDENTITY,
            [0.0, 0.0, 0.0],
        )
        .cstring("Haze001")
        .object_ref(None)
        .u32(0x002a_383d)
        .f32(300.0)
        .f32(100.0)
        .f32(400.0)
        .f32(0.0)
        .f32(-150.0)
        .f32(1860.0)
        .f32(0.0)
        .u32(0);
        extra.extend(r.finish());
    }
    b.after_edob(records.len() as u32, &extra);
    b.build().unwrap()
}

fn assert_vec(got: Vec3, want: [f32; 3]) {
    assert!(
        got.abs_diff_eq(Vec3::from(want), 1e-6),
        "{got:?} != {want:?}"
    );
}

fn assert_quat(got: Quat, want: [f32; 4]) {
    assert!(
        got.abs_diff_eq(Quat::from_array(want), 1e-6),
        "{got:?} != {want:?}"
    );
}

#[test]
fn reads_gravity_and_counts() {
    let world = World::from_bytes(&world_file(NORMAL)).unwrap();
    assert_vec(world.gravity, [0.0, -30.0, 0.0]);
    assert_eq!(world.entity_count, 4);
    assert_eq!(world.static_models.len(), 2);
    assert_eq!(world.spawn_markers.len(), 1);
    assert_eq!(world.chapters.len(), 1);
}

#[test]
fn a_static_model_is_placed_by_its_renderables_world_placement() {
    let world = World::from_bytes(&world_file(NORMAL)).unwrap();
    let m = &world.static_models[0];
    assert_eq!(
        (m.serial, m.name.as_str(), &m.model, m.has_hull),
        (
            1,
            "Column",
            &ModelRef::External("Content/Test/Box.mdl".into()),
            true
        )
    );
    assert_eq!(m.model.path(), Some("Content/Test/Box.mdl"));
    // The renderable's *world* placement, not its local one.
    assert_quat(m.rotation, ROT_Y90);
    assert_vec(m.translation, [6.0, 0.0, 0.0]);
    assert_vec(m.scale, [2.0, 2.0, -1.0]);
    // The transform scales first (mirroring z), then rotates, then translates.
    let p = m.transform().transform_point3(Vec3::new(0.0, 0.0, 1.0));
    // (0,0,1) -> scale (0,0,-1) -> rotate 90 deg about y: (-1,0,0) -> translate by (6,0,0).
    assert_vec(p, [5.0, 0.0, 0.0]);
}

#[test]
fn a_static_model_without_mechanism_has_no_hull_and_may_embed_its_model() {
    let world = World::from_bytes(&world_file(NORMAL)).unwrap();
    let m = &world.static_models[1];
    assert_eq!(
        (m.serial, m.name.as_str(), &m.model, m.has_hull),
        (2, "", &ModelRef::Embedded(15), false)
    );
    assert_eq!(m.model.path(), None);
    assert_vec(m.translation, [0.0, 2.0, 0.0]);
    assert_vec(m.scale, [1.0, 1.0, 1.0]);
}

#[test]
fn markers_take_the_world_placement_of_their_top_aspect() {
    let world = World::from_bytes(&world_file(NORMAL)).unwrap();
    let s = &world.spawn_markers[0];
    assert_eq!((s.serial, s.name.as_str()), (3, "Start01"));
    assert_quat(s.rotation, ROT_Y90);
    assert_vec(s.translation, [10.0, 0.0, 10.0]);
    let c = &world.chapters[0];
    assert_eq!((c.serial, c.name.as_str()), (4, "Chapter001"));
    assert_quat(c.rotation, IDENTITY);
    assert_vec(c.translation, [1.0, 2.0, 3.0]);
}

#[test]
fn without_a_world_info_there_is_no_chain() {
    let world = World::from_bytes(&world_file(NORMAL)).unwrap();
    assert_eq!(world.first_chapter, None);
    assert_eq!(world.chapters[0].next, None);
    assert_eq!(world.spawn_markers[0].next, None);
    assert!(world.chapter_chain().is_empty());
}

#[test]
fn the_chain_starts_at_the_world_infos_first_chapter_not_the_first_in_the_list() {
    let world = World::from_bytes(&world_file(Variant {
        chain: ChainKind::Linked,
        ..NORMAL
    }))
    .unwrap();
    assert_eq!(world.entity_count, 6);
    let names: Vec<_> = world.chapters.iter().map(|c| c.serial).collect();
    assert_eq!(names, [4, 6]);
    assert_eq!(world.first_chapter, Some(1));
    assert_eq!(
        (world.chapters[0].next, world.chapters[1].next),
        (None, Some(0))
    );
    assert_eq!(world.chapter_chain(), [1, 0]);
    let first = &world.chapters[1];
    assert_quat(first.rotation, ROT_Y90);
    assert_vec(first.translation, [4.0, 5.0, 6.0]);
}

#[test]
fn a_chain_that_comes_back_stops_before_the_repeat() {
    let world = World::from_bytes(&world_file(Variant {
        chain: ChainKind::Cycle,
        ..NORMAL
    }))
    .unwrap();
    assert_eq!(world.chapters[0].next, Some(1));
    assert_eq!(world.chapter_chain(), [1, 0]);
}

#[test]
fn a_first_chapter_handle_that_names_no_chapter_gives_no_chain() {
    let world = World::from_bytes(&world_file(Variant {
        chain: ChainKind::Dangling,
        ..NORMAL
    }))
    .unwrap();
    assert_eq!(world.first_chapter, None);
    assert_eq!(world.chapters[1].next, Some(0));
    assert!(world.chapter_chain().is_empty());
}

#[test]
fn a_world_without_a_sky_entity_has_no_sky() {
    assert_eq!(World::from_bytes(&world_file(NORMAL)).unwrap().sky, None);
}

#[test]
fn the_sky_is_read_from_its_renderable_and_record() {
    let world = World::from_bytes(&world_file(Variant {
        sky: SkyKind::WithRecord,
        ..NORMAL
    }))
    .unwrap();
    assert_eq!(world.entity_count, 5);
    let sky = world.sky.expect("a sky");
    assert_eq!((sky.serial, sky.name.as_str()), (5, "MDG_VillageDay_Bckg"));
    // The face members `.10`..`.15` are North, West, South, East, Floor, Ceiling.
    assert_eq!(sky.face(SkyFace::North), Some(NORTH_TEX));
    assert_eq!(sky.face(SkyFace::West), Some(WEST_TEX));
    assert_eq!(sky.face(SkyFace::South), Some(SOUTH_TEX));
    assert_eq!(sky.face(SkyFace::East), Some(EAST_TEX));
    assert_eq!(sky.face(SkyFace::Floor), None);
    assert_eq!(sky.face(SkyFace::Ceiling), Some(CEILING_TEX));
    assert_eq!(sky.face_count(), 5);
    // The record's values win over the renderable's (they agree in the retail data).
    assert_eq!(sky.texture.as_deref(), Some(SOUTH_TEX));
    assert_eq!(sky.color, 0xFA7E_7E7E);
    assert_eq!((sky.size, sky.viewer_y, sky.flag_4), (32.0, -2.5, 0.0));
    assert_eq!(sky.flag_5, Some(true));
    assert_eq!(sky.viewer_value, Some(5.22));
    assert_quat(sky.rotation, ROT_Y90);
    assert_vec(sky.translation, [1.0, 2.0, 3.0]);
}

#[test]
fn a_sky_without_a_record_takes_the_renderables_values() {
    let world = World::from_bytes(&world_file(Variant {
        sky: SkyKind::WithoutRecord,
        ..NORMAL
    }))
    .unwrap();
    let sky = world.sky.expect("a sky");
    assert_eq!((sky.serial, sky.name.as_str()), (5, ""));
    assert_eq!(sky.face(SkyFace::North), Some(NORTH_TEX));
    assert_eq!(sky.texture, None);
    assert_eq!(sky.color, 0x00FF_FFFF);
    assert_eq!((sky.size, sky.viewer_y, sky.flag_4), (64.0, 0.0, 1.0));
    assert_eq!(sky.flag_5, None);
}

#[test]
fn a_sky_may_have_no_face_textures() {
    let world = World::from_bytes(&world_file(Variant {
        sky: SkyKind::NoFaces,
        ..NORMAL
    }))
    .unwrap();
    let sky = world.sky.expect("a sky");
    assert!(SkyFace::ALL.iter().all(|f| sky.face(*f).is_none()));
    assert_eq!(sky.face_count(), 0);
    assert_eq!(sky.texture, None);
    assert_eq!(sky.color, 0xFA7E_7E7E);
}

#[test]
fn sky_faces_name_their_members_in_order() {
    let members: Vec<_> = SkyFace::ALL.iter().map(|f| f.member()).collect();
    assert_eq!(members, ["10", "11", "12", "13", "14", "15"]);
    for (i, f) in SkyFace::ALL.iter().enumerate() {
        assert_eq!(f.index(), i);
    }
}

fn with_lightmap(lightmap: LightmapKind) -> World {
    World::from_bytes(&world_file(Variant { lightmap, ..NORMAL })).unwrap()
}

#[test]
fn a_static_model_reads_the_lightmap_of_its_oven_modifier() {
    let world = with_lightmap(LightmapKind::One);
    let lm = world.static_models[0]
        .lightmap
        .as_ref()
        .expect("model 0 is lightmapped");
    assert_eq!(lm.modifier, 101);
    assert_eq!(lm.texture_object, Some(110));
    assert_eq!(
        (lm.texture_param.name.as_str(), lm.uv_param.name.as_str()),
        ("3 texture", "3 uvmap")
    );
    assert_eq!(lm.uv_map.name, "Shadow");
    assert_eq!(lm.other_oven_modifiers, 0);
    // The texture is decoded from the world's own `CStaticTexture` object.
    let t = lm.texture.as_ref().expect("the lightmap decodes");
    assert_eq!((t.width, t.height, t.flags), (4, 4, 0x71));
    assert_eq!(t.format, ss2_format_tex::PixelFormat::Bgr8);
    assert_eq!(t.data, lightmap_pixels(1));
    // The other static model has no modifier list.
    assert!(world.static_models[1].lightmap.is_none());
    // The counts cover every model instance, also the one no static model uses.
    assert_eq!(
        world.lightmap_counts,
        LightmapCounts {
            lit_instances: 2,
            oven_modifiers: 2,
            several: 0,
        }
    );
}

#[test]
fn of_several_oven_modifiers_the_first_is_taken_and_the_rest_counted() {
    let world = with_lightmap(LightmapKind::Two);
    let lm = world.static_models[0].lightmap.as_ref().unwrap();
    assert_eq!((lm.modifier, lm.texture_object), (101, Some(110)));
    assert_eq!(lm.other_oven_modifiers, 1);
    assert_eq!(
        world.lightmap_counts,
        LightmapCounts {
            lit_instances: 2,
            oven_modifiers: 3,
            several: 1,
        }
    );
}

#[test]
fn a_missing_or_broken_chain_is_no_lightmap() {
    for kind in [
        LightmapKind::NoList,
        LightmapKind::NoOven,
        LightmapKind::DanglingList,
    ] {
        let world = with_lightmap(kind);
        assert!(world.static_models[0].lightmap.is_none());
        // Only the lone instance is lit.
        assert_eq!(world.lightmap_counts.lit_instances, 1);
    }
}

#[test]
fn a_lightmap_whose_texture_cannot_be_read_says_why() {
    let world = with_lightmap(LightmapKind::DanglingTexture);
    let lm = world.static_models[0].lightmap.as_ref().unwrap();
    assert_eq!(lm.texture_object, None);
    let err = lm.texture.as_ref().unwrap_err();
    assert!(err.contains("999"), "{err}");

    let world = with_lightmap(LightmapKind::BadPixels);
    let lm = world.static_models[0].lightmap.as_ref().unwrap();
    assert_eq!(lm.texture_object, Some(110));
    let err = lm.texture.as_ref().unwrap_err();
    assert!(err.contains("zlib"), "{err}");
}

#[test]
fn a_world_without_a_haze_entity_has_no_haze() {
    assert!(
        World::from_bytes(&world_file(NORMAL))
            .unwrap()
            .hazes
            .is_empty()
    );
}

#[test]
fn a_haze_is_read_from_its_record_and_placed_by_its_renderable() {
    let world = World::from_bytes(&world_file(Variant {
        haze: HazeKind::WithRecord,
        ..NORMAL
    }))
    .unwrap();
    let [h] = world.hazes.as_slice() else {
        panic!("{:?}", world.hazes);
    };
    assert_eq!((h.serial, h.name.as_str()), (8, "Haze001"));
    // `1_3_village`'s numbers, from the record.
    assert_eq!(h.color, 0x002a_383d);
    assert_vec(h.size, [300.0, 100.0, 400.0]);
    assert_eq!(
        (h.fade, h.near, h.far, h.value_10),
        (0.0, -150.0, 1860.0, 0.0)
    );
    assert_vec(h.local_box.0, [-150.0, -50.0, -200.0]);
    assert_vec(h.local_box.1, [150.0, 50.0, 200.0]);
    assert_quat(h.rotation, ROT_Y90);
    assert_vec(h.translation, [10.0, 0.0, 20.0]);
    // The parent is entity 3, the static model with serial 1.
    assert_eq!(h.parent, Some(1));
}

#[test]
fn a_haze_without_a_record_takes_the_renderables_values() {
    let world = World::from_bytes(&world_file(Variant {
        haze: HazeKind::WithoutRecord,
        ..NORMAL
    }))
    .unwrap();
    let h = &world.hazes[0];
    assert_eq!(h.name, "");
    assert_eq!(h.color, 0x112a_383d);
    assert_vec(h.size, [300.0, 100.0, 400.0]);
    assert_eq!(
        (h.fade, h.near, h.far, h.value_10),
        (5.0, -15.0, 186.0, 7.0)
    );
    assert_eq!(h.parent, None);
}

#[test]
fn an_embedded_model_is_read_once_from_the_world() {
    let world = World::from_bytes(&world_file(NORMAL)).unwrap();
    // Entity 4 names object 15, an empty configuration: it is listed, and why it cannot be read
    // is kept (the world itself still reads).
    assert_eq!(world.embedded_models.len(), 1);
    let e = world.embedded_model(15).unwrap();
    assert_eq!(e.object, 15);
    assert!(e.model.is_err(), "{e:?}");
    assert!(
        world.embedded_model(51).is_none(),
        "an external model is not embedded"
    );
}

#[test]
fn a_world_needs_exactly_one_root() {
    for worlds in [0, 2] {
        let err = World::from_bytes(&world_file(Variant { worlds, ..NORMAL }))
            .unwrap_err()
            .to_string();
        assert!(err.contains("CWorld"), "{err}");
    }
}

#[test]
fn a_dangling_entity_or_a_model_that_is_not_an_mdl_is_an_error() {
    let err = World::from_bytes(&world_file(Variant {
        entity_override: Some(77),
        ..NORMAL
    }))
    .unwrap_err()
    .to_string();
    assert!(err.contains("77"), "{err}");
    let err = World::from_bytes(&world_file(Variant {
        model_file: "Content/Test/Box.tex",
        ..NORMAL
    }))
    .unwrap_err()
    .to_string();
    assert!(err.contains("Box.tex"), "{err}");
    assert!(World::from_bytes(b"junk").is_err());
}

#[test]
fn nothing_panics_on_damaged_files() {
    let good = world_file(Variant {
        sky: SkyKind::WithRecord,
        lightmap: LightmapKind::Two,
        ..NORMAL
    });
    for len in (0..good.len()).step_by(7) {
        let _ = World::from_bytes(&good[..len]);
    }
    for i in 0..good.len() {
        let mut bad = good.clone();
        bad[i] ^= 0x5A;
        let _ = World::from_bytes(&bad);
    }
}

#[test]
fn the_loader_plugs_into_the_asset_manager() {
    let file = world_file(NORMAL);
    let vfs = MemoryVfs::from_files([("levels/a.wld", file.as_slice())]).unwrap();
    let assets = AssetManager::new(Arc::new(vfs));
    assets.register_loader(WldLoader).unwrap();
    let h = assets.load::<World>("LEVELS/A.WLD").unwrap();
    assert_eq!(h.wait(), LoadState::Ready);
    assert_eq!(h.get().unwrap().static_models.len(), 2);
}
