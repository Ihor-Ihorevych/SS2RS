# Netcode architecture guide

For people who write gameplay code and need to know how it meets the network, and for people who write netcode and need to
know what the rest of it assumes. It describes the **model** (agreed in the design spec), the **pieces that exist**, and the
**pieces that do not yet**, and it says which is which on every page. Where something is only proposed, it points to the RFC
that proposes it: until a maintainer approves that RFC, treat it as a plan, not a rule.

## 1. The model

From `docs/superpowers/specs/2026-11-21-ss2rs-design.md`:

- **Server-authoritative.** One simulation is the truth: the server's (a dedicated server, or the host of a listen server).
  Clients send **inputs**, never state.
- **Fixed timestep.** The simulation advances in ticks; time inside the simulation *is* the tick counter.
- **Snapshot interpolation for remote entities.** A client draws everyone else a little in the past, blended between snapshots.
- **Prediction and reconciliation for the local player.** A client runs its own player's movement ahead of the server with the
  inputs it has sent, and corrects itself when the server's answer differs.
- **Server-side lag compensation** for hitscan and melee (the server rewinds targets to what the shooter saw).
- **Co-op first**, join-in-progress included. **Deterministic lockstep or rollback was rejected**: it would depend on identical
  floating-point behaviour across CPU architectures, and join-in-progress is hard with it.

## 2. Layers

```
ss2-game     gameplay: player, weapons, enemies, level logic            (stub: no gameplay code yet)
ss2-sim      entities, components, the tick loop, replication           (stub; RFC 0005 proposes hecs behind a facade)
ss2-net      protocol logic over unreliable datagrams: pure, no sockets, no clock    (implemented, section 3)
ss2-transport   sockets, encryption, connections: QUIC datagrams       (proposed in RFC 0006; does not exist)
```

`ss2-sim`, `ss2-physics` and `ss2-net` are **sim-safe**: no `HashMap`/`HashSet`, no wall-clock time, no thread-order dependence
(`docs/CODING_STANDARDS.md`; `clippy.toml` in each crate enforces the first two). That is why the transport, which needs sockets
and a clock, is proposed as a separate crate. The design spec lists "transport" under `ss2-net`; RFC 0006 explains the change.

## 3. What exists in `ss2-net`

Everything is generic over the payload type, takes time as a number of microseconds, and was tested against the **network
simulator** (`docs/TESTING.md` section 3.7) with the numbers quoted in its own page. None of it is called by any game or transport
code yet.

| Module | Page | What it guarantees |
|---|---|---|
| `netsim` | `TESTING.md` 3.7 | deterministic latency, jitter, loss, duplication, reordering and bandwidth cap for tests; presets, including the **100 ms and 2 % loss** acceptance profile |
| `timesync` | `TIMESYNC.md` | clock offset and round trip from ping and pong (shortest round trip wins), tick conversion, jitter buffer delay |
| `inputstream` | `INPUTSTREAM.md` | tick-stamped inputs repeated until acknowledged; server-side validation, rate limit, per-tick buffering |
| `snapshots` | `SNAPSHOTS.md` | interpolation, short extrapolation, teleport and despawn handling, adaptive delay for remote entities |
| `events` | `EVENTS.md` | server-to-client events delivered exactly once, in order, with late-join suppression |
| `protocol` | `PROTOCOL.md` | protocol version rules, schema hash, handshake compatibility check |
| `netstats` | `NETSTATS.md` | traffic per tick by kind, round trip, jitter and loss over sliding windows, for an overlay and profiling |

Two properties run through all of them. **Reliability is built on unreliable datagrams by repetition** (inputs, events), so a lost
packet is repaired by the next one with no resend round trip; and **state is never repaired by resending**: the next snapshot
replaces it.

## 4. A tick, step by step

Steps marked **(built)** are the modules above; **(planned)** steps have a task but no code.

**Client, every frame**

1. Sample the input; stamp it with a tick that is a few ticks ahead of the server's; send the latest few unacknowledged frames
   (`InputSender`, built) in a datagram.
2. Run the local player's movement with that input (prediction, planned: T08-009).
3. Keep the clock in step with the server (`ClockSync`, built: ping every 100 ms at first, then every second).
4. Draw remote entities at `server_now - delay` (`SnapshotBuffer`, `InterpolationDelay`, `RenderClock`, built).
5. Hand out the events that arrived, in order (`EventReceiver`, built); acknowledge them in the next packet.

**Server, every tick**

1. Receive the inputs (`InputBuffer::receive`, built: refuses old, far-ahead, conflicting and flooding input), take one per client for
   this tick (`take`, built: repeats the last for a few ticks, then neutral).
2. Run the simulation phases in a fixed order: input, AI, move, physics, damage, cleanup, replicate (planned: T08-003).
3. Build the snapshot for each client from the replicated components (planned: T08-007) and put the unacknowledged events and the
   acknowledgement of the client's inputs in the same packet (`EventLog::packet`, `ack_tick`, built).
