# Graphics quality

`ss2_render_api::quality` (T04-020) is the **contract for graphics quality**: presets, named per-setting values, and a dynamic-resolution controller. It is device-free logic: no GPU, no window, no clock. The renderer reads a `QualitySettings` to decide what to draw and how well; the settings screen (T10-012) and a console use the same list.

## Presets

`QualityTier` is `Low < Medium < High < Ultra`; `QualitySettings::preset(tier)` gives the values and `QualitySettings::default()` is `High`. `tier_matching()` says which preset the values equal exactly, or `None` for "Custom" (a settings screen shows it after the player changes one value).

| Setting | Low | Medium | High | Ultra |
|---|---|---|---|---|
| `shadows` | off | low | medium | high |
| `texture_detail` | quarter | half | full | full |
| `anisotropy` | 1 | 4 | 8 | 16 |
| `anti_aliasing` | off | fxaa | msaa2 | msaa4 |
| `lod_bias` | 2.0 | 1.0 | 0.0 | -0.5 |
| `view_distance` | 0.6 | 0.8 | 1.0 | 1.5 |
| `particle_density` | 0.3 | 0.6 | 0.85 | 1.0 |
| `bloom` | off | on | on | on |
| `render_scale` | 0.7 | 0.85 | 1.0 | 1.0 |
| `dynamic_resolution` | on | on | off | off |
| `max_decals` | 50 | 150 | 400 | 1000 |

**These numbers are my first guesses, not measurements.** Nothing was timed on any GPU, and most of the settings control passes that do not exist yet (shadows, anti-aliasing, bloom, decals). A test checks the one property that matters now: **no setting of a lower tier is better than the tier above** (a higher `lod_bias` is coarser).

## Settings by name

`QualitySettings::options()` lists every setting with its kind (`Choice` words, `Range`, `Count`, `Toggle`) in the order a screen shows them; `get(name)` returns the value as text and `set(name, text)` reads it back. Reading is loose (any case, spaces around, `on/true/1/yes`) and checking is strict: an unknown name, a word that is not in the list, a number outside its range, `NaN`, `inf` or a non-integer count is **refused with a message that names the setting and says what it takes, and nothing changes** (tested for 23 bad values). A test sets every option of every preset from its own text and gets the preset back. `validate()` checks values that were assigned to the fields directly.

## Dynamic resolution

`DynamicResolution` is fed one frame time in milliseconds per frame and returns the render scale for the next frame (the UI stays at full resolution; only the 3D scene is scaled).

- It decides on the **average of a window** (8 frames by default), not on single frames; a frame time that is not a positive number (a stall on a window event, `NaN`) is **ignored** and does not count toward the window.
- **Down quickly**: a window over the target (16.7 ms by default) lowers the scale by one step (0.05), **two steps if it is more than 1.5 times too slow**.
- **Up slowly**: a window under 80 % of the target counts as calm; **four calm windows in a row** raise the scale one step. A window in the middle band breaks the run. The scale never goes above the **ceiling** (the player's `render_scale`) or below `min_scale` (0.5), and always sits on the step grid.
- Deterministic: no clock inside; the same frame times give the same scales. In the doctest a scene costing 25 ms at full scale and scaling with the pixel count settles below 0.85 within 200 frames; in the test with 28 ms it ends with a frame under the target and changes the scale at most twice in the last 200 frames (it does not hunt).

The configuration is validated (`DynamicResolutionConfig::validate`).

## How it was tested

Twenty unit tests and a doctest. Fifteen deliberate breakages: ten failed a test at once, **five survived my first tests**. Four were real gaps (the two exact-threshold comparisons, a zero frame time counted, and the decal bound off by one); I added a test for each and they now fail. The fifth (removing the "below the ceiling" check on the step up) is equivalent: the step is clamped to the ceiling anyway.

## Not covered (follow-up T04-020b)

- **Nothing reads the settings yet**: no renderer pass honours them (shadows, anti-aliasing, bloom and decals do not exist), and `ss2-render` does not render at a scale. The `lod_bias` and `view_distance` hooks belong to T04-024.
- **No settings file and no screen** (T10-012), **no console command** (`r_shadows high` is what `set` is for), and **no auto-detection** of a tier from the GPU (and the frame-time feed from a real frame timer).
- **Presets are unmeasured.** They need tuning on real GPUs, including a phone (Phase 2, T06-006b uses the tier for the voice limit).
