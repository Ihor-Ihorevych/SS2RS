use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// Type table shared by the synthetic files (ids 0 to 40).
fn types(b: &mut MetaBuilder) {
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
    b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(4, "IDENT", 4, 4));
    b.type_block(TypeBlock::enumeration(5, "SndFlags", 4));
    b.type_block(TypeBlock::enumeration(6, "EPlatform", 4));
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
    b.type_block(TypeBlock::pointer(10, "CStaticSound*", 100));
    b.type_block(TypeBlock::pointer(11, "CEffect*", 101));
    b.type_block(TypeBlock::alias(12, "BOOL", 2));
    // Sound scheme.
    b.type_block(TypeBlock::class(
        13,
        "CWeightedSoundInScheme",
        1,
        5,
        None,
        &[
            ("1", 0),
            ("2", 10),
            ("3", 5),
            ("4", 3),
            ("5", 3),
            ("6", 3),
            ("7", 6),
        ],
    ));
    b.type_block(TypeBlock::short(
        14,
        "CStaticArray<CWeightedSoundInScheme>",
        0,
        6,
        13,
    ));
    b.type_block(TypeBlock::class(
        15,
        "CSoundInScheme",
        1,
        5,
        None,
        &[("1", 4), ("2", 14), ("3", 6)],
    ));
    b.type_block(TypeBlock::short(
        16,
        "CStaticArray<CSoundInScheme>",
        0,
        6,
        15,
    ));
    b.type_block(TypeBlock::pointer(17, "CSoundScheme*", 18));
    b.type_block(TypeBlock::class(
        18,
        "CSoundScheme",
        1,
        5,
        Some(9),
        &[("1", 16), ("2", 17)],
    ));
    // Particle scheme.
    b.type_block(TypeBlock::class(
        19,
        "CWeightedPFXInScheme",
        1,
        5,
        None,
        &[("1", 0), ("2", 11), ("3", 0)],
    ));
    b.type_block(TypeBlock::short(
        20,
        "CStaticArray<CWeightedPFXInScheme>",
        0,
        6,
        19,
    ));
    b.type_block(TypeBlock::class(
        21,
        "CPFXInScheme",
        1,
        5,
        None,
        &[("1", 4), ("2", 20)],
    ));
    b.type_block(TypeBlock::short(22, "CStaticArray<CPFXInScheme>", 0, 6, 21));
    b.type_block(TypeBlock::pointer(23, "CParticleScheme*", 24));
    b.type_block(TypeBlock::class(
        24,
        "CParticleScheme",
        1,
        5,
        Some(9),
        &[("1", 22), ("2", 23)],
    ));
    // Material.
    b.type_block(TypeBlock::class(
        25,
        "CMaterialAttributes",
        1,
        5,
        Some(9),
        &[("1", 3), ("2", 3), ("4", 3), ("3", 3)],
    ));
    b.type_block(TypeBlock::class(
        26,
        "CSamMaterial",
        1,
        5,
        Some(25),
        &[
            ("sm_fMovementSpeedMultiplier", 3),
            ("sm_bApplySpeedMultiplierOnPlayersOnly", 12),
            ("sm_fDamageApplyPeriod", 3),
            ("sm_slApplyDamageAmount", 0),
            ("sm_bApplyDamageOnPlayersOnly", 12),
            ("sm_ppsParticleScheme", 23),
            ("sm_pssSoundScheme", 17),
        ],
    ));
}

fn alt(weight: i32, sound: Option<u32>, flags: u32, pitch: f32, var: f32, vol: f32) -> Vec<u8> {
    ObjectBytes::new()
        .i32(weight)
        .object_ref(sound)
        .u32(flags)
        .f32(pitch)
        .f32(var)
        .f32(vol)
        .u32(0x3)
        .finish()
}

/// A `.scs`: events `Fire` (two alternatives) and `Step` (one), base scheme external.
pub fn sound_scheme_file() -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let f1 = b.file_ref("Content/Test/Sounds/fire1.wav", 0);
    let f2 = b.file_ref("Content/Test/Sounds/fire2.wav", 0);
    let base = b.file_ref("Content/Test/Sounds/Default.scs", 0);
    b.external_type(100, "CStaticSound");
    b.external_type(102, "CSoundScheme");
    types(&mut b);
    let fire = b.identifier("Fire");
    let step = b.identifier("Step");
    b.external_object(10, f1, 1, 100);
    b.external_object(11, f2, 1, 100);
    b.external_object(12, base, 1, 102);
    b.object(0, 7, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        18,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .array_header(b"STAR", 2)
            .u32(fire)
            .array_header(b"STAR", 2)
            .raw(&alt(1, Some(10), 2, 1.0, 0.05, 1.0))
            .raw(&alt(3, Some(11), 6, 1.2, 0.1, 0.8))
            .u32(0x3)
            .u32(step)
            .array_header(b"STAR", 1)
            .raw(&alt(1, None, 3, 1.0, 0.0, 1.0))
            .u32(0x1)
            .object_ref(Some(12))
            .finish(),
    );
    b.build().unwrap()
}

/// A `.mat` with an embedded particle scheme and an embedded sound scheme.
pub fn material_file() -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let pfx = b.file_ref("Content/Test/Effects/hit.pfx", 0);
    let wav = b.file_ref("Content/Test/Sounds/hit.wav", 0);
    b.external_type(100, "CStaticSound");
    b.external_type(101, "CEffect");
    types(&mut b);
    let hit = b.identifier("BulletHit");
    b.external_object(10, pfx, 1, 101);
    b.external_object(11, wav, 1, 100);
    b.object(0, 7, ObjectBytes::new().i32(1).finish());
    b.object(
        1,
        26,
        ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .f32(50.0)
            .f32(0.25)
            .f32(0.5)
            .f32(0.125) // attributes .1 .2 .4 .3
            .f32(0.75)
            .u32(1)
            .f32(1.0)
            .i32(50)
            .u32(0)
            .object_ref(Some(2))
            .object_ref(Some(3))
            .finish(),
    );
    b.object(
        2,
        24,
        ObjectBytes::new()
            .i32(-1)
            .object_ref(None)
            .array_header(b"STAR", 1)
            .u32(hit)
            .array_header(b"STAR", 1)
            .i32(1)
            .object_ref(Some(10))
            .i32(0)
            .object_ref(None)
            .finish(),
    );
    b.object(
        3,
        18,
        ObjectBytes::new()
            .i32(-1)
            .object_ref(None)
            .array_header(b"STAR", 1)
            .u32(hit)
            .array_header(b"STAR", 1)
            .raw(&alt(1, Some(11), 2, 1.0, 0.05, 1.0))
            .u32(0xFFFF_FFFF)
            .object_ref(None)
            .finish(),
    );
    b.build().unwrap()
}
