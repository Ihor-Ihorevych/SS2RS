//! The legacy texture layouts of CTSEMETA versions 4 to 7 (`docs/re/tex-legacy.md`, T01-083).
//!
//! The container frame of these versions is read by `ss2-meta` (`ctsemeta-leftovers.md`). What
//! differs is the `CBaseTexture` class: its layout version (the `kind` word of its type block)
//! and its declared members select how the pixel array `.18` is stored:
//!
//! | Class layout | `.18` | Pixel layout from |
//! |---:|---|---|
//! | 5 | raw pixel bytes | `.17` `TexStoredFormat` (`.6` is a quality setting) |
//! | 6 | `u32` compressed byte count (outer count minus four), then one zlib stream | `.6` `TexFormat` |
//! | 7 | raw pixel bytes | `.6` `TexFormat` |
//! | 9 | one zlib stream if flag `0x20` is set, else raw pixel bytes | `.6` `TexFormat` |
//!
//! Version 4 names the members (`tx_pixWidth`, ...) where later versions number them; both are
//! accepted. The pixels themselves are packed as in version 8. A 24- or 32-bit texture stores
//! its base level, or (observed once, in layout 7) its whole mip chain: the payload size tells
//! which. Frames follow one another (unobserved: every retail file has one frame); only the first
//! is kept.

use std::io::Read;

use flate2::read::ZlibDecoder;
use ss2_core::Result;
use ss2_meta::{Container, Struct, TypeShape, TypeSystem, Value};

use crate::{FLAG_CUBE_MAP, MAX_PIXEL_BYTES, MAX_SIDE, PixelFormat, Texture, bad};

/// `TexFlags` bit that selects zlib in class layout 9.
const FLAG_ZLIB: u32 = 0x20;
/// Most frames accepted (every retail texture has one).
const MAX_FRAMES: u32 = 4096;

// The `CBaseTexture` members a legacy reader uses: the numbered name of versions 5 to 7 and the
// source name of version 4.
const WIDTH: [&str; 2] = ["1", "tx_pixWidth"];
const HEIGHT: [&str; 2] = ["2", "tx_pixHeight"];
const FLAGS: [&str; 2] = ["4", "tx_ulFlags"];
const FRAMES: [&str; 2] = ["5", "tx_ctFrames"];
const FORMAT: [&str; 2] = ["6", "tx_txqDefault"];
const STORED: [&str; 2] = ["17", "tx_txfStored"];
const PIXELS: [&str; 2] = ["18", "tx_aubFrames"];
const PALETTE: [&str; 2] = ["8", "tx_acolFrames"];
const WRAP_U: [&str; 2] = ["9", "tx_eWrapU"];
const WRAP_V: [&str; 2] = ["10", "tx_eWrapV"];
const BORDER: [&str; 2] = ["14", "tx_colBorder"];

/// The members of one decoded texture object.
struct Members<'a> {
    types: &'a TypeSystem,
    value: &'a Struct,
}

impl Members<'_> {
    /// The value and declared type name of `CBaseTexture` member `names`, if declared.
    fn get(&self, names: [&str; 2]) -> Option<(&Value, &str)> {
        let layout = self.types.members(self.value.type_id)?;
        let i = layout
            .iter()
            .position(|m| m.owner_name == "CBaseTexture" && names.contains(&m.name.as_str()))?;
        let value = self.value.values.get(i)?;
        Some((value, self.types.type_name(layout[i].type_id).unwrap_or("")))
    }

    fn int(&self, names: [&str; 2]) -> Result<Option<i64>> {
        match self.get(names) {
            None => Ok(None),
            Some((v, _)) => v
                .as_i64()
                .map(Some)
                .ok_or_else(|| bad(format!("member CBaseTexture.{} is not a number", names[0]))),
        }
    }

    fn required(&self, names: [&str; 2]) -> Result<i64> {
        self.int(names)?.ok_or_else(|| {
            bad(format!(
                "the texture has no member CBaseTexture.{}",
                names[0]
            ))
        })
    }

    /// A flag, code or colour word (0 if absent), its bits kept.
    fn word(&self, names: [&str; 2]) -> Result<u32> {
        Ok(self.int(names)?.unwrap_or(0) as u32)
    }
}

