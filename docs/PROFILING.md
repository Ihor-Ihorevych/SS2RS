# Profiling

The client can stream frame timings to a profiler while it runs. This is optional, off by default, and costs
nothing in a normal build.

The profiler is [puffin](https://github.com/EmbarkStudios/puffin) (MIT or Apache-2.0, pure Rust, no C++ build and no
version-matched GUI as Tracy has). It is a build-time feature (`profile`) of `ss2-app`, switched on at run time with
an environment variable.

## Quick start

1. Install the viewer once: `cargo install --locked puffin_viewer`.
2. Start the app with profiling built in and turned on:

   ```
   scripts/profile.sh                 # Linux, macOS, Git Bash
   scripts/profile.ps1                # Windows PowerShell
   ```

   Both run `cargo run -p ss2-app --features profile` with `SS2_PROFILE=1`. Extra arguments go to the app
   (`scripts/profile.sh --smoke-frames 3000`). If `puffin_viewer` is on the `PATH` the script also starts it.
3. In the viewer, connect to `127.0.0.1:8585` (the default of `puffin_viewer`). Each frame of the app appears as a
   bar with its scopes underneath. Use the viewer's **Save** button to keep a capture as a `.puffin` file and open
   it later with `puffin_viewer --file capture.puffin`.

## What is measured

| Scope | Where | What it covers |
|---|---|---|
| frame | `profiling::frame()` at the start of `redraw` | the boundary between two frames (puffin groups scopes by it) |
| `redraw` | `crates/ss2-app/src/app.rs` | the whole redraw handler |
| `render` | same | `Renderer::render` (building and submitting the frame) |

To time more code, add `profiling::profile_scope!("name");` (a string literal) at the top of a block in `ss2-app`.
The scope ends with the block. Without the `profile` feature the macro expands to nothing.

The `tracing` spans of the engine (for example the `job` span of the worker pool, `ss2_core::jobs`) are **not**
forwarded to puffin: they are written to the log (at `trace` level) and a puffin scope has to be added by hand. A
bridge from `tracing` to the profiler is not part of this task.

## Settings

| `SS2_PROFILE` | Effect |
|---|---|
| unset, empty, `0`, `off`, `false` | no profiling (the default) |
| `1`, `on`, `true`, `yes` | serve on `127.0.0.1:8585` |
| `IP:PORT`, for example `127.0.0.1:9000` or `[::1]:8585` | serve there |
| anything else | a warning is logged and profiling stays off |

The server accepts connections from whoever can reach the address. The default is local to the machine; an address
such as `0.0.0.0:8585` makes the stream available to the network and the app logs a warning. Profile data is timing
only (scope names and durations), no player or game data.

A build without the feature that is started with `SS2_PROFILE=1` only logs that it has no profiler; rebuild with
`--features profile`. The options and the status are written to the log file (see `docs/user/troubleshooting.md`).

## Checking that it works without the viewer

With the app running (`SS2_PROFILE=1 cargo run -p ss2-app --features profile -- --smoke-frames 400000`), a plain TCP
connection to `127.0.0.1:8585` returns a stream that starts with the bytes `02 00 50 46 44 34` (`PFD4`, the puffin
data packet) and contains the scope names `redraw` and `render`. This is how the integration was checked on Windows.

## Not covered

- GPU timings (T04-019) and the in-game overlay (T10-007) are separate tasks.
- Tracy is not integrated: its client library compiles C++ and the viewer must match the library version, which
  makes setups fragile. If a developer needs it, a `profile-tracy` feature can be added next to `profile` later.
- Network profiling (`net_graph`, T08-018) and frame pacing (T03-013) have their own tasks.
- `ss2-server` is not instrumented yet.
