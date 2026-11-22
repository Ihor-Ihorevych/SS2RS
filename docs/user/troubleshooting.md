# Troubleshooting

This page lists problems that can happen with what exists today. Items marked **planned** are not
built yet, so the fix is to wait for them. For the install steps, see
[Install and point SS2RS at your game data](install.md).

## Symptom, cause, fix

| Symptom | Cause | Fix |
|---|---|---|
| `no game folder: pass --data ... or set SS2_DATA_DIR` | `gro-tool` was started without a data folder. | Set `SS2_DATA_DIR` to the folder with the `.gro` files, or pass `--data <folder>`. |
| `no .gro archives in <folder>: point at the folder that contains All_PC_01.gro` | The folder has no `.gro` file directly inside it. This is usually the parent `Serious Sam 2` folder. | Point at the folder that directly contains `All_PC_01.gro`. |
| Tests print `SKIPPED (no game data)` and count as passed | `SS2_DATA_DIR` is not set, or it points at a folder without `.gro` files. | This is expected without game data. To run the data-gated tests, set `SS2_DATA_DIR` as in [install](install.md#pointing-ss2rs-at-the-data-today). See [testing](../TESTING.md). |
| The engine reads without error, but some files are "not found" | A base archive (`All_PC_01` to `03`, `Movies_PC_01` or `02`) is missing. Only the archives present are mounted, and nothing checks for completeness. | Put the missing archive back from your own copy. `ss2-doctor <folder>` (T11-013) names missing and damaged archives. |
| Behaviour differs from the original, with no error | A `Patch_02_*.gro` file is missing, renamed or from another copy. Patches replace base files, so the older copy is used. `ss2-doctor <folder>` (T11-013) reports the patch level it finds and warns when there are no patches or a file name is not understood; it cannot tell a renamed copy from the original. | Restore the original patch file names from your own copy (the list is in the [inventory](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/inventory.md)). |
| An archive fails to open, and the error names it | The archive is damaged or incomplete, for example a copy that stopped early or a file that was edited. One bad archive stops the whole mount. | Copy the archive again from your own install. Do not edit it. |
| A patch seems to have no effect | Patches are loaded in numeric order, and the newest copy of a file wins. The file was renamed, so the order changed. | Restore the original file name. |
| A `.gro` file you copied into the folder changes the game | Every `.gro` file directly inside the folder is loaded. | Remove files that are not part of the install from that folder. |
| Mod files in `Content` have no effect | The engine does not read sub-folders or mod overlays yet. | Nothing to fix today. Mod loading is planned (T13-001). |
| Error about a path that is too long on Windows (general advice, not checked against this project) | The install is in a deep folder, and Windows limits path length. | Move the install to a shorter path, for example directly under a drive folder. |
| `no suitable GPU adapter` at start-up, a black window, or a crash when the window opens | The graphics driver or GPU is not usable by the renderer. The project does not yet publish a list of supported GPUs. | Update the graphics driver from the GPU vendor and try again. If it still fails, file a crash report with the GPU and driver version (see below). |
| Linker or compiler error when building from source on Windows | The C/C++ build tools are missing. | Install the Visual Studio Build Tools with "Desktop development with C++", as listed in [DEV_TOOLS.md](../DEV_TOOLS.md). |
| The window opens, but the game does not start | There is no playable game yet. The placeholder menu is the only screen. | Expected at this stage. See [FAQ](faq.md#what-works-today). |

## Where to see the log

The program writes a log file `ss2-app.log` and prints the same text in the terminal where you start it. On Windows the
file is `%LOCALAPPDATA%\SS2RS\logs\ss2-app.log`; on Linux `~/.local/state/SS2RS/logs/ss2-app.log`; in portable mode (`--portable`,
or an empty file `ss2-portable` next to the program) it is in `ss2-data/logs` next to the program. Set the `SS2_LOG` variable to
change the detail, for example `SS2_LOG=debug`. If the program panics it also writes a crash report, `crash-<number>.txt`, to
the `crashes` folder inside the log folder: attach it to your report (see the next section).

## What to include in a report

Use the matching template in the
[issue templates folder](https://github.com/Ihor-Ihorevych/SS2RS/tree/main/.github/ISSUE_TEMPLATE):
crash, desync, visual parity, parity or bug. Each asks for:

1. the SS2RS version or commit, or the release tag;
2. your operating system and version;
3. your GPU and driver version;
4. whether `SS2_DATA_DIR` points to an unmodified retail install (yes, no or not sure);
5. the store, version and patch level of the game. The names of the `Patch_*` files in the folder are
   enough;
6. the steps to reproduce, what you expected and what happened;
7. the terminal log (with `SS2_LOG=debug`), and the panic message and backtrace if there is one
   (`RUST_BACKTRACE=1`);
8. for a desync, the log of every co-op peer from the same session.

Remove your user name and personal paths from the log before you post it.

**Do not attach or paste game files or game text.** That means `.gro`, `.wld`, `.tex`, `.mdl`,
`.exe`, `.dll` and other files from the game folder, and dialogue or menu text. The
[triage rules](../TRIAGE.md#8-game-content-in-reports) explain why. To identify your copy, you may
post the SHA-256 hash of a file you own, for example from `Get-FileHash -Algorithm SHA256` in
PowerShell or `sha256sum` on Linux and macOS.

A screenshot of the SS2RS window is fine if it shows only the bug and no copied game art. When in
doubt, describe the problem in words.

Security problems must not be reported as public issues. Use the private path in
[SECURITY.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/SECURITY.md).
