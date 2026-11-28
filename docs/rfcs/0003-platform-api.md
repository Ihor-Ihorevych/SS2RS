# RFC 0003: Platform events, input actions, lifecycle and locations (contract v0)

- **Number:** 0003 (`docs/rfcs/0003-platform-api.md`)
- **Authors:** @lxstyo
- **Status:** in review
- **Approvers:** owner of `ss2-platform` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T00-016 (this contract); unblocks T03-001/002/003/005/012/020 (platform implementation), T10 (UI input), T09 (player input), T04/T06 (reacting to surface and suspend events)

## Summary

Add a small crate `ss2-platform-api` that defines everything the app, UI and game code may know about the platform: device-independent input and lifecycle events, rebindable input **actions**, a lifecycle state tracker that answers "may I render now?", the file-system locations the app may use, and the `AppHandler` trait the platform loop calls. No `winit`, `gilrs`, window handle or OS type appears in it. Ship working helpers (`LifecycleTracker`, `MockInput`, `run_script`) so every consumer can be written and tested now.

## Motivation

`ss2-platform` links `winit` and `gilrs` (and later Android glue). If UI, gameplay or the headless server depended on it they would pull a windowing stack in and could not be tested without devices. The roadmap (T00-016) also requires the *portability guard rails* from the design (§8): lifecycle must handle suspend/resume and surface loss, nothing may assume a mouse, and paths must come from a platform layer rather than `current_dir`. Freezing the contract lets T03 (platform), T10 (UI) and T09 (player) proceed in parallel.

```
ss2-ui / ss2-game / ss2-app / ss2-server ──► ss2-platform-api ◄── ss2-platform (winit, gilrs, Android glue)
```

## Detailed design

All code is in `crates/ss2-platform-api/`; rustdoc examples run as tests.

### Events

```rust
pub enum PlatformEvent { Lifecycle(LifecycleEvent), Input(InputEvent) }   // #[non_exhaustive]
```

- **`InputEvent`**: `Key{key, pressed, repeat}`, `Text(String)`, `MouseMotion{dx,dy}` (raw, for aiming), `CursorMoved{x,y}` (absolute, for menus), `MouseButton`, `Scroll`, `GamepadConnected/Disconnected`, `GamepadButton`, `GamepadAxis` (dead-zone filtered by the platform), `Touch` (Phase 2; present so UI is written without assuming a mouse). Coordinates: window pixels, origin top-left, y down.
- **Keys are physical positions** (`KeyCode`, layout independent), typed characters arrive separately as `Text`. This is what both gameplay (WASD must work on AZERTY) and the console (needs real characters) need. `KeyCode`, `MouseButton`, `GamepadButton` (standard layout, position-named) and `GamepadAxis` are our own enums, `#[non_exhaustive]`; **variant names are the serialized form of saved bindings and must never be renamed** (a test pins this).
- **`LifecycleEvent`**: `Resumed`, `Suspended`, `SurfaceReady(SurfaceInfo)`, `SurfaceLost`, `Resized`, `ScaleFactorChanged`, `FocusChanged`, `LowMemory`, `CloseRequested`. Sizes are physical pixels.
- Every event enum is `#[non_exhaustive]`, so adding a variant is not a breaking change; consumers keep a wildcard arm.

### Lifecycle

`LifecycleTracker` folds the lifecycle stream into `can_render()`: not suspended, a surface exists, and it is not minimized (`0 x 0`). Guaranteed order from the platform: `Resumed` then `SurfaceReady`; `Suspended` is always preceded by `SurfaceLost`; after the next `Resumed` a new `SurfaceReady` arrives before drawing is legal again; size events only with a surface. Desktop never sends `Suspended` (minimize is a zero-size `Resized`); Android sends the full cycle on every background switch. The tracker **does not trust** these guarantees: it never panics, `Suspended` also drops the surface, and size events without a surface are ignored. The renderer (T04-001) uses it to know when to drop and re-create its surface and GPU resources.

### Input actions

Gameplay and UI read **actions**, not keys: `ActionQuery { pressed, just_pressed, just_released, value, context }` by action name (`&str`). Bindings are data: `ActionMap` maps `ActionId` (a string, because names come from files and mods) to `Vec<Binding>` where `Binding` is `Key | Mouse | GamepadButton | GamepadAxis{axis, positive}`. Several bindings per action are normal; `actions_using` supports rebind-conflict warnings. `ActionMap` and everything in it derive `serde` and round-trip through TOML (tested, including "unknown key name is an error, not a panic"); a `BTreeMap` keeps the file and iteration order stable. `InputContext` (`Game | Menu | Console`) says which set receives input. Analog sticks are expressed as two half-axis bindings (`move_left`/`move_right`), so every action value is `0..=1`.
The real event-to-action implementation is T03-005; `MockInput` (hand-driven, with correct one-frame edge semantics) is the test double.

### Locations

`PlatformPaths { config_dir, save_dir, cache_dir, log_dir, game_data_dir: Option<PathBuf> }` with `under(root)` (portable mode and test sandboxes), `settings_file()`, `bindings_file()`, `save_file(name)` (rejects separators, `..`, drive letters, control characters, so mod- or network-supplied names cannot escape the save folder) and `ensure_dirs()`. Per-OS resolution (XDG, `%APPDATA%`, macOS, Android app storage) is T03-012 and produces this struct; **no other crate resolves directories itself**. Saves and cache are separate folders (cache may be deleted). `game_data_dir` is desktop-only; the Phase 2 Android port reads game data through the file-provider trait (T02-005).

