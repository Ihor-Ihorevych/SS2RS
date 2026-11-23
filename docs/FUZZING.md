# Fuzzing and Miri

Parsers and loaders must answer any input with a value or an error, never a crash. Fuzzing feeds them random and mutated
input to look for the exceptions (T12-007). The unit tests of `docs/TESTING.md` check the cases someone thought of; the fuzzer
finds the ones nobody did.

## What is fuzzed

The `fuzz/` folder is a separate `cargo-fuzz` project (`ss2-fuzz`, its own `Cargo.lock`, not a member of the main workspace
because it needs nightly Rust). Each target is one file in `fuzz/fuzz_targets/` and states the property it checks:

| Target | Entry point | Property |
|---|---|---|
| `console_line` | `Console::execute`, `complete`, `tokenize` (with the cheat commands registered and `sv_cheats 1`) | any line gives a value or an error |
| `launch_options` | `parse_launch_options` for the client and the server | any command line is parsed or refused with a message |
| `settings_file` | `Settings::load_from` on a file of any bytes | never fails; the volumes come out inside 0 to 1 |
| `bindings_toml` | `BindingSet::from_toml_str` | parsed or refused; what parses can be written and read back identically |
| `vfs_path` | `VfsPath::new`, `parent`, `join` | normalizing twice changes nothing; no empty, `.` or `..` segment, no upper case or backslash |
| `gro_archive` | `ArchiveVfs::from_file` on any bytes, then list and read | opened or refused; what opens can be walked and read |
| `install_check` | `check_install` on a folder with one file of any bytes under a base, patch or foreign name | always a report |
| `archive_names` | `archive_load_order`, `PatchLevel::from_file_name` | the order does not depend on the input order |
| `session_message` | `ss2_net::session::decode` on any bytes, then a `SessionServer` fed the message from several peers | decoded or refused; what decodes round-trips; the server never holds more than its slots or failure records |
| `discovery_message` | `ss2_net::discovery::decode` on raw bytes and on a scripted valid reply damaged in a few places, an `Announcer` and a `Scanner` fed the datagram, and `parse_direct` and `decode_invite` on the same bytes as text | decoded or refused; **no reply is ever longer than its query**; the scanner's list stays bounded; a code that reads encodes back to the same address |
| `replay_parse` | `Replay::parse` on raw bytes, and on a scripted valid replay damaged in a few places | read or refused; what reads can be played through and written out again to the same records |
| `save_parse` | `ss2_sim::save::parse` on raw bytes, and on a world built by a script, saved, damaged in a few places and **re-sealed with a correct checksum** | read or refused; what reads restores into an empty world, saves again, and reads again with the same entities and header |

The `.wld`, `.tex`, `.mdl` and the other game formats are not fuzzed yet: their readers do not exist (track T02, with the
fuzz targets of T02-030). Add a target in `fuzz/fuzz_targets/`, a `[[bin]]` entry in `fuzz/Cargo.toml` and the name to the
matrix of `.github/workflows/nightly.yml` and to the table above; `cargo test -p xtask` fails when these four lists differ.

## Running it

```
cargo install --locked cargo-fuzz          # once
rustup toolchain install nightly           # once
cd fuzz
cargo +nightly fuzz run vfs_path -- -max_total_time=60 -max_len=4096
cargo +nightly fuzz list                    # the targets
```

On **Windows** (MSVC) the binaries load the AddressSanitizer runtime, which is not on the `PATH`: you get
`STATUS_DLL_NOT_FOUND` (`0xc0000135`) until you add the folder of `clang_rt.asan_dynamic-x86_64.dll`
(under `VC\\Tools\\MSVC\\<version>\\bin\\Hostx64\\x64` in your Visual Studio or Build Tools install) to `PATH`. A first build takes
a few minutes. On Linux and WSL nothing special is needed.

The corpus (`fuzz/corpus/`) and the crash files (`fuzz/artifacts/`) are not committed. A target that is run with no
corpus starts from nothing; for the archive targets a small ZIP file placed in `fuzz/corpus/gro_archive/` helps libFuzzer find
the structure faster (build one with `ss2_testkit::fixtures::ZipBuilder`). **Never put a real `.gro`, `.wld` or any other game
file in the corpus or commit one**: `cargo xtask guard` rejects them and the clean-room rules (`CLEANROOM.md`) forbid it.

## Nightly runs

`.github/workflows/nightly.yml` runs every target for 5 minutes (the `seconds` input of a manual run changes that) and
Miri on three pure crates. A crash fails the job and uploads the crashing input as an artifact, and GitHub notifies the owner.
The `miri` job is marked `continue-on-error` until a maintainer has seen it pass on Linux: only `ss2-game` was run under
Miri here, on Windows, where Miri cannot do file-system calls (`ss2-platform-api` stops at `CreateDirectoryW`).

The workspace sets `unsafe_code = "deny"`, so neither AddressSanitizer nor Miri has Rust code of ours to check for memory
errors; they guard against undefined behaviour inside the dependencies (`zip`, `flate2`, `toml`) and catch logic that only
looks safe. Nothing else needs a sanitizer today. If `unsafe` is ever allowed in a crate (the platform layer for Android, for
example), add that crate to the Miri line and run its tests under AddressSanitizer in this workflow.

## When the fuzzer finds something

1. Download the artifact (or take `fuzz/artifacts/<target>/crash-...`) and reproduce:
   `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/crash-...`.
2. Write a normal unit test with that input in the crate that owns the code (the same crate-owner rules as any bug), watch it
   fail, fix the code, watch it pass.
3. Keep the input as a seed: copy a **small hand-made or synthetic** reproducer to `fuzz/seeds/<target>/` with a note of where it
   came from (a crash found with game data may contain game bytes: reduce it with `cargo fuzz tmin` and check it by hand first, or
   describe the case in words and build it with a test builder instead).

## What the first runs found

Each of the eight targets ran for 30 to 60 seconds on Windows (20,000 to 140,000 executions each, `console_line` the
slowest) with no crash in the code under test. One run stopped on an assertion of my own in `vfs_path`: it required that a
normalized path never contains `..`, but a name such as `a..b` is legal (only a whole `..` segment is refused). The target now
checks segments. This is a smoke test, not coverage: the nightly job runs 5 minutes per target, and a long run on a
corpus is where bugs are found.
