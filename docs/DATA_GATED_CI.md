# Data-gated CI design

Status: **design only, not implemented.** Roadmap task T12-002 (`docs/roadmap/T12-qa-release.md`, size M,
P0, no dependencies). Tracking issue #338. Nothing here creates a workflow, runner, label or script.
**PROPOSED** marks decisions for maintainers. Until they are made, the interim process in section 6 applies.

## 1. The problem

### 1.1 Tests that need the install

Rust tests that need the install call `ss2_testkit::require_game_data!()` (`crates/ss2-testkit/src/lib.rs`).
Spec tools under `tools/re/` read the install from a command-line argument or `SS2_DATA_DIR` (section 5 lists both).

Skip behaviour (`crates/ss2-testkit/src/lib.rs`; `docs/TESTING.md` section 3.3):

- `game_data_dir()` reads `SS2_DATA_DIR`. Unset or empty gives `NotSet`. Not a directory gives `NotADirectory`.
  A directory with no `*.gro` file directly inside gives `NoArchives`.
- On any of these the macro prints `SKIPPED (no game data): <reason>` to stderr and returns from the test.
  The test ends normally, so libtest counts it as **passed**. Cargo's summary does not show skips.
- libtest hides the output of passing tests, so the marker is visible only with `-- --nocapture` or on failure.
- When `SS2_DATA_DIR` is set but wrong, the reason includes the directory path. On a contributor's machine that
  path can contain a user name.

### 1.2 Why hosted runners cannot run them

- The game content is proprietary. `CLEANROOM.md` (Forbidden) bans committing game files; `cargo xtask guard`
  enforces the extension list in `docs/COMPLIANCE.md` section 6.1.
- `docs/PUBLISHING.md` section 3, gate 1, requires "no game content and no derived data". `docs/TESTING.md`
  section 1 says CI may not need the install; section 6 says data-gated tests never print or save game content.
- A GitHub-hosted runner is not controlled by the project, and GitHub stores its job logs and artifacts. Copying
  the install there, or uploading output derived from it, breaks these rules.
- The `test` job in `.github/workflows/ci.yml` runs on hosted runners with no data, so data-gated tests skip there.
  No nightly test run exists (`docs/TESTING.md` section 5). Real coverage today is manual, on maintainers' machines.

### 1.3 What runs today without data

- `ci.yml`: `cargo test --workspace --locked` (data-gated tests skip), `lint`, `deny`, `dco`, `guard`
  (`cargo xtask guard` and `check-claims`) and `secrets`.
- Synthetic fixtures (`ss2_testkit::fixtures`, `testdata/`) cover parsers without the install.
- The golden-image menu test (`crates/ss2-render/tests/golden_menu.rs`) skips without a GPU adapter.
- No CI job reads the install or runs a `tools/re/` script.

## 2. Design of the data-gated job

### 2.1 Workflow and triggers (PROPOSED: `.github/workflows/data-gated.yml`, not created)

- `push` to `main`: tests merged code.
- `schedule`: nightly, time TBD (UTC).
- `workflow_dispatch`: maintainers with write access.
- `pull_request`, type `labeled` only, when the label `run-data-tests` is applied. It never runs on
  `synchronize`, so a later push runs only after a maintainer re-applies the label. It checks out
  `github.event.pull_request.head.sha`, the commit the maintainer saw.

