//! `.lfx` lens flare configurations (spec `docs/re/fx.md`).
//!
//! A [`FlareConfiguration`] (class `CFlareConfiguration`) is a static array of [`FlarePart`]s:
//! textured quads laid out along the line from the light to the screen centre. 191 parts in 37
//! retail files. The meaning of the part members is **medium** confidence (taken from the order
//! of the editor help and the value ranges); units and formulas are open.

use ss2_core::Result;
use ss2_meta::ObjectRef;

use crate::read::Ctx;

const WHAT: &str = ".lfx flare configuration";

/// One flare part (`CFlarePart`, 40 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct FlarePart {
    /// `.1`: the flare image, a `.tex` path as listed in the file's `RFIL`.
    pub texture: String,
    /// `.10`: multiplicative tint (alpha byte `0xFF` in all retail parts; channel order open).
    pub color: u32,
    /// `.2`: offset along the light-to-screen-centre axis: 1 at the light, 0 at the centre,
    /// negative beyond the centre, above 1 past the light (-2.0 to 1.061).
    pub offset: f32,
    /// `.3`: size relative to the screen (0.012 to 0.961).
    pub size: f32,
    /// `.4`: aspect of the quad (1 = square; 2.5 for the window flare).
    pub aspect: f32,
    /// `.5`: light amplification (brightness multiplier, 0.126 to 6.271).
    pub amplification: f32,
    /// `.6`: light desaturation (0 to 1).
    pub desaturation: f32,
    /// `.7`: falloff factor (0.05 to 8.96).
    pub falloff: f32,
    /// `.8`: rotation factor in degrees (90 in most parts).
    pub rotation: f32,
    /// `.9` `FlareFlags`: bit 0 fade by size, bit 1 fade by intensity; other bits not seen.
    pub flags: u32,
}

/// A decoded `.lfx` file.
#[derive(Clone, Debug, PartialEq)]
pub struct FlareConfiguration {
    /// The parts in file order (1 to 11; at least one).
    pub parts: Vec<FlarePart>,
}

impl FlareConfiguration {
    /// Read a `.lfx` file.
    ///
    /// # Errors
    /// [`ss2_core::Error::Invalid`] naming what is wrong (also an empty part list or a texture
    /// pointer that is not an external file, spec invariant 2).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = Ctx::decode(bytes, WHAT)?;
        let ctx = Ctx {
            container: &container,
            decoded: &decoded,
            what: WHAT,
        };
        let root = ctx.root("CFlareConfiguration")?;
        const P: &str = "CFlarePart";
        let parts = root
            .subs("CFlareConfiguration", "1")?
            .iter()
            .map(|p| {
                let texture: ObjectRef = p.get(P, "1")?;
                Ok(FlarePart {
                    texture: ctx.required_file(texture, "a flare texture")?,
                    color: p.get(P, "10")?,
                    offset: p.get(P, "2")?,
                    size: p.get(P, "3")?,
                    aspect: p.get(P, "4")?,
                    amplification: p.get(P, "5")?,
                    desaturation: p.get(P, "6")?,
                    falloff: p.get(P, "7")?,
                    rotation: p.get(P, "8")?,
                    flags: p.get(P, "9")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if parts.is_empty() {
            return Err(ctx.bad("a flare configuration has no parts"));
        }
        Ok(Self { parts })
    }

    /// The distinct textures of the parts in order of appearance (compared case-insensitively).
    pub fn referenced_files(&self) -> Vec<&str> {
        let mut seen = std::collections::BTreeSet::new();
        self.parts
            .iter()
            .map(|p| p.texture.as_str())
            .filter(|t| seen.insert(t.to_lowercase()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    use super::*;

    /// A one-part flare mirroring the test vector of `fx.md` (`kleerofski_diamond.lfx`), plus an
    /// optional second part.
    pub(crate) fn flare_file(parts: usize) -> Vec<u8> {
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

    #[test]
    fn reads_the_parts_of_a_flare() {
        let f = FlareConfiguration::from_bytes(&flare_file(2)).unwrap();
        assert_eq!(f.parts.len(), 2);
        let p = &f.parts[0];
        assert_eq!(p.texture, "Content/Test/Flare.tex");
        assert_eq!(p.color, 0xFFFF_FFFF);
        assert_eq!((p.offset, p.size, p.aspect), (1.0, 0.13, 1.0));
        assert_eq!((p.amplification, p.desaturation), (1.0, 0.5));
        assert_eq!((p.falloff, p.rotation, p.flags), (0.05, 90.0, 1));
        assert_eq!(f.parts[1].offset, 0.0);
        // Both parts share one texture.
        assert_eq!(f.referenced_files(), ["Content/Test/Flare.tex"]);
    }

    #[test]
    fn a_flare_without_parts_or_with_junk_is_refused() {
        let err = FlareConfiguration::from_bytes(&flare_file(0))
            .unwrap_err()
            .to_string();
        assert!(err.contains("no parts"), "{err}");
        assert!(FlareConfiguration::from_bytes(b"junk").is_err());
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        let good = flare_file(2);
        for len in (0..good.len()).step_by(5) {
            let _ = FlareConfiguration::from_bytes(&good[..len]);
        }
        for i in (0..good.len()).step_by(3) {
            let mut bad = good.clone();
            bad[i] ^= 0x55;
            let _ = FlareConfiguration::from_bytes(&bad);
        }
    }

    /// Data-gated: all 37 `.lfx` files of the retail install load with the 191 parts of `fx.md`.
    #[test]
    fn every_flare_of_the_real_install_loads() {
        let dir = ss2_testkit::require_game_data!();
        let files = crate::testutil::install_files(&dir, "lfx");
        let (mut parts, mut textures) = (0usize, 0usize);
        for (path, bytes) in &files {
            let f = FlareConfiguration::from_bytes(bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
            parts += f.parts.len();
            textures += f.referenced_files().len();
            assert!(f.parts.iter().all(|p| p.color >> 24 == 0xFF), "{path}");
        }
        eprintln!(
            ".lfx: {} files, {parts} parts, {textures} textures",
            files.len()
        );
        assert_eq!(files.len(), 37);
        assert_eq!(parts, 191);
    }
}
