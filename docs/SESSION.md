# Sessions: joining, staying connected, leaving

`ss2_net::session` (T08-006) is the life of a connection **above the transport**: a client says hello, the server admits or refuses it
with a reason a person can read, both sides keep each other alive, and either side says goodbye. It is pure logic over messages like the
rest of `ss2-net`: no sockets, no clock (time is a number of microseconds passed in), no randomness. The transport (RFC 0006, still a
proposal) carries the messages and is expected to encrypt them, so the password travels inside that protection.

## Joining

The client sends **`Hello`**: protocol version and schema hash (`docs/PROTOCOL.md`), the hash of its mods, a name, the session password and
an optional auth token. The server checks in this order and stops at the first failure, answering **`Refused(reason)`**:

1. the peer is **banned** (`Banned`);
2. the peer is **locked out** for guessing the password (`RateLimited { retry_after_ms }`), whatever it sends now;
3. the **builds can talk** (`VersionMismatch { server }`, `SchemaMismatch`: `check_handshake`);
4. the **mods match** (`ModMismatch`);
5. the **name** is 1 to 32 bytes, without control characters and not blank (`InvalidName`) and **not in use**, compared without case (`NameTaken`);
6. the **password** is right (`BadPassword`), compared in constant time; a failure counts toward the lockout;
7. there is **room** (`ServerFull`);
8. the **auth token** is accepted by the function the host provides (`AuthFailed`).

Then **`Welcome`**: the server's handshake (the client checks it too and leaves with `Incompatible` if the builds cannot talk, whatever
the server claimed), the slot (the **lowest free** one), the tick rate and the server's tick. A **repeated hello** from a peer that is
already in (the welcome was lost) is answered with the **same welcome** and takes no second slot; the client sends hello again after 0.5 s,
then 1 s, 2 s, 4 s, 4 s... and gives up after 10 s (`TimedOut`). A refused client stops retrying.

`SessionClient::refusal_text()` gives a sentence for the player ("the server is newer (protocol 1.2, this build speaks 0.9): update this
game"), built on `Compat::explanation` for builds that cannot talk.

## Staying connected and leaving

Both sides send **`Keepalive`** when they have sent nothing for the keepalive interval (default 2 s) and drop a peer that has been silent
longer than the idle timeout (default 10 s): a server that goes quiet is `TimedOut` on the client, a client that goes quiet is `Left { TimedOut }` on the
server. Any session message from the peer counts as life (other traffic: see below). **`Disconnect`** says goodbye: `Quit`, `Kicked(reason)` (the reason is cut to 64 bytes at a
character boundary), `Banned`, `ServerShutdown`.

**Any datagram is proof of life.** The session layer only sees session messages, so the integration must call `SessionServer::heard_from(peer, now)` for every datagram that arrives from a connected peer (input packets, acknowledgements) and `SessionClient::heard_from_server(now)` for every datagram from the server; without them a player whose game traffic is flowing is dropped when five keepalives in a row are lost (found by the soak test, `docs/SOAK.md`: at 30 % loss each way every bot was dropped within half an hour). **Nothing calls them yet** (T08-004).

The host can `kick(slot, reason)` (not a ban: the player may return), `ban(peer)` (disconnects it if connected and refuses it until `unban`),
and `shutdown()`. The server's `take_events()` reports `Joined { peer, slot, name }` and `Left { peer, slot, reason }`; the client's reports
`Connected`, `Refused`, `Incompatible`, `Disconnected`, `TimedOut`.

## The wire format

One byte of kind, then little-endian fields; a blob is a 16-bit length and the bytes.

```
Hello      = 1 · major u16 · minor u16 · schema u64 · mod_hash u64 · name blob(<=32) · password blob(<=128) · token blob(<=256)
Welcome    = 2 · major u16 · minor u16 · schema u64 · mod_hash u64 · slot u8 · tick_hz u32 · server_tick u64
Refused    = 3 · code u8 (0 version + major u16 + minor u16, 1 schema, 2 mods, 3 password, 4 full, 5 banned, 6 name, 7 taken, 8 auth, 9 rate limited + retry_after_ms u32)
Keepalive  = 4 · seq u32
Disconnect = 5 · code u8 (0 quit, 1 kicked + reason blob(<=64), 2 banned, 3 shutdown)
```

The decoder checks every length against its limit **before** copying anything, refuses unknown kinds and codes, truncated packets,
bytes after the message and text that is not UTF-8, each with a named error. A hello with a name over 32 bytes therefore cannot be decoded
and the server ignores it; a client should keep names short (nothing clips the name for it).

## Hostile peers

- **A refused peer costs the server nothing but a bounded counter.** Only wrong passwords are remembered (per peer: a count in a window and
  a lockout time), at most 1,024 peers; when full, expired records go first, then the oldest record that is not a lockout, so a flood of
  addresses cannot easily flush out a real lockout. A test sends 50,000 wrong passwords from 50,000 peers.
- **Guessing the password** (default: 5 wrong in 60 s) locks the peer out for 5 minutes, **even for the right password**, and tells it how
  long to wait.
- **Messages from a peer that never said hello** (a keepalive, a disconnect, a welcome) are ignored and create nothing.
- **Duplicate hellos** (a duplicating link, a retry) never take a second slot.

## Tests

Twenty-three tests plus a **fuzz target**, `session_message` (`docs/FUZZING.md`): every message through the wire, the decoder on every
truncation of a hello and on 20,000 random byte strings, joining on a perfect link, joining on a link with 20 % loss and 10 % duplication
(for 20 seeds: always exactly one slot and one `Joined`), the refusals for version, schema, mods, names, password, a full server, the auth
function, the lockout and its expiry, the window forgetting old failures, the bounded failure memory, kicks, bans and unbans, a minute of
keepalives over the lossy link, both kinds of timeout, the retry backoff and the give-up time, a polite leave and a shutdown, and messages
from strangers. The fuzz target (90 seconds locally, 1.7 million runs, no finding) feeds any bytes to the decoder and, if they decode, to a
server from several peers, checking that the message round-trips and that slots and failure records stay within bounds; it has not run in CI.

Thirteen deliberate breakages each fail a test (the password comparison ignoring length, no lockout, a window that never expires, the ban
check removed, a duplicate hello admitted again, no capacity check, names compared with case, a keepalive that does not refresh the server's
timer, no backoff, the welcome's handshake not checked, the failure memory unbounded, the highest free slot, the kick reason not cut). **Three
survived at first** (the comparison ignoring length, which only shows for a trailing zero byte; the kick reason; and two that had matched
nothing because rustfmt had reflowed the code) and I added tests and re-ran them to see them fail. Two of my own tests were wrong, not the code: a keepalive
test on a lossy link with a 6 s timeout failed because three keepalives in a row can be lost, and an over-long name is dropped by the
decoder instead of being refused.

## Not covered

- **Per-address limits on connection attempts** (a flood of hellos from one address costs a decode and a few checks each, but is not limited),
  **bandwidth caps**, and a **handshake timeout on the server** for a peer that connected at the transport level and says nothing: these are
  the transport's and the dedicated server's job (RFC 0006, T08-020).
- **Reconnect** with the same identity (T08-014), **session discovery** and invite codes (T08-019), **host migration**.
- **Authentication**: the token is passed to a function the host provides; what a token is (a platform account, a signed invite) is undecided.
- **Real sockets and real encryption**: nothing here has run over a network; the tests use the simulator.
- **Wiring** into the server and client simulation and the dedicated server binary (T08-004, T08-020).