```yaml
# PROPOSED: .github/workflows/data-gated.yml (not created)
name: data-gated
on:
  push:
    branches: [main]
  schedule:
    - cron: "TBD"
  workflow_dispatch:
  pull_request:
    types: [labeled]                  # not synchronize, not pull_request_target
permissions:
  contents: read                      # the only permission; no secrets referenced
concurrency:
  group: data-gated
  cancel-in-progress: false
jobs:
  data-tests:
    if: github.event_name != 'pull_request' || github.event.label.name == 'run-data-tests'
    runs-on: [self-hosted, ss2-data]
    timeout-minutes: TBD
    environment: data-gated           # PROPOSED, if the plan allows: a required reviewer approves each run
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - name: Workspace tests (SS2_DATA_DIR comes from the runner service)
        run: |
          set +e
          cargo test --workspace --locked -- --nocapture > "$RUNNER_TEMP/test.log" 2>&1
          echo $? > "$RUNNER_TEMP/test.exit"
      - name: Publish counts and test names only
        if: always()
        run: python3 .github/scripts/data_gated_summary.py "$RUNNER_TEMP/test.log" >> "$GITHUB_STEP_SUMMARY"
      - name: Fail if the tests failed
        run: test "$(cat "$RUNNER_TEMP/test.exit")" -eq 0
      - name: Delete the raw log
        if: always()
        run: rm -f "$RUNNER_TEMP/test.log" "$RUNNER_TEMP/test.exit"
```

`data_gated_summary.py` would be a new script (none exists). Verifier steps follow the same log rules.

### 2.2 Runner and data location (PROPOSED)

- Labels `self-hosted` and `ss2-data`; one job at a time per runner.
- The install's `.gro` archives sit in a fixed folder on the runner (path TBD), with nothing else
  (`docs/user/install.md`). The runner account has read-only access to it.
- `SS2_DATA_DIR` is set in the **runner service's** environment. It never appears in the workflow, a step `env:`,
  a repository variable or an artifact, and no step prints it.

### 2.3 What the job runs

1. `cargo test --workspace --locked`, as in the sketch. With data, the three data-gated tests run instead of skipping.
2. A nightly subset of `tools/re/` verifiers (PROPOSED). Runtimes come from each script's usage text or `tools/re/README.md`.

| Script (`tools/re/`) | Runtime stated | Source |
|---|---|---|
| `weapons_verify.py`, `enemies_verify.py` | about 1 minute | docstring |
| `camera_verify.py` | about 3 minutes | docstring |
| `audio_verify.py`, `movie_verify.py`, `l10n_verify.py` | about a minute | `README.md` |
| `mdl_verify.py`, `ans_verify.py`, `mtr_verify.py` | about 5 minutes each | `README.md` |
| `wld_verify.py` | about 20 minutes | `README.md` |
| `validate_ctsemeta.py` | not stated (TBD) | none |

The other world scripts are stated at 10 to 20 minutes each. Running all of them (about 20 scripts) takes several
hours, so PROPOSED: the nightly runs the short list and the full sweep runs weekly or on demand.

**Pass rule is open.** Exit status is not uniform. `validate_ctsemeta.py` tallies assertion failures and prints
them (line 58); whether it then exits non-zero was not checked. Each verifier needs a defined pass rule (section 8).

### 2.4 What is published

Only the job summary (`$GITHUB_STEP_SUMMARY`):

- from cargo: total, passed, failed and ignored counts (the `test result:` lines); the number of
  `SKIPPED (no game data)` markers (reason text not printed); names of failed tests;
- from each verifier: its name, exit status and runtime.

No artifact is uploaded and no cache is written by this job.

### 2.5 Redaction rules

- **R1. The raw log stays on the runner.** It is written under `$RUNNER_TEMP` and deleted by an `if: always()` step.
- **R2. No artifacts, no caches.** The data job never calls `upload-artifact` and never saves a cache.
- **R3. Assertion messages print no data.** `crates/ss2-vfs/src/install.rs` line 289, `assert_eq!(&head[..8], b"CTSEMETA")`,
  prints the first eight bytes of a game file on failure. PROPOSED: check the header with a boolean. Output from
  `tools/gro-tool/tests/cli.rs` can contain archive paths; it stays in the local log (R1). Review checks new tests.
- **R4. Python tracebacks can echo data.** `tools/re/validate_ctsemeta.py` line 37, `assert t == b"EXOB", t`, prints
  four bytes of the file. Tracebacks stay in the local log and are never summarised.
