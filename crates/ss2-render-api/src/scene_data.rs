//! CPU-side drawable data: meshes, textures and materials as plain values (RFC 0013).
//!
//! These types carry what a level loader produced to whatever draws it, without naming a file
//! format or a GPU API. `ss2-world` builds them from the game's files; `ss2-render` uploads them.
//!
//! ```
//! use ss2_render_api::scene_data::{MeshData, MeshVertex};
//!
//! let v = |x: f32, z: f32| MeshVertex { position: [x, 0.0, z], normal: [0.0, 1.0, 0.0], uv: [x, z] };
//! let quad = MeshData { vertices: vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)],
//!                       indices: vec![0, 1, 2, 0, 2, 3] };
//! quad.validate().unwrap();
//! assert_eq!(quad.bounds().unwrap().max.x, 1.0);
//! ```

use ss2_core::math::Vec3;
use ss2_core::{Error, Result};

use crate::Aabb;

/// One vertex of a static mesh: 32 bytes, laid out for direct upload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    /// Position in the mesh's own space.
    pub position: [f32; 3],
    /// Unit normal in the mesh's own space.
    pub normal: [f32; 3],
    /// Texture coordinate, as authored (no flip applied).
    pub uv: [f32; 2],
}

/// An indexed triangle list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    /// The vertices.
    pub vertices: Vec<MeshVertex>,
    /// Three indices per triangle, into `vertices`.
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Check that the mesh can be drawn: whole triangles, indices in range, finite values.
    ///
    /// # Errors
    /// [`Error::Invalid`] describing the first problem found.
    pub fn validate(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("mesh", detail));
        if !self.indices.len().is_multiple_of(3) {
            return bad(format!(
                "{} indices is not whole triangles",
                self.indices.len()
            ));
        }
        if let Some(i) = self
            .indices
            .iter()
            .find(|i| **i as usize >= self.vertices.len())
        {
            return bad(format!("index {i} but {} vertices", self.vertices.len()));
        }
        if let Some(i) = self
            .vertices
            .iter()
            .position(|v| !(v.position.iter().chain(&v.normal).chain(&v.uv)).all(|c| c.is_finite()))
        {
            return bad(format!("vertex {i} is not finite"));
        }
        Ok(())
    }

    /// The box around all vertices, or `None` for a mesh without vertices.
    pub fn bounds(&self) -> Option<Aabb> {
        Aabb::from_points(self.vertices.iter().map(|v| Vec3::from(v.position)))
    }
}

/// How the bytes of a [`TextureData`] level are encoded. All are colour data in sRGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexelFormat {
    /// 4 bytes per texel: red, green, blue, alpha.
    Rgba8,
    /// BC1 (DXT1): 8 bytes per 4x4 block.
    Bc1,
    /// BC2 (DXT3): 16 bytes per 4x4 block.
    Bc2,
    /// BC3 (DXT5): 16 bytes per 4x4 block.
    Bc3,
}

impl TexelFormat {
    /// The number of bytes of one mip level of `width` x `height` texels.
    pub fn level_size(self, width: u32, height: u32) -> u64 {
        let (w, h) = (u64::from(width), u64::from(height));
        let blocks = w.div_ceil(4) * h.div_ceil(4);
        match self {
            Self::Rgba8 => w * h * 4,
            Self::Bc1 => blocks * 8,
            Self::Bc2 | Self::Bc3 => blocks * 16,
        }
    }
}

/// A texture with its mip chain.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureData {
    /// Width of level 0 in texels.
    pub width: u32,
    /// Height of level 0 in texels.
    pub height: u32,
    /// Encoding of every level.
    pub format: TexelFormat,
    /// Level 0 first; each next level halves both sides (never below 1). At least one level.
    pub mips: Vec<Vec<u8>>,
    /// Repeat (`true`) or clamp (`false`) along U.
    pub repeat_u: bool,
    /// Repeat (`true`) or clamp (`false`) along V.
    pub repeat_v: bool,
}

impl TextureData {
    /// A single texel of one colour (RGBA8), repeating: the stand-in for a missing texture.
    pub fn solid(rgba: [u8; 4]) -> Self {
        Self {
            width: 1,
            height: 1,
            format: TexelFormat::Rgba8,
            mips: vec![rgba.to_vec()],
            repeat_u: true,
            repeat_v: true,
        }
    }

