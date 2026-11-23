# T10 — UI, HUD, Menus, Fonts, Localization (`ss2-ui`)

Menus come from `Content/SeriousSam2/Menus/*` (data) and `EngineGui.dll` (widget model, per
T01-036); HUD params from `HudParams.rsc`-style databases. **PC first: mouse, keyboard, gamepad.**
Touch/mobile layout is Phase 2; the framework just must not bake in "mouse always exists".

## Framework (Wave 0 — no game data needed)
- [x] **T10-001** · UI framework decision + core: retained vs immediate mode; layout (anchors/flex), styling/skins, event routing, focus system; renders via T04-017: the **decision** is `docs/rfcs/0010-ui-framework.md` (**in review**: our own immediate-style layer for the game, `egui` for tools only); the core that exists is the menu slice, anchors and scale (T10-017), the draw budget (T10-023); **stack/flex layout, a skin data model, a general focus graph and text wrapping are not built** (T10-001b) · `L` · `P0`
- [ ] **T10-001b** · UI core after RFC 0010: stack and flex layout, the `Skin` data model (placeholder now, original data T10-004/T10-033), a general focus graph shared by mouse, keyboard and gamepad (with T10-005), event routing beyond the menu's pointer code, a small widget set with tests and golden images, text wrapping and coloured runs (T10-002) · `L` · `P0` · Deps: T10-001
- [ ] **T10-002** · Text rendering: glyph atlas from `.fnt` or fallback TTF, shaping basics, Unicode ranges (Latin/Cyrillic/…), wrapping, colored runs · `L` · `P0` · Deps: T02-025
- [x] **T10-005** · Navigation model: mouse hover, keyboard/gamepad focus graph, consistent back/confirm; input-device-agnostic focus so touch can be added later: `ss2_ui::focus` (`docs/NAVIGATION.md`): a focus graph with spatial neighbours and explicit links, wrapping, hover-focus and press-release activation, the same events for every device; property-tested; **`Menu` does not use it yet, and scrolling lists, modals, text fields and controls that consume a direction are not modelled** (T10-005b) · `M` · `P0`
- [ ] **T10-005b** · Use the focus model: move `Menu` and its pointer code onto `FocusGraph`, controls that consume a direction (sliders), modal focus traps, scrolling lists and tab groups, held-direction repeat, text fields with the IME (T10-022) · `M` · `P0` · Deps: T10-005
- [x] **T10-017** · Resolution/scale policy: virtual canvas, DPI scaling, aspect ratios (16:9 → 21:9 ultrawide): `ss2_ui::scale` (`docs/UI_SCALING.md`): letterbox or expand, a safe area, the user's smaller-UI scale, a text floor with the OS scale factor, anchors; **nothing uses it yet** (T10-017b) · `M` · `P0`
- [ ] **T10-017b** · Adopt the scale policy: move the menu screens and the app from `Canvas::fit` to `ScreenMetrics`, draw a full-window background in `Expand` mode, anchor the HUD in `safe_area`, golden screenshots at 16:9, 21:9, 4:3 and portrait, the aspect and UI-scale settings (T10-012) · `S` · `P0` · Deps: T10-017

## Menus & flows
- [ ] **T10-004** · Menu system recreating original flow: main menu, new/continue/load/save, options, credits; data-driven from original menu definitions where specified · `XL` → split per screen · `P1`
- [ ] **T10-012** · Settings screens: video (quality, resolution, FOV, vsync), audio buses, controls/rebinding (T03-005), gameplay, network · `L` · `P1`
- [ ] **T10-018** · Co-op lobby/session UI: host/join, direct IP/invite code/server browser, player list, ready state, character select · `L` · `P1` · Deps: T08-019
- [ ] **T10-009** · Loading screens with progress and tips, shader/cache warm-up status · `S` · `P1` · 🟢
- [x] **T10-016** · Credits, legal/attribution screens: the **About** screen (unofficial-fan-engine notice, licence, bundled-font credit, pointer to `NOTICE`); a per-dependency licence list is not shown (it is in `NOTICE` and `cargo deny` output) · `S` · `P2` · 🟢
- [ ] **T10-019** · First-run data-location wizard (desktop): find/select the SS2 install, validate version/patch level via `ss2-doctor` (T11-013), remember choice · `M` · `P0`
- [ ] **T10-020** · Pause menu, death/respawn screens, end-of-level summary · `M` · `P1`

