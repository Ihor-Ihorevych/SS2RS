# T03 — Platform, Window, Input, Android & Other Targets

Crates: `ss2-platform` (portable abstraction), `ss2-app` (client/listen-server exe), `ss2-server` (headless).
Principle: **PC first (Windows → Linux/macOS); Android and others are a later port (Phase 2, from M9).**
Until then only cheap guard rails apply: portable crates reach files/paths through platform traits and
handle suspend/surface-loss in the lifecycle contract; no mouse-only assumptions.

## Core platform (Wave 0)
- [x] **T03-001** · Window + event loop on `winit`, `ApplicationHandler`-style lifecycle; contract from T00-016 (suspend/resume/surface-lost/DPI change) · `M` · `P0`
- [x] **T03-002** · Keyboard/mouse input: raw motion, cursor capture/release, key repeat, text input events for console/chat · `M` · `P0` · 🟢
- [x] **T03-003** · Gamepad via `gilrs` (+ Android native pads): mapping DB, dead zones, triggers, rumble: `ss2_platform::gamepad` (`docs/GAMEPADS.md`): standard layout, radial dead zones, triggers, the `gilrs` mapping DB plus `SDL_GAMECONTROLLERCONFIG`, rumble; polled by the window loop; **no real pad has been tried** and Android's native pads are T03-021 · `M` · `P1` · 🟢
- [ ] **T03-004** · Touch input layer: multi-touch tracking, gesture primitives, per-finger ownership (look vs move) · `M` · `PHASE 2`
- [x] **T03-005** · Input action system: data-driven action maps, contexts (menu/game/console), rebinding, serialization · `M` · `P0`
- [x] **T03-012** · Platform paths: config/save/cache/data dirs per OS (`directories`), portable mode flag · `S` · `P0` · 🟢
- [x] **T03-013** · Fixed-timestep driver + frame pacing: accumulator, max catch-up, interpolation alpha, vsync/uncapped, busy-wait policy · `M` · `P0`
- [x] **T03-014** · Settings/config: TOML profile, schema + migration, per-platform defaults, CLI overrides · `M` · `P0`
- [x] **T03-015** · Console backend: cvars/commands registry, history, autocomplete (UI in T10-007) · `M` · `P1`
- [x] **T03-016** · Crash/panic handler: minidump-like report, last log tail, "copy to clipboard" · `S` · `P2` · 🟢
- [x] **T03-017** · Display modes: windowed/borderless/exclusive, resolution list, HiDPI scale, vsync, monitor selection · `M` · `P1`
- [x] **T03-018** · Job system/thread policy (rayon or custom): worker count, big.LITTLE affinity hints, named threads, tracing spans · `M` · `P1`
- [x] **T03-019** · Logging backends: stdout and size-rotated file (`ss2_core::logfile`, `init_logging_with_file`); `logcat`/Oslog split into T03-034 · `S` · `P0` · 🟢
- [x] **T03-020** · CLI & modes: `ss2-app [--connect host] [--listen] [--level X] [--data DIR]`, `ss2-server` headless (no window/GPU/audio) · `M` · `P1`

## Other PC platforms (after Windows works; before M8)
- [x] **T03-026** · Windows: portable zip (`release.yml`), DPI-awareness manifest (per-monitor v2, long paths, UTF-8 code page) and the Windows 10+ baseline in `ss2-app.exe`; installer and code signing split into T03-035 · `S` · `P1` · 🟢
- [ ] **T03-011** · Linux: Wayland+X11, AppImage/Flatpak, Steam Deck/gamescope, handheld controller layouts · `M` · `P2`
- [ ] **T03-010** · macOS: `.app` bundle, Metal path verification, notarization notes, Apple Silicon + x86_64 · `M` · `P2`