    /// Check sizes: non-zero, at most a full mip chain, each level exactly as long as its format
    /// needs.
    ///
    /// # Errors
    /// [`Error::Invalid`] describing the first problem found.
    pub fn validate(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("texture", detail));
        if self.width == 0 || self.height == 0 {
            return bad(format!("size {}x{}", self.width, self.height));
        }
        let full_chain = 32 - self.width.max(self.height).leading_zeros();
        if self.mips.is_empty() || self.mips.len() > full_chain as usize {
            return bad(format!(
                "{} mip levels, a {}x{} texture has 1 to {full_chain}",
                self.mips.len(),
                self.width,
                self.height
            ));
        }
        for (i, level) in self.mips.iter().enumerate() {
            let (w, h) = ((self.width >> i).max(1), (self.height >> i).max(1));
            let want = self.format.level_size(w, h);
            if level.len() as u64 != want {
                return bad(format!(
                    "level {i} ({w}x{h} {:?}) has {} bytes, needs {want}",
                    self.format,
                    level.len()
                ));
            }
        }
        Ok(())
    }
}

/// How a material's alpha is used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlphaMode {
    /// Alpha is ignored.
    Opaque,
    /// Texels with alpha below `cutoff` are not drawn.
    Mask {
        /// Threshold in `[0, 1]`.
        cutoff: f32,
    },
    /// Alpha blends with what is behind.
    Blend,
}

/// What a surface looks like, as far as the playable-MVP renderer needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialData {
    /// Linear RGBA multiplier on the texture.
    pub base_color: [f32; 4],
    /// Index into the owner's texture list; `None` draws `base_color` alone.
    pub base_texture: Option<u32>,
    /// Alpha handling.
    pub alpha: AlphaMode,
    /// Draw both faces.
    pub double_sided: bool,
}

impl MaterialData {
    /// The stand-in for a missing material: opaque mid grey, no texture, double-sided.
    pub const FALLBACK: Self = Self {
        base_color: [0.5, 0.5, 0.5, 1.0],
        base_texture: None,
        alpha: AlphaMode::Opaque,
        double_sided: true,
    };

    /// Check that colours and the cutoff are finite and in range.
    ///
    /// # Errors
    /// [`Error::Invalid`] describing the first problem found.
    pub fn validate(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("material", detail));
        if !self.base_color.iter().all(|c| c.is_finite()) {
            return bad(format!("base colour {:?}", self.base_color));
        }
        if let AlphaMode::Mask { cutoff } = self.alpha
            && !(0.0..=1.0).contains(&cutoff)
        {
            return bad(format!("alpha cutoff {cutoff}"));
        }
        Ok(())
    }
}

/// A face of a sky cube (RFC 0013 amendment 1), in the order sky data lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CubeFace {
    /// Towards +X.
    PosX,
    /// Towards -X.
    NegX,
    /// Towards +Y, the sky above.
    PosY,
    /// Towards -Y, below.
    NegY,
    /// Towards +Z.
    PosZ,
    /// Towards -Z.
    NegZ,
}

impl CubeFace {
    /// Every face, in index order.
    pub const ALL: [Self; 6] = [
        Self::PosX,
        Self::NegX,
        Self::PosY,
        Self::NegY,
        Self::PosZ,
        Self::NegZ,
    ];

    /// The face's position in [`CubeFace::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// The orientation convention, seen from inside the cube: the direction the face looks towards, and the world
    /// directions of its image's right and down (texel (0, 0) is the image's top-left corner).
    pub fn basis(self) -> FaceBasis {
        let (forward, right, down) = match self {
            Self::PosX => (Vec3::X, Vec3::Z, Vec3::NEG_Y),
            Self::NegX => (Vec3::NEG_X, Vec3::NEG_Z, Vec3::NEG_Y),
            Self::PosY => (Vec3::Y, Vec3::X, Vec3::NEG_Z),
            Self::NegY => (Vec3::NEG_Y, Vec3::X, Vec3::Z),
            Self::PosZ => (Vec3::Z, Vec3::NEG_X, Vec3::NEG_Y),
            Self::NegZ => (Vec3::NEG_Z, Vec3::X, Vec3::NEG_Y),
        };
        FaceBasis {
            forward,
            right,
            down,
        }
    }
}

