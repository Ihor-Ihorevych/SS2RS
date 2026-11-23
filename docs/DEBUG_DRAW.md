# Debug draw

`ss2_render_api::DebugDraw` (T04-018) is the recorder for shapes that help a developer see what the game is doing: colliders,
paths, culling volumes, network positions. It is GPU-free like the rest of `ss2-render-api`, so physics, gameplay and net code can
call it, and a headless server can compile it.

## Use

```rust
let mut debug = DebugDraw::new(10_000, 256);       // at most 10,000 lines and 256 labels at a time
debug.overlays_mut().set(DebugCategory::Physics, true);   // from the console: `r_debug physics`

debug.aabb(DebugCategory::Physics, bounds, DebugColor::GREEN);
debug.sphere(DebugCategory::Audio, Sphere::new(position, range), DebugColor::YELLOW);
debug.arrow(DebugCategory::Net, predicted, authoritative, DebugColor::RED);
debug.label(DebugCategory::Gameplay, spawn_point, "spawn 3", DebugColor::WHITE);
debug.line_for(DebugCategory::Physics, from, to, DebugColor::RED, 30);   // a raycast, kept for 30 ticks

// per tick: the renderer reads debug.lines() and debug.project_labels(view_projection, viewport); then
debug.advance_tick();
```

| Shape | Call | Lines |
|---|---|---|
| line | `line`, `line_for(.., ticks)` | 1 |
| box (axis-aligned, oriented) | `aabb`, `obb(.., matrix)` | 12 |
| circle | `circle(centre, normal, radius)` | 24 |
| sphere | `sphere` | 72 (three circles) |
| axes of a transform | `axes(matrix, length)` | 3 (x red, y green, z blue) |
| arrow | `arrow(from, to)` | 5 |
| text | `label(position, text)`, `project_labels(view_projection, viewport)` for the 2D renderer | |

## Rules

- **A category that is off records nothing** (the call returns at once), so debug calls can stay in the code. Everything is off by
  default; `DebugCategory::from_name` and `DebugOverlays::toggle` serve the console command. Categories: physics, navigation,
  culling, net, audio, gameplay, render, misc.
- **Bounded:** at most the configured number of lines and labels. A shape that does not fit is **not recorded in part**; it is counted
  in `dropped_lines()` / `dropped_labels()` (reset each tick) so an overlay can say "debug draw is full".
- **Lifetime:** a shape lasts one tick unless `line_for` asks for more; `advance_tick` ends the tick.
- **Bad input is dropped:** a NaN or infinite position, a negative radius or a zero normal records nothing instead of poisoning the
  vertex buffer. Labels are cut to 64 characters at a character boundary.
- `project_labels` places labels on a viewport of the given pixel size and leaves out those behind the camera or outside the view.

## Tests

Fifteen unit tests and a doctest: the category gate, console names (all round-trip), the edges, corners and radius of each shape, an
oriented box following its matrix, closed flat circles, zero-length arrows, non-finite input, lifetimes (including that a shape keeps
its **own** lifetime across ticks), the caps and the all-or-nothing rule, label cutting, label projection (centre, behind, outside,
y downwards). Deliberate breakages (a lost category gate, a lifetime vector that drifted from its shapes, an ignored cap, an
unflipped y, a byte-wise label cut) each fail a test; the lifetime one first survived and the alignment test was added for it.

## Not covered (follow-up T04-018b)

The **wgpu pass** that draws the lines (a line list with depth test on and off, in the debug overlay stage of the frame graph) and
the text labels through the 2D renderer; the `r_debug` console command; a frustum shape (`Frustum` keeps planes, not corners).
`RenderWorld` is unchanged (the recorder is separate, so the render contract of RFC 0002 did not change).
