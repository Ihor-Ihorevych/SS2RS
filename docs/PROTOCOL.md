# Network protocol: versioning, compatibility and changelog

`ss2_net::protocol` (T08-030) defines how two peers decide whether they can talk. It does not define the messages
themselves: there is no wire format yet (the transport is T08-005, the handshake T08-006, the snapshot protocol T08-007).

## The version

`ProtocolVersion { major, minor }`, shown as `1.4`. It is the **protocol** version, not the product version: a release can
change the game without touching it, and a client and a server must refuse each other cleanly when it differs
(`docs/RELEASING.md` section 1).

| Change | Bump | Meaning |
|---|---|---|
| a field removed, retyped or reordered; a message removed or its meaning changed; a new required step | **major** | an older peer cannot understand it |
| a new message or a new trailing field that an older peer can ignore | **minor** | compatible: the peers talk at the **lower** minor of the two |
| before the first release (**major 0**) | any change is a minor bump | every minor may break, so peers must have **exactly the same** minor |

## The schema hash

A version number is a promise made by people. `Schema` is a check made by the machine: list every message and its fields in wire
order (`Schema::new().message("Input", &[("tick", "u64"), ("buttons", "u32")])...`) and `Schema::hash()` gives a 64-bit FNV-1a hash
of that layout (length-prefixed, so `("ab", "c")` differs from `("a", "bc")`). Changing a field name, a type, the order of fields or
of messages, or adding a field, changes the hash. FNV-1a is a fingerprint against **accidents**, not a security measure.

The build keeps its schema next to the message definitions and a test pins the hash of it, so that changing a message without
bumping the version fails the build. (The pinned value in `crates/ss2-net/src/protocol.rs` is a test of the hash function and the
encoding; the game's real schema does not exist yet.)

## The handshake check

Each peer sends `Handshake { version, schema_hash }` first. `check_handshake(local, remote)` answers:

- `Compatible(version)`: same major (and, at 0.x, same minor); `version` is the lower minor, which both must restrict themselves to;
- `VersionMismatch { local, remote }`: the majors differ (or the 0.x minors do); `explanation()` says which side must update;
- `SchemaMismatch`: the **same** version but **different** layouts, which means the two builds are not from the same source or the
  version was not bumped. The hashes of different minors are not compared, because they differ by design.

`Compat::explanation()` returns a sentence for the player or the log (`the other side is newer (protocol 2.0, this build speaks
1.0): update this game`).

## Mod hash and session password

The roadmap line for the connection (T08-006) also checks a mod hash. That is a different mechanism (the loaded content, not the
wire format) and belongs there.

## How to change the protocol

1. Change the messages and the `Schema` in the same commit.
2. Decide major or minor with the table above; bump `PROTOCOL_VERSION` (when it exists) accordingly.
3. Re-pin the hash in the schema test.
4. Add a line to the changelog below.

## Wire-format changelog

| Version | Change |
|---|---|
| 0.0 | No wire format exists. The version check and the schema hash are defined (T08-030); the messages are not (T08-005, T08-006, T08-007). |

## Not covered

- The messages, the handshake exchange and the retry or timeout behaviour (T08-006).
- Negotiating optional features beyond the lower minor, and protocol upgrades of a running session.
