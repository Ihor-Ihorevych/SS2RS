//! ss2-format-fnt — the `.fnt` bitmap font loader (roadmap T02-025; spec `docs/re/fnt.md`).
//!
//! A font is a **bitmap atlas** plus a table of glyph rectangles. The file holds one `CFont` and
//! the atlas as one `CStaticTexture` stored inside it. [`Font::from_bytes`] returns both: the
//! glyph records ([`Glyph`]), the code table that maps a character code to a glyph, the line
//! height, and the atlas as a [`ss2_format_tex::Texture`] (the glyph shapes are in its **alpha**
//! channel; decoding the DXT blocks is the transcoder's job, T02-013).
//!
//! The glyph lookup and the placement of a line of text follow the spec's layout rule:
//! [`Font::glyph_index`], [`Font::glyph`], [`Font::layout`] and [`Font::text_width`]. Wrapping,
//! shaping and colour are the UI's job (T10-002).
//!
//! ```ignore
//! let font = ss2_format_fnt::Font::from_bytes(&bytes)?;
//! for p in font.layout("Hello") {
//!     let g = &font.glyphs[p.glyph];
//!     // copy the rectangle (g.x, g.y, g.width, g.height) of `font.atlas` to (p.x, p.y)
//! }
//! // or through the asset manager:  assets.register_loader(FntLoader)?;
//! //                               let h = assets.load::<Font>("Content/Fonts/GameConsoleFont.fnt")?;
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_format_tex::Texture;
use ss2_meta::{Container, MetaDeserialize, ObjectRef};

const WHAT: &str = ".fnt font";

/// One glyph: a rectangle of the atlas and how to place it (`CFontCharacter`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    /// Left edge of the rectangle in the atlas, in pixels (origin top-left).
    pub x: u32,
    /// Top edge of the rectangle in the atlas.
    pub y: u32,
    /// Width of the rectangle (may be 0).
    pub width: u32,
    /// Height of the rectangle (may be 0).
    pub height: u32,
    /// Draw the rectangle this many pixels to the right of the pen position (negative for some
    /// proportional glyphs, for example `A` and `V`).
    pub x_offset: i32,
    /// Member `.6`: a y offset; 0 in every glyph of the retail fonts.
    pub y_offset: i32,
    /// Move the pen by this many pixels after the glyph.
    pub advance: i32,
    /// Member `.8`: 0 in every glyph of the retail fonts (meaning unknown).
    pub reserved: i32,
}

/// A glyph placed by [`Font::layout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedGlyph {
    /// The character that was laid out.
    pub ch: char,
    /// Index into [`Font::glyphs`].
    pub glyph: usize,
    /// Where to draw the top-left corner of the glyph rectangle, relative to the text origin:
    /// the pen position plus the glyph's x offset.
    pub x: i32,
    /// The line number times the line height, plus the glyph's y offset.
    pub y: i32,
}

/// A decoded `.fnt` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    /// The glyph atlas. The glyph rectangles refer to its pixels; its alpha channel holds the
    /// glyph shapes.
    pub atlas: Texture,
    /// The glyph records; index 0 is the fallback glyph.
    pub glyphs: Vec<Glyph>,
    /// Entry `c` is the glyph index for character code `c`; 0 means "no glyph" (use glyph 0).
    /// 127 entries (ASCII only), 8,231 or 8,483 in the retail fonts.
    pub code_table: Vec<u32>,
    /// `CFont.3`: the height of a line in pixels; a new line is this much lower.
    pub line_height: i32,
    /// `CFont.2`: the advance of a monospaced font (in proportional fonts a larger number that
    /// no glyph uses).
    pub mono_advance: i32,
    /// `CFont.4`: the atlas width in pixels (equal to `atlas.width` in every retail font).
    pub atlas_width: i32,
    /// `CFont.1`: a size value just below the line height in most fonts (meaning not determined).
    pub size_hint: i32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CFont")]
struct RawFont {
    #[meta(name = "1")]
    size_hint: i32,
    #[meta(name = "2")]
    mono_advance: i32,
    #[meta(name = "3")]
    line_height: i32,
    #[meta(name = "4")]
    atlas_width: i32,
    #[meta(name = "5")]
    atlas: ObjectRef,
    #[meta(name = "6")]
    glyphs: Vec<RawGlyph>,
    #[meta(name = "7")]
    code_table: Vec<i32>,
}

