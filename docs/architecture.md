# Architecture overview

This page explains how SS2RS is put together **as it exists today**, and where the planned parts
will plug in. The longer reasoning and the alternatives we rejected are in the
[design document](superpowers/specs/2026-11-21-ss2rs-design.md); the interfaces between systems are
the [RFCs](rfcs/0000-template.md).

## The idea in one picture

Every major system is split in two: a small, **dependency-light API crate** that defines the
contract, and an **implementation crate** that links the heavy libraries. Game code only ever
depends on the API crates.

```
                    gameplay, UI, world, app logic
                  (ss2-game, ss2-ui, ss2-world, ss2-app)
                                   |
        +--------------+-----------+-----------+----------------+
        v              v                       v                v
  ss2-vfs +      ss2-render-api         ss2-platform-api   ss2-audio-api
  ss2-assets     (RenderWorld extract)  (events, actions)  (voices, buses)
  (files, assets)       ^                       ^                ^
        |               | implements            | implements     | implements
        v               |                       |                |
  .gro / loose dirs   ss2-render              ss2-platform     ss2-audio
  (format loaders)    (wgpu, glyphon)         (winit, gilrs)   (cpal, decoders)

                  everything depends on ss2-core (errors, logging, math, settings)
```

Why: the **simulation and the headless dedicated server must not link a GPU, a window or an audio
device**, and every system must be testable without one. Each API crate ships a working
`Null*`/`Mock*` implementation, so code written against the contract runs in a plain unit test.

## Crates today

Status key: **done** = implemented and used; **contract** = API merged with a mock, the real
implementation is a roadmap task; **stub** = empty crate reserving the name.

| Crate | Role | Status |
|---|---|---|
| `ss2-core` | errors, logging, settings, shared math (`ss2_core::math`, a `glam` re-export) | done |
| `ss2-vfs` | `Vfs` trait, `VfsPath` (case-insensitive, normalized), in-memory mock | contract ([RFC 0001](rfcs/0001-vfs-and-asset-api.md)); archive reader is T02-001 |
| `ss2-assets` | `AssetLoader`, typed `Handle<T>`, `AssetManager` | contract ([RFC 0001](rfcs/0001-vfs-and-asset-api.md)); async manager is T02-009 |
| `ss2-render-api` | per-tick `RenderWorld` extract, camera, resource ids, `Renderer` trait | contract ([RFC 0002](rfcs/0002-render-frontend-api.md)) |
| `ss2-render` | wgpu renderer | partial: 2D quads and text, offscreen render and golden-image tests |
| `ss2-platform-api` | platform events, input actions, lifecycle, paths, `AppHandler` | contract ([RFC 0003](rfcs/0003-platform-api.md)) |
| `ss2-platform` | window, input devices (`winit`, `gilrs`) | partial: window and lifecycle loop (`window::run`, T03-001), keyboard and mouse events with cursor capture (T03-002), menu navigation mapping; gamepad is T03-003 |
| `ss2-audio-api` | `Audio` trait, sound ids, voices, buses, music | contract ([RFC 0004](rfcs/0004-audio-api.md)) |
| `ss2-audio` | mixer (`cpal`), decoders | stub |
| `ss2-ui` | UI framework, headless draw lists, placeholder menu | partial: menu slice |
| `ss2-app` | the client executable | partial: placeholder menu in a window |
| `ss2-testkit` | data-gated tests (`SS2_DATA_DIR`), synthetic fixtures (ZIP builder) | done |
| `ss2-bench` | `criterion` benchmarks and their fixtures ([budgets](PERF_BUDGETS.md)) | done |
| `ss2-meta`, `ss2-shaderxlat`, `ss2-physics`, `ss2-world`, `ss2-sim`, `ss2-net`, `ss2-game`, `ss2-video`, `ss2-script`, `ss2-server` | formats, shaders, physics, world, simulation, netcode, gameplay, video, mods, dedicated server | stubs, tracked by roadmap tracks T02, T05, T07, T08, T09, T06, T13 |

