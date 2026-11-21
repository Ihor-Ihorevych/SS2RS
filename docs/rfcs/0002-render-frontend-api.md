# RFC 0002: Renderer front-end API (contract v0)

- **Number:** 0002 (`docs/rfcs/0002-render-frontend-api.md`; 0001 is the merged VFS/asset contract)
- **Authors:** @lxstyo
- **Status:** in review
- **Approvers:** owner of `ss2-render` and `ss2-core` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T00-015 (this contract); unblocks T04-026 (extract + double buffering), T04-004/005/006/007/017 (renderer internals), T07/T09 (anything that spawns visible things), T10 (HUD submission)

## Summary

Add a small, GPU-free crate `ss2-render-api` that defines everything the rest of the engine may say to the renderer: a per-tick `RenderWorld` extract (camera, renderables, lights, UI draw list), opaque resource ids, a `RenderResources` registry, and a `Renderer` trait with `submit` / `render(alpha)`. Ship `NullRenderer` and `MockResources` so consumers can be written and tested before the wgpu renderer implements the trait. Re-export `glam` as `ss2_core::math` so the whole engine shares one math library.

## Motivation

The design decision "renderer is fed through an extract interface so sim never touches GPU types" (design §8) needs a concrete, frozen form before T04 and T07/T09 can proceed in parallel. Today `ss2-render` links `wgpu`, `glyphon` and window glue; if the simulation depended on it, the headless dedicated server (`ss2-server`) would need a graphics stack and sim code could reach GPU types. A separate, tiny API crate fixes the layering:

```
ss2-sim / ss2-world / ss2-game / ss2-ui ──► ss2-render-api ◄── ss2-render (wgpu)
                                                 ▲
                              ss2-app / ss2-server ┘ (pick NullRenderer or the wgpu one)
```

## Detailed design

All code is in `crates/ss2-render-api/` (rustdoc examples compile as tests).

### Conventions (`ss2_core::math`)

Right-handed, **Y up, -Z forward**, radians, clip depth `0..=1`, clip Y up (wgpu/Direct3D). Types: `Vec2/3/4`, `Quat`, `Mat4` from `glam`. Converting from the original game's conventions is the loaders' job (T02/T01 specs), never the consumers'. Colors in the extract are **linear**.

### The extract: `RenderWorld`

```rust
pub struct RenderWorld {
    pub tick: u64,
    pub camera: Camera,
    pub renderables: Vec<Renderable>,
    pub lights: Vec<Light>,
    pub ui: ss2_ui::DrawList,   // 2D/UI/HUD, reused as-is
}
```

- **Plain owned data, `Send + 'static`**: no borrows into the sim, no GPU types, so it moves to a render thread or is simply dropped on a server. `reset(tick)` clears it while keeping allocations for per-tick reuse.
- `Camera { position, rotation, projection }` with `Projection::Perspective { fov_y, near }` (**infinite far, reverse-Z**, as T04-004 decided) or `Orthographic { height, near, far }` (also reverse-Z). `view_matrix`, `projection_matrix(aspect)`, `view_projection(aspect)`, `forward/up/right`, `is_valid`.
- `Renderable { id: RenderId, model: ModelId, transform, material_override, tint, animation, flags }`. `RenderId` is the stable per-entity identity the renderer uses to interpolate between ticks. `AnimState { clip, time }` says *what* plays *when*; the renderer samples the pose, so the sim never touches skeleton data. `RenderFlags` (`CAST_SHADOWS`, `VIEWMODEL`) is a bitset that can grow.
- `Light { kind: Point | Spot{..}, position, color, intensity, radius, casts_shadow }`: dynamic lights only; baked lighting is not extracted.
- `validate()` checks the invariants (valid camera, unique `RenderId`s, finite and normalized transforms/tints/lights, sane animation time). Renderers call it in `submit` and **reject** a bad world rather than drawing NaNs or panicking.

### Resources: ids, not handles

`ModelId`, `MaterialId`, `TextureId`, `ClipId` are `Copy` newtypes over `NonZeroU32` (so `Option<Id>` is free) that **mean nothing outside the renderer that issued them**. `RenderResources::{model,material,texture,clip}(&mut self, path: &str) -> Id` is cheap and idempotent (same normalized path, same id) and returns an id **immediately even while the asset is loading or after it failed**: the renderer shows a placeholder or skips the object. Callers resolve once at spawn and keep the id; asset load errors surface through `ss2-assets` (RFC 0001), not here. Paths are `&str` and normalized like `VfsPath`; `ss2_vfs::VfsPath` (RFC 0001, merged) exists, but `&str` keeps the API free of a dependency on `ss2-vfs` and is ergonomic for literals; callers holding a `VfsPath` pass `path.as_str()`.

