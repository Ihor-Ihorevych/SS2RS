// The 3D world pass (RFC 0013): textured static meshes, one matrix per instance, reverse-Z depth and a light that
// comes from the camera (a "headlight"), two-sided because the winding of the game's meshes is not yet verified.

struct Camera {
    view_proj: mat4x4<f32>,
    // xyz: the direction the light travels towards the scene, unit.
    light_dir: vec4<f32>,
    // xyz: the eye in world space.
    eye: vec4<f32>,
    // rgb: linear fog colour; a: strength (0: no fog).
    fog_colour: vec4<f32>,
    // x: near, y: far (RFC 0013 amendment 4: linear in view distance).
    fog_range: vec4<f32>,
};

struct Material {
    tint: vec4<f32>,
    // x: alpha cutoff (texels below it are not drawn; 0 keeps all); y: 1 = unlit (the sky), 0 = lit;
    // z: 1 = lightmapped (full bright: base x lightmap, no light term), 0 = not.
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var base_texture: texture_2d<f32>;
@group(1) @binding(1) var base_sampler: sampler;
@group(1) @binding(2) var<uniform> material: Material;
@group(1) @binding(3) var lightmap_texture: texture_2d<f32>;
@group(1) @binding(4) var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
    @location(7) lightmap_uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) lightmap_uv: vec2<f32>,
    @location(3) world: vec3<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(v.model_0, v.model_1, v.model_2, v.model_3);
    var out: VertexOut;
    let world = model * vec4<f32>(v.position, 1.0);
    out.clip = camera.view_proj * world;
    out.world = world.xyz;
    out.normal = (model * vec4<f32>(v.normal, 0.0)).xyz;
    out.uv = v.uv;
    out.lightmap_uv = v.lightmap_uv;
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
    // Sampled for every fragment (uniform control flow); used only when the material is lightmapped.
    let baked = textureSample(lightmap_texture, lightmap_sampler, f.lightmap_uv).rgb;
    let light = select(vec3<f32>(shade), baked, material.params.z > 0.5);
    // Linear distance fog; the sky (unlit) is never fogged.
    let d = distance(f.world, camera.eye.xyz);
    let span = max(camera.fog_range.y - camera.fog_range.x, 1e-4);
    let fog = camera.fog_colour.a * clamp((d - camera.fog_range.x) / span, 0.0, 1.0);
    let amount = select(fog, 0.0, material.params.y > 0.5);
    return vec4<f32>(mix(texel.rgb * light, camera.fog_colour.rgb, amount), 1.0);
}
