# testdata/

Synthetic fixtures for tests that must run on plain CI (no game data available there).

Rules:
- Only **hand-made, license-clean** files, each **<= 256 KiB**.
- **Never** excerpts, trimmed copies or re-encodings of Croteam files (`.gro` contents, `.tex`, `.mdl`, ...).
  `cargo xtask guard` rejects proprietary extensions here except small hand-made fixtures.
- Fixture *builders* (`write_minimal_zip`, a minimal CTSEMETA writer, ...) arrive with the first
  loader tasks (T02-001 / T02-006); until then this folder is intentionally empty.
- Tests that need the real game use `ss2_testkit::require_game_data!()` and `SS2_DATA_DIR`.
