# Testing strategy

This page says which kinds of tests exist in SS2RS, where they live, how to run them, whether they
need the player's own Serious Sam 2 data, and when CI runs them. It describes the repository as it is
today. Layers that do not exist yet are marked **planned** with the roadmap task that will add them.
Tooling is not invented here: every command below exists in the repo or is a standard cargo command.

## 1. Rules that apply to every layer

- **Plain CI has no game data.** Nothing in CI may need `Serious Sam 2/`. Tests that need the install
  must use `ss2_testkit::require_game_data!()` and skip cleanly without it (see
  `crates/ss2-testkit/src/lib.rs`).
- **No game content in git.** Fixtures are hand-made, license-clean, and at most 256 KiB
  (`testdata/README.md`). Tests prefer builders that write files at run time over committed binaries.
  `cargo xtask guard` enforces this.
- **Deterministic.** Tests must not depend on the wall clock, thread order, or unseeded randomness.
  `ss2-sim`, `ss2-physics` and `ss2-net` also have clippy rules against `HashMap`/`HashSet` and time
  types (`docs/CODING_STANDARDS.md`).
- **Never panic on bad input.** Malformed-input tests assert an error result, not a panic.
- **Headless.** Tests of parsers, UI logic and simulation must construct without a window or GPU.
  GPU tests skip when no adapter exists.
- **Definition of Done** (`docs/roadmap/README.md` section 4): tests included, unit plus data-gated
  where relevant, `cargo fmt`, `clippy -D warnings` and `cargo deny` clean.

## 2. Summary table

| Layer | Status | Lives in | Needs game data | Runs on PR | Runs nightly or weekly |
|---|---|---|---|---|---|
| Unit | exists | `#[cfg(test)]` modules, `*_tests.rs` | no | yes | no |
| Synthetic fixture | exists | `ss2_testkit::fixtures`, `testdata/` | no | yes | no |
| Data-gated | exists | any crate, plus `tools/gro-tool/tests/` | yes (`SS2_DATA_DIR`) | runs and skips | no |
| Golden image | exists (menu only) | `crates/ss2-render/tests/golden_*.rs` | no (needs a GPU adapter) | yes, skips without adapter | no |
| Golden audio | planned, T06-013 | none yet | no | n/a | n/a |
| Sim scenario | planned, T08-017 | none yet | no | n/a | n/a |
| Netsim | planned, T08-017 | none yet | no | n/a | n/a |
| Soak | planned, T12-016 | none yet | yes (its own server) | n/a | n/a |
| Fuzz | T12-007 (targets), T02-030 (format readers) | `fuzz/`, `cargo +nightly fuzz run <target>` | no | n/a | nightly (not yet run in CI) |

Scheduled workflows are listed in section 5. None of them runs the test suite. There is no nightly
test workflow yet.

## 3. Layers

### 3.1 Unit tests (exists)

- **What:** one function, type or module, with no external data.
- **Where:** inside the crate, in a `#[cfg(test)] mod tests` block, or in a sibling file named
  `*_tests.rs` included from the module (examples: `crates/ss2-ui/src/draw_tests.rs`,
  `crates/ss2-core/src/settings_persistence_tests.rs`). Integration-style tests of a crate's public
  API go in `crates/<crate>/tests/`.
- **Naming:** test functions describe the behaviour (`unset_is_not_set`, `path_with_spaces_is_accepted`).
  There is no required prefix.
- **Run:**
  ```
  cargo test --workspace --locked          # everything (what CI runs)
  cargo test -p ss2-vfs                    # one crate
  cargo test -p ss2-vfs path_normalize     # one test by name filter
  ```
  `cargo nextest run` also works if installed (`docs/DEV_TOOLS.md`); it is optional.
- **Game data:** no.
- **When:** every pull request (`test` job in `.github/workflows/ci.yml`, on `ubuntu-latest` and
  `windows-latest`). macOS runs only on pushes to `main` and manual dispatch, to save minutes.

### 3.2 Synthetic-fixture tests (exists)

- **What:** tests of parsers, containers and loaders that run on inputs built in the test itself or
  stored as small hand-made files. They exercise the same code paths as data-gated tests without the
  real install.
- **Where and how:**
  - `ss2_testkit::fixtures::ZipBuilder` and `write_minimal_zip` write a stored-only ZIP, the container
    format of `.gro` archives. Used by `crates/ss2-vfs/src/archive.rs` and `install.rs`.
  - `ss2_testkit::fixtures::ctsemeta::MetaBuilder` writes synthetic CTSEMETA format-8 files from the
    clean-room specs in `docs/re/`. It refuses to write a file that breaks an invariant a reader
    checks. A reader test can then corrupt the bytes to build a malformed case.
  - `testdata_dir()` and `testdata_path()` locate `testdata/` from any crate. Committed files need a
    one-line provenance note in `testdata/README.md`.
