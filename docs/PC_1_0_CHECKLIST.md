# PC 1.0 release checklist and go/no-go criteria

The checklist a maintainer works through before tagging **PC 1.0** (the `1.0.0` release, milestone M8).
Each criterion has evidence and an owner role. The go/no-go table in section 9 is the decision record.

Status: **draft (T12-017)**. Nothing has been released yet (see [RELEASING.md](RELEASING.md)). Items
marked **PROPOSED** are targets for maintainers to confirm; they are not agreed yet.

## 1. Scope of 1.0

- **Platforms:** PC only. Windows, then Linux and macOS (roadmap M2 and M8; [design spec](superpowers/specs/2026-11-21-ss2rs-design.md)). Android and other platforms are later phases: M9 "Android port" starts only after M8, and
  M10 covers other platforms. Every `PHASE 2` task is out of scope for 1.0.
- **Content:** the campaign is complete (M7): all campaign levels completable, all weapons, enemies and
  bosses, difficulty levels, saves, cutscenes and video.
- **Co-op:** co-op is part of 1.0 (netcode milestones M4 to M6 precede M8). Section 5 lists what must be
  shown.
- **Mods:** Renovation and InSamnity2 ported (M8, roadmap T13).
- **Data:** the engine plays only from the player's own install, pointed to with `SS2_DATA_DIR`.
  No game content ships.