4. Send it as a datagram. A late joiner gets the full state as one **bulk** message instead (planned: T08-013).

## 5. Rules for gameplay code

### Sim-safe rules (in force today, for the crates that are sim-safe)

- Iterate in an order that depends only on what happened, never on a hasher, an address or a thread: use `BTreeMap`, `BTreeSet` or a
  `Vec`. Do not read the clock; time is the tick counter. Randomness only from a seeded generator whose state is part of the
  replicated state.
- Code that **predicts** (the player's movement) must give the **same result on the client and on the server** for the same inputs and
  state, on x86_64 and aarch64: the same operations in the same order, no platform-specific math (roadmap T07-026 tests this).

### What to replicate (proposed in RFC 0005; not yet a rule)

Each component type is registered once with a name, a numeric id and a **replication mode**. The guidance, until the replication
framework (T08-007) fixes the vocabulary:

| Kind of thing | Mode | Why |
|---|---|---|
| State the other players must see and that changes continuously (position, orientation, health, animation state) | in the **snapshot**, quantized | the next snapshot replaces a lost one |
| A thing that happens once (an explosion, a pickup taken, a chat line, a sound) | a **reliable event** (`EventLog`) | it cannot be reconstructed from state, and must not play twice |
| Something every client can derive from replicated state (a particle effect, a footstep, a muzzle flash that follows the shot event) | **not replicated**; derive it on the client | bandwidth |
| The local player's own input | an **input frame** (`InputSender`), never state | the client does not tell the server where it is |

An event for something that also changes state (a pickup removes an item and plays a sound): replicate the state change in the
snapshot and the sound as an event, and let the late joiner get the state without the sound (the event log's late-join cursor does this).

### Writing predicted code (planned: T08-009)

Predicted code is a **pure function of (state, input)**. It may read only replicated state, so that the server, which has exactly that
state, computes the same answer. It must not call anything that reads the clock, a hash order, an unseeded generator or state the
server does not have (the client's UI, its camera). When prediction and the server disagree, the client replays its unacknowledged
inputs from the server's state; code that cannot be replayed (it has side effects outside the replicated state) must be kept out of
the predicted path, and side effects such as sounds triggered by predicted movement are decided by the event rules above.

## 6. Testing netcode

- **Unit and scenario tests** drive the pieces with a virtual clock and `ss2_net::netsim` (`NetSim`, `Duplex`, `LinkConfig` presets).
  Each module's page lists what it was run against. The acceptance profile of the roadmap is `LinkConfig::acceptance_100ms_2pct()`.
- **Determinism tests** for the sim-safe crates: run the same script twice, compare; the entity store gets an iteration-order test
  (RFC 0005).
- **Fuzzing** (`docs/FUZZING.md`) covers parsers that take untrusted bytes; the packet decoders will join it when the wire format exists.
- **A real network** is not tested anywhere yet: the socket-level conditioner (T08-017b) and the transport (RFC 0006) are missing.
  Every number in these pages comes from a simulation.

## 7. What the player sees when something goes wrong

| Event | Effect | Handled by |
|---|---|---|
| a packet is lost | nothing visible: inputs and events repeat, the next snapshot replaces the state | redundancy |
| snapshots arrive late | remote entities extrapolate for up to 100 ms, then hold still (`Sample::Stale`: show a connection warning) | `SnapshotBuffer` |
| the delay rises (jitter) | the picture holds for the difference, then continues | `RenderClock` |
| an entity teleports or respawns | a jump, not a streak | `SnapshotFlags::teleport` |
| an event is older than the log keeps | skipped, not waited for | `EventReceiver` |
| a client floods or forges inputs | refused and counted; a kick policy can use the count | `InputBuffer`, T08-021 |
| client and server run different protocol versions | refused with a message that says who must update | `check_handshake` |
| the client's prediction is wrong | it is corrected (smoothly) | planned: T08-009 |

## 8. Glossary

- **Tick**: one step of the fixed-timestep simulation. **Snapshot**: the replicated state at a tick. **Input frame**: one tick's input from one client.
- **Interpolation delay**: how far in the past remote entities are drawn. **Reconciliation**: correcting the predicted local player to the server's state.
- **Datagram**: one unreliable packet. **Bulk**: a large reliable message, only for joining and level data.
- **Sim-safe**: allowed in code that must behave identically on every machine and every run.

## 9. Not covered yet

- The replication framework, prediction, lag compensation, join-in-progress, co-op rules, the server and client split: tasks T08-007, T08-009, T08-010, T08-013, T08-023, T08-004.
- The transport and the connection (T08-005's RFC 0006, T08-006), the wire format of every message, and session discovery (T08-019).
- Anti-cheat beyond the input stream's sanity limits (T08-021), net profiling (T08-018) and replays (T08-025).
- This guide is **provisional** where it describes the planned parts (sections 4 and 5): revise it when T08-007 and T08-009 land.