The `xtask` crate holds repository tooling (proprietary-content guard, roadmap sync), not engine code.

## How data and control flow

**Game data (planned, contracts merged).** The player's install is a set of `.gro` archives (plain
ZIP) layered with patches and mods. The VFS presents them as one case-insensitive tree. Almost every
asset starts with a `CTSEMETA` container ([spec](re/ctsemeta-container.md)); one reader decodes it
and format crates turn it into assets through `AssetLoader`. Consumers hold a `Handle<T>`, which may
still be loading: **never assume an asset is ready**.

**One frame (contracts merged).**

1. The platform loop (`ss2-platform`) turns window and gamepad events into `PlatformEvent`s and
   calls `AppHandler::event`, then `AppHandler::frame` ([RFC 0003](rfcs/0003-platform-api.md)).
2. Input events become **actions** (`fire`, `move_forward`) through a rebindable `ActionMap`;
   gameplay reads actions, never keys.
3. The simulation advances in fixed ticks (its own counter, not wall-clock time).
4. After each tick the simulation builds a `RenderWorld` (camera, renderables, lights, UI draw list)
   and `submit`s it. The renderer keeps the last two and, each display frame, draws the blend
   between them by `alpha` ([RFC 0002](rfcs/0002-render-frontend-api.md)). The simulation never
   touches GPU types.
5. Gameplay calls `Audio::play` for sounds and moves emitters with `update_voice`; the mixer runs
   on its own thread ([RFC 0004](rfcs/0004-audio-api.md)).
6. The lifecycle tracker says whether rendering is allowed (suspend, lost surface, minimized).

**Co-op (planned, T08).** One server-authoritative simulation; clients predict their own player and
interpolate everything else. The same movement code runs on server and client, which is why the
simulation crates have the strict rules below.

## Conventions every crate follows

- **Coordinates:** right-handed, **Y up, -Z forward**, radians, clip depth `0..=1` with reverse-Z.
  Conversion from the original game's conventions is the loaders' job, never the consumers'.
- **Colors** in contracts are **linear** RGBA.
- **Ids over handles for per-frame data:** renderer, audio and platform resources are small `Copy`
  ids resolved once (at spawn or level load) from a path; they are valid immediately even while the
  asset loads, and a missing asset is a placeholder or silence, not an error at the call site.
- **Never panic on bad input.** Game files, packets and settings are untrusted: return
  `ss2_core::Error`. Problems with *data* live in the result (a `Failed` handle); problems with
  *how you called* return `Err`.
- **Sim-safe crates** (`ss2-sim`, `ss2-physics`, `ss2-net`): no `HashMap`/`HashSet` iteration
  order, no wall-clock time, no thread-order dependence. Enforced by clippy `disallowed-types`.
  See [coding standards](CODING_STANDARDS.md).
- **Paths come from the platform layer** (`PlatformPaths`); no crate calls `current_dir` or
  resolves per-user directories itself.
- **Event and kind enums are `#[non_exhaustive]`** where new variants are expected, so adding one is
  not a breaking change; always keep a wildcard arm.

## Clean room

Specs in `docs/re/` describe *what the original does* in prose (byte layouts, rules, test
vectors); implementers write Rust **from the specs**, never from decompiler output or SS1 (GPL)
code. Game content and tools' projects are never committed; CI enforces it. Details are in
`CLEANROOM.md` in the repository root.

## Where to go next

- Changing an interface? Write an [RFC](rfcs/0000-template.md) first; contracts are frozen on merge.
- Picking up work? Follow the claim protocol in `AGENTS.md`, then read the track file in the
  [roadmap](roadmap/README.md).
- Touching a hot path? Add a benchmark and check the [budgets](PERF_BUDGETS.md).
- Wondering what a file format means? Check the [specs](re/README.md) before guessing.
