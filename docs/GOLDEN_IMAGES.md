# Golden-image harness

`ss2_render::golden` (T04-021) compares a rendered image with a stored one and, when they differ, leaves the evidence. It replaces the comparison code that
`crates/ss2-render/tests/golden_menu.rs` carried (a mean difference and a PNG loader) and is the one every later golden test (T10-015 screens, T04 passes) should use.

## What a check does

```rust
let mode = Mode::from_env(failures_dir);   // UPDATE_GOLDEN=1 -> update
golden::check_golden(&golden_dir, "menu_main_1280x720.png", &rendered, &Tolerance::default(), &mode)?;
```

- **Normal mode:** read `testdata/golden/<name>`, compare, pass or fail.
- **Update mode** (`UPDATE_GOLDEN` set, usually through `cargo xtask golden update`, which then prints a review report of what changed): write the rendered image as the golden.
- Names are flat `*.png` file names (letters, digits, `.`, `_`, `-`; no folders, no `..`), so a check can never write outside the golden folder.
- **A failure leaves two files** in the folder you give (`target/golden-failures/` in the menu tests): `<name>.actual.png`, what was rendered, and `<name>.diff.png`, what changed (black where
  equal, red where not, **amplified eight times** so a change of 3 levels shows). The error message says how many pixels differ, the limits and where the files are, and that
  `UPDATE_GOLDEN=1` regenerates the golden if the change was intended.
- Other errors say what is wrong: no golden yet (`Missing`), a golden that is not an 8-bit RGBA PNG (`Unreadable`), a different size (`SizeChanged`).

## The tolerance, and why it has two limits

`Tolerance { mean_abs: 4.0, channel_threshold: 16, max_differing_fraction: 0.001 }` by default:

- the **mean** absolute difference over all channels must be at most 4 (the limit the menu goldens always had; it catches a uniform brightness or colour shift);
- the share of **pixels that differ by more than 16 levels in some channel** must be at most 0.1 %.

The second limit is the reason for the change. A button that is missing from a 720p screen is about 2 % of the pixels: its difference **averaged over the whole image is under 2 levels**, below
the noise of another GPU's anti-aliasing, so a mean alone passes a screen with a hole in it (a test builds exactly that case). A different rasteriser rounds an edge or a glyph
differently and moves many pixels by a few levels (under the threshold, ignored); a missing or moved element moves a block by a lot (over it, counted). Loosen the limits for one
check on purpose with `Tolerance { max_differing_fraction: 0.01, ..Tolerance::default() }`; do not raise the defaults to make one test pass.

## Which adapter

`Renderer::new_offscreen` tries a hardware adapter and then the software fallback. Set **`SS2_FORCE_SOFTWARE_ADAPTER=1`** to use **only** the software adapter, so that a result does not depend on the
GPU in the machine (this is what CI should do, see below). On this machine (Windows, DirectX 12 software rasteriser) the five menu goldens, which were first made on whatever adapter was
there, **match under the forced software adapter** as well as under the hardware one, so the tolerance holds across those two.

**A crash:** once, with the software adapter forced, the menu test process died with an access violation (`0xc0000005`) after four of five tests, when the tests ran in parallel. The tests now
take a lock so only one creates a device at a time; four parallel forced runs and one normal run afterwards passed. I did not find the cause (it may be the driver or wgpu), and I cannot say
the crash is gone, only that I could not reproduce it after the lock.

## How it was tested

Fifteen unit tests (written first, seen to fail on the missing API) cover: the statistics and the exact threshold boundary (16 is not a differing pixel, 17 is; the alpha channel counts); a size mismatch; the missing-button
case (a tiny mean but a failure); noise that passes and a small real change that fails; the mean limit; the difference image (black, red, amplified, saturating); PNG round trip and damaged files;
a pass leaves nothing behind; a failure writes both images and names them; a missing golden and a changed size; update mode (creates folders, replaces); thirteen file names that must be refused
(`../x.png`, `/etc/x.png`, `a..b.png`, `C:x.png`, …); the adapter choice. One end-to-end test renders the real main menu and checks it **fails** against the options-menu golden and leaves
both files. Fourteen deliberate breakages each fail a test (either limit ignored, the threshold off by one, no amplification, a wrapping amplification, a name check removed, update mode ignored,
everything accepted, `true` not read as true, forcing ignored, a size change not seen); the first run left the threshold boundary, the `..` check and the force switch unproven,
so I added the boundary and name tests and fixed the mutation scripts that had silently matched nothing after rustfmt reflowed the code.

## Not done (follow-up T04-021b)

