# Publishing crates to crates.io

Status: **policy v0**. Nothing is published. This page is the decision of roadmap task
T00-025. It decides which workspace crates may become crates.io libraries, the gate they must pass,
and how names and versions are handled. It does not change any `Cargo.toml`.

## 1. Current state and expectations

- All 26 workspace members (23 crates under `crates/`, `tools/gro-tool`, `tools/net-tool` and `xtask`) set
  `publish = false` in their manifest. All of them inherit `license = "Apache-2.0"` and
  `edition = "2024"` from `[workspace.package]` in the
  [root `Cargo.toml`](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/Cargo.toml).
- [`release.toml`](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/release.toml) also sets
  `publish = false` for `cargo-release`, so a release cannot publish by accident.
- Expect `publish = false` to stay on every crate. It is removed from a crate only by a PR for that
  crate that passes every point of §3, names its task ID, and removes the `publish = false` line
  and the workspace-wide `publish = false` in `release.toml` is changed in the same PR.

## 2. Inventory and classes

Classes: **1** library candidate (publishable later, not now) · **2** internal engine crate, never
published · **3** binary, tool or xtask, never published · **4** undecided (decision pending, reason given).

| Crate | Manifest description (abridged) | Class | Reason |
|---|---|---|---|
| `ss2-vfs` | virtual file system over .gro archives and loose dirs | 1 | Contract v0 (RFC 0001) plus archive reader; code and tests use synthetic data only. Publish once its API is final |
| `ss2-core` | ids, errors, logging, time helpers | 4 | Base of `ss2-vfs` and `ss2-assets`; must be published with them or split first |
| `ss2-assets` | asset manager, loader registry, cooked cache | 4 | Depends on `ss2-vfs`; `AssetManager` task T02-009 is open |
| `ss2-meta` | CTSEMETA container and reflection reader | 4 | Good candidate (format reader, no game content by design), but the crate is a one-line stub; T02-006..008 are open |
| `ss2-render-api` | GPU-free renderer front-end contract | 4 | Depends on `ss2-ui`; reuse outside SS2RS not established |
| `ss2-platform-api` | windowing-free platform contract | 4 | Frozen contract (RFC 0003); reuse outside SS2RS not established |
| `ss2-audio-api` | device-free audio contract | 4 | Frozen contract (RFC 0004); reuse outside SS2RS not established |
| `ss2-shaderxlat` | D3D9 shader assembly translation | 4 | Possibly reusable; provenance of its inputs must be documented first (T05, CLEANROOM.md) |
| `ss2-script` | mod scripting runtime | 4 | Runtime choice still open (spike T13-002) |
| `ss2-app` | client and listen-server executable | 3 | Binary |
| `ss2-server` | headless dedicated server executable | 3 | Binary |
| `tools/gro-tool` | list and extract files from a game install | 3 | Tool that reads a player's install; never published |
| `tools/net-tool` | inspect replay files, run the network simulator | 3 | Development tool; reads no game data; not in the release archives |
| `xtask` | repository tooling | 3 | Repo-only tooling |
| `ss2-testkit` | data-gated test helpers | 2 | Reads `SS2_DATA_DIR`; test support only |
| `ss2-bench` | criterion benchmark harness | 2 | Benchmarks only (docs/PERF_BUDGETS.md) |
| `ss2-sim` | fixed-step simulation, entity model | 2 | Sim-safe, game-specific |
| `ss2-net` | transport, sessions, snapshots, prediction | 2 | Game-specific co-op netcode |
| `ss2-physics` | collision, character controller | 2 | Game-specific |
| `ss2-game` | gameplay: player, weapons, enemies | 2 | Game-specific |
| `ss2-world` | level instance, navigation, mechanisms | 2 | Game-specific |
| `ss2-render` | wgpu renderer | 2 | Engine renderer; uses the bundled placeholder font |
| `ss2-audio` | mixer, 3D sound, music | 2 | Engine audio implementation |
| `ss2-video` | Theora cutscene playback | 2 | Thin game-specific wrapper |
| `ss2-ui` | UI framework, HUD, menus, localization | 2 | Game UI (track T10) |
| `ss2-platform` | window, input, lifecycle, paths | 2 | Engine platform implementation |

## 3. Gate before a crate is published

A crate is published only when **all** of these hold, checked in the PR that removes its
`publish = false`:

1. **No game content and no derived data.** `cargo xtask guard` passes. Test fixtures are synthetic
   (T00-010). Any format knowledge comes from a reviewed spec in `docs/re/`, never from decompiled
   output (CLEANROOM.md).
2. **Stable API.** Every public item that is part of the contract has its RFC accepted. Run
   `cargo semver-checks` ([DEV_TOOLS.md](DEV_TOOLS.md)) against the previous published version
   (not needed for the first publish).
