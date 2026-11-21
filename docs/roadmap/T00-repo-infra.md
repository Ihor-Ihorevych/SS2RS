# T00 — Repo, Infrastructure, Governance & Contracts

Owner crates: workspace root, `ss2-core`, `ss2-testkit`. Goal: **M0** — everyone can start in parallel.

## Repository & tooling
- [x] **T00-001** · Workspace skeleton · `S` · `P0`
  Cargo workspace with empty crates from the design's crate map, `rust-toolchain.toml` (pinned stable), edition, `[workspace.dependencies]`, `[workspace.lints]`, MSRV policy. Done when: `cargo build --workspace` passes; every crate has `README` + doc comment owner.
- [~] **T00-002** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · License & dependency policy · `S` · `P0` · 🟢
  `LICENSE` (Apache-2.0, already in the repo), `NOTICE`, `license = "Apache-2.0"` in every crate, `deny.toml` (allow-list MIT/Apache/BSD/ISC/Zlib/MPL; deny GPL/AGPL unless owner approves), DCO bot config. Done when: `cargo deny check` is in CI.
- [~] **T00-003** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · CI: build/test/lint · `M` · `P0`
  GitHub Actions: Win/Linux/macOS, `fmt`, `clippy -D warnings`, `test`, cache, nightly scheduled job. Done when: required checks block merges.
- [~] **T00-004** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · CI: portability guard (non-blocking) · `S` · `P2`
  Optional job: `cargo check` for `aarch64-linux-android` on portable crates only, so PC code doesn't accumulate desktop-only assumptions before the Phase 2 port. Failures are warnings, not merge blockers, until M8. Done when: job runs nightly.
- [~] **T00-005** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · Clean-room & provenance policy · `S` · `P0`
  `CLEANROOM.md`: roles (spec author vs implementer), what may be read (specs, public docs, own measurements; SS1 for concepts only), what must never be committed. PR template checkbox + provenance field; `docs/re/` CODEOWNERS requiring RE-team review. Done when: policy merged and linked from CONTRIBUTING.
- [~] **T00-006** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · Proprietary-content guard · `S` · `P0` · 🟢
  `.gitignore` for `Serious Sam 2/`, `*.gro`, `*.dll`, `*.exe`, Ghidra projects; CI script rejecting added files with proprietary extensions or >5 MB blobs; secrets scan. Done when: negative test PR is rejected.
- [~] **T00-007** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · CONTRIBUTING & labels · `S` · `P0` · 🟢
  Workflow from roadmap §3/§10, commit style, DCO how-to, label set (`track:T04`, `size:M`, `prio:P0`, `spike`, `spec`, `good-first`).
- [~] **T00-008** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · Roadmap → GitHub issues sync · `M` · `P1`
  Script (`tools/roadmap-sync`) that parses these markdown files and creates/updates issues by ID, idempotently; reflects checkbox state. Done when: dry-run diff + live run on a scratch repo.
- [x] **T00-009** · `ss2-testkit` data-gated tests · `S` · `P0`
  `require_game_data!()`, `game_data_dir()`, `SS2_DATA_DIR`, skip-with-message semantics, helpers to open the VFS once per test binary. Done when: example test skips on CI, runs locally.
- [ ] **T00-010** · Synthetic fixtures policy & generators · `S` · `P1`
  Folder `testdata/` with license-clean tiny hand-made files; builder utilities (write minimal CTSEMETA/zip) so parser tests don't need real data.
- [x] **T00-011** · Error/logging conventions in `ss2-core` · `S` · `P0`
  `tracing` setup, `thiserror` error style, `Result` aliases, panic policy (no panics on bad input), log targets per crate.
- [~] **T00-012** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · Coding standards & sim-safe rules · `S` · `P0`
  `unsafe` policy, naming, module layout, determinism rules for sim-path crates (+ clippy `disallowed_types` for `HashMap`/`Instant` in them).
- [~] **T00-013** [INPRG @Ihor-Ihorevych/agent · branch `m0-ci` · 2026-11-21] · RFC process · `S` · `P0` · 🟢
  `docs/rfcs/0000-template.md`, numbering, review SLA (3 working days), who approves (crate owner + 1).

## Contracts (RFC → merged `v0` stub crate with mocks)
Each contract task yields: RFC, public trait/struct stub, a mock implementation, a doc example. Consumers may start the moment the stub merges.
- [ ] **T00-014** · Contract: `Vfs` + `AssetLoader` + `AssetManager` API · `M` · `P0` · Deps: T00-001, T00-013
- [ ] **T00-015** · Contract: Renderer front-end (`RenderWorld` extract, `Renderable`, `Camera`, material handles, 2D/UI draw list) · `M` · `P0`
- [ ] **T00-016** · Contract: Platform events, input actions, app lifecycle (suspend/resume, surface lost), file-system locations · `M` · `P0`
- [ ] **T00-017** · Contract: Sim — entity model, component/property reflection, tick API, replication attributes, event bus · `L` · `P0` · Deps: T08-001
- [ ] **T00-018** · Contract: Audio API (sound handles, 3D emitters, music, buses) · `S` · `P1`
- [ ] **T00-019** · Benchmark harness & perf budgets doc (criterion, per-platform budgets) · `M` · `P1`
- [ ] **T00-020** · Docs site (mdBook + rustdoc deploy), architecture overview page · `M` · `P1` · 🟢
- [ ] **T00-021** · Release engineering: semver policy, changelog, `cargo-release`, artifacts naming · `M` · `P2`
- [ ] **T00-022** · Code of conduct, security policy, community channels (chat, forum) · `S` · `P1` · 🟢
- [ ] **T00-023** · Naming/branding & trademark disclaimer ("not affiliated with Croteam/Devolver"), logo-free repo · `S` · `P1`
- [ ] **T00-024** · Dependency update bot & vulnerability alerts (Dependabot/renovate, `cargo audit`) · `S` · `P2` · 🟢
- [ ] **T00-026** · Developer tools list `docs/DEV_TOOLS.md` (required/recommended/by-track: Git+gh, rustup, MSVC/C toolchain, cargo-deny, Ghidra+JDK, ImHex, cargo-fuzz, RenderDoc, naga-cli, FFmpeg, mdbook, nextest, llvm-cov, …) linked from CONTRIBUTING and README · `S` · `P0` · 🟢
- [ ] **T00-027** · `cargo xtask doctor`: checks the contributor's machine against `docs/DEV_TOOLS.md` (required vs role tools, versions, `SS2_DATA_DIR`, `core.hooksPath`), prints install hints, exit 1 only for missing *required* tools · `M` · `P1` · Deps: T00-026
- [ ] **T00-028** · `cargo xtask claim <ID> [--who HANDLE]` / `release <ID> --reason …`: automates the `[INPRG]` protocol from `AGENTS.md` §1 — pull, verify the box is `[ ]` and deps are `[x]`, edit the line (tag with handle/branch/date), signed-off commit `ID: [INPRG] claim`, push to `main`, detect a lost race and exit non-zero without overwriting; `release` restores `[ ]` with a `[RELEASED]` commit · `M` · `P0` · Deps: T00-008 parser
- [ ] **T00-029** · Claim consistency check in CI + stale report: fail if a line has both `[x]` and `INPRG`, an `INPRG` tag lacks handle/branch/date, two tags exist on one task, or a claimed task's deps are not done; weekly job lists claims older than 7 days without pushes on their branch · `M` · `P1` · Deps: T00-028
- [ ] **T00-025** · Crate publishing policy (which crates are libraries on crates.io, name reservation) · `S` · `P3`
