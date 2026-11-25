use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    /// A glyph record as the file stores it: `(x, y, w, h, x offset, y offset, advance, reserved)`.
    pub type G = (i32, i32, i32, i32, i32, i32, i32, i32);

    /// A version-8 `.fnt` with a `width` x `height` 32-bit atlas, the glyphs and the code table.
    pub fn font_file(
        width: i32,
        height: i32,
        line: i32,
        glyphs: &[G],
        table: &[i32],
    ) -> Vec<u8> {
        let pixels = vec![0x80u8; (width * height * 4) as usize];
        let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
        z.write_all(&pixels).expect("writing to a Vec");
        let blob = z.finish().expect("finish");

        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::alias(2, "PIX", 0));
        b.type_block(TypeBlock::primitive(3, "ULONG", 4, 4));
        b.type_block(TypeBlock::alias(4, "COLOR", 3));
        b.type_block(TypeBlock::enumeration(5, "TexFlags", 4));
        b.type_block(TypeBlock::enumeration(6, "TexFormat", 4));
        b.type_block(TypeBlock::enumeration(7, "GfxTexWrap", 4));
        b.type_block(TypeBlock::primitive(8, "UBYTE", 1, 1));
        b.type_block(TypeBlock::short(9, "CStaticArray<UBYTE>", 0, 6, 8));
        b.type_block(TypeBlock::class(
            10,
            "CResourceFile",
            3,
            5,
            None,
            &[("1", 1)],
        ));
        b.type_block(TypeBlock::pointer(11, "CResourceFile*", 10));
        b.type_block(TypeBlock::class(
            12,
            "CResource",
            4,
            5,
            None,
            &[("1", 1), ("2", 11)],
        ));
        b.type_block(TypeBlock::class(
            13,
            "CBaseTexture",
            9,
            5,
            Some(12),
            &[
                ("1", 2),
                ("2", 2),
                ("4", 5),
                ("6", 6),
                ("18", 9),
                ("9", 7),
                ("10", 7),
                ("14", 4),
            ],
        ));
        b.type_block(TypeBlock::class(14, "CTexture", 1, 5, Some(13), &[]));
        b.type_block(TypeBlock::class(15, "CStaticTexture", 1, 5, Some(14), &[]));
        b.type_block(TypeBlock::class(
            16,
            "CFontCharacter",
            1,
            5,
            None,
            &[
                ("1", 0),
                ("2", 0),
                ("3", 0),
                ("4", 0),
                ("5", 0),
                ("6", 0),
                ("7", 0),
                ("8", 0),
            ],
        ));
        b.type_block(TypeBlock::short(
            17,
            "CStaticArray<CFontCharacter>",
            0,
            6,
            16,
        ));
        b.type_block(TypeBlock::short(18, "CStaticArray<SLONG>", 0, 6, 0));
        b.type_block(TypeBlock::pointer(19, "CStaticTexture*", 15));
        b.type_block(TypeBlock::class(
            20,
            "CFont",
            1,
            5,
            Some(12),
            &[
                ("1", 0),
                ("2", 0),
                ("3", 0),
                ("4", 0),
                ("5", 19),
                ("6", 17),
                ("7", 18),
            ],
        ));
        b.object(0, 10, ObjectBytes::new().i32(1).finish());
        let mut font = ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .i32(line - 1) // .1
            .i32(7) // .2
            .i32(line) // .3
            .i32(width) // .4
            .object_ref(Some(2)) // .5
            .array_header(b"STAR", glyphs.len() as u32);
        for g in glyphs {
            font = font
                .i32(g.0)
                .i32(g.1)
                .i32(g.2)
                .i32(g.3)
                .i32(g.4)
                .i32(g.5)
                .i32(g.6)
                .i32(g.7);
        }
        font = font.array_header(b"STAR", table.len() as u32);
        for &t in table {
            font = font.i32(t);
        }
        b.object(1, 20, font.finish());
        b.object(
            2,
            15,
            ObjectBytes::new()
                .i32(1)
                .object_ref(Some(0))
                .i32(width)
                .i32(height)
                .u32(0x24)
                .u32(1) // format code 1: 32 bit
                .array_header(b"STAR", blob.len() as u32)
                .raw(&blob)
                .u32(0x51)
                .u32(0x51)
                .u32(0)
                .finish(),
        );
        b.build().expect("a consistent synthetic font")
    }

    /// A small proportional font: glyph 0 is a box, 1 is the space, 2 is `A` (with a negative
    /// x offset), 3 is `B`. Codes 32, 65, 66 are mapped; the table ends at 66.
    pub fn sample() -> Vec<u8> {
        let glyphs: [G; 4] = [
            (0, 0, 4, 8, 1, 0, 5, 0),
            (4, 0, 1, 8, 0, 0, 3, 0),
            (5, 0, 6, 8, -2, 0, 6, 0),
            (11, 0, 5, 8, 1, 0, 7, 0),
        ];
        let mut table = vec![0i32; 67];
        table[32] = 1;
        table[65] = 2;
        table[66] = 3;
        font_file(16, 8, 9, &glyphs, &table)
    }
