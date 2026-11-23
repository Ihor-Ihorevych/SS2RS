//! **Texture sampling rules**: filtering, anisotropy, mip bias, sRGB against linear, normal-map decoding and cube-map addressing (roadmap T04-008,
//! `docs/TEXTURE_SAMPLING.md`).
//!
//! Device-free: the decisions a renderer needs *before* it creates a sampler or a texture view, and the maths a shader and a CPU-side test must agree on.
//!
//! * [`TextureUsage`] says what a texture holds; [`TextureUsage::colorspace`] says how to read its bytes (colour is sRGB, everything else is linear data).
//! * [`sampler_for`] picks a [`SamplerDesc`] from the usage, the player's [`QualitySettings`], the device's anisotropy limit and the render scale.
//! * [`mips_to_skip`] says how many of the biggest mips to leave out for the "texture detail" setting, without shrinking small textures.
//! * [`srgb_to_linear`] / [`linear_to_srgb`] are the standard sRGB transfer functions; [`decode_normal`], [`decode_normal_xy`] and [`encode_normal`] read and write
//!   tangent-space normal maps (three channels or two).
//! * [`cube_face_direction`] / [`direction_to_cube_face`] address a cube map with the usual face order (+X, -X, +Y, -Y, +Z, -Z).

use ss2_core::math::Vec3;

use crate::quality::{QualitySettings, TextureDetail};

/// What a texture holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureUsage {
    /// Colour as an artist sees it (albedo, decals, particles): **sRGB** encoded.
    Color,
    /// A tangent-space normal map: **linear** data.
    Normal,
    /// Other data (roughness, masks, lookup tables): **linear**.
    Data,
    /// High dynamic range colour: linear floats.
    Hdr,
    /// A cube map (sky, reflections): colour, so sRGB unless it is [`Hdr`](Self::Hdr) data stored in a float format.
    Cubemap,
    /// Interface graphics: sRGB, drawn at screen resolution, never mip-mapped.
    Ui,
}

/// How the bytes of a texture are to be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    /// Gamma-encoded: the GPU converts to linear when sampling (use an `*Srgb` format).
    Srgb,
    /// Plain numbers.
    Linear,
}

impl TextureUsage {
    /// How to read the texture. Using the wrong one is the classic washed-out or too-dark picture: normal maps and masks read as sRGB are bent, colours read as linear
    /// look pale.
    pub fn colorspace(self) -> ColorSpace {
        match self {
            Self::Color | Self::Cubemap | Self::Ui => ColorSpace::Srgb,
            Self::Normal | Self::Data | Self::Hdr => ColorSpace::Linear,
        }
    }

    /// Whether the texture has (and is sampled with) a mip chain.
    pub fn uses_mips(self) -> bool {
        self != Self::Ui
    }
}

/// How to combine texels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// The nearest texel.
    Nearest,
    /// A blend of the neighbours.
    Linear,
}

/// What happens outside `0..1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressMode {
    /// Tile.
    Repeat,
    /// Stretch the edge texel.
    ClampToEdge,
}

/// A sampler, as the wgpu side will create it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SamplerDesc {
    /// Filter when the texture is minified.
    pub min_filter: Filter,
    /// Filter when it is magnified.
    pub mag_filter: Filter,
    /// Filter between mip levels; `None` for a texture without mips.
    pub mip_filter: Option<Filter>,
    /// Anisotropic filtering, 1 (off) to 16.
    pub anisotropy: u8,
    /// Behaviour outside the texture.
    pub address: AddressMode,
    /// Added to the mip level the hardware picks (negative: sharper).
    pub mip_bias: f32,
}

/// The mip bias that keeps textures as sharp **in the final picture** when the scene is drawn at a lower resolution: the scene is `scale` of the size, so each
/// pixel covers `1 / scale` times more texture, and a bias of `log2(scale)` (negative) asks for the sharper mip. Scales are clamped to `0.25..=1`; NaN counts as 1.
pub fn mip_bias_for_render_scale(scale: f32) -> f32 {
    if scale.is_nan() {
        return 0.0;
    }
    scale.clamp(0.25, 1.0).log2()
}