## HUD & in-game UI
- [ ] **T10-003** · HUD elements: health, armor, ammo, weapon list, crosshair (per weapon), damage indicators, pickup messages, score/kill feed, boss bar, objectives · `L` · `P1`
- [x] **T10-007** · Developer console UI (T03-015 backend), FPS/net/perf overlays: `ss2_ui::devconsole` (`docs/DEV_CONSOLE_UI.md`): the console view model with editing, history, completion, scrollback and drawing, frame statistics and a corner overlay; **the app does not call it yet and it has not been seen on screen** (T10-007b) · `M` · `P1` · 🟢
- [ ] **T10-007b** · Wire the developer console and overlays into the app: map key and text events to `ConsoleKey` and `handle_text`, the console toggle key, draw order, feed `FrameStats`, cvars `r_show_fps` and `net_graph`, a golden screenshot of the console, the network and GPU timing overlays (T08-018b, T04-019) · `S` · `P1` · Deps: T10-007, T03-033
- [ ] **T10-008** · Netricsa UI (T09-011 data) · `M` · `P2`
- [ ] **T10-013** · Text effects (`.tfx`): in-game conversation/subtitle styles, typewriter effects · `M` · `P2`
- [ ] **T10-014** · Co-op overlays: scoreboard, chat, player names/health over heads, ping markers, voice indicators · `M` · `P1`
- [ ] **T10-021** · Cutscene overlays: letterbox, skip prompt, subtitles (T06-012) · `S` · `P2`

## Localization & accessibility
- [ ] **T10-011** · Localization pipeline: string tables from game data (languages per T01-041) + our own UI strings, runtime language switch · `L` · `P2`
- [~] **T10-010** [INPRG @Ihor-Ihorevych/agent · branch `T10-010-accessibility-settings` · 2026-11-23] · Accessibility: text scale, colorblind-safe HUD options, subtitle size/background, reduced flashing, hold/toggle options · `M` · `P2`

## Quality
- [ ] **T10-015** · UI golden-image tests for each screen at 3 resolutions + 2 aspect ratios · `M` · `P1`
- [x] **T10-023** · UI performance budget (draw calls; zero steady-state allocations per frame): `docs/UI_BUDGET.md`: at most 24 quads and 32 text runs per frame, zero allocations in a warm frame (`Menu::draw_into`, `Cow` text, an allocation-free layout), measured once with a counting allocator and kept as a safe structural test (no owned text, no list growth); **the counting allocator itself needs `unsafe` approval, and the renderer side, the dev console overlay and frame time are not measured** (T10-023b) · `S` · `P2`
- [ ] **T10-023b** · UI budget follow-ups: a committed counting-allocator test if a maintainer approves the `unsafe`, allocations and GPU draw calls of the renderer per frame (text buffers, vertex data), the developer console overlay (T10-007b), the HUD and every new screen added to the frame budget test, a frame-time benchmark of `draw_into` · `S` · `P2` · Deps: T10-023

## PHASE 2 — Mobile UI (not scheduled before M8)
- [ ] **T10-006** · Touch HUD (virtual controls from T03-009), responsive layout, customization screen, left-handed mode · `L` · `PHASE 2`
- [ ] **T10-024** · Android data-import wizard (SAF picker, progress, resume) — pairs with T03-007 · `M` · `PHASE 2`
- [ ] **T10-025** · Safe-area/notch handling, portrait warning, minimum touch-target sizes · `M` · `PHASE 2`
- [ ] **T10-022** · IME / on-screen keyboard for chat and names on Android · `M` · `PHASE 2`

## Menu slice (PC, placeholder skin) — partial T10-001/T10-004/T10-005/T10-017 only; original-data skin remains T10-004/T10-033
- [x] **T10-030** · `NavInput`, UI geometry, `DrawList`, 1920×1080 virtual canvas fit (letterbox, 0×0-safe) · `M` · `P1`
- [x] **T10-031** · Menu model (Main/Options/Pause), focus navigation, settings toggles/sliders, command output · `M` · `P1` · Deps: T10-030
- [x] **T10-032** · Placeholder-skin layout, draw list and pointer input (hover/press/release rules) · `M` · `P1` · Deps: T10-031
- [ ] **T10-033** · Menu skin and flow from original `.rsc`/`.tex`/`.fnt` data, replacing the placeholder skin · `L` · `P1` · Deps: T02-006, T02-007, T02-012, T02-023, T02-025, T01-036