- **R5. No write modes.** Scripts with `--table`, `--markdown`, `--json`, `--png` or `--modules` write files. The job
  does not use them. `tools/re/inventory.py` writes to a hard-coded path on its author's machine
  (`C:\Games\SS2Rewrite\docs\re\inventory.md`, line 53) and must not run on a runner until fixed.
- **R6. No paths.** The skip message prints the directory path (section 1.1). Removing it is a small change to
  `SkipReason`'s `Display`, outside T12-002.
- **R7. Verifier stdout stays local.** Several scripts print class names or key shapes (`l10n_verify.py` prints
  "a few key shapes"). That output is local only.

Policy basis: `docs/TESTING.md` section 1 ("assert only facts stated in `docs/re/`") and section 6 (never print or
save game content); `docs/PUBLISHING.md` section 3, gate 1 ("no derived data").

## 3. Trust model for fork PRs and self-hosted runners

A self-hosted runner executes whatever the workflow checks out, with the machine's own privileges, and `cargo test`
runs code from the checkout by design. A fork PR could therefore run arbitrary code on the operator's machine.
GitHub's guidance warns against self-hosted runners on public repositories for this reason (external, not in this
repo; re-read before adoption).

**Threats.** (1) A PR changes a test, `build.rs`, a `tools/re/` script or `.cargo/config.toml` so that it reads
`SS2_DATA_DIR` and sends bytes out (network, log, comment, artifact). (2) A PR uses the host: reads the runner's
registration files, persists between jobs, or mines. (3) A PR poisons a cache that a later run trusts.

**Mitigations (PROPOSED).**

1. **Label-gated, maintainer-only.** Only a maintainer applies `run-data-tests`, after reading the diff.
2. **`pull_request`, never `pull_request_target`**, which runs base-repository context with secrets and a write token on fork code.
3. **No secrets, read-only token**: `permissions: contents: read`; no `secrets.*`.
4. **Environment approval** (`environment: data-gated` with a required reviewer), if the plan allows.
5. **Ephemeral runner**: one job per registration, then removed. GitHub documents an ephemeral option (verify).
6. **Low-privilege account, containerised if possible.** The job account cannot read the runner's registration files
   or write to the data folder.
7. **Egress default deny**: GitHub (job control and checkout), the crates.io index and download hosts, and the Rust
   toolchain host unless pre-installed. The exact list is TBD. An allowed host remains a possible channel.
8. **One machine or VM per operator**; no runner shared between operators or repositories.
9. **Operator allow-list**: only people named by a repository admin may register a runner. Runner names carry no
   personal data (`AGENTS.md` section 1).
10. **Per-job cleanup**: fresh checkout (`actions/checkout` cleans by default) and a deleted work folder. A build
    cache is saved only by pushes to `main` and scheduled runs, never by labelled PR runs.
11. **Review gate**: `.github/CODEOWNERS` already assigns `/.github/` and `/docs/re/` to the repository admin. PRs
    touching `tools/re/`, `crates/ss2-testkit/`, `.github/`, `Cargo.toml`, `build.rs` or `.cargo/` need the diff read first.
12. **No review, no run.** If maintainers cannot review the diff, the label is not applied and the job does not run.
    The PR then relies on the author's local results, posted in the PR (`AGENTS.md` section 1c).

**Residual risks.** A reviewed diff can hide an exfiltration. An allowed host can carry data. A malicious maintainer
or a compromised operator machine can read the data regardless, and the operator reads it by design.

## 4. Contributor-operated runners

### 4.1 Roles

- **Operator**: a contributor with a legal copy. Runs the machine, keeps the data read-only, keeps the runner current.
- **Repository admin**: approves registrations, creates the label and the environment, removes runners. The repository
  is owned by a user account, so runners are registered per repository and organisation runner groups do not apply (verify).
- **Maintainer**: reads the PR diff and applies `run-data-tests`.

### 4.2 Registering a runner (PROPOSED)

