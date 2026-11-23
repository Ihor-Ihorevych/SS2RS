# The 2D / UI renderer

`ss2_render` (T04-017) draws the 2D layer of the game: flat quads, text, **images**, **9-slice images**, **clipping** (scissor) and **render-to-texture**. Quads and text came with T04-035; this task added the rest, on one pipeline with a
white texel for flat quads. The logic that needs no GPU (cutting a 9-slice, the clip stack, batching) is plain functions with unit tests; the GPU part is tested with pixel assertions on pictures made in code.

**What does not exist yet:** `ss2-ui` (its `DrawList`, `Quad`, `TextRun`) was **not changed** (a cross-crate API change needs an RFC), so no screen produces these commands today. A [`UiFrame`] wraps a `DrawList`
(`UiFrame::from(&list)`), and the renderer takes either, so the existing menus draw exactly as before. How `ss2-ui` adopts the commands is in "Adopting it from `ss2-ui`" below (follow-up T04-017b).

## The commands

A `UiFrame` is a list of `UiCommand`s drawn **in order**, back to front, in canvas units (`Canvas::fit`: 1920x1080 mapped to the window, letterboxed):

| Command | Builder | What it does |
|---|---|---|
| `Quad` | `quad(rect, color)` | a filled rectangle (the same `ss2_ui::Quad`) |
| `Text` | `text(pos, text, size, color, align)` | one line of text (the same `ss2_ui::TextRun`) |
| `Image` | `image(handle, rect)`, `image_ex(handle, rect, uv, tint, filter)` | an image, or part of it (`UvRect`), multiplied by a tint, sampled `Nearest` or `Linear` |
| `NineSlice` | `nine_slice(handle, rect, insets, scale)` or `push(UiCommand::NineSlice(..))` for tint, filter and `Fill::Tile` | a bordered image |
| `PushClip` / `PopClip` | `push_clip(rect)`, `pop_clip()` | clip what follows to a rectangle |

A frame is reused like a `DrawList`: `clear()` keeps the capacity. `UiFrame::draw_call_count(canvas, target, size_of)` says how many GPU draw calls a frame takes (see Batching).

```rust
let panel = renderer.create_image(&panel_pixels)?;            // keeps a CPU copy
let mut frame = UiFrame::new();
frame.nine_slice(panel, rect, Insets::uniform(12.0), 1.0)
     .push_clip(list_area)
     .quad(row, color).text(pos, "Row", 34.0, white, TextAlign::Left)
     .pop_clip();
renderer.render_frame(&frame, clear)?;                        // a window; or render_frame_to_image (offscreen)
```

`Renderer::render`, `render_to_image` and `capture_frame` still take a `DrawList`; `render_frame`, `render_frame_to_image` and `capture_ui_frame` are their `UiFrame` twins.

## 9-slice (`nine_slice`)

`nine_slice(dest, insets, texture_size, scale, fill) -> Vec<Cell>` (a `Cell` is a destination `Rect` and a `UvRect`), row by row from the top left. Insets are in **texels**.

- The **corners keep their size**: an inset of `n` texels is drawn `n * scale` units wide or high and a corner is never stretched.
- The **edges** keep the border's thickness and fill between the corners; the **centre** fills the rest. `Fill::Stretch`: one cell each (nine in all). `Fill::Tile`: repeated at the size of their texels times `scale`, and the **last repeat is cut off, not squeezed** (its texture rectangle is cut in proportion).
  Tiling that would take more than `MAX_CELLS` (4096) cells falls back to stretching.
- A destination **smaller than the borders** shrinks the borders on that axis by one factor (left and right together, top and bottom together, independently), so nothing is negative or overlaps; the centre then has no width (or height) and its cells are left out. Four corners remain when both axes are too small.
- Insets that are negative or not finite count as zero; insets larger than the texture are scaled down to fit it. Zero insets: one cell with the whole texture.
- A texture whose insets use up the whole texture on an axis has **no centre** there: the middle of the destination is left empty (a gap), not filled with a stretched edge texel.
- Empty result for a destination without area, a zero-size texture, or a `scale` that is not a positive finite number. No NaN or negative size is ever produced (a unit test sweeps destination sizes down to 1e-6).
- Cells are **not snapped to whole pixels**: with an integer canvas scale, integer positions and an integer `scale` the borders are whole pixels (the GPU test checks every corner pixel at five sizes); otherwise the rasteriser decides, and with `Linear`
  sampling the texel row next to a border bleeds half a texel across the cell edge, as in every 9-slice drawn with a bilinear sampler.

