# Transparency: blend modes and draw order

`ss2_render_api::transparency` (T04-016) holds the **device-free decisions** about transparent drawing: what each blend mode does, the order a frame's draws go in, when to use alpha-to-coverage, and the soft-particle fade. It draws nothing; the wgpu backend maps these descriptions one to one when materials (T04-007) and particles (T04-013) exist.

## Blend modes

| Mode | Equation (`src x f_src + dst x f_dst`) | Writes depth | Order matters |
|---|---|---|---|
| `Opaque` | replaces | yes | no |
| `AlphaTest` | replaces where the texture's alpha passes the material's cut-off, discards the rest | yes | no |
| `Alpha` | `src x alpha + dst x (1 - alpha)` | no | **yes** |
| `Additive` | `src x alpha + dst` | no | no (a sum) |
| `Premultiplied` | `src + dst x (1 - alpha)` | no | **yes** |

`BlendMode::factors()` returns the factors (`Factor::{Zero, One, SrcAlpha, OneMinusSrcAlpha}`) or `None` for the unblended modes; the tests evaluate the equations on numbers to check they do what the table says, including that additive commutes and the other two do not. `name`/`from_name` are for material files and consoles.

## Draw order

`sort_draws(&mut [DrawItem])` orders a frame's draws (`DrawItem` = id, mode, material, depth, layer, bias) and returns how many of each kind there are:

1. **Opaque**, grouped by material (fewer state changes), nearest first within a material (early depth rejection);
2. **AlphaTest**, the same;
3. **Blended** (alpha, additive, premultiplied), by **layer** (lower first), then **farthest first**, so nearer things cover what is behind them. Additive draws take part in the same sort rather than going last: a glow behind a glass pane has to be drawn before the pane.

`bias` is a distance added to a blended draw's depth for sorting (positive: drawn earlier, as if farther), for the things an artist wants in front of or behind where their position says (a decal card on a surface, a halo around a light). It does not affect opaque draws.

The order is **total and deterministic**: ties go to material, then to `RenderId`, so the same frame draws in the same order on every run and machine, whatever order the draws came in (tested on 300 draws with many equal depths and materials, shuffled 20 times). A non-finite depth is treated as the farthest possible rather than breaking the order.

`view_depth(camera, point)` is the distance **along the view direction**, not the straight-line distance: two cards side by side at one depth do not swap places as the camera turns.

## Alpha-to-coverage

`uses_alpha_to_coverage(mode, msaa_samples, enabled)` is true only for an **alpha-tested** draw, with **more than one sample per pixel**, with the option on. It softens the hard edges of foliage and fences; with one sample it would do nothing useful, and for blended modes it is the wrong tool.

## Soft particles

`soft_particle_fade(scene_depth, particle_depth, fade_distance)` is 0 where a particle touches the geometry behind it and 1 once it is `fade_distance` in front, linearly, so a smoke card does not cut a hard line at the floor. `linear_depth_from_reverse_z(depth, near)` turns a depth-buffer value into the distance the fade needs: `near / depth`, for the engine's **reverse-Z, infinite far plane** projection. A test checks it against the real projection matrix (`ss2_core::math::projection::perspective_infinite_reverse`) at five distances rather than restating the formula. A scene depth that is NaN or infinite (sky) leaves the particle fully visible; a non-positive fade distance turns the effect off.

## How it was tested

Seventeen unit tests and a doctest. **Twenty-three deliberate breakages: twenty-two failed a test.** One survived and is equivalent (`depth <= 0` against `depth < 0` in the reverse-Z conversion: a depth of exactly 0 gives `near / 0 = infinity` either way). Six of them could not be applied on the first try because rustfmt had reflowed the text; I redid those against the formatted source (a mutation that applies nothing proves nothing) and added the two tie-break mutations (material and id) then.

## Not covered (follow-up T04-016b)

- **No pipelines and no passes**: nothing maps `BlendFactors` to wgpu state, nothing creates the soft-particle depth read, and the renderer does not call the sort. Materials (T04-007) say which mode a draw uses; I have not seen how the original marks them.
- **Sorting is per draw**, not per triangle: two big intersecting transparent meshes, or one concave one, can still look wrong. Order-independent transparency is not attempted.
- **A depth is one number per draw** (the caller picks the point, usually the centre); a long thing like a beam is sorted by that point.
- **No sort cost measurement** for thousands of particles (the sort is an unstable comparison sort over the draws: the order is total, so stability does not matter).
- **No multiply or other modes** until the material data says they are needed.
