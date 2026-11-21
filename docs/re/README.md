# Reverse-engineering specs

This folder holds **prose specifications** of the original game's formats and behavior, written by spec authors (roadmap track T01) and used by implementers. See [`CLEANROOM.md`](../../CLEANROOM.md): specs describe *what* the original does; they never contain decompiler output, disassembly, or game files. Reviews fall under `CODEOWNERS`.

One file per topic, named `<topic>.md` (for example `ctsemeta-container.md`, `tex.md`, `player-movement.md`). Use this template:

```markdown
# <Topic>

- **Roadmap task:** T01-nnn
- **Author / reviewer:**
- **Confidence:** low | medium | high   (low needs a second reviewer)

## Summary
What this is and where the game uses it.

## Evidence
How we know: observed files, measurements, documentation. Never decompiler output.

## Layout / rules
Byte or field layout (offsets, sizes, endianness, versions), or the behavior and its numbers
(units, ranges, formulas).

## Invariants
What is always true; what a parser must reject.

## Open questions
What we do not know yet.

## Test vectors
Small hex dumps and the values they must decode to; or input sequences and expected outcomes.
Hand-made or measured only — no excerpts of copyrighted files beyond a few bytes needed to
illustrate a header.
```

Specs are the contract between spec authors and implementers: if an implementer finds one wrong or incomplete, open an issue (label `spec`) instead of reading the original binary.