#[derive(MetaDeserialize)]
#[meta(class = "CFontCharacter")]
struct RawGlyph {
    #[meta(name = "1")]
    x: i32,
    #[meta(name = "2")]
    y: i32,
    #[meta(name = "3")]
    width: i32,
    #[meta(name = "4")]
    height: i32,
    #[meta(name = "5")]
    x_offset: i32,
    #[meta(name = "6")]
    y_offset: i32,
    #[meta(name = "7")]
    advance: i32,
    #[meta(name = "8")]
    reserved: i32,
}

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

impl Font {
    /// Read a `.fnt` file.
    ///
    /// Checks the reader invariants of `fnt.md`: format version 8; exactly one `CFont` whose
    /// atlas is the file's one `CStaticTexture`; at least one glyph; every glyph rectangle
    /// inside the atlas; every code-table value below the glyph count. The extra editor records
    /// after the objects are ignored.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        let version = container.header.format_version;
        if version != 8 {
            return Err(bad(format!(
                "format version {version}, only 8 is supported"
            )));
        }
        let decoded = container.decode_objects()?;
        let class_of = |id: u32| {
            decoded
                .object(id)
                .and_then(|o| decoded.types.type_name(o.type_id))
        };
        let mut fonts = decoded
            .objects
            .iter()
            .filter(|o| class_of(o.id) == Some("CFont"));
        let font = fonts
            .next()
            .ok_or_else(|| bad("the file has no CFont object"))?;
        if fonts.next().is_some() {
            return Err(bad("the file has more than one CFont object"));
        }
        let raw: RawFont = decoded.typed(font.id)?;

        let atlas_id = raw
            .atlas
            .0
            .ok_or_else(|| bad("the font has no atlas texture"))?;
        if class_of(atlas_id) != Some("CStaticTexture") {
            return Err(bad(format!(
                "the atlas (object {atlas_id}) is not a CStaticTexture of this file"
            )));
        }
        let textures = decoded
            .objects
            .iter()
            .filter(|o| class_of(o.id) == Some("CStaticTexture"))
            .count();
        if textures != 1 {
            return Err(bad(format!(
                "{textures} CStaticTexture objects, expected 1"
            )));
        }
        let atlas = Texture::from_bytes(bytes)?;

