# Network threat model

What can go wrong when SS2RS talks to other players over a network, what is done about it, and what is not. This is the threat
model that `SECURITY.md` points to for the netcode ("it must validate every field, rate-limit, and never trust client-reported game
state"). It is written **before the transport and the game exist**, so it states which defences are **built** (modules of `ss2-net`,
each tested), which are **proposed** (RFC 0005, RFC 0006) and which are only **planned** (a task), and it marks every claim that
nobody has tested. A threat model that says "mitigated" about something that does not exist would be worse than none.

Related pages: `docs/NETCODE.md` (the architecture), `docs/PROTOCOL.md`, `docs/INPUTSTREAM.md`, `docs/EVENTS.md`, `docs/FUZZING.md`,
`docs/rfcs/0006-network-transport.md`, `SECURITY.md` (reporting).

## 1. What is being protected

| Asset | Why it matters |
|---|---|
| The **host's machine and the player's machine** | a packet must never crash, hang or compromise either (parsers, buffers) |
| The **fairness of the session** for the people in it | co-op is a shared experience; one player ruining it for the others is the main game-level threat |
| **Availability** of a server or a session | flooding it, or filling its memory, ends the game for everyone |
| **Privacy** of the players | a peer learns the IP address of the others; invite codes and the session password are secrets |
| The **authority of the simulation** | the server's state is the truth; if a client could write to it, every other protection fails |

**Not protected:** the game's data files (the player owns them), single-player cheating (the player may do what they like; the cheat
commands of `docs/CHEATS.md` are an example), and anything about the original game's own bugs.

## 2. Who might attack, and what each can do

| Attacker | Position | Capability |
|---|---|---|
| **Malicious client** | a player in the session | sends any bytes on its own connection; can modify its own game; can run a cheat tool |
| **Malicious host or server** | the server of a session a victim joins | sends any bytes to the victim's client; controls the simulation the victim sees |
| **Network attacker (passive)** | on the path | reads packets |
| **Network attacker (active)** | on the path | drops, delays, reorders, duplicates, injects and replays packets; may try to hijack a connection |
| **Stranger on the internet** | can reach a listening server's port | sends packets to a server it is not part of; scans for servers |
| **Hostile content provider** | supplies a mod or a level | the session loads their content (post-1.0, `docs/MODDING_POLICY.md`) |

Outside this model: an attacker with control of the victim's own machine; a malicious maintainer; physical access.

## 3. Threats and defences

Status: **built** = code exists and is tested (the page named is where); **proposed** = in an RFC that is in review; **planned** =
a roadmap task, nothing written; **gap** = nothing is planned. "Untested" means no test or measurement against the real thing.

### 3.1 Malformed or hostile packets (a client or a server sends garbage)

| Threat | Defence | Status |
|---|---|---|
| A packet crashes the receiver (panic, out-of-bounds) | the rule "never panic on bad input" (`docs/architecture.md`); parsers return errors; the parsers that exist are fuzzed (`docs/FUZZING.md`) | **built** for the parsers that exist; the **packet decoders do not exist yet**, so they are not fuzzed (add them with the wire format) |
| A field that claims a huge size makes the receiver allocate gigabytes | every buffer in `ss2-net` has a bound: the input buffer accepts only ticks within `max_ahead_ticks` (30) so it holds at most that many frames per client; the event receiver holds at most 4,096 out-of-order events; the event log keeps 256; the snapshot buffer 64; the unacked input history 64 (`INPUTSTREAM.md`, `EVENTS.md`, `SNAPSHOTS.md`) | **built** (tested) for those structures; **the decoder must bound lengths before allocating** (planned with the wire format) |
| A packet from another build or a different protocol | version and schema-hash check before anything else (`PROTOCOL.md`): refused with a message | **built** (the check, and the session handshake that carries it, `docs/SESSION.md`) |
| A peer sends an older or a newer message layout | same | **built** |

### 3.2 Cheating by a malicious client

The model is **server-authoritative**: the client sends **inputs, never state** (`NETCODE.md` section 1), so the main protection is
structural: there is no message by which a client can set its position, health or ammunition.