- **Definition of 1.0 in the roadmap:** M8 exit criteria, in [roadmap section 5](roadmap/README.md#5-milestones-and-exit-criteria).
  If this checklist and the roadmap disagree, the roadmap wins and this file is corrected in a PR.

## 2. Performance

Source of the budgets: [PERF_BUDGETS.md](PERF_BUDGETS.md). The numbers are defined there and are
**proposed** until T12-004 measures them on the reference PCs and the owners accept them. This
checklist does not repeat them.

- [ ] The reference machines are fixed (Low, Mid, High tiers in PERF_BUDGETS section 2). TBD by T12-004.
- [ ] Frame-time budgets (PERF_BUDGETS section 3) are met on **Mid** for the 1.0 scene set. TBD by T12-004.
- [ ] Frame-time budgets on **Low** are met (the "still playable" floor). TBD by T12-004.
- [ ] Memory budgets (PERF_BUDGETS section 4): process and GPU memory in a level, on each tier. TBD by T12-004.
- [ ] Load-time budgets (PERF_BUDGETS section 4): cold start, warm level load. TBD by T12-004.
- [ ] No hitches above the threshold in PERF_BUDGETS section 3 in a 10-minute run on Mid.
- [ ] Micro-benchmarks compared against the last release on the same machine; any change more than 10 %
  slower has the sentence required by PERF_BUDGETS section 6.
- [ ] Any breach that remains is recorded as an issue against the subsystem owner (PERF_BUDGETS section 6).

Budgets may move only by PR to PERF_BUDGETS.md with measurements (its section 6).

## 3. Parity with the original game

Source of the process: [PARITY.md](PARITY.md). Templates are in `docs/parity/templates/`.

- [ ] Every campaign level has a level checklist (PARITY section 1.1) signed off, or every open row is
  listed as `differs` with a parity issue number.
- [ ] Every weapon in the campaign has a weapon checklist (PARITY section 1.2), signed off in the same way.
- [ ] Every enemy and boss in the campaign has an enemy checklist (PARITY section 1.3), signed off in the
  same way.
- [ ] Every row is `match`, `differs` or `not checked`. A `not checked` row has a reason in the Notes column.
- [ ] Every `differs` row is either fixed and re-measured, or an **accepted difference** with a reason
  from PARITY section 4 and the owners who accepted it.
- [ ] Each checklist with a P0 item has a second signer (PARITY section 3).
- [ ] Evidence is text or CSV, with no game files, game text or screenshots of game content
  (PARITY section 2).

## 4. Stability

- [ ] **Crash-free sessions: PROPOSED at 99 % over the beta period**, counted as sessions that end without
  a crash or hang, from playtest reports (T12-011) and soak runs. Telemetry is off by default, so the
  count comes from reports, not from automatic data. The method for counting is not yet defined.
- [ ] **No open S1 crash or data-loss issue** in the triage labels of [TRIAGE.md](TRIAGE.md) section 3.
- [ ] Automated test layers in [TESTING.md](TESTING.md) section 2 pass on `main`: unit, synthetic fixture,
  data-gated (on a machine with the data, see T12-002), and golden image.
- [ ] Crash reports can be filed with the logs the build provides. The crash handler (T03-016) is either in
  the build or its absence is stated in the release notes.
- [ ] **Soak runs:** the soak layer is planned (TESTING section 3.8, T12-016, T08-029). Until it exists
  the target cannot be checked. **PROPOSED:** a long co-op bot run with no leak trend and no desync,
  repeated over several nights before the release, once T12-016 is done.
- [ ] Save files written by 1.0 load in 1.0 (the save format version from RELEASING section 1 is recorded).

## 5. Co-op and netcode

The netcode is defined by T08 and M4 to M6. This section says what must be **demonstrated** for 1.0,
not how it is built.

- [ ] Dedicated server and listen server both run a full campaign level in co-op on PC (T08-004, T08-020).
- [ ] Two or more PC clients join the same level; join-in-progress works (T08-013).
- [ ] Prediction is smooth at 100 ms latency and 2 % loss in the network simulator (T08-017, M4 exit
  criterion).
- [ ] Co-op session with the same level and inputs shows **no desync** between peers for the length
  agreed in the soak plan. **PROPOSED:** no desync in the bot sessions of T08-029 and the soak runs of
  T12-016, and no desync report that is still open at S1.
- [ ] The protocol version check refuses mismatched clients cleanly (T08-006, T08-030; RELEASING section 1).
- [ ] Bandwidth per tick is measured and recorded (T08-018), so that the 1.0 numbers are known.

No roadmap task yet names a desync detector inside the engine. T08-029 and T12-016 assert on desync in
test runs. The owner of an in-engine detector is for maintainers to decide.

## 6. Packaging and install

Source of the process: [RELEASING.md](RELEASING.md).

- [ ] Release files are named and built as RELEASING section 4 says, for each PC platform in scope.
- [ ] Each archive contains exactly the files listed in RELEASING section 4 (`ss2-app`, `ss2-server`,
  `LICENSE`, `NOTICE`, `README.md`) and no game content. Checked by hand on each archive.
- [ ] `SHA256SUMS` matches the archives (T12-008).
- [ ] Each archive starts and runs `ss2-app --smoke-frames 5` on a clean machine (RELEASING section 3, step 6).
- [ ] Windows: installer or portable zip, DPI-awareness manifest, Windows 10 or later as the baseline
  (T03-026). The baseline is recorded in the release notes.
- [ ] Linux and macOS packages as decided by T12-009, T03-011 and T03-010. If a platform is not packaged
  for 1.0, the release notes say so.
- [ ] Unsigned binaries are stated in the release notes until code signing is done (RELEASING section 6;
  T03-026, T03-010).
- [ ] Players are told how to point the engine at their own install (`SS2_DATA_DIR`) in the packaged
  README and the user docs (section 9).

## 7. Legal and clean room

Source of the process: [CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md) and [AGENTS.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/AGENTS.md) section 2.

- [ ] `cargo xtask guard` passes on `main` and on the release commit. No game content is tracked, and no
  game content is in any release archive (RELEASING section 4).
- [ ] Clean-room record: every spec in `docs/re/` used by an implementer is traceable, and no decompiler
  output or SS1 code is in the repository (CLEANROOM; T12-010 for the audit log).
- [ ] DCO: every commit in the release range has a `Signed-off-by` line (`dco` job in CI; the loop in
  TESTING section 4 can be run by hand).
- [ ] `LICENSE` (Apache-2.0) and `NOTICE` are present and correct. `NOTICE` carries the "not affiliated
  with Croteam or Devolver Digital" statement and the trademark line.
- [ ] Third-party licenses: `cargo deny check` passes, and the license report for the release is attached
  to it (T12-010, RELEASING section 6). The SBOM is part of T12-010 and is open.
- [ ] Bundled assets (for example the Inter font, under OFL-1.1) are listed with their license files.
- [ ] The **"unofficial fan engine" notice** is shown in the README, in the packaged files, and on the
  credits and legal screen (T10-016). The wording is for maintainers to approve.
- [ ] **Name and trademark:** the final project name is decided and the trademark review is done (T00-031).
  If the name changes, crates and docs are renamed under that task before the release tag.
- [ ] The release notes state that the player needs their own legally obtained copy of Serious Sam 2.

## 8. Documentation and support

- [ ] User docs exist: install and point to your data, FAQ, troubleshooting, supported game versions
  (T12-014). Open until that task is done.
- [ ] Docs site builds and its links resolve (`mdbook build docs` and `check_book_links.py`, as in the
  `docs` CI job).
- [ ] Known-issues list for 1.0 is written and linked from the release notes. Every open S1 and S2 issue
  is on it. No such file exists yet.
- [ ] Issue templates in `.github/ISSUE_TEMPLATE/` (crash, desync, visual parity, gameplay parity, bug,
  task) are current (T12-015).
- [ ] Triage follows [TRIAGE.md](TRIAGE.md): labels exist, and every S1 is acknowledged within the target
  in its section 6.
- [ ] `CHANGELOG.md` has a section for 1.0 with one line per user-visible change, and the entries for
  accepted parity differences are under **Changed** (RELEASING section 2).
- [ ] Security reports go through the private path in [SECURITY.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/SECURITY.md).

## 9. Go/no-go

Each row must be **met** before the tag, or the decision is **no-go** for that row. "Open" means the
evidence does not exist yet. The status column is updated in the release PR. As of this draft, every row is
open because nothing has been released and the project is pre-alpha.

| # | Criterion | Evidence required | Owner role | Status |
|---|---|---|---|---|
| 1 | M8 exit criteria met | Roadmap track files show the M8 tasks `[x]` and merged PRs | Maintainers | Open |
| 2 | Frame and memory budgets (Mid, and Low floor) | T12-004 measurement report on the reference PCs, against PERF_BUDGETS sections 3 and 4 | Performance owner (T12) and subsystem owners | Open |
| 3 | Load and start times | T12-004 report, cold start and warm level load | Performance owner (T12) | Open |
| 4 | Level checklists | One signed `docs/parity/level` checklist per campaign level, or open rows listed | Parity owner (T12) | Open |
| 5 | Weapon checklists | One signed `docs/parity/weapon` checklist per campaign weapon | Parity owner (T12) and gameplay owner (T09) | Open |
| 6 | Enemy checklists | One signed `docs/parity/enemy` checklist per campaign enemy and boss | Parity owner (T12) and gameplay owner (T09) | Open |
| 7 | Accepted parity differences | Each `accepted difference` row has a reason and named owners | Maintainers | Open |
| 8 | Crash-free sessions (PROPOSED 99 %) | Session counts and crash reports from the beta period (T12-011) | QA owner (T12) | Open |
| 9 | No open S1 crash or data loss | Triage labels: zero open `severity:S1` issues | Triage lead | Open |
| 10 | Soak runs (PROPOSED: no leaks, no desync) | T12-016 run logs, once the farm exists | QA owner (T12) and network owner (T08) | Open |
| 11 | Co-op: listen and dedicated server, join-in-progress | Recorded session on PC, with the session steps in the release notes | Network owner (T08) | Open |
| 12 | Netsim criterion (100 ms, 2 % loss) | Netsim run result from T08-017 | Network owner (T08) | Open |
| 13 | No desync in co-op test sessions (PROPOSED) | Bot and soak run logs (T08-029, T12-016) | Network owner (T08) | Open |
| 14 | Protocol version check | Mismatched-client test result (T08-006) | Network owner (T08) | Open |
| 15 | Release archives and checksums | Archive list matches RELEASING section 4; `SHA256SUMS` verified | Release manager | Open |
| 16 | Smoke run on each packaged platform | `ss2-app --smoke-frames 5` log per archive | Release manager | Open |
| 17 | Windows baseline and DPI manifest | T03-026 done; baseline in release notes | Platform owner (T03) | Open |
| 18 | No game content in repo or archives | `cargo xtask guard` pass; archive listing checked | Legal and clean-room owner | Open |
| 19 | License report and SBOM | `cargo deny check` pass; SBOM and license report attached (T12-010) | Legal and clean-room owner | Open |
| 20 | Name and trademark decision | T00-031 done; NOTICE and README updated | Maintainers | Open |
| 21 | Unofficial fan engine notice | Notice present in README, packaged files, credits screen (T10-016) | Maintainers | Open |
| 22 | User docs and known issues | T12-014 done; known-issues list linked from release notes | Docs owner (T12) | Open |
| 23 | Changelog and release notes | `CHANGELOG.md` section for 1.0; release notes reviewed | Release manager | Open |

**Who decides:** the maintainers. A go is a decision by the maintainers listed in `.github/CODEOWNERS`
for the affected areas, recorded in the release PR or its tracking issue. No single contributor, and no
automated job, makes the decision.

**What happens on a no-go:**

1. The release PR is not merged and no tag is pushed (RELEASING section 3, step 4).
2. The failing rows are recorded in the tracking issue with their evidence or the lack of it.
3. Each failing row gets an issue against its owner, labelled with its severity and priority (TRIAGE
   sections 2 and 3), and, if it needs planned work, a roadmap task (TRIAGE section 7).
4. A pre-release (`-beta.N` or `-rc.N`, RELEASING section 5) may be cut for playtesting instead, if the
   maintainers decide so. It is not the 1.0 tag.
5. The checklist is re-run from section 1 once the rows are fixed.
