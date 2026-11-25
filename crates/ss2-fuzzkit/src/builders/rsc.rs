use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// Types 0 to 7 shared by the fixtures: `SLONG`, `FLOAT`, `IDENT`, `BOOL`, `CString`, a
/// `CResource` base with one member, its container and the `CResourceFile` that lists the
/// shared roots.
fn base_types(b: &mut MetaBuilder) {
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::primitive(1, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(2, "IDENT", 4, 4));
    b.type_block(TypeBlock::primitive(3, "BOOL", 4, 4));
    b.type_block(TypeBlock::primitive(4, "CString", 4, 4));
    b.type_block(TypeBlock::class(5, "CResource", 4, 5, None, &[("1", 0)]));
    b.type_block(TypeBlock::short(6, "CDynamicContainer<CResource>", 0, 8, 5));
    b.type_block(TypeBlock::class(
        7,
        "CResourceFile",
        3,
        5,
        None,
        &[("rf_crsShared", 6)],
    ));
}

/// A one-object menu screen whose root class is `root_class`, with two gadgets.
pub fn menu_screen_file(root_class: &str) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let screen = b.identifier("SCreateMatchMenuScreen");
    base_types(&mut b);
    b.type_block(TypeBlock::class(
        8,
        "CMenuGadgetLayout",
        1,
        5,
        None,
        &[("1", 1)],
    ));
    b.type_block(TypeBlock::short(
        9,
        "CDynamicContainer<CMenuGadgetLayout>",
        0,
        8,
        8,
    ));
    b.type_block(TypeBlock::class(
        10,
        root_class,
        1,
        5,
        Some(5),
        &[("1", 2), ("3", 4), ("4", 4), ("2", 9)],
    ));
    b.object(0, 7, ObjectBytes::new().dynamic_container(&[1]).finish());
    b.object(
        1,
        10,
        ObjectBytes::new()
            .i32(1)
            .u32(screen)
            .cstring("CMSUnderlay")
            .cstring("")
            .dynamic_container(&[2, 3])
            .finish(),
    );
    b.object(2, 8, ObjectBytes::new().f32(0.5).finish());
    b.object(3, 8, ObjectBytes::new().f32(0.25).finish());
    b.build().unwrap()
}

/// A tool; `creator` is the class of its entity creator (`None` = null).
pub fn tool_file(root_class: &str, with_model: bool, creator: Option<&str>) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let throw = b.identifier("Throw");
    let hand = b.identifier("RightHand");
    let empty = b.identifier("");
    let model = b.file_ref("Content/Test/Models/axe.mdl", 0x4_0000);
    b.external_type(100, "CModelConfiguration");
    base_types(&mut b);
    b.type_block(TypeBlock::class(
        8,
        "Quaternion4f",
        1,
        5,
        None,
        &[("1", 1), ("2", 1), ("3", 1), ("4", 1)],
    ));
    b.type_block(TypeBlock::class(
        9,
        "Vector3f",
        1,
        5,
        None,
        &[("1", 1), ("2", 1), ("3", 1)],
    ));
    b.type_block(TypeBlock::class(
        10,
        "QuatVect",
        1,
        5,
        None,
        &[("1", 8), ("2", 9)],
    ));
    let mut lv: Vec<(&str, u32)> = vec![("lv_bRelativeToAttachment", 3)];
    lv.extend(
        [
            "lv_fMinVelocity",
            "lv_fMaxVelocity",
            "lv_fMinHeading",
            "lv_fMaxHeading",
            "lv_fMinPitch",
            "lv_fMaxPitch",
            "lv_fMinRotationH",
            "lv_fMaxRotationH",
            "lv_fMinRotationP",
            "lv_fMaxRotationP",
            "lv_fMinRotationB",
            "lv_fMaxRotationB",
        ]
        .map(|n| (n, 1)),
    );
    b.type_block(TypeBlock::class(11, "CLaunchVelocities", 1, 5, None, &lv));
    b.type_block(TypeBlock::pointer(12, "Ptr<CModelConfiguration>", 100));
    b.type_block(TypeBlock::class(13, "CToolEntityCreator", 1, 5, None, &[]));
    b.type_block(TypeBlock::class(
        14,
        "CDebrisToolEntityCreator",
        1,
        5,
        Some(13),
        &[("dtec_fLifeTime", 1)],
    ));
    b.type_block(TypeBlock::class(
        15,
        "CMysteryToolEntityCreator",
        1,
        5,
        Some(13),
        &[],
    ));
    b.type_block(TypeBlock::pointer(16, "CToolEntityCreator*", 13));
    b.type_block(TypeBlock::class(
        17,
        root_class,
        1,
        5,
        Some(5),
        &[
            ("ct_idDrawAnimationTerm", 2),
            ("ct_bLerpDrawAnimation", 3),
            ("ct_idStoreAnimationTerm", 2),
            ("ct_bLerpStoreAnimation", 3),
            ("ct_idPickAnimationTerm", 2),
            ("ct_bLerpPickAnimation", 3),
            ("ct_idThrowAnimationTerm", 2),
            ("ct_bLerpThrowAnimation", 3),
            ("ct_pmcModel", 12),
            ("ct_slHealth", 0),
            ("ct_idLimb", 2),
            ("ct_idAttachment", 2),
            ("ct_idTargetBone", 2),
            ("ct_qvTargetOffset", 10),
            ("ct_fStretch", 1),
            ("ct_bStretchWithOwner", 3),
            ("ct_bThrownWhenOwnerDies", 3),
            ("ct_lvLaunchVelocities", 11),
            ("ct_ptecToolEntityCreator", 16),
            ("ct_bStraightenUpWhenThrowing", 3),
        ],
    ));
    if with_model {
        b.external_object(5, model, 1, 100);
    }
    b.object(0, 7, ObjectBytes::new().dynamic_container(&[1]).finish());
    let mut o = ObjectBytes::new()
        .i32(1)
        .u32(empty)
        .u32(1)
        .u32(empty)
        .u32(1)
        .u32(empty)
        .u32(0)
        .u32(throw)
        .u32(1)
        .object_ref(with_model.then_some(5))
        .i32(40)
        .u32(empty)
        .u32(hand)
        .u32(empty);
    // QuatVect: identity rotation, position (1, 2, 3).
    for f in [0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 3.0] {
        o = o.f32(f);
    }
    // Stretch, stretch with owner, thrown when the owner dies, relative to the attachment.
    o = o.f32(2.0).u32(1).u32(0).u32(1);
    for f in [
        5.0, 20.0, 0.0, 10.0, 30.0, 60.0, -90.0, 90.0, -45.0, 45.0, 0.0, 1.0,
    ] {
        o = o.f32(f);
    }
    o = o.object_ref(creator.map(|_| 2)).u32(1);
    b.object(1, 17, o.finish());
    match creator {
        Some("CDebrisToolEntityCreator") => {
            b.object(2, 14, ObjectBytes::new().f32(5.0).finish());
        }
        Some(_) => {
            b.object(2, 15, Vec::new());
        }
        None => {}
    }
    b.build().unwrap()
}
