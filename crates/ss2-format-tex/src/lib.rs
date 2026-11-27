//! ss2-format-tex — the `.tex` loader (roadmap T02-012; spec `docs/re/tex.md`, `tex-flags.md`).
//!
//! A `.tex` file is a CTSEMETA container with a `CResourceFile` and one `CStaticTexture`. The
//! texture holds its size, a format code, flags, wrap modes and **one zlib-compressed byte array
//! with all pixel data**. [`Texture::from_bytes`] reads it into a [`Texture`]: the metadata and the
//! raw, still block-compressed pixels with their mip chain laid out for direct access
//! ([`Texture::level`]). Converting DXT blocks to RGBA or uploading them is the job of the
//! transcoder (T02-013).
//!
//! Format version 8 is the format of nearly every file. The 155 development-build textures of
//! container versions 4 to 7 (125 of them the copy the game reads) store their pixels in the
//! legacy `CBaseTexture` layouts of `tex-legacy.md` (raw or zlib, with or without a length
//! prefix, the pixel layout from `TexStoredFormat` or `TexFormat`); [`Texture::from_bytes`] reads
//! them into the same [`Texture`] (T02-041), with [`Texture::format_version`], the stored
//! [`Texture::frames`] count and the [`Texture::palette`] array kept for inspection.
//!
//! ```ignore
//! use ss2_format_tex::{PixelFormat, TexLoader, Texture};
//! // `bytes` is the content of a `.tex` file, for example read through the VFS:
//! let tex = Texture::from_bytes(&bytes)?;
//! println!("{} x {} {:?}, {} mip levels", tex.width, tex.height, tex.format, tex.mip_levels);
//! let largest = tex.level(0, 0).unwrap();         // face 0, level 0: the largest
//! // or through the asset manager:  assets.register_loader(TexLoader)?;
//! //                               let h = assets.load::<Texture>("Content/.../X.tex")?;
//! ```

use std::io::Read;

use flate2::read::ZlibDecoder;
use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_meta::{Bytes, Container, MetaDeserialize};

mod legacy;

const WHAT: &str = ".tex texture";
/// `TexFlags` bit: the pixel data is a cube map (six faces, each with its mip chain).
pub const FLAG_CUBE_MAP: u32 = 0x02;
/// Wrap code of repeating textures (`GfxTexWrap`, `tex-flags.md`).
pub const WRAP_REPEAT: u32 = 0x50;
/// Wrap code of clamped textures.
pub const WRAP_CLAMP: u32 = 0x51;
/// Most pixel bytes accepted after inflating (the largest real texture, 4096 x 2048 x 4, is 32 MiB;
/// a cube map of 2048 x 2048 DXT1 is 16 MiB).
pub(crate) const MAX_PIXEL_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_SIDE: u32 = 16_384;

/// How the pixels of a texture are stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// 24 bit, bytes in the order blue, green, red; one level.
    Bgr8,
    /// 32 bit, bytes in the order blue, green, red, alpha; one level.
    Bgra8,
    /// DXT1 / BC1: 8 bytes per 4 x 4 block; full mip chain.
    Dxt1,
    /// DXT3 / BC2: 16 bytes per block (explicit 4-bit alpha); full mip chain.
    Dxt3,
    /// DXT5 / BC3: 16 bytes per block (interpolated alpha); full mip chain.
    Dxt5,
}

impl PixelFormat {
    /// The layout of a `TexFormat` code of the files (`tex.md`): 0, 2, 6 are 24-bit; 1, 3, 4, 5,
    /// 11 are 32-bit; 7, 8, 9 are DXT1, DXT3, DXT5. The codes that share a layout differ in the
    /// intended use of the texture, not in the data (`tex-flags.md`).
    pub fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 | 2 | 6 => Self::Bgr8,
            1 | 3 | 4 | 5 | 11 => Self::Bgra8,
            7 => Self::Dxt1,
            8 => Self::Dxt3,
            9 => Self::Dxt5,
            _ => return None,
        })
    }

    /// `true` for the block-compressed formats, which carry a mip chain.
    pub fn is_compressed(self) -> bool {
        matches!(self, Self::Dxt1 | Self::Dxt3 | Self::Dxt5)
    }

    /// Bytes of one level of `width x height` pixels.
    pub fn level_bytes(self, width: u32, height: u32) -> u64 {
        let (w, h) = (u64::from(width), u64::from(height));
        match self {
            Self::Bgr8 => w * h * 3,
            Self::Bgra8 => w * h * 4,
            Self::Dxt1 => w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * 8,
            Self::Dxt3 | Self::Dxt5 => w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * 16,
        }
    }
}