        if raw.glyphs.is_empty() {
            return Err(bad("the font has no glyphs (glyph 0 is the fallback)"));
        }
        let (aw, ah) = (i64::from(atlas.width), i64::from(atlas.height));
        let mut glyphs = Vec::with_capacity(raw.glyphs.len());
        for (i, g) in raw.glyphs.iter().enumerate() {
            let (x, y, w, h) = (
                i64::from(g.x),
                i64::from(g.y),
                i64::from(g.width),
                i64::from(g.height),
            );
            if x < 0 || y < 0 || w < 0 || h < 0 || x + w > aw || y + h > ah {
                return Err(bad(format!(
                    "glyph {i}: rectangle ({x}, {y}, {w}, {h}) is outside the {aw} x {ah} atlas"
                )));
            }
            glyphs.push(Glyph {
                x: g.x.unsigned_abs(),
                y: g.y.unsigned_abs(),
                width: g.width.unsigned_abs(),
                height: g.height.unsigned_abs(),
                x_offset: g.x_offset,
                y_offset: g.y_offset,
                advance: g.advance,
                reserved: g.reserved,
            });
        }
        let mut code_table = Vec::with_capacity(raw.code_table.len());
        for (code, &v) in raw.code_table.iter().enumerate() {
            match usize::try_from(v) {
                Ok(index) if index < glyphs.len() => code_table.push(v.unsigned_abs()),
                _ => {
                    return Err(bad(format!(
                        "code {code} maps to glyph {v}, but the font has {} glyphs",
                        glyphs.len()
                    )));
                }
            }
        }
        Ok(Self {
            atlas,
            glyphs,
            code_table,
            line_height: raw.line_height,
            mono_advance: raw.mono_advance,
            atlas_width: raw.atlas_width,
            size_hint: raw.size_hint,
        })
    }

    /// The glyph index for character code `code`: the table entry, or 0 (the fallback glyph) if
    /// the code lies outside the table or its entry is 0.
    pub fn glyph_index(&self, code: u32) -> usize {
        self.code_table
            .get(code as usize)
            .map_or(0, |&i| i as usize)
    }

    /// `true` if the font has a glyph of its own for `code` (not the fallback).
    pub fn has_glyph(&self, code: u32) -> bool {
        self.glyph_index(code) != 0
    }

    /// The glyph used for character code `code` (the fallback glyph 0 if unmapped).
    pub fn glyph(&self, code: u32) -> &Glyph {
        &self.glyphs[self.glyph_index(code)]
    }

    /// The glyph rectangle of `glyph` as texture coordinates `[u0, v0, u1, v1]` (0 to 1,
    /// origin top-left).
    pub fn uv(&self, glyph: &Glyph) -> [f32; 4] {
        let (w, h) = (self.atlas.width as f32, self.atlas.height as f32);
        [
            glyph.x as f32 / w,
            glyph.y as f32 / h,
            (glyph.x + glyph.width) as f32 / w,
            (glyph.y + glyph.height) as f32 / h,
        ]
    }

    /// Place the characters of `text` by the spec's rule: start the pen at the origin; for each
    /// character take its glyph, draw the rectangle at `pen.x + x_offset`, then advance the pen
    /// by the glyph's advance. A line feed starts a new line ([`Font::line_height`] pixels lower,
    /// pen back at 0) and places nothing; a carriage return is skipped. There is no kerning.
    pub fn layout(&self, text: &str) -> Vec<PlacedGlyph> {
        let mut out = Vec::with_capacity(text.len());
        let (mut pen, mut line) = (0i32, 0i32);
        for ch in text.chars() {
            match ch {
                '\n' => {
                    pen = 0;
                    line = line.saturating_add(self.line_height);
                    continue;
                }
                '\r' => continue,
                _ => {}
            }
            let index = self.glyph_index(u32::from(ch));
            let g = &self.glyphs[index];
            out.push(PlacedGlyph {
                ch,
                glyph: index,
                x: pen.saturating_add(g.x_offset),
                y: line.saturating_add(g.y_offset),
            });
            pen = pen.saturating_add(g.advance);
        }
        out
    }

    /// The width in pixels of the widest line of `text` (the sum of the glyph advances).
    pub fn text_width(&self, text: &str) -> i32 {
        text.split('\n')
            .map(|line| {
                line.chars().filter(|&c| c != '\r').fold(0i32, |w, c| {
                    w.saturating_add(self.glyph(u32::from(c)).advance)
                })
            })
            .max()
            .unwrap_or(0)
    }

    /// The height in pixels of `text`: the number of lines times the line height.
    pub fn text_height(&self, text: &str) -> i32 {
        let lines = i32::try_from(text.split('\n').count()).unwrap_or(i32::MAX);
        lines.saturating_mul(self.line_height)
    }
}

/// The `.fnt` loader: register it with `AssetManager::register_loader` to load [`Font`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct FntLoader;

impl AssetLoader for FntLoader {
    type Asset = Font;

