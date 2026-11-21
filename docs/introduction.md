# SS2RS

An open-source, clean-room rewrite in **Rust** of the engine that runs *Serious Sam 2*. It loads
**your own copy** of the game's data and aims to play the game the way the original does, with a
new server-authoritative co-op netcode. PC first (Windows, then Linux and macOS); Android and other
platforms come after the PC release.

> **Status: pre-alpha, nothing playable yet.** The workspace builds, the contracts between the
> major systems are defined and merged, and a placeholder menu runs. Everything else is planned in
> the [roadmap](roadmap/README.md).

This site is built from the `docs/` folder of the repository, so every page here is a normal
markdown file you can also read and edit on GitHub (the "edit" button at the top of each page opens
it).

## Where to start

| If you want to... | Read |
|---|---|
| understand how the pieces fit | [Architecture overview](architecture.md) |
| see the original goals and decisions | [Design](superpowers/specs/2026-11-21-ss2rs-design.md) |
| pick up a task | [Roadmap](roadmap/README.md), then the claim protocol in `AGENTS.md` in the repository root |
| set up your machine | [Developer tools](DEV_TOOLS.md) |
| write code that fits in | [Coding standards](CODING_STANDARDS.md) |
| propose or review an interface | [RFCs](rfcs/0000-template.md) |
| learn what we know about the game's formats | [Reverse-engineering specs](re/README.md) |
| check or set performance goals | [Performance budgets](PERF_BUDGETS.md) |

## API documentation

The generated Rust API docs (`cargo doc`) are published next to this book under `api/` when the
site is deployed; locally run `cargo doc --workspace --no-deps --open`. Start at the `ss2-*-api`
crates: they are the contracts.

## What this project is not

It is **not** a copy of Croteam's code and ships **no game content**. Behavior is re-created from
observation and written specs under a [clean-room process](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md).
It is **not** affiliated with, endorsed by, or sponsored by Croteam or Devolver Digital. "Serious
Sam" and "Serious Engine" are trademarks of their respective owners.

## Editing this site

Add a page by creating the markdown file and listing it in `SUMMARY.md` (next to `book.toml` in `docs/`); a page not
listed there is not part of the site. CI builds the site on every pull request that touches
`docs/`, so a broken `SUMMARY.md` fails the check.
