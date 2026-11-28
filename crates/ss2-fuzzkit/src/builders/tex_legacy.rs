//! The legacy (container version 4 to 7) texture builder of `crates/ss2-format-tex/src/testing.rs` (`tex-legacy.md`).

use flate2::Compression;
use flate2::write::ZlibEncoder;
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
use std::io::Write;

/// `pixels` as one zlib stream.
pub fn zlib(pixels: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(pixels).expect("writing to a Vec");
    z.finish().expect("finish")
}

/// One legacy texture (`tex-legacy.md`): container version 4 to 7 and `CBaseTexture` class
/// layout 5, 6, 7 or 9, each with the member order and types of that layout.
#[derive(Clone, Debug)]
pub struct Legacy {
    /// Container format version, 4 to 7 (4 uses the `tx_*` member names).
    pub version: u32,
    /// The class-layout version (`kind` of `CBaseTexture`): 5, 6, 7 or 9.
    pub layout: u32,
    pub width: i32,
    pub height: i32,
    pub flags: u32,
    /// `.5` frame count (layouts 5, 6, 7; layout 9 has no such member).
    pub frames: i32,
    /// `.6`: `TexQuality` in layout 5, `TexFormat` in the others.
    pub code: u32,
    /// `.17` `TexStoredFormat` (layout 5 only).
    pub stored: u32,
    /// `.8` `CStaticArray<COLOR>` (layout 5 only).
    pub palette: Vec<u32>,
    /// The `.18` bytes exactly as stored (raw pixels, or the zlib framing of the layout).
    pub blob: Vec<u8>,
    /// Write a `CColorTexture` (with some numbered controls) instead of a `CStaticTexture`.
    pub color_texture: bool,
}

impl Legacy {
    /// A layout-5 texture of container version 6 with stored format `stored` and raw `blob`.
    pub fn layout5(width: i32, height: i32, stored: u32, blob: &[u8]) -> Self {
        Self {
            version: 6,
            layout: 5,
            width,
            height,
            flags: 0x21,
            frames: 1,
            code: 2,
            stored,
            palette: Vec::new(),
            blob: blob.to_vec(),
            color_texture: false,
        }
    }

    /// A texture of layout 6, 7 or 9 with `TexFormat` code `code` and the `.18` bytes `blob`.
    pub fn with_format(layout: u32, width: i32, height: i32, code: u32, blob: &[u8]) -> Self {
        Self {
            version: if layout == 9 { 7 } else { 6 },
            layout,
            width,
            height,
            flags: if layout == 9 { 0x21 } else { 0x01 },
            frames: 1,
            code,
            stored: 0,
            palette: Vec::new(),
            blob: blob.to_vec(),
            color_texture: false,
        }
    }

    /// The layout-6 framing of `pixels`: the compressed length as a `u32`, then the stream.
    pub fn layout6_blob(pixels: &[u8]) -> Vec<u8> {
        let z = zlib(pixels);
        let mut blob = (z.len() as u32).to_le_bytes().to_vec();
        blob.extend_from_slice(&z);
        blob
    }

    /// The file.
    pub fn build(&self) -> Vec<u8> {
        let v4 = self.version == 4;
        let name = |numbered: &'static str, alias: &'static str| if v4 { alias } else { numbered };
        let mut b = MetaBuilder::new();
        b.format_version(self.version)
            .version("$Version: Serious Sam 2; 2.016.00:32799; 2004-10-dev$");
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
        b.type_block(TypeBlock::enumeration(16, "TexQuality", 4));
        b.type_block(TypeBlock::enumeration(17, "TexStoredFormat", 4));
        b.type_block(TypeBlock::short(18, "CStaticArray<COLOR>", 0, 6, 4));
        b.type_block(TypeBlock::class(
            10,
            "CResourceFile",
            3,
            5,
            None,
            &[(name("1", "rf_iNextFreeID"), 1)],
        ));
        b.type_block(TypeBlock::pointer(11, "CResourceFile*", 10));
        b.type_block(TypeBlock::class(
            12,
            "CResource",
            4,
            5,
            None,
            &[(name("1", "rs_iID"), 1), (name("2", "rs_prf"), 11)],
        ));
        let (w, h, flags) = (
            name("1", "tx_pixWidth"),
            name("2", "tx_pixHeight"),
            name("4", "tx_ulFlags"),
        );
        let (frames, code, stored) = (
            name("5", "tx_ctFrames"),
            name("6", "tx_txqDefault"),
            name("17", "tx_txfStored"),
        );
        let (blob, palette) = (name("18", "tx_aubFrames"), name("8", "tx_acolFrames"));
        let (wu, wv, border) = (
            name("9", "tx_eWrapU"),
            name("10", "tx_eWrapV"),
            name("14", "tx_colBorder"),
        );
        let members: Vec<(&str, u32)> = match self.layout {
            5 => vec![
                (w, 2),
                (h, 2),
                (flags, 5),
                (frames, 1),
                (code, 16),
                (stored, 17),
                (blob, 9),
                (palette, 18),
                (wu, 7),
                (wv, 7),
                (border, 4),
            ],
            9 => vec![
                (w, 2),
                (h, 2),
                (flags, 5),
                (code, 6),
                (blob, 9),
                (wu, 7),
                (wv, 7),
                (border, 4),
            ],
            _ => vec![
                (w, 2),
                (h, 2),
                (flags, 5),
                (code, 6),
                (frames, 1),
                (blob, 9),
                (wu, 7),
                (wv, 7),
                (border, 4),
            ],
        };
        b.type_block(TypeBlock::class(
            13,
            "CBaseTexture",
            self.layout,
            5,
            Some(12),
            &members,
        ));
        b.type_block(TypeBlock::class(14, "CTexture", 1, 5, Some(13), &[]));
        if self.color_texture {
            b.type_block(TypeBlock::primitive(19, "FLOAT", 4, 4));
            b.type_block(TypeBlock::pointer(20, "Ptr<CTexture>", 14));
            b.type_block(TypeBlock::class(
                15,
                "CColorTexture",
                1,
                5,
                Some(14),
                &[("1", 20), ("2", 1), ("3", 19), ("7", 1)],
            ));
        } else {
            b.type_block(TypeBlock::class(15, "CStaticTexture", 1, 5, Some(14), &[]));
        }
        b.object(0, 10, ObjectBytes::new().i32(1).finish());
        let mut o = ObjectBytes::new()
            .i32(1)
            .object_ref(Some(0))
            .i32(self.width)
            .i32(self.height)
            .u32(self.flags);
        match self.layout {
            5 => {
                o = o
                    .i32(self.frames)
                    .u32(self.code)
                    .u32(self.stored)
                    .array_header(b"STAR", self.blob.len() as u32)
                    .raw(&self.blob)
                    .array_header(b"STAR", self.palette.len() as u32);
                for c in &self.palette {
                    o = o.u32(*c);
                }
            }
            9 => {
                o = o
                    .u32(self.code)
                    .array_header(b"STAR", self.blob.len() as u32)
                    .raw(&self.blob);
            }
            _ => {
                o = o
                    .u32(self.code)
                    .i32(self.frames)
                    .array_header(b"STAR", self.blob.len() as u32)
                    .raw(&self.blob);
            }
        }
        o = o.u32(0x50).u32(0x51).u32(0x0080_8080);
        if self.color_texture {
            o = o.object_ref(Some(2)).i32(0).f32(1.0).i32(256);
        }
        b.object(1, 15, o.finish());
        b.build().expect("a consistent synthetic legacy texture")
    }
}
