use std::collections::BTreeMap as HashMap;
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// A class declaration: name, base, members `(name, type name)`.
type Decl = (
    &'static str,
    Option<&'static str>,
    &'static [(&'static str, &'static str)],
);

const H: &str = "CGraphHolder";
const DECLS: &[Decl] = &[
    ("FLOAT", None, &[]),
    ("INDEX", None, &[]),
    ("BOOL", None, &[]),
    ("ULONG", None, &[]),
    ("COLOR", None, &[]),
    ("CString", None, &[]),
    ("GfuBlendType", None, &[]),
    ("EnumX", None, &[]),
    ("Vector2f", None, &[("1", "FLOAT"), ("2", "FLOAT")]),
    ("CControlPoint", None, &[("3", "Vector2f"), ("4", "INDEX")]),
    ("CStaticStackArray<CControlPoint>", None, &[]),
    (
        "CGraph",
        None,
        &[
            ("7", "FLOAT"),
            ("8", "FLOAT"),
            ("9", "ULONG"),
            ("10", "INDEX"),
            ("11", "INDEX"),
            ("12", "FLOAT"),
        ],
    ),
    ("CGraph*", None, &[]),
    (
        "CLinearGraph",
        Some("CGraph"),
        &[("1", "FLOAT"), ("2", "CStaticStackArray<CControlPoint>")],
    ),
    (
        H,
        None,
        &[("gh_fValue", "FLOAT"), ("gh_pgrGraph", "CGraph*")],
    ),
    (
        "Box3f",
        None,
        &[
            ("1", "FLOAT"),
            ("2", "FLOAT"),
            ("3", "FLOAT"),
            ("4", "FLOAT"),
            ("5", "FLOAT"),
            ("6", "FLOAT"),
        ],
    ),
    (
        "CRelativePlacement",
        None,
        &[
            ("rp_ghOffsetX", H),
            ("rp_ghOffsetY", H),
            ("rp_ghOffsetZ", H),
            ("rp_ghVelocityX", H),
            ("rp_ghVelocityY", H),
            ("rp_ghVelocityZ", H),
            ("rp_ghHeading", H),
            ("rp_ghPitch", H),
            ("rp_ghBanking", H),
            ("rp_ghWeight", H),
        ],
    ),
    ("CColorKeyFrame", None, &[("1", "FLOAT"), ("2", "COLOR")]),
    ("CStaticStackArray<CColorKeyFrame>", None, &[]),
    (
        "CGradient",
        None,
        &[("2", "CStaticStackArray<CColorKeyFrame>")],
    ),
    ("CTexture*", None, &[]),
    (
        "CParticleEffectTexture",
        None,
        &[
            ("1", "CTexture*"),
            ("2", "INDEX"),
            ("3", "INDEX"),
            ("4", "FLOAT"),
            ("5", "BOOL"),
        ],
    ),
    (
        "CParticlesRendering",
        None,
        &[
            ("1", "GfuBlendType"),
            ("3", "BOOL"),
            ("4", "FLOAT"),
            ("5", "EnumX"),
        ],
    ),
    (
        "CEmitterShape",
        None,
        &[
            ("es_ghPivotOffsetX", H),
            ("es_ghPivotOffsetY", H),
            ("es_ghPivotOffsetZ", H),
            ("4", "EnumX"),
            ("es_ghOverrideRatio", H),
        ],
    ),
    ("CEmitterShape*", None, &[]),
    (
        "CSphereEmitterShape",
        Some("CEmitterShape"),
        &[
            ("ses_ghEmitPosHMin", H),
            ("ses_ghEmitPosHMax", H),
            ("ses_ghEmitPosPMin", H),
            ("ses_ghEmitPosPMax", H),
            ("ses_ghEmitPosRMin", H),
            ("ses_ghEmitPosRMax", H),
        ],
    ),
    (
        "CBoxEmitterShape",
        Some("CEmitterShape"),
        &[("bes_ghBoxW", H), ("bes_ghBoxH", H), ("bes_ghBoxL", H)],
    ),
    ("CMysteryShape", Some("CEmitterShape"), &[]),
    (
        "CMachineParticleEffectLife",
        None,
        &[
            ("21", "CEmitterShape*"),
            ("mpel_ghEmittingFreq", H),
            ("mpel_ghEmittedParticleLifeMin", H),
            ("mpel_ghEmittedParticleLifeMax", H),
            ("mpel_ghEmitVelocityMin", H),
            ("mpel_ghEmitVelocityMax", H),
            ("mpel_ghLaunchSpinMin", H),
            ("mpel_ghLaunchSpinMax", H),
            ("mpel_ghBirthSizeMin", H),
            ("mpel_ghBirthSizeMax", H),
            ("mpel_ghEmitDirHMin", H),
            ("mpel_ghEmitDirHMax", H),
            ("mpel_ghEmitDirPMin", H),
            ("mpel_ghEmitDirPMax", H),
            ("20", "BOOL"),
        ],
    ),
    (
        "CParticleLife",
        None,
        &[
            ("1", "CGradient"),
            ("3", "CGradient"),
            ("14", "FLOAT"),
            ("2", "BOOL"),
            ("pl_ghSizeMultiplier", H),
            ("pl_ghVelocityMultiplier", H),
            ("pl_ghVelocityDamping", H),
            ("pl_ghWeightMin", H),
            ("pl_ghWeightMax", H),
            ("pl_ghSpinMultiplier", H),
            ("pl_ghSpinDamping", H),
            ("pl_ghHModifier", H),
            ("pl_ghPModifier", H),
            ("pl_ghBModifier", H),
            ("pl_ghRising", H),
        ],
    ),
    (
        "CParticlesCollision",
        None,
        &[("1", "BOOL"), ("2", "FLOAT"), ("3", "FLOAT")],
    ),
    (
        "CParticleEffect",
        None,
        &[
            ("1", "CString"),
            ("2", "BOOL"),
            ("11", "EnumX"),
            ("8", "FLOAT"),
            ("3", "CParticleEffectTexture"),
            ("4", "CParticlesRendering"),
            ("5", "INDEX"),
            ("6", "CRelativePlacement"),
            ("7", "EnumX"),
            ("9", "BOOL"),
            ("10", "BOOL"),
        ],
    ),
    (
        "CMachineParticleEffect",
        Some("CParticleEffect"),
        &[
            ("1", "CMachineParticleEffectLife"),
            ("2", "CParticleLife"),
            ("3", "CParticlesCollision"),
            ("4", "FLOAT"),
            ("5", "EnumX"),
            ("6", "BOOL"),
        ],
    ),
    (
        "CAtomicParticleEffect",
        Some("CParticleEffect"),
        &[
            ("1", "INDEX"),
            ("2", "INDEX"),
            ("3", "FLOAT"),
            ("5", "CGradient"),
            ("6", "CGradient"),
            ("ape_ghTrailWidth", H),
            ("8", "FLOAT"),
            ("9", "FLOAT"),
            ("10", "FLOAT"),
            ("11", "FLOAT"),
        ],
    ),
    ("CParticleEffect*", None, &[]),
    ("CDynamicContainer<CParticleEffect>", None, &[]),
    ("CResource", None, &[("1", "INDEX")]),
    ("CDynamicContainer<CResource>", None, &[]),
    (
        "CResourceFile",
        None,
        &[("rf_crsShared", "CDynamicContainer<CResource>")],
    ),
    (
        "CEffect",
        Some("CResource"),
        &[
            ("12", "EnumX"),
            ("1", "FLOAT"),
            ("ef_boxInitialBoundingBox", "Box3f"),
            ("3", "INDEX"),
            ("10", "FLOAT"),
            ("11", "FLOAT"),
            ("ef_ghStretchOverLife", H),
            ("ef_ghStretchOverDistance", H),
            ("6", "CRelativePlacement"),
            ("5", "CDynamicContainer<CParticleEffect>"),
            ("21", "FLOAT"),
        ],
    ),
];