## Scissor (`ScissorStack`, `to_pixels`)

- The stack starts **unclipped**. `push(rect)` clips to the **intersection** of the rectangle with the current clip, `pop()` restores the one before. A rectangle without area (zero or negative size, NaN, infinity) or an intersection without area makes the clip **empty**:
  everything under it is skipped (not drawn, not counted as a draw call) and it stays empty under more pushes until popped. `pop()` on an empty stack returns `false` and changes nothing; in a frame an unbalanced `PopClip` is ignored (and logged); clips still open at the end are closed by the end of the frame.
- Clip rectangles are in canvas units and intersected there. `to_pixels(rect, canvas, target)` turns one into a pixel rectangle: **a pixel is inside when its centre is inside the rectangle** (a centre exactly on a left or top edge is inside, on a right or bottom edge it is not), i.e.
  `x0 = ceil(left - 0.5)`, `x1 = ceil(right - 0.5)`, clamped to the target. This is the rule the rasteriser uses for the quads, so **a clipped quad ends on exactly the pixel where the unclipped quad would have ended**; a rectangle that holds no pixel centre is empty. Because each edge is rounded on its own and rounding is monotone, clipping with an intersection gives the same pixels as intersecting the pixel rectangles.
- The clip is **not** limited to the canvas: in a letterboxed window it can reach into the bars.
- On the GPU it is `set_scissor_rect` before each batch. **Text** is clipped by the same rectangle twice: glyphon's `TextBounds` (glyphs outside are not made) and the scissor (pixels outside are not written).

## Batching (`Plan`)

The frame is turned into **batches**, one GPU draw call each (text counted as one call per batch):

- **Order is kept**: batches are drawn in command order; nothing moves across anything else (a test puts text before and after an opaque quad and checks which is visible).
- Consecutive commands **merge** when they have the same kind, image, sampling and scissor rectangle. A different tint or texture rectangle is vertex data and does not split. Flat quads have no image (a white texel is bound), so they merge with each other.
- Text and geometry never merge (different pipelines); consecutive text runs with the same scissor are one batch (one glyphon renderer each, sharing one glyph atlas).
- **Skipped commands** (empty clip, unknown image) draw nothing and do not split the batches around them.
- `draw_call_count` is the length of the batch list, so it is testable without a GPU: a `DrawList` is 2 calls (its quads, its text); a panel, two labels, a clipped list of three rows (quad, icon, text each) and a footer is `1 + 1 + 3 * 3 + 1 = 11`.

## Images and recovery

`create_image(&ImageRgba8) -> ImageHandle` uploads a picture and **keeps a CPU copy** (shared, not copied again); `create_render_target(w, h)` makes an empty picture to draw into; `destroy_image(handle)`; `image_size`, `image_count`, `read_image` (slow, for tests and tools).
Handles are numbers that are never reused and stay valid across a rebuilt device; they belong to the renderer that made them. Using one that was destroyed (or never made) is `RenderError::Image` **before anything is drawn**.

- Uploaded images have **straight alpha** and are `Rgba8Unorm` (`Rgba8UnormSrgb` when the target is an sRGB format, so a 1:1 draw shows the bytes in either case). Maximum size is the device's texture limit; a bad size or byte count is an error and changes nothing. No mipmaps.
- **Flat quads are unchanged** by the white texel they sample (1.0 times the colour is the colour; the old goldens still pass).
- **After `device.destroy()`** (or a real loss) the next frame rebuilds the device (`RenderStats::device_rebuilds`) and then makes every image again: uploaded images from the kept pixels, render targets from their **recipes** (below), so the same handles draw the same pictures. A test builds a scene with an image, 9-slice, scissored text, a target and a target drawn from a target, destroys the device twice, and compares the frames pixel for pixel.

## Render-to-texture

`render_to_target(target, &frame, clear)` draws a frame into a render target; the target is then an image like any other (`image`, `nine_slice`, tint, `Linear` or `Nearest`, a part of it).

