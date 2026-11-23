# Gamepads

`ss2_platform::gamepad` (T03-003) turns what `gilrs` reports into the `InputEvent`s of the platform contract (`ss2-platform-api`, RFC 0003),
in the **standard layout**, with **dead zones**. The window loop (`ss2_platform::window::run`) polls it once per loop iteration, so an app on that
loop receives `GamepadConnected`, `GamepadDisconnected`, `GamepadButton` and `GamepadAxis` events with no further work.

## What the contract promises, and what the module does

| Promise | How |
|---|---|
| Buttons are the **standard layout** (`South` is `A` on Xbox, `Cross` on PlayStation) | `button_from_gilrs`: the face buttons, bumpers, sticks, D-pad, Start, Select and Guide; the analog trigger "buttons", `C`, `Z` and unknown buttons have no place and map to nothing |
| Sticks run -1 to 1, **up and right positive**; triggers 0 to 1 | `gilrs` already reports up as positive; triggers come from the analog value of `LeftTrigger2` and `RightTrigger2` (or from the `Z` axes on pads that report them so) |
| Values are **dead-zone filtered** | a **radial** dead zone on each stick (default 0.15): inside the circle the stick reads exactly 0, outside it the length is stretched so that the edge reads 0 and full deflection reads 1, the direction is kept, and a square pad's corner is limited to the unit circle; triggers have their own (default 0.05) |
| A pad that disconnects **releases** everything | the translator sends a release for every held button and a zero for every non-zero axis, then `GamepadDisconnected` |

Also: a **pad that was not announced** is announced before its first input; a connect for a pad that is already known says nothing; a button
that is already pressed (or a release of one that is not held) says nothing; an axis change under 0.005 is not reported, but **a return to exactly 0
always is** (a stick settling at 0.004 must not stay 0.004 forever); a D-pad that a pad reports as a **hat axis** becomes D-pad buttons;
NaN and infinite values are ignored; a silly configured dead zone is limited to 0.95 rather than dividing by zero.

## The pieces

- `GamepadTranslator`: pure. `translate(pad, PadEvent) -> Vec<InputEvent>`. `PadEvent` is what `gilrs` said, reduced to values that can be built
  without a real pad, which is what makes everything above testable.
- `GamepadSource`: the shell around a real `gilrs` instance: `poll(&mut Vec<InputEvent>)`, `name(pad)` and `rumble(pad, strong, weak, duration)`.
- **The mapping database:** `gilrs` carries the SDL game controller database (hundreds of pads). Extra mappings in the SDL `gamecontrollerdb` format
  can be passed to `GamepadSource::new`, and the window loop passes the standard **`SDL_GAMECONTROLLERCONFIG`** environment variable.
- **Rumble:** `rumble` plays a strong and a weak motor effect and keeps it until the next one; it returns `false` for a pad that is not there or
  cannot rumble. It is not reachable from a `PlatformApp` yet (the source lives in the window loop): T03-033 wires it.

## How it was tested

Seventeen tests: the whole button map, the dead zone maths (inside, edge, half way, full, a diagonal, a corner, signs; a **circle, not a square**),
the trigger dead zone, announcing pads, change-only buttons, stick pairs (an axis re-reported when the other moves), independent sticks, noise
and the forced return to zero (including the stuck-near-zero case), triggers from button values, hats, disconnects, two pads not sharing state, bad
values and silly dead zones, and the real source built and polled with no pad attached (it started on the author's Windows machine and reported no
events). **Eleven deliberate breakages each fail a test** (the dead zone not stretched; a square dead zone; the direction not kept; no trigger dead
zone; the hat's up inverted; a disconnect that does not release buttons; no forced zero; no noise filter; pads not announced; Guide mapped wrong; the dead zone
not limited). **Three survived at first** (the square dead zone, the dead zone limit and the forced zero) because my tests used points where both versions agree; I added the missing
points and re-ran them to see them fail.

## What was **not** tested

- **No real pad was used.** Nothing here has seen an actual Xbox, PlayStation or Switch controller; the mapping of each is `gilrs`'s database and
  whatever it does with the pad. Rumble has never run.
- **The `gilrs` events that carry a code** (button presses and axis changes) cannot be built outside `gilrs` (`Code` is private), so the function that reduces them to
  a `PadEvent` is tested only for the events that can be built; its other arms are plain field copies checked by the compiler.
- **Triggers reported as `Z` axes** are treated as 0 to 1 (negative reads 0); a pad that reports them as -1 to 1 with -1 at rest would read as a
  half-pressed trigger. I do not know of one that `gilrs` maps this way.
- Linux and macOS (only Windows ran it); **Android's native pads** (`GameActivity`, T03-021).

## Not covered

- The app (`ss2-app`) still polls `gilrs` itself; moving it onto `window::run` is T03-033.
- **Dead zones in the settings** (a slider, per stick), **stick curves and aim assist** (T03-025), **gyro**, **hot-plug UI** and **per-pad bindings**.
- Several pads as several players (the events carry a `GamepadId`; what to do with them is a gameplay decision).
