# Instructions for AI agents (and humans) working in this repo

Project: SS2RS — open-source Rust engine that plays *Serious Sam 2* from the player's own data.
Read first: `docs/superpowers/specs/2026-11-21-ss2rs-design.md`, `docs/roadmap/README.md`, `docs/DEV_TOOLS.md`.

## 1. Claim before you start (`[INPRG]` protocol)

Many agents and people work in parallel. **A task nobody can see you working on is a task someone else
will start.** Before writing any code or spec for a roadmap task (`docs/roadmap/T*.md`):

1. `git pull --rebase origin main`, then read the claims: `grep -rn "INPRG" docs/roadmap/`
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
4. **Commit and push that single-line change straight to `main`** (it is the only thing allowed to skip a PR):

   ```
   git commit -s -m "T02-006: [INPRG] claim (@your-github-handle)"
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
8. **Stale claims:** an `[INPRG]` task whose branch/PR has no push for 7 days may be pinged; after 48 more hours
   anyone may release it with a `[RELEASED]` commit saying `stale`.

Rules of thumb: one claim at a time per agent unless the tasks are tiny and adjacent; never claim what you do
not intend to start today; never mark `[x]` without a merged PR.

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
cargo xtask doctor           # check your tools against docs/DEV_TOOLS.md   (T00-027)
cargo xtask claim T02-006    # claim a task per §1 automatically            (T00-028)
cargo xtask roadmap-sync     # dry run: roadmap -> GitHub issues
grep -rn "INPRG" docs/roadmap/   # who is working on what right now
```
(`xtask` subcommands appear as their roadmap tasks land; until then do the steps by hand.)
