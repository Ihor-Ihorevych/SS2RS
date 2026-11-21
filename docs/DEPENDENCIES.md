# Dependencies and vulnerability alerts

How third-party code gets into SS2RS, how it is kept current, and what we do when an advisory is
published. Policy lives in two files: [`deny.toml`](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/deny.toml)
(licenses, banned sources, advisories) and [`.github/dependabot.yml`](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/.github/dependabot.yml)
(update PRs).

## Adding a dependency

- Prefer the standard library and a dependency we already have. Say why a new crate is needed in the
  PR.
- It must pass `cargo deny check`: permissive licenses only (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0,
  Unicode-3.0, CC0-1.0), **never GPL/AGPL** (SS1 is GPL and must not enter the project, see
  `CLEANROOM.md`), registry sources only.
- Declare it once in `[workspace.dependencies]` in the root `Cargo.toml` and use
  `name.workspace = true` in crates. Turn off default features you do not need (see how `criterion`
  is declared).
- Dependencies of the **sim-safe** crates (`ss2-sim`, `ss2-physics`, `ss2-net`) must not bring
  non-determinism: no hash-order iteration or wall-clock in the code path they use.
- Build tools and CI actions count too: pin to a version, never `@main`.

## How updates arrive

**Dependabot** opens update pull requests weekly (Mondays, 06:00 UTC), at most 3 for Cargo and 2 for
GitHub Actions at a time:

| Kind | Handling |
|---|---|
| Cargo **patch** updates | grouped into one PR |
| Cargo **minor / major** updates | one PR each. For `0.x` crates (wgpu, winit, glam, ...) a *minor* bump is a breaking change, which is why they are not grouped |
| GitHub Actions | minor and patch grouped; majors separate |

PR titles start with `T00-024: deps` to satisfy the title convention. The DCO check skips PRs opened
by `dependabot[bot]` (a bot cannot sign off); by merging, the maintainer takes responsibility for the
change. **Review a bump like any change**: read the upstream changelog for anything that touches our
contracts, run the tests, and for graphics/windowing crates (`wgpu`, `winit`, `glyphon`) also run the
app (`cargo run -p ss2-app`) and the golden-image tests.

## Advisories

Three layers, all using the RustSec database:

1. **Every pull request**: the `deny` job in `ci.yml` runs `cargo deny check` (advisories, licenses,
   bans, sources).
2. **Every Monday**: the `audit` workflow runs `cargo deny check advisories`, so a new advisory for a
   dependency we already use is noticed even if nobody opened a PR. A failing run notifies the
   repository owner.
3. **GitHub Dependabot alerts and security updates** (owner setting, see below): alerts on the
   repository and automatic PRs for vulnerable versions.

Locally, `cargo deny check` is the gate; `cargo audit` (see [developer tools](DEV_TOOLS.md)) prints
the same database with more detail.

### When an advisory is reported

1. **Vulnerability with a fixed version:** update the crate (Dependabot will usually have opened the
   PR), merge as soon as CI is green.
2. **No fix yet, or we are not exposed:** do **not** silence it quietly. Add an entry to `ignore` in
   `deny.toml` with `id` and a `reason`, and a *revisit condition* in the comment above it, and add a
   row to the table below. An ignore without a reason is rejected in review.
3. **Unmaintained crate:** treat as a warning; plan a replacement when a maintained alternative
   exists, and list it below.
4. If we ship a build affected by a real vulnerability, say so in the changelog's **Security**
   section (see [releasing](RELEASING.md)).

### Known advisories (reviewed 2026-11-21)

| Advisory | Crate | Kind | Pulled in by | Why accepted / revisit when |
|---|---|---|---|---|
| RUSTSEC-2026-0192 | `ttf-parser 0.25.1` | unmaintained | `winit` Wayland client-side decorations | in `deny.toml` `ignore`. Only parses a bundled UI font. Revisit when `winit` leaves `sctk-adwaita` (winit 0.31+) |
| RUSTSEC-2026-0253 | `lru 0.16.4` | unsound (possible use-after-free in `LruCache::pop()`, fixed in `lru >= 0.18.2`) | `glyphon 0.12.0` (text atlas cache) | not in `ignore`, because `cargo deny` reports it as a warning and does not fail. We never call `lru` ourselves and `glyphon 0.12.0` is the latest release, so there is **no upgrade path today**. Revisit when `glyphon` releases a version using `lru >= 0.18.2`; Dependabot will open that PR |

## What the repository owner has to switch on

These are repository settings; they need admin rights and cannot be set from a pull request:

- **Settings > Code security > Dependabot alerts**: on (at the time of writing the API reports them
  as disabled).
- **Dependabot security updates**: on, so vulnerable versions get a PR automatically.
- Optionally **Dependency graph** and **Secret scanning** (CI already runs `gitleaks`).

The weekly update PRs (`dependabot.yml`) work without these settings.
