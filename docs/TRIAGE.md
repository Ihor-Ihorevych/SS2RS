# Bug triage

How a bug report becomes a labelled, prioritised item, and where it goes next. This is a volunteer
project: the targets in §6 are goals, not guarantees.

Roadmap files in `docs/roadmap/` remain the source of truth for planned work. Issues mirror them
(`cargo xtask roadmap-sync`); an issue that needs planned work links to a roadmap task (§7).

## 1. Intake

Use the template that matches the report (`.github/ISSUE_TEMPLATE/`):

| Template | Use it for |
|---|---|
| Crash (`crash.md`) | the engine exits, panics, hangs or freezes |
| Desync (`desync.md`) | two or more co-op peers disagree about game state |
| Visual parity (`visual-parity.md`) | a rendering difference from the original game |
| Gameplay parity (`parity.md`) | a difference in numbers, timing or behaviour from the original game |
| Bug (`bug.md`) | anything else that behaves wrongly |
| Task (`task.md`) | a proposed roadmap task |

Security problems must not be filed as issues. Use the private report described in [SECURITY.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/SECURITY.md).

## 2. Labels

Every triaged issue carries:

- **Type:** `bug`, plus `crash`, `desync`, `parity`, `visual` or `gameplay` as the template sets them.
- **Severity:** exactly one of `severity:S1` to `severity:S4` (§3).
- **Priority:** one of `prio:P0` to `prio:P3`, as in the roadmap (§3).
- **Area:** `track:Txx` (for example `track:T08` for netcode).
- **Status,** when it applies: `needs-info`, `duplicate`, `not-reproduced`, `wont-fix`.

The `crash`, `desync`, `visual`, `gameplay`, `severity:*` and status labels are not all on the
repository yet; a maintainer creates any that are missing before first use.

## 3. Severity and priority

Severity describes the effect on the player. Set it from evidence in the report. When the evidence
is unclear, pick the higher level and lower it after reproduction.

| Severity | Meaning | Default priority | Examples |
|---|---|---|---|
| **S1** critical | Crash, hang or data loss on a supported path during normal play; save or config corruption; **desync** between co-op peers | P0 | Engine panics when loading the first level from an unmodified retail install; two peers' enemy positions diverge after five minutes of co-op; a save file is overwritten |
| **S2** major | Gameplay or parity break that blocks a level, weapon, enemy or mission from working as in the original, or a crash in an optional feature | P1 | An enemy's health differs from the original by a measured amount; a trigger never fires; a weapon cannot be fired after a reload; a crash when a specific video file plays |
| **S3** minor | Wrong behaviour that is small and recoverable, or a workaround exists | P2 | A HUD number is off by one; a sound is mixed too loud; a load time misses the budget in [PERF_BUDGETS.md](PERF_BUDGETS.md) |
| **S4** cosmetic | Visual or text defect with no effect on play | P3 | One-pixel shimmer on a texture edge; a typo in a log message; a menu item one pixel out of line |

Adjustments:

- Raise one level when the bug has no workaround and affects every player.
- A bug that happens only on one platform or GPU keeps its level; put the platform in the title.
- If the **original** game behaves the same way in the same scenario, the difference is not a parity
  bug. Close it as `wont-fix` with a note that says so.

## 4. Triage steps

A maintainer, or any contributor acting as triager, works through these steps:

1. **Check completeness.** The template fields are present: version or commit, OS, GPU and driver,
   whether `SS2_DATA_DIR` is an unmodified retail install and its patch level, steps, expected and
   actual behaviour, and logs. If something is missing, apply `needs-info` (§5).
2. **Check for game content.** If the report attaches or quotes game files, remove the content at
   once (§8) and tell the reporter why. Do not re-share it.
3. **Classify the cause.** Decide whether this is an engine bug, a problem with the installed data
   (wrong version, modified files, missing files), or a difference that the original game shares.
   For data problems, ask the reporter to run `ss2-doctor <folder>` (T11-013; from a source checkout: `cargo run -p ss2-vfs --bin ss2-doctor -- <folder>`) and paste its output.
4. **Reproduce if you can.** Note the result in the issue. A report that cannot be reproduced after
   two requests for information is closed as `not-reproduced`.
5. **Label and prioritise** as in §2 and §3.
6. **Check for duplicates** (§5).
7. **Decide whether it needs a roadmap task** (§7).

## 5. Duplicates and missing information

**Duplicates.** Search open and closed issues for the symptom, the level or the log message before
labelling. If a report repeats an existing one, apply `duplicate`, link the canonical issue in a
comment, and close. Copy any new logs or repro steps into the canonical issue first.

**Needs more information.** Apply `needs-info` and comment with the exact fields that are missing.
Do not ask for something the template already requested. When the reporter has not replied in
30 days, close the issue with a note that it can be reopened once the information is available.

## 6. Response goals

These are targets for volunteers, not commitments:

- First response (acknowledgement, or a request for information) within **7 days**.
- S1 reports acknowledged within **3 days**, and considered in the next planning round.
- A triage pass over new issues **once a week**.

If a target is missed, the issue stays open and carries no penalty. Anyone may pick up triage.

## 7. When an issue becomes a roadmap task

Open a roadmap task (through the Task template) when the fix:

- needs a spec, an RFC or new behaviour that is not yet written down;
- crosses crate boundaries or changes a contract (`CODEOWNERS`, `docs/rfcs/`);
- belongs to a class of problems (for example several desync reports with one cause).

Small, local fixes do not need a task. Their PR still says `Fixes #<n>` in the description.

A roadmap task ID is never renumbered. Claims follow [AGENTS.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/AGENTS.md) §1, so the task is
claimed on `main` before work starts. The issue links the task ID and is closed when the PR merges.

## 8. Game content in reports

SS2RS never contains Croteam's game content, and neither may a report. Reports must not attach
or paste:

- game archives or data files: `*.gro`, `*.wld`, `*.tex`, `*.mdl`, `*.skl`, `*.mtr`, `*.ep`, `*.ogv`,
  and any other file from the game folder;
- executables or libraries from the game or its installer: `*.exe`, `*.dll`;
- text copied from the game (dialogue, help files, menus, strings taken from binaries);
- decompiler output or Ghidra project files.

Allowed:

- engine logs, with user names and personal paths removed;
- command output and test results typed or pasted as text;
- hashes of files you own (for example the SHA-256 of `All_PC_01.gro`, from
  `Get-FileHash -Algorithm SHA256` in PowerShell or `sha256sum` elsewhere);
- measurements as numbers or CSV that you typed or generated yourself;
- file **names** (for example the patch file names in the install folder);
- screenshots of the SS2RS window, cropped to the bug, when they show no copied game art beyond
  what the bug needs. Maintainers may ask for a screenshot to be removed; describe the bug in words
  instead when in doubt.

If a report breaks these rules, a maintainer removes the attachment or edits the body and comments
where the content was removed. GitHub may keep earlier versions of an edited comment, so the
reporter should never attach game files in the first place.

## 9. Changes to this process

Changes to severity levels, labels or templates are made by pull request, titled with the task ID
that covers them, like every other change.
