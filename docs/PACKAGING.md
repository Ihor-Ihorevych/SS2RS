# PC packaging and distribution

Status: **plan v0** (roadmap task T12-009, issue #343). Nothing has been released. Each item is marked
**IMPLEMENTED** (the repository has the code or workflow today), **PROPOSED** (a suggestion, not in the
repo, not agreed) or **OPEN** (no decision, or blocked by an open task). Release mechanics are in
[RELEASING.md](RELEASING.md); the 1.0 go/no-go list is in [PC_1_0_CHECKLIST.md](PC_1_0_CHECKLIST.md).

## 1. Scope

- **PC first:** Windows (x64), Linux (x64), macOS (Apple Silicon). M8 ("packaged PC releases") is the
  target ([roadmap section 5](roadmap/README.md#5-milestones-and-exit-criteria)).
- **Not in this plan:** Android (T12-018, T03-024) and iOS (T03-027). They are `PHASE 2`, after M8.
- **What a package contains:** the binaries that `release.yml` builds and the fixed file list in
  [RELEASING.md](RELEASING.md) section 4.

  | Item | Source | Status |
  |---|---|---|
  | `ss2-app` (window; `--smoke-frames`, `--screenshot`, `--size`) | `crates/ss2-app` | IMPLEMENTED (workflow) |
  | `ss2-server` | `crates/ss2-server` | IMPLEMENTED (workflow); the binary only prints `not implemented yet` |
  | `LICENSE`, `NOTICE`, `README.md` | repository root | IMPLEMENTED (workflow) |
  | `gro-tool` (lists and extracts files of a player's install) | `tools/gro-tool` | OPEN: built, not in the list. Shipping it changes `release.yml` and RELEASING section 4 |

  `xtask` and the test and benchmark crates are never packaged.
- **The user guide is not in the archive.** [install](user/install.md) is only on the docs site. The
  packaged `README.md` must tell players to point the engine at their data (PC_1_0_CHECKLIST section 6): OPEN.

### 1.1 No game content in any package (absolute rule)

No package may contain game data, game executables or derived game data. The rule comes from
[CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md) (forbidden files and
redistribution), [COMPLIANCE.md](COMPLIANCE.md) section 2.4 (archives hold `LICENSE`, `NOTICE`,
`README.md` and the binaries) and [PUBLISHING.md](PUBLISHING.md) section 3, gate 1.

**What enforces it today:**

- `cargo xtask guard` (`xtask/src/guard.rs`) checks **git-tracked files** (`guard`) or **staged files**
  (`guard --staged`, pre-commit). It rejects a `Serious Sam 2` folder component, the extensions `.gro`,
  `.dll`, `.exe`, `.chm` anywhere, `.wld`, `.tex`, `.mdl`, `.bmf`, `.ogv`, `.xmv` outside `testdata/`,
  files over 5 MiB and `testdata/` files over 256 KiB. It does not read contents or hashes, and it
  **does not check archives or folders outside git**.
- `release.yml` copies a fixed list of files, never a directory.
- COMPLIANCE section 6.2 asks for a hand check of each archive's listing.

**PROPOSED release step:** run the guard over the exact package contents before publishing. The guard
only sees git paths, so this needs either a temporary git index holding the package folder and
`cargo xtask guard --staged`, or a small `xtask` change that takes a directory. Either is its own PR.

### 1.2 How players supply their data

- Today the only way is the environment variable **`SS2_DATA_DIR`**, pointing at the folder with the
  `.gro` archives ([install guide](user/install.md)).
- The first-run wizard (T10-019, `P0`) is **planned**; it needs `ss2-doctor` (T11-013), also not built.
- No package step downloads, copies or bundles game data.

## 2. Package formats per platform

| Platform | Format | Status |
|---|---|---|
| All | GitHub Release assets: archives and `SHA256SUMS`, as a draft that a human publishes | IMPLEMENTED (`release.yml` draft-release job); not yet run for a tag |
| Windows x64 | portable zip (`ss2rs-<ver>-x86_64-pc-windows-msvc.zip`) | IMPLEMENTED (`Compress-Archive`) |
| Windows x64 | installer (WiX/MSI or Inno Setup) | PROPOSED, after the zip; depends on T03-026 (open) |
| Windows x64 | code signing | OPEN: T03-026; no certificate in the repo. Release notes say the binaries are unsigned |
| Linux x64 | tar.gz (`ss2rs-<ver>-x86_64-unknown-linux-gnu.tar.gz`) | IMPLEMENTED (workflow) |
| Linux x64 | AppImage | PROPOSED, T03-011 (`P2`, open). See 2.1 |
| Linux x64 | Flatpak | PROPOSED, T03-011 (open). See 2.1 |
| macOS arm64 | tar.gz with the two binaries in a folder (`ss2rs-<ver>-aarch64-apple-darwin.tar.gz`) | IMPLEMENTED (workflow, tags and manual runs only). Not a `.app` |
| macOS | `.app` bundle in a zip or dmg (dmg is PROPOSED) | OPEN: T03-010 (`P2`) |
| macOS | Intel (x86_64) archive | OPEN: `release.yml` builds only the runner's host triple; T03-010 asks for both |
| macOS | codesign and notarisation | OPEN: needs an Apple Developer account, which the repository **does not have** |

### 2.1 Linux containers: design points (PROPOSED, nothing built)

- **AppImage:** the Linux build links `gilrs` (gamepad), and `release.yml` installs `libudev-dev` to
  build. Whether the binary needs `libudev` at run time, and whether to bundle it, is not checked.
  Test on a clean system.
- **Flatpak:** the sandbox denies arbitrary folder access. The engine must read a folder the player
  picks. Use the file-chooser portal (access only to the picked folder, fits T10-019) or an explicit
  `--filesystem=` permission in the manifest. Gamepad (`/dev/input`) and display sockets are also needed.
  Flathub would need the no-data rule of section 6 accepted.

## 3. Naming, versions, checksums and metadata

- **Version:** one product version in `[workspace.package] version` (root `Cargo.toml`), semver with
  `0.y.z` before 1.0 and `1.0.0` for M8 ([RELEASING section 1](RELEASING.md#1-version-numbers)).
  IMPLEMENTED as policy. The workspace is at `0.0.0`; the first release is planned as `0.1.0` (M1).
- **Tag:** `vX.Y.Z`. `release.yml` fails if it differs from the workspace version (IMPLEMENTED).
- **Dev builds:** non-tag runs name archives `ss2rs-<ver>-dev.<sha7>-<target>` and keep them as workflow artifacts for 14 days (IMPLEMENTED).
- **Archive names:** `ss2rs-<version>-<target triple>.<ext>`, unpacked into a folder of the same name.
- **Checksums:** `SHA256SUMS` (`sha256sum` format, one line per archive), attached to the draft (IMPLEMENTED). Verifying it is a release step.
- **SBOM:** PROPOSED: CycloneDX JSON from `cargo-cyclonedx`, named `ss2rs-<version>-sbom.cdx.json`, attached to the draft and listed in `SHA256SUMS` (COMPLIANCE section 3). No generator is installed.
- **Licences:** `cargo deny list` output is PROPOSED to attach. Bundled assets (Inter, OFL-1.1) are named in `NOTICE`. There is **no separate third-party notices file** (OPEN, COMPLIANCE section 8).
- **Provenance and signing:** OPEN. Options: GitHub artifact attestations, Sigstore signing of `SHA256SUMS`. No task, no decision.
- **Reproducible builds:** an OPEN goal, owned by T12-008 (not duplicated here). Present: `Cargo.lock` with `--locked`, the pinned toolchain (`rust-toolchain.toml`, 1.99.1) and the `dist` profile (`Cargo.toml`). Missing: a rebuild-and-compare job and normalised archive timestamps.
- **Release notes:** the `CHANGELOG.md` section for the version, extracted by `release.yml`, with a fixed footer on unsigned binaries, no game data and `SS2_DATA_DIR` (IMPLEMENTED). The 1.0 checklist also needs the Windows baseline, unpackaged platforms and the "you need your own copy of Serious Sam 2" statement (PC_1_0_CHECKLIST sections 6 and 7).

## 4. Build and release process

- **Who:** a maintainer cuts the release from `main`; a human publishes it (RELEASING section 3). The go/no-go decision is the maintainers' (PC_1_0_CHECKLIST section 9).
- **Actions does not run.** Since 2026-11-22 every job fails at start because of an account billing block
  (AGENTS.md section 1c), so the tag-triggered `release.yml` cannot run. Until it works, build and check on a maintainer machine and record each command in the release PR. Step 4 below is PROPOSED.

**Release checklist (copy into the release PR):**

1. [ ] `git fetch origin && git switch main && git pull --ff-only origin main`; `grep -rn INPRG docs/roadmap/` shows no claim that blocks the release.
2. [ ] Milestone exit criteria met (roadmap section 5); for 1.0, the go/no-go table (PC_1_0_CHECKLIST section 9).
3. [ ] `cargo release version <bump>` and `cargo release replace` (dry run, then `--execute`) (RELEASING section 3). Commit `release: vX.Y.Z` with `git commit -s`; PR; merge.
4. [ ] PROPOSED local build, on each platform you can reach: `cargo build --profile dist --locked -p ss2-app -p ss2-server`. Copy exactly the binaries, `LICENSE`, `NOTICE` and `README.md` into `ss2rs-<ver>-<target>/`; archive it (zip on Windows, tar.gz elsewhere); run section 1.1 on the folder; run `ss2-app --smoke-frames 5` on a clean machine.
5. [ ] `sha256sum ss2rs-* > SHA256SUMS`, then `sha256sum -c SHA256SUMS`.
6. [ ] PROPOSED: `gh release create vX.Y.Z <archives> SHA256SUMS --draft --notes-file <notes>` (`--prerelease` for `-alpha`, `-beta`, `-rc` tags). Notes are the changelog section.
7. [ ] Human review of the draft; archive listings checked by hand; publish.
8. [ ] Local checks, run after merging `origin/main` (AGENTS.md section 1c): `cargo fmt --all -- --check`, clippy with `-D warnings`, `cargo test --workspace --locked`, `cargo deny check`, `cargo xtask guard`, `cargo xtask check-claims`, the book build and link check, the rustdoc build, and the DCO loop. Results go in the PR; "not run" is stated as such.

**Cross-compilation:** none is set up. Each archive is built on a runner of its own platform (`release.yml` uses each runner's host triple). A macOS archive therefore needs macOS. Whether a cross-build is ever acceptable is OPEN.

**Rehearsal:** the `release` workflow runs on pull requests that touch it and on manual dispatch. It creates no release. It is blocked with the rest of Actions.

## 5. Install, configuration, saves, updates and uninstall

- **Install:** extract the archive anywhere and run the binary. No installer, shortcut, file association or PATH entry (section 2).
- **Settings (IMPLEMENTED):** `Settings::default_path` (`crates/ss2-core/src/settings.rs`) puts `settings.toml` in the OS config directory under `SS2RS/`, found with the `directories` crate (for example `%APPDATA%\SS2RS\settings.toml` on Windows). Saved through a temporary file; a missing or broken file gives defaults (T03-030, done). Schema and migration: T03-014 (`P0`, open).
- **Key bindings:** `bindings_file` in `crates/ss2-platform-api/src/paths.rs` is planned; the feature is T03-005 (open).
- **Saves (OPEN):** `PlatformPaths` has a `save_dir` and, since T03-012, resolves the per-OS directories (`PlatformPaths::resolve` with `UserDirs::detect()`: `%APPDATA%\SS2RS`, `~/.config/SS2RS` and so on) and a **portable mode** (a file `ss2-portable` next to the executable, or an explicit flag: everything below `ss2-data/` next to the executable); the app does not call it yet and no save is written yet. Save format versioning follows RELEASING section 1.
- **Data location (OPEN):** `SS2_DATA_DIR` today; the remembered choice is T10-019; a `--data` option is T03-020. None exists.
- **Logs and crash reports (OPEN):** T03-019 and T03-016. The log location is not decided.
- **Updates (OPEN):** new versions are announced on GitHub Releases only. No roadmap task adds an updater or update check, so the engine does not check for updates. An automatic update would need a new task.
- **Uninstall:** nothing is registered with the system. For a portable archive, delete the folder. The settings file in the config directory stays until removed by hand. No saves or logs exist yet. The uninstall text belongs in the user guide (T12-014) once the installer question is settled.

## 6. Distribution channels

- **Policy (IMPLEMENTED):** GitHub Releases on `Ihor-Ihorevych/SS2RS` is the only channel at first. Releases stay drafts until a human publishes them.
- **Constraint for every other channel (PROPOSED):** a channel may carry only an engine package with no game content, which asks the player for their own data. [PUBLISHING.md](PUBLISHING.md) covers crates.io only and says nothing about package managers; this constraint follows from CLEANROOM.md and COMPLIANCE section 2.4. The store policy review for user-supplied data is T12-018 (`PHASE 2`).
- **Other channels (OPEN, none started):** winget, Homebrew, Flathub (section 2.1), AUR. Each needs a maintainer decision and its own policy check. No roadmap task covers them.
- **Stores (Steam, GOG and similar): OPEN, out of scope** unless a maintainer decides otherwise. PUBLISHING.md does not mention them; T12-014 mentions Steam and GOG only as editions of the game the player owns.
- **crates.io:** not a release channel; every crate keeps `publish = false` (PUBLISHING.md section 1).

## 7. Gaps and open questions

1. The guard cannot check archives or folders outside git; the archive check stays manual until section 1.1's PROPOSED step is adopted.
2. `ss2-server` is a stub; decide whether its archive ships in 0.1.0 (RELEASING section 4).
3. Whether `gro-tool` ships (section 1).
4. The packaged `README.md` does not yet explain `SS2_DATA_DIR` to players.
5. macOS: no `.app`, no Intel archive, no codesign or notarisation, no Apple Developer account.
6. Windows: no installer, no code signing; the 10+ baseline and DPI manifest are T03-026 (open).
7. Linux: no AppImage or Flatpak (T03-011). The Flatpak folder-access design and whether `libudev` is needed at run time are unchecked.
8. No clean-machine test exists for any archive (VC runtime, system libraries). The smoke run is the only check.
9. Provenance, attestations and signing of `SHA256SUMS` are undecided.
10. Reproducible builds (T12-008) and nightly artifacts are not built; `release.yml` has no schedule.
11. No SBOM generator and no third-party notices file (COMPLIANCE sections 3 and 8).
12. Saves, logs and `--data` are open (T03-019, T03-020) and the app does not yet use the per-OS directories of T03-012; uninstall text depends on them.
13. No update check exists and none is in the roadmap.
14. Actions is blocked, so `release.yml` has never run on a tag; the first archives are untested.
15. Package channels and store questions (section 6) have no owner.
