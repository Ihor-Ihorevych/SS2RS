# Changelog

All notable changes to SS2RS are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html); how both work in this project is
described in [`docs/RELEASING.md`](docs/RELEASING.md).

Add your entry under **Unreleased** in the same pull request, one line, written for a player or
mod author.

## [Unreleased]

### Added

- Pre-alpha foundations: Cargo workspace, CI (build, test, lint, license and dependency policy,
  proprietary-content guard, DCO), clean-room process and contribution workflow (T00-001..013).
- Contracts between the major systems, each with a mock so work can proceed in parallel: virtual
  file system and asset loading (RFC 0001), renderer front-end (RFC 0002), platform events, input
  actions and lifecycle (RFC 0003), audio (RFC 0004).
- Benchmark harness and proposed performance budgets (T00-019), documentation site and
  architecture overview (T00-020), release engineering policy and tooling (T00-021).
- Placeholder main menu (no game data used): `cargo run -p ss2-app`.
- Main menu: **Choose Game Folder...** picks another Serious Sam 2 folder without restarting, and Ctrl+V pastes a path into the first-run wizard (Windows) (T10-035).
- Effect file loaders (`ss2-format-fx`, T02-024): `.pfx` particle effects, `.lfx` lens flares, `.tfx` text effects and `.vfx` vibration effects as typed data.
- Animation sets (`.ans`) load, with pose sampling (interpolation, looping), animation events and root-motion deltas (T02-016).
