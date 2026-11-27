# T00 — Repo, Infrastructure, Governance & Contracts

Owner crates: workspace root, `ss2-core`, `ss2-testkit`. Goal: **M0** — everyone can start in parallel.

## Repository & tooling
- [x] **T00-001** · Workspace skeleton · `S` · `P0`
  Cargo workspace with empty crates from the design's crate map, `rust-toolchain.toml` (pinned stable), edition, `[workspace.dependencies]`, `[workspace.lints]`, MSRV policy. Done when: `cargo build --workspace` passes; every crate has `README` + doc comment owner.
- [x] **T00-002** · License & dependency policy · `S` · `P0` · 🟢
  `LICENSE` (Apache-2.0, already in the repo), `NOTICE`, `license = "Apache-2.0"` in every crate, `deny.toml` (allow-list MIT/Apache/BSD/ISC/Zlib/MPL; deny GPL/AGPL unless owner approves), DCO bot config. Done when: `cargo deny check` is in CI.
- [x] **T00-003** · CI: build/test/lint · `M` · `P0`
  GitHub Actions: Win/Linux/macOS, `fmt`, `clippy -D warnings`, `test`, cache, nightly scheduled job. Done when: required checks block merges.
- [x] **T00-004** · CI: portability guard (non-blocking) · `S` · `P2`
  Optional job: `cargo check` for `aarch64-linux-android` on portable crates only, so PC code doesn't accumulate desktop-only assumptions before the Phase 2 port. Failures are warnings, not merge blockers, until M8. Done when: job runs nightly.
- [x] **T00-005** · Clean-room & provenance policy · `S` · `P0`
  `CLEANROOM.md`: roles (spec author vs implementer), what may be read (specs, public docs, own measurements; SS1 for concepts only), what must never be committed. PR template checkbox + provenance field; `docs/re/` CODEOWNERS requiring RE-team review. Done when: policy merged and linked from CONTRIBUTING.
- [x] **T00-006** · Proprietary-content guard · `S` · `P0` · 🟢
  `.gitignore` for `Serious Sam 2/`, `*.gro`, `*.dll`, `*.exe`, Ghidra projects; CI script rejecting added files with proprietary extensions or >5 MB blobs; secrets scan. Done when: negative test PR is rejected.
- [x] **T00-007** · CONTRIBUTING & labels · `S` · `P0` · 🟢
  Workflow from roadmap §3/§10, commit style, DCO how-to, label set (`track:T04`, `size:M`, `prio:P0`, `spike`, `spec`, `good-first`).
- [x] **T00-008** · Roadmap → GitHub issues sync · `M` · `P1`
  Script (`tools/roadmap-sync`) that parses these markdown files and creates/updates issues by ID, idempotently; reflects checkbox state. Done when: dry-run diff + live run on a scratch repo.
- [x] **T00-009** · `ss2-testkit` data-gated tests · `S` · `P0`
  `require_game_data!()`, `game_data_dir()`, `SS2_DATA_DIR`, skip-with-message semantics, helpers to open the VFS once per test binary. Done when: example test skips on CI, runs locally.
- [x] **T00-010** · Synthetic fixtures policy & generators · `S` · `P1`
  Folder `testdata/` with license-clean tiny hand-made files; builder utilities (write minimal CTSEMETA/zip) so parser tests don't need real data.
- [x] **T00-011** · Error/logging conventions in `ss2-core` · `S` · `P0`
  `tracing` setup, `thiserror` error style, `Result` aliases, panic policy (no panics on bad input), log targets per crate.
- [x] **T00-012** · Coding standards & sim-safe rules · `S` · `P0`
  `unsafe` policy, naming, module layout, determinism rules for sim-path crates (+ clippy `disallowed_types` for `HashMap`/`Instant` in them).
- [x] **T00-013** · RFC process · `S` · `P0` · 🟢
  `docs/rfcs/0000-template.md`, numbering, review SLA (3 working days), who approves (crate owner + 1).

