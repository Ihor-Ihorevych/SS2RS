# The loading screen model

`ss2_ui::loading` (T10-009) is what a loading screen needs to know, without drawing anything or reading a clock: **weighted stages**, a progress bar that **never goes backwards**, an **estimate of the time left**, a **status line**, and **tips that rotate without repeating**. A stage can be anything that takes time: reading archives, building the world, uploading textures, **compiling shaders and warming the pipeline cache** (T04-007's on-disk cache, T05-008).

```rust
let mut load = LoadingTracker::new();
let world = load.add_stage("Building the world", 3.0)?;
let shaders = load.add_stage("Compiling shaders", 1.0)?;
load.set_total(world, 200);          // when known; it may grow as work is discovered
load.advance(world, 1);              // as work completes
load.update(dt);                     // every frame, with the time that passed
draw_bar(load.displayed());          // 0..1
draw_text(load.status());            // "Building the world: 120 / 200"
```

## Progress

`fraction()` is the weighted mean of the stages (a stage counts 0 until its total is known, 1 once finished, and work past the total counts as complete). `displayed()` is what to draw: it **chases the true progress at most 1.5 of the bar a second** (a jump from 10 % to 90 % takes about half a second instead of one frame) and **never moves back**, even if a stage discovers more work and the true progress drops. `is_done()` is true when every stage is finished **and** the bar has reached the end, so a screen does not close with the last of the animation unseen.

## The estimate

`eta_seconds()` uses a smoothed speed (an exponential average of the bar's progress per second) and says nothing (`None`) until **two seconds** have passed, while nothing is moving, and when the work is done. A fall in the true progress (more work found) is **not** read as a negative speed: tested by finding ten times the work at 21 % and still getting a positive estimate ten steps later.

## Status and tips

`status()` is the first stage that is not finished and its counts (`"Archives: 12 / 40"`, never showing more than the total), or just its name while its size is unknown; `None` when all are finished.

`TipRotator::new(tips, seconds, seed)` shows each tip once before any repeats, in an order fixed by the seed (a shuffle that can leave a tip in place; a test rules out the cyclic-only mistake), changing every `seconds` (at least 1; NaN counts as 5). Odd times (negative, NaN, infinite) show the first tip; no tips shows nothing. Seed 0 behaves as seed 1.

## How it was tested

Fifteen unit tests and two doctests: the weighted mean in three configurations, unknown and overshot totals, finishing, stage limits and bad weights (and ids from nowhere), the bar's chase, its never going back and its finish, bad time steps, the estimate's silence and its value on a steady 10 %-a-second load, its smoothing against a hand-computed value (8.9 s at 2.1 s in), the late discovery of work, the status line through a whole load, and the tip cycle, interval, seeds and odd inputs. **Twenty-four deliberate breakages: sixteen failed a test at once and eight survived**; two were equivalent and I **removed the code** (an exact-1.0 shortcut and a clamp on the fraction: each term is at most its weight and floating-point addition is monotonic, so the sum cannot exceed the total weight, and with every stage finished the two sums are the same number), and six were real gaps (negative speed, the smoothing factor, seed 0, the shuffle's range, the minimum tip interval, and an infinite time). Three of those only fell once I chose inputs that tell the right code from the mutant: with three tips `usize::MAX` is a multiple of 3 and a saturated time landed on the same tip by chance, so the test uses four; and the first late-discovery test used a drop too small to push the mutated speed below zero for long enough.

## Not covered (follow-up T10-009b)

- **Nothing draws it**: no bar, text or tip is drawn, and nothing in the engine reports progress to a tracker (the loaders, the world, the renderer's warm-up do not exist yet or do not call it).
- **Tips are caller-supplied text**: the game's own tips and their localization (T10-011) are not here, and the screen's background art and layout are the menu skin's (T10-033).
- **No cancellation or error display** (a failed load is the caller's to handle: it can finish a stage and show its own message), no per-file names, and no persistence of how long loads took last time to improve the estimate.
- **The speed (1.5 a second), the smoothing (0.9) and the two-second wait are my numbers**, not tuned on a screen.
