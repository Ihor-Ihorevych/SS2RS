use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// Ways to break the synthetic file.
#[derive(Clone, Copy, Default)]
pub struct Flaws {
    /// The translation envelope says 4 keys but lists 3.
    count_mismatch: bool,
    /// The translation keys are not ascending.
    descending: bool,
    /// A translation key lies after the last frame.
    key_out_of_range: bool,
    /// A rotation key is the zero quaternion.
    zero_quat: bool,
    /// The rotation envelope has no keys.
    empty_track: bool,
    /// The frame rate is 0.
    zero_fps: bool,
    /// The first frame is after the last.
    reversed_range: bool,
    /// The event holder lists an object that does not exist.
    dangling_event: bool,
}

fn types(b: &mut MetaBuilder) {
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(3, "IDENT", 4, 4));
    b.type_block(TypeBlock::primitive(4, "ULONG", 4, 4));
    b.type_block(TypeBlock::alias(5, "BOOL", 4));
    b.type_block(TypeBlock::enumeration(6, "AnimEventFlags", 4));
    b.type_block(TypeBlock::class(
        7,
        "Vector3f",
        1,
        5,
        None,
        &[("1", 2), ("2", 2), ("3", 2)],
    ));
    b.type_block(TypeBlock::class(
        8,
        "Quaternion4f",
        1,
        5,
        None,
        &[("1", 2), ("2", 2), ("3", 2), ("4", 2)],
    ));
    b.type_block(TypeBlock::class(9, "CAnimFrame", 1, 5, None, &[("1", 1)]));
    b.type_block(TypeBlock::class(
        10,
        "CVec3AnimFrame",
        1,
        5,
        Some(9),
        &[("1", 7)],
    ));
    b.type_block(TypeBlock::class(
        11,
        "CQuatAnimFrame",
        1,
        5,
        Some(9),
        &[("1", 8)],
    ));
    b.type_block(TypeBlock::class(
        12,
        "CFloatAnimFrame",
        1,
        5,
        Some(9),
        &[("1", 2)],
    ));
    b.type_block(TypeBlock::short(
        13,
        "CStaticArray<CVec3AnimFrame>",
        0,
        6,
        10,
    ));
    b.type_block(TypeBlock::short(
        14,
        "CStaticArray<CQuatAnimFrame>",
        0,
        6,
        11,
    ));
    b.type_block(TypeBlock::short(
        15,
        "CStaticArray<CFloatAnimFrame>",
        0,
        6,
        12,
    ));
    b.type_block(TypeBlock::class(
        16,
        "CAnimEnvelope",
        1,
        5,
        None,
        &[("1", 3), ("2", 1), ("3", 1), ("4", 1), ("5", 2)],
    ));
    b.type_block(TypeBlock::class(
        17,
        "CVec3LenAnimEnvelope",
        1,
        5,
        Some(16),
        &[("1", 2), ("2", 13)],
    ));
    b.type_block(TypeBlock::class(
        18,
        "CQuatAnimEnvelope",
        1,
        5,
        Some(16),
        &[("1", 14)],
    ));
    b.type_block(TypeBlock::class(
        19,
        "CFloatAnimEnvelope",
        1,
        5,
        Some(16),
        &[("1", 15)],
    ));
    b.type_block(TypeBlock::dynamic_container(
        20,
        "CDynamicContainer<CAnimEnvelope>",
        16,
    ));
    b.type_block(TypeBlock::class(
        21,
        "CAnimEvent",
        1,
        5,
        None,
        &[
            ("ae_fFrame", 2),
            ("ae_ulFlags", 6),
            ("ae_bOnlyInFirstLoop", 5),
        ],
    ));
    b.type_block(TypeBlock::class(
        22,
        "CSamSoundAnimEvent",
        1,
        5,
        Some(21),
        &[("ssae_idSound", 3)],
    ));
    b.type_block(TypeBlock::class(23, "CPlainAnimEvent", 1, 5, Some(21), &[]));
    b.type_block(TypeBlock::pointer(24, "CStaticSound*", 100));
    b.type_block(TypeBlock::class(
        25,
        "CFileAnimEvent",
        1,
        5,
        Some(21),
        &[("fae_pSound", 24), ("fae_fVolume", 2), ("fae_slCount", 0)],
    ));
    b.type_block(TypeBlock::dynamic_container(
        26,
        "CDynamicContainer<CAnimEvent>",
        21,
    ));
    b.type_block(TypeBlock::class(
        27,
        "CAnimEventHolder",
        1,
        5,
        None,
        &[("aeh_caeEvents", 26)],
    ));
    b.type_block(TypeBlock::pointer(28, "CAnimEventHolder*", 27));
    b.type_block(TypeBlock::class(
        29,
        "CAnimation",
        1,
        5,
        None,
        &[
            ("1", 3),
            ("2", 1),
            ("3", 1),
            ("4", 2),
            ("5", 2),
            ("6", 20),
            ("8", 28),
        ],
    ));
    b.type_block(TypeBlock::dynamic_container(
        30,
        "CDynamicContainer<CAnimation>",
        29,
    ));
    b.type_block(TypeBlock::class(
        31,
        "CAnimSet",
        1,
        5,
        None,
        &[("1", 30), ("3", 3)],
    ));
}

