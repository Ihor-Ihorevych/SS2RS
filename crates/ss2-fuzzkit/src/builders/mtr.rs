use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    /// A file with one preset: one specular config whose arguments hold a colour, a texture slot
    /// with its UV map, a float, the common blend type, and one modifier.
    pub fn material_file(texture_external: bool, shader_flags: u32) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let module = b.file_ref("Bin/Shaders.module", 0);
        let tex = b.file_ref("Content/Test/Textures/body.tex", 0);
        b.external_type(100, "CShader");
        b.external_type(101, "CStaticTexture");
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
        b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
        b.type_block(TypeBlock::enumeration(4, "EPlatform", 4));
        b.type_block(TypeBlock::enumeration(5, "GfuBlendType", 4));
        b.type_block(TypeBlock::class(
            6,
            "CResourceFile",
            3,
            5,
            None,
            &[("1", 1)],
        ));
        b.type_block(TypeBlock::pointer(7, "CResourceFile*", 6));
        b.type_block(TypeBlock::class(
            8,
            "CResource",
            4,
            5,
            None,
            &[("1", 1), ("2", 7)],
        ));
        // Wrappers.
        b.type_block(TypeBlock::pointer(9, "CShader*", 100));
        b.type_block(TypeBlock::pointer(10, "CStaticTexture*", 101));
        b.type_block(TypeBlock::class(
            11,
            "CShaderArgValCOLOR",
            1,
            5,
            None,
            &[("1", 2)],
        ));
        b.type_block(TypeBlock::class(
            12,
            "CShaderArgValFLOAT",
            1,
            5,
            None,
            &[("1", 3)],
        ));
        b.type_block(TypeBlock::class(
            13,
            "CShaderArgValTexture",
            1,
            5,
            None,
            &[("1", 10)],
        ));
        b.type_block(TypeBlock::class(
            14,
            "CShaderArgValUVMap",
            1,
            5,
            None,
            &[("1", 2)],
        ));
        b.type_block(TypeBlock::class(
            15,
            "CShaderArgValBlendType",
            1,
            5,
            None,
            &[("1", 5)],
        ));
        // Modifier list and the argument classes.
        b.type_block(TypeBlock::class(
            16,
            "CShaderModifier",
            1,
            5,
            None,
            &[("1", 4)],
        ));
        b.type_block(TypeBlock::class(
            17,
            "CConstantShaderModifier",
            1,
            5,
            Some(16),
            &[],
        ));
        b.type_block(TypeBlock::pointer(18, "CShaderModifier*", 16));
        b.type_block(TypeBlock::short(
            19,
            "CDynamicContainer<CShaderModifier>",
            0,
            8,
            16,
        ));
        b.type_block(TypeBlock::class(
            20,
            "CShaderModifierList",
            1,
            5,
            None,
            &[("1", 19)],
        ));
        b.type_block(TypeBlock::class(
            21,
            "CShaderArgs",
            1,
            5,
            None,
            &[("1", 20)],
        ));
        b.type_block(TypeBlock::class(
            22,
            "CShaderCommonParams",
            1,
            5,
            Some(21),
            &[("1", 15)],
        ));
        b.type_block(TypeBlock::class(
            23,
            "CSpecularShaderArgs",
            1,
            5,
            Some(22),
            &[("9", 11), ("21", 13), ("22", 14), ("8", 12)],
        ));
        b.type_block(TypeBlock::pointer(24, "CSpecularShaderArgs*", 23));
        b.type_block(TypeBlock::class(
            25,
            "CShaderConfig",
            1,
            5,
            None,
            &[("1", 9), ("2", 24), ("3", 4)],
        ));
        b.type_block(TypeBlock::short(
            26,
            "CStaticArray<CShaderConfig>",
            0,
            6,
            25,
        ));
        b.type_block(TypeBlock::pointer(27, "CResource*", 8));
        b.type_block(TypeBlock::class(
            28,
            "CShaderPreset",
            1,
            5,
            Some(8),
            &[("1", 26), ("2", 27), ("3", 0)],
        ));
        b.external_object(2, module, shader_flags, 100);
        b.external_object(4, tex, 1, 101);

        b.object(0, 6, ObjectBytes::new().i32(1).finish());
        b.object(
            1,
            28,
            ObjectBytes::new()
                .i32(1)
                .object_ref(Some(0))
                .array_header(b"STAR", 1)
                .object_ref(Some(2))
                .object_ref(Some(3))
                .u32(0x3)
                .object_ref(None)
                .i32(255)
                .finish(),
        );
        // Object 3: the arguments (base members first: CShaderArgs, CShaderCommonParams, own).
        let texture_ref = if texture_external { Some(4) } else { Some(9) };
        b.object(
            3,
            23,
            ObjectBytes::new()
                .raw(&ObjectBytes::new().dynamic_container(&[5]).finish())
                .u32(0x1F4)
                .u32(0xFF7F7F7F)
                .object_ref(texture_ref)
                .u32(1)
                .f32(30.0)
                .finish(),
        );
        b.object(5, 17, ObjectBytes::new().u32(0x3).finish());
        b.build().unwrap()
    }
