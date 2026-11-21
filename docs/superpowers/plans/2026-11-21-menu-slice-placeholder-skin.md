# Menu Slice (Placeholder Skin) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Windows-first `ss2-app` that opens a window and shows a working, placeholder-skinned menu (Main → Options, Pause) driven by keyboard, mouse and gamepad, with persisted settings and a screenshot mode for golden-image tests. No game data is used.

**Architecture:** `ss2-ui` is headless: menu model, focus/navigation, layout and a `DrawList` of quads and text. `ss2-core` owns the tiny shared types (`NavInput`, `Settings`). `ss2-platform` maps winit/gilrs input to `NavInput`. `ss2-render` turns a `DrawList` into pixels with `wgpu` (quads) and `glyphon` (text), windowed or offscreen. `ss2-app` wires it together. Everything except GPU/window code is unit-testable without a display.

**Tech Stack:** Rust 1.99.1; `wgpu 30`, `glyphon 0.12`, `winit 0.30` (stable line; 0.31 is still beta), `gilrs 0.11`, `serde 1` + `toml 1`, `directories 6`, `png 0.18`, `pollster 1`, `bytemuck 1`, `thiserror 2`, `clap`-free hand-rolled CLI.

**Spec:** `docs/superpowers/specs/2026-11-21-ss2rs-design.md` (G1 PC first; §8 architecture; renderer fed through an extract-style interface). Roadmap tasks are named in each task; the slice adds new IDs (Task 0).

## Global Constraints

- PC first (Windows primary; Linux/macOS must still compile). No Android/touch work (`PHASE 2`).
- Workspace rules from the M0 plan still hold: Rust `1.99.1`, edition 2024, license Apache-2.0, `unsafe_code = "deny"`, `cargo fmt`, `clippy -D warnings`, `cargo deny`, DCO sign-off (`git commit -s`), task-ID-prefixed commits.
- Claim protocol (`AGENTS.md` §1): claim each new roadmap ID `[INPRG]` on `main` before starting it; tick it `[x]` and remove the tag in the same branch's last commit.
- `ss2-ui` has **no** dependency on `wgpu`, `winit`, `gilrs` or any OS API; `ss2-render` has **no** dependency on `winit` (it takes `Into<wgpu::SurfaceTarget<'static>>`).
- Virtual canvas is **1920×1080** units; the UI is laid out in those units and scaled/letterboxed to the window (T10-017).
- Placeholder look (decided, not original art): background `(0.06, 0.06, 0.08)`, idle item `(0.16, 0.17, 0.20)`, focused item `(0.85, 0.45, 0.10)`, text white; font **Inter Regular (SIL OFL 1.1)** embedded with its license text. No Croteam assets anywhere.
- Defaults: `vsync = true`, `fullscreen = false`, `master_volume = 0.8`, `music_volume = 0.6`, `sfx_volume = 0.8`; volume step `0.05`, range `[0.0, 1.0]`.
- Tests that need a GPU print `SKIPPED (no GPU adapter): <reason>` and pass when no adapter exists.

## Review Focus

1. **Minimized window (0×0 size):** no panic, no division by zero, no rendering while minimized; restores correctly. (Tasks 1, 8)
2. **Corrupt, empty, partial or unreadable `settings.toml`, or an unwritable config dir:** app still starts with defaults, never panics; saving is atomic so a crash can't leave a half-written file. (Task 4)
3. **Out-of-range or `NaN` values in settings** (`master_volume = 7.0`, `nan`) are clamped or defaulted, never passed on. (Task 4)
4. **Gamepad stick drift and holding:** values under the dead zone never navigate; a held stick repeats at the set rate, not every frame. (Task 5)
5. **Pointer edge cases:** press outside every item does nothing; press on A then release on B does not activate either; hover moves focus; `Back` on the main menu does nothing. (Tasks 2, 3)
6. **No GPU adapter** (CI, remote desktop): offscreen tests skip with a message, windowed start fails with a readable error, not a panic. (Tasks 6–8)

