# Anti-aliasing planning

`ss2_render_api::anti_aliasing` (T04-029) decides **which anti-aliasing method to use**, given what the player asked for (`QualitySettings::anti_aliasing`) and what the device can do, and sizes the render targets it needs. It is device-free: it draws nothing, runs no shader and creates no GPU object. The MSAA resolve, the FXAA shader and the TAA history blend belong to the renderer (T04-029b).

## The plan

`plan_anti_aliasing(mode, temporal, MsaaSupport)` returns an `AaPlan { msaa_samples, fxaa, taa, degraded }`:

| Request | Plan |
|---|---|
| `temporal` (TAA) | 1 sample, TAA on, FXAA off: TAA **replaces** the others (all three together costs a lot and looks worse) |
| `Off` | 1 sample, nothing else |
| `Fxaa` | 1 sample, FXAA on |
| `Msaa2` / `Msaa4` | that many samples if the device supports them; otherwise **the best it has below the request**; and if it has none, 1 sample **with FXAA**, so the player still gets some smoothing |

`degraded` is true whenever the player got less than they asked for, so a settings screen can say so. It never rounds **up**: a device with only 4x does not give 2x (it falls back to FXAA instead; 4x costs more than the player chose). `MsaaSupport::from_counts` takes the counts the device reports for the formats in use (only 2, 4, 8, 16 mean anything; one sample is always supported); `best_at_most(n)` finds the largest supported count not above `n`.

With the quality presets on a device that has everything: Low draws no anti-aliasing, Medium FXAA, High 2x MSAA, Ultra 4x. On a device with no MSAA, High and Ultra fall back to FXAA and are marked degraded (tested).

**TAA** has no setting in `QualitySettings` yet (the enum has Off, FXAA, MSAA 2x and 4x); `temporal` is a separate switch the caller passes. `taa_jitter(plan, frame, w, h)` is the projection jitter for the frame (the Halton sequence from `jitter_for_frame`) when the plan uses TAA and **zero otherwise**, so a camera can always add it without checking.

## FXAA settings

`FxaaSettings { subpixel, edge_threshold, edge_threshold_min }` with the algorithm's usual defaults (0.75, 0.166, 0.0833) and `validate()` against the documented useful ranges (0 to 1; 0.063 to 0.333; 0.0312 to 0.0833), including both ends.

## Memory

`render_target_plan(plan, window_w, window_h, render_scale, bytes_per_pixel)` sizes the scene targets (the window times the render scale, clamped to 0.25 to 1, rounded to the nearest pixel and never below 1 by 1) and counts the colour memory: **MSAA multiplies the colour target by the sample count and adds a single-sample resolve target** (4x on a 1080p `Rgba16Float` target: 5 times 16.6 MB = 83 MB, against 16.6 MB for none), and TAA adds a history buffer. Depth is not counted (it is multisampled as well; the renderer adds it with its own format). The arithmetic saturates instead of overflowing. This is what a phone must see before it asks for MSAA.

Alpha-to-coverage (`transparency::uses_alpha_to_coverage`) needs `msaa_samples > 1`; pass it `plan.msaa_samples`.

## How it was tested

Twelve unit tests and a doctest: the support bits (no mixing of the sample counts), the best count under a wish (including that it never rounds up and that 0 or `u32::MAX` are safe), every row of the table above, degradation to a lower count and to FXAA, TAA replacing everything and never being degraded, the preset table on a full and an empty device, the jitter matching the camera's and being zero without TAA, the FXAA ranges at both ends and just outside, memory for none, 4x and TAA, the render scale's clamp and rounding, and saturation on huge sizes (which found that my first multiplication overflowed). **Twenty-five deliberate breakages: all twenty-five failed a test**; one of my scripts first did not apply because rustfmt had reflowed the lines, and was redone. As before, this shows these edits were seen, not that the tests are complete.

## Not covered (follow-up T04-029b)

- **Nothing draws it**: no multisampled targets are created, no resolve, no FXAA shader, no TAA history, motion vectors, neighbourhood clamping or disocclusion handling (the part of TAA that is hard). The FXAA parameters are those of the published algorithm, untested on this renderer's images.
- **The device's real MSAA support** has to come from `wgpu` (`TextureFormatFeatures` for the colour and depth formats), which is not read yet, and the cost of MSAA on a phone's tiled GPU (cheap in memory bandwidth, so it may be the better default there than FXAA) is not measured.
- **TAA is not a quality setting** and has no screen option; and the mobile policy (prefer MSAA on tile-based GPUs) is not encoded.