/// The sampler for a texture of this usage under the player's settings.
///
/// * [`Ui`](TextureUsage::Ui): linear, no mips, clamped, no anisotropy, no bias: it is drawn 1 to 1.
/// * [`Cubemap`](TextureUsage::Cubemap): trilinear, clamped (a repeating cube map shows seams), no anisotropy.
/// * Everything else: trilinear, repeating, **anisotropy from the settings but never above what the device supports**, and the render-scale mip bias.
pub fn sampler_for(
    usage: TextureUsage,
    settings: &QualitySettings,
    device_max_anisotropy: u8,
    render_scale: f32,
) -> SamplerDesc {
    match usage {
        TextureUsage::Ui => SamplerDesc {
            min_filter: Filter::Linear,
            mag_filter: Filter::Linear,
            mip_filter: None,
            anisotropy: 1,
            address: AddressMode::ClampToEdge,
            mip_bias: 0.0,
        },
        TextureUsage::Cubemap => SamplerDesc {
            min_filter: Filter::Linear,
            mag_filter: Filter::Linear,
            mip_filter: Some(Filter::Linear),
            anisotropy: 1,
            address: AddressMode::ClampToEdge,
            mip_bias: mip_bias_for_render_scale(render_scale),
        },
        TextureUsage::Color | TextureUsage::Normal | TextureUsage::Data | TextureUsage::Hdr => {
            SamplerDesc {
                min_filter: Filter::Linear,
                mag_filter: Filter::Linear,
                mip_filter: Some(Filter::Linear),
                anisotropy: settings
                    .anisotropy
                    .clamp(1, device_max_anisotropy.clamp(1, 16)),
                address: AddressMode::Repeat,
                mip_bias: mip_bias_for_render_scale(render_scale),
            }
        }
    }
}

/// How many mip levels a texture of this size has: `floor(log2(max(width, height))) + 1`, and 0 for an empty texture.
pub fn mip_count(width: u32, height: u32) -> u32 {
    let longest = width.max(height);
    if longest == 0 {
        0
    } else {
        32 - longest.leading_zeros()
    }
}

/// The size of mip `level` (each level halves, never below 1).
pub fn mip_size(width: u32, height: u32, level: u32) -> (u32, u32) {
    let shift = level.min(31);
    ((width >> shift).max(1), (height >> shift).max(1))
}

/// The smallest texture the "texture detail" setting will shrink: a texture whose longest side is already at most this is left whole.
pub const MIN_REDUCED_SIZE: u32 = 64;

/// How many of the **largest** mip levels to leave out when loading a texture of `width` x `height` at the given detail: 2 for quarter size, 1 for half, 0 for full,
/// **fewer** if skipping would take the longest side below [`MIN_REDUCED_SIZE`] (shrinking an icon saves nothing and looks worse), and never so many that no level is left.
pub fn mips_to_skip(width: u32, height: u32, detail: TextureDetail) -> u32 {
    let wanted = match detail {
        TextureDetail::Quarter => 2,
        TextureDetail::Half => 1,
        TextureDetail::Full => 0,
    };
    let longest = width.max(height);
    let mut skip = 0;
    while skip < wanted && (longest >> (skip + 1)) >= MIN_REDUCED_SIZE {
        skip += 1;
    }
    skip
}

/// sRGB (0 to 1) to linear light, by the standard transfer function. Input outside 0 to 1 is clamped; NaN gives 0.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c.is_nan() {
        return 0.0;
    }
    let c = c.clamp(0.0, 1.0);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light (0 to 1) to sRGB, the inverse of [`srgb_to_linear`]. Input outside 0 to 1 is clamped; NaN gives 0.
