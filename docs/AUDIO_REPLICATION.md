# Replicated audio events (co-op)

`ss2_audio_api::replication` (T06-015) is how sounds that one player causes reach the other players' speakers in co-op: **who hears what**, **one-shots against loops**, and **what a late joiner gets**. It is device-free and **transport-free**: nothing here sends a packet. The server side (`AudioReplicator`) decides, `AudioEvent` is the message with a compact validated byte form, and the client side (`AudioEventPlayer`) turns messages into `Audio` calls.

## The messages

`AudioEvent` has three kinds:

- **`OneShot { sound, params }`**: play once and forget (a gunshot, a pickup). **Never replayed** to anyone who joins later: it is already stale.
- **`LoopStart { emitter, sound, params }`** and **`LoopStop { emitter, fade_ms }`**: a loop (an engine, a torch, a hum) is **state**. It lasts until the matching stop.

Sounds are named on the wire by a **`SoundRef`**, a number both sides agree on (an index into a table built from the level's sound list), not by `SoundId`, which only means something to the audio system that issued it. Loop emitters are named by an `EmitterId` (an entity and a slot, say).

**Wire form** (`encode` / `decode`): a tag byte, little-endian fields, one byte each for bus, priority, volume (0 to 4 in steps of 0.016) and pitch (0.1 to 4 in steps of 0.015), a placement flag and, if placed, position and the two ranges as `f32`. A placement's **velocity is not sent** (zero on arrival). `decode` must consume the message exactly and **never panics**: it refuses an unknown tag, a short or over-long message, an unknown bus or flag, and parameters that fail `PlayParams::validate` (NaN, a range with min above max). A test feeds it 20,000 pseudo-random byte strings and checks that whatever decodes re-encodes and decodes again.

## Who hears what (the server)

- **`Scope`**: `All` (everyone in earshot), `Owner(client)` (only that client: a damage cue, a sound of your own) and `AllExceptOwner(client)` (everyone but the owner, who **played it at once by prediction** and must not hear it twice).
- **Earshot** is the sound's own `max_distance` from each client's listener (a client has one position); a sound **with no placement** (2D) is in earshot of everyone in scope, at any distance. A listener exactly at `max_distance` hears it.
- `route_one_shot(params, scope, listeners)` returns the clients to send a one-shot to, in client order, each once.
- **Loops**: `start_loop`, `stop_loop`, `move_loop` change the server's list; **`update(listeners)`** (once per tick, with every connected client and where it listens) returns the events to send, in client then emitter order: it **stops** ended loops for the clients that know them (with the fade), **starts** loops that are in range and in scope for clients that do not know them yet, and **stops** loops a client knows that are now out of range or out of scope. Starting is at `max_distance`, stopping at **1.25 times** it (`LEAVE_FACTOR`), so a listener walking along the edge does not make a loop start and stop every step.
- **A late joiner** is simply a client the replicator has not seen: its first `update` gives it a `LoopStart` for every loop in earshot, at once, and nothing else. **A client missing from `listeners` is forgotten** and, if it returns, is new.
- Replacing a loop (`start_loop` for an existing emitter) stops the old one for the clients that know it and then starts the new one. A moving emitter's current position goes to clients that start hearing it later; clients that already hear it keep the position they were told (no per-tick updates yet).

## The client

`AudioEventPlayer::new(sounds)` maps `SoundRef(n)` to `sounds[n]`. `apply(audio, event)` plays one-shots, starts loops (a **duplicate `LoopStart` for an emitter already playing is ignored**: reliable delivery may send it twice) and stops them with the fade; a stop for an unknown emitter is ignored; a sound outside the table or parameters the audio refuses are **counted in `dropped`** and skipped, never a panic and never an obstacle to later events. The **event kind** decides looping, whatever the `looped` flag in the parameters says. `clear` stops every loop at once (level change, disconnect).

## How it was tested

Twenty-one unit tests and two doctests: routing by scope and range (the edge inclusive, 2D at any distance, a client listed twice), starting loops for clients in range only and not repeating, entering and leaving with hysteresis at 59, 70, 75 and 75.5 units from a 60-unit range, ending a loop for only the clients that know it with the right fade, a late joiner getting exactly the loops in earshot (and the 2D one) and no one-shots, scope on loops and a disconnected client being forgotten, replacing a loop, a moving emitter, the wire round trip within the quantization (every bus, the edges of volume and pitch, values beyond the range clamped), malformed messages (every truncation of a valid one, a trailing byte, a bad bus, a bad flag, NaN, an inverted range), 20,000 random byte strings, the client's play, loop, duplicate and refusal behaviour, the stop's **fade time** (through a recording `Audio`) and the whole pipeline (server to bytes to client) for a late joiner. **Twenty-three deliberate breakages: twenty failed a test at once, and three more needed attention**: two survived my first tests and were real (the wire's rounding, which my tolerance was too loose to see, and the fade time of a stop, which `NullAudio` does not record, so I wrote a small recording `Audio`), and one did not apply the first time because rustfmt had reflowed the line and failed a test when redone.

## Not covered (follow-up T06-015b)

- **No transport**: nothing sends these bytes. The netcode's transport (RFC 0006) is not approved or built, so there is no channel choice either: loops and their stops want **reliable, ordered** delivery, one-shots **unreliable**; the replicator does not retransmit or acknowledge, so a lost `LoopStart` is only repaired if the caller re-sends the state (a periodic full refresh of a client's known set would do it).
- **Nothing produces the events**: no weapon, enemy or ambient emitter calls `route_one_shot` or `start_loop`, and the table of `SoundRef`s is not built from a level's sound list.
- **No per-tick emitter updates** (a moving engine's position, a changing volume): only the position at the time a client starts hearing the loop is sent. No sound occlusion (a wall between emitter and listener still counts as earshot), no priority-based dropping when the network is congested, and no team scopes.
- **The numbers are mine**: the 1.25 hysteresis, the wire's quantization and the byte layout are design choices, not parity with the original game, which was not looked at.
