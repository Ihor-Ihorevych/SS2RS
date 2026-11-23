# Releasing

How versions are numbered, how the changelog is kept, how a release is cut, and what the release
files are called. Status: **policy v0**. Nothing has been released yet; the first release is
`0.1.0` (see §1). Mobile artifacts and code signing are later (§6).

## 1. Version numbers

One **product version** for the whole workspace, written once in `Cargo.toml` under
`[workspace.package] version` (every crate inherits it). It follows [Semantic Versioning
2.0.0](https://semver.org) with the usual `0.y.z` meaning before 1.0:

| Bump | When | Milestone |
|---|---|---|
| `0.y.0` (minor) | a roadmap milestone lands, or anything a user or mod author must adapt to | M1 = `0.1.0`, M2 = `0.2.0`, ... M7 = `0.7.0` |
| `0.y.z` (patch) | fixes and small improvements with no breaking change | any time |
| `1.0.0` | **M8**: PC 1.0 (campaign complete, mods, perf budgets met, packaged releases) | M8 |
| `-alpha.N`, `-beta.N`, `-rc.N` | pre-releases of the above, for playtests (T12-011) | optional |

Milestones are defined in the [roadmap](roadmap/README.md#5-milestones-and-exit-criteria). A release
is cut when the exit criteria of its milestone are demonstrably met, not on a schedule.

### Interface stability inside the workspace

Crates are `publish = false` (publishing policy: T00-025), so "semver" for them means: **who may
break whom**.

- The contract crates (`ss2-vfs`, `ss2-assets`, `ss2-render-api`, `ss2-platform-api`,
  `ss2-audio-api`, and the `ss2-core` items they use) are **frozen on merge**
  ([RFC process](rfcs/0000-template.md)). Compatible additions (a new `#[non_exhaustive]` variant, a
  new default trait method) need the crate owner's approval; anything else needs a new RFC.
- Run `cargo semver-checks` (see [developer tools](DEV_TOOLS.md)) on the contract crates before
  cutting a release and treat a reported break as needing an RFC reference in the changelog.
- Everything else is internal and may change freely inside its crate.

### Things that are **not** the product version

These are versioned separately, because they must stay compatible across product versions and are
decided by their own tracks. Each is bumped only when its format changes, and each change is a
changelog entry:

- **Save-game format version** (T09, T03-014): old saves must load or fail with a clear message.
- **Settings/bindings schema version** (T03-014): migrated, never silently dropped.
- **Network protocol version** (T08): a client and server must refuse each other cleanly when it
  differs, regardless of product version. The rules, the schema hash and the wire-format changelog are in
  `docs/PROTOCOL.md` (`ss2_net::protocol`).
- **Cooked-cache version** (T02-011): a mismatch simply rebuilds the cache.

## 2. Changelog

[`CHANGELOG.md`](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CHANGELOG.md) follows [Keep a
Changelog](https://keepachangelog.com): newest first, an `[Unreleased]` section at the top, and under
each version the groups **Added, Changed, Deprecated, Removed, Fixed, Security**.

- **The PR author adds the entry** under `[Unreleased]` in the same PR, one line, written for a
  *user or mod author*, not for a reviewer. Pure internals (refactors, tests, CI) need no entry.
- Mention the roadmap task ID (`T04-026`) and, for an interface change, the RFC number.
- Behavior that differs from the original game on purpose goes under **Changed** with the reason.
- A save, settings, protocol or cache version bump (above) always gets an entry.

## 3. Cutting a release

Releases are cut from `main` by a maintainer. `cargo-release` (`cargo install --locked
cargo-release`) does the mechanical edits; the commit and tag are made by hand so they carry the
DCO sign-off.

1. **Prepare.** `git fetch origin && git checkout main && git pull --rebase origin main`. CI on
   `main` is green. The milestone's exit criteria are met (roadmap §5; for 1.0, the go/no-go list
   in [PC_1_0_CHECKLIST.md](PC_1_0_CHECKLIST.md)). `cargo semver-checks` on the contract crates is clean or explained.
2. **Look before you leap.** Dry run (the default; nothing is written):
   ```
   cargo release version minor      # or patch / major / 0.2.0-alpha.1
   cargo release replace
   ```
   Check the version it would set and the changelog edit it would make.
3. **Do it.** Same commands with `--execute`, then review `git diff`:
   ```
   cargo release version minor --execute
   cargo release replace --execute
   ```
   `replace` turns `[Unreleased]` in `CHANGELOG.md` into the new version with today's date and adds a
   fresh empty `[Unreleased]`.
4. **Commit and tag by hand** (DCO):
   ```
   git switch -c release-v0.2.0
   git commit -a -s -m "release: v0.2.0"
   ```
   Open a PR titled `release: v0.2.0`, wait for CI, merge it. Then tag the merge commit:
   ```
   git fetch origin && git switch main && git pull --rebase origin main
   git tag -a v0.2.0 -m "SS2RS 0.2.0"
   git push origin v0.2.0
   ```
5. **The tag triggers the release workflow** (`.github/workflows/release.yml`). It checks the tag
   matches the workspace version, builds the binaries for each platform with the `dist` profile,
   packages them, and creates a **draft** GitHub release with the archives, a `SHA256SUMS` file and
   the changelog section as notes.
6. **Verify, then publish.** Download an archive on each platform you can reach, run
   `ss2-app --smoke-frames 5`, confirm it contains **no game content** (§4), then publish the draft.
   A release is only ever published by a human.

To rehearse the packaging without a tag, run the **release** workflow manually
(*Actions > release > Run workflow*); it builds and uploads the archives as workflow artifacts and
creates nothing.

## 4. Release files

The per-platform package formats, versions and distribution plan are in [PACKAGING.md](PACKAGING.md).

Names are `ss2rs-<version>-<target>.<ext>` where `<target>` is the Rust target triple:

| Platform | File |
|---|---|
| Windows (x64) | `ss2rs-0.2.0-x86_64-pc-windows-msvc.zip` |
| Linux (x64) | `ss2rs-0.2.0-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `ss2rs-0.2.0-aarch64-apple-darwin.tar.gz` |

Each archive unpacks into a single folder of the same name containing exactly: `ss2-app` and
`ss2-server` (`.exe` on Windows), `LICENSE`, `NOTICE`, `README.md`. **No game content, ever**: the
packaging step copies a fixed list of files, never a directory, and the proprietary-content guard
(`cargo xtask guard`) already blocks game files from the repository. Players point the app at their
own install (`SS2_DATA_DIR`).

The release also carries `SHA256SUMS` (one line per archive, `sha256sum` format). Source archives
are GitHub's automatic ones for the tag.

Binaries are built with the `dist` Cargo profile (`cargo build --profile dist`): the release
profile plus thin LTO, one codegen unit and stripped symbols. It is a separate profile so everyday
builds stay fast.

## 5. Pre-releases and hotfixes

- A pre-release uses a `-alpha.N` / `-beta.N` / `-rc.N` version and the same steps; mark the GitHub
  release as a pre-release when publishing. Playtest channels and the feedback form: [PLAYTEST.md](PLAYTEST.md).
- A hotfix is a patch release from `main`. While there are no long-lived release branches (true
  until 1.0), a fix goes to `main` first and is released from there.

## 6. Reproducible builds and nightly builds

The same commit gives the same release bytes (roadmap T12-008). Three things make it so:

- **`scripts/dist-flags.sh`** prints the `RUSTFLAGS` of a release build: `--remap-path-prefix` for the checkout (`/src`), the
  cargo home (`/cargo`), the rustup home (`/rustup`) and, when `CARGO_TARGET_DIR` is set, the target folder (`/target`), so
  the folders the build ran in are not in the binary; and, on Windows, the linker flag `/Brepro`, which puts a hash instead of the
  build time in the executable. `release.yml` builds with `RUSTFLAGS="$(scripts/dist-flags.sh)"`.
- **`scripts/make_archive.py`** makes the `.tar.gz` or `.zip`: entries sorted, the time of the **commit** (`SOURCE_DATE_EPOCH`,
  never the clock), no owner, normalised modes, no time or name in the gzip header, a zip that says it was made on Unix. Its tests
  run in CI (`python3 -m unittest discover -s scripts`).
- **`scripts/repro-check.sh [package...]`** builds the packages twice into two empty target folders and compares the SHA-256 of
  the binaries (`ss2-server` by default, about 25 seconds; `ss2-app` about 6 minutes per build). The `reproducible` job in
  `release.yml` runs it for `ss2-server` on Linux and Windows. Exit status 0 identical, 1 different.

**What was checked, and what was not** (on the maintainer's Windows machine, Rust 1.99.1, MSVC):

- Without the flags, two builds of `ss2-server` differ. With them they are identical, also when one is built from a checkout in
  another folder.
- `ss2-app` was **not** reproducible at first: the build script of the `glutin` crate leaves its output folder (inside the target
  folder) in the binary. Remapping the target folder fixed it; with that, `ss2-server` and `ss2-app` are both identical across two
  builds into different target folders (the `ss2-app` check was not repeated from a checkout in another folder).
- **Not checked:** Linux and macOS (the expectation that their linkers leave no time stamp comes from their documentation, not from a
  run), a different Rust toolchain version or a different cargo registry state (the lockfile pins the crates, the toolchain is pinned
  by `rust-toolchain.toml`), and the archives of two different machines: the compressed bytes depend on the zlib that Python links.
  The check compares binaries, not archives.

`release.yml` also runs **nightly** (03:23 UTC, Linux and Windows only, macOS minutes cost ten times as much): the same build and
packaging as a manual run, kept as workflow artifacts for 14 days. Each package step prints the archive's SHA-256 into the run
summary, and the draft-release job verifies `SHA256SUMS` with `sha256sum --check --strict` before attaching it. The changelog
section as release notes was already in place.

To check by hand: `scripts/repro-check.sh ss2-server ss2-app`.

## 7. Not yet covered

- **Code signing and notarization** (Windows SmartScreen, macOS Gatekeeper): until M8 the binaries
  are unsigned and the OS will warn; the release notes say so. Tracked with T03-035 and T03-010.
- **Linux packaging** (AppImage/Flatpak, Steam Deck): T03-011.
- **Android** (signed APK/AAB, store policies): Phase 2, T12-018.
- **Publishing crates to crates.io**: T00-025 decides which crates, if any; see [PUBLISHING.md](PUBLISHING.md).
- **SBOM and license audit per release**: T12-010; `cargo deny check` already gates every PR.
