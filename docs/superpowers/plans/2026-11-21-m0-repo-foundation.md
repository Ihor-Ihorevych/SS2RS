# M0 Repo Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `C:\Games\SS2Rewrite` into a buildable Cargo workspace with CI, safety guards, policy docs and tooling, so many contributors can start Wave 0 of the roadmap in parallel (milestone M0, minus the contract stubs).

**Architecture:** One Cargo workspace at the repo root: stub crates under `crates/` (one per owner area), plus an `xtask` crate for repo tooling (proprietary-content guard, roadmap→GitHub-issues sync). CI runs fmt/clippy/test on Win/Linux/macOS, `cargo deny`, the guard and a DCO check. Policy lives in markdown files at the root and `docs/`.

**Tech Stack:** Rust 1.99.1 (pinned), edition 2024, `thiserror`, `tracing`/`tracing-subscriber`, `clap`, `anyhow`, `regex`, `tempfile`; GitHub Actions; `cargo-deny`; `gh` CLI.

**Spec:** `docs/superpowers/specs/2026-11-21-ss2rs-design.md` (§5 constraints, §8 architecture). Task IDs below (T00-nnn) refer to `docs/roadmap/T00-repo-infra.md`.

**Out of scope (next plan):** contracts T00-014…018 (they need T08-001 and RFC review), T00-019 benchmarks, T00-020 mdBook site, T00-021 release engineering, T00-024/025. Synthetic-fixture *builders* (T00-010) arrive with the first loader task (T02-001), because they need the `zip` crate; only the policy is written here.

## Global Constraints

- Toolchain pinned in `rust-toolchain.toml`: channel `1.99.1`, components `rustfmt`, `clippy`; workspace `rust-version = "1.99"`; `edition = "2024"`.
- License: **Apache-2.0** (repo `LICENSE` already present); every crate sets `license = "Apache-2.0"`, `publish = false`.
- Crate names use the `ss2-` prefix; binaries are `ss2-app` and `ss2-server`.
- `unsafe_code = "deny"` workspace-wide (allowed only in audited modules via explicit `#![allow]` + review).
- Sim-safe crates `ss2-sim`, `ss2-physics`, `ss2-net`: no `HashMap`/`HashSet` (iteration order) and no `Instant`/`SystemTime` (wall clock).
- Never commit proprietary game content: directory `Serious Sam 2/` and extensions `.gro .dll .exe .wld .tex .mdl .bmf .ogv .xmv .chm` (blob limit 5 MiB).
- Contributions carry DCO sign-off (`git commit -s`); PR title/commit starts with the task ID, e.g. `T00-001: …`.
- PC first: CI covers Windows, Linux, macOS. The Android `cargo check` job is **non-blocking** (portability guard only).
- Data-gated tests read `SS2_DATA_DIR` and must skip cleanly when it is absent; plain CI never has game data.
- Work on branch `m0-foundation`; `main` is protected only by CI after Task 8.
- **Claim protocol (`AGENTS.md` §1):** before starting each task, claim the matching roadmap line(s) by hand — set `- [~]`, add `[INPRG @handle · branch `m0-foundation` · date]`, commit `T00-nnn: [INPRG] claim (@handle)` and push to `main` — and remove the tag when the task's work lands. (Task 6's parser and the later T00-028 automation take over once they exist.) Claim commits are the only direct pushes to `main`.

## Review Focus

1. `SS2_DATA_DIR` set to a path with spaces, to the *parent* of the install (e.g. `C:\Games\SS2Rewrite`), or to a folder with no `.gro`: must give a clear skip reason, never panic or run tests against the wrong folder. (Task 3)
2. Guard input quirks: upper-case extensions (`ALL_PC_01.GRO`), backslash paths, paths with spaces (`Serious Sam 2/…`), a proprietary file renamed to `.bin` (caught by size). (Task 4)
3. Logging initialised twice or from many test threads at once must not panic. (Task 2)
4. Roadmap parser: items with a description on the following indented line, lettered sub-IDs (`T09-015a`), `PHASE 2` instead of a P-level, and running the sync twice must create nothing the second time. (Task 6)
5. The sim-safe lint must actually fire in `ss2-sim` while `ss2-core` still may use `HashMap`. (Task 5)

---