/// Read the texture of a version 4 to 7 container.
pub(crate) fn read(container: &Container<'_>) -> Result<Texture> {
    let version = container.header.format_version;
    let layout = container
        .types
        .iter()
        .find(|t| t.name == "CBaseTexture" && matches!(t.shape, TypeShape::Struct { .. }))
        .map(|t| t.kind)
        .ok_or_else(|| bad("the file has no CBaseTexture class"))?;
    let decoded = container.decode_objects()?;
    let object = decoded
        .objects
        .iter()
        .find(|o| {
            decoded
                .types
                .members(o.type_id)
                .is_some_and(|m| m.iter().any(|m| m.owner_name == "CBaseTexture"))
        })
        .ok_or_else(|| bad("the file has no texture object"))?;
    let value = object
        .value
        .as_struct()
        .ok_or_else(|| bad("the texture object is not a class instance"))?;
    let m = Members {
        types: &decoded.types,
        value,
    };

    let (raw_w, raw_h) = (m.required(WIDTH)?, m.required(HEIGHT)?);
    let side = |v: i64| u32::try_from(v).ok().filter(|s| (1..=MAX_SIDE).contains(s));
    let (Some(width), Some(height)) = (side(raw_w), side(raw_h)) else {
        return Err(bad(format!(
            "size {raw_w} x {raw_h} is not between 1 and {MAX_SIDE}"
        )));
    };
    let flags = m.word(FLAGS)?;
    let frames = match m.int(FRAMES)? {
        None => 1,
        Some(n) => u32::try_from(n)
            .ok()
            .filter(|n| (1..=MAX_FRAMES).contains(n))
            .ok_or_else(|| bad(format!("frame count {n} is not between 1 and {MAX_FRAMES}")))?,
    };
    let format_code = m.word(FORMAT)?;
    // Layout 5 declares `.17` `TexStoredFormat` for the bytes; the others `.6` `TexFormat`.
    let format = match m.get(STORED) {
        Some((v, "TexStoredFormat")) => {
            let stored = v.as_i64().unwrap_or(-1);
            stored_format(stored)
                .ok_or_else(|| bad(format!("unknown stored format {stored} (layout {layout})")))?
        }
        _ => match m.get(FORMAT) {
            Some((_, "TexFormat")) => PixelFormat::from_code(format_code)
                .ok_or_else(|| bad(format!("unknown format code {format_code}")))?,
            _ => {
                return Err(bad(format!(
                    "class layout {layout} declares neither TexStoredFormat .17 nor TexFormat .6"
                )));
            }
        },
    };
    let palette = match m.get(PALETTE) {
        None => Vec::new(),
        Some((Value::List(items), _)) => items
            .iter()
            .map(|c| c.as_i64().map(|c| c as u32))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| bad("the palette .8 holds something other than colours"))?,
        Some(_) => return Err(bad("the palette .8 is not an array")),
    };
    let blob = match m.get(PIXELS) {
        Some((Value::Bytes(b), _)) => b.as_slice(),
        Some(_) => return Err(bad("the pixel array .18 is not a byte array")),
        None => return Err(bad("the texture has no pixel array .18")),
    };

    let faces: u32 = if flags & FLAG_CUBE_MAP != 0 { 6 } else { 1 };
    if faces == 6 && !format.is_compressed() {
        return Err(bad(
            "a cube map must be DXT (the files only have DXT1 cube maps)",
        ));
    }
    // The level counts a frame may have: DXT always carries its whole mip chain; the 24- and
    // 32-bit layouts their base level or (the layout-7 case of the spec) the whole chain.
    let chain = u32::BITS - width.max(height).leading_zeros();
    let level_counts: &[u32] = if format.is_compressed() {
        &[chain]
    } else if chain > 1 {
        &[1, chain]
    } else {
        &[1]
    };
    let frame_bytes = |levels: u32| -> u64 {
        (0..levels)
            .map(|l| format.level_bytes((width >> l).max(1), (height >> l).max(1)))
            .sum::<u64>()
            * u64::from(faces)
    };
    let all_frames = |levels: u32| frame_bytes(levels) * u64::from(frames);
    let largest = all_frames(*level_counts.last().expect("one level count at least"));
    if largest > MAX_PIXEL_BYTES {
        return Err(bad(format!(
            "{largest} pixel bytes are more than the {MAX_PIXEL_BYTES} accepted"
        )));
    }

    let mut data = match layout {
        5 | 7 => blob.to_vec(),
        6 => {
            let (count, stream) = blob
                .split_first_chunk::<4>()
                .ok_or_else(|| bad("layout 6: the pixel array has no compressed byte count"))?;
            let count = u32::from_le_bytes(*count) as usize;
            if count != stream.len() {
                return Err(bad(format!(
                    "layout 6: the compressed byte count {count} is not the {} bytes that follow it",
                    stream.len()
                )));
            }
            inflate(stream, largest)?
        }
        9 if flags & FLAG_ZLIB != 0 => inflate(blob, largest)?,
        9 => blob.to_vec(),
        other => {
            return Err(bad(format!(
                "class layout {other} of CBaseTexture is not known (5, 6, 7 and 9 are)"
            )));
        }
    };
    let got = data.len() as u64;
    let Some(&mip_levels) = level_counts.iter().find(|&&l| all_frames(l) == got) else {
        let expected: Vec<String> = level_counts
            .iter()
            .map(|&l| all_frames(l).to_string())
            .collect();
        return Err(bad(format!(
            "the pixel data has {} bytes, {} expected for {width} x {height} {format:?} \
             (layout {layout}, {frames} frame(s), flags {flags:#x})",
            if got > largest {
                "more than".to_string()
            } else {
                got.to_string()
            },
            expected.join(" or ")
        )));
    };
    data.truncate(frame_bytes(mip_levels) as usize);
    Ok(Texture {
        width,
        height,
        format,
        format_code,
        flags,
        wrap_u: m.word(WRAP_U)?,
        wrap_v: m.word(WRAP_V)?,
        border_color: m.word(BORDER)?,
        faces,
        mip_levels,
        data,
        format_version: version,
        frames,
        palette,
    })
}

/// The pixel layout of a layout-5 `TexStoredFormat` code.
fn stored_format(code: i64) -> Option<PixelFormat> {
    Some(match code {
        0 => PixelFormat::Bgr8,
        1 => PixelFormat::Bgra8,
        2 => PixelFormat::Dxt1,
        3 => PixelFormat::Dxt3,
        4 => PixelFormat::Dxt5,
        _ => return None,
    })
}

/// Inflate exactly one complete zlib stream that fills `stream`, reading at most `limit + 1`
/// bytes (one more than the largest valid size, to tell "too much" from "exact").
fn inflate(stream: &[u8], limit: u64) -> Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(stream);
    let mut data = Vec::with_capacity(limit as usize);
    (&mut z)
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|e| bad(format!("the pixel data is not a valid zlib stream: {e}")))?;
    if data.len() as u64 > limit {
        // Too long: the size check names it.
        return Ok(data);
    }
    // The stream must end where the array ends.
    let used = z.total_in();
    if used != stream.len() as u64 {
        return Err(bad(format!(
            "the zlib stream ends at byte {used} of {}: bytes after the end of the stream",
            stream.len()
        )));
    }
    Ok(data)
}

#[cfg(test)]
mod tests;
