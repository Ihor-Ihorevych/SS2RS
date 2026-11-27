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
}

const NORMAL: Variant = Variant {
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
    let good = world_file(NORMAL);
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
