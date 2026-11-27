//! The sky's faces: which [`CubeFace`] each game face is, how its image is turned into the RFC 0013
//! orientation convention ([`CubeFace::basis`]) and the tint (roadmap T07-033; spec
//! `docs/re/environment.md`, "Sky").
//!
//! ## Where each game face goes
//!
//! The spec places the `CCubeRenderable` face members at fixed engine axes (a convention, not a
//! measurement: North = -Z and East = +X follow the right-handed, y-up, -Z-forward frame) and
//! gives, from matching the pixels along every pair of face edges, how the images meet:
//!
//! - the four ring faces stand upright and each one's **left** edge continues unflipped into the
//!   **right** edge of its western neighbour (North's left meets West's right, West's left
//!   South's right, South's left East's right, East's left North's right);
//! - each ring face's **top** edge meets the ceiling: North's top along the ceiling's bottom and
//!   West's top along the ceiling's left, unflipped; South's top along the ceiling's top and
//!   East's top along the ceiling's right, mirrored;
//! - the floor's top meets North's bottom and the floor's right East's bottom, unflipped (the
//!   floor's seams with West and South are not confirmed by the spec).
//!
//! Put into world directions, seen from inside the cube, each image's right and down are:
//!
//! | Face (member) | Looks towards | Image right | Image down | [`CubeFace`] | Turn needed |
//! |---|---|---|---|---|---|
//! | North (`.10`) | -Z | +X | -Y | `NegZ` | none |
//! | West (`.11`) | -X | -Z | -Y | `NegX` | none |
//! | South (`.12`) | +Z | -X | -Y | `PosZ` | none |
//! | East (`.13`) | +X | +Z | -Y | `PosX` | none |
//! | Floor (`.14`) | -Y | +X | +Z | `NegY` | none |
//! | Ceiling (`.15`) | +Y | +X | -Z | `PosY` | none |
//!
//! The derivation for the ring: an upright image seen from inside whose left neighbour is the
//! face to the viewer's left is unmirrored, so its right is `forward x up` (facing -Z, right is
//! +X: East, the face North's right edge meets). For the ceiling, North's top edge runs from -X to
//! +X along z = -1 and meets the ceiling's bottom row unflipped, so the ceiling's right is +X and
//! its bottom is at -Z (down = -Z); its left column is then at x = -1 (West, unflipped, as the
//! spec says) and its top row at z = +1 (South, mirrored, as the spec says). For the floor,
//! North's bottom edge meets its top row unflipped: right +X, top at -Z (down = +Z), which puts
//! East along its right column, unflipped, as the spec says.
//!
//! (The spec's prose summary says reading the ring images left to right moves west, and its
//! worked example tiles them left to right in North, West, South, East order; both disagree with
//! its own seam table, where North's left meets West's. The table is what was measured and is
//! what is used here; the rendered skies show no seams that way.)
//!
//! Every game face therefore already is in the convention: no face needs a turn. The loader still
//! computes the turn from the table above ([`turn`]) and applies it ([`orient`]) so that a change of
//! the spec's table only changes the table.

use ss2_core::math::Vec3;
use ss2_format_wld::SkyFace;
use ss2_render_api::scene_data::{CubeFace, FaceBasis, TexelFormat, TextureData};

/// The cube face of a game face and the world directions of its image's right and down, seen
/// from inside (the table of the module documentation).
pub(crate) fn game_face(face: SkyFace) -> (CubeFace, Vec3, Vec3) {
    match face {
        SkyFace::North => (CubeFace::NegZ, Vec3::X, Vec3::NEG_Y),
        SkyFace::West => (CubeFace::NegX, Vec3::NEG_Z, Vec3::NEG_Y),
        SkyFace::South => (CubeFace::PosZ, Vec3::NEG_X, Vec3::NEG_Y),
        SkyFace::East => (CubeFace::PosX, Vec3::Z, Vec3::NEG_Y),
        SkyFace::Floor => (CubeFace::NegY, Vec3::X, Vec3::Z),
        SkyFace::Ceiling => (CubeFace::PosY, Vec3::X, Vec3::NEG_Z),
    }
}