- Coordinates in a target are **its own pixels** (origin top left; the 1920x1080 canvas is not applied), and the target is cleared first.
- A target holds **premultiplied alpha** and is drawn with a premultiplied pipeline (`fs_premultiplied`; tint is applied to the premultiplied colour), so a semi-transparent picture comes out as if drawn directly (a test overlaps two 50 % quads on a transparent target and compares with drawing them in place: within 2 levels; an opaque scene is **pixel-identical** to drawing in place, text included). `clear` is premultiplied before use. `read_image` returns the premultiplied pixels.
- **Nested use** works to any depth: render the inner target first, then draw it in the outer one. Re-rendering the inner one changes what the outer shows the next time the outer is rendered.
- **A picture cannot be drawn into itself.** `render_to_target` is an error (`RenderError::Image`, nothing drawn, nothing recorded) if the frame draws `target`, or draws a target that was rendered (however indirectly) from `target`: it is a check on the recipes (`would_loop`), so a loop `a -> c -> b -> a` is refused when the last edge is made.
- **Recipes and recovery.** The frame and clear colour of each target's last render are kept (a copy). After a rebuilt device every target is drawn again from its recipe, **each after the targets it was drawn from** (`replay_order`, a depth-first order that does not depend on handle numbers, and survives a loop or a missing image). Consequences:
  a target drawn from other targets shows *their latest* content after recovery, not the content they had when it was rendered (the same as what its next render would show); a target drawn from an image that was destroyed since is drawn again without that image (the command is skipped); recovery costs one render pass per target.
- Targets have the **format of the main target** (so the one pipeline serves them); `read_image` supports 8-bit RGBA and BGRA. On a float target the capture path makes a temporary renderer in RGBA8 and draws the images again there (tested with `Rgba16Float`); a BGRA target is tested too.

## Adopting it from `ss2-ui` (T04-017b)

`ss2-ui` keeps producing `DrawList`s. To use the new commands it needs an RFC for: a `DrawList` that can hold the new commands (or `DrawList` replaced by an equivalent of `UiFrame`, which has the same `clear()`-keeps-capacity behaviour for `docs/UI_BUDGET.md`), an image handle type
that `ss2-ui` can name without depending on `wgpu` (`ImageHandle` is a plain number and could move to `ss2-core` or `ss2-ui`), and a place that registers the images (the app, with the renderer). Until then the commands are built by whoever calls `ss2-render` (the golden tests, a later HUD).

## How it was tested

*(The numbers are filled in at the end of this section.)*

## Not covered (follow-up T04-017b and later)

- **`ss2-ui` does not produce these commands** (above): no menu uses an image, a 9-slice or a clip yet, and there is no skin.
- **No texture atlas** (every image is its own texture and its own batch break) and no automatic atlas packing; many small icons are many draw calls.
- **No rotated, skewed or masked drawing**, no rounded-corner or rounded clip (clips are axis-aligned rectangles), no per-corner colours, no blend modes other than straight alpha for images and premultiplied for targets, no additive.
- **No MSAA** and no mipmaps: shrinking an image a lot with `Linear` sampling aliases.
- **Text** is still one line per `TextRun`, laid out by glyphon when the frame is prepared (shaping is not cached across frames beyond glyphon's own atlas); a clip splits text batches.
- **Render targets**: no resize, no reading back as a `DrawList`-free capture other than `read_image`; they take the main target's format (no HDR or single-channel targets); after recovery a target is *replayed*, so a target whose recipe depended on state outside the frame and the images is not restorable to the content it had (it cannot be: only the frame and images exist).
- **Allocation**: `render_to_target` copies the frame (recipe) every render, and the planner allocates its vertex and batch lists each frame; the zero-allocation budget of `docs/UI_BUDGET.md` is about the UI side and is **not measured here**.
- **Windowed rendering was not run**: everything was tested on offscreen renderers (there is no window in the test environment); the windowed path shares `draw` and differs only in where the view comes from.
- **Linux, Android and Apple**: only this one Windows machine was used (Vulkan, DX12, GL and the DX12 software adapter); CI does not run (billing), so nothing was run on Linux.
