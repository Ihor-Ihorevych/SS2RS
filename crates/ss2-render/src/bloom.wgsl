// The bloom chain (RFC 0013 amendment 6, `docs/re/environment.md` "Bloom"): a full-screen triangle per pass.

struct Params {
    // The source texture's texel size in UV.
    texel: vec2<f32>,
    // The two blur taps, in source texels.
    tap_a: vec2<f32>,
    tap_b: vec2<f32>,
    // x: the high-pass strength in 0..=1.
    k: vec2<f32>,
    // rgb: the tint (the ramp colour in the tint pass, the bloom colour times the fade in the add pass).
    tint: vec4<f32>,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(0) @binding(2) var<uniform> p: Params;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VertexOut {
    let xy = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VertexOut;
    out.clip = vec4<f32>(xy * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(xy.x, 1.0 - xy.y);
    return out;
}

// A 4x4 box around `uv` from four bilinear taps.
fn box4(uv: vec2<f32>) -> vec3<f32> {
    let t = p.texel;
    return (textureSample(src, src_sampler, uv + t * vec2<f32>(-1.0, -1.0)).rgb
        + textureSample(src, src_sampler, uv + t * vec2<f32>(1.0, -1.0)).rgb
        + textureSample(src, src_sampler, uv + t * vec2<f32>(-1.0, 1.0)).rgb
        + textureSample(src, src_sampler, uv + t * vec2<f32>(1.0, 1.0)).rgb) * 0.25;
}

// A quarter-size step: the box filter, drawn twice at the two blur taps and averaged.
@fragment
fn fs_down(f: VertexOut) -> @location(0) vec4<f32> {
    let a = box4(f.uv + p.tap_a * p.texel);
    let b = box4(f.uv + p.tap_b * p.texel);
    return vec4<f32>((a + b) * 0.5, 1.0);
}

// The high-pass (only what is brighter than 1 - k survives, stretched back to 0..1) and the ramp tint.
@fragment
fn fs_tint(f: VertexOut) -> @location(0) vec4<f32> {
    let c = textureSample(src, src_sampler, f.uv).rgb;
    let k = max(p.k.x, 1e-3);
    let bright = max(c - vec3<f32>(1.0 - k), vec3<f32>(0.0)) / k;
    return vec4<f32>(bright * p.tint.rgb, 1.0);
}

// Added onto the frame (the pipeline blends One + One on colour only).
@fragment
fn fs_add(f: VertexOut) -> @location(0) vec4<f32> {
    let c = textureSample(src, src_sampler, f.uv).rgb;
    return vec4<f32>(c * p.tint.rgb, 0.0);
}

// A plain copy of the frame.
@fragment
fn fs_copy(f: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(src, src_sampler, f.uv);
}
