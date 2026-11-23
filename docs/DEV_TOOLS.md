# Developer Tools

The tools every contributor is expected to have, by role. Keep this file the single source of truth:
changing a required tool means a PR to this file (and to CI if CI uses it). Nothing here is vendored
in the repo. Run `cargo xtask doctor` to check your machine against this list (`--role T06` also requires that track's tools; it exits 1 only when a required tool is missing, everything else is a warning with an install hint).

Legend: **Required** = you can't build/test/contribute without it · **Role** = required for that
track's work · **Optional** = nice to have.

## 1. Everyone (Required)

| Tool | Why | Notes |
|---|---|---|
| **Git** + **GitHub CLI (`gh`)** | version control, PRs, roadmap-sync | `git config core.hooksPath .githooks` once per clone (proprietary-content guard). `gh auth login` once; pushing anything under `.github/workflows/` additionally needs the `workflow` scope: `gh auth refresh -h github.com -s workflow` |
| **Rust via `rustup`** | build | `rust-toolchain.toml` pins the version (currently 1.99.1) and installs `rustfmt`+`clippy` automatically |
| **C toolchain / linker** | Rust linking | Windows: Visual Studio Build Tools (MSVC, "Desktop development with C++"); Linux: `build-essential`; macOS: Xcode Command Line Tools |
| **`cargo-deny`** | license/advisory checks (CI runs it) | `cargo install --locked cargo-deny` |
| **Your own legal copy of Serious Sam 2** | data-gated tests, running the engine | set `SS2_DATA_DIR` to the folder containing `All_PC_01.gro`; never commit it |

## 2. Everyone (Recommended)

| Tool | Why | Install |
|---|---|---|
| `cargo-nextest` | faster, clearer test runs | `cargo install --locked cargo-nextest` |
| `cargo-llvm-cov` | coverage | `cargo install --locked cargo-llvm-cov` |
| `cargo-audit` | RustSec advisories locally | `cargo install --locked cargo-audit` |
| `cargo-machete` / `cargo-udeps` | unused dependencies (`udeps` needs nightly) | `cargo install --locked cargo-machete cargo-udeps` |
| `cargo-semver-checks` | API breaks in published-style crates | `cargo install --locked cargo-semver-checks` |
| `cargo-expand`, `cargo-bloat` | macro/size debugging | `cargo install --locked cargo-expand cargo-bloat` |
| `cargo-release` | version bump and changelog roll-forward when cutting a release ([releasing](RELEASING.md)); maintainers only | `cargo install --locked cargo-release` |
| Python 3.12+ | helper scripts (`.github/scripts/`), Ghidra scripting glue | python.org / package manager |

## 3. By track

| Track | Tool | Why | Required for |
|---|---|---|---|
| **T01 RE/specs** | **Ghidra** (latest stable release from the official NSA GitHub releases) + the JDK that release requires (its README states the version; JDK 17+ is the floor) | analyse `Engine.dll`, `Core.dll`, `Sam2Game.dll`, `GfxOGL.dll` | Role |
| T01 | **ImHex** (or any hex editor with pattern language) | inspect/annotate formats next to `meta-dump` | Role |
| T01 | Debugger of choice (x64dbg/WinDbg) | runtime checks, instrumentation harness (T11-008) | Optional |
| **T02 data** | **`cargo-fuzz`** (needs nightly; works on Windows with the ASan runtime on the `PATH`, see `docs/FUZZING.md`) | parser fuzzing (T12-007, T02-030) | Role |
| T02 | `cargo-mutants` | test-quality checks on parsers | Optional |
| **T03 platform** | Android NDK + `cargo-ndk`, `rustup target add aarch64-linux-android` | **Phase 2 only** (after M8); the non-blocking portability job uses the target | Phase 2 |
| **T04/T05 render/shaders** | **RenderDoc** | GPU frame capture/debugging | Role (from M2) |
| T04/T05 | Vulkan runtime/SDK tools (`vulkaninfo`), GPU vendor tools (optional) | adapter/limits inspection | Role |
| T05 | `naga-cli` (`cargo install --locked naga-cli`) | validate/convert shaders by hand | Role |
| **T06 audio/video** | **FFmpeg** | inspect/convert `.ogv`/audio when writing and verifying decoders | Role |
| **T07/T08 physics/net** | `puffin_viewer` (T11-011; how to install and use it: `docs/PROFILING.md`) | frame/tick profiling | Optional |
| T08 | A network conditioner (the repo's own netsim, T08-017) | no external tool needed | — |
| **T13 mods** | `ss2-doctor --mod` (built from this repo: `cargo run -p ss2-vfs --bin ss2-doctor -- <install> --mod <Name>`; [Mod validation](MOD_VALIDATION.md)) | lints a mod for missing assets, bad references, version mismatch and conflicts | Optional |
| **T00/T12 infra & docs** | **`mdbook`** (`cargo install --locked mdbook`) | docs site (T00-020) | Role |
| T12 | `cargo-miri` (nightly) | UB checks on any `unsafe` | Optional |

## 4. Rules

1. **Pinned where it matters:** Rust is pinned by `rust-toolchain.toml`; CI uses the same file. Install other
   cargo tools with `--locked`.
2. **No proprietary tooling in the repo:** Ghidra projects, hex-editor templates containing game data, and
   decompiler exports stay on your machine (`.gitignore` covers common cases). Share *prose specs* in `docs/re/`.
3. **New required tool?** PR this file + CI + `cargo xtask doctor` together, explain why existing tools don't do it.
4. **Windows is the primary dev platform** (PC first). Linux and macOS must keep working; Android tooling stays
   Phase 2 and never blocks a PC contributor.
