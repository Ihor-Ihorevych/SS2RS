# Roadmap progress report

`cargo xtask progress` prints how far the roadmap is: for each track (T00 to T13) the tasks **done**, **claimed** and **open** among the work scheduled now, the same for `PHASE 2` (Android and the rest, not before milestone M8), and the **follow-up tasks** still open. `cargo xtask progress --json` prints the same numbers as one JSON object for other tools (the net-tool of T11-010 plans to feed it, see T11-010b).

It reads the task lines of `docs/roadmap/T*.md` with the same parser as `cargo xtask roadmap-sync`, in the working tree, so it needs no network and no clock, and the same files always give the same report. Nothing is written to the repository: the numbers go stale the moment a PR merges, so the page is generated when you ask, not committed.

## What the numbers mean

- A task ticked `[x]` is **done**, `[~]` is **claimed**, `[ ]` is **open**. Tasks marked `PHASE 2` are counted on their own.
- **Follow-ups are the part of this report to read first.** A task is often ticked with a narrower scope than its title (the device-free logic is done, the GPU or the wiring is left), and the left-over part becomes a follow-up task: an ID with a letter after the number, such as `T04-016b`. Those count as open. At the time of writing about one open task in six scheduled now is a follow-up, which is what "45 % done" does not show.
- It counts **roadmap lines, not work**: an `S` task weighs the same as an `XL` one, and a task ticked with a narrow scope counts as done.
- It says **nothing yet about the game**: not how many file types load, not how many levels load, not which weapons or enemies exist. See below.

A snapshot, from the day this was written: 205 of 452 tasks scheduled now were done (45 %), 46 open ones were follow-ups, and none of the 29 `PHASE 2` tasks were done. Run the command for today's numbers.

## How it was tested

Eleven unit tests: counting per track and per phase on a sample roadmap, follow-ups counted only while open and scheduled now (and across tracks: the total is the sum, not the largest), totals and the rounded-down percentage (no division by zero), the ID helpers, an empty roadmap, the Markdown (summary line and one row per track compared as exact text), the 20-cell bar's rounding at its edges, the JSON compared as exact text with a balance check, and one test over the **real roadmap** (every task is in exactly one bucket, no task ID appears twice, all tracks are there). **Fourteen deliberate breakages** (ticks inverted, the percentage denominator and rounding, the follow-up rules, the sums, the bar, the empty guard): thirteen failed a test, one of them only after I added a test (the sum of follow-ups across tracks needed a sample with follow-ups in two tracks); one is equivalent (treating any letter as a follow-up marker where only lowercase ones occur) and was left. A few mutation scripts first did not apply because rustfmt had reflowed the lines and were redone.

## Not covered (follow-up T11-012b)

The title of T11-012 promises more than this: **file types loadable, the share of levels that load, weapons and enemies implemented, fed by the data-gated runner.** None of that exists. It needs the "load everything" runner (T02-031) and the loaders it runs, which are mostly not written, and a data-gated CI job, which cannot run while the account's Actions are blocked. What is here is the part that needs only the roadmap. T11-012 is therefore **ticked with this narrower scope and its GitHub issue is left open on purpose**; T11-012b lists the rest.

Also not done: a weighted measure by task size, a history of the numbers over time, a page on the documentation site, and a CI step that posts the report on each PR.