## PHASE 2 — Android port (M9, not scheduled before M8)
- [ ] **T03-006** · Android build pipeline: Gradle/`cargo-ndk` (or `xbuild`), `GameActivity`, ABIs arm64 (+x86_64 emulator), debug+release APK/AAB, CI artifact · `L` · `PHASE 2`
- [ ] **T03-007** · Game-data import: user picks the folder (SAF) or copies the install to app storage; progress UI; verify by listing `.gro` files + version; resumable; **no data bundled in the APK** · `L` · `PHASE 2`
- [ ] **T03-008** · Lifecycle: surface destroyed/recreated, `onPause` freeing GPU resources, immersive mode, notch/safe-area insets, orientation lock, back-button · `M` · `PHASE 2`
- [ ] **T03-021** · Android input glue: touch + attached mouse/keyboard + gamepad, IME for chat, haptics · `M` · `PHASE 2`
- [ ] **T03-022** · Android perf: thermal state API hooks, Adaptive Performance/frame-pacing (Swappy), sustained-perf mode, battery saver detection · `M` · `PHASE 2`
- [ ] **T03-023** · Android storage/memory: low-memory callbacks → asset cache shrink; large-file mmap behavior on content URIs · `M` · `PHASE 2`
- [ ] **T03-024** · Android packaging: signing, versioning, permissions minimal, Play/F-Droid policy notes (data supplied by user) · `M` · `PHASE 2`
- [ ] **T03-009** · Touch controls framework: virtual stick, look-pad, fire/jump/crouch/reload/weapon-switch buttons, layout presets, per-button scale/opacity, left/right-handed, edit mode (UI side in T10-006) · `L` · `PHASE 2`
- [ ] **T03-025** · Aim assist & gyro aiming (optional), sensitivity curves, auto-fire option · `M` · `PHASE 2`

## PHASE 2 — Other platforms (M10)
- [ ] **T03-027** · iOS exploratory bring-up (Xcode project, Metal, data import via Files) · `L` · `PHASE 2`
- [ ] **T03-028** · Raspberry Pi / ARM Linux handheld check (Vulkan/GLES fallback) · `M` · `PHASE 2`
- [ ] **T03-029** · Web (wasm) feasibility note only (no work) · `S` · `PHASE 2` · 🟢