### Task 1: Workspace skeleton, lints, license metadata (T00-001, part of T00-002)

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml`, `NOTICE`, `.cargo/config.toml`
- Create: `crates/<name>/Cargo.toml` + `crates/<name>/src/lib.rs` for each lib crate below; `crates/ss2-app/src/main.rs`, `crates/ss2-server/src/main.rs`

**Interfaces:**
- Produces: workspace members `crates/*` (and `xtask` after Task 4); `[workspace.dependencies]` entries `thiserror = "2"`, `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`, `tempfile = "3"`, `clap = { version = "4", features = ["derive"] }`, `anyhow = "1"`, `regex = "1"`; alias `cargo xtask`.
- Lib crates (name — one-line description, owner track): `ss2-core` (ids, errors, logging, time; T00), `ss2-testkit` (data-gated test helpers; T00/T12), `ss2-vfs` (T02), `ss2-meta` (CTSEMETA reader; T02), `ss2-assets` (T02), `ss2-platform` (T03), `ss2-render` (T04), `ss2-shaderxlat` (T05), `ss2-audio` and `ss2-video` (T06), `ss2-physics` and `ss2-world` (T07), `ss2-sim` and `ss2-net` (T08), `ss2-game` (T09), `ss2-ui` (T10), `ss2-script` (T13). `ss2-format-*` crates are created by T02 owners, not now.

- [ ] **Step 1: Create branch** `git switch -c m0-foundation`.
- [ ] **Step 2: Write root `Cargo.toml`** with `[workspace] resolver = "3"`, `members = ["crates/*"]`, `[workspace.package]` (`version = "0.0.0"`, `edition = "2024"`, `license = "Apache-2.0"`, `rust-version = "1.99"`, `repository = "https://github.com/Ihor-Ihorevych/SS2RS"`), the dependency table above, and `[workspace.lints.rust] unsafe_code = "deny"`, `[workspace.lints.clippy] all = { level = "warn", priority = -1 }`.
- [ ] **Step 3: Write `rust-toolchain.toml`** (`channel = "1.99.1"`, `components = ["rustfmt", "clippy"]`, `profile = "minimal"`), `rustfmt.toml` (`edition = "2024"`, `max_width = 100`), `.cargo/config.toml` with `[alias] xtask = "run --quiet --package xtask --"`.
- [ ] **Step 4: Generate crate stubs** with a one-off shell loop (not committed). Each `Cargo.toml` inherits `version/edition/license/rust-version` from the workspace, sets `description`, `publish = false`, `[lints] workspace = true`; each `lib.rs` is a single `//!` doc line naming the crate's purpose and owner track. `ss2-app`/`ss2-server` `main.rs` print `"<name>: not implemented yet"`.
- [ ] **Step 5: `NOTICE`** (project name "SS2RS", "Licensed under Apache-2.0", disclaimer that it is unaffiliated with Croteam/Devolver and contains no Croteam assets) and **`deny.toml`**: start from `cargo deny init`, then allow-list licenses `MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0, Unicode-3.0, CC0-1.0`, `[bans] multiple-versions = "warn"`, `[sources] unknown-registry = "deny"`, `unknown-git = "deny"`.
- [ ] **Step 6: Verify**
  Run: `cargo build --workspace --locked` after a first `cargo build --workspace` (creates `Cargo.lock`), then `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check`.
  Expected: all exit 0; `cargo metadata --no-deps --format-version 1` lists 19 packages (17 libs + 2 bins) all with license `Apache-2.0`.
- [ ] **Step 7: Commit** `git add -A && git commit -s -m "T00-001: Cargo workspace skeleton, lints, deny config"` (`Cargo.lock` is committed).

---

### Task 2: `ss2-core` errors and logging (T00-011)

**Files:**
- Create: `crates/ss2-core/src/error.rs`, `crates/ss2-core/src/logging.rs`
- Modify: `crates/ss2-core/src/lib.rs`, `crates/ss2-core/Cargo.toml` (deps `thiserror`, `tracing`, `tracing-subscriber`)
- Test: unit tests inside both modules

**Interfaces:**
- Produces:
  - `pub enum Error { Io { path: PathBuf, source: std::io::Error }, Invalid { what: &'static str, detail: String }, NotFound(String) }` (`thiserror`), `pub type Result<T> = std::result::Result<T, Error>`, constructors `Error::io(path: impl Into<PathBuf>, source: std::io::Error) -> Error` and `Error::invalid(what: &'static str, detail: impl Into<String>) -> Error`.
  - `pub fn init_logging(default_filter: &str) -> bool` — installs a global `tracing` subscriber (env filter from `SS2_LOG`, falling back to `default_filter`); returns `true` only for the call that installed it, `false` if one already existed. `pub fn init_for_tests()` = `init_logging("warn")` with output captured by the test writer.
  - Re-exports from `lib.rs`: `pub use error::{Error, Result};`, `pub mod logging;`.

- [ ] **Step 1: Write failing tests**
  `error::tests::invalid_displays_what_and_detail`: `Error::invalid("CTSEMETA header", "bad magic").to_string() == "invalid CTSEMETA header: bad magic"`.
  `error::tests::io_error_names_the_path`: message contains `Content/a.gro` and the source text.
  `logging::tests::second_init_returns_false`: first `init_logging("warn")` may be true or false, the second is `false`, neither panics.
  `logging::tests::concurrent_init_installs_exactly_once`: 8 threads call `init_logging`, at most one `true`, none panic.
- [ ] **Step 2: Run** `cargo test -p ss2-core` → Expected: FAIL (items not defined).
- [ ] **Step 3: Implement** both modules with the signatures above; use `tracing_subscriber::fmt().with_env_filter(...).try_init().is_ok()` so double init returns `false` instead of panicking.
- [ ] **Step 4: Run** `cargo test -p ss2-core` and `cargo clippy -p ss2-core --all-targets -- -D warnings` → Expected: PASS.
- [ ] **Step 5: Commit** `git commit -s -am "T00-011: ss2-core error type and logging init"` (add new files first).

---

### Task 3: `ss2-testkit` data-gated tests (T00-009, policy part of T00-010)

**Files:**
- Create: `crates/ss2-testkit/src/lib.rs` (replace stub), `testdata/README.md`
- Modify: `crates/ss2-testkit/Cargo.toml` (dev-dep `tempfile`)
- Test: unit tests in `lib.rs`

**Interfaces:**
- Produces:
  - `pub const DATA_DIR_ENV: &str = "SS2_DATA_DIR"`.
  - `pub enum SkipReason { NotSet, NotADirectory(PathBuf), NoArchives(PathBuf) }` implementing `Display` with an actionable message (e.g. `NoArchives` says "point SS2_DATA_DIR at the folder that contains All_PC_01.gro").
  - `pub fn resolve_game_data_dir(value: Option<std::ffi::OsString>) -> Result<PathBuf, SkipReason>` — pure (no env access): `None`/empty → `NotSet`; not a directory → `NotADirectory`; no file directly inside whose extension is `gro` (case-insensitive) → `NoArchives`.
  - `pub fn game_data_dir() -> Result<PathBuf, SkipReason>` = `resolve_game_data_dir(std::env::var_os(DATA_DIR_ENV))`.
  - `#[macro_export] macro_rules! require_game_data` — expands to the `PathBuf` on success; otherwise prints `SKIPPED (no game data): <reason>` to stderr and `return`s from the calling test.

- [ ] **Step 1: Write failing tests:**
  `unset_is_not_set` (`None` and `Some("")` → `NotSet`); `missing_dir_is_not_a_directory`; `empty_dir_has_no_archives`; `parent_of_install_is_rejected` (temp dir containing only a subfolder `Serious Sam 2/` holding `All_PC_01.gro` → `NoArchives`); `dir_with_gro_is_accepted` (temp dir with `All_PC_01.GRO` → `Ok`); `path_with_spaces_is_accepted` (temp dir named `Serious Sam 2` containing `a.gro`); `macro_skips_or_runs` (calls `require_game_data!()`, and if it gets a path asserts `is_dir()`).
- [ ] **Step 2: Run** `cargo test -p ss2-testkit` → Expected: FAIL.
- [ ] **Step 3: Implement** as specified. `testdata/README.md`: states that only hand-made, license-clean, ≤256 KiB fixtures may live here, never excerpts of Croteam files, and that builders (`write_minimal_zip`, minimal CTSEMETA writer) arrive with T02-001/T02-006.
- [ ] **Step 4: Run** `cargo test -p ss2-testkit` and clippy `-D warnings` → Expected: PASS (with and without `SS2_DATA_DIR` set; try `SS2_DATA_DIR="C:\Games\SS2Rewrite\Serious Sam 2"` locally and confirm `macro_skips_or_runs` takes the run path).
- [ ] **Step 5: Commit** `T00-009: ss2-testkit data-gated test helpers`.

---

### Task 4: `xtask guard` — proprietary-content guard + pre-commit hook (T00-006)

**Files:**
- Create: `xtask/Cargo.toml`, `xtask/src/main.rs`, `xtask/src/lib.rs`, `xtask/src/guard.rs`, `.githooks/pre-commit`
- Modify: root `Cargo.toml` (`members = ["crates/*", "xtask"]`)
- Test: unit tests in `guard.rs`

**Interfaces:**
- Produces (`xtask::guard`):
  - `pub const MAX_BYTES: u64 = 5 * 1024 * 1024;`, `pub const TESTDATA_MAX_BYTES: u64 = 256 * 1024;`
  - `pub enum Reason { ProprietaryDir, ForbiddenExtension(String), TooLarge { bytes: u64, limit: u64 } }`, `pub struct Violation { pub path: String, pub reason: Reason }` (both `Debug`, `PartialEq`).
  - `pub fn check_path(path: &str, size_bytes: u64) -> Option<Violation>` — normalizes `\` to `/`, lower-cases for comparison. Rules: any path component equal to `Serious Sam 2` → `ProprietaryDir`; extension in `{gro, dll, exe, wld, tex, mdl, bmf, ogv, xmv, chm}` → `ForbiddenExtension`; size > `MAX_BYTES` → `TooLarge`. Exemption: paths under `testdata/` may use `tex/mdl/bmf/wld/ogv/xmv` extensions (hand-made fixtures) but never `gro/dll/exe/chm`, and are limited to `TESTDATA_MAX_BYTES`.
  - `pub fn run(mode: Mode) -> anyhow::Result<Vec<Violation>>` with `pub enum Mode { AllTracked, Staged }`; reads `git ls-files -z` / `git diff --cached --name-only --diff-filter=AM -z`, sizes via `std::fs::metadata`.
- CLI: `cargo xtask guard [--staged]`; prints each violation and exits 1 if any.

- [ ] **Step 1: Write failing tests:** `forbids_gro_dll_exe_case_insensitively` (`ALL_PC_01.GRO`, `Bin/Engine.DLL`); `normalizes_backslashes` (`docs\x.gro`); `flags_serious_sam_dir_with_spaces`; `flags_renamed_blob_by_size` (`assets/x.bin`, 6 MiB → `TooLarge`); `allows_small_text_and_rs_files`; `testdata_allows_tex_but_not_gro` and `testdata_size_limit_is_256_kib`.
- [ ] **Step 2: Run** `cargo test -p xtask` → Expected: FAIL.
- [ ] **Step 3: Implement** `check_path`, `run`, and the `clap` CLI (`guard` subcommand, `--staged` flag) in `main.rs`/`lib.rs`.
- [ ] **Step 4: Run** `cargo test -p xtask` → Expected: PASS; then `cargo xtask guard` on the repo → Expected: exit 0.
- [ ] **Step 5: `.githooks/pre-commit`** (POSIX sh, executable): `exec cargo xtask guard --staged`. Document `git config core.hooksPath .githooks` (done in Task 7's CONTRIBUTING).
- [ ] **Step 6: Negative test of the hook:** `git config core.hooksPath .githooks`, create `git-hook-test.gro` (2 bytes), `git add -f` it, `git commit -s -m x` → Expected: commit rejected with the violation printed; then `git reset -q git-hook-test.gro && rm git-hook-test.gro`.
- [ ] **Step 7: Commit** `T00-006: xtask guard and pre-commit hook against proprietary content`.

---

### Task 5: Sim-safe lints (code part of T00-012)

**Files:**
- Create: `crates/ss2-sim/clippy.toml`, `crates/ss2-physics/clippy.toml`, `crates/ss2-net/clippy.toml` (identical content)

**Interfaces:**
- Produces: in each file `disallowed-types = [...]` listing `std::collections::HashMap`, `std::collections::HashSet`, `std::time::Instant`, `std::time::SystemTime`, each with a `reason` pointing at `docs/CODING_STANDARDS.md#sim-safe`.

- [ ] **Step 1: Write the three `clippy.toml` files.**
- [ ] **Step 2: Prove the lint fires:** temporarily add `use std::collections::HashMap;` plus a `fn _x(_: HashMap<u8, u8>) {}` to `crates/ss2-sim/src/lib.rs`; run `cargo clippy -p ss2-sim -- -D warnings` → Expected: FAIL naming `disallowed_types`. Do the same in `ss2-core` → Expected: PASS (no restriction there).
- [ ] **Step 3: Revert the temporary lines** and confirm `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] **Step 4: Commit** `T00-012: forbid HashMap/Instant in sim-safe crates via clippy`.

---

### Task 6: `xtask roadmap-sync` — roadmap → GitHub issues (T00-008)

**Files:**
- Create: `xtask/src/roadmap.rs`
- Modify: `xtask/src/lib.rs`, `xtask/src/main.rs` (new subcommand)
- Test: unit tests in `roadmap.rs` plus one test reading the real `docs/roadmap/T*.md`

**Interfaces:**
- Produces (`xtask::roadmap`):
  - `pub struct RoadmapTask { pub id: String, pub title: String, pub size: Option<String>, pub prio: Option<String>, pub phase2: bool, pub good_first: bool, pub spike: bool, pub spec: bool, pub in_progress: bool, pub deps: Vec<String>, pub body: String, pub source: String }`.
  - `pub fn parse_roadmap(text: &str, source_file: &str) -> Vec<RoadmapTask>` — an item starts at a line matching `^- \[[ x~]\] \*\*(T\d\d-\d{3}[a-z]?)\*\*`; its body is that line plus following lines that start with two spaces. `title` is the first non-empty ` · `-separated segment after the ID with markdown bold/backticks removed; `size` is the first backticked token matching `^(S|M|L|XL)$`; `prio` the first matching `^P[0-3]$`; `phase2` when a `PHASE 2` token exists; `good_first` when the line contains 🟢; `spike`/`spec` when it contains `SPIKE`/`SPEC`; `in_progress` when the box is `[~]` or the line contains `[INPRG` (the `[INPRG …]` tag sits right after the ID and must not leak into `title`); `deps` are IDs after `Deps:`.
  - `impl RoadmapTask { pub fn issue_title(&self) -> String /* "T02-006: <title>" */; pub fn labels(&self) -> Vec<String> /* track:T02, size:M, prio:P0 | phase-2, good-first, spike, spec, in-progress */ }`.
  - `pub fn tasks_to_create<'a>(tasks: &'a [RoadmapTask], existing_issue_titles: &[String]) -> Vec<&'a RoadmapTask>` — excludes tasks whose ID prefix (`"<id>:"`) already starts an existing title.