/// How an image's texels move to put it into another basis of the same face: a source texel at
/// centred coordinates `(su, sv)` (both from -1 to 1, right and down) lands at `(u, v)` with
/// `su = uu * u + uv * v` and `sv = vu * u + vv * v`. Each coefficient is -1, 0 or 1; `uv != 0`
/// transposes (the sides swap).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Turn {
    uu: i32,
    uv: i32,
    vu: i32,
    vv: i32,
}

impl Turn {
    /// No change.
    pub(crate) const NONE: Self = Self {
        uu: 1,
        uv: 0,
        vu: 0,
        vv: 1,
    };

    /// Whether the sides swap.
    fn transposes(self) -> bool {
        self.uv != 0
    }

    /// The source texel of output texel `(x, y)` in an output image of `w x h` (the source is
    /// `h x w` when the turn transposes, else `w x h`).
    fn source(self, x: u32, y: u32, w: u32, h: u32) -> (u32, u32) {
        let (sw, sh) = if self.transposes() { (h, w) } else { (w, h) };
        // Doubled centred coordinates keep everything in integers.
        let u = 2 * i64::from(x) - (i64::from(w) - 1);
        let v = 2 * i64::from(y) - (i64::from(h) - 1);
        let su = i64::from(self.uu) * u + i64::from(self.uv) * v;
        let sv = i64::from(self.vu) * u + i64::from(self.vv) * v;
        let sx = (su + i64::from(sw) - 1) / 2;
        let sy = (sv + i64::from(sh) - 1) / 2;
        (sx as u32, sy as u32)
    }
}

/// The turn that takes an image whose right and down point along `right` and `down` into the
/// convention `to`; `None` when the two do not describe the same face plane with unit axes.
pub(crate) fn turn(right: Vec3, down: Vec3, to: FaceBasis) -> Option<Turn> {
    let dot = |a: Vec3, b: Vec3| {
        let d = a.dot(b).round();
        ((a.dot(b) - d).abs() < 1e-4).then_some(d as i32)
    };
    let t = Turn {
        uu: dot(to.right, right)?,
        uv: dot(to.down, right)?,
        vu: dot(to.right, down)?,
        vv: dot(to.down, down)?,
    };
    // The 2 x 2 matrix must be a signed permutation.
    let ok = (t.uu.abs() + t.uv.abs() == 1)
        && (t.vu.abs() + t.vv.abs() == 1)
        && (t.uu.abs() + t.vu.abs() == 1);
    ok.then_some(t)
}

/// Bits of one 4 x 4 block that hold per-texel indices, as `(first bit, bits per texel)` with
/// texel `4 * row + column` at `first + k * bits` of the block read as a little-endian `u128`
/// (`BC1`: the colour indices; `BC2`: the explicit alpha and the colour indices; `BC3`: the
/// alpha indices and the colour indices). Everything else of a block (end points) moves with it.
fn index_fields(format: TexelFormat) -> &'static [(u32, u32)] {
    match format {
        TexelFormat::Bc1 => &[(32, 2)],
        TexelFormat::Bc2 => &[(0, 4), (96, 2)],
        TexelFormat::Bc3 => &[(16, 3), (96, 2)],
        _ => &[],
    }
}

/// Apply `turn` to a texture: every mip level of an RGBA8 image is remapped texel by texel; a
/// block-compressed image is remapped block by block with each block's texel indices moved
/// inside it, so nothing is decoded and nothing is lost.
pub(crate) fn orient(t: TextureData, turn: Turn) -> TextureData {
    if turn == Turn::NONE {
        return t;
    }
    let (w, h) = if turn.transposes() {
        (t.height, t.width)
    } else {
        (t.width, t.height)
    };
    let mips = t
        .mips
        .iter()
        .enumerate()
        .map(|(level, data)| {
            let (lw, lh) = ((w >> level).max(1), (h >> level).max(1));
            match t.format {
                TexelFormat::Rgba8 => orient_rgba(data, turn, lw, lh),
                format => orient_blocks(data, turn, lw, lh, index_fields(format)),
            }
        })
        .collect();
    let (repeat_u, repeat_v) = if turn.transposes() {
        (t.repeat_v, t.repeat_u)
    } else {
        (t.repeat_u, t.repeat_v)
    };
    TextureData {
        width: w,
        height: h,
        format: t.format,
        mips,
        repeat_u,
        repeat_v,
    }
}