---

### Task 0: Roadmap entries and claims (bookkeeping)

**Files:** Modify `docs/roadmap/T03-platform.md`, `T04-renderer.md`, `T10-ui.md`.

**Interfaces:** Produces the IDs used below: `T10-030` UI geometry/canvas/`NavInput`; `T10-031` menu model + navigation; `T10-032` placeholder-skin layout/draw/hit-test; `T03-030` settings persistence; `T03-031` nav input mapping (keyboard/gamepad); `T03-032` `ss2-app` window loop + menu wiring; `T04-035` wgpu context + 2D batcher + glyphon text; `T04-036` offscreen render + golden tests.

- [ ] **Step 1:** Append the eight tasks (size `M`, `P1`, `Deps:` pointing at each other in task order, one-line description each, noting "partial T10-004/T10-001…; original-data menu skin remains T10-004") to their track files. Also append `T10-033` "Menu skin and flow from original `.rsc`/`.tex`/`.fnt` data (replaces placeholder skin)" `L` `P1`, unclaimed.
- [ ] **Step 2:** Run `cargo xtask roadmap-sync` only if it exists; otherwise skip. Commit `T10-030: add menu-slice roadmap entries`, push to `main` (docs-only).
- [ ] **Step 3:** Claim `T10-030` per `AGENTS.md` §1 (and each following ID right before its task).

---

### Task 1: `NavInput`, geometry, draw list, canvas fit (T10-030)

**Files:**
- Create: `crates/ss2-core/src/input.rs`; `crates/ss2-ui/src/geometry.rs`, `crates/ss2-ui/src/draw.rs`
- Modify: `crates/ss2-core/src/lib.rs`, `crates/ss2-ui/src/lib.rs`, `crates/ss2-ui/Cargo.toml` (dep `ss2-core`)
- Test: unit tests in `geometry.rs` and `draw.rs`

**Interfaces:**
- Produces (`ss2_core::input`): `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub enum NavInput { Up, Down, Left, Right, Activate, Back }`.
- Produces (`ss2_ui`): `#[derive(Debug, Clone, Copy, PartialEq, Default)] pub struct Vec2 { pub x: f32, pub y: f32 }`; `pub struct Rect { pub x, y, w, h: f32 }` with `pub fn contains(&self, p: Vec2) -> bool` (half-open: left/top inclusive, right/bottom exclusive); `pub struct Color(pub [f32; 4])`; `pub struct Quad { pub rect: Rect, pub color: Color }`; `pub enum TextAlign { Left, Center }`; `pub struct TextRun { pub pos: Vec2, pub text: String, pub size: f32, pub color: Color, pub align: TextAlign }` (`pos` is the left edge for `Left`, the horizontal center for `Center`, top for both); `#[derive(Default)] pub struct DrawList { pub quads: Vec<Quad>, pub texts: Vec<TextRun> }`; `pub const CANVAS_W: f32 = 1920.0; pub const CANVAS_H: f32 = 1080.0;`; `pub struct Canvas { pub scale: f32, pub offset: Vec2 }` with `pub fn fit(width_px: u32, height_px: u32) -> Option<Canvas>` (`None` if either is 0), `pub fn to_screen(&self, p: Vec2) -> Vec2`, `pub fn to_canvas(&self, px: Vec2) -> Vec2`. `fit` uses `scale = min(w/1920, h/1080)` and centers (letterbox).