/// One mip level of one face: its size and its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MipLevel<'a> {
    /// Width in pixels (at least 1).
    pub width: u32,
    /// Height in pixels (at least 1).
    pub height: u32,
    /// The bytes of the level, in the layout of [`Texture::format`].
    pub data: &'a [u8],
}

/// A decoded `.tex` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    /// Width of the largest level, in pixels (a power of two).
    pub width: u32,
    /// Height of the largest level, in pixels (a power of two).
    pub height: u32,
    /// How the pixels are stored.
    pub format: PixelFormat,
    /// The `TexFormat` code as stored (it says what the texture is used for; `tex-flags.md`).
    pub format_code: u32,
    /// The `TexFlags` word (bit [`FLAG_CUBE_MAP`] marks a cube map; `0x10` a baked light map).
    pub flags: u32,
    /// Wrap code for U ([`WRAP_REPEAT`], [`WRAP_CLAMP`], …).
    pub wrap_u: u32,
    /// Wrap code for V.
    pub wrap_v: u32,
    /// The `COLOR` member `CBaseTexture.14` (probably the border colour; 0 in nearly every file).
    pub border_color: u32,
    /// 1, or 6 for a cube map.
    pub faces: u32,
    /// Number of mip levels per face: the whole chain down to 1 x 1 for DXT, 1 otherwise.
    pub mip_levels: u32,
    /// All pixels of the first frame: face after face, each face its levels largest first,
    /// without padding.
    pub data: Vec<u8>,
    /// The CTSEMETA format version of the file: 8, or 4 to 7 for the legacy layouts of
    /// `tex-legacy.md`.
    pub format_version: u32,
    /// The stored frame count (`CBaseTexture.5`; 1 where the layout has no such member). Only the
    /// first frame is in [`Texture::data`]; every retail texture has exactly one.
    pub frames: u32,
    /// The `COLOR` array `CBaseTexture.8` of class layout 5, as stored; empty in every retail
    /// file and in every other layout. The pixels do not refer to it.
    pub palette: Vec<u32>,
}

#[derive(MetaDeserialize)]
#[meta(class = "CStaticTexture")]
struct RawTexture {
    #[meta(owner = "CBaseTexture", name = "1")]
    width: i32,
    #[meta(owner = "CBaseTexture", name = "2")]
    height: i32,
    #[meta(owner = "CBaseTexture", name = "4")]
    flags: u32,
    #[meta(owner = "CBaseTexture", name = "6")]
    format_code: u32,
    #[meta(owner = "CBaseTexture", name = "18")]
    blob: Bytes,
    #[meta(owner = "CBaseTexture", name = "9")]
    wrap_u: u32,
    #[meta(owner = "CBaseTexture", name = "10")]
    wrap_v: u32,
    #[meta(owner = "CBaseTexture", name = "14")]
    border_color: u32,
}

pub(crate) fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

impl Texture {
    /// Read a `.tex` file.
    ///
    /// Checks the rules of `tex.md`: version 8; size and format code known; the inflated size
    /// equals the size predicted for the format, the faces and the mip chain; nothing is
    /// allocated from a declared size before it is bounded.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong. Versions 4 to 7 follow the rules of
    /// `tex-legacy.md` instead: the class layout is known, the pixel array is exactly one raw
    /// frame set or one complete zlib stream (layout 6: with a matching byte count in front) of
    /// the predicted size.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        let version = container.header.format_version;
        if version != 8 {
            return legacy::read(&container);
        }
        let decoded = container.decode_objects()?;
        let object = decoded
            .objects
            .iter()
            .find(|o| decoded.types.type_name(o.type_id) == Some("CStaticTexture"))
            .ok_or_else(|| bad("the file has no CStaticTexture object"))?;
        let raw: RawTexture = ss2_meta::FromValue::from_value(&object.value, &decoded.types)?;

