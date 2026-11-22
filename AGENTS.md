# Instructions for AI agents (and humans) working in this repo

Project: SS2RS — open-source Rust engine that plays *Serious Sam 2* from the player's own data.
Read first: `docs/superpowers/specs/2026-11-21-ss2rs-design.md`, `docs/roadmap/README.md`, `docs/DEV_TOOLS.md`.

## 1. Claim before you start (`[INPRG]` protocol)

Many agents and people work in parallel. **A task nobody can see you working on is a task someone else
will start.** Before writing any code or spec for a roadmap task (`docs/roadmap/T*.md`):

1. **Always fetch first — before every new task, no exceptions.** Run `git fetch origin` (then `git pull --rebase origin main`) *before you pick a task, before you claim it, and before you start work on it* — even if you finished another task five minutes ago. Claims only exist for you once you have fetched; working from a stale checkout is exactly how two agents end up taking the same task. Then read the claims: `grep -rn "INPRG" docs/roadmap/`
2. Pick a task that is `- [ ]` (unclaimed), whose **Deps** are all `- [x]` (or have merged stubs/mocks), and
   whose size is not `XL` (split XL tasks first: append new IDs, never renumber).
   `PHASE 2` tasks are off-limits until milestone M8.
3. **Claim it by editing its line in the track file** — change `- [ ]` to `- [~]` and insert the tag right after
   the bold ID:

   ```
   - [~] **T02-006** [INPRG @your-github-handle · branch `T02-006-meta-reader` · 2026-11-22] · CTSEMETA container reader · …
   ```
   Use your GitHub handle (or `<handle>/agent` for an agent working for that person). **No emails or other
   personal data.**
4. **Commit and push that single-line change straight to `main`** (it is the only thing allowed to skip a PR). Put `[skip ci]` at the end of the message: a claim changes no code, and every push to `main` would otherwise run the whole build matrix:

   ```
   git commit -s -m "T02-006: [INPRG] claim (@your-github-handle) [skip ci]"
   git push origin main
   ```
5. **If the push is rejected**, someone pushed first: `git pull --rebase origin main`. If your task line is now
   `[INPRG]` by someone else (or the rebase conflicts on it), **you lost the race — abort
   (`git rebase --abort`, `git reset --hard origin/main`) and pick another task.** Never overwrite another
   agent's claim.
6. Work on branch `T02-006-meta-reader`. Push it early and open a **draft PR** titled `T02-006: …` so progress is
   visible. Push at least every working day.
7. **Finish:** in the same PR change the line to `- [x]` and **remove** the `[INPRG …]` tag.
   **Give up / blocked:** push `T02-006: [RELEASED] unclaim (<reason>)` which restores `- [ ]` and removes the tag;
   leave notes on the issue or in the PR.
8. **Stale claims** (`cargo xtask claims` lists them; a weekly job posts the same table, and
   `cargo xtask check-claims` fails CI on contradictory or malformed claim lines): an `[INPRG]` task whose branch/PR has no push for 7 days may be pinged; after 48 more hours
   anyone may release it with a `[RELEASED]` commit saying `stale`.

Rules of thumb: **fetch before every new task, and again before opening a PR** (merge or rebase `origin/main` into your branch so you build on the latest claims, contracts and specs); one claim at a time per agent unless the tasks are tiny and adjacent; never claim what you do
not intend to start today; never mark `[x]` without a merged PR.

## 1b. Merge your own PR as soon as it is green (standing permission)

When **all** checks on your pull request have passed (skipped checks are fine), **merge it yourself, without asking**:

1. `git fetch origin`, then confirm the PR is mergeable (no conflicts) and no check is failing or still running.
2. Merge with a **merge commit** (`gh pr merge <n> --merge`), never squash or rebase: every signed-off commit must survive.
3. Then `git switch main && git pull --ff-only origin main`.

Limits: never merge while a check is red or pending, never merge a PR with conflicts (merge `origin/main` into your branch, push, wait again), never merge a PR somebody else opened unless the owner asked you to, and never use admin overrides to get around a failing check (while Actions cannot start jobs, see §1c). If a failure is not yours (for example another PR broke `main`), say so and stop instead of merging.

## 1c. While GitHub Actions cannot run: build and test locally (temporary rule)

Since 2026-11-22 the repository's Actions jobs do not start: GitHub reports every job as failed after 1 to 3 seconds with the annotation *"The job was not started because recent account payments have failed or your spending limit needs to be increased"* (a billing problem, not a code problem). A check that never started proves nothing, so **until this section is removed you run what CI would run yourself, on your own machine**.

