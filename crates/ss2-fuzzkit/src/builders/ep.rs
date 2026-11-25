use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// A projectile-like file: root `SGenericProjectileParams` (base `SBaseProjectileParams`, base
/// `CEntityParams`) with a model reference, an identifier, an enumeration, a float, and one
/// owned sub-object (a blast).
pub fn params_file(root_derives_from_entity_params: bool) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let model = b.file_ref("Content/Test/Models/axe.mdl", 0x4_0000);
    let name = b.identifier("Axe");
    b.external_type(100, "CModelConfiguration");
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(3, "IDENT", 4, 4));
    b.type_block(TypeBlock::enumeration(4, "EDamageType", 4));
    b.type_block(TypeBlock::pointer(5, "CModelConfiguration*", 100));
    b.type_block(TypeBlock::short(6, "CDynamicContainer<CResource>", 0, 8, 7));
    b.type_block(TypeBlock::class(7, "CResource", 4, 5, None, &[("1", 1)]));
    b.type_block(TypeBlock::class(
        8,
        "CResourceFile",
        3,
        5,
        None,
        &[("rf_crsShared", 6)],
    ));
    b.type_block(TypeBlock::class(
        9,
        "CEntityParams",
        1,
        5,
        if root_derives_from_entity_params {
            None
        } else {
            Some(7)
        },
        &[],
    ));
    b.type_block(TypeBlock::class(
        10,
        "CSingleBlast",
        1,
        5,
        None,
        &[("sb_fRadius", 2)],
    ));
    b.type_block(TypeBlock::pointer(11, "CSingleBlast*", 10));
    b.type_block(TypeBlock::class(
        12,
        "SBaseProjectileParams",
        1,
        5,
        Some(9),
        &[("bp_pmcModel", 5), ("bp_idName", 3)],
    ));
    let parent_of_root = if root_derives_from_entity_params {
        12
    } else {
        7
    };
    b.type_block(TypeBlock::class(
        13,
        "SGenericProjectileParams",
        1,
        5,
        Some(parent_of_root),
        &[("gpp_edtDamage", 4), ("gpp_pblast", 11), ("gpp_fSpeed", 2)],
    ));
    b.external_object(5, model, 1, 100);
    b.object(0, 8, ObjectBytes::new().dynamic_container(&[1]).finish());
    let mut members = ObjectBytes::new();
    if !root_derives_from_entity_params {
        members = members.i32(1);
    } else {
        members = members.object_ref(Some(5)).u32(name);
    }
    b.object(1, 13, members.u32(2).object_ref(Some(2)).f32(30.5).finish());
    b.object(2, 10, ObjectBytes::new().f32(2.5).finish());
    b.build().unwrap()
}
