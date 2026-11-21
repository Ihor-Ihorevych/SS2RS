# T10 — UI, HUD, Menus, Fonts, Localization (`ss2-ui`)

Menus come from `Content/SeriousSam2/Menus/*` (data) and `EngineGui.dll` (widget model, per
T01-036); HUD params from `HudParams.rsc`-style databases. **PC first: mouse, keyboard, gamepad.**
Touch/mobile layout is Phase 2; the framework just must not bake in "mouse always exists".

## Framework (Wave 0 — no game data needed)
- [ ] **T10-001** · UI framework decision + core: retained vs immediate mode; layout (anchors/flex), styling/skins, event routing, focus system; renders via T04-017 · `L` · `P0`
- [ ] **T10-002** · Text rendering: glyph atlas from `.fnt` or fallback TTF, shaping basics, Unicode ranges (Latin/Cyrillic/…), wrapping, colored runs · `L` · `P0` · Deps: T02-025
- [ ] **T10-005** · Navigation model: mouse hover, keyboard/gamepad focus graph, consistent back/confirm; input-device-agnostic focus so touch can be added later · `M` · `P0`
- [ ] **T10-017** · Resolution/scale policy: virtual canvas, DPI scaling, aspect ratios (16:9 → 21:9 ultrawide) · `M` · `P0`

## Menus & flows
- [ ] **T10-004** · Menu system recreating original flow: main menu, new/continue/load/save, options, credits; data-driven from original menu definitions where specified · `XL` → split per screen · `P1`
- [ ] **T10-012** · Settings screens: video (quality, resolution, FOV, vsync), audio buses, controls/rebinding (T03-005), gameplay, network · `L` · `P1`
- [ ] **T10-018** · Co-op lobby/session UI: host/join, direct IP/invite code/server browser, player list, ready state, character select · `L` · `P1` · Deps: T08-019
- [ ] **T10-009** · Loading screens with progress and tips, shader/cache warm-up status · `S` · `P1` · 🟢
- [ ] **T10-016** · Credits, legal/attribution screens (open-source licenses, "unofficial fan engine" notice) · `S` · `P2` · 🟢
- [ ] **T10-019** · First-run data-location wizard (desktop): find/select the SS2 install, validate version/patch level via `ss2-doctor` (T11-013), remember choice · `M` · `P0`
- [ ] **T10-020** · Pause menu, death/respawn screens, end-of-level summary · `M` · `P1`

## HUD & in-game UI
- [ ] **T10-003** · HUD elements: health, armor, ammo, weapon list, crosshair (per weapon), damage indicators, pickup messages, score/kill feed, boss bar, objectives · `L` · `P1`
- [ ] **T10-007** · Developer console UI (T03-015 backend), FPS/net/perf overlays · `M` · `P1` · 🟢
- [ ] **T10-008** · Netricsa UI (T09-011 data) · `M` · `P2`
- [ ] **T10-013** · Text effects (`.tfx`): in-game conversation/subtitle styles, typewriter effects · `M` · `P2`
- [ ] **T10-014** · Co-op overlays: scoreboard, chat, player names/health over heads, ping markers, voice indicators · `M` · `P1`
- [ ] **T10-021** · Cutscene overlays: letterbox, skip prompt, subtitles (T06-012) · `S` · `P2`

## Localization & accessibility
- [ ] **T10-011** · Localization pipeline: string tables from game data (languages per T01-041) + our own UI strings, runtime language switch · `L` · `P2`
- [ ] **T10-010** · Accessibility: text scale, colorblind-safe HUD options, subtitle size/background, reduced flashing, hold/toggle options · `M` · `P2`

## Quality
- [ ] **T10-015** · UI golden-image tests for each screen at 3 resolutions + 2 aspect ratios · `M` · `P1`
- [ ] **T10-023** · UI performance budget (draw calls; zero steady-state allocations per frame) · `S` · `P2`

## PHASE 2 — Mobile UI (not scheduled before M8)
- [ ] **T10-006** · Touch HUD (virtual controls from T03-009), responsive layout, customization screen, left-handed mode · `L` · `PHASE 2`
- [ ] **T10-024** · Android data-import wizard (SAF picker, progress, resume) — pairs with T03-007 · `M` · `PHASE 2`
- [ ] **T10-025** · Safe-area/notch handling, portrait warning, minimum touch-target sizes · `M` · `PHASE 2`
- [ ] **T10-022** · IME / on-screen keyboard for chat and names on Android · `M` · `PHASE 2`