1. The admin creates a registration in the repository's Actions runner settings. The token it shows expires.
2. The operator prepares a dedicated machine or VM (section 3). Installs the toolchain pinned in
   `rust-toolchain.toml` (currently 1.99.1; see `docs/DEV_TOOLS.md`). On Linux, `libudev-dev` and `pkg-config`, as
   `ci.yml` does. On Windows, the MSVC build tools.
3. Copies the `.gro` archives into the fixed folder: no other `.gro` files, no renames, no edits (`docs/user/install.md`).
4. Registers the runner with labels `self-hosted` and `ss2-data`, under a dedicated account, ephemeral if GitHub supports it for this runner.
5. Sets `SS2_DATA_DIR` in the service environment, pointing at the archive folder (not its parent).
6. Verifies the install (4.3).

### 4.3 Verifying a supported version

`docs/user/install.md` documents one version: the retail PC copy patched to 1.0, with 14 archives. No other edition
or patch level is documented as working. `docs/re/inventory.md` lists the 14 archive names, each entry count (exact)
and each size in MB to one decimal place (rounded).

- Compare names and entry counts with that table, using `cargo run -p gro-tool -- stats` (`install.md`). Do not run
  `tools/re/inventory.py` on a runner (R5).
- A mismatch is recorded in the summary and, PROPOSED, does not block tests until a version checker exists. That
  checker exists as `ss2-doctor` (T11-013; it checks the archives, their readability and the patch level, not the file contents); the first-run wizard (T10-019) is not built.

### 4.4 What a runner reports (PROPOSED)

A fingerprint step writes to the job summary: the 14 archive names; each archive's entry count and byte size; and the
match result against `docs/re/inventory.md`. It reports no file contents and no hashes of game files. The docs do not
state a rule on hashes of proprietary files; "hashes are not published" is a PROPOSED rule, not an existing one (section 8).

### 4.5 Revoking

The admin removes the runner in the repository's runner settings. The operator removes it from the machine with a fresh
removal token, then deletes the data folder and the runner account. After a suspected compromise, the admin also
removes environment and label access, and a maintainer reviews recent summaries.

### 4.6 Quorum (PROPOSED)

A data-gated change is ready when hosted checks pass and at least one `ss2-data` runner passes on the head commit with
a fingerprint that matches `docs/re/inventory.md`. Disagreeing runners go to a maintainer before merge. One runner is
a single point of failure; "at least one" is the minimum.

## 5. Data-gated tests and spec tools today

**Rust: 3 call sites in the workspace** (grep of `require_game_data`):

| Test | File | Crate | Asserts |
|---|---|---|---|
| `macro_skips_or_runs` | `crates/ss2-testkit/src/lib.rs` | `ss2-testkit` | self-test: the path is a directory |
| `the_real_install_mounts_and_patch_overrides_apply` | `crates/ss2-vfs/src/install.rs` | `ss2-vfs` | mounts the install; over 10,000 files; patch overrides |
| `the_real_install_lists_the_menu_screens` | `tools/gro-tool/tests/cli.rs` | `gro-tool` | `gro-tool find` returns over 90 menu screens |

Two are real data-gated tests; one is the macro's self-test. The per-loader "full install" tests named in
`docs/roadmap/T02-data-formats.md` do not exist yet.

**Spec tools:** 46 Python files in `tools/re/`. 44 are command-line scripts that read an install: `ai_verify`,
`ans_verify`, `audio_verify`, `bmf_verify`, `bosses_verify`, `camera_verify`, `class_catalog`, `destruction_verify`,
`edob_verify`, `effects_verify`, `enemies_verify`, `ep_schema`, `fnt_verify`, `fx_verify`, `inventory`, `l10n_verify`,
`mat_verify`, `mdl_verify`, `misc_verify`, `movie_verify`, `mtr_verify`, `pfx_verify`, `puppet_stats`, `rules_verify`,
`shader_verify`, `skl_verify`, `tex_verify`, `validate_ctsemeta`, `vehicles_verify`, `walk_objects`, `weapons_verify`,
`wld_entity_schema`, `wld_entity_verify`, `wld_env_verify`, `wld_geometry_verify`, `wld_lighting_verify`,
`wld_logic_verify`, `wld_macro_verify`, `wld_nav_verify`, `wld_physics_verify`, `wld_root_verify`, `wld_sound_verify`,
`wld_verify`, `wld_vis_verify`. The other two, `ctse_values.py` and `texpng.py`, are libraries. None runs in CI.

