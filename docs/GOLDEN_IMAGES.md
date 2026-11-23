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
- **Goldens for the rest** (HUD, 3D passes) and the **resolutions and aspect ratios** of T10-015 and T10-017b.
- **Subfolders** in the golden folder (names are flat today).
- Comparing **audio or simulation traces** (the same `UPDATE_GOLDEN` workflow applies, but no helper exists).
