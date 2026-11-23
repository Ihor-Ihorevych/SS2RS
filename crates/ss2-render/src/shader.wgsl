// Textured, tinted 2D triangles positioned in window pixels. A flat quad samples a white 1x1 texel.

struct Viewport {
    // xy = target size in pixels, zw unused (padding).
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(2) @binding(0) var image_sampler: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let ndc = vec2<f32>(
        in.pos.x / viewport.size.x * 2.0 - 1.0,
        1.0 - in.pos.y / viewport.size.y * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

// A texel with straight alpha (uploaded images, flat quads): blended with SrcAlpha / OneMinusSrcAlpha.
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, in.uv) * in.color;
}

// A texel with premultiplied alpha (a render target): the tint is applied to the premultiplied color
// and blended with One / OneMinusSrcAlpha.
@fragment
fn fs_premultiplied(in: VsOut) -> @location(0) vec4<f32> {
    let t = textureSample(image, image_sampler, in.uv);
    return vec4<f32>(t.rgb * in.color.rgb * in.color.a, t.a * in.color.a);
}
