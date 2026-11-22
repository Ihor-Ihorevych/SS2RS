# Compliance procedure

Status: **policy v0** (roadmap task T12-010), written 2026-11-22. It describes what the repository does
today. Items marked **PROPOSED** do not exist yet: no tool, workflow or file implements them. No release
has been made, so none has gone through this procedure.

The procedure covers the license audit, the SBOM per release, the clean-room audit log, and DCO sign-off.
The rules themselves are in the documents linked here; this page says who checks what, how, and in what order.

## 1. Scope and responsibilities

- **Every release**: [RELEASING](RELEASING.md) and the go/no-go table in [PC 1.0 checklist](PC_1_0_CHECKLIST.md), section 9, rows 18 and 19.
- **Every pull request**: `cargo deny`, DCO, provenance statement, no game content.
- **Clean-room work**: roles and forbidden sources are defined in [CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md). Not restated here.

- **Spec author (T01)** records provenance in the spec header and the log.
- **Independent spec reviewer (T01-049)** re-reads the spec and fills the **Reviewer** line ([conventions](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/conventions.md) section 3).
- **Implementer** writes from specs and public documents, ticks the PR provenance box, adds the log row.
- **Maintainers** (`CODEOWNERS`) approve changes to `deny.toml`, `docs/re/`, `CLEANROOM.md`, `.github/`, and decide releases.
- **Legal and clean-room owner**: the release-time role in the go/no-go table. `CODEOWNERS` is a single placeholder owner; no person holds this role in the repository.

## 2. License audit

### 2.1 What is checked

`deny.toml` is the only license policy. CI runs `cargo deny check` on every pull request (`deny` job in
[ci.yml](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/.github/workflows/ci.yml)). It enforces:

- **`[licenses] allow`**: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC,
  Zlib, MPL-2.0, Unicode-3.0, CC0-1.0. Anything else fails. GPL and AGPL are not listed. Changing the list
  needs the project owner's approval (the file header).
- **`confidence-threshold = 0.8`** for license detection.
- **`[advisories] ignore`**: one entry, `RUSTSEC-2026-0192`, with a reason and a revisit condition.
- **`[bans]`**: `multiple-versions = "warn"` (warning only).
- **`[sources]`**: crates.io only; unknown registries and all git sources denied.

Local run on 2026-11-22, cargo-deny 0.20.2: `advisories ok, bans ok, licenses ok, sources ok`, with
duplicate-version warnings. The `lru` advisory in [DEPENDENCIES.md](DEPENDENCIES.md) is a warning under this config.

**Scope limit:** cargo-deny reads Cargo metadata only. Bundled assets are not checked. The Inter font
(`crates/ss2-render/assets/Inter-Regular.ttf`, OFL-1.1, `OFL.txt` beside it) is one; it is recorded in
[NOTICE](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/NOTICE) instead.

### 2.2 How to run it

```
cargo install --locked cargo-deny   # DEV_TOOLS.md section 1 (required)
cargo deny check                    # the gate: advisories, licenses, bans, sources
cargo deny check advisories         # what the weekly audit workflow runs
cargo deny list                     # per-crate license inventory (a report, not a gate)
```

### 2.3 A new license, or an exception

1. **License not on the list:** replace the dependency first. [DEPENDENCIES.md](DEPENDENCIES.md) allows
   permissive licenses only; GPL and AGPL never (`deny.toml` header, [CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md)).
2. **Permissive license not on the list:** a separate PR that edits the allow-list, with the reason. The
   project owner approves it.
3. **Single-crate exception:** none is in use. **PROPOSED:** a `[[licenses.exceptions]]` entry in `deny.toml`
   with the crate, version, license, reason and revisit condition, approved by the owner of `/deny.toml`.
4. **Advisory exception:** an `ignore` entry with `id`, `reason` and a revisit condition, and a row in the
   table in [DEPENDENCIES.md](DEPENDENCIES.md). No reason, no merge.

### 2.4 Project license and no game content

- The project is **Apache-2.0** (`LICENSE`; `license = "Apache-2.0"` inherited by each crate, see
  [PUBLISHING.md](PUBLISHING.md) section 1). `NOTICE` carries the affiliation statement and the font notice.
- Release archives contain `LICENSE`, `NOTICE`, `README.md` and no game content ([RELEASING.md](RELEASING.md) section 4).
- The no-game-content rules are in [CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md)
  (Forbidden), [MODDING_POLICY.md](MODDING_POLICY.md) sections 3 and 6, and [PUBLISHING.md](PUBLISHING.md) section 3.1.
  The repository check is `cargo xtask guard` (section 6.1). Its rules match MODDING_POLICY section 6. It does not
  check release archives (section 6.2).

## 3. SBOM per release

**Purpose.** An SBOM lists the exact crate versions and licenses in a release build. It serves downstream
license review, vulnerability tracking, and the "License report and SBOM" row of the go/no-go table.

