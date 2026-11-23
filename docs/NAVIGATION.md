# Navigation model: focus for mouse, keyboard, gamepad and touch

`ss2_ui::focus` (T10-005) decides **which control has focus and how every device moves it**, so a screen is written once. It is the focus model chosen in RFC 0010 (`docs/rfcs/0010-ui-framework.md`): immediate-style drawing, retained navigation state.

## How it works

A screen describes its controls as a `FocusGraph`: `FocusNode { id, rect, enabled }` (ids chosen by the screen, rectangles in canvas units) plus optional **explicit links** (`graph.link(from, Dir::Right, to)`). A `Focus` holds the current node and turns a
`FocusInput` into a `FocusEvent`:

| Input | What happens |
|---|---|
| `Nav(Up / Down / Left / Right)` | nothing focused yet: the first control (top row, then leftmost). Otherwise the **neighbour** in that direction (below); `Moved { from, to }`, or `None` if blocked |
| `Nav(Activate)` | `Activate(id)` of the focused control (nothing if none, or if it was disabled) |
| `Nav(Back)` | `Back` (the screen decides: close, go up a level) |
| `PointerMoved(p)` | hovering an **enabled** control focuses it (leaving keeps the focus where it was) |
| `PointerPressed(p)` | focuses the control under the pointer and remembers it |
| `PointerReleased(p)` | `Activate` only if released over **the same** control that was pressed; otherwise nothing |

A touch screen maps to the pointer inputs, a gamepad and keyboard to `NavInput` (the type the platform actions already produce): nothing else knows about devices.

**The neighbour in a direction:** an explicit link to an **enabled** node wins (a link to a missing, disabled or its own node is ignored and the layout decides). Otherwise the enabled node whose centre is ahead (strictly, along the axis)
with the lowest score, **along + 2 x across** (distance along the axis plus twice the offset to the side, between centres), so a node straight ahead beats a nearer one far off to the side; equal scores go to the **lower id**, so the answer never depends on the order
nodes were added. With `Wrap::Around`, nothing ahead means continuing from the opposite side: the node behind with the smallest offset across (the same row or column), then the farthest behind.

Housekeeping: `refocus(&graph)` after a screen is rebuilt (the focus moves to the first enabled control if its node is gone or disabled); `set(&graph, id)` to open a screen on a default button; `graph.clear()` keeps the allocation so a frame can rebuild the graph.
A stale focus is also dropped by `handle` itself, so a control that disappears or is disabled can never be activated.

## How it was tested

Twenty-four unit tests (written with the code) and **four property tests** (`crates/ss2-ui/tests/focus_props.rs`, 256 cases each by default; I also ran 20,000 each without a failure): over random layouts with random disabled nodes, a move leads only to an existing,
enabled node other than the start; over random input sequences (including the screen changing under the focus) the focus always rests on an enabled node and an `Activate` names an enabled one; on a regular grid the arrows walk rows and columns, `cols` steps with wrapping return to the start, and
down-then-up and right-then-left undo each other; and insertion order changes nothing. **Fifteen deliberate breakages each fail a test**; two survived my first tests (the weight of the sideways offset, and the order of the wrap key) because the layouts I had chosen did not
tell them apart, so I changed the layouts and re-ran them. The property tests do not catch those two: they pass with either, because the invariants do not say which neighbour is best.

## Not covered (follow-up T10-005b)

- **`Menu` does not use it yet.** It still has its own list focus and its own pointer code (`crates/ss2-ui/src/pointer.rs`), which agree with these rules for a vertical list but are separate code. Moving the menu screens onto `FocusGraph` is the follow-up; nothing else uses it today.
- **Scrolling lists, tab groups, modal dialogs that trap focus, text fields** (and the IME on Android, T10-022): not modelled; a modal is a different graph today.
- **Repeat and acceleration** of held directions, and the stick-to-direction conversion (`StickNavigator` in `ss2-app` does it for the menu).
- **Sliders and other controls that use left and right themselves** (`Left` on a focused slider should change the value, not move focus): the screen decides before it calls `handle`; the model has no notion of a control that consumes a direction.
- **Touch** beyond a pointer (multi-touch, gestures, per-finger ownership, T03-004, Phase 2).
- **Accessibility** (screen readers): not provided by any part of this.
