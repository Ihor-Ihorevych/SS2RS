use super::fx_vfx::{
    T_FLOAT, T_GRAPH_PTR, T_INDEX, T_RESOURCE, T_RESOURCE_FILE, graph_base, graph_types, points,
};
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// A configuration with one move effect, one size letter effect, a font and the 640 x 480
/// virtual-screen placement. Type ids: 19 `CFontInstance`, 20 `CTextEffect`, 21 move effect,
/// 22 `CLetterEffect`, 23 size letter effect, 24 `CTextEffectConfiguration`, 25 `CStepUpGraph`.
pub fn text_file(unknown_effect: bool) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let font = b.file_ref("Content/Test/Font.fnt", 0);
    let empty = b.identifier("empty");
    b.external_type(100, "CFont");
    graph_types(&mut b);
    b.type_block(TypeBlock::short(14, "Ptr<CFont>", 0, 11, 100));
    b.type_block(TypeBlock::alias(35, "PIX", 0));
    b.type_block(TypeBlock::primitive(15, "IDENT", 4, 4));
    b.type_block(TypeBlock::enumeration(16, "FontFlags", 4));
    b.type_block(TypeBlock::pointer(26, "CTextEffect*", 20));
    b.type_block(TypeBlock::pointer(27, "CLetterEffect*", 22));
    b.type_block(TypeBlock::short(
        28,
        "CDynamicContainer<CTextEffect>",
        0,
        8,
        20,
    ));
    b.type_block(TypeBlock::short(
        29,
        "CDynamicContainer<CLetterEffect>",
        0,
        8,
        22,
    ));
    b.type_block(TypeBlock::enumeration(30, "TextAlignmentH", 4));
    b.type_block(TypeBlock::enumeration(31, "TextAlignmentV", 4));
    b.type_block(TypeBlock::enumeration(32, "GfuBlendType", 4));
    b.type_block(TypeBlock::enumeration(33, "TextSide", 4));
    b.type_block(TypeBlock::class(
        19,
        "CFontInstance",
        1,
        5,
        None,
        &[
            ("1", 14),
            ("2", 8),
            ("3", 16),
            ("4", T_FLOAT),
            ("5", T_FLOAT),
            ("6", T_FLOAT),
            ("7", T_FLOAT),
            ("8", 35),
            ("9", T_FLOAT),
            ("10", 8),
            ("11", T_FLOAT),
            ("12", T_FLOAT),
            ("13", T_FLOAT),
            ("14", T_FLOAT),
            ("16", 0),
            ("15", 15),
        ],
    ));
    b.type_block(TypeBlock::class(
        20,
        "CTextEffect",
        1,
        5,
        None,
        &[
            ("5", 7),
            ("1", T_GRAPH_PTR),
            ("2", T_GRAPH_PTR),
            ("3", T_GRAPH_PTR),
            ("4", T_GRAPH_PTR),
        ],
    ));
    b.type_block(TypeBlock::class(
        21,
        "CMoveTextEffect",
        1,
        5,
        Some(20),
        &[("1", 4)],
    ));
    b.type_block(TypeBlock::class(
        22,
        "CLetterEffect",
        1,
        5,
        None,
        &[
            ("6", 7),
            ("1", T_GRAPH_PTR),
            ("2", T_GRAPH_PTR),
            ("3", T_GRAPH_PTR),
            ("4", T_GRAPH_PTR),
            ("5", T_FLOAT),
        ],
    ));
    b.type_block(TypeBlock::class(
        23,
        "CSizeLetterTextEffect",
        1,
        5,
        Some(22),
        &[
            ("1", T_FLOAT),
            ("2", T_FLOAT),
            ("3", T_FLOAT),
            ("4", T_FLOAT),
        ],
    ));
    b.type_block(TypeBlock::class(
        24,
        "CTextEffectConfiguration",
        1,
        5,
        Some(T_RESOURCE),
        &[
            ("1", 28),
            ("2", 29),
            ("3", T_FLOAT),
            ("4", T_FLOAT),
            ("5", T_FLOAT),
            ("6", 4),
            ("14", 4),
            ("7", 19),
            ("8", 30),
            ("9", 31),
            ("10", 32),
            ("12", 7),
            ("13", T_FLOAT),
            ("15", 7),
            ("20", 8),
        ],
    ));
    b.type_block(TypeBlock::class(
        25,
        "CStepUpGraph",
        1,
        5,
        Some(13),
        &[("1", 6)],
    ));
    // A class a `.tfx` must not contain.
    b.type_block(TypeBlock::class(
        34,
        "CMysteryTextEffect",
        1,
        5,
        Some(20),
        &[],
    ));
    b.external_object(9, font, 1, 100);
    b.object(
        0,
        T_RESOURCE_FILE,
        ObjectBytes::new().dynamic_container(&[1]).finish(),
    );
    let vec2 = |x: f32, y: f32| ObjectBytes::new().f32(x).f32(y).finish();
    let font_bytes = ObjectBytes::new()
        .object_ref(Some(9))
        .u32(0xFFFF_FFFF)
        .u32(1)
        .f32(0.0)
        .f32(0.0)
        .f32(0.3)
        .f32(0.75)
        .i32(90)
        .f32(1.0)
        .u32(0x7800_0000)
        .f32(0.0)
        .f32(1.0)
        .f32(1.0)
        .f32(0.0)
        .i32(0)
        .u32(empty)
        .finish();
    b.object(
        1,
        24,
        ObjectBytes::new()
            .i32(1)
            .dynamic_container(&[2])
            .dynamic_container(&[3])
            .f32(0.5)
            .f32(3.0)
            .f32(0.5)
            .raw(&vec2(320.0, 358.0))
            .raw(&vec2(640.0, 480.0))
            .raw(&font_bytes)
            .u32(1)
            .u32(1)
            .u32(501)
            .u32(0)
            .f32(0.5)
            .u32(1)
            .u32(0)
            .finish(),
    );
    b.object(
        2,
        if unknown_effect { 34 } else { 21 },
        {
            let o = ObjectBytes::new()
                .u32(1)
                .object_ref(Some(4))
                .object_ref(Some(5))
                .object_ref(Some(4))
                .object_ref(Some(5));
            if unknown_effect {
                o
            } else {
                o.raw(&vec2(80.0, 80.0))
            }
        }
        .finish(),
    );
    b.object(
        3,
        23,
        ObjectBytes::new()
            .u32(1)
            .object_ref(Some(5))
            .object_ref(Some(5))
            .object_ref(Some(4))
            .object_ref(Some(5))
            .f32(0.02)
            .f32(0.9)
            .f32(1.0)
            .f32(0.9)
            .f32(1.0)
            .finish(),
    );
    b.object(
        4,
        25,
        graph_base(0)
            .raw(&points(&[(0.0, 0.0), (1.0, 1.0)]).finish())
            .finish(),
    );
    b.object(
        5,
        super::fx_vfx::T_LINEAR,
        graph_base(3)
            .f32(1.0)
            .raw(&points(&[(0.0, 1.0), (1.0, 1.0)]).finish())
            .finish(),
    );
    let _ = T_INDEX;
    b.build().unwrap()
}
