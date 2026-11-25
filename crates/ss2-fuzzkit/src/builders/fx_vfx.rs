use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    /// Type ids of the graph classes in [`graph_types`].
    pub const T_RESOURCE_FILE: u32 = 11;
    pub const T_RESOURCE: u32 = 10;
    pub const T_RESOURCE_LIST: u32 = 9;
    pub const T_GRAPH_PTR: u32 = 12;
    pub const T_LINEAR: u32 = 17;
    pub const T_SINE: u32 = 18;
    pub const T_FLOAT: u32 = 2;
    pub const T_INDEX: u32 = 1;

    /// Types 0 to 18: primitives, `CResource`, `CResourceFile`, `CGraph` and its linear and sine
    /// classes (as in `fx.md`/`rsc-ctl.md`).
    pub fn graph_types(b: &mut MetaBuilder) {
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(3, "ULONG", 4, 4));
        b.type_block(TypeBlock::class(
            4,
            "Vector2f",
            1,
            5,
            None,
            &[("1", 2), ("2", 2)],
        ));
        b.type_block(TypeBlock::class(
            5,
            "CControlPoint",
            1,
            5,
            None,
            &[("3", 4), ("4", 1)],
        ));
        b.type_block(TypeBlock::short(
            6,
            "CStaticStackArray<CControlPoint>",
            0,
            7,
            5,
        ));
        b.type_block(TypeBlock::primitive(7, "BOOL", 4, 4));
        b.type_block(TypeBlock::primitive(8, "COLOR", 4, 4));
        b.type_block(TypeBlock::class(
            13,
            "CGraph",
            1,
            5,
            None,
            &[
                ("7", 2),
                ("8", 2),
                ("9", 3),
                ("10", 1),
                ("11", 1),
                ("12", 2),
            ],
        ));
        b.type_block(TypeBlock::short(
            T_RESOURCE_LIST,
            "CDynamicContainer<CResource>",
            0,
            8,
            T_RESOURCE,
        ));
        b.type_block(TypeBlock::class(
            T_RESOURCE,
            "CResource",
            4,
            5,
            None,
            &[("1", T_INDEX)],
        ));
        b.type_block(TypeBlock::class(
            T_RESOURCE_FILE,
            "CResourceFile",
            3,
            5,
            None,
            &[("rf_crsShared", T_RESOURCE_LIST)],
        ));
        b.type_block(TypeBlock::pointer(T_GRAPH_PTR, "CGraph*", 13));
        b.type_block(TypeBlock::class(
            T_LINEAR,
            "CLinearGraph",
            1,
            5,
            Some(13),
            &[("1", 2), ("2", 6)],
        ));
        b.type_block(TypeBlock::class(
            T_SINE,
            "CSineGraph",
            1,
            5,
            Some(13),
            &[("1", 6)],
        ));
    }

    /// The members of a `CGraph` base (`.7` to `.12`) for a linear (`kind_code` 3) graph.
    pub fn graph_base(kind_code: u32) -> ObjectBytes {
        ObjectBytes::new()
            .f32(1.0)
            .f32(1.0)
            .u32(kind_code)
            .i32(137)
            .i32(2)
            .f32(0.0)
    }

    /// Two control points as stored: `(x, y)` and the garbage index.
    pub fn points(pts: &[(f32, f32)]) -> ObjectBytes {
        let mut o = ObjectBytes::new().array_header(b"SSAR", pts.len() as u32);
        for (i, (x, y)) in pts.iter().enumerate() {
            o = o.f32(*x).f32(*y).u32(if i == 0 { 0xCDCD_CDCD } else { 0 });
        }
        o
    }

    /// `bothfreqmax`-like file: low motor a linear ramp (object 2), high motor a sine (object 3),
    /// duration 5.
    pub fn vibro_file(low: bool, high: bool) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        graph_types(&mut b);
        b.type_block(TypeBlock::class(
            19,
            "CXboxVibroFX",
            1,
            5,
            Some(T_RESOURCE),
            &[("4", T_GRAPH_PTR), ("5", T_GRAPH_PTR), ("6", T_FLOAT)],
        ));
        b.object(
            0,
            T_RESOURCE_FILE,
            ObjectBytes::new().dynamic_container(&[1]).finish(),
        );
        b.object(
            1,
            19,
            ObjectBytes::new()
                .i32(1)
                .object_ref(low.then_some(2))
                .object_ref(high.then_some(3))
                .f32(5.0)
                .finish(),
        );
        b.object(
            2,
            T_LINEAR,
            graph_base(3)
                .f32(1.0)
                .raw(&points(&[(0.0, 0.0), (1.0, 1.0)]).finish())
                .finish(),
        );
        b.object(
            3,
            T_SINE,
            graph_base(4)
                .raw(&points(&[(0.0, 0.5), (1.0, 1.0)]).finish())
                .finish(),
        );
        b.build().unwrap()
    }
