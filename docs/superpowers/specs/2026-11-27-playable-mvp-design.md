# Playable MVP: pick a level and run around in it

- **Date:** 2026-11-27
- **Roadmap:** T00-035 (this document and RFC 0013); the slices listed under "Work breakdown"
- **Contract:** [RFC 0013 Level scene boundary](../../rfcs/0013-level-scene-boundary.md)

## Intent

Today `ss2-app` opens a window with a placeholder menu, and "New Game" shows an empty screen. Nothing mounts
the game data, lists levels, reads a `.wld`, draws in 3D or turns input into movement. The owner wants
**to see the game running soon**: start `ss2-app`, pick a level from a list, and walk around it with
collision. This is the first visible slice of milestone M3 ("Walk a level").

## Goals

- Start `ss2-app`, choose a level from a list built from the player's own install, see a loading bar, then
  stand in the level.
- The level shows its textured static geometry with flat lighting.
- Walk with gravity, jump and step up small ledges; a key toggles free-fly; Esc pauses and returns to the
  menu, where another level can be picked.
- Start at a spawn marker if the level has one, otherwise at a sensible fallback.
- An offscreen mode, `ss2-app --level NAME --screenshot OUT`, renders a real level to a PNG without opening
  a window.

## Non-goals

Enemies, weapons, HUD, audio, co-op, lightmaps and dynamic lights, sky, particles, doors and other
mechanisms, saves, the styled menu skin, the first-run data wizard (T10-019b), and the measured movement
model (T01-024).

## Anti-goals

- Throwaway code. Every piece lands as a claimed roadmap task in the crate where it belongs and is tested
  to the Definition of Done, even when its scope is narrow.
- Breaking the clean-room rule to go faster (see "Constraints").
- Committing game content, including screenshots of real levels.

## Constraints

- **Clean room (CLEANROOM.md).** The agent planning this wrote the `.wld`, `.mdl`, `.bmf`, `.mtr` and `.tex`
  specs, so it must not write the code that reads those formats. Fresh agents that read only `docs/re/`,
  public documents and existing product code write those slices. The boundary between the two sides is RFC
  0013: format-neutral types the planning agent may define and consume.
- **One binary.** The MVP lives inside `ss2-app`, which first moves onto `ss2_platform::window::run`
  (T03-033) for cursor capture and raw mouse motion. That task is claimed by another contributor; it is
  taken over only once released.
- **Sim-safe crates.** The controller in `ss2-physics` uses no `HashMap`/`HashSet` and no wall clock.
- **CI cannot run** (billing). Every PR is merged after the full local gate on its final tree.

## Approaches considered

1. **Build inside `ss2-app` now, on its current winit loop.** Matches the request most directly, but
   collides with T03-033 and needs mouse capture added to a loop that is about to be replaced.
2. **A separate viewer binary on the new platform loop.** Fastest to pixels and conflict-free, but the
   owner wants one binary.
3. **Take over T03-033 first, then build inside `ss2-app`.** Chosen by the owner: one binary, built on the
   loop it will keep. The cost (a prerequisite before play) is reduced by doing every library slice and an
   offscreen screenshot mode first, and by a small input shim if the takeover is late.

Within the chosen approach, each XL roadmap task on the path (T02-020 world loader, T02-017 model loader,
T04-005/T04-009 mesh and world rendering, T07-005 controller, T07-023 world instance) is cut to a narrow
slice. The shortcut that makes this small: levels are placed models (`docs/re/wld-geometry.md`), and
`ss2_format_bmf::Mesh::from_bytes` already reads the mesh embedded in most `.mdl` files.

## Design

All logic lives in library crates; `ss2-app` is glue. RFC 0013 fixes the types between the pieces:

```
.wld/.mdl/.tex bytes --(ss2-format-wld, ss2-format-mdl, ss2-format-tex)--> format types
format types --(ss2_world::load::load_level)--> LevelScene            (format-neutral)
LevelScene --(ss2_world::collision::build_static)--> QueryWorld        (ss2-physics)
LevelScene --(app)--> Renderer::create_mesh/create_texture/set_world   (ss2-render)
QueryWorld + input --(ss2_physics::controller::Walker)--> position --> FreeFlyCamera --> Camera
```

