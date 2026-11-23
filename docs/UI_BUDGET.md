# UI frame budget

T10-023 sets what one frame of the UI may cost, and a test that keeps it there. It covers the **UI side** (building the draw list); what the renderer does with the list is not covered (see the end).

## The budget

| | limit | today (menu screens) |
|---|---|---|
| quads per frame (each is a draw item in the renderer) | at most **24** | main 4, options 12, about 1 |
| text runs per frame | at most **32** | main 5, options 12, about 9 |
| **allocations in a warm frame** | **0** | **0**, measured once with a counting allocator (200 warm frames of each screen); kept as a structural test, see below |

"Warm" means the draw list has already grown to the size of the largest screen (the first frame or two may allocate). The limits are generous on purpose: a HUD will need more than a menu, and this
is the number a new screen must justify, not a target to fill.

## How to stay inside it

- **Keep one `DrawList` and call `Menu::draw_into(&mut list)` every frame.** It clears the list (keeping its capacity) and refills it. `Menu::draw()` makes a new list and allocates (3 to 6 allocations per frame on
  today's screens, measured once with the counting allocator); use it in tests and one-off captures only. `ss2-app` keeps one list.
- **`TextRun::text` is a `Cow<'static, str>`**: a fixed label is `Cow::Borrowed` (`"Back".into()`) and costs nothing; text built at run time is owned and allocates. Prefer fixed text; the slider's "37%" comes from
  a table of 101 static strings instead of `format!`.
- **Lay out without a `Vec`**: `layout::item_rect_iter` yields the rectangles one by one (`item_rects` still returns a `Vec` for pointer hit tests, which run per input event, not per frame).

## How it is tested

**Measured once, not kept:** a global allocator that counts allocations was run over the three screens: **0 allocations in 200 warm frames of `draw_into`** on each (and after moving the focus), against 3 to 6 per frame through `draw()`.
It is **not committed**, because it is `unsafe` and the workspace denies `unsafe_code` (`docs/CODING_STANDARDS.md`, "Unsafe": one small module, `// SAFETY:` comments, and the approval of the crate owner and a second maintainer). A maintainer who approves it can have it
back from T10-023b; I did not want to merge an exception to the rule by myself.

**What is kept** (`crates/ss2-ui/tests/frame_budget.rs`, safe code): the quad and text budgets for each screen; in 200 warm frames of each screen and after stepping the focus through every item, **no text run owns a `String`** (fixed text is `Cow::Borrowed`) and **the draw list never
has to grow** (its capacities do not change); and a reused list equals a fresh one and keeps nothing of the previous frame. These are the two things that make this code allocate, so they stand in for the count; **what they cannot see** is an
allocation that is made and freed inside `draw_into` (a temporary `Vec`, say): the first version of the layout had one (a `Vec` of rectangles per frame) and only the counting allocator showed it. I broke the code to check the kept tests can fail:
an owned title, an owned About line, an owned button label and a list that is not cleared each fail them; a percentage table clamped at 99 is caught by a separate unit test.

## Not covered (follow-up T10-023b)

- **A committed allocation counter** (above), if a maintainer approves the `unsafe`.
- **The renderer**: `ss2-render` builds text buffers and vertex data from the list each frame; its allocations, the GPU draw calls it makes (a text run is not one draw call there) and the frame time are **not measured**.
- **The developer console overlay** (`ConsoleView::draw`) clones its lines into new `String`s every frame; it is not budgeted (T10-007b adopts it).
- **The HUD and other screens** do not exist; each must come with its own entry in the test.
- **Time**: there is no frame-time budget for the UI here (no benchmark of `draw_into`); only counts.
- **Text shaping and the glyph atlas** (T10-002) when they exist.