| Threat | Defence | Status |
|---|---|---|
| **Speed or teleport hack** (claim a position, move faster than allowed) | the client cannot claim a position; the server simulates movement from the inputs with its own code, so the speed is the server's | **proposed** (the model); the simulation does not exist (T08-003, T09), so **untested** |
| **Impossible inputs** (an aim angle outside the range, a button mask with unused bits, a movement vector longer than 1) | **sanity bounds on every input field, enforced on receipt**: refuse or clamp, and count | **gap in code**: the stream layer (`InputBuffer`) is generic and cannot know what an input means; the game's input type must provide a validity check that the server calls (T08-021b) |
| **Input flooding** (many frames per second, to act faster) | a token bucket in simulation ticks: new frames cost one token, 16 burst, 2 per tick refill; excess is refused and counted (`violations()`, `INPUTSTREAM.md`) | **built** (tested: a normal client is never limited over 2,000 ticks; a flood of 500 frames keeps 8) |
| **Rewriting the past** (send a different input for a tick already sent) | the first input for a tick stands; a different second one is refused and counted (`conflicting`) | **built** (tested) |
| **Acting in the future or the past** (inputs stamped far ahead to prepare, or for ticks already simulated) | refused: `too_far_ahead` beyond 30 ticks, `too_old` below the tick being simulated | **built** (tested) |
| **Lag switch / selective packet loss** to gain an advantage | server-side lag compensation is **bounded**: a maximum rewind window, so a fake lag cannot reach arbitrarily far back | **built as a library** (`docs/LAG_COMPENSATION.md`: the window is capped and every rewind reports what it was clamped to; tested), **not called by any server yet** (T08-010b) |
| **Clock manipulation** | the client's clock estimate (`ClockSync`) only places server data on the client's own timeline; the server's tick is the only time that matters, and a client that lies about it gains nothing | **built** (by design: nothing in the server reads a client clock) |
| **Aimbots, wallhacks, macros** | **cannot be prevented**: they run on the player's own machine and read or send plausible inputs. The only defence against wallhacks is not sending data the player should not see (relevancy filtering). A co-op game accepts that a friend can cheat | **gap by decision**; relevancy filtering is **planned** (T08-007) |
| **Repeated violations** | a counter per client (`violations()`) is available for a policy: warn, drop, kick, ban | the **counter is built**; **kick and ban are built** (`docs/SESSION.md`); the **policy** that decides when to call them is the game's and is planned |

### 3.3 A malicious server or host

A host controls the simulation the others see, so it can do anything inside the game; the question is what it can do **outside** it.

| Threat | Defence | Status |
|---|---|---|
| Crash or exploit a joining client with malformed packets | the same parsing defences as 3.1; the client treats the server's packets as untrusted too | **built** for the pieces that exist (e.g. the event receiver bounds its buffer against a hostile server) |
| Make the client load a hostile level or mod | content is loaded through the same sandboxed, validated readers as local content; mods are post-1.0 and need a sandbox (`MODDING_POLICY.md`, `SECURITY.md`) | **gap** until mods exist; levels come from the player's own install |
| Read files on the client | nothing in the protocol names a file path of the client; the server cannot request one | **proposed** by design; **untested** |
| Cheat *as the host* in its own game | **accepted**: it is the host's game; a listen server is not a trusted referee | **gap by decision** |

### 3.4 The network attacker

| Threat | Defence | Status |
|---|---|---|
| Eavesdropping on the traffic | encryption of the transport (QUIC with TLS 1.3 in RFC 0006) | **proposed**; **nothing is encrypted today** (no transport) |
| Modifying or injecting packets | authenticated encryption; a modified packet fails authentication | **proposed** |
| **Replaying** old packets | the transport's own replay protection (QUIC packet numbers) **and** the tick checks above (a replayed input for a past tick is `too_old`; a duplicate is counted and ignored); events are deduplicated by sequence number (`EVENTS.md`) | **built** for the application layer (tested over a link that duplicates 10 % of packets); transport **proposed** |
| Man in the middle on connect | the client **pins the host's key or certificate fingerprint**, learned from the invite or the discovery step (RFC 0006); without a certificate authority there is no other way for a listen server | **proposed**, not designed in detail, **untested** |
| Hijacking a connection | the encrypted, authenticated transport; an attacker cannot forge packets for an existing connection | **proposed** |

### 3.5 Denial of service