### The loop

```rust
pub trait AppHandler {
    fn event(&mut self, event: PlatformEvent) -> AppControl;
    fn frame(&mut self, frame: &FrameInfo) -> AppControl;   // Continue | Exit
}
```

One thread, `event` before the `frame` that should see it; `frame` also runs while suspended (the app decides whether to render, audio/net may keep ticking). `FrameInfo` carries wall-clock `delta` for rendering and the fixed-timestep accumulator (T03-013); simulation never reads it. `run_script(handler, steps)` drives a handler with `Event`/`Frame` steps and a fixed 1/60 s delta, stopping at the first `Exit`: the test double for the window loop, and what the headless server and CI use.

Not in the contract: window creation, cursor capture, display modes (T03-017), the fixed-timestep driver itself (T03-013), the settings schema (T03-014), the console (T03-015).

## Alternatives considered

- **Re-export `winit` types.** Rejected: drags a windowing stack into every consumer and the headless server, and ties saved bindings to a third-party enum's names.
- **Keep `NavInput` in `ss2-core` as the model and extend it.** `NavInput` (menu navigation only) stays where it is; actions are broader (rebinding, analog, gamepad axes) and would bloat `ss2-core`.
- **Actions as an enum.** Rejected: names come from data and mods; a string id keeps the engine free of game-specific action lists.
- **Callback-based input (`on_key(...)`).** Rejected: an event enum plus per-frame action queries is easy to record, replay and script.
- **Putting paths in `ss2-core::settings`.** Rejected: settings already hard-code one directory; a `PlatformPaths` value passed in is testable and portable.

## Compatibility and migration

- New crate only; no existing code changes. `ss2-platform` (T03) will depend on it and translate `winit`/`gilrs` into these types; `ss2-core::input::NavInput` and `ss2-core::settings::default_path` stay until T03 migrates them (T03-012/T03-014 own that).
- **Window handoff (T03-001).** A renderer needs the real window to create its surface, which this windowing-free crate cannot carry. It lives in `ss2-platform`: `window::run(config, &mut dyn PlatformApp)`, where `PlatformApp: AppHandler` adds `window_ready(&PlatformWindow)` (called between `Resumed` and `SurfaceReady`) and `window_lost()` (called before the window goes away). `AppHandler` itself is unchanged, so headless code and tests keep using only this crate.
- **Input delivery (T03-002).** `window::run` delivers `InputEvent`s for keys (physical, with auto-repeat), typed text (control characters filtered; Enter, Tab and Backspace are keys), absolute cursor position, mouse buttons, scroll (lines; touchpad pixels converted at 20 px per line) and raw relative mouse motion (only while the window has focus). The platform keeps the contract's promise that **losing focus releases every held key and button** by synthesizing the releases itself, drops `winit`'s own synthetic key events and any release of something the app never saw pressed, and releases a cursor capture (`PlatformWindow::set_cursor_captured`, which also stops absolute positions; the app captures again on `FocusChanged(true)` if it wants to).
- After merge the signatures are frozen; compatible additions (new enum variants, new methods) need owner approval, anything else a new RFC. The design doc crate map gains `ss2-platform-api`.

## Open questions

- **Gamepad identity and multiple local players**: v0 treats "any pad" as one input source in `Binding`; per-pad bindings (split-screen co-op) are decided by the `ss2-platform` owner with T03-005.
- **Text input / IME composition events** (preedit): needed for CJK chat later; added as `InputEvent` variants when T03-002 implements text input.
- **Action value for sticks**: half-axis bindings keep values `0..=1`; if a consumer needs a signed value, a `value_signed` helper can be added without breaking.
- **Rumble / haptics output** (platform → device) is not in v0; decided with T03-003.

## Amendments

### Amendment 1: reading the clipboard (T10-035)

The first-run wizard's path field needs paste, and nothing in the contract reached the clipboard. A compatible addition:

```rust
pub trait Clipboard { fn text(&mut self) -> Option<String>; }   // the text on it, as it is; None when there is none
pub struct MockClipboard { pub text: Option<String> }           // for tests; MockClipboard::with_text(..)
```

- **Read only, text only.** Copying out of the app, images and change notifications are not needed by anything yet.
- **The app owns the policy.** The platform hands the text over as it is (new lines included); what a field keeps is the field's
  business (`ss2_ui::data_wizard::TextField` drops control characters). The platform reports no modifier state, so the key
  combination (Ctrl+V, Shift+Insert) is recognised by the app from the Ctrl and Shift key events it already gets.
- **Implementation** (`ss2_platform::clipboard::SystemClipboard`): on Windows `CF_UNICODETEXT` through `windows-sys`, the crate's
  only `unsafe`, in one small module with a `// SAFETY:` comment at every block (`docs/CODING_STANDARDS.md`). `arboard` was tried
  first and refused by `cargo deny` (its `clipboard-win` is BSL-1.0, which `deny.toml` does not allow). Elsewhere it returns `None`
  for now, so pasting does nothing on Linux and macOS (T10-037).
