# Hot reload: detecting change, and the settings text

Two small, device-free pieces (T04-033; also what the loose-file reload of T02-011b needs): a **file watcher** that decides when a file has really changed, and a **settings text** that applies all-or-nothing. Neither reads the file system or a clock, and neither touches a GPU.

## `ss2_core::hotreload::FileWatcher`

The caller lists the watched files and their `FileStamp` (modification time and size) each poll; `poll` returns the **added, changed and removed** files, once each, sorted by path.

- **Settling.** A change is reported only after the **same stamp has been seen on `settle` further polls** (`FileWatcher::new(settle)`; 0 reports at once, 1 needs two polls in a row). A file still being written (its stamp keeps moving) produces nothing until it stops, and then one event for the final state. That is what keeps a hot reload from reading a half-written shader or settings file.
- **Editors that save by deleting and recreating** (write a temporary file, delete the old one, rename) give **one `Changed`**, not a `Removed` followed by an `Added`: the removal has to settle too, and it does not, because the file is back. A file that vanishes and returns with the **same** stamp within the settle time is no event at all, and one that appears and vanishes before settling leaves no trace (its bookkeeping is dropped).
- **`baseline(current)`** takes the files that exist now as the starting point without reporting them (the first scan at start-up).
- **Order and duplicates:** events are in path order; if a path is listed twice the last stamp wins. `known()` counts the files that have been reported present.

A model-checked test generates 300 random snapshots of five files (with stamps and absences flapping from poll to poll) for each settle from 0 to 3, applies every event the way a caller would (checking each is sensible: no file added twice, none changed before it exists, none removed while present, none "changed" to what it already was) and then holds one final snapshot: the events must add up to **exactly that snapshot**, and an unchanged snapshot reports nothing more.

## `QualitySettings::apply_text`

`ss2_render_api::quality::QualitySettings::apply_text(text)` applies the text of a graphics settings file: one `name = value` (or `name value`) per line, `#` comments, blank lines, CRLF or LF, and the special name `preset` that loads a whole tier at that point so that later lines adjust it. It is **all or nothing**: if any line is wrong, **nothing** is applied, and the `ApplyReport` lists every bad line with its number (1-based) and message, so a half-saved edit never leaves the renderer in a mixed state. When it succeeds, `changed` lists the settings whose value actually changed, in the screen order, so a renderer rebuilds only what it must (a changed `render_scale` or `anti_aliasing` needs new targets; a changed `bloom` does not). Applying the same text twice reports nothing the second time.

A hot reload of the settings file is then: poll the watcher; on a `Changed` for the file, read it, call `apply_text`; on `errors`, show them (the old settings stay) and wait for the next save.

## How it was tested

Eleven tests of the watcher (plus a doctest) and six of the settings text (plus a doctest). **Twenty-two deliberate breakages: twenty failed a test at once.** Two survived: a baseline entry's `stable` count (**equivalent**: it only matters when the stamp differs from the reported one, and a difference resets it, so I simplified it to 0), and `known()` counting the stamp last seen instead of the one last reported (**real**: no test looked at `known()` while a change was unsettled; I added those assertions). One mutation did not apply the first time because rustfmt had reflowed the line; redone, it failed a test.

## Not covered (follow-up T04-033b)

- **No file system**: nothing reads real modification times, lists a directory, or calls `poll` on a timer; the app has no file for these settings yet, and `ss2_core::settings` (the engine's own TOML settings) is a different file with its own overrides.
- **Shader hot reload is not done**: this only says *when* a shader file has changed. Recompiling it, rebuilding the pipelines that use it, and keeping the old pipeline if the new source does not compile need the shader and pipeline code (T05, T04-007), which does not exist yet.
- **Applying a changed setting** (rebuilding render targets for a new scale, changing the MSAA count) is the renderer's; `apply_text` only reports what changed.
- **No change events for directories**, renames as renames (a rename is a removal and an addition), or file contents (a stamp that does not change but a content that does, on a file system with coarse timestamps, is missed: the size helps, a hash would be a follow-up).
