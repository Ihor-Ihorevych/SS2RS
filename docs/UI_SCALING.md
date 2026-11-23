# Resolution and scale policy of the UI

`ss2_ui::scale` (T10-017) says how the 1920 by 1080 design canvas meets a window of any size, any aspect ratio and any display scaling. Every screen is laid
out in **canvas units**; this module decides what a unit is in pixels and where the canvas lies.

## The rule

**A 1920 by 1080 reference area is always fully visible.** What happens to the rest of the window is a choice (`ScaleSettings::aspect`):

| Mode | Canvas | Use |
|---|---|---|
| `Letterbox` | exactly 1920 by 1080, bars around it (what `Canvas::fit` always did) | the old behaviour; the menu screens as they are today |
| `Expand` (default) | grows to cover the whole window: wider on an ultrawide, taller on a 4:3 or portrait window; **no bars** | a background that fills the screen, a HUD that anchors to the edges or sits in the safe area |

`ScreenMetrics::new(width_px, height_px, dpi_scale, &settings)` gives:

- `canvas`: a `Canvas` (scale and offset), the same type the renderer (`quads_to_vertices`) and the pointer code already use, so they need no change;
- `size`: the canvas size in units (1920 by 1080 letterboxed; for example 2580 by 1080 expanded on a 3440 by 1440 window; 1920 by 1440 on 1024 by 768);
- `safe_area`: the region a HUD should stay in: the whole canvas up to **21:9**, centred beyond that (on 32:9 the HUD does not run to the far edges of the screen);
- `reference_area()`: the 1920 by 1080 rectangle, centred in the canvas;
- `text_px(size_units)`: the physical pixel size for a text size, **never below 12 px** times the operating system's scale factor (a 20-unit label in a 640 by 360 window would be 7 pixels).

## Scale and DPI

- The scale is `min(width / 1920, height / 1080)` from the **physical** size of the window, so a 4K window at an operating-system scale of 150 % simply has a scale of 2.0 and
  crisp text; the factor `dpi_scale` is not multiplied in again. It is used only for the smallest-text floor, so that text stays as legible as on a 100 % screen.
- **The user can make the UI smaller** (`user_scale` from 0.5 to 1.0): the canvas then shows more units. Larger values are limited to 1.0, because a bigger UI would cut the
  reference area off; **a "bigger UI" option therefore needs a different design** (scrolling or reflowing screens) and is not offered. NaN and infinities read as 1.0.
- A window with no area (minimised) gives `None`; NaN, zero or negative `dpi_scale` reads as 1.0.

## Anchoring

`anchor(Anchor::BottomLeft, area, size, margin)` places a box in an area: seven anchors (the four corners, top and bottom centre, the middle). A box bigger than the area is
pinned to the area's top left rather than put at negative offsets. A **HUD element anchored in `safe_area`** moves inward on an ultrawide (a health bar stays near the
middle of the picture); anchored in the whole canvas (`Rect { 0, 0, size }`) it stays on the screen's edge, so choose per element.

## How it was tested

Thirteen tests, written first and seen to fail on the missing API: `Letterbox` equals `Canvas::fit` for five window shapes; `Expand` agrees at 16:9; an ultrawide gets a
wider canvas, no bars, a 21:9 centred safe area, and on 32:9 the safe area stays 21:9; a 4:3 and a portrait window grow downwards and keep the reference area inside; the
**reference area is fully visible for eleven awkward windows in both modes** (including 100 by 2000 and 2000 by 100); `Expand` covers the whole window; the user scale
(smaller shows more canvas, silly values limited, never bigger than 1); the text floor with and without a high-DPI factor; zero-sized windows and a bad DPI; the anchors; a HUD in the safe area against one on
the edge; the pointer mapping back to units. Eleven deliberate breakages each fail a test (the scale taken from the larger side, a user scale that may grow the UI or reach
zero, no safe-area limit, a bad DPI accepted, the DPI ignored in the text floor, an oversized box not pinned, a wrong anchor, an uncentred reference area, an offset in `Expand`);
three of my mutation scripts matched nothing the first time because rustfmt had reflowed the code, and I re-ran them against the formatted text.

## Not covered (follow-up T10-017b)

- **Nothing uses it yet.** The menu screens and the app still call `Canvas::fit`; moving them onto `ScreenMetrics` (and drawing a full-window background in `Expand` mode) is the follow-up. I have not looked at an ultrawide screenshot.
- **Integer scale snapping** for the placeholder skin's pixel-exact art (an open design question for the original-data skin, T10-004), **safe areas for consoles and TVs**, and **a "bigger UI"** (needs reflowing layouts, see above).
- **Android** insets and notches (Phase 2, T03-021).
- The **settings screen** with the aspect mode and the UI scale (T10-012) and the text size of accessibility (T10-010).