## 6. Interim process (now)

GitHub Actions jobs have not started since 2026-11-22 because of an account billing problem (`AGENTS.md` section 1c).
Until that section is removed, authors run the checks locally. For a loader or spec-tool change they also run the
data-gated tests with `SS2_DATA_DIR` set to their own install, and post the result in the PR. "Passed" is claimed only
for what was run. Nothing in this design runs until Actions works and a runner is registered. The data job needs its
own "first real run" check before its results count.

## 7. Costs and limits

- **Runner minutes:** GitHub does not bill self-hosted runner minutes (verify). The machine, power, disk, bandwidth and
  the operator's time are the operator's cost.
- **Disk:** the 14 archives total about 3,012 MB on disk (sum of the "On disk" column in `docs/re/inventory.md`), before
  checkout and build output (size TBD).
- **Build cache:** reuse of `target/` and the Cargo registry saves time but conflicts with per-job cleanup (mitigation 10).
- **Runtime:** per-script figures are in section 2.3. The `cargo test` runtime is TBD; no figure is stated in the repo.
- **Queueing:** one job per runner, so a long nightly delays labelled PR runs. PROPOSED: a separate runner or a priority rule.
- **Availability:** if the operator's machine is offline, data results are missing; hosted CI is unaffected.
- **Dependants:** T12-004 (`docs/PERF_BUDGETS.md`, timing runs), T11-012 (dashboard "fed by the data-gated runner"),
  T12-016 (soak farm, which needs a real level).

## 8. Gaps and open questions

1. **Visibility.** `.github/workflows/docs.yml` says the repository is private. If it becomes public, job logs and
   summaries are public. Decide which case this design targets.
2. **Environment approval** availability on this account's plan is not checked.
3. **Label** `run-data-tests` does not exist yet (admin task).
4. **Pass rule per verifier** is not defined (section 2.3).
5. **Skip counting.** Skips count as passed, the marker is hidden under default capture, and it does not name the test.
   The parser needs a check for lines split by test output. A PROPOSED change would have the macro print the test name too.
6. **Skip message prints the path** (R6). Outside T12-002.
7. **`tools/re/inventory.py`** writes to a hard-coded absolute path (R5).
8. **Byte sizes and hashes.** No document says whether exact byte sizes or hashes of retail archives may be published.
   `docs/re/inventory.md` records "paths, sizes and counts"; `docs/PUBLISHING.md` section 3 says "no derived data".
9. **File names in summaries.** `docs/COMPLIANCE.md` section 4 says the clean-room log never holds an install file name,
   while `docs/re/inventory.md` lists them. The summary rule must choose.
10. **Version detection** is by archive names only: `ss2-doctor` (T11-013) reports the patch level from the newest `Patch_*.gro` name and checks that every archive opens; it does not compare file contents or counts against a reference. The wizard (T10-019) is planned.
11. **Egress allow-list** (GitHub, crates.io, Rust toolchain hosts) is not defined; take it from GitHub's and Cargo's documentation.
12. **Ephemeral runners and runner-version pinning** are assumed; verify both.
13. **Quorum and single operator.** Section 4.6 has no second operator in sight.
14. **Operator allow-list.** No written list exists. `.github/CODEOWNERS` names one owner for `/.github/`.
15. **Runtimes and timeouts.** `cargo test` runtime, nightly time and `timeout-minutes` are TBD.
16. **Downstream format.** T11-012 and T12-016 need machine-readable results; the summary is human-readable only.
