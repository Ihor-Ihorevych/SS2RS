# RFC 0010: The in-game UI framework: our own immediate-style layer; `egui` only for tools

- **Number:** 0010 (`docs/rfcs/0010-ui-framework.md`)
- **Authors:** @Ihor-Ihorevych/agent
- **Status:** in review
- **Approvers:** owner of `ss2-ui` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T10-001 (this decision); informs T10-002 (text), T10-005 (navigation), T10-003/008/009/014/018/020 (screens), T10-012 (settings), T11-004/005 (developer tools)

## Summary

The **in-game UI stays our own small layer** in `ss2-ui`: each frame, code turns UI state into a **`DrawList`** (quads and text runs in canvas units, see `docs/UI_SCALING.md`, `docs/UI_BUDGET.md`), the renderer draws it, and the
things that must persist between frames (which screen, which item has focus, a slider's value) live in plain state structs: **immediate-style drawing, retained navigation state**. Layout is a few primitives (anchors and the safe
area, which exist; stacks and flex rows, which do not), skins are data, input comes in as **actions** (`UiInput`), and focus is an explicit graph that is the same for mouse, keyboard, gamepad and, later, touch.
**`egui` is for the developer tools** (T11-004 asset viewer, T11-005 world viewer), in their own binaries, and is never linked into the game.

This is a decision with evidence about what exists and what the alternatives cost, **not a measured comparison of two finished UIs**. Where I did not test something I say so.

## What a game UI needs here

1. **Looks like the original, from the original's data** (T10-004, T10-033): textured frames, bitmap fonts, animated backgrounds. A skin is art plus placement, not a stock widget theme.
2. **Gamepad first** (the roadmap has Steam Deck and handheld layouts, T03-011) and mouse, keyboard and later touch, through **one focus model** (`NavInput`: up, down, left, right, activate, back).
3. **Any resolution and aspect ratio** with a fixed design canvas (`UI_SCALING.md`) and a text size floor.
4. **A frame budget**: no allocation in a warm frame, bounded draw calls (`UI_BUDGET.md`).
5. **Golden-image tests** of every screen (T10-015) and **localisation** (T10-011) and **accessibility** (T10-010: text scale, colour-safe options).
6. **No dependency that fights the renderer or the licence policy** (`deny.toml`: permissive licences only).

## What exists (the facts the decision rests on)

`ss2-ui` is about 3,100 lines with tests: `Canvas` (design canvas to pixels), `Menu` with screens, items, focus, sliders and toggles, `UiInput` and pointer hit testing, `DrawList` (quads and `Cow` text runs), the placeholder skin,
the scale policy with anchors and safe area, the developer console view, and golden-image tests of three screens (`ss2-render/tests/golden_menu.rs`). The renderer draws a `DrawList` with `wgpu` and `glyphon` (`ss2-render`). A menu
frame costs 4 to 12 quads and 5 to 12 text runs, and **0 allocations** once warm (measured once; `UI_BUDGET.md`). So the "own" option is not hypothetical: the layer is there, it is small, it is tested, and screens are built on it.

## Options

### A. Our own layer (chosen)

- **For:** draws exactly what the skin says; the focus model is ours, so gamepad navigation, "back" and the wrap-around rules are what the original's menus need and are not fitted onto another library's idea of focus; the frame budget is under our control
  (it is tested: zero allocations); the renderer already takes a `DrawList`; no new dependency; golden tests are straightforward because output is a plain list; it can run in the server-less headless tests.
- **Against:** **we write the widgets** (button, toggle, slider, list, text, tab, text input) and **text layout** (wrapping, coloured runs, T10-002), and every accessibility feature (screen-reader support above all) is ours to build. A small widget set is manageable;
  a rich one (a settings screen with 50 controls, a server browser with a sortable list, text editing with IME on Android) is a lot of work and will be tempting to under-build.

### B. `egui` in the game

- **For:** a large widget set, text editing and layout out of the box, an immediate-mode API that suits quick screens.
- **Against, measured or checked:** `egui-wgpu` 0.33.3 depends on **`wgpu` 27**; the workspace is on **`wgpu` 30**, so using it means a second `wgpu` in the build (or waiting for a matching release): the renderer's version is a project-wide decision and a UI library should not force it.
  The `egui` core is 37 crates in its tree; with the winit and wgpu integrations the tree is 217 lines of `cargo tree` before removing what the workspace already has (I did not compute the difference).
  **Not measured:** its per-frame allocations (it rebuilds a mesh each frame, and I would expect it to allocate; I did not test), and how far its keyboard focus can be bent to a controller-first menu with the original's rules. Its default look is a tool look; matching
  the original's skin means custom painting for everything, which is most of the work of option A anyway.

### C. A retained widget tree (our own, or a library such as Bevy UI, Slint or iced)

- **For:** a tree gives layout, focus and hit testing for free and suits animation and complex nesting.
- **Against:** none of the libraries was evaluated; they bring their own renderer or runtime (Bevy UI needs Bevy, which RFC 0005 rejected for the ECS; some GUI toolkits have licences that need checking against `deny.toml`). A retained tree of our own is more machinery than a menu slice and a HUD need today; the **retained part we do need is only the navigation state**, and that is a struct.
  Revisit only if the screens turn out to need something a per-frame function cannot express well (nested scrolling lists, drag and drop in an inventory).

## Decision

1. **`ss2-ui` is the in-game UI**, immediate-style: a frame function turns state into a `DrawList`; `ss2-render` draws it. State that persists is explicit: `Menu`/screen state, a focus index or graph, slider values.
2. **Layout primitives, in this order:** anchors and safe area (built: `UI_SCALING.md`), vertical and horizontal **stacks** with gaps and alignment, **flex** (grow and shrink) where a row must fill its space, and text measurement and wrapping from T10-002. No constraint solver.
3. **Skins are data**: a `Skin` maps the semantic parts of a widget (frame, focused frame, label colour, font) to quads and text; the placeholder skin exists, the original-data skin is T10-004 and T10-033.
4. **Input is actions** (`UiInput`, the platform API's actions): one focus model for every device; pointer hover moves focus; touch will map to the same events (T03-004, Phase 2).
5. **The frame budget is a rule**: new screens are added to `crates/ss2-ui/tests/frame_budget.rs`.
6. **`egui` is for the tools** (T11-004, T11-005): separate binaries, never a dependency of `ss2-game` or `ss2-app`, free to use their own `wgpu` version. Nothing in `ss2-ui` may depend on it.

## What this costs, and what would change the decision

- **The widget set is ours to build and to keep small.** A rule for the UI tasks: a screen that needs a widget we do not have asks for it in `ss2-ui`, with a test and a golden image, before the screen is built; it does not grow a private copy.
- **Accessibility** (screen readers, T10-010) is not solved by any option I looked at for a game with a custom renderer; the plan is text scale, colour-safe themes, subtitle options and full gamepad navigation, and nothing more is promised.
- **I would revisit this** if a screen turns out to need rich text editing or dense tables that cost more to write than a second `wgpu` or an integration costs to carry, or if `egui` ships a `wgpu` 30 integration and a measured controller-first navigation.

## What is not done

The decision does not build the core. Left for **T10-001b**: stack and flex layout, the `Skin` data model, a general focus graph (T10-005 may take it), event routing beyond the menu's own pointer code, and text wrapping and coloured runs (T10-002).
The original-data skin (T10-004, T10-033) and the screens from the original's flow are not part of this RFC.