**Current state: none.** A search for `cyclonedx`, `cargo-sbom`, `cargo-about`, `sbom` and `spdx` finds only
[RELEASING.md](RELEASING.md) section 6, [PC_1_0_CHECKLIST.md](PC_1_0_CHECKLIST.md) and the T12-010 line. No SBOM tool
is in `Cargo.toml`, `Cargo.lock`, [DEV_TOOLS.md](DEV_TOOLS.md) or any workflow.

**Generator (PROPOSED, NOT INSTALLED OR INTEGRATED): `cargo-cyclonedx`** (CycloneDX JSON), or `cargo-about`
for a license-text report. On adoption it goes into [DEV_TOOLS.md](DEV_TOOLS.md) and `release.yml` in one PR, pinned with `--locked`.

**Storage (PROPOSED).** Attached to the **draft** GitHub release beside the archives, from the `assets/` folder
that `release.yml` uploads ([RELEASING.md](RELEASING.md) section 3, step 5). Proposed name
`ss2rs-<version>-sbom.cdx.json`. Note that `release.yml` checksums `ss2rs-*`, so the SBOM would be checksummed
too. The SBOM is not committed to the repository.

**Steps (PROPOSED).** On the tag with a clean `git status`: (1) generate the SBOM with `--locked`; (2) run
`cargo deny list`; (3) verify the JSON parses (`python -I -m json.tool <file>`), its components match
`cargo metadata --format-version 1 --locked`, and every license is on the allow-list or is a workspace crate;
(4) confirm both files are in `SHA256SUMS`, attach them to the draft, and record the commands in the release PR.

## 4. Clean-room audit log

**Where:** [docs/compliance/clean-room-log.md](compliance/clean-room-log.md). Created by this change with its
header, rules and an example template. The table is empty.

**Provenance today.** [docs/re/README.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/README.md)
requires the task, an **Author / reviewer** line and a confidence word in each spec header;
[conventions.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/conventions.md) requires the Evidence
section to name the `tools/re/` script behind each count, and the reviewer to record name and date.

**Current state (2026-11-22).** `docs/re/` has 60 files. 17 have no Author line, 10 of them generated
`*-schema.md` files. Every Reviewer line that exists reads "none yet". **No independent review is recorded for
any spec.** No implementation is mapped to a spec anywhere.

**Per spec:** file, author and reviewer (handle and date, or "none yet"), and the source data by **file type and
tool only** (for example "binary, Ghidra; counts from `tools/re/<script>`"). Never an install file name, a quote,
a byte dump, an address or a decompiler identifier.

**Per implementation unit:** task ID, implementer, PR, specs and public documents used, reviewer, date, and
whether the PR's provenance box is ticked.

**Rules for adding a row:**

1. The implementer adds the row in the implementation PR, on the same branch.
2. Only specs that exist in `docs/re/` are named, with the roadmap task that uses them.
3. A spec author may not add or review a row for a topic they specified ([CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md) roles).
4. Rows are never deleted. A correction is a new commit with a note in the row.
5. Unclear source: stop and open an issue ([CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md), "When unsure").

**Reviewer checklist for an implementation PR (PROPOSED wording):**

- [ ] The provenance box is ticked, and the PR names the `docs/re/` specs and public documents it used.
- [ ] Each spec used is in the log with its author and reviewer state.
- [ ] No decompiler output, disassembly, Ghidra export, SS1 (GPL) code or text in the diff, comments or test data.
- [ ] Test vectors come from the spec, public documents, or the author's own measurements.
- [ ] No file in the diff came from the game install: `cargo xtask guard` passes and the reviewer opens every new binary or data file.
- [ ] The implementer is not the spec author for that topic.
- [ ] Every commit is signed off (section 5).

**Seed rows: none.** A row links a spec to an implementation PR. Git history was not matched to roadmap tasks
spec by spec, so no row is certain. The maintainers fill the first rows from PR history.

## 5. DCO sign-off

**Enforcement today.** The `dco` job in [ci.yml](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/.github/workflows/ci.yml)
runs on `pull_request` only. It checks each non-merge commit in `base..head` for `^Signed-off-by: .+ <.+>`, and
skips a PR only when its author is `dependabot[bot]` ([DEPENDENCIES.md](DEPENDENCIES.md)). Rule: every commit is
`git commit -s` (AGENTS.md section 2, [CONTRIBUTING.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CONTRIBUTING.md)).
The local loop is in AGENTS.md section 1c.

**Limits.** No check runs on pushes to `main`, so a commit pushed there without a PR is not checked. Merge
commits are skipped, so a `git merge` commit needs no sign-off.

**Current state.** Of 202 non-merge commits on `origin/main` (2026-11-22), five lack a sign-off: `43ea46a`,
`02f6be5`, `732b9e9`, `a68c4bf`, `0c4a75c`, all from 2026-11-21. They stay; `main` is not rewritten.

**Fix for a branch missing a sign-off (PROPOSED standard).** On your own feature branch only:

```
git fetch origin
git rebase --signoff origin/main      # adds Signed-off-by to each commit of the branch
git push --force-with-lease origin <your-branch>
```

