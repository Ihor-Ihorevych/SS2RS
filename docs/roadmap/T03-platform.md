# T03 — Platform, Window, Input, Android & Other Targets

Crates: `ss2-platform` (portable abstraction), `ss2-app` (client/listen-server exe), `ss2-server` (headless).
Principle: **PC first (Windows → Linux/macOS); Android and others are a later port (Phase 2, from M9).**
Until then only cheap guard rails apply: portable crates reach files/paths through platform traits and
handle suspend/surface-loss in the lifecycle contract; no mouse-only assumptions.

## Core platform (Wave 0)
- [ ] **T03-001** · Window + event loop on `winit`, `ApplicationHandler`-style lifecycle; contract from T00-016 (suspend/resume/surface-lost/DPI change) · `M` · `P0`
- [ ] **T03-002** · Keyboard/mouse input: raw motion, cursor capture/release, key repeat, text input events for console/chat · `M` · `P0` · 🟢
- [ ] **T03-003** · Gamepad via `gilrs` (+ Android native pads): mapping DB, dead zones, triggers, rumble · `M` · `P1` · 🟢
- [ ] **T03-004** · Touch input layer: multi-touch tracking, gesture primitives, per-finger ownership (look vs move) · `M` · `PHASE 2`
- [ ] **T03-005** · Input action system: data-driven action maps, contexts (menu/game/console), rebinding, serialization · `M` · `P0`
- [ ] **T03-012** · Platform paths: config/save/cache/data dirs per OS (`directories`), portable mode flag · `S` · `P0` · 🟢
- [ ] **T03-013** · Fixed-timestep driver + frame pacing: accumulator, max catch-up, interpolation alpha, vsync/uncapped, busy-wait policy · `M` · `P0`
- [ ] **T03-014** · Settings/config: TOML profile, schema + migration, per-platform defaults, CLI overrides · `M` · `P0`
- [ ] **T03-015** · Console backend: cvars/commands registry, history, autocomplete (UI in T10-007) · `M` · `P1`
- [ ] **T03-016** · Crash/panic handler: minidump-like report, last log tail, "copy to clipboard" · `S` · `P2` · 🟢
- [ ] **T03-017** · Display modes: windowed/borderless/exclusive, resolution list, HiDPI scale, vsync, monitor selection · `M` · `P1`
- [ ] **T03-018** · Job system/thread policy (rayon or custom): worker count, big.LITTLE affinity hints, named threads, tracing spans · `M` · `P1`
- [ ] **T03-019** · Logging backends: stdout/file rotate/`logcat`/Oslog · `S` · `P0` · 🟢
- [ ] **T03-020** · CLI & modes: `ss2-app [--connect host] [--listen] [--level X] [--data DIR]`, `ss2-server` headless (no window/GPU/audio) · `M` · `P1`

## Other PC platforms (after Windows works; before M8)
- [ ] **T03-026** · Windows: installer/portable zip, DPI-awareness manifest, Win10+ baseline · `S` · `P1` · 🟢
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