pub fn linear_to_srgb(l: f32) -> f32 {
    if l.is_nan() {
        return 0.0;
    }
    let l = l.clamp(0.0, 1.0);
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// An 8-bit sRGB channel to linear.
pub fn srgb8_to_linear(v: u8) -> f32 {
    srgb_to_linear(f32::from(v) / 255.0)
}

fn unpack(v: u8) -> f32 {
    f32::from(v) / 255.0 * 2.0 - 1.0
}

/// A tangent-space normal from a three-channel map: each channel maps `0..255` to `-1..1`, then the vector is normalized. (A texel can never decode to a
/// zero-length vector: eight bits have no exact middle, so every channel is at least 0.0039 away from 0.)
pub fn decode_normal(r: u8, g: u8, b: u8) -> Vec3 {
    Vec3::new(unpack(r), unpack(g), unpack(b)).normalize()
}

/// A tangent-space normal from a **two-channel** map (only x and y stored, as BC5 does): `z = sqrt(1 - x^2 - y^2)`, with `x` and `y` first scaled back inside the
/// unit disc if rounding put them just outside. `z` is never negative: a tangent-space normal points out of the surface.
pub fn decode_normal_xy(r: u8, g: u8) -> Vec3 {
    let (mut x, mut y) = (unpack(r), unpack(g));
    let len2 = x * x + y * y;
    if len2 > 1.0 {
        let s = len2.sqrt().recip();
        x *= s;
        y *= s;
    }
    Vec3::new(x, y, (1.0 - x * x - y * y).max(0.0).sqrt())
}

/// A unit normal as three 8-bit channels (the inverse of [`decode_normal`] up to rounding). The input is normalized first; a zero or non-finite vector gives
/// the flat normal.
pub fn encode_normal(n: Vec3) -> [u8; 3] {
    let n = if n.is_finite() {
        n.try_normalize().unwrap_or(Vec3::Z)
    } else {
        Vec3::Z
    };
    let pack = |v: f32| ((v * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
    [pack(n.x), pack(n.y), pack(n.z)]
}

/// A face of a cube map, in the usual order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CubeFace {
    /// +X (right).
    PosX,
    /// -X (left).
    NegX,
    /// +Y (up).
    PosY,
    /// -Y (down).
    NegY,
    /// +Z (towards the viewer in a right-handed, -Z-forward world).
    PosZ,
    /// -Z (forward).
    NegZ,
}

impl CubeFace {
    /// The six faces in layer order.
    pub const ALL: [CubeFace; 6] = [
        Self::PosX,
        Self::NegX,
        Self::PosY,
        Self::NegY,
        Self::PosZ,
        Self::NegZ,
    ];

    /// The layer index, 0 to 5.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// The direction through the point `(u, v)` of a face, where `u` and `v` run over `-1..1` across the face (`(-1, -1)` is the texture's first texel corner). The
/// result is **not** normalized (the face centre is a unit vector, the corners are longer), by the usual cube-map convention.
pub fn cube_face_direction(face: CubeFace, u: f32, v: f32) -> Vec3 {
    match face {
        CubeFace::PosX => Vec3::new(1.0, -v, -u),
        CubeFace::NegX => Vec3::new(-1.0, -v, u),
        CubeFace::PosY => Vec3::new(u, 1.0, v),
        CubeFace::NegY => Vec3::new(u, -1.0, -v),
        CubeFace::PosZ => Vec3::new(u, -v, 1.0),
        CubeFace::NegZ => Vec3::new(-u, -v, -1.0),
    }
}

/// The face a direction hits and where on it, as `(face, u, v)` with `u` and `v` in `-1..1`: the inverse of [`cube_face_direction`]. The face is the one of the
/// largest absolute component; on a tie the order is X, then Y, then Z. A zero or non-finite direction gives `None`.
pub fn direction_to_cube_face(dir: Vec3) -> Option<(CubeFace, f32, f32)> {
    if !dir.is_finite() || dir == Vec3::ZERO {
        return None;
    }
    let (ax, ay, az) = (dir.x.abs(), dir.y.abs(), dir.z.abs());
    let (face, major, sc, tc) = if ax >= ay && ax >= az {
        if dir.x > 0.0 {
            (CubeFace::PosX, ax, -dir.z, -dir.y)
        } else {
            (CubeFace::NegX, ax, dir.z, -dir.y)
        }
    } else if ay >= az {
        if dir.y > 0.0 {
            (CubeFace::PosY, ay, dir.x, dir.z)
        } else {
            (CubeFace::NegY, ay, dir.x, -dir.z)
        }
    } else if dir.z > 0.0 {
        (CubeFace::PosZ, az, dir.x, -dir.y)
    } else {
        (CubeFace::NegZ, az, -dir.x, -dir.y)
    };
    Some((face, sc / major, tc / major))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::QualityTier;

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    // ---- colour spaces ------------------------------------------------------------------------------

    #[test]
    fn colour_is_srgb_and_data_is_linear() {
        use TextureUsage::*;
        for (usage, space) in [
            (Color, ColorSpace::Srgb),
            (Ui, ColorSpace::Srgb),
            (Cubemap, ColorSpace::Srgb),
            (Normal, ColorSpace::Linear),
            (Data, ColorSpace::Linear),
            (Hdr, ColorSpace::Linear),
        ] {
            assert_eq!(usage.colorspace(), space, "{usage:?}");
        }
        assert!(!Ui.uses_mips());
        for u in [Color, Normal, Data, Hdr, Cubemap] {
            assert!(u.uses_mips(), "{u:?}");
        }
    }

    #[test]
    fn the_srgb_curve_matches_known_values_and_round_trips() {
        // Published reference points of the sRGB transfer function.
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!(close(srgb_to_linear(1.0), 1.0, 1e-6));
        assert!(
            close(srgb_to_linear(0.5), 0.214_04, 1e-4),
            "{}",
            srgb_to_linear(0.5)
        );
        assert!(
            close(srgb_to_linear(0.04045), 0.04045 / 12.92, 1e-7),
            "the linear toe"
        );
        assert!(close(srgb8_to_linear(128), 0.2158, 1e-3));
        assert!(
            close(linear_to_srgb(0.18), 0.4614, 1e-3),
            "middle grey, {}",
            linear_to_srgb(0.18)
        );
        assert!(close(linear_to_srgb(0.5), 0.7354, 1e-3));
        for i in 0..=1000 {
            let c = i as f32 / 1000.0;
            assert!(close(linear_to_srgb(srgb_to_linear(c)), c, 2e-5), "{c}");
        }
        // Monotonic, and the 8-bit table is strictly increasing so no two codes collapse.
        let table: Vec<f32> = (0..=255u8).map(srgb8_to_linear).collect();
        assert!(table.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn srgb_conversions_clamp_and_survive_nan() {
        assert_eq!(srgb_to_linear(-1.0), 0.0);
        assert!(close(srgb_to_linear(7.0), 1.0, 1e-6));
        assert_eq!(srgb_to_linear(f32::NAN), 0.0);
        assert_eq!(linear_to_srgb(f32::NAN), 0.0);
        assert_eq!(linear_to_srgb(-3.0), 0.0);
        assert!(close(linear_to_srgb(9.0), 1.0, 1e-6));
        assert!(close(srgb_to_linear(f32::INFINITY), 1.0, 1e-6));
    }

    // ---- samplers ------------------------------------------------------------------------------------

    #[test]
    fn world_textures_are_trilinear_repeating_and_anisotropic_up_to_the_device() {
        let ultra = QualitySettings::preset(QualityTier::Ultra); // 16x
        let s = sampler_for(TextureUsage::Color, &ultra, 16, 1.0);
        assert_eq!(
            (s.min_filter, s.mag_filter, s.mip_filter),
            (Filter::Linear, Filter::Linear, Some(Filter::Linear))
        );
        assert_eq!(
            (s.anisotropy, s.address, s.mip_bias),
            (16, AddressMode::Repeat, 0.0)
        );
        assert_eq!(
            sampler_for(TextureUsage::Color, &ultra, 4, 1.0).anisotropy,
            4,
            "the device limit wins"
        );
        let low = QualitySettings::preset(QualityTier::Low); // 1x
        assert_eq!(
            sampler_for(TextureUsage::Color, &low, 16, 1.0).anisotropy,
            1
        );
        // A device that reports 0 or an absurd limit still gives a legal value.
        assert_eq!(
            sampler_for(TextureUsage::Color, &ultra, 0, 1.0).anisotropy,
            1
        );
        assert_eq!(
            sampler_for(TextureUsage::Color, &ultra, 255, 1.0).anisotropy,
            16
        );
        for usage in [TextureUsage::Normal, TextureUsage::Data, TextureUsage::Hdr] {
            assert_eq!(
                sampler_for(usage, &ultra, 8, 1.0).anisotropy,
                8,
                "{usage:?}"
            );
        }
    }

    #[test]
    fn anisotropy_never_exceeds_sixteen_even_for_settings_that_were_never_validated() {
        // Built directly, bypassing `set` and `validate`.
        let wild = QualitySettings {
            anisotropy: 200,
            ..QualitySettings::default()
        };
        assert_eq!(
            sampler_for(TextureUsage::Color, &wild, 255, 1.0).anisotropy,
            16
        );
        assert_eq!(
            sampler_for(TextureUsage::Color, &wild, 8, 1.0).anisotropy,
            8
        );
        let zero = QualitySettings {
            anisotropy: 0,
            ..QualitySettings::default()
        };
        assert_eq!(
            sampler_for(TextureUsage::Color, &zero, 16, 1.0).anisotropy,
            1
        );
    }

    #[test]
    fn interface_textures_and_cube_maps_have_their_own_samplers() {
        let ultra = QualitySettings::preset(QualityTier::Ultra);
        let ui = sampler_for(TextureUsage::Ui, &ultra, 16, 0.5);
        assert_eq!(
            (ui.mip_filter, ui.anisotropy, ui.address, ui.mip_bias),
            (None, 1, AddressMode::ClampToEdge, 0.0),
            "the UI ignores the render scale"
        );
        let cube = sampler_for(TextureUsage::Cubemap, &ultra, 16, 1.0);
        assert_eq!(
            (cube.address, cube.anisotropy, cube.mip_filter),
            (AddressMode::ClampToEdge, 1, Some(Filter::Linear))
        );
    }

    #[test]
    fn rendering_smaller_asks_for_sharper_mips() {
        assert_eq!(mip_bias_for_render_scale(1.0), 0.0);
        assert!(
            close(mip_bias_for_render_scale(0.5), -1.0, 1e-6),
            "half the resolution: one mip sharper"
        );
        assert!(close(mip_bias_for_render_scale(0.25), -2.0, 1e-6));
        assert!(close(mip_bias_for_render_scale(0.7), 0.7f32.log2(), 1e-6));
        assert_eq!(
            mip_bias_for_render_scale(2.0),
            0.0,
            "never blurrier than normal"
        );
        assert!(
            close(mip_bias_for_render_scale(0.01), -2.0, 1e-6),
            "clamped at a quarter"
        );
        assert_eq!(mip_bias_for_render_scale(f32::NAN), 0.0);
        let s = sampler_for(TextureUsage::Data, &QualitySettings::default(), 16, 0.5);
        assert!(close(s.mip_bias, -1.0, 1e-6));
        assert!(close(
            sampler_for(TextureUsage::Cubemap, &QualitySettings::default(), 16, 0.5).mip_bias,
            -1.0,
            1e-6
        ));
    }

    // ---- mips -----------------------------------------------------------------------------------------

    #[test]
    fn mip_counts_and_sizes_follow_the_longest_side() {
        assert_eq!(mip_count(1, 1), 1);
        assert_eq!(mip_count(2, 1), 2);
        assert_eq!(mip_count(256, 256), 9);
        assert_eq!(mip_count(256, 64), 9, "the longest side decides");
        assert_eq!(mip_count(255, 255), 8);
        assert_eq!(mip_count(257, 3), 9);
        assert_eq!(mip_count(0, 0), 0);
        assert_eq!(mip_count(0, 16), 5);
        assert_eq!(mip_count(u32::MAX, 1), 32);
        assert_eq!(mip_size(256, 64, 0), (256, 64));
        assert_eq!(mip_size(256, 64, 2), (64, 16));
        assert_eq!(mip_size(256, 64, 7), (2, 1), "never below 1");
        assert_eq!(mip_size(256, 64, 99), (1, 1));
    }

    #[test]
    fn texture_detail_skips_big_mips_but_leaves_small_textures_alone() {
        use TextureDetail::*;
        assert_eq!(mips_to_skip(2048, 2048, Full), 0);
        assert_eq!(mips_to_skip(2048, 2048, Half), 1);
        assert_eq!(mips_to_skip(2048, 2048, Quarter), 2);
        // 256: quarter would leave 64 (allowed); 128: quarter would leave 32, so only one level is skipped; 64 stays whole.
        assert_eq!(mips_to_skip(256, 256, Quarter), 2);
        assert_eq!(mips_to_skip(128, 128, Quarter), 1);
        assert_eq!(mips_to_skip(128, 128, Half), 1);
        assert_eq!(mips_to_skip(64, 64, Half), 0);
        assert_eq!(mips_to_skip(64, 64, Quarter), 0);
        assert_eq!(mips_to_skip(16, 16, Quarter), 0);
        assert_eq!(mips_to_skip(1, 1, Quarter), 0);
        assert_eq!(mips_to_skip(0, 0, Quarter), 0);
        // The longest side decides, not the area.
        assert_eq!(mips_to_skip(512, 4, Quarter), 2);
        // Never skips so many that nothing is left.
        for (w, h) in [(300, 300), (4096, 8), (65, 65), (130, 1)] {
            for d in [Quarter, Half, Full] {
                let skip = mips_to_skip(w, h, d);
                assert!(skip < mip_count(w, h).max(1));
                assert!(
                    skip == 0 || w.max(h) >> skip >= MIN_REDUCED_SIZE,
                    "{w}x{h} {d:?} skips {skip}"
                );
            }
        }
    }

    // ---- normal maps ----------------------------------------------------------------------------------

    #[test]
    fn a_flat_normal_map_texel_decodes_to_straight_up() {
        // (128, 128, 255) is the usual flat-blue texel.
        let n = decode_normal(128, 128, 255);
        assert!(
            close(n.x, 0.0, 0.01) && close(n.y, 0.0, 0.01) && close(n.z, 1.0, 1e-3),
            "{n:?}"
        );
        assert!(close(n.length(), 1.0, 1e-5));
        assert_eq!(
            decode_normal(0, 0, 0),
            Vec3::new(-1.0, -1.0, -1.0).normalize()
        );
        assert!(decode_normal(255, 128, 128).x > 0.99);
    }

    #[test]
    fn three_channel_normals_round_trip_within_eight_bit_precision() {
        let mut seed = 0x9E37_79B9u64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 20_001) as f32 / 10_000.0 - 1.0
        };
        for _ in 0..500 {
            let n = Vec3::new(rnd(), rnd(), rnd().abs() + 0.05).normalize();
            let [r, g, b] = encode_normal(n);
            let back = decode_normal(r, g, b);
            assert!(back.dot(n) > 0.9999, "{n:?} -> {:?} -> {back:?}", [r, g, b]);
            assert!(close(back.length(), 1.0, 1e-5));
        }
        assert_eq!(encode_normal(Vec3::Z), [128, 128, 255]);
        assert_eq!(
            encode_normal(Vec3::new(f32::NAN, 0.0, 1.0)),
            [128, 128, 255],
            "bad input gives the flat normal"
        );
        assert_eq!(encode_normal(Vec3::ZERO), [128, 128, 255]);
        assert_eq!(
            encode_normal(Vec3::new(0.0, 0.0, 5.0)),
            [128, 128, 255],
            "it normalizes first"
        );
    }

    #[test]
    fn two_channel_normals_rebuild_z_and_stay_unit_length() {
        let flat = decode_normal_xy(128, 128);
        assert!(close(flat.z, 1.0, 1e-3), "{flat:?}");
        // x = 0.6, y = 0.0 is exactly representable up to rounding: z = 0.8.
        let n = decode_normal_xy((0.8 * 255.0f32).round() as u8, 128); // 0.8 packs to 0.6 in the -1..1 range
        assert!(close(n.length(), 1.0, 1e-4), "{n:?}");
        assert!(n.z >= 0.0);
        // Rounding can put x and y just outside the unit disc: they are scaled back, so z is (nearly) 0 and the length is 1; the square root turns a rounding error of 3e-8 into a z of about 2e-4.
        let corner = decode_normal_xy(255, 255);
        assert!(
            corner.z < 1e-3 && close(corner.length(), 1.0, 1e-5),
            "{corner:?}"
        );
        assert!(close(corner.x, corner.y, 1e-6));
        // Every fifth red value with every green value gives a finite unit vector that points out of the surface.
        for r in (0..=255u8).step_by(5) {
            for g in 0..=255u8 {
                let n = decode_normal_xy(r, g);
                assert!(
                    n.is_finite() && close(n.length(), 1.0, 1e-4) && n.z >= 0.0,
                    "({r}, {g}) -> {n:?}"
                );
            }
        }
    }

    // ---- cube maps -------------------------------------------------------------------------------------

    #[test]
    fn each_face_centre_looks_down_its_axis() {
        let expect = [
            (CubeFace::PosX, Vec3::X),
            (CubeFace::NegX, Vec3::NEG_X),
            (CubeFace::PosY, Vec3::Y),
            (CubeFace::NegY, Vec3::NEG_Y),
            (CubeFace::PosZ, Vec3::Z),
            (CubeFace::NegZ, Vec3::NEG_Z),
        ];
        for (face, axis) in expect {
            assert_eq!(cube_face_direction(face, 0.0, 0.0), axis, "{face:?}");
            let (f, u, v) = direction_to_cube_face(axis).unwrap();
            assert_eq!((f, u, v), (face, 0.0, 0.0));
        }
        assert_eq!(CubeFace::ALL.map(CubeFace::index), [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn face_corners_follow_the_standard_convention() {
        // The first texel corner (u = -1, v = -1) of each face, per the usual cube-map tables.
        assert_eq!(
            cube_face_direction(CubeFace::PosX, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0)
        );
        assert_eq!(
            cube_face_direction(CubeFace::NegX, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0)
        );
        assert_eq!(
            cube_face_direction(CubeFace::PosY, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0)
        );
        assert_eq!(
            cube_face_direction(CubeFace::NegY, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0)
        );
        assert_eq!(
            cube_face_direction(CubeFace::PosZ, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, 1.0)
        );
        assert_eq!(
            cube_face_direction(CubeFace::NegZ, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0)
        );
        // A direction well inside +X, a bit towards -Z and +Y, lands left of centre and above it on that face.
        let (f, u, v) = direction_to_cube_face(Vec3::new(1.0, 0.5, -0.25)).unwrap();
        assert_eq!(f, CubeFace::PosX);
        assert!(close(u, 0.25, 1e-6) && close(v, -0.5, 1e-6), "{u} {v}");
    }

    #[test]
    fn addressing_a_cube_map_round_trips_over_every_face() {
        for face in CubeFace::ALL {
            for iu in -10i32..=10 {
                for iv in -10i32..=10 {
                    let (u, v) = (iu as f32 / 10.0, iv as f32 / 10.0);
                    let dir = cube_face_direction(face, u, v) * 3.7; // length does not matter
                    let (f2, u2, v2) = direction_to_cube_face(dir).unwrap();
                    let on_edge = iu.abs() == 10 || iv.abs() == 10;
                    if !on_edge {
                        assert_eq!(f2, face, "{face:?} ({u}, {v})");
                    }
                    // Edges and corners may belong to a neighbouring face (a tie), but the direction must still map back to the same ray.
                    let again = cube_face_direction(f2, u2, v2);
                    assert!(
                        close(again.normalize().dot(dir.normalize()), 1.0, 1e-5),
                        "{face:?} ({u}, {v}) -> {f2:?} ({u2}, {v2})"
                    );
                    assert!(u2.abs() <= 1.0 + 1e-6 && v2.abs() <= 1.0 + 1e-6);
                }
            }
        }
    }

    #[test]
    fn ties_go_to_x_then_y_then_z_and_bad_directions_have_no_face() {
        assert_eq!(
            direction_to_cube_face(Vec3::new(1.0, 1.0, 1.0)).unwrap().0,
            CubeFace::PosX
        );
        assert_eq!(
            direction_to_cube_face(Vec3::new(0.0, 1.0, 1.0)).unwrap().0,
            CubeFace::PosY
        );
        assert_eq!(
            direction_to_cube_face(Vec3::new(-1.0, -1.0, -1.0))
                .unwrap()
                .0,
            CubeFace::NegX
        );
        assert_eq!(
            direction_to_cube_face(Vec3::new(0.0, -1.0, -1.0))
                .unwrap()
                .0,
            CubeFace::NegY
        );
        assert_eq!(
            direction_to_cube_face(Vec3::new(0.0, 0.0, 2.0)).unwrap().0,
            CubeFace::PosZ
        );
        assert_eq!(
            direction_to_cube_face(Vec3::new(0.0, 0.0, -2.0)).unwrap().0,
            CubeFace::NegZ
        );
        for bad in [
            Vec3::ZERO,
            Vec3::new(f32::NAN, 0.0, 1.0),
            Vec3::new(f32::INFINITY, 0.0, 0.0),
        ] {
            assert!(direction_to_cube_face(bad).is_none(), "{bad:?}");
        }
    }
}
