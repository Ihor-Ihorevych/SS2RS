# testdata/

Synthetic fixtures for tests that must run on plain CI (no game data available there).

Rules:
- Only **hand-made, license-clean** files, each **<= 256 KiB**.
- **Never** excerpts, trimmed copies or re-encodings of Croteam files (`.gro` contents, `.tex`, `.mdl`, ...).
  `cargo xtask guard` rejects proprietary extensions here except small hand-made fixtures.
- Fixture *builders* live in `ss2_testkit::fixtures` and generate files at test time, so most tests
  need no committed file at all (prefer a builder over a binary in git):
  - `ZipBuilder` / `write_minimal_zip`: minimal stored-only ZIP (what `.gro` archives are); output is
    deterministic, so tests can compare exact bytes.
  - `testdata_dir()` / `testdata_path()`: locate this folder from any crate's tests.
  - A minimal CTSEMETA writer follows once the container spec (T01-004/T01-005) is merged: **T00-033**.
- A committed fixture needs a one-line provenance note (who made it, how) next to it or in this file.
- Tests that need the real game use `ss2_testkit::require_game_data!()` and `SS2_DATA_DIR`.
