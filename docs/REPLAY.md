# Replay files

`ss2_net::replay` (T08-025) is the file format and the reader and writer for **replays**: a recorded session that can be played
back, attached to a bug report, and checked for desyncs. It is pure data and logic (no clock, no hash map, no I/O of its own),
so it sits with the other sim-safe netcode modules and can be fuzzed.

It knows nothing about what an input or a snapshot **means**: payloads are opaque bytes that the game encodes. What it
guarantees is the container.

## What is in a replay

| Part | Content |
|---|---|
| **Header** | build text, protocol version, **schema hash** (a replay of another schema cannot be played), tick rate, random seed, map, player count, up to 16 short `key=value` notes |
| `Input` records | a player's input for a tick |
| `Snapshot` records | the whole game state **before** the inputs of that tick: keyframes to start playback or to seek from |
| `Event` records | gameplay events (for tools that read replays) |
| `StateHash` records | a hash of the state after a tick: playback calls `check_hash` and the first tick that differs is where it **drifted** from the recording |
| **End** record | the number of records: a replay with a right end record is **complete** |

Records are in tick order (the writer refuses a record for an earlier tick; the reader stops at one).

## The byte format (version 1, little endian)

```
file    = "SS2R" · version u16 · header_len u32 · header · crc32(header) u32 · record*
record  = kind u8 · len u32 · payload · crc32(kind, len, payload) u32
kinds   = 1 Input (tick u64, player u8, data) · 2 Snapshot (tick u64, data) · 3 Event (tick u64, data)
          · 4 StateHash (tick u64, hash u64) · 0xFF End (record count u64)
header  = build str · protocol u32 · schema_hash u64 · tick_hz u32 · seed u64 · map str · players u8 · meta_count u8 · (key str, value str)*
str     = len u16 · UTF-8 bytes (at most 256)
```

The CRC is the usual CRC-32 (IEEE 802.3, as in zip and PNG).

## Damaged, cut and hostile files

A crash is when a replay is most wanted, and a file attached to an issue is untrusted input, so the reader is built for both:

- **Only a bad header is an error** (`BadMagic`, `UnsupportedVersion`, `HeaderTruncated`, `HeaderCorrupt`, `HeaderInvalid`, each with a message).
  Anything wrong after the header ends the replay **at the last good record** and `Replay::end()` says why
  (`MissingEnd`, `CutShort`, `BadChecksum`, `UnknownKind`, `TooLarge`, `OutOfOrder`, `TooManyRecords`, `BadPayload`, `CountMismatch`,
  `TrailingData`), with the byte offset. A recording whose game crashed (no end record) is therefore still playable up to the crash.
- **Every record is checked by CRC-32** before it is used. A test flips **every** bit position of a replay (three bits per byte) and
  checks that the records read are always a prefix of the original and the file is never called complete; another cuts the file at
  every byte.
- **Lengths are checked before anything is read or allocated** (a record claiming 4 GiB is refused as `TooLarge` without reading it);
  memory is bounded by `Limits` (4 MiB per record, 5,000,000 records by default).
- The fuzz target `replay_parse` (`docs/FUZZING.md`) feeds raw bytes **and** a script that builds a valid replay and damages it in a few
  places (so the checksums are right and the fuzzer reaches the record logic); a replay that reads must play through and write out
  again to the same records. 90 seconds, 90,000 runs, no finding.

## Using it

```rust
// Recording (the game calls this once per tick)
let mut writer = ReplayWriter::new(header)?;
writer.push(Record::Snapshot { tick, data: state_bytes })?;     // every few seconds
writer.push(Record::Input { tick, player, data: input_bytes })?;
writer.push(Record::StateHash { tick, hash })?;
file.write_all(&writer.drain())?;                // optional: stream a long session to disk; never repeats bytes
file.write_all(&writer.finish())?;               // the rest, plus the end record

// Playback
let replay = Replay::parse(&bytes, &Limits::default())?;
let mut playback = Playback::new(&replay);
let (tick, state) = playback.seek(wanted_tick).unwrap();   // latest keyframe at or before: restore `state`
while let Some((tick, records)) = playback.step() {         // that tick's inputs, events and hashes, in order
    // apply the inputs, step the sim, then:
    // playback.check_hash(tick, sim.hash()) -> Match | Mismatch { expected } | NotRecorded
}
```

A keyframe is the state **before** the inputs of its tick, so after restoring it, `step()` returns that tick's inputs first.

## Not covered (follow-up T08-025b)

This is the container only. Still to do: recording from the simulation (it does not exist yet: T08-003), the playback driver in the
game and the `--replay <file>` launch option, a console command to save the last minutes, **compression** (snapshots will want it; a
compressed record kind or whole-file wrapper needs a format version), replays across **different builds** (only the schema hash is
compared; whether two builds with the same schema simulate identically is the determinism work of T07-026 and T08-026), and any
**privacy review** of what a replay contains (player names in the notes, the host's name): a replay is meant to be attached to a
public issue.