        let (width, height) = match (u32::try_from(raw.width), u32::try_from(raw.height)) {
            (Ok(w), Ok(h)) if (1..=MAX_SIDE).contains(&w) && (1..=MAX_SIDE).contains(&h) => (w, h),
            _ => {
                return Err(bad(format!(
                    "size {} x {} is not between 1 and {MAX_SIDE}",
                    raw.width, raw.height
                )));
            }
        };
        let format = PixelFormat::from_code(raw.format_code)
            .ok_or_else(|| bad(format!("unknown format code {}", raw.format_code)))?;
        let faces: u32 = if raw.flags & FLAG_CUBE_MAP != 0 { 6 } else { 1 };
        if faces == 6 && !format.is_compressed() {
            return Err(bad(
                "a cube map must be DXT (the files only have DXT1 cube maps)",
            ));
        }
        let mip_levels = if format.is_compressed() {
            u32::BITS - width.max(height).leading_zeros()
        } else {
            1
        };
        let face_bytes: u64 = (0..mip_levels)
            .map(|l| format.level_bytes((width >> l).max(1), (height >> l).max(1)))
            .sum();
        let expected = face_bytes * u64::from(faces);
        if expected > MAX_PIXEL_BYTES {
            return Err(bad(format!(
                "{expected} pixel bytes are more than the {MAX_PIXEL_BYTES} accepted"
            )));
        }

        let mut data = Vec::with_capacity(expected as usize);
        ZlibDecoder::new(raw.blob.0.as_slice())
            .take(expected + 1)
            .read_to_end(&mut data)
            .map_err(|e| bad(format!("the pixel data is not a valid zlib stream: {e}")))?;
        if data.len() as u64 != expected {
            return Err(bad(format!(
                "the pixel data has {} bytes after inflating, {expected} expected for {width} x {height} code {} (flags {:#x})",
                if data.len() as u64 > expected {
                    "more than".to_string()
                } else {
                    data.len().to_string()
                },
                raw.format_code,
                raw.flags
            )));
        }
        Ok(Self {
            width,
            height,
            format,
            format_code: raw.format_code,
            flags: raw.flags,
            wrap_u: raw.wrap_u,
            wrap_v: raw.wrap_v,
            border_color: raw.border_color,
            faces,
            mip_levels,
            data,
            format_version: 8,
            frames: 1,
            palette: Vec::new(),
        })
    }

    /// `true` for a cube map (six faces).
    pub fn is_cube_map(&self) -> bool {
        self.faces == 6
    }

    /// Bytes of one face: its whole mip chain.
    pub fn face_bytes(&self) -> usize {
        self.data.len() / self.faces as usize
    }

    /// Mip level `level` (0 is the largest) of face `face`; `None` if out of range.
    pub fn level(&self, face: u32, level: u32) -> Option<MipLevel<'_>> {
        if face >= self.faces || level >= self.mip_levels {
            return None;
        }
        let size = |l: u32| (self.width >> l).max(1);
        let high = |l: u32| (self.height >> l).max(1);
        let offset: u64 = (0..level)
            .map(|l| self.format.level_bytes(size(l), high(l)))
            .sum();
        let len = self.format.level_bytes(size(level), high(level));
        let start = face as usize * self.face_bytes() + offset as usize;
        Some(MipLevel {
            width: size(level),
            height: high(level),
            data: self.data.get(start..start + len as usize)?,
        })
    }
}

/// The `.tex` loader: register it with `AssetManager::register_loader` to load [`Texture`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct TexLoader;

impl AssetLoader for TexLoader {
    type Asset = Texture;