/// Build the type table: each declaration's index is its type id. Names ending in `*`, and the
/// `CDynamicContainer<..>` and `CStaticStackArray<..>` forms, become pointers and containers.
fn types(b: &mut MetaBuilder) -> HashMap<&'static str, u32> {
    let id: HashMap<&str, u32> = DECLS
        .iter()
        .enumerate()
        .map(|(i, d)| (d.0, i as u32))
        .collect();
    b.external_type(500, "CTexture");
    for (i, (name, base, members)) in DECLS.iter().enumerate() {
        let i = i as u32;
        let block = if let Some(inner) = name.strip_suffix('*') {
            if inner == "CTexture" {
                TypeBlock::short(i, name, 0, 11, 500)
            } else {
                TypeBlock::pointer(i, name, id[inner])
            }
        } else if let Some(inner) = name
            .strip_prefix("CDynamicContainer<")
            .and_then(|s| s.strip_suffix('>'))
        {
            TypeBlock::dynamic_container(i, name, id[inner])
        } else if let Some(inner) = name
            .strip_prefix("CStaticStackArray<")
            .and_then(|s| s.strip_suffix('>'))
        {
            TypeBlock::short(i, name, 0, 7, id[inner])
        } else if members.is_empty() && base.is_none() && !name.starts_with("CMystery") {
            match *name {
                "FLOAT" | "ULONG" => TypeBlock::primitive(i, name, 4, 4),
                "INDEX" => TypeBlock::alias(i, name, id["ULONG"]),
                "BOOL" | "COLOR" => TypeBlock::alias(i, name, id["ULONG"]),
                "CString" => TypeBlock::primitive(i, name, 0, 1),
                _ => TypeBlock::enumeration(i, name, 4),
            }
        } else {
            let members: Vec<(&str, u32)> = members.iter().map(|(n, t)| (*n, id[t])).collect();
            TypeBlock::class(i, name, 1, 5, base.map(|b| id[b]), &members)
        };
        b.type_block(block);
    }
    id
}