- **Naming:** same as unit tests; put a fixture name in the test name when it matters.
- **Run:** same as unit tests (`cargo test --workspace --locked`).
- **Game data:** no. This is the main way parser code is tested on plain CI.
- **When:** every pull request. Malformed-input coverage is also the target of the fuzz layer (3.9).

### 3.3 Data-gated tests (exists)

- **What:** tests that read the player's own install, for example mounting all archives or checking a
  real file against a spec.
- **Where:** `crates/ss2-vfs/src/install.rs` and `tools/gro-tool/tests/cli.rs` today.
- **How they are written:** the first line of the test is
  ```rust
  let dir = ss2_testkit::require_game_data!();
  ```
  The macro returns the directory from `SS2_DATA_DIR`. Without it, or if the folder has no `.gro`
  archive directly inside, it prints `SKIPPED (no game data): <reason>` to stderr and returns.
  The test then counts as passed. Mark the test with a doc comment that says it is data-gated, as
  `tools/gro-tool/tests/cli.rs` does; there is no naming prefix.
- **Run:**
  ```
  SS2_DATA_DIR="C:\path\to\folder\with\All_PC_01.gro" cargo test --workspace --locked
  ```
  Point the variable at the folder that contains the `.gro` files, not at the `Serious Sam 2` parent
  folder. The `parent_of_install_is_rejected` unit test documents this.
- **Game data:** yes. Never commit the data or any output derived from it.
- **When:** pull requests run these tests, but hosted runners have no game data, so they skip. Real
  coverage needs a machine with a legal copy. The planned self-hosted runner design (where data never
  leaves the machine and only pass/fail is reported) is **planned, see T12-002**; the design is in [DATA_GATED_CI.md](DATA_GATED_CI.md).
- **Scripts that read the install** (not cargo tests): the Python checks in `tools/re/` take the
  install path as an argument and read `.gro` files read-only, to re-check numbers quoted in
  `docs/re/`. Example:
  ```
  python -I tools/re/validate_ctsemeta.py "C:\path\to\Serious Sam 2"
  ```
  They are not run in CI.

### 3.4 Golden-image tests (exists, menu only)

- **What:** renders a scene offscreen and compares the RGBA result against a stored PNG with a mean
  absolute difference tolerance.
- **Where:** `crates/ss2-render/tests/golden_menu.rs`. Goldens are in `testdata/golden/`
  (`menu_main_1280x720.png`, `menu_options_1280x720.png`). The comparison helper is
  `ss2_render::mean_abs_diff` (`crates/ss2-render/src/image_diff.rs`).
- **Naming:** `<screen>_matches_golden` for whole-image comparisons; `<behaviour>` for pixel checks
  (for example `focus_highlight_is_orange_at_first_button`).
- **Run:**
  ```
  cargo test -p ss2-render --test golden_menu
  UPDATE_GOLDEN=1 cargo test -p ss2-render --test golden_menu   # rewrite goldens; review the diff
  cargo xtask golden update    # the same for every golden test target, then a review report
  cargo xtask golden report    # only report how the goldens differ from git HEAD
  ```
  The report (T11-015) lists each added, modified and deleted golden file with, for images, the
  size change or the mean and maximum channel difference and the share of changed pixels; with
  `--fail-on-change` the command exits 1 when anything differs. Restore the goldens you do not
  accept with `git checkout -- testdata/golden`.
- **Game data:** no. The menu is placeholder art, not game content.
- **Adapter:** the test asks `Renderer::new_offscreen` for a GPU adapter. Without one it prints
  `SKIPPED (no GPU adapter): ...` and passes. Whether the hosted CI runners expose an adapter is not
  verified in this document; check the test output in a CI run before relying on it.
- **When:** pull requests, through the workspace test job. Any change to a golden is a visible binary
  diff in review; say why in the PR.
- **Planned:** a headless harness with software adapters in CI and tolerance diffing (**planned, see
  T04-021**); UI golden images for each screen at three resolutions and two aspect ratios
  (**planned, see T10-015**).

### 3.5 Golden-audio tests (planned, T06-013)

- **What (planned):** run the mixer offline into a deterministic WAV buffer and compare it with a
  stored reference within a tolerance.
- **Where:** none yet. `crates/ss2-audio-api` has a headless null device, which is a stub, not a golden test.
- **Game data:** to be decided with T06-013; synthetic references are preferred.
- **When:** pull requests, once it exists.

### 3.6 Sim-scenario tests (planned, T08-017)

- **What (planned):** scripted inputs run through the simulation for a fixed number of ticks; the
  resulting state is compared with an expected trace or hash. Two runs must match exactly.
- **Where:** none yet. `ss2-sim`, `ss2-physics` and `ss2-net` contain no tests today. Replay recording
  for bug reports is T08-025 (`P2`).
- **Game data:** not for synthetic scenarios; scenarios that load a real level will be data-gated.
- **When:** pull requests, once the runner exists.

### 3.7 Netsim tests (planned, T08-017)