    fn extensions(&self) -> &[&str] {
        &["tex"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Texture> {
        Texture::from_bytes(&ctx.read()?)
    }
}

/// Builders of synthetic `.tex` files for the tests (no game data).
#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_vfs::MemoryVfs;

    use super::testing::{texture_file, texture_file_with_blob};
    use super::*;

    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 % 251) as u8).collect()
    }

    #[test]
    fn the_spec_test_vector_is_a_white_bgr_image() {
        // `WhiteBckg_001.tex`: 16 x 16, flags 0x20, code 0, wrap 0x50/0x50, this exact blob.
        let blob = [
            0x78, 0x9C, 0xFB, 0xFF, 0x7F, 0x14, 0x8C, 0x82, 0x91, 0x0B, 0x00, 0x44, 0xD3, 0xFD,
            0x1F,
        ];
        let t = Texture::from_bytes(&texture_file_with_blob(16, 16, 0x20, 0, 0x50, 0x50, &blob))
            .unwrap();
        assert_eq!(
            (t.width, t.height, t.format, t.flags),
            (16, 16, PixelFormat::Bgr8, 0x20)
        );
        assert_eq!(
            (t.wrap_u, t.wrap_v, t.faces, t.mip_levels),
            (WRAP_REPEAT, WRAP_REPEAT, 1, 1)
        );
        assert_eq!(t.data.len(), 768);
        assert!(t.data.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn uncompressed_formats_have_one_level() {
        for (code, format, bpp) in [
            (0, PixelFormat::Bgr8, 3),
            (2, PixelFormat::Bgr8, 3),
            (6, PixelFormat::Bgr8, 3),
            (1, PixelFormat::Bgra8, 4),
            (3, PixelFormat::Bgra8, 4),
            (4, PixelFormat::Bgra8, 4),
            (5, PixelFormat::Bgra8, 4),
            (11, PixelFormat::Bgra8, 4),
        ] {
            let px = pattern(8 * 4 * bpp);
            let t = Texture::from_bytes(&texture_file(8, 4, 0x21, code, 0x51, 0x50, &px)).unwrap();
            assert_eq!((t.format, t.format_code, t.mip_levels), (format, code, 1));
            assert_eq!((t.wrap_u, t.wrap_v), (WRAP_CLAMP, WRAP_REPEAT));
            let l = t.level(0, 0).unwrap();
            assert_eq!((l.width, l.height, l.data), (8, 4, px.as_slice()));
            assert!(t.level(0, 1).is_none() && t.level(1, 0).is_none());
        }
    }

    #[test]
    fn dxt_textures_carry_the_whole_mip_chain() {
        // DXT1 4x4: 8 + 8 + 8 bytes (4x4, 2x2, 1x1 each one block).
        let px = pattern(24);
        let t = Texture::from_bytes(&texture_file(4, 4, 0x21, 7, 0x50, 0x50, &px)).unwrap();
        assert_eq!((t.format, t.mip_levels), (PixelFormat::Dxt1, 3));
        let sizes: Vec<_> = (0..3)
            .map(|l| t.level(0, l).unwrap())
            .map(|m| (m.width, m.height, m.data.len()))
            .collect();
        assert_eq!(sizes, [(4, 4, 8), (2, 2, 8), (1, 1, 8)]);
        assert_eq!(t.level(0, 1).unwrap().data, &px[8..16]);

        // DXT5 8x4: 2x1 blocks (32), then 1 block each for 4x2, 2x1, 1x1 (16 each) = 80.
        let px = pattern(80);
        let t = Texture::from_bytes(&texture_file(8, 4, 0x21, 9, 0x50, 0x50, &px)).unwrap();
        assert_eq!(
            (t.format, t.mip_levels, t.data.len()),
            (PixelFormat::Dxt5, 4, 80)
        );
        let sizes: Vec<_> = (0..4)
            .map(|l| t.level(0, l).unwrap())
            .map(|m| (m.width, m.height, m.data.len()))
            .collect();
        assert_eq!(sizes, [(8, 4, 32), (4, 2, 16), (2, 1, 16), (1, 1, 16)]);

        // DXT3 shares the block size of DXT5.
        let t =
            Texture::from_bytes(&texture_file(8, 4, 0x21, 8, 0x50, 0x50, &pattern(80))).unwrap();
        assert_eq!(t.format, PixelFormat::Dxt3);
    }

    #[test]
    fn a_cube_map_has_six_faces_each_with_its_chain() {
        // 8x8 DXT1 face: 32 + 8 + 8 + 8 = 56 bytes; six faces = 336.
        let px = pattern(336);
        let t = Texture::from_bytes(&texture_file(8, 8, 0x23, 7, 0x51, 0x51, &px)).unwrap();
        assert!(t.is_cube_map());
        assert_eq!((t.faces, t.mip_levels, t.face_bytes()), (6, 4, 56));
        let f3 = t.level(3, 0).unwrap();
        assert_eq!((f3.width, f3.data.len()), (8, 32));
        assert_eq!(f3.data, &px[3 * 56..3 * 56 + 32]);
        assert_eq!(t.level(5, 3).unwrap().data, &px[5 * 56 + 48..6 * 56]);
        assert!(t.level(6, 0).is_none());
    }

    #[test]
    fn bad_files_are_refused_with_a_reason() {
        let err = |bytes: Vec<u8>| Texture::from_bytes(&bytes).unwrap_err().to_string();
        // Wrong inflated size (too short, too long).
        assert!(err(texture_file(4, 4, 0x20, 0, 0x50, 0x50, &pattern(47))).contains("47 bytes"));
        assert!(err(texture_file(4, 4, 0x20, 0, 0x50, 0x50, &pattern(49))).contains("more than"));
        // Not zlib.
        assert!(
            err(texture_file_with_blob(
                4,
                4,
                0x20,
                0,
                0x50,
                0x50,
                b"not zlib at all"
            ))
            .contains("zlib")
        );
        // Unknown format code, bad sizes, cube map that is not DXT.
        assert!(
            err(texture_file(4, 4, 0x20, 10, 0x50, 0x50, &pattern(48)))
                .contains("unknown format code 10")
        );
        assert!(err(texture_file(0, 4, 0x20, 0, 0x50, 0x50, &[])).contains("size 0 x 4"));
        assert!(err(texture_file(-3, 4, 0x20, 0, 0x50, 0x50, &[])).contains("size -3"));
        assert!(err(texture_file(4, 4, 0x22, 1, 0x50, 0x50, &pattern(64))).contains("cube map"));
        // A decompression bomb is cut off at the predicted size.
        let bomb = {
            use flate2::{Compression, write::ZlibEncoder};
            use std::io::Write;
            let mut z = ZlibEncoder::new(Vec::new(), Compression::best());
            z.write_all(&vec![0u8; 10_000_000]).unwrap();
            z.finish().unwrap()
        };
        assert!(
            err(texture_file_with_blob(4, 4, 0x20, 0, 0x50, 0x50, &bomb)).contains("more than")
        );
        // Not a container at all.
        assert!(Texture::from_bytes(b"hello").is_err());
    }

    #[test]
    fn the_loader_plugs_into_the_asset_manager() {
        let tex = texture_file(4, 4, 0x21, 7, 0x50, 0x50, &pattern(24));
        let vfs = MemoryVfs::from_files([
            ("textures/a.tex", tex.as_slice()),
            ("textures/bad.tex", b"junk".as_slice()),
        ])
        .unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(TexLoader).unwrap();
        let h = assets.load::<Texture>("Textures/A.TEX").unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().mip_levels, 3);
        let bad = assets.load::<Texture>("textures/bad.tex").unwrap();
        assert_eq!(bad.wait(), LoadState::Failed);
        assert!(
            bad.error().unwrap().contains("CTSEMETA container"),
            "{:?}",
            bad.error()
        );
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        let good = texture_file(8, 4, 0x21, 9, 0x50, 0x50, &pattern(80));
        for len in 0..good.len() {
            let _ = Texture::from_bytes(&good[..len]);
        }
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            let _ = Texture::from_bytes(&bad);
        }
    }

    /// Data-gated: every `.tex` of the retail install loads, the version-8 files and the legacy
    /// textures of versions 4 to 7 that the game reads (125 effective paths, `tex-legacy.md`).
    /// Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_texture_of_the_real_install_loads() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        use std::collections::BTreeMap;
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        let (mut cubes, mut old, mut failures) = (0usize, 0usize, Vec::new());
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if path.extension() != Some("tex") {
                    continue;
                }
                let bytes = vfs.read(&path).unwrap();
                match Texture::from_bytes(&bytes) {
                    Ok(t) => {
                        *counts.entry(format!("{:?}", t.format)).or_default() += 1;
                        cubes += usize::from(t.is_cube_map());
                        old += usize::from(t.format_version < 8);
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "formats {counts:?}; {cubes} cube maps; {old} legacy (version 4 to 7) files read; {} failures",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(5)]
        );
        assert!(counts.values().sum::<usize>() > 2400, "{counts:?}");
        assert!(cubes >= 30, "only {cubes} cube maps");
        assert!(old >= 100, "only {old} legacy textures");
    }
}