- **Level list:** `ss2_vfs::levels::discover_levels` walks `content/serioussam2/levels` for level `.nfo`
  files (the walker already proven in `ss2-game`'s data tests) and pairs each with the `.wld` next to it.
- **Picker:** a scrolling list widget in `ss2-ui`, opened by the existing "New Game" command.
- **Loading:** a worker thread builds the `LevelScene` and the collision world; the main thread uploads to
  the GPU a few resources per frame and draws a bar from `ss2_ui::loading::LoadingTracker`.
- **3D pass:** reverse-Z `Depth32Float`, one textured static-mesh pipeline with per-instance matrices, a
  fixed directional light term, alpha cutoff; the existing 2D pass draws on top. No culling at first,
  because winding and the UV direction are unverified until the first rendered comparison.
- **Textures:** DXT data uploaded as BC1/2/3 when the adapter supports it, otherwise white.
- **Movement:** `FreeFlyCamera` (already in `ss2-game`) owns the view in both modes; in walk mode the
  camera sits at the walker's eye. The walker is two stacked spheres swept against the level triangles
  (capsule casts do not exist yet), fixed 1/120 s substeps, gravity from the level, jump, slide, step-up,
  ground probe. Its numbers are tuned by feel and marked provisional until T01-024 measures the original.
- **Collision geometry:** each collidable instance's coarsest render LOD, transformed into one `TriMesh`
  (the real collision-mesh layout is still being specified, T01-055).

## Work breakdown

| ID | Slice | Crate | Writer |
|---|---|---|---|
| T00-035 | This document, RFC 0013, the roadmap lines | docs | planning agent |
| T04-037 | CPU drawable data (`scene_data`) | `ss2-render-api` | planning agent |
| T07-027 | `LevelScene` and `test_room()` | `ss2-world` | planning agent |
| T04-038 | 3D static mesh pass | `ss2-render` | planning agent |
| T07-029 | Collision from a `LevelScene` | `ss2-world` | planning agent |
| T07-030 | Provisional walking controller | `ss2-physics` | planning agent |
| T02-038 | Level discovery | `ss2-vfs` | planning agent |
| T10-034 | Level picker widget | `ss2-ui` | planning agent |
| T02-035 | Minimal `.wld` reader | new `ss2-format-wld` | fresh spec-only agent |
| T02-036 | Minimal `.mdl` reader | new `ss2-format-mdl` | fresh spec-only agent |
| T07-028 | `load_level` (format types to `LevelScene`) | `ss2-world` | fresh spec-only agent |
| T03-036 | App data session and `--level --screenshot` | `ss2-app` | planning agent |
| T03-033 | App onto the platform loop (takeover) | `ss2-app` | planning agent, once released |
| T03-037 | Play scene: picker, loading, walk/fly, pause | `ss2-app` | planning agent |

Order: T04-037 and T07-027 first (they give everyone `test_room()`); then the fresh agents' readers in
parallel with the renderer, collision, controller, discovery and picker slices; then `load_level`; then the
offscreen screenshot (**milestone A: a real level as a PNG**); then T03-033 and the play scene
(**milestone B: playable**). If T03-033 is not released by then, the play scene is written against
`ss2_platform_api::InputEvent` and fed by a small shim in the current loop, which T03-033 later deletes.

## Verification

- Unit tests on `test_room()`: scene validation; the controller stands, stops at walls, climbs a 0.4 m step
  but not a 1.0 m one, jumps to the expected height, never tunnels at 60 m/s and repeats the same trace.
- A committed golden image of `test_room()` from the offscreen 3D pass.
- Data-gated tests on the real install: all worlds and models parse, 58 levels are discovered,
  `DM_DockingBay` loads with instances and textures. No real-level image is committed.
- Milestone A: `ss2-app --level DM_DockingBay --screenshot out.png` looked at by eye, then `1_1_jungle`
  with load time and memory recorded.
- Milestone B: a manual run from menu to level list, loading bar, walking, jumping, free-fly, pause, back,
  another level.

## Left to the builders

Exact internal structure of each slice, test fixtures, error wording, the tuning numbers of the
controller, and which levels to cut if large campaign levels prove too slow to load (first cuts: collide
only near the start, cap textures at 512 px, ship with the small deathmatch levels first).