- CLI: `cargo xtask roadmap-sync [--apply]`. Default is **dry run** (prints counts per track and the first 5 titles). `--apply` runs `gh label create` for missing labels and `gh issue create --title … --body … --label …` per missing task, reading existing issues from `gh issue list --state all --limit 2000 --json title`.

- [ ] **Step 1: Write failing tests:** `parses_single_line_item` (T02-006 style line → id/title/size `M`/prio `P0`/deps `T01-004`); `parses_item_with_indented_description` (T00-001 style: description on next line goes into `body`, title still `Workspace skeleton`); `phase2_has_no_prio_label` (labels contain `phase-2`, none starting `prio:`); `lettered_sub_ids_are_parsed` (line mentioning `T09-015a…` inside text does not create a task, only `**T09-015**` does); `issue_title_and_labels`; `claimed_item_is_in_progress` (a `- [~] **T02-006** [INPRG @a · branch `x` · 2026-11-22] · CTSEMETA reader · `M` · `P0`` line → `in_progress`, title `CTSEMETA reader`, label `in-progress`); `second_run_creates_nothing` (feed `tasks_to_create` the titles of everything it returned first → empty); `parses_all_real_roadmap_files` (reads every `docs/roadmap/T*.md` relative to `CARGO_MANIFEST_DIR/..`, asserts ≥ 370 tasks and all IDs unique).
- [ ] **Step 2: Run** `cargo test -p xtask roadmap` → Expected: FAIL.
- [ ] **Step 3: Implement** parser, label/title helpers, `tasks_to_create`, and the `gh`-driven CLI (`std::process::Command`, no extra crates beyond `regex`/`anyhow`).
- [ ] **Step 4: Run** tests → PASS; then `cargo xtask roadmap-sync` (dry run) → Expected: prints 370 tasks and 0 existing; **do not run `--apply` here** — creating ~370 issues in the user's repo needs their explicit go-ahead (ask at Task 8).
- [ ] **Step 5: Commit** `T00-008: xtask roadmap-sync (dry-run by default)`.

