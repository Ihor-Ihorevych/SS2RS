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
| `meta_container` | `ss2_meta::Container::parse`, the type system and `decode_objects`, then `write` | read or refused; what decodes writes, reads back to the same objects and writes again to the same bytes |
| `format_tex` | `ss2_format_tex::Texture::from_bytes` | read or refused; the levels of what reads add up to the pixel buffer exactly |
| `format_bmf` | `ss2_format_bmf::Mesh::from_bytes` | read or refused; every stream has one entry per vertex and every index is inside the vertices |
| `format_ans` | `ss2_format_ans::AnimSet::from_bytes`, then sampling, events and deltas at edge times (NaN, infinity, negative, huge) | read or refused; every pose is parallel to its tracks |
| `format_ep` | `ss2_format_ep::ParamTree::from_bytes`, `EntityParams::from_bytes` | read or refused; an entity parameter file derives from `CEntityParams` |
| `format_fnt` | `ss2_format_fnt::Font::from_bytes`, then glyph lookup and text layout | read or refused; every code finds a glyph inside the table |
| `format_fx` | `ss2_format_fx` `pfx::Effect`, `lfx::FlareConfiguration`, `tfx::TextEffectConfiguration`, `vfx::VibrationEffect` on the same bytes | read or refused |
| `format_mat` | `ss2_format_mat` `PhysicalMaterial`, `SoundScheme`, `ParticleScheme` on the same bytes | read or refused |
| `format_mtr` | `ss2_format_mtr::MaterialFile::from_bytes` | read or refused |
| `format_rsc` | `ss2_format_rsc::ResourceDatabase::from_bytes`, `CharacterTool::from_bytes` | read or refused |
| `format_wld` | `ss2_format_wld::World::from_bytes` | read or refused; a world that reads has no more models and markers than entities, and every model transform builds |
| `audio_decode` | `ss2_audio::decode` (`sniff`, `probe`, `decode`, `decode_wav`, `VorbisStream`, `last_granule`) and `ss2_audio::wav::decode` | decoded or refused, **no panic even one the decoder catches itself**; decoded sound has whole frames and finite samples |
| `audio_event` | `ss2_audio_api::replication::AudioEvent::decode` | decoded or refused; what decodes encodes to the same bytes again |
| `game_state_bytes` | `ss2_game` `CarryOver`, `UnlockSet`, `CampaignNote`, `CampaignDifficulty`, `LevelProgress` `from_bytes` (the last also with a corrected checksum) | read or refused; what reads writes and reads again to the same bytes |
| `mods_text` | `ss2_vfs::ModManifest::parse`, `Nfo::parse`, `parse_current_mod` | read or refused; every key that was read is found again |
| `locate_text` | `ss2_vfs::parse_reg_query`, `parse_library_folders`, `load_choice` | always an answer |
| `lobby_text` | `ss2_ui::lobby::parse_address`, `normalize_invite` | read or refused; normalizing an invite twice changes nothing |
| `net_chunks` | `ss2_net::join::ChunkSender` and `ChunkReceiver`, driven by `arbitrary` | an honest transfer (random order, repeats, drops) always delivers the blob intact; chunks with any header never panic or grow the receiver past its limit |

The first twelve targets (T12-007) keep their property in the file of the target; the others (seventeen from T02-030, then one per reader added since) are one line
each, `fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_tex(data))`, and their property lives in the workspace crate
`crates/ss2-fuzzkit`. That split is deliberate: **the same properties also run on stable**, in the normal test suite
(`cargo test -p ss2-fuzzkit`, next section), so a parser that breaks one is caught on the pull request and not a night
later. Add a target the same way: a function in `crates/ss2-fuzzkit/src/targets.rs`, an entry in `TARGETS` (`lib.rs`) and
seeds in `seeds.rs`, a file in `fuzz/fuzz_targets/` calling it, a `[[bin]]` in `fuzz/Cargo.toml`, the name in the matrix of
`.github/workflows/nightly.yml` and a row in the table above; `cargo test -p xtask` fails when the four lists differ and
`cargo test -p ss2-fuzzkit` fails when the crate, the fuzz folder and the committed seeds differ.

Every CTSEMETA reader is fuzzed through its own target and, underneath, `meta_container`: all the formats of the game that
have a loader today are containers (`.tex`, `.bmf`, `.ans`, `.ep`, `.fnt`, `.pfx .lfx .tfx .vfx`, `.mat .scs`, `.mtr`, `.rsc .ctl`, `.wld`).

### What is not fuzzed, and why

A reader that does not exist cannot be fuzzed. These stay open as the roadmap task `T02-030b` and each gets its target in the
pull request that adds the reader (the definition of done of every reader task says so):

| Reader | Why not yet |
|---|---|
| the full `.wld` world (T02-020; the minimal reader of T02-035 is `format_wld`), `.mdl` model (T02-017), `.skl` skeleton (T02-015) | the loader does not exist (`ss2-world` is an empty crate) |
| `.srt .dsc` and string tables (T02-027), `.preload` (T02-028), `.cfg .mac .scp` (T02-029) | the loaders do not exist |
| `ss2-server` configuration (`ServerSettings::from_toml`), `ss2-app` command line (`parse_args`) and the remembered data location | these live in binary crates with no library target, so a fuzz crate cannot call them; moving them into a library is a change in another crate |
| `ss2-render` golden-image PNG decoder, `ss2-render` GPU name parsing | test and diagnostics code behind `wgpu` (a large dependency for a fuzz binary); the PNG decoding itself is the `png` crate |
| `ss2-script`, `ss2-shaderxlat` | empty crates |
| Android content provider (T02-005) | Phase 2 |