fn holder(value: f32, graph: Option<u32>) -> ObjectBytes {
    ObjectBytes::new().f32(value).object_ref(graph)
}

fn holders(o: ObjectBytes, n: usize, value: f32) -> ObjectBytes {
    (0..n).fold(o, |o, _| o.raw(&holder(value, None).finish()))
}

fn gradient(color: u32) -> ObjectBytes {
    ObjectBytes::new()
        .array_header(b"SSAR", 1)
        .f32(0.0)
        .u32(color)
}

/// A `.pfx` with one machine effect (the emitter object is `emitter_class`) and one atomic effect.
/// Object ids: 0 file, 1 `CEffect`, 2 machine, 3 emitter, 4 linear graph, 5 atomic.
pub fn pfx_file(emitter_class: &str) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let tex = b.file_ref("Content/Test/Spark.tex", 0x4_0000);
    let id = types(&mut b);
    b.external_object(9, tex, 1, 500);
    b.object(
        0,
        id["CResourceFile"],
        ObjectBytes::new().dynamic_container(&[1]).finish(),
    );
    let placement = || holders(ObjectBytes::new(), 10, 0.0);
    let texture = |cols: u32| {
        ObjectBytes::new()
            .object_ref(Some(9))
            .u32(cols)
            .u32(1)
            .f32(0.07)
            .u32(1)
    };
    let rendering = ObjectBytes::new().u32(501).u32(0).f32(0.5).u32(0);
    // CEffect: base CResource.1, then its own members in declared order.
    let effect = ObjectBytes::new()
        .i32(1)
        .u32(2)
        .f32(5.0)
        .f32(-0.25)
        .f32(-0.25)
        .f32(-0.25)
        .f32(0.25)
        .f32(0.25)
        .f32(0.25)
        .u32(500)
        .f32(300.0)
        .f32(500.0)
        .raw(&holder(1.0, None).finish())
        .raw(&holder(1.0, None).finish())
        .raw(&placement().finish())
        .dynamic_container(&[2, 5])
        .f32(0.0);
    b.object(1, id["CEffect"], effect.finish());
    // Machine effect: CParticleEffect members, then its own.
    let mut m = ObjectBytes::new()
        .cstring("sparks")
        .u32(0)
        .u32(1)
        .f32(0.0)
        .raw(&texture(2).finish())
        .raw(&rendering.clone().finish())
        .u32(10_000)
        .raw(&placement().finish())
        .u32(8)
        .u32(0)
        .u32(0);
    // CMachineParticleEffectLife: the emitter pointer, 13 holders, a flag.
    m = m.object_ref(Some(3)).raw(&holder(25.0, Some(4)).finish());
    m = holders(m, 12, 1.0).u32(0);
    // CParticleLife: two gradients, a float, a flag, 11 holders.
    m = m
        .raw(&gradient(0xFF85_8585).finish())
        .raw(&gradient(0xFF00_0000).finish())
        .f32(2.0)
        .u32(1);
    m = holders(m, 11, 1.0);
    // Collision, member 4, shape, flag.
    m = m.u32(1).f32(0.3).f32(0.5).f32(1.5).u32(2).u32(0);
    b.object(2, id["CMachineParticleEffect"], m.finish());
    // Emitter: CEmitterShape members, then the class's own holders.
    let shape = |own: usize| {
        let o = holders(ObjectBytes::new(), 3, 0.0)
            .u32(1)
            .raw(&holder(0.0, None).finish());
        holders(o, own, 2.0)
    };
    match emitter_class {
        "CSphereEmitterShape" => b.object(3, id[emitter_class], shape(6).finish()),
        "CBoxEmitterShape" => b.object(3, id[emitter_class], shape(3).finish()),
        _ => b.object(3, id[emitter_class], shape(0).finish()),
    };
    b.object(
        4,
        id["CLinearGraph"],
        ObjectBytes::new()
            .f32(1.0)
            .f32(1.0)
            .u32(3)
            .i32(137)
            .i32(2)
            .f32(0.0)
            .f32(1.0)
            .array_header(b"SSAR", 2)
            .f32(0.0)
            .f32(0.0)
            .u32(0xCDCD_CDCD)
            .f32(1.0)
            .f32(1.0)
            .u32(0)
            .finish(),
    );
    // An atomic effect like the flies of the spec's test vector.
    let a = ObjectBytes::new()
        .cstring("")
        .u32(1)
        .u32(1)
        .f32(0.0)
        .raw(&texture(1).finish())
        .raw(&rendering.finish())
        .u32(10_000)
        .raw(&placement().finish())
        .u32(8)
        .u32(0)
        .u32(0)
        .i32(3)
        .i32(2)
        .f32(1.311)
        .raw(&gradient(0xFF85_8585).finish())
        .raw(&gradient(0xFF00_0000).finish())
        .raw(&holder(0.6, None).finish())
        .f32(0.0)
        .f32(4.0)
        .f32(1.5)
        .f32(0.5);
    b.object(5, id["CAtomicParticleEffect"], a.finish());
    b.build().unwrap()
}
