# Free-fly debug camera

`ss2_game::freefly` (T07-020) is the camera for flying anywhere through a level, through walls, before the character controller exists.
It is also what the `noclip` cheat (`docs/CHEATS.md`) and a spectator view will use. It is pure logic: the app turns keys, mouse and gamepad
into a `FreeFlyInput` each frame and renders from `FreeFlyCamera::camera()` (a `ss2_render_api::Camera`). No window, GPU or clock is touched,
so it is tested without any of them.

```rust
let mut cam = FreeFlyCamera::new(Vec3::new(0.0, 2.0, 5.0), FreeFlyConfig::default());
// every frame:
cam.update(dt_seconds, &FreeFlyInput {
    movement: Vec3::new(strafe, rise, forward),   // each -1..1; a stick may give less
    look: Vec2::new(mouse_dx, mouse_dy),          // pixels; right and down are positive
    boost: shift_down,
    slow: ctrl_down,
});
render(&cam.camera());
// mouse wheel: cam.scroll_speed(notches); teleport: cam.set_position(p); aim: cam.look_at(target)
```

## Behaviour

- Conventions of `ss2_core::math`: right-handed, Y up, -Z forward. Yaw 0 looks down -Z and a positive yaw turns left; a positive pitch
  looks up; **no roll**. Mouse right turns right, mouse up looks up.
- **Forward and back follow the view** (including up and down), **strafing is level**, **rising is straight up in the world**, even when looking up.
- A **diagonal is no faster** than straight ahead (the push is limited to length 1); a half-pushed stick moves at half speed.
- `boost` (default x4) and `slow` (default x0.25) scale the speed; both together cancel. The **scroll wheel** changes the cruising speed by
  a factor of 1.25 per notch, between 0.1 and 1000.
- **Pitch stops at 89 degrees** (also when aiming with `look_at`), so the view never flips. Yaw is kept within one turn so that precision
  does not drift after many turns.
- **Smoothing:** `accel_time` is the time to reach about two thirds of the target speed (default 0.08 s; 0 follows the input at once).
- **Frame-rate independent:** the same time in 20 steps or 100 steps goes the same distance (with `accel_time` 0).
- **Bad input is ignored, never NaN:** a non-finite movement or look, a negative or NaN `dt`, a non-finite position. A `dt` of 0 applies
  only the look. A `dt` longer than 0.25 s (a breakpoint, a stall) counts as 0.25 s so the camera is not flung away.

## Tests

Twenty-four unit tests in the crate (the camera's share is twelve), written first and seen to fail on the missing API. Ten deliberate
breakages (the mouse x and y signs, the diagonal limit, the step limit, a negative `dt` allowed, a non-finite look allowed, a lost yaw wrap,
the slow key ignored, rising along the view's up instead of the world's, the pitch limit missing from `look_at`) each fail a test. Two
survived at first and were fixed by strengthening the tests: rising was only tested while looking level, and the `look_at` pitch was only
tested against a loose bound. Three of my first tests also stepped a whole second at once and ran into the step limit (the limit is the
designed behaviour); they now step in small increments.

## Not covered

- **Colliding with the world** (noclip means no collision, so none is needed; a collision mode waits for the character controller, T07-005).
- **Reading the keys, mouse and gamepad** and applying `sensitivity` from the settings: the app's input mapping (T03-033, T10).
- **Rebindable actions** for these controls (they are `ActionMap` names the app declares) and a console command to toggle the camera.
- **Roll, orbit or follow modes** (the spectator camera is T08-024).