fn vec3_key(frame: i32, v: [f32; 3]) -> Vec<u8> {
    ObjectBytes::new()
        .i32(frame)
        .f32(v[0])
        .f32(v[1])
        .f32(v[2])
        .finish()
}

fn quat_key(frame: i32, q: [f32; 4]) -> Vec<u8> {
    ObjectBytes::new()
        .i32(frame)
        .f32(q[0])
        .f32(q[1])
        .f32(q[2])
        .f32(q[3])
        .finish()
}

fn envelope_head(target: u32, first: i32, last: i32, count: i32) -> ObjectBytes {
    ObjectBytes::new()
        .u32(target)
        .i32(first)
        .i32(last)
        .i32(count)
        .f32(1.0 / (last - first + 1) as f32)
}

const SQ: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// One set `Moves` with the animation `Walk` (frames 10 to 14 at 10 fps): translation of bone
/// `Root` (keys at 10, 12, 14), rotation of `Root` (identity at 10, 90 degrees about z at 14),
/// the float channel `Glow` (0 at 10, 1 at 14) and four events.
pub fn file(flaws: Flaws) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let wav = b.file_ref("Content/Test/Sounds/Click.wav", 0);
    b.external_type(100, "CStaticSound");
    types(&mut b);
    let moves = b.identifier("Moves");
    let walk = b.identifier("Walk");
    let root = b.identifier("Root");
    let glow = b.identifier("Glow");
    let step = b.identifier("step_sound");
    b.external_object(50, wav, 1, 100);

    let (first, last) = if flaws.reversed_range {
        (14, 10)
    } else {
        (10, 14)
    };
    b.object(
        0,
        31,
        ObjectBytes::new()
            .dynamic_container(&[1])
            .u32(moves)
            .finish(),
    );
    let fps = if flaws.zero_fps { 0.0 } else { 10.0 };
    b.object(
        1,
        29,
        ObjectBytes::new()
            .u32(walk)
            .i32(first)
            .i32(last)
            .f32(0.1)
            .f32(fps)
            .dynamic_container(&[2, 3, 4])
            .object_ref(Some(5))
            .finish(),
    );

    // Translation: (0,0,0) at 10, (2,0,0) at 12, (2,4,0) at 14.
    let mut keys = vec![
        vec3_key(10, [0.0; 3]),
        vec3_key(12, [2.0, 0.0, 0.0]),
        vec3_key(14, [2.0, 4.0, 0.0]),
    ];
    if flaws.descending {
        keys.swap(1, 2);
    }
    if flaws.key_out_of_range {
        keys[2] = vec3_key(15, [2.0, 4.0, 0.0]);
    }
    let listed = if flaws.count_mismatch { 4 } else { 3 };
    let mut t = envelope_head(root, 10, 14, listed)
        .f32(0.5)
        .array_header(b"STAR", 3);
    for k in &keys {
        t = t.raw(k);
    }
    b.object(2, 17, t.finish());

    // Rotation.
    let zero = if flaws.zero_quat {
        [0.0; 4]
    } else {
        [0.0, 0.0, SQ, SQ]
    };
    let r = if flaws.empty_track {
        envelope_head(root, 10, 14, 0).array_header(b"STAR", 0)
    } else {
        envelope_head(root, 10, 14, 2)
            .array_header(b"STAR", 2)
            .raw(&quat_key(10, [0.0, 0.0, 0.0, 1.0]))
            .raw(&quat_key(14, zero))
    };
    b.object(3, 18, r.finish());

    // Scalar channel.
    b.object(
        4,
        19,
        envelope_head(glow, 10, 14, 2)
            .array_header(b"STAR", 2)
            .i32(10)
            .f32(0.0)
            .i32(14)
            .f32(1.0)
            .finish(),
    );

    // Events: `Early` before the range, a sound at 11, a first-loop-only click at 13 and a file
    // event at 14.
    let holder_ids: &[u32] = if flaws.dangling_event {
        &[6, 99]
    } else {
        &[6, 7, 8, 9]
    };
    b.object(
        5,
        27,
        ObjectBytes::new().dynamic_container(holder_ids).finish(),
    );
    b.object(
        6,
        22,
        ObjectBytes::new()
            .f32(11.0)
            .u32(1)
            .u32(0)
            .u32(step)
            .finish(),
    );
    b.object(7, 23, ObjectBytes::new().f32(13.0).u32(4).u32(1).finish());
    b.object(
        8,
        25,
        ObjectBytes::new()
            .f32(14.0)
            .u32(0)
            .u32(0)
            .object_ref(Some(50))
            .f32(0.5)
            .i32(-3)
            .finish(),
    );
    b.object(9, 23, ObjectBytes::new().f32(9.0).u32(0).u32(0).finish());
    b.build().unwrap()
}
