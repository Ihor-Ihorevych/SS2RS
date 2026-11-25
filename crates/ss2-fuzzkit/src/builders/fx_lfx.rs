use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    /// A one-part flare mirroring the test vector of `fx.md` (`kleerofski_diamond.lfx`), plus an
    /// optional second part.
    pub fn flare_file(parts: usize) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let tex = b.file_ref("Content/Test/Flare.tex", 0x4_0000);
        b.external_type(100, "CTexture");
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(3, "ULONG", 4, 4));
        b.type_block(TypeBlock::alias(4, "COLOR", 3));
        b.type_block(TypeBlock::enumeration(5, "FlareFlags", 4));
        b.type_block(TypeBlock::short(6, "Ptr<CTexture>", 0, 11, 100));
        b.type_block(TypeBlock::class(
            7,
            "CFlarePart",
            1,
            5,
            None,
            &[
                ("1", 6),
                ("10", 4),
                ("2", 2),
                ("3", 2),
                ("4", 2),
                ("5", 2),
                ("6", 2),
                ("7", 2),
                ("8", 2),
                ("9", 5),
            ],
        ));
        b.type_block(TypeBlock::short(8, "CStaticArray<CFlarePart>", 0, 6, 7));
        b.type_block(TypeBlock::short(
            9,
            "CDynamicContainer<CResource>",
            0,
            8,
            10,
        ));
        b.type_block(TypeBlock::class(10, "CResource", 4, 5, None, &[("1", 1)]));
        b.type_block(TypeBlock::class(
            11,
            "CResourceFile",
            3,
            5,
            None,
            &[("rf_crsShared", 9)],
        ));
        b.type_block(TypeBlock::class(
            12,
            "CFlareConfiguration",
            1,
            5,
            Some(10),
            &[("1", 8)],
        ));
        b.external_object(5, tex, 1, 100);
        b.object(0, 11, ObjectBytes::new().dynamic_container(&[1]).finish());
        let mut o = ObjectBytes::new()
            .i32(1)
            .array_header(b"STAR", parts as u32);
        for i in 0..parts {
            o = o
                .object_ref(Some(5))
                .u32(0xFFFF_FFFF)
                .f32(1.0 - i as f32)
                .f32(0.13)
                .f32(1.0)
                .f32(1.0)
                .f32(0.5)
                .f32(0.05)
                .f32(90.0)
                .u32(1);
        }
        b.object(1, 12, o.finish());
        b.build().unwrap()
    }