## Contracts (RFC → merged `v0` stub crate with mocks)
Each contract task yields: RFC, public trait/struct stub, a mock implementation, a doc example. Consumers may start the moment the stub merges.
- [x] **T00-014** · Contract: `Vfs` + `AssetLoader` + `AssetManager` API · `M` · `P0` · Deps: T00-001, T00-013
- [x] **T00-015** · Contract: Renderer front-end (`RenderWorld` extract, `Renderable`, `Camera`, material handles, 2D/UI draw list) · `M` · `P0`
- [x] **T00-016** · Contract: Platform events, input actions, app lifecycle (suspend/resume, surface lost), file-system locations · `M` · `P0`
- [x] **T00-017** · Contract: Sim — entity model, component/property reflection, tick API, replication attributes, event bus: `docs/rfcs/0009-sim-contract.md` (**in review**): `ss2_sim::{EntityId, ClassRegistry, World, SimpleWorld}` and a conformance suite for any real `World`; the tick loop and event bus already existed; **behaviour (lifecycle callbacks, spawn-from-record) and the real store are T08-002** · `L` · `P0` · Deps: T08-001
- [x] **T00-018** · Contract: Audio API (sound handles, 3D emitters, music, buses) · `S` · `P1`
- [x] **T00-019** · Benchmark harness & perf budgets doc (criterion, per-platform budgets) · `M` · `P1`
- [x] **T00-020** · Docs site (mdBook + rustdoc deploy), architecture overview page · `M` · `P1` · 🟢
- [x] **T00-021** · Release engineering: semver policy, changelog, `cargo-release`, artifacts naming · `M` · `P2`
- [x] **T00-022** · Code of conduct, security policy, community channels (chat, forum) · `S` · `P1` · 🟢
- [x] **T00-023** · Naming/branding & trademark disclaimer ("not affiliated with Croteam/Devolver"), logo-free repo · `S` · `P1`
- [x] **T00-024** · Dependency update bot & vulnerability alerts (Dependabot/renovate, `cargo audit`) · `S` · `P2` · 🟢
- [x] **T00-026** · Developer tools list `docs/DEV_TOOLS.md` (required/recommended/by-track: Git+gh, rustup, MSVC/C toolchain, cargo-deny, Ghidra+JDK, ImHex, cargo-fuzz, RenderDoc, naga-cli, FFmpeg, mdbook, nextest, llvm-cov, …) linked from CONTRIBUTING and README · `S` · `P0` · 🟢
- [x] **T00-027** · `cargo xtask doctor`: checks the contributor's machine against `docs/DEV_TOOLS.md` (required vs role tools, versions, `SS2_DATA_DIR`, `core.hooksPath`), prints install hints, exit 1 only for missing *required* tools · `M` · `P1` · Deps: T00-026
- [x] **T00-028** · `cargo xtask claim <ID> [--who HANDLE]` / `release <ID> --reason …`: automates the `[INPRG]` protocol from `AGENTS.md` §1 — pull, verify the box is `[ ]` and deps are `[x]`, edit the line (tag with handle/branch/date), signed-off commit `ID: [INPRG] claim (@handle) [skip ci]`, push to `main`, detect a lost race and exit non-zero without overwriting; `release` restores `[ ]` with a `[RELEASED]` commit · `M` · `P0` · Deps: T00-008 parser
- [x] **T00-029** · Claim consistency check in CI + stale report: fail if a line has both `[x]` and `INPRG`, an `INPRG` tag lacks handle/branch/date, two tags exist on one task, or a claimed task's deps are not done; weekly job lists claims older than 7 days without pushes on their branch · `M` · `P1` · Deps: T00-028
- [x] **T00-025** · Crate publishing policy (which crates are libraries on crates.io, name reservation) · `S` · `P3`
- [ ] **T00-030** · Community channels: set up a chat server/forum, link it from README and CONTRIBUTING (split from T00-022, which delivered the code of conduct and security policy) · `S` · `P1` · 🟢
- [ ] **T00-031** · Final project name decision (SS2RS is a working title) and rename of crates/docs if it changes; trademark review of the name (split from T00-023, which delivered the affiliation disclaimers in README and NOTICE) · `S` · `P1`
- [x] **T00-033** · Minimal CTSEMETA writer for fixtures in `ss2_testkit::fixtures` (split from T00-010, which delivered the ZIP builder and fixture policy) · `S` · `P1` · Deps: T01-004, T01-005
- [x] **T00-032** · CI cost: docs-only pushes to `main` skip CI (`paths-ignore` on the push trigger; pull requests unfiltered) and claim commits end with `[skip ci]` · `S` · `P1` · 🟢
- [x] **T00-034** · Standing rule: an agent merges its own PR as soon as all checks are green (merge commit, never squash; never red/pending/conflicting; never others' PRs) · `S` · `P1` · 🟢
- [x] **T00-035** · Playable MVP plan: design doc, RFC 0013 (level scene boundary between format readers, world assembly, the 3D pass and the app) and the roadmap slices that implement it (T02-035..038, T03-036/037, T04-037/038, T07-027..030, T10-034) · `S` · `P0`
- [x] **T00-036** · Sky and lightmap plan: design doc, RFC 0013 amendment (`SkyData` in `LevelScene`, the face orientation convention) and the roadmap slices (T07-032, T04-039, T02-039, T07-033, T03-038) · `S` · `P0`
