// The 3D world pass (RFC 0013): textured static meshes, one matrix per instance, reverse-Z depth and a light that
// comes from the camera (a "headlight"), two-sided because the winding of the game's meshes is not yet verified.

struct Camera {
    view_proj: mat4x4<f32>,
    // xyz: the direction the light travels towards the scene, unit.
    light_dir: vec4<f32>,
};

struct Material {
    tint: vec4<f32>,
    // x: alpha cutoff (texels below it are not drawn; 0 keeps all); y: 1 = unlit (the sky), 0 = lit.
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var base_texture: texture_2d<f32>;
@group(1) @binding(1) var base_sampler: sampler;
@group(1) @binding(2) var<uniform> material: Material;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(v.model_0, v.model_1, v.model_2, v.model_3);
    var out: VertexOut;
    out.clip = camera.view_proj * model * vec4<f32>(v.position, 1.0);
    out.normal = (model * vec4<f32>(v.normal, 0.0)).xyz;
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_main(f: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(base_texture, base_sampler, f.uv) * material.tint;
    if texel.a < material.params.x {
        discard;
    }
    let n = normalize(f.normal);
    let lit = 0.55 + 0.45 * abs(dot(n, -camera.light_dir.xyz));
    let shade = select(lit, 1.0, material.params.y > 0.5);
    return vec4<f32>(texel.rgb * shade, 1.0);
}