## Running it

```
cargo install --locked cargo-fuzz          # once
rustup toolchain install nightly           # once
cd fuzz
cargo +nightly fuzz run vfs_path -- -max_total_time=60 -max_len=4096
cargo +nightly fuzz list                    # the targets

# the way the nightly job runs a target: start from its committed seeds, and bound time and memory
mkdir -p corpus/format_tex && cp seeds/format_tex/* corpus/format_tex/
cargo +nightly fuzz run format_tex -- -max_total_time=60 -max_len=8192 -timeout=20 -rss_limit_mb=2048 -malloc_limit_mb=1024
```

The stable checks, which need no nightly, no libFuzzer and no sanitizer:

```
cargo test -p ss2-fuzzkit                              # 3,000 inputs per target, about 15 seconds
SS2_FUZZKIT_ITERS=100000 cargo test -p ss2-fuzzkit --test smoke every_target   # a longer sweep
UPDATE_FIXTURES=1 cargo test -p ss2-fuzzkit --test smoke the_committed         # rewrite fuzz/seeds/ after changing a builder
```

On **Windows** (MSVC) the binaries load the AddressSanitizer runtime, which is not on the `PATH`: you get
`STATUS_DLL_NOT_FOUND` (`0xc0000135`) until you add the folder of `clang_rt.asan_dynamic-x86_64.dll`
(under `VC\\Tools\\MSVC\\<version>\\bin\\Hostx64\\x64` in your Visual Studio or Build Tools install) to `PATH`. A first build takes
a few minutes. On Linux and WSL nothing special is needed.

The corpus a run grows (`fuzz/corpus/`) and the crash files (`fuzz/artifacts/`) are not committed. What is committed is the
**seed corpus**, `fuzz/seeds/<target>/*.seed`: a few tiny, synthetic inputs per target (230 KiB in all), from which a run
starts instead of from nothing. Where they come from, in this order: the file writers of the repository (the testkit's
`MetaBuilder` and the byte layouts of each format crate's own tests, which are written from the clean-room specs in
`docs/re/`), the hand-made files of `testdata/` (each has a provenance note there), and a few lines of text written by hand.
**No seed is read from a game install, and never put a real `.gro`, `.wld` or any other game file in a corpus or commit one**:
`cargo xtask guard` rejects them and the clean-room rules (`CLEANROOM.md`) forbid it. The seeds are generated, not edited by
hand: `cargo test -p ss2-fuzzkit` fails when `fuzz/seeds/` differs from what the builders write, and
`UPDATE_FIXTURES=1 cargo test -p ss2-fuzzkit --test smoke the_committed` rewrites it. A crash found by a nightly run may be
added as a seed only after it is reduced and checked by hand (see the last section).

## Stable checks: the same properties without nightly

`cargo test -p ss2-fuzzkit` (part of `cargo test --workspace`) runs every target of the 17 above on its seeds and on 3,000
deterministic inputs each: a seed with one or two random edits (flipped bit, byte, a number replaced by 0, 1, 255, 65535,
2^31 - 1, 2^31, 2^32 - 1; a range cut, repeated, or truncated), two seeds spliced, a seed cut anywhere, and random bytes. It
fails on a panic **including one that the code under test catches itself** (the Vorbis decoder runs `lewton` under
`catch_unwind`; libFuzzer would report that panic, so the test does), on an input that takes more than 10 seconds, and when a
seed stops being a valid file of its format (a builder that drifted from its reader). It is not coverage-guided and does not
check memory: that is what the nightly run, with AddressSanitizer and `-rss_limit_mb`, adds.

## Nightly runs

`.github/workflows/nightly.yml` runs every target for 5 minutes (the `seconds` input of a manual run changes that), each starting
from its seeds and with `-timeout=20 -rss_limit_mb=2048 -malloc_limit_mb=1024` so that a hang or a runaway allocation is a failure, and
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

### T02-030 (the 17 format, sound and state targets)

Each target ran 60 seconds under libFuzzer with AddressSanitizer on Windows, starting from its seeds (from 585 executions for
`audio_decode`, which decodes whole Vorbis streams, to 858,000 for `audio_event`; 17 of 17 ended without a crash, timeout or
out-of-memory). The stable sweep (`SS2_FUZZKIT_ITERS=60000`, a debug build) stopped at the 8,463th input of
`format_ans`: `Animation::span_seconds` computed `last_frame - first_frame` in `i32`, which overflows for a file whose frames run
from about -2^31 to 2^31 (a panic in a debug build, a wrong number in a release build). It is fixed in `ss2-format-ans` (the
difference is computed in 64 bits) with a unit test, and the sweep then passed 40,000 inputs for each of the 17 targets. The
nightly run and a longer sweep are where more will be found.