- [ ] **Step 1: Write failing tests:** `canvas_fit_exact_16_9` (1920×1080 → scale 1.0, offset (0,0)); `canvas_fit_ultrawide_centers` (2560×1080 → scale 1.0, offset.x 320); `canvas_fit_small` (1280×720 → scale ≈ 0.6667, offset (0,0)); `canvas_zero_size_is_none` (0×720 and 1280×0 → `None`); `canvas_roundtrip` (`to_canvas(to_screen(p)) ≈ p` for a few points, tolerance 1e-3); `rect_contains_is_half_open`; `nav_input_is_copy_and_hashable` (use as `HashSet` key).
- [ ] **Step 2: Run** `cargo test -p ss2-core -p ss2-ui` → Expected: FAIL (items undefined).
- [ ] **Step 3: Implement** the types as specified; `lib.rs` re-exports them.
- [ ] **Step 4: Run** tests + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check` → Expected: PASS.
- [ ] **Step 5: Commit** `T10-030: NavInput, UI geometry, draw list and canvas fit`.

---

### Task 2: Menu model and navigation (T10-031)

**Files:**
- Create: `crates/ss2-ui/src/menu.rs`, `crates/ss2-ui/src/settings_view.rs` (slider/toggle helpers if needed)
- Modify: `crates/ss2-ui/src/lib.rs`, `crates/ss2-ui/Cargo.toml`; `crates/ss2-core/src/lib.rs` + new `crates/ss2-core/src/settings.rs` containing **only the `Settings` struct, `Default`, and `clamped()`** (persistence is Task 4)
- Test: unit tests in `menu.rs`

**Interfaces:**
- Consumes: `ss2_core::input::NavInput`; `ss2_ui::Vec2`.
- Produces (`ss2_core::settings`): `#[derive(Debug, Clone, PartialEq)] pub struct Settings { pub vsync: bool, pub fullscreen: bool, pub master_volume: f32, pub music_volume: f32, pub sfx_volume: f32 }` with `Default` per Global Constraints and `pub fn clamped(self) -> Settings` (clamps volumes to `[0,1]`, replaces `NaN` with the default for that field).
- Produces (`ss2_ui`): `pub enum ScreenId { Main, Options, Pause }`; `pub enum UiInput { Nav(NavInput), PointerMoved(Vec2), PointerPressed(Vec2), PointerReleased(Vec2) }`; `pub enum MenuCommand { StartGame, Resume, QuitToMenu, Quit, SettingsChanged(Settings) }`; `pub struct Menu`; `Menu::new(settings: Settings) -> Menu` (starts on `Main`, first focusable item focused); `pub fn open_pause(&mut self)` (stack = `[Pause]`); `pub fn open_main(&mut self)` (stack = `[Main]`); `pub fn screen(&self) -> ScreenId`; `pub fn focused(&self) -> Option<usize>` (index into the screen's items); `pub fn settings(&self) -> &Settings`; `pub fn handle(&mut self, input: UiInput) -> Vec<MenuCommand>`.
- Screen contents (decided): **Main** "SS2RS": `New Game` (→ `StartGame`), `Options` (opens Options), `Quit` (→ `Quit`). **Options** "Options": toggle `Vsync`, toggle `Fullscreen`, sliders `Master`, `Music`, `SFX`, a spacer, `Back`. **Pause** "Paused": `Resume`, `Options`, `Quit to Menu`. `Back` input: Options → pop to previous screen; Pause → `Resume`; Main → nothing.

- [ ] **Step 1: Write failing tests** (all headless, keyboard path only in this task):
  `starts_on_main_with_first_item_focused`; `down_up_wrap_around` (Up from first → last, Down from last → first); `spacer_is_skipped_by_focus` (Options: Down from `SFX` lands on `Back`); `activate_new_game_emits_start_game`; `activate_options_opens_options_and_back_returns_to_main`; `back_on_main_emits_nothing`; `back_on_pause_emits_resume`; `toggle_vsync_flips_and_emits_settings_changed` (new settings in the command, `menu.settings()` updated); `slider_right_adds_step_and_emits`; `slider_clamps_at_one_without_emitting` (Master 1.0, Right → no command); `slider_left_clamps_at_zero`; `quit_to_menu_from_pause_emits_quit_to_menu_and_shows_main`.
- [ ] **Step 2: Run** `cargo test -p ss2-core -p ss2-ui` → Expected: FAIL.
- [ ] **Step 3: Implement** `Menu` with a screen stack and per-screen item lists (`Item::{Button, Toggle, Slider, Spacer}` kept private to the crate); focus index over focusable items only; `handle` returns commands, never panics on any input sequence.
- [ ] **Step 4: Run** tests, clippy, fmt → Expected: PASS.
- [ ] **Step 5: Commit** `T10-031: menu model, screens and keyboard/gamepad navigation`.

---

### Task 3: Layout, draw list and pointer input (T10-032)

**Files:**
- Create: `crates/ss2-ui/src/layout.rs`
- Modify: `crates/ss2-ui/src/menu.rs` (pointer handling, `draw`, `item_rects`), `crates/ss2-ui/src/lib.rs`
- Test: unit tests in `layout.rs` and `menu.rs`

**Interfaces:**
- Consumes: Task 2's `Menu`, `UiInput`; Task 1's `DrawList`, `Rect`, `Color`.
- Produces: `Menu::item_rects(&self) -> Vec<Rect>` (one rect per item incl. spacers, canvas units) and `Menu::draw(&self) -> DrawList`. Layout (decided): title text centered at x=960, y=160, size 96; items start at y=320; button/toggle/slider height 72, width 640, x=640, gap 16; spacer height 36. Idle/focused colors per Global Constraints. A button draws one quad + one label (centered); a toggle draws quad + label left (x+24) + value text `On`/`Off` right-aligned look via a second centered text at x+560; a slider draws quad, label left, a track quad (x+300..x+600, height 8) and a fill quad proportional to value, plus value text.
- Pointer rules: `PointerMoved` over a focusable item focuses it (no command); `PointerPressed` records the pressed item; `PointerReleased` over the **same** item activates it (as `Activate`); press/release elsewhere or on different items does nothing; a press on a slider track sets the value from the x position (snapped to step 0.05) and emits `SettingsChanged`.

- [ ] **Step 1: Write failing tests:** `button_rects_are_centered_640_wide` (first button x=640,w=640,h=72); `rects_do_not_overlap_and_follow_gap` (consecutive gap 16, spacer 36 handled); `focused_item_uses_accent_color_others_idle` (inspect `draw().quads`); `draw_has_title_and_one_label_per_button` (texts contain "SS2RS", "New Game", "Options", "Quit"); `hover_moves_focus_without_command`; `press_release_same_item_activates`; `press_a_release_b_does_nothing`; `press_outside_all_items_does_nothing`; `slider_track_press_sets_value_snapped` (press at 50 % of the track → 0.50; emits `SettingsChanged`); `empty_pointer_sequence_never_panics` (random-ish sequence of 200 inputs over each screen, asserts no panic and `focused()` always in range).
- [ ] **Step 2: Run** `cargo test -p ss2-ui` → Expected: FAIL.
- [ ] **Step 3: Implement** layout and drawing; keep hit-testing driven by `item_rects`.
- [ ] **Step 4: Run** tests, clippy, fmt → Expected: PASS.
- [ ] **Step 5: Commit** `T10-032: placeholder-skin layout, draw list and pointer input`.

---

### Task 4: Settings persistence (T03-030)

**Files:**
- Modify: `crates/ss2-core/src/settings.rs`, `crates/ss2-core/Cargo.toml` (deps `serde` with `derive`, `toml`, `directories`; dev-dep `tempfile`)
- Test: unit tests in `settings.rs`

**Interfaces:**
- Consumes: `Settings` from Task 2.
- Produces: `Settings` gains `Serialize`/`Deserialize` with `#[serde(default)]` (missing/unknown fields tolerated); `pub fn load_from(path: &Path) -> Settings` (never fails: missing file → defaults; unreadable/corrupt → defaults + `tracing::warn!`; result passed through `clamped()`); `pub fn save_to(&self, path: &Path) -> crate::Result<()>` (creates parent dirs; writes `path.with_extension("toml.tmp")` then renames, so the file is replaced atomically); `pub fn default_path() -> Option<PathBuf>` (`<config_dir>/SS2RS/settings.toml` via `directories::ProjectDirs`).

- [ ] **Step 1: Write failing tests:** `roundtrip_preserves_values`; `missing_file_gives_defaults`; `corrupt_file_gives_defaults` (`"not toml {{{"`); `empty_file_gives_defaults`; `partial_file_fills_missing_fields` (only `vsync = false`); `unknown_fields_are_ignored`; `out_of_range_volume_is_clamped` (`master_volume = 7.0` → 1.0, `-1.0` → 0.0); `nan_volume_falls_back_to_default` (`music_volume = nan` → 0.6); `save_creates_missing_parent_dirs`; `save_leaves_no_tmp_file` (after save, only `settings.toml` exists); `save_to_unwritable_path_returns_error_not_panic` (path under a file instead of a dir).
- [ ] **Step 2: Run** `cargo test -p ss2-core settings` → Expected: FAIL.
- [ ] **Step 3: Implement** as specified (clamp after deserialize; never `unwrap` file contents).
- [ ] **Step 4: Run** `cargo test --workspace`, `cargo deny check`, clippy, fmt → Expected: PASS (deny: new deps must be permissive-licensed).
- [ ] **Step 5: Commit** `T03-030: settings load/save (TOML, atomic, clamped)`.

---

### Task 5: Navigation input mapping (T03-031)

**Files:**
- Create: `crates/ss2-platform/src/nav.rs`
- Modify: `crates/ss2-platform/src/lib.rs`, `crates/ss2-platform/Cargo.toml` (deps `ss2-core`, `winit = "0.30"`, `gilrs = "0.11"`)
- Test: unit tests in `nav.rs`

**Interfaces:**
- Consumes: `ss2_core::input::NavInput`.
- Produces: `pub fn map_key(key: winit::keyboard::KeyCode) -> Option<NavInput>` (ArrowUp/W→Up, ArrowDown/S→Down, ArrowLeft/A→Left, ArrowRight/D→Right, Enter/NumpadEnter/Space→Activate, Escape/Backspace→Back); `pub fn map_gamepad_button(b: gilrs::Button) -> Option<NavInput>` (DPad directions; South→Activate; East→Back; Start→Back); `pub struct StickNavigator`; `StickNavigator::new() -> Self`; `pub fn update(&mut self, x: f32, y: f32, now_ms: u64) -> Option<NavInput>` where `y > 0` is **up** (gilrs convention). Dead zone `0.5` on the dominant axis; first press fires immediately, first repeat after `350` ms, then every `120` ms; returning inside the dead zone resets so the next push fires immediately; the dominant axis wins on diagonals.

- [ ] **Step 1: Write failing tests:** `keyboard_mapping_table` (all keys above; `KeyCode::KeyQ` → `None`); `gamepad_buttons`; `stick_under_deadzone_is_ignored` (0.49, 0.3 drift → `None` for 100 updates); `stick_push_fires_once_immediately`; `stick_hold_repeats_after_350_then_120_ms` (times 0, 100, 349 → only the first fires; 350 fires; 400 → none; 470 fires); `stick_release_and_repush_fires_immediately`; `stick_diagonal_picks_dominant_axis` (x 0.6, y −0.9 → Down); `stick_direction_change_fires_immediately`.
- [ ] **Step 2: Run** `cargo test -p ss2-platform` → Expected: FAIL.
- [ ] **Step 3: Implement** (pure functions/state machine; no window or device access).
- [ ] **Step 4: Run** tests, clippy, fmt, `cargo deny check` → Expected: PASS.
- [ ] **Step 5: Commit** `T03-031: keyboard/gamepad navigation input mapping`.

---

### Task 6: Renderer — wgpu context, quad batch, glyphon text (T04-035)

**Files:**
- Create: `crates/ss2-render/src/{lib.rs,batch.rs,context.rs,text.rs,shader.wgsl}`, `crates/ss2-render/assets/Inter-Regular.ttf`, `crates/ss2-render/assets/OFL.txt`
- Modify: `crates/ss2-render/Cargo.toml` (deps `ss2-ui`, `wgpu = "30"`, `glyphon = "0.12"`, `bytemuck` with `derive`, `pollster = "1"`, `thiserror`, `tracing`), `NOTICE` (add Inter/OFL attribution)
- Test: unit tests in `batch.rs` (CPU only); GPU construction exercised in Task 7

**Interfaces:**
- Consumes: `ss2_ui::{DrawList, Canvas, Color, Quad, TextRun}`.
- Produces: `#[repr(C)] #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq)] pub struct Vertex { pub pos: [f32; 2], pub color: [f32; 4] }` (`pos` in **window pixels**); `pub fn quads_to_vertices(list: &DrawList, canvas: &Canvas) -> Vec<Vertex>` (6 vertices per quad, two triangles, canvas→pixel via `Canvas::to_screen`, quad sizes scaled by `canvas.scale`); `#[derive(Debug, thiserror::Error)] pub enum RenderError { #[error("no suitable GPU adapter: {0}")] NoAdapter(String), #[error("device error: {0}")] Device(String), #[error("surface error: {0}")] Surface(String) }`; `pub struct Renderer`; `Renderer::new_windowed(target: impl Into<wgpu::SurfaceTarget<'static>>, size: (u32, u32), vsync: bool) -> Result<Renderer, RenderError>`; `Renderer::new_offscreen(size: (u32, u32)) -> Result<Renderer, RenderError>` (tries a hardware adapter, then `force_fallback_adapter`); `pub fn resize(&mut self, width: u32, height: u32)` (ignores 0×0); `pub fn set_vsync(&mut self, vsync: bool)`; `pub fn render(&mut self, list: &DrawList, clear: Color) -> Result<(), RenderError>` (windowed; recovers from `Outdated/Lost` by reconfiguring, returns `Ok` when the window is 0×0 without drawing); `pub struct ImageRgba8 { pub width: u32, pub height: u32, pub pixels: Vec<u8> }`; `pub fn render_to_image(&mut self, list: &DrawList, clear: Color) -> Result<ImageRgba8, RenderError>` (alpha-premultiplied output is **not** used; opaque clear).
- Text: draw `list.texts` with `glyphon` using the embedded Inter font; font size = `TextRun.size * canvas.scale`; `Center` alignment centers on `pos.x`.

- [ ] **Step 1: Write failing CPU tests:** `one_quad_makes_six_vertices`; `vertices_apply_scale_and_offset` (canvas from 2560×1080: quad at (0,0,100,50) → first vertex x=320, last vertex covers width 100); `vertices_carry_color`; `empty_list_makes_no_vertices`; `zero_area_quad_is_still_six_vertices_without_nan` (no `NaN` in output).
- [ ] **Step 2: Run** `cargo test -p ss2-render` → Expected: FAIL.
- [ ] **Step 3: Implement** `batch.rs` first (tests go green), then the GPU modules (`context.rs`: instance/adapter/device/surface; `text.rs`: glyphon wiring; `shader.wgsl`: pass-through colored triangles with pixel→NDC conversion in the vertex stage via a viewport uniform). Download Inter Regular and its `OFL.txt` from the official Inter release; record the version in `OFL.txt`'s sibling comment in `NOTICE`.
- [ ] **Step 4: Run** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `cargo deny check` → Expected: PASS; also `cargo xtask guard` if Task 4 of the M0 plan has landed (the `.ttf` is not a forbidden extension).
- [ ] **Step 5: Commit** `T04-035: wgpu 2D renderer with quad batch and glyphon text`.

---

### Task 7: Offscreen screenshots and golden tests (T04-036)

**Files:**
- Create: `crates/ss2-render/tests/golden_menu.rs`, `testdata/golden/menu_main_1280x720.png`, `testdata/golden/menu_options_1280x720.png`; `crates/ss2-render/src/image_diff.rs`
- Modify: `crates/ss2-render/Cargo.toml` (dev-deps `ss2-ui`, `ss2-core`, `png = "0.18"`)
- Test: `golden_menu.rs`; unit tests in `image_diff.rs`

**Interfaces:**
- Consumes: `Renderer::new_offscreen`, `render_to_image`, `ss2_ui::Menu`.
- Produces: `pub fn mean_abs_diff(a: &ImageRgba8, b: &ImageRgba8) -> Option<f32>` (`None` if sizes differ; average absolute per-channel difference in 0–255 units); golden tolerance `mean_abs_diff <= 4.0`. Golden files are generated with `UPDATE_GOLDEN=1 cargo test -p ss2-render --test golden_menu`, must each be ≤ 256 KiB, and are reviewed by eye before commit.

- [ ] **Step 1: Write failing tests:** `mean_abs_diff_identical_is_zero`, `mean_abs_diff_detects_change` (one pixel changed by 255 on a 2×2 image → `255/16`), `mean_abs_diff_size_mismatch_is_none`; golden tests `main_menu_matches_golden`, `options_menu_matches_golden` (render `Menu::new(Settings::default())` at 1280×720 with clear `(0.06,0.06,0.08)`; skip with `SKIPPED (no GPU adapter): …` when `new_offscreen` returns `NoAdapter`); `focus_highlight_is_orange_at_first_button` (sample the pixel at the first button's center-left in the rendered image: red channel > 180, blue < 60); `moving_focus_moves_highlight` (after one `Down`, the same pixel is no longer orange, the second button's is).
- [ ] **Step 2: Run** `cargo test -p ss2-render` → Expected: FAIL (items undefined / no golden file).
- [ ] **Step 3: Implement** `image_diff.rs`; run once with `UPDATE_GOLDEN=1` to create both PNGs; open them (`Read` the PNG) and confirm they look like the layout of Task 3 (title, three buttons / options list, orange focus) before committing.
- [ ] **Step 4: Run** `cargo test -p ss2-render` (without `UPDATE_GOLDEN`), then the whole workspace + clippy + fmt → Expected: PASS.
- [ ] **Step 5: Commit** `T04-036: offscreen rendering, image diff and golden menu tests`.

---

### Task 8: `ss2-app` window loop and menu wiring (T03-032)

**Files:**
- Create: `crates/ss2-app/src/{cli.rs,state.rs,app.rs}`
- Modify: `crates/ss2-app/src/main.rs`, `crates/ss2-app/Cargo.toml` (deps `ss2-core`, `ss2-ui`, `ss2-render`, `ss2-platform`, `winit = "0.30"`, `gilrs = "0.11"`, `png = "0.18"`, `pollster = "1"`, `tracing`)
- Test: unit tests in `cli.rs` and `state.rs`

**Interfaces:**
- Consumes: everything above.
- Produces (`cli`): `#[derive(Debug, PartialEq)] pub enum Mode { Window, Screenshot { screen: ScreenId, out: PathBuf, size: (u32, u32) }, Smoke { frames: u32 } }`; `pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Mode, String>` for `ss2-app` (no args → `Window`), `ss2-app --screenshot <main|options|pause> <out.png> [--size WxH]` (default `1280x720`), `ss2-app --smoke-frames <N>`; unknown flags, bad sizes (`0x0`, `abc`) and missing values → `Err` with a readable message.
- Produces (`state`): `pub enum Scene { Menu, Playing }`; `pub struct AppState { pub scene: Scene, pub settings: Settings }`; `#[derive(Debug, Default, PartialEq)] pub struct Effects { pub exit: bool, pub set_vsync: Option<bool>, pub set_fullscreen: Option<bool>, pub save_settings: bool }`; `pub fn apply(&mut self, cmd: MenuCommand) -> Effects` (`StartGame`→`Playing`; `Resume`→`Playing`; `QuitToMenu`→`Menu`; `Quit`→`exit`; `SettingsChanged(s)` → stores `s`, sets `set_vsync`/`set_fullscreen` only for fields that changed, `save_settings = true`).
- Window behavior (decided): window 1280×720 titled "SS2RS", resizable; `Esc` while `Playing` opens the pause menu (`Menu::open_pause`, scene `Menu`); `Playing` draws only the clear color `(0.02, 0.03, 0.05)` (placeholder scene); mouse positions converted with `Canvas::to_canvas`; gamepads polled each frame through `gilrs` + `StickNavigator`; settings loaded from `Settings::default_path()` at start and saved on change; vsync/fullscreen (borderless) applied live; rendering is skipped while the window is 0×0 or occluded; a startup failure prints a readable error and exits with code 1.

- [ ] **Step 1: Write failing tests:** cli — `no_args_is_window`, `screenshot_main_with_default_size`, `screenshot_with_custom_size`, `screenshot_rejects_zero_size`, `screenshot_missing_path_is_error`, `smoke_frames_parses`, `unknown_flag_is_error`; state — `start_game_enters_playing`, `resume_enters_playing`, `quit_to_menu_returns_to_menu`, `quit_sets_exit`, `settings_changed_reports_only_changed_fields` (vsync flips → `set_vsync = Some(false)`, `set_fullscreen = None`, `save_settings = true`), `volume_only_change_has_no_vsync_or_fullscreen_effect`.
- [ ] **Step 2: Run** `cargo test -p ss2-app` → Expected: FAIL.
- [ ] **Step 3: Implement** `cli.rs`, `state.rs`, then `app.rs` (winit 0.30 `ApplicationHandler`), then `main.rs` dispatching `Mode`: `Screenshot` renders one frame offscreen to PNG and exits 0 (exit 1 with message when no adapter); `Smoke` opens the window, renders N frames, exits 0.
- [ ] **Step 4: Verify automatically:** `cargo test --workspace`, clippy, fmt, `cargo deny check` → Expected: PASS. Then `cargo run -p ss2-app -- --screenshot options .superpowers/out/options.png` and `Read` the PNG → Expected: options screen as specified; `cargo run -p ss2-app -- --smoke-frames 120` → Expected: exit code 0.
- [ ] **Step 5: Verify manually (hand to the user):** `cargo run -p ss2-app`; check keyboard (arrows/Enter/Esc), mouse hover/click, a gamepad if available, toggling Vsync/Fullscreen, volume sliders, minimizing and restoring the window, closing and re-opening (settings persisted). Record anything off as a finding; do not claim this step done without the user's confirmation.
- [ ] **Step 6: Commit** `T03-032: ss2-app window loop and menu wiring`; tick Task 0's IDs `[x]` and remove `[INPRG]` tags in the same last commit.

---

## Self-review notes

- **Coverage of the agreed scope:** window + input (T03-032, T03-031), 2D renderer + text (T04-035), UI framework + menu screens (T10-030…032), settings (T03-030), headless/visual verification (T04-036). The original-data menu skin stays as `T10-033`/`T10-004`, deliberately not started.
- **Type consistency:** `NavInput` (Task 1) is consumed by Tasks 2 and 5; `Settings` (Task 2 struct, Task 4 persistence) by Tasks 2, 4, 8; `DrawList`/`Canvas` (Task 1) by Tasks 3, 6, 7; `Menu::{handle, draw, item_rects}` (Tasks 2–3) by Tasks 7–8; `Renderer` API (Task 6) by Tasks 7–8.
- **Deliberate rulings:** `winit 0.30` (stable) instead of 0.31 beta; `ss2-render` takes `Into<SurfaceTarget>` to stay winit-free; `NavInput` lives in `ss2-core` so `ss2-ui` stays free of platform crates; the slice uses new narrow task IDs instead of ticking the broad originals (`T10-004`, `T04-017`, …) it only partially covers.
- **Not covered, on purpose:** audio playback (volumes are stored only), touch, original skin/fonts (`.fnt`), the full action-rebinding system (T03-005), 9-slice/render-to-texture (T04-017 remainder).