fn orient_rgba(data: &[u8], turn: Turn, w: u32, h: u32) -> Vec<u8> {
    let sw = if turn.transposes() { h } else { w };
    let mut out = Vec::with_capacity(data.len());
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = turn.source(x, y, w, h);
            let i = ((sy * sw + sx) * 4) as usize;
            out.extend_from_slice(&data[i..i + 4]);
        }
    }
    out
}

fn orient_blocks(data: &[u8], turn: Turn, w: u32, h: u32, fields: &[(u32, u32)]) -> Vec<u8> {
    let block_bytes = if fields.len() == 1 { 8 } else { 16 };
    let sw = if turn.transposes() { h } else { w };
    let blocks_across = |width: u32| width.div_ceil(4);
    let read = |bx: u32, by: u32| -> u128 {
        let i = ((by * blocks_across(sw) + bx) * block_bytes) as usize;
        let mut bytes = [0u8; 16];
        bytes[..block_bytes as usize].copy_from_slice(&data[i..i + block_bytes as usize]);
        u128::from_le_bytes(bytes)
    };
    let mut out = Vec::with_capacity(data.len());
    for by in 0..h.div_ceil(4) {
        for bx in 0..blocks_across(w) {
            // Every texel of an output block comes from one source block (the sides are powers of
            // two: a multiple of 4, or a single block).
            let (sx, sy) = turn.source(bx * 4, by * 4, w, h);
            let mut block = read(sx / 4, sy / 4);
            let source = block;
            for ty in 0..4 {
                for tx in 0..4 {
                    let (x, y) = (bx * 4 + tx, by * 4 + ty);
                    if x >= w || y >= h {
                        continue;
                    }
                    let (sx, sy) = turn.source(x, y, w, h);
                    let (k_out, k_in) = (4 * ty + tx, 4 * (sy % 4) + sx % 4);
                    for &(first, bits) in fields {
                        let mask = (1u128 << bits) - 1;
                        let value = (source >> (first + k_in * bits)) & mask;
                        let at = first + k_out * bits;
                        block = (block & !(mask << at)) | (value << at);
                    }
                }
            }
            out.extend_from_slice(&block.to_le_bytes()[..block_bytes as usize]);
        }
    }
    out
}