/// See [`CubeFace::basis`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceBasis {
    /// Where the face is, seen from the cube's centre.
    pub forward: Vec3,
    /// The world direction of the image's +u (right).
    pub right: Vec3,
    /// The world direction of the image's +v (down).
    pub down: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> MeshVertex {
        MeshVertex {
            position: [x, y, z],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        }
    }

    fn triangle() -> MeshData {
        MeshData {
            vertices: vec![v(0.0, 0.0, 0.0), v(2.0, 0.0, 0.0), v(0.0, -1.0, 3.0)],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn every_cube_face_is_upright_and_seen_from_inside() {
        for (i, face) in CubeFace::ALL.into_iter().enumerate() {
            assert_eq!(face.index(), i);
            let b = face.basis();
            // Right, down and forward are unit and at right angles.
            for v in [b.forward, b.right, b.down] {
                assert!((v.length() - 1.0).abs() < 1e-6);
            }
            assert!(b.right.dot(b.down).abs() < 1e-6 && b.right.dot(b.forward).abs() < 1e-6);
            // Seen from inside, right = forward x up (up is -down), as for a camera looking at the face: not mirrored.
            assert!(
                (b.forward.cross(-b.down) - b.right).length() < 1e-6,
                "{face:?}"
            );
            // The side faces stand upright.
            if b.forward.y == 0.0 {
                assert_eq!(b.down, Vec3::NEG_Y, "{face:?}");
            }
        }
        // Looking up from facing -Z, the image's top is behind (+Z); looking down, it is ahead (-Z).
        assert_eq!(CubeFace::PosY.basis().down, Vec3::NEG_Z);
        assert_eq!(CubeFace::NegY.basis().down, Vec3::Z);
    }

    #[test]
    fn a_vertex_is_32_bytes() {
        assert_eq!(size_of::<MeshVertex>(), 32);
    }

    #[test]
    fn a_good_mesh_validates_and_has_bounds() {
        let m = triangle();
        m.validate().unwrap();
        let b = m.bounds().unwrap();
        assert_eq!(b.min, Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(b.max, Vec3::new(2.0, 0.0, 3.0));
        assert_eq!(MeshData::default().bounds(), None);
        MeshData::default().validate().unwrap();
    }

    #[test]
    fn bad_meshes_are_rejected() {
        let mut m = triangle();
        m.indices.push(0);
        assert!(m.validate().is_err(), "partial triangle");
        let mut m = triangle();
        m.indices[2] = 3;
        assert!(m.validate().is_err(), "index out of range");
        let mut m = triangle();
        m.vertices[1].position[0] = f32::NAN;
        assert!(m.validate().is_err(), "NaN position");
        let mut m = triangle();
        m.vertices[1].uv[1] = f32::INFINITY;
        assert!(m.validate().is_err(), "infinite uv");
        let mut m = triangle();
        m.vertices[1].normal[2] = f32::NAN;
        assert!(m.validate().is_err(), "NaN normal");
    }

    #[test]
    fn level_sizes_follow_the_format() {
        assert_eq!(TexelFormat::Rgba8.level_size(3, 5), 60);
        assert_eq!(TexelFormat::Bc1.level_size(4, 4), 8);
        assert_eq!(
            TexelFormat::Bc1.level_size(1, 1),
            8,
            "a partial block is a whole block"
        );
        assert_eq!(TexelFormat::Bc1.level_size(5, 8), 2 * 2 * 8);
        assert_eq!(TexelFormat::Bc2.level_size(8, 8), 4 * 16);
        assert_eq!(TexelFormat::Bc3.level_size(256, 128), 64 * 32 * 16);
        assert_eq!(
            TexelFormat::Rgba8.level_size(65_536, 65_536),
            1 << 34,
            "no overflow"
        );
    }

    fn chain(format: TexelFormat, w: u32, h: u32, levels: usize) -> TextureData {
        let mips = (0..levels)
            .map(|i| {
                let (lw, lh) = ((w >> i).max(1), (h >> i).max(1));
                vec![0u8; usize::try_from(format.level_size(lw, lh)).unwrap()]
            })
            .collect();
        TextureData {
            width: w,
            height: h,
            format,
            mips,
            repeat_u: true,
            repeat_v: true,
        }
    }

    #[test]
    fn good_textures_validate() {
        chain(TexelFormat::Rgba8, 8, 2, 4).validate().unwrap();
        chain(TexelFormat::Bc1, 16, 16, 5).validate().unwrap();
        chain(TexelFormat::Bc3, 6, 6, 1).validate().unwrap();
        let s = TextureData::solid([1, 2, 3, 4]);
        s.validate().unwrap();
        assert_eq!(
            (s.width, s.height, s.mips[0].as_slice()),
            (1, 1, &[1u8, 2, 3, 4][..])
        );
        assert!(s.repeat_u && s.repeat_v);
    }

    #[test]
    fn bad_textures_are_rejected() {
        assert!(
            chain(TexelFormat::Rgba8, 0, 4, 1).validate().is_err(),
            "zero width"
        );
        assert!(
            chain(TexelFormat::Rgba8, 4, 4, 0).validate().is_err(),
            "no levels"
        );
        assert!(
            chain(TexelFormat::Rgba8, 4, 4, 4).validate().is_err(),
            "too many levels"
        );
        let mut t = chain(TexelFormat::Bc1, 8, 8, 2);
        t.mips[1].pop();
        assert!(t.validate().is_err(), "short level");
    }

    #[test]
    fn materials_validate_their_numbers() {
        MaterialData::FALLBACK.validate().unwrap();
        let mut m = MaterialData::FALLBACK;
        m.base_color[3] = f32::NAN;
        assert!(m.validate().is_err());
        let mut m = MaterialData::FALLBACK;
        m.alpha = AlphaMode::Mask { cutoff: 1.5 };
        assert!(m.validate().is_err());
        m.alpha = AlphaMode::Mask { cutoff: 0.5 };
        m.validate().unwrap();
    }
}
