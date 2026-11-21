# Coding standards

## Unsafe

`unsafe_code = "deny"` is set for the whole workspace. A crate that genuinely needs `unsafe` must (1) isolate it in one small module with `#![allow(unsafe_code)]` *only there*, (2) document the invariants in a `// SAFETY:` comment at every block, (3) have tests (and Miri where practical), and (4) be approved in review by the crate owner and a second maintainer. Parsers of untrusted data should not need `unsafe`.

## Errors

- Library code returns `ss2_core::Result<T>` / `ss2_core::Error` (or a crate-specific `thiserror` enum that converts into it).
- **Never panic on bad input.** Malformed game files, packets and settings produce errors, not panics. `unwrap`/`expect` are for invariants that cannot be violated by input, with a message saying why.
- Add context (`Error::io(path, source)`, `Error::invalid("what", "detail")`) instead of bare messages.

## Logging

Use `tracing` (`info!`, `warn!`, …) with the crate name as the target. Binaries call `ss2_core::logging::init_logging("info")`; users override with `SS2_LOG`, e.g. `SS2_LOG=ss2_vfs=debug`. No `println!` in libraries.

## Naming and layout

- Crates are `ss2-*`; modules and files are `snake_case`; one clear responsibility per file.
- Public items are documented. Tests live next to the code (`#[cfg(test)]`) or in `tests/`.
- Keep GPU/window/OS types out of headless crates (`ss2-ui`, `ss2-core`, parsers, simulation).

## Sim-safe

`ss2-sim`, `ss2-physics` and `ss2-net` (and movement code in `ss2-game` that feeds prediction) must behave identically on every machine and every run, because the server and clients re-run the same movement code and compare results.

- **No `HashMap` / `HashSet`** (iteration order varies between runs). Use `BTreeMap`/`BTreeSet` or a `Vec` with a stable order. Enforced by `disallowed-types` in each crate's `clippy.toml`.
- **No wall-clock time** (`Instant`, `SystemTime`). Time is the simulation tick counter. Enforced the same way.
- No dependence on thread scheduling order: parallel work must produce the same result regardless of which thread finishes first.
- No platform-dependent math in the prediction path: avoid `f32` intrinsics whose results differ across CPUs/compilers; prefer the same operation order everywhere and test x86_64 against aarch64 (roadmap T07-026).
- No randomness except from a seeded generator that is part of the replicated state.

If you must break a rule, add a local `#[allow(clippy::disallowed_types)]` with a comment explaining why it cannot affect simulation results, and say so in the PR.
