## Task

Task ID(s): <!-- e.g. T02-006 — the PR title must start with it -->

## Summary

<!-- What changed and why, in a few lines. -->

## Checklist

- [ ] The claim commit (`ID: [INPRG] claim`) is on `main`; this PR ticks the box `[x]` and removes the `[INPRG …]` tag
- [ ] Tests added/updated; `cargo test --workspace` passes (data-gated tests skip cleanly without `SS2_DATA_DIR`)
- [ ] `cargo fmt`, `cargo clippy -D warnings` and `cargo deny check` pass
- [ ] Public items are documented; the spec/RFC is updated if behavior or a contract changed
- [ ] **No proprietary files** (no game data, binaries, Ghidra projects) — `cargo xtask guard` passes
- [ ] **Provenance:** I wrote this from specs (`docs/re/`), public docs or my own measurements — **not** from decompiler output or from Serious Engine 1 (GPL) code
- [ ] Every commit is signed off (`git commit -s`, DCO)