## Menu slice (PC, placeholder skin) — see `docs/superpowers/plans/2026-11-21-menu-slice-placeholder-skin.md`
- [x] **T03-030** · Settings persistence: `Settings` (vsync, fullscreen, volumes) load/save as TOML in the platform config dir, atomic write, clamped, corrupt/missing file → defaults · `M` · `P1` · Deps: T10-031
- [x] **T03-031** · Navigation input mapping: keyboard (arrows/WASD/Enter/Esc) and gamepad (D-pad, buttons, stick with dead zone + repeat) → `NavInput` · `M` · `P1` · Deps: T10-030
- [x] **T03-032** · `ss2-app` window loop and menu wiring (winit 0.30): CLI (`--screenshot`, `--smoke-frames`), scenes (menu/placeholder game), live vsync/fullscreen, minimize-safe · `M` · `P1` · Deps: T04-036, T03-031, T03-030
- [x] **T03-033** · Move `ss2-app` onto `ss2_platform::window::run`: the app implements `PlatformApp` (renderer created in `window_ready`, dropped in `window_lost`) and reads input as `PlatformEvent`s instead of raw `winit`/`gilrs` calls (split from T03-001, which delivered the loop and lifecycle; the input events are T03-002/T03-003) (done: the app implements `AppHandler` + `PlatformApp`, maps keys and gamepad buttons from `ss2_platform_api` (`ss2-app/src/input.rs`), stick repeat from frame deltas, `winit`/`gilrs` dropped from `ss2-app`; headless tests through `run_script`; a 180-frame smoke run on Windows) · `M` · `P1` · Deps: T03-001, T03-002, T03-003
- [ ] **T03-034** · Platform log backends: Android `logcat` and macOS `OSLog` layers for `tracing` (behind `cfg(target_os)`; needs the `tracing-android` / `tracing-oslog` crates and the target toolchains for the portability check; the workspace forbids `unsafe`) · `S` · `P2` · Deps: T03-019
- [ ] **T03-035** · Windows installer (WiX/MSI or Inno Setup) and Authenticode code signing (needs a certificate and the owner's decision; unsigned builds are stated in the release notes until then) · `M` · `P2` · Deps: T03-026
- [x] **T03-036** · App level session: mount the install (`--data`, then `SS2_DATA_DIR`, then discovery), load a level by id and draw it, and an offscreen `--level NAME --screenshot OUT [--size WxH]` mode (`ss2-app/src/level_session.rs`: `GameData` mounts the install and finds a level by id or world name, `upload` puts a `LevelScene` on the renderer with white stand-ins for texture formats the GPU lacks, `start_camera` stands at eye height on the start; checked on `dm_dockingbay` and `1_1_jungle` by eye and by a data-gated test that stores no image; playable MVP milestone A; RFC 0013 (`docs/rfcs/0013-level-scene-boundary.md`)) · `S` · `P0` · Deps: T04-038, T07-028, T02-038
- [x] **T03-037** · App play scene (playable MVP milestone B): the level picker from "New Game", loading on a worker with a progress bar, free-fly and walking with mouse look, pause and back to the menu (thin slices of T10-004, T10-009b and T11-005); built on the platform loop of T03-033 (`ss2-app/src/play.rs`: install levels plus a built-in test room, loading on a worker thread with a `LoadingTracker` bar, the HUD line; `play_input.rs`: keyboard, mouse and gamepad to `PlayInput`; `app.rs`: picker, loading, play, pause, quit to menu, cursor capture, re-upload after a device rebuild, `--level NAME`; `TTRS:` level names shown by their default text; headless tests through `run_script`, a real window run into `1_1_jungle`; `docs/PLAYING.md`) · `M` · `P0` · Deps: T03-036, T07-031, T10-034
- [x] **T03-038** · The app draws the level's sky: `level_session::upload` uploads `LevelScene::sky` with `Renderer::set_sky`, in the play scene and the `--screenshot` mode (the face textures reuse the scene's uploaded textures and stand-ins; `UploadReport::sky`); tested on `test_room()` (looking straight up shows the sky's top face, a level without a sky clears as before); real levels show their sky once T07-033 fills `LevelScene::sky` (design `docs/superpowers/specs/2026-11-27-sky-and-lightmaps-design.md`) · `S` · `P0` · Deps: T04-039, T07-032
- [x] **T03-039** · The app draws lightmaps: `level_session::upload` makes one batch per lit instance with its lightmap (design `docs/superpowers/specs/2026-11-27-lightmaps-design.md`); unlit placements stay grouped, a part without lightmap UVs keeps the flat light, `UploadReport::lightmapped`; tested on `test_room()` (the floor's shadow ratio from above). The check on real levels is T03-039b · `S` · `P0` · Deps: T04-040, T07-034
- [x] **T03-039b** · Check lightmaps on real levels by eye once T07-035 fills them: shadows where the original has them (`1_1_jungle`, `dm_stadium`, a city level), the brightness factor (x1 or an overbright x2, `docs/superpowers/specs/2026-11-27-lightmaps-design.md`) and the UV orientation; record the result in the design note and fix the renderer or loader if needed · `S` · `P0` · Deps: T03-039, T07-035 *Delivered: shadows correct on `dm_stadium`, `1_1_jungle` and `1_7_city_metropolis`; V as authored; brightness x1 (overbright x2 blows out); no code change needed.*
- [x] **T03-040** · The app fogs the world: each frame `renderer.set_fog(scene.fog_at(eye))`; checked by eye on `1_3_village` · `S` · `P0` · Deps: T07-041, T04-042 *Delivered: checked on `1_3_village` (start inside its haze: near -150, far 1860, dark blue-grey; distant cliffs darken, nothing washed out), `1_2_approachingvillage` and `1_1_jungle`; the screenshot log prints the fog used.*
- [x] **T03-041** · The app blooms the world: each frame `renderer.set_blooms(&scene.blooms_at(eye))`, also in `--screenshot`; checked by eye on `1_3_village` and `dm_hole` · `S` · `P1` · Deps: T07-048, T04-043 *Delivered: `1_3_village`'s start is inside its bloom (colour 0x463c26, strength 1.24): the chain only brightens, 0.5 % of channels, at most +19/255, on the moon and bright sky (the high-pass keeps only the brightest); nothing darkens. `dm_hole`'s start faces a plain wall, so nothing there blooms to judge. The screenshot log prints the blooms used.*
