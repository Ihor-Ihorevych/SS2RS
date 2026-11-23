# Accessibility options

This page describes the code (`ss2_ui::accessibility`, T10-010). What the game should offer, and the checklist with owners and status, is [the accessibility review](ACCESSIBILITY.md) (T12-012).

`ss2_ui::accessibility` (T10-010) holds the accessibility **settings** and the **decisions and maths** behind them. It draws nothing and reads no input device: a settings screen (T10-012) and a console set the options by name, and the HUD, subtitles, effects and input code ask the functions what to do.

## The settings

`AccessibilitySettings` (default: everything off, text scale 1.0) has ten named options, set and read as text with `set(name, value)` / `get(name)` (any case and surrounding spaces; an unknown name or a value outside what is allowed is **refused with a message saying what is allowed, and nothing changes**; `OPTION_NAMES` lists them in screen order):

| Option | Values | What it decides |
|---|---|---|
| `text_scale` | 0.8 to 2.0 | `scaled_text(px)`: the size of all text. Separate from the UI scale of `ss2_ui::scale`, which only shrinks the whole layout. |
| `colorblind` | off, protanopia, deuteranopia, tritanopia | which HUD palette to use |
| `subtitle_size` | small, medium, large, extra_large | 0.8, 1.0, 1.3, 1.7 times |
| `subtitle_background` | none, translucent, opaque | the alpha of the box: 0, 0.6, 1 |
| `reduce_flashing` | on/off | `flash_strength()` is 0.25 instead of 1, and `FlashLimiter` allows at most 3 flashes a second |
| `reduce_motion` | on/off | `motion_strength()` is 0 instead of 1 (screen shake, the underwater warp, camera bob) |
| `high_contrast` | on/off | text must reach a contrast of 7 (WCAG AAA) instead of 4.5, subtitle boxes are opaque |
| `crouch`, `sprint`, `aim` | hold, toggle | `HoldToggle` |

## Colour blindness

`simulate(mode, rgb)` shows a colour as a person with protanopia, deuteranopia or tritanopia sees it (the Machado, Oliveira and Fernandes 2009 matrices at full severity, in **linear** RGB). `perceived_difference(mode, a, b)` is how far apart two colours look *to that person*: the "redmean" distance of the simulated sRGB values, 0 for the same colour and about 765 for black and white.

The point is a HUD whose **good, caution and danger colours stay apart for everyone**. `StatusPalette::traditional()` (green, yellow, red) and `StatusPalette::safe()` (blue, yellow and vermilion from the Okabe-Ito palette) are checked against that measure:

| Smallest difference between two of the three colours | typical | protan | deutan | tritan |
|---|---|---|---|---|
| traditional | 291 | **71** | **111** | 384 |
| safe | 288 | 296 | 262 | 342 |

`MIN_DISTINGUISHABLE` is 200: the safe palette is above it for every kind of vision, the traditional one falls below it for protan and deutan players (green against yellow, green against red), which is the problem the option exists to fix. `StatusPalette::for_mode` gives the traditional palette for typical vision and the safe one otherwise.

## Contrast

`relative_luminance` and `contrast_ratio` follow the WCAG definitions (21 for black on white, 1 for the same colour). `ensure_contrast(fg, bg, min)` moves `fg` toward black or white **by the least amount** that reaches the ratio (it picks the side with more room: the crossover is a luminance of about 0.18, where black and white give the same contrast), returns it untouched if it already passes (also when it is exactly at the ratio), and returns the better of pure black and white if the ratio is impossible. `subtitle_style(preferred_text)` combines the size, the text scale, the box and a text colour lifted to the contrast the player needs.

## Hold or toggle

`HoldToggle` is the state of one action: in **hold** mode it is active while the button is down; in **toggle** mode every *press* (the edge, not the held button) switches it. `cancel()` switches it off without a press (the player stopped sprinting, a level loaded); changing the mode switches it off and **a button already held through a mode change is not a new press**.

## Flashing

`FlashLimiter::allow(now, limit)` says whether a flash may fire: with reduced flashing on, no more than three in any second (WCAG 2.3.1), counting only flashes that fired. The clock is the caller's, so it is deterministic.

## How it was tested

Twenty unit tests and two doctests: greys staying grey under every simulation, red and green collapsing for protan and deutan players and blue and yellow for a tritan one, the difference being symmetric and about 765 for black and white, the palette table above, the WCAG definitions at their corners, `ensure_contrast` in every direction including the crossover and the exact-ratio and impossible cases, the hold and toggle sequences edge by edge, the limiter over three seconds of tenth-second flashes, every option reading back through its own text and 14 bad values refused without a change, and the subtitle style in six combinations. **Thirty-one deliberate breakages.** Twenty-one failed a test at once; **three survived my first tests and were real gaps** (the toe of the sRGB curve, which side of 0.18 black or white is chosen on, and the exact-ratio case); I added a test for each, and three variants of the 0.18 threshold I then tried (0.1, 0.12, 0.25) still survived until I made the test insist that the colour is moved only as far as needed (not all the way to white or black). Four mutations did not apply the first time because rustfmt had reflowed the lines; three of those then failed a test and the fourth (an `is_finite` check) was **redundant, so I removed it**.

**One thing I could not verify independently:** I typed the three Machado matrices from my memory of the paper. The test that every row sums to 1 (greys stay grey) and the qualitative behaviour (red and green collapse for red-green blindness, blue and yellow for tritan) agree with them, but I did not compare the numbers with a second implementation, so a digit could be wrong in a way those tests would not see. The follow-up lists checking them against the paper.

## Not covered (follow-up T10-010b)

- **No screen and no file**: the options are a value; nothing shows them (T10-012), saves them, or applies them at start-up.
- **Nothing consumes them yet**: text layout does not call `scaled_text`, the HUD does not use the palette (T10-003), subtitles are not drawn (T06-012, T10-021), `ScreenEffects` does not take `flash_strength` or the limiter, camera bob and the underwater warp do not honour `reduce_motion`, and the input layer does not use `HoldToggle`.
- **Matrices to verify** against Machado et al. 2009 (above), and **severity below 1.0** (partial colour deficiency) is not offered; there is no daltonisation (shifting the colours rather than choosing safe ones).
- **No screen-reader or narration support**, no full controller remapping, no one-handed layouts.
- **The 200 threshold is mine**: it separates my two palettes well, but it is not a standard.