    fn extensions(&self) -> &[&str] {
        &["fnt"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Font> {
        Font::from_bytes(&ctx.read()?)
    }
}

/// Builders of synthetic `.fnt` files for the tests (no game data).
#[cfg(test)]
mod testing {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use std::io::Write;

    /// A glyph record as the file stores it: `(x, y, w, h, x offset, y offset, advance, reserved)`.
    pub(crate) type G = (i32, i32, i32, i32, i32, i32, i32, i32);

    /// A version-8 `.fnt` with a `width` x `height` 32-bit atlas, the glyphs and the code table.
    pub(crate) fn font_file(
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
    pub(crate) fn sample() -> Vec<u8> {
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_format_tex::PixelFormat;
    use ss2_vfs::MemoryVfs;

    use super::testing::{G, font_file, sample};
    use super::*;

    #[test]
    fn reads_glyphs_table_and_atlas() {
        let f = Font::from_bytes(&sample()).unwrap();
        assert_eq!(f.glyphs.len(), 4);
        assert_eq!(f.code_table.len(), 67);
        assert_eq!(
            (f.line_height, f.mono_advance, f.atlas_width, f.size_hint),
            (9, 7, 16, 8)
        );
        assert_eq!(
            (f.atlas.width, f.atlas.height, f.atlas.format),
            (16, 8, PixelFormat::Bgra8)
        );
        assert_eq!(
            f.glyphs[2],
            Glyph {
                x: 5,
                y: 0,
                width: 6,
                height: 8,
                x_offset: -2,
                y_offset: 0,
                advance: 6,
                reserved: 0
            }
        );
    }

    #[test]
    fn unmapped_and_out_of_table_codes_use_the_fallback_glyph() {
        let f = Font::from_bytes(&sample()).unwrap();
        assert_eq!(f.glyph_index(u32::from('A')), 2);
        assert_eq!(f.glyph_index(32), 1);
        assert_eq!(f.glyph_index(33), 0); // inside the table, entry 0
        assert_eq!(f.glyph_index(0x2122), 0); // outside the table
        assert_eq!(f.glyph_index(u32::MAX), 0);
        assert!(f.has_glyph(66) && !f.has_glyph(67));
        assert_eq!(f.glyph(0x2122), &f.glyphs[0]);
    }

    #[test]
    fn layout_follows_the_pen_and_the_offsets() {
        let f = Font::from_bytes(&sample()).unwrap();
        let p = f.layout("AB A\nB");
        let got: Vec<_> = p.iter().map(|g| (g.ch, g.glyph, g.x, g.y)).collect();
        // A: pen 0, offset -2; B: pen 6, +1; space: pen 13; A: pen 16, -2; line 2: B pen 0, +1.
        assert_eq!(
            got,
            [
                ('A', 2, -2, 0),
                ('B', 3, 7, 0),
                (' ', 1, 13, 0),
                ('A', 2, 14, 0),
                ('B', 3, 1, 9)
            ]
        );
        assert_eq!(f.text_width("AB A\nB"), 6 + 7 + 3 + 6);
        assert_eq!(f.text_height("AB A\nB"), 18);
        assert_eq!(f.text_height(""), 9);
        assert_eq!(f.text_width(""), 0);
        // Carriage returns are skipped, unknown characters use glyph 0.
        assert_eq!(f.layout("\r\u{20AC}")[0].glyph, 0);
        assert_eq!(f.layout("\r").len(), 0);
    }

    #[test]
    fn uv_is_the_rectangle_over_the_atlas_size() {
        let f = Font::from_bytes(&sample()).unwrap();
        assert_eq!(f.uv(&f.glyphs[2]), [5.0 / 16.0, 0.0, 11.0 / 16.0, 1.0]);
    }

    #[test]
    fn bad_fonts_are_refused_with_a_reason() {
        let err = |bytes: Vec<u8>| Font::from_bytes(&bytes).unwrap_err().to_string();
        let ok: G = (0, 0, 4, 8, 0, 0, 5, 0);
        // A rectangle outside the atlas, a negative one.
        assert!(
            err(font_file(16, 8, 9, &[ok, (14, 0, 4, 8, 0, 0, 5, 0)], &[0])).contains("glyph 1")
        );
        assert!(err(font_file(16, 8, 9, &[(0, 0, 4, 9, 0, 0, 5, 0)], &[0])).contains("outside"));
        assert!(err(font_file(16, 8, 9, &[(-1, 0, 4, 8, 0, 0, 5, 0)], &[0])).contains("outside"));
        assert!(err(font_file(16, 8, 9, &[(0, 0, -4, 8, 0, 0, 5, 0)], &[0])).contains("outside"));
        // A table value at or above the glyph count, or negative.
        assert!(err(font_file(16, 8, 9, &[ok], &[0, 1])).contains("code 1 maps to glyph 1"));
        assert!(err(font_file(16, 8, 9, &[ok], &[0, -1])).contains("glyph -1"));
        // No glyphs at all.
        assert!(err(font_file(16, 8, 9, &[], &[])).contains("no glyphs"));
        // Not a container.
        assert!(Font::from_bytes(b"hello").is_err());
    }

    #[test]
    fn the_loader_plugs_into_the_asset_manager() {
        let vfs = MemoryVfs::from_files([
            ("fonts/a.fnt", sample().as_slice()),
            ("fonts/bad.fnt", b"junk".as_slice()),
        ])
        .unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(FntLoader).unwrap();
        let h = assets.load::<Font>("Fonts/A.FNT").unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().glyphs.len(), 4);
        let bad = assets.load::<Font>("fonts/bad.fnt").unwrap();
        assert_eq!(bad.wait(), LoadState::Failed);
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        let good = sample();
        for len in 0..good.len() {
            let _ = Font::from_bytes(&good[..len]);
        }
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            let _ = Font::from_bytes(&bad);
        }
    }

    /// Data-gated: every `.fnt` of the retail install loads and reproduces the counts and the
    /// invariants of `fnt.md` (21 fonts; table lengths 127 x 5, 8,231 x 3, 8,483 x 13; the table
    /// values are distinct and cover glyphs 1..n; the atlas formats; the test vector). Skips
    /// without `SS2_DATA_DIR`.
    #[test]
    fn every_font_of_the_real_install_loads() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        use std::collections::BTreeMap;
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut fonts, mut failures) = (BTreeMap::new(), Vec::new());
        let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();
        let mut formats: BTreeMap<String, usize> = BTreeMap::new();
        let (mut covers, mut widths, mut wraps) = (0usize, 0usize, 0usize);
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if path.extension() != Some("fnt") {
                    continue;
                }
                match Font::from_bytes(&vfs.read(&path).unwrap()) {
                    Ok(f) => {
                        *lengths.entry(f.code_table.len()).or_default() += 1;
                        *formats.entry(format!("{:?}", f.atlas.format)).or_default() += 1;
                        let mut used: Vec<u32> =
                            f.code_table.iter().copied().filter(|&v| v != 0).collect();
                        used.sort_unstable();
                        let want: Vec<u32> = (1..f.glyphs.len() as u32).collect();
                        covers += usize::from(used == want);
                        widths += usize::from(f.atlas_width as u32 == f.atlas.width);
                        wraps += usize::from(f.atlas.wrap_u == 0x51 && f.atlas.wrap_v == 0x51);
                        fonts.insert(path.as_str().to_ascii_lowercase(), f);
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        assert!(failures.is_empty(), "failures: {failures:?}");
        assert_eq!(fonts.len(), 21);
        assert_eq!(lengths, BTreeMap::from([(127, 5), (8231, 3), (8483, 13)]));
        assert_eq!(formats.get("Dxt5"), Some(&10), "{formats:?}");
        assert_eq!(formats.get("Bgra8"), Some(&7), "{formats:?}");
        assert_eq!(formats.get("Dxt3"), Some(&4), "{formats:?}");
        assert_eq!((covers, widths, wraps), (21, 21, 21));
        let game = fonts
            .keys()
            .filter(|k| !k.contains("seriouseditor2"))
            .count();
        assert_eq!(game, 13);

        // The test vector of the spec: GameConsoleFont.
        let f = fonts
            .iter()
            .find(|(k, _)| k.ends_with("fonts/gameconsolefont.fnt"))
            .map(|(_, f)| f)
            .expect("GameConsoleFont.fnt");
        assert_eq!(
            (f.size_hint, f.mono_advance, f.line_height, f.atlas_width),
            (9, 7, 12, 128)
        );
        assert_eq!(
            (f.atlas.width, f.atlas.height, f.atlas.format_code),
            (128, 64, 9)
        );
        assert_eq!(
            (f.atlas.flags, f.glyphs.len(), f.code_table.len()),
            (0x24, 96, 127)
        );
        assert!(f.code_table[..32].iter().all(|&v| v == 0));
        assert!((32..127).all(|c| f.code_table[c] as usize == c - 31));
        let g = |i: usize| {
            let g = f.glyphs[i];
            (
                g.x, g.y, g.width, g.height, g.x_offset, g.y_offset, g.advance, g.reserved,
            )
        };
        let (g0, g1, g2) = (g(0), g(1), g(2));
        assert_eq!(g0, (1, 1, 5, 12, 1, 0, 7, 0));
        assert_eq!(g1, (7, 1, 1, 12, 0, 0, 7, 0));
        assert_eq!(g2, (9, 1, 1, 12, 3, 0, 7, 0));
        assert_eq!(g(34), (63, 13, 7, 12, 0, 0, 7, 0));
        assert_eq!(g(95), (77, 49, 7, 12, 0, 0, 7, 0));
        assert_eq!(f.glyph_index(u32::from('A')), 34);
        assert_eq!(f.text_width("Hello"), 35);
    }
}
