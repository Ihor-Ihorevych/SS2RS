# Playtest program

How SS2RS builds reach testers, how testers report back, how crash reports are triaged and how findings
reach the roadmap. Task **T12-011** (issue #345). Status: **draft v0**. Nothing has been released, no playtest
has run and no channel below is live. Items marked **PROPOSED** are not agreed.

## 1. Purpose and scope

**Purpose.** Put real hardware, drivers and game installs in front of the code before the milestones that need
them, and make the reports usable.

**Who can playtest.** People who own a legal copy of *Serious Sam 2* and can point the engine at their own install
folder, the one that directly contains `All_PC_01.gro` ([install](user/install.md)). SS2RS ships no game content
([CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md), [COMPLIANCE.md](COMPLIANCE.md) section 2.4).

**What a playtest build is today.**

- **No published build exists.** [install](user/install.md): "Until a release exists, the only way to run the code is
  to build it from source" ([DEV_TOOLS.md](DEV_TOOLS.md) lists the toolchain).
- **What runs.** `ss2-app` opens a placeholder menu that uses no game data. Only `gro-tool` and the data-gated tests
  read the install (`SS2_DATA_DIR`). A playtest today covers the window, input, settings, launch options and logging.
  It does not cover gameplay.
- **Packaging** ([PACKAGING.md](PACKAGING.md), `.github/workflows/release.yml`) has never run on a tag: **GitHub Actions
  cannot run jobs** (billing block since 2026-11-22, [AGENTS.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/AGENTS.md)
  section 1c). Until it is fixed, a maintainer builds and checks each archive locally ([RELEASING.md](RELEASING.md) section 3).
- **Milestones** ([roadmap README section 5](roadmap/README.md#5-milestones-and-exit-criteria)): M3 first walkable level,
  M4 co-op, M6 first playable slice, M7 campaign, M8 PC 1.0. None is marked complete; no dates are promised here.
  Before M3 only the platform and menus are testable; PC 1.0 needs playtest data for PC_1_0_CHECKLIST rows 8 and 9.

## 2. Build channels

| Channel | What it is | How it is published | Status |
|---|---|---|---|
| `nightly` | Build of `main` at one commit | Archives from a manual `release` workflow run, kept as workflow artifacts for 14 days; not a GitHub Release | Workflow step IMPLEMENTED, not run. No schedule exists (T12-008). Until then a maintainer builds locally. |
| `playtest` | Pre-release for one test, e.g. tag `v0.1.0-beta.1` | Tag `v<version>` with a `-`; `release.yml` creates a **draft** release with `--prerelease`; a human publishes | IMPLEMENTED in the workflow, not run |
| `stable` | Milestone or 1.0 release, e.g. `v0.1.0`, `v1.0.0` | Tag without `-`; same draft-then-publish flow, not a pre-release | IMPLEMENTED in the workflow, not run |

**Version strings.**

- The product version is `[workspace.package] version` in `Cargo.toml`: `0.0.0` today. `release.yml` fails unless the
  tag equals it without `v`, so `v0.1.0-beta.1` needs `version = "0.1.0-beta.1"` first.
- Non-tag archives: `ss2rs-<ver>-dev.<sha7>-<target>`. Archives: `ss2rs-<version>-<target>.<ext>` ([RELEASING](RELEASING.md)
  section 4), e.g. `x86_64-pc-windows-msvc.zip`.

**Package contents.** The fixed list in `release.yml`: `ss2-app`, `ss2-server` (`.exe` on Windows), `LICENSE`, `NOTICE`,
`README.md`, in one folder named like the archive. No game files. `gro-tool` is not packaged (OPEN, [PACKAGING](PACKAGING.md) section 1).

**Verifying a download.** `SHA256SUMS` (one `sha256sum` line per archive) is attached to the draft (IMPLEMENTED).
Linux and macOS: `sha256sum -c SHA256SUMS`. Windows: `Get-FileHash -Algorithm SHA256 <archive>` and compare with the
matching line. Binaries are unsigned ([RELEASING](RELEASING.md) section 6); signing and provenance are OPEN.

**Reporting the build.**

- `ss2-app --version` (or `-V`) prints `ss2-app <version>`, e.g. `ss2-app 0.0.0`. It does **not** print the commit.
- Source builds: add `git rev-parse --short HEAD`. Archives: add the archive file name.
- PROPOSED: include the commit in the version string (not in the repo today).

**Launch options that exist** (`ss2_core::cli`, `crates/ss2-core/src/cli.rs`):

| Option | `ss2-app` today | `ss2-server` today |
|---|---|---|
| `--data DIR`, `--level NAME`, `--portable` | accepted; logged as not used | accepted; not used |
| `--set KEY=VALUE` (repeatable) | applied, e.g. `--set vsync=off` | accepted; not used |
| `--connect HOST[:PORT]`, `--listen` | exits: "multiplayer is not implemented yet" | not an option |
| `--bind ADDR` | not an option | accepted; prints "not implemented yet" |
| `-h`/`--help`, `-V`/`--version` | usage / version | usage / version |

`ss2-app` also has `--smoke-frames N` (open, render N frames, exit; see [install](user/install.md)) and
`--screenshot main|options|pause OUT.png [--size WxH]`. Environment: `SS2_DATA_DIR` (install folder), `SS2_LOG`
(log filter, e.g. `SS2_LOG=debug`; `crates/ss2-core/src/logging.rs`).

## 3. Structured feedback form

Copy the block into a new issue, or into a comment on the round's tracking issue. Choose the issue template for the
finding first (section 5). Write "not measured" for a field you did not check.

````markdown
**Build** (output of `ss2-app --version`; commit for a source build; archive name, if any):
**Build check:** SHA-256 matched / not matched / not checked

**OS and version:**
**CPU:**
**GPU and driver version:**
**RAM:**
**Display:** resolution, scaling (%), monitors, windowed / fullscreen, refresh rate:
**Input:** keyboard / mouse / gamepad (model):
**Game install:** store and version; patch level (names of the `Patch_*` files are enough);
`SS2_DATA_DIR` is an unmodified retail install: yes / no / not sure

**Areas tried** (tick what you tried):
- [ ] menus  - [ ] settings  - [ ] input  - [ ] audio  - [ ] video (window, resolution, fullscreen, vsync)
- [ ] levels  - [ ] performance (fps, stutter, load time)  - [ ] saves  - [ ] co-op  - [ ] crash or hang  - [ ] other:

**Steps** (from a fresh start if you can):
1.

**Expected:**
**Actual:**
**Frequency:** always / sometimes / once
**Severity** (your guess; the triager decides): S1 crash, hang, data loss or desync / S2 blocks a level, weapon,
enemy or mission / S3 small wrong behaviour with a workaround / S4 cosmetic

**Attachments** (never game files):
- [ ] log: terminal output with `SS2_LOG=debug` (section 4)
- [ ] crash report `crash-*.txt`, if the build writes one (none is written today)
- [ ] screenshot of the SS2RS window only, showing no game art beyond the bug

**User names and personal paths removed** from the log and screenshots: yes / no

Never attach or paste game files (`*.gro`, `*.wld`, `*.tex`, `*.mdl`, `*.exe`, `*.dll`, other files from the game
folder), game dialogue or menu text, or decompiler output. Rules: [TRIAGE section 8](TRIAGE.md#8-game-content-in-reports).
````

**Where logs go today.** `ss2-app` calls `init_logging`, which writes to the terminal only. No log file is created.
Capture it yourself:

- Linux, macOS: `SS2_LOG=debug ./ss2-app 2>&1 | tee ss2.log`
- Windows PowerShell: `$env:SS2_LOG = "debug"; .\ss2-app.exe *>&1 | Tee-Object -FilePath ss2.log`

**Designed, not wired** (no binary calls these):

- `ss2_core::logfile` writes `ss2.log` (rotated at 5 MiB, five old files kept) through `init_logging_with_file`.
- `ss2_core::crash` writes `crash-<unix time>.txt` on panic with the last 50 log lines, through `install_panic_hook`
  (tested in `crates/ss2-core/tests/crash_hook.rs`). Today a panic prints to stderr only; a backtrace needs `RUST_BACKTRACE=1`.
- The folder is `PlatformPaths::log_dir` (`crates/ss2-platform-api/src/paths.rs`): `%LOCALAPPDATA%\SS2RS\logs` on Windows,
  `~/.local/state/SS2RS/logs` on Linux (on Windows and macOS the `directories` crate has no state directory, so the code
  falls back to the local data directory). `PlatformPaths::resolve` is called by no binary, so portable mode is inactive.

**Wired:** `settings.toml` at `Settings::default_path()` (`%APPDATA%\SS2RS\settings.toml` on Windows,
`crates/ss2-core/src/settings.rs`), read by the app and written when the Options screen saves.

## 4. Privacy

**No telemetry by default.** Checked 2026-11-22: a search of `crates/` for `reqwest`, `ureq`, `hyper`, `isahc`, `curl`,
`sentry`, `telemetry`, `analytics`, `std::net` and `tokio` finds no network or analytics code. `Cargo.lock` has no package
named `reqwest`, `hyper`, `ureq`, `isahc`, `curl`, `sentry`, `rustls` or `native-tls`. There is no update check
([PACKAGING](PACKAGING.md) section 5), and multiplayer is not implemented. This is a search, not an audit. Repeat it before
each playtest:

```
grep -rniE "reqwest|ureq|hyper|isahc|curl|sentry|telemetry|analytics|std::net|tokio" crates --include=*.rs
```

- The only data that leaves a machine is what a tester chooses to post to GitHub. Crash reports are text the tester
  attaches by hand; nothing uploads them. Paths in a report's location and backtrace can contain the user name
  (`C:\Users\<name>\...`). **Redact before posting:** the user name in every path, the machine name if it shows, and
  personal file names ([TRIAGE section 8](TRIAGE.md#8-game-content-in-reports)).
- **Future telemetry.** PROPOSED: none is added without an RFC in
  [docs/rfcs/](https://github.com/Ihor-Ihorevych/SS2RS/tree/main/docs/rfcs) covering data, opt-in, storage and removal.

## 5. Which template for which finding

Use the template in `.github/ISSUE_TEMPLATE/` that matches ([TRIAGE section 1](TRIAGE.md#1-intake)):

| Finding | Template | Labels the template sets |
|---|---|---|
| Exit, panic, freeze, hang | `crash.md` | `bug`, `crash` |
| Co-op peers disagree | `desync.md` | `bug`, `desync` |
| Rendering differs from the original | `visual-parity.md` | `parity`, `visual` |
| Numbers, timing or behaviour differ from the original | `parity.md` (checklist rows: [PARITY.md](PARITY.md)) | `parity`, `gameplay` |
| Anything else that behaves wrongly | `bug.md` | `bug` |
| Proposed roadmap task | `task.md` | `task` |
| Security problem | none: private report, [SECURITY.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/SECURITY.md) | - |

A session with no single finding: one `bug.md` issue per finding, or the section 3 form as a comment on the tracking
issue. PROPOSED: a playtest template (none exists).

## 6. Crash-report triage

**Format.** A crash report ([crash.rs](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/crates/ss2-core/src/crash.rs)) has,
in order: the line `SS2RS crash report`; `Version`; `System` (OS and architecture); `Thread`; `Panic` (message);
`Location` (`file:line:column` or `unknown`); a `Backtrace:` block; and `Last N log lines:`. A missing part is a `needs-info` request.

**Duplicates.** Search open and closed issues for the exact `Panic:` text and the `Location:` value. The same message and
location in the same version: apply `duplicate`, link the canonical issue, close ([TRIAGE section 5](TRIAGE.md#5-duplicates-and-missing-information)).
Locations move with the code, so compare locations within one version only.

**Labels** ([TRIAGE section 2](TRIAGE.md#2-labels)), checked with `gh label list` on 2026-11-22:

- Exist: `bug`, `duplicate`, `wontfix`, `prio:P0`-`prio:P3`, `track:T00`-`track:T13`, `spec`, `spike`, `phase-2`.
- **Do not exist:** `crash`, `desync`, `parity`, `visual`, `gameplay`, `task`, `severity:S1`-`severity:S4`, `needs-info`,
  `not-reproduced`.
- **Mismatch:** TRIAGE says `wont-fix`; the repo has `wontfix`. PROPOSED: use `wontfix` and fix TRIAGE.
- A maintainer creates missing labels before first use. Until then, template labels are not applied.

**Severity** ([TRIAGE section 3](TRIAGE.md#3-severity-and-priority)): S1 crash, hang, data loss or desync (P0); S2 blocks a
level, weapon, enemy or mission (P1); S3 small, with a workaround (P2); S4 cosmetic (P3). Raise one level with no workaround
that hits every player. A difference the original game shares is closed as `wontfix`.

**Who is notified.** The reporter, by comment, for acknowledgement and `needs-info`. The owners in
[.github/CODEOWNERS](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/.github/CODEOWNERS) for the crate named in `Location`.
S1 issues go to the next planning round ([TRIAGE section 6](TRIAGE.md#6-response-goals)).

**Response targets** (PROPOSED; the same goals as [TRIAGE section 6](TRIAGE.md#6-response-goals)): first response 7 days;
S1 crash or desync acknowledged in 3 days; S2 to S4 in the weekly triage pass; a playtest round triaged within 7 days of
closing. Missing information closes after 30 days ([TRIAGE section 5](TRIAGE.md#5-duplicates-and-missing-information)).

## 7. How findings enter the roadmap

1. **Triage** the issue (section 6; [TRIAGE section 4](TRIAGE.md#4-triage-steps)).
2. **Small local fix:** a PR saying `Fixes #<n>`. No task ([TRIAGE section 7](TRIAGE.md#7-when-an-issue-becomes-a-roadmap-task)).
3. **Needs a spec, an RFC or crosses crates:** a `task.md` issue, then a roadmap task in `docs/roadmap/` (IDs are never
   renumbered). Claim it on `main` first ([AGENTS.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/AGENTS.md) section 1).
4. **Original-game behaviour not yet written down:** a spec task in `docs/re/` (`docs/re/README.md`), prose only, by a spec
   author under [CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md). Testers do not write specs
   and do not post original-game internals.
5. **Parity evidence:** a checklist row in [PARITY.md](PARITY.md), signed off in the PR that completes it.

## 8. Schedule and roles

Roles, not people. One person may hold several.

| Step | What happens | Role |
|---|---|---|
| 1. Announce | Tracking issue: channel, build, areas, deadline, the section 3 rules | maintainer |
| 2. Build | Build the archive on a machine the maintainer controls; run [PACKAGING](PACKAGING.md) section 4 checks; record the commands in the PR | maintainer |
| 3. Run | Install, run, fill in the section 3 form | tester |
| 4. Collect | Gather the round's issues; apply the section 5 template and section 6 labels | triager |
| 5. Triage | Completeness, game-content check, cause, duplicates, severity, roadmap decision (section 7) | triager; maintainer for S1 |
| 6. Close | Summary on the tracking issue; close it; count crashes for PC_1_0_CHECKLIST row 8 | maintainer |

## 9. Gaps and open questions

1. No channel is built by CI (Actions blocked); no scheduled `nightly` exists (T12-008).
2. No binary writes a log file or crash report, and portable mode is inactive (section 3). The roadmap marks T03-016, T03-019,
   T03-012 and T03-020 done, but their code is not called by the binaries.
3. `--version` omits the commit; pre-release version strings are untested.
4. No playtest label, template or board exists; TRIAGE and the templates use labels that do not exist (section 6), and
   `wont-fix` and `wontfix` differ.
5. The telemetry rule (section 4) is PROPOSED; there is no crash-free measurement (PC_1_0_CHECKLIST row 8).
6. No channel for stores or package managers ([PACKAGING](PACKAGING.md) section 6). The packaged README does not yet explain `SS2_DATA_DIR`.