/// The sky's tint from its stored colour: the spec reads the value as ARGB; the low three bytes
/// are red, green and blue (`0xRRGGBB`), taken as sRGB and turned into linear values. The top
/// (alpha) byte is ignored: it is 0 in most retail skies (`0x00FFFFFF`), which would make them
/// invisible if it were an opacity. The tint's alpha is 1.
pub(crate) fn tint(color: u32) -> [f32; 4] {
    let channel = |shift: u32| srgb_to_linear(((color >> shift) & 0xFF) as f32 / 255.0);
    [channel(16), channel(8), channel(0), 1.0]
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `w x h` RGBA8 image whose texel `(x, y)` is `[x, y, 0, 255]`.
    fn coded(w: u32, h: u32) -> TextureData {
        let mut texels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                texels.extend([x as u8, y as u8, 0, 255]);
            }
        }
        TextureData {
            width: w,
            height: h,
            format: TexelFormat::Rgba8,
            mips: vec![texels],
            repeat_u: true,
            repeat_v: false,
        }
    }

    /// The world point (on the face plane, centred coordinates) of texel `(x, y)` of a `w x h`
    /// image whose right and down are `right` and `down`.
    fn world_of(x: u32, y: u32, w: u32, h: u32, right: Vec3, down: Vec3) -> Vec3 {
        let u = (2 * x + 1) as f32 / w as f32 - 1.0;
        let v = (2 * y + 1) as f32 / h as f32 - 1.0;
        right * u + down * v
    }

    /// Every way an image can lie on the -Z face: four turns, each mirrored or not.
    fn layouts() -> Vec<(Vec3, Vec3)> {
        let axes = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y];
        let mut out = Vec::new();
        for right in axes {
            for down in axes {
                if right.dot(down) == 0.0 {
                    out.push((right, down));
                }
            }
        }
        assert_eq!(out.len(), 8);
        out
    }

    #[test]
    fn every_layout_is_turned_so_each_texel_keeps_its_world_place() {
        let to = CubeFace::NegZ.basis();
        let (w, h) = (4, 2);
        for (right, down) in layouts() {
            let turn = turn(right, down, to).unwrap();
            let out = orient(coded(w, h), turn);
            let (ow, oh) = (out.width, out.height);
            assert_eq!(ow * oh, w * h);
            for y in 0..oh {
                for x in 0..ow {
                    let i = ((y * ow + x) * 4) as usize;
                    let (sx, sy) = (u32::from(out.mips[0][i]), u32::from(out.mips[0][i + 1]));
                    let here = world_of(x, y, ow, oh, to.right, to.down);
                    let there = world_of(sx, sy, w, h, right, down);
                    assert!(
                        (here - there).length() < 1e-5,
                        "layout right {right} down {down}: texel ({x}, {y}) shows ({sx}, {sy})"
                    );
                }
            }
            // The repeat flags travel with the axes.
            let transposed = right.dot(to.right) == 0.0;
            assert_eq!(out.repeat_u, !transposed);
        }
    }

    #[test]
    fn an_image_already_in_the_convention_is_left_alone() {
        let to = CubeFace::PosY.basis();
        let turn = turn(to.right, to.down, to).unwrap();
        assert_eq!(turn, Turn::NONE);
        assert_eq!(orient(coded(4, 2), turn), coded(4, 2));
        // Axes of another face plane, or not unit, do not make a turn.
        assert_eq!(super::turn(Vec3::Z, Vec3::NEG_Y, to), None);
        assert_eq!(super::turn(Vec3::X * 2.0, Vec3::NEG_Z, to), None);
    }

    /// The texel indices of one BC1 block (row-major).
    fn bc1_indices(block: &[u8]) -> [u8; 16] {
        let bits = u32::from_le_bytes(block[4..8].try_into().unwrap());
        std::array::from_fn(|k| ((bits >> (2 * k)) & 3) as u8)
    }

    #[test]
    fn block_compressed_images_move_blocks_and_the_indices_inside_them() {
        // An 8 x 4 BC1 image of two blocks: the left block's end points 0x1111/0x2222 with only
        // its top-left texel at index 1, the right block 0x3333/0x4444 with index 2 at its
        // top-right texel. Mirroring left-right swaps the blocks and mirrors inside them.
        let mut data = Vec::new();
        data.extend(0x1111u16.to_le_bytes());
        data.extend(0x2222u16.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend(0x3333u16.to_le_bytes());
        data.extend(0x4444u16.to_le_bytes());
        data.extend((2u32 << 6).to_le_bytes());
        // A 4 x 2 mip: one block, index 3 at texel (1, 1).
        let mut mip = Vec::new();
        mip.extend(0x5555u16.to_le_bytes());
        mip.extend(0x6666u16.to_le_bytes());
        mip.extend((3u32 << (2 * 5)).to_le_bytes());
        let t = TextureData {
            width: 8,
            height: 4,
            format: TexelFormat::Bc1,
            mips: vec![data, mip],
            repeat_u: true,
            repeat_v: true,
        };
        let to = CubeFace::NegZ.basis();
        let mirror = turn(Vec3::NEG_X, Vec3::NEG_Y, to).unwrap();
        let out = orient(t.clone(), mirror);
        assert_eq!((out.width, out.height), (8, 4));
        let (left, right) = out.mips[0].split_at(8);
        assert_eq!(&left[..4], &[0x33, 0x33, 0x44, 0x44]);
        let mut want = [0u8; 16];
        want[0] = 2; // was the right block's top-right
        assert_eq!(bc1_indices(left), want);
        assert_eq!(&right[..4], &[0x11, 0x11, 0x22, 0x22]);
        let mut want = [0u8; 16];
        want[3] = 1; // was the left block's top-left
        assert_eq!(bc1_indices(right), want);
        // In the 4 x 2 level only the valid texels move: (1, 1) goes to (2, 1).
        let mut want = [0u8; 16];
        want[4 + 2] = 3;
        assert_eq!(bc1_indices(&out.mips[1]), want);
        // A quarter turn swaps the sides: 4 x 8, two blocks stacked.
        let quarter = turn(Vec3::NEG_Y, Vec3::X, to).unwrap();
        let out = orient(t, quarter);
        assert_eq!((out.width, out.height, out.mips[0].len()), (4, 8, 16));
    }

    #[test]
    fn the_face_table_follows_the_spec_seams() {
        // An edge of a face image as a world segment (start, end), read left to right or top to
        // bottom.
        #[derive(Clone, Copy)]
        enum Edge {
            Top,
            Bottom,
            Left,
            Right,
        }
        let edge = |face: SkyFace, e: Edge| -> (Vec3, Vec3) {
            let (cube, right, down) = game_face(face);
            let f = cube.basis().forward;
            let p = |u: f32, v: f32| f + right * u + down * v;
            match e {
                Edge::Top => (p(-1.0, -1.0), p(1.0, -1.0)),
                Edge::Bottom => (p(-1.0, 1.0), p(1.0, 1.0)),
                Edge::Left => (p(-1.0, -1.0), p(-1.0, 1.0)),
                Edge::Right => (p(1.0, -1.0), p(1.0, 1.0)),
            }
        };
        use Edge::*;
        use SkyFace::*;
        // `environment.md`, "How the face images tile": (first, its edge, second, its edge,
        // mirrored).
        let seams = [
            (North, Left, West, Right, false),
            (North, Right, East, Left, false),
            (West, Left, South, Right, false),
            (South, Left, East, Right, false),
            (North, Top, Ceiling, Bottom, false),
            (West, Top, Ceiling, Left, false),
            (South, Top, Ceiling, Top, true),
            (East, Top, Ceiling, Right, true),
            (North, Bottom, Floor, Top, false),
            (East, Bottom, Floor, Right, false),
        ];
        for (a, ea, b, eb, mirrored) in seams {
            let (a0, a1) = edge(a, ea);
            let (mut b0, mut b1) = edge(b, eb);
            if mirrored {
                std::mem::swap(&mut b0, &mut b1);
            }
            assert!(
                (a0 - b0).length() < 1e-6 && (a1 - b1).length() < 1e-6,
                "{a:?} and {b:?} do not meet as the spec says"
            );
        }
        // Each game face sits on its cube face, and every one is already in the convention.
        for face in SkyFace::ALL {
            let (cube, right, down) = game_face(face);
            assert_eq!(
                turn(right, down, cube.basis()),
                Some(Turn::NONE),
                "{face:?}"
            );
        }
    }

    #[test]
    fn the_tint_reads_the_low_three_bytes_as_srgb() {
        assert_eq!(tint(0x00FF_FFFF), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(tint(0xFFFF_FFFF), [1.0, 1.0, 1.0, 1.0]);
        let t = tint(0xFA7E_7E7E);
        assert!(
            (t[0] - 0.2086).abs() < 1e-3 && t[0] == t[1] && t[1] == t[2],
            "{t:?}"
        );
        let t = tint(0x00FF_0000);
        assert_eq!(t, [1.0, 0.0, 0.0, 1.0]);
    }
}