### The trait: `Renderer`

```rust
pub trait Renderer {
    fn resources(&mut self) -> &mut dyn RenderResources;
    fn submit(&mut self, world: RenderWorld) -> Result<()>;   // once per sim tick
    fn render(&mut self, alpha: f32) -> Result<FrameStats>;   // once per display frame
}
```

**Interpolation rule** (part of the contract, so sim and renderer agree): the renderer keeps the previous and newest submitted world. `alpha` in `[0, 1]` runs from previous to newest. An object in both (same `RenderId`) is drawn at `Transform::interpolate` (lerp position/scale, slerp rotation); only in newest → drawn at its newest transform; only in previous → not drawn. The camera is interpolated the same way; UI always comes from the newest world. Display is thus at most one tick behind the sim in exchange for smooth motion at any frame rate (works with the fixed-timestep driver, T03-013). `submit` requires strictly increasing ticks, `render` rejects a non-finite `alpha` and clamps other out-of-range values.

Not in the contract (deliberately): window/surface creation and resize (platform contract, T00-016), GPU resource upload (T04-003 sits *behind* `RenderResources`), render graph, quality settings (T04-020), screenshots (T04-028).

### The mocks

- `NullRenderer`: validates and counts, draws nothing, keeps the last two worlds inspectable (`previous()`, `newest()`). Used by the headless server and by tests of code that feeds a renderer.
- `MockResources`: sequential ids per kind starting at 1, normalizes paths, `path_of_*` for assertions.

### `ss2-core` change

Adds `glam` as a workspace dependency and `ss2_core::math` (re-exports plus `projection::{perspective_infinite_reverse, orthographic}` for the wgpu depth convention). Other crates should use `ss2_core::math`, not `glam` directly, so a version bump is one edit.

## Alternatives considered

- **Put the contract inside `ss2-render`.** Rejected: the sim and the headless server would depend on `wgpu`/`glyphon`, defeating the extract design.
- **Put it in `ss2-core`.** Rejected: core is imported by everything including network and tools; render concepts would bloat and entangle it.
- **Pass asset `Handle<T>`s (RFC 0001) in the extract.** Rejected for v0: the model/material asset types do not exist yet (T02-014..018), the sim would need to know them, and handles are heavier than a `Copy` id in a per-tick hot path. The renderer can resolve ids to assets on its side.
- **A retained scene graph the sim mutates.** Rejected: cross-thread mutation and ownership problems; a per-tick snapshot is simpler, lock-free and matches the double-buffering planned in T04-026.
- **Component-reflection-based extraction (generic over the sim's entities).** Rejected here: depends on the unfinished sim contract (T00-017). That contract can later offer a helper that fills a `RenderWorld`; this API stays the target.

## Compatibility and migration

- New crate plus an addition to `ss2-core` (`math`, `glam` dependency). `ss2-ui` is unchanged (its `DrawList` is reused). No existing code changes; `ss2-render` later depends on `ss2-render-api` and implements `Renderer` (T04-026).
- The crate map in the design doc gains one line (`ss2-render-api`). After merge the signatures are frozen: changes need a new RFC. Compatible additions (new `RenderFlags` bits, new `Projection`/`LightKind` variants are *not* compatible: the enums are exhaustive, so adding a variant needs an RFC).
- Order: T04-026 implements `Renderer`; T07/T09 start building `RenderWorld`s against `NullRenderer` immediately.

## Open questions

- **Attachments** (weapon on hand, T04-006): parent `RenderId` + bone name vs. sim-computed world transforms. Decided by the `ss2-render` owner with T04-006.
- **UI textures/images**: `ss2_ui::DrawList` has colored quads and text only; images need `TextureId` there. Decided jointly with T10 (`ss2-ui` owner).
- **Particles and decals** are not in v0; they are likely separate lists (`pfx`/`decals`) added with T04-013/014 via RFC.
- **Instancing hints** (hordes, T04-023): whether the extract should carry a per-model instance grouping or the renderer should derive it. Decided with T04-023.
- **Skinned-model pose override** (ragdolls / physics-driven bones): a sim-supplied pose instead of `AnimState`. Decided with T07/T09.
