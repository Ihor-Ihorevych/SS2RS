//! From `crates/ss2-format-wld/src/tests.rs` (the synthetic world of its unit tests).

use std::f32::consts::FRAC_1_SQRT_2;

use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// What to change in the synthetic world.
#[derive(Clone, Copy)]
pub struct Variant {
    /// How many `CWorld` objects (normally 1).
    pub worlds: usize,
    /// The external file of the first model (normally `Content/Test/Box.mdl`).
    pub model_file: &'static str,
    /// An entity id of `CWorld.1` that is replaced by this (to make it dangle).
    pub entity_override: Option<u32>,
}

pub const NORMAL: Variant = Variant {
    worlds: 1,
    model_file: "Content/Test/Box.mdl",
    entity_override: None,
};

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
pub fn world_file(v: Variant) -> Vec<u8> {
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
        &[("1", 4), ("3", 17)],
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
    b.external_object(50, module, 10, 100);
    b.external_object(51, mdl, 1, 101);

    let mut entities = vec![3, 4, 5, 6];
    if let Some(id) = v.entity_override {
        entities[1] = id;
    }
    for i in 0..v.worlds {
        let id = if i == 0 { 1 } else { 90 + i as u32 };
        b.object(
            id,
            13,
            ObjectBytes::new()
                .dynamic_container(&entities)
                .dynamic_container(&[60, 61, 62])
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
    b.object(6, 28, entity(4).finish());
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
    b.after_edob(3, &extra);
    b.build().unwrap()
}
