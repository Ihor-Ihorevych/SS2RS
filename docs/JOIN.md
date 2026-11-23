# Joining a game in progress

`ss2_net::join` (T08-013) is the machinery for letting a player into a session that is already running: **streaming the full state** to them, **tracking where each joiner is**, and **keeping the events consistent** across the hand-over from snapshot to live play. It is pure logic like the rest of `ss2-net` (time is a number of microseconds passed in, no randomness, no `HashMap`) and it carries **no messages yet** (see the end).

## Chunked transfer

A full state is one big blob (the encoded world: how it is encoded is the simulation's business, see `docs/SAVES.md`). `ChunkSender` cuts it into numbered **`Chunk`s** of 1,024 bytes (the transport's packet size is the limit), each carrying the transfer id, its index, the total count, the blob's length and a checksum. `ChunkReceiver` takes them in any order and any number of times, and answers with a **`ChunkAck`**: one bit per chunk. The sender then repeats **only the missing chunks**, and **cycles** through them (a call continues after the chunks the last call sent and wraps, so a lost chunk comes round again within one pass of the missing ones instead of the first ones being resent forever). When every chunk has arrived, `finish()` checks the length and the **checksum** (FNV-1a, an integrity check, not a defence against tampering: the transport authenticates) and returns the blob; a corrupt blob resets the receiver so a clean copy can be taken.

The receiver is **strict about what it accepts**: the first chunk fixes the transfer id, the count, the length and the checksum, and an impossible header (zero chunks, a length over the limit, a count that does not match the length at the chunk size) is refused before anything is allocated; every later chunk must agree, its index must be in range and its data must have exactly the right length (the last chunk is the remainder). A refused chunk changes nothing (a bad *first* chunk does not even fix the header). Tested with loss, reordering, duplicated chunks and lost acknowledgements together: a 100-chunk blob arrives intact having sent under three copies of each chunk, with one third of them dropped.

## Tracking joiners: `JoinManager`

Per joiner the manager keeps a **phase**: `Queued` -> `LoadingLevel` -> `AwaitingBaseline` -> `Transferring` -> `CatchingUp` -> `InGame`, and refuses a step in the wrong phase (`WrongPhase`) without changing anything.

- **Admission.** `begin(peer)` queues; `tick(now)` admits queued joiners **oldest first, up to `max_active` (2) at a time** (snapshots are expensive; the rest wait, and `queue_position` says where). At most `max_tracked` (16) are tracked.
- **Level sync.** `level_ready(peer, hash)` compares the joiner's level hash with the server's: a mismatch **fails the join** (`LevelMismatch`) and frees the place. `set_level(hash)` (the server changed level) cancels everyone in progress.
- **The snapshot.** `start_transfer(peer, baseline_tick, event_seq, blob)` records the tick the snapshot was taken at and **the last event sequence number it contains**, and starts a `ChunkSender` for the joiner; `next_chunks`, `on_ack` and `snapshot_applied` carry it through. `snapshot_applied` is refused until every chunk is acknowledged.
- **Catching up.** `progress(peer, joiner_tick, server_tick)` moves the joiner in once it is within `catch_up_ticks` (3) of the server (exactly 3 behind counts; 4 does not; a joiner ahead of the server counts).
- **Timeouts**, in `tick`: 120 s to load the level, **10 s without new transfer progress** (an acknowledgement of nothing new does not reset the timer), 15 s to catch up. A failed join frees its place **in the same tick**, so the next queued joiner is admitted at once.

## Late-joiner event suppression

The reliable event stream (`docs/EVENTS.md`) already lets a client that joins *now* start after the events so far. For a joiner who spends seconds receiving a snapshot that is not enough: events happen **while the snapshot is in flight**, and they must neither be lost nor replayed. So the baseline is **the event sequence number at the moment the snapshot was taken**: `event_cursor(peer)` returns an `EventCursor` that has "acknowledged" exactly that much. The events inside the snapshot (their effects are in the state) are never sent; the events after it are sent once the joiner is ready. A test runs that with a real `EventLog` and `EventReceiver`: five old events are skipped and the two that happened during the transfer arrive in order.

One more failure: if the log was **pruned** past the first event the joiner needs while the snapshot was in flight (a slow joiner), the snapshot is too old and `snapshot_applied` fails the join (`SnapshotTooOld`); the joiner must start a new transfer. The check is exact: a log that still starts at the first needed event is fine.

## How it was tested

Twenty-two unit tests and two doctests: blobs of eleven awkward sizes round-tripping, the lossy channel above, the cycling of repeats (and that only the one missing chunk is repeated), a dozen kinds of bad chunk and a bad header, a corrupt blob, impossible blobs and foreign acknowledgements, acknowledgement bits beyond the total, the checksum against the published FNV-1a values, every phase in order, every out-of-order step, the placement limits, a level mismatch, the queue order, the exact times of the three timeouts, a failed join making room in the same tick, a snapshot too old (three log positions), the event cursor against a real log, a level change, and a joiner ahead of the server. **Twenty-six deliberate breakages: twenty-two failed a test and one was caught by hanging; three survived my first tests**: two real (the bounds of `ChunkAck::has`, and exactly the allowed number of ticks behind), now covered, and one equivalent (dropping the sender after the snapshot is applied only frees memory). The first run of the tests also found a **real bug in my first `next_chunks`**: it recomputed the scan position from a cursor that the loop itself was moving, so it skipped chunks.

## Not covered (follow-up T08-013b)

- **No messages.** The session protocol has no `Chunk`, `ChunkAck` or "level ready" message, and no channel for them; nothing calls `JoinManager`. Adding them is a protocol change (`docs/PROTOCOL.md`) that also needs the transport (RFC 0006, not approved or built).
- **The full state itself.** Nothing encodes the world into the blob or applies it: that needs the entity store (T08-002) and a way to encode and decode everything replicated (`ss2_sim::save` is the likely basis, unchecked).
- **Spawn rules are not done** (where and when the joining player appears, with what, how they are protected for a moment): that is gameplay, T09. **This is why its GitHub issue is left open**: its title promises them.
- **No bandwidth control**: how many chunks to send per tick is the caller's choice (`next_chunks(max)`), with no estimate of the joiner's bandwidth and no pacing against the live snapshot stream. The limits (2 at a time, the timeouts, 1,024 bytes) are my numbers.
