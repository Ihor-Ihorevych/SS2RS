# RFC 0013: Level scene boundary

- **Number:** 0013 (`docs/rfcs/0013-level-scene-boundary.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-render-api`, `ss2-render`, `ss2-world`, `ss2-physics`, `ss2-vfs` **plus one** other maintainer (the project has one maintainer today, so like RFCs 0001-0012 this proceeds "in review")
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T00-035 (this RFC); defines the interfaces of T04-037, T07-027, T04-038, T07-028, T07-029, T07-030, T02-038, T10-034, T03-036, T03-037; slices of T02-020, T02-017, T04-005, T04-009, T04-026, T07-002, T07-005, T07-023
- **Design:** [Playable MVP](../superpowers/specs/2026-11-27-playable-mvp-design.md)

## Summary

A small set of **format-neutral** types that carry one loaded level from the readers of the game's files to
the renderer, the collision world and the app: `ss2_render_api::scene_data` (meshes, textures, materials as
plain CPU data), `ss2_world::scene::LevelScene` (the level: models, instances, spawn points, gravity), a
loader function, a collision builder, a provisional walking controller, a level list and the renderer calls
that draw a level. Nothing here names a file format.

## Motivation

The playable MVP (pick a level, walk around it) touches eight crates. The format readers must be written by
people who did not write the format specs (CLEANROOM.md), and everything downstream of them is written by
someone who did. A fixed, format-neutral boundary lets both sides work in parallel: the downstream side
builds and tests against a procedural `LevelScene::test_room()`, the readers produce a `LevelScene` from real
data. AGENTS.md section 2 requires an RFC for cross-crate APIs.

## Detailed design

Coordinates are the engine's: right-handed, Y up, metres, `-Z` forward (`docs/GEOMETRY.md`). Math types are
`ss2_core::math` (glam). Errors are `ss2_core::Error` unless stated.

### `ss2_render_api::scene_data` (T04-037)

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshVertex { pub position: [f32; 3], pub normal: [f32; 3], pub uv: [f32; 2] }  // 32 bytes

pub struct MeshData { pub vertices: Vec<MeshVertex>, pub indices: Vec<u32> }   // triangle list
impl MeshData {
    pub fn validate(&self) -> Result<()>;          // indices % 3 == 0, in range, finite values
    pub fn bounds(&self) -> Option<Aabb>;          // None when empty
}

pub enum TexelFormat { Rgba8, Bc1, Bc2, Bc3 }       // colour data, sRGB-encoded
pub struct TextureData {
    pub width: u32, pub height: u32, pub format: TexelFormat,
    pub mips: Vec<Vec<u8>>,                         // level 0 first; at least one
    pub repeat_u: bool, pub repeat_v: bool,
}
impl TextureData { pub fn validate(&self) -> Result<()>; }   // sizes match the format and each level

pub enum AlphaMode { Opaque, Mask { cutoff: f32 }, Blend }
pub struct MaterialData {
    pub base_color: [f32; 4],                       // linear RGBA multiplier
    pub base_texture: Option<u32>,                  // index into the owner's texture list
    pub alpha: AlphaMode,
    pub double_sided: bool,
}
```

### `ss2_world::scene` (T07-027)

```rust
pub struct ModelPart { pub mesh: MeshData, pub material: u32 }       // index into LevelScene::materials
pub struct CollisionData { pub positions: Vec<Vec3>, pub indices: Vec<u32> }
pub struct ModelData { pub name: String, pub parts: Vec<ModelPart>, pub collision: Option<CollisionData> }
pub struct Instance { pub model: u32, pub transform: ss2_render_api::Transform, pub collidable: bool }
pub struct SpawnPoint { pub position: Vec3, pub yaw: f32 }           // feet position; yaw 0 looks down -Z

pub struct LevelScene {
    pub name: String,
    pub gravity: Vec3,
    pub models: Vec<ModelData>,
    pub materials: Vec<MaterialData>,
    pub textures: Vec<TextureData>,
    pub instances: Vec<Instance>,
    pub spawns: Vec<SpawnPoint>,
    pub skipped: Vec<String>,                       // what the loader left out, one line each
}
impl LevelScene {
    pub fn validate(&self) -> Result<()>;           // every index in range, every mesh and texture valid
    pub fn bounds(&self) -> Option<Aabb>;           // world-space bounds of all instances
    pub fn start(&self) -> SpawnPoint;              // first spawn, else above the centre of bounds, else origin
    pub fn test_room() -> LevelScene;               // procedural: floor, walls, a 0.4 m and a 1.0 m step, a ramp
}

pub enum LoadStage { World, Models, Textures, Collision }
pub struct LoadProgress { pub stage: LoadStage, pub done: u32, pub total: u32 }
```

Rules:

- `Instance::transform.scale` may be **negative** on any axis (mirrored placements). Mesh winding stays as
  authored; consumers that cull handle the mirror (the MVP renderer does not cull).
- `ModelData::collision` is the model's own collision mesh where it has one, else its coarsest PC render LOD
  (Amendment 5). `None` means the model does not collide.
- A loader that cannot read one model, material or texture **does not fail the load**: it records a line in
  `skipped` and uses a grey material, a white texture, or drops the instance.

### `ss2_world::load` (T07-028)

```rust
pub struct LoadOptions { pub max_texture_size: u32 }     // larger textures drop their top mips
pub fn load_level(
    vfs: &dyn ss2_vfs::Vfs,
    wld: &ss2_vfs::VfsPath,
    options: &LoadOptions,
    progress: &mut dyn FnMut(LoadProgress),
) -> Result<LevelScene>;
```

Fails only when the `.wld` itself cannot be read. Runs on any thread (the app calls it from a worker).

### `ss2_world::collision` (T07-029)

```rust
pub fn build_static(scene: &LevelScene) -> Result<ss2_physics::query::QueryWorld>;
```

Every `collidable` instance's `collision` data, transformed by its matrix, in one `TriMesh` with material
`PhysMaterialId(0)`.

### `ss2_physics::controller` (T07-030)

```rust
pub struct WalkConfig {
    pub radius: f32, pub height: f32, pub eye_height: f32, pub step_height: f32,
    pub max_slope_cos: f32, pub speed: f32, pub accel: f32, pub friction: f32,
    pub air_accel: f32, pub jump_speed: f32,
}
impl WalkConfig { pub const PROVISIONAL: Self; }   // tuned by feel; replaced by T01-024's numbers

pub struct WalkInput { pub wish: Vec2, pub yaw: f32, pub jump: bool }   // wish: x right, y forward, |wish| <= 1
pub struct Walker { pub position: Vec3, pub velocity: Vec3, pub grounded: bool }   // position = feet
impl Walker {
    pub fn new(position: Vec3) -> Self;
    pub fn eye(&self, cfg: &WalkConfig) -> Vec3;
    pub fn step(&mut self, world: &dyn SpatialQuery, cfg: &WalkConfig, gravity: Vec3,
                input: &WalkInput, dt: f32);
}
```

`step` is a pure function of (state, input, world, dt): fixed internal substeps, no clock, no hash maps.

### `ss2_vfs::levels` (T02-038)

```rust
pub struct LevelInfo {
    pub id: String,              // the level's folder name, e.g. "1_1_jungle"
    pub group: String,           // the folder above, e.g. "01_mdigbo" or "multiplayer"
    pub name: String,            // the .nfo NAME, may be empty
    pub map_types: Vec<String>,  // MAP_TYPE entries, upper case, e.g. ["SINGLE", "COOP"]
    pub wld: VfsPath,            // the .wld in the same folder
}
pub fn discover_levels(vfs: &dyn Vfs) -> Result<Vec<LevelInfo>>;   // sorted by group, then id
```

A level `.nfo` without a `.wld` next to it is left out.

### `ss2_render::Renderer` additions (T04-038)

```rust
pub struct MeshHandle(u32);
pub struct TextureHandle(u32);
pub struct WorldBatch {
    pub mesh: MeshHandle, pub texture: Option<TextureHandle>,   // None: white
    pub tint: [f32; 4], pub alpha_cutoff: Option<f32>,
    pub instances: Vec<Mat4>,                                   // local to world
}
impl Renderer {
    pub fn supports(&self, format: TexelFormat) -> bool;
    pub fn create_mesh(&mut self, mesh: &MeshData) -> Result<MeshHandle, RenderError>;
    pub fn create_texture(&mut self, texture: &TextureData) -> Result<TextureHandle, RenderError>;
    pub fn set_world(&mut self, batches: Vec<WorldBatch>) -> Result<(), RenderError>;
    pub fn clear_world(&mut self);                  // drops meshes, textures and batches
    pub fn world_generation(&self) -> u64;          // changes when a device rebuild dropped the world
    pub fn render_world(&mut self, camera: &Camera, overlay: &DrawList, clear: Color) -> Result<(), RenderError>;
    pub fn render_world_to_image(&mut self, camera: &Camera, overlay: Option<&DrawList>,
                                 clear: Color) -> Result<ImageRgba8, RenderError>;
}
```

The world pass clears colour and depth, then the existing 2D pass draws the overlay on top. `render()`
(2D only) is unchanged. On device loss the world is dropped and `world_generation()` changes; the owner of
the `LevelScene` uploads it again. This is a direct API on the wgpu renderer, not yet the `RenderWorld`
contract of RFC 0002; T04-026 later builds that contract on top of it.

### Usage

```rust
let levels = ss2_vfs::levels::discover_levels(&vfs)?;
let scene = ss2_world::load::load_level(&vfs, &levels[0].wld, &LoadOptions { max_texture_size: 2048 }, &mut |_| {})?;
let physics = ss2_world::collision::build_static(&scene)?;
let mut walker = Walker::new(scene.start().position);
// per frame
walker.step(&physics, &WalkConfig::PROVISIONAL, scene.gravity, &input, dt);
renderer.render_world(&camera_at(walker.eye(&WalkConfig::PROVISIONAL)), &ui, clear)?;
```

### Amendment 1: sky (T00-036)

`ss2_world::scene::LevelScene` gains `pub sky: Option<SkyData>` (`None`: the level has no sky; the renderer shows its
clear colour):

```rust
pub enum CubeFace { PosX, NegX, PosY, NegY, PosZ, NegZ }          // index 0..6, `CubeFace::ALL` in this order
pub struct SkyData {
    pub faces: [Option<u32>; 6],   // texture index per face in CubeFace order; None: that face shows `tint` alone
    pub tint: [f32; 4],            // linear RGBA multiplier on every face
}
```

**Orientation convention**, seen from inside the cube in engine space (right-handed, Y up): a face image's texel
(0, 0) is its top-left corner, and the image's right and down point in these world directions:

| Face | Image right | Image down |
|---|---|---|
| `PosX` | +Z | −Y |
| `NegX` | −Z | −Y |
| `PosY` | +X | −Z |
| `NegY` | +X | +Z |
| `PosZ` | −X | −Y |
| `NegZ` | +X | −Y |

The loader rotates or mirrors the game's face images into this convention (`docs/re/environment.md`, T01-405); the
renderer (`Renderer::set_sky(Option<SkyBatch>)`, T04-039) draws the faces unlit around the camera behind everything
and drops them with the world.

### Amendment 2: lightmaps (T00-037)

- `ss2_render_api::scene_data::MeshData` gains `pub lightmap_uvs: Option<Vec<[f32; 2]>>`: the lightmap UV of each
  vertex (same length as `vertices`, finite), present only on meshes that are lightmapped; `MeshVertex` is unchanged.
- `ss2_world::scene::Instance` gains `pub lightmap: Option<u32>`: index into `LevelScene::textures` of this
  placement's own lightmap (each lightmap belongs to one instance). It is used only with parts that carry
  `lightmap_uvs`.
- `ss2_render::WorldBatch` gains `pub lightmap: Option<TextureHandle>`. A batch with a lightmap is drawn **full
  bright**: base colour times the lightmap sampled with the mesh's lightmap UVs, no run-time light term
  (`docs/re/lighting.md`); a mesh without lightmap UVs reads them as zero.
- The loader picks the lightmap UV stream of a mesh by the rule of `docs/re/wld-lighting.md` (T01-407).

### Amendment 3: sky placement (T07-038)

`SkyData` gains `rotation: Quat` (the sky entity's placement rotation, applied in full; its position is never used),
`size: f32` (the cube's half-extent, centre to face, 64 in most levels) and `viewer_height: f32` (the viewer's height inside the cube
relative to its centre, in the units of `size`; the sign is the open question of `docs/re/environment.md`). The
renderer keeps the cube centred on the camera, turned by `rotation`, and shifted by `viewer_height` scaled to its own
draw size (T04-041).

### Amendment 4: distance haze (T00-038)

A level's haze volumes (`docs/re/environment.md`, "Haze", T01-044/T01-070) cross the boundary so the renderer can
draw the original's distance fog:

- `ss2_render_api::Fog { colour: [f32; 3], near: f32, far: f32, strength: f32 }`: one linear fog for a frame. A
  fragment at view depth `d` (along the camera's forward axis, as the original's vertex programs use clip-space
  depth, T01-412) is mixed towards `colour` by `strength * clamp((d - near) / (far - near), 0, 1)`
  (the original's screen fog is plain linear fog, `scale = 1/(far - near)`, `bias = -far * scale`).
- `ss2_world::scene::HazeVolume { min: Vec3, max: Vec3, rotation: Quat, colour: [f32; 4], fade_margin: f32, near: f32,
  far: f32 }` and `LevelScene::hazes: Vec<HazeVolume>` in the file's list order. The box is `min..max` in the
  volume's own frame, turned by `rotation` about the world origin (the haze renderable's world rotation, which includes a
  parent's when the haze is attached; 13 hazes are turned, T07-042); `colour` is linear RGBA (the alpha is carried, its use is open); `fade_margin` is `.5` (at least 0.001),
  `near`/`far` are `.6`/`.7` (`far >= near`, as the original clamps).
- `LevelScene::fog_at(eye: Vec3) -> Option<Fog>`: each volume's weight is
  `fade = 1 - clamp(distance(eye, box inset by fade_margin) / fade_margin, 0, 1)`; every volume with `fade > 0`
  contributes (T01-412), weighted by its fade times (the largest contributing box diagonal / its own box diagonal)
  (T01-415): near, far and colour are the averages by that weight, and when the plain fades sum to less than 1
  near is multiplied and far divided by that sum (the range widens towards no fog); `strength` is 1; `None` when
  every `fade` is 0. (First written as "the strongest wins", corrected by T01-412.)
- `Renderer::set_fog(Option<Fog>)`, called by the app each frame with `fog_at(camera eye)`. The sky is not fogged
  (open; revisit with T01-412).

### Amendment 5: collision meshes (T07-036)

`CollisionData` gains `pub from_collision_mesh: bool`. The loader fills `ModelData::collision` from the model's own
collision mesh (`docs/re/bmf.md`, "Collision mesh", read by `ss2_format_bmf::CollisionMesh`, T02-042), in model space
like the render parts, and sets the flag; a model without a collision mesh, or with an empty one, keeps the coarsest
PC render LOD with the flag clear (procedural geometry such as the test room also has it clear). Consumers that only
need triangles ignore the flag.

### Amendment 6: bloom (T00-043)

A level's bloom entities (`docs/re/environment.md`, "Bloom", T01-417/T01-423/T01-424) cross the boundary so the
renderer can draw the original's bloom:

- `ss2_world::scene::BloomVolume { min: Vec3, max: Vec3, rotation: Quat, colour: [f32; 4], fade_margin: f32,
  strength: f32, blur: f32, balance: f32, high_pass: f32, ramp: f32 }` and `LevelScene::blooms: Vec<BloomVolume>` in
  the file's list order. The box and its turn follow `HazeVolume`; `colour` is linear RGBA from the renderable's
  `.3`; `fade_margin` is `.2` (at least 0.001); `strength` is renderable `.4` (record `.6`), `blur` `.6` (record
  `.8`), `balance` `.12` (record `.13`), `high_pass` `.13` (record `.16`), `ramp` `.18` (record's second `.21`).
  The HDR-only threshold `.19` and the mask texture `.7` (null in every retail bloom) are not carried.
- `ss2_render_api::scene_data::Bloom { colour: [f32; 4], fade: f32, strength: f32, blur: f32, balance: f32,
  high_pass: f32, ramp: f32 }`: one active bloom for one frame.
- `LevelScene::blooms_at(eye: Vec3) -> Vec<Bloom>`: every volume whose box holds the eye, with its edge fade
  (`1 - clamp(distance(eye, box inset by fade_margin) / fade_margin, 0, 1)`, as for haze) above 0, in list order.
  The original runs each one's whole chain in turn on the frame; every bloom command has the same sort key, so they
  run in the level's own object order (T01-422), which list order is.
- `Renderer::set_blooms(&[Bloom])`, called by the app each frame. After the world pass and before the 2D overlay,
  the renderer runs per bloom: `intensity = clamp(fade * strength * BLOOM_BIAS, 0, 2)` (`BLOOM_BIAS` = 1.0, the
  video option's value, provisional), skipped below a small floor or on a target smaller than 32x32; then the
  repeated quarter-size downscale with the two-tap blur (offsets of 1.5 texels scaled by `blur`, split by
  `balance`) until both sides are at most 128; one high-pass pass (`high_pass * sqrt(intensity)`, at most 1); one
  pass tinting by the fixed three-colour ramp at `ramp`; an additive composite of `colour` (RGB times `fade`, at
  most 1; alpha not faded) onto the frame.

### Amendment 7: skinned meshes (T04-006)

Animated models cross the boundary as plain data in a new module, `ss2_render_api::skinning`; it names no file format,
like `scene_data`. All additions:

- `SkinnedVertex { position, normal, uv, joints: [u16; 4], weights: [f32; 4] }` (56 bytes, `Pod`) and
  `SkinnedMeshData { vertices, indices, inverse_bind: Vec<Mat4> }`: a joint indexes `inverse_bind` (1 to
  `MAX_BONES` = 256 bones); weights are not negative and are normalised by their sum when drawn; all zero leaves the
  vertex at its bind position. `validate()`, `bounds()` and `skinned_position(i, palette)` (the CPU reference of the
  shader).
- A **palette** is one bone-to-model matrix per entry of `inverse_bind`; a vertex moves by
  `Σ wᵢ · palette[jᵢ] · inverse_bind[jᵢ]`, so the bind pose leaves the mesh where it was authored.
- `SkeletonData::new(Vec<SkeletonBone { id, parent: Option<u32>, rest: BoneTransform, inverse_bind }>)` (parents
  before children, ids unique), `BoneTransform { rotation, translation }`, `rest_pose()`, `inverse_binds()`,
  `index_of(id)`, `pose_from_tracks(translations, rotations)` (rest pose with each `(bone id, value)` track value
  replacing that bone's part; ids that are no bone are skipped, `docs/re/ans.md` rule 3) and
  `model_transforms(&local) -> Vec<Mat4>` (`model(bone) = model(parent) · local(bone)`). `skin_matrices(palette,
  inverse_bind)` and `attachment_transform(instance, palette, bone, offset)` (a model held by a bone).
- The format readers stay independent of the renderer: the caller copies a `.skl` LOD's bones into `SkeletonData` and
  zips an `.ans` animation's track ids with a sampled `Pose` into `pose_from_tracks` (the module documentation shows the
  three lines). The adapter for levels belongs in `ss2-world` (T04-044).
- `ss2_render::Renderer` gains `create_skinned_mesh(&SkinnedMeshData) -> SkinnedMeshHandle`,
  `set_skinned(Vec<SkinnedDraw { mesh, texture, tint, alpha_cutoff, transform, palette }>)` (draw `i` becomes
  skinned instance `i`), `set_bone_palette(instance, &[Mat4])`, `set_skinned_transform(instance, Mat4)` and
  `skinned_count()`. A palette whose length is not the mesh's bone count, or with a non-finite matrix, is refused.
  Skinned meshes and instances are dropped with the world. They are drawn after the static batches by a second
  pipeline of the world shader (vertex entry `vs_skinned`, the same fragment stage: lighting, fog, alpha cutoff; no
  lightmap); each instance owns a uniform buffer of `MAX_BONES` skin matrices, multiplied on the CPU when its palette
  is set.

## Alternatives considered

- **Feed the RFC 0002 `RenderWorld` contract directly.** It refers to models by `ModelId` through a resource
  registry that has no implementation behind it; building that first (T04-026) is more work than the MVP
  needs. The direct API here is what T04-026 will wrap.
- **Put `LevelScene` in `ss2-render-api`.** Rejected: it carries collision data and spawn points, which are
  not rendering concerns. `ss2-world` is the level-instance crate (T07-023).
- **Let format crates produce `LevelScene` themselves.** Rejected: format crates stay independent of the
  renderer and world crates; the adapter lives in `ss2-world`.

## Compatibility and migration

All additions; no existing signature changes. New dependencies: `ss2-render -> ss2-render-api` (already in
RFC 0002's diagram), `ss2-world -> ss2-render-api, ss2-physics, ss2-vfs`, and `ss2-app -> ss2-world,
ss2-physics, ss2-game, ss2-render-api`. New crates `ss2-format-wld` and `ss2-format-mdl` depend only on
`ss2-core`, `ss2-meta` and other `ss2-format-*` crates.

## Open questions

- The UV v-direction and front-face winding of meshes are not yet verified (`docs/re/bmf.md`). The first
  rendered comparison settles them; any flip is done in the loader so the boundary stays "as drawn".
- How single-player and co-op choose their start (T01-210). Until then `start()` uses the first spawn point.
