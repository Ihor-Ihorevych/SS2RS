# UI frame budget

T10-023 sets what one frame of the UI may cost, and a test that keeps it there. It covers the **UI side** (building the draw list); what the renderer does with the list is not covered (see the end).

## The budget

| | limit | today (menu screens) |
|---|---|---|
| quads per frame (each is a draw item in the renderer) | at most **24** | main 4, options 12, about 1 |
| text runs per frame | at most **32** | main 5, options 12, about 9 |
| **allocations in a warm frame** | **0** | **0** (200 frames of each screen, and after moving the focus) |

"Warm" means the draw list has already grown to the size of the largest screen (the first frame or two may allocate). The limits are generous on purpose: a HUD will need more than a menu, and this
is the number a new screen must justify, not a target to fill.

## How to stay inside it

- **Keep one `DrawList` and call `Menu::draw_into(&mut list)` every frame.** It clears the list (keeping its capacity) and refills it. `Menu::draw()` makes a new list and allocates (3 to 6 allocations per frame on
  today's screens, measured by the test); use it in tests and one-off captures only. `ss2-app` keeps one list.
- **`TextRun::text` is a `Cow<'static, str>`**: a fixed label is `Cow::Borrowed` (`"Back".into()`) and costs nothing; text built at run time is owned and allocates. Prefer fixed text; the slider's "37%" comes from
  a table of 101 static strings instead of `format!`.
- **Lay out without a `Vec`**: `layout::item_rect_iter` yields the rectangles one by one (`item_rects` still returns a `Vec` for pointer hit tests, which run per input event, not per frame).

## How it is tested

`crates/ss2-ui/tests/frame_budget.rs` replaces the global allocator with one that counts allocations and runs the screens: it checks the quad and text budgets, that 200 warm frames of `draw_into` allocate **nothing** on each screen
(and after moving the focus), that the counter works (`draw()` allocates), and that a reused list equals a fresh one and does not keep the previous frame. It is **one test function on purpose**: the counter is process wide.
I broke the code to be sure it can fail: a formatted string per slider, a list that is not cleared, an owned title, and a `Vec` of rectangles per frame each fail it; a percentage table clamped at 99 is caught by a separate unit test.

**The counting allocator is `unsafe`**, and the workspace denies `unsafe_code` (`docs/CODING_STANDARDS.md`, "Unsafe"): it is isolated in one small module of that test file with `#[allow(unsafe_code)]` only there, every `unsafe` block says why it is sound,
and it needs the approval of the crate owner and a second maintainer (asked for in the pull request). There is no safe way to count allocations.

## Not covered (follow-up T10-023b)

- **The renderer**: `ss2-render` builds text buffers and vertex data from the list each frame; its allocations, the GPU draw calls it makes (a text run is not one draw call there) and the frame time are **not measured**.
- **The developer console overlay** (`ConsoleView::draw`) clones its lines into new `String`s every frame; it is not budgeted (T10-007b adopts it).
- **The HUD and other screens** do not exist; each must come with its own entry in the test.
- **Time**: there is no frame-time budget for the UI here (no benchmark of `draw_into`); only counts.
- **Text shaping and the glyph atlas** (T10-002) when they exist.
