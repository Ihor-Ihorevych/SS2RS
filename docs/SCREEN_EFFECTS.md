# Screen-space effects

`ss2_render_api::screen_effects` (T04-031) is the state behind the **damage and pickup flashes, screen fades, the underwater overlay and the cutscene letterbox**. It is device-free: the game drives a `ScreenEffects` with events and with time, and each frame asks it what to draw. It draws nothing itself.

```rust
let mut fx = ScreenEffects::new();
fx.flash(FlashKind::Damage, [1.0, 0.0, 0.0], 0.5, 0.4);   // red, at most half strength, 0.4 s
fx.update(dt);                                            // seconds, from the caller
let frame = fx.frame(width, height);                      // quads to draw, bar height, distortion
```

**Time comes from the caller** (`update(dt)`): no clock inside, so the same calls always give the same frames (tested over 60 frames) and a paused game stops its flashes. A `dt` that is negative, zero or not finite is ignored.

Colours are **linear RGBA, not premultiplied**: a quad of colour `c` and alpha `a` replaces a share `a` of the picture with `c`. Inputs are clamped (alpha to 0 to 1, colour channels to 0 to 1, NaN to 0).

## What each effect does

- **Flashes** (`flash(kind, color, peak, duration)`): start at alpha `peak` and ease out as `peak x (1 - age / duration)^2`, gone at the end. Kinds are `Damage`, `Pickup`, `Heal` and `Custom(u8)`. **Re-triggering a kind restarts it from the stronger of its current alpha and the new peak**, so a burst of weak hits neither stacks into a solid screen nor weakens a strong flash. Different kinds are drawn together in the order of the enum, whatever order they were triggered in. A zero peak or a non-positive duration does nothing.
- **Fades** (`fade_to(color, alpha, seconds)`): a smoothstep from where the screen is now to `alpha` of `color`: gentle at both ends, and **reversing in mid-fade does not jump**. A completed fade-out holds (a black screen stays black until faded back in); a duration of zero jumps at once. The fade is drawn on top of everything else.
- **Underwater** (`set_underwater(bool)`): blends in and out over 0.35 s (turning around mid-blend is smooth); gives a blue-green tint quad at up to 35 % alpha and an `underwater_distortion` in 0 to 1 for the renderer to warp the picture by.
- **Letterbox** (`set_letterbox(Some(aspect), seconds)` / `None`): bars slide in so the picture has the requested aspect on a taller screen (`bar = (height - width / aspect) / 2`, times the slide amount) and back out. A screen already as wide as the aspect, or wider, gets **no** bars; it would need pillarboxing, which this does not do. Bad sizes or aspects give no bars.
- `is_active()` says whether anything is visible or about to be, so the renderer can skip its pass.

`frame(width, height)` returns the quads **back to front** (underwater tint, flashes, fade), the height of **each** of the two bars in pixels, and the distortion amount.

## How it was tested

Sixteen unit tests and a doctest: the flash curve at its start, middle and end and that it never gets stronger, re-triggering, kind order, useless and out-of-range flash inputs, the fade's smoothstep at three points, holding and reversing, jumps, clamping, the underwater blend both ways and mid-blend, the letterbox maths on a 1920 x 1080 screen (the picture really is 2:1), sliding in and out, screens too wide or too small, everything at once and robust to bad time steps, and determinism. **Twenty-six deliberate breakages: twenty-two failed a test at once and four survived**: one real gap (the fade alpha was not clamped; now tested) and three equivalent ones (`>=` against `>` at the exact end of a fade, where the formula reaches the same value; a guard against a zero-alpha flash quad that the removal of finished flashes already makes unreachable; and the clearing of the remembered aspect after the bars left, which changed nothing visible, so I **removed that code**).

## Not covered (follow-up T04-031b)

- **Nothing draws it**: no full-screen quad pass, no distortion shader (the underwater warp needs a post pass and the HDR target, T04-012), no letterbox bars drawn, and the game does not trigger any of it (damage events, the water volume test, cutscenes).
- **The numbers are mine**: the flash curve, the 0.35 s underwater blend, the tint and its 35 % alpha, and the 0.6 s letterbox slide are reasonable defaults, not measured against the original game or tuned on screen.
- **No pillarbox**, no fades to a texture or a per-vignette (low health) effect, and no stacking of several fades.
