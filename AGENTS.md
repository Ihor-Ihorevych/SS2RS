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

Limits: never merge while a check is red or pending, never merge a PR with conflicts (merge `origin/main` into your branch, push, wait again), never merge a PR somebody else opened unless the owner asked you to, and never use admin overrides to get around a failing check. If a failure is not yours (for example another PR broke `main`), say so and stop instead of merging.

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
