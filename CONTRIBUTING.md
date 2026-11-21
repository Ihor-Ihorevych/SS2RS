# Contributing to SS2RS

Thanks for helping! Start with the [README](README.md), the [design](docs/superpowers/specs/2026-11-21-ss2rs-design.md) and the [roadmap](docs/roadmap/README.md).

## Setup

1. Install the tools listed in **[`docs/DEV_TOOLS.md`](docs/DEV_TOOLS.md)** — everyone uses that toolset. `rustup` picks the pinned Rust version from `rust-toolchain.toml` automatically.
2. Enable the repository hooks once per clone (blocks commits of proprietary game content):
   ```
   git config core.hooksPath .githooks
   ```
3. Optional, for data-gated tests: point `SS2_DATA_DIR` at the folder containing your `All_PC_01.gro`. Tests that need it skip cleanly without it.

## Claim a task, then work on a branch

Work comes from the roadmap files in `docs/roadmap/`. **Claim before you start** — the full protocol (including what to do when you lose a race) is in [`AGENTS.md`](AGENTS.md) §1:

1. **Always `git fetch origin` before every new task** (then `git pull --rebase origin main`), and again before opening a PR. Then `grep -rn INPRG docs/roadmap/` to see what is taken.
2. Pick an open `- [ ]` task whose dependencies are done, not `XL`, not `PHASE 2`.
3. Mark it `- [~] **ID** [INPRG @your-handle · branch `…` · date]`, commit `ID: [INPRG] claim (@your-handle) [skip ci]` and push **that one-line change** to `main`. `cargo xtask claim ID` does steps 1-3 for you (and `cargo xtask release ID --reason ...` gives a claim back).
4. Work on branch `T02-006-short-name`, open a draft PR titled `T02-006: …` early, and tick `[x]` / remove the tag in the same PR.

## Commits and pull requests

- Sign off every commit (**DCO**): `git commit -s`. CI rejects commits without `Signed-off-by:`.
- Start commit and PR titles with the task ID: `T02-006: CTSEMETA container reader`.
- Keep PRs small (about 400 lines) and focused on one task.
- Fill in the PR template, including the clean-room provenance statement ([`CLEANROOM.md`](CLEANROOM.md)).
- **Merge your own PR as soon as every check is green** (merge commit, never squash); never merge red, pending or conflicting PRs, nor someone else's PR. Details: [`AGENTS.md`](AGENTS.md) §1b.

## Rules of the road

The workflow rules and the **Definition of Done** are in the [roadmap README](docs/roadmap/README.md) (§3, §4). In short: contracts before code, spec → implement → verify, stay inside your crate (`CODEOWNERS`), tests included, `cargo fmt`, `cargo clippy -D warnings`, `cargo deny`, and docs updated. Coding rules: [`docs/CODING_STANDARDS.md`](docs/CODING_STANDARDS.md). Cross-crate API changes need an RFC ([`docs/rfcs/`](docs/rfcs/0000-template.md)).

## Dependencies

Adding or updating a dependency (licenses, `[workspace.dependencies]`, Dependabot PRs, advisories): [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md).

## Releases

Versioning, the changelog and how a release is cut are in [`docs/RELEASING.md`](docs/RELEASING.md). If your change is user-visible, add one line under **Unreleased** in [`CHANGELOG.md`](CHANGELOG.md) in the same PR.

## Labels

`track:Txx`, `size:S|M|L|XL`, `prio:P0..P3`, `phase-2`, `spike`, `spec`, `good-first`, `in-progress`. GitHub issues mirror the roadmap (`cargo xtask roadmap-sync`); **the roadmap files are the source of truth.**

## Useful commands

```
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo xtask guard                 # proprietary-content check
cargo xtask doctor                # check your machine against docs/DEV_TOOLS.md
cargo xtask claim T02-006         # claim a roadmap task (see AGENTS.md section 3)
cargo xtask check-claims          # are the roadmap claims consistent? (CI runs it)
cargo xtask claims                # active claims, idle time, stale ones
cargo xtask roadmap-sync          # dry run: roadmap -> GitHub issues
```
