# Clean-room audit log

One row per **spec and implementation pair**: which spec in `docs/re/` an implementation PR relied on,
who wrote and reviewed the spec, from which kinds of source, and who implemented it and reviewed the PR.
The process is in [COMPLIANCE.md](../COMPLIANCE.md) section 4. The rules are in
[CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md).

Never record here a file name from the game install, a quotation, a byte dump, an address, or a decompiler
or Ghidra identifier. Describe the source data by file type and tool only.

## Rules for adding a row

1. The implementer adds the row in the implementation PR, on the same branch.
2. Name only specs that exist in `docs/re/`, and the roadmap task that uses them.
3. A spec author may not add or review a row for a topic they specified.
4. Rows are never deleted. A correction is a new commit that says what changed.
5. If a source is unclear, stop and open an issue before adding the row.

## Columns

| Column | Content |
|---|---|
| Spec | `docs/re/<file>.md` and its roadmap task |
| Spec author | handle (`@handle` or `@handle/agent`) |
| Spec reviewer | handle and date, or `none yet` |
| Source data | file types and tool, for example "binary, Ghidra; counts from `tools/re/<script>`" |
| Implementation task | roadmap ID, for example `T02-006` |
| Implementer | handle |
| PR | number |
| Implementation reviewer | handle and date |
| Provenance box | `yes` when ticked in the PR |

## Example row (template, not data)

```markdown
| Spec | Spec author | Spec reviewer | Source data | Implementation task | Implementer | PR | Implementation reviewer | Provenance box |
|---|---|---|---|---|---|---|---|---|
| `docs/re/<topic>.md` (T01-nnn) | @author | @reviewer (YYYY-MM-DD) | <file types>; <tool>; counts from `tools/re/<script>.py` | T02-nnn | @implementer | #nnn | @reviewer (YYYY-MM-DD) | yes |
```

## Log

No rows yet. The maintainers fill the first rows from pull request history. A row is not created
from memory or guesswork.

| Spec | Spec author | Spec reviewer | Source data | Implementation task | Implementer | PR | Implementation reviewer | Provenance box |
|---|---|---|---|---|---|---|---|---|