- **What (planned):** a network conditioner (latency, jitter, loss, reordering, bandwidth cap), usable
  in-process and at socket level. The M4 exit criterion is prediction smooth at 100 ms and 2 % loss
  (`docs/roadmap/README.md`, section 5).
- **Where:** none yet. The netsim CLI front-end is T11-010 (`P2`).
- **Game data:** no. **When:** pull requests for unit-level use; long runs nightly, once they exist.

### 3.8 Soak tests (planned, T12-016)

- **What (planned):** long co-op sessions driven by bots against a dedicated server, nightly, with
  leak and desync detectors.
- **Where:** none yet. It needs a dedicated server and a farm (`T12-016`, `P2`).
- **Game data:** the bot sessions need a real level, so the soak farm will be data-gated and run on
  a machine that has the data (see T12-002).
- **When:** planned nightly. Not on pull requests.

### 3.9 Fuzz tests (T12-007; the game-format targets are T02-030)

- **What:** `cargo-fuzz` targets for the parsers and loaders that exist today (console lines, launch options, settings and
  key bindings files, virtual paths, `.gro` archives, the install checker, archive names), listed in `docs/FUZZING.md`.
  A nightly workflow also runs Miri on three pure crates. The readers of the game formats (T02) get their targets with
  T02-030.
- **Where:** `fuzz/` (a separate `cargo-fuzz` project, `fuzz/fuzz_targets/*.rs`) and `.github/workflows/nightly.yml`.
  `cargo test -p xtask` fails when the target lists of the folder, the manifest, the workflow and the doc differ.
- **Tooling:** `cargo-fuzz` and nightly Rust; it works on Windows (MSVC) when the AddressSanitizer runtime is on the `PATH`,
  see `docs/FUZZING.md`. Miri is optional for contributors.
- **Game data:** no; synthetic inputs by design (never a real game file in the corpus). **When:** nightly in CI, not on
  pull requests. **Status:** the targets build and were run locally for 30 to 60 seconds each without a finding in the
  code; the nightly workflow has **not run in CI yet** (the Actions billing block).

## 4. Other checks that run on every pull request

These are not test layers, but a PR must pass them. Source: `.github/workflows/ci.yml` unless noted.

| Job | What it checks | Command |
|---|---|---|
| `test` | workspace tests, Windows and Linux (macOS on `main` only) | `cargo test --workspace --locked` |
| `lint` | formatting and clippy with warnings denied | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `deny` | licenses and advisories | `cargo deny check` |
| `dco` | every commit has `Signed-off-by` | `git commit -s` |
| `guard` | no proprietary content tracked; roadmap claims consistent | `cargo xtask guard`; `cargo xtask check-claims` |
| `secrets` | secret scan of the PR's commits | gitleaks action |
| `docs` (`docs.yml`, when `docs/`, `crates/`, `Cargo.*` or the link checker change) | mdBook builds, internal links resolve, rustdoc builds with `-D warnings` | `mdbook build docs`; `python3 .github/scripts/check_book_links.py target/book`; `cargo doc --workspace --no-deps --locked` |
| `release` (`release.yml`) | packaging still works, when `release.yml`, `Cargo.toml`, `Cargo.lock` or `release.toml` change | `cargo build --profile dist --locked -p ss2-app -p ss2-server` |

Benchmarks are compiled by `clippy --all-targets` but not timed in CI. Timing runs are manual; see
`docs/PERF_BUDGETS.md`.

## 5. Scheduled and nightly jobs

| Workflow | When (UTC) | What | Blocking |
|---|---|---|---|
| `audit.yml` | Mondays 05:00 | `cargo deny check advisories` | owner is notified on failure |
| `portability.yml` | daily 03:00 | `cargo check --target aarch64-linux-android` for portable crates | no (`continue-on-error`) |
| `claims.yml` | Mondays 06:00 | `cargo xtask check-claims`; stale-claim report | no |

| `nightly.yml` | daily 03:17 | `fuzz` (8 targets, 5 minutes each) and `miri` (three pure crates); see `docs/FUZZING.md` | no |

The nightly fuzz and Miri jobs exist (T12-007) but have **not run in CI yet** (the Actions billing block); the soak layer is
planned under T12-016 and the format-reader fuzz targets under T02-030.

## 6. Adding a test

- **Unit:** same file, in `#[cfg(test)] mod tests`, with no files on disk.
- **Synthetic fixture:** build the input with a `ss2_testkit::fixtures` builder. Commit a file only if no
  builder can produce it, and add its provenance line to `testdata/README.md`.
- **Data-gated:** start with `let dir = ss2_testkit::require_game_data!();`, assert only facts stated in
  `docs/re/`, and never print or save game content.
- **Golden image:** generate the PNG with `cargo xtask golden update` (or `UPDATE_GOLDEN=1`), look at it, and explain the change in the PR.
- **Fuzz, scenario, netsim or soak:** no tooling exists yet. Take the matching `T` task (section 3)
  rather than adding it inside an unrelated PR.