---

### Task 7: Policy and contributor documents (T00-005, 007, 012 doc, 013, 022, 023)

**Files (all new):**
`README.md`, `CLEANROOM.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `.github/CODEOWNERS`, `.github/PULL_REQUEST_TEMPLATE.md` (includes: "claim commit `[INPRG]` is on `main`; this PR ticks `[x]` and removes the tag"), `.github/ISSUE_TEMPLATE/task.md`, `bug.md`, `parity.md`, `docs/rfcs/0000-template.md`, `docs/CODING_STANDARDS.md`, `docs/re/README.md`
(`docs/DEV_TOOLS.md` already exists — T00-026, written with the roadmap update; this task only links to it.)

**Interfaces / required content (the implementer writes the prose):**
- `README.md`: what the project is, status "pre-alpha, nothing playable", you need your own legal copy of Serious Sam 2, trademark/affiliation disclaimer (no Croteam assets, not endorsed), links to design doc, roadmap, CONTRIBUTING, `docs/DEV_TOOLS.md`.
- `CLEANROOM.md`: two roles (**spec author** reads binaries/Ghidra and writes `docs/re/*.md`; **implementer** writes Rust from specs and public docs only); allowed sources (specs, our own measurements, public docs, SS1 for concepts only); forbidden (copying SS1 or decompiler output, committing game files or Ghidra projects); PR provenance statement wording; what to do when unsure (stop, open an issue).
- `CONTRIBUTING.md`: setup (install the tools listed in `docs/DEV_TOOLS.md` — link it prominently, state that everyone uses that toolset; `rustup` picks the pinned toolchain; `git config core.hooksPath .githooks`), branch naming `T02-006-short-name`, PR title `T02-006: …`, `git commit -s` DCO explanation, roadmap §3 workflow rules and §4 Definition of Done verbatim, label set (`track:Txx`, `size:*`, `prio:*`, `phase-2`, `spike`, `spec`, `good-first`), how to claim a task, how to run data-gated tests (`SS2_DATA_DIR`).
- `CODE_OF_CONDUCT.md`: Contributor Covenant 2.1 text from the official source; **enforcement contact = GitHub private reporting / issue to maintainers; do not put any personal email in the repo** (owner may add one later).
- `SECURITY.md`: report via GitHub private vulnerability reporting; scope note (parsers handle untrusted game/mod files; netcode handles untrusted peers).
- `CODEOWNERS`: `* @Ihor-Ihorevych` and `/docs/re/ @Ihor-Ihorevych` (placeholders until track owners exist; comment says so).
- `PULL_REQUEST_TEMPLATE.md`: task ID line, summary, checkboxes — DoD items, "no proprietary files", "provenance: I wrote this from specs/public docs, not from decompiled output or SS1 code", DCO sign-off.
- Issue templates: `task` (ID, deps, done-when), `bug` (platform, version, repro, game version/patch level), `parity` (what differs from original, evidence as numbers/text).
- `docs/rfcs/0000-template.md`: sections Summary, Motivation, Detailed design, Alternatives, Compatibility/migration, Open questions; header lists number, authors, status, owners to approve (crate owner + 1), review SLA 3 working days.
- `docs/CODING_STANDARDS.md`: sections *Unsafe*, *Errors* (use `ss2_core::{Error, Result}`, never panic on bad input), *Logging* (`tracing`, targets = crate names), *Naming/layout*, and an anchored section `## Sim-safe` (id `sim-safe`) listing the lint rules from Task 5 plus "no thread-order dependence, no platform-dependent math in the prediction path".
- `docs/re/README.md`: the spec template from roadmap T01-001 (summary, evidence, layout, invariants, open questions, confidence, test vectors, reviewer).

- [ ] **Step 1: Write all files** per the list above.
- [ ] **Step 2: Verify presence and links:** run `for f in README.md CLEANROOM.md CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md .github/CODEOWNERS .github/PULL_REQUEST_TEMPLATE.md docs/rfcs/0000-template.md docs/CODING_STANDARDS.md docs/re/README.md; do test -s "$f" || echo MISSING $f; done` → Expected: no output; and `grep -c "sim-safe" docs/CODING_STANDARDS.md` ≥ 1; `grep -rn "@gmail" .` → Expected: no matches.
- [ ] **Step 3: Run** `cargo xtask guard` → Expected: exit 0.
- [ ] **Step 4: Commit** `T00-005: clean-room policy, contributor docs and templates (T00-007/013/022/023)`.

---

### Task 8: CI, DCO check and M0 exit (T00-003, T00-002 deny job, T00-004)

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/portability.yml`
- Modify: `docs/roadmap/T00-repo-infra.md` (tick finished boxes)

**Interfaces:**
- Produces workflows:
  - `ci.yml` — triggers `push` to `main` and `pull_request`. Jobs: `test` (matrix `ubuntu-latest`, `windows-latest`, `macos-latest`: checkout, `actions-rust-lang/setup-rust-toolchain@v1` (reads `rust-toolchain.toml`), `cargo test --workspace --locked`); `lint` (ubuntu: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`); `deny` (`EmbarkStudios/cargo-deny-action@v2`); `guard` (`cargo xtask guard`); `dco` (PR only, `fetch-depth: 0`: for each non-merge commit in `base_sha..head_sha` require a `Signed-off-by:` line, else fail naming the SHA); `secrets` (`gitleaks/gitleaks-action@v2`).
  - `portability.yml` — nightly cron + manual: `rustup target add aarch64-linux-android` then `cargo check --target aarch64-linux-android -p ss2-core -p ss2-vfs -p ss2-meta -p ss2-assets -p ss2-sim -p ss2-net`; the job sets `continue-on-error: true`.

- [ ] **Step 1: Write both workflows** exactly as above (action versions are majors; if a newer major exists when you implement, use the current one).
- [ ] **Step 2: Local pre-flight:** `cargo test --workspace --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo deny check`, `cargo xtask guard` → Expected: all exit 0.
- [ ] **Step 3: Push the branch** `git push -u origin m0-foundation`, open a PR with `gh pr create --base main --title "M0: repo foundation (T00-001…013)" --body "…"` and watch `gh pr checks --watch` → Expected: all required jobs green on three OSes. Fix failures in follow-up commits (each signed off).
- [ ] **Step 4: DCO negative test (throwaway branch `dco-test`):** commit without `-s`, open a draft PR, confirm the `dco` job fails, then close the PR and delete the branch.
- [ ] **Step 5: Ask the user** whether to run `cargo xtask roadmap-sync --apply` (creates ~370 issues + labels in `Ihor-Ihorevych/SS2RS`) and whether to enable branch protection requiring `test`, `lint`, `deny`, `guard`, `dco`. Warn that the `[INPRG]` claim protocol needs direct pushes of one-line roadmap edits to `main`, so protection must allow those (ruleset bypass for maintainers/agent accounts) or claims must move to a different mechanism. Do none of this without a yes.
- [ ] **Step 6: Tick the completed T00 boxes** (001–003, 005–009, 011–013, 022, 023 and the deny part of 002; leave 004 as "nightly job exists") in `docs/roadmap/T00-repo-infra.md`, commit `T00: tick M0 tasks`, push, and after green CI merge the PR (squash disabled: keep sign-offs; use merge commit).
- [ ] **Step 7: M0 exit check:** on `main`, `cargo build --workspace --locked`, `cargo test --workspace --locked` pass and the latest `ci` run on `main` is green → report to the user that M0 minus contracts is done and the next plan is the contracts (T00-014…018).

---

## Self-review notes

- **Spec coverage:** T00-001 (Task 1), 002 (1, 8), 003 (8), 004 (8), 005 (7), 006 (4), 007 (7), 008 (6), 009 (3), 010 policy (3), 011 (2), 012 (5, 7), 013 (7), 022/023 (7). Not covered on purpose: 014–021, 024, 025 (see "Out of scope").
- **Type consistency:** `Error`/`Result`/`init_logging` (Task 2) are the names CODING_STANDARDS (Task 7) refers to; `check_path`/`Mode` (Task 4) are used by the hook (Task 4) and CI (Task 8); `RoadmapTask`/`tasks_to_create` (Task 6) match the CLI description.
- **Deliberate deviations from the roadmap text:** only Apache-2.0 (user's repo choice) instead of dual; T00-004 is `P2` non-blocking; T00-010 builders deferred to T02.
