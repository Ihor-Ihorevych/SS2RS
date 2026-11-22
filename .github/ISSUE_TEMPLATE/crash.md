---
name: Crash or hang
about: The engine exits, panics, freezes or stops responding
labels: ["bug", "crash"]
---

<!-- Do not attach game files (*.gro, *.wld, *.tex, *.mdl, *.exe, *.dll) or copied game text. See docs/TRIAGE.md section 8. -->

**What happened** (exit, panic message, freeze, hang at a load screen):

**Steps to reproduce** (from a fresh start if you can; the level or menu where it happens):

**Frequency:** always / sometimes (about how often) / once

**Expected:**

**Actual:**

**Environment**
- Version or commit (`git rev-parse --short HEAD`, or the release tag):
- OS and version:
- GPU and driver version:
- `SS2_DATA_DIR` points to an unmodified retail install? (yes / no / not sure):
- Game store and version (Steam / GOG / retail disc) and patch level (the names of the `Patch_*` files in the install folder are enough):
- Did it work in an earlier version? (which one):

**Logs:** run with `SS2_LOG=debug` and attach the log file. Also paste the panic message and backtrace (`RUST_BACKTRACE=1`). Remove user names and personal paths.