For only the last commit, `git commit --amend -s --no-edit` and a force push do the same. This rewrites a PR
branch, which AGENTS.md section 1b otherwise forbids for signed-off commits, so maintainers confirm the exception
first. Do not add a fix-up commit that leaves an unsigned commit in the history.

**Agents.** The claim handle for an agent working for a person is `<handle>/agent` (AGENTS.md section 1, step 3).
The repository does not define whose identity an agent's sign-off carries. **PROPOSED:** the accountable human's
name and address from their git identity; maintainers confirm.

## 6. Pre-release compliance checklist

### 6.1 What `cargo xtask guard` checks

Source: `xtask/src/guard.rs`. Run as `cargo xtask guard` (all tracked files; CI `guard` job) or
`cargo xtask guard --staged` (pre-commit hook). It rejects a path with a `Serious Sam 2` folder component;
`.gro`, `.dll`, `.exe`, `.chm` anywhere; `.wld`, `.tex`, `.mdl`, `.bmf`, `.ogv`, `.xmv` outside `testdata/`;
files over 5 MiB, and `testdata/` files over 256 KiB. It does **not** check contents, hashes, file names beyond
the extension, quoted game text, release archives, or secrets.

### 6.2 Checklist (copy into the release PR)

```markdown
## Pre-release compliance checklist

- [ ] Release commit is on `main`; local check results are in the PR (CI jobs may not start, AGENTS.md section 1c)
- [ ] `cargo deny check` passes on the release commit; warnings are listed in the PR
- [ ] `cargo deny check advisories` passes; every `ignore` in `deny.toml` still has its revisit condition
- [ ] License inventory (`cargo deny list`) attached to the draft release (PROPOSED)
- [ ] SBOM (CycloneDX JSON) generated from the tag, verified and attached to the draft release (PROPOSED, not integrated)
- [ ] Bundled non-Cargo assets listed with their license files (Inter, OFL-1.1, `OFL.txt`)
- [ ] `LICENSE` and `NOTICE` current, and included in every archive
- [ ] Third-party notices: `NOTICE` covers the bundled assets; no separate notices file exists (section 8)
- [ ] `cargo xtask guard` passes on the release commit
- [ ] Each archive listed by hand: only `ss2-app`, `ss2-server`, `LICENSE`, `NOTICE`, `README.md`; no game content
- [ ] `SHA256SUMS` matches each archive
- [ ] Clean-room log is current for every spec that code in the release range relies on
- [ ] Every commit in the release range has a sign-off: `git log --no-merges -E --invert-grep --grep='^Signed-off-by: .+ <.+>' <previous-tag>..HEAD` prints nothing
- [ ] License headers: none required today (section 8, item 14)
- [ ] Secrets: `gitleaks detect` run by hand, or the CI `secrets` job green (gitleaks is not installed locally)
- [ ] Release notes say the binaries are unsigned and that the player needs their own copy of Serious Sam 2
```

## 7. Actions do not start: checks run locally

Since 2026-11-22 GitHub Actions jobs do not start (billing; AGENTS.md section 1c). Until that section is removed,
the local commands there and this page's checks replace CI. Each command and its result goes in the PR
description; a missing tool is reported as not run, never as passed.

## 8. Gaps and open questions

1. **No SBOM generator** is chosen, installed or in CI (section 3).
2. **`release.yml` makes no SBOM or license report** and does not check archive contents.
3. **`cargo deny` covers Cargo metadata only.** Asset licenses such as OFL-1.1 are never compared with the allow-list. Decide whether they join the policy.
4. **`deny.toml` sets `all-features = false`.** Whether every target's dependencies are covered is not verified.
5. **No separate third-party notices file** exists; `NOTICE` names the font only.
6. **No license exception has been added.** The approver is the `/deny.toml` owner in CODEOWNERS, not a written list.
7. **No spec has an independent reviewer.** T01-049 is open.
8. **No spec-to-implementation mapping exists**; the log is empty.
9. **The PR template does not ask which specs were used**, only for the provenance box and the task ID. Adding that line is a follow-up template change.
10. **DCO is not checked on `main`**; five commits there lack a sign-off (section 5).
11. **Agent sign-off identity is not defined** (section 5).
12. **Wrong reference:** [PC_1_0_CHECKLIST.md](PC_1_0_CHECKLIST.md) section 7 points to "the loop in TESTING section 4". TESTING section 4 lists the job only; the loop is in AGENTS.md section 1c.
13. **No local secrets scan:** gitleaks is not in [DEV_TOOLS.md](DEV_TOOLS.md) and not installed; the only scan is the blocked CI job.
14. **License headers:** none of the 86 `.rs` files under `crates/`, `tools/` and `xtask/` carries a `Copyright` or `SPDX-License-Identifier` line. No rule requires one; the maintainers decide.
15. **Actions are blocked** (section 7): every "passed" here means "run locally" until AGENTS.md section 1c is removed.