- **A CI job that installs a software adapter** (Mesa's lavapipe on Linux, WARP on Windows) and sets `SS2_FORCE_SOFTWARE_ADAPTER=1`. **GitHub Actions jobs do not start at the moment (billing), so I
  could not try it, and the goldens are checked only on this one Windows machine.** Until a job exists, the golden tests skip without an adapter (a green run proves nothing about rendering).
  Goldens made on a Linux software adapter may need `max_differing_fraction` loosened; nobody has measured it.
- **Goldens for the rest** (HUD, 3D passes, the settings, lobby and loading screens once they draw; see "The screen matrix" below) and **portrait** (T10-017b).
- **Subfolders** in the golden folder (names are flat today).
- Comparing **audio or simulation traces** (the same `UPDATE_GOLDEN` workflow applies, but no helper exists).

## The screen matrix (T10-015)

`crates/ss2-render/tests/golden_screens.rs` photographs **twenty-one screens at five sizes**, one hundred and five goldens (`menu_<screen>_<w>x<h>.png`, `ui_<screen>_<w>x<h>.png`, about 5.9 MB in all):

- the screens: the four menu screens (main, options, pause, about), the **developer console** with scrollback in all three line colours and typed text, the **frame-time overlay** over the main menu, and the **death screens** (solo, and co-op with a countdown) and the **end-of-level summary** (finished, and part-way through its count) of `docs/SESSION_SCREENS.md`, and the five **co-op overlays** of `docs/COOP_OVERLAYS.md` (scoreboard with seven players, chat with history and an open team input, nameplates, ping markers on and off screen, voice indicators), each drawn by the real `CoopOverlays` over the plain background, and the six screens of the **first-run wizard** of `docs/FIRST_RUN.md` (the list with three folders, nothing found, path entry, the clean result, the result with warnings, the refusal);
- the sizes: **three resolutions at 16:9** (1280x720, 1920x1080, 2560x1440), **21:9** (2560x1080, pillarboxed) and **4:3** (1024x768, letterboxed).

Each image is also checked **without** a golden, so that regenerating a golden cannot hide a fault: it has the size asked for; the bars outside the canvas are exactly the background colour (and there are bars exactly when the window is not 16:9); the canvas is centred; and on a menu the focused button is orange at the place `Canvas::fit` puts it. One more test checks that the 1280x720 and 2560x1440 pictures of the main menu agree pixel for pixel on every flat region (the same layout at 1x and 2x), and one that a different screen fails against each size's golden. `every_screen_has_a_golden_at_every_size` needs **no adapter**: a golden missing from the matrix fails even where the rendering tests skip.

**Regenerate** with `SS2_FORCE_SOFTWARE_ADAPTER=1 cargo xtask golden update` (the software adapter, so the goldens do not depend on the machine; `golden_screens` is in the list of targets). The goldens were made with the software adapter and **pass on this machine's Vulkan, DX12, GL and software adapters** (the three older 1280x720 goldens were made on another adapter and differ from a software render by one level in two percent of the pixels, far inside the tolerance, so they were left alone).

**What it caught.** Four deliberate breakages of the canvas fit (the pillarbox not centred, the scale taken from the larger side, the letterbox top-aligned, `to_screen` ignoring the offset): **the old 1280x720 tests passed all four** (at 16:9 the offset is zero), the matrix failed all four. Three of them were also run with the goldens **regenerated from the broken code**: the golden-free checks still failed, as they are meant to.

**Not covered (T10-015b).** Screens that do not draw yet (the HUD, the settings, lobby, loading and subtitle screens: add each to `SHOTS` as it lands); portrait (T10-017b); a CI job on a software adapter (see T04-021b: **GitHub Actions jobs do not start at the moment, so these tests ran only on this one Windows machine**, and goldens from a Linux software adapter may need a looser tolerance, which nobody has measured); the text in the pictures is not checked beyond the golden (a changed font would change every golden on purpose).

## The 2D renderer screens (T04-017)

`golden_screens.rs` also photographs **three pictures of the 2D renderer** (`docs/UI_RENDERER.md`) at the same five sizes: **fifteen more goldens** (`ui2d_<picture>_<w>x<h>.png`, 30 to 165 KB each, about 1.4 MB in all), **ninety in all**. They are built in the test from images made in code (no game content), as `UiFrame`s, because `ss2-ui` does not produce the new commands yet:

- `ui2d_panels`: the same 48x48 bordered image as a 9-slice at five sizes (96x64 to 300x300 and a 700x120 strip), one smaller than its borders (20x20), one with the border drawn at 2x, tinted and nearest-sampled, and two tiled ones (1x and 2x);
- `ui2d_list`: a list of ten rows scrolled so that the first and last are cut, each row a quad, an image and text clipped to its own cell (a nested clip), a scrollbar, a window with big text cut by its edge, and an image and a 9-slice cut by windows with fractional edges;
- `ui2d_target`: a card (9-slice, badge, clipped text) rendered into a target and drawn 1:1, at 2x with linear sampling, at 60 % over a pattern, a part of it, and a second target made from the first (the second copy faded) drawn 1:1 and enlarged.

Without a golden, each picture is checked for the size, for **nothing in the bars** outside the canvas, and (the list) for **no row colour in the strips just above and below the clip window**, with the second row where it should be; one more test checks that the card drawn through a target at 1:1 equals the card drawn in place **pixel for pixel**; `every_2d_renderer_screen_has_a_golden_at_every_size` needs no adapter. The goldens were made with `SS2_FORCE_SOFTWARE_ADAPTER=1` and **pass on the software, Vulkan, DX12 and GL adapters of this machine** without any tolerance change (the whole file: 23 tests in 214 s on the software adapter, 136 s on Vulkan, 111 s on DX12, 70 s on GL; `golden_menu`: 7 tests on each). Regenerate only these with `UPDATE_GOLDEN=1 SS2_FORCE_SOFTWARE_ADAPTER=1 cargo test -p ss2-render --test golden_screens -- nine_slice_panels a_scissored_list render_to_texture`.