| Threat | Defence | Status |
|---|---|---|
| A flood of connection attempts to a listening server | per-address limits on half-open and total connections; QUIC limits the amplification of unauthenticated packets by design (RFC 9000, 3x until the address is validated); a retry token can be required under load | **partly built**: the session layer keeps what it remembers about refused peers bounded and holds no state for them (`docs/SESSION.md`); per-address limits on attempts, half-open limits and retry tokens are **planned** (T08-020) and the transport's (**proposed**); **untested** |
| A flood of data from a connected client | the input rate limit above; a total per-client bandwidth cap | input limit **built**; bandwidth cap **planned** (not part of the session layer: the transport and the dedicated server, T08-020) |
| Memory exhaustion | every buffer is bounded (3.1) | **built** for the structures that exist |
| **Amplification / reflection** (the server answers a spoofed address with more data than it was sent) | do not answer unauthenticated packets beyond a handshake of bounded size | **proposed** with the transport; **untested** |
| Slowloris-style (open a connection and say nothing) | idle and handshake timeouts | **built** in the session layer for the client and for connected peers (join retries and a 10 s give-up, a keepalive and a 10 s idle timeout; `docs/SESSION.md`); a server-side timeout for a peer that connected at the transport and never says hello is the transport's job (**planned**) |
| A slow client holds up the server | the server never waits for a client: a missing input is repeated for 5 ticks, then neutral (`InputBuffer::take`) | **built** (tested) |

### 3.6 Information and privacy

| Threat | Defence | Status |
|---|---|---|
| A peer learns the IP addresses of the others (peer-to-peer exposure) | with a **client-server** model a client sees only the server's address and the server sees the clients'; the host of a listen server sees everyone's; a relay for invite codes (T08-019) would hide the host's address only if it relays traffic | **inherent to the model**; document it in the player-facing text (**no task covers a privacy note yet**; the install and FAQ pages in `docs/user/` are the place) |
| Invite code or session password guessed | the password is checked **inside** the encrypted connection and the number of attempts is limited per address | **built** for the password (a lockout after repeated wrong guesses, `docs/SESSION.md`; untested against a real network); the **length and format of invite codes** are undecided (T08-019) |
| Player names or chat text used to attack a log, a terminal or the UI (control characters, very long strings, markup) | sanitize and bound text at the boundary; log with structured fields, never by concatenating raw text into a format string | **gap**: no chat or name handling exists; add the rule to T08-012's consumers and T10 |
| Information a client should not have (enemies behind walls) | relevancy filtering: send only the entities a client may know about | **planned** (T08-007) |

### 3.7 Cheating the persistence

Saves and scores (T08-022) can be edited by their owner: single-player saves are the player's. A co-op save is the host's.
Marking saves made with cheats (`docs/CHEATS.md` "Not covered") is a product decision, not a defence.

## 4. Decisions this model makes

1. **Client-server only**; no peer-to-peer game state. (Design spec.)
2. **The server never trusts a client-reported state or clock**; the protocol has no message for it.
3. **Every packet field is validated and every buffer bounded before use**; a violation is counted, not fatal.
4. **Anti-cheat is limited to what the server can know**: it does not try to detect a modified client, and says so to the player.
5. **Host cheating and client-side cheats on the player's own machine are accepted.**
6. **Security of the transport is bought, not built** (RFC 0006): the project does not design its own cryptography.

## 5. Gaps, in the order I would close them

1. The **wire format decoders**: the session messages (T08-006) and the replay file (T08-025) bound every length before allocating and have fuzz targets (`docs/FUZZING.md`); the **snapshot, input and event packets** do not have a decoder yet, and each will need the same bounds and a **fuzz target** with it.
2. **Input validity checks by the game** (T08-021b): the sanity bounds on aim, movement and buttons, called by the server on every accepted frame; count and refuse.
3. **Per-address connection limits, bandwidth caps, a transport-level handshake timeout and the violation policy** (T08-020 and the transport; the session layer of T08-006 is built).
4. **Certificate or key pinning** in the join flow (T08-019, RFC 0006), with a design review.
5. **Relevancy filtering** and **bounded lag compensation** (T08-007, T08-010).
6. A **real-network test** of the transport under hostile conditions (T08-017b): so far everything was run in a simulator or on loopback.

## 6. Reporting

A vulnerability in any of this: use the private reporting of `SECURITY.md`; do not open a public issue.
