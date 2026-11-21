# Clean-room policy

SS2RS must be an **independent** implementation. We replicate *behavior*, never *code*. This protects the project legally and keeps the codebase ours.

## Roles

| Role | Does | Must not |
|---|---|---|
| **Spec author** (roadmap track T01) | Analyzes the original game's binaries and data (Ghidra, hex editors, instrumentation) and writes **prose specs** in `docs/re/` — formats, rules, measured numbers, test vectors. | Commit decompiler output, disassembly, or Ghidra projects. Write product code for the thing they specified. |
| **Implementer** (every other track) | Writes Rust **from the specs**, public documentation and our own measurements. | Read decompiler output of the original binaries, or copy any original/SS1 code. |

One person may be a spec author for one topic and an implementer for another, but never both for the same topic.

## Allowed sources

- Specs in `docs/re/` (reviewed under `CODEOWNERS`).
- Public documentation and file-format knowledge; our own black-box measurements.
- The Serious Sam 1 (Serious Engine 1, **GPL**) source for **concepts and architecture only** — read it, then close it; never copy or translate its code. Anything derived from GPL code would force the GPL onto this project.
- Standard algorithms and well-known libraries with compatible licenses (`cargo deny` enforces the allow-list).

## Forbidden

- Copying or mechanically translating SS1 source or decompiler output.
- Committing game files: `Serious Sam 2/`, `*.gro`, `*.dll`, `*.exe`, original textures/models/worlds, or Ghidra projects. `cargo xtask guard` (pre-commit hook and CI) enforces this.
- Redistributing the game's data, binaries, or Croteam's help files verbatim.

## In every pull request

The PR template asks you to state that you wrote the change from specs/public docs, not from decompiled output or SS1 code, and to sign off the commits (DCO, `git commit -s`).

## When unsure

Stop, and open an issue describing what you were about to use and where it came from. A delay is cheap; a tainted commit is not.
