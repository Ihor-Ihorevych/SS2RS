// Flat-colored 2D triangles positioned in window pixels.

struct Viewport {
    // xy = window size in pixels, zw unused (padding).
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let ndc = vec2<f32>(
        in.pos.x / viewport.size.x * 2.0 - 1.0,
        1.0 - in.pos.y / viewport.size.y * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