3. **Docs.** Every public item is documented, and `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate>
   --no-deps --locked` passes (the same flags as the docs workflow).
4. **Tests.** `cargo test -p <crate> --locked` passes, and data-gated tests skip cleanly without
   `SS2_DATA_DIR`.
5. **Crate README and metadata.** The crate has a `README.md` (none exists today, see §7) and
   `readme`, `keywords` and `categories` in its manifest. `description` and `repository` are already set.
6. **License.** `license = "Apache-2.0"` is inherited. The root `LICENSE` and `NOTICE` must be part
   of the package: `cargo package --list -p <crate>` shows whether they are included, and a copy or
   an `include` entry is added if not.
7. **Changelog.** One line under `[Unreleased]` in [CHANGELOG.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CHANGELOG.md),
   as [RELEASING.md](RELEASING.md) §2 requires.
8. **Dependencies.** Every dependency is available on crates.io. A path dependency on an unpublished
   crate blocks publishing. `cargo deny check` passes.
9. **Dry run.** `cargo publish --dry-run -p <crate>` passes.

## 4. Versioning while the project is 0.x

- The version rules are those of [RELEASING.md](RELEASING.md) §1: `0.y.0` for a breaking change or
  milestone, `0.y.z` for compatible fixes. Under 0.x a change to `y` is breaking for cargo users.
- **Open decision: lockstep or per-crate versions.** `release.toml` sets `shared-version = true`, so
  today every crate moves with the product version. Recommendation: keep lockstep for the first
  publish set, and revisit if a crate changes much less often than the rest.
- A `0.0.x` name placeholder (§6) is exempt from lockstep and is recorded in the changelog.

## 5. Who publishes, and how

- **Maintainers only.** A crate is published by a maintainer with the project's crates.io account.
  The API token is never committed (the secrets scan in CI and `cargo xtask guard` guard the repo).
- Per release, after §3 holds: `cargo publish --dry-run -p <crate>`, then `cargo publish -p <crate>`,
  in dependency order (for example `ss2-core` before `ss2-vfs`).
- [RELEASING.md](RELEASING.md) has no publish step yet: it only cuts the version, changelog and tag.
  The first publish PR adds the publish step there and to `release.yml`.

## 6. Name reservation

- **No name is reserved yet.** The final project name is pending in roadmap task **T00-031**
  ([roadmap T00](roadmap/T00-repo-infra.md): "Final project name decision"). All crate names use
  the `ss2-` prefix, and the names may change with it.
- Do **not** publish placeholder crates, claim names or create crates.io reservations before T00-031
  is decided and recorded in the roadmap.
- **Proposal after T00-031:** publish a `0.0.1` placeholder for each library-candidate name that
  is free and in the final set, as an empty crate with `description`, `license` and `repository`.
  The placeholder PR references T00-031, and the placeholder is not used for real releases.
- Availability is not checked here; it is checked on crates.io when T00-031 is decided.

## 7. Yanking

- A published version cannot be overwritten. The only removal is `cargo yank --version <v> <crate>`.
- Yank a version for a security vulnerability, a licence or content violation (§3.1), or a build that
  does not compile from crates.io. Do not yank for an ordinary bug: release a fix instead.
- A yank gets a changelog entry (under Security or Removed, [RELEASING.md](RELEASING.md) §2) and a
  note in the PR or issue that says why.

## 8. Follow-ups (not done by this task)

- **No crate has a `README.md`.** T00-001 required one per crate; `crates/*/README.md` does not exist.
- **Manifests lack `readme`, `keywords` and `categories`** (needed for crates.io pages).
- **`ss2-meta` is a one-line stub** (`crates/ss2-meta/src/lib.rs`) and cannot be published.
- **`ss2-vfs` has a dev-dependency on `ss2-testkit`** (`crates/ss2-vfs/Cargo.toml`). Confirm with
  `cargo publish --dry-run` that a publish-ready `ss2-vfs` does not need `ss2-testkit` published.
- **`ss2-render-api` depends on `ss2-ui`** (`crates/ss2-render-api/Cargo.toml`). A contract crate
  should not pull in a game UI crate before it is a publish candidate.
- Every manifest already sets `publish = false`, which matches this policy; no manifest change is needed.

## 9. Decisions

| Question | Decision | Status |
|---|---|---|
| Can any crate be published now? | No. All 26 members keep `publish = false` | In force |
| First library candidate | `ss2-vfs` (after its API is final and RFC 0001 is accepted) | Candidate |
| Format readers (`ss2-meta`) | Candidate after T02-006..008 land | Pending |
| Name reservation | None until T00-031 is decided; then `0.0.1` placeholders (§6) | Pending T00-031 |
| Versioning | Lockstep for the first publish set | Open; revisit before first publish |
| Publish step in release tooling | Added with the first publish PR, not before | Pending |
| Yank rules | As in §7 | In force once merged |
