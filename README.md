# SS2RS

An open-source, clean-room rewrite in **Rust** of the engine that runs *Serious Sam 2*. It loads **your own copy** of the game's data and aims to play the game the way the original does — with a new, server-authoritative **co-op** netcode.

> **Status: pre-alpha — nothing playable yet.** Today the repository contains the workspace, a placeholder-skin menu (no game data used), and a 380-task [roadmap](docs/roadmap/README.md).

## What it is — and is not

- A new engine written in Rust (`wgpu` renderer, own simulation and netcode), **PC first** (Windows, then Linux/macOS). Android and other platforms come after the PC release.
- **Not** a copy of Croteam's code. Behavior is re-created from observation and written specs under a [clean-room process](CLEANROOM.md).
- **Not** shipping any game content. You need your own legally obtained Serious Sam 2 install; the engine will read it from a folder you point it at (`SS2_DATA_DIR`).
- **Not** affiliated with, endorsed by, or sponsored by Croteam or Devolver Digital. "Serious Sam" and "Serious Engine" are trademarks of their respective owners.

## Try the placeholder menu

```
cargo run -p ss2-app                                       # interactive window
cargo run -p ss2-app -- --screenshot options out.png       # render a screen to a PNG
```

## Where to look

| | |
|---|---|
| Design | [`docs/superpowers/specs/2026-11-21-ss2rs-design.md`](docs/superpowers/specs/2026-11-21-ss2rs-design.md) |
| Roadmap & milestones | [`docs/roadmap/README.md`](docs/roadmap/README.md) |
| Contributing | [`CONTRIBUTING.md`](CONTRIBUTING.md) · tools: [`docs/DEV_TOOLS.md`](docs/DEV_TOOLS.md) |
| Clean-room rules | [`CLEANROOM.md`](CLEANROOM.md) |
| Instructions for AI agents | [`AGENTS.md`](AGENTS.md) |

## License

[Apache-2.0](LICENSE). See [`NOTICE`](NOTICE) for attributions (including the bundled OFL-licensed placeholder font).
