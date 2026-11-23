# Texture sampling rules

`ss2_render_api::texture_sampling` (T04-008) holds the **decisions and the maths** a renderer needs before it creates a sampler or a texture view, and that a shader and a CPU-side test must agree on. It is device-free: it creates nothing on a GPU.

## sRGB against linear

`TextureUsage` says what a texture holds (`Color`, `Normal`, `Data`, `Hdr`, `Cubemap`, `Ui`); `colorspace()` says how to read its bytes: **colour, cube maps and UI are sRGB** (use an `*Srgb` format so the GPU converts when sampling), **normal maps, data and HDR are linear**. Getting it wrong is the classic washed-out or too-dark picture (colour read as linear) or bent normals (a normal map read as sRGB).

`srgb_to_linear`, `linear_to_srgb` and `srgb8_to_linear` implement the standard transfer function (a linear toe below 0.04045 and a 2.4 power above). They are checked against published reference points (0.5 -> 0.214, middle grey 0.18 -> 0.4614), round-trip over 1,001 values within 2e-5, and the 256-entry table is strictly increasing. Input is clamped to 0 to 1 and NaN gives 0.

## Samplers

`sampler_for(usage, &QualitySettings, device_max_anisotropy, render_scale)` returns a `SamplerDesc`:

- **World textures** (colour, normal, data, HDR): trilinear (linear min, mag and mip filters), repeating, **anisotropy from the quality setting but never above the device's limit** and never outside 1 to 16 (a device reporting 0 or 255 still gives a legal value; settings that were never validated are clamped too), and the render-scale mip bias.
- **Cube maps**: trilinear, **clamped to the edge** (a repeating cube map shows seams), no anisotropy.
- **UI**: linear, **no mips**, clamped, no anisotropy, no bias: it is drawn one to one at full resolution, whatever the 3D scale.

**Mip bias for a lower render scale.** When the scene is drawn at `scale` of the window size (dynamic resolution, `quality`), each pixel covers `1 / scale` times more texture, so the textures look softer than they need to. `mip_bias_for_render_scale(scale) = log2(scale)` (clamped to 0.25 to 1, NaN gives 0) asks the hardware for a sharper mip: half resolution gives -1, a quarter -2, full resolution 0.

## Mips and the texture-detail setting

`mip_count(w, h)` is `floor(log2(max(w, h))) + 1` (0 for an empty texture); `mip_size` halves each level and never goes below 1. `mips_to_skip(w, h, detail)` says how many of the **largest** levels to leave out when loading: 2 for `Quarter`, 1 for `Half`, 0 for `Full`, but **fewer if that would take the longest side below 64 pixels** (shrinking an icon saves nothing and looks worse). A 2048 texture at quarter detail skips 2; a 128 skips 1; a 64 skips none.

## Normal maps

- `decode_normal(r, g, b)`: each channel maps 0 to 255 onto -1 to 1, then the vector is normalized (a flat texel `(128, 128, 255)` decodes to straight up within a hundredth).
- `decode_normal_xy(r, g)` for **two-channel** maps (BC5 style): `z = sqrt(1 - x^2 - y^2)`, with x and y first scaled back inside the unit disc if rounding put them just outside, and z never negative. Across 13,312 combinations (every fifth red value with every green value) the result is finite, unit length and points out of the surface. At the very edge of the disc the square root turns a rounding error of 3e-8 into a z of about 2e-4; the vector is still unit length.
- `encode_normal` packs a normal back to three bytes (it normalizes first; a zero or non-finite vector gives the flat normal). Round trip: the decoded normal agrees with the original to better than 0.9999 (cosine) over 500 random normals.

## Cube maps

`CubeFace` in the usual order (+X, -X, +Y, -Y, +Z, -Z). `cube_face_direction(face, u, v)` gives the direction through a point of a face (`u` and `v` over -1 to 1) and `direction_to_cube_face(dir)` is its inverse (the face of the largest component, ties going to X, then Y, then Z; a zero or non-finite direction has none). The six face centres and the first-texel corner of each face are checked against the conventional table, and a sweep of 441 points on every face round-trips to the same ray, **edges and corners included** (an edge may belong to either face; the direction must still match).

## How it was tested

Sixteen unit tests. **Thirty-seven deliberate breakages.** Two were rejected by the **compiler** (they made a `match` non-exhaustive, which is a catch but not a test failing, so I do not count them), thirty-two failed a test, and **three survived**: one real gap (the sixteen-times ceiling on anisotropy when a device reports more, now tested) and two equivalent, where I **removed code instead**: a guard against the all-zero normal texel that 8 bits cannot produce, and a check that "never skips every mip" that the 64-pixel floor already implies. A comparison at the exact boundary of the sRGB toe (0.04045) is equivalent too, because the two formulas meet there; I left it. About a dozen of my mutation scripts first failed to apply because rustfmt had reflowed the lines, and were redone against the formatted source.

## Not covered (follow-up T04-008b)

- **Nothing creates a GPU sampler or texture**: no wgpu `SamplerDescriptor`, no choice of an `Srgb` format per texture, no mip generation, no upload of only the mips that remain after skipping (the texture loader of T04-005/T05 does that; reading real texture files is a separate, spec-based task).
- **Compression** (BC1, BC5, BC7, ASTC for phones), **anisotropy below the limit on a phone**, and **texture arrays** are not modelled; the two-channel normal decode assumes a BC5-like map.
- **Shader-side code** is not written: the maths here is what the shader has to match, and nothing checks the shader against it yet.
- **Which usage a given game texture has** comes from the material system (T04-007); nothing here reads a material.