**Before every push of code, and again before every merge**, on the tree *after* you merged `origin/main` into your branch:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                 # on Windows and, if you can, also on Linux/WSL (CI runs both)
cargo deny check
cargo xtask guard
cargo xtask check-claims
# docs or roadmap changes:
mdbook build docs && python .github/scripts/check_book_links.py target/book
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
# every commit of the branch must carry a sign-off (the dco job):
for c in $(git rev-list --no-merges origin/main..HEAD); do git show -s --format=%B $c | grep -qE '^Signed-off-by: .+ <.+>' || echo "no sign-off: $c"; done
# if you touched platform code (the portability job, informational): cargo check --target aarch64-linux-android -p ss2-core -p ss2-vfs -p ss2-meta -p ss2-assets -p ss2-sim -p ss2-net
```

- Tools you do not have (`cargo-deny` and `mdbook` are listed in `docs/DEV_TOOLS.md`): install them; `gitleaks` (the CI secrets job) is optional locally. If you cannot run something, say so in the PR and do not claim it passed. Never commit keys, tokens or personal data.
- Put the result in the **PR description** (one line per command: passed, or the failure). "Not run" is an honest answer; "passed" without having run it is not.
- Data-gated tests skip without `SS2_DATA_DIR`; run them with it set to your own install when you change a loader or a spec tool.
- **Merging (§1b)** still needs every check that *ran* to be green. While the only red checks are jobs that *did not start* (the billing annotation above; look at `gh run view <id> --json jobs` and the check annotations), your local results, posted in the PR as above, take their place. Never merge over a check that ran and failed, and never merge when a local command failed.
- Remove this section when a pull request's jobs run their steps again (a run of `ci.yml` on `main` with steps, not an instant failure).

## 2. Hard rules (violations get the PR rejected)

- **Clean room** (`CLEANROOM.md`): spec writers (T01) analyse binaries and write prose specs in `docs/re/`;
  implementers write Rust from specs/public docs only. Never copy SS1 (GPL) code or decompiler output.
- **Never commit game content**: `Serious Sam 2/`, `*.gro`, `*.dll`, `*.exe`, `.wld/.tex/.mdl/…`, Ghidra projects.
  The pre-commit hook (`git config core.hooksPath .githooks`) and CI (`cargo xtask guard`) enforce it.
- **DCO**: every commit `git commit -s`. Commit/PR titles start with the task ID: `T02-006: …`.
- **No unclaimed work**: do not do a roadmap task without a claim on `main` first (small fixes outside the
  roadmap are fine).
- **Stay in your crate** (see `CODEOWNERS`); cross-crate API changes need an RFC (`docs/rfcs/`).
- **Tests**: data-gated tests use `ss2_testkit::require_game_data!()` and must skip cleanly without `SS2_DATA_DIR`.
- **Sim-safe crates** (`ss2-sim`, `ss2-physics`, `ss2-net`): no `HashMap`/`HashSet`, no wall-clock time.
- **PC first**: do not start Android/mobile work (`PHASE 2`) before M8.
- Definition of Done: roadmap README §4 (fmt, clippy `-D warnings`, deny, tests, docs, tick the box).

## 3. Useful commands

```
cargo xtask guard            # proprietary-content check (add --staged before committing)
cargo xtask doctor           # check your tools against docs/DEV_TOOLS.md
cargo xtask claim T02-006    # §1 steps 1-5 in one go (see below)
cargo xtask release T02-006 --reason "blocked on T01-012"   # give a claim back (§1 step 7)
cargo xtask check-claims     # are the roadmap claims consistent? (CI runs this on every PR)
cargo xtask claims           # who claimed what, how long idle, which claims are stale
cargo xtask roadmap-sync     # dry run: roadmap -> GitHub issues
grep -rn "INPRG" docs/roadmap/   # who is working on what right now
```

`cargo xtask claim <ID>` must be run from a clean `main`: it fetches, checks the task is `[ ]`, not
`XL`, not `PHASE 2` and that its deps are `[x]`, edits the one line, commits it signed off as
`ID: [INPRG] claim (@handle) [skip ci]`, pushes to `main`, and if the push is rejected drops its own
commit and looks again, so it **never overwrites another claim**. Your handle defaults to the logged-in
`gh` user (`--who` overrides it). It then prints the `git switch -c <branch>` to run. Exit status:
`0` claimed, `3` not available (taken, blocked, finished, or you lost the race: pick another task),
`2` real error. `--dry-run` previews from any branch. Use the branch name it prints (it is recorded
in the claim, and the stale report looks for pushes to exactly that branch). Doing the steps by hand (above) is still fine.
